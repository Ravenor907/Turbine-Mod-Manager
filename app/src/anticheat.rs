//! Games with anti-cheat (0.57), the app's side: the ask-once answer, me3 and Seamless Co-op downloads, starting
//! me3 on the host with fixed arguments, the original record of the game folder, and the lines for Before you play.
//! The rules and the checks themselves are the engine's (turbine_engine::anticheat).

use crate::actions::Qt;
use crate::backend::{qobject, Slot, Worker};
use crate::online::Net;
use cxx_qt_lib::QString;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use turbine_engine::anticheat::{self as ac, AcState, Mode, Original};
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
    use turbine_engine::json::quote;
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

/// Play with mods for a game me3 starts: the profile for what is in the package folder now, then me3 on the host.
/// Answers false (and says why) when it couldn't start.
pub(crate) fn launch_me3(w: &Worker, slot: &Slot) -> bool {
    let Some(game) = slot.store.me3_game() else { return false };
    let me3 = ac::me3_dir(&w.data_home);
    if let Err(e) = ac::me3_ready(&me3) {
        w.say(format!("{e}. Press Play again: Turbine gets me3 first."), "warn");
        return false;
    }
    let (profile, natives) = match ac::write_profile(slot.store.root(), &game) {
        Ok(p) => p,
        Err(e) => {
            w.say(format!("Couldn't write the me3 profile: {e}"), "error");
            return false;
        }
    };
    let args = ac::launch_args(&me3, &profile);
    crate::log::line(
        "launch",
        &format!("{} through me3 {} (offline, no anti-cheat): {} ({} natives)", slot.game.name, ac::ME3_VERSION, args.join(" "), natives.len()),
    );
    if Path::new("/.flatpak-info").exists() {
        match std::process::Command::new(&args[0]).args(&args[1..]).stdin(std::process::Stdio::null()).spawn() {
            Ok(_) => true,
            Err(e) => {
                w.say(format!("Couldn't start me3: {e}"), "error");
                false
            }
        }
    } else {
        // Outside the Flatpak there is no flatpak-spawn: the same program, the same fixed arguments.
        match std::process::Command::new(&args[2]).args(&args[3..]).stdin(std::process::Stdio::null()).spawn() {
            Ok(_) => true,
            Err(e) => {
                w.say(format!("Couldn't start me3: {e}"), "error");
                false
            }
        }
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
                    Err(e) => w.say(format!("Couldn't remove {rel}: {e}"), "error"),
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
                    Err(e) => w.say(format!("Couldn't save the answer: {e}"), "error"),
                }
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
