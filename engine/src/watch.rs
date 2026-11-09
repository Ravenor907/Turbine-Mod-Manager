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
use std::time::Duration;

/// How long after opening the page a download still counts (seconds).
pub const WINDOW_SECS: u64 = 30 * 60;
/// How often `wait` looks (seconds).
pub const POLL_SECS: u64 = 1;
/// Endings of the files browsers write while a download is still going.
pub const PARTIAL_SUFFIXES: &[&str] = &[".part", ".crdownload", ".download", ".tmp"];
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
    pub fn knows_files(&self) -> bool {
        self.files.iter().any(|f| !f.file_name.is_empty() || f.size_bytes > 0)
    }

    /// Is a finished file in Downloads (its name and size) this mod's download? By name first, then by size; with
    /// no file list, any archive is a guess. None when the file list says it isn't one of the mod's files.
    pub fn match_file(&self, name: &str, size: u64) -> Option<(Option<ExpectedFile>, Confidence)> {
        if !self.knows_files() {
            return Some((None, Confidence::Guess));
        }
        if let Some(f) = self.files.iter().find(|f| same_download_name(name, &f.file_name)) {
            return Some((Some(f.clone()), Confidence::Name));
        }
        self.files.iter().find(|f| same_size(size, f.size_bytes)).map(|f| (Some(f.clone()), Confidence::Size))
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
            if !exp.knows_files() && s.modified > exp.started_at.saturating_add(WINDOW_SECS) {
                continue;
            }
            let Some((file, confidence)) = exp.match_file(&name, s.size) else { continue };
            let found = Found { path: s.path.clone(), size: s.size, file, confidence };
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

/// When a file counts as arrived: the later of its modification time and its status change time (seconds since
/// 1970). Some browsers give a finished download the server's own date, which can be years old; the status change
/// time is still the moment the file was written or renamed into Downloads, which the player's system sets.
pub fn arrived_secs(mtime: i64, ctime: i64) -> u64 {
    mtime.max(ctime).max(0) as u64
}

/// `arrived_secs` for a file's metadata.
pub fn arrived(m: &fs::Metadata) -> u64 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        arrived_secs(m.mtime(), m.ctime())
    }
    #[cfg(not(unix))]
    {
        m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0)
    }
}

/// The regular files directly in these folders (not inside subfolders, links not followed). `modified` is when
/// each one arrived (see `arrived`).
pub fn scan(folders: &[PathBuf]) -> Vec<Seen> {
    let mut out = Vec::new();
    for dir in folders {
        let Ok(rd) = fs::read_dir(dir) else { continue };
        for e in rd.flatten() {
            let Ok(m) = e.path().symlink_metadata() else { continue };
            if m.is_file() {
                out.push(Seen { path: e.path(), size: m.len(), modified: arrived(&m) });
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
    let p = downloads_folder(home);
    if !out.contains(&p) {
        out.push(p);
    }
    out
}

/// 0.61 (integration I14): the desktop's Downloads folder as the sandbox sees it, for the watcher and Turbine's own
/// browser alike: $XDG_DOWNLOAD_DIR when set, else user-dirs.dirs in $XDG_CONFIG_HOME (where Flatpak puts the
/// host's copy) or ~/.config, else ~/Downloads.
pub fn downloads_folder(home: &Path) -> PathBuf {
    let env = std::env::var("XDG_DOWNLOAD_DIR").ok();
    let mut configs: Vec<PathBuf> = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).into_iter().collect();
    configs.push(home.join(".config"));
    downloads_folder_from(home, env.as_deref(), &configs)
}

/// The same, from the given environment value and config folders (first one with a Downloads line wins).
pub fn downloads_folder_from(home: &Path, env: Option<&str>, configs: &[PathBuf]) -> PathBuf {
    if let Some(p) = env.map(str::trim).filter(|p| p.starts_with('/')).map(PathBuf::from) {
        if p != home {
            return p;
        }
    }
    for dir in configs.iter().filter(|d| d.is_absolute()) {
        if let Some(p) = fs::read_to_string(dir.join("user-dirs.dirs")).ok().and_then(|t| parse_user_dirs(&t, home)) {
            return p;
        }
    }
    home.join("Downloads")
}

// ------------------------------------------------------------------------------------------------ always-on catching
//
// While Turbine runs it watches Downloads for every new mod archive, however the player got to the download (a Nexus
// page opened from Turbine, from the browser's own bookmarks, a link someone sent). A `Catcher` looks once a second
// and reports, for each file newer than the last time Turbine looked:
// - a browser's unfinished download (for "Downloading <name>" on the status line);
// - why a file is left alone (not a zip, 7z or rar; still being written; empty; the player said it isn't a mod);
// - a finished archive, once its size has stayed the same between looks and the browser's unfinished file is gone.
// Deciding which game a finished archive goes into is `decide`. Files are only ever read, never moved or deleted.

/// The folder in Downloads the browser helper sorts Nexus downloads into, one subfolder per game.
pub const GAME_MODS: &str = "Game Mods";
/// The subfolder for a download whose game Turbine doesn't know.
pub const OTHER_FOLDER: &str = "Other";

/// The folder name for a game inside Game Mods: its name without characters browsers or file systems refuse
/// (< > : " / \\ | ? * and control characters), spaces kept, at most 80 characters, never "." or "..".
pub fn game_folder_name(name: &str) -> String {
    let cleaned: String = name.chars().filter(|c| !c.is_control() && !"<>:\"/\\|?*".contains(*c)).collect();
    let words = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let t: String = words.trim_matches('.').chars().take(80).collect();
    let t = t.trim().to_string();
    if t.is_empty() { OTHER_FOLDER.to_string() } else { t }
}

/// The folders to watch for these Downloads folders: each one, its Game Mods folder, and every subfolder of that
/// (links not followed). Asked again now and then, so a new game's folder is picked up.
pub fn with_game_mods(downloads: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for d in downloads {
        out.push(d.clone());
        let gm = d.join(GAME_MODS);
        out.push(gm.clone());
        if let Ok(rd) = fs::read_dir(&gm) {
            let mut subs: Vec<PathBuf> = rd.flatten().filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false)).map(|e| e.path()).collect();
            subs.sort();
            out.extend(subs);
        }
    }
    out
}

/// The game folder a file sits in, when it is directly inside Downloads/Game Mods/<game folder>/ ("Other" doesn't
/// count). The answer is the folder's name, to compare with `game_folder_name` of each game.
pub fn game_mods_folder_of(path: &Path) -> Option<String> {
    let dir = path.parent()?;
    let gm = dir.parent()?;
    if gm.file_name()? != GAME_MODS {
        return None;
    }
    let name = dir.file_name()?.to_string_lossy().into_owned();
    (name != OTHER_FOLDER && !name.is_empty()).then_some(name)
}

/// On the very first run, archives this many seconds old still count (and Turbine asks before adding any of them).
pub const FIRST_RUN_SECS: u64 = 24 * 60 * 60;
/// The most ignored files remembered (the oldest are forgotten first).
pub const MAX_IGNORED: usize = 500;

/// Why a file in Downloads was left alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Skip {
    /// Not a zip, 7z or rar file.
    NotArchive,
    /// The browser is still writing it (its .part, .crdownload or .download file is there).
    Unfinished,
    /// Its size changed since the last look.
    Growing,
    /// Zero bytes.
    Empty,
    /// The player said it isn't a mod.
    Ignored,
}

impl Skip {
    /// Plain words for the activity log.
    pub fn words(self) -> &'static str {
        match self {
            Skip::NotArchive => "not a zip, 7z or rar file",
            Skip::Unfinished => "the browser is still downloading it",
            Skip::Growing => "still being written",
            Skip::Empty => "it is empty",
            Skip::Ignored => "you said it isn't a mod",
        }
    }
}

/// A finished archive that arrived in Downloads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arrival {
    pub path: PathBuf,
    pub size: u64,
    /// When it arrived (seconds since 1970, see `arrived`).
    pub arrived: u64,
    /// It arrived before this Turbine started (found by the catch-up look).
    pub catch_up: bool,
}

/// What one look found. Skips are said once per file (again only when the reason changes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Skipped { path: PathBuf, why: Skip },
    /// The newest unfinished download: its final name (without the browser's ending) and its size so far.
    Downloading { name: String, size: u64 },
    Ready(Arrival),
}

/// The key a file is remembered by in the ignore list: its full path and size (a new download with the same name
/// but other contents is asked about again).
pub fn ignore_key(path: &Path, size: u64) -> String {
    format!("{size}|{}", path.display())
}

/// Watches folders for every new finished archive.
#[derive(Clone, Debug)]
pub struct Catcher {
    pub folders: Vec<PathBuf>,
    /// Files that arrived before this are left alone.
    pub since: u64,
    /// When this Turbine started: files that arrived before it are catch-up files.
    pub started: u64,
    /// How many looks in a row a file's size must stay the same before it counts as finished (at least 1).
    pub stable_looks: u32,
    /// Size at the last look, and how many looks in a row it stayed that size.
    sizes: HashMap<PathBuf, (u64, u32)>,
    /// What was last said about each file (a skip, or "ready" with its size and arrival).
    told: HashMap<PathBuf, Told>,
    ignored: std::collections::HashSet<String>,
    /// Files that count however old they are (ones Turbine was still asking about when it last closed).
    again: std::collections::HashSet<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Told {
    Skip(Skip),
    Ready(u64, u64),
}

impl Catcher {
    pub fn new(folders: Vec<PathBuf>, since: u64, started: u64) -> Catcher {
        Catcher {
            folders,
            since,
            started,
            stable_looks: 1,
            sizes: HashMap::new(),
            told: HashMap::new(),
            ignored: Default::default(),
            again: Default::default(),
        }
    }

    /// Report this file again even though it arrived before `since` (an unanswered question from the last run).
    pub fn ask_again(&mut self, path: &Path) {
        self.again.insert(path.to_path_buf());
    }

    /// Never report this file as ready again (the player said it isn't a mod). Takes `ignore_key`s.
    pub fn ignore(&mut self, key: &str) {
        self.ignored.insert(key.to_string());
    }

    pub fn is_ignored(&self, path: &Path, size: u64) -> bool {
        self.ignored.contains(&ignore_key(path, size))
    }

    fn tell(&mut self, path: &Path, what: Told, out: &mut Vec<Event>) {
        if self.told.get(path) == Some(&what) {
            return;
        }
        self.told.insert(path.to_path_buf(), what.clone());
        match what {
            Told::Skip(why) => out.push(Event::Skipped { path: path.to_path_buf(), why }),
            Told::Ready(..) => {}
        }
    }

    /// One look at the folders' listing.
    pub fn step(&mut self, listing: &[Seen]) -> Vec<Event> {
        let mut out = Vec::new();
        let mut busy: Vec<PathBuf> = Vec::new();
        let mut newest_partial: Option<(u64, String, u64)> = None;
        for s in listing {
            let name = file_name(&s.path);
            let lc = name.to_ascii_lowercase();
            if let Some(sfx) = PARTIAL_SUFFIXES.iter().find(|x| lc.ends_with(*x)) {
                let base = &name[..name.len() - sfx.len()];
                busy.push(s.path.with_file_name(base));
                if s.modified + SLACK_SECS >= self.since && newest_partial.as_ref().map(|b| s.modified >= b.0).unwrap_or(true) {
                    // Chromium names a download "Unconfirmed 123456.crdownload" until it knows the name.
                    let shown = if base.starts_with("Unconfirmed ") { "a file".to_string() } else { base.to_string() };
                    newest_partial = Some((s.modified, shown, s.size));
                }
            }
        }
        let mut sizes = HashMap::new();
        for s in listing {
            if s.modified < self.since && !self.again.contains(&s.path) {
                continue;
            }
            let name = file_name(&s.path);
            let lc = name.to_ascii_lowercase();
            if PARTIAL_SUFFIXES.iter().any(|x| lc.ends_with(x)) {
                continue;
            }
            if !archives::is_archive_name(&name) {
                self.tell(&s.path, Told::Skip(Skip::NotArchive), &mut out);
                continue;
            }
            let looks = match self.sizes.get(&s.path) {
                Some((size, n)) if *size == s.size => n + 1,
                _ => 0,
            };
            sizes.insert(s.path.clone(), (s.size, looks));
            if busy.contains(&s.path) {
                self.tell(&s.path, Told::Skip(Skip::Unfinished), &mut out);
                continue;
            }
            if s.size == 0 {
                self.tell(&s.path, Told::Skip(Skip::Empty), &mut out);
                continue;
            }
            if looks < self.stable_looks.max(1) {
                if self.sizes.contains_key(&s.path) {
                    self.tell(&s.path, Told::Skip(Skip::Growing), &mut out);
                }
                continue;
            }
            if self.is_ignored(&s.path, s.size) {
                self.tell(&s.path, Told::Skip(Skip::Ignored), &mut out);
                continue;
            }
            let ready = Told::Ready(s.size, s.modified);
            if self.told.get(&s.path) != Some(&ready) {
                self.told.insert(s.path.clone(), ready);
                out.push(Event::Ready(Arrival { path: s.path.clone(), size: s.size, arrived: s.modified, catch_up: s.modified < self.started }));
            }
        }
        // Files that went away are forgotten, so a new download with the same name counts again.
        self.told.retain(|p, _| listing.iter().any(|s| &s.path == p));
        self.sizes = sizes;
        if let Some((_, name, size)) = newest_partial {
            out.insert(0, Event::Downloading { name, size });
        }
        out
    }

    /// Looks at the real folders once.
    pub fn poll(&mut self) -> Vec<Event> {
        let listing = scan(&self.folders);
        self.step(&listing)
    }
}

/// What Turbine keeps between runs about catching downloads (in its own data folder).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CatchState {
    /// Everything that arrived before this was dealt with (seconds since 1970).
    pub last_seen: u64,
    /// Files the player said aren't mods (`ignore_key`s), oldest first.
    pub ignored: Vec<String>,
    /// Files Turbine was still asking about when it closed (full paths): asked again next time.
    pub pending: Vec<String>,
}

impl CatchState {
    pub fn parse(text: &str) -> Option<CatchState> {
        let j = crate::json::parse(text).ok()?;
        let last_seen = match j.get("lastSeen") {
            Some(crate::json::Json::Num(n)) if *n >= 0.0 => *n as u64,
            _ => return None,
        };
        let list = |key: &str| -> Vec<String> {
            j.get(key).map(crate::json::Json::as_array).unwrap_or(&[]).iter().filter_map(|x| x.as_str().map(str::to_string)).collect()
        };
        Some(CatchState { last_seen, ignored: list("ignored"), pending: list("pending") })
    }

    pub fn to_json(&self) -> String {
        let list = |v: &[String]| v.iter().map(|k| crate::json::quote(k)).collect::<Vec<_>>().join(",");
        format!("{{\"lastSeen\":{},\"ignored\":[{}],\"pending\":[{}]}}\n", self.last_seen, list(&self.ignored), list(&self.pending))
    }

    /// Remembers a file the player said isn't a mod (forgetting the oldest past `MAX_IGNORED`).
    pub fn ignore(&mut self, key: &str) {
        self.ignored.retain(|k| k != key);
        self.ignored.push(key.to_string());
        while self.ignored.len() > MAX_IGNORED {
            self.ignored.remove(0);
        }
    }

    /// Reads the saved state; None when there is none yet (the first run) or it can't be read.
    pub fn load(path: &Path) -> Option<CatchState> {
        CatchState::parse(&fs::read_to_string(path).ok()?)
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.new");
        fs::write(&tmp, self.to_json())?;
        fs::rename(&tmp, path)
    }
}

/// Where the catch-up look starts, and whether this is the first run (then Turbine asks before adding anything
/// from it). With a saved state, from when Turbine last looked (a little before, for coarse file times).
pub fn catch_up_since(state: Option<&CatchState>, now: u64) -> (u64, bool) {
    match state {
        Some(s) if s.last_seen > 0 => (s.last_seen.min(now).saturating_sub(SLACK_SECS), false),
        _ => (now.saturating_sub(FIRST_RUN_SECS), true),
    }
}

/// How Turbine knew which game a download is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Basis {
    /// It was saved into Downloads/Game Mods/<game>/ (by the browser helper).
    Folder,
    /// The browser helper saw the Nexus page it came from.
    Helper,
    /// Nexus knows the file by its MD5.
    Md5,
    /// A Nexus mod page opened from Turbine, and the file matched it (`Confidence`).
    Page(Confidence),
    /// The game the player last had open in Turbine.
    LastGame,
    /// Nothing to go on.
    Nothing,
}

/// Which game a finished download goes into, and whether Turbine is sure enough to add it by itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decision {
    /// The best game's ID (Turbine's), when there is one.
    pub game: Option<String>,
    pub basis: Basis,
    /// Add without asking.
    pub sure: bool,
}

/// Decides, in this order: the browser helper's page, Nexus's MD5 lookup, a matching Nexus page opened from
/// Turbine, then the game last open in Turbine. The first three are sure (a page match only by name or size; a
/// guess is not), the last never is. On the first run's catch-up, Turbine always asks.
pub fn decide(helper: Option<&str>, md5: Option<&str>, page: Option<(&str, Confidence)>, last_game: Option<&str>, first_run_catch_up: bool) -> Decision {
    decide_all(None, helper, md5, page, last_game, first_run_catch_up)
}

/// `decide` with the Game Mods folder first: a file in a game's folder there is always a sure match, on the first
/// run's catch-up too (only the browser helper puts files there).
pub fn decide_all(folder: Option<&str>, helper: Option<&str>, md5: Option<&str>, page: Option<(&str, Confidence)>, last_game: Option<&str>, first_run_catch_up: bool) -> Decision {
    if let Some(g) = folder {
        return Decision { game: Some(g.to_string()), basis: Basis::Folder, sure: true };
    }
    let (game, basis, sure) = if let Some(g) = helper {
        (Some(g), Basis::Helper, true)
    } else if let Some(g) = md5 {
        (Some(g), Basis::Md5, true)
    } else if let Some((g, c)) = page {
        (Some(g), Basis::Page(c), c != Confidence::Guess)
    } else if let Some(g) = last_game {
        (Some(g), Basis::LastGame, false)
    } else {
        (None, Basis::Nothing, false)
    };
    Decision { game: game.map(str::to_string), basis, sure: sure && !first_run_catch_up }
}

impl Basis {
    /// Plain words for the activity log.
    pub fn words(self) -> &'static str {
        match self {
            Basis::Folder => "it was saved in that game's Game Mods folder",
            Basis::Helper => "the browser helper saw its Nexus page",
            Basis::Md5 => "Nexus Mods knows the file",
            Basis::Page(Confidence::Name) => "it matches a Nexus page opened from Turbine by name",
            Basis::Page(Confidence::Size) => "it matches a Nexus page opened from Turbine by size",
            Basis::Page(Confidence::Guess) => "a Nexus page was opened from Turbine, but its file list isn't known",
            Basis::LastGame => "it is the game last open in Turbine",
            Basis::Nothing => "nothing says which game it is for",
        }
    }
}
