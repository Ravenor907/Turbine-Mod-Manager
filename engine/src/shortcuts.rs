//! The player's non-Steam games (Steam's shortcuts.vdf), read only. In Game Mode gamescope shows only what Steam
//! itself starts, so Turbine opens the player's browser through Steam: it looks for a Chrome-family browser the
//! player added to Steam as a non-Steam game and starts it with steam://rungameid/<id>. The browser helper then
//! opens the Nexus page in it.
//!
//! The file lives at <Steam>/userdata/<account>/config/shortcuts.vdf (binary VDF, see vdf::parse_binary). Each
//! shortcut keeps its 32-bit appid; very old files have none, and then Steam's own rule gives it:
//! crc32(exe + name) with the top bit set. The id Steam's rungameid link takes is (appid << 32) | 0x02000000.
//! Turbine never writes this file.

use crate::vdf::{self, Vdf};
use std::path::{Path, PathBuf};

/// One non-Steam game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shortcut {
    pub appid: u32,
    pub name: String,
    pub exe: String,
    pub launch_options: String,
}

/// Chrome-family browsers Turbine can find among shortcuts, in the order it prefers them, each with the words
/// (lower case) that mark it in the program path or launch options. Chrome-family because the browser helper is a
/// Chrome extension.
pub const BROWSERS: &[(&str, &[&str])] = &[
    ("Google Chrome", &["com.google.chrome", "google-chrome", "/opt/google/chrome/"]),
    ("Brave", &["com.brave.browser", "brave-browser", "/usr/bin/brave"]),
    ("Chromium", &["org.chromium.chromium", "/usr/bin/chromium", "chromium-browser"]),
    ("Microsoft Edge", &["com.microsoft.edge", "microsoft-edge"]),
    ("Vivaldi", &["com.vivaldi.vivaldi", "vivaldi-stable", "/usr/bin/vivaldi"]),
];

/// The standard CRC-32 (IEEE, as zip uses), which Steam uses for old shortcut ids.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// Steam's appid for a shortcut that has none stored: crc32 of the program path (as stored, quotes and all) followed
/// by the name, with the top bit set.
pub fn legacy_appid(exe: &str, name: &str) -> u32 {
    let mut s = Vec::with_capacity(exe.len() + name.len());
    s.extend_from_slice(exe.as_bytes());
    s.extend_from_slice(name.as_bytes());
    crc32(&s) | 0x8000_0000
}

/// The 64-bit id steam://rungameid/ takes for a shortcut.
pub fn game_id(appid: u32) -> u64 {
    (u64::from(appid) << 32) | 0x0200_0000
}

/// steam://rungameid/<id> for a shortcut.
pub fn rungameid_url(appid: u32) -> String {
    format!("steam://rungameid/{}", game_id(appid))
}

fn field<'a>(v: &'a Vdf, key: &str) -> &'a str {
    v.get(key).and_then(Vdf::as_str).unwrap_or("")
}

/// The shortcuts in a shortcuts.vdf file's bytes. A broken file gives an error; a shortcut with an unreadable appid
/// gets Steam's computed one.
pub fn parse(bytes: &[u8]) -> Result<Vec<Shortcut>, vdf::VdfError> {
    let root = vdf::parse_binary(bytes)?;
    let list = root.get("shortcuts").map(Vdf::entries).unwrap_or(&[]);
    let mut out = Vec::new();
    for (_, s) in list {
        if !matches!(s, Vdf::Obj(_)) {
            continue;
        }
        let name = field(s, "AppName").to_string();
        let exe = field(s, "Exe").to_string();
        let launch_options = field(s, "LaunchOptions").to_string();
        let appid = field(s, "appid").parse::<u32>().ok().filter(|a| *a != 0).unwrap_or_else(|| legacy_appid(&exe, &name));
        out.push(Shortcut { appid, name, exe, launch_options });
    }
    Ok(out)
}

/// Which Chrome-family browser a shortcut starts, by its program path or launch options (never by its name alone, a
/// player can call anything "Chrome").
pub fn browser_of(s: &Shortcut) -> Option<&'static str> {
    let hay = format!("{} {}", s.exe, s.launch_options).to_lowercase();
    BROWSERS.iter().find(|(_, marks)| marks.iter().any(|m| hay.contains(m))).map(|(n, _)| *n)
}

/// The preferred browser among some shortcuts (BROWSERS order, then file order).
pub fn pick_browser(list: &[Shortcut]) -> Option<(&'static str, &Shortcut)> {
    BROWSERS.iter().find_map(|(name, _)| list.iter().find(|s| browser_of(s) == Some(*name)).map(|s| (*name, s)))
}

/// A browser found among the player's shortcuts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserShortcut {
    /// "Google Chrome", "Brave" and so on.
    pub browser: String,
    /// The name the player sees in Steam.
    pub name: String,
    pub appid: u32,
    /// The file it was read from.
    pub file: PathBuf,
}

impl BrowserShortcut {
    pub fn url(&self) -> String {
        rungameid_url(self.appid)
    }

    /// {"found":true,"browser","name","appid","gameid","url"} for the app and the activity log.
    pub fn to_json(&self) -> String {
        format!(
            "{{\"found\":true,\"browser\":{},\"name\":{},\"appid\":{},\"gameid\":\"{}\",\"url\":{}}}",
            crate::json::quote(&self.browser),
            crate::json::quote(&self.name),
            self.appid,
            game_id(self.appid),
            crate::json::quote(&self.url())
        )
    }
}

/// Every shortcuts.vdf under Steam's folders in this home, the most recently changed first (that is the account
/// signed in now).
pub fn shortcut_files(home: &Path) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = crate::steam::steam_roots(home);
    // A Steam folder without steamapps (a fresh install) still has userdata.
    for rel in [".local/share/Steam", ".steam/steam"] {
        let p = home.join(rel);
        let real = std::fs::canonicalize(&p).unwrap_or(p);
        if real.join("userdata").is_dir() && !roots.contains(&real) {
            roots.push(real);
        }
    }
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    for root in roots {
        let Ok(rd) = std::fs::read_dir(root.join("userdata")) else { continue };
        for e in rd.flatten() {
            let f = e.path().join("config").join("shortcuts.vdf");
            if let Ok(m) = std::fs::metadata(&f) {
                if m.is_file() && !files.iter().any(|(_, p)| *p == f) {
                    files.push((m.modified().unwrap_or(std::time::UNIX_EPOCH), f));
                }
            }
        }
    }
    files.sort_by_key(|f| std::cmp::Reverse(f.0));
    files.into_iter().map(|(_, p)| p).collect()
}

/// The player's Chrome-family browser among their non-Steam games, if they added one. Read only.
pub fn find_browser(home: &Path) -> Option<BrowserShortcut> {
    for f in shortcut_files(home) {
        // shortcuts.vdf is small; anything huge is not one.
        let Ok(bytes) = std::fs::read(&f) else { continue };
        if bytes.len() > 16 << 20 {
            continue;
        }
        let Ok(list) = parse(&bytes) else { continue };
        if let Some((browser, s)) = pick_browser(&list) {
            return Some(BrowserShortcut { browser: browser.into(), name: s.name.clone(), appid: s.appid, file: f });
        }
    }
    None
}

/// Words that mark Turbine's own shortcut (the Flatpak's id, or its program), lower case.
pub const TURBINE_MARKS: &[&str] = &["io.github.ravenor907.turbinemodmanager", "turbine-mod-manager"];

/// Is this shortcut Turbine itself (by its program path or launch options, never by its name alone)?
pub fn is_turbine(s: &Shortcut) -> bool {
    let hay = format!("{} {}", s.exe, s.launch_options).to_lowercase();
    TURBINE_MARKS.iter().any(|m| hay.contains(m))
}

/// Turbine's own entry among the player's non-Steam games, if it was added to Steam: after signing in with the
/// player's own browser in Game Mode, steam://rungameid/<its id> brings Turbine back to the front. Read only.
pub fn find_turbine(home: &Path) -> Option<(u32, String)> {
    for f in shortcut_files(home) {
        let Ok(bytes) = std::fs::read(&f) else { continue };
        if bytes.len() > 16 << 20 {
            continue;
        }
        let Ok(list) = parse(&bytes) else { continue };
        if let Some(s) = list.iter().find(|s| is_turbine(s)) {
            return Some((s.appid, s.name.clone()));
        }
    }
    None
}

/// The steam://rungameid link for a game id Steam put in the environment (SteamGameId, set for a game or
/// non-Steam shortcut Steam started), when it is a plausible one.
pub fn rungameid_from_env(value: &str) -> Option<String> {
    let v = value.trim();
    (!v.is_empty() && v.len() <= 20 && v.bytes().all(|b| b.is_ascii_digit()) && v != "0").then(|| format!("steam://rungameid/{v}"))
}
