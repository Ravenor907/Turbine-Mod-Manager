//! Games with anti-cheat (0.57), the app's side: the ask-once answer, me3 and Seamless Co-op downloads, starting
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
    if let Some(st) = state(slot) {
        if let Some(c) = ac::state_check(&title(slot), &st) {
            out.push(c);
        }
    }
    out
}

/// What the rules JSON says about it (anygame::rules_extra adds these to "antiCheat").
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
        quote(match mode {
            Mode::Me3 => "me3",
            Mode::Official => "official",
            Mode::Offline => "offline",
            Mode::Refuse => "refuse",
            Mode::Warn => "warn",
        }),
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
    let (prog, rest) = if Path::new("/.flatpak-info").exists() { (&args[0], &args[1..]) } else { (&args[2], &args[3..]) };
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

/// Turbine's Flatpak id, when it runs as one (the helper checks it is still installed).
const FLATPAK_ID: &str = "io.github.ravenor907.TurbineModManager";

/// The helper in Turbine's data folder, written again when it changed. Logged.
pub(crate) fn ensure_helper(data: &Path) {
    let id = Path::new("/.flatpak-info").exists().then_some(FLATPAK_ID);
    match ac::write_helper(data, id) {
        Ok(true) => crate::log::line("launch", &format!("Steam launch helper written: {}", ac::helper_path(data).display())),
        Ok(false) => {}
        Err(e) => crate::log::line("launch", &format!("Couldn't write the Steam launch helper: {e}")),
    }
}

/// The game's Launch Options in Steam as they are now: (set, the text Turbine needs, whether they run the helper).
pub(crate) fn launch_option_state(slot: &Slot, data: &Path) -> (Option<String>, String, bool) {
    let set = turbine_engine::steam::launch_options(&turbine_engine::home_dir(), &slot.game.store_id);
    let need = ac::launch_option(data);
    let ok = set.as_deref().is_some_and(|s| ac::launch_option_ok(s, data));
    (set, need, ok)
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
        Some(s) => format!("\"{s}\"{}", if ok { " (they run Turbine's helper)" } else { " (they don't run Turbine's helper)" }),
    };
    crate::log::line("launch", &format!("{}: Steam Launch Options {words}; Turbine needs \"{need}\"", slot.game.name));
    // Undone and not set again: the one-time step, with "Use the line again".
    let undone = ac::undone_file(data, &slot.game.store_id).exists() && ac::still_undone(data, &slot.game.store_id, set.as_deref(), false);
    if !ok || undone {
        if undone {
            crate::log::line("launch", &format!("{}: the launch options change was undone; not started with mods until it is set again", slot.game.name));
        }
        if let Some(s) = &set {
            if let Err(e) = ac::remember_original(data, &slot.game.store_id, s) {
                crate::log::line("launch", &format!("{}: couldn't keep the Launch Options as they were: {e}", slot.game.name));
            }
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
    let (profile, natives) = ac::write_profile_for(slot.store.root(), &game, coop).map_err(|e| format!("Couldn't write the me3 profile: {}", crate::log::plain(&e.to_string())))?;
    let args = ac::helper_args(&me3, &profile);
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
        (None, Some(g)) => ac::write_profile_for(slot.store.root(), g, ac::Coop::Solo).ok().map(|(p, _)| ac::helper_args(&ac::me3_dir(data), &p)),
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
            "{n} mod files are still in {}'s folder, so Turbine didn't start it with its anti-cheat. Game menu, Reset game files, or {}",
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

    /// Gets me3 (pinned, checked), then plays when `then_play`.
    pub fn install_me3(mut self: Pin<&mut Self>, then_play: bool) {
        let Some(game) = self.current_game() else { return };
        self.as_mut().say("Getting me3 from its GitHub release...", "work");
        let data = crate::backend::data_home();
        self.net_send(
            "Install me3",
            Box::new(move |net: &mut Net, qt: &Qt| {
                let mut prog = crate::actions::progress_for(qt);
                match net.fetch_me3(&data, &mut prog) {
                    Ok(_) => {
                        crate::actions::say_for(qt, "me3 is installed.", "ok");
                        let id = game.id();
                        crate::actions::post_for(qt, move |t| {
                            if then_play {
                                t.play(&QString::from(id.as_str()));
                            }
                        });
                    }
                    Err(e) => crate::actions::say_for(qt, format!("Couldn't get me3: {e}"), "error"),
                }
            }),
        );
    }

    /// Gets Seamless Co-op (pinned, checked) and adds it to the current game like a loader.
    pub fn install_seamless(mut self: Pin<&mut Self>) {
        let Some(game) = self.current_game() else { return };
        self.as_mut().say("Getting Seamless Co-op from its GitHub release...", "work");
        self.net_send(
            "Install Seamless Co-op",
            Box::new(move |net: &mut Net, qt: &Qt| {
                let mut prog = crate::actions::progress_for(qt);
                match net.fetch_seamless(&mut prog) {
                    Ok(f) => {
                        let g = game.clone();
                        crate::actions::post_for(qt, move |t| t.add_fetched(g, f));
                    }
                    Err(e) => crate::actions::say_for(qt, format!("Couldn't get Seamless Co-op: {e}"), "error"),
                }
            }),
        );
    }
}
