//! What 0.3 adds to the bridge: Get Mods (Thunderstore and Nexus Mods), downloads, the loader install, installer
//! choices (FOMOD), the Load order tab, profiles, mod updates, the Nexus account and Turbine's own updates.
//!
//! Network work runs on the download lane (online.rs); anything that changes a game's mods runs on the mod worker
//! (backend.rs), so a download never stalls browsing and the store is only ever touched from one thread.

use crate::backend::{data_home, qobject, Slot, Worker};
use crate::online::{self, Fetched, GameInfo, Have, Net};
use core::pin::Pin;
use cxx_qt::{CxxQtThread, CxxQtType, Threading};
use cxx_qt_lib::QString;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use turbine_engine::json::{self, quote, Json};
use turbine_engine::net::SourceInfo;
use turbine_engine::profiles::{self, Profiles, Share};
use turbine_engine::rules::Kind;
use turbine_engine::store::Import;
use turbine_engine::{plugins, thunderstore, Game, Runtime, Store as Launcher};

type Qt = CxxQtThread<qobject::Turbine>;
pub(crate) type NetTask = Box<dyn FnOnce(&mut Net, &Qt) + Send>;

const APP_ID: &str = "io.github.ravenor907.TurbineModManager";

/// The Thunderstore community of a Steam game, when it has one.
pub(crate) fn community(game: &Game) -> Option<&'static str> {
    if game.store != Launcher::Steam {
        return None;
    }
    thunderstore::community_for_steam_app(&game.store_id)
}

/// What the download lane needs to know about a game, from its store.
pub(crate) fn game_info(slot: &Slot) -> GameInfo {
    let g = &slot.game;
    let mods = slot.store.mods();
    GameInfo {
        id: g.id(),
        name: g.name.clone(),
        community: community(g).map(str::to_string),
        domain: turbine_engine::nexus::domain_for_store_id(&g.store_id).map(str::to_string),
        kind: match slot.rules.kind {
            Kind::Bethesda => "bethesda",
            Kind::BepInEx => "bepinex",
            Kind::Smapi => "smapi",
            Kind::Generic => "generic",
        }
        .into(),
        loader_ok: slot.rules.loader_installed(g),
        native: g.runtime == Runtime::Native,
        steam: g.store == Launcher::Steam,
        have: slot
            .store
            .installed_sources()
            .into_iter()
            .map(|(id, source)| {
                let added = mods.iter().find(|m| m.id == id).map(|m| m.added).unwrap_or(0);
                Have { mod_id: id, source, added }
            })
            .collect(),
    }
}

/// Turbine's cache folder (in the Flatpak, ~/.var/app/<id>/cache/turbine-mod-manager: the host can read it, which
/// the self-update's installer needs).
fn cache_dir() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| turbine_engine::home_dir().join(".cache"))
        .join("turbine-mod-manager")
}

// ---------------------------------------------------------------------------------------------------- posting back

fn post<F>(qt: &Qt, f: F)
where
    F: FnOnce(Pin<&mut qobject::Turbine>) + Send + 'static,
{
    let _ = qt.queue(f);
}

/// A sentence for the status line from the download lane.
fn say(qt: &Qt, text: impl Into<String>, level: &'static str) {
    let text = text.into();
    crate::log::line("net", &format!("[{level}] {text}"));
    post(qt, move |t| t.message(QString::from(text.as_str()), QString::from(level)));
}

fn set<F>(qt: &Qt, f: F)
where
    F: FnOnce(Pin<&mut qobject::Turbine>) + Send + 'static,
{
    post(qt, f);
}

fn err_json(e: &str) -> String {
    format!("{{\"error\":{}}}", quote(e))
}

/// Progress for the status line and the rail's Downloads ring, a few times a second at most.
fn progress(qt: &Qt) -> impl FnMut(&str, u64, Option<u64>) + '_ {
    let mut last = Instant::now() - Duration::from_secs(1);
    move |label: &str, got: u64, total: Option<u64>| {
        let done = total.is_some_and(|t| got >= t);
        if !done && last.elapsed() < Duration::from_millis(150) {
            return;
        }
        last = Instant::now();
        let j = format!(
            "{{\"active\":true,\"label\":{},\"got\":{},\"total\":{}}}",
            quote(label),
            got,
            total.map(|t| t.to_string()).unwrap_or_else(|| "0".into())
        );
        post(qt, move |t| t.set_download_json(QString::from(j.as_str())));
    }
}

/// Every downloaded file goes to the mod worker to be added.
fn hand_over(qt: &Qt, game: &Game) -> impl FnMut(Fetched) + '_ {
    let game = game.clone();
    move |f: Fetched| {
        let g = game.clone();
        post(qt, move |t| t.add_fetched(g, f));
    }
}

fn remove_download(path: &Path) {
    let _ = std::fs::remove_file(path);
    if let Some(dir) = path.parent() {
        if dir.file_name().is_some_and(|n| n.to_string_lossy().starts_with("named-")) {
            let _ = std::fs::remove_dir(dir);
        }
    }
}

// ---------------------------------------------------------------------------------------------------- mod worker side

/// Adds (or, with `replaces`, updates) a mod from an archive. An installer with choices waits for the player.
pub(crate) fn add_archive(w: &mut Worker, game: &Game, path: &Path, source: SourceInfo, replaces: Option<String>, label: &str) {
    let result = w.slot(game).and_then(|s| match &replaces {
        Some(id) => s.store.replace_mod(id, path, source),
        None => s.store.import_archive(path, source),
    });
    match result {
        Ok(Import::Added(done)) => {
            for n in &done.notes {
                crate::log::line("engine", &format!("{}: {n}", done.item.name));
            }
            let verb = if replaces.is_some() { "Updated" } else { "Added" };
            w.say(format!("{verb} {}", done.item.name), "ok");
            if let Some(id) = replaces {
                let gid = game.id();
                w.post(move |t| t.drop_update(&gid, &id));
            }
        }
        Ok(Import::NeedsChoices(p)) => {
            w.say(format!("{} has installer choices", p.name), "work");
            w.pending.push_back((game.id(), *p));
            if w.pending.len() == 1 {
                post_fomod(w);
            }
        }
        Err(e) => w.say(if e.contains(label) { e } else { format!("Couldn't add {label}: {e}") }, "error"),
    }
    w.show(game, true);
}

/// Puts the first waiting installer on screen (or takes the installer screen away when none is left).
fn post_fomod(w: &Worker) {
    let json = match w.pending.front() {
        None => String::new(),
        Some((gid, p)) => {
            let data = p.to_json();
            let mut paths: Vec<String> = Vec::new();
            if let Ok(j) = json::parse(&data) {
                if let Some(i) = j.str_at("image") {
                    paths.push(i.to_string());
                }
                if let Some(step) = j.get("step") {
                    for g in step.get("groups").map(Json::as_array).unwrap_or(&[]) {
                        for o in g.get("options").map(Json::as_array).unwrap_or(&[]) {
                            if let Some(i) = o.str_at("image") {
                                paths.push(i.to_string());
                            }
                        }
                    }
                }
            }
            let images: Vec<String> = paths
                .iter()
                .filter_map(|a| p.image_file(a).map(|f| format!("{}:{}", quote(a), quote(&f.to_string_lossy()))))
                .collect();
            format!(
                "{{\"game\":{},\"waiting\":{},\"data\":{},\"images\":{{{}}}}}",
                quote(gid),
                w.pending.len(),
                data,
                images.join(",")
            )
        }
    };
    w.post(move |t| t.set_fomod_json(QString::from(json.as_str())));
}

fn game_of(w: &Worker, gid: &str) -> Option<Game> {
    w.slots.get(gid).map(|s| s.game.clone())
}

/// Keeps plugins.txt in step after an apply (Skyrim): new plugins are listed, removed ones dropped.
pub(crate) fn sync_order(w: &mut Worker, game: &Game) {
    let plugins = w.slot(game).map(|s| s.rules.plugins).unwrap_or(false);
    if plugins {
        if let Err(e) = plugins::sync(game) {
            crate::log::line("engine", &format!("Load order not synced: {e}"));
        }
    }
}

fn order_json(game: &Game, lo: Result<plugins::LoadOrder, String>) -> String {
    match lo {
        Ok(lo) => format!("{{\"game\":{},\"plugins\":{},\"error\":\"\"}}", quote(&game.id()), lo.to_json()),
        Err(e) => format!("{{\"game\":{},\"plugins\":[],\"error\":{}}}", quote(&game.id()), quote(&e)),
    }
}

fn profiles_json(w: &mut Worker, game: &Game) -> String {
    match w.slot(game).and_then(|s| Profiles::open(&s.store).map_err(|e| format!("Profiles couldn't be read ({e})."))) {
        Ok(p) => format!("{{\"game\":{},\"data\":{},\"error\":\"\"}}", quote(&game.id()), p.to_json()),
        Err(e) => format!("{{\"game\":{},\"data\":{{\"active\":\"\",\"profiles\":[]}},\"error\":{}}}", quote(&game.id()), quote(&e)),
    }
}

fn post_profiles(w: &mut Worker, game: &Game) {
    let j = profiles_json(w, game);
    w.post(move |t| t.set_profiles_json(QString::from(j.as_str())));
}

/// Reads a share against the game's mods and keeps it for "Get them".
fn plan_share(w: &mut Worker, game: &Game, share: Share) {
    let json = match w.slot(game).and_then(|s| profiles::plan_import(&share, &s.store)) {
        Ok(plan) => {
            let j = format!(
                "{{\"game\":{},\"profile\":{},\"plan\":{},\"error\":\"\"}}",
                quote(&game.id()),
                quote(&share.profile),
                profiles::plan_json(&plan)
            );
            w.share = Some((game.id(), share));
            j
        }
        Err(e) => format!("{{\"game\":{},\"profile\":\"\",\"plan\":null,\"error\":{}}}", quote(&game.id()), quote(&e)),
    };
    w.post(move |t| t.set_import_json(QString::from(json.as_str())));
}

/// After a share's mods are in: a profile with its order and on/off, switched to.
fn finish_share(w: &mut Worker, game: &Game) {
    let Some((gid, share)) = w.share.take() else { return };
    if gid != game.id() {
        return;
    }
    let result = w.slot(game).and_then(|s| {
        let mut p = Profiles::open(&s.store).map_err(|e| format!("Profiles couldn't be read ({e})."))?;
        let names = p.names();
        let mut name = share.profile.trim().to_string();
        if name.is_empty() {
            name = "Shared profile".into();
        }
        let base = name.clone();
        let mut n = 2;
        while names.iter().any(|x| x.eq_ignore_ascii_case(&name)) {
            name = format!("{base} {n}");
            n += 1;
        }
        let order = profiles::share_order(&share, &s.store);
        let old = s.store.order();
        p.save_current(&s.store).map_err(|e| format!("The current profile couldn't be saved ({e})."))?;
        s.store.set_order(&order).map_err(|e| format!("The profile couldn't be made ({e})."))?;
        let made = p.create(&s.store, &name);
        s.store.set_order(&old).map_err(|e| format!("The profile couldn't be made ({e})."))?;
        made?;
        let r = p.switch(&mut s.store, &name)?;
        Ok((name, r))
    });
    match result {
        Ok((name, r)) => {
            w.note(game, &r);
            w.say(format!("Profile {name} is ready and in use"), "ok");
        }
        Err(e) => w.say(e, "error"),
    }
    let empty = String::new();
    w.post(move |t| t.set_import_json(QString::from(empty.as_str())));
    post_profiles(w, game);
    w.show(game, false);
}

// ---------------------------------------------------------------------------------------------------- the object

impl qobject::Turbine {
    /// Queues a job for the download lane (started the first time).
    fn net_send(mut self: Pin<&mut Self>, name: &str, task: NetTask) {
        if self.rust().net.is_none() {
            let (tx, rx) = mpsc::channel::<NetTask>();
            let qt = self.qt_thread();
            let spawned = std::thread::Builder::new().name("turbine-net".into()).spawn(move || {
                let mut net = Net::new(crate::VERSION, &cache_dir(), &crate::log::folder(&data_home()));
                while let Ok(task) = rx.recv() {
                    if catch_unwind(AssertUnwindSafe(|| task(&mut net, &qt))).is_err() {
                        say(&qt, "Something went wrong with that download. The details are in the activity log.", "error");
                    }
                    post(&qt, |mut t| {
                        let n = (*t.net_busy() - 1).max(0);
                        t.as_mut().set_net_busy(n);
                        if n == 0 {
                            t.as_mut().set_download_json(QString::from("{\"active\":false}"));
                        }
                    });
                }
            });
            if spawned.is_err() {
                self.as_mut().say("Turbine couldn't start its download lane. Close it and open it again.", "error");
                return;
            }
            self.as_mut().rust_mut().net = Some(tx);
        }
        let n = *self.net_busy() + 1;
        self.as_mut().set_net_busy(n);
        let name = name.to_string();
        let wrapped: NetTask = Box::new(move |net: &mut Net, qt: &Qt| {
            let at = Instant::now();
            crate::log::line("net", &format!("Start: {name}"));
            task(net, qt);
            crate::log::line("net", &format!("Done: {name} after {} ms", at.elapsed().as_millis()));
        });
        let sent = self.rust().net.as_ref().map(|tx| tx.send(wrapped).is_ok()).unwrap_or(false);
        if !sent {
            let n = (*self.net_busy() - 1).max(0);
            self.as_mut().set_net_busy(n);
        }
    }

    fn info(&self) -> GameInfo {
        self.rust().game_info.clone()
    }

    /// A downloaded file arrived: the mod worker adds it.
    pub(crate) fn add_fetched(mut self: Pin<&mut Self>, game: Game, f: Fetched) {
        self.as_mut().edited();
        let name = format!("Add {}", f.label);
        self.send(
            Some(game.id()),
            &name,
            true,
            Box::new(move |w: &mut Worker| {
                add_archive(w, &game, &f.path, f.source, f.replaces, &f.label);
                remove_download(&f.path);
            }),
        );
    }

    /// An update went in: its badge goes.
    pub(crate) fn drop_update(mut self: Pin<&mut Self>, game: &str, mod_id: &str) {
        let now = self.updates_json().to_string();
        let Ok(j) = json::parse(&now) else { return };
        if j.str_at("game") != Some(game) {
            return;
        }
        let keep: Vec<String> = j
            .get("items")
            .map(Json::as_array)
            .unwrap_or(&[])
            .iter()
            .filter(|i| i.str_at("mod") != Some(mod_id))
            .map(|i| {
                format!(
                    "{{\"mod\":{},\"kind\":{},\"id\":{},\"from\":{},\"to\":{},\"target\":{},\"deprecated\":{}}}",
                    quote(i.str_at("mod").unwrap_or("")),
                    quote(i.str_at("kind").unwrap_or("")),
                    quote(i.str_at("id").unwrap_or("")),
                    quote(i.str_at("from").unwrap_or("")),
                    quote(i.str_at("to").unwrap_or("")),
                    quote(i.str_at("target").unwrap_or("")),
                    i.get("deprecated").and_then(Json::as_bool).unwrap_or(false)
                )
            })
            .collect();
        let out = format!("{{\"game\":{},\"items\":[{}],\"note\":\"\"}}", quote(game), keep.join(","));
        self.as_mut().set_updates_json(QString::from(out.as_str()));
    }

    pub fn set_sample_data(self: Pin<&mut Self>, dir: &QString) {
        let dir = PathBuf::from(dir.to_string());
        self.net_send(
            "Use recorded samples",
            Box::new(move |net: &mut Net, qt: &Qt| {
                net.use_samples(&dir);
                net.sample_sign_in();
                let j = net.account_json("ok", "");
                set(qt, move |t| t.set_account_json(QString::from(j.as_str())));
            }),
        );
    }

    // ------------------------------------------------------------------------------------------------ Get Mods

    pub fn browse(self: Pin<&mut Self>, text: &QString, category: &QString, sort: &QString, source: &QString) {
        let info = self.info();
        let (text, category, sort, source) = (text.to_string(), category.to_string(), sort.to_string(), source.to_string());
        let name = format!("Browse {} on {source} \"{text}\"", info.name);
        self.net_send(
            &name,
            Box::new(move |net: &mut Net, qt: &Qt| {
                let r = if source == "nexus" { net.browse_nexus(&info, &text, &sort) } else { net.browse_ts(&info, &text, &category, &sort) };
                let j = match r {
                    Ok(j) => format!("{{\"game\":{},\"query\":{},{}", quote(&info.id), quote(&text), &j[1..]),
                    Err(e) => format!("{{\"game\":{},\"query\":{},\"error\":{},\"items\":[]}}", quote(&info.id), quote(&text), quote(&e)),
                };
                set(qt, move |t| t.set_browse_json(QString::from(j.as_str())));
            }),
        );
    }

    pub fn details(mut self: Pin<&mut Self>, id: &QString, source: &QString) {
        let info = self.info();
        let (id, source) = (id.to_string(), source.to_string());
        self.as_mut().set_details_json(QString::from("{\"loading\":true}"));
        self.net_send(
            &format!("Details of {id}"),
            Box::new(move |net: &mut Net, qt: &Qt| {
                let r = if source == "nexus" {
                    id.parse::<u64>().map_err(|_| "That mod can't be read.".to_string()).and_then(|m| net.details_nexus(&info, m))
                } else {
                    net.details_ts(&info, &id)
                };
                let j = r.unwrap_or_else(|e| err_json(&e));
                set(qt, move |t| t.set_details_json(QString::from(j.as_str())));
            }),
        );
    }

    pub fn install(mut self: Pin<&mut Self>, id: &QString, source: &QString) {
        let Some(game) = self.current_game() else { return };
        let info = self.info();
        let (id, source) = (id.to_string(), source.to_string());
        self.as_mut().say("Getting it ready to download...", "work");
        self.net_send(
            &format!("Install {id} from {source}"),
            Box::new(move |net: &mut Net, qt: &Qt| {
                let mut each = hand_over(qt, &game);
                let mut prog = progress(qt);
                let r = if source == "nexus" {
                    id.parse::<u64>()
                        .map_err(|_| "That mod can't be read.".to_string())
                        .and_then(|m| net.install_nexus(&info, m, 0, None, &mut each, &mut prog))
                } else {
                    net.install_ts(&info, &[id.clone()], false, &mut each, &mut prog)
                };
                if let Err(e) = r {
                    say(qt, e, "error");
                }
            }),
        );
    }

    pub fn install_loader(mut self: Pin<&mut Self>) {
        let Some(game) = self.current_game() else { return };
        let info = self.info();
        self.as_mut().say("Getting the mod loader from its official home...", "work");
        self.net_send(
            &format!("Install the loader for {}", game.name),
            Box::new(move |net: &mut Net, qt: &Qt| {
                let mut prog = progress(qt);
                match net.fetch_loader(&info, &mut prog) {
                    Ok(f) => hand_over(qt, &game)(f),
                    Err(e) => say(qt, e, "error"),
                }
            }),
        );
    }

    pub fn cancel_download(self: Pin<&mut Self>) {
        online::CANCEL.store(true, Ordering::SeqCst);
        crate::log::line("net", "Download cancelled by the player");
    }

    // ------------------------------------------------------------------------------------------------ FOMOD

    fn fomod_job<F>(self: Pin<&mut Self>, name: &str, f: F)
    where
        F: FnOnce(&mut Worker, &Game, turbine_engine::store::Pending) -> Option<turbine_engine::store::Pending> + Send + 'static,
    {
        self.send(
            None,
            name,
            false,
            Box::new(move |w: &mut Worker| {
                let Some((gid, p)) = w.pending.pop_front() else { return };
                let Some(game) = game_of(w, &gid) else { return };
                if let Some(back) = f(w, &game, p) {
                    w.pending.push_front((gid, back));
                }
                post_fomod(w);
            }),
        );
    }

    pub fn fomod_choose(self: Pin<&mut Self>, group: i32, option: i32) {
        let (g, o) = (group.max(0) as usize, option.max(0) as usize);
        self.fomod_job("Installer choice", move |w, _, mut p| {
            if let Err(e) = p.session.toggle(g, o) {
                w.say(e, "warn");
            }
            Some(p)
        });
    }

    pub fn fomod_next(self: Pin<&mut Self>) {
        self.fomod_job("Installer next step", move |w, game, mut p| match p.session.next() {
            Ok(true) => Some(p),
            Ok(false) => {
                let name = p.name.clone();
                let r = w.slot(game).and_then(|s| s.store.finish_install(p));
                match r {
                    Ok(done) => {
                        for n in &done.notes {
                            crate::log::line("engine", &format!("{name}: {n}"));
                        }
                        w.say(format!("Added {}", done.item.name), "ok");
                    }
                    Err(e) => w.say(e, "error"),
                }
                w.show(game, true);
                None
            }
            Err(e) => {
                w.say(e, "warn");
                Some(p)
            }
        });
    }

    pub fn fomod_back(self: Pin<&mut Self>) {
        self.fomod_job("Installer back a step", move |_, _, mut p| {
            p.session.back();
            Some(p)
        });
    }

    pub fn fomod_cancel(self: Pin<&mut Self>) {
        self.fomod_job("Installer cancelled", move |w, game, p| {
            let name = p.name.clone();
            if let Ok(s) = w.slot(game) {
                s.store.cancel_install(p);
            }
            w.say(format!("{name} wasn't added"), "ok");
            None
        });
    }

    // ------------------------------------------------------------------------------------------------ Load order

    fn order_job<F>(self: Pin<&mut Self>, name: &str, f: F)
    where
        F: FnOnce(&mut plugins::LoadOrder) -> Result<(), String> + Send + 'static,
    {
        let Some(game) = self.current_game() else { return };
        self.send(
            Some(game.id()),
            name,
            false,
            Box::new(move |w: &mut Worker| {
                let r = plugins::LoadOrder::read(&game).map_err(|e| format!("The load order couldn't be read ({e}).")).and_then(|mut lo| {
                    f(&mut lo)?;
                    lo.save().map_err(|e| {
                        format!("The load order couldn't be saved ({e}). Start the game once from Steam so it makes its settings folder.")
                    })?;
                    Ok(lo)
                });
                if let Err(e) = &r {
                    w.say(e.clone(), "warn");
                }
                let j = order_json(&game, plugins::LoadOrder::read(&game).map_err(|e| e.to_string()));
                w.post(move |t| t.set_order_json(QString::from(j.as_str())));
            }),
        );
    }

    pub fn load_order(self: Pin<&mut Self>) {
        let Some(game) = self.current_game() else { return };
        self.send(
            Some(game.id()),
            "Read the load order",
            false,
            Box::new(move |w: &mut Worker| {
                let j = order_json(&game, plugins::sync(&game).map_err(|e| format!("The load order couldn't be read ({e}).")));
                w.post(move |t| t.set_order_json(QString::from(j.as_str())));
            }),
        );
    }

    pub fn order_move(self: Pin<&mut Self>, name: &QString, index: i32) {
        let (name, index) = (name.to_string(), index.max(0) as usize);
        crate::log::line("action", &format!("Load order: move {name} to {}", index + 1));
        self.order_job("Move a plugin", move |lo| lo.move_to(&name, index).map(|_| ()));
    }

    pub fn order_toggle(self: Pin<&mut Self>, name: &QString, on: bool) {
        let name = name.to_string();
        crate::log::line("action", &format!("Load order: turn {} {name}", if on { "on" } else { "off" }));
        self.order_job("Turn a plugin on or off", move |lo| lo.set_active(&name, on));
    }

    pub fn order_sort(self: Pin<&mut Self>) {
        crate::log::line("action", "Load order: sort automatically");
        self.order_job("Sort the load order", move |lo| {
            lo.sort();
            Ok(())
        });
    }

    // ------------------------------------------------------------------------------------------------ profiles

    fn profile_job<F>(self: Pin<&mut Self>, name: &str, f: F)
    where
        F: FnOnce(&mut Worker, &Game) + Send + 'static,
    {
        let Some(game) = self.current_game() else { return };
        self.send(
            Some(game.id()),
            name,
            true,
            Box::new(move |w: &mut Worker| {
                f(w, &game);
                post_profiles(w, &game);
            }),
        );
    }

    pub fn profiles(self: Pin<&mut Self>) {
        self.profile_job("Read profiles", |_, _| {});
    }

    pub fn profile_switch(self: Pin<&mut Self>, name: &QString) {
        let name = name.to_string();
        self.profile_job("Switch profile", move |w, game| {
            let r = w.slot(game).and_then(|s| {
                let mut p = Profiles::open(&s.store).map_err(|e| format!("Profiles couldn't be read ({e})."))?;
                p.switch(&mut s.store, &name)
            });
            match r {
                Ok(r) => {
                    w.note(game, &r);
                    w.say(format!("Using profile {name}"), "ok");
                }
                Err(e) => w.say(e, "error"),
            }
            w.show(game, false);
        });
    }

    pub fn profile_create(self: Pin<&mut Self>, name: &QString) {
        let name = name.to_string();
        self.profile_job("New profile", move |w, game| {
            let r = w.slot(game).and_then(|s| {
                let mut p = Profiles::open(&s.store).map_err(|e| format!("Profiles couldn't be read ({e})."))?;
                p.create(&s.store, &name)
            });
            match r {
                Ok(()) => w.say(format!("Made profile {name} from the mods as they are now"), "ok"),
                Err(e) => w.say(e, "error"),
            }
        });
    }

    pub fn profile_export(self: Pin<&mut Self>, name: &QString) {
        let name = name.to_string();
        self.profile_job("Export profile", move |w, game| {
            let home = turbine_engine::home_dir();
            let r = w.slot(game).and_then(|s| {
                let p = Profiles::open(&s.store).map_err(|e| format!("Profiles couldn't be read ({e})."))?;
                profiles::export(&p, &s.store, &name, &home)
            });
            match r {
                Ok(path) => w.say(
                    format!("Saved to Downloads as {}", path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()),
                    "ok",
                ),
                Err(e) => w.say(e, "error"),
            }
        });
    }

    pub fn list_shares(mut self: Pin<&mut Self>) {
        let dir = turbine_engine::home_dir().join("Downloads");
        let mut found: Vec<(std::time::SystemTime, String, String)> = std::fs::read_dir(&dir)
            .map(|rd| {
                rd.flatten()
                    .filter_map(|e| {
                        let n = e.file_name().to_string_lossy().into_owned();
                        let ok = n.to_ascii_lowercase().ends_with(".json") && n.starts_with("Turbine profile");
                        let m = e.metadata().ok()?;
                        ok.then(|| (m.modified().unwrap_or(std::time::UNIX_EPOCH), n, e.path().to_string_lossy().into_owned()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        found.sort_by(|a, b| b.0.cmp(&a.0));
        let items: Vec<String> = found.iter().map(|(_, n, p)| format!("{{\"name\":{},\"path\":{}}}", quote(n), quote(p))).collect();
        self.as_mut().set_shares_json(QString::from(format!("[{}]", items.join(",")).as_str()));
    }

    pub fn profile_import_file(self: Pin<&mut Self>, path: &QString) {
        let path = PathBuf::from(path.to_string());
        self.profile_job("Read a share file", move |w, game| match profiles::read_share(&path) {
            Ok(share) => plan_share(w, game, share),
            Err(e) => {
                let j = format!("{{\"game\":{},\"profile\":\"\",\"plan\":null,\"error\":{}}}", quote(&game.id()), quote(&e));
                w.post(move |t| t.set_import_json(QString::from(j.as_str())));
            }
        });
    }

    pub fn profile_import_code(mut self: Pin<&mut Self>, code: &QString) {
        let Some(game) = self.current_game() else { return };
        let info = self.info();
        let code = code.to_string();
        self.as_mut().set_import_json(QString::from("{\"loading\":true}"));
        self.net_send(
            "Read a Thunderstore profile code",
            Box::new(move |net: &mut Net, qt: &Qt| match net.read_code(&info, &code) {
                Ok(share) => post(qt, move |t| {
                    t.send(Some(game.id()), "Plan a profile import", false, Box::new(move |w: &mut Worker| plan_share(w, &game, share)))
                }),
                Err(e) => {
                    let j = format!("{{\"game\":{},\"profile\":\"\",\"plan\":null,\"error\":{}}}", quote(&info.id), quote(&e));
                    set(qt, move |t| t.set_import_json(QString::from(j.as_str())));
                }
            }),
        );
    }

    pub fn profile_import_go(self: Pin<&mut Self>) {
        let Some(game) = self.current_game() else { return };
        self.send(
            Some(game.id()),
            "Get a profile's mods",
            false,
            Box::new(move |w: &mut Worker| {
                let Some((_, share)) = w.share.clone() else { return };
                let plan = match w.slot(&game).and_then(|s| profiles::plan_import(&share, &s.store)) {
                    Ok(p) => p,
                    Err(e) => return w.say(e, "error"),
                };
                let wanted: Vec<String> = plan
                    .automatic
                    .iter()
                    .filter(|m| m.kind == "thunderstore")
                    .map(|m| m.version.as_ref().map(|v| format!("{}-{v}", m.id)).unwrap_or_else(|| m.id.clone()))
                    .collect();
                if wanted.is_empty() {
                    finish_share(w, &game);
                    return;
                }
                let g = game.clone();
                w.post(move |t| {
                    let info = t.info();
                    t.net_send(
                        "Download a profile's mods",
                        Box::new(move |net: &mut Net, qt: &Qt| {
                            let mut each = hand_over(qt, &g);
                            let mut prog = progress(qt);
                            if let Err(e) = net.install_ts(&info, &wanted, false, &mut each, &mut prog) {
                                say(qt, e, "error");
                            }
                            // Queued after every download's import, so the profile sees all of them.
                            let g2 = g.clone();
                            post(qt, move |t| {
                                t.send(Some(g2.id()), "Make the shared profile", true, Box::new(move |w: &mut Worker| finish_share(w, &g2)))
                            });
                        }),
                    );
                });
            }),
        );
    }

    // ------------------------------------------------------------------------------------------------ updates

    pub fn check_updates(self: Pin<&mut Self>) {
        let info = self.info();
        if info.id.is_empty() || !info.have.iter().any(|h| h.source.kind == "thunderstore" || h.source.kind == "nexus") {
            return;
        }
        self.net_send(
            &format!("Check updates for {}", info.name),
            Box::new(move |net: &mut Net, qt: &Qt| {
                let j = net.check_updates(&info).unwrap_or_else(|e| format!("{{\"game\":{},\"items\":[],\"note\":{}}}", quote(&info.id), quote(&e)));
                set(qt, move |t| t.set_updates_json(QString::from(j.as_str())));
            }),
        );
    }

    fn update_entries(&self, only: Option<&str>) -> Vec<(String, String, String)> {
        let text = self.updates_json().to_string();
        let Ok(j) = json::parse(&text) else { return Vec::new() };
        if j.str_at("game") != Some(self.rust().current.as_str()) {
            return Vec::new();
        }
        j.get("items")
            .map(Json::as_array)
            .unwrap_or(&[])
            .iter()
            .filter(|i| only.is_none() || i.str_at("mod") == only)
            .map(|i| {
                (
                    i.str_at("mod").unwrap_or("").to_string(),
                    i.str_at("kind").unwrap_or("").to_string(),
                    i.str_at("target").unwrap_or("").to_string(),
                )
            })
            .collect()
    }

    fn run_updates(mut self: Pin<&mut Self>, list: Vec<(String, String, String)>) {
        let Some(game) = self.current_game() else { return };
        if list.is_empty() {
            return;
        }
        let info = self.info();
        self.as_mut().say(&format!("Updating {}...", if list.len() == 1 { "1 mod".to_string() } else { format!("{} mods", list.len()) }), "work");
        self.net_send(
            "Download updates",
            Box::new(move |net: &mut Net, qt: &Qt| {
                for (mod_id, kind, target) in list {
                    let mut each = hand_over(qt, &game);
                    let mut prog = progress(qt);
                    if let Err(e) = net.fetch_update(&info, &mod_id, &kind, &target, &mut each, &mut prog) {
                        say(qt, e, "error");
                    }
                }
            }),
        );
    }

    pub fn update_mod(self: Pin<&mut Self>, mod_id: &QString) {
        let list = self.update_entries(Some(&mod_id.to_string()));
        self.run_updates(list);
    }

    pub fn update_all(self: Pin<&mut Self>) {
        let list = self.update_entries(None);
        self.run_updates(list);
    }

    // ------------------------------------------------------------------------------------------------ Nexus account

    pub fn set_nexus_key(mut self: Pin<&mut Self>, key: &QString) {
        let key = key.to_string();
        crate::log::secret(&key);
        crate::log::line("action", "Nexus key entered");
        self.as_mut().set_account_json(QString::from("{\"state\":\"checking\"}"));
        self.net_send(
            "Check the Nexus key",
            Box::new(move |net: &mut Net, qt: &Qt| {
                let j = net.set_key(&key).unwrap_or_else(|e| net.account_json("error", &e));
                set(qt, move |t| t.set_account_json(QString::from(j.as_str())));
            }),
        );
    }

    pub fn nexus_sign_out(self: Pin<&mut Self>) {
        crate::log::line("action", "Nexus sign out");
        self.net_send(
            "Sign out of Nexus",
            Box::new(move |net: &mut Net, qt: &Qt| {
                let j = net.sign_out();
                set(qt, move |t| t.set_account_json(QString::from(j.as_str())));
            }),
        );
    }

    pub fn account_status(self: Pin<&mut Self>) {
        self.net_send(
            "Nexus account status",
            Box::new(move |net: &mut Net, qt: &Qt| {
                let j = if net.has_key() {
                    match net.account() {
                        Ok(_) => net.account_json("ok", ""),
                        Err(e) => net.account_json("error", &e),
                    }
                } else {
                    net.account_json("none", "")
                };
                set(qt, move |t| t.set_account_json(QString::from(j.as_str())));
            }),
        );
    }

    // ------------------------------------------------------------------------------------------------ Turbine itself

    pub fn self_check(mut self: Pin<&mut Self>, labs: bool, back: bool) {
        self.as_mut().set_self_json(QString::from("{\"state\":\"checking\"}"));
        self.net_send(
            if back { "Look for the latest stable Turbine" } else { "Look for a newer Turbine" },
            Box::new(move |net: &mut Net, qt: &Qt| {
                let j = match net.self_check(crate::VERSION, labs, back) {
                    Ok(Some(r)) => {
                        let j = online::release_json(if back { "stable" } else { "available" }, Some(&r), "");
                        net.release = Some(r);
                        j
                    }
                    Ok(None) => {
                        net.release = None;
                        online::release_json("current", None, "")
                    }
                    Err(e) => online::release_json("error", None, &e),
                };
                set(qt, move |t| t.set_self_json(QString::from(j.as_str())));
            }),
        );
    }

    pub fn self_install(self: Pin<&mut Self>) {
        self.net_send(
            "Install a Turbine update",
            Box::new(move |net: &mut Net, qt: &Qt| {
                let Some(r) = net.release.clone() else {
                    let j = online::release_json("error", None, "Check for updates first.");
                    return set(qt, move |t| t.set_self_json(QString::from(j.as_str())));
                };
                let busy = online::release_json("installing", Some(&r), "");
                set(qt, move |t| t.set_self_json(QString::from(busy.as_str())));
                let mut prog = progress(qt);
                let j = match net.self_install(&r, &mut prog) {
                    Ok(()) => online::release_json("ready", Some(&r), ""),
                    Err(e) => online::release_json("error", Some(&r), &e),
                };
                set(qt, move |t| t.set_self_json(QString::from(j.as_str())));
            }),
        );
    }

    pub fn restart(self: Pin<&mut Self>) {
        crate::log::line("update", "Restarting into the new version");
        let mut cmd = if Path::new("/.flatpak-info").exists() {
            let mut c = std::process::Command::new("flatpak-spawn");
            c.args(["--host", "flatpak", "run", APP_ID]);
            c
        } else {
            std::process::Command::new(std::env::current_exe().unwrap_or_else(|_| "turbine-mod-manager".into()))
        };
        if let Err(e) = cmd.stdin(std::process::Stdio::null()).spawn() {
            crate::log::line("update", &format!("Restart failed: {e}"));
        }
    }
}
