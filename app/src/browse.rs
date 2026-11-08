//! Nexus inside Turbine (NexusBrowser.qml): the decisions about what the page asks for, kept apart from Qt so they
//! are tested on their own (rustc --test app/src/browse.rs --extern turbine_engine=<the engine's rlib>).
//!
//! - `plan`: a download the page started: refused unless the player pressed something in the page in the last 60
//!   seconds, and it is https, a zip, 7z or rar file, under 64 GB, with room for it (twice its size, or a floor when
//!   the size isn't known, counting what running downloads still need). It goes into Downloads/Game Mods/<Game>/ as a
//!   sure match only when it comes from Nexus's own file hosts (`sure_source`); anything else goes into Game Mods/Other
//!   and Turbine asks which game it is for. `keep_going` stops a running download past 64 GB or near a full disk.
//! - `nxm_permit` and `nxm_from_page`: a Mod Manager Download link. QML's navigation guard gives a permit only to a
//!   navigation of the main frame while it is on nexusmods.com; webglue.cpp's scheme handler checks the request's
//!   initiator is nexusmods.com and hands it here, where the link must have its key, expiry and account number, a
//!   permit, a press of the player's in the last 30 seconds, and must not have been used before.
//! - `lock_instance`: one Turbine at a time holds the lock; only that one removes partial downloads and opens the
//!   browser's profile.
//! - `log_url`: page addresses go into the activity log as host and path only (signed links and codes stay out).
//! - `start_url`, `zoom_default`, `same_page`, `user_matches`, the pill's words.
//! - the build's stand-in page: only with --selftest-data and --browser-test=<url> does its 127.0.0.1 host count as
//!   Nexus (and plain http from it as allowed).
//!
//! Turbine never clicks, fills in or changes anything on Nexus's pages: every click is the player's own press.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// The largest download the browser takes.
pub const MAX_BYTES: u64 = 64 << 30;
/// Said when a download isn't a mod archive.
pub const ONLY_ARCHIVES: &str = "Turbine only takes zip, 7z or rar mods";
/// The folder inside Downloads, and the one for files whose game isn't known (the same as the watcher's).
pub const GAME_MODS: &str = "Game Mods";
pub const OTHER: &str = "Other";
/// A Mod Manager Download link counts only within this long of the player's last press in the page.
pub const NXM_PRESS: Duration = Duration::from_secs(30);
/// The navigation guard's permit for a link lasts this long (the scheme handler sees it a moment later).
pub const NXM_PERMIT: Duration = Duration::from_secs(10);
/// A download counts as the player's only within this long of his last press in the page (Nexus waits 5 seconds).
pub const DL_PRESS: Duration = Duration::from_secs(60);
/// Free space a download of unknown size needs before it starts, and what a running download leaves free at least.
pub const FREE_FLOOR: u64 = 1 << 30;
pub const FREE_STOP: u64 = 512 << 20;
/// The longest file name most file systems take, in bytes, and how much of it a name's stem may use (room is left for
/// " (999)" and ".download").
pub const NAME_MAX: usize = 255;
pub const STEM_MAX: usize = 180;

// ------------------------------------------------------------------------------------------------ presses

/// How long after the view left Nexus (and came back) a Mod Manager Download link still asks before anything is
/// turned on.
pub const CAME_BACK: Duration = Duration::from_secs(120);

/// The player's presses in the page, tied to the page they were made on. A press counts for a download (60 seconds)
/// and for at most one Mod Manager Download link (30 seconds). The moment the main frame goes to a page that isn't on
/// Nexus (a link, a redirect, a pop-up opened in the view), the press is gone: a page reached that way can't bounce
/// back to Nexus and spend it. A link that comes soon after such a trip asks before anything is turned on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Presses {
    at: Option<Instant>,
    nxm_used: bool,
    left_nexus: Option<Instant>,
}

impl Presses {
    /// A tap, click or button press in the page.
    pub fn press(&mut self, now: Instant) {
        self.at = Some(now);
        self.nxm_used = false;
    }
    /// The main frame went to (or loaded) a page that isn't on Nexus.
    pub fn left_nexus(&mut self, now: Instant) {
        self.at = None;
        self.nxm_used = false;
        self.left_nexus = Some(now);
    }
    /// The press a download may count on.
    pub fn for_download(&self) -> Option<Instant> {
        self.at
    }
    /// One press, one Mod Manager Download link: Ok uses it up.
    pub fn take_for_nxm(&mut self, now: Instant) -> Result<(), String> {
        if !within(self.at, now, NXM_PRESS) {
            return Err("came with no press of yours in the last 30 seconds".into());
        }
        if self.nxm_used {
            return Err("came after the one link your last press allows".into());
        }
        self.nxm_used = true;
        Ok(())
    }
    /// Did the view leave Nexus a short while ago (and come back)?
    pub fn came_back(&self, now: Instant) -> bool {
        within(self.left_nexus, now, CAME_BACK)
    }
}

static PRESSES: Mutex<Presses> = Mutex::new(Presses { at: None, nxm_used: false, left_nexus: None });

fn presses() -> std::sync::MutexGuard<'static, Presses> {
    PRESSES.lock().unwrap_or_else(|e| e.into_inner())
}

/// The player pressed A, RT or R3 in the page, or tapped or clicked it (never a scroll or a drag).
pub fn note_press(now: Instant) {
    presses().press(now);
}

/// The main frame is going to (or loaded) this address: off Nexus, the press is gone. True when it left Nexus.
pub fn note_main_frame(url: &str, now: Instant) -> bool {
    let u = url.trim();
    if u.is_empty() || u.to_ascii_lowercase().starts_with("about:") || is_nexus_page(u) {
        return false;
    }
    presses().left_nexus(now);
    true
}

/// When the player last pressed something in the page (for a download).
pub fn last_press() -> Option<Instant> {
    presses().for_download()
}

/// Was `press` within `within` before `now` (and not after it)?
pub fn within(press: Option<Instant>, now: Instant, within: Duration) -> bool {
    press.is_some_and(|at| at <= now && now.duration_since(at) <= within)
}

/// What the page asked to download.
#[derive(Clone, Debug, Default)]
pub struct Request {
    pub url: String,
    /// The name the server or page gave (may be empty).
    pub suggested: String,
    /// Bytes, or -1 when unknown.
    pub total: i64,
}

/// Where a download goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub dir: PathBuf,
    pub name: String,
    /// The game's folder name inside Game Mods ("Other" when it isn't known or the download isn't a sure match).
    pub folder: String,
    /// From Nexus's own file hosts after a press: added to the page's game without asking.
    pub sure: bool,
}

impl Plan {
    pub fn path(&self) -> PathBuf {
        self.dir.join(&self.name)
    }
}

/// The host of an address (lower case, no port), and whether it is https.
pub fn host_of(url: &str) -> Option<(String, bool)> {
    let (scheme, rest) = url.split_once("://")?;
    let https = scheme.eq_ignore_ascii_case("https");
    if !https && !scheme.eq_ignore_ascii_case("http") {
        return None;
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if authority.contains('@') {
        return None;
    }
    let host = authority.rsplit_once(':').map(|(h, p)| if p.chars().all(|c| c.is_ascii_digit()) { h } else { authority }).unwrap_or(authority);
    let host = host.to_ascii_lowercase();
    if host.is_empty() || !host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-') {
        return None;
    }
    Some((host, https))
}

/// Is this host Nexus's own (nexusmods.com or anything under it)?
pub fn is_nexus_host(host: &str) -> bool {
    let h = host.to_ascii_lowercase();
    h == "nexusmods.com" || h.ends_with(".nexusmods.com")
}

/// Is this host one Nexus serves its files from (nexus-cdn.com, or its own site)?
pub fn is_nexus_file_host(host: &str) -> bool {
    let h = host.to_ascii_lowercase();
    is_nexus_host(&h) || h == "nexus-cdn.com" || h.ends_with(".nexus-cdn.com")
}

/// An address for the activity log: scheme, host and path only. The query and the fragment (Nexus's signed file
/// links carry md5, expires and user_id there, and sign-in pages their codes) and any user name never go in.
pub fn log_url(url: &str) -> String {
    let url = url.trim();
    let Some((scheme, rest)) = url.split_once("://") else {
        // data:, blob:, about: and the like: the scheme only.
        return url.split(':').next().map(|s| format!("{}:", s.chars().take(16).collect::<String>())).unwrap_or_default();
    };
    let cut = rest.find(['?', '#', ';']).unwrap_or(rest.len());
    let rest = &rest[..cut];
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let host = authority.rsplit('@').next().unwrap_or("");
    let path: String = path.chars().take(160).collect();
    format!("{}://{}{}", scheme.to_ascii_lowercase(), host.to_ascii_lowercase(), path)
}

/// The same page, with its query's parts in any order (and the fragment ignored)?
pub fn same_page(a: &str, b: &str) -> bool {
    fn parts(u: &str) -> Option<(String, Vec<String>)> {
        let u = u.split('#').next().unwrap_or("");
        let (base, query) = u.split_once('?').unwrap_or((u, ""));
        let (scheme, rest) = base.split_once("://")?;
        let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
        let mut q: Vec<String> = query.split('&').filter(|p| !p.is_empty()).map(str::to_string).collect();
        q.sort();
        Some((format!("{}://{}/{}", scheme.to_ascii_lowercase(), host.to_ascii_lowercase(), path.trim_end_matches('/')), q))
    }
    match (parts(a), parts(b)) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

/// A Mod Manager Download link is for one Nexus account (its user_id): refused when the account Turbine is signed in
/// with (its key) is another. `account` 0 means not known (no key): nothing to compare.
pub fn user_matches(link_user: Option<u64>, account: u64) -> Result<(), String> {
    match link_user {
        Some(u) if account != 0 && u != account => Err("This Mod Manager Download link is for another Nexus account than the key in Turbine. Sign in on the page with the same account.".into()),
        _ => Ok(()),
    }
}

// ------------------------------------------------------------------------------------------------ the stand-in page

static TEST_ORIGIN: Mutex<Option<String>> = Mutex::new(None);

/// The build's stand-in Nexus page: `origin` is "http://127.0.0.1:<port>". Only main.rs sets it, and only when both
/// --selftest-data and --browser-test are given.
pub fn set_test_origin(origin: &str) {
    *TEST_ORIGIN.lock().unwrap_or_else(|e| e.into_inner()) = Some(origin.trim_end_matches('/').to_ascii_lowercase());
}

fn test_origin() -> Option<String> {
    TEST_ORIGIN.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Is `url` on the stand-in page's origin (in the build's test run)?
pub fn is_test_url(url: &str) -> bool {
    match test_origin() {
        Some(o) => {
            let u = url.to_ascii_lowercase();
            u == o || u.starts_with(&format!("{o}/")) || u.starts_with(&format!("{o}?"))
        }
        None => false,
    }
}

/// Does this page count as Nexus (its own hosts, or the stand-in in the test run)?
pub fn is_nexus_page(url: &str) -> bool {
    is_test_url(url) || host_of(url).is_some_and(|(h, https)| https && is_nexus_host(&h))
}

/// The game's Nexus domain and mod and file ids of a page (www.nexusmods.com/<domain>/mods/<id>?...file_id=<n>, or
/// the stand-in's same path).
pub fn page_ids(url: &str) -> Option<(String, u64, Option<u64>)> {
    if !is_nexus_page(url) {
        return None;
    }
    let rest = url.split_once("://")?.1;
    let path_query = rest.split_once('/').map(|(_, p)| p).unwrap_or("");
    let (path, query) = path_query.split_once('?').unwrap_or((path_query, ""));
    let query = query.split('#').next().unwrap_or("");
    let mut parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    if parts.first() == Some(&"games") {
        parts.remove(0);
    }
    let domain = parts.first()?.to_ascii_lowercase();
    if !domain.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') || domain.len() > 64 {
        return None;
    }
    let mod_id = match (parts.get(1), parts.get(2)) {
        (Some(&"mods"), Some(id)) => id.parse::<u64>().ok().filter(|n| *n > 0)?,
        _ => return None,
    };
    let file = query.split('&').find_map(|kv| kv.strip_prefix("file_id=")).and_then(|v| v.parse::<u64>().ok()).filter(|n| *n > 0);
    Some((domain, mod_id, file))
}

// ------------------------------------------------------------------------------------------------ downloads

/// A file name from what the page suggested (or the address's last part): no folders, no characters file systems or
/// browsers refuse, no leading dots. Long names are cut by bytes (file systems count bytes, and a CJK or Cyrillic
/// character takes 2 to 4): the stem to at most STEM_MAX bytes on a character boundary, the extension kept, so the
/// name plus " (999)" and ".download" stays within NAME_MAX.
pub fn clean_name(suggested: &str, url: &str) -> String {
    let from = if suggested.trim().is_empty() { last_part(url) } else { suggested.to_string() };
    let base = from.rsplit(['/', '\\']).next().unwrap_or("").to_string();
    let replaced: String = base.chars().map(|c| if c.is_control() || "<>:\"|?*".contains(c) { '_' } else { c }).collect();
    let t = replaced.trim_start_matches('.').trim();
    let (stem, ext) = split_ext(t);
    // An "extension" this long isn't one: the whole name is the stem.
    let (stem, ext) = if ext.len() > 16 { (t, "") } else { (stem, ext) };
    let stem = cut_bytes(stem, STEM_MAX).trim_end();
    if stem.is_empty() {
        return String::new();
    }
    format!("{stem}{ext}")
}

/// The longest start of `s` that is at most `max` bytes and ends on a character boundary.
fn cut_bytes(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// The last part of an address's path, with %xx decoded.
fn last_part(url: &str) -> String {
    let rest = url.split_once("://").map(|(_, r)| r).unwrap_or(url);
    let path = rest.split(['?', '#']).next().unwrap_or("");
    let last = path.rsplit('/').next().unwrap_or("");
    percent_decode(last)
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let hex = |c: u8| (c as char).to_digit(16);
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `name` in `dir`, or "name (2).ext", "name (3).ext" ... when one is there already (or about to be: `taken`).
pub fn unique_name(dir: &Path, name: &str, taken: &[PathBuf]) -> String {
    let free = |n: &str| {
        let p = dir.join(n);
        !p.exists() && !taken.contains(&p)
    };
    if free(name) {
        return name.to_string();
    }
    let (stem, ext) = split_ext(name);
    for i in 2..1000 {
        let n = format!("{stem} ({i}){ext}");
        if free(&n) {
            return n;
        }
    }
    format!("{stem} ({}){ext}", std::process::id())
}

/// "Mod 1.0.zip" as ("Mod 1.0", ".zip"); ".tar.gz" style endings aren't mod archives, so one ending is enough.
fn split_ext(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    }
}

/// A download is a sure match for the page's game only when it comes from Nexus's own file hosts (or the build's
/// stand-in) and the player pressed something in the page in the last 60 seconds.
pub fn sure_source(url: &str, press: Option<Instant>, now: Instant) -> bool {
    let host_ok = is_test_url(url) || host_of(url).is_some_and(|(h, https)| https && is_nexus_file_host(&h));
    host_ok && within(press, now, DL_PRESS)
}

/// Said when the page starts a download with no press of the player's.
pub const NO_PRESS: &str = "Turbine only takes a download you start yourself. Press the download button on the page";

/// Decides where a download goes, or why it is refused. `downloads`: the player's Downloads folder; `folder`: the
/// game's folder name inside Game Mods (None when no installed game fits the page); `free`: free bytes where it
/// goes, when known; `pending`: bytes running downloads still have to come; `taken`: downloads already on their way
/// (their names count as taken); `press`: the player's last press in the page.
#[allow(clippy::too_many_arguments)]
pub fn plan(req: &Request, downloads: &Path, folder: Option<&str>, free: Option<u64>, pending: u64, taken: &[PathBuf], press: Option<Instant>, now: Instant) -> Result<Plan, String> {
    if !within(press, now, DL_PRESS) {
        return Err(NO_PRESS.into());
    }
    let https = host_of(&req.url).is_some_and(|(_, s)| s);
    if !https && !is_test_url(&req.url) {
        return Err("Turbine only takes downloads from secure (https) addresses".into());
    }
    let name = clean_name(&req.suggested, &req.url);
    if name.is_empty() || !turbine_engine::archives::is_archive_name(&name) {
        return Err(ONLY_ARCHIVES.into());
    }
    if req.total > 0 && req.total as u64 > MAX_BYTES {
        return Err(format!("{name} is over 64 GB, more than Turbine takes"));
    }
    if let Some(f) = free {
        let need = if req.total > 0 { (req.total as u64).saturating_mul(2) } else { FREE_FLOOR }.saturating_add(pending);
        if f < need {
            return Err(format!("Not enough free space for {name}: it needs {} free", size_words(need)));
        }
    }
    let sure = sure_source(&req.url, press, now);
    let folder = folder.filter(|_| sure).map(turbine_engine::watch::game_folder_name).unwrap_or_else(|| OTHER.to_string());
    let dir = downloads.join(GAME_MODS).join(&folder);
    let name = unique_name(&dir, &name, taken);
    Ok(Plan { dir, name, folder, sure })
}

/// A running download, checked twice a second: stopped (Err: why) past 64 GB, or when the disk it goes to is nearly
/// full (the partial file then goes at once).
pub fn keep_going(got: i64, free: Option<u64>) -> Result<(), String> {
    if got > 0 && got as u64 > MAX_BYTES {
        return Err("it went over 64 GB, more than Turbine takes".into());
    }
    if let Some(f) = free {
        if f < FREE_STOP {
            return Err(format!("the disk is nearly full (less than {} free)", size_words(FREE_STOP)));
        }
    }
    Ok(())
}

/// The files a stopped download may have left: the part Qt writes while it runs, and the planned file itself (only
/// Turbine's browser writes to that name: it was free when the download was planned).
pub fn partials_of(path: &Path) -> Vec<PathBuf> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    if name.is_empty() || !turbine_engine::archives::is_archive_name(&name) {
        return Vec::new();
    }
    vec![path.with_file_name(format!("{name}.download")), path.with_file_name(format!("{name}.crdownload")), path.to_path_buf()]
}

/// Removes what a download stopped for good left behind (a full disk, too big). Returns the removed files.
pub fn remove_partial(path: &Path) -> Vec<PathBuf> {
    partials_of(path).into_iter().filter(|p| p.is_file() && std::fs::remove_file(p).is_ok()).collect()
}

pub fn size_words(b: u64) -> String {
    if b >= 1 << 30 {
        format!("{:.1} GB", b as f64 / (1u64 << 30) as f64)
    } else if b >= 1 << 20 {
        format!("{:.1} MB", b as f64 / (1u64 << 20) as f64)
    } else if b >= 1024 {
        format!("{} KB", b / 1024)
    } else {
        format!("{b} bytes")
    }
}

// ------------------------------------------------------------------------------------------------ Labs

/// The file in Turbine's data folder that says Labs' "Nexus page: draw without the GPU" is on (read before Qt
/// starts, so it can't live in Qt's settings).
pub const NO_GPU_FILE: &str = "browser-no-gpu";

pub fn no_gpu_wanted(data_dir: &Path) -> bool {
    data_dir.join(NO_GPU_FILE).is_file()
}

// ------------------------------------------------------------------------------------------------ one Turbine

extern "C" {
    // The C library's flock(2): a lock on an open file that the system drops when the process ends, however it ends.
    fn flock(fd: std::os::raw::c_int, operation: std::os::raw::c_int) -> std::os::raw::c_int;
}
const LOCK_EX: std::os::raw::c_int = 2;
const LOCK_NB: std::os::raw::c_int = 4;

/// The lock file in Turbine's data folder.
pub const LOCK_FILE: &str = "turbine.lock";

/// Held for as long as this Turbine runs (the file stays open).
#[derive(Debug)]
pub struct InstanceLock {
    _file: std::fs::File,
}

/// What locking found.
#[derive(Debug)]
pub enum Lock {
    /// This is the only Turbine: it may remove partial downloads and open the browser's profile.
    Mine(InstanceLock),
    /// Another Turbine holds it.
    Taken,
    /// It couldn't be made or locked (why): it isn't known whether another Turbine runs.
    Unknown(String),
}

impl Lock {
    pub fn mine(&self) -> bool {
        matches!(self, Lock::Mine(_))
    }
}

/// Takes the one-Turbine lock (never waits).
pub fn lock_instance(path: &Path) -> Lock {
    use std::os::unix::fs::OpenOptionsExt;
    use std::os::unix::io::AsRawFd;
    if let Some(dir) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(dir) {
            return Lock::Unknown(format!("its folder couldn't be made ({e})"));
        }
    }
    let file = match std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).mode(0o600).open(path) {
        Ok(f) => f,
        Err(e) => return Lock::Unknown(format!("the lock file couldn't be opened ({e})")),
    };
    // SAFETY: flock on a file descriptor this function owns for as long as the call lasts.
    if unsafe { flock(file.as_raw_fd(), LOCK_EX | LOCK_NB) } == 0 {
        return Lock::Mine(InstanceLock { _file: file });
    }
    let e = std::io::Error::last_os_error();
    if e.kind() == std::io::ErrorKind::WouldBlock {
        Lock::Taken
    } else {
        Lock::Unknown(format!("the lock didn't work ({e})"))
    }
}

// ------------------------------------------------------------------------------------------------ leftovers

/// Turbine's browser writes a download as "<name>.download" until it is done. A run that ended mid-download leaves
/// one behind, and Qt keeps no history to resume it: at the next start each one in Downloads/Game Mods/<folder>/ is
/// removed (only those: a mod archive's name with ".download" after it), and only by the Turbine that holds the
/// one-Turbine lock (another one running may be writing them right now). Returns the removed files.
pub fn remove_partials(lock: &Lock, downloads: &Path) -> Vec<PathBuf> {
    let mut gone = Vec::new();
    if !lock.mine() {
        return gone;
    }
    let Ok(rd) = std::fs::read_dir(downloads.join(GAME_MODS)) else { return gone };
    for dir in rd.flatten() {
        if !dir.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let Ok(files) = std::fs::read_dir(dir.path()) else { continue };
        for f in files.flatten() {
            let name = f.file_name().to_string_lossy().into_owned();
            let Some(base) = name.strip_suffix(".download") else { continue };
            if !turbine_engine::archives::is_archive_name(base) || !f.file_type().map(|t| t.is_file()).unwrap_or(false) {
                continue;
            }
            if std::fs::remove_file(f.path()).is_ok() {
                gone.push(f.path());
            }
        }
    }
    gone
}

// ------------------------------------------------------------------------------------------------ nxm links

/// Permits from the navigation guard (a main-frame navigation while the page is on Nexus), and every link used.
static PERMITS: Mutex<Vec<(String, Instant)>> = Mutex::new(Vec::new());
static USED: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// What happened to an nxm link from the page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NxmFromPage {
    /// Checked and queued; the redacted link for the log.
    Queued(String),
    /// Not taken; why, for the log.
    Refused(String),
}

/// The navigation guard (QML) saw the page navigate to an nxm link: a permit only when it is the main frame and the
/// page it leaves is on Nexus (`page_url`). Err: why it is refused (the navigation is then stopped there).
pub fn nxm_permit(url: &str, main_frame: bool, page_url: &str, now: Instant) -> Result<(), String> {
    if !main_frame {
        return Err("it came from a frame inside the page (an ad or another site), not the page itself".into());
    }
    if !is_nexus_page(page_url) {
        return Err(format!("the page isn't on nexusmods.com ({})", host_of(page_url).map(|(h, _)| h).unwrap_or_else(|| "no web address".into())));
    }
    let mut p = PERMITS.lock().unwrap_or_else(|e| e.into_inner());
    p.retain(|(_, at)| now.saturating_duration_since(*at) < NXM_PERMIT);
    p.push((key_of(url), now));
    if p.len() > 20 {
        p.remove(0);
    }
    Ok(())
}

/// Is a request's initiator (its origin, "https://www.nexusmods.com") Nexus's own site, or the build's stand-in?
/// webglue.cpp checks the same before anything reaches Rust.
pub fn initiator_ok(initiator: &str) -> bool {
    let i = initiator.trim().trim_end_matches('/');
    is_test_url(i) || host_of(i).is_some_and(|(h, https)| https && is_nexus_host(&h))
}

/// One form for a link, whatever the order of its query's parts (the used-link list and the permits compare this).
pub fn nxm_key(link: &turbine_engine::nxm::NxmLink) -> String {
    format!(
        "{}/{}/{}/{}/{}/{}",
        link.domain,
        link.mod_id,
        link.file_id,
        link.key.as_deref().unwrap_or(""),
        link.expires.unwrap_or(0),
        link.user_id.unwrap_or(0)
    )
}

fn key_of(url: &str) -> String {
    turbine_engine::nxm::parse(url.trim()).map(|l| nxm_key(&l)).unwrap_or_else(|_| url.trim().to_string())
}

/// A Mod Manager Download link from the page, after webglue.cpp's scheme handler checked its initiator. Queued
/// (`push`, single::push in the app) only with all of: its key, expiry and account number; an initiator on Nexus; the
/// navigation guard's permit for this very link; a press of the player's within 30 seconds that no other link used
/// and that wasn't followed by a trip off Nexus (`presses`); and never used before (in any order of its parts).
/// Queued links that came soon after the view left Nexus and came back are remembered: they ask before anything is
/// turned on (`must_ask`).
pub fn nxm_from_page(url: &str, initiator: &str, presses: &mut Presses, now: Instant, push: &mut dyn FnMut(String)) -> NxmFromPage {
    let url = url.trim();
    if url.len() > 2048 {
        return NxmFromPage::Refused("the link is too long".into());
    }
    let link = match turbine_engine::nxm::parse(url) {
        Ok(l) => l,
        Err(e) => return NxmFromPage::Refused(e),
    };
    let r = link.redacted();
    let key = nxm_key(&link);
    if link.key.as_deref().unwrap_or("").is_empty() || link.expires.is_none() || link.user_id.is_none() {
        return NxmFromPage::Refused(format!("{r} has no key, expiry or account number (Nexus's own button always gives them)"));
    }
    if !initiator_ok(initiator) {
        return NxmFromPage::Refused(format!("{r} came from {}, not nexusmods.com", log_url(initiator)));
    }
    let permit = {
        let mut p = PERMITS.lock().unwrap_or_else(|e| e.into_inner());
        p.retain(|(_, at)| now.saturating_duration_since(*at) < NXM_PERMIT);
        p.iter().position(|(u, _)| *u == key).map(|i| p.remove(i))
    };
    if permit.is_none() {
        return NxmFromPage::Refused(format!("{r} wasn't the page itself going there (a frame, an image or a script's request)"));
    }
    if USED.lock().unwrap_or_else(|e| e.into_inner()).iter().any(|u| *u == key) {
        return NxmFromPage::Refused(format!("{r} was used already"));
    }
    if let Err(e) = presses.take_for_nxm(now) {
        return NxmFromPage::Refused(format!("{r} {e}"));
    }
    {
        let mut used = USED.lock().unwrap_or_else(|e| e.into_inner());
        used.push(key.clone());
        if used.len() > 2000 {
            used.remove(0);
        }
    }
    if presses.came_back(now) {
        let mut a = ASK.lock().unwrap_or_else(|e| e.into_inner());
        a.push(key);
        if a.len() > 200 {
            a.remove(0);
        }
    }
    push(url.to_string());
    NxmFromPage::Queued(r)
}

/// The same, with Turbine's own record of the player's presses.
pub fn nxm_from_page_now(url: &str, initiator: &str, now: Instant, push: &mut dyn FnMut(String)) -> NxmFromPage {
    let mut p = presses();
    nxm_from_page(url, initiator, &mut p, now, push)
}

/// Links that came soon after the view left Nexus and came back: Turbine asks before turning their file on.
static ASK: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn must_ask(url: &str) -> bool {
    let k = key_of(url);
    ASK.lock().unwrap_or_else(|e| e.into_inner()).iter().any(|u| *u == k)
}

/// Did this link come from Turbine's browser? Its file then goes into Game Mods/<Game>.
pub fn from_page(url: &str) -> bool {
    let k = key_of(url);
    USED.lock().unwrap_or_else(|e| e.into_inner()).iter().any(|u| *u == k)
}

// ------------------------------------------------------------------------------------------------ pages

/// The page a mod's Get it opens: the mod's Files tab with that file picked, where Slow download is. A key's Mod
/// Manager Download address, once Turbine has seen the one Nexus uses for a file, may replace this later.
pub fn start_url(domain: &str, mod_id: u64, file_id: u64) -> String {
    let base = format!("https://www.nexusmods.com/{domain}/mods/{mod_id}?tab=files");
    if file_id > 0 { format!("{base}&file_id={file_id}") } else { base }
}

/// How large pages start: the window's height over 640, between 1 and 2, times the Interface size (percent).
pub fn zoom_default(window_height: f64, size_percent: f64) -> f64 {
    let z = (window_height / 640.0).clamp(1.0, 2.0) * (size_percent / 100.0);
    (z * 100.0).round() / 100.0
}

// ------------------------------------------------------------------------------------------------ the status pill

/// The download pill's words for one download.
pub fn pill(state: &str, name: &str, got: i64, total: i64, game: &str) -> String {
    match state {
        "requested" => format!("Starting {name}"),
        "progress" if total > 0 => format!("Downloading {name} {}%", ((got as f64 / total as f64) * 100.0).floor().clamp(0.0, 99.0) as i64),
        "progress" => format!("Downloading {name} {}", size_words(got.max(0) as u64)),
        "adding" => "Adding...".to_string(),
        "added" if !game.is_empty() => format!("Added to {game}: {name}. It's on."),
        "added" => format!("Added: {name}. It's on."),
        "interrupted" => format!("{name} stopped. Try again"),
        "cancelled" => format!("{name} was cancelled"),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("turbine-browse-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn req(url: &str, name: &str, total: i64) -> Request {
        Request { url: url.into(), suggested: name.into(), total }
    }

    /// A plan made just after a press.
    fn planned(r: &Request, dl: &Path, folder: Option<&str>, free: Option<u64>, taken: &[PathBuf]) -> Result<Plan, String> {
        let now = Instant::now();
        plan(r, dl, folder, free, 0, taken, Some(now), now)
    }

    #[test]
    fn a_slow_download_goes_into_its_games_folder() {
        let dl = temp("plan");
        let p = planned(&req("https://cf-files.nexus-cdn.com/files/1/Bigger.zip?md5=x", "TurbineTest-Bigger_Pockets-1.0.3.zip", 1000), &dl, Some("Skyrim Special Edition"), Some(1 << 30), &[]).unwrap();
        assert_eq!(p.path(), dl.join("Game Mods/Skyrim Special Edition/TurbineTest-Bigger_Pockets-1.0.3.zip"));
        assert_eq!(p.folder, "Skyrim Special Edition");
        assert!(p.sure);
        let p = planned(&req("https://example.org/req.7z", "", -1), &dl, None, None, &[]).unwrap();
        assert_eq!(p.path(), dl.join("Game Mods/Other/req.7z"));
    }

    #[test]
    fn only_nexus_file_hosts_after_a_press_are_a_sure_match() {
        let dl = temp("sure");
        let now = Instant::now();
        let press = Some(now - Duration::from_secs(6));
        // Nexus's file host, 6 seconds after Slow download: the page's game.
        let p = plan(&req("https://cf-files.nexus-cdn.com/x/A.zip", "A.zip", 10), &dl, Some("Valheim"), None, 0, &[], press, now).unwrap();
        assert!(p.sure && p.folder == "Valheim");
        // Any other host (an ad frame, a modder's site): Game Mods/Other, and Turbine asks.
        let p = plan(&req("https://evil.example/A.zip", "A.zip", 10), &dl, Some("Valheim"), None, 0, &[], press, now).unwrap();
        assert!(!p.sure && p.folder == OTHER, "{p:?}");
        let p = plan(&req("https://nexus-cdn.com.evil.example/A.zip", "A.zip", 10), &dl, Some("Valheim"), None, 0, &[], press, now).unwrap();
        assert!(!p.sure && p.folder == OTHER);
        // Plain http from Nexus's host isn't taken at all.
        assert!(plan(&req("http://cf-files.nexus-cdn.com/x/A.zip", "A.zip", 10), &dl, Some("Valheim"), None, 0, &[], press, now).is_err());
        assert!(sure_source("https://www.nexusmods.com/x/A.zip", press, now));
        assert!(!sure_source("https://cf-files.nexus-cdn.com/x/A.zip", Some(now - Duration::from_secs(61)), now));
    }

    #[test]
    fn no_download_starts_without_a_press() {
        let dl = temp("nopress");
        let now = Instant::now();
        let r = req("https://cf-files.nexus-cdn.com/x/A.zip", "A.zip", 10);
        assert_eq!(plan(&r, &dl, Some("Valheim"), None, 0, &[], None, now).unwrap_err(), NO_PRESS);
        assert_eq!(plan(&r, &dl, Some("Valheim"), None, 0, &[], Some(now - Duration::from_secs(61)), now).unwrap_err(), NO_PRESS);
        // A press "after" now (clock trouble) doesn't count either.
        assert_eq!(plan(&r, &dl, Some("Valheim"), None, 0, &[], Some(now + Duration::from_secs(5)), now).unwrap_err(), NO_PRESS);
        assert!(plan(&r, &dl, Some("Valheim"), None, 0, &[], Some(now - Duration::from_secs(59)), now).is_ok());
    }

    #[test]
    fn refusals_say_why() {
        let dl = temp("refuse");
        assert_eq!(planned(&req("https://x.nexus-cdn.com/a.exe", "setup.exe", 10), &dl, None, None, &[]).unwrap_err(), ONLY_ARCHIVES);
        assert!(planned(&req("http://x.nexus-cdn.com/a.zip", "a.zip", 10), &dl, None, None, &[]).unwrap_err().contains("https"));
        assert!(planned(&req("ftp://x/a.zip", "a.zip", 10), &dl, None, None, &[]).is_err());
        assert!(planned(&req("https://x/a.zip", "a.zip", (65i64) << 30), &dl, None, None, &[]).unwrap_err().contains("64 GB"));
        let e = planned(&req("https://x/a.zip", "a.zip", 600), &dl, None, Some(1000), &[]).unwrap_err();
        assert!(e.contains("Not enough free space"), "{e}");
    }

    #[test]
    fn unknown_sizes_need_free_space_and_running_downloads_count() {
        let dl = temp("space");
        let now = Instant::now();
        let p = Some(now);
        // Unknown size: a floor of free space.
        assert!(plan(&req("https://x/a.zip", "a.zip", -1), &dl, None, Some(10), 0, &[], p, now).unwrap_err().contains("Not enough free space"));
        assert!(plan(&req("https://x/a.zip", "a.zip", -1), &dl, None, Some(FREE_FLOOR), 0, &[], p, now).is_ok());
        // Two 10 GB downloads with 25 GB free: the second one counts what the first still needs.
        let g = 1u64 << 30;
        assert!(plan(&req("https://x/a.zip", "a.zip", 10 << 30), &dl, None, Some(25 * g), 0, &[], p, now).is_ok());
        assert!(plan(&req("https://x/b.zip", "b.zip", 10 << 30), &dl, None, Some(25 * g), 10 * g, &[], p, now).is_err());
        // While it runs: past 64 GB, or the disk nearly full, it stops.
        assert!(keep_going(1 << 20, Some(10 * g)).is_ok());
        assert!(keep_going(1 << 20, None).is_ok());
        assert!(keep_going(((64u64 << 30) + 1) as i64, Some(100 * g)).unwrap_err().contains("64 GB"));
        assert!(keep_going(1 << 20, Some(FREE_STOP - 1)).unwrap_err().contains("nearly full"));
    }

    #[test]
    fn a_stopped_download_leaves_nothing_behind() {
        let dl = temp("stopped");
        let f = dl.join("Big.zip");
        std::fs::write(dl.join("Big.zip.download"), b"x").unwrap();
        std::fs::write(dl.join("Other.zip"), b"x").unwrap();
        let gone = remove_partial(&f);
        assert_eq!(gone, vec![dl.join("Big.zip.download")]);
        assert!(!dl.join("Big.zip.download").exists() && dl.join("Other.zip").exists());
        assert!(partials_of(&dl.join("notes.txt")).is_empty());
    }

    #[test]
    fn names_are_cleaned_and_made_unique() {
        assert_eq!(clean_name("../../etc/evil.zip", ""), "evil.zip");
        assert_eq!(clean_name("..hidden.zip", ""), "hidden.zip");
        assert_eq!(clean_name("a<b>c|d?.zip", ""), "a_b_c_d_.zip");
        assert_eq!(clean_name("", "https://x.nexus-cdn.com/files/Lantern%20Glow-2.0.zip?x=1"), "Lantern Glow-2.0.zip");
        let dl = temp("unique");
        let dir = dl.join("Game Mods/Valheim");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Mod.zip"), b"x").unwrap();
        assert_eq!(unique_name(&dir, "Mod.zip", &[]), "Mod (2).zip");
        assert_eq!(unique_name(&dir, "Mod.zip", &[dir.join("Mod (2).zip")]), "Mod (3).zip");
        assert_eq!(unique_name(&dir, "New.zip", &[]), "New.zip");
        let p = planned(&req("https://x/Mod.zip", "Mod.zip", 5), &dl, Some("Valheim"), None, &[]).unwrap();
        assert_eq!(p.name, "Mod.zip");
        let p = planned(&req("https://cf-files.nexus-cdn.com/Mod.zip", "Mod.zip", 5), &dl, Some("Valheim"), None, &[]).unwrap();
        assert_eq!(p.name, "Mod (2).zip");
    }

    #[test]
    fn long_and_non_english_names_fit_in_255_bytes_and_keep_their_ending() {
        let dl = temp("long");
        let worst = |n: &str| format!("{n} (999).download").len();
        // 150 CJK characters (450 bytes), 126 Cyrillic (252 bytes), 300 ASCII: each cut on a character boundary.
        for (stem, ext) in [("漢".repeat(150), ".zip"), ("Ж".repeat(126), ".7z"), ("a".repeat(300), ".rar"), ("é".repeat(200), ".zip")] {
            let n = clean_name(&format!("{stem}{ext}"), "");
            assert!(n.ends_with(ext), "{n}");
            assert!(n.len() <= NAME_MAX && worst(&n) <= NAME_MAX, "{} bytes", n.len());
            assert!(turbine_engine::archives::is_archive_name(&n));
            // The file can really be made, with its ".download" while it runs.
            std::fs::write(dl.join(format!("{n}.download")), b"x").unwrap();
            let p = planned(&req("https://cf-files.nexus-cdn.com/x", &format!("{stem}{ext}"), 5), &dl, None, None, &[]).unwrap();
            assert!(p.name.ends_with(ext));
        }
        // No ending at all, or a bogus long one: cut as a whole and refused as not an archive (the right message).
        assert_eq!(clean_name(&"a".repeat(300), "").len(), STEM_MAX);
        assert_eq!(planned(&req("https://x/y", &format!("{}.zip", "a".repeat(300)), 5), &dl, None, None, &[]).map(|p| p.name.len()), Ok(STEM_MAX + 4));
    }

    #[test]
    fn nexus_pages_and_hosts() {
        assert_eq!(page_ids("https://www.nexusmods.com/skyrimspecialedition/mods/266?tab=files&file_id=9"), Some(("skyrimspecialedition".into(), 266, Some(9))));
        assert_eq!(page_ids("https://www.nexusmods.com/games/valheim/mods/7"), Some(("valheim".into(), 7, None)));
        assert_eq!(page_ids("https://evil.example/skyrimspecialedition/mods/266"), None);
        assert_eq!(page_ids("https://nexusmods.com.evil.example/skyrimspecialedition/mods/266"), None);
        assert_eq!(page_ids("http://www.nexusmods.com/skyrimspecialedition/mods/266"), None);
        assert!(is_nexus_host("users.nexusmods.com") && is_nexus_host("nexusmods.com"));
        assert!(!is_nexus_host("evilnexusmods.com") && !is_nexus_host("nexusmods.com.evil"));
        assert!(is_nexus_file_host("cf-files.nexus-cdn.com") && !is_nexus_file_host("evilnexus-cdn.com"));
        assert_eq!(host_of("https://user@www.nexusmods.com/"), None);
        assert_eq!(host_of("https://www.nexusmods.com:443/x"), Some(("www.nexusmods.com".into(), true)));
    }

    #[test]
    fn the_stand_in_counts_only_when_set() {
        // Before it is set, the stand-in's address is an ordinary http page.
        assert!(!is_test_url("http://127.0.0.1:47800/skyrimspecialedition/mods/1"));
        assert!(!initiator_ok("http://127.0.0.1:47800"));
        set_test_origin("http://127.0.0.1:47800");
        assert!(initiator_ok("http://127.0.0.1:47800"));
        assert_eq!(page_ids("http://127.0.0.1:47800/skyrimspecialedition/mods/1?tab=files&file_id=2"), Some(("skyrimspecialedition".into(), 1, Some(2))));
        assert!(!is_test_url("http://127.0.0.1:47801/x"));
        assert!(!is_test_url("http://127.0.0.1:478000/x"));
        let dl = temp("standin");
        assert!(planned(&req("http://127.0.0.1:47800/files/TurbineTest-Bigger_Pockets-1.0.3.zip", "TurbineTest-Bigger_Pockets-1.0.3.zip", 10), &dl, None, None, &[]).is_ok());
    }

    const L: &str = "nxm://skyrimspecialedition/mods/1/files/2?key=abc&expires=4102444800&user_id=1";
    const PAGE: &str = "https://www.nexusmods.com/skyrimspecialedition/mods/1?tab=files&file_id=2";
    const NEXUS: &str = "https://www.nexusmods.com";

    /// A press made `ago` before `now`.
    fn pressed(now: Instant, ago: u64) -> Presses {
        let mut p = Presses::default();
        p.press(now - Duration::from_secs(ago));
        p
    }

    #[test]
    fn nxm_from_the_page_needs_the_main_frame_on_nexus_a_press_and_a_fresh_link() {
        let mut got = Vec::new();
        let now = Instant::now();
        let l = "nxm://skyrimspecialedition/mods/1/files/5?key=abc&expires=4102444800&user_id=1";
        // The navigation guard: only the main frame, only while the page is on Nexus.
        assert!(nxm_permit(l, false, PAGE, now).unwrap_err().contains("frame"));
        assert!(nxm_permit(l, true, "https://github.com/someone/mod", now).unwrap_err().contains("isn't on nexusmods.com"));
        assert!(nxm_permit(l, true, "http://www.nexusmods.com/x", now).is_err());
        // No permit: a frame, an image or a request from a script never gets one.
        let mut p = pressed(now, 6);
        assert!(matches!(nxm_from_page(l, NEXUS, &mut p, now, &mut |u| got.push(u)), NxmFromPage::Refused(ref e) if e.contains("wasn't the page itself")));
        nxm_permit(l, true, PAGE, now).unwrap();
        // The scheme handler's initiator must be Nexus too.
        assert!(matches!(nxm_from_page(l, "https://ads.example", &mut p, now, &mut |u| got.push(u)), NxmFromPage::Refused(ref e) if e.contains("not nexusmods.com")));
        assert!(matches!(nxm_from_page(l, "null", &mut p, now, &mut |u| got.push(u)), NxmFromPage::Refused(_)));
        let r = nxm_from_page(l, NEXUS, &mut p, now, &mut |u| got.push(u));
        assert!(matches!(r, NxmFromPage::Queued(ref s) if !s.contains("abc") && !s.contains("user_id")), "{r:?}");
        assert_eq!(got.len(), 1);
        assert!(from_page(l) && !must_ask(l));
        // The same link again (a page repeating it, or the other route), even with a fresh press: never a second
        // download, and not with its parts in another order either.
        let mut p = pressed(now, 1);
        nxm_permit(l, true, PAGE, now).unwrap();
        assert!(matches!(nxm_from_page(l, NEXUS, &mut p, now, &mut |u| got.push(u)), NxmFromPage::Refused(ref e) if e.contains("used already")));
        let reordered = "nxm://skyrimspecialedition/mods/1/files/5?user_id=1&expires=4102444800&key=abc";
        nxm_permit(reordered, true, PAGE, now).unwrap();
        assert!(matches!(nxm_from_page(reordered, NEXUS, &mut p, now, &mut |u| got.push(u)), NxmFromPage::Refused(ref e) if e.contains("used already")));
        assert!(from_page(reordered));
        assert_eq!(got.len(), 1);
    }

    #[test]
    fn nxm_from_the_page_needs_a_press_in_the_last_30_seconds() {
        let mut got = Vec::new();
        let now = Instant::now();
        let l = "nxm://skyrimspecialedition/mods/1/files/6?key=abc&expires=4102444800&user_id=1";
        let mut future = Presses::default();
        future.press(now + Duration::from_secs(3));
        for mut press in [Presses::default(), pressed(now, 31), future] {
            nxm_permit(l, true, PAGE, now).unwrap();
            assert!(matches!(nxm_from_page(l, NEXUS, &mut press, now, &mut |u| got.push(u)), NxmFromPage::Refused(ref e) if e.contains("no press")), "{press:?}");
        }
        assert!(got.is_empty() && !from_page(l));
        // The permit is only good for a moment.
        nxm_permit(l, true, PAGE, now).unwrap();
        let later = now + NXM_PERMIT + Duration::from_secs(1);
        let mut p = Presses::default();
        p.press(later);
        assert!(matches!(nxm_from_page(l, NEXUS, &mut p, later, &mut |u| got.push(u)), NxmFromPage::Refused(_)));
        nxm_permit(l, true, PAGE, now).unwrap();
        assert!(matches!(nxm_from_page(l, NEXUS, &mut pressed(now, 29), now, &mut |u| got.push(u)), NxmFromPage::Queued(_)));
    }

    #[test]
    fn nxm_from_the_page_needs_key_expiry_and_account() {
        let mut got = Vec::new();
        let now = Instant::now();
        for l in [
            "nxm://skyrimspecialedition/mods/1/files/7",
            "nxm://skyrimspecialedition/mods/1/files/7?key=abc&expires=4102444800",
            "nxm://skyrimspecialedition/mods/1/files/7?user_id=1",
            "nxm://",
            "https://www.nexusmods.com/",
        ] {
            let _ = nxm_permit(l, true, PAGE, now);
            assert!(matches!(nxm_from_page(l, NEXUS, &mut pressed(now, 0), now, &mut |u| got.push(u)), NxmFromPage::Refused(_)), "{l}");
        }
        assert!(got.is_empty());
        let _ = L;
    }

    /// The second break test's H1b: the player presses on an outside link from a mod page; the outside page at once
    /// sends the view to Nexus's Mod Manager Download address (&nmm=1), and Nexus's page goes to the nxm link.
    #[test]
    fn a_bounce_off_nexus_and_back_never_spends_the_press() {
        let mut got = Vec::new();
        let t0 = Instant::now();
        let mut p = Presses::default();
        p.press(t0); // A on the outside link, on a Nexus page
        let bounce = "https://www.nexusmods.com/skyrimspecialedition/mods/999?tab=files&file_id=5&nmm=1";
        let l = "nxm://skyrimspecialedition/mods/999/files/5?key=k&expires=4102444800&user_id=7";
        // The view goes to the outside page (its navigation, or a redirect through it): the press is gone.
        p.left_nexus(t0 + Duration::from_millis(300));
        // Back on Nexus's nmm=1 page, which goes to the nxm link by itself 6 seconds after the press.
        let now = t0 + Duration::from_secs(6);
        nxm_permit(l, true, bounce, now).unwrap();
        let r = nxm_from_page(l, NEXUS, &mut p, now, &mut |u| got.push(u));
        assert!(matches!(r, NxmFromPage::Refused(ref e) if e.contains("no press")), "{r:?}");
        assert!(got.is_empty());
        // A press on Nexus after coming back works, but the file asks before anything is turned on.
        p.press(now + Duration::from_secs(1));
        nxm_permit(l, true, bounce, now + Duration::from_secs(2)).unwrap();
        assert!(matches!(nxm_from_page(l, NEXUS, &mut p, now + Duration::from_secs(2), &mut |u| got.push(u)), NxmFromPage::Queued(_)));
        assert!(must_ask(l));
        // One press, one link: a second, different link on the same press is refused.
        let l2 = "nxm://skyrimspecialedition/mods/998/files/6?key=k&expires=4102444800&user_id=7";
        nxm_permit(l2, true, bounce, now + Duration::from_secs(3)).unwrap();
        let r2 = nxm_from_page(l2, NEXUS, &mut p, now + Duration::from_secs(3), &mut |u| got.push(u));
        assert!(matches!(r2, NxmFromPage::Refused(ref e) if e.contains("one link")), "{r2:?}");
        assert_eq!(got.len(), 1);
        // Long after the trip, a press and a link no longer ask.
        let late = t0 + CAME_BACK + Duration::from_secs(10);
        let l3 = "nxm://skyrimspecialedition/mods/997/files/8?key=k&expires=4102444800&user_id=7";
        p.press(late);
        nxm_permit(l3, true, PAGE, late).unwrap();
        assert!(matches!(nxm_from_page(l3, NEXUS, &mut p, late, &mut |u| got.push(u)), NxmFromPage::Queued(_)));
        assert!(!must_ask(l3));
    }

    #[test]
    fn going_off_nexus_drops_the_press_and_nexus_pages_keep_it() {
        let now = Instant::now();
        let mut p = Presses::default();
        p.press(now);
        assert!(p.for_download().is_some());
        p.left_nexus(now);
        assert!(p.for_download().is_none() && p.came_back(now));
        // The app's record: Nexus pages and about:blank keep the press, anything else drops it.
        note_press(now);
        assert!(!note_main_frame("https://www.nexusmods.com/skyrimspecialedition/mods/1", now));
        assert!(!note_main_frame("about:blank", now));
        assert!(last_press().is_some());
        assert!(note_main_frame("https://evil.example/bounce", now));
        assert!(last_press().is_none());
        note_press(now);
        assert!(note_main_frame("data:text/html,x", now));
        assert!(last_press().is_none());
    }

    #[test]
    fn one_turbine_holds_the_lock_and_only_it_removes_partials() {
        let d = temp("lock");
        let first = lock_instance(&d.join(LOCK_FILE));
        assert!(first.mine(), "{first:?}");
        let second = lock_instance(&d.join(LOCK_FILE));
        assert!(matches!(second, Lock::Taken), "{second:?}");
        // The second Turbine never removes the first one's running downloads.
        let dir = d.join("Downloads/Game Mods/Valheim");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Running.zip.download"), b"x").unwrap();
        assert!(remove_partials(&second, &d.join("Downloads")).is_empty());
        assert!(remove_partials(&Lock::Unknown("test".into()), &d.join("Downloads")).is_empty());
        assert!(dir.join("Running.zip.download").exists());
        // Once the first one ends, the next start takes the lock and cleans up.
        drop(first);
        let third = lock_instance(&d.join(LOCK_FILE));
        assert!(third.mine());
        assert_eq!(remove_partials(&third, &d.join("Downloads")), vec![dir.join("Running.zip.download")]);
    }

    #[test]
    fn logged_addresses_keep_no_signed_parts() {
        assert_eq!(
            log_url("https://cf-files.nexus-cdn.com/files/1/Big.zip?md5=SECRET&expires=1&user_id=1234567"),
            "https://cf-files.nexus-cdn.com/files/1/Big.zip"
        );
        assert_eq!(log_url("https://users.nexusmods.com/oauth/callback?code=SECRET#state=x"), "https://users.nexusmods.com/oauth/callback");
        assert_eq!(log_url("https://name:pw@www.nexusmods.com/a"), "https://www.nexusmods.com/a");
        assert_eq!(log_url("data:text/html;base64,SECRET"), "data:");
        assert_eq!(log_url("HTTPS://WWW.NEXUSMODS.COM"), "https://www.nexusmods.com");
        assert!(!log_url(L).contains("user_id") && !log_url(L).contains("abc"));
        // Path parameters (";code=") are cut too.
        assert_eq!(log_url("https://users.nexusmods.com/auth/callback;code=SECRET"), "https://users.nexusmods.com/auth/callback");
    }

    #[test]
    fn same_page_ignores_the_order_of_the_query() {
        assert!(same_page(
            "https://www.nexusmods.com/skyrimspecialedition/mods/1?tab=files&file_id=2",
            "https://www.nexusmods.com/skyrimspecialedition/mods/1?file_id=2&tab=files#x"
        ));
        assert!(!same_page("https://www.nexusmods.com/skyrimspecialedition/mods/1?tab=files&file_id=2", "https://www.nexusmods.com/skyrimspecialedition/mods/1?tab=files&file_id=3"));
        assert!(!same_page("https://www.nexusmods.com/a", "https://evil.example/a"));
    }

    #[test]
    fn the_link_must_be_for_the_signed_in_account() {
        assert!(user_matches(Some(1234), 1234).is_ok());
        assert!(user_matches(Some(1234), 0).is_ok());
        assert!(user_matches(None, 1234).is_ok());
        assert!(user_matches(Some(1234), 99).unwrap_err().contains("another Nexus account"));
    }

    #[test]
    fn start_pages_zoom_and_pill() {
        assert_eq!(start_url("witcher3", 2109, 1), "https://www.nexusmods.com/witcher3/mods/2109?tab=files&file_id=1");
        assert_eq!(start_url("witcher3", 2109, 0), "https://www.nexusmods.com/witcher3/mods/2109?tab=files");
        assert_eq!(zoom_default(800.0, 100.0), 1.25);
        assert_eq!(zoom_default(1080.0, 100.0), 1.69);
        assert_eq!(zoom_default(400.0, 100.0), 1.0);
        assert_eq!(zoom_default(2160.0, 100.0), 2.0);
        assert_eq!(pill("progress", "SkyUI", 42, 100, ""), "Downloading SkyUI 42%");
        assert_eq!(pill("progress", "SkyUI", 100, 100, ""), "Downloading SkyUI 99%");
        assert_eq!(pill("added", "SkyUI", 0, 0, "Skyrim Special Edition"), "Added to Skyrim Special Edition: SkyUI. It's on.");
        assert_eq!(pill("adding", "x", 0, 0, ""), "Adding...");
    }

    #[test]
    fn percent_decoding_keeps_odd_input() {
        assert_eq!(percent_decode("a%20b"), "a b");
        assert_eq!(percent_decode("a%2"), "a%2");
        assert_eq!(percent_decode("%zz"), "%zz");
        assert_eq!(percent_decode("%"), "%");
    }

    #[test]
    fn leftovers_of_a_stopped_download_go_and_nothing_else() {
        let dl = temp("partials");
        let dir = dl.join("Game Mods/Valheim");
        std::fs::create_dir_all(&dir).unwrap();
        for n in ["Mod.zip.download", "Mod.zip", "notes.txt.download", "Other.7z.crdownload"] {
            std::fs::write(dir.join(n), b"x").unwrap();
        }
        std::fs::write(dl.join("Loose.zip.download"), b"x").unwrap();
        let lock = lock_instance(&dl.join(LOCK_FILE));
        let gone = remove_partials(&lock, &dl);
        assert_eq!(gone, vec![dir.join("Mod.zip.download")]);
        assert!(dir.join("Mod.zip").exists() && dir.join("notes.txt.download").exists() && dir.join("Other.7z.crdownload").exists());
        assert!(dl.join("Loose.zip.download").exists());
    }
}
