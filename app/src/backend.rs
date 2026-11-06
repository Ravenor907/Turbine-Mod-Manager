//! The bridge between the QML interface and the engine: one object, `Turbine`, available to QML as a singleton.
//!
//! Everything that touches a game's mods runs on one worker thread, in the order it was asked for, so the interface
//! never freezes while a zip is unpacked or files are placed. Results come back to the interface thread through
//! cxx-qt's thread queue and land in the properties below; plain sentences for the status line arrive as `message`.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(QString, games)]
        #[qproperty(QString, problems)]
        #[qproperty(QString, version)]
        #[qproperty(bool, steamos)]
        #[qproperty(bool, gamemode)]
        #[qproperty(bool, pads)]
        #[qproperty(QString, mods_json, cxx_name = "modsJson")]
        #[qproperty(QString, rules_json, cxx_name = "rulesJson")]
        #[qproperty(QString, downloads_json, cxx_name = "downloadsJson")]
        #[qproperty(QString, check_json, cxx_name = "checkJson")]
        #[qproperty(bool, busy)]
        #[qproperty(i32, pending)]
        #[qproperty(QString, reset_for, cxx_name = "resetFor")]
        #[qproperty(QString, browse_json, cxx_name = "browseJson")]
        #[qproperty(QString, details_json, cxx_name = "detailsJson")]
        #[qproperty(QString, download_json, cxx_name = "downloadJson")]
        #[qproperty(i32, net_busy, cxx_name = "netBusy")]
        #[qproperty(QString, fomod_json, cxx_name = "fomodJson")]
        #[qproperty(QString, order_json, cxx_name = "orderJson")]
        #[qproperty(QString, profiles_json, cxx_name = "profilesJson")]
        #[qproperty(QString, shares_json, cxx_name = "sharesJson")]
        #[qproperty(QString, import_json, cxx_name = "importJson")]
        #[qproperty(QString, updates_json, cxx_name = "updatesJson")]
        #[qproperty(QString, account_json, cxx_name = "accountJson")]
        #[qproperty(QString, self_json, cxx_name = "selfJson")]
        #[qproperty(QString, code_json, cxx_name = "codeJson")]
        #[qproperty(QString, handler_json, cxx_name = "handlerJson")]
        type Turbine = super::TurbineRust;

        /// Looks for games again (Steam and Heroic).
        #[qinvokable]
        fn scan(self: Pin<&mut Self>);

        /// Reads the controllers. QML calls this on a timer while the window is active.
        #[qinvokable]
        fn poll(self: Pin<&mut Self>);

        /// Turbine's window came back to the front: drops controller presses made meanwhile (in a game).
        #[qinvokable]
        fn wake(self: Pin<&mut Self>);

        /// Shows this game's mods and rules (opens its store the first time).
        #[qinvokable]
        fn select(self: Pin<&mut Self>, game_id: &QString);

        /// Adds a mod from a zip file to the current game.
        #[qinvokable]
        #[cxx_name = "importZip"]
        fn import_zip(self: Pin<&mut Self>, path: &QString);

        /// For the build's screenshot checks: adds a zip as if it came from `kind` (e.g. "thunderstore") as `id` at
        /// `version`.
        #[qinvokable]
        #[cxx_name = "selftestImport"]
        fn selftest_import(self: Pin<&mut Self>, path: &QString, kind: &QString, id: &QString, version: &QString);

        #[qinvokable]
        #[cxx_name = "setEnabled"]
        fn set_enabled(self: Pin<&mut Self>, id: &QString, on: bool);

        #[qinvokable]
        #[cxx_name = "moveTo"]
        fn move_to(self: Pin<&mut Self>, id: &QString, index: i32);

        #[qinvokable]
        #[cxx_name = "removeMod"]
        fn remove_mod(self: Pin<&mut Self>, id: &QString);

        #[qinvokable]
        #[cxx_name = "undoRemove"]
        fn undo_remove(self: Pin<&mut Self>, id: &QString);

        /// Applies waiting changes to the game now (does nothing when there are none).
        #[qinvokable]
        #[cxx_name = "applyNow"]
        fn apply_now(self: Pin<&mut Self>);

        /// Applies waiting changes and waits for it to finish. Only for when Turbine closes.
        #[qinvokable]
        fn finish(self: Pin<&mut Self>);

        /// Takes every mod file out of the current game, then starts it.
        #[qinvokable]
        #[cxx_name = "playOriginal"]
        fn play_original(self: Pin<&mut Self>);

        /// The way back when Turbine's records are damaged: every mod file out, the game's own files back, mods off.
        #[qinvokable]
        #[cxx_name = "resetGame"]
        fn reset_game(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "restoreSetup"]
        fn restore_setup(self: Pin<&mut Self>, id: &QString);

        /// The check before Play for the current game. The answer arrives as `checked`.
        #[qinvokable]
        fn check(self: Pin<&mut Self>);

        /// Applies, then asks QML to open the game's launch link (`launch`).
        #[qinvokable]
        fn play(self: Pin<&mut Self>, game_id: &QString);

        /// Reads the zip files in Downloads into `downloadsJson`.
        #[qinvokable]
        #[cxx_name = "listDownloads"]
        fn list_downloads(self: Pin<&mut Self>);

        /// One line in the activity log from the interface (taps, pages, menu choices, window changes).
        #[qinvokable]
        fn log(self: &Self, kind: &QString, text: &QString);

        /// Puts text on the clipboard.
        #[qinvokable]
        #[cxx_name = "copyText"]
        fn copy_text(self: &Self, text: &QString);

        /// The Labs report, written out in plain text. item: what it's about (may be empty); interface: page,
        /// focus, window and look, from QML; perf: Labs > Performance check's report or ""; shot: the screenshot's
        /// file name or "".
        #[qinvokable]
        fn report(self: &Self, item: &QString, note: &QString, interface: &QString, perf: &QString, crash: bool, shot: &QString) -> QString;

        /// Saves a report into Downloads, with the screenshot at `shot` copied beside it when not "". Answers JSON:
        /// {"report": file name, "shot": file name or "", "error": "" or what went wrong}.
        #[qinvokable]
        #[cxx_name = "saveReport"]
        fn save_report(self: &Self, text: &QString, stamp: &QString, shot: &QString) -> QString;

        /// Date and time for file names: 2026-11-06_14-03-22.
        #[qinvokable]
        fn stamp(self: &Self) -> QString;

        /// Where the report screen keeps its screenshot until Send.
        #[qinvokable]
        #[cxx_name = "shotTemp"]
        fn shot_temp(self: &Self) -> QString;

        #[qinvokable]
        #[cxx_name = "hasCrashLog"]
        fn has_crash_log(self: &Self) -> bool;

        /// Microseconds since Turbine started (for frame times).
        #[qinvokable]
        #[cxx_name = "nowUs"]
        fn now_us(self: &Self) -> f64;

        /// Facts for Labs > Performance check, as JSON: startup, memory, processor time, controller waits, device.
        #[qinvokable]
        #[cxx_name = "perfFacts"]
        fn perf_facts(self: &Self) -> QString;

        /// The window has drawn its first frame: notes how long starting took.
        #[qinvokable]
        #[cxx_name = "markShown"]
        fn mark_shown(self: Pin<&mut Self>);

        // ------------------------------------------------ 0.3: online sources and everything after (actions.rs)

        /// Recorded samples instead of the internet (the build's screenshot checks).
        #[qinvokable]
        #[cxx_name = "setSampleData"]
        fn set_sample_data(self: Pin<&mut Self>, dir: &QString);

        /// The screenshot checks: the sample Nexus account is a free one.
        #[qinvokable]
        #[cxx_name = "setSampleFree"]
        fn set_sample_free(self: Pin<&mut Self>);

        /// The next Nexus link that arrived (from the command line or a later launch), or "" when none waits.
        #[qinvokable]
        #[cxx_name = "takeLink"]
        fn take_link(self: &Self) -> QString;
        /// Reads a Nexus link: {"game": game id or "", "name": the game's name, "error": "" or why it can't be used}.
        #[qinvokable]
        #[cxx_name = "openLink"]
        fn open_link(self: &Self, link: &QString) -> QString;
        /// Downloads what a Nexus link points at and adds it to that game.
        #[qinvokable]
        #[cxx_name = "nxmDownload"]
        fn nxm_download(self: Pin<&mut Self>, link: &QString, game_id: &QString);
        /// Is Turbine the program for Nexus's Mod Manager Download links? The answer arrives in `handlerJson`.
        #[qinvokable]
        #[cxx_name = "handlerStatus"]
        fn handler_status(self: Pin<&mut Self>);
        /// Makes Turbine that program.
        #[qinvokable]
        #[cxx_name = "handlerSet"]
        fn handler_set(self: Pin<&mut Self>);

        /// Get Mods: source "thunderstore" or "nexus"; sort "downloads", "rating", "updated" or "name".
        #[qinvokable]
        fn browse(self: Pin<&mut Self>, text: &QString, category: &QString, sort: &QString, source: &QString);

        #[qinvokable]
        fn details(self: Pin<&mut Self>, id: &QString, source: &QString);

        /// Downloads a mod (and what it needs) and adds it to the current game.
        #[qinvokable]
        fn install(self: Pin<&mut Self>, id: &QString, source: &QString);

        #[qinvokable]
        #[cxx_name = "installLoader"]
        fn install_loader(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "cancelDownload"]
        fn cancel_download(self: Pin<&mut Self>);

        /// FOMOD installer: A on an option, next and back a step, cancel.
        #[qinvokable]
        #[cxx_name = "fomodChoose"]
        fn fomod_choose(self: Pin<&mut Self>, group: i32, option: i32);
        #[qinvokable]
        #[cxx_name = "fomodNext"]
        fn fomod_next(self: Pin<&mut Self>);
        #[qinvokable]
        #[cxx_name = "fomodBack"]
        fn fomod_back(self: Pin<&mut Self>);
        #[qinvokable]
        #[cxx_name = "fomodCancel"]
        fn fomod_cancel(self: Pin<&mut Self>);

        /// Load order (Skyrim): read, move, turn on or off, sort.
        #[qinvokable]
        #[cxx_name = "loadOrder"]
        fn load_order(self: Pin<&mut Self>);
        #[qinvokable]
        #[cxx_name = "orderMove"]
        fn order_move(self: Pin<&mut Self>, name: &QString, index: i32);
        #[qinvokable]
        #[cxx_name = "orderToggle"]
        fn order_toggle(self: Pin<&mut Self>, name: &QString, on: bool);
        #[qinvokable]
        #[cxx_name = "orderSort"]
        fn order_sort(self: Pin<&mut Self>);

        /// Profiles of the current game.
        #[qinvokable]
        fn profiles(self: Pin<&mut Self>);
        #[qinvokable]
        #[cxx_name = "profileSwitch"]
        fn profile_switch(self: Pin<&mut Self>, name: &QString);
        #[qinvokable]
        #[cxx_name = "profileCreate"]
        fn profile_create(self: Pin<&mut Self>, name: &QString);
        #[qinvokable]
        #[cxx_name = "profileExport"]
        fn profile_export(self: Pin<&mut Self>, name: &QString);
        /// Share files in Downloads, into `sharesJson`.
        #[qinvokable]
        #[cxx_name = "listShares"]
        fn list_shares(self: Pin<&mut Self>);
        #[qinvokable]
        #[cxx_name = "profileImportFile"]
        fn profile_import_file(self: Pin<&mut Self>, path: &QString);
        #[qinvokable]
        #[cxx_name = "profileImportCode"]
        fn profile_import_code(self: Pin<&mut Self>, code: &QString);
        /// Uploads a profile's Thunderstore mods (and its BepInEx settings) and makes a code to share, into `codeJson`.
        #[qinvokable]
        #[cxx_name = "profileCode"]
        fn profile_code(self: Pin<&mut Self>, name: &QString);
        /// Installs one version of a mod ("Owner-Name-1.2.3" on Thunderstore; a file ID on Nexus Mods).
        #[qinvokable]
        #[cxx_name = "installVersion"]
        fn install_version(self: Pin<&mut Self>, id: &QString, source: &QString, version: &QString);
        /// Gets the mods of the share being imported and makes its profile.
        #[qinvokable]
        #[cxx_name = "profileImportGo"]
        fn profile_import_go(self: Pin<&mut Self>);

        /// Mod updates for the current game.
        #[qinvokable]
        #[cxx_name = "checkUpdates"]
        fn check_updates(self: Pin<&mut Self>);
        #[qinvokable]
        #[cxx_name = "updateMod"]
        fn update_mod(self: Pin<&mut Self>, mod_id: &QString);
        #[qinvokable]
        #[cxx_name = "updateAll"]
        fn update_all(self: Pin<&mut Self>);

        /// Nexus Mods account.
        #[qinvokable]
        #[cxx_name = "setNexusKey"]
        fn set_nexus_key(self: Pin<&mut Self>, key: &QString);
        #[qinvokable]
        #[cxx_name = "nexusSignOut"]
        fn nexus_sign_out(self: Pin<&mut Self>);
        #[qinvokable]
        #[cxx_name = "accountStatus"]
        fn account_status(self: Pin<&mut Self>);

        /// Turbine's own updates. labs: pre-releases count; back: the newest stable even if older.
        #[qinvokable]
        #[cxx_name = "selfCheck"]
        fn self_check(self: Pin<&mut Self>, labs: bool, back: bool);
        #[qinvokable]
        #[cxx_name = "selfInstall"]
        fn self_install(self: Pin<&mut Self>);
        /// Starts the newly installed Turbine; QML quits right after.
        #[qinvokable]
        fn restart(self: Pin<&mut Self>);

        /// A controller button went down or up: "a", "b", "x", "y", "lb", "rb", "lt", "rt", "start", "view",
        /// "up", "down", "left", "right" (D-pad and left stick).
        #[qsignal]
        fn pad(self: Pin<&mut Self>, button: QString, pressed: bool);

        /// A sentence for the status line. level: "work" (in progress), "ok", "warn" or "error".
        #[qsignal]
        fn message(self: Pin<&mut Self>, text: QString, level: QString);

        /// The check before Play finished; the same JSON as `checkJson`.
        #[qsignal]
        fn checked(self: Pin<&mut Self>, json: QString);

        /// Pages to open in the browser (free accounts): title, then JSON [{"name","url"}].
        #[qsignal]
        fn pages(self: Pin<&mut Self>, title: QString, json: QString);

        /// Open this link to start the game (steam://... or Heroic's link).
        #[qsignal]
        fn launch(self: Pin<&mut Self>, url: QString);
    }

    impl cxx_qt::Threading for Turbine {}
}

use crate::pad::Pads;
use core::pin::Pin;
use cxx_qt::{CxxQtThread, CxxQtType, Threading};
use cxx_qt_lib::QString;
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::time::Duration;
use turbine_engine::json::quote;
use turbine_engine::proton;
use turbine_engine::rules::{self, GameRules, Kind};
use turbine_engine::store::{self, Store};
use turbine_engine::Game;

/// The check before Play's line about a game's one-time Steam Launch Option (QML hides it once it's been seen).
const LAUNCH_OPTION_TITLE: &str = "One-time Steam setting for the mod loader";

/// Said when a part of the engine isn't finished in this build (it panics instead of answering).
const NOT_READY: &str = "Not ready yet: this part of Turbine isn't finished in this build.";

pub struct TurbineRust {
    pub(crate) games: QString,
    pub(crate) problems: QString,
    pub(crate) version: QString,
    pub(crate) steamos: bool,
    pub(crate) gamemode: bool,
    pub(crate) pads: bool,
    pub(crate) mods_json: QString,
    pub(crate) rules_json: QString,
    pub(crate) downloads_json: QString,
    pub(crate) check_json: QString,
    pub(crate) busy: bool,
    pub(crate) pending: i32,
    pub(crate) reset_for: QString,
    pub(crate) reader: Option<Pads>,
    /// The games from the last scan, to hand to the worker.
    pub(crate) found: Vec<Game>,
    /// The game on screen.
    pub(crate) current: String,
    /// The game the waiting changes belong to.
    pub(crate) dirty: String,
    /// Long jobs (import, apply, play) not finished yet.
    pub(crate) inflight: i32,
    pub(crate) jobs: Option<Sender<Task>>,
    /// The current game's mods in a few lines, for reports (kept up to date by the worker).
    pub(crate) mods_summary: Vec<String>,
    /// The last engine report (apply, play, restore, reset), for reports.
    pub(crate) last_report: String,
    /// Recent warnings and errors said in the status line, with their times.
    pub(crate) recent: std::collections::VecDeque<String>,
    /// Milliseconds from start to the first frame drawn.
    pub(crate) startup_ms: f64,
    // 0.3
    pub(crate) browse_json: QString,
    pub(crate) details_json: QString,
    pub(crate) download_json: QString,
    pub(crate) net_busy: i32,
    pub(crate) fomod_json: QString,
    pub(crate) order_json: QString,
    pub(crate) profiles_json: QString,
    pub(crate) shares_json: QString,
    pub(crate) import_json: QString,
    pub(crate) updates_json: QString,
    pub(crate) account_json: QString,
    pub(crate) self_json: QString,
    pub(crate) code_json: QString,
    pub(crate) handler_json: QString,
    /// The download lane (started the first time it's needed).
    pub(crate) net: Option<Sender<crate::actions::NetTask>>,
    /// What the download lane needs to know about the game on screen (kept up to date by the worker).
    pub(crate) game_info: crate::online::GameInfo,
}

impl Default for TurbineRust {
    fn default() -> Self {
        Self {
            games: QString::from("[]"),
            problems: QString::default(),
            version: QString::from(crate::VERSION),
            steamos: crate::system::is_steamos(),
            gamemode: crate::system::in_game_mode(),
            pads: false,
            mods_json: QString::default(),
            rules_json: QString::default(),
            downloads_json: QString::from("[]"),
            check_json: QString::from("[]"),
            busy: false,
            pending: 0,
            reset_for: QString::default(),
            reader: None,
            found: Vec::new(),
            current: String::new(),
            dirty: String::new(),
            inflight: 0,
            jobs: None,
            mods_summary: Vec::new(),
            last_report: String::new(),
            recent: std::collections::VecDeque::new(),
            startup_ms: 0.0,
            browse_json: QString::default(),
            details_json: QString::default(),
            download_json: QString::from("{\"active\":false}"),
            net_busy: 0,
            fomod_json: QString::default(),
            order_json: QString::default(),
            profiles_json: QString::default(),
            shares_json: QString::from("[]"),
            import_json: QString::default(),
            updates_json: QString::default(),
            account_json: QString::from("{\"state\":\"none\"}"),
            self_json: QString::from("{\"state\":\"idle\"}"),
            code_json: QString::default(),
            handler_json: QString::from("{\"state\":\"unknown\"}"),
            net: None,
            game_info: crate::online::GameInfo::default(),
        }
    }
}

// ---------------------------------------------------------------------------------------------------- worker thread

pub(crate) struct Slot {
    pub(crate) game: Game,
    pub(crate) rules: GameRules,
    pub(crate) store: Store,
}

pub(crate) struct Worker {
    pub(crate) data_home: PathBuf,
    pub(crate) slots: HashMap<String, Slot>,
    pub(crate) qt: CxxQtThread<qobject::Turbine>,
    /// Installers waiting for the player's choices (game id, installer), oldest first.
    pub(crate) pending: std::collections::VecDeque<(String, store::Pending)>,
    /// A profile share being imported (game id, share).
    pub(crate) share: Option<(String, turbine_engine::profiles::Share)>,
    /// LOOT's messages per game: (what the plugins looked like when they were read, plugin name lowercased: messages).
    pub(crate) loot: HashMap<String, (String, HashMap<String, Vec<crate::loot::Msg>>)>,
}

pub(crate) type Job = Box<dyn FnOnce(&mut Worker) + Send>;

struct Task {
    /// The game this job works on; its store is dropped (and opened again next time) if the job panics.
    game: Option<String>,
    /// What it is, for the activity log.
    name: String,
    long: bool,
    run: Job,
}

impl Worker {
    pub(crate) fn post<F>(&self, f: F)
    where
        F: FnOnce(Pin<&mut qobject::Turbine>) + Send + 'static,
    {
        let _ = self.qt.queue(f);
    }

    /// Remembers whether this game's records need a Reset (shown in the game menu).
    pub(crate) fn note(&self, game: &Game, report: &turbine_engine::deploy::Report) {
        let key = game.id();
        let needs = needs_reset(report);
        let words = format!("{} at {}: {report:?}", game.name, crate::log::file_stamp());
        crate::log::line("engine", &format!("Report for {key}: {report:?}"));
        self.post(move |mut t| {
            t.as_mut().rust_mut().last_report = words;
            let now = t.reset_for().to_string();
            if needs && now != key {
                t.as_mut().set_reset_for(QString::from(key.as_str()));
            } else if !needs && now == key {
                t.as_mut().set_reset_for(QString::default());
            }
        });
    }

    pub(crate) fn say(&self, text: impl Into<String>, level: &'static str) {
        let text = text.into();
        crate::log::line("engine", &format!("[{level}] {text}"));
        self.post(move |mut t| {
            if level == "warn" || level == "error" {
                remember(t.as_mut().rust_mut().get_mut(), &format!("[{level}] {text}"));
            }
            t.message(QString::from(text.as_str()), QString::from(level))
        });
    }

    /// The game's store and rules, opened the first time they're needed.
    pub(crate) fn slot(&mut self, game: &Game) -> Result<&mut Slot, String> {
        let key = game.id();
        if !self.slots.contains_key(&key) {
            let rules = GameRules::for_game(game);
            let store = Store::open(game, &self.data_home).map_err(|e| format!("Couldn't open {}'s mods: {e}", game.name))?;
            self.slots.insert(key.clone(), Slot { game: game.clone(), rules, store });
        }
        Ok(self.slots.get_mut(&key).expect("just inserted"))
    }

    /// Sends the game's mods (and rules) to the interface if it's still the game on screen.
    pub(crate) fn show(&mut self, game: &Game, with_rules: bool) {
        let key = game.id();
        let Some(slot) = self.slots.get(&key) else { return };
        let mods = slot.store.to_json();
        let summary = crate::labs::mods_lines(&slot.store);
        let rules = with_rules.then(|| rules_json(slot));
        let info = crate::actions::game_info(slot);
        self.post(move |mut t| {
            if t.rust().current != key {
                return;
            }
            t.as_mut().rust_mut().mods_summary = summary;
            t.as_mut().rust_mut().game_info = info;
            if let Some(r) = rules {
                t.as_mut().set_rules_json(QString::from(r.as_str()));
            }
            t.as_mut().set_mods_json(QString::from(mods.as_str()));
        });
    }
}

/// Keeps the last 30 warnings and errors for reports.
fn remember(r: &mut TurbineRust, text: &str) {
    if r.recent.len() >= 30 {
        r.recent.pop_front();
    }
    let (_, _, _, h, m, s, _) = crate::log::local_now();
    r.recent.push_back(format!("{h:02}:{m:02}:{s:02} {text}"));
}

fn rules_json(slot: &Slot) -> String {
    let r = &slot.rules;
    let kind = match r.kind {
        Kind::Bethesda => "bethesda",
        Kind::BepInEx => "bepinex",
        Kind::Smapi => "smapi",
        Kind::Generic => "generic",
    };
    let (loader, installed) = match &r.loader {
        Some(l) => (
            format!(
                "{{\"name\":{},\"explain\":{},\"source\":{}}}",
                quote(&l.name),
                quote(&l.explain),
                quote(&l.source)
            ),
            r.loader_installed(&slot.game),
        ),
        None => ("null".to_string(), false),
    };
    format!(
        "{{\"kind\":{},\"ordered\":{},\"plugins\":{},\"modRoot\":{},\"support\":{},\"loader\":{},\"loaderInstalled\":{},\"shortName\":{},\"launchOption\":{},\"launchHelp\":{},\"community\":{},\"domain\":{}}}",
        quote(kind),
        r.ordered,
        r.plugins,
        quote(&r.mod_root),
        quote(&r.support),
        loader,
        installed,
        r.short_name.as_deref().map(quote).unwrap_or_else(|| "null".to_string()),
        r.launch_option.as_deref().map(quote).unwrap_or_else(|| "null".to_string()),
        r.launch_help.as_deref().map(quote).unwrap_or_else(|| "null".to_string()),
        crate::actions::community(&slot.game).map(quote).unwrap_or_else(|| "null".to_string()),
        turbine_engine::nexus::domain_for_store_id(&slot.game.store_id).map(quote).unwrap_or_else(|| "null".to_string())
    )
}

// ---------------------------------------------------------------------------------------------------- Proton

/// BepInEx under Proton needs winhttp set to "native,builtin" in the game's prefix. Some(prefix) when this game
/// needs that and its loader is in place (a Steam game run through Proton, with BepInEx installed).
fn proton_prefix(slot: &Slot) -> Option<Option<PathBuf>> {
    let g = &slot.game;
    let wanted = g.store == turbine_engine::Store::Steam
        && proton::wants_winhttp(g, &slot.rules)
        && slot.rules.loader_installed(g);
    wanted.then(|| g.prefix.clone())
}

/// Lines for the check before Play about the Proton setting, when there is something to say.
fn proton_checks(slot: &Slot) -> Vec<rules::Check> {
    let Some(prefix) = proton_prefix(slot) else { return Vec::new() };
    let line = |level: &str, title: String, detail: String| rules::Check { level: level.into(), title, detail };
    let name = &slot.game.name;
    if crate::system::game_running(&slot.game.install_dir) == Some(true) {
        return vec![line(
            "stop",
            format!("{name} is still running"),
            "Close the game first. Turbine turns BepInEx on in the game's Proton settings, which only works while the \
             game is closed."
                .into(),
        )];
    }
    let now = match prefix {
        None => Err(proton::NOT_STARTED.to_string()),
        Some(p) => proton::read_override(&p, "winhttp"),
    };
    match now {
        Ok(Some(v)) if v.eq_ignore_ascii_case(proton::BEPINEX_VALUE) => Vec::new(),
        Ok(_) => vec![line(
            "ok",
            "BepInEx gets turned on in Proton".into(),
            "At Play, Turbine sets winhttp to \"native,builtin\" in this game's Proton settings so BepInEx loads. \
             A copy of the old settings is kept."
                .into(),
        )],
        Err(e) if e == proton::NOT_STARTED => vec![line(
            "warn",
            "BepInEx can't load yet".into(),
            format!("Proton makes this game's settings the first time it runs. {} Until then mods that need BepInEx don't load.", proton::NOT_STARTED),
        )],
        Err(e) => vec![line("warn", "BepInEx may not load".into(), e)],
    }
}

fn checks_json(checks: &[rules::Check]) -> String {
    let items: Vec<String> = checks
        .iter()
        .map(|c| format!("{{\"level\":{},\"title\":{},\"detail\":{}}}", quote(&c.level), quote(&c.title), quote(&c.detail)))
        .collect();
    format!("[{}]", items.join(","))
}

fn downloads_json(home: &Path) -> String {
    let items: Vec<String> = store::downloads(home)
        .iter()
        .map(|p| {
            let meta = std::fs::metadata(p).ok();
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let modified = meta
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            format!(
                "{{\"name\":{},\"path\":{},\"size\":{},\"modified\":{}}}",
                quote(&name),
                quote(&p.to_string_lossy()),
                size,
                modified
            )
        })
        .collect();
    format!("[{}]", items.join(","))
}

fn changes(n: i32) -> String {
    if n == 1 {
        "1 change".into()
    } else {
        format!("{n} changes")
    }
}

/// Where Turbine keeps mods on internal storage. TURBINE_DATA moves it for the build's screenshot checks.
pub fn data_home() -> PathBuf {
    for key in ["TURBINE_DATA", "XDG_DATA_HOME"] {
        if let Some(v) = std::env::var_os(key).filter(|v| !v.is_empty()) {
            return v.into();
        }
    }
    turbine_engine::home_dir().join(".local/share")
}

fn count(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// What an apply-like job did, for the status line: "Ready to play", then a few words on anything worth knowing.
fn applied_words(report: &turbine_engine::deploy::Report, game: &Game) -> String {
    let mut s = String::from("Ready to play");
    let mut add = |t: String| {
        s.push_str(". ");
        s.push_str(&t);
    };
    if !report.put_back.is_empty() {
        add(format!("Put back {} {} replaced", count(report.put_back.len(), "mod file", "mod files"), game.store.launcher()));
    }
    if !report.changed_by_others.is_empty() {
        let n = report.changed_by_others.len();
        add(format!("{} changed by something else {} left alone", count(n, "mod file", "mod files"), if n == 1 { "was" } else { "were" }));
    }
    let kept = report.player_changes_saved.len() + report.player_changes_restored.len();
    if kept > 0 {
        add(format!("Kept your changes to {}", count(kept, "settings file", "settings files")));
    }
    if report.copied_not_linked > 0 {
        add(format!("{} copied, not linked (another drive), so they use full space", count(report.copied_not_linked, "file", "files")));
    }
    if !report.left_in_backups.is_empty() {
        add(format!("{} stay in the backup folder; something else is in their place", count(report.left_in_backups.len(), "game file", "game files")));
    }
    if needs_reset(report) {
        add("Some of Turbine's records for this game couldn't be read. Reset is in the game menu (View)".into());
    }
    s
}

fn needs_reset(report: &turbine_engine::deploy::Report) -> bool {
    !report.bad_entries.is_empty() || report.quarantined > 0
}

// ---------------------------------------------------------------------------------------------------- the object

impl qobject::Turbine {
    pub fn scan(self: Pin<&mut Self>) {
        let at = std::time::Instant::now();
        let scan = turbine_engine::scan_games(&turbine_engine::home_dir());
        crate::log::line(
            "scan",
            &format!("Found {} games in {} ms; {} problems", scan.games.len(), at.elapsed().as_millis(), scan.problems.len()),
        );
        for g in &scan.games {
            crate::log::line("scan", &format!("{} ({}), {:?}, {}, build {}", g.name, g.id(), g.runtime, g.drive_label(), g.build.as_deref().unwrap_or("?")));
        }
        for p in &scan.problems {
            crate::log::line("scan", &format!("Problem: {p}"));
        }
        let mut this = self;
        this.as_mut().set_problems(QString::from(scan.problems.join("\n").as_str()));
        let json = scan.games_json();
        this.as_mut().rust_mut().found = scan.games;
        this.as_mut().set_games(QString::from(json.as_str()));
    }

    pub fn wake(mut self: Pin<&mut Self>) {
        crate::log::line("window", "Back in front; controller quiet for 250 ms");
        if let Some(r) = self.as_mut().rust_mut().reader.as_mut() {
            r.wake(Duration::from_millis(250));
        }
    }

    pub fn poll(mut self: Pin<&mut Self>) {
        if self.as_mut().rust_mut().reader.is_none() {
            let r = Pads::open();
            self.as_mut().rust_mut().reader = Some(r);
        }
        let (events, connected) = match self.as_mut().rust_mut().reader.as_mut() {
            Some(r) => (r.poll(), r.connected()),
            None => (Vec::new(), false),
        };
        if *self.pads() != connected {
            self.as_mut().set_pads(connected);
        }
        for (button, pressed) in events {
            if pressed {
                crate::log::line("pad", &format!("Press {button}"));
            }
            self.as_mut().pad(QString::from(button), pressed);
        }
    }

    pub(crate) fn find(&self, id: &str) -> Option<Game> {
        self.rust().found.iter().find(|g| g.id() == id).cloned()
    }

    pub(crate) fn current_game(&self) -> Option<Game> {
        self.find(&self.rust().current)
    }

    /// Queues a job for the worker thread (started the first time).
    pub(crate) fn send(mut self: Pin<&mut Self>, game: Option<String>, name: &str, long: bool, run: Job) {
        if self.rust().jobs.is_none() {
            let (tx, rx) = mpsc::channel::<Task>();
            let qt = self.qt_thread();
            let spawned = std::thread::Builder::new().name("turbine-worker".into()).spawn(move || {
                // Stores are made and used on this thread only.
                let mut worker = Worker {
                    data_home: data_home(),
                    slots: HashMap::new(),
                    qt,
                    pending: std::collections::VecDeque::new(),
                    share: None,
                    loot: HashMap::new(),
                };
                while let Ok(task) = rx.recv() {
                    let long = task.long;
                    let name = task.name;
                    let at = std::time::Instant::now();
                    crate::log::line("engine", &format!("Start: {name}"));
                    let failed = catch_unwind(AssertUnwindSafe(|| (task.run)(&mut worker))).is_err();
                    crate::log::line(
                        "engine",
                        &format!("{}: {name} after {} ms", if failed { "Stopped (not ready)" } else { "Done" }, at.elapsed().as_millis()),
                    );
                    if failed {
                        if let Some(key) = task.game {
                            worker.slots.remove(&key);
                        }
                        worker.say(NOT_READY, "error");
                    }
                    if long {
                        worker.post(|mut t| {
                            let left = (t.rust().inflight - 1).max(0);
                            t.as_mut().rust_mut().inflight = left;
                            t.as_mut().set_busy(left > 0);
                        });
                    }
                }
            });
            if spawned.is_err() {
                self.as_mut().message(QString::from("Turbine couldn't start its worker. Close it and open it again."), QString::from("error"));
                return;
            }
            self.as_mut().rust_mut().jobs = Some(tx);
        }
        if long {
            let n = self.rust().inflight + 1;
            self.as_mut().rust_mut().inflight = n;
            self.as_mut().set_busy(true);
        }
        let name = name.to_string();
        let sent = self.rust().jobs.as_ref().map(|tx| tx.send(Task { game, name, long, run }).is_ok()).unwrap_or(false);
        if !sent && long {
            let n = (self.rust().inflight - 1).max(0);
            self.as_mut().rust_mut().inflight = n;
            self.as_mut().set_busy(n > 0);
        }
    }

    pub(crate) fn say(self: Pin<&mut Self>, text: &str, level: &str) {
        self.message(QString::from(text), QString::from(level));
    }

    pub fn select(mut self: Pin<&mut Self>, game_id: &QString) {
        let id = game_id.to_string();
        if self.rust().current == id {
            return;
        }
        // Changes made to the game being left go into it first.
        self.as_mut().apply_now();
        self.as_mut().rust_mut().current = id.clone();
        self.as_mut().set_mods_json(QString::default());
        self.as_mut().set_rules_json(QString::default());
        let Some(game) = self.find(&id) else { return };
        self.send(
            Some(id), &format!("Open {}", game.name),
            false,
            Box::new(move |w: &mut Worker| match w.slot(&game) {
                Ok(_) => w.show(&game, true),
                Err(e) => w.say(e, "error"),
            }),
        );
    }

    /// Notes one more change waiting to be applied to the current game.
    pub(crate) fn edited(mut self: Pin<&mut Self>) {
        let cur = self.rust().current.clone();
        if self.rust().dirty != cur && *self.pending() > 0 {
            // Changes for another game are still waiting; put them in before starting on this one.
            self.as_mut().apply_now();
        }
        self.as_mut().rust_mut().dirty = cur;
        let n = *self.pending() + 1;
        self.as_mut().set_pending(n);
    }

    /// Runs a quick change to one mod on the worker, then shows the new list.
    fn change<F>(mut self: Pin<&mut Self>, f: F)
    where
        F: FnOnce(&mut Store) -> Result<(), String> + Send + 'static,
    {
        let Some(game) = self.current_game() else { return };
        self.as_mut().edited();
        self.send(
            Some(game.id()), "Change a mod",
            false,
            Box::new(move |w: &mut Worker| {
                let result = w.slot(&game).and_then(|s| f(&mut s.store));
                if let Err(e) = result {
                    w.say(e, "error");
                }
                w.show(&game, false);
            }),
        );
    }

    pub fn import_zip(mut self: Pin<&mut Self>, path: &QString) {
        let Some(game) = self.current_game() else { return };
        let path = PathBuf::from(path.to_string());
        let file = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        self.as_mut().say(&format!("Adding {file}..."), "work");
        self.as_mut().edited();
        self.send(
            Some(game.id()), &format!("Add {file}"),
            true,
            Box::new(move |w: &mut Worker| {
                let source = turbine_engine::net::SourceInfo { kind: "zip".into(), ..Default::default() };
                crate::actions::add_archive(w, &game, &path, source, None, &file);
            }),
        );
    }

    pub fn selftest_import(mut self: Pin<&mut Self>, path: &QString, kind: &QString, id: &QString, version: &QString) {
        let Some(game) = self.current_game() else { return };
        let path = PathBuf::from(path.to_string());
        let file = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let source = turbine_engine::net::SourceInfo {
            kind: kind.to_string(),
            id: id.to_string(),
            version: Some(version.to_string()).filter(|v| !v.is_empty()),
            ..Default::default()
        };
        self.as_mut().edited();
        self.send(
            Some(game.id()),
            &format!("Add {file} (test)"),
            true,
            Box::new(move |w: &mut Worker| crate::actions::add_archive(w, &game, &path, source, None, &file)),
        );
    }

    pub fn set_enabled(self: Pin<&mut Self>, id: &QString, on: bool) {
        let id = id.to_string();
        self.change(move |s| s.set_enabled(&id, on).map_err(|e| format!("Couldn't change that mod: {e}")));
    }

    pub fn move_to(self: Pin<&mut Self>, id: &QString, index: i32) {
        let id = id.to_string();
        let index = index.max(0) as usize;
        self.change(move |s| s.move_to(&id, index).map_err(|e| format!("Couldn't move that mod: {e}")));
    }

    pub fn remove_mod(self: Pin<&mut Self>, id: &QString) {
        let id = id.to_string();
        self.change(move |s| s.remove(&id).map_err(|e| format!("Couldn't uninstall that mod: {e}")));
    }

    pub fn undo_remove(self: Pin<&mut Self>, id: &QString) {
        let id = id.to_string();
        self.change(move |s| s.undo_remove(&id).map_err(|e| format!("Couldn't put that mod back: {e}")));
    }

    /// Takes the waiting changes off the count before the job runs (so leaving the window right after Play doesn't
    /// apply them twice) and puts them back if it fails. Returns the game they belong to and how many there were.
    fn take_pending(mut self: Pin<&mut Self>) -> (Option<Game>, i32) {
        let n = *self.pending();
        if n == 0 {
            return (None, 0);
        }
        self.as_mut().set_pending(0);
        let dirty = self.rust().dirty.clone();
        (self.find(&dirty), n)
    }

    pub fn apply_now(mut self: Pin<&mut Self>) {
        let (Some(game), n) = self.as_mut().take_pending() else { return };
        self.as_mut().say(&format!("Applying {} to the game...", changes(n)), "work");
        let name = format!("Apply {} to {}", changes(n), game.name);
        self.send(Some(game.id()), &name, true, apply_job(game, n));
    }

    pub fn finish(mut self: Pin<&mut Self>) {
        let (Some(game), n) = self.as_mut().take_pending() else { return };
        let (tx, rx) = mpsc::channel::<()>();
        let job = apply_job(game.clone(), n);
        self.send(
            Some(game.id()), "Apply before closing",
            false,
            Box::new(move |w: &mut Worker| {
                // Signal the wait below even if apply panics.
                struct Done(Sender<()>);
                impl Drop for Done {
                    fn drop(&mut self) {
                        let _ = self.0.send(());
                    }
                }
                let _done = Done(tx);
                job(w);
            }),
        );
        let _ = rx.recv_timeout(Duration::from_secs(60));
    }

    pub fn play_original(mut self: Pin<&mut Self>) {
        let Some(game) = self.current_game() else { return };
        // Waiting changes would only be undone by this; they stay in Turbine and go in at the next Play.
        let (_, n) = self.as_mut().take_pending();
        self.as_mut().say("Taking the mods out of the game...", "work");
        self.send(
            Some(game.id()), "Play original",
            true,
            Box::new(move |w: &mut Worker| {
                let result = w.slot(&game).and_then(|s| s.store.play_original().map_err(|e| e.to_string()));
                match result {
                    Ok(r) => {
                        w.note(&game, &r);
                        w.say("Starting the game with no mods. Your mods are kept.", "ok");
                        launch(w, &game);
                    }
                    Err(e) => {
                        w.post(move |mut t| {
                            let p = *t.pending() + n;
                            t.as_mut().set_pending(p);
                        });
                        w.say(format!("Couldn't take the mods out: {e}"), "error");
                    }
                }
            }),
        );
    }

    pub fn reset_game(mut self: Pin<&mut Self>) {
        let Some(game) = self.current_game() else { return };
        let _ = self.as_mut().take_pending();
        self.as_mut().say("Resetting the game's files...", "work");
        self.send(
            Some(game.id()), "Reset game files",
            true,
            Box::new(move |w: &mut Worker| {
                let result = w.slot(&game).and_then(|s| s.store.reset_game().map_err(|e| e.to_string()));
                match result {
                    Ok(r) => {
                        w.note(&game, &r);
                        let mut words = String::from("Game files reset. Your mods are still here, turned off");
                        if !r.left_in_backups.is_empty() {
                            words.push_str(&format!(
                                ". {} stay in the backup folder; something else is in their place",
                                count(r.left_in_backups.len(), "game file", "game files")
                            ));
                        }
                        w.say(words, "ok");
                    }
                    Err(e) => w.say(format!("Couldn't reset the game's files: {e}"), "error"),
                }
                w.show(&game, false);
            }),
        );
    }

    pub fn restore_setup(mut self: Pin<&mut Self>, id: &QString) {
        let Some(game) = self.current_game() else { return };
        let setup = id.to_string();
        let _ = self.as_mut().take_pending();
        self.as_mut().say("Putting back the saved setup...", "work");
        self.send(
            Some(game.id()), "Restore setup",
            true,
            Box::new(move |w: &mut Worker| {
                let result = w.slot(&game).and_then(|s| s.store.restore(&setup).map_err(|e| e.to_string()));
                match result {
                    Ok(r) => {
                        w.note(&game, &r);
                        w.say(applied_words(&r, &game), "ok")
                    }
                    Err(e) => w.say(format!("Couldn't put that setup back: {e}"), "error"),
                }
                w.show(&game, false);
            }),
        );
    }

    pub fn check(self: Pin<&mut Self>) {
        let Some(game) = self.current_game() else { return };
        self.send(
            Some(game.id()), "Check before Play",
            false,
            Box::new(move |w: &mut Worker| {
                let json = match w.slot(&game) {
                    Ok(s) => {
                        let needs = s.store.mods().iter().any(|m| m.enabled && m.needs_loader);
                        let free = crate::system::free_bytes(&s.game.install_dir);
                        let last = s.store.last_build();
                        let mut list = rules::pre_play(&s.game, &s.rules, last.as_deref(), needs, free);
                        let mut extra = proton_checks(s);
                        // Native games whose loader starts through a Steam Launch Option: the one-time setting.
                        if let (Some(help), Some(option)) = (&s.rules.launch_help, &s.rules.launch_option) {
                            if needs && s.rules.loader_installed(&s.game) {
                                extra.push(rules::Check {
                                    level: "warn".into(),
                                    title: LAUNCH_OPTION_TITLE.into(),
                                    detail: format!("{help}\n{option}"),
                                });
                            }
                        }
                        if !extra.is_empty() && list.len() == 1 && list[0].level == "ok" {
                            list.clear(); // "Everything checks out" no longer says it all
                        }
                        list.extend(extra);
                        checks_json(&list)
                    }
                    Err(e) => checks_json(&[rules::Check { level: "stop".into(), title: "Mods couldn't be read".into(), detail: e }]),
                };
                w.post(move |mut t| {
                    t.as_mut().set_check_json(QString::from(json.as_str()));
                    t.as_mut().checked(QString::from(json.as_str()));
                });
            }),
        );
    }

    pub fn play(mut self: Pin<&mut Self>, game_id: &QString) {
        let Some(game) = self.find(&game_id.to_string()) else { return };
        let (_, n) = self.as_mut().take_pending();
        self.as_mut().say("Getting the game ready...", "work");
        self.send(
            Some(game.id()), &format!("Play {}", game.name),
            true,
            Box::new(move |w: &mut Worker| {
                let put_back = move |w: &Worker| {
                    w.post(move |mut t| {
                        let p = *t.pending() + n;
                        t.as_mut().set_pending(p);
                    })
                };
                // A running game would write its Proton settings over Turbine's change when it closes.
                let running = w.slot(&game).ok().and_then(|s| proton_prefix(s)).is_some()
                    && crate::system::game_running(&game.install_dir) == Some(true);
                if running {
                    put_back(w);
                    w.say(format!("{} is still running. Close it, then press Play again.", game.name), "error");
                    return;
                }
                let result = w.slot(&game).and_then(|s| s.store.apply().map_err(|e| e.to_string()));
                crate::actions::sync_order(w, &game);
                // The loader may have just been put in place by apply, so this comes after it.
                let note = match w.slot(&game).ok().and_then(|s| proton_prefix(s)) {
                    None => None,
                    Some(None) => Some(proton::NOT_STARTED.to_string()),
                    Some(Some(p)) => proton::ensure_override(&p, "winhttp", proton::BEPINEX_VALUE).err(),
                };
                match result {
                    Ok(r) => {
                        w.note(&game, &r);
                        // Play went through with the mods as they are: that setup is the one Restore goes back to,
                        // and this game build no longer counts as "updated since".
                        if let Ok(s) = w.slot(&game) {
                            let marked = s.store.mark_played().and_then(|_| s.store.acknowledge_build());
                            if let Err(e) = marked {
                                w.say(format!("Couldn't note this setup as played: {e}"), "warn");
                            }
                        }
                        match note {
                            Some(e) => w.say(format!("Started without BepInEx this time. {e}"), "warn"),
                            None => w.say(applied_words(&r, &game), "ok"),
                        }
                        launch(w, &game);
                        w.show(&game, false);
                    }
                    Err(e) => {
                        w.post(move |mut t| {
                            let p = *t.pending() + n;
                            t.as_mut().set_pending(p);
                        });
                        w.say(format!("Couldn't get the mods in place, so the game wasn't started: {e}"), "error");
                    }
                }
            }),
        );
    }

    pub fn list_downloads(self: Pin<&mut Self>) {
        let home = turbine_engine::home_dir();
        self.send(
            None, "List Downloads",
            false,
            Box::new(move |w: &mut Worker| {
                let json = downloads_json(&home);
                w.post(move |t| t.set_downloads_json(QString::from(json.as_str())));
            }),
        );
    }
}

impl qobject::Turbine {
    pub fn log(&self, kind: &QString, text: &QString) {
        crate::log::line(&kind.to_string(), &text.to_string());
    }

    pub fn copy_text(&self, text: &QString) {
        let text = text.to_string().replace('\0', "");
        let Ok(c) = std::ffi::CString::new(text) else { return };
        // SAFETY: c is a valid C string for the call; qtglue.cpp copies it into the clipboard on this (the GUI) thread.
        unsafe { turbine_copy_text(c.as_ptr()) };
        crate::log::line("labs", "Copied text to the clipboard");
    }

    pub fn report(&self, item: &QString, note: &QString, interface: &QString, perf: &QString, crash: bool, shot: &QString) -> QString {
        let r = self.rust();
        let game = self.current_game();
        let mut problems: Vec<String> = Vec::new();
        let scan = self.problems().to_string();
        problems.extend(scan.lines().filter(|l| !l.trim().is_empty()).map(|l| format!("Scan: {l}")));
        problems.extend(r.recent.iter().cloned());
        let input = crate::labs::ReportInput {
            item: item.to_string(),
            note: note.to_string(),
            interface: interface.to_string(),
            perf: perf.to_string(),
            version: crate::VERSION.to_string(),
            gamemode: *self.gamemode(),
            pads: *self.pads(),
            game: game.as_ref().map(crate::labs::game_lines).unwrap_or_else(|| vec!["No game selected".into()]),
            mods: if game.is_some() { r.mods_summary.clone() } else { Vec::new() },
            last_report: if r.last_report.is_empty() { "None this run".into() } else { r.last_report.clone() },
            problems,
            crash: if crash { crate::log::crash_tail(6000) } else { String::new() },
            shot: shot.to_string(),
        };
        let mut text = crate::labs::build(&input);
        if let Some(p) = r.reader.as_ref() {
            let names = p.names();
            if !names.is_empty() {
                text = text.replace("Controller: connected", &format!("Controller: connected ({})", names.join(", ")));
            }
        }
        QString::from(text.as_str())
    }

    pub fn save_report(&self, text: &QString, stamp: &QString, shot: &QString) -> QString {
        let shot = shot.to_string();
        let from = (!shot.is_empty()).then(|| PathBuf::from(&shot));
        let json = match crate::labs::save(&text.to_string(), &stamp.to_string(), from.as_deref()) {
            Ok((report, shot)) => {
                crate::log::line("labs", &format!("Saved {report}{}", if shot.is_empty() { String::new() } else { format!(" and {shot}") }));
                format!("{{\"report\":{},\"shot\":{},\"error\":\"\"}}", quote(&report), quote(&shot))
            }
            Err(e) => {
                crate::log::line("labs", &format!("Couldn't save the report: {e}"));
                format!("{{\"report\":\"\",\"shot\":\"\",\"error\":{}}}", quote(&e))
            }
        };
        QString::from(json.as_str())
    }

    pub fn stamp(&self) -> QString {
        QString::from(crate::log::file_stamp().as_str())
    }

    pub fn shot_temp(&self) -> QString {
        QString::from(crate::labs::shot_temp().to_string_lossy().as_ref())
    }

    pub fn has_crash_log(&self) -> bool {
        crate::log::has_crash_log()
    }

    pub fn now_us(&self) -> f64 {
        crate::labs::started().elapsed().as_secs_f64() * 1_000_000.0
    }

    pub fn perf_facts(&self) -> QString {
        let r = self.rust();
        let (median, worst, count) = r.reader.as_ref().map(|p| p.waits()).unwrap_or((0, 0, 0));
        let names = r.reader.as_ref().map(|p| p.names().join(", ")).unwrap_or_default();
        let json = format!(
            "{{\"startupMs\":{:.0},\"memoryMb\":{:.1},\"cpuMs\":{:.0},\"threads\":{},\"padMedian\":{},\"padWorst\":{},\"padCount\":{},\"pads\":{},\"device\":{},\"system\":{},\"build\":{}}}",
            r.startup_ms,
            crate::labs::memory_mb(),
            crate::labs::cpu_ms(),
            crate::labs::threads(),
            median,
            worst,
            count,
            quote(&names),
            quote(&crate::labs::device()),
            quote(&crate::labs::system()),
            quote(crate::labs::COMMIT)
        );
        QString::from(json.as_str())
    }

    pub fn mark_shown(mut self: Pin<&mut Self>) {
        if self.rust().startup_ms > 0.0 {
            return;
        }
        let ms = crate::labs::started().elapsed().as_secs_f64() * 1000.0;
        self.as_mut().rust_mut().startup_ms = ms;
        crate::log::line("app", &format!("First frame drawn {ms:.0} ms after start"));
    }
}

extern "C" {
    // qtglue.cpp
    fn turbine_copy_text(text: *const std::os::raw::c_char);
}

/// Applies a game's waiting changes; `n` of them go back on the count if it fails.
fn apply_job(game: Game, n: i32) -> Job {
    Box::new(move |w: &mut Worker| {
        let result = w.slot(&game).and_then(|s| s.store.apply().map_err(|e| e.to_string()));
        crate::actions::sync_order(w, &game);
        match result {
            Ok(r) => {
                w.note(&game, &r);
                w.say(applied_words(&r, &game), "ok")
            }
            Err(e) => {
                w.post(move |mut t| {
                    let p = *t.pending() + n;
                    t.as_mut().set_pending(p);
                });
                w.say(format!("Couldn't apply the changes: {e}. Nothing was lost; Turbine tries again after the next change."), "error");
            }
        }
        w.show(&game, false);
    })
}

fn launch(w: &Worker, game: &Game) {
    match rules::launch_url(game) {
        Some(url) => {
            crate::log::line("launch", &format!("{} through {url}", game.name));
            w.post(move |t| t.launch(QString::from(url.as_str())))
        }
        None => w.say(format!("Turbine doesn't know how to start {} yet. Start it from its launcher.", game.name), "warn"),
    }
}
