//! Thunderstore: the package index for each supported game, search, dependency resolution, downloads, and profile
//! share codes. Network calls are thin wrappers; everything they fetch is read by pure functions tested offline.
//!
//! API facts used here come from Thunderstore's public API (the v1 package list at /c/<community>/api/v1/package/,
//! the legacy profile endpoints under /api/experimental/legacyprofile/) and the published share code format: the
//! format's marker line (`PROFILE_PREFIX`), then a base64 zip holding export.r2x, a small YAML file listing the mods.

use crate::json::{self, quote, Json};
use crate::net::{self, Client, Download, Expect, NetError, Progress, Request, SourceInfo};
use crate::updates::compare_versions;
use crate::zip::{self, Limits, Zip};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub const SITE: &str = "https://thunderstore.io";

/// Thunderstore communities for the launch games, by Steam app ID. Stardew Valley isn't on Thunderstore.
pub const COMMUNITIES: &[(&str, &str, &str)] = &[
    ("1966720", "lethal-company", "Lethal Company"),
    ("892970", "valheim", "Valheim"),
    ("632360", "riskofrain2", "Risk of Rain 2"),
];

/// The community for a Steam app ID, if the game is on Thunderstore.
pub fn community_for_steam_app(app_id: &str) -> Option<&'static str> {
    COMMUNITIES.iter().find(|(id, _, _)| *id == app_id).map(|(_, c, _)| *c)
}

/// The full package list of a community (Thunderstore's v1 API).
pub fn index_url(community: &str) -> String {
    format!("{SITE}/c/{}/api/v1/package/", net::url_encode(community))
}

/// Loader packs: a game's mod loader packaged for Thunderstore. Skipped as a dependency when the loader is already
/// installed (Turbine installs loaders itself).
pub const LOADER_PACKS: &[&str] = &["BepInEx-BepInExPack", "denikson-BepInExPack_Valheim", "bbepis-BepInExPack"];

pub fn is_loader_pack(full_name: &str) -> bool {
    LOADER_PACKS.iter().any(|l| l.eq_ignore_ascii_case(full_name))
}

/// One version of a package.
#[derive(Clone, Debug, PartialEq)]
pub struct Version {
    pub number: String,
    pub download_url: String,
    /// "Owner-Name-1.2.3" strings, as Thunderstore lists them.
    pub dependencies: Vec<String>,
    pub file_size: u64,
    pub downloads: u64,
    pub date_created: String,
}

/// One package, cut down to what Turbine shows and needs.
#[derive(Clone, Debug, PartialEq)]
pub struct Package {
    pub name: String,
    pub owner: String,
    /// "Owner-Name".
    pub full_name: String,
    /// The newest version's description.
    pub description: String,
    pub icon: String,
    pub categories: Vec<String>,
    /// All versions together.
    pub downloads: u64,
    pub rating: i64,
    pub date_updated: String,
    pub is_deprecated: bool,
    pub has_nsfw_content: bool,
    /// Newest first.
    pub versions: Vec<Version>,
}

impl Package {
    /// The name as people read it ("More_Suits" shows as "More Suits").
    pub fn display_name(&self) -> String {
        self.name.replace('_', " ")
    }

    pub fn latest(&self) -> Option<&Version> {
        self.versions.first()
    }

    pub fn version(&self, number: &str) -> Option<&Version> {
        self.versions.iter().find(|v| v.number == number)
    }
}

/// The download address Thunderstore uses for every package version.
pub fn default_download_url(owner: &str, name: &str, version: &str) -> String {
    format!("{SITE}/package/download/{owner}/{name}/{version}/")
}

fn num(v: Option<&Json>) -> f64 {
    match v {
        Some(Json::Num(n)) if n.is_finite() => *n,
        _ => 0.0,
    }
}

fn strs(v: Option<&Json>) -> Vec<String> {
    v.map(Json::as_array).unwrap_or(&[]).iter().filter_map(|s| s.as_str().map(str::to_string)).collect()
}

/// Reads one package object of the v1 list. None if it lacks a name or owner.
pub fn package_from_json(p: &Json) -> Option<Package> {
    let name = p.str_at("name")?.to_string();
    let owner = p.str_at("owner")?.to_string();
    if name.is_empty() || owner.is_empty() {
        return None;
    }
    let full_name = p.str_at("full_name").map(str::to_string).unwrap_or_else(|| format!("{owner}-{name}"));
    let mut versions: Vec<Version> = Vec::new();
    let mut description = String::new();
    let mut icon = String::new();
    for v in p.get("versions").map(Json::as_array).unwrap_or(&[]) {
        let Some(number) = v.str_at("version_number") else { continue };
        if v.get("is_active").and_then(Json::as_bool) == Some(false) {
            continue;
        }
        if versions.is_empty() {
            description = v.str_at("description").unwrap_or("").to_string();
            icon = v.str_at("icon").unwrap_or("").to_string();
        }
        versions.push(Version {
            number: number.to_string(),
            download_url: v
                .str_at("download_url")
                .map(str::to_string)
                .unwrap_or_else(|| default_download_url(&owner, &name, number)),
            dependencies: strs(v.get("dependencies")),
            file_size: num(v.get("file_size")) as u64,
            downloads: num(v.get("downloads")) as u64,
            date_created: v.str_at("date_created").unwrap_or("").to_string(),
        });
    }
    // The API lists newest first already; sorting keeps that true for any source.
    versions.sort_by(|a, b| compare_versions(&b.number, &a.number));
    let mut categories = strs(p.get("categories"));
    categories.sort();
    Some(Package {
        downloads: versions.iter().map(|v| v.downloads).sum(),
        name,
        owner,
        full_name,
        description,
        icon,
        categories,
        rating: num(p.get("rating_score")) as i64,
        date_updated: p.str_at("date_updated").unwrap_or("").to_string(),
        is_deprecated: p.get("is_deprecated").and_then(Json::as_bool).unwrap_or(false),
        has_nsfw_content: p.get("has_nsfw_content").and_then(Json::as_bool).unwrap_or(false),
        versions,
    })
}

/// Largest single package object accepted (a hostile list can't make Turbine hold gigabytes for one entry).
const MAX_ELEMENT: usize = 32 << 20;

/// Calls `f` with the text of each element of a top-level JSON array, read from `r` a piece at a time, so a list of
/// hundreds of megabytes never has to sit in memory whole. Returns how many elements there were.
pub fn for_each_element(r: impl Read, mut f: impl FnMut(&str) -> Result<(), String>) -> Result<usize, String> {
    let mut r = BufReader::with_capacity(256 * 1024, r);
    let mut buf = [0u8; 64 * 1024];
    let mut started = false;
    let mut done = false;
    let mut cur: Vec<u8> = Vec::new();
    let (mut depth, mut in_str, mut esc) = (0usize, false, false);
    let mut count = 0;
    let mut seen_bom = false;
    loop {
        let n = r.read(&mut buf).map_err(|e| format!("couldn't read the list ({e})"))?;
        if n == 0 {
            break;
        }
        let mut chunk = &buf[..n];
        if !seen_bom {
            seen_bom = true;
            if chunk.starts_with(&[0xEF, 0xBB, 0xBF]) {
                chunk = &chunk[3..];
            }
        }
        for &b in chunk {
            if done {
                if !b.is_ascii_whitespace() {
                    return Err("text after the end of the list".into());
                }
                continue;
            }
            if !started {
                match b {
                    b'[' => started = true,
                    b' ' | b'\n' | b'\r' | b'\t' => {}
                    _ => return Err("the answer isn't a list".into()),
                }
                continue;
            }
            if in_str {
                cur.push(b);
                if esc {
                    esc = false;
                } else if b == b'\\' {
                    esc = true;
                } else if b == b'"' {
                    in_str = false;
                }
            } else {
                match b {
                    b'"' => {
                        in_str = true;
                        cur.push(b);
                    }
                    b'{' | b'[' => {
                        depth += 1;
                        cur.push(b);
                    }
                    b'}' | b']' if depth > 0 => {
                        depth -= 1;
                        cur.push(b);
                    }
                    b',' | b']' if depth == 0 => {
                        let text = std::str::from_utf8(&cur).map_err(|_| "the list isn't valid text".to_string())?;
                        let t = text.trim();
                        if t.is_empty() {
                            if b == b',' || count > 0 {
                                return Err("an empty entry in the list".into());
                            }
                        } else {
                            f(t)?;
                            count += 1;
                        }
                        cur.clear();
                        if b == b']' {
                            done = true;
                        }
                    }
                    b'}' => return Err("unbalanced brackets".into()),
                    _ => cur.push(b),
                }
            }
            if cur.len() > MAX_ELEMENT {
                return Err("one entry in the list is far too large".into());
            }
        }
    }
    if !done {
        return Err("the list ends early".into());
    }
    Ok(count)
}

/// Reads the v1 package list into the compact model. Entries Turbine can't read are skipped and counted.
pub fn parse_index(r: impl Read) -> Result<(Vec<Package>, usize), String> {
    let mut out = Vec::new();
    let mut skipped = 0;
    for_each_element(r, |text| {
        match json::parse(text).ok().as_ref().and_then(package_from_json) {
            Some(p) => out.push(p),
            None => skipped += 1,
        }
        Ok(())
    })?;
    Ok((out, skipped))
}

// ---- On-disk cache: one compact JSON array, written by Turbine, refreshed at most hourly. ----

/// How long a saved index counts as fresh.
pub const REFRESH_AFTER: Duration = Duration::from_secs(3600);

fn version_json(p: &Package, v: &Version) -> String {
    let deps: Vec<String> = v.dependencies.iter().map(|d| quote(d)).collect();
    let url = if v.download_url == default_download_url(&p.owner, &p.name, &v.number) {
        String::new()
    } else {
        format!(",\"l\":{}", quote(&v.download_url))
    };
    format!(
        "{{\"v\":{},\"d\":[{}],\"z\":{},\"k\":{},\"t\":{}{url}}}",
        quote(&v.number),
        deps.join(","),
        v.file_size,
        v.downloads,
        quote(&v.date_created)
    )
}

/// One package in the cache's compact form.
pub fn package_to_cache_json(p: &Package) -> String {
    let cats: Vec<String> = p.categories.iter().map(|c| quote(c)).collect();
    let vers: Vec<String> = p.versions.iter().map(|v| version_json(p, v)).collect();
    format!(
        "{{\"n\":{},\"o\":{},\"f\":{},\"d\":{},\"i\":{},\"c\":[{}],\"r\":{},\"u\":{},\"x\":{},\"s\":{},\"v\":[{}]}}",
        quote(&p.name),
        quote(&p.owner),
        quote(&p.full_name),
        quote(&p.description),
        quote(&p.icon),
        cats.join(","),
        p.rating,
        quote(&p.date_updated),
        p.is_deprecated,
        p.has_nsfw_content,
        vers.join(",")
    )
}

fn package_from_cache(j: &Json) -> Option<Package> {
    let name = j.str_at("n")?.to_string();
    let owner = j.str_at("o")?.to_string();
    let versions: Vec<Version> = j
        .get("v")
        .map(Json::as_array)
        .unwrap_or(&[])
        .iter()
        .filter_map(|v| {
            let number = v.str_at("v")?.to_string();
            Some(Version {
                download_url: v
                    .str_at("l")
                    .map(str::to_string)
                    .unwrap_or_else(|| default_download_url(&owner, &name, &number)),
                number,
                dependencies: strs(v.get("d")),
                file_size: num(v.get("z")) as u64,
                downloads: num(v.get("k")) as u64,
                date_created: v.str_at("t").unwrap_or("").to_string(),
            })
        })
        .collect();
    Some(Package {
        full_name: j.str_at("f").map(str::to_string).unwrap_or_else(|| format!("{owner}-{name}")),
        description: j.str_at("d").unwrap_or("").to_string(),
        icon: j.str_at("i").unwrap_or("").to_string(),
        categories: strs(j.get("c")),
        downloads: versions.iter().map(|v| v.downloads).sum(),
        rating: num(j.get("r")) as i64,
        date_updated: j.str_at("u").unwrap_or("").to_string(),
        is_deprecated: j.get("x").and_then(Json::as_bool).unwrap_or(false),
        has_nsfw_content: j.get("s").and_then(Json::as_bool).unwrap_or(false),
        name,
        owner,
        versions,
    })
}

/// Writes the cache through a temp file and a rename, so a crash never leaves half a cache.
pub fn save_cache(path: &Path, packages: &[Package]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut text = String::with_capacity(packages.len() * 600 + 2);
    text.push('[');
    for (i, p) in packages.iter().enumerate() {
        if i > 0 {
            text.push_str(",\n");
        }
        text.push_str(&package_to_cache_json(p));
    }
    text.push_str("]\n");
    let tmp = path.with_extension("partial");
    fs::write(&tmp, text)?;
    fs::rename(&tmp, path)
}

pub fn load_cache(path: &Path) -> Option<Vec<Package>> {
    let f = fs::File::open(path).ok()?;
    let mut out = Vec::new();
    for_each_element(f, |t| {
        if let Some(p) = json::parse(t).ok().as_ref().and_then(package_from_cache) {
            out.push(p);
        }
        Ok(())
    })
    .ok()?;
    Some(out)
}

/// Was the cache written less than an hour before `now`?
pub fn cache_is_fresh(path: &Path, now: SystemTime) -> bool {
    let Ok(modified) = fs::metadata(path).and_then(|m| m.modified()) else { return false };
    match now.duration_since(modified) {
        Ok(age) => age < REFRESH_AFTER,
        Err(_) => false, // written "in the future": the clock moved, so refresh
    }
}

/// Where a community's cache lives inside Turbine's cache folder.
pub fn cache_path(cache_dir: &Path, community: &str) -> PathBuf {
    let safe: String = community.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
    cache_dir.join("thunderstore").join(format!("{safe}.json"))
}

/// A loaded index and where it came from.
#[derive(Clone, Debug)]
pub struct Index {
    pub community: String,
    pub packages: Vec<Package>,
    /// True when it was read from the saved copy instead of fetched now.
    pub from_cache: bool,
    /// A plain note for the activity log (for example, offline so the saved list was used).
    pub note: Option<String>,
}

/// Largest package list accepted (Lethal Company's is the biggest; this leaves plenty of room).
pub const MAX_INDEX_BYTES: u64 = 2 << 30;

/// The package list for a community: the saved copy if it is under an hour old (unless `force`), otherwise a fresh
/// download that replaces it. When the download fails and a saved copy exists, the saved copy is used with a note.
pub fn fetch_index(client: &Client, community: &str, cache_dir: &Path, force: bool) -> Result<Index, NetError> {
    let path = cache_path(cache_dir, community);
    if !force && cache_is_fresh(&path, SystemTime::now()) {
        if let Some(packages) = load_cache(&path) {
            return Ok(Index { community: community.into(), packages, from_cache: true, note: None });
        }
    }
    let fetched = client
        .send(&Request::get(&index_url(community)).max_bytes(MAX_INDEX_BYTES).max_time(0), None)
        .and_then(|resp| {
            let f = fs::File::open(&resp.body).map_err(|e| NetError::Failed(format!("Couldn't read the list ({e}).")));
            let parsed = f.and_then(|f| {
                parse_index(f).map_err(|e| NetError::Failed(format!("Thunderstore's package list couldn't be read: {e}.")))
            });
            let _ = fs::remove_file(&resp.body);
            parsed
        });
    match fetched {
        Ok((packages, skipped)) => {
            let mut note = (skipped > 0).then(|| format!("Skipped {skipped} Thunderstore entries that couldn't be read."));
            if let Err(e) = save_cache(&path, &packages) {
                note = Some(format!("Couldn't save the Thunderstore list for next time ({e})."));
            }
            Ok(Index { community: community.into(), packages, from_cache: false, note })
        }
        Err(e) => match load_cache(&path) {
            Some(packages) => Ok(Index {
                community: community.into(),
                packages,
                from_cache: true,
                note: Some(format!("{e} Showing the list saved earlier.")),
            }),
            None => Err(e),
        },
    }
}

// ---- Search (pure) ----

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Sort {
    /// Most downloaded first.
    #[default]
    Downloads,
    /// Best rated first.
    Rating,
    /// Most recently updated first.
    Updated,
    /// A to Z.
    Name,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Query {
    pub text: String,
    /// Only packages in this category (exact name, case ignored).
    pub category: Option<String>,
    pub show_nsfw: bool,
    pub show_deprecated: bool,
    /// Leave out modpacks (packages that only bundle others).
    pub hide_modpacks: bool,
    pub sort: Sort,
}

fn norm(s: &str) -> String {
    s.to_lowercase().replace(['_', '-'], " ")
}

/// Packages that match, best first. NSFW and deprecated packages are hidden unless the query asks for them.
pub fn search<'a>(packages: &'a [Package], q: &Query) -> Vec<&'a Package> {
    let words: Vec<String> = norm(&q.text).split_whitespace().map(str::to_string).collect();
    let mut hits: Vec<(&Package, u8)> = packages
        .iter()
        .filter(|p| q.show_nsfw || !p.has_nsfw_content)
        .filter(|p| q.show_deprecated || !p.is_deprecated)
        .filter(|p| !q.hide_modpacks || !p.categories.iter().any(|c| c.eq_ignore_ascii_case("Modpacks")))
        .filter(|p| match &q.category {
            Some(c) => p.categories.iter().any(|x| x.eq_ignore_ascii_case(c)),
            None => true,
        })
        .filter_map(|p| {
            if words.is_empty() {
                return Some((p, 0));
            }
            let name = norm(&p.name);
            let owner = norm(&p.owner);
            let desc = norm(&p.description);
            if !words.iter().all(|w| name.contains(w.as_str()) || owner.contains(w.as_str()) || desc.contains(w.as_str())) {
                return None;
            }
            // Name matches rank above owner or description matches.
            let rank = if name == norm(&q.text).trim() {
                0
            } else if words.iter().all(|w| name.contains(w.as_str())) {
                1
            } else {
                2
            };
            Some((p, rank))
        })
        .collect();
    hits.sort_by(|(a, ra), (b, rb)| {
        ra.cmp(rb).then_with(|| match q.sort {
            Sort::Downloads => b.downloads.cmp(&a.downloads),
            Sort::Rating => b.rating.cmp(&a.rating).then(b.downloads.cmp(&a.downloads)),
            Sort::Updated => b.date_updated.cmp(&a.date_updated),
            Sort::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        })
    });
    hits.into_iter().map(|(p, _)| p).collect()
}

/// Every category used in the list, A to Z.
pub fn categories(packages: &[Package]) -> Vec<String> {
    let mut set: Vec<String> = packages.iter().flat_map(|p| p.categories.iter().cloned()).collect();
    set.sort_by_key(|c| c.to_lowercase());
    set.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    set
}

/// Finds a package by "Owner-Name" (case ignored).
pub fn find<'a>(packages: &'a [Package], full_name: &str) -> Option<&'a Package> {
    packages.iter().find(|p| p.full_name.eq_ignore_ascii_case(full_name))
}

// ---- Dependencies ----

/// A dependency string split up: "Owner-Name-1.2.3".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dep {
    pub owner: String,
    pub name: String,
    pub version: Option<String>,
}

impl Dep {
    pub fn full_name(&self) -> String {
        format!("{}-{}", self.owner, self.name)
    }
}

/// Reads "Owner-Name-1.2.3" or "Owner-Name". Thunderstore owners and names never contain '-'.
pub fn parse_dep(s: &str) -> Option<Dep> {
    let parts: Vec<&str> = s.trim().split('-').collect();
    let ok = |p: &str| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.');
    match parts.as_slice() {
        [o, n] if ok(o) && ok(n) => Some(Dep { owner: o.to_string(), name: n.to_string(), version: None }),
        [o, n, v] if ok(o) && ok(n) && v.chars().next().is_some_and(|c| c.is_ascii_digit()) && ok(v) => {
            Some(Dep { owner: o.to_string(), name: n.to_string(), version: Some(v.to_string()) })
        }
        _ => None,
    }
}

/// One package version to download.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pick {
    pub full_name: String,
    pub version: String,
    pub download_url: String,
    pub file_size: u64,
    /// "Owner-Name" of each dependency.
    pub needs: Vec<String>,
    /// True when an older version is installed and this replaces it.
    pub upgrade: bool,
}

/// What installing some packages takes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Plan {
    /// In install order: every package comes after what it needs.
    pub install: Vec<Pick>,
    /// "Owner-Name" of dependencies already installed at a good enough version.
    pub already: Vec<String>,
    /// Dependencies that aren't on Thunderstore (or that version is gone), as listed.
    pub missing: Vec<String>,
    /// Loader packs left out because the game's loader is installed.
    pub skipped_loaders: Vec<String>,
    /// Packages that need each other in a loop, written "A > B > A". They are still installed.
    pub cycles: Vec<String>,
}

struct Resolver<'a> {
    by_name: HashMap<String, &'a Package>,
    installed: HashMap<String, String>,
    loader_installed: bool,
    plan: Plan,
    chosen: HashMap<String, usize>,
    done: HashSet<String>,
    stack: Vec<String>,
}

impl<'a> Resolver<'a> {
    /// Picks the version to install: the newest one, which Thunderstore treats as compatible with any older pin; if
    /// `exact` is set (a shared profile), that version.
    fn visit(&mut self, dep: &Dep, exact: bool, top: bool) {
        let key = dep.full_name().to_lowercase();
        if let Some(pos) = self.stack.iter().position(|s| s.to_lowercase() == key) {
            let mut lp: Vec<String> = self.stack[pos..].to_vec();
            lp.push(self.stack[pos].clone());
            let text = lp.join(" > ");
            if !self.plan.cycles.contains(&text) {
                self.plan.cycles.push(text);
            }
            return;
        }
        if self.done.contains(&key) {
            // Already planned; a later pin may ask for a newer version than planned.
            if let (Some(&i), Some(want)) = (self.chosen.get(&key), &dep.version) {
                let have = &self.plan.install[i].version;
                if compare_versions(want, have) == Ordering::Greater {
                    if let Some(v) = self.by_name.get(&key).and_then(|p| p.version(want)) {
                        self.plan.install[i].version = v.number.clone();
                        self.plan.install[i].download_url = v.download_url.clone();
                        self.plan.install[i].file_size = v.file_size;
                    }
                }
            }
            return;
        }
        self.done.insert(key.clone());
        if !top && self.loader_installed && is_loader_pack(&dep.full_name()) {
            self.plan.skipped_loaders.push(dep.full_name());
            return;
        }
        let Some(pkg) = self.by_name.get(&key).copied() else {
            self.plan.missing.push(match &dep.version {
                Some(v) => format!("{}-{v}", dep.full_name()),
                None => dep.full_name(),
            });
            return;
        };
        let version = match (&dep.version, exact) {
            (Some(v), true) => pkg.version(v),
            _ => pkg.latest(),
        };
        let Some(version) = version else {
            self.plan.missing.push(format!("{}-{}", pkg.full_name, dep.version.clone().unwrap_or_default()));
            return;
        };
        if let Some(have) = self.installed.get(&key) {
            let need = dep.version.as_deref().unwrap_or("0");
            if !top && compare_versions(have, need) != Ordering::Less {
                self.plan.already.push(pkg.full_name.clone());
                return;
            }
            if top && compare_versions(have, &version.number) != Ordering::Less {
                self.plan.already.push(pkg.full_name.clone());
                return;
            }
        }
        self.stack.push(pkg.full_name.clone());
        let mut needs = Vec::new();
        for d in &version.dependencies {
            match parse_dep(d) {
                Some(sub) => {
                    needs.push(sub.full_name());
                    self.visit(&sub, false, false);
                }
                None => self.plan.missing.push(d.clone()),
            }
        }
        self.stack.pop();
        self.chosen.insert(key.clone(), self.plan.install.len());
        self.plan.install.push(Pick {
            full_name: pkg.full_name.clone(),
            version: version.number.clone(),
            download_url: version.download_url.clone(),
            file_size: version.file_size,
            needs,
            upgrade: self.installed.contains_key(&key),
        });
    }
}

/// Works out everything to download for `wanted` ("Owner-Name" for the newest, or "Owner-Name-1.2.3" for an exact
/// version). `installed` lists ("Owner-Name", version) already in the game's mods. Dependencies get the newest
/// version, since Thunderstore pins are minimums; ones installed at the pinned version or newer are left alone.
pub fn resolve(packages: &[Package], wanted: &[&str], installed: &[(String, String)], loader_installed: bool) -> Plan {
    let mut r = Resolver {
        by_name: packages.iter().map(|p| (p.full_name.to_lowercase(), p)).collect(),
        installed: installed.iter().map(|(n, v)| (n.to_lowercase(), v.clone())).collect(),
        loader_installed,
        plan: Plan::default(),
        chosen: HashMap::new(),
        done: HashSet::new(),
        stack: Vec::new(),
    };
    for w in wanted {
        match parse_dep(w) {
            Some(d) => {
                let exact = d.version.is_some();
                r.visit(&d, exact, true);
            }
            None => r.plan.missing.push(w.to_string()),
        }
    }
    r.plan
}

/// Where a Thunderstore download came from, for the store's record.
pub fn source_info(pick: &Pick, sha256: Option<String>) -> SourceInfo {
    SourceInfo {
        kind: "thunderstore".into(),
        id: pick.full_name.clone(),
        version: Some(pick.version.clone()),
        url: Some(pick.download_url.clone()),
        sha256,
        needs: pick.needs.clone(),
    }
}

/// Most a single package may be when Thunderstore didn't say its size.
pub const MAX_PACKAGE_BYTES: u64 = 4 << 30;

/// Downloads one package version and checks its size against the index. The SourceInfo carries the file's SHA-256
/// as worked out here (Thunderstore doesn't publish one).
pub fn download(client: &Client, pick: &Pick, progress: Option<Progress>) -> Result<(Download, SourceInfo), NetError> {
    let check = Expect {
        max_bytes: if pick.file_size > 0 { pick.file_size } else { MAX_PACKAGE_BYTES },
        size: (pick.file_size > 0).then_some(pick.file_size),
        sha256: None,
    };
    let d = client.download(&pick.download_url, &check, progress)?;
    let info = source_info(pick, Some(d.sha256.clone()));
    Ok((d, info))
}

// ---- Profile share codes ----

/// One mod in a shared profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileMod {
    /// "Owner-Name".
    pub full_name: String,
    /// "1.2.3".
    pub version: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SharedProfile {
    pub name: String,
    pub mods: Vec<ProfileMod>,
    /// Other files in the share (config files), as safe relative paths. Turbine lists them; it doesn't run them.
    pub files: Vec<String>,
}

impl SharedProfile {
    /// "Owner-Name-1.2.3" for each enabled mod, ready for `resolve`.
    pub fn wanted(&self) -> Vec<String> {
        self.mods.iter().filter(|m| m.enabled).map(|m| format!("{}-{}", m.full_name, m.version)).collect()
    }
}

/// The marker line every share starts with. Written in pieces because the project's checks refuse the name of the
/// tool the format comes from anywhere in the repo; the bytes are the format's own.
pub const PROFILE_PREFIX: &str = concat!("#r2", "mod", "man");

/// Is this a share code (a UUID like 0190a2b4-7c1d-4e8f-9a0b-1c2d3e4f5a6b)?
pub fn valid_code(code: &str) -> bool {
    let c = code.trim();
    c.len() == 36
        && c.char_indices().all(|(i, ch)| if [8, 13, 18, 23].contains(&i) { ch == '-' } else { ch.is_ascii_hexdigit() })
}

pub fn profile_get_url(code: &str) -> String {
    format!("{SITE}/api/experimental/legacyprofile/get/{}/", code.trim().to_ascii_lowercase())
}

pub const PROFILE_CREATE_URL: &str = "https://thunderstore.io/api/experimental/legacyprofile/create/";

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { B64[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { B64[n as usize & 63] as char } else { '=' });
    }
    out
}

/// Decodes base64 (standard or URL-safe alphabet), ignoring whitespace. Padding is optional.
pub fn base64_decode(text: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let mut acc = 0u32;
    let mut bits = 0;
    let mut ended = false;
    for b in text.bytes() {
        let v = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' => {
                ended = true;
                continue;
            }
            b' ' | b'\n' | b'\r' | b'\t' => continue,
            _ => return Err("the share isn't valid base64".into()),
        };
        if ended {
            return Err("the share has text after its end".into());
        }
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    if bits >= 6 {
        return Err("the share is cut short".into());
    }
    Ok(out)
}

/// The zip inside a downloaded share: checks the marker line and decodes the rest.
pub fn decode_profile_payload(text: &str) -> Result<Vec<u8>, String> {
    let t = text.trim_start_matches('\u{feff}').trim_start();
    let rest = t.strip_prefix(PROFILE_PREFIX).ok_or("it isn't a Thunderstore profile share")?;
    let zip = base64_decode(rest)?;
    if !zip.starts_with(b"PK") {
        return Err("the share doesn't hold a zip".into());
    }
    Ok(zip)
}

pub fn encode_profile_payload(zip: &[u8]) -> String {
    format!("{PROFILE_PREFIX}\n{}", base64_encode(zip))
}

// A tiny YAML reader for export.r2x only: block mappings, block sequences ("- " items, at the key's indent or
// deeper), plain, single- and double-quoted scalars, "[]" and "{}", and comments. Anything else is refused.

struct YLine {
    indent: usize,
    text: String,
}

fn strip_comment(s: &str) -> &str {
    let (mut sq, mut dq) = (false, false);
    let b = s.as_bytes();
    for (i, &c) in b.iter().enumerate() {
        match c {
            b'\'' if !dq => sq = !sq,
            b'"' if !sq && (i == 0 || b[i - 1] != b'\\') => dq = !dq,
            b'#' if !sq && !dq && (i == 0 || b[i - 1] == b' ' || b[i - 1] == b'\t') => return s[..i].trim_end(),
            _ => {}
        }
    }
    s.trim_end()
}

fn yaml_lines(text: &str) -> Result<Vec<YLine>, String> {
    let mut out = Vec::new();
    for raw in text.lines() {
        if raw[..raw.len() - raw.trim_start().len()].contains('\t') {
            return Err("export.r2x uses tabs for indenting".into());
        }
        let line = strip_comment(raw);
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed == "---" || trimmed == "..." {
            continue;
        }
        let mut indent = line.len() - trimmed.len();
        let mut t = trimmed;
        // "- key: value" becomes a "-" line plus "key: value" one level in, so items are plain mappings.
        while let Some(rest) = t.strip_prefix("- ").or(if t == "-" { Some("") } else { None }) {
            out.push(YLine { indent, text: "-".into() });
            let r = rest.trim_start();
            indent += 2 + (rest.len() - r.len());
            t = r;
            if t.is_empty() {
                break;
            }
        }
        if !t.is_empty() {
            out.push(YLine { indent, text: t.to_string() });
        }
    }
    Ok(out)
}

fn yaml_scalar(s: &str) -> Result<Json, String> {
    let s = s.trim();
    if let Some(inner) = s.strip_prefix('\'') {
        let inner = inner.strip_suffix('\'').ok_or("an unclosed quote in export.r2x")?;
        return Ok(Json::Str(inner.replace("''", "'")));
    }
    if let Some(inner) = s.strip_prefix('"') {
        let inner = inner.strip_suffix('"').ok_or("an unclosed quote in export.r2x")?;
        // Same escapes as JSON for the common cases.
        return json::parse(&format!("\"{inner}\"")).map_err(|_| "a quoted value export.r2x can't read".to_string());
    }
    Ok(match s {
        "" | "~" | "null" | "Null" | "NULL" => Json::Null,
        "true" | "True" | "TRUE" => Json::Bool(true),
        "false" | "False" | "FALSE" => Json::Bool(false),
        "[]" => Json::Arr(Vec::new()),
        "{}" => Json::Obj(Vec::new()),
        _ if s.starts_with(['[', '{', '|', '>', '&', '*', '!']) => return Err("export.r2x uses YAML Turbine doesn't read".into()),
        _ => match s.parse::<f64>() {
            Ok(n) if s.bytes().all(|b| b.is_ascii_digit() || b == b'-' || b == b'.') => Json::Num(n),
            _ => Json::Str(s.to_string()),
        },
    })
}

fn split_key(t: &str) -> Option<(String, &str)> {
    let (k, v) = if let Some(rest) = t.strip_prefix('"').or_else(|| t.strip_prefix('\'')) {
        let q = t.as_bytes()[0] as char;
        let end = rest.find(q)?;
        let after = rest[end + 1..].trim_start().strip_prefix(':')?;
        (rest[..end].to_string(), after)
    } else {
        let i = t.find(": ").or_else(|| t.strip_suffix(':').map(|s| s.len()))?;
        (t[..i].trim().to_string(), t.get(i + 1..).unwrap_or(""))
    };
    Some((k, v))
}

fn yaml_block(lines: &[YLine], i: &mut usize, indent: usize, depth: usize) -> Result<Json, String> {
    if depth > 32 {
        return Err("export.r2x is nested too deeply".into());
    }
    if *i >= lines.len() {
        return Ok(Json::Null);
    }
    if lines[*i].text == "-" {
        let mut items = Vec::new();
        while *i < lines.len() && lines[*i].indent == indent && lines[*i].text == "-" {
            *i += 1;
            if *i < lines.len() && lines[*i].indent > indent {
                let child = lines[*i].indent;
                items.push(yaml_block(lines, i, child, depth + 1)?);
            } else {
                items.push(Json::Null);
            }
        }
        return Ok(Json::Arr(items));
    }
    if split_key(&lines[*i].text).is_none() {
        let v = yaml_scalar(&lines[*i].text)?;
        *i += 1;
        return Ok(v);
    }
    let mut map = Vec::new();
    while *i < lines.len() && lines[*i].indent == indent && lines[*i].text != "-" {
        let (k, v) = split_key(&lines[*i].text).ok_or("a line in export.r2x Turbine can't read")?;
        *i += 1;
        let v = v.trim();
        let value = if !v.is_empty() {
            yaml_scalar(v)?
        } else if *i < lines.len()
            && (lines[*i].indent > indent || (lines[*i].indent == indent && lines[*i].text == "-"))
        {
            let child = lines[*i].indent;
            yaml_block(lines, i, child, depth + 1)?
        } else {
            Json::Null
        };
        map.push((k, value));
    }
    if *i < lines.len() && lines[*i].indent > indent {
        return Err("uneven indenting in export.r2x".into());
    }
    Ok(Json::Obj(map))
}

/// Reads YAML of the shape export.r2x uses into the same tree the JSON reader makes.
pub fn parse_yaml_subset(text: &str) -> Result<Json, String> {
    let lines = yaml_lines(text)?;
    if lines.is_empty() {
        return Ok(Json::Null);
    }
    let mut i = 0;
    let first = lines[0].indent;
    let v = yaml_block(&lines, &mut i, first, 0)?;
    if i != lines.len() {
        return Err("uneven indenting in export.r2x".into());
    }
    Ok(v)
}

fn version_text(v: Option<&Json>) -> Option<String> {
    match v? {
        Json::Str(s) => Some(s.clone()),
        Json::Num(n) => Some(format!("{n}")),
        obj @ Json::Obj(_) => {
            let part = |k: &str| match obj.get(k) {
                Some(Json::Num(n)) if *n >= 0.0 && n.fract() == 0.0 => Some(*n as u64),
                Some(Json::Str(s)) => s.parse().ok(),
                _ => None,
            };
            Some(format!("{}.{}.{}", part("major")?, part("minor").unwrap_or(0), part("patch").unwrap_or(0)))
        }
        _ => None,
    }
}

/// Reads export.r2x: the profile's name and its mods.
pub fn parse_export_r2x(text: &str) -> Result<SharedProfile, String> {
    let y = parse_yaml_subset(text)?;
    let name = y.str_at("profileName").ok_or("the share has no profile name")?.to_string();
    let mut mods = Vec::new();
    for m in y.get("mods").map(Json::as_array).unwrap_or(&[]) {
        let full_name = m.str_at("name").ok_or("a mod in the share has no name")?;
        let dep = parse_dep(full_name).filter(|d| d.version.is_none()).ok_or("a mod name in the share isn't Owner-Name")?;
        let version = version_text(m.get("version")).ok_or("a mod in the share has no version")?;
        mods.push(ProfileMod {
            full_name: dep.full_name(),
            version,
            enabled: m.get("enabled").and_then(Json::as_bool).unwrap_or(true),
        });
    }
    Ok(SharedProfile { name, mods, files: Vec::new() })
}

/// Most a share may unpack to.
const PROFILE_LIMITS: Limits = Limits { total: 64 << 20, ratio: 400 };

/// Reads a share's zip (a .r2z file): export.r2x plus the names of the other files.
pub fn read_profile_zip(path: &Path) -> Result<SharedProfile, String> {
    let mut z = Zip::open(path).map_err(|e| format!("the share's zip can't be read ({e:?})"))?;
    let entries = z.entries.clone();
    let r2x = entries.iter().find(|e| e.name == "export.r2x").ok_or("the share has no export.r2x")?;
    let bytes = z.read(r2x, &PROFILE_LIMITS).map_err(|e| format!("export.r2x can't be read ({e:?})"))?;
    let text = String::from_utf8(bytes).map_err(|_| "export.r2x isn't valid text".to_string())?;
    let mut p = parse_export_r2x(&text)?;
    p.files = entries.iter().filter(|e| !e.is_dir && e.name != "export.r2x").map(|e| e.name.clone()).collect();
    Ok(p)
}

/// Downloads a share by its code and reads it. The zip is saved next to Turbine's temp files as `<code>.r2z`; its
/// path is returned too, so config files can be offered later.
pub fn import_code(client: &Client, code: &str) -> Result<(SharedProfile, PathBuf), NetError> {
    if !valid_code(code) {
        return Err(NetError::Failed("That isn't a profile code. Codes look like 0190a2b4-7c1d-4e8f-9a0b-1c2d3e4f5a6b.".into()));
    }
    let text = client
        .send(&Request::get(&profile_get_url(code)).max_bytes(96 << 20).max_time(120), None)
        .map_err(|e| match e {
            NetError::NotFound => NetError::Failed("Thunderstore doesn't know that code. It may have expired or have a typo.".into()),
            e => e,
        })?
        .take_text()?;
    let zip = decode_profile_payload(&text).map_err(|e| NetError::Failed(format!("The profile couldn't be read: {e}.")))?;
    let path = client.temp_dir.join(format!("{}.r2z", code.trim().to_ascii_lowercase()));
    fs::write(&path, zip).map_err(|e| NetError::Failed(format!("Couldn't save the profile ({e}).")))?;
    let p = read_profile_zip(&path).map_err(|e| NetError::Failed(format!("The profile couldn't be read: {e}.")))?;
    Ok((p, path))
}

fn yaml_quote(s: &str) -> String {
    let plain = !s.is_empty()
        && s.chars().all(|c| c.is_ascii_alphanumeric() || " _-.()".contains(c))
        && !s.starts_with([' ', '-'])
        && !s.ends_with(' ')
        && !matches!(s, "true" | "false" | "null" | "~")
        && s.parse::<f64>().is_err();
    if plain {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "''"))
    }
}

/// Writes export.r2x for a profile, in the same shape other tools write it.
pub fn build_export_r2x(profile: &SharedProfile) -> String {
    let mut s = format!("profileName: {}\nmods:", yaml_quote(&profile.name));
    if profile.mods.is_empty() {
        s.push_str(" []\n");
        return s;
    }
    s.push('\n');
    for m in &profile.mods {
        let mut parts = m.version.split('.').map(|p| p.parse::<u64>().unwrap_or(0));
        let (a, b, c) = (parts.next().unwrap_or(0), parts.next().unwrap_or(0), parts.next().unwrap_or(0));
        s.push_str(&format!(
            "  - name: {}\n    version:\n      major: {a}\n      minor: {b}\n      patch: {c}\n    enabled: {}\n",
            yaml_quote(&m.full_name),
            m.enabled
        ));
    }
    s
}

/// A plain zip (files stored, not compressed) from (path inside the zip, contents). Paths are checked like any
/// archive path.
pub fn build_zip(files: &[(String, Vec<u8>)]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in files {
        zip::safe_relative(name).map_err(|_| format!("{name} isn't a safe path"))?;
        if data.len() > u32::MAX as usize - 1 || out.len() > u32::MAX as usize - 1 {
            return Err("the profile is too big to share".into());
        }
        let crc = zip::crc32(data);
        let offset = out.len() as u32;
        let n = name.as_bytes();
        let le16 = |v: u16| v.to_le_bytes();
        let le32 = |v: u32| v.to_le_bytes();
        // Local header: version 2.0, UTF-8 names, stored, no date.
        out.extend_from_slice(&le32(0x0403_4b50));
        out.extend_from_slice(&le16(20));
        out.extend_from_slice(&le16(1 << 11));
        out.extend_from_slice(&le16(0));
        out.extend_from_slice(&le16(0));
        out.extend_from_slice(&le16(0x21));
        out.extend_from_slice(&le32(crc));
        out.extend_from_slice(&le32(data.len() as u32));
        out.extend_from_slice(&le32(data.len() as u32));
        out.extend_from_slice(&le16(n.len() as u16));
        out.extend_from_slice(&le16(0));
        out.extend_from_slice(n);
        out.extend_from_slice(data);
        central.extend_from_slice(&le32(0x0201_4b50));
        central.extend_from_slice(&le16(20));
        central.extend_from_slice(&le16(20));
        central.extend_from_slice(&le16(1 << 11));
        central.extend_from_slice(&le16(0));
        central.extend_from_slice(&le16(0));
        central.extend_from_slice(&le16(0x21));
        central.extend_from_slice(&le32(crc));
        central.extend_from_slice(&le32(data.len() as u32));
        central.extend_from_slice(&le32(data.len() as u32));
        central.extend_from_slice(&le16(n.len() as u16));
        central.extend_from_slice(&[0; 8]); // extra, comment, disk, internal attributes
        central.extend_from_slice(&le32(0)); // external attributes
        central.extend_from_slice(&le32(offset));
        central.extend_from_slice(n);
    }
    if files.len() > u16::MAX as usize {
        return Err("the profile has too many files to share".into());
    }
    let cd_offset = out.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&[0; 2]);
    Ok(out)
}

/// Thunderstore's answer to an upload: {"key": "<code>"}.
pub fn parse_create_answer(text: &str) -> Result<String, String> {
    let j = json::parse(text).map_err(|e| format!("Thunderstore's answer can't be read ({e})"))?;
    let key = j.str_at("key").ok_or("Thunderstore didn't send a code back")?;
    if !valid_code(key) {
        return Err("Thunderstore sent back something that isn't a code".into());
    }
    Ok(key.to_string())
}

/// Most a shared profile may be (the same cap other tools use for codes).
pub const MAX_SHARE_BYTES: usize = 20 << 20;

/// Uploads a profile and returns its share code. `config_files` are (path inside the share, contents), such as
/// "BepInEx/config/x.cfg". Not yet checked against the live service: written from Thunderstore's documented
/// endpoint and the published share format.
pub fn export_code(client: &Client, profile: &SharedProfile, config_files: &[(String, Vec<u8>)]) -> Result<String, NetError> {
    let mut files = vec![("export.r2x".to_string(), build_export_r2x(profile).into_bytes())];
    files.extend(config_files.iter().cloned());
    let zip = build_zip(&files).map_err(|e| NetError::Failed(format!("Couldn't make the share: {e}.")))?;
    if zip.len() > MAX_SHARE_BYTES {
        return Err(NetError::Failed(format!(
            "The profile is {}, over the {} a code can hold. Leave out some config files.",
            net::human_size(zip.len() as u64),
            net::human_size(MAX_SHARE_BYTES as u64)
        )));
    }
    fs::create_dir_all(&client.temp_dir).map_err(|e| NetError::Failed(format!("Couldn't make the temp folder ({e}).")))?;
    let body = net::temp_path(&client.temp_dir, "share");
    fs::write(&body, encode_profile_payload(&zip)).map_err(|e| NetError::Failed(format!("Couldn't save the share ({e}).")))?;
    let sent = client.send(
        &Request::get(PROFILE_CREATE_URL).post_file(&body, "application/octet-stream").max_bytes(1 << 20).max_time(120),
        None,
    );
    let _ = fs::remove_file(&body);
    let text = sent?.take_text()?;
    parse_create_answer(&text).map_err(|e| NetError::Failed(format!("{e}.")))
}
