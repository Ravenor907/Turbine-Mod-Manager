//! Mods for any game (0.56): Basic support's "Where do these files go?" page and the question about files the game
//! already has, the Nexus game of games without one in their data file ("Which game on Nexus is this?"), the
//! player's own game files, and anti-cheat. The engine decides (basic.rs, nexus_games.rs); this hands it to QML.

use crate::backend::{data_home, qobject, Slot, Worker};
use crate::online::Net;
use core::pin::Pin;
use cxx_qt::{CxxQtThread, CxxQtType};
use cxx_qt_lib::QString;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use turbine_engine::json::quote;
use turbine_engine::nexus_games::{self, Answer, Picks};
use turbine_engine::rules::{self, Support};
use turbine_engine::store::{self, PendingPlace, PlaceInfo, Store};
use turbine_engine::{basic, Game};

type Qt = CxxQtThread<qobject::Turbine>;

/// Turbine's own data folder (the activity log's folder): picks, the cached game list, the player's game files.
pub(crate) fn turbine_data() -> PathBuf {
    crate::log::folder(&data_home())
}

/// The player's own game files: <data>/games/*.json.
pub(crate) fn player_games_dir() -> PathBuf {
    turbine_data().join("games")
}

fn answers() -> &'static Mutex<HashMap<String, Answer>> {
    static A: std::sync::OnceLock<Mutex<HashMap<String, Answer>>> = std::sync::OnceLock::new();
    A.get_or_init(|| Mutex::new(HashMap::new()))
}

/// What Turbine knows about a game's Nexus game now.
pub(crate) fn nexus_answer(game: &Game) -> Answer {
    if let Some(a) = answers().lock().unwrap_or_else(|e| e.into_inner()).get(&game.id()) {
        return a.clone();
    }
    nexus_games::domain_for_game(game, &Picks::default(), None)
}

/// Anti-cheat: the data file's, or found in a Basic game's folder.
pub(crate) fn anti_cheat(slot: &Slot) -> Option<rules::AntiCheat> {
    if let Some(a) = &slot.rules.anti_cheat {
        return Some(a.clone());
    }
    if slot.rules.support_level == Support::Full {
        return None;
    }
    let index = basic::GameIndex::cached(&slot.game.install_dir);
    index.anti_cheat().map(|name| rules::AntiCheat { name: name.into(), level: "warn".into(), note: String::new() })
}

/// The line for Before you play while any mod is on in a game with anti-cheat.
pub(crate) fn anti_cheat_check(slot: &Slot) -> Option<rules::Check> {
    let a = anti_cheat(slot)?;
    if !slot.store.mods().iter().any(|m| m.enabled) {
        return None;
    }
    Some(rules::Check {
        level: if a.level == "stop" { "stop".into() } else { "warn".into() },
        title: format!("Anti-cheat: {}", a.name),
        detail: if a.level == "stop" {
            format!("{} doesn't allow changed game files. Play it without mods.", slot.game.name)
        } else {
            "Anti-cheat: online play with mods can get you banned. Use mods only in offline or single player modes.".into()
        },
    })
}

/// Fields added to the rules JSON (backend::rules_json): support, anti-cheat, the Nexus game, starter mods.
pub(crate) fn rules_extra(slot: &Slot) -> String {
    let r = &slot.rules;
    let ac = anti_cheat(slot)
        .map(|a| format!("{{\"name\":{},\"level\":{},\"note\":{}}}", quote(&a.name), quote(&a.level), quote(&a.note)))
        .unwrap_or_else(|| "null".into());
    let answer = nexus_answer(&slot.game);
    let (state, name, how, cands) = match &answer {
        Answer::Sure { name, how, .. } => ("sure", name.clone(), how.to_string(), "[]".to_string()),
        Answer::Ask(c) => ("ask", String::new(), String::new(), nexus_games::games_json(c)),
        Answer::NotOnNexus => ("none", String::new(), "your pick".into(), "[]".into()),
        Answer::Unknown => ("unknown", String::new(), String::new(), "[]".into()),
    };
    let starters: Vec<String> = r
        .starters
        .iter()
        .map(|s| format!("{{\"source\":{},\"id\":{},\"name\":{},\"what\":{}}}", quote(&s.source), quote(&s.id), quote(&s.name), quote(&s.what)))
        .collect();
    format!(
        ",\"supportLevel\":{},\"supportWords\":{},\"placesByDetection\":{},\"antiCheat\":{},\"domainState\":{},\"nexusName\":{},\"nexusHow\":{},\"nexusCandidates\":{},\"starters\":[{}],\"refusedWords\":{}",
        quote(r.support_level.key()),
        quote(r.support_level.words()),
        r.places_by_detection(),
        ac,
        quote(state),
        quote(&name),
        quote(&how),
        cands,
        starters.join(","),
        quote(basic::REFUSED_WORDS)
    )
}

fn where_words(to: &str) -> String {
    if to.is_empty() {
        "the game folder".into()
    } else {
        to.to_string()
    }
}

/// The activity log line for a placed mod, and the words for the status line.
pub(crate) fn note_placed(name: &str, info: &PlaceInfo) -> String {
    if info.rule == "you" {
        crate::log::line("engine", &format!("Placed by you: {} ({name})", where_words(&info.to)));
    } else {
        crate::log::line("engine", &format!("Placed by rule {}: {} ({name})", info.rule, where_words(&info.to)));
    }
    if !info.skipped.is_empty() {
        crate::log::line("engine", &format!("Skipped existing for {name}: {}", info.skipped.join(", ")));
    }
    format!("Added {name}. It went into {}.", where_words(&info.to))
}

/// A Basic game's mod waits for the player: queued, and shown when it's the first.
pub(crate) fn queue_place(w: &mut Worker, game: &Game, pending: PendingPlace) {
    if pending.sure {
        crate::log::line("engine", &format!("Asked about files {} already has: {} ({})", game.name, pending.name, pending.conflicts.join(", ")));
    } else {
        crate::log::line("engine", &format!("Asked where files go: {} ({})", pending.name, basic::placement_json(&pending.placement)));
    }
    w.place_pending.push_back((game.id(), pending));
    if w.place_pending.len() == 1 {
        post_place(w);
    }
}

/// Sends the first waiting placement to QML (or none).
pub(crate) fn post_place(w: &Worker) {
    let (json, files, gid) = match w.place_pending.front() {
        Some((gid, p)) => {
            let game_name = w.slots.get(gid).map(|s| s.game.name.clone()).unwrap_or_default();
            (format!("{{\"game\":{},\"gameName\":{},\"pending\":{}}}", quote(gid), quote(&game_name), p.to_json()), p.files.clone(), gid.clone())
        }
        None => (String::new(), Vec::new(), String::new()),
    };
    w.post(move |mut t| {
        t.as_mut().rust_mut().place_files = files;
        t.as_mut().rust_mut().place_game = gid;
        t.as_mut().set_place_json(QString::from(json.as_str()));
    });
}

fn set_qt(qt: &Qt, f: impl FnOnce(Pin<&mut qobject::Turbine>) + Send + 'static) {
    let _ = qt.queue(f);
}

/// The Nexus game of every game without one in its data file: the player's picks, then exact names in Nexus's game
/// list (fetched again when it is over a week old). Sure answers are registered so links and Get mods find them.
pub(crate) fn resolve(net: &mut Net, games: &[Game]) -> Vec<String> {
    let dir = turbine_data();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let mut lines = Vec::new();
    let need = games.iter().any(|g| rules::find_def(g.store, &g.store_id).and_then(|d| d.data.nexus.as_deref()).is_none());
    if need && !net.is_sample() && nexus_games::list_is_old(&dir, now) {
        match nexus_games::refresh_list(&net.client, &dir, crate::VERSION) {
            Ok(n) => lines.push(format!("Nexus game list: {n} games")),
            Err(e) => lines.push(format!("Nexus game list not fetched ({e}); the old copy stays")),
        }
    }
    let list = nexus_games::cached_list(&dir);
    let picks = Picks::load(&dir);
    let mut map = answers().lock().unwrap_or_else(|e| e.into_inner());
    for g in games {
        let a = nexus_games::domain_for_game(g, &picks, list.as_deref());
        match &a {
            Answer::Sure { domain, name, how } => {
                if *how != "data file" {
                    turbine_engine::nexus::register(g.store, &g.store_id, domain, name);
                    lines.push(format!("Nexus game for {}: {domain} ({how})", g.name));
                }
            }
            Answer::Ask(c) => {
                turbine_engine::nexus::unregister(g.store, &g.store_id);
                lines.push(format!("Nexus game for {}: ask ({} candidates)", g.name, c.len()));
            }
            Answer::NotOnNexus => {
                turbine_engine::nexus::unregister(g.store, &g.store_id);
                lines.push(format!("Nexus game for {}: not on Nexus (your pick)", g.name));
            }
            Answer::Unknown => {}
        }
        map.insert(g.id(), a);
    }
    lines
}

impl qobject::Turbine {
    /// The player's own game files, read again (at start and with each scan).
    pub(crate) fn load_player_games(&self) {
        let dir = player_games_dir();
        for p in rules::load_player_games(&dir) {
            crate::log::line("scan", &format!("Your game file {p}"));
        }
        let n = rules::player_games().len();
        if n > 0 {
            crate::log::line("scan", &format!("Your game files: {n} games from {}", dir.display()));
        }
    }

    /// Works out the Nexus game of every game after a scan (on the download lane), then shows the current game again.
    pub(crate) fn resolve_nexus(mut self: Pin<&mut Self>) {
        let games = self.rust().found.clone();
        self.as_mut().net_send(
            "Nexus games",
            Box::new(move |net: &mut Net, qt: &Qt| {
                for l in resolve(net, &games) {
                    crate::log::line("net", &l);
                }
                set_qt(qt, |t| t.reshow());
            }),
        );
    }

    /// Shows the current game's mods and rules again (after a Nexus answer changed).
    pub(crate) fn reshow(self: Pin<&mut Self>) {
        let Some(game) = self.current_game() else { return };
        self.send(
            Some(game.id()),
            "Show rules again",
            false,
            Box::new(move |w: &mut Worker| {
                if w.slots.contains_key(&game.id()) {
                    w.show(&game, true);
                }
            }),
        );
    }

    pub fn game_folders(&self, rel: &QString) -> QString {
        let gid = if self.rust().place_game.is_empty() { self.rust().current.clone() } else { self.rust().place_game.clone() };
        let Some(game) = self.find(&gid) else { return QString::from("[]") };
        let index = basic::GameIndex::cached(&game.install_dir);
        let made = store::made_dirs(&Store::root_for(&game, &data_home()));
        QString::from(basic::game_folders_json(&index, &rel.to_string(), &made).as_str())
    }

    pub fn place_check(&self, from: &QString, to: &QString) -> QString {
        let Some(game) = self.find(&self.rust().place_game) else { return QString::from("{\"error\":\"Nothing is waiting.\"}") };
        let files = self.rust().place_files.clone();
        let (from, to) = (from.to_string(), to.to_string());
        let j = match store::conflicts_in(&game, &Store::root_for(&game, &data_home()), &files, &from, &to) {
            Ok((c, r)) => {
                let n = basic::map_placement(&files, &from, &to).map(|m| m.len()).unwrap_or(0);
                format!(
                    "{{\"error\":\"\",\"files\":{n},\"conflicts\":[{}],\"refused\":[{}]}}",
                    c.iter().map(|x| quote(x)).collect::<Vec<_>>().join(","),
                    r.iter().map(|(p, w)| format!("{{\"path\":{},\"why\":{}}}", quote(p), quote(w))).collect::<Vec<_>>().join(",")
                )
            }
            Err(e) => format!("{{\"error\":{},\"files\":0,\"conflicts\":[],\"refused\":[]}}", quote(&e)),
        };
        QString::from(j.as_str())
    }

    pub fn finish_place(mut self: Pin<&mut Self>, from: &QString, to: &QString, replace: bool) {
        let (from, to) = (from.to_string(), to.to_string());
        let gid = self.rust().place_game.clone();
        let Some(game) = self.find(&gid) else { return };
        self.as_mut().edited();
        self.send(
            Some(gid.clone()),
            "Put a mod's files in place",
            true,
            Box::new(move |w: &mut Worker| {
                let Some((pgid, pending)) = w.place_pending.pop_front() else { return };
                if pgid != gid {
                    w.place_pending.push_front((pgid, pending));
                    return;
                }
                let result = w.slot(&game).and_then(|s| s.store.finish_place(pending, &from, &to, replace));
                match result {
                    Ok(done) => {
                        for n in &done.notes {
                            crate::log::line("engine", &format!("{}: {n}", done.item.name));
                        }
                        let words = done.placed.as_ref().map(|p| note_placed(&done.item.name, p)).unwrap_or_else(|| format!("Added {}", done.item.name));
                        if replace {
                            crate::log::line("engine", &format!("{} may replace {}'s own files (kept and put back when it goes)", done.item.name, game.name));
                        }
                        w.say(words, "ok");
                        crate::actions::show_notes(w, &done.item.name, &done.notes);
                        let (g, id, n, on) = (game.id(), done.item.id.clone(), done.item.name.clone(), done.item.enabled);
                        w.post(move |t| t.arrived(QString::from(g.as_str()), QString::from(id.as_str()), QString::from(n.as_str()), on));
                    }
                    Err(e) => w.say(e, "error"),
                }
                post_place(w);
                w.show(&game, true);
            }),
        );
    }

    pub fn cancel_place(self: Pin<&mut Self>) {
        let gid = self.rust().place_game.clone();
        self.send(
            Some(gid.clone()),
            "Not added",
            false,
            Box::new(move |w: &mut Worker| {
                let Some((pgid, pending)) = w.place_pending.pop_front() else { return };
                let name = pending.name.clone();
                if let Some(s) = w.slots.get_mut(&pgid) {
                    s.store.cancel_place(pending);
                }
                crate::log::line("engine", &format!("Not added: {name} (where its files go wasn't chosen)"));
                w.say("Not added. The file is still in Downloads.", "warn");
                post_place(w);
            }),
        );
    }

    pub fn change_placement(mut self: Pin<&mut Self>, id: &QString, to: &QString) {
        let (id, to) = (id.to_string(), to.to_string());
        let Some(game) = self.current_game() else { return };
        self.as_mut().edited();
        self.send(
            Some(game.id()),
            "Change where a mod goes",
            true,
            Box::new(move |w: &mut Worker| {
                let r = w.slot(&game).map_err(|e| e.to_string()).and_then(|s| s.store.change_placement(&id, &to).map_err(|e| e.to_string()));
                match r {
                    Ok(()) => {
                        crate::log::line("engine", &format!("Placed by you: {} (moved {id})", where_words(&to)));
                        w.say(format!("Moved. Its files go into {} now.", where_words(&to)), "ok");
                    }
                    Err(e) => w.say(e, "error"),
                }
                w.show(&game, true);
            }),
        );
    }

    pub fn set_replace(mut self: Pin<&mut Self>, id: &QString, replace: bool) {
        let id = id.to_string();
        let Some(game) = self.current_game() else { return };
        self.as_mut().edited();
        self.send(
            Some(game.id()),
            "Replace the game's files or not",
            false,
            Box::new(move |w: &mut Worker| {
                if let Ok(s) = w.slot(&game) {
                    if let Err(e) = s.store.set_replace(&id, replace) {
                        w.say(e.to_string(), "error");
                    }
                }
                w.show(&game, true);
            }),
        );
    }

    /// The player's answer to "Which game on Nexus is this?" ("" for "It isn't on Nexus").
    pub fn pick_nexus_game(mut self: Pin<&mut Self>, domain: &QString, name: &QString) {
        let Some(game) = self.current_game() else { return };
        let (domain, name) = (domain.to_string(), name.to_string());
        let dir = turbine_data();
        let mut picks = Picks::load(&dir);
        let key = nexus_games::key(game.store, &game.store_id);
        picks.set(&key, if domain.is_empty() { None } else { Some(domain.as_str()) });
        if let Err(e) = picks.save(&dir) {
            crate::log::line("net", &format!("Couldn't save the Nexus game pick ({e})"));
        }
        let a = if domain.is_empty() {
            turbine_engine::nexus::unregister(game.store, &game.store_id);
            Answer::NotOnNexus
        } else {
            turbine_engine::nexus::register(game.store, &game.store_id, &domain, if name.is_empty() { &game.name } else { &name });
            Answer::Sure { domain: domain.clone(), name: if name.is_empty() { game.name.clone() } else { name.clone() }, how: "your pick" }
        };
        crate::log::line("net", &format!("Nexus game for {}: {} (your pick)", game.name, if domain.is_empty() { "not on Nexus" } else { domain.as_str() }));
        answers().lock().unwrap_or_else(|e| e.into_inner()).insert(game.id(), a);
        self.as_mut().reshow();
    }

    /// Search Nexus's games by name (the answer lands in nexusGamesJson).
    pub fn search_nexus_games(mut self: Pin<&mut Self>, text: &QString) {
        let text = text.to_string();
        self.as_mut().set_nexus_games_json(QString::from("{\"state\":\"busy\",\"games\":[]}"));
        self.net_send(
            "Search Nexus games",
            Box::new(move |net: &mut Net, qt: &Qt| {
                let found: Result<Vec<nexus_games::NexusGame>, String> = if net.is_sample() {
                    // The build's checks: the recorded list, matched by the same words.
                    let words: Vec<String> = nexus_games::normalize(&text).split(' ').map(String::from).collect();
                    Ok(nexus_games::cached_list(&turbine_data())
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|g| {
                            let n = nexus_games::normalize(&g.name);
                            words.iter().all(|w| n.contains(w.as_str()))
                        })
                        .take(10)
                        .collect::<Vec<_>>())
                } else {
                    net.nexus_public_client().games_search(&text).map_err(|e| e.to_string())
                };
                let j = match found {
                    Ok(list) => {
                        crate::log::line("net", &format!("Nexus games for \"{text}\": {}", list.len()));
                        format!("{{\"state\":\"done\",\"games\":{}}}", nexus_games::games_json(&list))
                    }
                    Err(e) => {
                        crate::log::line("net", &format!("Nexus game search failed: {e}"));
                        format!("{{\"state\":\"error\",\"error\":{},\"games\":[]}}", quote(&e))
                    }
                };
                set_qt(qt, move |t| t.set_nexus_games_json(QString::from(j.as_str())));
            }),
        );
    }
}
