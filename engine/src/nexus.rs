//! Nexus Mods through its official API. Browsing needs no key: v2 GraphQL (search, a mod's details and
//! requirements, its files) is sent with Turbine's application headers only (`Nexus::public`). The player's personal
//! API key (pasted in Settings from their Nexus account page) adds v1 REST for the account, update lists, MD5
//! lookups and Premium download links. Requests send Turbine's application headers and respect Nexus's rate limit headers. Parsing is pure and
//! tested offline; the network calls are thin wrappers.

use crate::json::{self, quote, Json};
use crate::net::{self, Client, Download, Expect, NetError, Progress, Request, Response, SourceInfo};
use crate::nxm::{self, NxmLink};
use std::cell::Cell;
use std::fs;
use std::path::Path;

pub const API: &str = "https://api.nexusmods.com";
pub const GRAPHQL: &str = "https://api.nexusmods.com/v2/graphql";
pub const APP_NAME: &str = "TurbineModManager";

/// Nexus game domains: (store, store ID, domain, name). From the games' data files (engine/data/games), then the
/// games the player picked or Turbine matched by name this run (`register`).
fn table() -> Vec<(crate::games::Store, &'static str, &'static str, &'static str)> {
    static BUILT: std::sync::OnceLock<Vec<(crate::games::Store, &'static str, &'static str, &'static str)>> = std::sync::OnceLock::new();
    let mut out = BUILT
        .get_or_init(|| {
            let mut v = Vec::new();
            for d in crate::rules::games() {
                if let Some(domain) = d.data.nexus.as_deref() {
                    for (s, id) in d.ids {
                        v.push((*s, *id, domain, d.name));
                    }
                }
            }
            v
        })
        .clone();
    for d in crate::rules::player_games() {
        if let Some(domain) = d.data.nexus.as_deref() {
            for (s, id) in d.ids {
                out.push((*s, *id, domain, d.name));
            }
        }
    }
    out.extend(registered().read().unwrap_or_else(|e| e.into_inner()).iter().copied());
    out
}

type Registered = std::sync::RwLock<Vec<(crate::games::Store, &'static str, &'static str, &'static str)>>;

fn registered() -> &'static Registered {
    static R: std::sync::OnceLock<Registered> = std::sync::OnceLock::new();
    R.get_or_init(|| std::sync::RwLock::new(Vec::new()))
}

/// Notes a Nexus domain for a game without one in its data file (the player's pick or an exact name match), so
/// links, downloads and Get mods find the game. Registering the same game again replaces its domain.
pub fn register(store: crate::games::Store, store_id: &str, domain: &str, name: &str) {
    let mut r = registered().write().unwrap_or_else(|e| e.into_inner());
    if let Some(e) = r.iter_mut().find(|e| e.0 == store && e.1 == store_id) {
        if e.2 == domain {
            return;
        }
        e.2 = Box::leak(domain.to_string().into_boxed_str());
        e.3 = Box::leak(name.to_string().into_boxed_str());
        return;
    }
    r.push((store, Box::leak(store_id.to_string().into_boxed_str()), Box::leak(domain.to_string().into_boxed_str()), Box::leak(name.to_string().into_boxed_str())));
}

/// Takes a registered domain away again (the player said the game isn't on Nexus).
pub fn unregister(store: crate::games::Store, store_id: &str) {
    registered().write().unwrap_or_else(|e| e.into_inner()).retain(|e| !(e.0 == store && e.1 == store_id));
}

/// Nexus game domains as (store ID, domain, name), the shape the launch games' table had.
pub fn games() -> Vec<(&'static str, &'static str, &'static str)> {
    table().into_iter().map(|(_, id, d, n)| (id, d, n)).collect()
}

/// The Nexus domain for a store ID (any store; `domain_for` also checks the store).
pub fn domain_for_store_id(store_id: &str) -> Option<&'static str> {
    table().into_iter().find(|(_, id, _, _)| *id == store_id).map(|(_, _, d, _)| d)
}

/// The Nexus domain for a game by its store and ID.
pub fn domain_for(store: crate::games::Store, store_id: &str) -> Option<&'static str> {
    table().into_iter().find(|(s, id, _, _)| *s == store && *id == store_id).map(|(_, _, d, _)| d)
}

/// Store IDs of the Turbine games that use this Nexus domain (the reverse of `domain_for_store_id`).
pub fn store_ids_for_domain(domain: &str) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for (_, id, d, _) in table() {
        if d == domain && !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

/// The game's name for a Nexus domain Turbine supports.
pub fn game_name_for_domain(domain: &str) -> Option<&'static str> {
    table().into_iter().find(|(_, _, d, _)| *d == domain).map(|(_, _, _, n)| n)
}

fn known_domain(domain: &str) -> bool {
    table().iter().any(|(_, _, d, _)| *d == domain)
}

/// Does Turbine support this Nexus game domain?
pub fn supported_domain(domain: &str) -> bool {
    known_domain(domain)
}

/// The message free accounts get instead of a download (Nexus only gives download links to apps for Premium).
pub const FREE_ACCOUNT_MESSAGE: &str = "Nexus only lets Premium members download through apps. Download the file \
in Desktop Mode's browser, then add it from Downloads.";

/// Does this look like an API key? (Trimmed, no spaces or control characters, a sensible length.)
pub fn plausible_key(key: &str) -> bool {
    let k = key.trim();
    (20..=1024).contains(&k.len()) && k.bytes().all(|b| b.is_ascii_graphic())
}

/// Saves the key in a file only the player can read (the keyring may be locked in Game Mode).
pub fn save_key(path: &Path, key: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("partial");
    let _ = fs::remove_file(&tmp);
    let mut f = fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&tmp)?;
    f.write_all(key.trim().as_bytes())?;
    f.sync_all()?;
    fs::rename(&tmp, path)
}

pub fn load_key(path: &Path) -> Option<String> {
    let k = fs::read_to_string(path).ok()?;
    plausible_key(&k).then(|| k.trim().to_string())
}

/// The headers every request with a key carries.
pub fn headers(key: &str, app_version: &str) -> Vec<(String, String)> {
    let mut h = vec![("apikey".to_string(), key.trim().to_string())];
    h.extend(headers_public(app_version));
    h
}

/// The headers of a request without a key (browsing): Turbine's name and version, no `apikey`.
pub fn headers_public(app_version: &str) -> Vec<(String, String)> {
    vec![
        ("Application-Name".into(), APP_NAME.into()),
        ("Application-Version".into(), app_version.into()),
        ("Accept".into(), "application/json".into()),
    ]
}

/// What a call that needs the key says when there is none.
pub const NEEDS_KEY: &str = "This needs a Nexus Mods key (Settings, Getting mods).";

/// 0.63: a refusal from Nexus's API (401 or 403) that nothing more exact explains: here, and only here, the
/// account or the key is what to look at. Refusals from other sites never say so.
pub fn refused_words(e: NetError) -> NetError {
    match e {
        NetError::Denied(c) => NetError::Failed(format!("Nexus refused the request ({c}). Check your account or key in Settings.")),
        e => e,
    }
}

/// How long Turbine waits after Nexus says "too many requests" (seconds).
pub const BACKOFF_SECS: u64 = 60;

// ---- Rate limits ----

/// What Nexus said is left, from the x-rl-* headers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RateLimit {
    pub hourly_remaining: Option<u64>,
    pub daily_remaining: Option<u64>,
    pub hourly_reset: Option<String>,
    pub daily_reset: Option<String>,
}

pub fn rate_limit(headers: &[(String, String)]) -> RateLimit {
    let get = |n: &str| headers.iter().rev().find(|(k, _)| k.eq_ignore_ascii_case(n)).map(|(_, v)| v.trim().to_string());
    RateLimit {
        hourly_remaining: get("x-rl-hourly-remaining").and_then(|v| v.parse().ok()),
        daily_remaining: get("x-rl-daily-remaining").and_then(|v| v.parse().ok()),
        hourly_reset: get("x-rl-hourly-reset"),
        daily_reset: get("x-rl-daily-reset"),
    }
}

impl RateLimit {
    /// None while requests may be made; otherwise why not. Nexus allows the daily allowance once the hourly one is
    /// used up, so only both at zero (or a daily zero) stops Turbine.
    pub fn blocked(&self) -> Option<String> {
        match (self.hourly_remaining, self.daily_remaining) {
            (_, Some(0)) => Some(format!(
                "Nexus's daily request limit is used up.{} Turbine tries again after that.",
                self.daily_reset.as_deref().map(|r| format!(" It resets at {r}.")).unwrap_or_default()
            )),
            (Some(0), None) => Some(format!(
                "Nexus's hourly request limit is used up.{} Turbine tries again after that.",
                self.hourly_reset.as_deref().map(|r| format!(" It resets at {r}.")).unwrap_or_default()
            )),
            _ => None,
        }
    }
}

// ---- Parsing (pure) ----

fn n(v: Option<&Json>) -> f64 {
    match v {
        Some(Json::Num(x)) if x.is_finite() => *x,
        Some(Json::Str(s)) => s.trim().parse().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn s(v: &Json, k: &str) -> String {
    v.str_at(k).unwrap_or("").to_string()
}

fn b(v: &Json, k: &str) -> bool {
    v.get(k).and_then(Json::as_bool).unwrap_or(false)
}

fn parse(text: &str) -> Result<Json, String> {
    json::parse(text).map_err(|e| format!("Nexus's answer can't be read ({e})"))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Account {
    pub user_id: u64,
    pub name: String,
    pub is_premium: bool,
    pub is_supporter: bool,
}

/// Reads users/validate.json.
pub fn parse_validate(text: &str) -> Result<Account, String> {
    let j = parse(text)?;
    let name = j.str_at("name").ok_or("Nexus didn't confirm the key")?;
    Ok(Account {
        user_id: n(j.get("user_id")) as u64,
        name: name.to_string(),
        is_premium: b(&j, "is_premium"),
        is_supporter: b(&j, "is_supporter"),
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModInfo {
    pub mod_id: u64,
    pub domain: String,
    pub name: String,
    pub summary: String,
    pub version: String,
    pub author: String,
    pub picture_url: String,
    pub endorsements: u64,
    pub downloads: u64,
    pub adult: bool,
    /// False when the mod is hidden, under moderation or removed.
    pub available: bool,
    pub status: String,
    pub updated_timestamp: u64,
}

fn mod_from_json(j: &Json) -> Option<ModInfo> {
    let mod_id = n(j.get("mod_id")) as u64;
    if mod_id == 0 {
        return None;
    }
    Some(ModInfo {
        mod_id,
        domain: s(j, "domain_name"),
        name: s(j, "name"),
        summary: s(j, "summary"),
        version: s(j, "version"),
        author: s(j, "author"),
        picture_url: s(j, "picture_url"),
        endorsements: n(j.get("endorsement_count")) as u64,
        downloads: n(j.get("mod_downloads")) as u64,
        adult: b(j, "contains_adult_content"),
        available: j.get("available").and_then(Json::as_bool).unwrap_or(true),
        status: s(j, "status"),
        updated_timestamp: n(j.get("updated_timestamp")) as u64,
    })
}

/// Reads games/<domain>/mods/<id>.json.
pub fn parse_mod(text: &str) -> Result<ModInfo, String> {
    mod_from_json(&parse(text)?).ok_or_else(|| "Nexus's answer has no mod in it".to_string())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileInfo {
    pub file_id: u64,
    pub name: String,
    pub version: String,
    /// "MAIN", "UPDATE", "OPTIONAL", "OLD_VERSION", "MISCELLANEOUS", "ARCHIVED" or "DELETED".
    pub category: String,
    pub file_name: String,
    pub size_bytes: u64,
    pub uploaded_timestamp: u64,
    pub is_primary: bool,
    pub description: String,
}

impl FileInfo {
    /// Files a player would normally pick (not old, archived or deleted).
    pub fn is_current(&self) -> bool {
        !matches!(self.category.as_str(), "OLD_VERSION" | "ARCHIVED" | "DELETED" | "REMOVED")
    }

    /// Removed by the author: listed by Nexus but not downloadable.
    pub fn is_removed(&self) -> bool {
        matches!(self.category.as_str(), "DELETED" | "REMOVED")
    }
}

/// Plain headings for Nexus's file categories.
pub fn category_label(category: &str) -> &'static str {
    match category {
        "MAIN" => "Main files",
        "UPDATE" => "Updates",
        "OPTIONAL" => "Optional files",
        "MISCELLANEOUS" => "Other files",
        "OLD_VERSION" => "Old versions",
        "ARCHIVED" => "Archived",
        "DELETED" | "REMOVED" => "Removed",
        _ => "Other files",
    }
}

/// One heading of the "Other versions" list and its files, newest first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileGroup {
    /// Nexus's category ("MAIN", "OPTIONAL", ...).
    pub category: String,
    pub label: &'static str,
    pub files: Vec<FileInfo>,
}

/// The order the categories are listed in.
const GROUP_ORDER: &[&str] = &["MAIN", "UPDATE", "OPTIONAL", "MISCELLANEOUS", "OLD_VERSION", "ARCHIVED"];

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FileUpdate {
    pub old_file_id: u64,
    pub new_file_id: u64,
    pub uploaded_timestamp: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FileList {
    pub files: Vec<FileInfo>,
    /// Author-set "this file replaces that one" links.
    pub updates: Vec<FileUpdate>,
}

impl FileList {
    /// Follows the replacement links from `file_id` to the newest file that replaces it.
    pub fn newest_replacement(&self, file_id: u64) -> Option<&FileInfo> {
        let mut cur = file_id;
        let mut seen = vec![cur];
        while let Some(u) = self.updates.iter().filter(|u| u.old_file_id == cur).max_by_key(|u| u.uploaded_timestamp) {
            if seen.contains(&u.new_file_id) {
                break;
            }
            cur = u.new_file_id;
            seen.push(cur);
        }
        (cur != file_id).then(|| self.files.iter().find(|f| f.file_id == cur)).flatten()
    }

    /// The main file to offer: the primary one, else the newest MAIN file.
    pub fn main_file(&self) -> Option<&FileInfo> {
        self.files
            .iter()
            .filter(|f| f.is_current())
            .find(|f| f.is_primary)
            .or_else(|| self.files.iter().filter(|f| f.category == "MAIN").max_by_key(|f| f.uploaded_timestamp))
    }

    /// The file the mod page offers: the newest current MAIN file; if there is none, the newest file that can still
    /// be downloaded.
    pub fn newest_main(&self) -> Option<&FileInfo> {
        self.files
            .iter()
            .filter(|f| f.category == "MAIN")
            .max_by_key(|f| (f.uploaded_timestamp, f.file_id))
            .or_else(|| self.files.iter().filter(|f| !f.is_removed()).max_by_key(|f| (f.uploaded_timestamp, f.file_id)))
    }

    /// Every downloadable file of the mod, grouped by category (main, updates, optional, other, old versions,
    /// archived), newest first in each group. Removed files are left out; empty groups too.
    pub fn groups(&self) -> Vec<FileGroup> {
        let mut out: Vec<FileGroup> = Vec::new();
        let rank = |c: &str| GROUP_ORDER.iter().position(|g| *g == c).unwrap_or(GROUP_ORDER.len());
        let mut files: Vec<&FileInfo> = self.files.iter().filter(|f| !f.is_removed()).collect();
        files.sort_by(|a, b| {
            rank(&a.category).cmp(&rank(&b.category)).then(b.uploaded_timestamp.cmp(&a.uploaded_timestamp)).then(b.file_id.cmp(&a.file_id))
        });
        for f in files {
            // Unknown categories are shown under "Other files" with the miscellaneous ones.
            let cat = if rank(&f.category) == GROUP_ORDER.len() { "MISCELLANEOUS" } else { f.category.as_str() };
            match out.iter_mut().find(|g| g.category == cat) {
                Some(g) => g.files.push(f.clone()),
                None => out.push(FileGroup { category: cat.to_string(), label: category_label(cat), files: vec![f.clone()] }),
            }
        }
        out.sort_by_key(|g| rank(&g.category));
        out
    }

    /// The "Other versions" list for an installed file: every group, without the file that is installed.
    pub fn other_versions(&self, installed_file_id: u64) -> Vec<FileGroup> {
        self.groups()
            .into_iter()
            .filter_map(|mut g| {
                g.files.retain(|f| f.file_id != installed_file_id);
                (!g.files.is_empty()).then_some(g)
            })
            .collect()
    }

    pub fn file(&self, file_id: u64) -> Option<&FileInfo> {
        self.files.iter().find(|f| f.file_id == file_id)
    }

    /// Which file to fetch for a wanted file ID: that file while it can still be downloaded, else the newest one
    /// replacing it, else the main file. None (or no wanted ID) gives the main file.
    pub fn pick(&self, wanted: Option<u64>) -> Option<&FileInfo> {
        if let Some(id) = wanted {
            if let Some(f) = self.file(id).filter(|f| !f.is_removed()) {
                return Some(f);
            }
            if let Some(f) = self.newest_replacement(id).filter(|f| !f.is_removed()) {
                return Some(f);
            }
        }
        self.main_file()
    }
}

fn category_name(f: &Json) -> String {
    f.str_at("category_name").filter(|c| !c.is_empty()).map(str::to_string).unwrap_or_else(|| {
        match n(f.get("category_id")) as u64 {
            1 => "MAIN",
            2 => "UPDATE",
            3 => "OPTIONAL",
            4 => "OLD_VERSION",
            5 => "MISCELLANEOUS",
            6 => "DELETED",
            7 => "ARCHIVED",
            _ => "",
        }
        .to_string()
    })
}

fn file_from_json(f: &Json) -> Option<FileInfo> {
    let file_id = n(f.get("file_id")) as u64;
    if file_id == 0 {
        return None;
    }
    let bytes = n(f.get("size_in_bytes")) as u64;
    let kb = n(f.get("size_kb")).max(n(f.get("size"))) as u64;
    Some(FileInfo {
        file_id,
        name: s(f, "name"),
        version: f.str_at("version").filter(|v| !v.is_empty()).or(f.str_at("mod_version")).unwrap_or("").to_string(),
        category: category_name(f),
        file_name: s(f, "file_name"),
        size_bytes: if bytes > 0 { bytes } else { kb * 1024 },
        uploaded_timestamp: n(f.get("uploaded_timestamp")) as u64,
        is_primary: b(f, "is_primary"),
        description: s(f, "description"),
    })
}

/// Reads games/<domain>/mods/<id>/files/<file id>.json (one file).
pub fn parse_file(text: &str) -> Result<FileInfo, String> {
    file_from_json(&parse(text)?).ok_or_else(|| "Nexus's answer has no file in it".to_string())
}

/// Reads games/<domain>/mods/<id>/files.json.
pub fn parse_files(text: &str) -> Result<FileList, String> {
    let j = parse(text)?;
    let files = j.get("files").map(Json::as_array).unwrap_or(&[]).iter().filter_map(file_from_json).collect();
    let updates = j
        .get("file_updates")
        .map(Json::as_array)
        .unwrap_or(&[])
        .iter()
        .map(|u| FileUpdate {
            old_file_id: n(u.get("old_file_id")) as u64,
            new_file_id: n(u.get("new_file_id")) as u64,
            uploaded_timestamp: n(u.get("uploaded_timestamp")) as u64,
        })
        .filter(|u| u.old_file_id > 0 && u.new_file_id > 0)
        .collect();
    Ok(FileList { files, updates })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdatedEntry {
    pub mod_id: u64,
    pub latest_file_update: u64,
    pub latest_mod_activity: u64,
}

/// Reads games/<domain>/mods/updated.json: every mod of the game with a change in the period.
pub fn parse_updated(text: &str) -> Result<Vec<UpdatedEntry>, String> {
    let j = parse(text)?;
    if !matches!(j, Json::Arr(_)) {
        return Err("Nexus's update list isn't a list".into());
    }
    Ok(j.as_array()
        .iter()
        .map(|e| UpdatedEntry {
            mod_id: n(e.get("mod_id")) as u64,
            latest_file_update: n(e.get("latest_file_update")) as u64,
            latest_mod_activity: n(e.get("latest_mod_activity")) as u64,
        })
        .filter(|e| e.mod_id > 0)
        .collect())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DownloadLink {
    pub name: String,
    pub short_name: String,
    pub uri: String,
}

/// Reads .../download_link.json: a list of mirrors, the preferred one first.
pub fn parse_download_links(text: &str) -> Result<Vec<DownloadLink>, String> {
    let j = parse(text)?;
    let links: Vec<DownloadLink> = j
        .as_array()
        .iter()
        .filter_map(|l| {
            let uri = l.str_at("URI")?;
            net::allowed_url(uri).then(|| DownloadLink { name: s(l, "name"), short_name: s(l, "short_name"), uri: uri.to_string() })
        })
        .collect();
    if links.is_empty() {
        return Err("Nexus sent no download address Turbine can use".into());
    }
    Ok(links)
}

// ---- GraphQL search (v2) ----

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SearchSort {
    #[default]
    Endorsements,
    Downloads,
    Updated,
    Name,
    /// Most endorsed among the mods updated in the last 30 days. The v2 schema has no trending sort of its own, so
    /// this is the endorsements sort with an updatedAt filter; if Nexus refuses the filter, `search` asks again as
    /// Endorsements (and says so in `SearchPage::relaxed`).
    Trending,
    /// Newest first (createdAt, newest first); refused, it falls back to Endorsements the same way.
    Newest,
}

/// The days Trending looks back over.
pub const TRENDING_DAYS: u64 = 30;

impl SearchSort {
    /// Reads the app's sort names: "trending", "endorsements", "downloads", "updated", "newest", "name".
    pub fn from_key(key: &str) -> SearchSort {
        match key {
            "trending" => SearchSort::Trending,
            "downloads" => SearchSort::Downloads,
            "updated" => SearchSort::Updated,
            "newest" => SearchSort::Newest,
            "name" => SearchSort::Name,
            _ => SearchSort::Endorsements,
        }
    }

    fn field(self) -> (&'static str, &'static str) {
        match self {
            SearchSort::Trending => ("endorsements", "DESC"),
            SearchSort::Newest => ("createdAt", "DESC"),
            SearchSort::Endorsements => ("endorsements", "DESC"),
            SearchSort::Downloads => ("downloads", "DESC"),
            SearchSort::Updated => ("updatedAt", "DESC"),
            SearchSort::Name => ("name", "ASC"),
        }
    }
}

// facetsData with `facets: {categoryName: []}` lists the categories of everything the filter matches, with counts;
// `categoryName` in the filter keeps one of them (both answered without a key: the build's live check, Nov 7 2026).
const SEARCH_QUERY: &str = "query TurbineMods($filter: ModsFilter, $sort: [ModsSort!], $offset: Int, $count: Int) { \
mods(filter: $filter, sort: $sort, offset: $offset, count: $count, facets: {categoryName: []}) { totalCount facetsData \
nodes { modId name summary version author endorsements downloads pictureUrl thumbnailUrl adultContent updatedAt \
category } } }";

/// Trending's updatedAt filter value: seconds since 1970, at the start of that day (UTC). Nexus puts the value into a
/// search query as it is: a full time's ':' is read as the start of another term (the whole search refused with a 400
/// parse_exception), and a plain day ("2026-10-08") is read but matches nothing. Seconds match (the build's live
/// check, Nov 7 2026: 2,002 Witcher 3 mods updated in 30 days).
pub fn trending_since(now: u64) -> String {
    let since = now.saturating_sub(TRENDING_DAYS * 86_400);
    (since - since % 86_400).to_string()
}

/// "2026-11-07" for seconds since 1970 (UTC).
pub fn date_day(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    // Civil date from days since 1970 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// The JSON body for a search: mods for one game, optionally by name (wildcard match) and in one category ("" for
/// all), adult mods left out unless asked for, sorted (most endorsed first by default). Trending's 30 days are
/// counted back from `now`.
#[allow(clippy::too_many_arguments)]
pub fn search_body_in(domain: &str, text: &str, category: &str, sort: SearchSort, offset: u32, count: u32, adult: bool, now: u64) -> String {
    let mut filter = format!("\"op\":\"AND\",\"gameDomainName\":[{{\"value\":{},\"op\":\"EQUALS\"}}]", quote(domain));
    let t = text.trim();
    if !t.is_empty() {
        filter.push_str(&format!(",\"name\":[{{\"value\":{},\"op\":\"WILDCARD\"}}]", quote(t)));
    }
    if !category.trim().is_empty() {
        filter.push_str(&format!(",\"categoryName\":[{{\"value\":{},\"op\":\"EQUALS\"}}]", quote(category.trim())));
    }
    if !adult {
        filter.push_str(",\"adultContent\":[{\"value\":false,\"op\":\"EQUALS\"}]");
    }
    if sort == SearchSort::Trending {
        let since = trending_since(now);
        filter.push_str(&format!(",\"updatedAt\":[{{\"value\":{},\"op\":\"GTE\"}}]", quote(&since)));
    }
    let (field, dir) = sort.field();
    format!(
        "{{\"query\":{},\"variables\":{{\"filter\":{{{filter}}},\"sort\":[{{\"{field}\":{{\"direction\":\"{dir}\"}}}}],\"offset\":{offset},\"count\":{}}}}}",
        quote(SEARCH_QUERY),
        count.clamp(1, 100)
    )
}

#[derive(Clone, Debug, PartialEq)]
pub struct SearchMod {
    pub mod_id: u64,
    pub name: String,
    pub summary: String,
    pub version: String,
    pub author: String,
    pub endorsements: u64,
    pub downloads: u64,
    pub picture_url: String,
    pub thumbnail_url: String,
    pub adult: bool,
    pub updated_at: String,
    /// Its Nexus category ("Gameplay"); "" when not given.
    pub category: String,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct SearchPage {
    pub total: u64,
    pub mods: Vec<SearchMod>,
    /// The categories of everything the search matches, most mods first: (name, how many).
    pub categories: Vec<(String, u64)>,
    /// Nexus refused the asked-for sort (Trending or Newest) and this page is Most endorsed instead.
    pub relaxed: bool,
}

/// Reads a GraphQL search answer. GraphQL errors become one plain sentence.
pub fn parse_search(text: &str) -> Result<SearchPage, String> {
    let j = parse(text)?;
    if let Some(errs) = j.get("errors").map(Json::as_array).filter(|e| !e.is_empty()) {
        let msg = errs[0].str_at("message").unwrap_or("unknown error");
        if j.get("data").and_then(|d| d.get("mods")).is_none() {
            return Err(format!("Nexus couldn't search ({msg})"));
        }
    }
    let mods = j.get("data").and_then(|d| d.get("mods")).ok_or("Nexus's answer has no results")?;
    let mut categories: Vec<(String, u64)> = mods
        .get("facetsData")
        .and_then(|f| f.get("categoryName"))
        .map(Json::entries)
        .unwrap_or(&[])
        .iter()
        .map(|(k, v)| (k.trim().to_string(), n(Some(v)) as u64))
        .filter(|(k, c)| !k.is_empty() && *c > 0)
        .collect();
    categories.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    Ok(SearchPage {
        relaxed: false,
        categories,
        total: n(mods.get("totalCount")) as u64,
        mods: mods
            .get("nodes")
            .map(Json::as_array)
            .unwrap_or(&[])
            .iter()
            .filter_map(|m| {
                let mod_id = n(m.get("modId")) as u64;
                (mod_id > 0).then(|| SearchMod {
                    mod_id,
                    name: s(m, "name"),
                    summary: s(m, "summary"),
                    version: s(m, "version"),
                    author: s(m, "author"),
                    endorsements: n(m.get("endorsements")) as u64,
                    downloads: n(m.get("downloads")) as u64,
                    picture_url: s(m, "pictureUrl"),
                    thumbnail_url: s(m, "thumbnailUrl"),
                    adult: b(m, "adultContent"),
                    updated_at: s(m, "updatedAt"),
                    category: s(m, "category"),
                })
            })
            .collect(),
    })
}

/// The id Turbine records for a Nexus file: "<domain>:<mod id>:<file id>".
pub fn source_id(domain: &str, mod_id: u64, file_id: u64) -> String {
    format!("{domain}:{mod_id}:{file_id}")
}

/// Reads an id made by `source_id`.
pub fn parse_source_id(id: &str) -> Option<(String, u64, u64)> {
    let mut p = id.split(':');
    let (d, m, f) = (p.next()?, p.next()?.parse().ok()?, p.next()?.parse().ok()?);
    (p.next().is_none() && !d.is_empty()).then(|| (d.to_string(), m, f))
}

pub fn mod_page_url(domain: &str, mod_id: u64) -> String {
    format!("https://www.nexusmods.com/{domain}/mods/{mod_id}")
}

/// The mod's Files tab, where a free account presses Mod Manager Download.
pub fn files_page_url(domain: &str, mod_id: u64) -> String {
    format!("https://www.nexusmods.com/{domain}/mods/{mod_id}?tab=files")
}

/// The download page of one file.
pub fn file_page_url(domain: &str, mod_id: u64, file_id: u64) -> String {
    format!("https://www.nexusmods.com/{domain}/mods/{mod_id}?tab=files&file_id={file_id}")
}

/// Reads a Nexus mod page address ("https://www.nexusmods.com/<domain>/mods/<id>", with or without "www." and
/// anything after the ID) into its domain and mod ID.
pub fn parse_mod_page_url(url: &str) -> Option<(String, u64)> {
    let rest = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://"))?;
    let (host, path) = rest.split_once('/')?;
    if !host.eq_ignore_ascii_case("www.nexusmods.com") && !host.eq_ignore_ascii_case("nexusmods.com") {
        return None;
    }
    let mut p = path.split(['/', '?', '#']);
    let (domain, m, id) = (p.next()?, p.next()?, p.next()?);
    let ok = !domain.is_empty() && domain.len() <= 64 && domain.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit());
    let id: u64 = id.parse().ok().filter(|i| *i > 0 && id.bytes().all(|b| b.is_ascii_digit()))?;
    (ok && m == "mods").then(|| (domain.to_string(), id))
}

// ---- Requirements (v2 GraphQL) ----

/// One thing a mod needs: another Nexus mod, something outside Nexus (`external`, see `url`), or a DLC of the game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Requirement {
    pub name: String,
    /// 0 for DLC and for requirements outside Nexus.
    pub mod_id: u64,
    /// The required mod's game domain (usually the same game). Empty when it couldn't be worked out.
    pub domain: String,
    /// The page to open: the mod's Nexus page, or the outside address. Empty for DLC. Only http(s) addresses.
    pub url: String,
    pub notes: String,
    pub is_dlc: bool,
    pub external: bool,
}

// Nexus refuses a query whose complexity is over 10000 (the owner's device, Nov 7 2026: the details query with
// requirements in it came to 18440). Each query here asks for one thing, with small list counts; the build's live
// Nexus check sends these exact bodies and prints Nexus's answer. `MAX_REQUIREMENTS` keeps the list short.
pub const MAX_REQUIREMENTS: u32 = 20;

const REQUIREMENTS_QUERY: &str = "query TurbineRequirements($ids: [CompositeDomainWithIdInput!]!) { \
legacyModsByDomain(ids: $ids) { nodes { game { id domainName } modRequirements { \
nexusRequirements(count: 20) { nodes { modId modName gameId url notes externalRequirement } } \
dlcRequirements { notes gameExpansion { name } } } } } }";

/// The JSON body asking for a mod's requirements.
pub fn requirements_body(domain: &str, mod_id: u64) -> String {
    format!(
        "{{\"query\":{},\"variables\":{{\"ids\":[{{\"gameDomain\":{},\"modId\":{mod_id}}}]}}}}",
        quote(REQUIREMENTS_QUERY),
        quote(domain)
    )
}

fn clean_text(t: &str, max: usize) -> String {
    let t: String = t.chars().filter(|c| !c.is_control() || *c == '\n').collect();
    t.trim().chars().take(max).collect()
}

/// An address that is safe to hand to the browser: http(s), no spaces or control characters, not absurdly long.
fn browser_url(u: &str) -> Option<String> {
    let u = u.trim();
    let scheme = u.get(..8).is_some_and(|p| p.eq_ignore_ascii_case("https://")) || u.get(..7).is_some_and(|p| p.eq_ignore_ascii_case("http://"));
    (scheme && u.len() <= 2048 && u.bytes().all(|b| b.is_ascii_graphic())).then(|| u.to_string())
}

/// Reads the requirements answer. `domain` is the game the mod belongs to, used for requirements whose address
/// doesn't name a game. GraphQL errors become one plain sentence.
pub fn parse_requirements(text: &str, domain: &str) -> Result<Vec<Requirement>, String> {
    let j = parse(text)?;
    let node = j
        .get("data")
        .and_then(|d| d.get("legacyModsByDomain"))
        .and_then(|m| m.get("nodes"))
        .and_then(|list| list.as_array().first());
    let Some(node) = node else {
        if let Some(e) = j.get("errors").map(Json::as_array).and_then(|e| e.first()) {
            return Err(format!("Nexus couldn't list the requirements ({})", e.str_at("message").unwrap_or("unknown error")));
        }
        return Err("Nexus doesn't know that mod".into());
    };
    let own_game = n(node.get("game").and_then(|g| g.get("id")));
    let own_domain = node.get("game").and_then(|g| g.str_at("domainName")).filter(|d| !d.is_empty()).unwrap_or(domain);
    let reqs = node.get("modRequirements");
    let mut out = Vec::new();
    for r in reqs.and_then(|r| r.get("nexusRequirements")).and_then(|p| p.get("nodes")).map(Json::as_array).unwrap_or(&[]) {
        let external = b(r, "externalRequirement");
        let name = clean_text(r.str_at("modName").unwrap_or(""), 200);
        let notes = clean_text(r.str_at("notes").unwrap_or(""), 2000);
        let raw_url = r.str_at("url").unwrap_or("");
        if external {
            out.push(Requirement { name, mod_id: 0, domain: String::new(), url: browser_url(raw_url).unwrap_or_default(), notes, is_dlc: false, external: true });
            continue;
        }
        let mod_id = n(r.get("modId")) as u64;
        let (req_domain, mod_id) = match parse_mod_page_url(raw_url) {
            Some((d, id)) => (d, id),
            None if mod_id > 0 && (n(r.get("gameId")) == own_game || r.get("gameId").is_none()) => (own_domain.to_string(), mod_id),
            None => (String::new(), mod_id),
        };
        if mod_id == 0 {
            continue;
        }
        let url = if req_domain.is_empty() { browser_url(raw_url).unwrap_or_default() } else { mod_page_url(&req_domain, mod_id) };
        out.push(Requirement { name, mod_id, domain: req_domain, url, notes, is_dlc: false, external: false });
    }
    for d in reqs.and_then(|r| r.get("dlcRequirements")).map(Json::as_array).unwrap_or(&[]) {
        let name = clean_text(d.get("gameExpansion").and_then(|g| g.str_at("name")).unwrap_or(""), 200);
        if name.is_empty() {
            continue;
        }
        out.push(Requirement {
            name,
            mod_id: 0,
            domain: own_domain.to_string(),
            url: String::new(),
            notes: clean_text(d.str_at("notes").unwrap_or(""), 2000),
            is_dlc: true,
            external: false,
        });
    }
    Ok(out)
}

// ---- A mod's details and files without a key (v2 GraphQL) ----
//
// legacyModsByDomain answers a mod's page facts; its requirements are a request of their own (requirements_body),
// so neither query comes near Nexus's complexity limit. modFiles needs the game's numeric id, which the details answer carries (game.id).
// Field names follow Nexus's published v2 schema (api.nexusmods.com/v2/graphql): ModFile has fileId, name, version,
// category (MAIN, UPDATE, OPTIONAL, OLD_VERSION, MISCELLANEOUS, ARCHIVED, REMOVED), date (seconds since 1970),
// size (KB), sizeInBytes (a string) and primary.

const DETAILS_QUERY: &str = "query TurbineMod($ids: [CompositeDomainWithIdInput!]!) { \
legacyModsByDomain(ids: $ids) { nodes { modId name summary version author pictureUrl thumbnailUrl endorsements \
downloads adultContent status updatedAt game { id domainName } } } }";

const FILES_QUERY: &str = "query TurbineFiles($modId: ID!, $gameId: ID!) { modFiles(modId: $modId, gameId: $gameId) { \
fileId name version category date size sizeInBytes primary description } }";

/// The JSON body asking for a mod's details (requirements are asked apart: `requirements_body`).
pub fn details_body(domain: &str, mod_id: u64) -> String {
    format!("{{\"query\":{},\"variables\":{{\"ids\":[{{\"gameDomain\":{},\"modId\":{mod_id}}}]}}}}", quote(DETAILS_QUERY), quote(domain))
}

/// The JSON body asking for a mod's files (`game_id`: Nexus's number for the game, from the details).
pub fn files_body(game_id: u64, mod_id: u64) -> String {
    format!("{{\"query\":{},\"variables\":{{\"modId\":\"{mod_id}\",\"gameId\":\"{game_id}\"}}}}", quote(FILES_QUERY))
}

fn graphql_error(j: &Json, what: &str) -> Option<String> {
    let e = j.get("errors").map(Json::as_array).and_then(|e| e.first())?;
    Some(format!("Nexus couldn't {what} ({})", e.str_at("message").unwrap_or("unknown error")))
}

/// Seconds since 1970 for "2025-01-01T00:00:00Z" (the date part and the time; 0 when it can't be read).
/// One mod's current version and last change, from `legacyModsByDomain`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModStamp {
    pub mod_id: u64,
    pub name: String,
    pub version: String,
    /// Seconds since 1970 (0 when not given).
    pub updated: u64,
    /// Nexus's number for the game (for the files query).
    pub game_id: u64,
    /// 0.6 (intent I16): the mod's own words, so a mod added from a file gets its Nexus name, author, summary and
    /// picture without a key ("" when not given).
    pub author: String,
    pub summary: String,
    pub picture: String,
}

/// `legacyModsByDomain(ids: [{gameDomain, modId}])` answers with those mods (checked live without a key in the
/// build's Nexus check, Nov 7 2026).
pub fn by_id_body(domain: &str, ids: &[u64]) -> String {
    let list: Vec<String> = ids.iter().map(|id| format!("{{gameDomain: {}, modId: {id}}}", quote(domain))).collect();
    let q = format!("{{ legacyModsByDomain(ids: [{}]) {{ nodes {{ modId name version updatedAt gameId author summary thumbnailUrl }} }} }}", list.join(", "));
    format!("{{\"query\":{}}}", quote(&q))
}

pub fn parse_by_id(text: &str) -> Result<Vec<ModStamp>, String> {
    let j = parse(text)?;
    let nodes = j.get("data").and_then(|d| d.get("legacyModsByDomain")).and_then(|m| m.get("nodes"));
    let Some(nodes) = nodes else {
        let msg = j.get("errors").map(Json::as_array).and_then(|e| e.first()).and_then(|e| e.str_at("message")).unwrap_or("no answer");
        return Err(format!("Nexus couldn't look those mods up ({msg})"));
    };
    Ok(nodes
        .as_array()
        .iter()
        .filter_map(|m| {
            let mod_id = n(m.get("modId")) as u64;
            (mod_id > 0).then(|| ModStamp {
                mod_id,
                name: s(m, "name"),
                version: s(m, "version"),
                updated: iso_secs(&s(m, "updatedAt")),
                game_id: n(m.get("gameId")) as u64,
                author: clean_text(&s(m, "author"), 200),
                summary: clean_text(&s(m, "summary"), 2000),
                picture: browser_url(&s(m, "thumbnailUrl")).unwrap_or_default(),
            })
        })
        .collect())
}

pub fn iso_secs(t: &str) -> u64 {
    let b = t.as_bytes();
    let num = |a: usize, z: usize| t.get(a..z).and_then(|x| x.parse::<i64>().ok());
    let (Some(y), Some(m), Some(d)) = (num(0, 4), num(5, 7), num(8, 10)) else { return 0 };
    if b.len() < 10 || b[4] != b'-' || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return 0;
    }
    let (hh, mm, ss) = (num(11, 13).unwrap_or(0), num(14, 16).unwrap_or(0), num(17, 19).unwrap_or(0));
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    (days * 86_400 + hh * 3600 + mm * 60 + ss).max(0) as u64
}

/// A mod's page facts from the details answer, with Nexus's number for its game.
#[derive(Clone, Debug, PartialEq)]
pub struct ModDetails {
    pub info: ModInfo,
    pub game_id: u64,
    pub thumbnail_url: String,
}

/// Reads the details answer (`details_body`). Requirements come from the same text through `parse_requirements`.
pub fn parse_mod_details(text: &str, domain: &str) -> Result<ModDetails, String> {
    let j = parse(text)?;
    let node = j.get("data").and_then(|d| d.get("legacyModsByDomain")).and_then(|m| m.get("nodes")).and_then(|l| l.as_array().first());
    let Some(m) = node else {
        return Err(graphql_error(&j, "read the mod").unwrap_or_else(|| "Nexus doesn't know that mod".into()));
    };
    let mod_id = n(m.get("modId")) as u64;
    if mod_id == 0 {
        return Err("Nexus's answer has no mod in it".into());
    }
    let status = s(m, "status");
    let updated = s(m, "updatedAt");
    Ok(ModDetails {
        info: ModInfo {
            mod_id,
            domain: m.get("game").and_then(|g| g.str_at("domainName")).filter(|d| !d.is_empty()).unwrap_or(domain).to_string(),
            name: s(m, "name"),
            summary: s(m, "summary"),
            version: s(m, "version"),
            author: s(m, "author"),
            picture_url: s(m, "pictureUrl"),
            endorsements: n(m.get("endorsements")) as u64,
            downloads: n(m.get("downloads")) as u64,
            adult: b(m, "adultContent"),
            available: status.is_empty() || status.eq_ignore_ascii_case("published"),
            status,
            updated_timestamp: iso_secs(&updated),
        },
        game_id: n(m.get("game").and_then(|g| g.get("id"))) as u64,
        thumbnail_url: s(m, "thumbnailUrl"),
    })
}

/// Reads the files answer (`files_body`) into the same list v1 gives (without its replacement links).
pub fn parse_mod_files(text: &str) -> Result<FileList, String> {
    let j = parse(text)?;
    let Some(list) = j.get("data").and_then(|d| d.get("modFiles")) else {
        return Err(graphql_error(&j, "list the files").unwrap_or_else(|| "Nexus's answer has no files in it".into()));
    };
    let files = list
        .as_array()
        .iter()
        .filter_map(|f| {
            let file_id = n(f.get("fileId")) as u64;
            if file_id == 0 {
                return None;
            }
            let cat = s(f, "category").to_ascii_uppercase();
            let bytes = n(f.get("sizeInBytes")) as u64;
            Some(FileInfo {
                file_id,
                name: s(f, "name"),
                version: s(f, "version"),
                category: if cat == "REMOVED" { "DELETED".into() } else { cat },
                file_name: String::new(),
                size_bytes: if bytes > 0 { bytes } else { (n(f.get("size")) as u64) * 1024 },
                uploaded_timestamp: n(f.get("date")) as u64,
                is_primary: n(f.get("primary")) > 0.0 || b(f, "primary"),
                description: s(f, "description"),
            })
        })
        .collect();
    Ok(FileList { files, updates: Vec::new() })
}

/// A mod's requirements sorted against what is installed.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RequirementCheck {
    /// Nexus mods that aren't installed.
    pub missing: Vec<Requirement>,
    /// Nexus mods that are installed.
    pub installed: Vec<Requirement>,
    /// DLC and things outside Nexus: Turbine can't tell whether they're there, so they're listed for the player.
    pub unchecked: Vec<Requirement>,
}

/// The (domain, mod ID) of an installed Nexus mod. Old records hold only the mod ID; their domain is empty.
pub fn source_mod(src: &SourceInfo) -> Option<(String, u64)> {
    if src.kind != "nexus" {
        return None;
    }
    if let Some((d, m, _)) = parse_source_id(&src.id) {
        return Some((d, m));
    }
    let mut p = src.id.split(':');
    match (p.next(), p.next(), p.next()) {
        (Some(d), Some(m), None) => m.parse().ok().map(|m| (d.to_string(), m)),
        (Some(m), None, None) => m.parse().ok().map(|m| (String::new(), m)),
        _ => None,
    }
}

/// Which of a mod's requirements are missing, given where the installed mods came from (as
/// `Store::installed_sources` returns them).
pub fn check_requirements(reqs: &[Requirement], installed: &[(String, SourceInfo)]) -> RequirementCheck {
    let have: Vec<(String, u64)> = installed.iter().filter_map(|(_, s)| source_mod(s)).collect();
    let mut c = RequirementCheck::default();
    for r in reqs {
        if r.is_dlc || r.external || r.mod_id == 0 {
            c.unchecked.push(r.clone());
        } else if have.iter().any(|(d, m)| *m == r.mod_id && (d.is_empty() || r.domain.is_empty() || *d == r.domain)) {
            c.installed.push(r.clone());
        } else {
            c.missing.push(r.clone());
        }
    }
    c
}

/// What to do about one missing requirement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RequirementStep {
    /// Premium: Turbine downloads the mod's main file itself (`Nexus::download_by_id` with no file ID).
    Download { name: String, domain: String, mod_id: u64 },
    /// Free accounts (or a requirement Turbine can't fetch): open this page in the browser. Files fetched from it
    /// with Mod Manager Download come back as nxm links.
    OpenPage { name: String, url: String },
}

/// The steps for the missing requirements. Requirements for games Turbine doesn't support are always pages.
pub fn requirement_steps(missing: &[Requirement], premium: bool) -> Vec<RequirementStep> {
    missing
        .iter()
        .filter(|r| !r.is_dlc && !r.external)
        .filter_map(|r| {
            if premium && known_domain(&r.domain) && r.mod_id > 0 {
                Some(RequirementStep::Download { name: r.name.clone(), domain: r.domain.clone(), mod_id: r.mod_id })
            } else if !r.domain.is_empty() && r.mod_id > 0 {
                Some(RequirementStep::OpenPage { name: r.name.clone(), url: files_page_url(&r.domain, r.mod_id) })
            } else {
                (!r.url.is_empty()).then(|| RequirementStep::OpenPage { name: r.name.clone(), url: r.url.clone() })
            }
        })
        .collect()
}

fn requirement_json(r: &Requirement) -> String {
    format!(
        "{{\"name\":{},\"modId\":{},\"domain\":{},\"url\":{},\"notes\":{},\"isDlc\":{},\"external\":{}}}",
        quote(&r.name),
        r.mod_id,
        quote(&r.domain),
        quote(&r.url),
        quote(&r.notes),
        r.is_dlc,
        r.external
    )
}

/// The check and the steps as JSON for the app:
/// {"missing":[..],"installed":[..],"unchecked":[..],"steps":[{"action":"download"|"open","name","domain","modId","url"}]}.
pub fn requirements_json(check: &RequirementCheck, premium: bool) -> String {
    let list = |v: &[Requirement]| v.iter().map(requirement_json).collect::<Vec<_>>().join(",");
    let steps: Vec<String> = requirement_steps(&check.missing, premium)
        .iter()
        .map(|s| match s {
            RequirementStep::Download { name, domain, mod_id } => format!(
                "{{\"action\":\"download\",\"name\":{},\"domain\":{},\"modId\":{mod_id},\"url\":\"\"}}",
                quote(name),
                quote(domain)
            ),
            RequirementStep::OpenPage { name, url } => {
                format!("{{\"action\":\"open\",\"name\":{},\"domain\":\"\",\"modId\":0,\"url\":{}}}", quote(name), quote(url))
            }
        })
        .collect();
    format!(
        "{{\"missing\":[{}],\"installed\":[{}],\"unchecked\":[{}],\"steps\":[{}]}}",
        list(&check.missing),
        list(&check.installed),
        list(&check.unchecked),
        steps.join(",")
    )
}

pub fn source_info(domain: &str, mod_id: u64, file: &FileInfo, sha256: Option<String>) -> SourceInfo {
    SourceInfo {
        kind: "nexus".into(),
        id: source_id(domain, mod_id, file.file_id),
        version: (!file.version.is_empty()).then(|| file.version.clone()),
        url: Some(mod_page_url(domain, mod_id)),
        sha256,
        needs: Vec::new(),
        title: None,
        picture: None,
    }
}

// ---- Network ----

/// A Nexus session, with the player's key or without one (`public`: browsing only). Remembers the last rate limit
/// headers it saw and stops before a call that would be refused; after a "too many requests" answer it waits
/// `BACKOFF_SECS` before asking again.
pub struct Nexus {
    pub client: Client,
    key: Option<String>,
    app_version: String,
    pub limits: Cell<RateLimit>,
    /// No requests before this time (seconds since 1970) after a 429.
    pub backoff_until: Cell<u64>,
}

impl Nexus {
    pub fn new(client: Client, key: &str, app_version: &str) -> Nexus {
        Nexus::with_key(client, Some(key), app_version)
    }

    /// Browsing without a key: search, a mod's details, requirements and files (v2 GraphQL).
    pub fn public(client: Client, app_version: &str) -> Nexus {
        Nexus::with_key(client, None, app_version)
    }

    pub fn with_key(client: Client, key: Option<&str>, app_version: &str) -> Nexus {
        Nexus {
            client,
            key: key.map(|k| k.trim().to_string()).filter(|k| !k.is_empty()),
            app_version: app_version.into(),
            limits: Cell::new(RateLimit::default()),
            backoff_until: Cell::new(0),
        }
    }

    pub fn has_key(&self) -> bool {
        self.key.is_some()
    }

    fn request(&self, req: Request) -> Result<Response, NetError> {
        let current = self.limits.take();
        let blocked = current.blocked();
        self.limits.set(current);
        if let Some(why) = blocked {
            return Err(NetError::Failed(why));
        }
        let now = nxm::now();
        if self.backoff_until.get() > now {
            return Err(NetError::RateLimited);
        }
        let mut req = req;
        let hs = match &self.key {
            Some(k) => headers(k, &self.app_version),
            None => headers_public(&self.app_version),
        };
        for (k, v) in hs {
            req = req.header(&k, &v);
        }
        let resp = self.client.send(&req, None);
        match &resp {
            Ok(r) => {
                let rl = rate_limit(&r.headers);
                if rl.hourly_remaining.is_some() || rl.daily_remaining.is_some() {
                    self.limits.set(rl);
                }
            }
            Err(NetError::RateLimited) => self.backoff_until.set(now + BACKOFF_SECS),
            Err(_) => {}
        }
        resp
    }

    fn get(&self, path: &str) -> Result<String, NetError> {
        if self.key.is_none() {
            return Err(NetError::Failed(NEEDS_KEY.into()));
        }
        self.request(Request::get(&format!("{API}{path}")).max_bytes(32 << 20).max_time(60))?.take_text()
    }

    fn check_domain(domain: &str) -> Result<(), NetError> {
        if known_domain(domain) {
            Ok(())
        } else {
            Err(NetError::Failed(format!("Turbine doesn't support Nexus mods for {domain} yet.")))
        }
    }

    /// Checks the key and returns the account. A refused key gets a plain message.
    pub fn validate(&self) -> Result<Account, NetError> {
        if !self.key.as_deref().is_some_and(plausible_key) {
            return Err(NetError::Failed("That doesn't look like a Nexus API key. Copy it again from your Nexus account page.".into()));
        }
        let text = self.get("/v1/users/validate.json").map_err(|e| match e {
            NetError::Denied(_) => NetError::Failed("Nexus didn't accept that API key. Copy it again from your Nexus account page.".into()),
            e => e,
        })?;
        parse_validate(&text).map_err(|e| NetError::Failed(format!("{e}.")))
    }

    pub fn mod_info(&self, domain: &str, mod_id: u64) -> Result<ModInfo, NetError> {
        Self::check_domain(domain)?;
        let text = self.get(&format!("/v1/games/{domain}/mods/{mod_id}.json")).map_err(refused_words)?;
        parse_mod(&text).map_err(|e| NetError::Failed(format!("{e}.")))
    }

    pub fn files(&self, domain: &str, mod_id: u64) -> Result<FileList, NetError> {
        Self::check_domain(domain)?;
        let text = self.get(&format!("/v1/games/{domain}/mods/{mod_id}/files.json")).map_err(refused_words)?;
        parse_files(&text).map_err(|e| NetError::Failed(format!("{e}.")))
    }

    /// One request per game for every mod changed in `period` ("1d", "1w" or "1m").
    pub fn updated(&self, domain: &str, period: &str) -> Result<Vec<UpdatedEntry>, NetError> {
        Self::check_domain(domain)?;
        if !matches!(period, "1d" | "1w" | "1m") {
            return Err(NetError::Failed("Nexus only lists updates for a day, a week or a month.".into()));
        }
        let text = self.get(&format!("/v1/games/{domain}/mods/updated.json?period={period}")).map_err(refused_words)?;
        parse_updated(&text).map_err(|e| NetError::Failed(format!("{e}.")))
    }

    /// Download addresses for a file. Only Premium accounts get them through an app; free accounts get
    /// `FREE_ACCOUNT_MESSAGE` without a request being made.
    pub fn download_links(&self, account: &Account, domain: &str, mod_id: u64, file_id: u64) -> Result<Vec<DownloadLink>, NetError> {
        Self::check_domain(domain)?;
        if !account.is_premium {
            return Err(NetError::Failed(FREE_ACCOUNT_MESSAGE.into()));
        }
        let text = self.get(&format!("/v1/games/{domain}/mods/{mod_id}/files/{file_id}/download_link.json")).map_err(|e| match e {
            NetError::Denied(_) => NetError::Failed(FREE_ACCOUNT_MESSAGE.into()),
            e => e,
        })?;
        parse_download_links(&text).map_err(|e| NetError::Failed(format!("{e}.")))
    }

    /// Downloads a file (Premium only) and checks its size against Nexus's file list. Nexus publishes no SHA-256, so
    /// the SourceInfo carries the one worked out here.
    pub fn download(
        &self,
        account: &Account,
        domain: &str,
        mod_id: u64,
        file: &FileInfo,
        progress: Option<Progress>,
    ) -> Result<(Download, SourceInfo), NetError> {
        let links = self.download_links(account, domain, mod_id, file.file_id)?;
        self.fetch_checked(&links, domain, mod_id, file, progress)
    }

    /// Downloads from the first mirror and checks the size against the file list.
    fn fetch_checked(
        &self,
        links: &[DownloadLink],
        domain: &str,
        mod_id: u64,
        file: &FileInfo,
        progress: Option<Progress>,
    ) -> Result<(Download, SourceInfo), NetError> {
        let first = links.first().ok_or_else(|| NetError::Failed("Nexus sent no download address Turbine can use.".into()))?;
        // Nexus's sizes are rounded to whole KB on older files, so allow a little slack instead of an exact match.
        let max = if file.size_bytes > 0 { file.size_bytes + 1024 } else { 8 << 30 };
        let d = self.client.download(&first.uri, &Expect::up_to(max), progress)?;
        if file.size_bytes > 0 && d.size + 1024 < file.size_bytes {
            let _ = fs::remove_file(&d.path);
            return Err(NetError::Mismatch(format!("{} instead of {}", net::human_size(d.size), net::human_size(file.size_bytes))));
        }
        let info = source_info(domain, mod_id, file, Some(d.sha256.clone()));
        Ok((d, info))
    }

    /// One file's details (size, version, category).
    pub fn file_info(&self, domain: &str, mod_id: u64, file_id: u64) -> Result<FileInfo, NetError> {
        Self::check_domain(domain)?;
        let text = self.get(&format!("/v1/games/{domain}/mods/{mod_id}/files/{file_id}.json")).map_err(refused_words)?;
        parse_file(&text).map_err(|e| NetError::Failed(format!("{e}.")))
    }

    /// Downloads what an nxm link points at. Works for free accounts too: the link's key lets Nexus hand out this
    /// one download. The app adds the result to the game `link.store_ids()` names. Checks the expiry time first, so
    /// an old link gets `nxm::EXPIRED_MESSAGE` without a request.
    pub fn download_from_nxm(&self, link: &NxmLink, progress: Option<Progress>) -> Result<(Download, SourceInfo), NetError> {
        self.download_from_nxm_at(link, nxm::now(), progress)
    }

    /// `download_from_nxm` with the current time given (seconds since 1970).
    pub fn download_from_nxm_at(&self, link: &NxmLink, now: u64, progress: Option<Progress>) -> Result<(Download, SourceInfo), NetError> {
        if !link.supported() {
            return Err(NetError::Failed(format!("Turbine doesn't support Nexus mods for {} yet.", link.domain)));
        }
        if link.expired_at(now) {
            return Err(NetError::Failed(nxm::EXPIRED_MESSAGE.into()));
        }
        let file = self.file_info(&link.domain, link.mod_id, link.file_id)?;
        if file.is_removed() {
            return Err(NetError::Failed("The author removed this file from Nexus, so it can't be downloaded.".into()));
        }
        let text = self.get(&link.download_link_path()).map_err(|e| match e {
            // The file itself was just found, so a 404 or 410 here means the key's time is up (the computer's clock
            // may differ from Nexus's). A key made for another account, or an old one, is refused.
            NetError::NotFound if link.key.is_some() => NetError::Failed(nxm::EXPIRED_MESSAGE.into()),
            NetError::Denied(_) if link.key.is_some() => NetError::Failed(
                "Nexus didn't accept this download link. It may have expired or be for another Nexus account. Press Mod Manager Download on the Nexus page again.".into(),
            ),
            NetError::Denied(_) => NetError::Failed(FREE_ACCOUNT_MESSAGE.into()),
            e => e,
        })?;
        let links = parse_download_links(&text).map_err(|e| NetError::Failed(format!("{e}.")))?;
        self.fetch_checked(&links, &link.domain, link.mod_id, &file, progress)
    }

    /// Premium: downloads a mod by its IDs. With a file ID, that file while it can still be downloaded, else the
    /// newest file replacing it, else the main file; without one, the main file (for requirements). Free accounts get
    /// `FREE_ACCOUNT_MESSAGE` without a request.
    pub fn download_by_id(
        &self,
        account: &Account,
        domain: &str,
        mod_id: u64,
        file_id: Option<u64>,
        progress: Option<Progress>,
    ) -> Result<(Download, SourceInfo), NetError> {
        Self::check_domain(domain)?;
        if !account.is_premium {
            return Err(NetError::Failed(FREE_ACCOUNT_MESSAGE.into()));
        }
        let list = self.files(domain, mod_id)?;
        let file = list.pick(file_id).cloned().ok_or_else(|| NetError::Failed("This mod has no file on Nexus that can be downloaded.".into()))?;
        self.download(account, domain, mod_id, &file, progress)
    }

    /// A mod's requirements (other Nexus mods, outside downloads and DLC) through GraphQL. Sort them with
    /// `check_requirements` and turn the missing ones into steps with `requirement_steps`.
    pub fn requirements(&self, domain: &str, mod_id: u64) -> Result<Vec<Requirement>, NetError> {
        Self::check_domain(domain)?;
        let text = self.graphql(&requirements_body(domain, mod_id))?;
        parse_requirements(&text, domain).map_err(|e| NetError::Failed(format!("{e}.")))
    }

    /// POSTs a GraphQL body and returns the answer's text.
    fn graphql(&self, body_json: &str) -> Result<String, NetError> {
        fs::create_dir_all(&self.client.temp_dir).map_err(|e| NetError::Failed(format!("Couldn't make the temp folder ({e}).")))?;
        let body = net::temp_path(&self.client.temp_dir, "query");
        fs::write(&body, body_json).map_err(|e| NetError::Failed(format!("Couldn't save the request ({e}).")))?;
        let resp = self.request(Request::get(GRAPHQL).post_file(&body, "application/json").max_bytes(16 << 20).max_time(60));
        let _ = fs::remove_file(&body);
        resp?.take_text()
    }

    /// Nexus's games whose names hold these words ("Which game on Nexus is this?", Search Nexus).
    pub fn games_search(&self, text: &str) -> Result<Vec<crate::nexus_games::NexusGame>, NetError> {
        let answer = self.graphql(&crate::nexus_games::games_search_body(text))?;
        crate::nexus_games::parse_games_search(&answer).map_err(NetError::Failed)
    }

    /// Browses mods for a game through GraphQL. Trending and Newest that Nexus refuses (a GraphQL error) are asked
    /// again as Most endorsed, with `relaxed` set; any other failure is returned.
    pub fn search(&self, domain: &str, text: &str, sort: SearchSort, offset: u32, count: u32, adult: bool) -> Result<SearchPage, NetError> {
        self.search_in(domain, text, "", sort, offset, count, adult)
    }

    /// `search` in one category ("" for all).
    #[allow(clippy::too_many_arguments)]
    pub fn search_in(&self, domain: &str, text: &str, category: &str, sort: SearchSort, offset: u32, count: u32, adult: bool) -> Result<SearchPage, NetError> {
        Self::check_domain(domain)?;
        let now = nxm::now();
        let answer = self.graphql(&search_body_in(domain, text, category, sort, offset, count, adult, now))?;
        match parse_search(&answer) {
            Ok(p) => Ok(p),
            Err(_) if matches!(sort, SearchSort::Trending | SearchSort::Newest) => {
                let again = self.graphql(&search_body_in(domain, text, category, SearchSort::Endorsements, offset, count, adult, now))?;
                let mut p = parse_search(&again).map_err(|e| NetError::Failed(format!("{e}.")))?;
                p.relaxed = true;
                Ok(p)
            }
            Err(e) => Err(NetError::Failed(format!("{e}."))),
        }
    }

    /// Version and last change of many mods of one game in one GraphQL request (no key needed), for update checks.
    /// Asked 50 at a time.
    pub fn mods_by_id(&self, domain: &str, ids: &[u64]) -> Result<Vec<ModStamp>, NetError> {
        Self::check_domain(domain)?;
        let mut out = Vec::new();
        for chunk in ids.chunks(50) {
            let text = self.graphql(&by_id_body(domain, chunk))?;
            out.extend(parse_by_id(&text).map_err(|e| NetError::Failed(format!("{e}.")))?);
        }
        Ok(out)
    }

    /// A mod's page facts and its requirements, in one GraphQL request (no key needed).
    pub fn details(&self, domain: &str, mod_id: u64) -> Result<(ModDetails, Vec<Requirement>), NetError> {
        Self::check_domain(domain)?;
        let text = self.graphql(&details_body(domain, mod_id))?;
        let d = parse_mod_details(&text, domain).map_err(|e| NetError::Failed(format!("{e}.")))?;
        let reqs = parse_requirements(&text, domain).unwrap_or_default();
        Ok((d, reqs))
    }

    /// A mod's files through GraphQL (no key needed). `game_id` from `details`.
    pub fn files_public(&self, game_id: u64, mod_id: u64) -> Result<FileList, NetError> {
        let text = self.graphql(&files_body(game_id, mod_id))?;
        parse_mod_files(&text).map_err(|e| NetError::Failed(format!("{e}.")))
    }

    /// A mod's files: v1 with a key (it has the replacement links), otherwise GraphQL (the game's number is read
    /// from the details first when not given).
    pub fn files_any(&self, domain: &str, mod_id: u64, game_id: Option<u64>) -> Result<FileList, NetError> {
        if self.key.is_some() {
            return self.files(domain, mod_id);
        }
        let gid = match game_id.filter(|g| *g > 0) {
            Some(g) => g,
            None => self.details(domain, mod_id)?.0.game_id,
        };
        if gid == 0 {
            return Err(NetError::Failed("Nexus didn't say which game this mod is for.".into()));
        }
        self.files_public(gid, mod_id)
    }
}

// ---- Identifying a downloaded file (md5_search) ----
//
// GET /v1/games/{domain}/mods/md5_search/{md5}.json answers with every Nexus file that has this MD5, as a list of
// {"mod": <the mod, as games/<domain>/mods/<id>.json>, "file_details": <the file, as files/<id>.json, plus "md5">}
// (Nexus-Mods/node-nexus-api, src/Nexus.ts getFileByMD5 and src/types.ts IMD5Result, commit f3f9728, Nov 2026). The
// same file may be listed more than once (uploaded to two mods); the size tells them apart. No match is a 404.

/// One Nexus file with the MD5 that was asked about.
#[derive(Clone, Debug, PartialEq)]
pub struct Md5Match {
    pub domain: String,
    pub mod_info: ModInfo,
    pub file: FileInfo,
    /// The MD5 Nexus has for the file (lowercase hex); empty when it didn't say.
    pub md5: String,
}

/// The API path for an MD5 lookup.
pub fn md5_search_path(domain: &str, md5: &str) -> String {
    format!("/v1/games/{domain}/mods/md5_search/{}.json", md5.to_ascii_lowercase())
}

/// Reads an md5_search answer. `domain` is the game asked about (used when a mod doesn't name its own).
pub fn parse_md5_search(text: &str, domain: &str) -> Result<Vec<Md5Match>, String> {
    let j = parse(text)?;
    let list = match &j {
        Json::Arr(items) => items.as_slice(),
        _ => return Err("Nexus's answer isn't a list of files".into()),
    };
    let mut out = Vec::new();
    for item in list {
        let (Some(m), Some(f)) = (item.get("mod"), item.get("file_details")) else { continue };
        let (Some(mut mod_info), Some(file)) = (mod_from_json(m), file_from_json(f)) else { continue };
        if mod_info.domain.is_empty() {
            mod_info.domain = domain.to_string();
        }
        out.push(Md5Match { domain: mod_info.domain.clone(), mod_info, file, md5: s(f, "md5").to_ascii_lowercase() });
    }
    Ok(out)
}

/// The match for a file of this size: the one whose size is within 1 KB (Nexus rounds older sizes to whole KB),
/// preferring a current, primary file, then a newer upload; when no size fits, the first one still downloadable.
pub fn best_md5_match(matches: &[Md5Match], size: u64) -> Option<&Md5Match> {
    let rank = |m: &Md5Match| (m.file.is_current(), m.file.is_primary, m.file.uploaded_timestamp);
    let fits = |m: &&Md5Match| m.file.size_bytes == 0 || m.file.size_bytes.abs_diff(size) < 1024;
    matches.iter().filter(fits).max_by_key(|m| rank(m)).or_else(|| matches.iter().filter(|m| !m.file.is_removed()).max_by_key(|m| rank(m)))
}

/// The record for a file identified by its MD5: a proper Nexus source, so it gets the mod's name, version and update
/// checks like a file added through Mod Manager Download.
pub fn md5_source_info(m: &Md5Match, sha256: Option<String>) -> SourceInfo {
    // 0.6: named by its Nexus title, with its picture.
    SourceInfo {
        title: (!m.mod_info.name.trim().is_empty()).then(|| m.mod_info.name.trim().to_string()),
        picture: (!m.mod_info.picture_url.is_empty()).then(|| m.mod_info.picture_url.clone()),
        ..source_info(&m.domain, m.mod_info.mod_id, &m.file, sha256)
    }
}

impl Nexus {
    /// Looks a downloaded archive up by its MD5. Ok(None) when Nexus doesn't know the file (made elsewhere, or
    /// changed after download).
    pub fn identify(&self, domain: &str, archive: &Path) -> Result<Option<Md5Match>, NetError> {
        Self::check_domain(domain)?;
        let size = fs::metadata(archive).map_err(|e| NetError::Failed(format!("Couldn't read the file ({e}).")))?.len();
        let md5 = crate::md5::md5_file(archive).map_err(|e| NetError::Failed(format!("Couldn't read the file ({e}).")))?;
        let text = match self.get(&md5_search_path(domain, &md5)) {
            Ok(t) => t,
            Err(NetError::NotFound) => return Ok(None),
            Err(e) => return Err(e),
        };
        let matches = parse_md5_search(&text, domain).map_err(|e| NetError::Failed(format!("{e}.")))?;
        Ok(best_md5_match(&matches, size).cloned())
    }
}
