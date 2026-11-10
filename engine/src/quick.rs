//! 0.6: Turbine from Steam's Quick Access menu (the optional Decky plugin in decky/). The plugin runs Turbine without
//! its window, with one request on the command line, and reads one JSON line back:
//!
//! ```text
//! turbine-mod-manager --status <game> --json        what the plugin shows for a game
//! turbine-mod-manager --mods on|off <game> --json   Mods on / Mods off (exactly the mods that were on come back)
//! turbine-mod-manager --play <game> --json          Play with mods (--anyway after the plugin showed the checks)
//! turbine-mod-manager --play-coop <game> --json     Play Seamless Co-op (Elden Ring, when it is installed)
//! turbine-mod-manager --play-plain <game> --json    Play without mods (the mods stay on for next time)
//! turbine-mod-manager --open <game> --json          Turbine opens on this game
//! turbine-mod-manager --report <game> --json        Turbine opens Report a bug for this game
//! ```
//!
//! `<game>` is Turbine's id ("steam:489830"), a Steam app id ("489830"), the app id of a non-Steam shortcut (a
//! Heroic game added to Steam: its name is matched) or the game's name. This module has the parts that need no
//! window: reading the request, finding the game, the line sent to a Turbine that is already open, the Mods off
//! list, the last launch and the status answer. The app runs the rest with the same code its own buttons use.

use crate::games::Game;
use crate::json::{self, quote, Json};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// What the plugin asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Status,
    ModsOn,
    ModsOff,
    Play,
    PlayCoop,
    PlayPlain,
    Open,
    Report,
}

impl Action {
    /// The word used on Turbine's socket and in the activity log.
    pub fn word(self) -> &'static str {
        match self {
            Action::Status => "status",
            Action::ModsOn => "mods-on",
            Action::ModsOff => "mods-off",
            Action::Play => "play",
            Action::PlayCoop => "play-coop",
            Action::PlayPlain => "play-plain",
            Action::Open => "open",
            Action::Report => "report",
        }
    }

    pub fn from_word(w: &str) -> Option<Action> {
        [
            Action::Status,
            Action::ModsOn,
            Action::ModsOff,
            Action::Play,
            Action::PlayCoop,
            Action::PlayPlain,
            Action::Open,
            Action::Report,
        ]
        .into_iter()
        .find(|a| a.word() == w)
    }

    /// Changes the game's mods or starts it (only one Turbine may do that at a time).
    pub fn changes(self) -> bool {
        !matches!(self, Action::Status | Action::Open | Action::Report)
    }
}

/// One request from the command line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub action: Action,
    pub game: String,
    /// Answer with one JSON line (the plugin); without it, a sentence for a person at a terminal.
    pub json: bool,
    /// Play even though the check before Play has warnings (the plugin showed them and the player chose Play anyway).
    pub anyway: bool,
}

/// The longest game name or id accepted.
const MAX_GAME: usize = 200;

/// Reads the program's arguments (without the program's own name). None: none of these flags, so Turbine opens its
/// window as usual. Some(Err): a request that can't be read, in words.
pub fn parse_args(args: &[String]) -> Option<Result<Request, String>> {
    let flags = [
        ("--status", Action::Status),
        ("--mods", Action::ModsOn),
        ("--play", Action::Play),
        ("--play-coop", Action::PlayCoop),
        ("--play-plain", Action::PlayPlain),
        ("--open", Action::Open),
        ("--report", Action::Report),
    ];
    let mut found: Option<(usize, Action)> = None;
    for (i, a) in args.iter().enumerate() {
        let name = a.split('=').next().unwrap_or("");
        if let Some((_, act)) = flags.iter().find(|(f, _)| *f == name) {
            if found.is_some() {
                return Some(Err("Only one request at a time.".into()));
            }
            found = Some((i, *act));
        }
    }
    let (at, mut action) = found?;
    let json = args.iter().any(|a| a == "--json");
    let anyway = args.iter().any(|a| a == "--anyway");
    // The values after the flag (or after "="), skipping the switches.
    let mut values: Vec<String> = Vec::new();
    if let Some((_, v)) = args[at].split_once('=') {
        values.push(v.to_string());
    }
    values.extend(args[at + 1..].iter().take_while(|a| !a.starts_with("--")).cloned());
    if let Some(g) = args.iter().find_map(|a| a.strip_prefix("--game=")) {
        values.push(g.to_string());
    }
    if action == Action::ModsOn {
        let Some(first) = values.first().cloned() else { return Some(Err("--mods needs on or off, then the game.".into())) };
        action = match first.to_ascii_lowercase().as_str() {
            "on" => Action::ModsOn,
            "off" => Action::ModsOff,
            _ => return Some(Err("--mods needs on or off, then the game.".into())),
        };
        values.remove(0);
    }
    let game = values.join(" ").trim().to_string();
    if game.is_empty() {
        return Some(Err(format!("--{} needs the game: its Steam app id, Turbine's id or its name.", action_flag(action))));
    }
    if game.len() > MAX_GAME || game.chars().any(char::is_control) {
        return Some(Err("That game name can't be read.".into()));
    }
    Some(Ok(Request { action, game, json, anyway }))
}

fn action_flag(a: Action) -> &'static str {
    match a {
        Action::ModsOn | Action::ModsOff => "mods",
        other => other.word(),
    }
}

/// A name compared without case, spaces or punctuation ("ELDEN RING" and "Elden Ring" are the same game).
fn simple(name: &str) -> String {
    name.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

/// Finds the game the plugin means. `shortcuts`: Steam's non-Steam games (app id, name), so a Heroic game added to
/// Steam is found by the app id Steam runs it under.
pub fn find_game<'a>(games: &'a [Game], query: &str, shortcuts: &[(u32, String)]) -> Option<&'a Game> {
    let q = query.trim();
    if q.is_empty() {
        return None;
    }
    if let Some(g) = games.iter().find(|g| g.id() == q) {
        return Some(g);
    }
    if q.bytes().all(|b| b.is_ascii_digit()) {
        if let Some(g) = games.iter().find(|g| g.store == crate::games::Store::Steam && g.store_id == q) {
            return Some(g);
        }
        // A non-Steam shortcut: Steam's app id is the shortcut's, and its name is the game's.
        let id: u64 = q.parse().ok()?;
        let name = shortcuts.iter().find(|(a, _)| u64::from(*a) == id || crate::shortcuts::game_id(*a) == id).map(|(_, n)| n.clone())?;
        return by_name(games, &name);
    }
    by_name(games, q)
}

/// Steam's non-Steam games (app id, name), read only, for `find_game`.
pub fn shortcut_names(home: &Path) -> Vec<(u32, String)> {
    let mut out = Vec::new();
    for f in crate::shortcuts::shortcut_files(home) {
        let Ok(bytes) = fs::read(&f) else { continue };
        if bytes.len() > 16 << 20 {
            continue;
        }
        if let Ok(list) = crate::shortcuts::parse(&bytes) {
            out.extend(list.into_iter().map(|s| (s.appid, s.name)));
        }
    }
    out
}

fn by_name<'a>(games: &'a [Game], name: &str) -> Option<&'a Game> {
    let want = simple(name);
    if want.is_empty() {
        return None;
    }
    games.iter().find(|g| simple(&g.name) == want)
}

// ------------------------------------------------------------------------------------------- the running Turbine

/// What starts every request line on Turbine's socket.
pub const SOCKET_PREFIX: &str = "turbine:quick ";

/// A game id as it goes into a socket line: letters, digits and : . _ - as they are, anything else as %XX.
pub fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b":._-".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

pub fn decode(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = s.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// The line a Turbine that is already open gets for a request (it does it there, with its window).
pub fn socket_line(action: Action, game_id: &str) -> String {
    format!("{SOCKET_PREFIX}{} {}", action.word(), encode(game_id))
}

/// Reads a request line from the socket: (action, game id). Anything else is None (and dropped).
pub fn parse_socket_line(line: &str) -> Option<(Action, String)> {
    let rest = line.trim().strip_prefix(SOCKET_PREFIX)?;
    if rest.len() > 3 * MAX_GAME + 20 || !rest.chars().all(|c| c.is_ascii_graphic() || c == ' ') {
        return None;
    }
    let (word, game) = rest.split_once(' ')?;
    let action = Action::from_word(word).filter(|a| *a != Action::Status)?;
    if game.contains(' ') {
        return None;
    }
    let game = decode(game)?;
    (!game.is_empty() && game.len() <= MAX_GAME && !game.chars().any(char::is_control) && game.contains(':')).then_some((action, game))
}

/// The file a request waits in when Turbine isn't open yet (the plugin then starts Turbine through Steam).
pub const REQUEST_FILE: &str = "quick-request";
/// How long a waiting request counts: Turbine started later than this was opened some other way.
pub const REQUEST_MAX_AGE: u64 = 120;

/// Keeps a request for the next Turbine that starts (in Turbine's data folder).
pub fn write_request(dir: &Path, action: Action, game_id: &str, now: u64) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let tmp = dir.join(format!("{REQUEST_FILE}.tmp"));
    fs::write(&tmp, format!("{now}\n{}\n", socket_line(action, game_id)))?;
    fs::rename(tmp, dir.join(REQUEST_FILE))
}

/// The waiting request's socket line, taken (the file goes either way). None when there is none or it is too old.
pub fn take_request(dir: &Path, now: u64) -> Option<String> {
    let path = dir.join(REQUEST_FILE);
    let text = fs::read_to_string(&path).ok();
    let _ = fs::remove_file(&path);
    let text = text?;
    let mut lines = text.lines();
    let at: u64 = lines.next()?.trim().parse().ok()?;
    let line = lines.next()?.trim().to_string();
    (now.saturating_sub(at) <= REQUEST_MAX_AGE && at <= now + 5 && parse_socket_line(&line).is_some()).then_some(line)
}

// ------------------------------------------------------------------------------------------------ Mods off / on

/// The list Mods off keeps in the game's store folder: the mods that were on.
pub const HELD_FILE: &str = "mods-off.json";

pub fn held_path(store_root: &Path) -> PathBuf {
    store_root.join(HELD_FILE)
}

/// The mods Mods off turned off (empty when nothing is held).
pub fn read_held(store_root: &Path) -> Vec<String> {
    let Ok(text) = fs::read_to_string(held_path(store_root)) else { return Vec::new() };
    let Ok(v) = json::parse(&text) else { return Vec::new() };
    v.get("on").map(Json::as_array).unwrap_or(&[]).iter().filter_map(|x| x.as_str().map(String::from)).collect()
}

pub fn write_held(store_root: &Path, ids: &[String], now: u64) -> io::Result<()> {
    let path = held_path(store_root);
    let tmp = store_root.join(format!("{HELD_FILE}.tmp"));
    let list: Vec<String> = ids.iter().map(|i| quote(i)).collect();
    fs::write(&tmp, format!("{{\"on\":[{}],\"at\":{now}}}\n", list.join(",")))?;
    fs::rename(tmp, path)
}

// ------------------------------------------------------------------------------------------------ the last launch

pub const LAUNCH_FILE: &str = "last-launch.json";

/// How the last Play went, as Turbine saw it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Launch {
    /// Seconds since 1970 when Play was pressed.
    pub at: u64,
    /// "mods", "coop" or "plain".
    pub how: String,
    /// "started" (asked Steam to start it), "running", "closed", "not-started" (not seen running) or "failed".
    pub result: String,
    /// How long it ran, for "closed".
    pub seconds: Option<u64>,
    /// What Turbine said, in words ("" when nothing).
    pub words: String,
}

impl Launch {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"at\":{},\"how\":{},\"result\":{},\"seconds\":{},\"words\":{}}}",
            self.at,
            quote(&self.how),
            quote(&self.result),
            self.seconds.map(|s| s.to_string()).unwrap_or_else(|| "null".into()),
            quote(&self.words)
        )
    }
}

pub fn write_launch(store_root: &Path, launch: &Launch) -> io::Result<()> {
    let tmp = store_root.join(format!("{LAUNCH_FILE}.tmp"));
    fs::write(&tmp, launch.to_json() + "\n")?;
    fs::rename(tmp, store_root.join(LAUNCH_FILE))
}

/// Updates the result of the last launch (the watch after Play). Nothing when there is no launch on record.
pub fn update_launch(store_root: &Path, result: &str, seconds: Option<u64>) -> io::Result<()> {
    let Some(mut l) = read_launch(store_root) else { return Ok(()) };
    l.result = result.to_string();
    l.seconds = seconds;
    write_launch(store_root, &l)
}

pub fn read_launch(store_root: &Path) -> Option<Launch> {
    let text = fs::read_to_string(store_root.join(LAUNCH_FILE)).ok()?;
    let v = json::parse(&text).ok()?;
    let num = |k: &str| match v.get(k) {
        Some(Json::Num(n)) if *n >= 0.0 => Some(*n as u64),
        _ => None,
    };
    Some(Launch {
        at: num("at")?,
        how: v.str_at("how").unwrap_or("mods").to_string(),
        result: v.str_at("result")?.to_string(),
        seconds: num("seconds"),
        words: v.str_at("words").unwrap_or("").to_string(),
    })
}

// ------------------------------------------------------------------------------------------------ the status answer

/// The game's mods as its store's state.json lists them, read without changing anything (Turbine's window may be
/// open and working on them).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Peek {
    pub mods: usize,
    pub on: usize,
    /// Mods off is holding these many (they come back with Mods on).
    pub held: usize,
    /// Seamless Co-op is among the mods (Turbine's own install of it).
    pub seamless: bool,
    pub seamless_on: bool,
    /// 0.62: the highest version among the Seamless Co-op copies in the list, when one says its version.
    pub seamless_version: Option<String>,
}

/// The game's store folder when Turbine has made one (read only: nothing is created, unlike `Store::root_for`).
pub fn existing_root(game: &Game, data_home: &Path) -> Option<PathBuf> {
    let key = crate::store::folder_safe(&game.id().replace(':', "-"));
    [true, false]
        .into_iter()
        .map(|on_library| crate::store::choose_root(&key, &game.library, None, None, on_library, data_home))
        .find(|r| r.join("state.json").is_file())
}

/// Is Seamless Co-op's DLL somewhere in this staged mod folder (a few folders deep at most)?
fn staged_has_seamless(dir: &Path, depth: usize) -> bool {
    let Ok(rd) = fs::read_dir(dir) else { return false };
    let entries: Vec<_> = rd.flatten().collect();
    entries.iter().any(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case("ersc.dll") && e.path().is_file())
        || (depth > 0 && entries.iter().any(|e| e.path().is_dir() && staged_has_seamless(&e.path(), depth - 1)))
}

pub fn peek(store_root: &Path) -> Peek {
    let mut p = Peek::default();
    let Ok(text) = fs::read_to_string(store_root.join("state.json")) else { return p };
    let Ok(v) = json::parse(&text) else { return p };
    let sources = v.get("sources");
    for m in v.get("mods").map(Json::as_array).unwrap_or(&[]) {
        let Some(id) = m.str_at("id") else { continue };
        let on = m.get("enabled").and_then(Json::as_bool).unwrap_or(false);
        p.mods += 1;
        if on {
            p.on += 1;
        }
        let src = sources.and_then(|s| s.get(id));
        // 0.62: Seamless Co-op is known by its files (its ersc.dll in the mod's staged folder), so one from Nexus
        // counts like Turbine's own download.
        let own = src.and_then(|s| s.str_at("kind")) == Some("loader") && src.and_then(|s| s.str_at("id")) == Some("seamless");
        if own || staged_has_seamless(&crate::store::staged_dir(store_root, id), 4) {
            p.seamless = true;
            p.seamless_on |= on;
            if let Some(v) = m.str_at("version").filter(|v| !v.is_empty()) {
                if p.seamless_version.as_deref().is_none_or(|have| crate::updates::compare_versions(v, have) == std::cmp::Ordering::Greater) {
                    p.seamless_version = Some(v.to_string());
                }
            }
        }
    }
    let held = read_held(store_root);
    p.held = held.len();
    p
}

/// What else the status answer says, worked out by the app.
#[derive(Clone, Debug, Default)]
pub struct Extra {
    /// Turbine has a store for this game (mods were added once).
    pub has_store: bool,
    /// The game is running now (None: can't tell).
    pub running: Option<bool>,
    /// Turbine's window is open.
    pub turbine_open: bool,
    /// steam://rungameid for Turbine's own entry in Steam ("" when it isn't in Steam).
    pub turbine_url: String,
    /// The game's anti-cheat mode ("me3", "official", "offline", "refuse", "warn" or "").
    pub anti_cheat: String,
    /// Elden Ring through me3: Seamless Co-op can be played.
    pub coop_game: bool,
    /// The plain sentence for the panel ("" when the numbers say it all).
    pub words: String,
}

/// The one JSON line the plugin reads for a game.
pub fn status_json(game: &Game, p: &Peek, launch: Option<&Launch>, x: &Extra) -> String {
    let refused = x.anti_cheat == "refuse";
    format!(
        "{{\"ok\":true,\"found\":true,\"game\":{{\"id\":{},\"name\":{},\"store\":{},\"storeId\":{}}},\"modded\":{},\"mods\":{},\"on\":{},\"held\":{},\"modsOn\":{},\"coop\":{},\"coopOn\":{},\"refused\":{},\"antiCheat\":{},\"running\":{},\"turbineOpen\":{},\"turbineUrl\":{},\"lastLaunch\":{},\"words\":{}}}",
        quote(&game.id()),
        quote(&game.name),
        quote(game.store.key()),
        quote(&game.store_id),
        x.has_store && p.mods > 0,
        p.mods,
        p.on,
        p.held,
        p.on > 0,
        x.coop_game && p.seamless && !refused,
        p.seamless_on,
        refused,
        quote(&x.anti_cheat),
        x.running.map(|r| r.to_string()).unwrap_or_else(|| "null".into()),
        x.turbine_open,
        quote(&x.turbine_url),
        launch.map(Launch::to_json).unwrap_or_else(|| "null".into()),
        quote(&x.words)
    )
}

/// The answer when the game isn't one Turbine knows.
pub fn not_found_json(query: &str) -> String {
    format!(
        "{{\"ok\":true,\"found\":false,\"query\":{},\"words\":{}}}",
        quote(query),
        quote("Turbine doesn't see this game. It works with installed Steam games and games in Heroic.")
    )
}

/// Any other answer: `ok`, the words for the panel, and more fields (each starting with a comma).
pub fn answer_json(ok: bool, words: &str, more: &str) -> String {
    format!("{{\"ok\":{ok},\"words\":{}{more}}}", quote(words))
}
