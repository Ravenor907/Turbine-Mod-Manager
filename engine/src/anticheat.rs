//! Games with anti-cheat. What the game's data file says (`rules::AntiCheat`) decides the way Turbine plays
//! it with mods:
//!
//! - **me3** (Elden Ring, Nightreign, Armored Core VI, Dark Souls III, Sekiro): me3 starts the game's own program
//!   without the anti-cheat starter, offline, with mods from a Turbine-owned me3 profile folder. Nothing is ever
//!   placed in the game folder: the store deploys into the profile's package folder instead (`profile_mods`). me3
//!   is downloaded from its official GitHub release, pinned to one version and checked by SHA-256 (`ME3_*`),
//!   unpacked by this module (gzip and tar, names checked, only the files it needs) into Turbine's own data folder,
//!   and started on the host with fixed arguments (`launch_args`). The profile uses its own save file, so a
//!   modded save never meets the online game. Seamless Co-op loads through me3 as a native from the same profile
//!   folder (its documented way with me3), the newest release (loaders::fetch_seamless) or the player's own download; with it on, its own .co2 saves
//!   are used and kept.
//! - **official** (Halo MCC): the game has its own switch; Turbine shows which Play choice to pick.
//! - **offline**: no anti-cheat program, but online play flags changed files; Turbine says how to start offline.
//! - **refuse** (online-only games): Turbine adds no mods.
//! - **warn** (the default, and any game where Turbine finds anti-cheat by itself): a warning before Play.
//!
//! Before the normal (anti-cheat) launch, the game folder must match its original record exactly: `state` compares
//! the anti-cheat starter and the game's program with the SHA-256 recorded the first time Turbine saw them, and says
//! whether Turbine still has files in the game folder. Nothing on Nexus is automated here.

use crate::gamedata::ME3_GAMES;
use crate::inflate;
use crate::json::{self, quote, Json};
use crate::rules::{AntiCheat, Check};
use crate::sha256;
use crate::zip;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// me3, pinned: the Linux release from github.com/garyttierney/me3 (v0.13.0, 2026-08-11). The SHA-256 is the one
/// GitHub publishes for the asset, checked against the file itself.
pub const ME3_VERSION: &str = "0.13.0";
pub const ME3_URL: &str = "https://github.com/garyttierney/me3/releases/download/v0.13.0/me3-linux-amd64.tar.gz";
pub const ME3_SIZE: u64 = 10_140_960;
pub const ME3_SHA256: &str = "f31579deebf75b4702059694b664bdd8917c4773ab0c266715ca8f92e1e93fc1";
/// The files Turbine takes from the release (everything else in it is left out).
pub const ME3_FILES: &[&str] = &["bin/me3", "bin/win64/me3-launcher.exe", "bin/win64/me3_mod_host.dll", "bin/win64/me3.exe"];

/// Seamless Co-op for Elden Ring. No version is pinned: the mod refuses to run when it is out of date,
/// so a pinned release can never be right for long: Turbine gets the newest release its repository lists
/// (loaders::fetch_seamless, size and SHA-256 recorded at the download) or the player gets it from Nexus.
/// On Nexus: the game's domain and the mod's number (its page usually has the newest version first).
pub const SEAMLESS_NEXUS: (&str, u64) = ("eldenring", 510);

/// The save file me3 uses instead of the game's own, so a modded save is never the online one.
pub const ME3_SAVEFILE: &str = "TurbineModded.sl2";

/// How Turbine plays a game with anti-cheat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Warn,
    Me3,
    Official,
    Offline,
    Refuse,
}

pub fn mode(ac: &AntiCheat) -> Mode {
    match ac.mode.as_str() {
        "me3" if ac.me3.as_deref().is_some_and(|g| ME3_GAMES.contains(&g)) => Mode::Me3,
        "official" => Mode::Official,
        "offline" => Mode::Offline,
        "refuse" => Mode::Refuse,
        _ if ac.level == "refuse" => Mode::Refuse,
        _ => Mode::Warn,
    }
}

// ---- Words (the app shows them as they are) ----

/// The ask-once question before the first modded Play of an anti-cheat game started by me3.
pub fn ask_title(game: &str) -> String {
    format!("Play {game} with mods?")
}

pub fn ask_text(game: &str, ac: &AntiCheat) -> String {
    let offline = ac.offline_setting.as_deref().map(|s| format!("\n• Set the game to start offline too: {s}.")).unwrap_or_default();
    // Elden Ring: Seamless Co-op (game menu) brings its own co-op and keeps its own .co2 saves.
    let coop = ac.me3.as_deref() == Some("eldenring");
    let play = if coop {
        "It plays offline only: no normal co-op, invasions or messages. Seamless Co-op (in the Game menu) adds its own co-op."
    } else {
        "It plays offline only: no co-op, invasions or messages."
    };
    let save = if coop { "uses a separate save while mods are on (Seamless Co-op keeps its own .co2 saves)" } else { "uses a separate save while mods are on" };
    format!(
        "{game}'s anti-cheat stops mods from loading. To play with mods, Turbine starts the game without it, through Mod Engine 3.\
         \n• {play}\
         \n• Playing online with changed game files can get your account banned, so Turbine never does that.\
         \n• Turbine backs up your saves first, {save}, and never changes the game's own files.\
         \n• Play without mods starts the game the normal way, with its anti-cheat.{offline}\
         \n• To change your answer: VIEW, then Game menu."
    )
}

/// The ask needs an answer only where there is an anti-cheat or an online ban to turn away from (not Sekiro).
pub fn needs_ask(ac: &AntiCheat) -> bool {
    mode(ac) == Mode::Me3 && (ac.starter.is_some() || ac.offline_setting.is_some())
}

pub const STATUS_ROW: &str = "Anti-cheat: off while mods are on (offline only)";
pub const REFUSE_WORDS: &str =
    "This game needs its anti-cheat to play at all. Changing its files can get your account banned, so Turbine doesn't add mods to this game.";
pub const VERIFY_WORDS: &str = "To repair it, use Steam's Verify integrity of game files: game page, gear, Properties, Installed Files.";
/// Whose record it is: Turbine's own, from the first time it saw the game, not Steam's list of the game's files.
pub const RECORD_WORDS: &str = "Turbine compares them with its own record from the first time it saw the game (not Steam's list of the game's files).";

/// Only a game me3 starts has its folder checked against the record: Halo's mods live in its folder by design
/// (the player picks Steam's Play choice without anti-cheat), and an offline game has no anti-cheat program.
pub fn folder_checked(ac: &AntiCheat) -> bool {
    mode(ac) == Mode::Me3
}

pub fn official_words(game: &str, ac: &AntiCheat) -> String {
    let choice = ac.official_choice.as_deref().unwrap_or("the choice without anti-cheat");
    format!("{game} has its own switch. When you press Play, Steam asks how to start it: pick \"{choice}\". Matchmaking is off that way; custom games still work.")
}

pub fn offline_words(ac: &AntiCheat) -> Option<String> {
    ac.offline_setting.as_deref().map(|s| format!("Set {s} before playing with mods."))
}

/// The steps of the Halo guide, for Game details and the Help sheet.
pub fn official_guide(game: &str, ac: &AntiCheat) -> Vec<String> {
    vec![
        "Turn on the mods you want in Turbine. They go into the game folder only while they are on.".into(),
        format!("Press Play. Steam asks how to start {game}."),
        format!("Pick \"{}\".", ac.official_choice.as_deref().unwrap_or("Anti-Cheat Disabled")),
        "Play offline or in custom games. Matchmaking stays off without anti-cheat.".into(),
        "To play online again, turn the mods off (or Play without mods) and pick the normal option when Steam asks.".into(),
    ]
}

// ---- The original record of the game folder ----

/// The SHA-256 of the anti-cheat starter and the game's program, as first seen, with the game build then.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Original {
    pub build: Option<String>,
    /// (game-relative path, SHA-256)
    pub files: Vec<(String, String)>,
}

impl Original {
    pub fn load(path: &Path) -> Option<Original> {
        let v = json::parse(&fs::read_to_string(path).ok()?).ok()?;
        let files = v
            .get("files")
            .map(Json::as_array)
            .unwrap_or(&[])
            .iter()
            .filter_map(|f| Some((f.str_at("rel")?.to_string(), f.str_at("sha256")?.to_string())))
            .collect();
        Some(Original { build: v.str_at("build").map(str::to_string), files })
    }
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let files: Vec<String> = self.files.iter().map(|(r, h)| format!("{{\"rel\":{},\"sha256\":{}}}", quote(r), quote(h))).collect();
        let text = format!(
            "{{\"build\":{},\"files\":[{}]}}\n",
            self.build.as_deref().map(quote).unwrap_or_else(|| "null".into()),
            files.join(",")
        );
        if let Some(d) = path.parent() {
            fs::create_dir_all(d)?;
        }
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, text)?;
        fs::rename(tmp, path)
    }
}

/// The files the record covers: the starter and the game's program, where the data file names them.
pub fn watched(ac: &AntiCheat) -> Vec<String> {
    ac.starter.iter().chain(ac.game_exe.iter()).cloned().collect()
}

/// Reads the watched files now. Missing files are left out.
pub fn record_now(game_dir: &Path, ac: &AntiCheat, build: Option<&str>) -> io::Result<Original> {
    let mut files = Vec::new();
    for rel in watched(ac) {
        let p = game_dir.join(&rel);
        if p.is_file() {
            files.push((rel, sha256::file_hex(&p)?));
        }
    }
    Ok(Original { build: build.map(str::to_string), files })
}

/// Where the game folder stands for the normal launch with its anti-cheat.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AcState {
    /// The starter and the program are as recorded and Turbine has nothing in the game folder.
    Original,
    /// Nothing recorded yet (first sight): record it now.
    Unrecorded,
    /// The game updated (another build) and its files changed with it: the record is renewed.
    Updated,
    /// The same build, but the starter or the program changed or went missing: something outside Turbine
    /// replaced them (a swap by another tool; Steam's verify puts them back).
    ChangedOutside(Vec<String>),
    /// The starter is as recorded, but another tool left its backup copy beside it ("<starter>.backup"). Steam's
    /// verify doesn't remove extra files; Turbine offers to remove only that file, after a hold to confirm.
    LeftoverBackup(String),
    /// Turbine still has files in the game folder (a restore that didn't finish, or files a deploy put there).
    TurbineFilesLeft(usize),
}

/// Compares the game folder with its record. `ours_in_game`: how many files Turbine's game-folder record still
/// lists.
pub fn state(game_dir: &Path, ac: &AntiCheat, record: Option<&Original>, build: Option<&str>, ours_in_game: usize) -> AcState {
    if ours_in_game > 0 {
        return AcState::TurbineFilesLeft(ours_in_game);
    }
    let Some(rec) = record else { return AcState::Unrecorded };
    let mut changed = Vec::new();
    for rel in watched(ac) {
        let now = sha256::file_hex(&game_dir.join(&rel)).ok();
        let then = rec.files.iter().find(|(r, _)| *r == rel).map(|(_, h)| h.as_str());
        match (now.as_deref(), then) {
            (Some(n), Some(t)) if sha256::same_digest(n, t) => {}
            (None, None) => {}
            _ => changed.push(rel),
        }
    }
    let leftover = ac.starter.as_ref().map(|st| format!("{st}.backup")).filter(|b| game_dir.join(b).is_file());
    if changed.is_empty() {
        // Another tool's backup beside a starter that is as recorded: not a swap now, but a leftover to remove.
        return match leftover {
            Some(b) => AcState::LeftoverBackup(b),
            None => AcState::Original,
        };
    }
    if build.is_some() && rec.build.as_deref() != build {
        return AcState::Updated;
    }
    AcState::ChangedOutside(changed)
}

/// The line for Before you play about the game folder, when it isn't ready for the normal launch.
pub fn state_check(game: &str, s: &AcState) -> Option<Check> {
    match s {
        AcState::Original | AcState::Unrecorded => None,
        AcState::Updated => Some(Check {
            level: "warn".into(),
            title: format!("{game} updated"),
            detail: format!(
                "Turbine renews its record of the anti-cheat files now. {RECORD_WORDS} To be sure they are Steam's own, run Steam's Verify integrity of game files once: game page, gear, Properties, Installed Files."
            ),
        }),
        AcState::ChangedOutside(files) => Some(Check {
            level: "stop".into(),
            title: "The anti-cheat files changed".into(),
            detail: format!(
                "{} isn't as Turbine recorded it ({}). {RECORD_WORDS} Starting it online like this can get your account banned. {VERIFY_WORDS}",
                game,
                files.join(", ")
            ),
        }),
        AcState::LeftoverBackup(file) => Some(Check {
            level: "warn".into(),
            title: "Another tool's file is in the game folder".into(),
            detail: format!(
                "{file} was left by another program that used to start {game} without its anti-cheat. The anti-cheat starter itself is fine. Steam's Verify doesn't remove extra files, so to take out only that one: Game menu, Remove the leftover file (hold A)."
            ),
        }),
        AcState::TurbineFilesLeft(n) => Some(Check {
            level: "stop".into(),
            title: "Mod files are still in the game folder".into(),
            detail: format!(
                "Turbine couldn't take {n} {} out of {game}'s folder, so it won't start the game with its anti-cheat. Use Game menu, Reset game files, or Steam's Verify integrity of game files (game page, gear, Properties, Installed Files).",
                if *n == 1 { "file" } else { "files" }
            ),
        }),
    }
}

// ---- The me3 profile ----

/// The Turbine-owned me3 profile folder for a game (inside the game's mod store).
pub fn profile_dir(store_root: &Path) -> PathBuf {
    store_root.join("me3-profile")
}

/// Its package folder: where the store deploys the mods instead of the game folder.
pub fn profile_mods(store_root: &Path) -> PathBuf {
    profile_dir(store_root).join("mods")
}

pub fn profile_file(store_root: &Path) -> PathBuf {
    profile_dir(store_root).join("turbine.me3")
}

/// A mod path as it goes into the package folder: me3's packages stand for the game's data folder, so a leading
/// "Game/" (where Elden Ring keeps it) comes off.
pub fn package_rel(rel: &str) -> String {
    let r = rel.replace('\\', "/");
    match r.split_once('/') {
        Some((first, rest)) if first.eq_ignore_ascii_case("game") && !rest.is_empty() => rest.to_string(),
        _ => r,
    }
}

/// Every DLL in the package folder, as paths relative to the profile folder, in a fixed order (natives).
pub fn natives(mods: &Path) -> Vec<String> {
    fn walk(dir: &Path, rel: &str, out: &mut Vec<String>, depth: usize) {
        if depth > 8 {
            return;
        }
        let Ok(rd) = fs::read_dir(dir) else { return };
        let mut entries: Vec<_> = rd.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let name = e.file_name().to_string_lossy().to_string();
            let r = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_dir() {
                walk(&e.path(), &r, out, depth + 1);
            } else if name.to_ascii_lowercase().ends_with(".dll") {
                // A name me3's profile can't hold as written (a quote, a line break or another control character)
                // is left out rather than written wrongly.
                if !profile_safe(&r) {
                    continue;
                }
                out.push(format!("mods/{r}"));
            }
        }
    }
    let mut out = Vec::new();
    walk(mods, "", &mut out, 0);
    out
}

/// A path that goes into the profile exactly as it is: no quotes, backslashes or control characters.
pub fn profile_safe(rel: &str) -> bool {
    !rel.is_empty() && !rel.chars().any(|c| c.is_control() || c == '\'' || c == '"' || c == '\\')
}

fn toml_str(s: &str) -> String {
    // Single quotes (a literal string): no escapes, so a path stays as it is. Paths never hold a quote here.
    format!("'{}'", s.replace('\'', ""))
}

/// The profile text. `savefile`: the save me3 uses instead of the game's own (None while Seamless Co-op is on, which
/// keeps its own .co2 saves).
pub fn profile_text(game: &str, natives: &[String], savefile: Option<&str>) -> String {
    let mut t = String::from("# Written by Turbine Mod Manager. Turbine rewrites this file before each Play with mods.\nprofileVersion = \"v1\"\n");
    if let Some(s) = savefile {
        t.push_str(&format!("savefile = {}\n", toml_str(s)));
    }
    // me3 leaves the games' code protection (Arxan) running unless the profile says otherwise, and its timed
    // check then closes a modded game without a word after 20 to 30 seconds (the owner's Elden Ring 1.17.2). The
    // key is a top-level one of me3's v1 profile, so it stays above the first table. me3 forces it for Dark Souls
    // III anyway; writing it for every game keeps one profile shape.
    t.push_str("disable_arxan = true\n");
    t.push_str(&format!("\n[[supports]]\ngame = \"{game}\"\n\n[[packages]]\nid = \"turbine-mods\"\npath = 'mods'\n"));
    for n in natives.iter().filter(|n| profile_safe(n)) {
        t.push_str(&format!("\n[[natives]]\npath = {}\n", toml_str(n)));
    }
    t
}

// ---- me3 itself ----

/// Where this version of me3 lives in Turbine's data folder.
pub fn me3_dir(data: &Path) -> PathBuf {
    data.join("me3").join(ME3_VERSION)
}

/// The program on the host, with fixed arguments. Turbine runs these through `flatpak-spawn --host`.
pub fn launch_args(me3: &Path, profile: &Path) -> Vec<String> {
    vec![
        "flatpak-spawn".into(),
        "--host".into(),
        me3.join("bin/me3").to_string_lossy().into(),
        "--windows-binaries-dir".into(),
        me3.join("bin/win64").to_string_lossy().into(),
        "launch".into(),
        "-p".into(),
        profile.to_string_lossy().into(),
    ]
}

// ---- Starting through Steam ----
//
// In Game Mode a game only shows when Steam itself started it. So a modded launch goes through Steam too: the player
// sets the game's Launch Options once to `<helper> %command%` (the launch option doctor shows and copies the exact
// text; Turbine never writes Steam's settings), and Turbine starts the game with steam://rungameid/<app id>. The
// helper is a small script Turbine writes into its own data folder. When Turbine has mods on for the game (a
// "<app id>.me3" file next to it, one argument per line), it starts the game through me3 with Turbine's profile.
// Otherwise (no file, or "<app id>.plain" for one Play without mods) it runs Steam's own command unchanged.
// Steam's command stays whole in its own order (launch wrapper, reaper with the app id, runtime, Proton); me3's
// Windows build takes the place of the game's program under Steam's own Proton (`helper_args_for`). Without the
// reaper and the app id in front, the game starts the normal way. Every step goes to steam-launch.log next to it.

/// The helper's folder in Turbine's data folder.
pub fn launch_dir(data: &Path) -> PathBuf {
    data.join("steam-launch")
}

/// The helper itself.
pub fn helper_path(data: &Path) -> PathBuf {
    launch_dir(data).join("turbine-launch.sh")
}

/// "With mods" for one game: me3's arguments, one per line.
pub fn armed_file(data: &Path, app_id: &str) -> PathBuf {
    launch_dir(data).join(format!("{app_id}.me3"))
}

/// One Play without mods: the helper runs Steam's own command once and removes this.
pub fn plain_file(data: &Path, app_id: &str) -> PathBuf {
    launch_dir(data).join(format!("{app_id}.plain"))
}

/// The helper's own log of every launch.
pub fn helper_log(data: &Path) -> PathBuf {
    launch_dir(data).join("steam-launch.log")
}

/// The game's Launch Options as they were before Turbine's line (to paste back when undoing it).
pub fn original_file(data: &Path, app_id: &str) -> PathBuf {
    launch_dir(data).join(format!("{app_id}.original"))
}

/// The next Play only (Seamless Co-op): used once by the helper, then the normal file again.
pub fn once_file(data: &Path, app_id: &str) -> PathBuf {
    launch_dir(data).join(format!("{app_id}.me3.once"))
}

/// The exact text for the game's Steam Launch Options. Steam runs Launch Options through its shell, so the line
/// itself falls back to Steam's own command when the helper is gone (Turbine's data deleted): the game always starts.
pub fn launch_option(data: &Path) -> String {
    let p = helper_path(data).to_string_lossy().replace('"', "");
    format!("if [ -x \"{p}\" ]; then \"{p}\" %command%; else %command%; fi")
}

/// Do the Launch Options as set in Steam run the helper with Steam's command after it, in the line that falls back
/// to Steam's own command without the helper? A line with the helper alone (no fallback)
/// counts as not set, so Play shows the one-time step again with the line that falls back. In front of the line
/// only `export NAME=value;` statements may stand (`export PROTON_LOG=1; if ...`): `NAME=value if ...` is a shell
/// syntax error, so Steam would start nothing (`launch_option_fix` offers the export form).
pub fn launch_option_ok(set: &str, data: &Path) -> bool {
    let p = helper_path(data).to_string_lossy().to_string();
    let Some(i) = set.find(&p) else { return false };
    let compact: String = set.split_whitespace().collect::<Vec<_>>().join(" ");
    let Some(head) = line_head(&compact) else { return false };
    exports_only(head)
        && set[i + p.len()..].contains("%command%")
        && compact.contains("-x")
        && compact.contains("else %command%")
        && compact.trim_end().ends_with("fi")
}

/// The text in front of the line's `if [`, or None without one.
fn line_head(compact: &str) -> Option<&str> {
    let at = compact.match_indices("if [").map(|(n, _)| n).find(|&n| n == 0 || matches!(compact.as_bytes()[n - 1], b' ' | b';'))?;
    Some(&compact[..at])
}

fn is_name(n: &str) -> bool {
    let mut c = n.chars();
    c.next().is_some_and(|f| f.is_ascii_alphabetic() || f == '_') && c.all(|x| x.is_ascii_alphanumeric() || x == '_')
}

/// `NAME=value` words (no spaces inside a value).
fn assignments(words: &str) -> bool {
    let w: Vec<&str> = words.split_whitespace().collect();
    !w.is_empty() && w.iter().all(|a| a.split_once('=').is_some_and(|(n, v)| is_name(n) && !v.contains(';')))
}

/// Empty, or only `export NAME=value ...;` statements, each ended by `;`.
fn exports_only(head: &str) -> bool {
    let head = head.trim();
    if head.is_empty() {
        return true;
    }
    let Some(body) = head.strip_suffix(';') else { return false };
    body.split(';').all(|st| st.trim().strip_prefix("export ").is_some_and(assignments))
}

/// The line to offer when the Launch Options hold Turbine's line with `NAME=value` in front without `export` (a
/// syntax error): the same settings as `export NAME=value; ` in front of Turbine's line. None otherwise.
pub fn launch_option_fix(set: &str, data: &Path) -> Option<String> {
    if !launch_option_uses_helper(set, data) || launch_option_ok(set, data) {
        return None;
    }
    let compact: String = set.split_whitespace().collect::<Vec<_>>().join(" ");
    let head = line_head(&compact)?.trim();
    let bare = head.strip_prefix("export ").unwrap_or(head).trim_end_matches(';').trim();
    if !assignments(bare) {
        return None;
    }
    Some(format!("export {bare}; {}", launch_option(data)))
}

/// Undo the launch options change: until the player sets the line again (the Launch Options text then differs from
/// what it was at Undo), the helper starts nothing for the game.
pub fn undone_file(data: &Path, app_id: &str) -> PathBuf {
    launch_dir(data).join(format!("{app_id}.undone"))
}

/// Marks the game undone, with the Launch Options as they were then.
pub fn mark_undone(data: &Path, app_id: &str, set_now: &str) -> io::Result<()> {
    check_app_id(app_id)?;
    fs::create_dir_all(launch_dir(data))?;
    remove_if(&once_file(data, app_id))?;
    remove_if(&armed_file(data, app_id))?;
    fs::write(undone_file(data, app_id), set_now)
}

/// Is the game still undone? Not once the Launch Options were set again (changed since Undo, and set right); that
/// clears the mark. `again`: the player chose "Use the line again" in Turbine.
pub fn still_undone(data: &Path, app_id: &str, set_now: Option<&str>, again: bool) -> bool {
    let f = undone_file(data, app_id);
    let Ok(then) = fs::read_to_string(&f) else { return false };
    let set_again = set_now.is_some_and(|s| s != then && launch_option_ok(s, data));
    if again || set_again {
        let _ = fs::remove_file(&f);
        return false;
    }
    true
}

/// Do the Launch Options mention Turbine's helper at all (the line to undo)?
pub fn launch_option_uses_helper(set: &str, data: &Path) -> bool {
    set.contains(&*helper_path(data).to_string_lossy())
}

/// Keeps the player's Launch Options from before Turbine's line: whatever they are while they don't hold Turbine's
/// line is what Undo puts back (the newest, so options the player changed after an Undo aren't lost
/// to older ones).
pub fn remember_original(data: &Path, app_id: &str, set: &str) -> io::Result<()> {
    if launch_option_uses_helper(set, data) || !app_id.bytes().all(|b| b.is_ascii_digit()) || app_id.is_empty() {
        return Ok(());
    }
    fs::create_dir_all(launch_dir(data))?;
    fs::write(original_file(data, app_id), set)
}

/// What to paste back to undo Turbine's line: the player's earlier Launch Options ("" when they were empty).
pub fn original_options(data: &Path, app_id: &str) -> String {
    fs::read_to_string(original_file(data, app_id)).unwrap_or_default()
}

/// The helper's text. It runs only Turbine's own me3 from the file Turbine wrote, or Steam's own command.
/// `flatpak_id`: Turbine's Flatpak; when it is no longer installed the helper runs Steam's own command.
///
/// Steam's whole command stays as Steam built it (the launch wrapper with its options, the reaper with
/// `SteamLaunch AppId=<app>`, the runtime and Proton with its verb, in Steam's own order). With me3's Windows build
/// (the normal case) only the game's program after Proton's verb is swapped for me3.exe, so Proton is exactly the
/// one Steam chose. With the Linux me3 (used when the game's program isn't known) everything before the runtime
/// stays in front. When the reaper or `AppId=<app>` isn't in what stays in front, the game starts the normal way.
/// me3 runs as a child: its exit code, run time and last output lines go to steam-launch.log.
pub fn helper_text(flatpak_id: Option<&str>) -> String {
    let installed = match flatpak_id {
        Some(id) => format!(
            "# Turbine gone (uninstalled, its data kept): the game starts normally. Steam's library path and overlay are\n# left out of the check so they can't make it fail.\nif command -v flatpak > /dev/null 2>&1; then\n  env -u LD_LIBRARY_PATH -u LD_PRELOAD flatpak info {id} > /dev/null 2>&1\n  code=$?\n  if [ \"$code\" -ne 0 ]; then\n    say \"Turbine Mod Manager isn't installed any more (flatpak info answered $code): Steam's own command\"\n    exec \"$@\"\n  fi\nfi\n"
        ),
        None => String::new(),
    };
    let text = r##"#!/bin/bash
# Turbine Mod Manager's Steam launch helper. Steam runs it from a game's Launch Options (Turbine's line runs Steam's
# own command when this file is gone). With mods on in Turbine (a "<app id>.me3" file next to it, one argument per
# line, written by Turbine from its current state) it starts the game through Turbine's own me3 with Turbine's profile;
# "<app id>.me3.once" is for the next Play only. Otherwise, and on any doubt, it runs Steam's own command unchanged.
# Steam's command stays whole and in its own order: its launch wrapper, its reaper with the app id, its runtime and
# its Proton. With me3's Windows build only the game's program is swapped for me3.exe. It runs nothing else. Turbine
# writes this file again when it changes; every step goes to steam-launch.log.
here=$(cd "$(dirname "$0")" && pwd)
data=$(dirname "$here")
log="$here/steam-launch.log"
if [ -f "$log" ] && [ "$(wc -c < "$log")" -gt 262144 ]; then tail -n 400 "$log" > "$log.tmp" && mv -f "$log.tmp" "$log"; fi
say() { printf '%s %s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "$*" >> "$log" 2>/dev/null; }
args=("$@")
app="${SteamAppId:-${STEAM_COMPAT_APP_ID:-}}"
if [ -z "$app" ]; then
  for a in "$@"; do case "$a" in AppId=*) app="${a#AppId=}" ;; esac; done
fi
say "Steam started app ${app:-unknown} in $(pwd): $*"
if [ "$#" -eq 0 ]; then
  say "No command from Steam: the Launch Options need %command% after this file"
  exit 1
fi
case "$app" in
  ''|*[!0-9]*) say "No app id: Steam's own command"; exec "$@" ;;
esac
# 0.62: Steam's command runs Turbine's own Steam tool (the game uses "Turbine (mods)" and the Launch Options still
# hold this helper): the tool decides, and comes back here with the line it built. Nothing is used up here.
for a in "$@"; do
  case "$a" in
    */compatibilitytools.d/*/turbine-tool)
      say "Steam's command runs Turbine's Steam tool: it goes on from here"
      exec "$@" ;;
  esac
done
# Proton's own log, into this folder (Report a problem picks it up): on when Turbine's setting is on, or when the
# Launch Options turned it on themselves.
if [ -f "$here/proton-log.on" ]; then export PROTON_LOG=1; fi
if [ -n "${PROTON_LOG:-}" ] && [ "$PROTON_LOG" != 0 ]; then
  export PROTON_LOG_DIR="$here"
  say "Proton log on: $here/steam-$app.log"
fi
@INSTALLED@if [ -f "$here/$app.plain" ]; then
  if [ -n "$(find "$here/$app.plain" -mmin -10 2> /dev/null)" ]; then
    rm -f "$here/$app.plain" "$here/$app.me3.once"
    say "Play without mods (once): Steam's own command"
    exec "$@"
  fi
  rm -f "$here/$app.plain"
  say "An old Play without mods (over ten minutes) was left: ignored"
fi
flag="$here/$app.me3"
if [ -f "$here/$app.me3.once" ] && [ -z "$(find "$here/$app.me3.once" -mmin -10 2> /dev/null)" ]; then
  rm -f "$here/$app.me3.once"
  say "An old this-Play-only file (over ten minutes) was left: ignored"
fi
if [ -f "$here/$app.me3.once" ]; then
  flag="$here/$app.me3.once.run"
  mv -f "$here/$app.me3.once" "$flag"
  say "This Play only: $(head -n 1 "$flag")"
elif [ ! -f "$flag" ]; then
  say "No mods on in Turbine: Steam's own command"
  exec "$@"
fi
me3=()
while IFS= read -r line || [ -n "$line" ]; do
  line="${line%$'\r'}"
  case "$line" in '#'*|'') continue ;; esac
  me3+=("$line")
done < "$flag"
case "$flag" in *.run) rm -f "$flag" ;; esac
normal() {
  say "$1: Steam's own command, without mods"
  exec "${args[@]}"
}
# Only Turbine's own me3, from its data folder: the Windows build (run by Steam's own Proton) or the Linux one.
prog="${me3[0]}"
case "$prog" in
  "$data"/me3/*/bin/win64/me3.exe) mode=windows ;;
  "$data"/me3/*/bin/me3) mode=linux ;;
  *) normal "The mods file doesn't name Turbine's own me3 ($prog)" ;;
esac
if [ "${#me3[@]}" -lt 2 ] || [ ! -f "$prog" ] || { [ "$mode" = linux ] && [ ! -x "$prog" ]; }; then
  normal "me3 isn't where Turbine put it ($prog)"
fi
# The game and the profile must be there (a Windows path is Proton's Z: drive, the whole file system).
unix_path() {
  local p="${1#[A-Za-z]:}"
  printf '%s' "${p//\\//}"
}
for ((i = 1; i + 1 < ${#me3[@]}; i++)); do
  case "${me3[$i]}" in
    --exe|-p)
      f=$(unix_path "${me3[$((i + 1))]}")
      [ -f "$f" ] || normal "Not found: $f (${me3[$((i + 1))]})" ;;
  esac
done
# Steam's command: where its runtime starts (the first one of its runtime or Proton) and where Proton is.
cut=-1
proton=-1
for ((i = 0; i < ${#args[@]}; i++)); do
  case "${args[$i]}" in
    */proton)
      [ "$cut" -lt 0 ] && cut=$i
      proton=$i
      break ;;
    */_v2-entry-point|*/SteamLinuxRuntime*/*) [ "$cut" -lt 0 ] && cut=$i ;;
  esac
done
if [ "$mode" = windows ]; then
  # Everything up to Proton's verb stays; the game's program after it is swapped for me3.exe.
  if [ "$proton" -lt 0 ] || [ $((proton + 2)) -ge "${#args[@]}" ]; then
    normal "Steam's command has no Proton with a program to run (${args[*]})"
  fi
  keep=("${args[@]:0:$((proton + 2))}")
  extra=("${args[@]:$((proton + 3))}")
  say "Steam's Proton for this game: ${args[$proton]} ${args[$((proton + 1))]}; me3 starts the game in place of ${args[$((proton + 2))]}"
  cmd=("${keep[@]}" "${me3[@]}")
  if [ "${#extra[@]}" -gt 0 ]; then cmd+=(-- "${extra[@]}"); fi
else
  # The Linux me3 builds its own runtime and Proton: everything in front of Steam's stays.
  if [ "$cut" -lt 0 ]; then
    normal "Steam's command has no runtime or Proton (${args[*]})"
  fi
  keep=("${args[@]:0:$cut}")
  if [ "$proton" -ge 0 ]; then say "Steam's Proton for this game: ${args[$proton]} (the Linux me3 picks its own from Steam's settings)"; fi
  cmd=("${keep[@]}" "${me3[@]}")
fi
# Steam's reaper with this app's id must be in front: it is how Steam knows the game is running.
reaper=no
appid=no
for a in "${keep[@]}"; do
  case "$a" in
    */reaper) reaper=yes ;;
    "AppId=$app") appid=yes ;;
  esac
done
# Started by Turbine's Steam tool: Steam's reaper is above the tool, not in this line (the tool looked for it).
if [ "${TURBINE_TOOL_REAPER:-}" = yes ]; then
  reaper=yes
  appid=yes
fi
unset TURBINE_TOOL_REAPER
if [ "$reaper" != yes ] || [ "$appid" != yes ]; then
  normal "Steam's reaper with AppId=$app isn't in front of the game (reaper: $reaper, AppId: $appid; Steam's command: ${args[*]})"
fi
say "Kept in front: ${keep[*]}"
say "Mods on ($mode me3): ${cmd[*]}"
out="$here/me3-$app.log"
start=$SECONDS
"${cmd[@]}" > "$out" 2>&1
code=$?
say "me3 ended with exit code $code after $((SECONDS - start)) s"
tail -n 20 "$out" 2> /dev/null | sed -e 's/\x1b\][^\x07]*\x07//g' -e 's/\x1b\[[0-9;?]*[A-Za-z]//g' -e 's/\r$//' | while IFS= read -r l; do say "me3: $l"; done
# me3's own log (the Windows build keeps it inside the game's Proton prefix): a copy next to this file.
if [ "$mode" = windows ] && [ -n "${STEAM_COMPAT_DATA_PATH:-}" ]; then
  own=$(ls -t "$STEAM_COMPAT_DATA_PATH"/pfx/drive_c/users/steamuser/AppData/Local/garyttierney/me3/data/logs/*/*.log 2> /dev/null | head -n 1)
  if [ -n "$own" ] && cp -f "$own" "$here/me3-$app-own.log" 2> /dev/null; then say "me3's own log: $own (copied to me3-$app-own.log)"; fi
fi
exit "$code"
"##;
    text.replace("@INSTALLED@", &installed)
}

/// Writes the helper (executable) when it is missing or different. Answers true when it wrote it.
pub fn write_helper(data: &Path, flatpak_id: Option<&str>) -> io::Result<bool> {
    let path = helper_path(data);
    let text = helper_text(flatpak_id);
    if fs::read_to_string(&path).is_ok_and(|t| t == text) {
        return Ok(false);
    }
    fs::create_dir_all(launch_dir(data))?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, &text)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o755))?;
    }
    fs::rename(&tmp, &path)?;
    Ok(true)
}

/// me3's arguments for the helper (the program first), without flatpak-spawn: the helper already runs on the host.
/// This is the Linux me3, which builds its own runtime and Proton; `helper_args_for` with the game's program is the
/// normal way.
pub fn helper_args(me3: &Path, profile: &Path) -> Vec<String> {
    launch_args(me3, profile).into_iter().skip(2).collect()
}

/// A host path as Proton's programs see it: the Z: drive is the whole file system.
pub fn windows_path(p: &Path) -> String {
    format!("Z:{}", p.to_string_lossy().replace('/', "\\"))
}

/// me3's arguments for the helper. With the game's program (`game_exe`, the host path): me3's Windows build,
/// which the helper runs under Steam's own Proton in place of the game's program, with the game and the profile as
/// Z: paths (`--exe` means me3 needs no Steam lookup of its own; its Windows binaries are next to me3.exe). Without
/// it: the Linux me3 (`helper_args`).
pub fn helper_args_for(me3: &Path, profile: &Path, game_exe: Option<&Path>) -> Vec<String> {
    match game_exe {
        Some(exe) => vec![
            me3.join("bin/win64/me3.exe").to_string_lossy().into(),
            "launch".into(),
            "--exe".into(),
            windows_path(exe),
            "-p".into(),
            windows_path(profile),
        ],
        None => helper_args(me3, profile),
    }
}

/// Proton's own log for Steam launches of the games the helper starts (steam-<app id>.log in the helper's folder):
/// on while this file is there.
pub fn proton_log_file(data: &Path) -> PathBuf {
    launch_dir(data).join("proton-log.on")
}

/// Turns Proton's log on or off for the next Steam launches.
pub fn set_proton_log(data: &Path, on: bool) -> io::Result<()> {
    if on {
        fs::create_dir_all(launch_dir(data))?;
        fs::write(proton_log_file(data), b"on\n")
    } else {
        remove_if(&proton_log_file(data))
    }
}

/// The last `n` lines of a text file (empty when it can't be read).
fn last_lines(path: &Path, n: usize) -> Vec<String> {
    let Ok(b) = fs::read(path) else { return Vec::new() };
    let t = String::from_utf8_lossy(&b);
    let lines: Vec<&str> = t.lines().collect();
    lines[lines.len().saturating_sub(n)..].iter().map(|l| strip_escapes(l)).collect()
}

/// A line without terminal escape codes (colours, window titles) and carriage returns.
fn strip_escapes(l: &str) -> String {
    let mut out = String::new();
    let mut it = l.chars().peekable();
    while let Some(c) = it.next() {
        if c != '\u{1b}' {
            if c != '\r' {
                out.push(c);
            }
            continue;
        }
        match it.next() {
            Some('[') => {
                for x in it.by_ref() {
                    if x.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            Some(']') => {
                for x in it.by_ref() {
                    if x == '\u{7}' {
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// The Steam launch logs for Report a problem: the helper's log, then for each game me3's output, me3's own log
/// (copied from the game's Proton prefix) and Proton's log when there is one. Each with a heading line and its last
/// lines.
pub fn launch_report_lines(data: &Path) -> Vec<String> {
    let dir = launch_dir(data);
    let mut out = Vec::new();
    let add = |name: &str, n: usize, out: &mut Vec<String>| {
        let lines = last_lines(&dir.join(name), n);
        if !lines.is_empty() {
            out.push(format!("-- {name} (last {} lines) --", lines.len()));
            out.extend(lines);
        }
    };
    // First, in one sentence, which Proton Turbine's Steam tool used at the last launch and why (the tool
    // chooses by itself when Turbine has no record of the one Steam used), or why it started nothing.
    let recent = last_lines(&dir.join("steam-launch.log"), 80);
    if let Some(w) = crate::compat_tool::nothing_started(&recent).map(|w| format!("Turbine's Steam tool: {w}")).or_else(|| crate::compat_tool::proton_words(&recent).map(|w| format!("Turbine's Steam tool: {w}"))) {
        out.push(w);
    }
    add("steam-launch.log", 80, &mut out);
    let mut names: Vec<String> = fs::read_dir(&dir)
        .map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect())
        .unwrap_or_default();
    names.sort();
    for name in names.iter().filter(|n| n.starts_with("me3-") && n.ends_with(".log")) {
        add(name, 40, &mut out);
    }
    for name in names.iter().filter(|n| n.starts_with("steam-") && n.ends_with(".log") && *n != "steam-launch.log") {
        add(name, 60, &mut out);
    }
    out
}

fn check_app_id(app_id: &str) -> io::Result<()> {
    if app_id.is_empty() || !app_id.bytes().all(|b| b.is_ascii_digit()) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a Steam app id"));
    }
    Ok(())
}

fn write_flag(file: &Path, args: &[String], state: &str) -> io::Result<()> {
    if args.iter().any(|x| x.contains('\n') || x.contains('\r') || x.is_empty()) || state.contains('\n') {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "an argument holds a line break"));
    }
    if let Some(p) = file.parent() {
        fs::create_dir_all(p)?;
    }
    let tmp = file.with_extension("tmp");
    fs::write(&tmp, format!("# {state}\n{}\n", args.join("\n")))?;
    fs::rename(&tmp, file)
}

fn remove_if(file: &Path) -> io::Result<()> {
    match fs::remove_file(file) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

/// Mods on (Some: me3's arguments) or off (None) for the game's Steam launches. Off also clears a waiting
/// next-Play-only file; on clears a waiting Play without mods. A line break in an argument is refused.
pub fn arm(data: &Path, app_id: &str, args: Option<&[String]>) -> io::Result<()> {
    arm_with(data, app_id, args, "mods on")
}

/// `arm` with words saying the state it carries (the file's first line, in the helper's log).
pub fn arm_with(data: &Path, app_id: &str, args: Option<&[String]>, state: &str) -> io::Result<()> {
    check_app_id(app_id)?;
    match args {
        None => {
            remove_if(&once_file(data, app_id))?;
            remove_if(&armed_file(data, app_id))
        }
        // A waiting Play without mods stays: it is for the Play that asked for it (Turbine's Play with mods clears it).
        Some(a) => write_flag(&armed_file(data, app_id), a, state),
    }
}

/// The next Play only (Seamless Co-op): the helper uses this once, then the normal state again.
pub fn arm_once(data: &Path, app_id: &str, args: &[String], state: &str) -> io::Result<()> {
    check_app_id(app_id)?;
    write_flag(&once_file(data, app_id), args, state)?;
    remove_if(&plain_file(data, app_id))
}

/// A Play without mods that is still waiting goes (Turbine's own Play with mods).
pub fn clear_plain(data: &Path, app_id: &str) -> io::Result<()> {
    check_app_id(app_id)?;
    remove_if(&plain_file(data, app_id))
}

/// One Play without mods through Steam (the helper removes the file when it runs, and ignores one older than ten
/// minutes).
pub fn arm_plain_once(data: &Path, app_id: &str) -> io::Result<()> {
    check_app_id(app_id)?;
    fs::create_dir_all(launch_dir(data))?;
    remove_if(&once_file(data, app_id))?;
    fs::write(plain_file(data, app_id), b"once\n")
}

/// The helper's log lines since `since_len` bytes (what the helper did for the last launch), at most 40 lines.
pub fn helper_log_since(data: &Path, since_len: u64) -> Vec<String> {
    let Ok(text) = fs::read(helper_log(data)) else { return Vec::new() };
    let from = if since_len as usize > text.len() { 0 } else { since_len as usize };
    let tail = String::from_utf8_lossy(&text[from..]).to_string();
    let lines: Vec<String> = tail.lines().map(str::to_string).collect();
    let skip = lines.len().saturating_sub(40);
    lines.into_iter().skip(skip).collect()
}

/// Turbine's state for Steam's own Play of a game me3 starts: Ok when the helper may start me3 (mods on, the player
/// turned the anti-cheat off for them, nothing holds them back, the last apply worked, me3 is there); otherwise why
/// not, and the helper is cleared. Every way of turning mods off ends here. `undone`: also off after Undo the launch
/// options change until the line is set again.
pub fn helper_state_undone(mods_on: usize, consent: bool, held: bool, apply_failed: bool, me3_ok: bool, undone: bool) -> Result<(), &'static str> {
    if undone {
        return Err("the launch options change was undone");
    }
    if mods_on == 0 {
        Err("no mods on")
    } else if !consent {
        Err("anti-cheat is on (you haven't turned it off for mods)")
    } else if held {
        Err("held without mods until they update")
    } else if apply_failed {
        Err("the last apply failed")
    } else if !me3_ok {
        Err("Mod Engine 3 isn't installed")
    } else {
        Ok(())
    }
}

/// Which profile: Play with mods (solo, Turbine's own save) or Play Seamless Co-op (its .co2 saves).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coop {
    Solo,
    Seamless,
}

/// Is this a Seamless Co-op native?
pub fn is_seamless_native(n: &str) -> bool {
    let l = n.to_ascii_lowercase();
    l.ends_with("/ersc.dll") || l == "ersc.dll"
}

// ---- Seamless Co-op's session password ----
//
// The mod stops the game at once when its settings file has no session password ("Please enter a session password
// in your settings file"). The file it reads is the ersc_settings.ini next to its ersc.dll, and under me3 that is
// the copy inside Turbine's me3 profile for the game: so that copy, and no other, is what Turbine checks before a
// co-op Play.

/// What the player hears when co-op can't start for want of a password.
pub const COOP_NEEDS_PASSWORD: &str = "Seamless Co-op needs a session password. Set one in its settings (every player uses the same one), then play.";

/// The state of the session password in the file the mod will load.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoopPassword {
    /// A password is set (never kept or logged here).
    Set,
    /// The settings file is there and the password is empty.
    Empty(PathBuf),
    /// Seamless Co-op is in the profile but its settings file isn't next to it (where it would be).
    NoFile(PathBuf),
    /// Seamless Co-op isn't in the profile's package folder.
    NoMod,
}

impl CoopPassword {
    /// Ok when co-op can start; otherwise the words for the player.
    pub fn ready(&self) -> Result<(), String> {
        match self {
            CoopPassword::Set => Ok(()),
            CoopPassword::Empty(_) => Err(COOP_NEEDS_PASSWORD.to_string()),
            CoopPassword::NoFile(_) => Err(format!("Seamless Co-op's settings file isn't with the mod yet. {COOP_NEEDS_PASSWORD}")),
            CoopPassword::NoMod => Err("Seamless Co-op isn't on in My mods".to_string()),
        }
    }

    /// A short word for logs and the app: "set", "empty", "nofile" or "nomod".
    pub fn word(&self) -> &'static str {
        match self {
            CoopPassword::Set => "set",
            CoopPassword::Empty(_) => "empty",
            CoopPassword::NoFile(_) => "nofile",
            CoopPassword::NoMod => "nomod",
        }
    }
}

/// The session password in a settings file's text: the `cooppassword` key (any letter case, spaces around the key,
/// the `=` and the value allowed), from the [PASSWORD] section when the file has one, else from anywhere. Quotes
/// around the whole value come off; comment lines (`;` or `#`) are skipped. None when the key isn't in the file.
pub fn coop_password_in(text: &str) -> Option<String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut section = String::new();
    let mut in_section: Option<String> = None;
    let mut anywhere: Option<String> = None;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.split(']').next()) {
            section = name.trim().to_ascii_lowercase();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        if !key.trim().eq_ignore_ascii_case("cooppassword") {
            continue;
        }
        let mut v = value.trim();
        for q in ['"', '\''] {
            if v.len() >= 2 && v.starts_with(q) && v.ends_with(q) {
                v = v[1..v.len() - 1].trim();
            }
        }
        if section == "password" {
            in_section.get_or_insert(v.to_string());
        } else {
            anywhere.get_or_insert(v.to_string());
        }
    }
    in_section.or(anywhere)
}

/// The settings file the mod will load under me3: ersc_settings.ini (any letter case) in the folder of its ersc.dll
/// inside the profile's package folder. Answers (the file or where it would be, is it there); None without the mod.
pub fn coop_settings_file(store_root: &Path) -> Option<(PathBuf, bool)> {
    let mods = profile_mods(store_root);
    let native = natives(&mods).into_iter().find(|n| is_seamless_native(n))?;
    let dll = profile_dir(store_root).join(native);
    let dir = dll.parent()?.to_path_buf();
    let found = fs::read_dir(&dir).ok().and_then(|rd| {
        let mut names: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("ersc_settings.ini")) && p.is_file()).collect();
        names.sort();
        names.into_iter().next()
    });
    Some(match found {
        Some(f) => (f, true),
        None => (dir.join("ersc_settings.ini"), false),
    })
}

/// The session password's state in the file the mod will load for this game.
pub fn coop_password(store_root: &Path) -> CoopPassword {
    let Some((file, there)) = coop_settings_file(store_root) else { return CoopPassword::NoMod };
    if !there {
        return CoopPassword::NoFile(file);
    }
    let bytes = fs::read(&file).unwrap_or_default();
    match coop_password_in(&String::from_utf8_lossy(&bytes)) {
        Some(p) if !p.is_empty() => CoopPassword::Set,
        _ => CoopPassword::Empty(file),
    }
}

// ---- Which Seamless Co-op the game loads ----

/// Is this path Seamless Co-op's DLL (whatever folder it is in)?
pub fn is_seamless_file(rel: &str) -> bool {
    rel.replace('\\', "/").rsplit('/').next().is_some_and(|n| n.eq_ignore_ascii_case("ersc.dll"))
}

/// Is this list of files Seamless Co-op's own package, and nothing else: its ersc.dll in a SeamlessCoop folder with
/// ersc_settings.ini beside it, at the top or one folder down ("Game/SeamlessCoop/..."), and beside that folder
/// only a few loose files (its launcher, a readme)? A bigger mod that merely carries an ersc.dll somewhere is not
/// (such a bundle must not take Seamless Co-op's place in the list), and neither is a pack that holds a
/// whole SeamlessCoop folder among other folders: those are mods of their own.
pub fn is_seamless_package(files: &[String]) -> bool {
    let lower: Vec<String> = files.iter().map(|f| f.replace('\\', "/").to_ascii_lowercase()).collect();
    lower.iter().any(|f| {
        let Some(prefix) = f.strip_suffix("seamlesscoop/ersc.dll") else { return false };
        let folders = prefix.split('/').filter(|p| !p.is_empty()).count();
        if !(prefix.is_empty() || prefix.ends_with('/')) || folders > 1 || !lower.contains(&format!("{prefix}seamlesscoop/ersc_settings.ini")) {
            return false;
        }
        // Everything is in that one place: the SeamlessCoop folder, and loose files right beside it.
        let mut loose = 0;
        lower.iter().all(|x| match x.strip_prefix(prefix) {
            Some(rest) if rest.starts_with("seamlesscoop/") => true,
            Some(rest) if !rest.contains('/') => {
                loose += 1;
                loose <= 8
            }
            _ => false,
        })
    })
}

/// The words when an older Seamless Co-op download is not put over a newer installed one. The app asks ("Keep
/// <have>" by default, or "Use <this>", which is `Store::import_archive_allowing_older` for that same file).
pub fn older_seamless_words(have: &str, this: &str) -> String {
    format!("You have Seamless Co-op {have}. This file is {this}, which is older, so Turbine kept {have}.")
}

/// (the version installed, the older file's version) from `older_seamless_words`, for the app's question.
pub fn older_seamless_in(words: &str) -> Option<(String, String)> {
    let rest = &words[words.find("You have Seamless Co-op ")? + "You have Seamless Co-op ".len()..];
    let (have, rest) = rest.split_once(". This file is ")?;
    let (this, _) = rest.split_once(", which is older")?;
    Some((have.to_string(), this.to_string()))
}

/// The version a Windows program file says it is ("2.0.2"), from its version block (VS_FIXEDFILEINFO: the file
/// version's four numbers). None when the file carries none. Trailing ".0" parts beyond the second are dropped.
pub fn dll_version_in(bytes: &[u8]) -> Option<String> {
    let (struc, ms, ls) = crate::vcrun::fixed_file_info(bytes)?;
    // dwStrucVersion must be 1.0.
    if struc != 0x0001_0000 {
        return None;
    }
    let mut parts = vec![ms >> 16, ms & 0xFFFF, ls >> 16, ls & 0xFFFF];
    if parts.iter().all(|p| *p == 0) {
        return None;
    }
    while parts.len() > 2 && parts.last() == Some(&0) {
        parts.pop();
    }
    Some(parts.iter().map(u32::to_string).collect::<Vec<_>>().join("."))
}

/// `dll_version_in` for a file (at most 64 MB read).
pub fn dll_version(path: &Path) -> Option<String> {
    let meta = fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > 64 << 20 {
        return None;
    }
    dll_version_in(&fs::read(path).ok()?)
}

/// Seamless Co-op's DLL as the game will load it: next to the settings file in the profile's package folder.
pub fn coop_dll(store_root: &Path) -> Option<PathBuf> {
    let native = natives(&profile_mods(store_root)).into_iter().find(|n| is_seamless_native(n))?;
    Some(profile_dir(store_root).join(native))
}

/// Puts a session password into a settings file's text (the `cooppassword` line the mod reads, see
/// `coop_password_in`), keeping everything else and the line endings. None when the file has no such line.
pub fn coop_password_put(text: &str, password: &str) -> Option<String> {
    if password.contains(['\n', '\r']) {
        return None;
    }
    let mut section = String::new();
    let mut hit: Option<(usize, bool)> = None;
    let mut pos = 0;
    for raw in text.split_inclusive('\n') {
        let line = raw.trim();
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.split(']').next()) {
            section = name.trim().to_ascii_lowercase();
        } else if !line.starts_with(';') && !line.starts_with('#') && let Some((key, _)) = line.split_once('=') && key.trim().trim_start_matches('\u{feff}').eq_ignore_ascii_case("cooppassword") {
            let in_section = section == "password";
            if hit.is_none() || (in_section && hit.is_some_and(|(_, s)| !s)) {
                hit = Some((pos, in_section));
            }
        }
        pos += raw.len();
    }
    let (start, _) = hit?;
    let raw = text[start..].split_inclusive('\n').next()?;
    let ending = if raw.ends_with("\r\n") { "\r\n" } else if raw.ends_with('\n') { "\n" } else { "" };
    let key = raw.split_once('=')?.0;
    Some(format!("{}{}= {password}{ending}{}", &text[..start], key, &text[start + raw.len()..]))
}

/// The profile file for each way.
pub fn profile_file_for(store_root: &Path, coop: Coop) -> PathBuf {
    match coop {
        Coop::Solo => profile_file(store_root),
        Coop::Seamless => profile_dir(store_root).join("turbine-coop.me3"),
    }
}

/// Writes the profile for one way: Solo leaves Seamless Co-op out and uses Turbine's own save; Seamless takes it in
/// and keeps its .co2 saves. Seamless without Seamless Co-op in the package folder is an error, and so is Seamless
/// without a session password in the mod's settings file (`COOP_NEEDS_PASSWORD`; `coop_password` tells which).
pub fn write_profile_for(store_root: &Path, game: &str, coop: Coop) -> io::Result<(PathBuf, Vec<String>)> {
    let mods = profile_mods(store_root);
    fs::create_dir_all(&mods)?;
    let all = natives(&mods);
    let has = all.iter().any(|n| is_seamless_native(n));
    let (nat, save): (Vec<String>, Option<&str>) = match coop {
        Coop::Solo => (all.into_iter().filter(|n| !is_seamless_native(n)).collect(), Some(ME3_SAVEFILE)),
        // Never a co-op profile for a Play the mod would stop at once (no session password in its file).
        Coop::Seamless if has => match coop_password(store_root).ready() {
            Ok(()) => (all, None),
            Err(why) => return Err(io::Error::new(io::ErrorKind::InvalidInput, why)),
        },
        Coop::Seamless => return Err(io::Error::new(io::ErrorKind::NotFound, "Seamless Co-op isn't on in My mods")),
    };
    let file = profile_file_for(store_root, coop);
    let tmp = file.with_extension("tmp");
    fs::write(&tmp, profile_text(game, &nat, save))?;
    fs::rename(&tmp, &file)?;
    Ok((file, nat))
}

/// Is me3 there and unchanged (each file's SHA-256 as recorded when it was unpacked)?
pub fn me3_ready(dir: &Path) -> Result<(), String> {
    let manifest = fs::read_to_string(dir.join("turbine-files.json")).map_err(|_| "Mod Engine 3 isn't installed yet".to_string())?;
    let v = json::parse(&manifest).map_err(|_| "Mod Engine 3's file list can't be read".to_string())?;
    let mut n = 0;
    for f in v.get("files").map(Json::as_array).unwrap_or(&[]) {
        let (Some(rel), Some(want)) = (f.str_at("rel"), f.str_at("sha256")) else { continue };
        let got = sha256::file_hex(&dir.join(rel)).map_err(|_| format!("Mod Engine 3's {rel} is missing"))?;
        if !sha256::same_digest(want, &got) {
            return Err(format!("Mod Engine 3's {rel} changed since Turbine installed it"));
        }
        n += 1;
    }
    if n < ME3_FILES.len() {
        return Err("Mod Engine 3's files aren't all there".into());
    }
    Ok(())
}

/// Unpacks the checked me3 release (gzip, then tar) into `dir`: only `ME3_FILES`, regular files, safe names. Writes
/// turbine-files.json with each file's SHA-256 and marks bin/me3 executable. Never runs anything.
pub fn install_me3(tgz: &Path, dir: &Path) -> Result<Vec<(String, String)>, String> {
    let gz = fs::read(tgz).map_err(|e| format!("The Mod Engine 3 download can't be read ({e})"))?;
    if !sha256::same_digest(&sha256::digest_hex(&gz), ME3_SHA256) {
        return Err("The Mod Engine 3 download isn't the release Turbine expects (its SHA-256 is different)".into());
    }
    let tar = gunzip(&gz, 64 << 20)?;
    let entries = tar_entries(&tar)?;
    let tmp = dir.with_extension("part");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let mut got = Vec::new();
    for want in ME3_FILES {
        let (_, data) = entries
            .iter()
            .find(|(name, _)| name == want)
            .ok_or_else(|| format!("The Mod Engine 3 release has no {want}"))?;
        let out = tmp.join(want);
        fs::create_dir_all(out.parent().unwrap_or(&tmp)).map_err(|e| e.to_string())?;
        fs::write(&out, &tar[data.clone()]).map_err(|e| e.to_string())?;
        got.push((want.to_string(), sha256::digest_hex(&tar[data.clone()])));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(tmp.join("bin/me3"), fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    }
    let files: Vec<String> = got.iter().map(|(r, h)| format!("{{\"rel\":{},\"sha256\":{}}}", quote(r), quote(h))).collect();
    fs::write(
        tmp.join("turbine-files.json"),
        format!("{{\"version\":{},\"source\":{},\"files\":[{}]}}\n", quote(ME3_VERSION), quote(ME3_URL), files.join(",")),
    )
    .map_err(|e| e.to_string())?;
    let _ = fs::remove_dir_all(dir);
    if let Some(p) = dir.parent() {
        fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    fs::rename(&tmp, dir).map_err(|e| e.to_string())?;
    Ok(got)
}

/// gzip: the header, then raw DEFLATE (the trailer's CRC is not needed: the whole file's SHA-256 was checked).
pub fn gunzip(gz: &[u8], limit: usize) -> Result<Vec<u8>, String> {
    if gz.len() < 18 || gz[0] != 0x1f || gz[1] != 0x8b || gz[2] != 8 {
        return Err("not a gzip file".into());
    }
    let flags = gz[3];
    let mut i = 10;
    if flags & 4 != 0 {
        let n = *gz.get(i).ok_or("short gzip")? as usize | (*gz.get(i + 1).ok_or("short gzip")? as usize) << 8;
        i += 2 + n;
    }
    for bit in [8u8, 16] {
        if flags & bit != 0 {
            while *gz.get(i).ok_or("short gzip")? != 0 {
                i += 1;
            }
            i += 1;
        }
    }
    if flags & 2 != 0 {
        i += 2;
    }
    inflate::inflate(gz.get(i..).ok_or("short gzip")?, limit).map_err(|e| format!("the gzip data can't be read ({e:?})"))
}

/// A tar file's regular files: (name without "./", byte range). Links, devices and unsafe names are refused.
pub fn tar_entries(tar: &[u8]) -> Result<Vec<(String, std::ops::Range<usize>)>, String> {
    let mut out = Vec::new();
    let mut i = 0;
    let mut long_name: Option<String> = None;
    while i + 512 <= tar.len() {
        let h = &tar[i..i + 512];
        if h.iter().all(|b| *b == 0) {
            break;
        }
        let field = |a: usize, b: usize| {
            let f = &h[a..b];
            let end = f.iter().position(|c| *c == 0).unwrap_or(f.len());
            String::from_utf8_lossy(&f[..end]).to_string()
        };
        let size = usize::from_str_radix(field(124, 136).trim(), 8).map_err(|_| "a tar entry has a bad size")?;
        let kind = h[156];
        let prefix = field(345, 500);
        let mut name = long_name.take().unwrap_or_else(|| if prefix.is_empty() { field(0, 100) } else { format!("{prefix}/{}", field(0, 100)) });
        let data = i + 512..i + 512 + size;
        if data.end > tar.len() {
            return Err("the tar file is cut short".into());
        }
        match kind {
            b'L' => long_name = Some(String::from_utf8_lossy(&tar[data.clone()]).trim_end_matches('\0').to_string()),
            b'0' | 0 => {
                if let Some(s) = name.strip_prefix("./") {
                    name = s.to_string();
                }
                zip::safe_relative(&name).map_err(|_| format!("the tar file has an unsafe name: {name}"))?;
                out.push((name, data.clone()));
            }
            _ => {} // folders and headers need nothing; links and devices are never taken
        }
        i += 512 + size.div_ceil(512) * 512;
    }
    Ok(out)
}

// ---- Before you play ----

/// The anti-cheat lines for Before you play, for the way the game is played with mods. `mods_on`: any mod is on.
/// `me3_ok`: me3 is installed and unchanged (None: not asked, me3 isn't used here). `asked`: the player turned it off
/// for mods (the ask-once answer). `build_changed`: the game updated since the last modded Play.
pub fn checks(game: &str, ac: &AntiCheat, mods_on: bool, me3_ok: Option<&Result<(), String>>, asked: bool, build_changed: bool) -> Vec<Check> {
    let mut out = Vec::new();
    match mode(ac) {
        // Without mods the game plays normally, with its anti-cheat.
        Mode::Refuse if mods_on => out.push(Check { level: "stop".into(), title: format!("{game} needs its anti-cheat"), detail: REFUSE_WORDS.into() }),
        Mode::Official if mods_on => out.push(Check { level: "warn".into(), title: format!("Start {game} without anti-cheat"), detail: official_words(game, ac) }),
        Mode::Offline if mods_on => out.push(Check {
            level: "warn".into(),
            title: "Play offline with mods".into(),
            detail: offline_words(ac).unwrap_or_default() + " Online play with changed game files can get your account banned.",
        }),
        Mode::Me3 if mods_on => {
            if needs_ask(ac) && !asked {
                out.push(Check { level: "warn".into(), title: ask_title(game), detail: ask_text(game, ac) });
            } else if ac.starter.is_some() {
                out.push(Check { level: "ok".into(), title: STATUS_ROW.into(), detail: "Mod Engine 3 starts it offline with your mods. Your normal save stays untouched.".into() });
            }
            if let Some(w) = offline_words(ac) {
                out.push(Check { level: "warn".into(), title: "Start offline".into(), detail: w });
            }
            match me3_ok {
                // Nothing to fix: Turbine fetches it at Play.
                Some(Err(e)) if e.contains("isn't installed") => out.push(Check {
                    level: "ok".into(),
                    title: "Mod Engine 3 downloads now".into(),
                    detail: "It's the loader that starts the game offline (about 10 MB).".into(),
                }),
                Some(Err(e)) => out.push(Check { level: "warn".into(), title: "Mod Engine 3 needs installing again".into(), detail: format!("{e}. Turbine gets it again before the game starts.") }),
                _ => {}
            }
            if build_changed {
                out.push(Check {
                    level: "warn".into(),
                    title: format!("{game} updated"),
                    detail: "Mod Engine 3 and your mods may need updates too. If the game doesn't start, Play without mods still works.".into(),
                });
            }
        }
        Mode::Warn if mods_on => out.push(Check {
            level: if ac.level == "stop" { "stop".into() } else { "warn".into() },
            title: format!("Anti-cheat: {}", ac.name),
            detail: if ac.level == "stop" {
                format!("{game} doesn't allow changed game files. Play it without mods.")
            } else {
                "Online play with mods can get you banned. Use mods only in offline or single-player modes.".into()
            },
        }),
        _ => {}
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn er() -> AntiCheat {
        AntiCheat {
            name: "Easy Anti-Cheat".into(),
            level: "warn".into(),
            mode: "me3".into(),
            me3: Some("eldenring".into()),
            starter: Some("Game/start_protected_game.exe".into()),
            game_exe: Some("Game/eldenring.exe".into()),
            ..Default::default()
        }
    }

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("turbine-ac-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn modes() {
        assert_eq!(mode(&er()), Mode::Me3);
        let mut bad = er();
        bad.me3 = Some("notagame".into());
        assert_eq!(mode(&bad), Mode::Warn);
        let refuse = AntiCheat { name: "BattlEye".into(), level: "refuse".into(), mode: "refuse".into(), ..Default::default() };
        assert_eq!(mode(&refuse), Mode::Refuse);
        assert!(needs_ask(&er()));
        let sekiro = AntiCheat { name: "No anti-cheat".into(), level: "warn".into(), mode: "me3".into(), me3: Some("sekiro".into()), ..Default::default() };
        assert!(!needs_ask(&sekiro));
    }

    #[test]
    fn ask_words_warn_of_bans() {
        let t = ask_text("Elden Ring", &er());
        assert!(t.contains("banned") && t.contains("offline only") && t.contains("never changes the game's own files"));
        assert_eq!(ask_title("Elden Ring"), "Play Elden Ring with mods?");
        assert!(t.contains("Seamless Co-op") && t.contains(".co2"));
        let ac6 = AntiCheat { me3: Some("armoredcore6".into()), ..er() };
        assert!(!ask_text("Armored Core VI", &ac6).contains("Seamless"));
        assert!(!t.contains('\u{2014}') && !t.contains('\u{2013}'));
    }

    #[test]
    fn states() {
        let d = tmp("states");
        fs::create_dir_all(d.join("Game")).unwrap();
        fs::write(d.join("Game/start_protected_game.exe"), b"eac starter").unwrap();
        fs::write(d.join("Game/eldenring.exe"), b"the game").unwrap();
        let ac = er();
        assert_eq!(state(&d, &ac, None, Some("100"), 0), AcState::Unrecorded);
        let rec = record_now(&d, &ac, Some("100")).unwrap();
        assert_eq!(rec.files.len(), 2);
        let f = d.join("rec.json");
        rec.save(&f).unwrap();
        let rec = Original::load(&f).unwrap();
        assert_eq!(state(&d, &ac, Some(&rec), Some("100"), 0), AcState::Original);
        assert_eq!(state(&d, &ac, Some(&rec), Some("100"), 3), AcState::TurbineFilesLeft(3));
        // Another tool swapped the starter: the same build, a different file.
        fs::write(d.join("Game/start_protected_game.exe"), b"something else").unwrap();
        assert_eq!(state(&d, &ac, Some(&rec), Some("100"), 0), AcState::ChangedOutside(vec!["Game/start_protected_game.exe".into()]));
        assert!(state_check("Elden Ring", &state(&d, &ac, Some(&rec), Some("100"), 0)).unwrap().detail.contains("Verify integrity"));
        // The game updated: a new build changes them.
        assert_eq!(state(&d, &ac, Some(&rec), Some("101"), 0), AcState::Updated);
        // Steam's verify put the starter back, but another tool's backup sits beside it: a leftover, not a block.
        fs::write(d.join("Game/start_protected_game.exe"), b"eac starter").unwrap();
        fs::write(d.join("Game/start_protected_game.exe.backup"), b"eac starter").unwrap();
        let st = state(&d, &ac, Some(&rec), Some("100"), 0);
        assert_eq!(st, AcState::LeftoverBackup("Game/start_protected_game.exe.backup".into()));
        let c = state_check("Elden Ring", &st).unwrap();
        assert_eq!(c.level, "warn");
        assert!(c.detail.contains("Steam's Verify doesn't remove"));
        // A swapped starter with the backup: still a stop, and the words say whose record it is.
        fs::write(d.join("Game/start_protected_game.exe"), b"swapped").unwrap();
        let c = state_check("Elden Ring", &state(&d, &ac, Some(&rec), Some("100"), 0)).unwrap();
        assert_eq!(c.level, "stop");
        assert!(c.detail.contains("not Steam's list"));
        // After an update the record is renewed, with a plain word to run Steam's Verify.
        let c = state_check("Elden Ring", &AcState::Updated).unwrap();
        assert!(c.level == "warn" && c.detail.contains("Verify integrity"));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn profile() {
        let d = tmp("profile");
        let mods = profile_mods(&d);
        fs::create_dir_all(mods.join("SeamlessCoop")).unwrap();
        fs::create_dir_all(mods.join("parts")).unwrap();
        fs::write(mods.join("parts/a.partsbnd.dcx"), b"x").unwrap();
        fs::write(mods.join("SkipIntro.dll"), b"x").unwrap();
        let (file, nat) = write_profile_for(&d, "eldenring", Coop::Solo).unwrap();
        assert_eq!(nat, vec!["mods/SkipIntro.dll".to_string()]);
        let t = fs::read_to_string(&file).unwrap();
        assert!(t.contains("game = \"eldenring\"") && t.contains("path = 'mods'") && t.contains("savefile = 'TurbineModded.sl2'"));
        // Seamless Co-op on: its own .co2 saves, no savefile of Turbine's.
        fs::write(mods.join("SeamlessCoop/ersc.dll"), b"x").unwrap();
        fs::write(mods.join("SeamlessCoop/ersc_settings.ini"), b"[PASSWORD]\ncooppassword = x\n").unwrap();
        let (file, nat) = write_profile_for(&d, "eldenring", Coop::Seamless).unwrap();
        assert_eq!(nat.len(), 2);
        let t = fs::read_to_string(&file).unwrap();
        assert!(!t.contains("savefile") && t.contains("path = 'mods/SeamlessCoop/ersc.dll'"));
        assert_eq!(package_rel("Game/regulation.bin"), "regulation.bin");
        assert_eq!(package_rel("game/parts/x.dcx"), "parts/x.dcx");
        assert_eq!(package_rel("SeamlessCoop/ersc.dll"), "SeamlessCoop/ersc.dll");
        let args = launch_args(Path::new("/d/me3/0.13.0"), &file);
        assert_eq!(&args[..3], &["flatpak-spawn".to_string(), "--host".into(), "/d/me3/0.13.0/bin/me3".into()]);
        assert_eq!(args[5], "launch");
        let _ = fs::remove_dir_all(&d);
    }

    fn tar_of(files: &[(&str, &[u8], u8)]) -> Vec<u8> {
        let mut t = Vec::new();
        for (name, data, kind) in files {
            let mut h = [0u8; 512];
            h[..name.len()].copy_from_slice(name.as_bytes());
            let size = format!("{:011o}\0", data.len());
            h[124..136].copy_from_slice(size.as_bytes());
            h[156] = *kind;
            t.extend_from_slice(&h);
            t.extend_from_slice(data);
            t.resize(t.len().div_ceil(512) * 512, 0);
        }
        t.resize(t.len() + 1024, 0);
        t
    }

    #[test]
    fn profile_names_are_safe() {
        assert!(profile_safe("SeamlessCoop/ersc.dll"));
        for bad in ["a'b.dll", "a\nb.dll", "a\\b.dll", "a\"b.dll", ""] {
            assert!(!profile_safe(bad), "{bad:?}");
        }
        let t = profile_text("eldenring", &["mods/a'\n[[natives]]\npath='/evil.dll".to_string(), "mods/ok.dll".to_string()], None);
        assert!(!t.contains("evil") && t.contains("mods/ok.dll"));
        let d = tmp("names");
        let mods = profile_mods(&d);
        fs::create_dir_all(&mods).unwrap();
        fs::write(mods.join("bad\nname.dll"), b"x").unwrap();
        fs::write(mods.join("good.dll"), b"x").unwrap();
        assert_eq!(natives(&mods), vec!["mods/good.dll".to_string()]);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn only_me3_games_have_their_folder_checked() {
        assert!(folder_checked(&er()));
        let halo = AntiCheat { name: "Easy Anti-Cheat".into(), level: "warn".into(), mode: "official".into(), official_choice: Some("x".into()), ..Default::default() };
        assert!(!folder_checked(&halo));
    }

    #[test]
    fn tar_reading() {
        let t = tar_of(&[("./bin/me3", b"elf", b'0'), ("./bin/", b"", b'5'), ("./link", b"", b'2'), ("./bin/win64/me3.exe", b"mz", b'0')]);
        let e = tar_entries(&t).unwrap();
        let names: Vec<&str> = e.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["bin/me3", "bin/win64/me3.exe"]);
        assert_eq!(&t[e[0].1.clone()], b"elf");
        let bad = tar_of(&[("../evil", b"x", b'0')]);
        assert!(tar_entries(&bad).is_err());
    }

    #[test]
    fn gzip_stored_block() {
        // A gzip of "hello" made with a stored (uncompressed) DEFLATE block.
        let mut gz = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 3];
        gz.extend_from_slice(&[1, 5, 0, 0xfa, 0xff]);
        gz.extend_from_slice(b"hello");
        gz.extend_from_slice(&[0; 8]);
        assert_eq!(gunzip(&gz, 100).unwrap(), b"hello");
        assert!(gunzip(b"not gzip at all, not at all", 100).is_err());
    }

    #[test]
    fn me3_must_be_the_pinned_release() {
        let d = tmp("pinned");
        let f = d.join("me3.tar.gz");
        fs::write(&f, b"not the release").unwrap();
        assert!(install_me3(&f, &d.join("me3")).unwrap_err().contains("Turbine expects"));
        assert!(me3_ready(&d.join("me3")).unwrap_err().contains("isn't installed"));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn before_play() {
        let ac = er();
        let c = checks("Elden Ring", &ac, true, Some(&Err("me3 isn't installed yet".into())), false, false);
        assert!(c.iter().any(|x| x.title == "Play Elden Ring with mods?"));
        assert!(c.iter().any(|x| x.title == "Mod Engine 3 downloads now" && x.level == "ok"));
        let c = checks("Elden Ring", &ac, true, Some(&Ok(())), true, true);
        assert!(c.iter().any(|x| x.title == STATUS_ROW) && c.iter().any(|x| x.title == "Elden Ring updated"));
        assert!(checks("Elden Ring", &ac, false, None, false, false).is_empty());
        let halo = AntiCheat { name: "Easy Anti-Cheat".into(), level: "warn".into(), mode: "official".into(), official_choice: Some("Play Halo: MCC Anti-Cheat Disabled (Mods and Limited Services)".into()), ..Default::default() };
        assert!(checks("Halo MCC", &halo, true, None, false, false)[0].detail.contains("Anti-Cheat Disabled (Mods and Limited Services)"));
        assert_eq!(official_guide("Halo MCC", &halo).len(), 5);
        let ds3 = AntiCheat { name: "x".into(), level: "warn".into(), mode: "me3".into(), me3: Some("darksouls3".into()), offline_setting: Some("Network, Launch setting: Start offline".into()), ..Default::default() };
        assert!(checks("Dark Souls III", &ds3, true, Some(&Ok(())), true, false).iter().any(|x| x.detail.contains("Start offline")));
        let refuse = AntiCheat { name: "BattlEye".into(), level: "refuse".into(), mode: "refuse".into(), ..Default::default() };
        assert_eq!(checks("Destiny 2", &refuse, true, None, false, false)[0].level, "stop");
        assert!(checks("Destiny 2", &refuse, false, None, false, false).is_empty());
    }
}
