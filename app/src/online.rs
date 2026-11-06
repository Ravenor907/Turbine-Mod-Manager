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
use turbine_engine::json::{self, quote, Json};
use turbine_engine::net::{self, Client, Download, NetError, SourceInfo};
use turbine_engine::nexus::{self, Account, Nexus};
use turbine_engine::thunderstore::{self, Package, Query, Sort};
use turbine_engine::{loaders, selfupdate, updates};

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
}

fn err(e: NetError) -> String {
    e.to_string()
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
        }
    }

    /// Recorded samples instead of the internet (the build's screenshot checks).
    pub fn use_samples(&mut self, dir: &Path) {
        self.sample = Some(dir.to_path_buf());
        self.indexes.clear();
    }

    fn sample_text(&self, name: &str) -> Option<Result<String, String>> {
        let dir = self.sample.as_ref()?;
        Some(fs::read_to_string(dir.join(name)).map_err(|e| format!("The sample {name} couldn't be read ({e}).")))
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
                    quote(&p.icon),
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
        Ok(format!(
            "{{\"source\":\"thunderstore\",\"id\":{},\"name\":{},\"owner\":{},\"summary\":{},\"icon\":{},\"code\":{},\"version\":{},\"size\":{},\"updated\":{},\"downloads\":{},\"rating\":{},\"categories\":[{}],\"deprecated\":{},\"install\":[{}],\"already\":[{}],\"missing\":[{}],\"skippedLoaders\":[{}],\"premium\":true,\"message\":\"\"}}",
            quote(&p.full_name),
            quote(&p.display_name()),
            quote(&p.owner),
            quote(&p.description),
            quote(&p.icon),
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
        ))
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

    // ------------------------------------------------------------------------------------------------ Nexus

    pub fn has_key(&self) -> bool {
        self.key.is_some()
    }

    fn nexus(&self) -> Option<Nexus> {
        let key = self.key.as_ref()?;
        Some(Nexus::new(self.client.clone(), key, &self.version))
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
        self.key = Some(key);
        self.account = Some(account);
        Ok(self.account_json("ok", ""))
    }

    pub fn sign_out(&mut self) -> String {
        let _ = fs::remove_file(&self.key_file);
        self.key = None;
        self.account = None;
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
    pub fn sample_sign_in(&mut self) {
        if let Some(Ok(t)) = self.sample_text("nexus_validate.json") {
            self.account = nexus::parse_validate(&t).ok();
            self.key = Some("sample-key-for-screenshots".into());
        }
    }

    pub fn browse_nexus(&mut self, game: &GameInfo, text: &str, sort: &str) -> Result<String, String> {
        let domain = game.domain.clone().ok_or("This game isn't on Nexus Mods.")?;
        let s = match sort {
            "downloads" => nexus::SearchSort::Downloads,
            "updated" => nexus::SearchSort::Updated,
            "name" => nexus::SearchSort::Name,
            _ => nexus::SearchSort::Endorsements,
        };
        let page = if let Some(t) = self.sample_text("nexus_graphql_search.json") {
            nexus::parse_search(&t?)?
        } else {
            self.nexus().ok_or("Add your Nexus Mods key in Settings, Accounts to browse Nexus Mods.")?.search(&domain, text, s, 0, 60, false).map_err(err)?
        };
        let items: Vec<String> = page
            .mods
            .iter()
            .map(|m| {
                format!(
                    "{{\"id\":{},\"name\":{},\"owner\":{},\"summary\":{},\"downloads\":{},\"rating\":{},\"icon\":{},\"code\":false,\"version\":{},\"size\":0,\"updated\":{},\"installed\":{}}}",
                    quote(&m.mod_id.to_string()),
                    quote(&m.name),
                    quote(&m.author),
                    quote(&m.summary),
                    m.downloads,
                    m.endorsements,
                    quote(&m.thumbnail_url),
                    quote(&m.version),
                    quote(&day(&m.updated_at)),
                    game.have.iter().any(|h| h.source.kind == "nexus"
                        && nexus::parse_source_id(&h.source.id).is_some_and(|(_, id, _)| id == m.mod_id))
                )
            })
            .collect();
        Ok(format!(
            "{{\"source\":\"nexus\",\"total\":{},\"note\":\"\",\"categories\":[],\"items\":[{}]}}",
            page.total,
            items.join(",")
        ))
    }

    fn nexus_files(&self, domain: &str, mod_id: u64) -> Result<nexus::FileList, String> {
        if let Some(t) = self.sample_text("nexus_files.json") {
            return nexus::parse_files(&t?);
        }
        self.nexus().ok_or("Add your Nexus Mods key in Settings, Accounts first.")?.files(domain, mod_id).map_err(err)
    }

    pub fn details_nexus(&mut self, game: &GameInfo, mod_id: u64) -> Result<String, String> {
        let domain = game.domain.clone().ok_or("This game isn't on Nexus Mods.")?;
        let info = if let Some(t) = self.sample_text("nexus_mod.json") {
            nexus::parse_mod(&t?)?
        } else {
            self.nexus().ok_or("Add your Nexus Mods key in Settings, Accounts first.")?.mod_info(&domain, mod_id).map_err(err)?
        };
        let files = self.nexus_files(&domain, mod_id)?;
        let main = files.main_file().cloned();
        let premium = self.account().map(|a| a.is_premium).unwrap_or(false);
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
        Ok(format!(
            "{{\"source\":\"nexus\",\"id\":{},\"name\":{},\"owner\":{},\"summary\":{},\"icon\":{},\"code\":false,\"version\":{},\"size\":{},\"updated\":\"\",\"downloads\":{},\"rating\":{},\"categories\":[],\"deprecated\":{},\"install\":[{}],\"already\":[],\"missing\":[],\"skippedLoaders\":[],\"premium\":{},\"message\":{}}}",
            quote(&mod_id.to_string()),
            quote(&info.name),
            quote(&info.author),
            quote(&info.summary),
            quote(&info.picture_url),
            quote(&info.version),
            main.as_ref().map(|f| f.size_bytes).unwrap_or(0),
            info.downloads,
            info.endorsements,
            !info.available,
            pick,
            premium,
            quote(if premium { "" } else { nexus::FREE_ACCOUNT_MESSAGE })
        ))
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
        let file = if file_id == 0 { files.main_file() } else { files.files.iter().find(|f| f.file_id == file_id) }
            .cloned()
            .ok_or("Nexus lists no file to download for that mod.")?;
        let nx = self.nexus().ok_or("Add your Nexus Mods key in Settings, Accounts first.")?;
        let label = format!("{} {}", file.name, file.version).trim().to_string();
        CANCEL.store(false, Ordering::SeqCst);
                let mut cb = |got: u64, total: Option<u64>| {
            progress(&label, got, total);
            !CANCEL.load(Ordering::SeqCst)
        };
        let (d, source) = nx.download(&account, &domain, mod_id, &file, Some(&mut cb)).map_err(err)?;
        let path = rename(&d, &label, &ext_of(&file.file_name));
        each(Fetched { path, source, label, replaces });
        Ok(1)
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
        if let (Some(domain), Some(nx)) = (game.domain.clone(), self.nexus()) {
            if installed.iter().any(|m| m.kind == "nexus") && self.sample.is_none() {
                match nx.updated(&domain, nexus::update_period(20 * 86_400).unwrap_or("1m")) {
                    Ok(list) => {
                        for (mod_id, m) in updates::nexus_candidates(&domain, &installed, &list) {
                            match nx.files(&domain, mod_id) {
                                Ok(files) => found.extend(updates::nexus_update_from_files(m, &files)),
                                Err(e) => notes.push(err(e)),
                            }
                        }
                    }
                    Err(e) => notes.push(err(e)),
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
                            .find(|h| h.source.kind == "nexus" && nexus::parse_source_id(&h.source.id).is_some_and(|(hd, hm, _)| hd == d && hm == m))
                            .map(|h| h.mod_id.clone())?
                    }
                    _ => game.mod_for(&u.kind, &u.id)?,
                };
                Some(format!(
                    "{{\"mod\":{},\"kind\":{},\"id\":{},\"from\":{},\"to\":{},\"target\":{},\"deprecated\":{}}}",
                    quote(&mod_id),
                    quote(&u.kind),
                    quote(&u.id),
                    quote(u.installed_version.as_deref().unwrap_or("")),
                    quote(&u.new_version),
                    quote(&u.target),
                    u.deprecated
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
                let (_, nexus_mod, _) = nexus::parse_source_id(&have.source.id).ok_or("Turbine doesn't know where that mod came from.")?;
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

/// Reads a JSON string list helper for the interface's needs.
pub fn strings(text: &str, key: &str) -> Vec<String> {
    json::parse(text)
        .ok()
        .map(|j| j.get(key).map(Json::as_array).unwrap_or(&[]).iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let nb = n.browse_nexus(&game(), "", "").unwrap();
        assert!(json::parse(&nb).is_ok());
        let nd = n.details_nexus(&game(), 2400).unwrap();
        assert!(json::parse(&nd).is_ok(), "{nd}");
        assert!(n.install_ts(&game(), &[first], false, &mut |_| {}, &mut |_, _, _| {}).is_err());
        let r = n.self_check("0.2", false, false).unwrap();
        println!("{:?}", r.map(|r| r.version));
        let u = n.check_updates(&game()).unwrap();
        assert!(json::parse(&u).is_ok());
        n.sample_sign_in();
        assert!(n.account_json("ok", "").contains("\"hasKey\":true"));
    }
}
