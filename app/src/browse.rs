//! Nexus inside Turbine (NexusBrowser.qml): the decisions about what the page asks for, kept apart from Qt so they
//! are tested on their own (rustc --test app/src/browse.rs --extern turbine_engine=<the engine's rlib>).
//!
//! - `plan`: a download the page started (the player pressed Slow download, or a link to a file on another site):
//!   refused unless it is https, a zip, 7z or rar file, under 64 GB, with room for twice its size; otherwise it goes
//!   into Downloads/Game Mods/<Game>/ under its own name, cleaned, made unique with " (2)".
//! - `nxm_from_page`: a Mod Manager Download link the page navigated to (webglue.cpp's scheme handler, and QML's
//!   navigation guard): checked with nxm::parse, remembered as "from the browser" (so its file goes into Game Mods
//!   too) and queued like a link from any other place. The same link twice within 10 seconds is one link.
//! - `start_url`: the exact file page a mod's Get it opens.
//! - `zoom_default`: how large pages start.
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
/// The same nxm link again within this long is the same press (the scheme handler and the navigation guard both
/// see it).
pub const NXM_SAME: Duration = Duration::from_secs(10);

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
    /// The game's folder name inside Game Mods ("Other" when it isn't known).
    pub folder: String,
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
/// browsers refuse, no leading dots, at most 200 characters. The same rules the browser helper uses (save.js).
pub fn clean_name(suggested: &str, url: &str) -> String {
    let from = if suggested.trim().is_empty() { last_part(url) } else { suggested.to_string() };
    let base = from.rsplit(['/', '\\']).next().unwrap_or("").to_string();
    let replaced: String = base.chars().map(|c| if c.is_control() || "<>:\"|?*".contains(c) { '_' } else { c }).collect();
    let t = replaced.trim_start_matches('.').trim();
    t.chars().take(200).collect::<String>().trim().to_string()
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

/// Decides where a download goes, or why it is refused. `downloads`: the player's Downloads folder; `folder`: the
/// game's folder name inside Game Mods (None when no installed game fits the page); `free`: free bytes where it
/// goes, when known; `taken`: downloads already on their way (their names count as taken).
pub fn plan(req: &Request, downloads: &Path, folder: Option<&str>, free: Option<u64>, taken: &[PathBuf]) -> Result<Plan, String> {
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
    if let (Some(f), true) = (free, req.total > 0) {
        if f < (req.total as u64).saturating_mul(2) {
            return Err(format!("Not enough free space for {name}: it needs {} free", size_words((req.total as u64).saturating_mul(2))));
        }
    }
    let folder = folder.map(turbine_engine::watch::game_folder_name).unwrap_or_else(|| OTHER.to_string());
    let dir = downloads.join(GAME_MODS).join(&folder);
    let name = unique_name(&dir, &name, taken);
    Ok(Plan { dir, name, folder })
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

// ------------------------------------------------------------------------------------------------ nxm links

static FROM_PAGE: Mutex<Vec<(String, Instant)>> = Mutex::new(Vec::new());

/// What happened to an nxm link from the page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NxmFromPage {
    /// Checked and queued; the redacted link for the log.
    Queued(String),
    /// The same link a moment ago (the other route saw it first).
    Again,
    /// Not a link Turbine can use.
    Refused(String),
}

/// A Mod Manager Download link the page navigated to. `push` queues it (single::push in the app).
pub fn nxm_from_page(url: &str, now: Instant, push: &mut dyn FnMut(String)) -> NxmFromPage {
    let url = url.trim();
    if url.len() > 2048 {
        return NxmFromPage::Refused("the link is too long".into());
    }
    let link = match turbine_engine::nxm::parse(url) {
        Ok(l) => l,
        Err(e) => return NxmFromPage::Refused(e),
    };
    let mut seen = FROM_PAGE.lock().unwrap_or_else(|e| e.into_inner());
    seen.retain(|(_, at)| now.saturating_duration_since(*at) < Duration::from_secs(600));
    if seen.iter().any(|(u, at)| u == url && now.saturating_duration_since(*at) < NXM_SAME) {
        return NxmFromPage::Again;
    }
    seen.push((url.to_string(), now));
    if seen.len() > 50 {
        seen.remove(0);
    }
    drop(seen);
    push(url.to_string());
    NxmFromPage::Queued(link.redacted())
}

/// Did this link come from Turbine's browser (in the last 10 minutes)? Its file then goes into Game Mods/<Game>.
pub fn from_page(url: &str) -> bool {
    FROM_PAGE.lock().unwrap_or_else(|e| e.into_inner()).iter().any(|(u, _)| u == url.trim())
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

    #[test]
    fn a_slow_download_goes_into_its_games_folder() {
        let dl = temp("plan");
        let p = plan(&req("https://cf-files.nexus-cdn.com/files/1/Bigger.zip?md5=x", "TurbineTest-Bigger_Pockets-1.0.3.zip", 1000), &dl, Some("Skyrim Special Edition"), Some(1 << 30), &[]).unwrap();
        assert_eq!(p.path(), dl.join("Game Mods/Skyrim Special Edition/TurbineTest-Bigger_Pockets-1.0.3.zip"));
        assert_eq!(p.folder, "Skyrim Special Edition");
        let p = plan(&req("https://example.org/req.7z", "", -1), &dl, None, None, &[]).unwrap();
        assert_eq!(p.path(), dl.join("Game Mods/Other/req.7z"));
    }

    #[test]
    fn refusals_say_why() {
        let dl = temp("refuse");
        assert_eq!(plan(&req("https://x.nexus-cdn.com/a.exe", "setup.exe", 10), &dl, None, None, &[]).unwrap_err(), ONLY_ARCHIVES);
        assert!(plan(&req("http://x.nexus-cdn.com/a.zip", "a.zip", 10), &dl, None, None, &[]).unwrap_err().contains("https"));
        assert!(plan(&req("ftp://x/a.zip", "a.zip", 10), &dl, None, None, &[]).is_err());
        assert!(plan(&req("https://x/a.zip", "a.zip", (65i64) << 30), &dl, None, None, &[]).unwrap_err().contains("64 GB"));
        let e = plan(&req("https://x/a.zip", "a.zip", 600), &dl, None, Some(1000), &[]).unwrap_err();
        assert!(e.contains("Not enough free space"), "{e}");
        // Unknown size: no space check.
        assert!(plan(&req("https://x/a.zip", "a.zip", -1), &dl, None, Some(10), &[]).is_ok());
    }

    #[test]
    fn names_are_cleaned_and_made_unique() {
        assert_eq!(clean_name("../../etc/evil.zip", ""), "evil.zip");
        assert_eq!(clean_name("..hidden.zip", ""), "hidden.zip");
        assert_eq!(clean_name("a<b>c|d?.zip", ""), "a_b_c_d_.zip");
        assert_eq!(clean_name("", "https://x.nexus-cdn.com/files/Lantern%20Glow-2.0.zip?x=1"), "Lantern Glow-2.0.zip");
        assert_eq!(clean_name(&"a".repeat(300), "").chars().count(), 200);
        let dl = temp("unique");
        let dir = dl.join("Game Mods/Valheim");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Mod.zip"), b"x").unwrap();
        assert_eq!(unique_name(&dir, "Mod.zip", &[]), "Mod (2).zip");
        assert_eq!(unique_name(&dir, "Mod.zip", &[dir.join("Mod (2).zip")]), "Mod (3).zip");
        assert_eq!(unique_name(&dir, "New.zip", &[]), "New.zip");
        let p = plan(&req("https://x/Mod.zip", "Mod.zip", 5), &dl, Some("Valheim"), None, &[]).unwrap();
        assert_eq!(p.name, "Mod (2).zip");
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
        assert_eq!(host_of("https://user@www.nexusmods.com/"), None);
        assert_eq!(host_of("https://www.nexusmods.com:443/x"), Some(("www.nexusmods.com".into(), true)));
    }

    #[test]
    fn the_stand_in_counts_only_when_set() {
        // Before it is set, the stand-in's address is an ordinary http page.
        assert!(!is_test_url("http://127.0.0.1:47800/skyrimspecialedition/mods/1"));
        set_test_origin("http://127.0.0.1:47800");
        assert_eq!(page_ids("http://127.0.0.1:47800/skyrimspecialedition/mods/1?tab=files&file_id=2"), Some(("skyrimspecialedition".into(), 1, Some(2))));
        assert!(!is_test_url("http://127.0.0.1:47801/x"));
        assert!(!is_test_url("http://127.0.0.1:478000/x"));
        let dl = temp("standin");
        assert!(plan(&req("http://127.0.0.1:47800/files/TurbineTest-Bigger_Pockets-1.0.3.zip", "TurbineTest-Bigger_Pockets-1.0.3.zip", 10), &dl, None, None, &[]).is_ok());
    }

    #[test]
    fn nxm_links_from_the_page_are_checked_and_counted_once() {
        let mut got = Vec::new();
        let now = Instant::now();
        let l = "nxm://skyrimspecialedition/mods/1/files/2?key=abc&expires=4102444800&user_id=1";
        let r = nxm_from_page(l, now, &mut |u| got.push(u));
        assert!(matches!(r, NxmFromPage::Queued(ref s) if !s.contains("abc")), "{r:?}");
        assert_eq!(nxm_from_page(l, now + Duration::from_secs(2), &mut |u| got.push(u)), NxmFromPage::Again);
        assert_eq!(got.len(), 1);
        assert!(from_page(l));
        assert!(matches!(nxm_from_page("nxm://", now, &mut |u| got.push(u)), NxmFromPage::Refused(_)));
        assert!(matches!(nxm_from_page("https://www.nexusmods.com/", now, &mut |u| got.push(u)), NxmFromPage::Refused(_)));
        assert_eq!(got.len(), 1);
        // Pressed again later: a new press.
        assert!(matches!(nxm_from_page(l, now + Duration::from_secs(30), &mut |u| got.push(u)), NxmFromPage::Queued(_)));
        assert_eq!(got.len(), 2);
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
}
