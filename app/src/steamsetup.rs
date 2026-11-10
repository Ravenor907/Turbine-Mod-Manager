//! 0.6: changes to Steam's own settings with one Steam restart the player agreed to (I1): a me3 game's Launch
//! Options (and Undo putting the old ones back), and the optional co-op tile in Steam's library. The careful writers,
//! the host script and the busy check are the engine's (turbine_engine::steam_restart, steam_setup, shortcuts).
//! Also Turbine's own Steam library art and Labs' Proton log switch.

use crate::backend::qobject;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use turbine_engine::anticheat as ac;
use turbine_engine::json::quote;
use turbine_engine::steam_restart::{self as sr, Change, Job};
use turbine_engine::{shortcuts, steam, steam_art, steam_setup, Game};

/// Turbine's Flatpak id (the host script runs Turbine's --apply-steam-setup through it).
const FLATPAK_ID: &str = "io.github.ravenor907.TurbineModManager";

fn in_flatpak() -> bool {
    Path::new("/.flatpak-info").exists()
}

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// The localconfig.vdf that lists the game (the account signed in last first), or the newest one.
fn localconfig_for(home: &Path, app_id: &str) -> Option<PathBuf> {
    let files = steam_setup::localconfig_files(home);
    // 0.61: the signed-in account's file whenever it has one, even when another account lists the game.
    if let Some(f) = steam::signed_in_file(home, &files) {
        return Some(f);
    }
    files
        .iter()
        .find(|f| std::fs::read_to_string(f).ok().and_then(|t| steam::launch_options_in(&t, app_id)).is_some())
        .or(files.first())
        .cloned()
}

/// Turbine's own record of the art it put in Steam's grid folders.
fn art_record(data: &Path) -> PathBuf {
    data.join("steam-art.json")
}

/// The co-op tile's art set (installed with the Flatpak; packaging/steam-art next to a build otherwise).
fn art_set(set: &str) -> PathBuf {
    let installed = steam_art::installed_set(set);
    if installed.is_dir() {
        return installed;
    }
    std::env::current_exe()
        .ok()
        .and_then(|e| e.ancestors().map(|a| a.join("packaging/steam-art").join(set)).find(|p| p.is_dir()))
        .unwrap_or(installed)
}

/// Starts the restart for a job: checks first (no game running, no download under way), then the host script.
/// When Steam is closed already, the change is made here and now. Answers the words for the player on failure.
pub(crate) fn start(found: &[Game], game_mode: bool, job: Job) -> Result<&'static str, String> {
    let home = turbine_engine::home_dir();
    let data = crate::backend::data_home();
    let own = shortcuts::find_turbine(&home).map(|(id, _)| id);
    if let Some(why) = sr::busy(&home, found, own) {
        crate::log::line("steam-setup", &format!("Not restarting Steam for {}: {why}", job.what));
        return Err(why);
    }
    // 0.62: never in Game Mode while Steam runs. The session starts Steam again at once, so the file was written
    // after Steam had read it, and Steam wrote its own copy back (the owner's device, 0.6 and 0.61).
    if let Err(why) = turbine_engine::compat_tool::restart_and_write_allowed(game_mode, crate::system::steam_running()) {
        crate::log::line("steam-setup", &format!("No Steam restart for {} in Game Mode: Steam's files can't be written safely there", job.what));
        return Err(why.to_string());
    }
    if crate::system::steam_running() == Some(false) {
        crate::log::line("steam-setup", &format!("Steam is closed: making the change for {} now", job.what));
        let lines = sr::apply_job(&data, &job)?;
        for l in &lines {
            crate::log::line("steam-setup", l);
        }
        job.save(&data).map_err(|e| e.to_string())?;
        let _ = std::fs::write(sr::result_file(&data), "done \n");
        return Ok("done");
    }
    job.save(&data).map_err(|e| format!("Couldn't get the restart ready: {e}"))?;
    let _ = std::fs::remove_file(sr::result_file(&data));
    let script = sr::write_script(&data).map_err(|e| format!("Couldn't get the restart ready: {e}"))?;
    let exe = std::env::current_exe().unwrap_or_default();
    let apply = sr::apply_command(in_flatpak().then_some(FLATPAK_ID), &exe);
    // In Game Mode Turbine closes with Steam; its Steam entry brings it back. On the desktop it stays open.
    let back = if game_mode { own.map(shortcuts::rungameid_url).unwrap_or_default() } else { String::new() };
    let args = sr::script_args(&data, game_mode, &back, &apply);
    let mut cmd = if in_flatpak() {
        let mut c = Command::new("flatpak-spawn");
        c.arg("--host").arg("bash").arg(&script);
        c
    } else {
        let mut c = Command::new("bash");
        c.arg(&script);
        c
    };
    crate::log::line("steam-setup", &format!("Restarting Steam once for {} ({}): {} {}", job.what, if game_mode { "Game Mode" } else { "Desktop Mode" }, script.display(), args.join(" ")));
    let st = cmd.args(&args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status().map_err(|e| format!("Couldn't start the restart: {e}"))?;
    if !st.success() {
        return Err("Couldn't start the restart on this device.".into());
    }
    Ok("running")
}

/// `turbine-mod-manager --apply-steam-setup`: run by the host script once Steam has closed. Exit code 0 when the
/// job's changes are made.
pub fn apply_from_args() -> Option<i32> {
    if !std::env::args().any(|a| a == "--apply-steam-setup") {
        return None;
    }
    let data = crate::backend::data_home();
    crate::log::start_quiet(crate::log::folder(&data));
    let Some(job) = Job::load(&data) else {
        eprintln!("No Steam setup is waiting.");
        return Some(1);
    };
    match sr::apply_job(&data, &job) {
        Ok(lines) => {
            for l in &lines {
                crate::log::line("steam-setup", l);
                println!("{l}");
            }
            Some(0)
        }
        Err(e) => {
            crate::log::line("steam-setup", &format!("The change for {} failed: {e}", job.what));
            eprintln!("{e}");
            Some(1)
        }
    }
}

/// The job for a kind of change: "options" (Turbine's line for a me3 game), "undo" (the old Launch Options back),
/// "tile" or "untile" (the co-op tile). `resume`: what Turbine does once it's back.
fn job_for(g: &Game, kind: &str, resume: &str) -> Result<Job, String> {
    let home = turbine_engine::home_dir();
    let data = crate::backend::data_home();
    let name = turbine_engine::rules::GameRules::for_game(g).short_name.unwrap_or_else(|| g.name.clone());
    let change = match kind {
        "options" | "undo" => {
            let file = localconfig_for(&home, &g.store_id).ok_or("Steam's settings for this account weren't found. Start Steam once, then try again.")?;
            let set = steam::launch_options(&home, &g.store_id).unwrap_or_default();
            let value = if kind == "options" {
                let _ = ac::remember_original(&data, &g.store_id, &set);
                ac::launch_option(&data)
            } else {
                ac::original_options(&data, &g.store_id)
            };
            Change::Options { file, app_id: g.store_id.clone(), value }
        }
        "tile" => {
            let file = shortcuts::tile_file(&home).ok_or("Steam's list of non-Steam games for this account wasn't found. Start Steam once, then try again.")?;
            let tile = shortcuts::coop_tile(&name.to_uppercase(), &g.id(), FLATPAK_ID);
            Change::Tile { file, marker: shortcuts::coop_marker(&g.id()), tile, art: art_set("coop"), record: art_record(&data) }
        }
        "untile" => {
            let file = shortcuts::tile_file(&home).ok_or("Steam's list of non-Steam games wasn't found.")?;
            Change::NoTile { file, marker: shortcuts::coop_marker(&g.id()), record: art_record(&data) }
        }
        _ => return Err("not a Steam setup".into()),
    };
    let what = match kind {
        "options" => format!("{name}'s launch setting"),
        "undo" => format!("{name}'s old launch setting"),
        "tile" => format!("the {name} Co-op tile"),
        _ => format!("removing the {name} Co-op tile"),
    };
    Ok(Job { changes: vec![change], resume: resume.to_string(), what, at: now_secs() })
}

/// The co-op tile's state for a game: is it in Steam's library (or waiting for the restart)?
fn tile_there(g: &Game) -> bool {
    let home = turbine_engine::home_dir();
    let marker = shortcuts::coop_marker(&g.id());
    shortcuts::tile_file(&home)
        .and_then(|f| std::fs::read(f).ok())
        .and_then(|b| shortcuts::parse(&b).ok())
        .is_some_and(|l| l.iter().any(|s| shortcuts::is_turbine(s) && s.launch_options.contains(&marker)))
}

/// Turbine's own Steam library art: on unless the player removed it.
fn art_off_file(data: &Path) -> PathBuf {
    data.join("steam-art.off")
}

/// Puts Turbine's art in its Steam entry's grid folder (never over the player's own). Answers the words.
fn add_own_art(data: &Path, again: bool) -> Result<String, String> {
    let home = turbine_engine::home_dir();
    let (file, appid) = shortcuts::find_turbine_file(&home).ok_or("Turbine isn't in your Steam library yet. Add it to Steam first (Add a Non-Steam Game in Desktop Mode).")?;
    let done = steam_art::add_art_opts(&art_set("turbine"), &steam_art::grid_dir(&file), appid, steam_art::TURBINE_PARTS, &art_record(data), again).map_err(|e| e.to_string())?;
    let theirs = done.iter().filter(|(_, o)| *o == steam_art::ArtOutcome::PlayersOwn).count();
    crate::log::line("steam-setup", &format!("Steam library art for Turbine ({appid}) in {}: {}", steam_art::grid_dir(&file).display(), steam_art::summary(&done)));
    if steam_art::nothing_shown(&done) {
        return Err(format!("Turbine's art isn't in Steam: {}.", steam_art::summary(&done)));
    }
    Ok(if theirs > 0 { format!("Turbine's art is in Steam ({theirs} of your own pictures kept). Steam shows it after it starts again.") } else { "Turbine's art is in Steam. Steam shows it after it starts again.".to_string() })
}

/// At start (and so after every update): Turbine's art, unless the player removed it. Quiet when nothing changes.
pub(crate) fn art_at_start() {
    let data = crate::backend::data_home();
    if art_off_file(&data).exists() || shortcuts::find_turbine_file(&turbine_engine::home_dir()).is_none() {
        return;
    }
    if let Err(e) = add_own_art(&data, false) {
        crate::log::line("steam-setup", &format!("Steam library art: {e}"));
    }
}

impl qobject::Turbine {
    /// Starts one Steam restart for a game's change ("options", "undo", "tile", "untile"); `resume` is what Turbine
    /// does once it's back ("play" or "coop" or ""). Answers {"ok","state","words"}: state "running" (the restart
    /// is under way), "done" (Steam was closed: made at once), or ok false with the reason in words.
    pub fn steam_setup_start(&self, game_id: &QString, kind: &QString, resume: &QString) -> QString {
        let Some(g) = self.find(&game_id.to_string()) else { return QString::from("{\"ok\":false,\"state\":\"\",\"words\":\"That game isn't found any more.\"}") };
        let kind = kind.to_string();
        let resume = match resume.to_string().as_str() {
            "play" => format!("play {} solo", g.id()),
            "coop" => format!("play {} coop", g.id()),
            _ => String::new(),
        };
        let data = crate::backend::data_home();
        let r = job_for(&g, &kind, &resume).and_then(|job| {
            if kind == "undo" {
                // 0.6 (break L8): marked undone only once the change is really made (now, or when the restart says done).
                let _ = std::fs::create_dir_all(data.join("steam-setup"));
                let _ = std::fs::write(undo_file(&data), &g.store_id);
            }
            if kind == "options" {
                let _ = ac::still_undone(&data, &g.store_id, None, true);
            }
            start(&self.rust().found, *self.gamemode(), job)
        });
        match (&r, kind.as_str()) {
            (Ok(state), "undo") if *state == "done" => undo_made(&data),
            (Err(_), "undo") => {
                let _ = std::fs::remove_file(undo_file(&data));
            }
            _ => {}
        }
        let json = match r {
            Ok(state) => format!("{{\"ok\":true,\"state\":\"{state}\",\"words\":\"\"}}"),
            Err(e) => format!("{{\"ok\":false,\"state\":\"\",\"words\":{}}}", quote(&e)),
        };
        QString::from(json.as_str())
    }

    /// The restart's state: {"state": "none" | "running" | "done" | "failed", "words", "resume", "what", "age"}
    /// (age in seconds since it was asked).
    pub fn steam_setup_state(&self) -> QString {
        let data = crate::backend::data_home();
        let job = Job::load(&data);
        let (state, words) = match (&job, sr::read_result(&data)) {
            (None, _) => ("none".to_string(), String::new()),
            (Some(_), None) => ("running".to_string(), String::new()),
            (Some(_), Some(r)) => r,
        };
        let (resume, what, age) = job.map(|j| (j.resume, j.what, now_secs().saturating_sub(j.at))).unwrap_or_default();
        QString::from(format!("{{\"state\":{},\"words\":{},\"resume\":{},\"what\":{},\"age\":{age}}}", quote(&state), quote(&words), quote(&resume), quote(&what)).as_str())
    }

    /// The restart is over and Turbine went on (or the player closed its message): its files go.
    pub fn steam_setup_finish(&self) {
        let data = crate::backend::data_home();
        match sr::read_result(&data) {
            Some((state, _)) if state == "done" => undo_made(&data),
            _ => {
                let _ = std::fs::remove_file(undo_file(&data));
            }
        }
        if let Ok(t) = std::fs::read_to_string(sr::log_file(&data)) {
            let tail: Vec<&str> = t.lines().rev().take(12).collect();
            for l in tail.into_iter().rev() {
                crate::log::line("steam-setup", &format!("Restart: {l}"));
            }
        }
        sr::finish(&data);
    }

    /// At start: the changes to Steam's settings made earlier that Steam wrote over (it was still running): their
    /// names, for Turbine to offer them again. Those that held are done. {"lost":["..."]}
    pub fn steam_changes_check(&self) -> QString {
        let data = crate::backend::data_home();
        if steam_setup::pending(&data).is_empty() || Job::load(&data).is_some() {
            return QString::from("{\"lost\":[]}");
        }
        let (held, lost) = steam_setup::check_after_restart_since(&data, steam_started());
        let names: Vec<String> = lost
            .iter()
            .map(|c| match c {
                steam_setup::SteamChange::Options { app_id, .. } => self.rust().found.iter().find(|g| g.store_id == *app_id).map(|g| format!("{}'s launch setting", g.name)).unwrap_or_else(|| format!("launch setting {app_id}")),
                steam_setup::SteamChange::Tile { .. } => "the Co-op tile".to_string(),
                steam_setup::SteamChange::NoTile { .. } => "removing the Co-op tile".to_string(),
            })
            .collect();
        crate::log::line("steam-setup", &format!("Steam settings check: {} held, {} written over by Steam{}", held.len(), lost.len(), if names.is_empty() { String::new() } else { format!(" ({})", names.join(", ")) }));
        let items: Vec<String> = names.iter().map(|n| quote(n)).collect();
        QString::from(format!("{{\"lost\":[{}]}}", items.join(",")).as_str())
    }

    /// The co-op tile for a game: {"there": in Steam's list now}.
    pub fn coop_tile_json(&self, game_id: &QString) -> QString {
        let there = self.find(&game_id.to_string()).is_some_and(|g| tile_there(&g));
        QString::from(format!("{{\"there\":{there}}}").as_str())
    }

    /// 0.62: Turbine's Steam tool for a game (turbine_engine::compat_tool::state_json): is it written, does Steam
    /// list it yet, does the game use it, the steps for the player, the Proton it runs, and how the Co-op tile can
    /// be added here. Read only, except that the tool's folder is written when it is missing.
    pub fn steam_tool_json(&self, game_id: &QString) -> QString {
        use turbine_engine::compat_tool as tool;
        let none = "{\"installed\":false,\"listed\":false,\"chosen\":false,\"route\":\"none\",\"steps\":[],\"restartFirst\":true,\"proton\":\"\",\"protonName\":\"\",\"lastProton\":\"\",\"nothingStarted\":\"\",\"using\":[],\"toolName\":\"Turbine (mods)\",\"tile\":\"none\",\"tileWords\":\"\"}";
        let Some(g) = self.find(&game_id.to_string()) else { return QString::from(none) };
        let Some(root) = crate::anticheat::steam_root() else { return QString::from(none) };
        let data = crate::backend::data_home();
        crate::anticheat::ensure_helper(&data);
        let set = steam::launch_options(&turbine_engine::home_dir(), &g.store_id);
        let running = crate::system::steam_running();
        let ask = tool::Ask {
            steam_root: &root,
            data: &data,
            app_id: &g.store_id,
            game_name: &g.name,
            launch_options_ok: set.as_deref().is_some_and(|s| ac::launch_option_ok(s, &data)),
            steam_started: steam_started(),
            game_mode: *self.gamemode(),
            decky_plugin: crate::system::decky_plugin_installed(),
            steam_running: running != Some(false),
        };
        QString::from(tool::state_json(&ask).as_str())
    }

    /// 0.62: the Proton Turbine's Steam tool runs a game with, for the row "Proton for modded play":
    /// {"name","dir","how" ("you", "steam", "tool" or "auto"),"words","auto","list":[{"name","dir","kind",
    /// "usable","current"}]} (turbine_engine::compat_tool::proton_json).
    pub fn proton_json(&self, game_id: &QString) -> QString {
        let none = "{\"name\":\"\",\"dir\":\"\",\"how\":\"auto\",\"words\":\"\",\"auto\":\"\",\"list\":[]}";
        let (Some(g), Some(root)) = (self.find(&game_id.to_string()), crate::anticheat::steam_root()) else { return QString::from(none) };
        let data = crate::backend::data_home();
        let _ = turbine_engine::compat_tool::remember_proton_with(&root, &data, &g.store_id);
        QString::from(turbine_engine::compat_tool::proton_json(&root, &data, &g.store_id).as_str())
    }

    /// 0.62: the player's pick of the Proton for a game's modded play: one of protonJson's `dir`s, or "" for
    /// Automatic. Answers {"ok","words"}.
    pub fn proton_set(&self, game_id: &QString, dir: &QString) -> QString {
        let answer = |ok: bool, words: &str| QString::from(format!("{{\"ok\":{ok},\"words\":{}}}", quote(words)).as_str());
        let Some(g) = self.find(&game_id.to_string()) else { return answer(false, "That game isn't found any more.") };
        let data = crate::backend::data_home();
        let dir = dir.to_string();
        let pick = (!dir.is_empty()).then(|| PathBuf::from(&dir));
        // Only a Proton from the list (installed now) can be picked.
        if let Some(p) = &pick {
            let listed = crate::anticheat::steam_root().is_some_and(|r| turbine_engine::compat_tool::installed_protons(&r).iter().any(|x| x.dir == *p));
            if !listed {
                return answer(false, "That Proton isn't installed any more.");
            }
        }
        match turbine_engine::compat_tool::set_proton_pick(&data, &g.store_id, pick.as_deref()) {
            Ok(()) => {
                let name = pick.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned());
                crate::log::line("launch", &format!("{}: Proton for modded play: {}", g.name, name.as_deref().unwrap_or("Automatic")));
                answer(true, &match name {
                    Some(n) => format!("{} runs with {n} when it starts with mods.", g.name),
                    None => format!("Turbine chooses the Proton for {} again.", g.name),
                })
            }
            Err(e) => answer(false, &format!("Couldn't keep that choice: {e}")),
        }
    }

    /// 0.62: how the Co-op tile can be added (or taken out) here and now: {"how","words","label"}. "direct":
    /// Desktop Mode with Steam closed (coopTileSet does it at once); "restart": Desktop Mode with Steam running
    /// (one restart, asked first); "later": Game Mode (the words say where); "none": no such tile for this game.
    pub fn coop_tile_offer_json(&self, game_id: &QString) -> QString {
        use turbine_engine::compat_tool as tool;
        let Some(g) = self.find(&game_id.to_string()) else { return QString::from(tool::NO_TILE_JSON) };
        if turbine_engine::rules::GameRules::for_game(&g).anti_cheat.as_ref().and_then(|a| a.me3_game()) != Some("eldenring") {
            return QString::from(tool::NO_TILE_JSON);
        }
        let game_mode = *self.gamemode();
        let plugin = game_mode && crate::system::decky_plugin_installed();
        let way = tool::tile_way(game_mode, plugin, crate::system::steam_running() != Some(false));
        QString::from(way.offer_json(!tile_there(&g)).as_str())
    }

    /// 0.62: adds or takes out the Co-op tile at once. Only with Steam closed (Desktop Mode); otherwise it answers
    /// why not and changes nothing. {"ok","words"}.
    pub fn coop_tile_set(&self, game_id: &QString, add: bool) -> QString {
        let answer = |ok: bool, words: &str| QString::from(format!("{{\"ok\":{ok},\"words\":{}}}", quote(words)).as_str());
        let Some(g) = self.find(&game_id.to_string()) else { return answer(false, "That game isn't found any more.") };
        if crate::system::steam_running() != Some(false) {
            crate::log::line("steam-setup", "Co-op tile: Steam is running, nothing changed");
            return answer(false, turbine_engine::compat_tool::TILE_NEEDS_STEAM_CLOSED);
        }
        let data = crate::backend::data_home();
        match job_for(&g, if add { "tile" } else { "untile" }, "").and_then(|job| sr::apply_job(&data, &job)) {
            Ok(lines) => {
                for l in &lines {
                    crate::log::line("steam-setup", l);
                }
                answer(true, if add { "Co-op tile added. It shows in your Steam library when Steam starts." } else { "Co-op tile removed from your Steam library." })
            }
            Err(e) => {
                crate::log::line("steam-setup", &format!("Co-op tile: {e}"));
                answer(false, &e)
            }
        }
    }

    /// Settings, Steam library art: {"found": Turbine is in Steam's library, "on": its art is wanted}.
    pub fn steam_art_json(&self) -> QString {
        let data = crate::backend::data_home();
        let found = shortcuts::find_turbine_file(&turbine_engine::home_dir()).is_some();
        QString::from(format!("{{\"found\":{found},\"on\":{}}}", !art_off_file(&data).exists()).as_str())
    }

    /// Settings, Steam library art: Add (on) or Remove (off; only Turbine's own pictures go). Answers the words.
    pub fn steam_art_set(&self, on: bool) -> QString {
        let data = crate::backend::data_home();
        let words = if on {
            let _ = std::fs::remove_file(art_off_file(&data));
            add_own_art(&data, true).unwrap_or_else(|e| e)
        } else {
            let _ = std::fs::write(art_off_file(&data), "off\n");
            match shortcuts::find_turbine_file(&turbine_engine::home_dir()) {
                Some((file, appid)) => match steam_art::remove_art(&steam_art::grid_dir(&file), appid, &art_record(&data)) {
                    Ok(gone) => {
                        crate::log::line("steam-setup", &format!("Steam library art for Turbine removed ({} files; your own pictures stay)", gone.len()));
                        "Turbine's art is out of Steam (your own pictures stay). Steam shows its own after it starts again.".to_string()
                    }
                    Err(e) => format!("Couldn't remove it: {e}"),
                },
                None => "Turbine isn't in your Steam library.".to_string(),
            }
        };
        QString::from(words.as_str())
    }

    /// Labs: Proton's own log for Steam launches of me3 games, into the helper's folder (Report a problem picks it up).
    pub fn proton_log_on(&self) -> bool {
        ac::proton_log_file(&crate::backend::data_home()).exists()
    }

    pub fn set_proton_log(&self, on: bool) {
        match ac::set_proton_log(&crate::backend::data_home(), on) {
            Ok(()) => crate::log::line("labs", &format!("Proton log for Steam launches {}", if on { "on" } else { "off" })),
            Err(e) => crate::log::line("labs", &format!("Couldn't change the Proton log setting: {e}")),
        }
    }
}

/// 0.6 (break M3): when the running Steam started (its oldest process, asked on the host), or None when that can't
/// be told (no Steam, or no answer): then a change that is in the file counts as held.
fn steam_started() -> Option<std::time::SystemTime> {
    use std::process::{Command, Stdio};
    let host = |args: &[&str]| {
        let mut c = if std::path::Path::new("/.flatpak-info").exists() {
            let mut c = Command::new("flatpak-spawn");
            c.arg("--host").args(args);
            c
        } else {
            let mut c = Command::new(args[0]);
            c.args(&args[1..]);
            c
        };
        c.stdin(Stdio::null()).stderr(Stdio::null()).output().ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    let pid = host(&["pgrep", "-o", "-x", "steam"])?;
    if pid.is_empty() || !pid.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let secs: u64 = host(&["ps", "-o", "etimes=", "-p", &pid])?.trim().parse().ok()?;
    std::time::SystemTime::now().checked_sub(std::time::Duration::from_secs(secs + 1))
}

/// 0.6 (break L8): the game whose Launch Options an Undo is putting back, until the restart says it's done.
fn undo_file(data: &Path) -> PathBuf {
    data.join("steam-setup").join("undo-for.txt")
}

/// The Undo is made: Turbine's line is out of the Launch Options, so the game is marked undone (and not before).
fn undo_made(data: &Path) {
    let Ok(app_id) = std::fs::read_to_string(undo_file(data)) else { return };
    let _ = std::fs::remove_file(undo_file(data));
    let app_id = app_id.trim().to_string();
    let back = ac::original_options(data, &app_id);
    if let Err(e) = ac::mark_undone(data, &app_id, &back) {
        crate::log::line("launch", &format!("{app_id}: couldn't mark the launch options change undone: {e}"));
        let _ = ac::arm(data, &app_id, None);
    } else {
        crate::log::line("launch", &format!("{app_id}: the launch options change is undone"));
    }
}
