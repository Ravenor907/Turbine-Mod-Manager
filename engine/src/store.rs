//! A game's mods: staged files, on/off and order, Recently removed (7 days, with undo), setups that worked, and
//! applying it all to the game folder through `deploy`.
//!
//! CONTRACT (the app relies on these signatures; keep them stable).
//!
//! Layout of a store (one per game):
//! - `mods/<id>/...`      each mod's files, laid out relative to the game folder
//! - `removed/<id>/...`   mods in Recently removed (kept 7 days)
//! - `state.json`         order, on/off, names, Recently removed, setups, last build (written atomically)
//! - `deploy/`            the deploy record, plan and backups of game files mods replace
//! - `tmp/`               archives being unpacked (cleared on open)
//!
//! The store sits on the same drive as the game so deploy can hard link instead of copying.

use crate::deploy::{self, Paths, Record, Report};
use crate::games::Game;
use crate::json::{self, quote, Json};
use crate::rules::GameRules;
use crate::zip::{self, Limits, Zip};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq)]
pub struct Mod {
    /// Stable ID, unique within the game.
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub version: Option<String>,
    /// Where it came from, in words: "Zip file", later "Thunderstore", "Nexus Mods".
    pub source: String,
    pub files: usize,
    pub needs_loader: bool,
    /// Seconds since 1970.
    pub added: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Removed {
    pub item: Mod,
    pub removed_at: u64,
}

/// A setup (which mods on, in which order) saved after a deploy, to go back to.
#[derive(Debug, Clone, PartialEq)]
pub struct Setup {
    pub id: String,
    pub saved_at: u64,
    pub mods_on: usize,
}

/// The parts of a game's rules the store needs. `GameRules` is the real one; tests may pass their own.
pub trait ModRules: Send {
    fn map_archive(&self, files: &[String], mod_name: &str) -> Result<Vec<(String, String)>, String>;
    fn needs_loader(&self, game_files: &[String]) -> bool;
}

impl ModRules for GameRules {
    fn map_archive(&self, files: &[String], mod_name: &str) -> Result<Vec<(String, String)>, String> {
        GameRules::map_archive(self, files, mod_name)
    }
    fn needs_loader(&self, game_files: &[String]) -> bool {
        GameRules::needs_loader(self, game_files)
    }
}

/// How long Recently removed keeps a mod.
pub const KEEP_REMOVED_SECS: u64 = 7 * 24 * 60 * 60;
/// How many setups are kept.
pub const KEEP_SETUPS: usize = 10;
/// Name of the hidden folder on another drive's library.
pub const DRIVE_FOLDER: &str = ".turbine-mod-manager";

struct SavedSetup {
    setup: Setup,
    /// Mod IDs in order with their on/off state.
    order: Vec<(String, bool)>,
}

struct RemovedEntry {
    removed: Removed,
    /// Where it was in the order, for Undo.
    position: usize,
    archive: String,
}

pub struct Store {
    root: PathBuf,
    game: Game,
    rules: Box<dyn ModRules>,
    mods: Vec<Mod>,
    /// Archive file name of each mod (same index as `mods`), to notice the same download added again.
    archives: Vec<String>,
    removed_list: Vec<Removed>,
    removed_extra: Vec<RemovedEntry>,
    setup_list: Vec<Setup>,
    saved: Vec<SavedSetup>,
    last_build: Option<String>,
    clock: Option<u64>,
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn not_found(id: &str) -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, format!("There is no mod called {id} here."))
}

/// A name that is safe as a single folder name on Linux and Windows-style file systems (exFAT SD cards).
pub fn folder_safe(name: &str) -> String {
    let out: String = name
        .chars()
        .map(|c| if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') { '_' } else { c })
        .collect();
    let out = out.trim().trim_matches('.').trim().to_string();
    if out.is_empty() {
        "Mod".into()
    } else {
        out
    }
}

/// Lowercase letters, digits and dashes, for IDs and folder names Turbine makes.
fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() {
        "mod".into()
    } else {
        out
    }
}

fn all_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

fn looks_like_version(s: &str) -> bool {
    let v = s.strip_prefix(['v', 'V']).unwrap_or(s);
    let parts: Vec<&str> = v.split('.').collect();
    (parts.len() >= 2 && parts.iter().all(|p| all_digits(p))) || (s != v && all_digits(v))
}

/// A readable name and version from a download's file name: "SkyUI_5_2_SE-12604-5-2SE-1573517013.zip" becomes
/// ("SkyUI 5 2 SE", Some("5.2SE")), "Author-Cool_Mod-1.2.3.zip" becomes ("Author-Cool Mod", Some("1.2.3")).
pub fn clean_name(file_name: &str) -> (String, Option<String>) {
    let stem = match file_name.rsplit_once('.') {
        Some((s, ext)) if ext.eq_ignore_ascii_case("zip") || ext.eq_ignore_ascii_case("7z") || ext.eq_ignore_ascii_case("rar") => s,
        _ => file_name,
    };
    let parts: Vec<&str> = stem.split('-').collect();
    let mut keep = parts.len();
    let mut version = None;
    // Nexus Mods style: name-<mod id>-<version parts>-<upload time>.
    let last = parts.len().saturating_sub(1);
    if parts.len() >= 3 && all_digits(parts[last]) && parts[last].len() >= 9 {
        if let Some(i) = (1..last).find(|&i| all_digits(parts[i])) {
            keep = i;
            let v = parts[i + 1..last].join(".");
            if !v.is_empty() {
                version = Some(v);
            }
        }
    } else if parts.len() >= 2 && looks_like_version(parts[last]) {
        keep = last;
        version = Some(parts[last].trim_start_matches(['v', 'V']).to_string());
    }
    let base = parts[..keep].join("-").replace('_', " ");
    let name = base.split_whitespace().collect::<Vec<_>>().join(" ");
    let name = if name.is_empty() { stem.replace('_', " ").trim().to_string() } else { name };
    (if name.is_empty() { "Mod".into() } else { name }, version)
}

/// Where a game's store goes. `library_dev` and `home_dev` are the device numbers of the game's library and of the
/// data folder (None when unknown). A library on another drive gets a hidden folder at its top, if it can be written.
pub fn choose_root(game_key: &str, library: &Path, library_dev: Option<u64>, home_dev: Option<u64>, library_writable: bool, data_home: &Path) -> PathBuf {
    let removable = ["/run/media", "/media", "/mnt"].iter().any(|p| library.starts_with(p));
    let other_drive = match (library_dev, home_dev) {
        (Some(a), Some(b)) => a != b,
        _ => removable,
    };
    if other_drive && library_writable {
        library.join(DRIVE_FOLDER).join(game_key)
    } else {
        data_home.join("turbine-mod-manager").join(game_key)
    }
}

/// Device number of a path, or of its nearest existing parent folder.
fn device_of(p: &Path) -> Option<u64> {
    let mut cur = Some(p);
    while let Some(c) = cur {
        if let Ok(m) = fs::metadata(c) {
            return Some(m.dev());
        }
        cur = c.parent();
    }
    None
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

/// Every file under `base`, as ('/' separated relative path, full path), sorted.
fn walk_files(base: &Path) -> Vec<(String, PathBuf)> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) {
        let Ok(rd) = fs::read_dir(dir) else { return };
        let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            let Ok(m) = fs::symlink_metadata(&p) else { continue };
            if m.is_dir() {
                walk(base, &p, out);
            } else if m.is_file() {
                if let Ok(rel) = p.strip_prefix(base) {
                    let rel: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
                    out.push((rel.join("/"), p));
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(base, base, &mut out);
    out
}

fn remove_dir(p: &Path) -> io::Result<()> {
    match fs::remove_dir_all(p) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

fn num(v: Option<&Json>) -> u64 {
    match v {
        Some(Json::Num(n)) if *n >= 0.0 => *n as u64,
        Some(Json::Str(s)) => s.parse().unwrap_or(0),
        _ => 0,
    }
}

fn mod_from(v: &Json) -> Option<Mod> {
    Some(Mod {
        id: v.str_at("id")?.to_string(),
        name: v.str_at("name").unwrap_or("Mod").to_string(),
        enabled: v.get("enabled").and_then(Json::as_bool).unwrap_or(false),
        version: v.str_at("version").map(String::from),
        source: v.str_at("source").unwrap_or("Zip file").to_string(),
        files: num(v.get("files")) as usize,
        needs_loader: v.get("needsLoader").and_then(Json::as_bool).unwrap_or(false),
        added: num(v.get("added")),
    })
}

fn opt(s: &Option<String>) -> String {
    s.as_deref().map(quote).unwrap_or_else(|| "null".into())
}

fn mod_json(m: &Mod) -> String {
    format!(
        "\"id\":{},\"name\":{},\"enabled\":{},\"version\":{},\"source\":{},\"files\":{},\"needsLoader\":{},\"added\":{}",
        quote(&m.id),
        quote(&m.name),
        m.enabled,
        opt(&m.version),
        quote(&m.source),
        m.files,
        m.needs_loader,
        m.added
    )
}

impl Store {
    /// Opens (or creates) the store for a game. Mods are staged on the same drive as the game so they can be hard
    /// linked: under `data_home` for internal storage, in a hidden folder on the SD card or other drive otherwise.
    pub fn open(game: &Game, data_home: &Path) -> io::Result<Store> {
        Store::open_with(game, data_home, Box::new(GameRules::for_game(game)))
    }

    /// Like `open`, with the rules given (tests use rules of their own).
    pub fn open_with(game: &Game, data_home: &Path, rules: Box<dyn ModRules>) -> io::Result<Store> {
        Store::open_at(game, &Store::root_for(game, data_home), rules)
    }

    /// Where `open` puts this game's store.
    pub fn root_for(game: &Game, data_home: &Path) -> PathBuf {
        let key = folder_safe(&game.id().replace(':', "-"));
        let lib_dev = fs::metadata(&game.library).ok().map(|m| m.dev());
        let home_dev = device_of(data_home);
        let first = choose_root(&key, &game.library, lib_dev, home_dev, true, data_home);
        if first.starts_with(&game.library) && fs::create_dir_all(&first).is_err() {
            return choose_root(&key, &game.library, lib_dev, home_dev, false, data_home);
        }
        first
    }

    /// Opens the store kept in `root`.
    pub fn open_at(game: &Game, root: &Path, rules: Box<dyn ModRules>) -> io::Result<Store> {
        for d in ["mods", "removed", "deploy"] {
            fs::create_dir_all(root.join(d))?;
        }
        remove_dir(&root.join("tmp"))?;
        let mut s = Store {
            root: root.to_path_buf(),
            game: game.clone(),
            rules,
            mods: Vec::new(),
            archives: Vec::new(),
            removed_list: Vec::new(),
            removed_extra: Vec::new(),
            setup_list: Vec::new(),
            saved: Vec::new(),
            last_build: None,
            clock: None,
        };
        let mut dirty = s.load()?;
        dirty |= s.tidy()?;
        if dirty {
            s.save()?;
        }
        Ok(s)
    }

    /// The folder this store lives in.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Fixes the time the store uses ("now"), for tests. Not used by the app.
    #[doc(hidden)]
    pub fn set_clock(&mut self, now: u64) {
        self.clock = Some(now);
    }

    fn now(&self) -> u64 {
        self.clock.unwrap_or_else(now_secs)
    }

    fn state_path(&self) -> PathBuf {
        self.root.join("state.json")
    }

    fn mod_dir(&self, id: &str) -> PathBuf {
        self.root.join("mods").join(id)
    }

    fn removed_dir(&self, id: &str) -> PathBuf {
        self.root.join("removed").join(id)
    }

    /// The deploy bookkeeping for this game.
    pub fn deploy_paths(&self) -> Paths {
        let d = self.root.join("deploy");
        Paths { game: self.game.install_dir.clone(), backups: d.join("backups"), record: d.join("record.json"), plan: d.join("plan.json") }
    }

    /// Reads state.json. Returns true when it needed rewriting (missing or unreadable).
    fn load(&mut self) -> io::Result<bool> {
        let path = self.state_path();
        let text = match fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(e),
        };
        let v = match json::parse(&text) {
            Ok(v) => v,
            Err(_) => {
                // Keep the damaged file for a look later; the mods themselves are picked up again from their folders.
                fs::rename(&path, self.root.join("state.damaged.json"))?;
                return Ok(true);
            }
        };
        for m in v.get("mods").map(Json::as_array).unwrap_or(&[]) {
            if let Some(item) = mod_from(m) {
                if self.mods.iter().any(|x| x.id == item.id) {
                    continue;
                }
                self.archives.push(m.str_at("archive").unwrap_or("").to_string());
                self.mods.push(item);
            }
        }
        for r in v.get("removed").map(Json::as_array).unwrap_or(&[]) {
            if let Some(item) = mod_from(r) {
                let removed = Removed { item, removed_at: num(r.get("removedAt")) };
                self.removed_extra.push(RemovedEntry {
                    removed: removed.clone(),
                    position: num(r.get("position")) as usize,
                    archive: r.str_at("archive").unwrap_or("").to_string(),
                });
            }
        }
        for s in v.get("setups").map(Json::as_array).unwrap_or(&[]) {
            let Some(id) = s.str_at("id") else { continue };
            let order = s
                .get("order")
                .map(Json::as_array)
                .unwrap_or(&[])
                .iter()
                .filter_map(|o| Some((o.str_at("id")?.to_string(), o.get("enabled").and_then(Json::as_bool).unwrap_or(false))))
                .collect();
            self.saved.push(SavedSetup {
                setup: Setup { id: id.to_string(), saved_at: num(s.get("savedAt")), mods_on: num(s.get("modsOn")) as usize },
                order,
            });
        }
        self.last_build = v.str_at("lastBuild").map(String::from);
        self.sync_lists();
        Ok(false)
    }

    /// Purges old Recently removed items and picks up mod folders the state doesn't know (a stop right after
    /// unpacking). Returns true when something changed.
    fn tidy(&mut self) -> io::Result<bool> {
        let mut changed = false;
        let now = self.now();
        let mut keep = Vec::new();
        for r in std::mem::take(&mut self.removed_extra) {
            if now.saturating_sub(r.removed.removed_at) >= KEEP_REMOVED_SECS {
                remove_dir(&self.removed_dir(&r.removed.item.id))?;
                changed = true;
            } else {
                keep.push(r);
            }
        }
        self.removed_extra = keep;
        // Leftovers: folders in removed/ nothing points to, and half-made folders.
        if let Ok(rd) = fs::read_dir(self.root.join("removed")) {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if !self.removed_extra.iter().any(|r| r.removed.item.id == name) {
                    remove_dir(&e.path())?;
                }
            }
        }
        if let Ok(rd) = fs::read_dir(self.root.join("mods")) {
            let mut names: Vec<String> = rd.flatten().filter(|e| e.path().is_dir()).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
            names.sort();
            for name in names {
                if name.starts_with('.') {
                    remove_dir(&self.mod_dir(&name))?;
                    continue;
                }
                if self.mods.iter().any(|m| m.id == name) {
                    continue;
                }
                let files = walk_files(&self.mod_dir(&name)).len();
                self.mods.push(Mod {
                    id: name.clone(),
                    name: name.replace('-', " "),
                    enabled: false,
                    version: None,
                    source: "Zip file".into(),
                    files,
                    needs_loader: false,
                    added: now,
                });
                self.archives.push(String::new());
                changed = true;
            }
        }
        self.sync_lists();
        Ok(changed)
    }

    fn sync_lists(&mut self) {
        self.removed_list = self.removed_extra.iter().map(|r| r.removed.clone()).collect();
        self.setup_list = self.saved.iter().map(|s| s.setup.clone()).collect();
    }

    fn save(&mut self) -> io::Result<()> {
        self.sync_lists();
        let mods: Vec<String> =
            self.mods.iter().zip(&self.archives).map(|(m, a)| format!("{{{},\"archive\":{}}}", mod_json(m), quote(a))).collect();
        let removed: Vec<String> = self
            .removed_extra
            .iter()
            .map(|r| {
                format!(
                    "{{{},\"archive\":{},\"removedAt\":{},\"position\":{}}}",
                    mod_json(&r.removed.item),
                    quote(&r.archive),
                    r.removed.removed_at,
                    r.position
                )
            })
            .collect();
        let setups: Vec<String> = self
            .saved
            .iter()
            .map(|s| {
                let order: Vec<String> = s.order.iter().map(|(id, on)| format!("{{\"id\":{},\"enabled\":{}}}", quote(id), on)).collect();
                format!(
                    "{{\"id\":{},\"savedAt\":{},\"modsOn\":{},\"order\":[{}]}}",
                    quote(&s.setup.id),
                    s.setup.saved_at,
                    s.setup.mods_on,
                    order.join(",")
                )
            })
            .collect();
        let text = format!(
            "{{\"version\":1,\n\"mods\":[\n{}\n],\n\"removed\":[\n{}\n],\n\"setups\":[\n{}\n],\n\"lastBuild\":{}}}\n",
            mods.join(",\n"),
            removed.join(",\n"),
            setups.join(",\n"),
            opt(&self.last_build)
        );
        write_atomic(&self.state_path(), &text)
    }

    pub fn mods(&self) -> &[Mod] {
        &self.mods
    }

    pub fn removed(&self) -> &[Removed] {
        &self.removed_list
    }

    /// Saved setups, newest first.
    pub fn setups(&self) -> &[Setup] {
        &self.setup_list
    }

    fn index_of(&self, id: &str) -> io::Result<usize> {
        self.mods.iter().position(|m| m.id == id).ok_or_else(|| not_found(id))
    }

    fn new_id(&self, name: &str) -> String {
        let base = slug(name);
        let taken = |id: &str| {
            self.mods.iter().any(|m| m.id == id)
                || self.removed_extra.iter().any(|r| r.removed.item.id == id)
                || self.mod_dir(id).exists()
                || self.removed_dir(id).exists()
        };
        if !taken(&base) {
            return base;
        }
        (2..).map(|n| format!("{base}-{n}")).find(|id| !taken(id)).unwrap_or(base)
    }

    /// Adds a mod from a zip file. Returns the new mod (enabled, at the end of the order).
    /// The same download added again replaces the old one's files and keeps its place and on/off state.
    pub fn import_zip(&mut self, archive: &Path) -> Result<Mod, String> {
        let file_name = archive.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let shown = if file_name.is_empty() { archive.to_string_lossy().into_owned() } else { file_name.clone() };
        let fail = |what: String| format!("{shown} couldn't be added: {what}.");
        let io_fail = |e: io::Error| fail(format!("couldn't read or write a file ({e})"));

        let (name, version) = clean_name(&file_name);
        let existing = if file_name.is_empty() { None } else { self.archives.iter().position(|a| *a == file_name) };
        let id = match existing {
            Some(i) => self.mods[i].id.clone(),
            None => self.new_id(&name),
        };

        let tmp = self.root.join("tmp");
        fs::create_dir_all(&tmp).map_err(io_fail)?;
        let unpacked = tmp.join(format!("{id}.unpacked"));
        let staged = tmp.join(format!("{id}.staged"));
        let _ = remove_dir(&unpacked);
        let _ = remove_dir(&staged);
        let result = (|| -> Result<(Vec<String>, usize), String> {
            let mut z = Zip::open(archive).map_err(|e| fail(e.to_string()))?;
            let written = z.extract_all(&unpacked, &Limits::default()).map_err(|e| fail(e.to_string()))?;
            let files: Vec<String> = written
                .iter()
                .map(|p| p.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/"))
                .collect();
            if files.is_empty() {
                return Err(fail("the zip file is empty".into()));
            }
            let map = self.rules.map_archive(&files, &folder_safe(&name)).map_err(|e| {
                let e = e.trim_end_matches('.').to_string();
                fail(e)
            })?;
            if map.is_empty() {
                return Err(fail("none of its files is something Turbine knows where to put".into()));
            }
            fs::create_dir_all(&staged).map_err(io_fail)?;
            let mut game_files = Vec::new();
            for (from, to) in &map {
                let from_rel = zip::safe_relative(from).map_err(|e| fail(e.to_string()))?;
                let to_rel = zip::safe_relative(to).map_err(|e| fail(e.to_string()))?;
                if to_rel.as_os_str().is_empty() {
                    continue;
                }
                let src = unpacked.join(&from_rel);
                if !src.is_file() {
                    continue;
                }
                let dest = staged.join(&to_rel);
                if let Some(p) = dest.parent() {
                    fs::create_dir_all(p).map_err(io_fail)?;
                }
                fs::rename(&src, &dest).map_err(io_fail)?;
                game_files.push(to_rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/"));
            }
            let count = walk_files(&staged).len();
            if count == 0 {
                return Err(fail("none of its files is something Turbine knows where to put".into()));
            }
            Ok((game_files, count))
        })();
        let _ = remove_dir(&unpacked);
        let (game_files, count) = match result {
            Ok(r) => r,
            Err(e) => {
                let _ = remove_dir(&staged);
                return Err(e);
            }
        };
        let needs_loader = self.rules.needs_loader(&game_files);

        // Swap the new files in.
        let dir = self.mod_dir(&id);
        let swap = (|| -> io::Result<()> {
            if dir.exists() {
                let old = tmp.join(format!("{id}.old"));
                remove_dir(&old)?;
                fs::rename(&dir, &old)?;
                fs::rename(&staged, &dir)?;
                remove_dir(&old)
            } else {
                fs::rename(&staged, &dir)
            }
        })();
        if let Err(e) = swap {
            let _ = remove_dir(&staged);
            return Err(io_fail(e));
        }

        let now = self.now();
        let item = match existing {
            Some(i) => {
                let m = &mut self.mods[i];
                m.name = name;
                m.version = version;
                m.files = count;
                m.needs_loader = needs_loader;
                m.added = now;
                m.clone()
            }
            None => {
                let m = Mod { id, name, enabled: true, version, source: "Zip file".into(), files: count, needs_loader, added: now };
                self.mods.push(m.clone());
                self.archives.push(file_name);
                m
            }
        };
        self.save().map_err(io_fail)?;
        Ok(item)
    }

    pub fn set_enabled(&mut self, id: &str, on: bool) -> io::Result<()> {
        let i = self.index_of(id)?;
        if self.mods[i].enabled != on {
            self.mods[i].enabled = on;
            self.save()?;
        }
        Ok(())
    }

    /// Moves a mod to a position in the order (0 = first, lowest priority). Past the end means last.
    pub fn move_to(&mut self, id: &str, index: usize) -> io::Result<()> {
        let i = self.index_of(id)?;
        let to = index.min(self.mods.len() - 1);
        if i != to {
            let m = self.mods.remove(i);
            let a = self.archives.remove(i);
            self.mods.insert(to, m);
            self.archives.insert(to, a);
            self.save()?;
        }
        Ok(())
    }

    /// Takes a mod out of the game and keeps it in Recently removed for 7 days.
    /// The game folder changes at the next `apply` (the app applies a few seconds after each change).
    pub fn remove(&mut self, id: &str) -> io::Result<()> {
        let i = self.index_of(id)?;
        let dest = self.removed_dir(id);
        remove_dir(&dest)?;
        let src = self.mod_dir(id);
        if src.exists() {
            fs::rename(&src, &dest)?;
        }
        let item = self.mods.remove(i);
        let archive = self.archives.remove(i);
        let removed = Removed { item, removed_at: self.now() };
        self.removed_extra.insert(0, RemovedEntry { removed, position: i, archive });
        self.save()
    }

    /// Puts a mod from Recently removed back where it was in the order.
    pub fn undo_remove(&mut self, id: &str) -> io::Result<()> {
        let r = self.removed_extra.iter().position(|r| r.removed.item.id == id).ok_or_else(|| not_found(id))?;
        let src = self.removed_dir(id);
        let dest = self.mod_dir(id);
        if src.exists() {
            remove_dir(&dest)?;
            fs::rename(&src, &dest)?;
        }
        let entry = self.removed_extra.remove(r);
        let at = entry.position.min(self.mods.len());
        self.mods.insert(at, entry.removed.item);
        self.archives.insert(at, entry.archive);
        self.save()
    }

    /// Every file the enabled mods put in the game, in order (later ones win).
    fn wanted(&self) -> Vec<(String, PathBuf)> {
        self.mods.iter().filter(|m| m.enabled).flat_map(|m| walk_files(&self.mod_dir(&m.id))).collect()
    }

    /// Makes the game folder match the enabled mods in order. Saves a setup when it changed something.
    pub fn apply(&mut self) -> io::Result<Report> {
        self.apply_stopping(None)
    }

    /// `apply` that stops after `stop_after` steps as if the power went out (for tests).
    #[doc(hidden)]
    pub fn apply_stopping(&mut self, stop_after: Option<usize>) -> io::Result<Report> {
        let report = self.deploy(self.wanted(), stop_after)?;
        if report.added + report.removed > 0 {
            self.save_setup();
            self.last_build = self.game.build.clone();
            self.save()?;
        }
        Ok(report)
    }

    /// Runs deploy. Files in the game whose staged copy was replaced since (the same download added again) are
    /// taken out first, so the new copy goes in.
    fn deploy(&self, wanted: Vec<(String, PathBuf)>, stop_after: Option<usize>) -> io::Result<Report> {
        let paths = self.deploy_paths();
        let recovered = deploy::recover(&paths)?.is_some();
        let record = Record::load(&paths.record)?;
        let stale: HashSet<String> = record
            .files
            .iter()
            .filter(|p| match fs::metadata(&p.source) {
                Ok(m) if p.copy => m.len() != p.size || m.mtime() * 1_000_000_000 + m.mtime_nsec() != p.mtime,
                Ok(m) => m.ino() != p.ino,
                Err(_) => false,
            })
            .map(|p| p.rel.to_lowercase())
            .collect();
        let mut first = Report::default();
        if !stale.is_empty() {
            let without: Vec<(String, PathBuf)> = wanted.iter().filter(|(rel, _)| !stale.contains(&rel.to_lowercase())).cloned().collect();
            first = deploy::apply(&paths, &without, stop_after)?;
        }
        let mut report = deploy::apply(&paths, &wanted, stop_after)?;
        report.added += first.added;
        report.removed += first.removed;
        report.recovered |= recovered || first.recovered;
        let mut changed = first.changed_by_others;
        changed.append(&mut report.changed_by_others);
        report.changed_by_others = changed;
        Ok(report)
    }

    fn save_setup(&mut self) {
        let now = self.now();
        let base = now.to_string();
        let id = if self.saved.iter().any(|s| s.setup.id == base) {
            (2..).map(|n| format!("{base}-{n}")).find(|id| !self.saved.iter().any(|s| s.setup.id == *id)).unwrap_or(base)
        } else {
            base
        };
        let order: Vec<(String, bool)> = self.mods.iter().map(|m| (m.id.clone(), m.enabled)).collect();
        let mods_on = order.iter().filter(|(_, on)| *on).count();
        self.saved.insert(0, SavedSetup { setup: Setup { id, saved_at: now, mods_on }, order });
        self.saved.truncate(KEEP_SETUPS);
        self.sync_lists();
    }

    /// Takes every mod file out of the game (nothing is deleted; mods stay on/off as they were).
    pub fn play_original(&mut self) -> io::Result<Report> {
        self.deploy(Vec::new(), None)
    }

    /// Goes back to a saved setup and applies it. Mods that are gone are skipped; mods added since are turned off and
    /// kept after the others.
    pub fn restore(&mut self, setup_id: &str) -> io::Result<Report> {
        let s = self
            .saved
            .iter()
            .find(|s| s.setup.id == setup_id)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "That saved setup isn't there any more."))?;
        let order = s.order.clone();
        let mut mods = Vec::new();
        let mut archives = Vec::new();
        for (id, on) in &order {
            if let Some(i) = self.mods.iter().position(|m| m.id == *id) {
                let mut m = self.mods.remove(i);
                m.enabled = *on;
                mods.push(m);
                archives.push(self.archives.remove(i));
            }
        }
        for m in &mut self.mods {
            m.enabled = false;
        }
        mods.append(&mut self.mods);
        archives.append(&mut self.archives);
        self.mods = mods;
        self.archives = archives;
        self.save()?;
        self.apply()
    }

    /// The player chose to play this game build with its mods (Play anyway after an update warning, or a normal
    /// Play): remember it, so the update warning doesn't repeat for the same build.
    pub fn acknowledge_build(&mut self) -> io::Result<()> {
        if self.last_build != self.game.build {
            self.last_build = self.game.build.clone();
            self.save()?;
        }
        Ok(())
    }

    /// The game build recorded at the last apply (for the update guard).
    pub fn last_build(&self) -> Option<String> {
        self.last_build.clone()
    }

    /// Everything the interface shows, as one JSON object: {"mods":[...],"removed":[...],"setups":[...]}.
    pub fn to_json(&self) -> String {
        let mods: Vec<String> = self.mods.iter().map(|m| format!("{{{}}}", mod_json(m))).collect();
        let removed: Vec<String> = self
            .removed_list
            .iter()
            .map(|r| format!("{{\"id\":{},\"name\":{},\"removedAt\":{}}}", quote(&r.item.id), quote(&r.item.name), r.removed_at))
            .collect();
        let setups: Vec<String> = self
            .setup_list
            .iter()
            .map(|s| format!("{{\"id\":{},\"savedAt\":{},\"modsOn\":{}}}", quote(&s.id), s.saved_at, s.mods_on))
            .collect();
        format!(
            "{{\"mods\":[{}],\"removed\":[{}],\"setups\":[{}],\"lastBuild\":{}}}",
            mods.join(","),
            removed.join(","),
            setups.join(","),
            opt(&self.last_build)
        )
    }
}

/// Zip files in the Downloads folder, newest first.
pub fn downloads(home: &Path) -> Vec<PathBuf> {
    let Ok(rd) = fs::read_dir(home.join("Downloads")) else { return Vec::new() };
    let mut found: Vec<(SystemTime, PathBuf)> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("zip")))
        .filter_map(|p| {
            let m = fs::metadata(&p).ok()?;
            m.is_file().then(|| (m.modified().unwrap_or(UNIX_EPOCH), p))
        })
        .collect();
    found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    found.into_iter().map(|(_, p)| p).collect()
}
