//! Games with anti-cheat (0.57). What the game's data file says (`rules::AntiCheat`) decides the way Turbine plays
//! it with mods:
//!
//! - **me3** (Elden Ring, Nightreign, Armored Core VI, Dark Souls III, Sekiro): me3 starts the game's own program
//!   without the anti-cheat starter, offline, with mods from a Turbine-owned me3 profile folder. Nothing is ever
//!   placed in the game folder: the store deploys into the profile's package folder instead (`profile_mods`). me3
//!   is downloaded from its official GitHub release, pinned to one version and checked by SHA-256 (`ME3_*`),
//!   unpacked by this module (gzip and tar, names checked, only the files it needs) into Turbine's own data folder,
//!   and started on the host with fixed arguments (`launch_args`). The profile uses its own save file, so a
//!   modded save never meets the online game. Seamless Co-op loads through me3 as a native from the same profile
//!   folder (its documented way with me3), downloaded the same way (`SEAMLESS_*`); with it on, its own .co2 saves
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

/// Seamless Co-op for Elden Ring, pinned: the official GitHub release (v2.0.1, 2026-09-03; the repository moved
/// from LukeYui to yuiamoroll, which GitHub redirects). Size and SHA-256 checked against the file itself.
pub const SEAMLESS_VERSION: &str = "2.0.1";
pub const SEAMLESS_URL: &str = "https://github.com/yuiamoroll/EldenRingSeamlessCoopRelease/releases/download/v2.0.1/Seamless.Co-op.v2.0.1.zip";
pub const SEAMLESS_SIZE: u64 = 8_078_349;
pub const SEAMLESS_SHA256: &str = "848ae27e1c77217590dac401c012a17d9b4f05b9b9a24aa33af54d8d7b8ba2a1";
/// Seamless Co-op's DLL inside its zip; me3 loads it as a native.
pub const SEAMLESS_DLL: &str = "SeamlessCoop/ersc.dll";

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
    format!("Turn off anti-cheat for {game}?")
}

pub fn ask_text(game: &str, ac: &AntiCheat) -> String {
    let offline = ac.offline_setting.as_deref().map(|s| format!(" Also set the game to start offline: {s}.")).unwrap_or_default();
    format!(
        "{game}'s anti-cheat stops mods from loading. To play with mods, Turbine starts the game without it, through me3. \
         The game then plays offline only: no co-op, invasions or messages. Playing online with changed game files can get \
         your account banned, so Turbine never does that. Turbine saves a copy of your save files first, uses a separate \
         save while mods are on, and never changes the game's own files. Play without mods starts it the normal way, with \
         its anti-cheat.{offline}"
    )
}

/// The ask needs an answer only where there is an anti-cheat or an online ban to turn away from (not Sekiro).
pub fn needs_ask(ac: &AntiCheat) -> bool {
    mode(ac) == Mode::Me3 && (ac.starter.is_some() || ac.offline_setting.is_some())
}

pub const STATUS_ROW: &str = "Anti-cheat: off while mods are on (offline only)";
pub const REFUSE_WORDS: &str =
    "This game needs its anti-cheat to play at all. Changing its files can get your account banned, so Turbine won't add mods to it.";
pub const VERIFY_WORDS: &str = "Use Steam's Verify integrity of game files to repair it: game page, gear, Properties, Installed Files.";

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
        "To play online again, turn the mods off (or Play without mods) and pick the normal Play choice.".into(),
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
    /// Turbine still has files in the game folder (a restore that didn't finish, or files from before 0.57).
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
    // A starter with a backup beside it was swapped by something (another tool's way of starting without it).
    if let Some(st) = &ac.starter {
        if game_dir.join(format!("{st}.backup")).is_file() && !changed.contains(st) {
            changed.push(st.clone());
        }
    }
    if changed.is_empty() {
        return AcState::Original;
    }
    if build.is_some() && rec.build.as_deref() != build {
        return AcState::Updated;
    }
    AcState::ChangedOutside(changed)
}

/// The line for Before you play about the game folder, when it isn't ready for the normal launch.
pub fn state_check(game: &str, s: &AcState) -> Option<Check> {
    match s {
        AcState::Original | AcState::Unrecorded | AcState::Updated => None,
        AcState::ChangedOutside(files) => Some(Check {
            level: "stop".into(),
            title: "The anti-cheat files changed".into(),
            detail: format!(
                "{} isn't as Steam installed it ({}). Starting it online like this can get your account banned. {VERIFY_WORDS}",
                game,
                files.join(", ")
            ),
        }),
        AcState::TurbineFilesLeft(n) => Some(Check {
            level: "stop".into(),
            title: "Mod files are still in the game folder".into(),
            detail: format!(
                "Turbine couldn't take {n} {} out of {game}'s folder, so it won't start the game with its anti-cheat. Game menu, Reset game files, or {VERIFY_WORDS}",
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
                out.push(format!("mods/{r}"));
            }
        }
    }
    let mut out = Vec::new();
    walk(mods, "", &mut out, 0);
    out
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
    t.push_str(&format!("\n[[supports]]\ngame = \"{game}\"\n\n[[packages]]\nid = \"turbine-mods\"\npath = 'mods'\n"));
    for n in natives {
        t.push_str(&format!("\n[[natives]]\npath = {}\n", toml_str(n)));
    }
    t
}

/// Writes the profile for what is in the package folder now. Returns the profile file and the natives listed.
pub fn write_profile(store_root: &Path, game: &str) -> io::Result<(PathBuf, Vec<String>)> {
    let mods = profile_mods(store_root);
    fs::create_dir_all(&mods)?;
    let nat = natives(&mods);
    let seamless = nat.iter().any(|n| n.to_ascii_lowercase().ends_with("/ersc.dll"));
    let text = profile_text(game, &nat, if seamless { None } else { Some(ME3_SAVEFILE) });
    let file = profile_file(store_root);
    let tmp = file.with_extension("tmp");
    fs::write(&tmp, text)?;
    fs::rename(&tmp, &file)?;
    Ok((file, nat))
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

/// Is me3 there and unchanged (each file's SHA-256 as recorded when it was unpacked)?
pub fn me3_ready(dir: &Path) -> Result<(), String> {
    let manifest = fs::read_to_string(dir.join("turbine-files.json")).map_err(|_| "me3 isn't installed yet".to_string())?;
    let v = json::parse(&manifest).map_err(|_| "me3's file list can't be read".to_string())?;
    let mut n = 0;
    for f in v.get("files").map(Json::as_array).unwrap_or(&[]) {
        let (Some(rel), Some(want)) = (f.str_at("rel"), f.str_at("sha256")) else { continue };
        let got = sha256::file_hex(&dir.join(rel)).map_err(|_| format!("me3's {rel} is missing"))?;
        if !sha256::same_digest(want, &got) {
            return Err(format!("me3's {rel} changed since Turbine installed it"));
        }
        n += 1;
    }
    if n < ME3_FILES.len() {
        return Err("me3's files aren't all there".into());
    }
    Ok(())
}

/// Unpacks the checked me3 release (gzip, then tar) into `dir`: only `ME3_FILES`, regular files, safe names. Writes
/// turbine-files.json with each file's SHA-256 and marks bin/me3 executable. Never runs anything.
pub fn install_me3(tgz: &Path, dir: &Path) -> Result<Vec<(String, String)>, String> {
    let gz = fs::read(tgz).map_err(|e| format!("The me3 download can't be read ({e})"))?;
    if !sha256::same_digest(&sha256::digest_hex(&gz), ME3_SHA256) {
        return Err("The me3 download isn't the pinned release (its SHA-256 is different)".into());
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
            .ok_or_else(|| format!("The me3 release has no {want}"))?;
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
            b'5' | b'x' | b'g' => {}
            _ => {} // links and devices are never taken
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
        Mode::Official if mods_on => out.push(Check { level: "warn".into(), title: "Pick the Play choice without anti-cheat".into(), detail: official_words(game, ac) }),
        Mode::Offline if mods_on => out.push(Check {
            level: "warn".into(),
            title: "Play offline with mods".into(),
            detail: offline_words(ac).unwrap_or_default() + " Online play with changed game files can get your account banned.",
        }),
        Mode::Me3 if mods_on => {
            if needs_ask(ac) && !asked {
                out.push(Check { level: "warn".into(), title: ask_title(game), detail: ask_text(game, ac) });
            } else if ac.starter.is_some() {
                out.push(Check { level: "ok".into(), title: STATUS_ROW.into(), detail: "me3 starts it offline with your mods; your normal save is left alone.".into() });
            }
            if let Some(w) = offline_words(ac) {
                out.push(Check { level: "warn".into(), title: "Start offline".into(), detail: w });
            }
            match me3_ok {
                Some(Err(e)) if e.contains("isn't installed") => out.push(Check {
                    level: "warn".into(),
                    title: "me3 isn't installed yet".into(),
                    detail: format!("Turbine gets me3 {ME3_VERSION} from its GitHub release (checked) the first time you play with mods."),
                }),
                Some(Err(e)) => out.push(Check { level: "warn".into(), title: "me3 needs installing again".into(), detail: format!("{e}. Turbine gets it again before starting.") }),
                _ => {}
            }
            if build_changed {
                out.push(Check {
                    level: "warn".into(),
                    title: format!("{game} updated"),
                    detail: "me3 and your mods may need updates too. If it doesn't start, Play without mods works.".into(),
                });
            }
        }
        Mode::Warn if mods_on => out.push(Check {
            level: if ac.level == "stop" { "stop".into() } else { "warn".into() },
            title: format!("Anti-cheat: {}", ac.name),
            detail: if ac.level == "stop" {
                format!("{game} doesn't allow changed game files. Play it without mods.")
            } else {
                "Anti-cheat: online play with mods can get you banned. Use mods only in offline or single player modes.".into()
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
        assert_eq!(ask_title("Elden Ring"), "Turn off anti-cheat for Elden Ring?");
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
        // Steam's verify put the starter back, but another tool's backup sits beside it.
        fs::write(d.join("Game/start_protected_game.exe"), b"eac starter").unwrap();
        fs::write(d.join("Game/start_protected_game.exe.backup"), b"eac starter").unwrap();
        assert!(matches!(state(&d, &ac, Some(&rec), Some("100"), 0), AcState::ChangedOutside(_)));
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
        let (file, nat) = write_profile(&d, "eldenring").unwrap();
        assert_eq!(nat, vec!["mods/SkipIntro.dll".to_string()]);
        let t = fs::read_to_string(&file).unwrap();
        assert!(t.contains("game = \"eldenring\"") && t.contains("path = 'mods'") && t.contains("savefile = 'TurbineModded.sl2'"));
        // Seamless Co-op on: its own .co2 saves, no savefile of Turbine's.
        fs::write(mods.join("SeamlessCoop/ersc.dll"), b"x").unwrap();
        let (file, nat) = write_profile(&d, "eldenring").unwrap();
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
        assert!(install_me3(&f, &d.join("me3")).unwrap_err().contains("pinned"));
        assert!(me3_ready(&d.join("me3")).unwrap_err().contains("isn't installed"));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn before_play() {
        let ac = er();
        let c = checks("Elden Ring", &ac, true, Some(&Err("me3 isn't installed yet".into())), false, false);
        assert!(c.iter().any(|x| x.title == "Turn off anti-cheat for Elden Ring?"));
        assert!(c.iter().any(|x| x.title == "me3 isn't installed yet"));
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
