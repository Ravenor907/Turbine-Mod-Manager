//! 0.6: Turbine from Steam's Quick Access menu, without its window. The optional Decky plugin (decky/) runs
//! `flatpak run io.github.ravenor907.TurbineModManager --status <game> --json` (and --mods on|off, --play,
//! --play-coop, --play-plain, --open, --report) and reads one JSON line back. The requests and the answer's shape
//! are the engine's (turbine_engine::quick, tested there); this runs them with the same jobs Turbine's own buttons
//! use (backend.rs: check_json, play_job, play_original_job, mods_job), on a worker with no window.
//!
//! One Turbine changes a game's mods at a time: when Turbine's window is open, a request that changes something goes
//! to it over its socket (single.rs) and it does it there, on screen. Status only reads (state.json, the last
//! launch), so it works either way and never creates anything.

use crate::backend::{self, Worker};
use turbine_engine::anticheat::{self as ac, Mode};
use turbine_engine::json::{self, quote, Json};
use turbine_engine::quick::{self as q, Action, Request};
use turbine_engine::rules::GameRules;
use turbine_engine::Game;

/// The request on Turbine's command line, if there is one.
pub fn wanted() -> Option<Result<Request, String>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    q::parse_args(&args)
}

/// Answers the request on standard output (one JSON line with --json, else the words) and gives the exit code.
pub fn run(req: Result<Request, String>) -> i32 {
    let json_out = std::env::args().any(|a| a == "--json");
    let answer = match req {
        Ok(r) => answer(&r),
        Err(e) => q::answer_json(false, &e, ""),
    };
    let parsed = json::parse(&answer).ok();
    let ok = parsed.as_ref().and_then(|v| v.get("ok")).and_then(Json::as_bool).unwrap_or(false);
    // 0.6: the co-op tile in Steam's library (--from-tile): no plugin reads the answer, so Turbine opens the game's
    // link itself, or its own window on the game when something needs the player first.
    if std::env::args().any(|a| a == "--from-tile") {
        from_tile(parsed.as_ref(), ok);
    }
    if json_out {
        println!("{answer}");
    } else {
        let words = parsed.as_ref().and_then(|v| v.str_at("words")).unwrap_or("").to_string();
        println!("{}", if words.is_empty() { answer } else { words });
    }
    if ok {
        0
    } else {
        1
    }
}

/// Opens a steam:// link on the host (Steam passes it to the running Steam).
fn open_steam_link(url: &str) {
    use std::process::Stdio;
    match crate::system::host("steam").arg(url).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn() {
        Ok(_) => crate::log::line("quick", &format!("Co-op tile: opened {url}")),
        Err(e) => crate::log::line("quick", &format!("Co-op tile: couldn't open {url}: {e}")),
    }
}

fn from_tile(answer: Option<&Json>, ok: bool) {
    let launch = answer.and_then(|v| v.str_at("launch")).unwrap_or("");
    if ok && !launch.is_empty() {
        open_steam_link(launch);
        return;
    }
    // Something needs the player (the session password, a check before Play): Turbine opens on the game.
    let data = backend::data_home();
    let folder = crate::log::folder(&data);
    let game = std::env::args().skip_while(|a| a != "--play-coop").nth(1).unwrap_or_default();
    let now = crate::now_secs();
    let sent = answer.and_then(|v| v.get("sent")).and_then(Json::as_bool) == Some(true);
    let waiting = answer.and_then(|v| v.get("waiting")).and_then(Json::as_bool) == Some(true);
    if !sent && !waiting {
        // Turbine's own Play Seamless Co-op on screen: it opens the session password first when it isn't set.
        let _ = q::write_request(&folder, Action::PlayCoop, &game, now);
    }
    let words = answer.and_then(|v| v.str_at("words")).unwrap_or("");
    crate::log::line("quick", &format!("Co-op tile: Turbine opens on {game} first ({words})"));
    if let Some((id, _)) = turbine_engine::shortcuts::find_turbine(&turbine_engine::home_dir()) {
        open_steam_link(&turbine_engine::shortcuts::rungameid_url(id));
    }
}

fn answer(r: &Request) -> String {
    let data = backend::data_home();
    let folder = crate::log::folder(&data);
    crate::log::start_quiet(folder.clone());
    crate::log::line("quick", &format!("Steam's menu asks: {} {}", r.action.word(), r.game));
    turbine_engine::rules::load_player_games(&crate::anygame::player_games_dir());
    let home = turbine_engine::home_dir();
    let scan = turbine_engine::scan_games(&home);
    let shortcuts = q::shortcut_names(&home);
    let Some(game) = q::find_game(&scan.games, &r.game, &shortcuts).cloned() else {
        crate::log::line("quick", &format!("Steam's menu: no game matches \"{}\"", r.game));
        return q::not_found_json(&r.game);
    };
    // Held from before the main lock until after it: a Turbine started meanwhile waits for this request instead of
    // closing as a second copy (browse.rs, QUICK_MARK).
    let quick_mark = crate::browse::lock_instance(&folder.join(crate::browse::QUICK_MARK));
    // Turbine's window holds this lock while it is open (browse.rs).
    let lock = crate::browse::lock_instance(&folder.join(crate::browse::LOCK_FILE));
    // 0.61 (break L3): both taken by someone else is another request from the menu, not Turbine's window.
    let busy = crate::browse::another_request_holds(&quick_mark, &lock);
    let open = matches!(lock, crate::browse::Lock::Taken) && !busy;
    if busy && r.action != Action::Status {
        drop(lock);
        crate::log::line("quick", "Steam's menu: another request from the menu is still running");
        return q::answer_json(false, "Turbine is busy with another request from this menu. Try again in a moment.", "");
    }
    let turbine_url = turbine_engine::shortcuts::find_turbine(&home).map(|(id, _)| turbine_engine::shortcuts::rungameid_url(id)).unwrap_or_default();
    let out = match r.action {
        Action::Status => {
            drop(lock);
            status(&game, &data, open, &turbine_url)
        }
        Action::Open | Action::Report => {
            drop(lock);
            open_turbine(r.action, &game, open, &folder, &turbine_url)
        }
        _ if open => forward(r.action, &game, &turbine_url),
        _ => {
            if !lock.mine() {
                crate::log::line("quick", "Steam's menu: Turbine's lock couldn't be taken; going on (nothing else holds it)");
            }
            let out = change(r, &game, &data);
            drop(lock);
            out
        }
    };
    crate::log::line("quick", &format!("Steam's menu answer: {out}"));
    out
}

/// The anti-cheat mode word for a game ("" when it has none).
fn ac_word(rules: &GameRules) -> (String, bool) {
    let Some(a) = rules.anti_cheat.as_ref() else { return (String::new(), false) };
    let m = ac::mode(a);
    (crate::anticheat::mode_word(m).to_string(), m == Mode::Me3 && a.me3_game() == Some("eldenring"))
}

fn status(game: &Game, data: &std::path::Path, open: bool, turbine_url: &str) -> String {
    let root = q::existing_root(game, data);
    let peek = root.as_deref().map(q::peek).unwrap_or_default();
    let launch = root.as_deref().and_then(q::read_launch);
    let rules = GameRules::for_game(game);
    let (anti_cheat, coop_game) = ac_word(&rules);
    let words = if anti_cheat == "refuse" {
        ac::REFUSE_WORDS.to_string()
    } else if peek.on == 0 && peek.held > 0 {
        format!("Mods off. Mods on brings back {}.", if peek.held == 1 { "the 1 mod that was on".to_string() } else { format!("the {} mods that were on", peek.held) })
    } else if peek.mods == 0 {
        "No mods for this game yet. Open Turbine to add some.".to_string()
    } else {
        String::new()
    };
    let x = q::Extra {
        has_store: root.is_some(),
        running: crate::system::game_running(&game.install_dir),
        turbine_open: open,
        turbine_url: turbine_url.to_string(),
        anti_cheat,
        coop_game,
        words,
    };
    q::status_json(game, &peek, launch.as_ref(), &x)
}

/// Open and Report: Turbine opens on this game. Open now: it gets the request at once; not open: the request waits
/// for it, and the plugin starts Turbine through Steam (its entry there, `turbineUrl`).
fn open_turbine(action: Action, game: &Game, open: bool, folder: &std::path::Path, turbine_url: &str) -> String {
    let line = q::socket_line(action, &game.id());
    if open {
        return match crate::single::send_request(&line) {
            Ok(()) => q::answer_json(true, "", &format!(",\"sent\":true,\"turbineUrl\":{}", quote(turbine_url))),
            Err(e) => q::answer_json(false, &format!("Turbine is open but didn't answer ({e}). Switch to it with the Steam button."), &format!(",\"turbineUrl\":{}", quote(turbine_url))),
        };
    }
    let now = crate::now_secs();
    if let Err(e) = q::write_request(folder, action, &game.id(), now) {
        return q::answer_json(false, &format!("Couldn't ask Turbine to open {}: {e}", game.name), "");
    }
    let words = if turbine_url.is_empty() {
        "Turbine isn't in your Steam library yet. In Desktop Mode, add it to Steam (Add a Non-Steam Game), then try again.".to_string()
    } else {
        String::new()
    };
    q::answer_json(!turbine_url.is_empty(), &words, &format!(",\"sent\":false,\"turbineUrl\":{}", quote(turbine_url)))
}

/// Turbine's window is open: it does the change there, on screen.
fn forward(action: Action, game: &Game, turbine_url: &str) -> String {
    match crate::single::send_request(&q::socket_line(action, &game.id())) {
        Ok(()) => q::answer_json(
            true,
            "Turbine is open, so it does this there. If it asks something, switch to Turbine.",
            &format!(",\"sent\":true,\"turbineUrl\":{}", quote(turbine_url)),
        ),
        Err(e) => q::answer_json(false, &format!("Turbine is open but didn't answer ({e}). Switch to it with the Steam button."), &format!(",\"turbineUrl\":{}", quote(turbine_url))),
    }
}

/// The last thing Turbine said that wasn't "working on it": (words, level).
fn last_said(w: &Worker) -> (String, &'static str) {
    let Some(h) = &w.headless else { return (String::new(), "ok") };
    let said = h.said.lock().unwrap_or_else(|e| e.into_inner());
    said.iter().rev().find(|(_, l)| *l != "work").map(|(t, l)| (t.clone(), *l)).unwrap_or((String::new(), "ok"))
}

fn launched(w: &Worker) -> Option<String> {
    w.headless.as_ref().and_then(|h| h.launch.lock().unwrap_or_else(|e| e.into_inner()).clone())
}

/// Mods on or off, or a Play: the same jobs as Turbine's buttons, here with no window.
fn change(r: &Request, game: &Game, data: &std::path::Path) -> String {
    let mut w = Worker::headless(data.to_path_buf());
    match r.action {
        Action::ModsOn | Action::ModsOff => {
            backend::mods_job(game.clone(), r.action == Action::ModsOn)(&mut w);
            let (words, level) = last_said(&w);
            q::answer_json(level != "error" && level != "warn", &words, "")
        }
        Action::Play | Action::PlayCoop | Action::PlayPlain => play(r, game, &mut w),
        _ => q::answer_json(false, "That isn't something Steam's menu can ask.", ""),
    }
}

fn play(r: &Request, game: &Game, w: &mut Worker) -> String {
    if crate::system::game_running(&game.install_dir) == Some(true) {
        return q::answer_json(false, &format!("{} is already running.", game.name), "");
    }
    let mut how = match r.action {
        Action::PlayCoop => "coop",
        Action::PlayPlain => "plain",
        _ => "mods",
    };
    if how != "plain" {
        let slot = match w.slot(game) {
            Ok(s) => s,
            Err(e) => return q::answer_json(false, &e, ""),
        };
        let any_on = slot.store.mods().iter().any(|m| m.enabled);
        // Online-only anti-cheat: never with a mod on (Turbine's own Play says the same).
        if slot.rules.anti_cheat.as_ref().is_some_and(|a| ac::mode(a) == Mode::Refuse) && any_on {
            return q::answer_json(false, ac::REFUSE_WORDS, "");
        }
        // Held after a game update: without mods until they update, as Turbine's own Play does.
        if any_on && crate::anticheat::hold_file(slot.store.root()).exists() {
            crate::log::line("quick", &format!("{}: mods are held until they update, so Play from Steam's menu is without mods", game.name));
            how = "plain";
        }
        // Seamless Co-op: turned on first when it is off (Turbine's own Play Seamless Co-op does the same).
        if how == "coop" {
            // 0.62: Seamless Co-op from anywhere (known by its files), the copy the game loads.
            let id = slot.store.seamless().map(|m| m.id);
            let Some(id) = id else {
                return q::answer_json(false, "Seamless Co-op isn't installed. Open Turbine to get it (game menu).", ",\"needsTurbine\":true");
            };
            if !slot.store.mods().iter().any(|m| m.id == id && m.enabled) {
                if let Err(e) = slot.store.set_enabled(&id, true) {
                    return q::answer_json(false, &format!("Couldn't turn Seamless Co-op on: {e}"), "");
                }
            }
            // The session password: every co-op player needs the same one, so an empty one needs the player first.
            // (0.62: in Turbine's copy and in the copy the mod loads; the Play job checks that copy again.)
            if crate::modsettings::coop_password(&slot.game, slot.store.root(), &id).is_some_and(|p| !p.set()) {
                crate::log::line("quick", &format!("{}: Seamless Co-op's session password isn't set: Turbine opens its settings first", game.name));
                // 0.62: the co-op start waits for Turbine's window, which opens Co-op settings on the password (the
                // plugin and the tile start Turbine through Steam with `turbineUrl`).
                let folder = crate::log::folder(&backend::data_home());
                let now = crate::now_secs();
                let waiting = q::write_request(&folder, Action::PlayCoop, &game.id(), now).is_ok();
                let url = turbine_engine::shortcuts::find_turbine(&turbine_engine::home_dir()).map(|(id, _)| turbine_engine::shortcuts::rungameid_url(id)).unwrap_or_default();
                return q::answer_json(false, crate::modsettings::NEEDS_PASSWORD, &format!(",\"needsTurbine\":true,\"password\":false,\"waiting\":{waiting},\"turbineUrl\":{}", quote(&url)));
            }
        }
    }
    // The check before Play: anything that stops it, or warnings the player hasn't said yes to yet.
    if how != "plain" {
        let list = json::parse(&backend::check_json(w, game)).unwrap_or(Json::Arr(Vec::new()));
        let shown: Vec<&Json> = list
            .as_array()
            .iter()
            .filter(|c| c.get("once").and_then(Json::as_bool) != Some(true))
            .filter(|c| matches!(c.str_at("level"), Some("stop") | Some("warn")))
            .collect();
        let stops = shown.iter().any(|c| c.str_at("level") == Some("stop"));
        if stops || (!shown.is_empty() && !r.anyway) {
            let items: Vec<String> = shown
                .iter()
                .map(|c| format!("{{\"level\":{},\"title\":{}}}", quote(c.str_at("level").unwrap_or("")), quote(c.str_at("title").unwrap_or(""))))
                .collect();
            let first = shown.iter().find(|c| !stops || c.str_at("level") == Some("stop")).and_then(|c| c.str_at("title")).unwrap_or("");
            let words = if stops { format!("{first}. Open Turbine to sort it out.") } else { "Before you play:".to_string() };
            crate::log::line("quick", &format!("{}: Play from Steam's menu waits for the check ({} lines)", game.name, items.len()));
            return q::answer_json(false, &words, &format!(",\"checks\":[{}],\"ask\":{},\"needsTurbine\":{stops}", items.join(","), !stops));
        }
    }
    let job = match how {
        "plain" => backend::play_original_job(game.clone(), 0),
        "coop" => backend::play_job(game.clone(), ac::Coop::Seamless, false, 0),
        _ => backend::play_job(game.clone(), ac::Coop::Solo, false, 0),
    };
    job(w);
    let (words, level) = last_said(w);
    match launched(w) {
        Some(url) => q::answer_json(true, &words, &format!(",\"launch\":{},\"how\":{}", quote(&url), quote(how))),
        None => {
            let words = if words.is_empty() || level == "ok" { format!("Turbine needs one more step for {} first. Open Turbine on this game.", game.name) } else { words };
            q::answer_json(false, &words, ",\"needsTurbine\":true")
        }
    }
}
