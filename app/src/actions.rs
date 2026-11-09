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

pub(crate) type Qt = CxxQtThread<qobject::Turbine>;
pub(crate) type NetTask = Box<dyn FnOnce(&mut Net, &Qt) + Send>;

const APP_ID: &str = "io.github.ravenor907.TurbineModManager";
/// Said when a Mod Manager Download link arrives and there is no Nexus key.
pub const NO_KEY_NXM: &str = "Mod Manager Download needs your Nexus API key. Use Slow download (or Manual download) instead, Turbine picks it up from Downloads.";

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
        domain: turbine_engine::nexus::domain_for(g.store, &g.store_id).map(str::to_string),
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

/// For anticheat.rs: the same helpers.
pub(crate) fn progress_for(qt: &Qt) -> impl FnMut(&str, u64, Option<u64>) + '_ {
    progress(qt)
}
pub(crate) fn say_for(qt: &Qt, text: impl Into<String>, level: &'static str) {
    say(qt, text, level)
}
pub(crate) fn post_for<F>(qt: &Qt, f: F)
where
    F: FnOnce(Pin<&mut qobject::Turbine>) + Send + 'static,
{
    post(qt, f)
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
fn hand_over<'a>(qt: &'a Qt, game: &Game) -> impl FnMut(Fetched) + 'a {
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

/// What adding an archive came to (catch.rs tells the overlay and the browser helper).
pub(crate) enum Outcome {
    Added { name: String, notes: Vec<String> },
    Choices { name: String },
    Failed(String),
}

/// Install notes the player may want to act on (settings lines to add by hand, menus, REDmod): shown after adding.
pub(crate) fn show_notes(w: &Worker, name: &str, notes: &[String]) {
    let notes: Vec<String> = notes.iter().filter(|n| !n.trim().is_empty()).cloned().collect();
    if notes.is_empty() {
        return;
    }
    let j = format!("[{}]", notes.iter().map(|n| quote(n)).collect::<Vec<_>>().join(","));
    let name = name.to_string();
    w.post(move |t| t.added(QString::from(name.as_str()), QString::from(j.as_str())));
}

/// Adds (or, with `replaces`, updates) a mod from an archive. An installer with choices waits for the player.
pub(crate) fn add_archive(w: &mut Worker, game: &Game, path: &Path, source: SourceInfo, replaces: Option<String>, label: &str) -> Outcome {
    let result = w.slot(game).and_then(|s| match &replaces {
        Some(id) => s.store.replace_mod(id, path, source),
        None => s.store.import_archive(path, source),
    });
    let out = match result {
        Ok(Import::Added(done)) => {
            for n in &done.notes {
                crate::log::line("engine", &format!("{}: {n}", done.item.name));
            }
            let verb = if replaces.is_some() { "Updated" } else { "Added" };
            match &done.placed {
                // A Basic game's mod: where it went.
                Some(p) => {
                    let words = crate::anygame::note_placed(&done.item.name, p);
                    w.say(if replaces.is_some() { words.replacen("Added", "Updated", 1) } else { words }, "ok");
                }
                None => w.say(format!("{verb} {}", done.item.name), "ok"),
            }
            show_notes(w, &done.item.name, &done.notes);
            if let Some(id) = replaces {
                let gid = game.id();
                w.post(move |t| t.drop_update(&gid, &id));
            } else {
                let (gid, id, n, on) = (game.id(), done.item.id.clone(), done.item.name.clone(), done.item.enabled);
                w.post(move |t| t.arrived(QString::from(gid.as_str()), QString::from(id.as_str()), QString::from(n.as_str()), on));
            }
            Outcome::Added { name: done.item.name.clone(), notes: done.notes.clone() }
        }
        Ok(Import::NeedsChoices(p)) => {
            let name = p.name.clone();
            w.say(format!("{} has installer choices", p.name), "work");
            w.next_session += 1;
            let sid = w.next_session;
            w.pending.push_back((game.id(), *p, sid));
            if w.pending.len() == 1 {
                post_fomod(w);
            }
            Outcome::Choices { name }
        }
        Ok(Import::NeedsPlace(p)) => {
            let name = p.name.clone();
            w.say(if p.sure { format!("{} has files {} already has", p.name, game.name) } else { format!("Where do {}'s files go?", p.name) }, "work");
            crate::anygame::queue_place(w, game, *p);
            Outcome::Choices { name }
        }
        Err(e) => {
            let e = if e.contains(label) { e } else { format!("Couldn't add {label}: {e}") };
            w.say(e.clone(), "error");
            Outcome::Failed(e)
        }
    };
    w.show(game, true);
    out
}

/// A picture for the browser helper's choices panel, as a data: URL (small ones only).
fn data_url(p: &turbine_engine::store::Pending, archive_path: &str) -> Option<String> {
    let f = p.image_file(archive_path)?;
    let m = std::fs::metadata(&f).ok()?;
    if m.len() > 400 * 1024 {
        return None;
    }
    let lc = archive_path.to_ascii_lowercase();
    let mime = if lc.ends_with(".png") {
        "png"
    } else if lc.ends_with(".jpg") || lc.ends_with(".jpeg") {
        "jpeg"
    } else if lc.ends_with(".gif") {
        "gif"
    } else if lc.ends_with(".webp") {
        "webp"
    } else if lc.ends_with(".bmp") {
        "bmp"
    } else {
        return None;
    };
    let bytes = std::fs::read(&f).ok()?;
    Some(format!("data:image/{mime};base64,{}", crate::bridge::base64(&bytes)))
}

/// The installer's current step for the browser helper ("choices"); option ids are "g<group>o<option>".
fn choices_message(p: &turbine_engine::store::Pending, sid: u64) -> Option<String> {
    let v = p.session.current()?;
    let groups: Vec<String> = v
        .groups
        .iter()
        .enumerate()
        .map(|(gi, g)| {
            let kind = match g.kind {
                turbine_engine::fomod::GroupKind::ExactlyOne => "one",
                turbine_engine::fomod::GroupKind::AtMostOne => "atmostone",
                turbine_engine::fomod::GroupKind::AtLeastOne => "atleastone",
                turbine_engine::fomod::GroupKind::All => "all",
                turbine_engine::fomod::GroupKind::Any => "any",
            };
            let options: Vec<String> = g
                .options
                .iter()
                .enumerate()
                .map(|(oi, o)| {
                    let state = if o.kind == turbine_engine::fomod::PluginType::NotUsable {
                        "disabled"
                    } else if !o.can_change && o.selected {
                        "required"
                    } else if !o.can_change {
                        "disabled"
                    } else if o.selected {
                        "selected"
                    } else {
                        "available"
                    };
                    let image = o.image.as_deref().and_then(|i| p.session.archive_path(i)).and_then(|a| data_url(p, &a));
                    format!(
                        "{{\"id\":{},\"name\":{},\"description\":{},\"image\":{},\"state\":{}}}",
                        quote(&crate::catch::option_id(gi, oi)),
                        quote(&o.name),
                        quote(&o.description),
                        image.as_deref().map(quote).unwrap_or_else(|| "null".into()),
                        quote(state)
                    )
                })
                .collect();
            format!("{{\"title\":{},\"kind\":{},\"options\":[{}]}}", quote(&g.name), quote(kind), options.join(","))
        })
        .collect();
    Some(format!(
        "{{\"type\":\"choices\",\"sessionId\":{},\"modName\":{},\"stepTitle\":{},\"stepIndex\":{},\"stepCount\":{},\"groups\":[{}]}}",
        quote(&sid.to_string()),
        quote(&p.name),
        quote(&v.name),
        v.number.saturating_sub(1),
        if v.is_last { v.number } else { v.number + 1 },
        groups.join(",")
    ))
}

/// Tells the browser helper about the installer on screen (or that the last one is done).
fn send_choices(w: &mut Worker) {
    let front = w.pending.front().map(|(_, p, sid)| (*sid, choices_message(p, *sid)));
    if let Some(prev) = w.fomod_sent {
        if front.as_ref().map(|(s, _)| *s) != Some(prev) {
            crate::bridge::broadcast(&format!("{{\"type\":\"choices.done\",\"sessionId\":{}}}", quote(&prev.to_string())));
            w.fomod_sent = None;
        }
    }
    if let Some((sid, Some(m))) = front {
        crate::bridge::broadcast(&m);
        w.fomod_sent = Some(sid);
    }
}

/// The helper just connected: the installer waiting now, if any.
pub(crate) fn resend_fomod(w: &mut Worker) {
    w.fomod_sent = None;
    send_choices(w);
}

/// Puts the first waiting installer on screen (or takes the installer screen away when none is left).
fn post_fomod(w: &mut Worker) {
    send_choices(w);
    let json = match w.pending.front() {
        None => String::new(),
        Some((gid, p, _)) => {
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

/// Next on the installer: the next step, or the install when this was the last one. Returns the installer when it
/// is still running.
fn installer_next(w: &mut Worker, game: &Game, mut p: turbine_engine::store::Pending) -> Option<turbine_engine::store::Pending> {
    match p.session.next() {
        Ok(true) => Some(p),
        Ok(false) => {
            let name = p.name.clone();
            let r = w.slot(game).and_then(|s| s.store.finish_install_import(p));
            match r {
                // A Basic game: the installer's files go through "Where do these files go?" like any other mod.
                Ok(Import::NeedsPlace(pp)) => {
                    w.say(if pp.sure { format!("{} has files {} already has", pp.name, game.name) } else { format!("Where do {}'s files go?", pp.name) }, "work");
                    crate::anygame::queue_place(w, game, *pp);
                }
                Ok(Import::NeedsChoices(_)) => w.say(format!("{name} couldn't be added: its installer stopped early."), "error"),
                Ok(Import::Added(done)) => {
                    for n in &done.notes {
                        crate::log::line("engine", &format!("{name}: {n}"));
                    }
                    w.say(format!("Added {}", done.item.name), "ok");
                    show_notes(w, &done.item.name, &done.notes);
                    let (g, m, notes) = (game.name.clone(), done.item.name.clone(), done.notes.clone());
                    w.post(move |t| t.caught_added(&g, &m, notes, true));
                    let (gid, id, n, on) = (game.id(), done.item.id.clone(), done.item.name.clone(), done.item.enabled);
                    w.post(move |t| t.arrived(QString::from(gid.as_str()), QString::from(id.as_str()), QString::from(n.as_str()), on));
                }
                Err(e) => {
                    let g = game.name.clone();
                    let e2 = e.clone();
                    w.post(move |mut t| {
                        if t.rust().catching.busy {
                            t.as_mut().set_status(&e2, "stop", None, &g, "", true, "");
                        }
                    });
                    w.say(e, "error")
                }
            }
            w.show(game, true);
            None
        }
        Err(e) => {
            w.say(e.clone(), "warn");
            crate::bridge::broadcast(&format!("{{\"type\":\"error\",\"text\":{}}}", quote(&e)));
            Some(p)
        }
    }
}

/// Installer choices from the browser helper's panel. They count only for the installer and step on screen: when
/// the player already answered in Turbine, the helper just gets the current step again.
pub(crate) fn bridge_choice(w: &mut Worker, what: &str, session: &str, step: usize, selected: &[String]) {
    let Some((gid, mut p, sid)) = w.pending.pop_front() else {
        return send_choices(w);
    };
    let current = p.session.current();
    let stale = sid.to_string() != session || (what == "choice" && current.as_ref().map(|v| v.number.saturating_sub(1)) != Some(step));
    let Some(game) = game_of(w, &gid).filter(|_| !stale) else {
        w.pending.push_front((gid, p, sid));
        w.fomod_sent = None;
        return send_choices(w);
    };
    let back = match what {
        "choice.back" => {
            p.session.back();
            Some(p)
        }
        "choice.cancel" => {
            let name = p.name.clone();
            if let Ok(s) = w.slot(&game) {
                s.store.cancel_install(p);
            }
            w.say(format!("{name} wasn't added"), "ok");
            let g = game.name.clone();
            w.post(move |t| t.set_status(&format!("{name} wasn't added"), "warn", None, &g, "", true, ""));
            None
        }
        _ => {
            let view = current.expect("checked above");
            let mut err = None;
            for (gi, g) in view.groups.iter().enumerate() {
                for (oi, o) in g.options.iter().enumerate() {
                    if selected.iter().any(|x| *x == crate::catch::option_id(gi, oi)) && !o.selected {
                        if let Err(e) = p.session.select(gi, oi) {
                            err = Some(e);
                        }
                    }
                }
            }
            for (gi, g) in view.groups.iter().enumerate() {
                for (oi, o) in g.options.iter().enumerate() {
                    if !selected.iter().any(|x| *x == crate::catch::option_id(gi, oi)) && o.selected && o.can_change {
                        let _ = p.session.deselect(gi, oi);
                    }
                }
            }
            match err {
                Some(e) => {
                    crate::bridge::broadcast(&format!("{{\"type\":\"error\",\"text\":{}}}", quote(&e)));
                    Some(p)
                }
                None => installer_next(w, &game, p),
            }
        }
    };
    if let Some(p) = back {
        w.pending.push_front((gid, p, sid));
        // The same session again: the helper gets the new step.
        w.fomod_sent = None;
    }
    post_fomod(w);
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

/// The Load order tab's data: the plugins, LOOT's messages for them, and which sort ran last (with its note).
fn order_json(w: &Worker, game: &Game, lo: Result<plugins::LoadOrder, String>, sorted_by: &str, note: &str) -> String {
    let msgs = w.loot.get(&game.id()).map(|(_, m)| crate::loot::messages_json(m)).unwrap_or_else(|| "{}".into());
    // Which of Turbine's mods placed each plugin (by file name, any case), for the details sheet's "Go to its mod".
    let owners: Vec<String> = w
        .slots
        .get(&game.id())
        .map(|s| {
            let mut seen = std::collections::HashSet::new();
            s.store
                .mod_files()
                .into_iter()
                .filter(|m| s.store.mods().iter().any(|x| x.id == m.id))
                .flat_map(|m| {
                    let id = m.id.clone();
                    m.files.into_iter().filter_map(move |f| {
                        let base = f.rsplit(['/', '\\']).next().unwrap_or("").to_ascii_lowercase();
                        let plugin = base.ends_with(".esp") || base.ends_with(".esm") || base.ends_with(".esl");
                        plugin.then(|| (base, id.clone()))
                    })
                })
                .filter(|(b, _)| seen.insert(b.clone()))
                .map(|(b, id)| format!("{}:{}", quote(&b), quote(&id)))
                .collect()
        })
        .unwrap_or_default();
    match lo {
        Ok(lo) => format!(
            "{{\"game\":{},\"plugins\":{},\"owners\":{{{}}},\"messages\":{},\"sortedBy\":{},\"note\":{},\"error\":\"\"}}",
            quote(&game.id()),
            lo.to_json(),
            owners.join(","),
            msgs,
            quote(sorted_by),
            quote(note)
        ),
        Err(e) => format!(
            "{{\"game\":{},\"plugins\":[],\"messages\":{{}},\"sortedBy\":\"\",\"note\":\"\",\"error\":{}}}",
            quote(&game.id()),
            quote(&e)
        ),
    }
}

fn post_order(w: &Worker, game: &Game, lo: Result<plugins::LoadOrder, String>, sorted_by: &str, note: &str) {
    let j = order_json(w, game, lo, sorted_by, note);
    w.post(move |t| t.set_order_json(QString::from(j.as_str())));
}

/// What the plugins look like (names, sizes, times, on or off) and which masterlist: LOOT's messages are read again
/// only when this changes.
fn loot_key(lo: &plugins::LoadOrder, lists: &crate::loot::Lists) -> String {
    let stamp = |p: &Path| {
        std::fs::metadata(p)
            .ok()
            .map(|m| format!("{}:{:?}", m.len(), m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs())))
            .unwrap_or_default()
    };
    let mut key = format!("{}|{}", stamp(&lists.masterlist), stamp(&lists.prelude));
    for p in lo.plugins() {
        key.push_str(&format!("|{}{}{}", if p.active { "*" } else { "" }, p.name, stamp(&lo.files().data.join(&p.name))));
    }
    key
}

/// Reads LOOT's messages for the game's plugins when they changed since the last time, then shows the load order.
fn loot_messages(w: &mut Worker, game: &Game, lists: Option<crate::loot::Lists>) {
    let lo = plugins::LoadOrder::read(game);
    if let (Ok(lo), Some(l)) = (&lo, &lists) {
        let key = loot_key(lo, l);
        if w.loot.get(&game.id()).map(|(k, _)| k != &key).unwrap_or(true) {
            let at = Instant::now();
            match crate::loot::messages(game, lo, l) {
                Ok(m) => {
                    crate::log::line("loot", &format!("Messages for {} plugins in {} ms", m.len(), at.elapsed().as_millis()));
                    w.loot.insert(game.id(), (key, m));
                }
                Err(e) => crate::log::line("loot", &format!("No messages: {e}")),
            }
        }
    }
    post_order(w, game, lo.map_err(|e| format!("The load order couldn't be read ({e}).")), "", "");
}

/// Sorts with LOOT when its masterlist is here, otherwise with the engine's own sort (masters first, each plugin
/// after its masters), and says which one it used.
fn sort_order(w: &mut Worker, game: &Game, lists: Option<crate::loot::Lists>, why: String) {
    let mut lo = match plugins::LoadOrder::read(game) {
        Ok(lo) => lo,
        Err(e) => {
            w.say(format!("The load order couldn't be read ({e})."), "warn");
            return post_order(w, game, Err(e.to_string()), "", "");
        }
    };
    let mut note = why;
    let mut by = "masters";
    let mut general = 0;
    if let Some(l) = &lists {
        w.say("Sorting with LOOT...", "work");
        match crate::loot::sort(game, &lo, l) {
            Ok(s) => {
                crate::loot::apply(&mut lo, &s);
                for g in &s.general {
                    crate::log::line("loot", &format!("General message [{}]: {}", g.level, g.text));
                }
                general = s.general.iter().filter(|g| g.level != "say").count();
                let key = loot_key(&lo, l);
                w.loot.insert(game.id(), (key, s.messages));
                by = "loot";
            }
            Err(e) => {
                crate::log::line("loot", &format!("LOOT sort failed: {e}"));
                note = format!("LOOT couldn't sort: {e}.");
            }
        }
    }
    if by != "loot" {
        lo.sort();
    }
    let saved = lo.save();
    let words = match (&saved, by) {
        (Err(e), _) => {
            w.say(format!("The load order couldn't be saved ({e}). Start the game once from Steam so it makes its settings folder."), "warn");
            String::new()
        }
        (Ok(()), "loot") => {
            let n = w.loot.get(&game.id()).map(|(_, m)| m.len()).unwrap_or(0);
            let mut s = String::from("Sorted with LOOT's masterlist");
            if n > 0 {
                s.push_str(&format!(". LOOT has notes on {}", if n == 1 { "1 plugin".to_string() } else { format!("{n} plugins") }));
            }
            if general > 0 {
                s.push_str(&format!(" and {general} general {}; they're in the activity log", if general == 1 { "warning" } else { "warnings" }));
            }
            w.say(s.clone(), "ok");
            s
        }
        (Ok(()), _) => {
            let s = if note.is_empty() {
                "Sorted by masters only (LOOT's masterlist isn't here)".to_string()
            } else {
                format!("Sorted by masters only. {note}")
            };
            w.say(s.clone(), if note.is_empty() { "ok" } else { "warn" });
            s
        }
    };
    let now = plugins::LoadOrder::read(game).map_err(|e| e.to_string());
    post_order(w, game, now, by, &words);
}

fn profiles_json(w: &mut Worker, game: &Game) -> String {
    match w.slot(game).and_then(|s| Profiles::open(&s.store).map_err(|e| format!("Loadouts couldn't be read ({e})."))) {
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
            // Nexus entries: what Premium fetches, and the pages a free account opens.
            let domain = turbine_engine::nexus::domain_for(game.store, &game.store_id);
            let j = format!(
                "{{\"game\":{},\"profile\":{},\"plan\":{},\"nexusPremium\":{},\"nexusFree\":{},\"error\":\"\"}}",
                quote(&game.id()),
                quote(&share.profile),
                profiles::plan_json(&plan),
                profiles::nexus_steps_json(&profiles::nexus_steps(&plan, domain, true)),
                profiles::nexus_steps_json(&profiles::nexus_steps(&plan, domain, false))
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
        let mut p = Profiles::open(&s.store).map_err(|e| format!("Loadouts couldn't be read ({e})."))?;
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
        p.save_current(&s.store).map_err(|e| format!("The current loadout couldn't be saved ({e})."))?;
        s.store.set_order(&order).map_err(|e| format!("The loadout couldn't be made ({e})."))?;
        let made = p.create(&s.store, &name);
        s.store.set_order(&old).map_err(|e| format!("The loadout couldn't be made ({e})."))?;
        made?;
        let r = p.switch(&mut s.store, &name)?;
        Ok((name, r))
    });
    match result {
        Ok((name, r)) => {
            w.note(game, &r);
            w.say(format!("Loadout {name} is ready and in use"), "ok");
        }
        Err(e) => w.say(e, "error"),
    }
    let empty = String::new();
    w.post(move |t| t.set_import_json(QString::from(empty.as_str())));
    post_profiles(w, game);
    w.show(game, false);
}

/// Profiles > Create a Thunderstore code: {"state": "working" | "ready" | "error", "profile", "code", "left" (mods a
/// code can't hold), "files" (settings files included), "message"}.
fn code_json(state: &str, profile: &str, code: &str, left: usize, files: usize, message: &str) -> String {
    format!(
        "{{\"state\":{},\"profile\":{},\"code\":{},\"left\":{},\"files\":{},\"message\":{}}}",
        quote(state),
        quote(profile),
        quote(code),
        left,
        files,
        quote(message)
    )
}

/// The game's BepInEx settings files (BepInEx/config/*.cfg), which a Thunderstore code carries along. Only plain
/// files, each under 1 MB, 8 MB in all; they're read, never changed.
fn config_files(game: &Game) -> Vec<(String, Vec<u8>)> {
    let Some(dir) = turbine_engine::rules::find_ci(&game.install_dir, "BepInEx/config") else { return Vec::new() };
    let mut names: Vec<(String, PathBuf)> = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.flatten()
                .filter_map(|e| {
                    let n = e.file_name().to_string_lossy().into_owned();
                    let m = std::fs::symlink_metadata(e.path()).ok()?;
                    (m.is_file() && m.len() <= 1 << 20 && n.to_ascii_lowercase().ends_with(".cfg")).then(|| (n, e.path()))
                })
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    let mut total = 0usize;
    let mut out = Vec::new();
    for (n, p) in names {
        let Ok(bytes) = std::fs::read(&p) else { continue };
        total += bytes.len();
        if total > 8 << 20 {
            break;
        }
        out.push((format!("BepInEx/config/{n}"), bytes));
    }
    out
}

/// One Nexus file (from Other versions) in place of the mod's installed file.
fn install_nexus_version(
    net: &mut Net,
    info: &GameInfo,
    id: &str,
    file: &str,
    each: &mut dyn FnMut(Fetched),
    prog: &mut dyn FnMut(&str, u64, Option<u64>),
) -> Result<usize, String> {
    let mod_id: u64 = id.parse().map_err(|_| "That mod can't be read.".to_string())?;
    let file_id: u64 = file.parse().map_err(|_| "That file can't be read.".to_string())?;
    let replaces = info
        .have
        .iter()
        .find(|h| h.source.kind == "nexus" && turbine_engine::nexus::parse_source_id(&h.source.id).is_some_and(|(_, m, _)| m == mod_id))
        .map(|h| h.mod_id.clone());
    net.install_nexus(info, mod_id, file_id, replaces, each, prog)
}

// ---------------------------------------------------------------------------------------------------- the object

impl qobject::Turbine {
    /// Queues a job for the download lane (started the first time).
    pub(crate) fn net_send(mut self: Pin<&mut Self>, name: &str, task: NetTask) {
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

    pub(crate) fn info(&self) -> GameInfo {
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
                // "Whichever mod came from this Nexus mod": a newer file from the site takes the old one's place.
                let replaces = match f.replaces.as_deref().and_then(|r| r.strip_prefix(online::NEXUS_MOD)) {
                    Some(key) => {
                        let (d, m) = key.split_once(':').unwrap_or(("", key));
                        let m: u64 = m.parse().unwrap_or(0);
                        w.slot(&game).ok().and_then(|s| {
                            s.store.installed_sources().into_iter().find_map(|(id, src)| {
                                turbine_engine::nexus::source_mod(&src).filter(|(sd, sm)| *sm == m && (sd.is_empty() || sd == d)).map(|_| id)
                            })
                        })
                    }
                    None => f.replaces,
                };
                add_archive(w, &game, &f.path, f.source, replaces, &f.label);
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
                net.sample_sign_in(false);
                let j = net.account_json("ok", "");
                set(qt, move |t| t.set_account_json(QString::from(j.as_str())));
            }),
        );
    }

    // ------------------------------------------------------------------------------------------------ Get Mods

    pub fn browse(self: Pin<&mut Self>, text: &QString, category: &QString, sort: &QString, source: &QString) {
        self.browse_page(text, category, sort, source, 0);
    }

    /// One page of Get mods from `offset` (the next 30 when the focus reaches the last row). A Nexus failure says
    /// "fallback" so the app shows Open on Nexus instead, and the activity log says why.
    pub fn browse_page(self: Pin<&mut Self>, text: &QString, category: &QString, sort: &QString, source: &QString, offset: i32) {
        let info = self.info();
        let (text, category, sort, source) = (text.to_string(), category.to_string(), sort.to_string(), source.to_string());
        let offset = offset.max(0) as u32;
        let name = format!("Browse {} on {source} \"{text}\" from {offset}", info.name);
        self.net_send(
            &name,
            Box::new(move |net: &mut Net, qt: &Qt| {
                let r = if source == "nexus" { net.browse_nexus(&info, &text, &category, &sort, offset) } else { net.browse_ts(&info, &text, &category, &sort) };
                let j = match r {
                    Ok(j) => format!("{{\"game\":{},\"query\":{},\"append\":{},{}", quote(&info.id), quote(&text), offset > 0, &j[1..]),
                    Err(e) if source == "nexus" => {
                        crate::log::line("net", &format!("Nexus browse without a key failed: {e}. Using Open on Nexus instead."));
                        format!("{{\"game\":{},\"query\":{},\"append\":{},\"fallback\":true,\"error\":{},\"items\":[]}}", quote(&info.id), quote(&text), offset > 0, quote(&e))
                    }
                    Err(e) => format!("{{\"game\":{},\"query\":{},\"append\":{},\"error\":{},\"items\":[]}}", quote(&info.id), quote(&text), offset > 0, quote(&e)),
                };
                set(qt, move |t| t.set_browse_json(QString::from(j.as_str())));
            }),
        );
    }

    /// Labs: browse Nexus once without a key and say what came back.
    pub fn nexus_keyless_check(self: Pin<&mut Self>) {
        let info = self.info();
        let domain = info.domain.clone().unwrap_or_else(|| "skyrimspecialedition".into());
        self.net_send(
            "Labs: Nexus without a key",
            Box::new(move |net: &mut Net, qt: &Qt| {
                let (ok, words) = match net.keyless_check(&domain) {
                    Ok(w) => (true, w),
                    Err(e) => (false, format!("Nexus browse without a key failed: {e}")),
                };
                crate::log::line("labs", &words);
                let j = format!("{{\"id\":\"nexus-keyless\",\"ok\":{ok},\"text\":{}}}", quote(&words));
                set(qt, move |t| t.set_labs_json(QString::from(j.as_str())));
            }),
        );
    }

    /// The screenshot checks: browsing Nexus fails as if Nexus refused it.
    pub fn set_sample_nexus_fail(self: Pin<&mut Self>) {
        self.net_send(
            "Test run: Nexus browsing refused",
            Box::new(|net: &mut Net, _qt: &Qt| {
                net.sample_nexus_fail = true;
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
                    id.parse::<u64>().map_err(|_| "That mod can't be read.".to_string()).and_then(|m| {
                        // What it needs comes first (Premium downloads them; anything else is a page to open).
                        match net.install_requirements(&info, m, &mut each, &mut prog) {
                            Ok(pages) if !pages.is_empty() => {
                                say(qt, format!("It also needs {}, which Turbine can't download. Its details page has the links.", pages.join(", ")), "warn")
                            }
                            Ok(_) => {}
                            Err(e) => crate::log::line("net", &format!("Requirements not installed: {e}")),
                        }
                        net.install_nexus(&info, m, 0, None, &mut each, &mut prog)
                    })
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
                    // SKSE for Steam is only on Nexus: free accounts get its page to open.
                    Err(e) if info.kind == "bethesda" && e.contains("30379") => {
                        let page = turbine_engine::nexus::files_page_url(turbine_engine::loaders::SKSE_NEXUS_DOMAIN, turbine_engine::loaders::SKSE_NEXUS_MOD);
                        let j = format!("[{{\"name\":\"SKSE (Skyrim Script Extender)\",\"url\":{}}}]", quote(&page));
                        crate::log::line("net", &e);
                        post(qt, move |t| t.pages(QString::from("SKSE is on Nexus Mods"), QString::from(j.as_str())));
                    }
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
                let Some((gid, p, sid)) = w.pending.pop_front() else { return };
                let Some(game) = game_of(w, &gid) else { return };
                if let Some(back) = f(w, &game, p) {
                    w.pending.push_front((gid, back, sid));
                    // Same installer, new step or choices: the helper's panel follows.
                    w.fomod_sent = None;
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
        self.fomod_job("Installer next step", installer_next);
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
            let g = game.name.clone();
            w.post(move |mut t| {
                if t.rust().catching.busy {
                    t.as_mut().set_status(&format!("{name} wasn't added"), "warn", None, &g, "", true, "");
                }
            });
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
                let lo = plugins::LoadOrder::read(&game).map_err(|e| e.to_string());
                post_order(w, &game, lo, "", "");
            }),
        );
    }

    pub fn load_order(mut self: Pin<&mut Self>) {
        let Some(game) = self.current_game() else { return };
        let gid = game.id();
        let g = game.clone();
        self.as_mut().send(
            Some(game.id()),
            "Read the load order",
            false,
            Box::new(move |w: &mut Worker| {
                let lo = plugins::sync(&g).map_err(|e| format!("The load order couldn't be read ({e})."));
                post_order(w, &g, lo, "", "");
            }),
        );
        // Then LOOT's notes on the plugins, with its masterlist (downloaded at most once a day).
        self.net_send(
            "LOOT's masterlist",
            Box::new(move |net: &mut Net, qt: &Qt| {
                let (lists, note) = net.loot_lists(false);
                if !note.is_empty() {
                    crate::log::line("loot", &note);
                }
                if lists.is_none() {
                    return;
                }
                post(qt, move |t| {
                    t.send(Some(gid), "LOOT's notes on the plugins", false, Box::new(move |w: &mut Worker| loot_messages(w, &game, lists)))
                });
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

    /// Sort automatically: with LOOT when its masterlist can be had, otherwise by masters only.
    pub fn order_sort(mut self: Pin<&mut Self>) {
        crate::log::line("action", "Load order: sort automatically");
        let Some(game) = self.current_game() else { return };
        self.as_mut().say("Getting LOOT's masterlist...", "work");
        self.net_send(
            "LOOT's masterlist for sorting",
            Box::new(move |net: &mut Net, qt: &Qt| {
                let (lists, note) = net.loot_lists(false);
                post(qt, move |t| {
                    t.send(Some(game.id()), "Sort the load order", true, Box::new(move |w: &mut Worker| sort_order(w, &game, lists, note)))
                });
            }),
        );
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
                let mut p = Profiles::open(&s.store).map_err(|e| format!("Loadouts couldn't be read ({e})."))?;
                p.switch(&mut s.store, &name)
            });
            match r {
                Ok(r) => {
                    w.note(game, &r);
                    w.say(format!("Using loadout {name}"), "ok");
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
                let mut p = Profiles::open(&s.store).map_err(|e| format!("Loadouts couldn't be read ({e})."))?;
                p.create(&s.store, &name)
            });
            match r {
                Ok(()) => w.say(format!("Made loadout {name} from the mods as they are now"), "ok"),
                Err(e) => w.say(e, "error"),
            }
        });
    }

    pub fn profile_rename(self: Pin<&mut Self>, from: &QString, to: &QString) {
        let (old, new) = (from.to_string(), to.to_string());
        self.profile_job("Rename profile", move |w, game| {
            let r = w.slot(game).and_then(|s| {
                let mut p = Profiles::open(&s.store).map_err(|e| format!("Loadouts couldn't be read ({e})."))?;
                p.rename(&old, &new)
            });
            match r {
                Ok(()) => w.say(format!("Renamed loadout {old} to {new}"), "ok"),
                Err(e) => w.say(e, "error"),
            }
        });
    }

    pub fn profile_delete(self: Pin<&mut Self>, name: &QString) {
        let name = name.to_string();
        self.profile_job("Delete profile", move |w, game| {
            let r = w.slot(game).and_then(|s| {
                let mut p = Profiles::open(&s.store).map_err(|e| format!("Loadouts couldn't be read ({e})."))?;
                p.delete(&name)
            });
            match r {
                Ok(()) => w.say(format!("Deleted loadout {name}"), "ok"),
                Err(e) => w.say(e, "error"),
            }
        });
    }

    pub fn profile_export(self: Pin<&mut Self>, name: &QString) {
        let name = name.to_string();
        self.profile_job("Export profile", move |w, game| {
            let home = turbine_engine::home_dir();
            let r = w.slot(game).and_then(|s| {
                let p = Profiles::open(&s.store).map_err(|e| format!("Loadouts couldn't be read ({e})."))?;
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

    pub fn profile_code(mut self: Pin<&mut Self>, name: &QString) {
        let Some(game) = self.current_game() else { return };
        let name = name.to_string();
        crate::log::line("action", &format!("Create a Thunderstore code for profile {name}"));
        let working = format!("{{\"state\":\"working\",\"profile\":{},\"code\":\"\",\"left\":0,\"files\":0,\"message\":\"\"}}", quote(&name));
        self.as_mut().set_code_json(QString::from(working.as_str()));
        self.send(
            Some(game.id()),
            "Read a profile for its code",
            false,
            Box::new(move |w: &mut Worker| {
                let r = w.slot(&game).and_then(|s| {
                    let p = Profiles::open(&s.store).map_err(|e| format!("Loadouts couldn't be read ({e})."))?;
                    let order = p.order_of(&s.store, &name).ok_or_else(|| format!("The loadout {name} isn't there any more."))?;
                    Ok(profiles::share_mods(&s.store, &order))
                });
                let mods = match r {
                    Ok(m) => m,
                    Err(e) => {
                        let j = code_json("error", &name, "", 0, 0, &e);
                        return w.post(move |t| t.set_code_json(QString::from(j.as_str())));
                    }
                };
                let ts: Vec<(String, Option<String>, bool)> = mods
                    .iter()
                    .filter(|m| m.kind == "thunderstore" && !m.id.is_empty())
                    .map(|m| (m.id.clone(), m.version.clone(), m.enabled))
                    .collect();
                let left = mods.iter().filter(|m| m.kind != "thunderstore" && m.kind != "loader").count();
                if ts.is_empty() {
                    let j = code_json("error", &name, "", left, 0, "This profile has no mods from Thunderstore, so there is nothing a code can hold.");
                    return w.post(move |t| t.set_code_json(QString::from(j.as_str())));
                }
                let configs = config_files(&game);
                w.post(move |t| {
                    let info = t.info();
                    t.net_send(
                        "Upload a profile to Thunderstore",
                        Box::new(move |net: &mut Net, qt: &Qt| {
                            let j = match net.export_code(&info, &name, &ts, &configs) {
                                Ok(code) => {
                                    crate::log::line("net", &format!("Profile code made for {name}: {} mods, {} settings files", ts.len(), configs.len()));
                                    code_json("ready", &name, &code, left, configs.len(), "")
                                }
                                Err(e) => code_json("error", &name, "", left, 0, &e),
                            };
                            set(qt, move |t| t.set_code_json(QString::from(j.as_str())));
                        }),
                    );
                });
            }),
        );
    }

    pub fn install_version(mut self: Pin<&mut Self>, id: &QString, source: &QString, version: &QString) {
        let Some(game) = self.current_game() else { return };
        let info = self.info();
        let (id, source, version) = (id.to_string(), source.to_string(), version.to_string());
        crate::log::line("action", &format!("Install version {version} of {id} from {source}"));
        self.as_mut().say("Getting that version ready to download...", "work");
        self.net_send(
            &format!("Install {version} of {id}"),
            Box::new(move |net: &mut Net, qt: &Qt| {
                let mut each = hand_over(qt, &game);
                let mut prog = progress(qt);
                let r = if source == "nexus" {
                    install_nexus_version(net, &info, &id, &version, &mut each, &mut prog)
                } else {
                    net.install_ts_version(&info, &version, &mut each, &mut prog)
                };
                if let Err(e) = r {
                    say(qt, e, "error");
                }
            }),
        );
    }

    pub fn set_sample_free(self: Pin<&mut Self>) {
        self.net_send(
            "Use the free sample account",
            Box::new(move |net: &mut Net, qt: &Qt| {
                net.sample_sign_in(true);
                let j = net.account_json("ok", "");
                set(qt, move |t| t.set_account_json(QString::from(j.as_str())));
            }),
        );
    }

    // ------------------------------------------------------------------------------------------------ Nexus links

    pub fn take_link(&self) -> QString {
        QString::from(crate::single::take().unwrap_or_default().as_str())
    }

    pub fn open_link(&self, link: &QString) -> QString {
        let out = |game: &str, name: &str, error: &str| {
            QString::from(format!("{{\"game\":{},\"name\":{},\"error\":{}}}", quote(game), quote(name), quote(error)).as_str())
        };
        let l = match turbine_engine::nxm::parse(&link.to_string()) {
            Ok(l) => l,
            Err(e) => {
                crate::log::line("nxm", &format!("A Nexus link arrived that can't be read: {e}"));
                return out("", "", &e);
            }
        };
        crate::log::line("nxm", &format!("Mod Manager Download link arrived: {}", l.redacted()));
        if crate::online::nexus_key().is_none() && !crate::online::using_samples() {
            // Without a key Turbine can't ask Nexus for the file. Slow download works, and the page's mod is
            // remembered so that download goes into the right game by itself.
            crate::log::line("nxm", "No Nexus API key: pointing to Slow download instead");
            let ids = l.store_ids();
            let game = self
                .current_game()
                .filter(|g| ids.contains(&g.store_id.as_str()))
                .or_else(|| self.rust().found.iter().find(|g| ids.contains(&g.store_id.as_str())).cloned());
            return QString::from(
                format!(
                    "{{\"game\":{},\"name\":{},\"error\":{},\"needsKey\":true,\"page\":{}}}",
                    quote(&game.map(|g| g.id()).unwrap_or_default()),
                    quote(l.game_name().unwrap_or("")),
                    quote(NO_KEY_NXM),
                    quote(&turbine_engine::nexus::mod_page_url(&l.domain, l.mod_id))
                )
                .as_str(),
            );
        }
        let Some(name) = l.game_name() else {
            return out("", "", &format!("That Nexus link is for a game Turbine doesn't support ({}).", l.domain));
        };
        if l.expired_at(turbine_engine::nxm::now()) {
            return out("", name, turbine_engine::nxm::EXPIRED_MESSAGE);
        }
        let ids = l.store_ids();
        // The game on screen first when it matches (Stardew Valley can be both Steam and GOG).
        let found = self
            .current_game()
            .filter(|g| ids.contains(&g.store_id.as_str()))
            .or_else(|| self.rust().found.iter().find(|g| ids.contains(&g.store_id.as_str())).cloned());
        match found {
            Some(g) => out(&g.id(), name, ""),
            None => out("", name, &format!("{name} isn't installed here, so Turbine has nowhere to put this mod.")),
        }
    }

    pub fn nxm_download(mut self: Pin<&mut Self>, link: &QString, game_id: &QString) {
        let Some(game) = self.find(&game_id.to_string()) else { return };
        let link_text = link.to_string();
        let link = match turbine_engine::nxm::parse(&link_text) {
            Ok(l) => l,
            Err(e) => return self.as_mut().say(&e, "error"),
        };
        // Pressed in Turbine's browser: the file goes into Game Mods/<Game>, like a Slow download.
        let into = crate::browse::from_page(&link_text).then(|| crate::web::game_mods_dir(&game));
        if let Some(d) = &into {
            crate::log::line("nxm", &format!("From Turbine's browser: the file goes into {}", d.display()));
        }
        self.as_mut().say("Getting the file from Nexus Mods...", "work");
        self.net_send(
            &format!("Nexus link {}", link.redacted()),
            Box::new(move |net: &mut Net, qt: &Qt| {
                let mut prog = progress(qt);
                match net.fetch_nxm(&link, into.as_deref(), &mut prog) {
                    Ok(f) => match into {
                        Some(dir) => {
                            let g = game.clone();
                            post(qt, move |t| t.web_place_fetched(g, f, dir));
                        }
                        None => hand_over(qt, &game)(f),
                    },
                    Err(e) => say(qt, e, "error"),
                }
            }),
        );
    }

    /// The names of a Mod Manager Download link's mod and file, for "Get <mod> (<file>) for <game>?": land in
    /// nxmNameJson {"link","name","file"} ("" when Nexus can't say).
    pub fn nxm_ask_name(self: Pin<&mut Self>, link: &QString) {
        let text = link.to_string();
        let Ok(l) = turbine_engine::nxm::parse(&text) else { return };
        self.net_send(
            "The name of a Mod Manager Download link's mod",
            Box::new(move |net: &mut Net, qt: &Qt| {
                let name = net.mod_name(&l.domain, l.mod_id).unwrap_or_default();
                let file = net.file_name(&l.domain, l.mod_id, l.file_id).unwrap_or_default();
                let j = format!("{{\"link\":{},\"name\":{},\"file\":{}}}", quote(&text), quote(&name), quote(&file));
                set(qt, move |t| t.set_nxm_name_json(QString::from(j.as_str())));
            }),
        );
    }

    pub fn handler_status(self: Pin<&mut Self>) {
        self.net_send(
            "Check the Nexus link handler",
            Box::new(move |_: &mut Net, qt: &Qt| {
                let j = crate::system::nxm_handler_json();
                set(qt, move |t| t.set_handler_json(QString::from(j.as_str())));
            }),
        );
    }

    pub fn handler_set(self: Pin<&mut Self>) {
        crate::log::line("action", "Use Turbine for Nexus downloads");
        self.net_send(
            "Make Turbine the Nexus link handler",
            Box::new(move |_: &mut Net, qt: &Qt| {
                let r = crate::system::set_nxm_handler();
                let j = crate::system::nxm_handler_json();
                if let Err(e) = r {
                    say(qt, format!("Turbine couldn't be set for Nexus downloads ({e})."), "error");
                }
                set(qt, move |t| t.set_handler_json(QString::from(j.as_str())));
            }),
        );
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
                let Some((gid, share)) = w.share.clone() else { return };
                // The game the share was read for, not whichever game is showing now (LB/RB may have moved on).
                let game = if gid == game.id() {
                    game
                } else {
                    match w.slots.get(&gid).map(|s| s.game.clone()) {
                        Some(g) => g,
                        None => return w.say("That setup is for another game. Switch to that game and open it again.", "error"),
                    }
                };
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
                // Nexus mods Turbine can fetch for a Premium account (free accounts get their pages instead).
                let domain = turbine_engine::nexus::domain_for(game.store, &game.store_id);
                let fetch: Vec<(String, u64, Option<u64>, String)> = profiles::nexus_steps(&plan, domain, true)
                    .into_iter()
                    .filter_map(|st| match st.action {
                        profiles::NexusAction::Fetch { domain, mod_id, file_id } => Some((domain, mod_id, file_id, st.entry.name)),
                        _ => None,
                    })
                    .collect();
                if wanted.is_empty() && fetch.is_empty() {
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
                            if !wanted.is_empty() {
                                if let Err(e) = net.install_ts(&info, &wanted, false, &mut each, &mut prog) {
                                    say(qt, e, "error");
                                }
                            }
                            let premium = !fetch.is_empty() && net.account().map(|a| a.is_premium).unwrap_or(false);
                            if premium {
                                for (domain, mod_id, file_id, name) in &fetch {
                                    match net.fetch_by_id(domain, *mod_id, *file_id, name, &mut prog) {
                                        Ok(mut f) => {
                                            f.replaces = None;
                                            each(f)
                                        }
                                        Err(e) => say(qt, format!("{name}: {e}"), "error"),
                                    }
                                }
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
