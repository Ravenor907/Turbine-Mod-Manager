//! Profiles: named setups per game (which mods are on, in which order), switching between them, and share files.
//!
//! Profiles live in `profiles.json` in the game's store. Switching saves the current on/off and order into the
//! profile being left, loads the other one into the store and applies it to the game.
//!
//! A share file (JSON, written to ~/Downloads) lists a profile's mods and where they came from: Thunderstore mods by
//! their full name and version (enough to fetch them again), Nexus Mods by mod ID and version, and mods added from a
//! file by name only, with a note that they must be added by hand. Reading one tells which mods can be fetched
//! automatically and which can't.

use crate::deploy::Report;
use crate::json::{self, quote, Json};
use crate::nexus;
use crate::store::{folder_safe, SourceInfo, Store};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub const DEFAULT_PROFILE: &str = "Default";
/// Share file format version.
pub const SHARE_VERSION: u64 = 1;

#[derive(Debug, Clone, PartialEq)]
struct Profile {
    name: String,
    order: Vec<(String, bool)>,
}

/// A game's profiles.
#[derive(Debug, Clone)]
pub struct Profiles {
    path: PathBuf,
    active: String,
    list: Vec<Profile>,
}

fn write_atomic(path: &Path, text: &str) -> io::Result<()> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    let tmp = path.with_extension("json.tmp");
    {
        let mut f = File::create(&tmp)?;
        f.write_all(text.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)
}

fn clean_profile_name(name: &str) -> Result<String, String> {
    let n: String = name.chars().filter(|c| !c.is_control()).collect();
    let n = n.trim().to_string();
    if n.is_empty() {
        return Err("A loadout needs a name.".into());
    }
    if n.chars().count() > 60 {
        return Err("That name is too long (60 letters at most).".into());
    }
    Ok(n)
}

impl Profiles {
    /// Reads a game's profiles. The first time, there is one, "Default", holding the current setup.
    pub fn open(store: &Store) -> io::Result<Profiles> {
        let path = store.root().join("profiles.json");
        let mut p = Profiles { path, active: DEFAULT_PROFILE.into(), list: Vec::new() };
        if let Ok(text) = fs::read_to_string(&p.path) {
            if let Ok(v) = json::parse(&text) {
                for item in v.get("profiles").map(Json::as_array).unwrap_or(&[]) {
                    let Some(name) = item.str_at("name") else { continue };
                    if p.list.iter().any(|x| x.name.eq_ignore_ascii_case(name)) {
                        continue;
                    }
                    let order = item
                        .get("order")
                        .map(Json::as_array)
                        .unwrap_or(&[])
                        .iter()
                        .filter_map(|o| Some((o.str_at("id")?.to_string(), o.get("enabled").and_then(Json::as_bool).unwrap_or(false))))
                        .collect();
                    p.list.push(Profile { name: name.to_string(), order });
                }
                if let Some(a) = v.str_at("active") {
                    p.active = a.to_string();
                }
            }
        }
        if p.list.is_empty() {
            p.list.push(Profile { name: DEFAULT_PROFILE.into(), order: store.order() });
        }
        if !p.list.iter().any(|x| x.name == p.active) {
            p.active = p.list[0].name.clone();
        }
        Ok(p)
    }

    fn save(&self) -> io::Result<()> {
        let items: Vec<String> = self
            .list
            .iter()
            .map(|p| {
                let order: Vec<String> = p.order.iter().map(|(id, on)| format!("{{\"id\":{},\"enabled\":{}}}", quote(id), on)).collect();
                format!("{{\"name\":{},\"order\":[{}]}}", quote(&p.name), order.join(","))
            })
            .collect();
        write_atomic(&self.path, &format!("{{\"version\":1,\"active\":{},\"profiles\":[\n{}\n]}}\n", quote(&self.active), items.join(",\n")))
    }

    pub fn names(&self) -> Vec<String> {
        self.list.iter().map(|p| p.name.clone()).collect()
    }

    pub fn active(&self) -> &str {
        &self.active
    }

    fn find(&self, name: &str) -> Option<usize> {
        self.list.iter().position(|p| p.name.eq_ignore_ascii_case(name))
    }

    /// Keeps the store's current setup in the active profile.
    pub fn save_current(&mut self, store: &Store) -> io::Result<()> {
        let order = store.order();
        if let Some(i) = self.find(&self.active.clone()) {
            self.list[i].order = order;
        }
        self.save()
    }

    /// Makes a new profile with the current setup (it doesn't switch to it).
    pub fn create(&mut self, store: &Store, name: &str) -> Result<(), String> {
        let name = clean_profile_name(name)?;
        if self.find(&name).is_some() {
            return Err(format!("There's already a loadout called {name}."));
        }
        self.list.push(Profile { name, order: store.order() });
        self.save().map_err(|e| format!("The loadout couldn't be saved ({e})."))
    }

    pub fn rename(&mut self, old: &str, new: &str) -> Result<(), String> {
        let new = clean_profile_name(new)?;
        let i = self.find(old).ok_or_else(|| format!("There's no loadout called {old}."))?;
        if self.find(&new).is_some_and(|j| j != i) {
            return Err(format!("There's already a loadout called {new}."));
        }
        if self.active.eq_ignore_ascii_case(old) {
            self.active = new.clone();
        }
        self.list[i].name = new;
        self.save().map_err(|e| format!("The loadout couldn't be saved ({e})."))
    }

    /// Deletes a profile (not the one in use).
    pub fn delete(&mut self, name: &str) -> Result<(), String> {
        let i = self.find(name).ok_or_else(|| format!("There's no loadout called {name}."))?;
        if self.list[i].name == self.active {
            return Err("This loadout is in use. Switch to another one first.".into());
        }
        self.list.remove(i);
        self.save().map_err(|e| format!("The loadouts couldn't be saved ({e})."))
    }

    /// Switches: saves the current setup into the profile in use, loads `name` into the store and applies it.
    pub fn switch(&mut self, store: &mut Store, name: &str) -> Result<Report, String> {
        let i = self.find(name).ok_or_else(|| format!("There's no loadout called {name}."))?;
        self.save_current(store).map_err(|e| format!("The current loadout couldn't be saved ({e})."))?;
        let order = self.list[i].order.clone();
        store.set_order(&order).map_err(|e| format!("The loadout couldn't be loaded ({e})."))?;
        self.active = self.list[i].name.clone();
        self.save().map_err(|e| format!("The loadouts couldn't be saved ({e})."))?;
        store.apply().map_err(|e| format!("The loadout was loaded, but the game couldn't be updated ({e})."))
    }

    /// The order saved in a profile (for the active one, the store's current order).
    pub fn order_of(&self, store: &Store, name: &str) -> Option<Vec<(String, bool)>> {
        let i = self.find(name)?;
        Some(if self.list[i].name == self.active { store.order() } else { self.list[i].order.clone() })
    }

    /// {"active":..,"profiles":[{"name","mods","on"}]}
    pub fn to_json(&self) -> String {
        let items: Vec<String> = self
            .list
            .iter()
            .map(|p| {
                format!(
                    "{{\"name\":{},\"mods\":{},\"on\":{}}}",
                    quote(&p.name),
                    p.order.len(),
                    p.order.iter().filter(|(_, on)| *on).count()
                )
            })
            .collect();
        format!("{{\"active\":{},\"profiles\":[{}]}}", quote(&self.active), items.join(","))
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Share files
// ---------------------------------------------------------------------------------------------------------------

/// One mod in a share file.
#[derive(Debug, Clone, PartialEq)]
pub struct ShareMod {
    pub name: String,
    pub enabled: bool,
    /// "thunderstore", "nexus", "loader" or "zip".
    pub kind: String,
    /// Thunderstore: "Owner-Package"; Nexus Mods: the mod ID; zip: empty.
    pub id: String,
    pub version: Option<String>,
    pub url: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Share {
    pub game_name: String,
    /// Store key ("steam", "gog", ...) and the store's ID for the game.
    pub game_store: String,
    pub game_id: String,
    pub profile: String,
    pub mods: Vec<ShareMod>,
}

/// What a share file needs, sorted by how each mod can be got.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ImportPlan {
    /// Turbine can fetch these itself (Thunderstore packages, loaders).
    pub automatic: Vec<ShareMod>,
    /// From Nexus Mods: they need a Nexus Mods account. Premium accounts can fetch them, free accounts get pages to
    /// open (see `nexus_steps`).
    pub nexus: Vec<ShareMod>,
    /// Added from files: the player has to find and add them.
    pub by_hand: Vec<ShareMod>,
    /// Already installed here.
    pub installed: Vec<ShareMod>,
}

fn zip_note() -> String {
    "This mod was added from a file, so it can't be fetched automatically. Find it by its name and add it by hand.".into()
}

fn share_mod_json(m: &ShareMod) -> String {
    let opt = |s: &Option<String>| s.as_deref().map(quote).unwrap_or_else(|| "null".into());
    let full = (m.kind == "thunderstore" && !m.id.is_empty())
        .then(|| m.version.as_ref().map(|v| format!("{}-{v}", m.id)).unwrap_or_else(|| m.id.clone()));
    format!(
        "{{\"name\":{},\"enabled\":{},\"source\":{},\"id\":{},\"version\":{},\"fullName\":{},\"url\":{},\"note\":{}}}",
        quote(&m.name),
        m.enabled,
        quote(&m.kind),
        quote(&m.id),
        opt(&m.version),
        opt(&full),
        opt(&m.url),
        opt(&m.note)
    )
}

/// The share entries for a setup (mods in order, with on/off).
pub fn share_mods(store: &Store, order: &[(String, bool)]) -> Vec<ShareMod> {
    order
        .iter()
        .filter_map(|(id, on)| {
            let m = store.mods().iter().find(|m| m.id == *id)?;
            let src: SourceInfo = store.source_of(id);
            let kind = if src.kind.is_empty() { "zip".to_string() } else { src.kind.clone() };
            let by_file = kind == "zip" || src.id.is_empty();
            Some(ShareMod {
                name: m.name.clone(),
                enabled: *on,
                kind: if by_file { "zip".into() } else { kind },
                id: if by_file { String::new() } else { src.id.clone() },
                version: if by_file { m.version.clone() } else { src.version.clone().or_else(|| m.version.clone()) },
                url: if by_file { None } else { src.url.clone() },
                note: by_file.then(zip_note),
            })
        })
        .collect()
}

/// The share file as text.
pub fn share_json(share: &Share) -> String {
    let mods: Vec<String> = share.mods.iter().map(share_mod_json).collect();
    format!(
        "{{\"turbineProfile\":{SHARE_VERSION},\"app\":\"Turbine Mod Manager\",\"game\":{{\"name\":{},\"store\":{},\"storeId\":{}}},\"profile\":{},\"mods\":[\n{}\n]}}\n",
        quote(&share.game_name),
        quote(&share.game_store),
        quote(&share.game_id),
        quote(&share.profile),
        mods.join(",\n")
    )
}

/// Writes a profile to a share file in ~/Downloads and returns its path.
pub fn export(profiles: &Profiles, store: &Store, name: &str, home: &Path) -> Result<PathBuf, String> {
    let order = profiles.order_of(store, name).ok_or_else(|| format!("There's no loadout called {name}."))?;
    let game = store.game();
    let share = Share {
        game_name: game.name.clone(),
        game_store: game.store.key().into(),
        game_id: game.store_id.clone(),
        profile: name.to_string(),
        mods: share_mods(store, &order),
    };
    // 0.62: the desktop's Downloads folder (the one the player's file manager and browser show).
    let dir = crate::watch::downloads_folder(home);
    fs::create_dir_all(&dir).map_err(|e| format!("The Downloads folder couldn't be made ({e})."))?;
    let base = folder_safe(&format!("Turbine profile - {} - {}", game.name, name));
    let mut path = dir.join(format!("{base}.json"));
    let mut n = 2;
    while fs::symlink_metadata(&path).is_ok() {
        path = dir.join(format!("{base} ({n}).json"));
        n += 1;
    }
    write_atomic(&path, &share_json(&share)).map_err(|e| format!("The share file couldn't be written ({e})."))?;
    Ok(path)
}

/// Reads a share file's text.
pub fn parse_share(text: &str) -> Result<Share, String> {
    let bad = || "This isn't a Turbine profile share file.".to_string();
    let v = json::parse(text).map_err(|_| bad())?;
    let version = match v.get("turbineProfile") {
        Some(Json::Num(n)) => *n as u64,
        _ => return Err(bad()),
    };
    if version > SHARE_VERSION {
        return Err("This share file is from a newer Turbine. Update Turbine to read it.".into());
    }
    let game = v.get("game").ok_or_else(bad)?;
    let mut mods = Vec::new();
    for m in v.get("mods").map(Json::as_array).unwrap_or(&[]) {
        let Some(name) = m.str_at("name") else { continue };
        let kind = m.str_at("source").unwrap_or("zip").to_string();
        let kind = if matches!(kind.as_str(), "thunderstore" | "nexus" | "loader" | "zip") { kind } else { "zip".into() };
        mods.push(ShareMod {
            name: name.to_string(),
            enabled: m.get("enabled").and_then(Json::as_bool).unwrap_or(true),
            id: m.str_at("id").unwrap_or("").to_string(),
            version: m.str_at("version").map(String::from),
            url: m.str_at("url").map(String::from),
            note: m.str_at("note").map(String::from),
            kind,
        });
    }
    Ok(Share {
        game_name: game.str_at("name").unwrap_or("").to_string(),
        game_store: game.str_at("store").unwrap_or("").to_string(),
        game_id: game.str_at("storeId").unwrap_or("").to_string(),
        profile: v.str_at("profile").unwrap_or("Shared").to_string(),
        mods,
    })
}

/// Reads a share file.
pub fn read_share(path: &Path) -> Result<Share, String> {
    let m = fs::metadata(path).map_err(|e| format!("The share file couldn't be read ({e})."))?;
    if m.len() > 4 << 20 {
        return Err("This file is far too large to be a share file.".into());
    }
    parse_share(&fs::read_to_string(path).map_err(|e| format!("The share file couldn't be read ({e})."))?)
}

/// Is this installed mod the share entry? Same source and ID; for Nexus, the same mod even when another file of it
/// is installed (an update, or a share made with an older version).
fn same_source(m: &ShareMod, src: &SourceInfo, game_domain: Option<&str>) -> bool {
    if m.id.is_empty() || src.kind != m.kind {
        return false;
    }
    if src.id.eq_ignore_ascii_case(&m.id) {
        return true;
    }
    if m.kind == "nexus" {
        if let (Some((d, mid, _)), Some((sd, smid))) = (nexus_ids(m, game_domain), nexus::source_mod(src)) {
            return mid == smid && (sd.is_empty() || sd == d);
        }
    }
    false
}

/// Sorts a share file's mods by how they can be got, for this store's game. Refuses a file for another game.
pub fn plan_import(share: &Share, store: &Store) -> Result<ImportPlan, String> {
    let game = store.game();
    let same_game = share.game_id == game.store_id && share.game_store == game.store.key();
    if !same_game && !share.game_name.eq_ignore_ascii_case(&game.name) {
        return Err(format!("This share file is for {}, not {}.", share.game_name, game.name));
    }
    let installed = store.installed_sources();
    let mut plan = ImportPlan::default();
    let domain = nexus::domain_for_store_id(&game.store_id);
    for m in &share.mods {
        let have = installed.iter().any(|(id, src)| {
            same_source(m, src, domain)
                || (m.kind == "zip" && store.mods().iter().any(|x| x.id == *id && x.name.eq_ignore_ascii_case(&m.name)))
        });
        let mut m = m.clone();
        if have {
            plan.installed.push(m);
        } else if m.id.is_empty() || m.kind == "zip" {
            if m.note.is_none() {
                m.note = Some(zip_note());
            }
            plan.by_hand.push(m);
        } else if m.kind == "nexus" {
            plan.nexus.push(m);
        } else {
            plan.automatic.push(m);
        }
    }
    Ok(plan)
}

/// After the mods are in: the share's order and on/off as this store's mod IDs (mods not found are skipped), to
/// make a profile from.
pub fn share_order(share: &Share, store: &Store) -> Vec<(String, bool)> {
    let installed = store.installed_sources();
    let domain = nexus::domain_for_store_id(&store.game().store_id);
    share
        .mods
        .iter()
        .filter_map(|m| {
            installed
                .iter()
                .find(|(id, src)| {
                    same_source(m, src, domain)
                        || store.mods().iter().any(|x| x.id == *id && x.name.eq_ignore_ascii_case(&m.name))
                })
                .map(|(id, _)| (id.clone(), m.enabled))
        })
        .collect()
}

/// How one Nexus mod of a share file is got.
#[derive(Debug, Clone, PartialEq)]
pub enum NexusAction {
    /// Premium: Turbine downloads it (`Nexus::download_by_id`): that file while it can still be downloaded, else
    /// the newest one replacing it, else the main file.
    Fetch { domain: String, mod_id: u64, file_id: Option<u64> },
    /// Free accounts: open this page and press Mod Manager Download; the file comes back as an nxm link.
    OpenPage { url: String },
    /// Not enough is known to find it (no mod ID): the player has to look for it by name.
    ByHand,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NexusStep {
    pub entry: ShareMod,
    pub action: NexusAction,
}

/// The (domain, mod ID, file ID) a share entry names. Entries hold "<domain>:<mod>:<file>"; older ones may hold
/// "<domain>:<mod>" or just the mod ID, which is then taken to be for `game_domain`.
pub fn nexus_ids(m: &ShareMod, game_domain: Option<&str>) -> Option<(String, u64, Option<u64>)> {
    let domain_ok = |d: &str| !d.is_empty() && d.len() <= 64 && d.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit());
    let num = |s: &str| s.parse::<u64>().ok().filter(|n| *n > 0 && s.bytes().all(|b| b.is_ascii_digit()));
    let parts: Vec<&str> = m.id.trim().split(':').collect();
    let (d, mod_id, file_id) = match parts.as_slice() {
        [d, mid, fid] => (d.to_string(), num(mid)?, Some(num(fid)?)),
        [d, mid] => (d.to_string(), num(mid)?, None),
        [mid] => (game_domain?.to_string(), num(mid)?, None),
        _ => return None,
    };
    domain_ok(&d).then_some((d, mod_id, file_id))
}

/// What to do for each Nexus mod of an import plan. `game_domain` is this game's Nexus domain
/// (`nexus::domain_for_store_id`); `premium` is the signed-in account's.
pub fn nexus_steps(plan: &ImportPlan, game_domain: Option<&str>, premium: bool) -> Vec<NexusStep> {
    plan.nexus
        .iter()
        .map(|m| {
            let action = match nexus_ids(m, game_domain) {
                Some((domain, mod_id, file_id)) if premium && nexus::supported_domain(&domain) => {
                    NexusAction::Fetch { domain, mod_id, file_id }
                }
                Some((domain, mod_id, Some(file_id))) => NexusAction::OpenPage { url: nexus::file_page_url(&domain, mod_id, file_id) },
                Some((domain, mod_id, None)) => NexusAction::OpenPage { url: nexus::files_page_url(&domain, mod_id) },
                None => match m.url.as_deref().and_then(nexus::parse_mod_page_url) {
                    Some((domain, mod_id)) => NexusAction::OpenPage { url: nexus::files_page_url(&domain, mod_id) },
                    None => NexusAction::ByHand,
                },
            };
            NexusStep { entry: m.clone(), action }
        })
        .collect()
}

/// The steps as JSON: [{"mod":{..},"action":"fetch"|"open"|"byHand","domain","modId","fileId","url"}].
pub fn nexus_steps_json(steps: &[NexusStep]) -> String {
    let items: Vec<String> = steps
        .iter()
        .map(|s| {
            let (action, domain, mod_id, file_id, url) = match &s.action {
                NexusAction::Fetch { domain, mod_id, file_id } => ("fetch", domain.as_str(), *mod_id, *file_id, ""),
                NexusAction::OpenPage { url } => ("open", "", 0, None, url.as_str()),
                NexusAction::ByHand => ("byHand", "", 0, None, ""),
            };
            format!(
                "{{\"mod\":{},\"action\":\"{action}\",\"domain\":{},\"modId\":{mod_id},\"fileId\":{},\"url\":{}}}",
                share_mod_json(&s.entry),
                quote(domain),
                file_id.map(|f| f.to_string()).unwrap_or_else(|| "null".into()),
                quote(url)
            )
        })
        .collect();
    format!("[{}]", items.join(","))
}

/// The plan as JSON: {"automatic":[..],"nexus":[..],"byHand":[..],"installed":[..]}.
pub fn plan_json(plan: &ImportPlan) -> String {
    let list = |v: &[ShareMod]| v.iter().map(share_mod_json).collect::<Vec<_>>().join(",");
    format!(
        "{{\"automatic\":[{}],\"nexus\":[{}],\"byHand\":[{}],\"installed\":[{}]}}",
        list(&plan.automatic),
        list(&plan.nexus),
        list(&plan.by_hand),
        list(&plan.installed)
    )
}
