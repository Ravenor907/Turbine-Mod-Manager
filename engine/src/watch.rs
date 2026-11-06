//! "Slow download" from Nexus: the player presses it in their own browser, the browser saves the file in Downloads,
//! and Turbine picks it up and adds it to the right game.
//!
//! When Turbine opens a Nexus mod page it records an `Expectation`: which game, which mod, the files Nexus lists for
//! it (names and sizes, when known) and when the page was opened. A `Watcher` then looks at the Downloads folder (and
//! any others it is given) once a second for a new zip, 7z or rar archive that:
//! - was last changed after the page was opened;
//! - is finished: its size was the same at two checks in a row, and the browser's unfinished-download file next to
//!   it is gone (Firefox writes "<name>.part", Chromium-based browsers "<name>.crdownload", others "<name>.download");
//! - matches one of the mod's files by name (browsers add " (1)" or "(1)" to names they already have) or by size.
//!
//! When Nexus's file list isn't known, the newest finished archive within 30 minutes of opening the page is taken,
//! marked as a guess so the app can ask before adding it. Files that don't match are never touched: the watcher only
//! reads folder listings and file sizes, and hands back a path.
//!
//! `Watcher::step` is pure (a listing in, a match out) and is what the tests drive with made-up folders and clocks;
//! `scan`, `poll` and `wait` read the real folders.

use crate::archives;
use crate::nexus::FileList;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// How long after opening the page a download still counts (seconds).
pub const WINDOW_SECS: u64 = 30 * 60;
/// How often `wait` looks (seconds).
pub const POLL_SECS: u64 = 1;
/// Endings of the files browsers write while a download is still going.
pub const PARTIAL_SUFFIXES: &[&str] = &[".part", ".crdownload", ".download"];
/// Files changed this many seconds before the page was opened still count (clocks and file times are coarse).
const SLACK_SECS: u64 = 2;

/// One file Nexus lists for the mod.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpectedFile {
    pub file_id: u64,
    /// The download's file name on Nexus ("SkyUI_5_2_SE-12604-5-2SE.7z"); empty when unknown.
    pub file_name: String,
    /// Its size in bytes; 0 when unknown.
    pub size_bytes: u64,
    pub version: String,
}

/// What Turbine is waiting for after opening a Nexus mod page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expectation {
    /// Turbine's ID of the game the mod goes into.
    pub game_id: String,
    /// Nexus game domain ("cyberpunk2077").
    pub domain: String,
    pub mod_id: u64,
    /// The files Nexus lists for the mod (empty when the list couldn't be fetched).
    pub files: Vec<ExpectedFile>,
    /// When the page was opened (seconds since 1970).
    pub started_at: u64,
}

impl Expectation {
    pub fn new(game_id: &str, domain: &str, mod_id: u64, started_at: u64) -> Expectation {
        Expectation { game_id: game_id.into(), domain: domain.into(), mod_id, files: Vec::new(), started_at }
    }

    /// With the files from Nexus's file list (the ones that can still be downloaded).
    pub fn with_files(mut self, list: &FileList) -> Expectation {
        self.files = list
            .files
            .iter()
            .filter(|f| !f.is_removed())
            .map(|f| ExpectedFile { file_id: f.file_id, file_name: f.file_name.clone(), size_bytes: f.size_bytes, version: f.version.clone() })
            .collect();
        self
    }

    /// Past the 30 minutes: stop watching.
    pub fn expired_at(&self, now: u64) -> bool {
        now > self.started_at.saturating_add(WINDOW_SECS)
    }

    /// Does Turbine know anything to match on (a name or a size)?
    fn knows_files(&self) -> bool {
        self.files.iter().any(|f| !f.file_name.is_empty() || f.size_bytes > 0)
    }
}

/// One file in a watched folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seen {
    pub path: PathBuf,
    pub size: u64,
    /// Last changed (seconds since 1970).
    pub modified: u64,
}

/// How sure the match is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    /// Nothing to match on: the newest archive since the page was opened. Ask before adding it.
    Guess,
    /// Same size as one of the mod's files.
    Size,
    /// Same file name as one of the mod's files.
    Name,
}

impl Confidence {
    /// "high", "medium" or "low", for the app and the activity log.
    pub fn label(self) -> &'static str {
        match self {
            Confidence::Name => "high",
            Confidence::Size => "medium",
            Confidence::Guess => "low",
        }
    }
}

/// A finished download that belongs to the expectation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    pub path: PathBuf,
    pub size: u64,
    /// The Nexus file it matched, when it matched one.
    pub file: Option<ExpectedFile>,
    pub confidence: Confidence,
}

/// "name (1).zip" and "name(1).zip" are the browser's copies of "name.zip".
fn without_copy_number(stem: &str) -> &str {
    let s = stem.trim_end();
    if let Some(open) = s.rfind('(') {
        let inner = &s[open + 1..];
        if let Some(num) = inner.strip_suffix(')') {
            if !num.is_empty() && num.bytes().all(|b| b.is_ascii_digit()) {
                return s[..open].trim_end();
            }
        }
    }
    s
}

fn split_ext(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    }
}

/// Is `actual` (a file in Downloads) the download of `expected` (Nexus's file name)? Ignores case and the copy
/// number browsers add.
pub fn same_download_name(actual: &str, expected: &str) -> bool {
    if expected.is_empty() {
        return false;
    }
    let (a_stem, a_ext) = split_ext(actual);
    let (e_stem, e_ext) = split_ext(expected);
    a_ext.eq_ignore_ascii_case(e_ext) && without_copy_number(a_stem).eq_ignore_ascii_case(e_stem.trim_end())
}

/// Nexus's older files only have their size in whole KB, so sizes within 1 KB count as the same.
fn same_size(actual: u64, expected: u64) -> bool {
    expected > 0 && actual.abs_diff(expected) < 1024
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Watches folders for the download an expectation describes.
#[derive(Clone, Debug)]
pub struct Watcher {
    pub expectation: Expectation,
    pub folders: Vec<PathBuf>,
    /// Sizes at the previous look, to tell when a file stopped growing.
    sizes: HashMap<PathBuf, u64>,
}

impl Watcher {
    pub fn new(expectation: Expectation, folders: Vec<PathBuf>) -> Watcher {
        Watcher { expectation, folders, sizes: HashMap::new() }
    }

    /// One look at the folders' listing. Returns the best finished match, if there is one yet.
    pub fn step(&mut self, listing: &[Seen]) -> Option<Found> {
        let exp = &self.expectation;
        // Unfinished downloads: "<name>.part" means "<name>" isn't done.
        let busy: Vec<PathBuf> = listing
            .iter()
            .filter_map(|s| {
                let name = file_name(&s.path);
                let lc = name.to_ascii_lowercase();
                let suffix = PARTIAL_SUFFIXES.iter().find(|x| lc.ends_with(*x))?;
                Some(s.path.with_file_name(&name[..name.len() - suffix.len()]))
            })
            .collect();
        let since = exp.started_at.saturating_sub(SLACK_SECS);
        let mut sizes = HashMap::new();
        let mut best: Option<(Found, u64)> = None;
        for s in listing {
            let name = file_name(&s.path);
            if !archives::is_archive_name(&name) || s.modified < since {
                continue;
            }
            sizes.insert(s.path.clone(), s.size);
            let stable = self.sizes.get(&s.path) == Some(&s.size);
            if !stable || s.size == 0 || busy.contains(&s.path) {
                continue;
            }
            let found = if exp.knows_files() {
                let by_name = exp.files.iter().find(|f| same_download_name(&name, &f.file_name));
                let by_size = exp.files.iter().find(|f| same_size(s.size, f.size_bytes));
                match (by_name, by_size) {
                    (Some(f), _) => Found { path: s.path.clone(), size: s.size, file: Some(f.clone()), confidence: Confidence::Name },
                    (None, Some(f)) => Found { path: s.path.clone(), size: s.size, file: Some(f.clone()), confidence: Confidence::Size },
                    (None, None) => continue,
                }
            } else {
                if s.modified > exp.started_at.saturating_add(WINDOW_SECS) {
                    continue;
                }
                Found { path: s.path.clone(), size: s.size, file: None, confidence: Confidence::Guess }
            };
            let better = match &best {
                None => true,
                Some((b, m)) => (found.confidence, s.modified, &found.path) > (b.confidence, *m, &b.path),
            };
            if better {
                best = Some((found, s.modified));
            }
        }
        self.sizes = sizes;
        best.map(|(f, _)| f)
    }

    /// Looks at the real folders once.
    pub fn poll(&mut self) -> Option<Found> {
        let listing = scan(&self.folders);
        self.step(&listing)
    }
}

fn secs(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// The regular files directly in these folders (not inside subfolders, links not followed).
pub fn scan(folders: &[PathBuf]) -> Vec<Seen> {
    let mut out = Vec::new();
    for dir in folders {
        let Ok(rd) = fs::read_dir(dir) else { continue };
        for e in rd.flatten() {
            let Ok(m) = e.path().symlink_metadata() else { continue };
            if m.is_file() {
                out.push(Seen { path: e.path(), size: m.len(), modified: m.modified().map(secs).unwrap_or(0) });
            }
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

/// Looks once a second until a match turns up, `stop` says to give up (the player left the page, or Turbine is
/// closing), or the 30 minutes are over. `now` and `sleep` are given so tests can run it without waiting.
pub fn wait(watcher: &mut Watcher, now: &dyn Fn() -> u64, sleep: &dyn Fn(Duration), stop: &dyn Fn() -> bool) -> Option<Found> {
    loop {
        if stop() || watcher.expectation.expired_at(now()) {
            return None;
        }
        if let Some(f) = watcher.poll() {
            return Some(f);
        }
        sleep(Duration::from_secs(POLL_SECS));
    }
}

/// The Downloads folder from the text of ~/.config/user-dirs.dirs (XDG_DOWNLOAD_DIR="$HOME/Downloads"), when set.
pub fn parse_user_dirs(text: &str, home: &Path) -> Option<PathBuf> {
    for line in text.lines() {
        let line = line.trim();
        let Some(value) = line.strip_prefix("XDG_DOWNLOAD_DIR=") else { continue };
        let value = value.trim().trim_matches('"');
        let path = if let Some(rest) = value.strip_prefix("$HOME") {
            home.join(rest.trim_start_matches('/'))
        } else if value.starts_with('/') {
            PathBuf::from(value)
        } else {
            continue;
        };
        if path == home {
            return None; // "no Downloads folder" in this file's terms
        }
        return Some(path);
    }
    None
}

/// The folders to watch: ~/Downloads, and the desktop's Downloads folder when it is somewhere else.
pub fn default_folders(home: &Path) -> Vec<PathBuf> {
    let mut out = vec![home.join("Downloads")];
    if let Ok(text) = fs::read_to_string(home.join(".config/user-dirs.dirs")) {
        if let Some(p) = parse_user_dirs(&text, home) {
            if !out.contains(&p) {
                out.push(p);
            }
        }
    }
    out
}
