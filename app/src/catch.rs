//! Catching mods the player downloads in their own browser, and saying how it goes:
//! - the Downloads watcher: while Turbine runs it watches Downloads for every new zip, 7z or rar archive, however the
//!   player got to the download (and catches up at start on ones that arrived while it was closed). The game is
//!   decided in this order: the browser helper's page, Nexus Mods by the file's MD5 (with a key), a Nexus page opened
//!   from Turbine whose files it matches, then the game last open in Turbine. Sure answers add the file by itself;
//!   otherwise one question: "Found <file>. Add it to <game>?" (A add, X another game, B not a mod, never asked
//!   again). The activity log says each step: folders watched, files seen, why each was left alone or matched;
//! - the browser helper (bridge.rs): downloads the extension reports, its Mod Manager Download links, and installer
//!   choices made on the Nexus page;
//! - the status both show: `statusJson` for the overlay (over the browser, Desktop and Game Mode) and "status",
//!   "added" and "choices" messages for the helper's panel on the page;
//! - the browser check, the helper's folder in Downloads, and the Game Mode overlay's window property.
//!
//! Files in Downloads are never moved, changed or deleted: Turbine only reads them (the player's own downloads).

use crate::actions::set;
use crate::backend::{qobject, Worker};
use crate::now_secs;
use crate::bridge;
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use turbine_engine::json::{quote, Json};
use turbine_engine::net::SourceInfo;
use turbine_engine::watch::{self, Arrival, Basis, CatchState, Catcher, Confidence, Event, Expectation};
use turbine_engine::{browser, nexus, Game};

/// Where Turbine's Flatpak keeps the browser helper (installed from the repository's extension folder).
pub const HELPER_SOURCE: &str = "/app/share/turbine-mod-manager/extension";
/// The folder in Downloads the helper is copied to, for Load unpacked.
pub const HELPER_FOLDER: &str = "Turbine browser helper";
/// What the status says while a Nexus file page is open for a free account.
pub const SLOW_DOWNLOAD: &str =
    "Sign in to Nexus if it asks (free), then press Slow download on this page. Turbine adds the mod automatically when the file lands.";
/// The most Nexus pages remembered at once (each one for 30 minutes).
const MAX_WATCHES: usize = 8;
/// How long a browser's report of a download holds the Downloads watcher back once the file is ready: past this,
/// the watcher adds it itself (otherwise it could wait for a finish that never comes).
pub const DEFER: Duration = Duration::from_secs(20);

/// The folders Turbine watches and accepts helper downloads from: ~/Downloads and the desktop's Downloads folder.
/// Steam's own browser saves where it is told; where that is in Game Mode isn't known yet (see Labs, Steam's
/// built-in browser).
pub fn watch_folders() -> Vec<PathBuf> {
    watch::with_game_mods(&watch::default_folders(&turbine_engine::home_dir()))
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
        title: None,
        picture: None,
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

/// A finished download Turbine isn't sure about, waiting for the player's answer.
#[derive(Clone, Debug)]
pub struct Ask {
    pub path: PathBuf,
    pub size: u64,
    /// The best guess (None when no game fits at all).
    pub game: Option<Game>,
    /// What is known about it for that game: a Nexus page it matched, or Nexus's answer by MD5.
    pub hint: Option<(bridge::NexusRef, Option<String>)>,
    pub source: Option<SourceInfo>,
    /// Why that game, in words.
    pub why: String,
}

/// A Nexus page opened from Turbine that a download matched.
#[derive(Clone, Debug)]
pub struct PageMatch {
    pub game: Game,
    pub confidence: Confidence,
    pub r: bridge::NexusRef,
    pub version: Option<String>,
    pub started: u64,
}

fn ago(secs: u64) -> String {
    match secs {
        0..=119 => format!("{secs} seconds ago"),
        120..=7199 => format!("{} minutes ago", secs / 60),
        7200..=172_799 => format!("{} hours ago", secs / 3600),
        _ => format!("{} days ago", secs / 86_400),
    }
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
    /// Nexus mod pages opened from Turbine: what each one's download looks like (for 30 minutes).
    pub expects: Vec<Expectation>,
    /// Files already added (or being added), so the watcher and the helper never add the same one twice.
    pub handled: std::collections::VecDeque<PathBuf>,
    pub asks: Vec<Ask>,
    pub downloads: Vec<(String, HelperDownload)>,
    /// The Nexus mod page the helper last reported.
    pub page: Option<bridge::NexusRef>,
    /// Is the status a caught download's (so finishing its installer reports "Added")?
    pub busy: bool,
    /// Kept between runs: when Downloads was last looked at, ignored files, unanswered questions.
    pub state: CatchState,
    /// Where `state` is saved (None until watching starts, and always in the build's screenshot checks).
    pub state_path: Option<PathBuf>,
    /// This is Turbine's first run (its catch-up files are asked about, never added by themselves).
    pub first_run: bool,
    /// The unfinished download the watcher last saw (its name), for the status line.
    pub partial: String,
    /// Since when the same unfinished download has been seen without growing (a Save box waiting, after 3 s).
    pub partial_since: Option<std::time::Instant>,
    pub partial_size: u64,
    /// Downloads the browser helper reported with a known game: (file name, game id, Nexus page).
    pub helper_games: std::collections::VecDeque<(String, String, Option<bridge::NexusRef>)>,
    /// Finished files the watcher saw while the browser helper was still reporting them: held back until the helper
    /// says it finished, or until the deadline, then decided the usual way.
    pub deferred: Vec<(Arrival, std::time::Instant)>,
    /// The status text last written to the log (each new one is logged once).
    pub said: String,
    /// The browser saved a download outside Downloads, and the player was told (once a run).
    pub told_outside: bool,
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

/// The helper's "status" message (its panel on the page shows it).
fn status_message(text: &str, level: &str, progress: Option<f64>, game: &str) -> String {
    let p = progress.map(|p| format!("{:.3}", p.clamp(0.0, 1.0))).unwrap_or_else(|| "null".into());
    let g = if game.is_empty() { "null".to_string() } else { quote(game) };
    format!("{{\"type\":\"status\",\"text\":{},\"level\":{},\"progress\":{p},\"game\":{g}}}", quote(text), quote(level))
}

fn games_json(games: &[Game]) -> String {
    let items: Vec<String> = games
        .iter()
        .filter_map(|g| {
            let d = nexus::domain_for(g.store, &g.store_id)?;
            Some(format!(
                "{{\"id\":{},\"name\":{},\"domain\":{},\"folder\":{}}}",
                quote(&g.id()),
                quote(&g.name),
                quote(d),
                quote(&watch::game_folder_name(&g.name))
            ))
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
        let fits = |g: &Game| nexus::domain_for(g.store, &g.store_id) == Some(domain);
        self.current_game().filter(|g| fits(g)).or_else(|| self.rust().found.iter().find(|g| fits(g)).cloned())
    }

    /// Sets the overlay's status and tells the browser helper.
    pub(crate) fn set_status(mut self: Pin<&mut Self>, text: &str, level: &str, progress: Option<f64>, game: &str, added: &str, done: bool, ask: &str) {
        let j = status_text(true, text, level, progress, game, added, done, ask);
        // A hint ("press Slow download") never counts as busy: any Downloading or Adding status replaces it.
        self.as_mut().rust_mut().catching.busy = !done && level != "hint";
        if self.rust().catching.said != text {
            crate::log::line("catch", &format!("Status ({level}): {text}"));
            self.as_mut().rust_mut().catching.said = text.to_string();
        }
        self.as_mut().set_status_json(QString::from(j.as_str()));
        bridge::broadcast(&status_message(text, level, progress, game));
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
        let _ = bridge::BUNDLED.set(manifest_version(Path::new(HELPER_SOURCE)).unwrap_or_default());
        let _ = bridge::BEFORE_RELOAD.set(refresh_helper_copy);
        let _ = refresh_helper_copy();
        let qt = self.qt_thread();
        let handler: bridge::Handler = Arc::new(move |kind: &str, msg: &Json| {
            let (kind, msg) = (kind.to_string(), msg.clone());
            set(&qt, move |t| t.bridge_message(&kind, &msg));
        });
        let dir = crate::log::folder(&crate::backend::data_home());
        match bridge::start(&dir, |l| crate::log::line("bridge", l), handler) {
            Ok(port) => crate::log::line("bridge", &format!("Browser helper link on 127.0.0.1:{port}")),
            Err(e) => crate::log::line("bridge", &format!("Browser helper link not started: {e}")),
        }
        bridge::set_games(games_json(&self.rust().found));
        self.as_mut().bridge_refresh();
        self.as_mut().browser_check();
        self.start_downloads_watch();
    }

    /// Looks for a web browser on the host (in the background); the answer lands in `browserJson`.
    pub fn browser_check(self: Pin<&mut Self>) {
        let qt = self.qt_thread();
        let _ = std::thread::Builder::new().name("turbine-browser-check".into()).spawn(move || {
            let c = browser::check();
            crate::log::line("browser", &format!("Browser: {}", if c.found { format!("{} ({})", c.name, c.how) } else { "none found".into() }));
            let j = browser_json(&c.to_json());
            set(&qt, move |t| t.set_browser_json(QString::from(j.as_str())));
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
        let to = helper_folder();
        let r = bridge::copy_helper(Path::new(HELPER_SOURCE), &to);
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

    // ------------------------------------------------------------------------------------------------ Downloads

    /// Starts watching Downloads for every new mod archive, for as long as Turbine runs (once, at start; never in
    /// the build's screenshot checks). Archives that arrived since Turbine last looked are caught up on first; on the
    /// very first run, the last 24 hours count and Turbine asks before adding any of those.
    pub(crate) fn start_downloads_watch(mut self: Pin<&mut Self>) {
        let path = crate::log::folder(&crate::backend::data_home()).join("catch.json");
        // Game Mods folders named with a long dash get their plain name, before the
        // watcher looks at them.
        for l in watch::rename_dash_folders(&watch::default_folders(&turbine_engine::home_dir()), Some(&path)) {
            crate::log::line("catch", &l);
        }
        let saved = CatchState::load(&path);
        let now = now_secs();
        let (since, first) = watch::catch_up_since(saved.as_ref(), now);
        let state = saved.unwrap_or_default();
        let folders = watch_folders();
        for f in &folders {
            let there = f.is_dir();
            crate::log::line("catch", &format!("Watching {} for new mod archives (zip, 7z, rar){}", f.display(), if there { "" } else { "; it isn't there yet" }));
        }
        if first {
            crate::log::line("catch", "First run: archives from the last 24 hours count, and Turbine asks before adding any of them");
        } else {
            crate::log::line("catch", &format!("Catching up on archives that arrived since Turbine last looked ({})", ago(now.saturating_sub(since))));
        }
        let mut c = Catcher::new(folders, since, now);
        for k in &state.ignored {
            c.ignore(k);
        }
        for p in &state.pending {
            c.ask_again(Path::new(p));
        }
        if !state.pending.is_empty() {
            crate::log::line("catch", &format!("{} found {} still waiting for an answer from last time", state.pending.len(), if state.pending.len() == 1 { "file" } else { "files" }));
        }
        {
            let mut rm = self.as_mut().rust_mut();
            let k = &mut rm.catching;
            k.state = state;
            k.state_path = Some(path);
            k.first_run = first;
        }
        let qt = self.qt_thread();
        let _ = std::thread::Builder::new().name("turbine-downloads".into()).spawn(move || {
            let mut looks: u64 = 0;
            let mut had_partial = false;
            loop {
                // With the browser helper connected, its own report of a finished download comes first.
                c.stable_looks = if bridge::connected() { 3 } else { 1 };
                // A new game's Game Mods folder is picked up within a few seconds.
                if looks.is_multiple_of(5) {
                    let now_folders = watch_folders();
                    if now_folders != c.folders {
                        let added: Vec<String> = now_folders.iter().filter(|f| !c.folders.contains(f)).map(|f| f.display().to_string()).collect();
                        if !added.is_empty() {
                            crate::log::line("catch", &format!("Watching {} too", added.join(", ")));
                        }
                        c.folders = now_folders;
                    }
                }
                let events = c.poll();
                let partial = events.iter().any(|e| matches!(e, Event::Downloading { .. }));
                if !events.is_empty() || had_partial {
                    set(&qt, move |t| t.catch_events(events));
                }
                had_partial = partial;
                looks += 1;
                if looks % 30 == 5 {
                    set(&qt, |t| t.catch_save(false));
                }
                // Files held back for a browser's report whose finish never came.
                set(&qt, |t| t.catch_deferred());
                std::thread::sleep(Duration::from_secs(watch::POLL_SECS));
            }
        });
    }

    /// Writes what Turbine keeps about catching (when it last looked, ignored files, unanswered questions).
    /// `decided`: right after a file was dealt with, so it counts as seen; otherwise a few seconds are left open for
    /// files still settling.
    fn catch_save(mut self: Pin<&mut Self>, decided: bool) {
        let now = now_secs();
        let Some(path) = self.rust().catching.state_path.clone() else { return };
        let state = {
            let mut rm = self.as_mut().rust_mut();
            let k = &mut rm.catching;
            let t = if decided { now } else { now.saturating_sub(5) };
            k.state.last_seen = k.state.last_seen.max(t);
            k.state.pending = k.asks.iter().map(|a| a.path.to_string_lossy().into_owned()).collect();
            k.state.clone()
        };
        if let Err(e) = state.save(&path) {
            crate::log::line("catch", &format!("Couldn't save {}: {e}", path.display()));
        }
    }

    /// What the Downloads watcher saw in one look.
    pub(crate) fn catch_events(mut self: Pin<&mut Self>, events: Vec<Event>) {
        let mut partial = None;
        for e in events {
            match e {
                Event::Skipped { path, why } => crate::log::line("catch", &format!("Downloads: {} left alone: {}", label_of(&path), why.words())),
                Event::Downloading { name, size } => {
                    if self.rust().catching.partial_size != size {
                        let mut rm = self.as_mut().rust_mut();
                        rm.catching.partial_size = size;
                        rm.catching.partial_since = Some(std::time::Instant::now());
                    }
                    partial = Some(name)
                }
                Event::Ready(a) => self.as_mut().catch_arrival(a),
            }
        }
        match partial {
            Some(name) => {
                if self.rust().catching.partial != name {
                    crate::log::line("catch", &format!("Downloads: {name} is downloading"));
                    self.as_mut().rust_mut().catching.partial = name.clone();
                }
                // The browser helper reports downloads itself, with real progress. Without it there is no percent: the
                // browser writes a temporary file, and its Save box may still be waiting.
                if !bridge::connected() {
                    let waiting = self.rust().catching.partial_since.is_some_and(|t| t.elapsed() >= Duration::from_secs(3));
                    let text = if waiting { "Press Save in the browser".to_string() } else { format!("Downloading in the browser: {name}") };
                    let status = self.status_json().to_string();
                    if (!self.rust().catching.busy || status.contains("Downloading") || status.contains("Press Save")) && !status.contains(&quote(&text)) {
                        self.as_mut().set_status(&text, if waiting { "wait" } else { "busy" }, None, "", "", false, "");
                    }
                }
            }
            None => {
                if !self.rust().catching.partial.is_empty() {
                    self.as_mut().rust_mut().catching.partial.clear();
                    self.as_mut().rust_mut().catching.partial_since = None;
                    // The download ended without a mod to add (cancelled, or not an archive): no "Downloading" left.
                    let st = self.status_json().to_string();
                    if st.contains("\"text\":\"Downloading") || st.contains("\"text\":\"Press Save") {
                        self.status_done();
                    }
                }
            }
        }
    }

    /// A finished archive arrived in Downloads: decide which game it goes into (in the background when Nexus is
    /// asked about it), then add it or ask.
    pub(crate) fn catch_arrival(mut self: Pin<&mut Self>, a: Arrival) {
        let name = label_of(&a.path);
        crate::log::line(
            "catch",
            &format!("Downloads: {name} finished ({}, {})", crate::browse::size_words(a.size), if a.catch_up { "arrived while Turbine was closed" } else { "arrived just now" }),
        );
        if self.handled(&a.path) {
            crate::log::line("catch", &format!("{name} was already added"));
            return;
        }
        if self.rust().catching.state.ignored.contains(&watch::ignore_key(&a.path, a.size)) {
            crate::log::line("catch", &format!("{name}: you said it isn't a mod"));
            return;
        }
        if self.rust().catching.asks.iter().any(|x| x.path == a.path) {
            return;
        }
        if self.rust().catching.downloads.iter().any(|(_, d)| d.label == name) {
            if self.rust().catching.deferred.iter().any(|(x, _)| x.path == a.path) {
                return;
            }
            crate::log::line(
                "catch",
                &format!("{name}: the browser helper is reporting this download; it adds it when it finishes (Turbine adds it itself after {} seconds otherwise)", DEFER.as_secs()),
            );
            let until = std::time::Instant::now() + DEFER;
            self.as_mut().rust_mut().catching.deferred.push((a, until));
            return;
        }
        // 0. Saved into Downloads/Game Mods/<game> (by the browser helper): a sure match for that game.
        if let Some(folder) = watch::game_mods_folder_of(&a.path) {
            let game = self.rust().found.iter().find(|g| watch::game_folder_name(&g.name) == folder).cloned();
            if let Some(g) = game {
                let d = watch::decide_all(Some(&g.id()), None, None, None, None, false);
                crate::log::line("catch", &format!("{name}: {}, because {}; adding it", g.name, d.basis.words()));
                // The Nexus page the helper reported for this file comes first (Nexus's newer file
                // names carry no ids); then the mod ID and version from the older file name pattern, when it has one.
                let domain = nexus::domain_for(g.store, &g.store_id).unwrap_or("").to_string();
                let reported = self.rust().catching.helper_games.iter().rev().find(|(l, _, _)| *l == name).and_then(|(_, _, r)| r.clone());
                let hint = reported.map(|r| (r, None)).or_else(|| {
                    turbine_engine::store::nexus_ids_from_name(&name)
                        .filter(|_| !domain.is_empty())
                        .map(|(m, v)| (bridge::NexusRef { domain: domain.clone(), mod_id: m, file_id: None }, Some(v)))
                });
                self.as_mut().catch_add(g, a.path, hint, None);
                self.catch_save(true);
                return;
            }
            crate::log::line("catch", &format!("{name} is in Game Mods/{folder}, but no installed game has that name; deciding the usual way"));
        }
        // 1. The browser helper saw the Nexus page it came from.
        let helper = self
            .rust()
            .catching
            .helper_games
            .iter()
            .rev()
            .find(|(l, _, _)| *l == name)
            .and_then(|(_, g, r)| self.find(g).map(|g| (g, r.clone())));
        // 3. A Nexus page opened from Turbine whose files it matches (name, then size; a guess when its list isn't
        //    known), only for files that arrived after the page was opened.
        let now = now_secs();
        let mut page: Option<PageMatch> = None;
        for e in self.rust().catching.expects.iter().filter(|e| !e.expired_at(now)) {
            if a.arrived + 2 < e.started_at {
                continue;
            }
            let Some((file, confidence)) = e.match_file(&name, a.size) else { continue };
            if page.as_ref().map(|p| confidence > p.confidence || (confidence == p.confidence && e.started_at > p.started)).unwrap_or(true) && let Some(game) = self.find(&e.game_id) {
                let r = bridge::NexusRef { domain: e.domain.clone(), mod_id: e.mod_id, file_id: file.as_ref().map(|f| f.file_id).filter(|x| *x > 0) };
                page = Some(PageMatch { game, confidence, r, version: file.map(|f| f.version), started: e.started_at });
            }
        }
        if helper.is_some() || crate::online::nexus_key().is_none() {
            return self.catch_decide(a, helper, None, page);
        }
        // 2. Nexus Mods by the file's MD5, for each of the installed games it has (the likeliest first).
        let mut domains: Vec<(String, Game)> = Vec::new();
        let mut add = |g: &Game| {
            if let Some(d) = nexus::domain_for(g.store, &g.store_id) && !domains.iter().any(|(x, _)| x == d) {
                domains.push((d.to_string(), g.clone()));
            }
        };
        if let Some(p) = &page {
            add(&p.game);
        }
        if let Some(g) = self.current_game() {
            add(&g);
        }
        for g in &self.rust().found {
            add(g);
        }
        domains.truncate(6);
        crate::log::line("catch", &format!("Looking up {name} on Nexus Mods by its MD5"));
        let qt = self.qt_thread();
        let _ = std::thread::Builder::new().name("turbine-identify".into()).spawn(move || {
            let md5 = domains.into_iter().find_map(|(d, g)| identify(&d, &a.path).map(|s| (g, s)));
            set(&qt, move |t| t.catch_decide(a, None, md5, page));
        });
    }

    /// Every second: a file held back for the browser helper's report past its deadline is decided the usual way.
    pub(crate) fn catch_deferred(mut self: Pin<&mut Self>) {
        if self.rust().catching.deferred.is_empty() {
            return;
        }
        let now = std::time::Instant::now();
        let due: Vec<Arrival> = {
            let mut rm = self.as_mut().rust_mut();
            let d = &mut rm.catching.deferred;
            let (due, keep): (Vec<_>, Vec<_>) = std::mem::take(d).into_iter().partition(|(_, until)| *until <= now);
            *d = keep;
            due.into_iter().map(|(a, _)| a).collect()
        };
        for a in due {
            let name = label_of(&a.path);
            if self.handled(&a.path) {
                continue;
            }
            crate::log::line("catch", &format!("The browser didn't report {name} finished; adding it from Downloads"));
            self.as_mut().undefer_now(a);
        }
    }

    /// Decides a held-back file now: the helper's report of it is dropped (its game, when known, still counts).
    fn undefer_now(mut self: Pin<&mut Self>, a: Arrival) {
        let name = label_of(&a.path);
        {
            let mut rm = self.as_mut().rust_mut();
            let c = &mut rm.catching;
            c.deferred.retain(|(x, _)| x.path != a.path);
            c.downloads.retain(|(_, d)| d.label != name);
        }
        self.catch_arrival(a);
    }

    /// The helper said a download finished but Turbine couldn't take its report (or never saw it start): the
    /// watcher's copy of that file is decided the usual way, now if the watcher has it, otherwise when it does.
    fn fallback_to_watcher(mut self: Pin<&mut Self>, label: &str, path: Option<PathBuf>) {
        let held = self.rust().catching.deferred.iter().find(|(x, _)| label_of(&x.path) == label || path.as_ref().is_some_and(|p| *p == x.path)).map(|(x, _)| x.clone());
        self.as_mut().rust_mut().catching.downloads.retain(|(_, d)| d.label != label);
        match held {
            Some(a) => {
                crate::log::line("catch", &format!("{label}: deciding the watcher's copy the usual way"));
                self.undefer_now(a);
            }
            None => match path.filter(|p| p.is_file()) {
                // Already there and in a watched folder: decided the usual way now.
                Some(p) if accept_download(&p.to_string_lossy()).is_ok() => {
                    let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
                    crate::log::line("catch", &format!("{label}: deciding it the usual way"));
                    self.catch_arrival(Arrival { path: p, size, arrived: now_secs(), catch_up: false });
                }
                _ => crate::log::line("catch", &format!("{label}: the Downloads watcher adds it when it sees it")),
            },
        }
    }

    /// Picks the game in order (the helper, Nexus by MD5, a matching Nexus page, the game last open in Turbine), then
    /// adds the file when Turbine is sure, or asks.
    fn catch_decide(mut self: Pin<&mut Self>, a: Arrival, helper: Option<(Game, Option<bridge::NexusRef>)>, md5: Option<(Game, SourceInfo)>, page: Option<PageMatch>) {
        let name = label_of(&a.path);
        if self.handled(&a.path) || self.rust().catching.asks.iter().any(|x| x.path == a.path) {
            return;
        }
        let last = self.current_game();
        let first_catch_up = self.rust().catching.first_run && a.catch_up;
        let (hid, mid, pid, lid) = (
            helper.as_ref().map(|(g, _)| g.id()),
            md5.as_ref().map(|(g, _)| g.id()),
            page.as_ref().map(|p| (p.game.id(), p.confidence)),
            last.as_ref().map(Game::id),
        );
        let d = watch::decide(hid.as_deref(), mid.as_deref(), pid.as_ref().map(|(g, c)| (g.as_str(), *c)), lid.as_deref(), first_catch_up);
        let game = d.game.as_deref().and_then(|id| self.find(id));
        let gname = game.as_ref().map(|g| g.name.clone()).unwrap_or_else(|| "no game".into());
        crate::log::line(
            "catch",
            &format!(
                "{name}: {gname}, because {}; {}",
                d.basis.words(),
                if d.sure && game.is_some() { "adding it" } else if first_catch_up { "asking first (first run)" } else { "asking" }
            ),
        );
        // What is known about where it came from, for the mod's name, version and update checks.
        let (hint, source) = match d.basis {
            Basis::Helper => (helper.and_then(|(_, r)| r).map(|r| (r, None)), None),
            Basis::Md5 => (None, md5.map(|(_, s)| s)),
            Basis::Page(_) => (page.map(|p| (p.r, p.version)), None),
            _ => (None, None),
        };
        match game {
            Some(g) if d.sure => {
                self.as_mut().catch_add(g, a.path, hint, source);
                self.catch_save(true);
            }
            game => {
                if game.is_none() && self.rust().found.is_empty() {
                    crate::log::line("catch", &format!("{name}: no games found here, so there is nowhere to add it"));
                    return;
                }
                let ask = Ask { path: a.path, size: a.size, game, hint, source, why: d.basis.words().to_string() };
                self.as_mut().rust_mut().catching.asks.push(ask);
                self.as_mut().catch_save(true);
                // The first question shows; with more waiting, it says how many.
                self.show_ask();
            }
        }
    }

    /// Shows the first waiting question in Turbine (the "found" sheet) and on the status card over other apps.
    fn show_ask(mut self: Pin<&mut Self>) {
        let Some(ask) = self.rust().catching.asks.first().cloned() else { return };
        let more = self.rust().catching.asks.len() - 1;
        let name = label_of(&ask.path);
        let p = ask.path.to_string_lossy().into_owned();
        let (gid, gname) = ask.game.as_ref().map(|g| (g.id(), g.name.clone())).unwrap_or_default();
        let text = if gname.is_empty() { format!("Found {name}. Which game is it for?") } else { format!("Found {name}. Add it to {gname}?") };
        self.as_mut().set_status(&text, "info", None, &gname, "", false, &p);
        let j = format!(
            "{{\"path\":{},\"name\":{},\"size\":{},\"game\":{},\"gameName\":{},\"why\":{},\"more\":{more}}}",
            quote(&p),
            quote(&name),
            ask.size,
            quote(&gid),
            quote(&gname),
            quote(&ask.why)
        );
        self.asked(QString::from(j.as_str()));
    }

    /// The helper's report of this download, taken off the list.
    fn take_download(mut self: Pin<&mut Self>, id: &str) -> Option<HelperDownload> {
        let mut rm = self.as_mut().rust_mut();
        let downloads = &mut rm.catching.downloads;
        let i = downloads.iter().position(|(x, _)| x == id)?;
        Some(downloads.remove(i).1)
    }

    fn take_ask(mut self: Pin<&mut Self>, path: &Path) -> Option<Ask> {
        let mut rm = self.as_mut().rust_mut();
        let asks = &mut rm.catching.asks;
        let i = asks.iter().position(|a| a.path == path)?;
        Some(asks.remove(i))
    }

    /// After an answer: the next question, or the question goes away (in Turbine and on the card).
    fn next_ask(mut self: Pin<&mut Self>) {
        self.as_mut().catch_save(true);
        if self.rust().catching.asks.is_empty() {
            self.as_mut().asked(QString::from("{}"));
            if !self.rust().catching.busy || self.status_json().to_string().contains("\"ask\":\"/") {
                self.status_done();
            }
        } else {
            self.show_ask();
        }
    }

    /// The player added a file by hand (Add from Downloads): the watcher leaves it alone, and any question about it
    /// goes away.
    pub(crate) fn hand_added(mut self: Pin<&mut Self>, path: &Path) {
        self.as_mut().mark_handled(path);
        if self.as_mut().take_ask(path).is_some() {
            crate::log::line("catch", &format!("{} was added by hand; not asking about it", label_of(path)));
            self.next_ask();
        }
    }

    // ------------------------------------------------------------------------------------------------ Nexus pages

    /// Opening a Nexus mod page: a download that matches it goes into its game by itself (Downloads is watched all
    /// the time anyway). Answers {"watching", "game", "error"}.
    pub fn watch_page(mut self: Pin<&mut Self>, url: &QString) -> QString {
        let url = url.to_string();
        let out = |watching: bool, game: &str, error: &str| {
            QString::from(format!("{{\"watching\":{watching},\"game\":{},\"error\":{}}}", quote(game), quote(error)).as_str())
        };
        let Some(r) = bridge::nexus_page(&url) else { return out(false, "", "") };
        let Some(game) = self.game_for_domain(&r.domain) else {
            crate::log::line("catch", &format!("Nexus page for {} mod {}: that game isn't installed here", r.domain, r.mod_id));
            return out(false, "", "That game isn't installed here.");
        };
        let name = game.name.clone();
        let mod_page = r.mod_id != 0;
        self.as_mut().start_watch(game, r, None);
        // A free account presses Slow download on the page itself (Turbine never presses anything there): the status
        // in Turbine, over the browser and in the helper's panel says so until the download starts.
        let premium = turbine_engine::json::parse(&self.account_json().to_string()).ok().and_then(|a| a.get("premium").and_then(Json::as_bool)).unwrap_or(false);
        if mod_page && !premium {
            // A hint, not a wait: it never holds back the watcher's own "Downloading".
            crate::log::line("catch", "Status: press Slow download on the page (a free account downloads there)");
            self.as_mut().set_status(SLOW_DOWNLOAD, "hint", None, &name, "", false, "");
        }
        out(true, &name, "")
    }

    /// Labs, Steam's built-in browser: downloads from now on are guessed to be for the game on screen. Answers the
    /// steam:// address.
    pub fn watch_steam_browser(mut self: Pin<&mut Self>) -> QString {
        if let Some(game) = self.current_game() {
            let domain = nexus::domain_for(game.store, &game.store_id).unwrap_or("").to_string();
            let r = bridge::NexusRef { domain, mod_id: 0, file_id: None };
            self.as_mut().start_watch(game, r, Some(nexus::FileList::default()));
        }
        QString::from("steam://openurl/https://www.nexusmods.com")
    }

    /// Remembers what a Nexus page's download looks like. `files`: the mod's file list when already known;
    /// otherwise it is read from Nexus in the background when the player has a key.
    fn start_watch(mut self: Pin<&mut Self>, game: Game, r: bridge::NexusRef, files: Option<nexus::FileList>) {
        let started = now_secs();
        let mut exp = Expectation::new(&game.id(), &r.domain, r.mod_id, started);
        if let Some(l) = &files {
            exp = exp.with_files(l);
        }
        crate::log::line("catch", &format!("Nexus page for {} mod {} opened: a matching download goes into {}", r.domain, r.mod_id, game.name));
        self.as_mut().expect_page(exp.clone());
        if files.is_some() || r.mod_id == 0 {
            return;
        }
        if crate::online::using_samples() {
            return;
        }
        // With a key through v1; without one through GraphQL (no key needed).
        let k = crate::online::nexus_key();
        let qt = self.qt_thread();
        let _ = std::thread::Builder::new().name("turbine-files".into()).spawn(move || {
            let client = turbine_engine::net::Client::new(crate::VERSION, &crate::online::temp_dir());
            match nexus::Nexus::with_key(client, k.as_deref(), crate::VERSION).files_any(&r.domain, r.mod_id, None) {
                Ok(l) => {
                    let exp = exp.with_files(&l);
                    crate::log::line("catch", &format!("Nexus lists {} files for {}:{}; matching by name and size", exp.files.len(), r.domain, r.mod_id));
                    set(&qt, move |t| t.expect_page(exp));
                }
                Err(e) => crate::log::line("catch", &format!("Nexus's file list for {}:{} couldn't be read ({e}); Turbine will ask about its download", r.domain, r.mod_id)),
            }
        });
    }

    fn expect_page(mut self: Pin<&mut Self>, exp: Expectation) {
        let now = now_secs();
        let mut rm = self.as_mut().rust_mut();
        let list = &mut rm.catching.expects;
        list.retain(|e| !(e.domain == exp.domain && e.mod_id == exp.mod_id && e.game_id == exp.game_id) && !e.expired_at(now));
        // Only one page guesses at a time: the newest.
        if !exp.knows_files() {
            list.retain(|e| e.knows_files());
        }
        list.push(exp);
        while list.len() > MAX_WATCHES {
            list.remove(0);
        }
    }

    pub(crate) fn handled(&self, path: &Path) -> bool {
        let real = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        self.rust().catching.handled.contains(&real)
    }

    pub(crate) fn mark_handled(mut self: Pin<&mut Self>, path: &Path) {
        let real = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let mut rm = self.as_mut().rust_mut();
        let h = &mut rm.catching.handled;
        if h.len() >= 100 {
            h.pop_front();
        }
        h.push_back(real);
    }

    /// The opposite: the file may be decided again (Turbine's browser marks its downloads at accept, before they
    /// exist, and lets go of the mark to add them).
    pub(crate) fn unmark_handled(mut self: Pin<&mut Self>, path: &Path) {
        let real = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let raw = path.to_path_buf();
        self.as_mut().rust_mut().catching.handled.retain(|p| *p != real && *p != raw);
    }

    /// The player's answer to "Found <file>. Add it to <game>?": yes adds it; no means it isn't a mod, and Turbine
    /// never asks about that file again.
    pub fn watch_confirm(mut self: Pin<&mut Self>, path: &QString, yes: bool) {
        let path = PathBuf::from(path.to_string());
        let Some(ask) = self.as_mut().take_ask(&path) else { return };
        let name = label_of(&path);
        match (yes, ask.game.clone()) {
            (true, Some(game)) => {
                crate::log::line("action", &format!("Add {name} to {}", game.name));
                self.as_mut().catch_add(game, ask.path, ask.hint, ask.source);
            }
            _ => {
                crate::log::line("action", &format!("Not a mod: {name} (won't be asked about again)"));
                let key = watch::ignore_key(&ask.path, ask.size);
                self.as_mut().rust_mut().catching.state.ignore(&key);
            }
        }
        self.next_ask();
    }

    /// The player picked another game for a found file.
    pub fn catch_pick(mut self: Pin<&mut Self>, path: &QString, game_id: &QString) {
        let path = PathBuf::from(path.to_string());
        let Some(game) = self.find(&game_id.to_string()) else { return };
        let Some(ask) = self.as_mut().take_ask(&path) else { return };
        crate::log::line("action", &format!("Add {} to {} (picked)", label_of(&path), game.name));
        // What Nexus said about it only holds for the game it said.
        let same = ask.game.as_ref().map(|g| g.id() == game.id()).unwrap_or(false);
        let (hint, source) = if same { (ask.hint, ask.source) } else { (None, None) };
        self.as_mut().catch_add(game, ask.path, hint, source);
        self.next_ask();
    }

    /// Adds a caught download to its game: identified on Nexus first when the player has a key (unless `known`
    /// already says what it is), then added on the mod worker; the status says how it went.
    pub(crate) fn catch_add(mut self: Pin<&mut Self>, game: Game, path: PathBuf, hint: Option<(bridge::NexusRef, Option<String>)>, known: Option<SourceInfo>) {
        if self.handled(&path) {
            return;
        }
        self.as_mut().mark_handled(&path);
        let label = label_of(&path);
        self.as_mut().set_status(&format!("Adding {label} to {}", game.name), "busy", None, &game.name, "", false, "");
        self.as_mut().edited();
        let qt = self.qt_thread();
        let domain = hint.as_ref().map(|(r, _)| r.domain.clone()).or_else(|| nexus::domain_for(game.store, &game.store_id).map(str::to_string));
        let _ = std::thread::Builder::new().name("turbine-identify".into()).spawn(move || {
            let found = known.or_else(|| domain.as_deref().filter(|d| !d.is_empty()).and_then(|d| identify(d, &path)));
            let mut source = found
                .or_else(|| hint.as_ref().map(|(r, v)| hint_source(r, v.clone())))
                .unwrap_or_else(|| SourceInfo { kind: "zip".into(), ..Default::default() });
            // What it needs, from its Nexus page (no key needed), for "Needs" and "Needed by" in the mod menu.
            if source.needs.is_empty() && let Some((d, m)) = nexus::source_mod(&source).filter(|(d, _)| !d.is_empty()) {
                source.needs = crate::online::nexus_needs(&d, m);
            }
            set(&qt, move |mut t| {
                let name = format!("Add {label} (caught download)");
                // A free account's Update: this download of the mod's page takes the installed mod's place (keeping
                // its place in the list, on or off, and installer choices).
                let replaces = nexus::source_mod(&source).and_then(|(_, m)| {
                    let mut r = t.as_mut().rust_mut();
                    let i = r.update_waits.iter().position(|(x, _)| *x == m)?;
                    Some(r.update_waits.remove(i).1)
                });
                if let Some(id) = &replaces {
                    crate::log::line("catch", &format!("{label} is the update of {id}"));
                }
                t.send(
                    Some(game.id()),
                    &name,
                    true,
                    Box::new(move |w: &mut Worker| {
                        let out = crate::actions::add_archive(w, &game, &path, source, replaces, &label);
                        report(w, &game, &path, out);
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
            set(&qt, move |mut t| {
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
                // A newer helper goes into its folder in Downloads now too, so a reload (Turbine's, or the
                // player's own in chrome://extensions) picks it up.
                let _ = refresh_helper_copy();
                bridge::set_games(games_json(&self.rust().found));
                self.as_mut().bridge_refresh();
                // The status now, if there is one, then any installer choices already waiting.
                if let Ok(s) = turbine_engine::json::parse(&self.status_json().to_string()) && s.get("active").and_then(Json::as_bool) == Some(true) {
                    let p = match s.get("progress") {
                        Some(Json::Num(n)) if *n >= 0.0 => Some(*n),
                        _ => None,
                    };
                    bridge::broadcast(&status_message(s.str_at("text").unwrap_or(""), s.str_at("level").unwrap_or("ok"), p, s.str_at("game").unwrap_or("")));
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
                if let Some(g) = &d.game {
                    c.helper_games.push_back((label.clone(), g.id(), d.r.clone()));
                    if c.helper_games.len() > 20 {
                        c.helper_games.pop_front();
                    }
                }
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
                // Never 100% before the browser says it finished with its final name (its Save box may still wait).
                let p = (total > 0).then(|| (got as f64 / total as f64).min(0.99));
                let gname = d.game.as_ref().map(|g| g.name.clone()).unwrap_or_default();
                self.as_mut().set_status(&format!("Downloading {}", d.label), "busy", p, &gname, "", false, "");
            }
            "download.restartFailed" => {
                // The helper couldn't save it into Game Mods without a Save box (the link expired): the browser's own
                // download goes on, and may ask where to save.
                let file = msg.str_at("filename").unwrap_or("the file");
                crate::log::line("bridge", &format!("The helper couldn't restart {file} into Game Mods; the browser's own download goes on"));
                self.as_mut().set_status("Your browser may ask where to save.", "busy", None, "", "", false, "");
            }
            "download.waiting" => {
                crate::log::line("bridge", "The browser is waiting for Save");
                self.as_mut().set_status("Press Save in the browser", "wait", None, "", "", false, "");
            }
            "download.failed" => {
                let id = id_of(msg);
                let d = self.as_mut().take_download(&id);
                let Some(d) = d else { return };
                let why = msg.str_at("error").unwrap_or("");
                crate::log::line("bridge", &format!("Download of {} stopped: {why}", d.label));
                let gname = d.game.as_ref().map(|g| g.name.clone()).unwrap_or_default();
                let words = if why.eq_ignore_ascii_case("USER_CANCELED") { format!("{} was stopped", d.label) } else { format!("The download of {} stopped. Try it again on the page.", d.label) };
                self.as_mut().set_status(&words, "warn", None, &gname, "", true, "");
            }
            "download.finished" => {
                let id = id_of(msg);
                let d = self.as_mut().take_download(&id);
                let raw = msg.str_at("path").unwrap_or("");
                let label = d.as_ref().map(|d| d.label.clone()).filter(|l| !l.is_empty()).unwrap_or_else(|| label_of(Path::new(raw)));
                let path = match accept_download(raw) {
                    Ok(p) => p,
                    Err(e) => {
                        // Never silent: the watcher's own copy of the file (if it has one) is decided the usual way.
                        crate::log::line("bridge", &format!("Finished download not taken: {e}; the Downloads watcher takes {label} instead"));
                        let held = self.rust().catching.deferred.iter().any(|(x, _)| label_of(&x.path) == label);
                        self.as_mut().fallback_to_watcher(&label, None);
                        if d.is_some() && !held && !e.contains("isn't in Downloads") {
                            self.as_mut().set_status("Turbine can't read that download. Save mods to Downloads, then add them with Game menu, Add from Downloads.", "warn", None, "", "", true, "");
                        } else if d.is_some() && !held && !self.rust().catching.told_outside {
                            // Never silent (inapp-browser.md): once a run, where it went and the one setting that fixes it.
                            self.as_mut().rust_mut().catching.told_outside = true;
                            let folder = Path::new(raw).parent().map(|p| p.display().to_string()).unwrap_or_default();
                            crate::log::line("bridge", &format!("{label} was saved in {folder}, outside Downloads: told the player"));
                            self.as_mut().set_status(
                                &format!("Chrome saved {label} outside Downloads, where Turbine can't read it. In Chrome: Settings, Downloads, Location: Downloads."),
                                "warn", None, "", "", true, "",
                            );
                        }
                        return;
                    }
                };
                if d.is_none() {
                    // A finish Turbine never saw start (the helper restarted mid-download): decided the usual way.
                    crate::log::line("bridge", &format!("Finished download {label} wasn't reported starting; deciding it the usual way"));
                    self.as_mut().fallback_to_watcher(&label, Some(path));
                    return;
                }
                // Reported and taken: nothing is held back for it any more.
                self.as_mut().rust_mut().catching.deferred.retain(|(x, _)| x.path != path);
                let r = d.as_ref().and_then(|d| d.r.clone());
                let game = d.as_ref().and_then(|d| d.game.clone()).or_else(|| r.as_ref().and_then(|r| self.game_for_domain(&r.domain)));
                match game {
                    Some(g) => self.catch_add(g, path, r.map(|r| (r, None)), None),
                    None => {
                        let label = label_of(&path);
                        crate::log::line("bridge", &format!("{label}: no game for it here"));
                        self.as_mut().set_status(
                            &format!("Turbine doesn't know which game {label} is for. Open that game in Turbine, then Game menu, Add from Downloads."),
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
            // The sign-in from the player's own browser (web.rs checks it against the waiting request).
            "signin.cookies" => self.signin_cookies(msg),
            // Where the sign-in is in the browser (a word for the log), and the API key the player chose to give.
            "signin.status" => self.signin_status(msg),
            "signin.key" => self.signin_key(msg),
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

    /// A finished download from Turbine's browser that isn't a sure match (not from Nexus's own file host, or no
    /// press): never added by itself; Turbine asks which game it is for (the game on screen is the first guess).
    pub(crate) fn catch_ask(mut self: Pin<&mut Self>, a: Arrival, why: &str) {
        let name = label_of(&a.path);
        if self.rust().found.is_empty() {
            crate::log::line("catch", &format!("{name}: no games found here, so there is nowhere to add it"));
            return;
        }
        crate::log::line("catch", &format!("{name}: asking, because {why}"));
        let game = self.current_game();
        let ask = Ask { path: a.path, size: a.size, game, hint: None, source: None, why: why.to_string() };
        self.as_mut().rust_mut().catching.asks.push(ask);
        self.as_mut().catch_save(true);
        self.show_ask();
    }

    /// A caught mod went in (or its installer finished): the status and the helper hear about it.
    pub(crate) fn caught_added(mut self: Pin<&mut Self>, game: &str, name: &str, notes: Vec<String>, only_if_busy: bool) {
        if only_if_busy && !self.rust().catching.busy {
            return;
        }
        crate::log::line("catch", &format!("Added to {game}: {name}"));
        self.as_mut().set_status(&format!("Added to {game}: {name}. It's on."), "ok", None, game, name, true, "");
        let notes: Vec<String> = notes.iter().map(|n| quote(n)).collect();
        bridge::broadcast(&format!("{{\"type\":\"added\",\"game\":{},\"mod\":{},\"notes\":[{}],\"needs\":[]}}", quote(game), quote(name), notes.join(",")));
    }
}

/// What add_archive did with a caught download, for the status (and for its row in Turbine's browser, by its path).
pub(crate) fn report(w: &Worker, game: &Game, path: &Path, out: crate::actions::Outcome) {
    let g = game.name.clone();
    let p = path.to_path_buf();
    match out {
        crate::actions::Outcome::Added { name, notes } => w.post(move |mut t| {
            t.as_mut().add_result(&p, true, false, &name, &g, &format!("Added to {g}: {name}. It's on."));
            t.caught_added(&g, &name, notes, false)
        }),
        crate::actions::Outcome::Choices { name } => w.post(move |mut t| {
            t.as_mut().add_result(&p, true, false, &name, &g, &format!("{name} needs a choice from you. Make it in Turbine."));
            t.set_status(&format!("{name} needs a choice from you. Make it on the page or in Turbine."), "busy", None, &g, "", false, "")
        }),
        crate::actions::Outcome::Failed(e) => w.post(move |mut t| {
            t.as_mut().add_result(&p, false, false, "", &g, &e);
            t.set_status(&e, "stop", None, &g, "", true, "")
        }),
    }
}

/// A caught download that needed the player and was answered without being added (an older Seamless Co-op the
/// player chose not to use): its row and the status say it is done, so nothing waits on it or offers it again.
pub(crate) fn settled(w: &Worker, game: &Game, path: &Path, words: &str) {
    let (g, p, words) = (game.name.clone(), path.to_path_buf(), words.to_string());
    w.post(move |mut t| {
        t.as_mut().add_result(&p, true, false, "", &g, &words);
        t.set_status(&words, "ok", None, &g, "", true, "")
    });
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

/// The helper's folder in Downloads (Turbine puts it there for Load unpacked).
fn helper_folder() -> PathBuf {
    turbine_engine::watch::downloads_folder(&turbine_engine::home_dir()).join(HELPER_FOLDER)
}

/// The "version" in a helper folder's manifest.json, when it is Turbine's helper.
fn manifest_version(dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(dir.join("manifest.json")).ok()?;
    let m = turbine_engine::json::parse(&text).ok()?;
    if m.str_at("name") != Some("Turbine browser helper") {
        return None;
    }
    m.str_at("version").map(str::to_string).filter(|v| !v.is_empty())
}

/// At start, and before Turbine asks a helper to reload: when the helper in Downloads (the folder Turbine placed for
/// Load unpacked) is older than the one Turbine carries, the newer files are copied over it. Chrome loads an unpacked
/// helper from that folder, so its next reload is the new one. A folder that isn't Turbine's helper is left alone,
/// and so is anything in it that is a link (nothing is ever written through one, inside the folder or out of it).
/// True when the folder now holds Turbine's version, so asking the helper to reload is worth it.
pub fn refresh_helper_copy() -> bool {
    let (from, to) = (PathBuf::from(HELPER_SOURCE), helper_folder());
    if bridge::is_link(&to) || bridge::is_link(&to.join("manifest.json")) {
        crate::log::line("bridge", &format!("Left the browser helper in {} alone: it is a link", to.display()));
        return false;
    }
    let Some(have) = manifest_version(&to) else { return false };
    let Some(new) = manifest_version(&from) else { return false };
    if !bridge::version_older(&have, &new) {
        return true;
    }
    match bridge::copy_helper(&from, &to) {
        Ok(n) => {
            crate::log::line("bridge", &format!("Updated the browser helper in {} from {have} to {new} ({n} files)", to.display()));
            true
        }
        Err(e) => {
            crate::log::line("bridge", &format!("Couldn't update the browser helper in {} from {have} to {new}: {e}", to.display()));
            false
        }
    }
}

unsafe extern "C" {
    // qtglue.cpp
    fn turbine_overlay_mark(on: i32) -> i32;
}

/// An installer option's id in the helper's "choices" message: "g<group>o<option>".
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
