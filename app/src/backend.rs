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
        #[qproperty(QString, pad_name, cxx_name = "padName")]
        #[qproperty(QString, mods_json, cxx_name = "modsJson")]
        #[qproperty(QString, rules_json, cxx_name = "rulesJson")]
        #[qproperty(QString, downloads_json, cxx_name = "downloadsJson")]
        #[qproperty(QString, check_json, cxx_name = "checkJson")]
        #[qproperty(bool, busy)]
        #[qproperty(i32, pending)]
        /// The game the waiting changes belong to (the header only counts them on that game).
        #[qproperty(QString, pending_game, cxx_name = "pendingGame")]
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
        #[qproperty(QString, status_json, cxx_name = "statusJson")]
        #[qproperty(QString, browser_json, cxx_name = "browserJson")]
        #[qproperty(QString, bridge_json, cxx_name = "bridgeJson")]
        #[qproperty(f64, pointer_x, cxx_name = "pointerX")]
        #[qproperty(f64, pointer_y, cxx_name = "pointerY")]
        #[qproperty(bool, pointer_shown, cxx_name = "pointerShown")]
        #[qproperty(bool, pointer_paused, cxx_name = "pointerPaused")]
        #[qproperty(bool, stick_dpad, cxx_name = "stickDpad")]
        /// While the quick wheel is open: the left stick's angle (0 Up, clockwise), -1 near the middle.
        #[qproperty(i32, wheel_angle, cxx_name = "wheelAngle")]
        #[qproperty(QString, labs_json, cxx_name = "labsJson")]
        /// Turbine's browser has the controller (its page is on screen, no sheet or wheel over it): A clicks at the
        /// pointer (QML sets this).
        #[qproperty(bool, browser_input, cxx_name = "browserInput")]
        /// The page has a focused link or button (the read-only probe, QML sets this): A is Enter on it after the
        /// D-pad moved the page's focus (browserKeyNav).
        #[qproperty(bool, browser_enter, cxx_name = "browserEnter")]
        /// The D-pad moved the page's focus since the last pointer move or click: A presses Enter ("A Press").
        #[qproperty(bool, browser_key_nav, cxx_name = "browserKeyNav")]
        /// Counts the player's presses in the page (A, RT, R3, a click or tap): QML restarts its timers on each.
        #[qproperty(i32, browser_press_count, cxx_name = "browserPressCount")]
        /// A file Turbine's browser downloaded was added or not: {"n","path","ok","ask","name","game","text"}.
        #[qproperty(QString, add_result_json, cxx_name = "addResultJson")]
        /// A Mod Manager Download link's mod name, for the question before it downloads: {"link","name"}.
        #[qproperty(QString, nxm_name_json, cxx_name = "nxmNameJson")]
        /// A Basic game's mod waiting for "Where do these files go?" or the question about the game's own files:
        /// {"game","gameName","pending":{"name","placement","sure","conflicts","refused","tree"}}, or "".
        #[qproperty(QString, place_json, cxx_name = "placeJson")]
        /// "Which game on Nexus is this?": the search's answer {"state","games":[{"name","domain","mods"}]}.
        #[qproperty(QString, nexus_games_json, cxx_name = "nexusGamesJson")]
        type Turbine = super::TurbineRust;

        /// Looks for games again (Steam and Heroic).
        #[qinvokable]
        fn scan(self: Pin<&mut Self>);

        /// Reads the controllers. QML calls this on a timer while the window is active.
        #[qinvokable]
        fn poll(self: Pin<&mut Self>);

        /// For the build's screenshot checks: shows the controller pointer at (x, y), then clicks there with the
        /// left button when `click`, and turns the wheel `wheel` steps (positive scrolls down), all through the same
        /// mouse and wheel events the sticks and triggers make.
        #[qinvokable]
        #[cxx_name = "selftestPointer"]
        fn selftest_pointer(self: Pin<&mut Self>, x: f64, y: f64, click: bool, wheel: i32);

        /// Turbine's window came back to the front: drops controller presses made meanwhile (in a game).
        /// R3 with the pointer hidden: shows it at (x, y) (the focused item's center, worked out in QML).
        #[qinvokable]
        #[cxx_name = "pointerShowAt"]
        fn pointer_show_at(self: Pin<&mut Self>, x: f64, y: f64);

        /// For the build's checks: a pad press or release through the same path a controller's takes.
        #[qinvokable]
        #[cxx_name = "selftestPad"]
        fn selftest_pad(self: Pin<&mut Self>, button: &QString, pressed: bool);

        /// Touch: the pointer goes (the ring follows the finger instead).
        #[qinvokable]
        #[cxx_name = "pointerHide"]
        fn pointer_hide(self: Pin<&mut Self>);

        #[qinvokable]
        fn wake(self: Pin<&mut Self>);

        /// The left stick's wheel while the pointer is hidden: `steps` notches (positive down) at (x, y), the
        /// focused item's middle. The pointer stays hidden.
        #[qinvokable]
        #[cxx_name = "wheelAt"]
        fn wheel_at(self: Pin<&mut Self>, x: f64, y: f64, steps: i32);

        /// Turbine went to the background: a direction held on the left stick is let go (and stays let go until
        /// the stick is back in the middle), so nothing repeats while a game or the overlay has the screen.
        #[qinvokable]
        fn sleep(self: Pin<&mut Self>);

        /// Shows this game's mods and rules (opens its store the first time).
        #[qinvokable]
        fn select(self: Pin<&mut Self>, game_id: &QString);

        // ------------------------------------------------ 0.56: mods for any game (anygame.rs)

        /// The game's folders right in `rel` ("" for the game folder): [{"name","path","turbine"}].
        #[qinvokable]
        #[cxx_name = "gameFolders"]
        fn game_folders(self: &Self, rel: &QString) -> QString;
        /// What putting the waiting mod's `from` folder into the game's `to` would do:
        /// {"error","files","conflicts":[...],"refused":[{"path","why"}]}.
        #[qinvokable]
        #[cxx_name = "placeCheck"]
        fn place_check(self: &Self, from: &QString, to: &QString) -> QString;
        /// Adds the waiting mod there. `replace`: it may replace files the game already has.
        #[qinvokable]
        #[cxx_name = "finishPlace"]
        fn finish_place(self: Pin<&mut Self>, from: &QString, to: &QString, replace: bool);
        /// Drops the waiting mod (the file stays in Downloads).
        #[qinvokable]
        #[cxx_name = "cancelPlace"]
        fn cancel_place(self: Pin<&mut Self>);
        /// Moves a Basic game's mod to another folder of the game.
        #[qinvokable]
        #[cxx_name = "changePlacement"]
        fn change_placement(self: Pin<&mut Self>, id: &QString, to: &QString);
        /// Lets a Basic game's mod replace the game's own files, or not.
        #[qinvokable]
        #[cxx_name = "setReplace"]
        fn set_replace(self: Pin<&mut Self>, id: &QString, replace: bool);
        /// The player's answer to "Which game on Nexus is this?" (domain "" for "It isn't on Nexus").
        #[qinvokable]
        #[cxx_name = "pickNexusGame"]
        fn pick_nexus_game(self: Pin<&mut Self>, domain: &QString, name: &QString);
        /// Searches Nexus's games by name; the answer lands in nexusGamesJson.
        #[qinvokable]
        #[cxx_name = "searchNexusGames"]
        fn search_nexus_games(self: Pin<&mut Self>, text: &QString);

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

        /// The folder a mod's files are kept in (the mod menu's Open folder), or "" when there is none.
        #[qinvokable]
        #[cxx_name = "modFolder"]
        fn mod_folder(self: &Self, id: &QString) -> QString;
        /// 0.56: the current game's save backups (health.rs).
        #[qinvokable]
        #[cxx_name = "savesJson"]
        fn saves_json(self: &Self) -> QString;
        /// 0.56: puts a save backup back (the saves as they are now are kept first).
        #[qinvokable]
        #[cxx_name = "restoreSaves"]
        fn restore_saves(self: Pin<&mut Self>, id: &QString);
        /// 0.57: puts a save backup back without keeping the saves as they are now (after `savesKeepFailed` and the
        /// player's confirm).
        #[qinvokable]
        #[cxx_name = "restoreSavesAnyway"]
        fn restore_saves_anyway(self: Pin<&mut Self>, id: &QString);
        /// 0.56: Why won't it start? and Check game folder (read only).
        #[qinvokable]
        #[cxx_name = "gameHealth"]
        fn game_health(self: &Self) -> QString;
        /// The files a mod puts in the game (game-relative, A to Z, at most 2,000), as a JSON list.
        #[qinvokable]
        #[cxx_name = "modFiles"]
        fn mod_files(self: &Self, id: &QString) -> QString;

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

        /// Kingdom Come: puts the player's own Mods/mod_order.txt back as it was before Turbine changed it.
        #[qinvokable]
        #[cxx_name = "kcdRestore"]
        fn kcd_restore(self: Pin<&mut Self>);

        /// The check before Play for the current game. The answer arrives as `checked`.
        #[qinvokable]
        fn check(self: Pin<&mut Self>);

        /// Applies, then asks QML to open the game's launch link (`launch`).
        #[qinvokable]
        fn play(self: Pin<&mut Self>, game_id: &QString);

        /// 0.58: Play Seamless Co-op for the current game (through Steam and me3).
        #[qinvokable]
        #[cxx_name = "playCoop"]
        fn play_coop(self: Pin<&mut Self>);

        /// 0.6: Mods off or Mods on for a game, asked from Steam's menu while Turbine is open.
        #[qinvokable]
        #[cxx_name = "quickMods"]
        fn quick_mods(self: Pin<&mut Self>, game_id: &QString, on: bool);

        /// 0.58: a me3 game started outside Steam (Desktop Mode only).
        #[qinvokable]
        #[cxx_name = "playOutsideSteam"]
        fn play_outside_steam(self: Pin<&mut Self>, coop: bool);

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

        /// 0.6: Decky Loader and Turbine's Steam menu plugin on this device: {"checked","decky","plugin","url"}.
        #[qinvokable]
        #[cxx_name = "deckyJson"]
        fn decky_json(self: &Self) -> QString;

        /// Labs > Activity log: the last `lines` lines of the activity log, oldest first, one per line.
        #[qinvokable]
        #[cxx_name = "activityLog"]
        fn activity_log(self: &Self, lines: i32) -> QString;

        /// Labs > Activity log: saves the last 500 lines into Downloads. Answers {"file": name, "error": ""}.
        #[qinvokable]
        #[cxx_name = "saveActivityLog"]
        fn save_activity_log(self: &Self) -> QString;

        /// 0.58, Settings > Report a problem: saves Turbine-report-<stamp>.txt into Downloads with the recent log,
        /// the version, the settings QML sends (one "name: value" per line), the screen and every game with how
        /// Turbine plays it. Keys, tokens and cookies are hidden. Answers {"file","folder","error"}.
        #[qinvokable]
        #[cxx_name = "saveProblemReport"]
        fn save_problem_report(self: &Self, screen: &QString, settings: &QString) -> QString;

        /// 0.58, Edit settings: a mod's settings files ("" for the game's own): [{"rel","edited"}].
        #[qinvokable]
        #[cxx_name = "settingsFiles"]
        fn settings_files(self: &Self, mod_id: &QString) -> QString;

        /// Reads one settings file for the page on its own thread; the answer comes as `settingsLoaded`.
        #[qinvokable]
        #[cxx_name = "settingsLoad"]
        fn settings_load(self: Pin<&mut Self>, mod_id: &QString, rel: &QString);

        /// Saves the page's changes ({"fields":[{"section","key","occ","was","value"}]} or
        /// {"lines":[{"line","was","text"}]}), matched again in the file as it is now: {"ok","error"}.
        #[qinvokable]
        #[cxx_name = "settingsSave"]
        fn settings_save(self: &Self, mod_id: &QString, rel: &QString, edits: &QString) -> QString;

        /// Reset to default (the version before kept as a backup): {"ok","error"}.
        #[qinvokable]
        #[cxx_name = "settingsReset"]
        fn settings_reset(self: &Self, mod_id: &QString, rel: &QString) -> QString;

        /// 0.58: Elden Ring's saves, normal, modded and co-op apart, each with its backups.
        #[qinvokable]
        #[cxx_name = "erSavesJson"]
        fn er_saves_json(self: &Self) -> QString;

        /// Puts back one kind of Elden Ring save ("solo", "modded", "coop") from a backup.
        #[qinvokable]
        #[cxx_name = "restoreSaveKind"]
        fn restore_save_kind(self: Pin<&mut Self>, kind: &QString, id: &QString);

        /// The text on the clipboard ("" when there is none). Never logged.
        #[qinvokable]
        #[cxx_name = "clipboardText"]
        fn clipboard_text(self: &Self) -> QString;

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

        /// One page of Get mods from `offset` (infinite scroll).
        #[qinvokable]
        #[cxx_name = "browsePage"]
        fn browse_page(self: Pin<&mut Self>, text: &QString, category: &QString, sort: &QString, source: &QString, offset: i32);

        /// Labs: browse Nexus once without a key; the answer lands in `labsJson`.
        #[qinvokable]
        #[cxx_name = "nexusKeylessCheck"]
        fn nexus_keyless_check(self: Pin<&mut Self>);

        /// The screenshot checks: browsing Nexus fails as if Nexus refused it.
        #[qinvokable]
        #[cxx_name = "setSampleNexusFail"]
        fn set_sample_nexus_fail(self: Pin<&mut Self>);

        /// Recently removed: delete one mod for good, or all of them.
        #[qinvokable]
        #[cxx_name = "purgeRemoved"]
        fn purge_removed(self: Pin<&mut Self>, id: &QString);
        #[qinvokable]
        #[cxx_name = "purgeAllRemoved"]
        fn purge_all_removed(self: Pin<&mut Self>);

        /// Opens a web page: in Game Mode through the player's browser in Steam and the helper (launch.rs). Answers
        /// {"how": "shortcut" | "noshortcut" | "desktop", "name", "error", ...}; with "desktop" QML opens it.
        #[qinvokable]
        #[cxx_name = "openUrl"]
        fn open_url(self: Pin<&mut Self>, url: &QString) -> QString;

        /// Is there a Chrome-family browser among Steam's non-Steam games? {"found":false} or
        /// {"found":true,"browser","name","appid","gameid","url"} (read only, logged).
        #[qinvokable]
        #[cxx_name = "browserShortcut"]
        fn browser_shortcut(self: &Self) -> QString;

        /// Steam's own browser with this page: answers the steam://openurl link for QML to open (as Play opens
        /// steam://rungameid), or "" when the address can't be opened that way.
        #[qinvokable]
        #[cxx_name = "openInSteam"]
        fn open_in_steam(self: Pin<&mut Self>, url: &QString) -> QString;

        /// An HDMI or DisplayPort display is connected.
        #[qinvokable]
        #[cxx_name = "externalDisplay"]
        fn external_display(self: &Self) -> bool;

        #[qinvokable]
        fn details(self: Pin<&mut Self>, id: &QString, source: &QString);

        /// Downloads a mod (and what it needs) and adds it to the current game.
        #[qinvokable]
        fn install(self: Pin<&mut Self>, id: &QString, source: &QString);

        #[qinvokable]
        #[cxx_name = "installLoader"]
        fn install_loader(self: Pin<&mut Self>);

        /// 0.57, anti-cheat games: the ask-once answer (true: off while mods are on, me3 starts it offline).
        #[qinvokable]
        #[cxx_name = "antiCheatOff"]
        fn anti_cheat_off(self: Pin<&mut Self>, on: bool);

        /// Gets me3 from its pinned GitHub release (SHA-256 checked); plays after when `then_play`.
        #[qinvokable]
        #[cxx_name = "installMe3"]
        fn install_me3(self: Pin<&mut Self>, then_play: bool);

        /// Gets Seamless Co-op from its pinned GitHub release (SHA-256 checked) and adds it like a loader.
        #[qinvokable]
        #[cxx_name = "installSeamless"]
        fn install_seamless(self: Pin<&mut Self>);

        /// Removes only another tool's leftover "<starter>.backup" (after a hold to confirm in the interface).
        #[qinvokable]
        #[cxx_name = "removeAcLeftover"]
        fn remove_ac_leftover(self: Pin<&mut Self>);

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
        #[cxx_name = "profileRename"]
        fn profile_rename(self: Pin<&mut Self>, from: &QString, to: &QString);
        /// Deletes a profile (never the one in use).
        #[qinvokable]
        #[cxx_name = "profileDelete"]
        fn profile_delete(self: Pin<&mut Self>, name: &QString);
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

        // ------------------------------------------------ Nexus inside Turbine (web.rs, browse.rs)

        /// The browser is on screen or closed (the pointer stays while it is open).
        #[qinvokable]
        #[cxx_name = "browserMode"]
        fn browser_mode(self: Pin<&mut Self>, on: bool);
        /// An arrow key into the page ("up", "down", "left", "right").
        #[qinvokable]
        #[cxx_name = "browserKey"]
        fn browser_key(self: Pin<&mut Self>, dir: &QString);
        /// A page up or down into the page ("pageup", "pagedown"): the D-pad held for a while.
        #[qinvokable]
        #[cxx_name = "browserPageKey"]
        fn browser_page_key(self: Pin<&mut Self>, dir: &QString);
        /// A press and release of A on the page (the build's scripted presses).
        #[qinvokable]
        #[cxx_name = "browserClick"]
        fn browser_click(self: Pin<&mut Self>);
        /// The file page a mod's Get it opens.
        #[qinvokable]
        #[cxx_name = "browserStartUrl"]
        fn browser_start_url(self: &Self, domain: &QString, mod_id: f64, file_id: f64) -> QString;
        /// Is this page Nexus's (or the build's stand-in)?
        #[qinvokable]
        #[cxx_name = "browserIsNexus"]
        fn browser_is_nexus(self: &Self, url: &QString) -> bool;
        /// How large pages start for this window height and Interface size.
        #[qinvokable]
        #[cxx_name = "browserZoomDefault"]
        fn browser_zoom_default(self: &Self, height: f64, size: f64) -> f64;
        /// A Mod Manager Download link the navigation guard caught: true lets the navigation go on to the scheme
        /// handler (the main frame, on Nexus), false stops it there.
        #[qinvokable]
        #[cxx_name = "browserNxm"]
        fn browser_nxm(self: Pin<&mut Self>, url: &QString, main_frame: bool, page_url: &QString) -> bool;
        /// A mouse button or a finger went down on the page.
        #[qinvokable]
        #[cxx_name = "browserPointerDown"]
        fn browser_pointer_down(self: &Self);
        /// ... and was let go as a tap or click (the controller's presses are counted in Rust).
        #[qinvokable]
        #[cxx_name = "browserPressed"]
        fn browser_pressed(self: Pin<&mut Self>);
        /// The main frame is going to (or loaded) this address (a press made before it left Nexus no longer counts).
        #[qinvokable]
        #[cxx_name = "browserMainFrame"]
        fn browser_main_frame(self: &Self, url: &QString);
        /// Must Turbine ask before this link downloads (every link from Turbine's page)?
        #[qinvokable]
        #[cxx_name = "nxmMustAsk"]
        fn nxm_must_ask(self: &Self, link: &QString) -> bool;
        /// Looks up the link's mod and file names (land in nxmNameJson).
        #[qinvokable]
        #[cxx_name = "nxmAskName"]
        fn nxm_ask_name(self: Pin<&mut Self>, link: &QString);
        /// An address for the activity log: host and path only.
        #[qinvokable]
        #[cxx_name = "browserLogUrl"]
        fn browser_log_url(self: &Self, url: &QString) -> QString;
        /// The same page, the query's parts in any order?
        #[qinvokable]
        #[cxx_name = "browserSamePage"]
        fn browser_same_page(self: &Self, a: &QString, b: &QString) -> bool;
        /// The Nexus mod id of a page (0 when it isn't a mod's page).
        #[qinvokable]
        #[cxx_name = "browserPageMod"]
        fn browser_page_mod(self: &Self, url: &QString) -> f64;
        /// Signed in to Nexus in Turbine's browser (its sign-in cookie is there)?
        #[qinvokable]
        #[cxx_name = "browserSignedIn"]
        fn browser_signed_in(self: &Self) -> bool;
        /// A running download, twice a second: "" to go on, or why it must stop (past 64 GB, the disk nearly full).
        #[qinvokable]
        #[cxx_name = "browserDlCheck"]
        fn browser_dl_check(self: &Self, path: &QString, got: f64) -> QString;
        /// Where a download goes: {"ok","dir","name","path","game","folder","sure","mod","error"} (the file is marked
        /// as Turbine's). `pending`: bytes running downloads still have to come.
        #[qinvokable]
        #[cxx_name = "browserPlan"]
        fn browser_plan(self: Pin<&mut Self>, url: &QString, suggested: &QString, page_url: &QString, total: f64, pending: f64) -> QString;
        /// A download finished: added to its game.
        #[qinvokable]
        #[cxx_name = "browserDone"]
        fn browser_done(self: Pin<&mut Self>, path: &QString);
        /// A download stopped ("interrupted: <reason>" or "cancelled").
        #[qinvokable]
        #[cxx_name = "browserFailed"]
        fn browser_failed(self: Pin<&mut Self>, path: &QString, why: &QString);
        /// Settings, Accounts: Sign out of Nexus in Turbine.
        #[qinvokable]
        #[cxx_name = "browserSignOut"]
        fn browser_sign_out(self: Pin<&mut Self>);
        /// Labs: Nexus page: draw without the GPU (from the next start).
        #[qinvokable]
        #[cxx_name = "setWebNoGpu"]
        fn set_web_no_gpu(self: Pin<&mut Self>, on: bool);
        #[qinvokable]
        #[cxx_name = "webNoGpu"]
        fn web_no_gpu(self: &Self) -> bool;
        /// Sign in with my browser (Google and others): {"how","url","steam","text"}.
        #[qinvokable]
        #[cxx_name = "signinStart"]
        fn signin_start(self: Pin<&mut Self>) -> QString;
        /// The build's checks of the Downloads watcher's fallback (test runs only).
        #[qinvokable]
        #[cxx_name = "selftestCatch"]
        fn selftest_catch(self: Pin<&mut Self>, step: &QString) -> QString;

        // ------------------------------------------------ catching browser downloads (catch.rs)

        /// Once at start: the browser helper's link and the browser check.
        #[qinvokable]
        #[cxx_name = "startCatching"]
        fn start_catching(self: Pin<&mut Self>);
        /// Looks for a web browser again; the answer lands in `browserJson`.
        #[qinvokable]
        #[cxx_name = "browserCheck"]
        fn browser_check(self: Pin<&mut Self>);
        /// A Nexus mod page is being opened: watch Downloads for its file. {"watching","game","error"}.
        #[qinvokable]
        #[cxx_name = "watchPage"]
        fn watch_page(self: Pin<&mut Self>, url: &QString) -> QString;
        /// The player's answer to "Found <file>. Add it to <game>?": yes adds it to that game, no means it isn't a
        /// mod (never asked about again).
        #[qinvokable]
        #[cxx_name = "watchConfirm"]
        fn watch_confirm(self: Pin<&mut Self>, path: &QString, yes: bool);
        /// The player picked another game for a found file (X): adds it there.
        #[qinvokable]
        #[cxx_name = "catchPick"]
        fn catch_pick(self: Pin<&mut Self>, path: &QString, game_id: &QString);
        /// Labs: watch for a download from Steam's own browser; answers the steam:// address to open.
        #[qinvokable]
        #[cxx_name = "watchSteamBrowser"]
        fn watch_steam_browser(self: Pin<&mut Self>) -> QString;
        #[qinvokable]
        #[cxx_name = "bridgeRefresh"]
        fn bridge_refresh(self: Pin<&mut Self>);
        /// The pairing page's address (with its token), "" when the link isn't running.
        #[qinvokable]
        #[cxx_name = "bridgePairUrl"]
        fn bridge_pair_url(self: Pin<&mut Self>) -> QString;
        /// A new token (the old pairing stops working), then the pairing page's address.
        #[qinvokable]
        #[cxx_name = "bridgePairAgain"]
        fn bridge_pair_again(self: Pin<&mut Self>) -> QString;
        /// Copies the browser helper into Downloads: {"folder","error"}.
        #[qinvokable]
        #[cxx_name = "helperSetup"]
        fn helper_setup(self: Pin<&mut Self>) -> QString;
        /// Game Mode: marks the overlay window for gamescope (on) or takes the mark off.
        #[qinvokable]
        #[cxx_name = "overlayMark"]
        fn overlay_mark(self: Pin<&mut Self>, on: bool) -> i32;
        /// The overlay's result was shown long enough.
        #[qinvokable]
        #[cxx_name = "statusDone"]
        fn status_done(self: Pin<&mut Self>);

        /// Play options: Play without the script extender (or the game's mod loader) and the mods that need it, this
        /// one time.
        #[qinvokable]
        #[cxx_name = "playWithoutLoader"]
        fn play_without_loader(self: Pin<&mut Self>);
        /// The game closed right after a modded Play: that setup no longer counts as one that worked.
        #[qinvokable]
        #[cxx_name = "markNotWorking"]
        fn mark_not_working(self: Pin<&mut Self>);
        /// Free accounts' Update: the next download of this mod's Nexus page replaces the installed mod `mod_id`.
        #[qinvokable]
        #[cxx_name = "expectUpdate"]
        fn expect_update(self: Pin<&mut Self>, mod_id: &QString);

        /// After a modded Play: the game ran `seconds` (known: Turbine could see the game's process; otherwise the
        /// interface measures the time it was away).
        #[qsignal]
        #[cxx_name = "gameEnded"]
        fn game_ended(self: Pin<&mut Self>, game: QString, seconds: i32, known: bool);

        /// A mod was added with notes the player may want to act on: its name, then the notes as a JSON list.
        #[qsignal]
        fn added(self: Pin<&mut Self>, name: QString, notes: QString);

        /// A finished download Turbine isn't sure about: {"path","name","size","game","gameName","why","more"}
        /// ("game" is "" when no game fits; "more": how many others wait after it).
        #[qsignal]
        fn asked(self: Pin<&mut Self>, json: QString);

        /// A controller button went down or up: "a", "b", "x", "y", "lb", "rb", "lt", "rt", "start", "view", "r3",
        /// "up", "down", "left", "right" (the D-pad, and the left stick pushed and held one way). The triggers and
        /// R3 come here only while the pointer is hidden; while it shows they click (RT and R3 left, LT right).
        #[qsignal]
        fn pad(self: Pin<&mut Self>, button: QString, pressed: bool);

        /// A mod was just added (a caught download, a Premium or Thunderstore install, Add from Downloads): the
        /// game's id, the mod's id and name, and whether it is on.
        #[qsignal]
        fn arrived(self: Pin<&mut Self>, game: QString, id: QString, name: QString, on: bool);

        /// A sentence for the status line. level: "work" (in progress), "ok", "warn" or "error".
        #[qsignal]
        fn message(self: Pin<&mut Self>, text: QString, level: QString);

        /// 0.57: putting a save backup back stopped because the saves as they are now couldn't be kept first: the
        /// backup's id and why (the interface asks whether to put it back anyway).
        #[qsignal]
        #[cxx_name = "savesKeepFailed"]
        fn saves_keep_failed(self: Pin<&mut Self>, id: QString, why: QString);

        /// The check before Play finished; the same JSON as `checkJson`.
        #[qsignal]
        fn checked(self: Pin<&mut Self>, json: QString);

        /// The sign-in from the player's own browser went into Turbine's browser: what to say ("Signed in to Nexus as
        /// <name>"), and how to come back to the front in Game Mode (a steam:// link, or "" to tell the player).
        #[qsignal]
        #[cxx_name = "webSignedIn"]
        fn web_signed_in(self: Pin<&mut Self>, text: QString, back: QString);

        /// The sign-in from the player's own browser: where it is or what went wrong ("wait", "ok" or "warn"), for
        /// the pill and a toast. Never carries a cookie or key.
        #[qsignal]
        #[cxx_name = "webSigninNote"]
        fn web_signin_note(self: Pin<&mut Self>, text: QString, level: QString);

        /// Pages to open in the browser (free accounts): title, then JSON [{"name","url"}].
        #[qsignal]
        fn pages(self: Pin<&mut Self>, title: QString, json: QString);

        /// Open this link to start the game (steam://... or Heroic's link).
        #[qsignal]
        fn launch(self: Pin<&mut Self>, url: QString);

        /// 0.58: a me3 game's Steam Launch Options don't run Turbine's helper yet, so Play didn't start it:
        /// {"game","name","option","set","listed","coop"} for the one-time step.
        #[qsignal]
        #[cxx_name = "launchSetup"]
        fn launch_setup(self: Pin<&mut Self>, game: QString, json: QString);

        /// 0.58: one settings file read for the Edit settings page (see settingsLoad).
        #[qsignal]
        #[cxx_name = "settingsLoaded"]
        fn settings_loaded(self: Pin<&mut Self>, json: QString);

        /// 0.58: Play without mods until they update (the game update warning), for Turbine's and Steam's Play.
        #[qinvokable]
        #[cxx_name = "setModHold"]
        fn set_mod_hold(self: Pin<&mut Self>, on: bool);

        /// 0.58: the me3 games whose Steam Launch Options run Turbine's helper, with what to paste back:
        /// [{"game","name","uses","set","original","line"}] (only the current game when `current`).
        #[qinvokable]
        #[cxx_name = "launchUndoJson"]
        fn launch_undo_json(self: &Self, current: bool) -> QString;

        /// 0.58: after Undo, "Use the line again": the helper may start me3 for this game again.
        #[qinvokable]
        #[cxx_name = "launchRedo"]
        fn launch_redo(self: Pin<&mut Self>, game: &QString);

        /// 0.58: Undo the launch options change: the helper stops starting me3 for this game (logged).
        #[qinvokable]
        #[cxx_name = "launchUndo"]
        fn launch_undo(self: Pin<&mut Self>, game: &QString);

        /// 0.6: one Steam restart for a change to Steam's own settings ("options", "undo", "tile", "untile");
        /// `resume` "play", "coop" or "". Answers {"ok","state","words"} (steamsetup.rs).
        #[qinvokable]
        #[cxx_name = "steamSetupStart"]
        fn steam_setup_start(self: &Self, game_id: &QString, kind: &QString, resume: &QString) -> QString;

        /// 0.6: the Steam restart's state: {"state","words","resume","what","age"}.
        #[qinvokable]
        #[cxx_name = "steamSetupState"]
        fn steam_setup_state(self: &Self) -> QString;

        /// 0.6: the Steam restart is over: its files go (its log lines go to the activity log).
        #[qinvokable]
        #[cxx_name = "steamSetupFinish"]
        fn steam_setup_finish(self: &Self);

        /// 0.6: at start, changes to Steam's settings that Steam wrote over: {"lost":[names]}.
        #[qinvokable]
        #[cxx_name = "steamChangesCheck"]
        fn steam_changes_check(self: &Self) -> QString;

        /// 0.6: the co-op tile for a game: {"there"}.
        #[qinvokable]
        #[cxx_name = "coopTileJson"]
        fn coop_tile_json(self: &Self, game_id: &QString) -> QString;

        /// 0.6, Settings: Turbine's Steam library art: {"found","on"}.
        #[qinvokable]
        #[cxx_name = "steamArtJson"]
        fn steam_art_json(self: &Self) -> QString;

        /// 0.6, Settings: Steam library art Add (true) or Remove (false). Answers the words.
        #[qinvokable]
        #[cxx_name = "steamArtSet"]
        fn steam_art_set(self: &Self, on: bool) -> QString;

        /// 0.6, Labs: Proton's log for Steam launches of me3 games, in the helper's folder.
        #[qinvokable]
        #[cxx_name = "protonLogOn"]
        fn proton_log_on(self: &Self) -> bool;

        #[qinvokable]
        #[cxx_name = "setProtonLog"]
        fn set_proton_log(self: &Self, on: bool);

        /// 0.58: the game wasn't seen running about 20 seconds after Play.
        #[qsignal]
        #[cxx_name = "notStarted"]
        fn not_started(self: Pin<&mut Self>, game: QString);
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
    /// The first controller SDL sees ("" when none), so the button glyphs can follow it.
    pub(crate) pad_name: QString,
    pub(crate) mods_json: QString,
    pub(crate) rules_json: QString,
    pub(crate) downloads_json: QString,
    pub(crate) check_json: QString,
    pub(crate) busy: bool,
    pub(crate) pending: i32,
    pub(crate) pending_game: QString,
    /// Free accounts' updates waiting for their download: (Nexus mod id, installed mod id).
    pub(crate) update_waits: Vec<(u64, String)>,
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
    pub(crate) status_json: QString,
    pub(crate) browser_json: QString,
    pub(crate) bridge_json: QString,
    pub(crate) catching: crate::catch::Catching,
    pub(crate) pointer_x: f64,
    pub(crate) pointer_y: f64,
    pub(crate) pointer_shown: bool,
    pub(crate) pointer_paused: bool,
    /// The left stick is only a D-pad (no scroll wheel) while the quick wheel is open.
    pub(crate) stick_dpad: bool,
    pub(crate) wheel_angle: i32,
    /// Which sources hold each mouse button down: [left, right], a bit per source (RT 1, R3 2 on the left; LT 1
    /// on the right). The button is let go only when the last source lets go.
    pub(crate) mouse_src: [u8; 2],
    /// The controller pointer: where it is, the left stick's wheel, and the last look's time.
    pub(crate) pointer: crate::pointer::Pointer,
    pub(crate) pointer_at: Option<std::time::Instant>,
    /// Mouse buttons a trigger holds down: [left (RT), right (LT)].
    pub(crate) mouse_down: [bool; 2],
    /// Looks until the pointer's place is sent again after a click (what was under it may have been redrawn, so
    /// its outline comes back without moving the stick).
    pub(crate) rehover: u8,
    /// Time since the pointer's place was last sent: while it shows and rests, it is sent again now and then, so
    /// a list redrawn under it (a mod turned on, a sheet opened) gets its outline back.
    pub(crate) resend_ms: f64,
    pub(crate) labs_json: QString,
    pub(crate) browser_input: bool,
    pub(crate) browser_enter: bool,
    pub(crate) browser_key_nav: bool,
    pub(crate) browser_press_count: i32,
    pub(crate) add_result_json: QString,
    pub(crate) nxm_name_json: QString,
    /// A is Enter only after the D-pad (pointer.rs).
    pub(crate) key_nav: crate::pointer::KeyNav,
    pub(crate) add_results: i32,
    /// Presses of A, RT and R3 in the browser, and the clicks they made (the build checks the two match).
    pub(crate) browser_presses: u32,
    pub(crate) browser_clicks: u32,
    /// A is holding Enter down (not the mouse button).
    pub(crate) a_enter: bool,
    /// The last A press went to the page (its release goes there too).
    pub(crate) a_browser: bool,
    /// Downloads Turbine's browser accepted and hasn't finished adding.
    pub(crate) web_downloads: Vec<crate::web::WebDownload>,
    pub(crate) place_json: QString,
    pub(crate) nexus_games_json: QString,
    /// The waiting placement's archive files and game (for placeCheck).
    pub(crate) place_files: Vec<String>,
    pub(crate) place_game: String,
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
            pad_name: QString::default(),
            mods_json: QString::default(),
            rules_json: QString::default(),
            downloads_json: QString::from("[]"),
            check_json: QString::from("[]"),
            busy: false,
            pending: 0,
            pending_game: QString::default(),
            update_waits: Vec::new(),
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
            status_json: QString::from("{\"active\":false}"),
            browser_json: QString::from("{\"checked\":false}"),
            bridge_json: QString::from("{\"running\":false}"),
            labs_json: QString::default(),
            catching: crate::catch::Catching::default(),
            pointer_x: -1.0,
            pointer_y: -1.0,
            pointer_shown: false,
            pointer_paused: false,
            stick_dpad: false,
            wheel_angle: -1,
            mouse_src: [0, 0],
            pointer: crate::pointer::Pointer::default(),
            pointer_at: None,
            mouse_down: [false, false],
            rehover: 0,
            resend_ms: 0.0,
            browser_input: false,
            browser_enter: false,
            browser_key_nav: false,
            browser_press_count: 0,
            add_result_json: QString::from("{}"),
            nxm_name_json: QString::from("{}"),
            key_nav: crate::pointer::KeyNav::default(),
            add_results: 0,
            browser_presses: 0,
            browser_clicks: 0,
            a_enter: false,
            a_browser: false,
            web_downloads: Vec::new(),
            place_json: QString::default(),
            nexus_games_json: QString::from("{\"state\":\"idle\",\"games\":[]}"),
            place_files: Vec::new(),
            place_game: String::new(),
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
    /// The window's thread queue. None without a window (Steam's menu, quick.rs): `headless` gets what it would.
    pub(crate) qt: Option<CxxQtThread<qobject::Turbine>>,
    /// 0.6: run from Steam's menu without a window: what Turbine said and the launch link, for the answer.
    pub(crate) headless: Option<Headless>,
    /// Installers waiting for the player's choices (game id, installer), oldest first.
    pub(crate) pending: std::collections::VecDeque<(String, store::Pending, u64)>,
    /// Installer sessions are numbered for the browser helper; the last number given, and the one it was sent.
    pub(crate) next_session: u64,
    pub(crate) fomod_sent: Option<u64>,
    /// Games whose "The Witcher 3 hasn't made its settings folder" was said this run.
    pub(crate) witcher_said: std::collections::HashSet<String>,
    /// A profile share being imported (game id, share).
    pub(crate) share: Option<(String, turbine_engine::profiles::Share)>,
    /// LOOT's messages per game: (what the plugins looked like when they were read, plugin name lowercased: messages).
    pub(crate) loot: HashMap<String, (String, HashMap<String, Vec<crate::loot::Msg>>)>,
    /// Basic games' mods waiting for where their files go (game id, mod), oldest first.
    pub(crate) place_pending: std::collections::VecDeque<(String, store::PendingPlace)>,
}

pub(crate) type Job = Box<dyn FnOnce(&mut Worker) + Send>;

/// 0.6: what a run without a window collects in place of the interface (quick.rs answers with it).
#[derive(Default)]
pub(crate) struct Headless {
    /// Every sentence for the status line, with its level ("ok", "warn", "error", "work").
    pub(crate) said: std::sync::Mutex<Vec<(String, &'static str)>>,
    /// The link Play would have opened (steam://rungameid or heroic://launch).
    pub(crate) launch: std::sync::Mutex<Option<String>>,
}

pub(crate) struct Task {
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
        if let Some(qt) = &self.qt {
            let _ = qt.queue(f);
        }
    }

    /// 0.6: a worker with no window, for one request from Steam's menu (quick.rs). Same jobs, same stores.
    pub(crate) fn headless(data_home: PathBuf) -> Worker {
        Worker {
            data_home,
            slots: HashMap::new(),
            qt: None,
            headless: Some(Headless::default()),
            pending: std::collections::VecDeque::new(),
            next_session: 0,
            fomod_sent: None,
            witcher_said: std::collections::HashSet::new(),
            share: None,
            loot: HashMap::new(),
            place_pending: std::collections::VecDeque::new(),
        }
    }

    /// Remembers whether this game's records need a Reset (shown in the game menu).
    pub(crate) fn note(&self, game: &Game, report: &turbine_engine::deploy::Report) {
        let key = game.id();
        let needs = needs_reset(report);
        let words = format!("{} at {}: {report:?}", game.name, crate::log::file_stamp());
        crate::log::line("engine", &format!("Report for {key}: {report:?}"));
        // 0.56: Basic games' mods leave the game's own files alone (unless the player said so) and never place the
        // refused files.
        if !report.skipped_existing.is_empty() {
            crate::log::line("engine", &format!("Skipped existing (the game's own files): {}", report.skipped_existing.join(", ")));
        }
        for r in &report.refused {
            crate::log::line("engine", &format!("Refused {r}: Turbine never places it"));
        }
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
        if let Some(h) = &self.headless {
            h.said.lock().unwrap_or_else(|e| e.into_inner()).push((text.clone(), level));
        }
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
            let slot = Slot { game: game.clone(), rules, store };
            // 0.57: an online-only anti-cheat game: mod files from an earlier version come out at once.
            crate::anticheat::refused_cleanup(self, &slot);
            self.slots.insert(key.clone(), slot);
        }
        Ok(self.slots.get_mut(&key).expect("just inserted"))
    }

    /// Sends the game's mods (and rules) to the interface if it's still the game on screen.
    pub(crate) fn show(&mut self, game: &Game, with_rules: bool) {
        let key = game.id();
        let Some(slot) = self.slots.get(&key) else { return };
        // 0.58: after anything that changed this game (mods on or off, the anti-cheat answer, Reset, a loadout, a
        // removed mod), the Steam launch helper carries exactly Turbine's state now.
        crate::anticheat::sync_helper(self, slot);
        let mods = slot.store.to_json();
        // Who wins shared files, and requirements both ways, for the list and the mod menu.
        let overlaps = turbine_engine::overlaps::to_json(&slot.store.overlaps());
        let links = turbine_engine::needs::links_json(&turbine_engine::needs::links(&slot.store.installed_sources()));
        let warns = mod_warns_json(slot);
        let restore = matches!(slot.rules.family, rules::Family::Kcd | rules::Family::Kcd2) && turbine_engine::kcd::can_restore(slot.store.root());
        // 0.58, the game update warning: the build changed since mods were last played, and which mods may break.
        // The build last played with mods (an apply alone doesn't count, so the warning isn't lost to one).
        let played = std::fs::read_to_string(slot.store.root().join("played-build")).ok().map(|b| b.trim().to_string()).filter(|b| !b.is_empty());
        // 0.6: an update still waiting in Steam (queued, downloading, or a failed one) warns too ("pending": the words).
        let pending = turbine_engine::steam::pending_update(&slot.game).map(|p| turbine_engine::steam::pending_update_words(&slot.game.name, &p));
        let risky = || -> String {
            let r: Vec<String> = slot
                .store
                .mod_files()
                .iter()
                .filter(|m| m.enabled && rules::may_break_on_update(&m.files, slot.store.mods().iter().any(|x| x.id == m.id && x.needs_loader)))
                .map(|m| format!("{{\"id\":{},\"name\":{}}}", quote(&m.id), quote(&m.name)))
                .collect();
            r.join(",")
        };
        let pending_json = pending.as_deref().map(quote).unwrap_or_else(|| "null".into());
        let update = match (played.or_else(|| slot.store.last_build()), slot.game.build.as_deref()) {
            (Some(last), Some(now)) if last != now => {
                format!("{{\"last\":{},\"now\":{},\"risky\":[{}],\"pending\":{pending_json}}}", quote(&last), quote(now), risky())
            }
            (_, now) if pending.is_some() => {
                let now = now.unwrap_or("");
                format!("{{\"last\":{},\"now\":{},\"risky\":[{}],\"pending\":{pending_json}}}", quote(now), quote(now), risky())
            }
            _ => "null".into(),
        };
        let mods = format!("{},\"overlaps\":{overlaps},\"links\":{links},\"warns\":{warns},\"kcdRestore\":{restore},\"gameUpdate\":{update}}}", &mods[..mods.len() - 1]);
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

/// A short word for a mod's row when it may not work in this game, the way the check before Play says it
/// ({"<mod id>": "Made for KCD"}): Kingdom Come mods made for the other game or missing what the game loads.
fn mod_warns_json(slot: &Slot) -> String {
    if !matches!(slot.rules.family, rules::Family::Kcd | rules::Family::Kcd2) {
        return "{}".into();
    }
    let second = slot.rules.family == rules::Family::Kcd2;
    let other_domain = if second { "kingdomcomedeliverance" } else { "kingdomcomedeliverance2" };
    let items: Vec<String> = slot
        .store
        .mod_files()
        .iter()
        .filter_map(|m| {
            let from_other = turbine_engine::nexus::source_mod(&slot.store.source_of(&m.id)).is_some_and(|(d, _)| d == other_domain);
            turbine_engine::kcd::mod_badge(&m.name, &m.files, second, from_other).map(|b| format!("{}:{}", quote(&m.id), quote(&b)))
        })
        .collect();
    format!("{{{}}}", items.join(","))
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
    let base = format!(
        "{{\"kind\":{},\"ordered\":{},\"plugins\":{},\"modRoot\":{},\"support\":{},\"loader\":{},\"loaderInstalled\":{},\"shortName\":{},\"launchOption\":{},\"launchHelp\":{},\"community\":{},\"domain\":{},\"kcdRestore\":{}}}",
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
        turbine_engine::nexus::domain_for(slot.game.store, &slot.game.store_id).map(quote).unwrap_or_else(|| "null".to_string()),
        matches!(r.family, rules::Family::Kcd | rules::Family::Kcd2) && turbine_engine::kcd::can_restore(slot.store.root())
    );
    // 0.56: support, anti-cheat, the Nexus game and starter mods (anygame.rs).
    format!("{}{}}}", &base[..base.len() - 1], crate::anygame::rules_extra(slot))
}

// ---------------------------------------------------------------------------------------------------- Proton

/// The Proton DLL overrides this game needs for what is turned on (proton::wanted_overrides): winhttp for BepInEx
/// once its loader is in place, winmm and version for Cyberpunk's RED4ext and Cyber Engine Tweaks. For Steam games
/// run through Proton and Heroic games run through Wine or Proton. Some(prefix) with the list when there is something
/// to set.
fn wanted_overrides(slot: &Slot) -> Option<(Option<PathBuf>, Vec<(&'static str, &'static str)>)> {
    let g = &slot.game;
    // 0.6: Heroic games too: their Wine or Proton prefix comes from Heroic's own game settings (heroic::prefix_for).
    if slot.rules.kind == Kind::BepInEx && !slot.rules.loader_installed(g) {
        return None;
    }
    let mut files: Vec<String> = slot.store.mod_files().into_iter().filter(|m| m.enabled).flat_map(|m| m.files).collect();
    files.extend(rules::present_files(g, &slot.rules));
    let wanted = proton::wanted_overrides(g, &slot.rules, &files);
    (!wanted.is_empty()).then(|| (g.prefix.clone(), wanted))
}

fn dll_words(wanted: &[(&str, &str)]) -> String {
    let names: Vec<&str> = wanted.iter().map(|(d, _)| *d).collect();
    match names.len() {
        1 => names[0].to_string(),
        _ => format!("{} and {}", names[..names.len() - 1].join(", "), names[names.len() - 1]),
    }
}

/// What the overrides are for, in the player's words.
fn overrides_for(slot: &Slot) -> &'static str {
    if slot.rules.kind == Kind::BepInEx {
        "BepInEx"
    } else {
        "the mod frameworks"
    }
}

/// Lines for the check before Play about the Proton settings, when there is something to say.
fn proton_checks(slot: &Slot) -> Vec<rules::Check> {
    let Some((prefix, wanted)) = wanted_overrides(slot) else { return Vec::new() };
    let line = |level: &str, title: String, detail: String| rules::Check { level: level.into(), title, detail };
    let name = &slot.game.name;
    let what = overrides_for(slot);
    let bepinex = slot.rules.kind == Kind::BepInEx;
    if crate::system::game_running(&slot.game.install_dir) == Some(true) {
        return vec![line(
            "stop",
            format!("{name} is still running"),
            format!("Close the game first. Turbine turns {what} on in the game's Proton settings, which only works while the game is closed."),
        )];
    }
    let Some(prefix) = prefix else {
        return vec![line(
            "warn",
            if bepinex { "BepInEx can't load yet".into() } else { "Mod frameworks can't load yet".into() },
            format!("Proton makes this game's settings the first time it runs. {} Until then mods that need {what} don't load.", proton::NOT_STARTED),
        )];
    };
    // A Heroic prefix in a folder Turbine can't reach (outside what the Flatpak may read): say which folder.
    if slot.game.store != turbine_engine::Store::Steam && !prefix.exists() && prefix.parent().is_some_and(|p| !p.exists()) {
        return vec![line(
            "warn",
            if bepinex { "BepInEx may not load".into() } else { "Mod frameworks may not load".into() },
            format!("Turbine can't reach this game's Wine folder {} to turn {what} on. In Heroic, use a Wine prefix under ~/Games/Heroic.", prefix.display()),
        )];
    }
    let mut missing = Vec::new();
    for (dll, value) in &wanted {
        match proton::read_override(&prefix, dll) {
            Ok(Some(v)) if v.eq_ignore_ascii_case(value) => {}
            Ok(_) => missing.push((*dll, *value)),
            Err(e) => return vec![line("warn", if bepinex { "BepInEx may not load".into() } else { "Mod frameworks may not load".into() }, e)],
        }
    }
    if missing.is_empty() {
        return Vec::new();
    }
    vec![line(
        "ok",
        if bepinex { "BepInEx gets turned on in Proton".into() } else { "Mod frameworks get turned on in Proton".into() },
        format!(
            "At Play, Turbine sets {} to \"native,builtin\" in this game's Proton settings so {what} {} A copy of the old settings is kept.",
            dll_words(&missing),
            if bepinex { "loads." } else { "load." }
        ),
    )]
}

/// The Witcher 3: writes mods.settings (the mods' order and which are on) in the game's Proton folder. Only while
/// the game is closed (it rewrites the file when it quits). `say` puts a plain message on the status line.
fn witcher_sync(w: &mut Worker, game: &Game, say: bool) {
    let Ok(slot) = w.slot(game) else { return };
    if matches!(slot.rules.family, rules::Family::Kcd | rules::Family::Kcd2) {
        kcd_sync(w, game, say);
        return;
    }
    if slot.rules.family != rules::Family::Witcher3 {
        return;
    }
    if crate::system::game_running(&game.install_dir) == Some(true) {
        crate::log::line("engine", "The Witcher 3 is running: mods.settings left for later");
        return;
    }
    let mods = slot.store.mod_files();
    let list: Vec<(&[String], bool)> = mods.iter().map(|m| (m.files.as_slice(), m.enabled)).collect();
    match turbine_engine::witcher::sync_mods_settings(game.prefix.as_deref(), &list) {
        Ok(changed) => crate::log::line("engine", &format!("The Witcher 3 mods.settings {}", if changed { "written" } else { "already up to date" })),
        Err(e) => {
            crate::log::line("engine", &format!("The Witcher 3 mods.settings not written: {e}"));
            let key = game.id();
            if say || !w.witcher_said.contains(&key) {
                w.witcher_said.insert(key);
                let words = if e == turbine_engine::witcher::NOT_STARTED {
                    format!("Mods are in place, but their order waits. {e}")
                } else {
                    format!("The mods' order for The Witcher 3 couldn't be saved ({e}).")
                };
                w.say(words, "warn");
            }
        }
    }
}

/// Kingdom Come: Deliverance I and II: writes Mods/mod_order.txt (the mods' order and which load). Only while the
/// game is closed.
fn kcd_sync(w: &mut Worker, game: &Game, say: bool) {
    let Ok(slot) = w.slot(game) else { return };
    if crate::system::game_running(&game.install_dir) == Some(true) {
        crate::log::line("engine", &format!("{} is running: mod_order.txt left for later", game.name));
        return;
    }
    let mods = slot.store.mod_files();
    let keep = slot.store.root().to_path_buf();
    let list: Vec<(&[String], bool)> = mods.iter().map(|m| (m.files.as_slice(), m.enabled)).collect();
    match turbine_engine::kcd::sync_mod_order(&game.install_dir, &keep, &list) {
        Ok(changed) => crate::log::line("engine", &format!("{} mod_order.txt {}", game.name, if changed { "written" } else { "already up to date" })),
        Err(e) => {
            crate::log::line("engine", &format!("{} mod_order.txt not written: {e}", game.name));
            if say {
                w.say(format!("The mods' order for {} couldn't be saved ({e}).", game.name), "warn");
            }
        }
    }
}

fn checks_json(checks: &[rules::Check]) -> String {
    checks_json_with(checks, &[])
}

/// `checks_json` with more fields for some lines: (index in `checks`, JSON fields to add, starting with a comma).
fn checks_json_with(checks: &[rules::Check], extra: &[(usize, String)]) -> String {
    // Lines the player needs to read only once per game (the interface hides them after that).
    let once = |t: &str| t == turbine_engine::cyberpunk::REDMOD_TITLE || t == LAUNCH_OPTION_TITLE || t == turbine_engine::kcd::DEVMODE_TITLE;
    let items: Vec<String> = checks
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let more = extra.iter().filter(|(j, _)| *j == i).map(|(_, x)| x.as_str()).collect::<String>();
            format!("{{\"level\":{},\"title\":{},\"detail\":{},\"once\":{}{more}}}", quote(&c.level), quote(&c.title), quote(&c.detail), once(&c.title))
        })
        .collect();
    format!("[{}]", items.join(","))
}

/// The Witcher 3's script clashes in the check: for each "Only one of these works" line, the newest-added mod of the
/// group to offer turning off ("offer") and the scripts' paths (for Game details).
fn witcher_extras(s: &Slot, list: &[rules::Check]) -> Vec<(usize, String)> {
    if s.rules.family != rules::Family::Witcher3 {
        return Vec::new();
    }
    let files = s.store.mod_files();
    let on: Vec<(&str, &[String])> = files.iter().filter(|m| m.enabled).map(|m| (m.name.as_str(), m.files.as_slice())).collect();
    let groups = turbine_engine::witcher::conflict_groups(&on);
    let at: Vec<usize> = list.iter().enumerate().filter(|(_, c)| c.title == turbine_engine::witcher::CONFLICT_TITLE).map(|(i, _)| i).collect();
    let mut out = Vec::new();
    for (i, (names, scripts)) in at.into_iter().zip(groups) {
        let newest = s
            .store
            .mods()
            .iter()
            .enumerate()
            .filter(|(_, m)| m.enabled && names.contains(&m.name))
            .max_by_key(|(n, m)| (m.added, *n))
            .map(|(_, m)| m);
        let offer = newest
            .map(|m| format!(",\"offer\":{{\"key\":\"y\",\"label\":{},\"mod\":{}}}", quote(&format!("Turn off {}", m.name)), quote(&m.id)))
            .unwrap_or_default();
        let paths: Vec<String> = scripts.iter().map(|p| quote(p)).collect();
        out.push((i, format!("{offer},\"paths\":[{}]", paths.join(","))));
    }
    out
}

/// Archives in Downloads and its Game Mods folders. `folder`: the Game Mods folder ("" in Downloads itself);
/// `here`: in Downloads itself or in the Game Mods folder of the game on screen (`here_folder`).
fn downloads_json(home: &Path, here_folder: &str) -> String {
    let items: Vec<String> = store::downloads_all(home)
        .iter()
        .map(|(p, folder)| {
            let meta = std::fs::metadata(p).ok();
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let modified = meta
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let here = folder.is_empty() || folder.eq_ignore_ascii_case(here_folder);
            format!(
                "{{\"name\":{},\"path\":{},\"size\":{},\"modified\":{},\"folder\":{},\"here\":{}}}",
                quote(&name),
                quote(&p.to_string_lossy()),
                size,
                modified,
                quote(folder),
                here
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
        // The player's own game files first, so the games list says which games they add.
        self.load_player_games();
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
        let missing: Vec<(String, String, bool, bool)> = scan
            .games
            .iter()
            .filter(|g| g.store == turbine_engine::Store::Steam && (g.cover.is_none() || g.header.is_none()))
            .map(|g| (g.store_id.clone(), g.name.clone(), g.cover.is_none(), g.header.is_none()))
            .collect();
        this.as_mut().rust_mut().found = scan.games;
        this.as_mut().set_games(QString::from(json.as_str()));
        this.as_mut().fetch_missing_art(missing);
        this.as_mut().resolve_nexus();
    }

    /// Steam games with no cover or wide picture on the device: Steam's store has them. Downloaded once, in the
    /// background, one game after another (at most 30 a run), into Turbine's own cache; a game whose art couldn't be
    /// had is left alone for a week. When something was saved, the games are read again so the art shows.
    fn fetch_missing_art(self: Pin<&mut Self>, missing: Vec<(String, String, bool, bool)>) {
        static RUNNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if missing.is_empty() || crate::online::using_samples() || std::env::args().any(|a| a.starts_with("--screenshot=")) {
            return;
        }
        let Some(cache) = turbine_engine::steam::art_cache() else { return };
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let due: Vec<_> = missing.into_iter().filter(|(id, _, _, _)| turbine_engine::steam::cdn_art_due(&cache, id, now)).take(30).collect();
        if due.is_empty() || RUNNING.swap(true, std::sync::atomic::Ordering::SeqCst) {
            return;
        }
        crate::log::line("art", &format!("{} Steam game(s) have no art on this device: asking Steam's store once", due.len()));
        let qt = self.qt_thread();
        let spawned = std::thread::Builder::new().name("turbine-art".into()).spawn(move || {
            let client = turbine_engine::net::Client::new(crate::VERSION, &crate::online::temp_dir());
            let mut any = false;
            for (id, name, cover, header) in due {
                let (saved, said) = turbine_engine::steam::cdn_art(&client, &cache, &id, cover, header, now);
                crate::log::line("art", &format!("{name} ({id}): {said}"));
                any |= !saved.is_empty();
            }
            RUNNING.store(false, std::sync::atomic::Ordering::SeqCst);
            if any {
                let _ = qt.queue(|t| t.scan());
            }
        });
        if spawned.is_err() {
            RUNNING.store(false, std::sync::atomic::Ordering::SeqCst);
        }
    }

    pub fn wake(mut self: Pin<&mut Self>) {
        crate::log::line("window", "Back in front; controller quiet for 250 ms");
        let ups = match self.as_mut().rust_mut().reader.as_mut() {
            Some(r) => r.wake(Duration::from_millis(250)),
            None => Vec::new(),
        };
        // A stick still pushed from the game presses nothing until it has been back in the middle.
        let stick = self.as_mut().rust_mut().pointer.let_go();
        // Buttons let go while Turbine was in the background are let go here too.
        for (button, _) in ups.into_iter().chain(stick.map(|d| (d, false))) {
            self.as_mut().handle_pad(button, false);
        }
    }

    pub fn wheel_at(self: Pin<&mut Self>, x: f64, y: f64, steps: i32) {
        crate::pad::send(crate::pad::Kind::Wheel, x, y, 0, self.mouse_mask(), steps);
    }

    pub fn sleep(mut self: Pin<&mut Self>) {
        if let Some(d) = self.as_mut().rust_mut().pointer.let_go() {
            crate::log::line("pad", &format!("Left stick {d} let go (Turbine in the background)"));
            self.as_mut().handle_pad(d, false);
        }
    }

    pub fn poll(mut self: Pin<&mut Self>) {
        if self.as_mut().rust_mut().reader.is_none() {
            let r = Pads::open();
            self.as_mut().rust_mut().reader = Some(r);
        }
        let (events, connected, sticks, quiet, lost) = match self.as_mut().rust_mut().reader.as_mut() {
            Some(r) => (r.poll(), r.connected(), r.sticks(), r.quiet(), r.take_dropped_out()),
            None => (Vec::new(), false, crate::pointer::Sticks::default(), false, false),
        };
        if *self.pads() != connected {
            self.as_mut().set_pads(connected);
            let name = self.rust().reader.as_ref().and_then(|r| r.names().into_iter().next()).unwrap_or_default();
            self.as_mut().set_pad_name(QString::from(name.as_str()));
        }
        // The left stick pushed and held one way is a D-pad press, in with the buttons, and quiet with them just
        // after Turbine comes back to the front (releases still go through).
        let mut events = events;
        let dpad = *self.stick_dpad();
        let was = self.rust().pointer.dpad_only;
        self.as_mut().rust_mut().pointer.dpad_only = dpad;
        if was && !dpad {
            // The wheel just closed, maybe with the stick still pushed at a wedge: that push isn't a D-pad press.
            self.as_mut().rust_mut().pointer.hold_off();
        }
        // The quick wheel is open: the left stick points at a wedge by its angle instead of pressing the D-pad.
        let mut sticks = sticks;
        if dpad {
            let a = crate::pointer::wedge_angle(sticks.lx, sticks.ly).unwrap_or(-1);
            if *self.wheel_angle() != a {
                self.as_mut().set_wheel_angle(a);
            }
            sticks.lx = 0.0;
            sticks.ly = 0.0;
        } else if *self.wheel_angle() != -1 {
            self.as_mut().set_wheel_angle(-1);
        }
        events.extend(self.as_mut().pointer_tick(&sticks).into_iter().filter(|(_, down)| !(quiet && *down)));
        for (button, pressed) in events {
            self.as_mut().handle_pad(button, pressed);
        }
        if lost {
            // A controller went away: its buttons were let go above; the interface stops every hold too.
            self.as_mut().pad(QString::from("lost"), false);
        }
    }

    /// One press or release, from a pad or the build's checks (selftestPad).
    fn handle_pad(mut self: Pin<&mut Self>, button: &'static str, pressed: bool) {
        // Turbine's browser: A is a click at the pointer (one press, one click; held, it holds the button down).
        // A press that went to the page lets go there too, and one that went to Turbine's interface lets go there.
        if button == "a" {
            if pressed && *self.browser_input() && !self.rust().a_browser {
                self.as_mut().rust_mut().a_browser = true;
                return self.browser_a(true);
            }
            if !pressed && self.rust().a_browser {
                self.as_mut().rust_mut().a_browser = false;
                return self.browser_a(false);
            }
        }
        if pressed && *self.browser_input() && (button == "rt" || button == "r3") {
            let n = self.rust().browser_presses + 1;
            self.as_mut().rust_mut().browser_presses = n;
            crate::log::line("pad", &format!("Browser press {n} ({})", button.to_uppercase()));
            self.as_mut().browser_press_seen();
        }
        // While the pointer shows, RT and R3 are the left mouse button and LT the right one (held for dragging).
        // Each source is tracked on its own: R3 let go while RT still drags doesn't drop the drag, and a source
        // whose press went to the mouse never sends its release on to the interface (or the other way round).
        let mouse = match button {
            "rt" => Some((0, 1u8)),
            "r3" => Some((0, 2u8)),
            "lt" => Some((1, 1u8)),
            _ => None,
        };
        if let Some((i, bit)) = mouse {
            let src = self.rust().mouse_src[i];
            if pressed {
                if src & bit != 0 {
                    return;
                }
                if src != 0 || (self.rust().pointer.shown && !*self.pointer_paused()) {
                    self.as_mut().rust_mut().mouse_src[i] = src | bit;
                    if src == 0 {
                        self.as_mut().mouse_button(i, true, button);
                    }
                    return;
                }
            } else if src & bit != 0 {
                let left = src & !bit;
                self.as_mut().rust_mut().mouse_src[i] = left;
                if left == 0 {
                    self.as_mut().mouse_button(i, false, button);
                }
                return;
            }
        }
        if pressed {
            crate::log::line("pad", &format!("Press {button}"));
            // Any other press means the player is using the buttons: the pointer goes, so the triggers page
            // again at once. (R3 with the pointer hidden shows it at the focus: Main.qml.)
            if mouse.is_none() && self.rust().pointer.shown && !self.rust().pointer.sticky && self.rust().mouse_down == [false, false] {
                self.as_mut().pointer_hide();
            }
        }
        self.as_mut().pad(QString::from(button), pressed);
    }

    pub fn selftest_pad(mut self: Pin<&mut Self>, button: &QString, pressed: bool) {
        // "lost": what a controller going away sends after letting go of its buttons.
        const ALL: [&str; 21] = ["a", "b", "x", "y", "lb", "rb", "lt", "rt", "start", "view", "r3", "l3", "up", "down", "left", "right", "l4", "r4", "l5", "r5", "lost"];
        let b = button.to_string();
        if let Some(k) = ALL.iter().find(|k| **k == b) {
            self.as_mut().handle_pad(*k, pressed);
        }
    }

    pub fn pointer_show_at(mut self: Pin<&mut Self>, x: f64, y: f64) {
        crate::log::line("pad", &format!("Pointer shown at the focus ({x:.0}, {y:.0})"));
        self.as_mut().rust_mut().pointer.place(x, y);
        self.as_mut().pointer_publish();
        crate::pad::send(crate::pad::Kind::Move, x, y, 0, self.mouse_mask(), 0);
    }

    pub fn pointer_hide(mut self: Pin<&mut Self>) {
        if !self.rust().pointer.shown || self.rust().pointer.sticky || self.rust().mouse_down != [false, false] {
            return;
        }
        self.as_mut().rust_mut().pointer.shown = false;
        self.as_mut().pointer_publish();
        // Off the window, so nothing stays outlined under a hidden pointer.
        crate::pad::send(crate::pad::Kind::Move, -100.0, -100.0, 0, 0, 0);
    }

    /// One look at the sticks for the controller pointer: moves it (right stick), scrolls (left stick), hides it
    /// after 3 seconds without the right stick. Nothing while the on-screen keyboard is up (`pointerPaused`).
    fn pointer_tick(mut self: Pin<&mut Self>, sticks: &crate::pointer::Sticks) -> Vec<(&'static str, bool)> {
        let now = std::time::Instant::now();
        let dt = self.rust().pointer_at.map(|t| now.duration_since(t).as_secs_f64() * 1000.0).unwrap_or(16.0);
        self.as_mut().rust_mut().pointer_at = Some(now);
        if *self.pointer_paused() {
            if self.rust().pointer.shown {
                self.as_mut().rust_mut().pointer.shown = false;
                self.as_mut().pointer_publish();
            }
            // The keyboard is up: a held direction is let go now, so it doesn't repeat until the keyboard closes.
            return self.as_mut().rust_mut().pointer.let_go().map(|d| vec![(d, false)]).unwrap_or_default();
        }
        let Some((w, h)) = crate::pad::window_size() else { return Vec::new() };
        let t = {
            let mut rm = self.as_mut().rust_mut();
            rm.pointer.tick(sticks, dt, w, h)
        };
        let (x, y) = (self.rust().pointer.x, self.rust().pointer.y);
        let again = match self.rust().rehover {
            0 => false,
            n => {
                self.as_mut().rust_mut().rehover = n - 1;
                n == 1 && self.rust().pointer.shown
            }
        };
        let resend = {
            let mut rm = self.as_mut().rust_mut();
            if t.moved || again || !rm.pointer.shown {
                rm.resend_ms = 0.0;
                false
            } else {
                rm.resend_ms += dt;
                rm.resend_ms >= 400.0 && rm.mouse_down == [false, false]
            }
        };
        if resend {
            self.as_mut().rust_mut().resend_ms = 0.0;
        }
        if t.moved || again || resend {
            crate::pad::send(crate::pad::Kind::Move, x, y, 0, self.mouse_mask(), 0);
        }
        if t.hid {
            // Off the window, so nothing stays highlighted under a hidden pointer.
            crate::pad::send(crate::pad::Kind::Move, -100.0, -100.0, 0, 0, 0);
        }
        if t.wheel != 0 {
            if self.rust().pointer.shown {
                crate::pad::send(crate::pad::Kind::Wheel, x, y, 0, self.mouse_mask(), t.wheel);
            } else {
                // No pointer on screen: the wheel goes to whatever has the focus (Main.qml works out where that is
                // and calls wheelAt), not to the hidden pointer's last place.
                for _ in 0..t.wheel.unsigned_abs() {
                    self.as_mut().pad(QString::from("wheel"), t.wheel > 0);
                }
            }
        }
        if t.moved && self.rust().key_nav.on {
            // The right stick moved the pointer: A clicks at it again (not Enter on the page's focus).
            self.as_mut().rust_mut().key_nav.pointer();
            self.as_mut().set_browser_key_nav(false);
        }
        if t.moved || t.hid {
            if t.moved && !*self.pointer_shown() {
                crate::log::line("pad", "Pointer shown (right stick)");
            }
            self.as_mut().pointer_publish();
        }
        if t.wheel != 0 {
            crate::log::line("pad", &format!("Left stick wheel: {} notch(es)", t.wheel));
        }
        let mut out = Vec::new();
        if let Some(d) = t.release {
            out.push((d, false));
        }
        if let Some(d) = t.press {
            crate::log::line("pad", &format!("Left stick: {d}"));
            out.push((d, true));
        }
        out
    }

    fn mouse_mask(&self) -> i32 {
        let d = self.rust().mouse_down;
        i32::from(d[0]) | (i32::from(d[1]) << 1)
    }

    /// The pointer's place and whether it shows, for the pointer drawn in QML.
    pub(crate) fn pointer_publish(mut self: Pin<&mut Self>) {
        let (x, y, shown) = (self.rust().pointer.x, self.rust().pointer.y, self.rust().pointer.shown);
        if *self.pointer_x() != x {
            self.as_mut().set_pointer_x(x);
        }
        if *self.pointer_y() != y {
            self.as_mut().set_pointer_y(y);
        }
        if *self.pointer_shown() != shown {
            self.as_mut().set_pointer_shown(shown);
        }
    }

    /// A trigger as a mouse button: `i` 0 left (RT, R2 on some pads), 1 right (LT, L2).
    pub(crate) fn mouse_button(mut self: Pin<&mut Self>, i: usize, pressed: bool, from: &str) {
        if self.rust().mouse_down[i] == pressed {
            return;
        }
        self.as_mut().rust_mut().mouse_down[i] = pressed;
        let holding = self.rust().mouse_down.iter().any(|d| *d);
        self.as_mut().rust_mut().pointer.holding = holding;
        let (x, y) = (self.rust().pointer.x, self.rust().pointer.y);
        let button = if i == 0 { 1 } else { 2 };
        if pressed {
            if self.rust().key_nav.on {
                self.as_mut().rust_mut().key_nav.pointer();
                self.as_mut().set_browser_key_nav(false);
            }
            let what = if i == 0 { "Left click" } else { "Right click" };
            crate::log::line("pad", &format!("{what} ({}) at {x:.0}, {y:.0}", from.to_uppercase()));
            // In the browser every click is counted against the presses that made it (the build checks they match).
            if i == 0 && self.rust().pointer.sticky {
                let n = self.rust().browser_clicks + 1;
                self.as_mut().rust_mut().browser_clicks = n;
                crate::log::line("pad", &format!("Browser click {n} ({})", from.to_uppercase()));
            }
        }
        let kind = if pressed { crate::pad::Kind::Press } else { crate::pad::Kind::Release };
        crate::pad::send(kind, x, y, button, self.mouse_mask(), 0);
        if !pressed {
            self.as_mut().rust_mut().rehover = 6;
        }
    }

    pub fn selftest_pointer(mut self: Pin<&mut Self>, x: f64, y: f64, click: bool, wheel: i32) {
        crate::log::line("pad", &format!("Pointer check at {x:.0}, {y:.0} (click {click}, wheel {wheel})"));
        self.as_mut().rust_mut().pointer.place(x, y);
        self.as_mut().pointer_publish();
        crate::pad::send(crate::pad::Kind::Move, x, y, 0, 0, 0);
        if click {
            self.as_mut().mouse_button(0, true, "rt");
            self.as_mut().mouse_button(0, false, "rt");
        }
        if wheel != 0 {
            crate::pad::send(crate::pad::Kind::Wheel, x, y, 0, 0, wheel);
        }
    }

    pub(crate) fn find(&self, id: &str) -> Option<Game> {
        self.rust().found.iter().find(|g| g.id() == id).cloned()
    }

    pub(crate) fn current_game(&self) -> Option<Game> {
        self.find(&self.rust().current)
    }

    /// The current game's mod loader by its short name ("SKSE", "BepInEx"), or "the mod loader".
    pub(crate) fn loader_short(&self) -> String {
        self.current_game()
            .and_then(|g| GameRules::for_game(&g).loader.map(|l| l.name.split(" (").next().unwrap_or("").to_string()))
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| "the mod loader".to_string())
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
                    qt: Some(qt),
                    headless: None,
                    pending: std::collections::VecDeque::new(),
                    next_session: 0,
                    fomod_sent: None,
                    witcher_said: std::collections::HashSet::new(),
                    share: None,
                    loot: HashMap::new(),
                    place_pending: std::collections::VecDeque::new(),
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
        self.as_mut().rust_mut().dirty = cur.clone();
        self.as_mut().set_pending_game(QString::from(cur.as_str()));
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
        self.as_mut().hand_added(&path);
        self.as_mut().edited();
        // With a Nexus key the file is looked up by its MD5 first, so it gets its Nexus name, version and updates.
        let domain = turbine_engine::nexus::domain_for(game.store, &game.store_id);
        if let (Some(d), true) = (domain, crate::online::nexus_key().is_some()) {
            return self.import_identified(game, path, d.to_string());
        }
        self.as_mut().say(&format!("Adding {file}..."), "work");
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
            Box::new(move |w: &mut Worker| {
                crate::actions::add_archive(w, &game, &path, source, None, &file);
            }),
        );
    }

    pub fn set_enabled(self: Pin<&mut Self>, id: &QString, on: bool) {
        let id = id.to_string();
        self.change(move |s| s.set_enabled(&id, on).map_err(|e| format!("Couldn't change that mod: {}", crate::log::plain(&e.to_string()))));
    }

    pub fn move_to(self: Pin<&mut Self>, id: &QString, index: i32) {
        let id = id.to_string();
        let index = index.max(0) as usize;
        self.change(move |s| s.move_to(&id, index).map_err(|e| format!("Couldn't move that mod: {}", crate::log::plain(&e.to_string()))));
    }

    pub fn remove_mod(self: Pin<&mut Self>, id: &QString) {
        let id = id.to_string();
        self.change(move |s| s.remove(&id).map_err(|e| format!("Couldn't uninstall that mod: {}", crate::log::plain(&e.to_string()))));
    }

    pub fn mod_folder(&self, id: &QString) -> QString {
        let id = id.to_string();
        if id.is_empty() || id.contains('/') || id.contains('\\') || id.starts_with('.') {
            return QString::default();
        }
        let Some(game) = self.current_game() else { return QString::default() };
        let dir = Store::root_for(&game, &data_home()).join("mods").join(&id);
        if dir.is_dir() {
            QString::from(dir.to_string_lossy().as_ref())
        } else {
            QString::default()
        }
    }

    pub fn mod_files(&self, id: &QString) -> QString {
        let dir = self.mod_folder(id).to_string();
        if dir.is_empty() {
            return QString::from("[]");
        }
        let root = std::path::PathBuf::from(&dir);
        let mut out: Vec<String> = Vec::new();
        let mut todo = vec![root.clone()];
        while let Some(d) = todo.pop() {
            let Ok(rd) = std::fs::read_dir(&d) else { continue };
            for e in rd.flatten() {
                let Ok(ft) = e.file_type() else { continue };
                let p = e.path();
                if ft.is_dir() {
                    todo.push(p);
                } else if ft.is_file() {
                    if let Ok(rel) = p.strip_prefix(&root) {
                        out.push(rel.to_string_lossy().replace('\\', "/"));
                    }
                }
                if out.len() >= 2000 {
                    break;
                }
            }
        }
        out.sort_by_key(|a| a.to_lowercase());
        QString::from(format!("[{}]", out.iter().map(|f| turbine_engine::json::quote(f)).collect::<Vec<_>>().join(",")).as_str())
    }

    pub fn undo_remove(self: Pin<&mut Self>, id: &QString) {
        let id = id.to_string();
        self.change(move |s| s.undo_remove(&id).map_err(|e| format!("Couldn't put that mod back: {}", crate::log::plain(&e.to_string()))));
    }

    pub fn purge_removed(self: Pin<&mut Self>, id: &QString) {
        let id = id.to_string();
        crate::log::line("action", &format!("Delete for good: {id}"));
        self.change(move |s| s.purge_removed(&id).map_err(|e| format!("Couldn't delete that mod: {}", crate::log::plain(&e.to_string()))));
    }

    pub fn purge_all_removed(self: Pin<&mut Self>) {
        crate::log::line("action", "Empty Recently removed");
        self.change(move |s| s.purge_all_removed().map(|_| ()).map_err(|e| format!("Couldn't empty Recently removed: {}", crate::log::plain(&e.to_string()))));
    }

    pub fn open_url(self: Pin<&mut Self>, url: &QString) -> QString {
        let gm = *self.gamemode();
        let b = self.browser_json().to_string();
        QString::from(crate::launch::open(&url.to_string(), gm, &b).as_str())
    }

    pub fn browser_shortcut(&self) -> QString {
        QString::from(crate::launch::browser_shortcut_json().as_str())
    }

    pub fn open_in_steam(self: Pin<&mut Self>, url: &QString) -> QString {
        QString::from(crate::launch::open_in_steam(&url.to_string()).as_str())
    }

    pub fn external_display(&self) -> bool {
        crate::launch::external_display()
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
        // "Saving..." in the status line while it runs (the header shows nothing about changes otherwise).
        self.as_mut().say(&format!("Saving {} to the game...", changes(n)), "work");
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
        self.send(Some(game.id()), "Play original", true, play_original_job(game, n));
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
                    Err(e) => w.say(format!("Couldn't reset the game's files: {}", crate::log::plain(&e.to_string())), "error"),
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
                    Err(e) => w.say(format!("Couldn't put that setup back: {}", crate::log::plain(&e.to_string())), "error"),
                }
                w.show(&game, false);
            }),
        );
    }

    pub fn kcd_restore(self: Pin<&mut Self>) {
        let Some(game) = self.current_game() else { return };
        self.send(
            Some(game.id()), "Put back mod_order.txt",
            false,
            Box::new(move |w: &mut Worker| {
                let Ok(keep) = w.slot(&game).map(|s| s.store.root().to_path_buf()) else { return };
                if crate::system::game_running(&game.install_dir) == Some(true) {
                    w.say(format!("Close {} first, then put mod_order.txt back.", game.name), "warn");
                    return;
                }
                match turbine_engine::kcd::restore_mod_order(&game.install_dir, &keep) {
                    Ok(true) => w.say("Your mod_order.txt is back as it was before Turbine. Turbine re-adds its own mods that are on at your next change or Play, so the file changes again then.", "ok"),
                    Ok(false) => w.say("There is no earlier mod_order.txt to put back.", "warn"),
                    Err(e) => w.say(e, "error"),
                }
                w.show(&game, true);
            }),
        );
    }

    pub fn check(self: Pin<&mut Self>) {
        let Some(game) = self.current_game() else { return };
        self.send(
            Some(game.id()), "Check before Play",
            false,
            Box::new(move |w: &mut Worker| {
                let json = check_json(w, &game);
                w.post(move |mut t| {
                    t.as_mut().set_check_json(QString::from(json.as_str()));
                    t.as_mut().checked(QString::from(json.as_str()));
                });
            }),
        );
    }

    pub fn play(self: Pin<&mut Self>, game_id: &QString) {
        self.play_how(game_id, turbine_engine::anticheat::Coop::Solo, false);
    }

    /// 0.58: Play Seamless Co-op (Elden Ring through me3 with Seamless Co-op's own .co2 saves).
    pub fn play_coop(self: Pin<&mut Self>) {
        let Some(game) = self.current_game() else { return };
        crate::log::line("action", &format!("Play Seamless Co-op: {}", game.name));
        self.play_how(&QString::from(game.id().as_str()), turbine_engine::anticheat::Coop::Seamless, false);
    }

    /// 0.6: Mods off or Mods on from Steam's menu while Turbine is open (waiting changes go in first).
    pub fn quick_mods(mut self: Pin<&mut Self>, game_id: &QString, on: bool) {
        let Some(game) = self.find(&game_id.to_string()) else { return };
        self.as_mut().apply_now();
        self.as_mut().say(if on { "Turning mods back on..." } else { "Turning mods off..." }, "work");
        self.send(Some(game.id()), &format!("Mods {} for {}", if on { "on" } else { "off" }, game.name), true, mods_job(game, on));
    }

    /// 0.58: a me3 game started outside Steam (Desktop Mode only; Game Mode won't show it).
    pub fn play_outside_steam(self: Pin<&mut Self>, coop: bool) {
        let Some(game) = self.current_game() else { return };
        crate::log::line("action", &format!("Play outside Steam (Desktop Mode): {}", game.name));
        let c = if coop { turbine_engine::anticheat::Coop::Seamless } else { turbine_engine::anticheat::Coop::Solo };
        self.play_how(&QString::from(game.id().as_str()), c, true);
    }

    fn play_how(mut self: Pin<&mut Self>, game_id: &QString, coop: turbine_engine::anticheat::Coop, direct: bool) {
        let Some(game) = self.find(&game_id.to_string()) else { return };
        let (_, n) = self.as_mut().take_pending();
        self.as_mut().say("Getting the game ready...", "work");
        self.send(Some(game.id()), &format!("Play {}", game.name), true, play_job(game, coop, direct, n));
    }

    pub fn play_without_loader(mut self: Pin<&mut Self>) {
        let Some(game) = self.current_game() else { return };
        let (_, n) = self.as_mut().take_pending();
        // 0.59 (T8): the game's own loader by name (SKSE, BepInEx, SMAPI), never "the script extender" for all.
        let loader = self.as_ref().loader_short();
        self.as_mut().say(&format!("Getting the game ready without {loader}..."), "work");
        self.send(
            Some(game.id()), &format!("Play {} without the loader", game.name),
            true,
            Box::new(move |w: &mut Worker| {
                let result = w.slot(&game).and_then(|s| {
                    let mut skip = s.store.loader_mods(&s.rules);
                    skip.extend(s.store.mods().iter().filter(|m| m.enabled && m.needs_loader).map(|m| m.id.clone()));
                    let left = skip.len();
                    s.store.apply_except(&skip).map(|r| (r, left)).map_err(|e| e.to_string())
                });
                match result {
                    Ok((r, left)) => {
                        w.note(&game, &r);
                        crate::log::line("launch", &format!("{} without the loader: {left} mods left out this time", game.name));
                        // 0.57: the same anti-cheat checks as Play.
                        let blocked = w.slot(&game).ok().and_then(|s| {
                            crate::anticheat::keep_record(s);
                            crate::anticheat::normal_launch_blocked(s)
                        });
                        match blocked {
                            Some(why) => w.say(why, "error"),
                            None => {
                                let wr: &Worker = w;
                                if wr.slots.get(&game.id()).is_none_or(|s| crate::anticheat::arm_plain(wr, s, true)) {
                                    w.say(format!("Starting without {loader} and the mods that need it. They come back next time."), "ok");
                                    launch(w, &game, "plain");
                                } else {
                                    w.say(format!("Couldn't set {} to start without mods, so it wasn't started.", game.name), "error");
                                }
                            }
                        }
                    }
                    Err(e) => {
                        w.post(move |mut t| {
                            let p = *t.pending() + n;
                            t.as_mut().set_pending(p);
                        });
                        w.say(format!("Couldn't take {loader} out, so the game wasn't started: {}", crate::log::plain(&e.to_string())), "error");
                    }
                }
                w.show(&game, false);
            }),
        );
    }

    pub fn mark_not_working(self: Pin<&mut Self>) {
        let Some(game) = self.current_game() else { return };
        self.send(
            Some(game.id()), "Setup closed early",
            false,
            Box::new(move |w: &mut Worker| {
                match w.slot(&game).and_then(|s| s.store.mark_not_working().map_err(|e| e.to_string())) {
                    Ok(done) => crate::log::line("engine", &format!("{}: this setup closed early{}", game.name, if done { "; no longer the one that worked" } else { "" })),
                    Err(e) => crate::log::line("engine", &format!("Couldn't note the setup as not working: {e}")),
                }
                w.show(&game, false);
            }),
        );
    }

    pub fn expect_update(mut self: Pin<&mut Self>, mod_id: &QString) {
        let id = mod_id.to_string();
        let info = self.info();
        let Some(m) = info.have.iter().find(|h| h.mod_id == id).and_then(|h| turbine_engine::nexus::source_mod(&h.source)).map(|(_, m)| m) else {
            return;
        };
        crate::log::line("catch", &format!("Waiting for the update of {id} (Nexus mod {m}): its download replaces it"));
        let mut r = self.as_mut().rust_mut();
        r.update_waits.retain(|(x, _)| *x != m);
        r.update_waits.push((m, id));
    }

    pub fn list_downloads(self: Pin<&mut Self>) {
        let home = turbine_engine::home_dir();
        let here = self.current_game().map(|g| turbine_engine::watch::game_folder_name(&g.name)).unwrap_or_default();
        self.send(
            None, "List Downloads",
            false,
            Box::new(move |w: &mut Worker| {
                let json = downloads_json(&home, &here);
                w.post(move |t| t.set_downloads_json(QString::from(json.as_str())));
            }),
        );
    }
}

impl qobject::Turbine {
    pub fn log(&self, kind: &QString, text: &QString) {
        crate::log::line(&kind.to_string(), &text.to_string());
    }

    pub fn clipboard_text(&self) -> QString {
        // SAFETY: qtglue.cpp returns a malloc'd, zero-ended copy (or null) made on this (the GUI) thread; it is read
        // once here and handed back to turbine_free_text.
        let text = unsafe {
            let p = turbine_clipboard_text();
            if p.is_null() {
                String::new()
            } else {
                let t = std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned();
                turbine_free_text(p);
                t
            }
        };
        crate::log::line("action", &format!("Read the clipboard ({} characters)", text.chars().count()));
        QString::from(text.as_str())
    }

    pub fn activity_log(&self, lines: i32) -> QString {
        QString::from(crate::log::last_lines(lines.clamp(1, 2000) as usize).join("\n").as_str())
    }

    pub fn save_problem_report(&self, screen: &QString, settings: &QString) -> QString {
        use turbine_engine::anticheat as ac;
        use turbine_engine::report;
        let games: Vec<String> = self
            .rust()
            .found
            .iter()
            .map(|g| {
                let rules = rules::GameRules::for_game(g);
                let how = match rules.anti_cheat.as_ref().map(ac::mode) {
                    Some(ac::Mode::Me3) => "me3 with mods (offline), through Steam's Launch Options",
                    Some(ac::Mode::Official) => "the game's own anti-cheat switch",
                    Some(ac::Mode::Offline) => "offline with mods",
                    Some(ac::Mode::Refuse) => "no mods (online-only anti-cheat)",
                    _ => "mods placed in the game folder",
                };
                format!("{} ({}): {}, {}, {}", g.name, g.id(), rules.support, how, if g.build.is_some() { format!("build {}", g.build.as_deref().unwrap_or("")) } else { "build unknown".into() })
            })
            .collect();
        let r = report::ProblemReport {
            version: format!("{} (build {})", crate::VERSION, crate::labs::COMMIT),
            when: crate::log::file_stamp(),
            device: vec![
                format!("Device: {}", crate::labs::device()),
                format!("System: {}", crate::labs::system()),
                format!("Mode: {}", if *self.gamemode() { "Game Mode" } else { "Desktop Mode" }),
            ],
            screen: screen.to_string(),
            settings: settings.to_string().lines().map(str::to_string).collect(),
            games,
            log: crate::log::last_lines(300),
            launch: ac::launch_report_lines(&data_home()),
        };
        let name = report::file_name(&crate::log::file_stamp());
        let dir = crate::labs::downloads();
        let res = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(dir.join(&name), report::build(&r)));
        let (file, error) = match res {
            Ok(()) => {
                crate::log::line("action", &format!("Report a problem: saved {} in {}", name, dir.display()));
                (name, String::new())
            }
            Err(e) => {
                crate::log::line("action", &format!("Report a problem: couldn't save it in {}: {e}", dir.display()));
                (String::new(), format!("Couldn't save it in Downloads ({e})."))
            }
        };
        QString::from(format!("{{\"file\":{},\"folder\":{},\"error\":{}}}", quote(&file), quote(&dir.display().to_string()), quote(&error)).as_str())
    }

    pub fn save_activity_log(&self) -> QString {
        let text = crate::log::last_lines(500).join("\n") + "\n";
        let name = format!("Turbine-activity-{}.log", crate::log::file_stamp());
        let dir = crate::labs::downloads();
        let r = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(dir.join(&name), text));
        let (file, error) = match r {
            Ok(()) => {
                crate::log::line("labs", &format!("Saved the activity log to Downloads as {name}"));
                (name, String::new())
            }
            Err(e) => {
                crate::log::line("labs", &format!("Couldn't save the activity log to Downloads: {e}"));
                (String::new(), format!("Couldn't save it in Downloads ({e})."))
            }
        };
        QString::from(format!("{{\"file\":{},\"error\":{}}}", turbine_engine::json::quote(&file), turbine_engine::json::quote(&error)).as_str())
    }

    pub fn decky_json(&self) -> QString {
        QString::from(crate::system::decky_json().as_str())
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
    fn turbine_clipboard_text() -> *mut std::os::raw::c_char;
    fn turbine_free_text(text: *mut std::os::raw::c_char);
}

/// Applies a game's waiting changes; `n` of them go back on the count if it fails.
/// Play: the check is done; applies, then starts the game (the window's Play, and Steam's menu through quick.rs).
pub(crate) fn play_job(game: Game, coop: turbine_engine::anticheat::Coop, direct: bool, n: i32) -> Job {
    Box::new(move |w: &mut Worker| {
        let put_back = move |w: &Worker| {
            w.post(move |mut t| {
                let p = *t.pending() + n;
                t.as_mut().set_pending(p);
            })
        };
        // A running game would write its Proton settings over Turbine's change when it closes.
        let running = w.slot(&game).ok().and_then(|s| wanted_overrides(s)).is_some()
            && crate::system::game_running(&game.install_dir) == Some(true);
        if running {
            put_back(w);
            w.say(format!("{} is still running. Close it, then press Play again.", game.name), "error");
            return;
        }
        // 0.57: a game me3 starts with mods: the player's answer first, and me3 itself (fetched, then Play
        // again).
        let me3_play = w.slot(&game).ok().and_then(|s| {
            let on = s.store.mods().iter().any(|m| m.enabled);
            let a = crate::anticheat::special(s)?;
            (on && s.store.me3_game().is_some()).then_some(turbine_engine::anticheat::needs_ask(a) && !crate::anticheat::turned_off(s))
        });
        if me3_play == Some(true) {
            put_back(w);
            w.say(format!("Answer the anti-cheat question for {} first: press Play again.", game.name), "warn");
            return;
        }
        if me3_play.is_some() && crate::anticheat::me3_state(&w.data_home).is_err() {
            put_back(w);
            crate::log::line("anticheat", &format!("{}: me3 isn't installed (or changed): getting it, then Play", game.name));
            w.post(move |t| t.install_me3(true));
            return;
        }
        // 0.56: a copy of the saves first, so a mod that breaks them can't take them with it. (0.57: for a game
        // me3 starts this is also the copy before its first modded launch.)
        crate::health::backup_before_play(w, &game);
        let result = w.slot(&game).and_then(|s| s.store.apply().map_err(|e| e.to_string()));
        crate::actions::sync_order(w, &game);
        if result.is_ok() {
            witcher_sync(w, &game, true);
        }
        // The loader may have just been put in place by apply, so this comes after it.
        let what = w.slot(&game).map(|s| overrides_for(s)).unwrap_or("mods");
        let note = match w.slot(&game).ok().and_then(|s| wanted_overrides(s)) {
            None => None,
            Some((None, _)) => Some(proton::NOT_STARTED.to_string()),
            Some((Some(p), wanted)) => match proton::ensure_overrides(&p, &wanted) {
                Ok(changed) => {
                    if !changed.is_empty() {
                        crate::log::line("engine", &format!("Proton settings for {}: set {} to native,builtin", game.name, changed.join(", ")));
                    }
                    None
                }
                Err(e) => Some(e),
            },
        };
        match result {
            Ok(r) => {
                w.note(&game, &r);
                if let Some(s) = w.slots.get(&game.id()) {
                    crate::anticheat::set_doubt(s, false);
                }
                // Play went through with the mods as they are: that setup is the one Restore goes back to,
                // and this game build no longer counts as "updated since".
                if let Ok(s) = w.slot(&game) {
                    let marked = s.store.mark_played().and_then(|_| s.store.acknowledge_build());
                    if s.store.mods().iter().any(|m| m.enabled) {
                        if let Some(b) = &s.game.build {
                            let _ = std::fs::write(s.store.root().join("played-build"), b);
                        }
                    }
                    if let Err(e) = marked {
                        w.say(format!("Couldn't note this setup as played: {e}"), "warn");
                    }
                }
                match note {
                    Some(e) => w.say(format!("Started without {what} this time. {e}"), "warn"),
                    None => w.say(applied_words(&r, &game), "ok"),
                }
                // 0.57: anti-cheat games. With mods on, me3 starts it offline from its profile; with none on,
                // Steam starts it normally, only when the game folder is original.
                let blocked = w.slot(&game).ok().and_then(|s| {
                    crate::anticheat::keep_record(s);
                    if me3_play.is_some() {
                        None
                    } else {
                        crate::anticheat::normal_launch_blocked(s)
                    }
                });
                if let Some(why) = blocked {
                    w.say(why, "error");
                } else if me3_play.is_some() {
                    // 0.58: through Steam (Game Mode shows a game only when Steam started it): Turbine's
                    // launch helper runs me3 from the game's Launch Options; outside Steam only when asked.
                    let wr: &Worker = w;
                    let Some(s) = wr.slots.get(&game.id()) else { return };
                    if direct {
                        if crate::anticheat::launch_me3(wr, s, coop) {
                            watch_game(w, &game);
                        }
                    } else {
                        match crate::anticheat::arm_me3(wr, s, coop) {
                            Ok(true) => {
                                launch(w, &game, if coop == turbine_engine::anticheat::Coop::Seamless { "coop" } else { "mods" });
                                watch_game(w, &game);
                            }
                            Ok(false) => {
                                let (set, need, _) = crate::anticheat::launch_option_state(s, &wr.data_home);
                                crate::log::line("launch", &format!("{}: not started; the one-time Launch Options step comes first", game.name));
                                let json = format!(
                                    "{{\"game\":{},\"name\":{},\"option\":{},\"set\":{},\"listed\":{},\"coop\":{},\"undone\":{},\"outdated\":{}}}",
                                    quote(&game.id()),
                                    quote(&crate::anticheat::title(s)),
                                    quote(&need),
                                    quote(set.as_deref().unwrap_or("")),
                                    set.is_some(),
                                    coop == turbine_engine::anticheat::Coop::Seamless,
                                    turbine_engine::anticheat::undone_file(&wr.data_home, &s.game.store_id).exists(),
                                    set.as_deref().is_some_and(|x| turbine_engine::anticheat::launch_option_uses_helper(x, &wr.data_home))
                                );
                                let id = game.id();
                                w.post(move |t| t.launch_setup(QString::from(id.as_str()), QString::from(json.as_str())));
                            }
                            Err(e) => w.say(format!("Couldn't start {} with mods: {e}", game.name), "error"),
                        }
                    }
                } else {
                    // A me3 game with no mods on: Steam's own command through the helper.
                    let wr: &Worker = w;
                    if wr.slots.get(&game.id()).is_none_or(|s| crate::anticheat::arm_plain(wr, s, false)) {
                        // 0.6: a Heroic game with a script loader on starts the loader through Heroic's link.
                        launch_with_loader(w, &game, if wr.slots.get(&game.id()).is_some_and(|s| s.store.mods().iter().any(|m| m.enabled)) { "mods" } else { "plain" });
                        watch_game(w, &game);
                    } else {
                        w.say(format!("Couldn't set {} to start normally, so it wasn't started.", game.name), "error");
                    }
                }
                w.show(&game, false);
            }
            Err(e) => {
                w.post(move |mut t| {
                    let p = *t.pending() + n;
                    t.as_mut().set_pending(p);
                });
                if let Some(s) = w.slots.get(&game.id()) {
                    crate::anticheat::set_doubt(s, true);
                    crate::anticheat::sync_helper(w, s);
                }
                w.say(format!("Couldn't get the mods in place, so the game wasn't started: {}", crate::log::plain(&e.to_string())), "error");
            }
        }
    })
}

/// Play without mods: every mod file out of the game, then it starts (the mods stay on for next time).
pub(crate) fn play_original_job(game: Game, n: i32) -> Job {
    Box::new(move |w: &mut Worker| {
        let result = w.slot(&game).and_then(|s| s.store.play_original().map_err(|e| e.to_string()));
        match result {
            Ok(r) => {
                w.note(&game, &r);
                // 0.57: an anti-cheat game starts normally only when its folder is original.
                let blocked = w.slot(&game).ok().and_then(|s| {
                    crate::anticheat::keep_record(s);
                    crate::anticheat::normal_launch_blocked(s)
                });
                match blocked {
                    Some(why) => w.say(why, "error"),
                    None => {
                        let wr: &Worker = w;
                        let ready = wr.slots.get(&game.id()).is_none_or(|s| crate::anticheat::arm_plain(wr, s, true));
                        if ready {
                            w.say("Starting the game with no mods. Your mods are kept.", "ok");
                            launch(w, &game, "plain");
                            watch_game_as(w, &game, false);
                        } else {
                            w.say(format!("Couldn't set {} to start without mods, so it wasn't started. Try again, or turn every mod off.", game.name), "error");
                        }
                    }
                }
            }
            Err(e) => {
                w.post(move |mut t| {
                    let p = *t.pending() + n;
                    t.as_mut().set_pending(p);
                });
                w.say(format!("Couldn't take the mods out: {}", crate::log::plain(&e.to_string())), "error");
            }
        }
    })
}

/// The check before Play for a game, as the JSON list `checked` gets (also asked by Steam's menu, quick.rs).
pub(crate) fn check_json(w: &mut Worker, game: &Game) -> String {
    let data = w.data_home.clone();
    match w.slot(game) {
        Ok(s) => {
            let needs = s.store.mods().iter().any(|m| m.enabled && m.needs_loader);
            let free = crate::system::free_bytes(&s.game.install_dir);
            let last = s.store.last_build();
            let mut list = rules::pre_play(&s.game, &s.rules, last.as_deref(), needs, free);
            rules::add_checks(&mut list, rules::mod_checks(&s.game, &s.rules, &s.store.mod_files()));
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
            // Kingdom Come: the first game's one-time note on developer mode (a few mods need it), and
            // mods that came from the other game's Nexus pages.
            if matches!(s.rules.family, rules::Family::Kcd | rules::Family::Kcd2) {
                let second = s.rules.family == rules::Family::Kcd2;
                let any_on = s.store.mods().iter().any(|m| m.enabled);
                if !second && any_on {
                    extra.push(turbine_engine::kcd::devmode_check(s.game.store == turbine_engine::games::Store::Steam));
                }
                let other_domain = if second { "kingdomcomedeliverance" } else { "kingdomcomedeliverance2" };
                let named: Vec<String> = s
                    .store
                    .mods()
                    .iter()
                    .filter(|m| m.enabled && !turbine_engine::kcd::name_says_other(&m.name, second))
                    .filter(|m| turbine_engine::nexus::source_mod(&s.store.source_of(&m.id)).is_some_and(|(d, _)| d == other_domain))
                    .map(|m| m.name.clone())
                    .collect();
                extra.extend(turbine_engine::kcd::checks(&[], second, &named));
            }
            // 0.56: anti-cheat while any mod is on.
            if let Some(c) = crate::anygame::anti_cheat_check(s) {
                extra.push(c);
            }
            // 0.57: games whose data file says how to play them with mods (me3, Halo's switch, offline,
            // refused), and whether the game folder is original for the normal launch.
            extra.extend(crate::anticheat::checks(s, &data));
            if !extra.is_empty() && list.len() == 1 && list[0].level == "ok" {
                list.clear(); // "Everything checks out" no longer says it all
            }
            list.extend(extra);
            checks_json_with(&list, &witcher_extras(s, &list))
        }
        Err(e) => checks_json(&[rules::Check { level: "stop".into(), title: "Mods couldn't be read".into(), detail: e }]),
    }
}

pub(crate) fn apply_job(game: Game, n: i32) -> Job {
    apply_job_saying(game, n, None)
}

/// `apply_job`, with words of its own in front of "Ready to play" when it worked (Mods off and on).
fn apply_job_saying(game: Game, n: i32, words: Option<String>) -> Job {
    Box::new(move |w: &mut Worker| {
        let result = w.slot(&game).and_then(|s| s.store.apply().map_err(|e| e.to_string()));
        crate::actions::sync_order(w, &game);
        if result.is_ok() {
            witcher_sync(w, &game, false);
        }
        match result {
            Ok(r) => {
                w.note(&game, &r);
                // 0.58: the last apply worked (show() below keeps the launch helper to the state).
                if let Some(s) = w.slots.get(&game.id()) {
                    crate::anticheat::set_doubt(s, false);
                }
                match &words {
                    Some(x) => w.say(format!("{x} {}", applied_words(&r, &game)), "ok"),
                    None => w.say(applied_words(&r, &game), "ok"),
                }
            }
            Err(e) => {
                w.post(move |mut t| {
                    let p = *t.pending() + n;
                    t.as_mut().set_pending(p);
                });
                // 0.58: on any doubt, Steam's Play never starts me3 until an apply works again.
                if let Some(s) = w.slots.get(&game.id()) {
                    crate::anticheat::set_doubt(s, true);
                    crate::anticheat::sync_helper(w, s);
                }
                w.say(format!("Couldn't apply the changes: {}. Nothing was lost; Turbine tries again after the next change.", crate::log::plain(&e.to_string())), "error");
            }
        }
        w.show(&game, false);
    })
}

/// 0.6, Mods off / Mods on (Steam's menu): every mod that is on goes off and the list is kept, or exactly those come
/// back on; then the same apply as any change. Not while the game runs (its files are in use).
pub(crate) fn mods_job(game: Game, on: bool) -> Job {
    Box::new(move |w: &mut Worker| {
        let word = if on { "on" } else { "off" };
        if crate::system::game_running(&game.install_dir) == Some(true) {
            w.say(format!("Close {} first, then turn its mods {word}.", game.name), "warn");
            return;
        }
        let changed = w.slot(&game).and_then(|s| (if on { s.store.release_held() } else { s.store.hold_all_off() }).map_err(|e| e.to_string()));
        match changed {
            Ok(0) if on => w.say(format!("Nothing to turn back on for {}. Choose its mods in Turbine.", game.name), "warn"),
            Ok(0) => w.say(format!("No mods were on for {}.", game.name), "ok"),
            Ok(n) => {
                let mods = count(n, "mod", "mods");
                crate::log::line("action", &format!("{}: Mods {word} from Steam's menu ({mods})", game.name));
                let words = if on {
                    format!("Mods on: {mods} back in {}.", game.name)
                } else {
                    format!("Mods off: {mods} out of {}. Mods on brings back exactly these.", game.name)
                };
                apply_job_saying(game.clone(), 0, Some(words))(w);
                return;
            }
            Err(e) => w.say(format!("Couldn't turn the mods {word}: {}", crate::log::plain(&e)), "error"),
        }
        w.show(&game, false);
    })
}

/// After a modded Play: watches the game's process on its own thread (asked on the host every 2 seconds) and says
/// how long it ran. A game that closes within a minute is likely broken by a mod; the interface offers a way back.
/// When the process can't be seen at all, `known` is false and the interface measures the time Turbine was away.
fn watch_game(w: &Worker, game: &Game) {
    watch_game_as(w, game, true)
}

/// 0.58: how long Play waits for the game to show before the "didn't start" sheet.
const START_WAIT_SECS: u64 = 20;

/// `modded`: a modded Play (a quick close offers a way back). Every Play also gets the launch check: when the game
/// isn't seen running about 20 seconds after Play, the interface says so with the likely fixes (`not_started`). The
/// Steam launch helper's own lines for this launch go into the activity log then too.
fn watch_game_as(w: &Worker, game: &Game, modded: bool) {
    if crate::online::using_samples() {
        return;
    }
    let Some(qt) = w.qt.clone() else { return };
    let (id, name, dir) = (game.id(), game.name.clone(), game.install_dir.clone());
    // 0.6: how it went, for Steam's menu (the last launch in the game's store folder).
    let root = w.slots.get(&game.id()).map(|s| s.store.root().to_path_buf());
    let record = move |result: &str, secs: Option<u64>| {
        if let Some(r) = &root {
            let _ = turbine_engine::quick::update_launch(r, result, secs);
        }
    };
    let data = w.data_home.clone();
    let helper_from = crate::anticheat::helper_log_len(&data);
    // Steam that has to start (or update) first takes longer: the check waits a minute then.
    let wait = if crate::system::steam_running() == Some(false) { 60 } else { START_WAIT_SECS };
    let _ = std::thread::Builder::new().name("turbine-game-watch".into()).spawn(move || {
        let tick = Duration::from_secs(2);
        let started = std::time::Instant::now();
        let mut seen: Option<std::time::Instant> = None;
        let mut unknown = 0;
        let mut told = false;
        // Up to 3 minutes for the game to start, then until it closes (at most 10 minutes of watching).
        for i in 0..300 {
            std::thread::sleep(tick);
            if !told && (seen.is_some() || started.elapsed() >= Duration::from_secs(wait)) {
                told = true;
                crate::anticheat::helper_log_to_activity(&data, helper_from);
                if seen.is_none() {
                    crate::log::line("launch", &format!("{name} wasn't seen running {wait} seconds after Play: the launch check opens"));
                    record("not-started", None);
                    let gid = id.clone();
                    let _ = qt.queue(move |t| t.not_started(QString::from(gid.as_str())));
                }
            }
            match crate::system::game_running(&dir) {
                Some(true) => {
                    if seen.is_none() {
                        seen = Some(std::time::Instant::now());
                        crate::log::line("launch", &format!("{name} is running ({} seconds after Play)", started.elapsed().as_secs()));
                        record("running", None);
                    }
                    if seen.is_some_and(|t| t.elapsed() > Duration::from_secs(90)) {
                        crate::log::line("launch", &format!("{name} is still running after 90 seconds"));
                        return;
                    }
                }
                Some(false) => {
                    if let Some(t) = seen {
                        let secs = t.elapsed().as_secs() as i32;
                        crate::log::line("launch", &format!("{name} closed after {secs} seconds"));
                        record("closed", Some(secs.max(0) as u64));
                        if modded {
                            let _ = qt.queue(move |t| t.game_ended(QString::from(id.as_str()), secs, true));
                        }
                        return;
                    }
                    if i >= 90 {
                        crate::log::line("launch", &format!("{name} wasn't seen running"));
                        return;
                    }
                }
                None => {
                    unknown += 1;
                    if unknown >= 3 {
                        crate::log::line("launch", "Can't see the game's process; the interface measures the time away instead");
                        if modded {
                            let _ = qt.queue(move |t| t.game_ended(QString::from(id.as_str()), -1, false));
                        }
                        return;
                    }
                }
            }
        }
    });
}

/// Play with mods: as `launch`, but a Heroic game whose script loader is on (SMAPI, SKSE) starts the loader through
/// Heroic's link (altExe), so nothing needs setting up in Heroic. Play original and Play without the loader keep the
/// plain link.
fn launch_with_loader(w: &Worker, game: &Game, how: &str) {
    let alt = w.slots.get(&game.id()).and_then(|s| rules::heroic_loader_exe(game, &s.rules));
    match alt.and_then(|exe| rules::launch_url_with(game, Some(&exe)).map(|u| (exe, u))) {
        Some((exe, url)) => {
            crate::log::line("launch", &format!("{} through {url} (Heroic starts {})", game.name, exe.display()));
            launch_at(w, game, how, url)
        }
        None => launch(w, game, how),
    }
}

/// `how`: "mods", "coop" or "plain", for the last launch Steam's menu shows.
fn launch(w: &Worker, game: &Game, how: &str) {
    match rules::launch_url(game) {
        Some(url) => {
            crate::log::line("launch", &format!("{} through {url}", game.name));
            launch_at(w, game, how, url)
        }
        None => w.say(format!("Turbine doesn't know how to start {} yet. Start it from its launcher.", game.name), "warn"),
    }
}

fn launch_at(w: &Worker, game: &Game, how: &str, url: String) {
    // 0.6: the last launch, for Steam's menu (the watch after Play adds how it went).
    if let Some(s) = w.slots.get(&game.id()) {
        let at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let l = turbine_engine::quick::Launch { at, how: how.to_string(), result: "started".into(), seconds: None, words: String::new() };
        if let Err(e) = turbine_engine::quick::write_launch(s.store.root(), &l) {
            crate::log::line("launch", &format!("Couldn't keep the last launch for Steam's menu: {e}"));
        }
    }
    // Without a window (Steam's menu), the plugin opens the link itself.
    if let Some(h) = &w.headless {
        *h.launch.lock().unwrap_or_else(|e| e.into_inner()) = Some(url);
        return;
    }
    w.post(move |t| t.launch(QString::from(url.as_str())))
}
