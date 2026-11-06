//! Catching mods the player downloads in their own browser, and saying how it goes:
//! - the Downloads watcher: opening a Nexus mod page from Turbine starts a `watch::Watcher` for that mod. A file that
//!   matches one of the mod's files by name or size is added by itself; a guess (Turbine couldn't read the mod's file
//!   list) is added only after the player says yes;
//! - the browser helper (bridge.rs): downloads the extension reports, its Mod Manager Download links, and installer
//!   choices made on the Nexus page;
//! - the status both show: `statusJson` for the overlay (over the browser, Desktop and Game Mode) and "status",
//!   "added" and "choices" messages for the helper's panel on the page;
//! - the browser check, the helper's folder in Downloads, and the Game Mode overlay's window property.
//!
//! Files in Downloads are never moved, changed or deleted: Turbine only reads them (the player's own downloads).

use crate::backend::{qobject, Worker};
use crate::bridge;
use core::pin::Pin;
use cxx_qt::{CxxQtThread, CxxQtType, Threading};
use cxx_qt_lib::QString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use turbine_engine::json::{quote, Json};
use turbine_engine::net::SourceInfo;
use turbine_engine::watch::{self, Confidence, Expectation, Found, Watcher};
use turbine_engine::{browser, nexus, Game};

type Qt = CxxQtThread<qobject::Turbine>;

/// Where Turbine's Flatpak keeps the browser helper (installed from the repository's extension folder).
pub const HELPER_SOURCE: &str = "/app/share/turbine-mod-manager/extension";
/// The folder in Downloads the helper is copied to, for Load unpacked.
pub const HELPER_FOLDER: &str = "Turbine browser helper";
/// The most watchers at once (one per Nexus page opened from Turbine, each for 30 minutes).
const MAX_WATCHES: usize = 8;

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn post<F>(qt: &Qt, f: F)
where
    F: FnOnce(Pin<&mut qobject::Turbine>) + Send + 'static,
{
    let _ = qt.queue(f);
}

/// The folders Turbine watches and accepts helper downloads from: ~/Downloads and the desktop's Downloads folder.
/// Steam's own browser saves where it is told; where that is in Game Mode isn't known yet (see Labs, Steam's
/// built-in browser).
pub fn watch_folders() -> Vec<PathBuf> {
    watch::default_folders(&turbine_engine::home_dir())
}

/// A finished download the helper reported: a real archive file directly in one of the watched folders.
pub fn accept_download(path: &str) -> Result<PathBuf, String> {
    let p = PathBuf::from(path);
    if !p.is_absolute() {
        return Err("the browser gave no full path".into());
    }
    let meta = std::fs::symlink_metadata(&p).map_err(|_| "the file isn't there (Turbine can only read Downloads)".to_string())?;
    if !meta.is_file() {
        return Err("it isn't a plain file".into());
    }
    let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    if !turbine_engine::archives::is_archive_name(&name) {
        return Err("it isn't a zip, 7z or rar file".into());
    }
    let real = p.canonicalize().map_err(|e| e.to_string())?;
    let parent = real.parent().map(Path::to_path_buf).unwrap_or_default();
    let ok = watch_folders().iter().any(|f| f.canonicalize().map(|c| c == parent).unwrap_or(false));
    if !ok {
        return Err("it isn't in Downloads".into());
    }
    Ok(real)
}

/// What Nexus says the file is, from what is known before it is identified: the mod, and the file when known.
fn hint_source(r: &bridge::NexusRef, version: Option<String>) -> SourceInfo {
    SourceInfo {
        kind: "nexus".into(),
        id: match r.file_id {
            Some(f) => nexus::source_id(&r.domain, r.mod_id, f),
            None => format!("{}:{}", r.domain, r.mod_id),
        },
        version: version.filter(|v| !v.is_empty()),
        url: Some(nexus::mod_page_url(&r.domain, r.mod_id)),
        sha256: None,
        needs: Vec::new(),
    }
}

/// Looks a file up on Nexus by its MD5 when the player has a key (never in the build's screenshot checks).
pub fn identify(domain: &str, path: &Path) -> Option<SourceInfo> {
    let key = crate::online::nexus_key()?;
    let client = turbine_engine::net::Client::new(crate::VERSION, &crate::online::temp_dir());
    let nx = nexus::Nexus::new(client, &key, crate::VERSION);
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    match nx.identify(domain, path) {
        Ok(Some(m)) => {
            crate::log::line("catch", &format!("{name} is {} {} on Nexus (mod {}, file {})", m.mod_info.name, m.file.version, m.mod_info.mod_id, m.file.file_id));
            Some(nexus::md5_source_info(&m, None))
        }
        Ok(None) => {
            crate::log::line("catch", &format!("Nexus doesn't know {name} by its MD5"));
            None
        }
        Err(e) => {
            crate::log::line("catch", &format!("Couldn't identify {name}: {e}"));
            None
        }
    }
}

/// A file found by a guess, waiting for the player's yes.
#[derive(Clone, Debug)]
pub struct Ask {
    pub path: PathBuf,
    pub game: Game,
    pub r: bridge::NexusRef,
}

/// A download the helper is reporting.
#[derive(Clone, Debug)]
pub struct HelperDownload {
    pub label: String,
    pub game: Option<Game>,
    pub r: Option<bridge::NexusRef>,
    pub total: u64,
}

/// State kept on the Turbine object.
#[derive(Default)]
pub struct Catching {
    pub started: bool,
    /// Running watchers by "domain:mod", with their stop switches; the key of the one that guesses, if any.
    pub watches: Vec<(String, Arc<AtomicBool>, bool)>,
    /// Files already added (or being added), so the watcher and the helper never add the same one twice.
    pub handled: std::collections::VecDeque<PathBuf>,
    pub asks: Vec<Ask>,
    pub downloads: Vec<(String, HelperDownload)>,
    /// The Nexus mod page the helper last reported.
    pub page: Option<bridge::NexusRef>,
    /// Is the status a caught download's (so finishing its installer reports "Added")?
    pub busy: bool,
}

fn status_text(active: bool, text: &str, level: &str, progress: Option<f64>, game: &str, added: &str, done: bool, ask: &str) -> String {
    format!(
        "{{\"active\":{active},\"text\":{},\"level\":{},\"progress\":{},\"game\":{},\"added\":{},\"done\":{done},\"ask\":{}}}",
        quote(text),
        quote(level),
        progress.map(|p| format!("{:.3}", p.clamp(0.0, 1.0))).unwrap_or_else(|| "-1".into()),
        quote(game),
        quote(added),
        quote(ask)
    )
}

fn games_json(games: &[Game]) -> String {
    let items: Vec<String> = games
        .iter()
        .filter_map(|g| {
            let d = nexus::domain_for_store_id(&g.store_id)?;
            Some(format!("{{\"id\":{},\"name\":{},\"domain\":{}}}", quote(&g.id()), quote(&g.name), quote(d)))
        })
        .collect();
    format!("[{}]", items.join(","))
}

fn label_of(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn id_of(msg: &Json) -> String {
    match msg.get("id") {
        Some(Json::Str(s)) => s.clone(),
        Some(Json::Num(n)) => format!("{n}"),
        _ => String::new(),
    }
}

fn num_of(msg: &Json, key: &str) -> u64 {
    match msg.get(key) {
        Some(Json::Num(n)) if *n >= 0.0 => *n as u64,
        _ => 0,
    }
}

impl qobject::Turbine {
    /// A game for a Nexus domain: the one on screen when it fits, otherwise the first installed one.
    pub(crate) fn game_for_domain(&self, domain: &str) -> Option<Game> {
        let fits = |g: &Game| nexus::domain_for_store_id(&g.store_id) == Some(domain);
        self.current_game().filter(|g| fits(g)).or_else(|| self.rust().found.iter().find(|g| fits(g)).cloned())
    }

    /// Sets the overlay's status and tells the browser helper.
    pub(crate) fn set_status(mut self: Pin<&mut Self>, text: &str, level: &str, progress: Option<f64>, game: &str, added: &str, done: bool, ask: &str) {
        let j = status_text(true, text, level, progress, game, added, done, ask);
        self.as_mut().rust_mut().catching.busy = !done;
        self.as_mut().set_status_json(QString::from(j.as_str()));
        let p = progress.map(|p| format!("{:.3}", p.clamp(0.0, 1.0))).unwrap_or_else(|| "null".into());
        let g = if game.is_empty() { "null".to_string() } else { quote(game) };
        bridge::broadcast(&format!("{{\"type\":\"status\",\"text\":{},\"level\":{},\"progress\":{p},\"game\":{g}}}", quote(text), quote(level)));
    }

    /// The overlay went away (6 seconds after a result): nothing to show until the next download.
    pub fn status_done(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().catching.busy = false;
        self.as_mut().set_status_json(QString::from(status_text(false, "", "ok", None, "", "", true, "").as_str()));
    }

    /// Once, at start (not in the screenshot checks): the browser helper's server and the browser check.
    pub fn start_catching(mut self: Pin<&mut Self>) {
        if self.rust().catching.started {
            return;
        }
        self.as_mut().rust_mut().catching.started = true;
        let _ = bridge::VERSION.set(crate::VERSION.to_string());
        let qt = self.qt_thread();
        let handler: bridge::Handler = Arc::new(move |kind: &str, msg: &Json| {
            let (kind, msg) = (kind.to_string(), msg.clone());
            post(&qt, move |t| t.bridge_message(&kind, &msg));
        });
        let dir = crate::log::folder(&crate::backend::data_home());
        match bridge::start(&dir, |l| crate::log::line("bridge", l), handler) {
            Ok(port) => crate::log::line("bridge", &format!("Browser helper link on 127.0.0.1:{port}")),
            Err(e) => crate::log::line("bridge", &format!("Browser helper link not started: {e}")),
        }
        bridge::set_games(games_json(&self.rust().found));
        self.as_mut().bridge_refresh();
        self.browser_check();
    }

    /// Looks for a web browser on the host (in the background); the answer lands in `browserJson`.
    pub fn browser_check(self: Pin<&mut Self>) {
        let qt = self.qt_thread();
        let _ = std::thread::Builder::new().name("turbine-browser-check".into()).spawn(move || {
            let c = browser::check();
            crate::log::line("browser", &format!("Browser: {}", if c.found { format!("{} ({})", c.name, c.how) } else { "none found".into() }));
            let j = browser_json(&c.to_json());
            post(&qt, move |t| t.set_browser_json(QString::from(j.as_str())));
        });
    }

    pub fn bridge_refresh(mut self: Pin<&mut Self>) {
        let j = bridge::status_json();
        if *self.bridge_json() != QString::from(j.as_str()) {
            self.as_mut().set_bridge_json(QString::from(j.as_str()));
        }
    }

    pub fn bridge_pair_url(self: Pin<&mut Self>) -> QString {
        crate::log::line("action", "Pair the browser helper");
        QString::from(bridge::pair_url().unwrap_or_default().as_str())
    }

    pub fn bridge_pair_again(mut self: Pin<&mut Self>) -> QString {
        crate::log::line("action", "Pair the browser helper again (new token)");
        if let Err(e) = bridge::pair_again() {
            self.as_mut().say(&e, "error");
            return QString::default();
        }
        self.as_mut().bridge_refresh();
        QString::from(bridge::pair_url().unwrap_or_default().as_str())
    }

    /// Copies the helper into Downloads for Load unpacked. Answers {"folder": path, "error": ""}.
    pub fn helper_setup(self: Pin<&mut Self>) -> QString {
        crate::log::line("action", "Set up the browser helper");
        let to = turbine_engine::home_dir().join("Downloads").join(HELPER_FOLDER);
        let from = std::env::var_os("TURBINE_HELPER_SOURCE").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(HELPER_SOURCE));
        let r = copy_helper(&from, &to);
        let (folder, error) = match r {
            Ok(n) => {
                crate::log::line("bridge", &format!("Copied the browser helper ({n} files) to {}", to.display()));
                (to.to_string_lossy().into_owned(), String::new())
            }
            Err(e) => {
                crate::log::line("bridge", &format!("Couldn't copy the browser helper: {e}"));
                (String::new(), format!("Turbine couldn't put the helper in Downloads ({e})."))
            }
        };
        QString::from(format!("{{\"folder\":{},\"error\":{}}}", quote(&folder), quote(&error)).as_str())
    }

    /// Marks the Game Mode overlay window for gamescope (on) or takes the mark off (off). See qtglue.cpp.
    pub fn overlay_mark(self: Pin<&mut Self>, on: bool) -> i32 {
        // SAFETY: a plain call into qtglue.cpp on the GUI thread; it only looks up Turbine's own window.
        let r = unsafe { turbine_overlay_mark(i32::from(on)) };
        crate::log::line("overlay", &format!("GAMESCOPE_EXTERNAL_OVERLAY {} ({})", if on { "set to 1" } else { "set to 0" }, match r {
            1 => "done",
            -1 => "the overlay window wasn't found",
            -2 => "not on X11, so gamescope can't be told",
            _ => "this build can't reach X11",
        }));
        r
    }

    // ------------------------------------------------------------------------------------------------ the watcher

    /// Opening a Nexus mod page: watch Downloads for its file. Answers {"watching", "game", "error"}.
    pub fn watch_page(mut self: Pin<&mut Self>, url: &QString) -> QString {
        let url = url.to_string();
        let out = |watching: bool, game: &str, error: &str| {
            QString::from(format!("{{\"watching\":{watching},\"game\":{},\"error\":{}}}", quote(game), quote(error)).as_str())
        };
        let Some(r) = bridge::nexus_page(&url) else { return out(false, "", "") };
        let Some(game) = self.game_for_domain(&r.domain) else {
            crate::log::line("catch", &format!("Not watching for {}:{}: no game for that domain here", r.domain, r.mod_id));
            return out(false, "", "That game isn't installed here.");
        };
        let name = game.name.clone();
        self.as_mut().start_watch(game, r, None);
        out(true, &name, "")
    }

    /// Labs, Steam's built-in browser: a guessing watcher for the game on screen. Answers the steam:// address.
    pub fn watch_steam_browser(mut self: Pin<&mut Self>) -> QString {
        if let Some(game) = self.current_game() {
            let domain = nexus::domain_for_store_id(&game.store_id).unwrap_or("").to_string();
            let r = bridge::NexusRef { domain, mod_id: 0, file_id: None };
            self.as_mut().start_watch(game, r, Some(nexus::FileList::default()));
        }
        QString::from("steam://openurl/https://www.nexusmods.com")
    }

    /// Starts (or restarts) a watcher. `files`: the mod's file list when already known; otherwise it is read from
    /// Nexus first when the player has a key.
    fn start_watch(mut self: Pin<&mut Self>, game: Game, r: bridge::NexusRef, files: Option<nexus::FileList>) {
        let key = format!("{}:{}", r.domain, r.mod_id);
        let stop = Arc::new(AtomicBool::new(false));
        {
            let mut rm = self.as_mut().rust_mut();
            let c = &mut rm.catching;
            c.watches.retain(|(k, s, _)| {
                let same = *k == key;
                if same {
                    s.store(true, Ordering::SeqCst);
                }
                !same && !s.load(Ordering::SeqCst)
            });
            while c.watches.len() >= MAX_WATCHES {
                let (_, s, _) = c.watches.remove(0);
                s.store(true, Ordering::SeqCst);
            }
            c.watches.push((key.clone(), stop.clone(), false));
        }
        let qt = self.qt_thread();
        let started = now_secs();
        let _ = std::thread::Builder::new().name("turbine-watch".into()).spawn(move || {
            let list = files.or_else(|| {
                let k = crate::online::nexus_key()?;
                if r.mod_id == 0 {
                    return None;
                }
                let client = turbine_engine::net::Client::new(crate::VERSION, &crate::online::temp_dir());
                match nexus::Nexus::new(client, &k, crate::VERSION).files(&r.domain, r.mod_id) {
                    Ok(l) => Some(l),
                    Err(e) => {
                        crate::log::line("catch", &format!("Nexus's file list for {}:{} couldn't be read ({e}); watching by guess", r.domain, r.mod_id));
                        None
                    }
                }
            });
            let mut exp = Expectation::new(&game.id(), &r.domain, r.mod_id, started);
            if let Some(l) = &list {
                exp = exp.with_files(l);
            }
            let guess = exp.files.iter().all(|f| f.file_name.is_empty() && f.size_bytes == 0);
            {
                let (k2, s2) = (key.clone(), stop.clone());
                post(&qt, move |t| t.watch_started(&k2, &s2, guess));
            }
            let folders = watch_folders();
            crate::log::line(
                "catch",
                &format!(
                    "Watching {} for {} mod {} ({}; {} files known)",
                    folders.iter().map(|f| f.display().to_string()).collect::<Vec<_>>().join(", "),
                    game.name,
                    r.mod_id,
                    if guess { "will ask before adding" } else { "adds a match by itself" },
                    exp.files.len()
                ),
            );
            let mut w = Watcher::new(exp, folders.clone());
            let mut partial = String::new();
            let found = loop {
                if stop.load(Ordering::SeqCst) || w.expectation.expired_at(now_secs()) {
                    break None;
                }
                if let Some(f) = w.poll() {
                    break Some(f);
                }
                // A download still being written: say so (unless the helper reports it with real progress).
                if !bridge::connected() {
                    if let Some((name, size)) = unfinished(&folders, started) {
                        let line = format!("{name}:{size}");
                        if line != partial {
                            partial = line;
                            let g = game.name.clone();
                            post(&qt, move |mut t| {
                                if !t.rust().catching.busy || t.status_json().to_string().contains("Downloading") {
                                    t.as_mut().set_status(&format!("Downloading {name}"), "busy", None, &g, "", false, "");
                                }
                            });
                        }
                    }
                }
                std::thread::sleep(Duration::from_secs(watch::POLL_SECS));
            };
            let k3 = key.clone();
            match found {
                Some(f) => post(&qt, move |t| t.watch_found(&k3, game, r, f)),
                None => post(&qt, move |mut t| t.as_mut().rust_mut().catching.watches.retain(|(k, s, _)| *k != k3 || !s.load(Ordering::SeqCst))),
            }
        });
    }

    fn watch_started(mut self: Pin<&mut Self>, key: &str, stop: &Arc<AtomicBool>, guess: bool) {
        let mut rm = self.as_mut().rust_mut();
                    let c = &mut rm.catching;
        if guess {
            // Only one watcher guesses at a time: the newest page's.
            for (k, s, g) in c.watches.iter() {
                if *g && k != key {
                    s.store(true, Ordering::SeqCst);
                }
            }
        }
        for w in c.watches.iter_mut() {
            if w.0 == key && Arc::ptr_eq(&w.1, stop) {
                w.2 = guess;
            }
        }
    }

    fn handled(&self, path: &Path) -> bool {
        let real = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        self.rust().catching.handled.contains(&real)
    }

    fn mark_handled(mut self: Pin<&mut Self>, path: &Path) {
        let real = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let mut rm = self.as_mut().rust_mut();
        let h = &mut rm.catching.handled;
        if h.len() >= 100 {
            h.pop_front();
        }
        h.push_back(real);
    }

    fn watch_found(mut self: Pin<&mut Self>, key: &str, game: Game, r: bridge::NexusRef, f: Found) {
        self.as_mut().rust_mut().catching.watches.retain(|(k, _, _)| k != key);
        let name = label_of(&f.path);
        crate::log::line("catch", &format!("Found {name} in Downloads ({} confidence) for {}", f.confidence.label(), game.name));
        if self.handled(&f.path) {
            crate::log::line("catch", &format!("{name} was already added"));
            return;
        }
        match f.confidence {
            Confidence::Name | Confidence::Size => {
                let r = bridge::NexusRef { file_id: f.file.as_ref().map(|x| x.file_id).filter(|x| *x > 0).or(r.file_id), ..r };
                let version = f.file.as_ref().map(|x| x.version.clone());
                self.catch_add(game, f.path, Some((r, version)));
            }
            Confidence::Guess => {
                let ask = Ask { path: f.path.clone(), game: game.clone(), r };
                self.as_mut().rust_mut().catching.asks.retain(|a| a.path != ask.path);
                self.as_mut().rust_mut().catching.asks.push(ask);
                let p = f.path.to_string_lossy().into_owned();
                self.as_mut().set_status(&format!("Found {name}. Add it to {}?", game.name), "warn", None, &game.name, "", false, &p);
                let j = format!(
                    "{{\"path\":{},\"name\":{},\"size\":{},\"game\":{},\"gameName\":{}}}",
                    quote(&p),
                    quote(&name),
                    f.size,
                    quote(&game.id()),
                    quote(&game.name)
                );
                self.asked(QString::from(j.as_str()));
            }
        }
    }

    /// The player's answer to "Add it?" for a file found by a guess.
    pub fn watch_confirm(mut self: Pin<&mut Self>, path: &QString, yes: bool) {
        let path = PathBuf::from(path.to_string());
        let ask = {
            let mut rm = self.as_mut().rust_mut();
            let asks = &mut rm.catching.asks;
            let i = asks.iter().position(|a| a.path == path);
            let taken = i.map(|i| asks.remove(i));
            taken
        };
        let Some(ask) = ask else { return };
        crate::log::line("action", &format!("{} {}", if yes { "Add" } else { "Don't add" }, label_of(&path)));
        if yes {
            let hint = (ask.r.mod_id > 0).then_some((ask.r, None));
            self.catch_add(ask.game, ask.path, hint);
        } else {
            self.status_done();
        }
    }

    /// Adds a caught download to its game: identified on Nexus first when the player has a key, then added on the mod
    /// worker; the status says how it went.
    pub(crate) fn catch_add(mut self: Pin<&mut Self>, game: Game, path: PathBuf, hint: Option<(bridge::NexusRef, Option<String>)>) {
        if self.handled(&path) {
            return;
        }
        self.as_mut().mark_handled(&path);
        let label = label_of(&path);
        self.as_mut().set_status(&format!("Adding {label} to {}", game.name), "busy", None, &game.name, "", false, "");
        self.as_mut().edited();
        let qt = self.qt_thread();
        let domain = hint.as_ref().map(|(r, _)| r.domain.clone()).or_else(|| nexus::domain_for_store_id(&game.store_id).map(str::to_string));
        let _ = std::thread::Builder::new().name("turbine-identify".into()).spawn(move || {
            let found = domain.as_deref().filter(|d| !d.is_empty()).and_then(|d| identify(d, &path));
            let source = found
                .or_else(|| hint.as_ref().map(|(r, v)| hint_source(r, v.clone())))
                .unwrap_or_else(|| SourceInfo { kind: "zip".into(), ..Default::default() });
            post(&qt, move |t| {
                let name = format!("Add {label} (caught download)");
                t.send(
                    Some(game.id()),
                    &name,
                    true,
                    Box::new(move |w: &mut Worker| {
                        let out = crate::actions::add_archive(w, &game, &path, source, None, &label);
                        report(w, &game, out);
                    }),
                )
            });
        });
    }

    /// A hand-added file (Add from Downloads) when the player has a Nexus key: identified by its MD5 first, so it
    /// gets the mod's name, version and update checks.
    pub(crate) fn import_identified(mut self: Pin<&mut Self>, game: Game, path: PathBuf, domain: String) {
        let qt = self.qt_thread();
        let label = label_of(&path);
        self.as_mut().say(&format!("Looking up {label} on Nexus Mods..."), "work");
        let _ = std::thread::Builder::new().name("turbine-identify".into()).spawn(move || {
            let source = identify(&domain, &path).unwrap_or_else(|| SourceInfo { kind: "zip".into(), ..Default::default() });
            post(&qt, move |mut t| {
                t.as_mut().say(&format!("Adding {label}..."), "work");
                t.send(
                    Some(game.id()),
                    &format!("Add {label}"),
                    true,
                    Box::new(move |w: &mut Worker| {
                        crate::actions::add_archive(w, &game, &path, source, None, &label);
                    }),
                )
            });
        });
    }

    // ------------------------------------------------------------------------------------------------ the helper

    /// A message from the browser helper (bridge.rs), on the interface thread.
    pub(crate) fn bridge_message(mut self: Pin<&mut Self>, kind: &str, msg: &Json) {
        match kind {
            "connected" => {
                bridge::set_games(games_json(&self.rust().found));
                self.as_mut().bridge_refresh();
                // The status now, if there is one, then any installer choices already waiting.
                if let Ok(s) = turbine_engine::json::parse(&self.status_json().to_string()) {
                    if s.get("active").and_then(Json::as_bool) == Some(true) {
                        let p = match s.get("progress") {
                            Some(Json::Num(n)) if *n >= 0.0 => format!("{n:.3}"),
                            _ => "null".into(),
                        };
                        let g = s.str_at("game").filter(|g| !g.is_empty()).map(quote).unwrap_or_else(|| "null".into());
                        bridge::broadcast(&format!(
                            "{{\"type\":\"status\",\"text\":{},\"level\":{},\"progress\":{p},\"game\":{g}}}",
                            quote(s.str_at("text").unwrap_or("")),
                            quote(s.str_at("level").unwrap_or("ok"))
                        ));
                    }
                }
                self.send(None, "Send installer choices to the browser helper", false, Box::new(|w: &mut Worker| crate::actions::resend_fomod(w)));
            }
            "disconnected" => self.as_mut().bridge_refresh(),
            "page" => {
                let r = msg.str_at("url").and_then(bridge::nexus_page);
                if let Some(r) = &r {
                    crate::log::line("bridge", &format!("Nexus page open: {} mod {}", r.domain, r.mod_id));
                }
                self.as_mut().rust_mut().catching.page = r;
            }
            "nxm" => {
                let url = msg.str_at("url").unwrap_or("").to_string();
                if crate::single::plausible_link(&url) {
                    crate::log::line("bridge", "Mod Manager Download link from the browser helper");
                    crate::single::push(url);
                } else {
                    crate::log::line("bridge", "The browser helper sent a link Turbine can't use");
                }
            }
            "download.started" => {
                let id = id_of(msg);
                let url = msg.str_at("finalUrl").filter(|u| !u.is_empty()).or(msg.str_at("url")).unwrap_or("");
                let referrer = msg.str_at("referrer").unwrap_or("");
                if !bridge::is_nexus_host(url) && !bridge::is_nexus_host(referrer) {
                    return;
                }
                let r = bridge::nexus_ref(url, referrer).or_else(|| self.rust().catching.page.clone());
                let game = r.as_ref().and_then(|r| self.game_for_domain(&r.domain));
                let file = msg.str_at("filename").unwrap_or("");
                let label = Path::new(file).file_name().map(|n| n.to_string_lossy().into_owned()).filter(|n| !n.is_empty()).unwrap_or_else(|| "a file from Nexus".into());
                let total = num_of(msg, "totalBytes");
                crate::log::line("bridge", &format!("Download started: {label} ({})", r.as_ref().map(|r| format!("{} mod {}", r.domain, r.mod_id)).unwrap_or_else(|| "mod not known".into())));
                let gname = game.as_ref().map(|g| g.name.clone()).unwrap_or_default();
                let d = HelperDownload { label: label.clone(), game, r, total };
                let mut rm = self.as_mut().rust_mut();
                let c = &mut rm.catching;
                c.downloads.retain(|(i, _)| *i != id);
                c.downloads.push((id, d));
                if c.downloads.len() > 20 {
                    c.downloads.remove(0);
                }
                self.as_mut().set_status(&format!("Downloading {label}"), "busy", Some(0.0), &gname, "", false, "");
            }
            "download.progress" => {
                let id = id_of(msg);
                let Some(d) = self.rust().catching.downloads.iter().find(|(i, _)| *i == id).map(|(_, d)| d.clone()) else { return };
                let got = num_of(msg, "receivedBytes");
                let total = num_of(msg, "totalBytes").max(d.total);
                let p = (total > 0).then(|| got as f64 / total as f64);
                let gname = d.game.as_ref().map(|g| g.name.clone()).unwrap_or_default();
                self.as_mut().set_status(&format!("Downloading {}", d.label), "busy", p, &gname, "", false, "");
            }
            "download.failed" => {
                let id = id_of(msg);
                let d = {
                    let mut rm = self.as_mut().rust_mut();
                    let c = &mut rm.catching;
                    let i = c.downloads.iter().position(|(x, _)| *x == id);
                    let taken = i.map(|i| c.downloads.remove(i).1);
                    taken
                };
                let Some(d) = d else { return };
                let why = msg.str_at("error").unwrap_or("");
                crate::log::line("bridge", &format!("Download of {} stopped: {why}", d.label));
                let gname = d.game.as_ref().map(|g| g.name.clone()).unwrap_or_default();
                let words = if why.eq_ignore_ascii_case("USER_CANCELED") { format!("{} was cancelled", d.label) } else { format!("The download of {} stopped. Try it again on the page.", d.label) };
                self.as_mut().set_status(&words, "warn", None, &gname, "", true, "");
            }
            "download.finished" => {
                let id = id_of(msg);
                let d = {
                    let mut rm = self.as_mut().rust_mut();
                    let c = &mut rm.catching;
                    let i = c.downloads.iter().position(|(x, _)| *x == id);
                    let taken = i.map(|i| c.downloads.remove(i).1);
                    taken
                };
                let path = msg.str_at("path").unwrap_or("");
                let path = match accept_download(path) {
                    Ok(p) => p,
                    Err(e) => {
                        crate::log::line("bridge", &format!("Finished download not taken: {e}"));
                        if d.is_some() {
                            self.as_mut().set_status("Turbine can't read that download. Save mods to Downloads, then add them from Get Mods, Add from Downloads.", "warn", None, "", "", true, "");
                        }
                        return;
                    }
                };
                let r = d.as_ref().and_then(|d| d.r.clone());
                let game = d.as_ref().and_then(|d| d.game.clone()).or_else(|| r.as_ref().and_then(|r| self.game_for_domain(&r.domain)));
                match game {
                    Some(g) => self.catch_add(g, path, r.map(|r| (r, None))),
                    None => {
                        let label = label_of(&path);
                        crate::log::line("bridge", &format!("{label}: no game for it here"));
                        self.as_mut().set_status(
                            &format!("Turbine doesn't know which game {label} is for. Add it from that game's Get Mods, Add from Downloads."),
                            "warn",
                            None,
                            "",
                            "",
                            true,
                            "",
                        );
                    }
                }
            }
            "choice" | "choice.back" | "choice.cancel" => {
                let session = msg.str_at("sessionId").unwrap_or("").to_string();
                let step = num_of(msg, "step") as usize;
                let selected: Vec<String> = msg.get("selected").map(Json::as_array).unwrap_or(&[]).iter().filter_map(|x| x.as_str().map(str::to_string)).take(200).collect();
                let what = kind.to_string();
                crate::log::line("bridge", &format!("Installer {what} from the browser"));
                self.send(
                    None,
                    "Installer choice from the browser",
                    false,
                    Box::new(move |w: &mut Worker| crate::actions::bridge_choice(w, &what, &session, step, &selected)),
                );
            }
            _ => {}
        }
    }

    /// A caught mod went in (or its installer finished): the status and the helper hear about it.
    pub(crate) fn caught_added(mut self: Pin<&mut Self>, game: &str, name: &str, notes: Vec<String>, only_if_busy: bool) {
        if only_if_busy && !self.rust().catching.busy {
            return;
        }
        self.as_mut().set_status(&format!("Added to {game}"), "ok", None, game, name, true, "");
        let notes: Vec<String> = notes.iter().map(|n| quote(n)).collect();
        bridge::broadcast(&format!("{{\"type\":\"added\",\"game\":{},\"mod\":{},\"notes\":[{}],\"needs\":[]}}", quote(game), quote(name), notes.join(",")));
    }
}

/// What add_archive did with a caught download, for the status.
fn report(w: &Worker, game: &Game, out: crate::actions::Outcome) {
    let g = game.name.clone();
    match out {
        crate::actions::Outcome::Added { name, notes } => w.post(move |t| t.caught_added(&g, &name, notes, false)),
        crate::actions::Outcome::Choices { name } => w.post(move |t| {
            t.set_status(&format!("{name} has installer choices. Pick them on the page or in Turbine."), "busy", None, &g, "", false, "")
        }),
        crate::actions::Outcome::Failed(e) => w.post(move |t| t.set_status(&e, "stop", None, &g, "", true, "")),
    }
}

/// The newest unfinished download (a browser's .part, .crdownload or .download file) started after `since`.
fn unfinished(folders: &[PathBuf], since: u64) -> Option<(String, u64)> {
    let mut best: Option<(u64, String, u64)> = None;
    for s in watch::scan(folders) {
        let name = label_of(&s.path);
        let lc = name.to_ascii_lowercase();
        let Some(sfx) = watch::PARTIAL_SUFFIXES.iter().find(|x| lc.ends_with(*x)) else { continue };
        if s.modified + 2 < since {
            continue;
        }
        let shown = name[..name.len() - sfx.len()].to_string();
        // Chromium names unfinished files "Unconfirmed 123456.crdownload" until the name is known.
        let shown = if shown.starts_with("Unconfirmed ") { "a file".to_string() } else { shown };
        if best.as_ref().map(|b| s.modified > b.0).unwrap_or(true) {
            best = Some((s.modified, shown, s.size));
        }
    }
    best.map(|(_, n, s)| (n, s))
}

/// The browser check's JSON plus the warning's words (title, text, steps), so the interface has one source.
pub fn browser_json(check: &str) -> String {
    let steps: Vec<String> = browser::INSTALL_STEPS.iter().map(|s| quote(s)).collect();
    format!(
        "{{\"checked\":true,\"result\":{check},\"title\":{},\"text\":{},\"steps\":[{}]}}",
        quote(browser::NO_BROWSER_TITLE),
        quote(browser::NO_BROWSER_TEXT),
        steps.join(",")
    )
}

/// Copies the helper's files (plain files only, tests left out) into `to`. Returns how many were copied.
fn copy_helper(from: &Path, to: &Path) -> Result<usize, String> {
    if !from.join("manifest.json").is_file() {
        return Err("this build has no browser helper in it".into());
    }
    fn walk(from: &Path, to: &Path, depth: usize, n: &mut usize) -> Result<(), String> {
        if depth > 6 {
            return Ok(());
        }
        std::fs::create_dir_all(to).map_err(|e| e.to_string())?;
        let rd = std::fs::read_dir(from).map_err(|e| e.to_string())?;
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || (depth == 0 && name == "test") {
                continue;
            }
            let Ok(m) = std::fs::symlink_metadata(e.path()) else { continue };
            if m.is_dir() {
                walk(&e.path(), &to.join(&name), depth + 1, n)?;
            } else if m.is_file() && m.len() <= 8 << 20 {
                std::fs::copy(e.path(), to.join(&name)).map_err(|err| format!("{name}: {err}"))?;
                *n += 1;
            }
        }
        Ok(())
    }
    let mut n = 0;
    walk(from, to, 0, &mut n)?;
    Ok(n)
}

extern "C" {
    // qtglue.cpp
    fn turbine_overlay_mark(on: i32) -> i32;
}

/// Starts a choice from the extension on a fresh session only when it names the installer on screen; used by tests
/// of the id format the extension sends back.
pub fn option_id(group: usize, option: usize) -> String {
    format!("g{group}o{option}")
}

#[cfg(test)]
mod tests {
    #[test]
    fn option_ids_round_trip() {
        assert_eq!(super::option_id(0, 2), "g0o2");
    }
}
