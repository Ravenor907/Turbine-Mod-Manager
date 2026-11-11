//! Games with anti-cheat, the app's side: the ask-once answer, me3 and Seamless Co-op downloads, starting
//! me3 on the host with fixed arguments, the original record of the game folder, and the lines for Before you play.
//! The rules and the checks themselves are the engine's (turbine_engine::anticheat).

use crate::actions::Qt;
use crate::backend::{qobject, Slot, Worker};
use crate::online::Net;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use turbine_engine::anticheat::{self as ac, AcState, Mode, Original};
use turbine_engine::json::quote;
use turbine_engine::rules::{AntiCheat, Check};

/// The game's name as Turbine shows it ("Elden Ring", not "ELDEN RING").
pub(crate) fn title(slot: &Slot) -> String {
    slot.rules.short_name.clone().unwrap_or_else(|| slot.game.name.clone())
}

/// The game's data-file anti-cheat, when it says more than a warning.
pub(crate) fn special(slot: &Slot) -> Option<&AntiCheat> {
    slot.rules.anti_cheat.as_ref().filter(|a| ac::mode(a) != Mode::Warn)
}

fn off_file(store_root: &Path) -> PathBuf {
    store_root.join("anticheat-off")
}

fn original_file(store_root: &Path) -> PathBuf {
    store_root.join("anticheat-original.json")
}

/// The player said "Turn it off for mods" for this game (asked once; Game details can change it).
pub(crate) fn turned_off(slot: &Slot) -> bool {
    off_file(slot.store.root()).exists()
}

pub(crate) fn me3_state(data: &Path) -> Result<(), String> {
    ac::me3_ready(&ac::me3_dir(data))
}

/// The record of the game folder, taken the first time and again after an update.
fn original(slot: &Slot) -> Option<Original> {
    Original::load(&original_file(slot.store.root()))
}

/// Only for a game me3 starts (Halo's mods live in its folder by design; see anticheat::folder_checked).
pub(crate) fn state(slot: &Slot) -> Option<AcState> {
    let a = special(slot).filter(|a| ac::folder_checked(a))?;
    Some(ac::state(&slot.game.install_dir, a, original(slot).as_ref(), slot.game.build.as_deref(), slot.store.files_in_game_folder()))
}

/// Records the starter and the game's program when there is no record yet, or the game updated. Logged.
pub(crate) fn keep_record(slot: &Slot) {
    let Some(a) = special(slot) else { return };
    if ac::watched(a).is_empty() {
        return;
    }
    let st = state(slot);
    if matches!(st, Some(AcState::Unrecorded) | Some(AcState::Updated)) {
        match ac::record_now(&slot.game.install_dir, a, slot.game.build.as_deref()).and_then(|r| r.save(&original_file(slot.store.root())).map(|_| r)) {
            Ok(r) => crate::log::line(
                "anticheat",
                &format!(
                    "{}: original record {} ({} files, build {})",
                    slot.game.name,
                    if st == Some(AcState::Updated) { "renewed after an update" } else { "taken" },
                    r.files.len(),
                    r.build.as_deref().unwrap_or("unknown")
                ),
            ),
            Err(e) => crate::log::line("anticheat", &format!("{}: couldn't record the original files ({e})", slot.game.name)),
        }
    }
}

/// The lines for Before you play.
pub(crate) fn checks(slot: &Slot, data: &Path) -> Vec<Check> {
    let Some(a) = special(slot) else { return Vec::new() };
    let on = slot.store.mods().iter().any(|m| m.enabled);
    let me3 = (ac::mode(a) == Mode::Me3).then(|| me3_state(data));
    let rec = original(slot);
    let build_changed = rec.as_ref().is_some_and(|r| r.build.is_some() && slot.game.build.is_some() && r.build != slot.game.build);
    let mut out = ac::checks(&title(slot), a, on, me3.as_ref(), turned_off(slot), build_changed);
    if let Some(st) = state(slot) && let Some(c) = ac::state_check(&title(slot), &st) {
        out.push(c);
    }
    out
}

/// What the rules JSON says about it (anygame::rules_extra adds these to "antiCheat").
/// An anti-cheat mode as the interface and Steam's menu name it.
pub(crate) fn mode_word(mode: Mode) -> &'static str {
    match mode {
        Mode::Me3 => "me3",
        Mode::Official => "official",
        Mode::Offline => "offline",
        Mode::Refuse => "refuse",
        Mode::Warn => "warn",
    }
}

pub(crate) fn json_fields(slot: &Slot, a: &AntiCheat) -> String {
    let mode = ac::mode(a);
    let name = title(slot);
    let guide: Vec<String> = if mode == Mode::Official { ac::official_guide(&name, a).iter().map(|s| quote(s)).collect() } else { Vec::new() };
    let data = crate::backend::data_home();
    format!(
        ",\"leftover\":{},\"mode\":{},\"me3\":{},\"needsAsk\":{},\"askTitle\":{},\"askText\":{},\"off\":{},\"statusRow\":{},\"refuseWords\":{},\"offlineWords\":{},\"officialWords\":{},\"guide\":[{}],\"me3Ready\":{},\"seamless\":{}",
        quote(&match state(slot) {
            Some(AcState::LeftoverBackup(f)) => f,
            _ => String::new(),
        }),
        quote(mode_word(mode)),
        quote(a.me3_game().unwrap_or("")),
        ac::needs_ask(a),
        quote(&ac::ask_title(&name)),
        quote(&ac::ask_text(&name, a)),
        turned_off(slot),
        quote(ac::STATUS_ROW),
        quote(ac::REFUSE_WORDS),
        quote(&ac::offline_words(a).unwrap_or_default()),
        quote(&if mode == Mode::Official { ac::official_words(&name, a) } else { String::new() }),
        guide.join(","),
        mode == Mode::Me3 && me3_state(&data).is_ok(),
        a.me3_game() == Some("eldenring"),
    )
}

/// Play with mods for a game me3 starts, outside Steam: the profile for what is in the package folder now, then me3
/// on the host. Desktop Mode only (Game Mode shows a game only when Steam started it). Answers false (and says why)
/// when it couldn't start.
pub(crate) fn launch_me3(w: &Worker, slot: &Slot, coop: ac::Coop) -> bool {
    let Some(game) = slot.store.me3_game() else { return false };
    let me3 = ac::me3_dir(&w.data_home);
    if let Err(e) = ac::me3_ready(&me3) {
        w.say(format!("{e}. Press Play again: Turbine gets me3 first."), "warn");
        return false;
    }
    if let Err(why) = coop_ready(slot, coop) {
        w.say(why, "warn");
        return false;
    }
    let (profile, natives) = match ac::write_profile_for(slot.store.root(), &game, coop) {
        Ok(p) => p,
        Err(e) => {
            w.say(format!("Couldn't write the me3 profile: {}", crate::log::plain(&e.to_string())), "error");
            return false;
        }
    };
    let args = ac::launch_args(&me3, &profile);
    crate::log::line(
        "launch",
        &format!("{} through me3 {} outside Steam (offline, no anti-cheat): {} ({} natives)", slot.game.name, ac::ME3_VERSION, args.join(" "), natives.len()),
    );
    let (prog, rest) = if crate::system::in_flatpak() { (&args[0], &args[1..]) } else { (&args[2], &args[3..]) };
    match std::process::Command::new(prog).args(rest).stdin(std::process::Stdio::null()).spawn() {
        Ok(_) => true,
        Err(e) => {
            w.say(format!("Couldn't start me3: {}", crate::log::plain(&e.to_string())), "error");
            false
        }
    }
}

/// A Steam game me3 starts: Steam starts it through Turbine's launch helper.
pub(crate) fn through_steam(slot: &Slot) -> bool {
    slot.game.store == turbine_engine::games::Store::Steam && special(slot).is_some_and(|a| ac::mode(a) == Mode::Me3)
}

/// The helper in Turbine's data folder, written again when it changed. Logged.
pub(crate) fn ensure_helper(data: &Path) {
    // Turbine's Flatpak id, when it runs as one (the helper checks it is still installed).
    let id = crate::system::in_flatpak().then_some(crate::APP_ID);
    match ac::write_helper(data, id) {
        Ok(true) => crate::log::line("launch", &format!("Steam launch helper written: {}", ac::helper_path(data).display())),
        Ok(false) => {}
        Err(e) => crate::log::line("launch", &format!("Couldn't write the Steam launch helper: {e}")),
    }
    // Turbine's Steam tool ("Turbine (mods)") next to Steam's other tools. A new folder Steam only reads, so
    // it can be written while Steam runs; Steam lists it after its next start.
    if let Some(root) = steam_root() {
        match turbine_engine::compat_tool::install(&root, data) {
            Ok(i) if !i.written.is_empty() => crate::log::line(
                "launch",
                &format!(
                    "Steam tool \"Turbine (mods)\" written to {} ({}){}",
                    turbine_engine::compat_tool::tool_dir(&root).display(),
                    i.written.join(", "),
                    if i.new_folder { ": Steam lists it after its next start" } else { "" }
                ),
            ),
            Ok(_) => {}
            Err(e) => crate::log::line("launch", &format!("Couldn't write the Steam tool: {e}")),
        }
    }
}

/// Before a co-op Play: the Seamless Co-op the game loads is the one in the list, and it has a session
/// password. What the check found goes to the activity log (never the password); a version that disagrees is
/// logged as a warning and doesn't stop the Play. Err: the words for the player.
pub(crate) fn coop_ready(slot: &Slot, coop: ac::Coop) -> Result<(), String> {
    if coop != ac::Coop::Seamless {
        return Ok(());
    }
    let c = slot.store.coop_check();
    crate::log::line(
        "launch",
        &format!(
            "{}: Seamless Co-op in the list: {} {}; its file says {}; in the game's mod folder: {}; session password: {}{}",
            slot.game.name,
            c.name.clone().unwrap_or_else(|| "none".into()),
            c.list_version.clone().unwrap_or_default(),
            c.file_version.clone().unwrap_or_else(|| "no version".into()),
            if c.profile_current { "the same file" } else { "a different file" },
            c.password.word(),
            if c.others.is_empty() { String::new() } else { format!("; not used: {}", c.others.join(", ")) }
        ),
    );
    if let Some(w) = c.warning() {
        crate::log::line("launch", &format!("{}: {w}", slot.game.name));
    }
    match c.blocks() {
        Some(why) => Err(why),
        None => Ok(()),
    }
}

/// What the engine's look at Seamless Co-op found, for the interface (`coopChecked`). `tidied`: the copies a
/// tidy just moved to Removed mods.
fn post_coop_check(w: &mut Worker, game: &turbine_engine::Game, tidied: &[String]) {
    let Ok(s) = w.slot(game) else { return };
    let c = s.store.coop_check();
    let names = |l: &[String]| l.iter().map(|n| quote(n)).collect::<Vec<_>>().join(",");
    let version_warning = if c.version_differs().is_some() { c.warning().unwrap_or_default() } else { String::new() };
    let json = format!(
        "{{\"installed\":{},\"name\":{},\"listVersion\":{},\"fileVersion\":{},\"others\":[{}],\"versionWarning\":{},\"tidied\":[{}]}}",
        c.mod_id.is_some(),
        quote(c.name.as_deref().unwrap_or("")),
        quote(c.list_version.as_deref().unwrap_or("")),
        quote(c.file_version.as_deref().unwrap_or("")),
        names(&c.others),
        quote(&version_warning),
        names(tidied),
    );
    crate::log::line("engine", &format!("{}: Seamless Co-op looked at: {} other copies{}", game.name, c.others.len(), if version_warning.is_empty() { "" } else { ", its version and its file's disagree" }));
    let id = game.id();
    w.post(move |t| t.coop_checked(QString::from(id.as_str()), QString::from(json.as_str())));
}

/// Steam's own folder (the first one Steam uses on this device).
pub(crate) fn steam_root() -> Option<PathBuf> {
    turbine_engine::steam::steam_roots(&turbine_engine::home_dir()).into_iter().next()
}

/// Does Steam start this game through Turbine's Steam tool (chosen in the game's Properties, Compatibility)?
pub(crate) fn via_tool(slot: &Slot, data: &Path, options_ok: bool) -> bool {
    use turbine_engine::compat_tool as tool;
    steam_root().is_some_and(|r| tool::route(&r, data, &slot.game.store_id, options_ok) == tool::Route::Tool)
}

/// The game's Launch Options in Steam as they are now: (set, the text Turbine needs, whether they run the helper).
pub(crate) fn launch_option_state(slot: &Slot, data: &Path) -> (Option<String>, String, bool) {
    let set = turbine_engine::steam::launch_options(&turbine_engine::home_dir(), &slot.game.store_id);
    let need = ac::launch_option(data);
    let options_ok = set.as_deref().is_some_and(|s| ac::launch_option_ok(s, data));
    // The Proton Steam itself last used for the game is kept for Turbine's Steam tool, and a game that uses
    // the tool is set up whatever its Launch Options say. Choosing the tool after an Undo sets it up again.
    let id = &slot.game.store_id;
    // (At every Play, launches through the tool included; default.proton is kept as well.)
    let _ = match steam_root() {
        Some(root) => turbine_engine::compat_tool::remember_proton_with(&root, data, id),
        None => turbine_engine::compat_tool::remember_proton(data, id),
    };
    let tool = via_tool(slot, data, options_ok);
    if tool && ac::undone_file(data, id).exists() {
        let _ = ac::still_undone(data, id, None, true);
        crate::log::line("launch", &format!("{}: Steam starts it through Turbine's Steam tool, so the earlier Undo no longer holds", slot.game.name));
    }
    (set, need, options_ok || tool)
}

/// The last apply of this game's mods failed: until one works, Steam's Play never starts me3.
fn doubt_file(slot: &Slot) -> PathBuf {
    slot.store.root().join("steam-launch-doubt")
}

/// Play without mods until the mods that may break update (the game update warning): Steam's Play too.
pub(crate) fn hold_file(store_root: &Path) -> PathBuf {
    store_root.join("hold-mods")
}

pub(crate) fn set_doubt(slot: &Slot, doubt: bool) {
    let f = doubt_file(slot);
    let _ = if doubt { std::fs::write(&f, b"the last apply failed\n") } else { std::fs::remove_file(&f) };
}

/// Why Steam's own Play must not start me3 now (None: mods on, and nothing in doubt).
pub(crate) fn not_armed_why(w: &Worker, slot: &Slot) -> Option<&'static str> {
    let on = slot.store.mods().iter().filter(|m| m.enabled).count();
    let consent = !special(slot).is_some_and(ac::needs_ask) || turned_off(slot);
    // Undo the launch options change sticks until the line is set again (read only when there is a mark).
    let id = &slot.game.store_id;
    let undone = ac::undone_file(&w.data_home, id).exists()
        && ac::still_undone(&w.data_home, id, turbine_engine::steam::launch_options(&turbine_engine::home_dir(), id).as_deref(), false);
    ac::helper_state_undone(on, consent, hold_file(slot.store.root()).exists(), doubt_file(slot).exists(), ac::me3_ready(&ac::me3_dir(&w.data_home)).is_ok(), undone).err()
}

/// Play with mods (or Seamless Co-op, for this Play only) through Steam. Answers Ok(true) when Steam can start it
/// now, Ok(false) when the Launch Options aren't set yet (nothing armed, logged), and Err with why it couldn't.
pub(crate) fn arm_me3(w: &Worker, slot: &Slot, coop: ac::Coop) -> Result<bool, String> {
    let game = slot.store.me3_game().ok_or("not a game me3 starts")?;
    let data = &w.data_home;
    let me3 = ac::me3_dir(data);
    ac::me3_ready(&me3)?;
    ensure_helper(data);
    let (set, need, ok) = launch_option_state(slot, data);
    let words = match &set {
        None => "not found (the game isn't in Steam's settings yet)".to_string(),
        Some(s) if s.is_empty() => "empty".to_string(),
        Some(s) => format!("\"{s}\"{}", if set.as_deref().is_some_and(|x| ac::launch_option_ok(x, data)) { " (they run Turbine's helper)" } else { " (they don't run Turbine's helper)" }),
    };
    if via_tool(slot, data, false) {
        crate::log::line("launch", &format!("{}: Steam starts it through Turbine's Steam tool (\"Turbine (mods)\")", slot.game.name));
    }
    // The long line once per run, and again only when the Launch Options changed.
    static SAID: std::sync::OnceLock<std::sync::Mutex<turbine_engine::report::LogOnce>> = std::sync::OnceLock::new();
    let text = format!("{}: Steam Launch Options {words}; Turbine needs \"{need}\"", slot.game.name);
    if SAID.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner()).changed(&slot.game.store_id, &text) {
        crate::log::line("launch", &text);
    }
    // Undone and not set again: the one-time step, with "Use the line again".
    let undone = ac::undone_file(data, &slot.game.store_id).exists() && ac::still_undone(data, &slot.game.store_id, set.as_deref(), false);
    if !ok || undone {
        if undone {
            crate::log::line("launch", &format!("{}: the launch options change was undone; not started with mods until it is set again", slot.game.name));
        }
        if let Some(s) = &set && let Err(e) = ac::remember_original(data, &slot.game.store_id, s) {
            crate::log::line("launch", &format!("{}: couldn't keep the Launch Options as they were: {e}", slot.game.name));
        }
        return Ok(false);
    }
    if let Some(why) = not_armed_why(w, slot) {
        let _ = ac::arm(data, &slot.game.store_id, None);
        return Err(format!("not started with mods: {why}"));
    }
    // The normal state first (solo), then this Play's own when it is Seamless Co-op. A Play without mods still
    // waiting from before goes: this is a Play with mods.
    sync_helper(w, slot);
    let _ = ac::clear_plain(data, &slot.game.store_id);
    coop_ready(slot, coop)?;
    let (profile, natives) = ac::write_profile_for(slot.store.root(), &game, coop).map_err(|e| format!("Couldn't write the me3 profile: {}", crate::log::plain(&e.to_string())))?;
    let args = ac::helper_args_for(&me3, &profile, game_exe(slot).as_deref());
    let r = match coop {
        ac::Coop::Seamless => ac::arm_once(data, &slot.game.store_id, &args, "Seamless Co-op, this Play only"),
        ac::Coop::Solo => ac::arm_with(data, &slot.game.store_id, Some(&args), &state_words(slot)),
    };
    r.map_err(|e| format!("Couldn't get the launch helper ready: {e}"))?;
    crate::log::line(
        "launch",
        &format!(
            "{} {} through Steam and me3 {} (offline, no anti-cheat): helper armed with {} ({} natives)",
            slot.game.name,
            if coop == ac::Coop::Seamless { "Seamless Co-op (this Play only)" } else { "with mods" },
            ac::ME3_VERSION,
            args.join(" "),
            natives.len()
        ),
    );
    Ok(true)
}

fn state_words(slot: &Slot) -> String {
    let on = slot.store.mods().iter().filter(|m| m.enabled).count();
    format!("mods on: {on} {} through me3, solo, written by Turbine", if on == 1 { "mod" } else { "mods" })
}

/// The game's own program in its folder (from the game data's gameExe): me3's Windows build starts it under Steam's
/// own Proton. None when the data doesn't name it or it isn't there: the helper then gets the Linux me3.
fn game_exe(slot: &Slot) -> Option<std::path::PathBuf> {
    let rel = special(slot)?.game_exe.as_ref()?;
    Some(slot.game.install_dir.join(rel)).filter(|p| p.is_file())
}

/// The normal launch for a game me3 starts (no mods on, or Play without mods): the helper runs Steam's own command.
/// `once`: Play without mods (the mods stay on for the next Play). Answers false when that couldn't be set up: the
/// game must not be started then (the helper would start me3).
#[must_use]
pub(crate) fn arm_plain(w: &Worker, slot: &Slot, once: bool) -> bool {
    if !through_steam(slot) {
        return true;
    }
    let data = &w.data_home;
    ensure_helper(data);
    let id = &slot.game.store_id;
    let armed = ac::armed_file(data, id).exists();
    let r = if once && armed { ac::arm_plain_once(data, id) } else { ac::arm(data, id, None) };
    match r {
        Ok(()) => {
            crate::log::line(
                "launch",
                &format!("{}: the launch helper runs Steam's own command{}", slot.game.name, if once && armed { " this once (Play without mods)" } else { " (no mods on)" }),
            );
            true
        }
        Err(e) => {
            crate::log::line("launch", &format!("{}: couldn't set the launch helper to Steam's own command: {e}", slot.game.name));
            // On any doubt, nothing armed at all.
            let _ = ac::arm(data, id, None);
            !ac::armed_file(data, id).exists()
        }
    }
}

/// The helper's file for a game, kept to Turbine's current state after anything changed (Steam's own Play button
/// uses it): armed only while mods are on, the player turned the anti-cheat off for them, nothing holds them back,
/// the last apply worked and me3 is there; cleared otherwise. Logged when it changes.
pub(crate) fn sync_helper(w: &Worker, slot: &Slot) {
    if !through_steam(slot) {
        return;
    }
    let data = &w.data_home;
    let id = &slot.game.store_id;
    let was = ac::armed_file(data, id).exists();
    let why = not_armed_why(w, slot);
    let game = slot.store.me3_game();
    let args = match (why, &game) {
        (None, Some(g)) => ac::write_profile_for(slot.store.root(), g, ac::Coop::Solo).ok().map(|(p, _)| ac::helper_args_for(&ac::me3_dir(data), &p, game_exe(slot).as_deref())),
        _ => None,
    };
    let r = match &args {
        Some(a) => ac::arm_with(data, id, Some(a), &state_words(slot)),
        None => ac::arm(data, id, None),
    };
    if let Err(e) = r {
        crate::log::line("launch", &format!("{}: couldn't update the launch helper ({e}); cleared", slot.game.name));
        let _ = std::fs::remove_file(ac::armed_file(data, id));
        let _ = std::fs::remove_file(ac::once_file(data, id));
        return;
    }
    if was != args.is_some() {
        crate::log::line(
            "launch",
            &match why {
                None => format!("{}: mods on, so Steam's Play starts it with them through me3", slot.game.name),
                Some(w) => format!("{}: Steam's Play starts it normally ({w})", slot.game.name),
            },
        );
    }
}

/// The size of the helper's log now (what it writes after this is the next launch's).
pub(crate) fn helper_log_len(data: &Path) -> u64 {
    std::fs::metadata(ac::helper_log(data)).map(|m| m.len()).unwrap_or(0)
}

/// Copies the helper's log lines since `from` into the activity log.
pub(crate) fn helper_log_to_activity(data: &Path, from: u64) {
    for l in ac::helper_log_since(data, from) {
        crate::log::line("launch", &format!("Steam launch helper: {l}"));
    }
}

/// Before the normal launch with the anti-cheat (no mods on, or Play without mods): the game folder must be
/// original. Answers the reason it isn't.
pub(crate) fn normal_launch_blocked(slot: &Slot) -> Option<String> {
    special(slot)?;
    match state(slot)? {
        AcState::ChangedOutside(files) => Some(format!(
            "{} isn't as Steam installed it ({}), so Turbine didn't start it with its anti-cheat. {}",
            slot.game.name,
            files.join(", "),
            ac::VERIFY_WORDS
        )),
        AcState::TurbineFilesLeft(n) => Some(format!(
            "{n} mod files are still in {}'s folder, so Turbine didn't start it with its anti-cheat. {}",
            slot.game.name,
            ac::VERIFY_WORDS
        )),
        _ => None,
    }
}

/// The words when an online-only game's mod files from an earlier version came out of its folder.
pub(crate) fn refused_cleanup(w: &Worker, slot: &Slot) {
    match slot.store.refused_cleanup() {
        Ok(0) => {}
        Ok(n) => w.say(
            format!(
                "{} needs its anti-cheat, so Turbine took its mod files out of the game folder ({n}). Your mods stay listed in Turbine, but they never go into this game.",
                title(slot)
            ),
            "warn",
        ),
        Err(e) => w.say(format!("Couldn't take the mod files out of {}'s folder: {e}. {}", title(slot), ac::VERIFY_WORDS), "error"),
    }
}

impl qobject::Turbine {
    /// Play without mods until they update (the game update warning), for Turbine's and Steam's Play alike.
    pub fn set_mod_hold(mut self: Pin<&mut Self>, on: bool) {
        let Some(game) = self.current_game() else { return };
        self.as_mut().send(
            Some(game.id()),
            "Hold mods until they update",
            false,
            Box::new(move |w: &mut Worker| {
                let Ok(s) = w.slot(&game) else { return };
                let f = hold_file(s.store.root());
                let r = if on { std::fs::write(&f, b"Play without mods until they update\n") } else { std::fs::remove_file(&f).or(Ok(())) };
                match r {
                    Ok(()) => crate::log::line("launch", &format!("{}: mods {} (Turbine's Play and Steam's)", game.name, if on { "held until they update" } else { "back on for Play" })),
                    Err(e) => w.say(format!("Couldn't save that: {}", crate::log::plain(&e.to_string())), "error"),
                }
                w.show(&game, false);
            }),
        );
    }

    /// "Use the line again" after Undo: the helper may start me3 again (Play then asks as usual).
    pub fn launch_redo(self: Pin<&mut Self>, game: &QString) {
        let Some(g) = self.find(&game.to_string()) else { return };
        let data = crate::backend::data_home();
        ac::still_undone(&data, &g.store_id, None, true);
        crate::log::line("launch", &format!("{}: the launch options line is used again", g.name));
    }

    /// The me3 games whose Steam Launch Options run Turbine's helper, and what to paste back.
    pub fn launch_undo_json(&self, current: bool) -> QString {
        let data = crate::backend::data_home();
        let cur = self.current_game().map(|g| g.id());
        let items: Vec<String> = self
            .rust()
            .found
            .iter()
            .filter(|g| g.store == turbine_engine::games::Store::Steam)
            .filter(|g| !current || cur.as_deref() == Some(g.id().as_str()))
            .filter(|g| turbine_engine::rules::GameRules::for_game(g).anti_cheat.as_ref().is_some_and(|a| ac::mode(a) == Mode::Me3))
            .filter_map(|g| {
                let set = turbine_engine::steam::launch_options(&turbine_engine::home_dir(), &g.store_id).unwrap_or_default();
                let uses = ac::launch_option_uses_helper(&set, &data);
                (uses || current).then(|| {
                    format!(
                        "{{\"game\":{},\"name\":{},\"uses\":{uses},\"set\":{},\"original\":{},\"line\":{}}}",
                        quote(&g.id()),
                        quote(&g.name),
                        quote(&set),
                        quote(&ac::original_options(&data, &g.store_id)),
                        quote(&ac::launch_option(&data))
                    )
                })
            })
            .collect();
        QString::from(format!("[{}]", items.join(",")).as_str())
    }

    /// Undo the launch options change for a game: the helper never starts me3 for it again until Play asks for the
    /// step again (Turbine shows the text to paste back).
    pub fn launch_undo(self: Pin<&mut Self>, game: &QString) {
        let Some(g) = self.find(&game.to_string()) else { return };
        let data = crate::backend::data_home();
        let set = turbine_engine::steam::launch_options(&turbine_engine::home_dir(), &g.store_id).unwrap_or_default();
        if let Err(e) = ac::mark_undone(&data, &g.store_id, &set) {
            crate::log::line("launch", &format!("{}: couldn't mark the launch options change undone: {e}", g.name));
            let _ = ac::arm(&data, &g.store_id, None);
        }
        let _ = ac::clear_plain(&data, &g.store_id);
        crate::log::line("launch", &format!("{}: Undo the launch options change: the helper starts nothing for it now; the player pastes back \"{}\"", g.name, ac::original_options(&data, &g.store_id)));
    }

    /// The Seamless Co-op copies in this game's list (the engine's coop_check); answered as `coopChecked`.
    pub fn coop_check(mut self: Pin<&mut Self>) {
        let Some(game) = self.current_game() else { return };
        self.as_mut().send(Some(game.id()), "Look at Seamless Co-op", false, Box::new(move |w: &mut Worker| post_coop_check(w, &game, &[])));
    }

    /// Two copies of Seamless Co-op: the one the game loads (the newest) stays, the others go to Removed mods
    /// (the engine's seamless_tidy carries a session password over). Then the list and `coopChecked` again.
    pub fn seamless_tidy(mut self: Pin<&mut Self>) {
        let Some(game) = self.current_game() else { return };
        self.as_mut().edited();
        self.as_mut().send(
            Some(game.id()),
            "Tidy up Seamless Co-op",
            false,
            Box::new(move |w: &mut Worker| {
                let gone = match w.slot(&game).and_then(|s| s.store.seamless_tidy().map_err(|e| e.to_string())) {
                    Ok(g) => {
                        if !g.is_empty() {
                            crate::log::line("engine", &format!("{}: Seamless Co-op tidied: moved to Removed mods: {}", game.name, g.join(", ")));
                            // The words name the version kept (the highest, turned on).
                            let words = w.slot(&game).map(|s| s.store.tidy_words(&g)).unwrap_or_else(|_| format!("Moved to Removed mods: {}.", g.join(", ")));
                            w.say(words, "ok");
                        }
                        g
                    }
                    Err(e) => {
                        w.say(format!("Couldn't tidy up Seamless Co-op: {}", crate::log::plain(&e)), "error");
                        Vec::new()
                    }
                };
                w.show(&game, false);
                post_coop_check(w, &game, &gone);
            }),
        );
    }

    /// Removes only another tool's "<starter>.backup" from the game folder, after a hold to confirm, and only while
    /// the starter itself is as Turbine recorded it.
    pub fn remove_ac_leftover(mut self: Pin<&mut Self>) {
        let Some(game) = self.current_game() else { return };
        self.as_mut().send(
            Some(game.id()),
            "Remove another tool's leftover file",
            false,
            Box::new(move |w: &mut Worker| {
                let Ok(s) = w.slot(&game) else { return };
                let Some(AcState::LeftoverBackup(rel)) = state(s) else {
                    w.say("There's no leftover file to remove now.", "warn");
                    return;
                };
                let path = s.game.install_dir.join(&rel);
                let name = title(s);
                match std::fs::remove_file(&path) {
                    Ok(()) => {
                        crate::log::line("anticheat", &format!("{name}: removed another tool's leftover {rel} (the starter is as recorded)"));
                        w.say(format!("Removed {rel}. {name}'s anti-cheat files are as Turbine recorded them."), "ok");
                    }
                    Err(e) => w.say(format!("Couldn't remove {rel}: {}", crate::log::plain(&e.to_string())), "error"),
                }
                w.show(&game, true);
            }),
        );
    }

    /// The ask-once answer: true is "Turn it off for mods", false turns it back on (Game details).
    pub fn anti_cheat_off(mut self: Pin<&mut Self>, on: bool) {
        let Some(game) = self.current_game() else { return };
        self.as_mut().send(
            Some(game.id()),
            "Anti-cheat answer",
            false,
            Box::new(move |w: &mut Worker| {
                let Ok(s) = w.slot(&game) else { return };
                let f = off_file(s.store.root());
                let r = if on { std::fs::write(&f, b"Turned off for mods by the player\n") } else { std::fs::remove_file(&f).or(Ok(())) };
                match r {
                    Ok(()) => crate::log::line(
                        "anticheat",
                        &format!("{}: anti-cheat {} (asked once)", game.name, if on { "off while mods are on: me3 starts it offline" } else { "left on: mods wait" }),
                    ),
                    Err(e) => w.say(format!("Couldn't save the answer: {}", crate::log::plain(&e.to_string())), "error"),
                }
                // show() keeps the launch helper to this answer (anti-cheat back on: Steam's Play starts it normally).
                w.show(&game, true);
            }),
        );
    }

    /// Installs Microsoft's Visual C++ 2022 runtime into the current game's Proton prefix (Cyberpunk's RED4ext
    /// and Cyber Engine Tweaks need it): download and check the installer, tell Wine to use Microsoft's DLLs, run the
    /// installer through the game's Proton, then look again.
    pub fn install_vc_runtime(mut self: Pin<&mut Self>) {
        let Some(game) = self.current_game() else { return };
        let Some(prefix) = game.prefix.clone().filter(|p| p.join("drive_c").is_dir()) else {
            self.as_mut().say("Start the game once from Steam (Proton sets it up then), then try again.", "error");
            return;
        };
        if crate::system::game_running(&game.install_dir) == Some(true) {
            self.as_mut().say(&format!("Close {} first: the runtime can't be installed while it runs.", game.name), "error");
            return;
        }
        self.as_mut().say("Getting the Visual C++ runtime from Microsoft...", "work");
        let data = crate::backend::data_home();
        self.net_send(
            "Install the Visual C++ runtime",
            Box::new(move |net: &mut Net, qt: &Qt| {
                use turbine_engine::vcrun;
                let mut prog = crate::actions::progress(qt);
                let exe = match net.fetch_vc_redist(&mut prog) {
                    Ok(e) => e,
                    Err(e) => return crate::actions::say(qt, format!("Couldn't get the Visual C++ runtime: {e}"), "error"),
                };
                let fail = |words: String| {
                    crate::log::line("vcrun", &words);
                    crate::actions::say(qt, words, "error");
                };
                let Some(root) = steam_root() else { return fail("Steam's folder wasn't found, so Proton can't be started.".into()) };
                let libs = turbine_engine::compat_tool::library_folders(&root);
                let recorded = turbine_engine::compat_tool::proton_choice(&data, &game.store_id);
                let Some(proton) = vcrun::find_proton(&libs, recorded.as_deref()) else {
                    return fail("No Proton was found. Start the game once from Steam (it installs Proton), then try again.".into());
                };
                let Some(runtime) = vcrun::runtime_command(&libs, &proton) else {
                    return fail(format!("{} needs the Steam Linux Runtime, which isn't installed. Start the game once from Steam, then try again.", proton.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()));
                };
                if let Err(e) = vcrun::set_overrides(&prefix) {
                    return fail(format!("Couldn't tell Proton to use Microsoft's runtime: {e}"));
                }
                crate::actions::say(qt, "Installing the Visual C++ runtime into the game's Proton setup (about a minute)...", "work");
                let compat = prefix.parent().map(Path::to_path_buf).unwrap_or_else(|| prefix.clone());
                let args = vcrun::install_args(&compat, &root, &runtime, &proton, &exe);
                crate::log::line("vcrun", &format!("Running: {}", args.join(" ")));
                let out = std::process::Command::new(&args[0]).args(&args[1..]).stdin(std::process::Stdio::null()).output();
                let _ = std::fs::remove_file(&exe);
                let code = match out {
                    Ok(o) => {
                        let tail = String::from_utf8_lossy(&o.stderr).lines().rev().take(8).collect::<Vec<_>>().join(" | ");
                        crate::log::line("vcrun", &format!("Installer finished with {:?}: {tail}", o.status.code()));
                        o.status.code().unwrap_or(-1)
                    }
                    Err(e) => return fail(format!("Proton couldn't be started ({e}).")),
                };
                let after = vcrun::state(Some(&prefix));
                crate::log::line("vcrun", &format!("After the installer: {}", after.words()));
                if !after.needs_install() {
                    crate::actions::say(qt, format!("The Visual C++ runtime is installed for {}. RED4ext and Cyber Engine Tweaks can load now.", game.name), "ok");
                } else if !vcrun::installer_ok(code) {
                    fail(vcrun::installer_words(code));
                } else {
                    fail(format!("The installer finished, but {} The details are in the activity log.", after.words().to_lowercase()));
                }
            }),
        );
    }

    /// Gets me3 (pinned, checked), then plays when `then_play`.
    pub fn install_me3(mut self: Pin<&mut Self>, then_play: bool) {
        let Some(game) = self.current_game() else { return };
        self.as_mut().say("Getting me3 from its GitHub release...", "work");
        let data = crate::backend::data_home();
        self.net_send(
            "Install me3",
            Box::new(move |net: &mut Net, qt: &Qt| {
                let mut prog = crate::actions::progress(qt);
                match net.fetch_me3(&data, &mut prog) {
                    Ok(_) => {
                        crate::actions::say(qt, "me3 is installed.", "ok");
                        let id = game.id();
                        crate::actions::set(qt, move |t| {
                            if then_play {
                                t.play(&QString::from(id.as_str()));
                            }
                        });
                    }
                    Err(e) => crate::actions::say(qt, format!("Couldn't get me3: {e}"), "error"),
                }
            }),
        );
    }

    /// Gets Seamless Co-op from its GitHub release (the newest one, checked) and adds it to the current game like a
    /// loader. GitHub can lag behind the mod's Nexus page, so with a copy installed that is as new or newer
    /// nothing is downloaded and the toast says so; otherwise the toast says which version is coming. (Opening the
    /// Nexus page first, anticheat::SEAMLESS_NEXUS, is the interface's choice; this is the download without Nexus.)
    pub fn install_seamless(mut self: Pin<&mut Self>) {
        let Some(game) = self.current_game() else { return };
        let have = turbine_engine::quick::existing_root(&game, &crate::backend::data_home()).and_then(|r| turbine_engine::quick::peek(&r).seamless_version);
        self.as_mut().say("Looking at Seamless Co-op's newest GitHub release...", "work");
        self.net_send(
            "Install Seamless Co-op",
            Box::new(move |net: &mut Net, qt: &Qt| {
                let mut prog = crate::actions::progress(qt);
                match net.fetch_seamless(have.as_deref(), &mut prog) {
                    Ok((Some(f), words)) => {
                        crate::log::line("net", &words);
                        // The toast names the version that is coming.
                        crate::actions::say(qt, format!("Getting {}...", f.label), "work");
                        let g = game.clone();
                        crate::actions::set(qt, move |t| t.add_fetched(g, f));
                    }
                    Ok((None, words)) => crate::actions::say(qt, words, "ok"),
                    Err(e) => crate::actions::say(qt, format!("Couldn't get Seamless Co-op: {e}"), "error"),
                }
            }),
        );
    }
}
