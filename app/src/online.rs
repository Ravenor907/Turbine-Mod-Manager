//! The download lane: everything that talks to the internet (Thunderstore, Nexus Mods, the loaders' homes, Turbine's
//! own releases) runs here, on its own thread, so browsing never waits for a download and the mod worker never waits
//! for the network. Nothing here touches a game's files: downloads are handed to the mod worker to add.
//!
//! With --selftest-data (the build's screenshot checks, which have no internet) answers come from recorded samples
//! instead, and downloads are refused.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Set by Cancel (from the interface thread); the download under way stops at its next progress report.
pub static CANCEL: AtomicBool = AtomicBool::new(false);
use turbine_engine::json::quote;
use turbine_engine::net::{self, Client, Download, NetError, SourceInfo};
use turbine_engine::nexus::{self, Account, Nexus};
use turbine_engine::thunderstore::{self, Package, Query, Sort};
use turbine_engine::{loaders, selfupdate, updates};

/// The player's Nexus key for work outside the download lane (identifying files, the Downloads watcher's file
/// lists). None without a key and always in the build's screenshot checks (recorded samples, no internet).
fn shared_key() -> std::sync::MutexGuard<'static, Option<String>> {
    static K: std::sync::OnceLock<std::sync::Mutex<Option<String>>> = std::sync::OnceLock::new();
    K.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner())
}
static SAMPLES: AtomicBool = AtomicBool::new(false);

/// The build's screenshot checks are using recorded samples (no internet, no key needed).
pub fn using_samples() -> bool {
    SAMPLES.load(Ordering::SeqCst)
}

pub fn nexus_key() -> Option<String> {
    if SAMPLES.load(Ordering::SeqCst) {
        return None;
    }
    shared_key().clone()
}

/// Where downloads and API answers go while they are read (the same folder the download lane uses).
pub fn temp_dir() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| turbine_engine::home_dir().join(".cache"))
        .join("turbine-mod-manager")
        .join("downloads")
}

/// A downloaded file ready for the mod worker.
pub struct Fetched {
    pub path: PathBuf,
    pub source: SourceInfo,
    /// What the player sees ("Jotunn 2.20.1").
    pub label: String,
    /// The mod it replaces (an update), if any.
    pub replaces: Option<String>,
}

/// One installed mod, as the download lane needs it: the store's mod id, where it came from, when it was added.
#[derive(Clone, Debug)]
pub struct Have {
    pub mod_id: String,
    pub source: SourceInfo,
    pub added: u64,
}

/// What the lane knows about the game on screen.
#[derive(Clone, Debug, Default)]
pub struct GameInfo {
    pub id: String,
    pub name: String,
    pub community: Option<String>,
    pub domain: Option<String>,
    /// "bethesda", "bepinex", "smapi" or "generic".
    pub kind: String,
    pub loader_ok: bool,
    pub native: bool,
    /// Steam (true) or GOG through Heroic, for the SKSE build.
    pub steam: bool,
    pub have: Vec<Have>,
}

impl GameInfo {
    fn installed_ts(&self) -> Vec<(String, String)> {
        self.have
            .iter()
            .filter(|h| h.source.kind == "thunderstore")
            .map(|h| (h.source.id.clone(), h.source.version.clone().unwrap_or_default()))
            .collect()
    }

    fn installed(&self) -> Vec<updates::Installed> {
        self.have
            .iter()
            .map(|h| updates::Installed {
                kind: h.source.kind.clone(),
                id: h.source.id.clone(),
                version: h.source.version.clone(),
                installed_at: h.added,
            })
            .collect()
    }

    fn mod_for(&self, kind: &str, id: &str) -> Option<String> {
        self.have.iter().find(|h| h.source.kind == kind && h.source.id.eq_ignore_ascii_case(id)).map(|h| h.mod_id.clone())
    }
}

pub struct Net {
    pub client: Client,
    cache: PathBuf,
    sample: Option<PathBuf>,
    indexes: HashMap<String, Arc<Vec<Package>>>,
    key: Option<String>,
    key_file: PathBuf,
    account: Option<Account>,
    version: String,
    /// The release the last self check found, for Install.
    pub release: Option<selfupdate::Release>,
    /// Screenshot checks: browsing Nexus without a key fails, as if Nexus refused it.
    pub sample_nexus_fail: bool,
}

fn err(e: NetError) -> String {
    e.to_string()
}

/// Marks `Fetched::replaces` as "whichever installed mod came from this Nexus mod" ("nexus-mod:<domain>:<mod id>");
/// the mod worker looks it up when the file is added.
pub const NEXUS_MOD: &str = "nexus-mod:";

/// Said instead of Install for free accounts.
pub const FREE_WORDS: &str =
    "Opens Nexus right here in Turbine. Sign in if it asks (free), then press Slow download. Turbine adds it and closes the page.";

/// "2026-11-06" from seconds since 1970 (UTC).
pub fn date_of(secs: u64) -> String {
    if secs == 0 {
        return String::new();
    }
    // Days to a civil date (Howard Hinnant's algorithm).
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}")
}

/// A short date from Thunderstore's "2024-05-01T12:00:00Z".
fn day(s: &str) -> String {
    s.get(..10).unwrap_or(s).to_string()
}

/// Names a downloaded file the way the player knows it, so the mod gets a readable name: "Jotunn 2.20.1.zip".
fn rename(d: &Download, label: &str, ext: &str) -> PathBuf {
    let safe: String = label.chars().map(|c| if c.is_alphanumeric() || " ._-()".contains(c) { c } else { '_' }).collect();
    let dir = d.path.parent().map(Path::to_path_buf).unwrap_or_default();
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|t| t.as_millis()).unwrap_or(0);
    let folder = dir.join(format!("named-{stamp}"));
    let target = folder.join(format!("{}.{ext}", safe.trim()));
    if fs::create_dir_all(&folder).is_ok() && fs::rename(&d.path, &target).is_ok() {
        target
    } else {
        d.path.clone()
    }
}

fn ext_of(name: &str) -> String {
    let low = name.to_ascii_lowercase();
    if low.ends_with(".7z") {
        "7z".into()
    } else if low.ends_with(".rar") {
        "rar".into()
    } else {
        "zip".into()
    }
}

/// Is this Thunderstore package one that runs its own code (it needs the mod loader)?
fn code_mod(p: &Package) -> bool {
    p.latest()
        .map(|v| v.dependencies.iter().any(|d| thunderstore::parse_dep(d).is_some_and(|d| thunderstore::is_loader_pack(&d.full_name()))))
        .unwrap_or(false)
}

impl Net {
    pub fn new(version: &str, cache: &Path, data: &Path) -> Net {
        let temp = cache.join("downloads");
        let key_file = data.join("nexus-key");
        let key = nexus::load_key(&key_file);
        if let Some(k) = &key {
            crate::log::secret(k);
        }
        *shared_key() = key.clone();
        Net {
            client: Client::new(version, &temp),
            cache: cache.to_path_buf(),
            sample: None,
            indexes: HashMap::new(),
            key,
            key_file,
            account: None,
            version: version.to_string(),
            release: None,
            sample_nexus_fail: false,
        }
    }

    /// Recorded samples instead of the internet (the build's checks).
    pub fn is_sample(&self) -> bool {
        self.sample.is_some()
    }

    /// Nexus without a key, for the game search.
    pub fn nexus_public_client(&self) -> Nexus {
        self.nexus_public()
    }

    /// Recorded samples instead of the internet (the build's screenshot checks).
    pub fn use_samples(&mut self, dir: &Path) {
        self.sample = Some(dir.to_path_buf());
        self.indexes.clear();
        SAMPLES.store(true, Ordering::SeqCst);
    }

    fn sample_text(&self, name: &str) -> Option<Result<String, String>> {
        let dir = self.sample.as_ref()?;
        Some(fs::read_to_string(dir.join(name)).map_err(|e| format!("The sample {name} couldn't be read ({e}).")))
    }

    /// A picture's address for the interface: none in the screenshot checks (they have no internet).
    fn pic(&self, url: &str) -> String {
        if let Some(dir) = &self.sample {
            // Recorded samples name local pictures as "sample:<file>"; the internet's are left out.
            return match url.strip_prefix("sample:") {
                Some(f) if !f.contains('/') && dir.join(f).is_file() => format!("file://{}", dir.join(f).display()),
                _ => String::new(),
            };
        }
        if url.starts_with("https://") {
            url.to_string()
        } else {
            String::new()
        }
    }

    fn no_downloads(&self) -> Result<(), String> {
        if self.sample.is_some() {
            return Err("Downloads are off in this test run.".into());
        }
        Ok(())
    }

    // ------------------------------------------------------------------------------------------------ Thunderstore

    pub fn index(&mut self, community: &str, force: bool) -> Result<(Arc<Vec<Package>>, Option<String>), String> {
        if !force {
            if let Some(i) = self.indexes.get(community) {
                return Ok((i.clone(), None));
            }
        }
        let (list, note) = if let Some(dir) = &self.sample {
            let f = fs::File::open(dir.join("thunderstore_v1_lethal.json")).map_err(|e| format!("The sample index couldn't be read ({e})."))?;
            let (p, _) = thunderstore::parse_index(f)?;
            (p, None)
        } else {
            let i = thunderstore::fetch_index(&self.client, community, &self.cache.join("thunderstore"), force).map_err(err)?;
            (i.packages, i.note)
        };
        crate::log::line("net", &format!("Thunderstore {community}: {} packages", list.len()));
        let arc = Arc::new(list);
        self.indexes.insert(community.to_string(), arc.clone());
        Ok((arc, note))
    }

    /// Get Mods from Thunderstore, as JSON for the interface (at most 150 results).
    pub fn browse_ts(&mut self, game: &GameInfo, text: &str, category: &str, sort: &str) -> Result<String, String> {
        let community = game.community.clone().ok_or("This game isn't on Thunderstore.")?;
        let (index, note) = self.index(&community, false)?;
        let q = Query {
            text: text.to_string(),
            category: (!category.is_empty()).then(|| category.to_string()),
            show_nsfw: false,
            show_deprecated: false,
            hide_modpacks: false,
            sort: match sort {
                "rating" => Sort::Rating,
                "updated" => Sort::Updated,
                "name" => Sort::Name,
                _ => Sort::Downloads,
            },
        };
        let found = thunderstore::search(&index, &q);
        let have = game.installed_ts();
        let items: Vec<String> = found
            .iter()
            .take(150)
            .map(|p| {
                let v = p.latest();
                format!(
                    "{{\"id\":{},\"name\":{},\"owner\":{},\"summary\":{},\"downloads\":{},\"rating\":{},\"icon\":{},\"code\":{},\"version\":{},\"size\":{},\"updated\":{},\"installed\":{}}}",
                    quote(&p.full_name),
                    quote(&p.display_name()),
                    quote(&p.owner),
                    quote(&p.description),
                    p.downloads,
                    p.rating,
                    quote(&self.pic(&p.icon)),
                    code_mod(p),
                    quote(v.map(|v| v.number.as_str()).unwrap_or("")),
                    v.map(|v| v.file_size).unwrap_or(0),
                    quote(&day(&p.date_updated)),
                    have.iter().any(|(n, _)| n.eq_ignore_ascii_case(&p.full_name))
                )
            })
            .collect();
        let cats: Vec<String> = thunderstore::categories(&index).iter().map(|c| quote(c)).collect();
        Ok(format!(
            "{{\"source\":\"thunderstore\",\"total\":{},\"note\":{},\"categories\":[{}],\"items\":[{}]}}",
            found.len(),
            quote(note.as_deref().unwrap_or("")),
            cats.join(","),
            items.join(",")
        ))
    }

    /// One Thunderstore package with what installing it takes.
    pub fn details_ts(&mut self, game: &GameInfo, full_name: &str) -> Result<String, String> {
        let community = game.community.clone().ok_or("This game isn't on Thunderstore.")?;
        let (index, _) = self.index(&community, false)?;
        let p = thunderstore::find(&index, full_name).ok_or("Thunderstore doesn't list that mod any more.")?;
        let plan = thunderstore::resolve(&index, &[full_name], &game.installed_ts(), game.loader_ok);
        let picks: Vec<String> = plan
            .install
            .iter()
            .map(|k| {
                let summary = thunderstore::find(&index, &k.full_name).map(|x| x.description.clone()).unwrap_or_default();
                format!(
                    "{{\"id\":{},\"name\":{},\"version\":{},\"size\":{},\"main\":{},\"upgrade\":{},\"summary\":{}}}",
                    quote(&k.full_name),
                    quote(&k.full_name.rsplit_once('-').map(|(_, n)| n.replace('_', " ")).unwrap_or_else(|| k.full_name.clone())),
                    quote(&k.version),
                    k.file_size,
                    k.full_name.eq_ignore_ascii_case(full_name),
                    k.upgrade,
                    quote(&summary)
                )
            })
            .collect();
        let list = |v: &[String]| v.iter().map(|s| quote(s)).collect::<Vec<_>>().join(",");
        let v = p.latest();
        let have = game.installed_ts().into_iter().find(|(n, _)| n.eq_ignore_ascii_case(&p.full_name)).map(|(_, v)| v);
        // Every version but the newest, newest first: "Other versions" on the details page.
        let others: Vec<String> = p
            .versions
            .iter()
            .skip(1)
            .map(|x| {
                format!(
                    "{{\"id\":{},\"version\":{},\"name\":{},\"size\":{},\"date\":{},\"downloads\":{},\"installed\":{},\"group\":\"Older versions\",\"note\":\"\"}}",
                    quote(&format!("{}-{}", p.full_name, x.number)),
                    quote(&x.number),
                    quote(&format!("{} {}", p.display_name(), x.number)),
                    x.file_size,
                    quote(&day(&x.date_created)),
                    x.downloads,
                    have.as_deref() == Some(x.number.as_str())
                )
            })
            .collect();
        let main = format!(
            "{{\"source\":\"thunderstore\",\"id\":{},\"name\":{},\"owner\":{},\"summary\":{},\"icon\":{},\"code\":{},\"version\":{},\"size\":{},\"updated\":{},\"downloads\":{},\"rating\":{},\"categories\":[{}],\"deprecated\":{},\"install\":[{}],\"already\":[{}],\"missing\":[{}],\"skippedLoaders\":[{}],\"premium\":true,\"message\":\"\"",
            quote(&p.full_name),
            quote(&p.display_name()),
            quote(&p.owner),
            quote(&p.description),
            quote(&self.pic(&p.icon)),
            code_mod(p),
            quote(v.map(|v| v.number.as_str()).unwrap_or("")),
            v.map(|v| v.file_size).unwrap_or(0),
            quote(&day(&p.date_updated)),
            p.downloads,
            p.rating,
            list(&p.categories),
            p.is_deprecated,
            picks.join(","),
            list(&plan.already),
            list(&plan.missing),
            list(&plan.skipped_loaders)
        );
        Ok(format!("{main},\"versions\":[{}],\"installedVersion\":{}}}", others.join(","), quote(have.as_deref().unwrap_or(""))))
    }

    /// Downloads a package and everything it needs. `each` gets every file as it arrives (for the mod worker);
    /// `progress` gets (label, bytes, total).
    pub fn install_ts(
        &mut self,
        game: &GameInfo,
        wanted: &[String],
        replace: bool,
        each: &mut dyn FnMut(Fetched),
        progress: &mut dyn FnMut(&str, u64, Option<u64>),
    ) -> Result<usize, String> {
        self.no_downloads()?;
        let community = game.community.clone().ok_or("This game isn't on Thunderstore.")?;
        let (index, _) = self.index(&community, false)?;
        let w: Vec<&str> = wanted.iter().map(String::as_str).collect();
        let plan = thunderstore::resolve(&index, &w, &game.installed_ts(), game.loader_ok);
        if !plan.missing.is_empty() {
            crate::log::line("net", &format!("Not on Thunderstore: {}", plan.missing.join(", ")));
        }
        let mut n = 0;
        for pick in &plan.install {
            let label = format!("{} {}", pick.full_name.rsplit_once('-').map(|(_, n)| n.replace('_', " ")).unwrap_or_default(), pick.version);
            CANCEL.store(false, Ordering::SeqCst);
                        let mut cb = |got: u64, total: Option<u64>| {
                progress(&label, got, total);
                !CANCEL.load(Ordering::SeqCst)
            };
            let (d, source) = thunderstore::download(&self.client, pick, Some(&mut cb)).map_err(err)?;
            let path = rename(&d, &label, "zip");
            let replaces = if replace || pick.upgrade { game.mod_for("thunderstore", &pick.full_name) } else { None };
            each(Fetched { path, source, label, replaces });
            n += 1;
        }
        Ok(n)
    }

    /// Installs one version ("Owner-Name-1.2.3"), older or newer than the one installed; it takes the installed one's
    /// place (same spot, on or off).
    pub fn install_ts_version(
        &mut self,
        game: &GameInfo,
        versioned: &str,
        each: &mut dyn FnMut(Fetched),
        progress: &mut dyn FnMut(&str, u64, Option<u64>),
    ) -> Result<usize, String> {
        let dep = thunderstore::parse_dep(versioned).filter(|d| d.version.is_some()).ok_or("That version can't be read.")?;
        let full = dep.full_name();
        let replaces = game.mod_for("thunderstore", &full);
        // Without the installed copy in the list, the chosen version is planned even when it's older.
        let mut g = game.clone();
        g.have.retain(|h| !(h.source.kind == "thunderstore" && h.source.id.eq_ignore_ascii_case(&full)));
        let mut wrap = |mut f: Fetched| {
            if f.source.id.eq_ignore_ascii_case(&full) {
                f.replaces = replaces.clone();
            }
            each(f)
        };
        self.install_ts(&g, &[versioned.to_string()], false, &mut wrap, progress)
    }

    /// Uploads a profile to Thunderstore and returns its code. `mods`: ("Owner-Name", version if known, on).
    pub fn export_code(
        &mut self,
        game: &GameInfo,
        name: &str,
        mods: &[(String, Option<String>, bool)],
        configs: &[(String, Vec<u8>)],
    ) -> Result<String, String> {
        if let Some(t) = self.sample_text("thunderstore_profile_create.json") {
            return thunderstore::parse_create_answer(&t?);
        }
        let community = game.community.clone().ok_or("This game isn't on Thunderstore.")?;
        let needs_index = mods.iter().any(|(_, v, _)| v.as_deref().unwrap_or("").is_empty());
        let index = if needs_index { Some(self.index(&community, false)?.0) } else { None };
        let list = mods
            .iter()
            .map(|(full, v, on)| {
                let version = match v.as_deref().filter(|v| !v.is_empty()) {
                    Some(v) => v.to_string(),
                    None => index
                        .as_ref()
                        .and_then(|i| thunderstore::find(i, full))
                        .and_then(|p| p.latest())
                        .map(|x| x.number.clone())
                        .unwrap_or_default(),
                };
                thunderstore::ProfileMod { full_name: full.clone(), version, enabled: *on }
            })
            .filter(|m| !m.version.is_empty())
            .collect();
        let profile = thunderstore::SharedProfile { name: name.to_string(), mods: list, files: Vec::new() };
        thunderstore::export_code(&self.client, &profile, configs).map_err(err)
    }

    // ------------------------------------------------------------------------------------------------ Nexus

    pub fn has_key(&self) -> bool {
        self.key.is_some()
    }

    fn nexus(&self) -> Option<Nexus> {
        let key = self.key.as_ref()?;
        Some(Nexus::new(self.client.clone(), key, &self.version))
    }

    /// For browsing: with the key when there is one, otherwise without (v2 GraphQL needs none).
    fn nexus_any(&self) -> Nexus {
        Nexus::with_key(self.client.clone(), self.key.as_deref(), &self.version)
    }

    fn nexus_public(&self) -> Nexus {
        Nexus::public(self.client.clone(), &self.version)
    }

    /// Labs: one search without a key, to show whether Nexus answers keyless browsing on this device.
    pub fn keyless_check(&mut self, domain: &str) -> Result<String, String> {
        if self.sample.is_some() {
            return if self.sample_nexus_fail { Err("Nexus answered 401 (test run)".into()) } else { Ok("Nexus answered with 12 mods (test run)".into()) };
        }
        let p = self.nexus_public().search(domain, "", nexus::SearchSort::Endorsements, 0, 5, false).map_err(err)?;
        Ok(format!("Nexus answered without a key: {} mods for {domain}", p.total))
    }

    /// Checks a key with Nexus and keeps it (saved in a file only the player can read) when it works.
    pub fn set_key(&mut self, key: &str) -> Result<String, String> {
        let key = key.trim().to_string();
        if !nexus::plausible_key(&key) {
            return Err("That doesn't look like a Nexus API key. It's a long line of letters and numbers.".into());
        }
        crate::log::secret(&key);
        let account = if let Some(t) = self.sample_text("nexus_validate.json") {
            nexus::parse_validate(&t?)?
        } else {
            Nexus::new(self.client.clone(), &key, &self.version).validate().map_err(err)?
        };
        if self.sample.is_none() {
            nexus::save_key(&self.key_file, &key).map_err(|e| format!("The key works, but couldn't be saved ({e})."))?;
        }
        if self.sample.is_none() {
            *shared_key() = Some(key.clone());
        }
        self.key = Some(key);
        self.account = Some(account);
        Ok(self.account_json("ok", ""))
    }

    pub fn sign_out(&mut self) -> String {
        let _ = fs::remove_file(&self.key_file);
        self.key = None;
        self.account = None;
        *shared_key() = None;
        self.account_json("none", "Signed out. The key was deleted from this device.")
    }

    /// The account, checked with Nexus the first time it's needed.
    pub fn account(&mut self) -> Result<Account, String> {
        if let Some(a) = &self.account {
            return Ok(a.clone());
        }
        let a = if let Some(t) = self.sample_text("nexus_validate.json") {
            nexus::parse_validate(&t?)?
        } else {
            self.nexus().ok_or("Add your Nexus Mods key in Settings, Accounts first.")?.validate().map_err(err)?
        };
        self.account = Some(a.clone());
        Ok(a)
    }

    pub fn account_json(&self, state: &str, message: &str) -> String {
        let (name, premium) = self.account.as_ref().map(|a| (a.name.as_str(), a.is_premium)).unwrap_or(("", false));
        format!(
            "{{\"state\":{},\"name\":{},\"premium\":{},\"hasKey\":{},\"message\":{}}}",
            quote(state),
            quote(name),
            premium,
            self.key.is_some() || (self.sample.is_some() && self.account.is_some()),
            quote(message)
        )
    }

    /// For the screenshot checks: signed in with the sample account.
    pub fn sample_sign_in(&mut self, free: bool) {
        if let Some(Ok(t)) = self.sample_text("nexus_validate.json") {
            self.account = nexus::parse_validate(&t).ok();
            if free {
                if let Some(a) = self.account.as_mut() {
                    a.is_premium = false;
                }
            }
            self.key = Some("sample-key-for-screenshots".into());
        }
    }

    /// Get mods for a Nexus game: one page of 30 from `offset`. Without a key (or when the keyed call fails) it asks
    /// without one; the error tells the app to fall back to opening Nexus in the browser.
    pub fn browse_nexus(&mut self, game: &GameInfo, text: &str, category: &str, sort: &str, offset: u32) -> Result<String, String> {
        let domain = game.domain.clone().ok_or("This game isn't on Nexus Mods.")?;
        let s = nexus::SearchSort::from_key(sort);
        let keyless = self.key.is_none();
        let page = if self.sample.is_some() {
            if self.sample_nexus_fail {
                return Err("Nexus answered 401 (test run)".into());
            }
            // Each game's own sample when there is one (Cyberpunk's grid shows Cyberpunk mods), else the shared one.
            let own = format!("nexus_graphql_search_{domain}.json");
            let t = self.sample_text(&own).filter(|t| t.is_ok()).or_else(|| self.sample_text("nexus_graphql_search.json")).ok_or("The search sample is missing.")??;
            let mut p = nexus::parse_search(&t)?;
            if !category.is_empty() {
                p.mods.retain(|m| m.category == category);
            }
            // A search in the samples keeps the mods whose name has the words.
            let words = text.trim().to_lowercase();
            if !words.is_empty() {
                p.mods.retain(|m| m.name.to_lowercase().contains(&words) || m.summary.to_lowercase().contains(&words));
            }
            if offset > 0 {
                p.mods.clear();
            }
            p
        } else {
            let first = self.nexus_any().search_in(&domain, text, category, s, offset, 30, false);
            match first {
                Ok(p) => p,
                Err(e) if !keyless => {
                    crate::log::line("net", &format!("Nexus browse with the key failed ({e}); trying without it"));
                    self.nexus_public().search_in(&domain, text, category, s, offset, 30, false).map_err(err)?
                }
                Err(e) => return Err(err(e)),
            }
        };
        if keyless && offset == 0 {
            crate::log::line("net", &format!("Nexus browse without a key: {} mods for {domain} ({} in all)", page.mods.len(), page.total));
        }
        if page.relaxed {
            crate::log::line("net", "Nexus refused that sort; showing Most endorsed instead");
        }
        let items: Vec<String> = page
            .mods
            .iter()
            .map(|m| {
                format!(
                    "{{\"id\":{},\"name\":{},\"owner\":{},\"summary\":{},\"downloads\":{},\"rating\":{},\"icon\":{},\"code\":false,\"version\":{},\"size\":0,\"updated\":{},\"category\":{},\"installed\":{}}}",
                    quote(&m.mod_id.to_string()),
                    quote(&m.name),
                    quote(&m.author),
                    quote(&m.summary),
                    m.downloads,
                    m.endorsements,
                    quote(&self.pic(if m.thumbnail_url.is_empty() { &m.picture_url } else { &m.thumbnail_url })),
                    quote(&m.version),
                    quote(&day(&m.updated_at)),
                    quote(&m.category),
                    game.have.iter().any(|h| h.source.kind == "nexus"
                        && nexus::parse_source_id(&h.source.id).map(|(_, id, _)| id).or_else(|| nexus::source_mod(&h.source).map(|(_, m)| m)).is_some_and(|id| id == m.mod_id))
                )
            })
            .collect();
        Ok(format!(
            "{{\"source\":\"nexus\",\"total\":{},\"offset\":{offset},\"relaxed\":{},\"keyless\":{keyless},\"note\":\"\",\"category\":{},\"categories\":[{}],\"items\":[{}]}}",
            page.total,
            page.relaxed,
            quote(category),
            page.categories.iter().map(|(c, _)| quote(c)).collect::<Vec<_>>().join(","),
            items.join(",")
        ))
    }

    fn nexus_files(&self, domain: &str, mod_id: u64) -> Result<nexus::FileList, String> {
        self.nexus_files_for(domain, mod_id, None)
    }

    fn nexus_files_for(&self, domain: &str, mod_id: u64, game_id: Option<u64>) -> Result<nexus::FileList, String> {
        if self.sample.is_some() {
            if self.graphql_sample(domain) {
                return nexus::parse_mod_files(&self.sample_text("nexus_graphql_files.json").ok_or("The files sample is missing.")??);
            }
            if let Some(t) = self.sample_text("nexus_files_all.json").filter(|t| t.is_ok()).or_else(|| self.sample_text("nexus_files.json")) {
                return nexus::parse_files(&t?);
            }
        }
        self.nexus_any().files_any(domain, mod_id, game_id).map_err(err)
    }

    /// The screenshot checks: the GraphQL details sample is used for its own game (The Witcher 3), the v1 samples
    /// for the rest.
    fn graphql_sample(&self, domain: &str) -> bool {
        self.sample_text("nexus_graphql_details.json")
            .and_then(|t| t.ok())
            .and_then(|t| nexus::parse_mod_details(&t, domain).ok())
            .is_some_and(|d| d.info.domain == domain)
    }

    /// What a Nexus mod needs (other Nexus mods, outside downloads, DLC), checked against the game's mods.
    fn nexus_requirements(&self, game: &GameInfo, domain: &str, mod_id: u64) -> Result<nexus::RequirementCheck, String> {
        let reqs = if self.sample.is_some() && self.graphql_sample(domain) {
            nexus::parse_requirements(&self.sample_text("nexus_graphql_details.json").ok_or("The details sample is missing.")??, domain)?
        } else if let Some(t) = self.sample_text("nexus_graphql_requirements.json") {
            nexus::parse_requirements(&t?, domain)?
        } else {
            self.nexus_any().requirements(domain, mod_id).map_err(err)?
        };
        let installed: Vec<(String, SourceInfo)> = game.have.iter().map(|h| (h.mod_id.clone(), h.source.clone())).collect();
        Ok(nexus::check_requirements(&reqs, &installed))
    }

    /// The installed file of a Nexus mod in this game, if any.
    fn installed_file(game: &GameInfo, domain: &str, mod_id: u64) -> Option<u64> {
        game.have.iter().find_map(|h| {
            nexus::source_mod(&h.source).filter(|(d, m)| *m == mod_id && (d.is_empty() || d == domain))?;
            nexus::parse_source_id(&h.source.id).map(|(_, _, f)| f)
        })
    }

    /// A Nexus mod's name (no key needed). The build's test runs have no internet: "" there.
    pub fn mod_name(&mut self, domain: &str, mod_id: u64) -> Result<String, String> {
        if self.sample.is_some() {
            return Ok(String::new());
        }
        let (d, _) = self.nexus_any().details(domain, mod_id).map_err(err)?;
        Ok(d.info.name)
    }

    /// A Nexus file's name and version (with the key; the build's test runs read the recorded file details).
    pub fn file_name(&mut self, domain: &str, mod_id: u64, file_id: u64) -> Result<String, String> {
        let file = if self.sample.is_some() {
            nexus::parse_file(&self.sample_text("nexus_file.json").ok_or("The sample file details are missing.")??)?
        } else {
            self.nexus().ok_or("no key")?.file_info(domain, mod_id, file_id).map_err(err)?
        };
        Ok(format!("{} {}", file.name, file.version).trim().to_string())
    }

    pub fn details_nexus(&mut self, game: &GameInfo, mod_id: u64) -> Result<String, String> {
        let domain = game.domain.clone().ok_or("This game isn't on Nexus Mods.")?;
        // The page's facts through GraphQL (no key needed); the v1 sample for the older screenshot checks.
        let (info, game_id) = if self.sample.is_some() && !self.graphql_sample(&domain) {
            (nexus::parse_mod(&self.sample_text("nexus_mod.json").ok_or("The mod sample is missing.")??)?, None)
        } else if self.sample.is_some() {
            let d = nexus::parse_mod_details(&self.sample_text("nexus_graphql_details.json").ok_or("The details sample is missing.")??, &domain)?;
            (d.info, Some(d.game_id))
        } else {
            let (d, _) = self.nexus_any().details(&domain, mod_id).map_err(err)?;
            (d.info, Some(d.game_id))
        };
        // Each part of the page stands alone: files that can't be read only leave the file lines empty (the note
        // says why); the page itself, its picture and Open on Nexus still show.
        let (files, files_note) = match self.nexus_files_for(&domain, mod_id, game_id) {
            Ok(f) => (f, String::new()),
            Err(e) => {
                crate::log::line("net", &format!("Files of {domain} {mod_id}: {e}"));
                (nexus::FileList::default(), format!("Turbine couldn't read its files ({e}). Open on Nexus shows them."))
            }
        };
        let main = files.newest_main().cloned();
        let premium = self.key.is_some() && self.account().map(|a| a.is_premium).unwrap_or(false);
        let pick = main
            .as_ref()
            .map(|f| {
                format!(
                    "{{\"id\":{},\"name\":{},\"version\":{},\"size\":{},\"main\":true,\"upgrade\":false,\"summary\":{}}}",
                    quote(&f.file_id.to_string()),
                    quote(&f.name),
                    quote(&f.version),
                    f.size_bytes,
                    quote(&f.description)
                )
            })
            .unwrap_or_default();
        // Requirements: a failure only means the page shows none (the note says why).
        let (needs, needs_note) = match self.nexus_requirements(game, &domain, mod_id) {
            Ok(c) => (nexus::requirements_json(&c, premium), String::new()),
            Err(e) => {
                crate::log::line("net", &format!("Requirements of {domain} {mod_id}: {e}"));
                ("null".to_string(), format!("Turbine couldn't check what it needs ({e})"))
            }
        };
        // Other versions: every file but the installed one (or the main file Install gets).
        let have = Self::installed_file(game, &domain, mod_id);
        let skip = have.or(main.as_ref().map(|f| f.file_id)).unwrap_or(0);
        let mut others: Vec<String> = Vec::new();
        for g in files.other_versions(skip) {
            for f in &g.files {
                let first_line = f.description.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("").to_string();
                others.push(format!(
                    "{{\"id\":{},\"version\":{},\"name\":{},\"size\":{},\"date\":{},\"downloads\":0,\"installed\":false,\"group\":{},\"note\":{}}}",
                    quote(&f.file_id.to_string()),
                    quote(&f.version),
                    quote(&f.name),
                    f.size_bytes,
                    quote(&date_of(f.uploaded_timestamp)),
                    quote(g.label),
                    quote(&first_line.chars().take(120).collect::<String>())
                ));
            }
        }
        let page = nexus::files_page_url(&domain, mod_id);
        let file_page = main.as_ref().map(|f| nexus::file_page_url(&domain, mod_id, f.file_id)).unwrap_or_else(|| page.clone());
        let main_line = main
            .as_ref()
            .map(|f| format!("Main file: {} {}{}", f.name, f.version, if f.size_bytes > 0 { format!(" · {}", net::human_size(f.size_bytes)) } else { String::new() }))
            .unwrap_or_default();
        let updated = if info.updated_timestamp > 0 { date_of(info.updated_timestamp) } else { String::new() };
        let detail_json = format!(
            ",\"filesNote\":{},\"filePage\":{},\"fileId\":{},\"mainLine\":{},\"updatedText\":{},\"author\":{},\"endorsements\":{},\"installed\":{},\"picture\":{}",
            quote(&files_note),
            quote(&file_page),
            quote(&main.as_ref().map(|f| f.file_id.to_string()).unwrap_or_default()),
            quote(&main_line),
            quote(&updated),
            quote(&info.author),
            info.endorsements,
            Self::installed_file(game, &domain, mod_id).is_some() || game.have.iter().any(|h| nexus::source_mod(&h.source).is_some_and(|(_, m)| m == mod_id)),
            quote(&self.pic(&info.picture_url))
        );
        let out = format!(
            "{{\"source\":\"nexus\",\"id\":{},\"name\":{},\"owner\":{},\"summary\":{},\"icon\":{},\"code\":false,\"version\":{},\"size\":{},\"updated\":\"\",\"downloads\":{},\"rating\":{},\"categories\":[],\"deprecated\":{},\"install\":[{}],\"already\":[],\"missing\":[],\"skippedLoaders\":[],\"premium\":{},\"message\":{},\"page\":{},\"needs\":{},\"needsNote\":{},\"versions\":[{}],\"installedVersion\":{},\"versionsNote\":{}}}",
            quote(&mod_id.to_string()),
            quote(&info.name),
            quote(&info.author),
            quote(&info.summary),
            quote(&self.pic(&info.picture_url)),
            quote(&info.version),
            main.as_ref().map(|f| f.size_bytes).unwrap_or(0),
            info.downloads,
            info.endorsements,
            !info.available,
            pick,
            premium,
            quote(if premium { "" } else { FREE_WORDS }),
            quote(&page),
            needs,
            quote(&needs_note),
            others.join(","),
            quote(&have.and_then(|f| files.file(f)).map(|f| f.version.clone()).unwrap_or_default()),
            quote(if !files_note.is_empty() { files_note.as_str() } else if premium { "" } else { "A opens that file's page on Nexus. Press Slow download there and Turbine adds it by itself." })
        );
        Ok(format!("{}{}}}", &out[..out.len() - 1], detail_json))
    }

    /// Downloads the requirements Turbine can fetch for a Premium account (the Download steps), each handed to
    /// `each`. Returns the names of the ones that need a page opened instead.
    pub fn install_requirements(
        &mut self,
        game: &GameInfo,
        mod_id: u64,
        each: &mut dyn FnMut(Fetched),
        progress: &mut dyn FnMut(&str, u64, Option<u64>),
    ) -> Result<Vec<String>, String> {
        let domain = game.domain.clone().ok_or("This game isn't on Nexus Mods.")?;
        let check = self.nexus_requirements(game, &domain, mod_id)?;
        let account = self.account()?;
        let mut pages = Vec::new();
        for step in nexus::requirement_steps(&check.missing, account.is_premium) {
            match step {
                nexus::RequirementStep::Download { name, domain, mod_id } => {
                    self.no_downloads()?;
                    let f = self.fetch_by_id(&domain, mod_id, None, &name, progress)?;
                    each(f);
                }
                nexus::RequirementStep::OpenPage { name, .. } => pages.push(name),
            }
        }
        Ok(pages)
    }

    /// One Nexus mod by its IDs (Premium): that file, or the newest replacing it, or the main file.
    pub fn fetch_by_id(
        &mut self,
        domain: &str,
        mod_id: u64,
        file_id: Option<u64>,
        name: &str,
        progress: &mut dyn FnMut(&str, u64, Option<u64>),
    ) -> Result<Fetched, String> {
        self.no_downloads()?;
        let account = self.account()?;
        let nx = self.nexus().ok_or("Add your Nexus Mods key in Settings, Accounts first.")?;
        let label = name.to_string();
        CANCEL.store(false, Ordering::SeqCst);
        let mut cb = |got: u64, total: Option<u64>| {
            progress(&label, got, total);
            !CANCEL.load(Ordering::SeqCst)
        };
        if !account.is_premium {
            return Err(nexus::FREE_ACCOUNT_MESSAGE.into());
        }
        // The same choice download_by_id makes, read here so the file keeps its kind (zip, 7z or rar).
        let files = nx.files(domain, mod_id).map_err(err)?;
        let file = files.pick(file_id).cloned().ok_or("This mod has no file on Nexus that can be downloaded.")?;
        let (d, source) = nx.download(&account, domain, mod_id, &file, Some(&mut cb)).map_err(err)?;
        let full = format!("{} {}", name, file.version).trim().to_string();
        let path = rename(&d, &full, &ext_of(&file.file_name));
        Ok(Fetched { path, source, label: full, replaces: Some(format!("{NEXUS_MOD}{domain}:{mod_id}")) })
    }

    /// Downloads what an nxm link (Mod Manager Download on the site) points at. Works for free accounts.
    /// `into`: Turbine's browser asked for it, so the file goes into that Game Mods folder afterwards (web.rs).
    pub fn fetch_nxm(&mut self, link: &turbine_engine::nxm::NxmLink, into: Option<&std::path::Path>, progress: &mut dyn FnMut(&str, u64, Option<u64>)) -> Result<Fetched, String> {
        if self.sample.is_some() {
            return self.pretend_nxm(link, into, progress);
        }
        if self.nexus().is_none() {
            return Err("Add your Nexus Mods key in Settings, Accounts first, then press Mod Manager Download again.".into());
        }
        // The link is for one account (its user_id): another account's link is refused before anything downloads.
        if link.user_id.is_some() {
            let me = self.account().map(|a| a.user_id).unwrap_or(0);
            crate::browse::user_matches(link.user_id, me)?;
        }
        let nx = self.nexus().ok_or("Add your Nexus Mods key in Settings, Accounts first, then press Mod Manager Download again.")?;
        let file = nx.file_info(&link.domain, link.mod_id, link.file_id).map_err(err)?;
        let label = format!("{} {}", file.name, file.version).trim().to_string();
        CANCEL.store(false, Ordering::SeqCst);
        let mut cb = |got: u64, total: Option<u64>| {
            progress(&label, got, total);
            !CANCEL.load(Ordering::SeqCst)
        };
        let (d, source) = nx.download_from_nxm(link, Some(&mut cb)).map_err(err)?;
        let path = rename(&d, &label, &ext_of(&file.file_name));
        Ok(Fetched { path, source, label, replaces: Some(format!("{NEXUS_MOD}{}:{}", link.domain, link.mod_id)) })
    }

    /// The screenshot checks have no internet: a link shows its progress from the recorded file details, then stops.
    fn pretend_nxm(&mut self, link: &turbine_engine::nxm::NxmLink, into: Option<&std::path::Path>, progress: &mut dyn FnMut(&str, u64, Option<u64>)) -> Result<Fetched, String> {
        let file = nexus::parse_file(&self.sample_text("nexus_file.json").ok_or("The sample file details are missing.")??)?;
        let label = format!("{} {}", file.name, file.version).trim().to_string();
        let total = file.size_bytes.max(1);
        CANCEL.store(false, Ordering::SeqCst);
        match into {
            Some(dir) => crate::log::line("nxm", &format!("Test run: pretending to download {} into Game Mods/{}", link.redacted(), dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())),
            None => crate::log::line("nxm", &format!("Test run: pretending to download {}", link.redacted())),
        }
        for i in 0..=200u64 {
            if CANCEL.load(Ordering::SeqCst) {
                break;
            }
            progress(&label, total * i.min(16) * 4 / 100, Some(total));
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        Err("Downloads are off in this test run.".into())
    }

    /// Downloads a Nexus file (Premium only). `file_id` 0 means the main file.
    pub fn install_nexus(
        &mut self,
        game: &GameInfo,
        mod_id: u64,
        file_id: u64,
        replaces: Option<String>,
        each: &mut dyn FnMut(Fetched),
        progress: &mut dyn FnMut(&str, u64, Option<u64>),
    ) -> Result<usize, String> {
        self.no_downloads()?;
        let domain = game.domain.clone().ok_or("This game isn't on Nexus Mods.")?;
        let account = self.account()?;
        if !account.is_premium {
            return Err(nexus::FREE_ACCOUNT_MESSAGE.into());
        }
        let files = self.nexus_files(&domain, mod_id)?;
        let file = if file_id == 0 { files.newest_main() } else { files.files.iter().find(|f| f.file_id == file_id) }
            .cloned()
            .ok_or("Nexus lists no file to download for that mod.")?;
        let nx = self.nexus().ok_or("Add your Nexus Mods key in Settings, Accounts first.")?;
        let label = format!("{} {}", file.name, file.version).trim().to_string();
        CANCEL.store(false, Ordering::SeqCst);
                let mut cb = |got: u64, total: Option<u64>| {
            progress(&label, got, total);
            !CANCEL.load(Ordering::SeqCst)
        };
        let (d, mut source) = nx.download(&account, &domain, mod_id, &file, Some(&mut cb)).map_err(err)?;
        source.needs = nexus_needs(&domain, mod_id);
        let path = rename(&d, &label, &ext_of(&file.file_name));
        each(Fetched { path, source, label, replaces });
        Ok(1)
    }

    // ------------------------------------------------------------------------------------------------ LOOT

    /// LOOT's masterlist for Skyrim Special Edition: the copy on this device, refreshed at most once a day (or the
    /// recorded sample in the screenshot checks). The note says what went wrong, if anything.
    pub fn loot_lists(&self, force: bool) -> (Option<crate::loot::Lists>, String) {
        if let Some(dir) = &self.sample {
            return match crate::loot::samples(dir) {
                Some(l) => (Some(l), String::new()),
                None => (None, "LOOT's masterlist isn't in the samples.".into()),
            };
        }
        let dir = crate::loot::folder(&self.cache);
        if let Err(e) = fs::create_dir_all(&dir) {
            return (crate::loot::on_device(&dir), format!("LOOT's masterlist folder couldn't be made ({e})."));
        }
        crate::loot::lists(&self.client, &dir, force)
    }

    // ------------------------------------------------------------------------------------------------ loaders

    pub fn fetch_loader(&mut self, game: &GameInfo, progress: &mut dyn FnMut(&str, u64, Option<u64>)) -> Result<Fetched, String> {
        self.no_downloads()?;
        CANCEL.store(false, Ordering::SeqCst);
                let (label, ext) = match game.kind.as_str() {
            "bethesda" => ("SKSE", "7z"),
            "bepinex" => ("BepInEx", "zip"),
            "smapi" => ("SMAPI", "zip"),
            _ => return Err("This game has no mod loader Turbine can install.".into()),
        };
        let mut cb = |got: u64, total: Option<u64>| {
            progress(label, got, total);
            !CANCEL.load(Ordering::SeqCst)
        };
        let got = match game.kind.as_str() {
            "bethesda" => {
                let edition = if game.steam { loaders::SkseEdition::SteamAe } else { loaders::SkseEdition::GogAe };
                let nx = self.nexus();
                let account = if nx.is_some() { self.account().ok() } else { None };
                let pair = match (&nx, &account) {
                    (Some(n), Some(a)) => Some((n, a)),
                    _ => None,
                };
                loaders::fetch_skse(&self.client, edition, pair, Some(&mut cb)).map_err(err)?
            }
            "bepinex" => {
                let community = game.community.clone().ok_or("Turbine doesn't know where this game's BepInEx comes from.")?;
                let (index, _) = self.index(&community, false)?;
                loaders::fetch_bepinex(&self.client, &index, &community, game.native, Some(&mut cb)).map_err(err)?
            }
            _ => loaders::fetch_smapi(&self.client, game.native, Some(&mut cb)).map_err(err)?,
        };
        for n in &got.plan.notes {
            crate::log::line("net", &format!("{label}: {n}"));
        }
        let version = got.source.version.clone().unwrap_or_default();
        let name = format!("{label} {version}");
        let path = rename(&got.download, &name, ext);
        Ok(Fetched { path, source: got.source, label: name, replaces: None })
    }

    // ------------------------------------------------------------------------------------------------ updates

    /// Mods with a newer version, as JSON: [{"mod","kind","id","from","to","target","deprecated"}].
    pub fn check_updates(&mut self, game: &GameInfo) -> Result<String, String> {
        let installed = game.installed();
        let mut found: Vec<updates::Update> = Vec::new();
        let mut notes = Vec::new();
        if let Some(c) = game.community.clone() {
            if installed.iter().any(|m| m.kind == "thunderstore") {
                match self.index(&c, false) {
                    Ok((index, _)) => found.extend(updates::thunderstore_updates(&installed, &index)),
                    Err(e) => notes.push(e),
                }
            }
        }
        // Nexus without a key: one GraphQL lookup for all of the game's Nexus mods (version and last change), then
        // the files of the ones that changed since they were added.
        if let Some(domain) = game.domain.clone() {
            if installed.iter().any(|m| m.kind == "nexus") {
                let ids = updates::nexus_mod_ids(&domain, &installed);
                let stamps = if self.sample.is_some() {
                    match self.sample_text("nexus_graphql_by_id.json") {
                        Some(Ok(t)) => nexus::parse_by_id(&t).map_err(|e| e.to_string()),
                        _ => Ok(Vec::new()),
                    }
                } else {
                    self.nexus_any().mods_by_id(&domain, &ids).map_err(err)
                };
                match stamps {
                    Ok(stamps) => {
                        for (mod_id, m, game_id) in updates::nexus_stamp_candidates(&domain, &installed, &stamps) {
                            match self.nexus_files_for(&domain, mod_id, Some(game_id).filter(|g| *g > 0)) {
                                Ok(files) => found.extend(updates::nexus_update_from_files(m, &files)),
                                Err(e) => notes.push(e),
                            }
                        }
                    }
                    Err(e) => notes.push(e),
                }
            }
        }
        let items: Vec<String> = found
            .iter()
            .filter_map(|u| {
                let mod_id = match u.kind.as_str() {
                    "nexus" => {
                        let (d, m, _) = nexus::parse_source_id(&u.id)?;
                        game.have
                            .iter()
                            .find(|h| nexus::source_mod(&h.source).is_some_and(|(hd, hm)| (hd == d || hd.is_empty()) && hm == m))
                            .map(|h| h.mod_id.clone())?
                    }
                    _ => game.mod_for(&u.kind, &u.id)?,
                };
                // Free accounts open the new file's page (the download then takes the old one's place).
                let page = nexus::parse_source_id(&u.id).map(|(d, m, f)| nexus::file_page_url(&d, m, f)).unwrap_or_default();
                Some(format!(
                    "{{\"mod\":{},\"kind\":{},\"id\":{},\"from\":{},\"to\":{},\"target\":{},\"deprecated\":{},\"page\":{}}}",
                    quote(&mod_id),
                    quote(&u.kind),
                    quote(&u.id),
                    quote(u.installed_version.as_deref().unwrap_or("")),
                    quote(&u.new_version),
                    quote(&u.target),
                    u.deprecated,
                    quote(&page)
                ))
            })
            .collect();
        crate::log::line("net", &format!("Update check for {}: {} found{}", game.name, items.len(), if notes.is_empty() { String::new() } else { format!("; {}", notes.join("; ")) }));
        Ok(format!("{{\"game\":{},\"items\":[{}],\"note\":{}}}", quote(&game.id), items.join(","), quote(&notes.join(" "))))
    }

    /// Downloads one update (`kind`, `target` as check_updates gave them) for mod `mod_id`.
    pub fn fetch_update(
        &mut self,
        game: &GameInfo,
        mod_id: &str,
        kind: &str,
        target: &str,
        each: &mut dyn FnMut(Fetched),
        progress: &mut dyn FnMut(&str, u64, Option<u64>),
    ) -> Result<usize, String> {
        match kind {
            "thunderstore" => self.install_ts(game, &[target.to_string()], true, each, progress),
            "nexus" => {
                let have = game.have.iter().find(|h| h.mod_id == mod_id).ok_or("That mod isn't installed any more.")?;
                let (_, nexus_mod) = nexus::source_mod(&have.source).ok_or("Turbine doesn't know where that mod came from.")?;
                let file_id: u64 = target.parse().map_err(|_| "Nexus's answer can't be read.".to_string())?;
                self.install_nexus(game, nexus_mod, file_id, Some(mod_id.to_string()), each, progress)
            }
            _ => Err("Turbine can't update mods added from a file. Add the new file from Downloads.".into()),
        }
    }

    // ------------------------------------------------------------------------------------------------ profiles

    /// A Thunderstore profile code as a share for the profile import.
    pub fn read_code(&mut self, game: &GameInfo, code: &str) -> Result<turbine_engine::profiles::Share, String> {
        self.no_downloads()?;
        let (p, path) = thunderstore::import_code(&self.client, code).map_err(err)?;
        let _ = fs::remove_file(path);
        let (store, id) = game.id.split_once(':').unwrap_or(("steam", ""));
        Ok(turbine_engine::profiles::Share {
            game_name: game.name.clone(),
            game_store: store.to_string(),
            game_id: id.to_string(),
            profile: if p.name.trim().is_empty() { "Shared profile".into() } else { p.name.clone() },
            mods: p
                .mods
                .iter()
                .map(|m| turbine_engine::profiles::ShareMod {
                    name: m.full_name.rsplit_once('-').map(|(_, n)| n.replace('_', " ")).unwrap_or_else(|| m.full_name.clone()),
                    enabled: m.enabled,
                    kind: "thunderstore".into(),
                    id: m.full_name.clone(),
                    version: Some(m.version.clone()),
                    url: None,
                    note: None,
                })
                .collect(),
        })
    }

    // ------------------------------------------------------------------------------------------------ Turbine itself

    /// Looks for a newer Turbine. `back`: the newest stable release even if it's older (Go back to stable).
    pub fn self_check(&mut self, current: &str, labs: bool, back: bool) -> Result<Option<selfupdate::Release>, String> {
        if let Some(t) = self.sample_text("github_turbine_releases.json") {
            let t = t?;
            if back {
                return selfupdate::pick_release(&t, false);
            }
            return match selfupdate::decide(&t, current, labs)? {
                selfupdate::Check::Available(r) => Ok(Some(*r)),
                selfupdate::Check::UpToDate => Ok(None),
            };
        }
        if back {
            let text = self
                .client
                .send(&net::Request::get(selfupdate::LATEST_URL).header("Accept", "application/vnd.github+json").max_bytes(8 << 20).max_time(30), None)
                .map_err(err)?
                .take_text()
                .map_err(err)?;
            return selfupdate::pick_release(&text, false);
        }
        match selfupdate::check(&self.client, current, labs).map_err(err)? {
            selfupdate::Check::Available(r) => Ok(Some(*r)),
            selfupdate::Check::UpToDate => Ok(None),
        }
    }

    /// Downloads a release into Turbine's cache (a folder the host can read) and installs it on the host.
    pub fn self_install(&mut self, release: &selfupdate::Release, progress: &mut dyn FnMut(&str, u64, Option<u64>)) -> Result<(), String> {
        self.no_downloads()?;
        let dir = self.cache.join("update");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).map_err(|e| format!("Couldn't make the update folder ({e})."))?;
        let client = Client { temp_dir: dir.clone(), ..self.client.clone() };
        let label = format!("Turbine {}", release.version);
        CANCEL.store(false, Ordering::SeqCst);
                let mut cb = |got: u64, total: Option<u64>| {
            progress(&label, got, total);
            !CANCEL.load(Ordering::SeqCst)
        };
        let d = selfupdate::download(&client, release, Some(&mut cb)).map_err(err)?;
        let bundle = dir.join(selfupdate::ASSET);
        fs::rename(&d.path, &bundle).map_err(|e| format!("Couldn't keep the update ({e})."))?;
        let args = selfupdate::install_args(&bundle);
        crate::log::line("update", &format!("Installing: {}", args.join(" ")));
        let out = std::process::Command::new(&args[0])
            .args(&args[1..])
            .stdin(std::process::Stdio::null())
            .output()
            .map_err(|e| format!("The installer couldn't start ({e})."))?;
        let _ = fs::remove_file(&bundle);
        if !out.status.success() {
            let msg = String::from_utf8_lossy(&out.stderr);
            let line = msg.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("it stopped with an error").trim().to_string();
            crate::log::line("update", &format!("Install failed: {msg}"));
            return Err(format!("The update couldn't be installed ({line})."));
        }
        crate::log::line("update", &format!("Installed Turbine {}", release.version));
        Ok(())
    }
}

/// What a Nexus mod needs, as recorded with it (needs::nexus_need entries), read from its page without a key. Empty
/// in the build's screenshot checks or when Nexus can't be reached (the mod menu then just says nothing about it).
pub fn nexus_needs(domain: &str, mod_id: u64) -> Vec<String> {
    if using_samples() {
        return Vec::new();
    }
    let nx = Nexus::public(Client::new(crate::VERSION, &temp_dir()), crate::VERSION);
    match nx.requirements(domain, mod_id) {
        Ok(list) => list
            .iter()
            .filter(|r| r.mod_id > 0 && !r.external && !r.is_dlc)
            .map(|r| turbine_engine::needs::nexus_need(if r.domain.is_empty() { domain } else { &r.domain }, r.mod_id, &r.name))
            .collect(),
        Err(e) => {
            crate::log::line("net", &format!("What {domain} {mod_id} needs couldn't be read: {e}"));
            Vec::new()
        }
    }
}

/// A release as JSON for Settings > Updates.
pub fn release_json(state: &str, r: Option<&selfupdate::Release>, message: &str) -> String {
    let notes: String = r.map(|r| r.notes.chars().take(1200).collect()).unwrap_or_default();
    format!(
        "{{\"state\":{},\"version\":{},\"prerelease\":{},\"notes\":{},\"size\":{},\"message\":{}}}",
        quote(state),
        quote(r.map(|r| r.version.as_str()).unwrap_or("")),
        r.map(|r| r.prerelease).unwrap_or(false),
        quote(&notes),
        r.map(|r| r.asset_size).unwrap_or(0),
        quote(message)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use turbine_engine::json;

    fn data() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("engine/tests/data")
    }

    fn game() -> GameInfo {
        GameInfo {
            id: "steam:1966720".into(),
            name: "Lethal Company".into(),
            community: Some("lethal-company".into()),
            domain: Some("lethalcompany".into()),
            kind: "bepinex".into(),
            loader_ok: true,
            native: false,
            steam: true,
            have: Vec::new(),
        }
    }

    #[test]
    fn browses_and_details_from_samples() {
        let tmp = std::env::temp_dir().join("turbine-online-test");
        let mut n = Net::new("0.3", &tmp, &tmp);
        n.use_samples(&data());
        let b = n.browse_ts(&game(), "", "", "downloads").unwrap();
        let j = json::parse(&b).unwrap();
        let items = j.get("items").unwrap().as_array();
        assert!(!items.is_empty());
        let first = items[0].str_at("id").unwrap().to_string();
        let d = n.details_ts(&game(), &first).unwrap();
        assert!(json::parse(&d).is_ok(), "{d}");
        let nb = n.browse_nexus(&game(), "", "", 0).unwrap();
        assert!(json::parse(&nb).is_ok());
        let nd = n.details_nexus(&game(), 2400).unwrap();
        assert!(json::parse(&nd).is_ok(), "{nd}");
        assert!(n.install_ts(&game(), &[first], false, &mut |_| {}, &mut |_, _, _| {}).is_err());
        let r = n.self_check("0.2", false, false).unwrap();
        println!("{:?}", r.map(|r| r.version));
        let u = n.check_updates(&game()).unwrap();
        assert!(json::parse(&u).is_ok());
        n.sample_sign_in(false);
        assert!(n.account_json("ok", "").contains("\"hasKey\":true"));
    }
}
