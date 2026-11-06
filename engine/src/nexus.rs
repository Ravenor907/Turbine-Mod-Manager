//! Nexus Mods through its official API, with the player's personal API key (pasted in Settings from their Nexus
//! account page). v1 REST for the account, mod details, files, update lists and download links; v2 GraphQL for
//! browsing. Requests send Turbine's application headers and respect Nexus's rate limit headers. Parsing is pure and
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

/// Nexus game domains for the launch games: (store ID, domain, name). GOG IDs share their game's domain; see
/// rules::GAMES for where each ID was checked.
pub const GAMES: &[(&str, &str, &str)] = &[
    ("489830", "skyrimspecialedition", "Skyrim Special Edition"),
    ("413150", "stardewvalley", "Stardew Valley"),
    ("1453375253", "stardewvalley", "Stardew Valley"),
    ("892970", "valheim", "Valheim"),
    ("1966720", "lethalcompany", "Lethal Company"),
    ("632360", "riskofrain2", "Risk of Rain 2"),
    ("1091500", "cyberpunk2077", "Cyberpunk 2077"),
    ("1423049311", "cyberpunk2077", "Cyberpunk 2077"),
    ("292030", "witcher3", "The Witcher 3"),
    ("1207664643", "witcher3", "The Witcher 3"),
    ("1495134320", "witcher3", "The Witcher 3"),
];

pub fn domain_for_store_id(store_id: &str) -> Option<&'static str> {
    GAMES.iter().find(|(id, _, _)| *id == store_id).map(|(_, d, _)| *d)
}

/// Store IDs of the Turbine games that use this Nexus domain (the reverse of `domain_for_store_id`).
pub fn store_ids_for_domain(domain: &str) -> Vec<&'static str> {
    GAMES.iter().filter(|(_, d, _)| *d == domain).map(|(id, _, _)| *id).collect()
}

/// The game's name for a Nexus domain Turbine supports.
pub fn game_name_for_domain(domain: &str) -> Option<&'static str> {
    GAMES.iter().find(|(_, d, _)| *d == domain).map(|(_, _, n)| *n)
}

fn known_domain(domain: &str) -> bool {
    GAMES.iter().any(|(_, d, _)| *d == domain)
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

/// The headers every request carries.
pub fn headers(key: &str, app_version: &str) -> Vec<(String, String)> {
    vec![
        ("apikey".into(), key.trim().into()),
        ("Application-Name".into(), APP_NAME.into()),
        ("Application-Version".into(), app_version.into()),
        ("Accept".into(), "application/json".into()),
    ]
}

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

    /// Few requests left: update checks should wait so browsing still works.
    pub fn low(&self) -> bool {
        let h = self.hourly_remaining.unwrap_or(u64::MAX);
        let d = self.daily_remaining.unwrap_or(u64::MAX);
        h < 10 && d < 50
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

    /// The plain heading for the file's category, as the details page shows it.
    pub fn category_label(&self) -> &'static str {
        category_label(&self.category)
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

/// The shortest period Nexus offers ("1d", "1w", "1m") that covers the time since the last check. None means it
/// has been longer than a month: check each mod's own details instead.
pub fn update_period(seconds_since_last_check: u64) -> Option<&'static str> {
    const DAY: u64 = 86_400;
    match seconds_since_last_check {
        s if s < DAY => Some("1d"),
        s if s < 7 * DAY => Some("1w"),
        s if s < 28 * DAY => Some("1m"),
        _ => None,
    }
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
}

impl SearchSort {
    fn field(self) -> (&'static str, &'static str) {
        match self {
            SearchSort::Endorsements => ("endorsements", "DESC"),
            SearchSort::Downloads => ("downloads", "DESC"),
            SearchSort::Updated => ("updatedAt", "DESC"),
            SearchSort::Name => ("name", "ASC"),
        }
    }
}

const SEARCH_QUERY: &str = "query TurbineMods($filter: ModsFilter, $sort: [ModsSort!], $offset: Int, $count: Int) { \
mods(filter: $filter, sort: $sort, offset: $offset, count: $count) { totalCount nodes { modId name summary version \
author endorsements downloads pictureUrl thumbnailUrl adultContent updatedAt } } }";

/// The JSON body for a search: mods for one game, optionally by name (wildcard match), adult mods left out unless
/// asked for, sorted (most endorsed first by default).
pub fn search_body(domain: &str, text: &str, sort: SearchSort, offset: u32, count: u32, adult: bool) -> String {
    let mut filter = format!("\"op\":\"AND\",\"gameDomainName\":[{{\"value\":{},\"op\":\"EQUALS\"}}]", quote(domain));
    let t = text.trim();
    if !t.is_empty() {
        filter.push_str(&format!(",\"name\":[{{\"value\":{},\"op\":\"WILDCARD\"}}]", quote(t)));
    }
    if !adult {
        filter.push_str(",\"adultContent\":[{\"value\":false,\"op\":\"EQUALS\"}]");
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
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct SearchPage {
    pub total: u64,
    pub mods: Vec<SearchMod>,
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
    Ok(SearchPage {
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

const REQUIREMENTS_QUERY: &str = "query TurbineRequirements($ids: [CompositeDomainWithIdInput!]!) { \
legacyModsByDomain(ids: $ids) { nodes { modId name game { id domainName } modRequirements { \
nexusRequirements(count: 100) { totalCount nodes { id modId modName gameId url notes externalRequirement } } \
dlcRequirements { notes gameExpansion { id name } } } } } }";

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

fn file_group_json(g: &FileGroup) -> String {
    let files: Vec<String> = g
        .files
        .iter()
        .map(|f| {
            format!(
                "{{\"fileId\":{},\"name\":{},\"version\":{},\"category\":{},\"fileName\":{},\"size\":{},\"uploaded\":{},\"primary\":{},\"description\":{}}}",
                f.file_id,
                quote(&f.name),
                quote(&f.version),
                quote(&f.category),
                quote(&f.file_name),
                f.size_bytes,
                f.uploaded_timestamp,
                f.is_primary,
                quote(&f.description)
            )
        })
        .collect();
    format!("{{\"category\":{},\"label\":{},\"files\":[{}]}}", quote(&g.category), quote(g.label), files.join(","))
}

/// File groups as JSON for the details page: [{"category","label","files":[{"fileId","name","version",...}]}].
pub fn file_groups_json(groups: &[FileGroup]) -> String {
    format!("[{}]", groups.iter().map(file_group_json).collect::<Vec<_>>().join(","))
}

pub fn source_info(domain: &str, mod_id: u64, file: &FileInfo, sha256: Option<String>) -> SourceInfo {
    SourceInfo {
        kind: "nexus".into(),
        id: source_id(domain, mod_id, file.file_id),
        version: (!file.version.is_empty()).then(|| file.version.clone()),
        url: Some(mod_page_url(domain, mod_id)),
        sha256,
        needs: Vec::new(),
    }
}

// ---- Network ----

/// A Nexus session with the player's key. Remembers the last rate limit headers it saw and stops before a call
/// that would be refused.
pub struct Nexus {
    pub client: Client,
    key: String,
    app_version: String,
    pub limits: Cell<RateLimit>,
}

impl Nexus {
    pub fn new(client: Client, key: &str, app_version: &str) -> Nexus {
        Nexus { client, key: key.trim().to_string(), app_version: app_version.into(), limits: Cell::new(RateLimit::default()) }
    }

    fn request(&self, req: Request) -> Result<Response, NetError> {
        let current = self.limits.take();
        let blocked = current.blocked();
        self.limits.set(current);
        if let Some(why) = blocked {
            return Err(NetError::Failed(why));
        }
        let mut req = req;
        for (k, v) in headers(&self.key, &self.app_version) {
            req = req.header(&k, &v);
        }
        let resp = self.client.send(&req, None);
        if let Ok(r) = &resp {
            let rl = rate_limit(&r.headers);
            if rl.hourly_remaining.is_some() || rl.daily_remaining.is_some() {
                self.limits.set(rl);
            }
        }
        resp
    }

    fn get(&self, path: &str) -> Result<String, NetError> {
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
        if !plausible_key(&self.key) {
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
        let text = self.get(&format!("/v1/games/{domain}/mods/{mod_id}.json"))?;
        parse_mod(&text).map_err(|e| NetError::Failed(format!("{e}.")))
    }

    pub fn files(&self, domain: &str, mod_id: u64) -> Result<FileList, NetError> {
        Self::check_domain(domain)?;
        let text = self.get(&format!("/v1/games/{domain}/mods/{mod_id}/files.json"))?;
        parse_files(&text).map_err(|e| NetError::Failed(format!("{e}.")))
    }

    /// One request per game for every mod changed in `period` ("1d", "1w" or "1m").
    pub fn updated(&self, domain: &str, period: &str) -> Result<Vec<UpdatedEntry>, NetError> {
        Self::check_domain(domain)?;
        if !matches!(period, "1d" | "1w" | "1m") {
            return Err(NetError::Failed("Nexus only lists updates for a day, a week or a month.".into()));
        }
        let text = self.get(&format!("/v1/games/{domain}/mods/updated.json?period={period}"))?;
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
        let text = self.get(&format!("/v1/games/{domain}/mods/{mod_id}/files/{file_id}.json"))?;
        parse_file(&text).map_err(|e| NetError::Failed(format!("{e}.")))
    }

    /// Every file of a mod grouped for the details page's "Other versions" (main, updates, optional, other, old
    /// versions, archived). One request.
    pub fn file_groups(&self, domain: &str, mod_id: u64) -> Result<Vec<FileGroup>, NetError> {
        Ok(self.files(domain, mod_id)?.groups())
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

    /// Browses mods for a game through GraphQL.
    pub fn search(&self, domain: &str, text: &str, sort: SearchSort, offset: u32, count: u32, adult: bool) -> Result<SearchPage, NetError> {
        Self::check_domain(domain)?;
        let text = self.graphql(&search_body(domain, text, sort, offset, count, adult))?;
        parse_search(&text).map_err(|e| NetError::Failed(format!("{e}.")))
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
    source_info(&m.domain, m.mod_info.mod_id, &m.file, sha256)
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
