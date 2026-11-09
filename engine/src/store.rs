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
//! - `tmp/`               archives being unpacked and installers waiting for choices (cleared on open)
//!
//! Every archive kind goes through `archives::unpack` (zip, 7z, rar). An archive with a FOMOD installer
//! (fomod/ModuleConfig.xml) returns `Import::NeedsChoices` with a session the app drives step by step; `finish_install`
//! then stages the chosen files. SMAPI's installer zip is set up from the bundle inside it, without running anything.
//! Each mod keeps where it came from (`SourceInfo`) for update checks and share files.
//!
//! The store sits on the game's own library (a hidden folder at its top) so deploy can hard link instead of copying:
//! inside the Flatpak, the app's data folder is a separate mount, and links from it into a game always fail.

use crate::archives::{self, Format};
use crate::basic::{self, Placement};
use crate::deploy::{self, Paths, Record, Report, Wanted};
use crate::fomod;
use crate::games::Game;
use crate::json::{self, quote, Json};
use crate::plugins;
use crate::rules::{self, GameRules};
use crate::zip::{self, Limits};
use std::collections::{HashMap, HashSet};
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
    /// The game was started with this setup (Play went through).
    pub played: bool,
}

/// Where an installed mod came from (kind "thunderstore", "nexus", "zip" or "loader", its ID there, version, the
/// download's address and SHA-256 when known, and the IDs of what it needs).
pub use crate::net::SourceInfo;

/// The parts of a game's rules the store needs. `GameRules` is the real one; tests may pass their own.
/// Only the first two are required; the rest have defaults that do nothing.
pub trait ModRules: Send {
    fn map_archive(&self, files: &[String], mod_name: &str) -> Result<Vec<(String, String)>, String>;
    fn needs_loader(&self, game_files: &[String]) -> bool;
    /// An archive inside this one to install from instead (SMAPI's install.dat), as its path in `files`.
    fn inner_archive(&self, _files: &[String]) -> Option<String> {
        None
    }
    /// Maps the inner archive's files (see `inner_archive`).
    fn map_inner(&self, _files: &[String]) -> Result<Vec<(String, String)>, String> {
        Err("This archive can't be installed.".into())
    }
    /// Files copied from the game folder into the mod as it is added: (game file, path in the mod).
    fn game_copies(&self, _game_files: &[String]) -> Vec<(String, String)> {
        Vec::new()
    }
    /// Files of the mod (game-relative) to mark executable (never run by Turbine).
    fn executables(&self, _game_files: &[String]) -> Vec<String> {
        Vec::new()
    }
    /// The folder FOMOD installers install into, relative to the game folder ("Data" for Bethesda games).
    fn mod_root(&self) -> String {
        String::new()
    }
    /// The game has plugins (FOMOD conditions can look at which are turned on).
    fn has_plugins(&self) -> bool {
        false
    }
    /// Notes for the activity log about an archive (paths inside it), added to the install's notes.
    fn install_notes(&self, _files: &[String]) -> Vec<String> {
        Vec::new()
    }
    /// Where a mod goes in a game Turbine places by looking at the files (Basic support, basic.rs). None: the game
    /// has rules of its own and `map_archive` decides (the default, so rules made for tests keep working).
    fn place(&self, _files: &[String], _mod_name: &str, _game: &Game) -> Option<Result<Placement, String>> {
        None
    }
    /// Turbine has no rules of its own for this game (Basic, or a player's data file): its mods never replace the
    /// game's own files unless the player said so for that mod, and never place a file in the refused table
    /// (basic::refused), however the files were laid out. False for rules made for tests.
    fn guarded(&self) -> bool {
        false
    }
}

impl ModRules for GameRules {
    fn map_archive(&self, files: &[String], mod_name: &str) -> Result<Vec<(String, String)>, String> {
        GameRules::map_archive(self, files, mod_name)
    }
    fn needs_loader(&self, game_files: &[String]) -> bool {
        GameRules::needs_loader(self, game_files)
    }
    fn inner_archive(&self, files: &[String]) -> Option<String> {
        self.smapi_bundle(files)
    }
    fn map_inner(&self, files: &[String]) -> Result<Vec<(String, String)>, String> {
        self.map_smapi_bundle(files)
    }
    fn game_copies(&self, game_files: &[String]) -> Vec<(String, String)> {
        GameRules::game_copies(self, game_files)
    }
    fn executables(&self, game_files: &[String]) -> Vec<String> {
        GameRules::executables(self, game_files)
    }
    fn mod_root(&self) -> String {
        self.mod_root.clone()
    }
    fn has_plugins(&self) -> bool {
        self.plugins
    }
    fn install_notes(&self, files: &[String]) -> Vec<String> {
        GameRules::install_notes(self, files)
    }
    fn place(&self, files: &[String], mod_name: &str, game: &Game) -> Option<Result<Placement, String>> {
        if !self.places_by_detection() {
            return None;
        }
        let index = basic::GameIndex::cached(&game.install_dir);
        Some(basic::place(files, mod_name, game, self, &index))
    }
    fn guarded(&self) -> bool {
        self.support_level != rules::Support::Full
    }
}

/// Game files a placement would replace (Turbine didn't place them), and files never placed with why.
pub type Conflicts = (Vec<String>, Vec<(String, &'static str)>);

/// Where a Basic game's mod was placed, kept per mod (state.json "placements") and used again for its updates.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlaceInfo {
    /// The archive folder whose contents went in ("" for the archive's top).
    pub from: String,
    /// The game-relative folder they went into.
    pub to: String,
    /// The detection rule that placed it, or "you" when the player picked the folder.
    pub rule: String,
    /// The player let this mod replace files the game already has (the originals are kept and put back).
    pub replace: bool,
    /// Files left out because the game already has them (when `replace` is false), as of the last apply.
    pub skipped: Vec<String>,
}

impl PlaceInfo {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"from\":{},\"to\":{},\"rule\":{},\"replace\":{},\"skipped\":[{}]}}",
            quote(&self.from),
            quote(&self.to),
            quote(&self.rule),
            self.replace,
            self.skipped.iter().map(|x| quote(x)).collect::<Vec<_>>().join(",")
        )
    }
}

/// A Basic game's mod waiting for the player: where its files go ("Where do these files go?"), or only whether it
/// may replace files the game already has (the placement was sure). Its unpacked files wait in the store's tmp
/// folder. Finish with `Store::finish_place` or drop with `Store::cancel_place`.
#[derive(Debug)]
pub struct PendingPlace {
    /// The name the mod will get.
    pub name: String,
    /// What detection found (the page opens on it).
    pub placement: Placement,
    /// Only the overwrite question is left: `placement` is Sure (or this mod's remembered placement).
    pub sure: bool,
    /// For a sure placement: game files these would replace (Turbine didn't place them), and files never placed.
    pub conflicts: Vec<String>,
    pub refused: Vec<(String, &'static str)>,
    /// The archive's files (paths inside it).
    pub files: Vec<String>,
    target: Target,
    unpacked: PathBuf,
    notes: Vec<String>,
    /// An installer's choices (when the files came from one), kept for the mod's next update once it is added.
    choices: Option<Vec<String>>,
}

impl PendingPlace {
    /// The archive as folders with file counts, for the "Mod files" pane.
    pub fn archive_tree_json(&self) -> String {
        basic::archive_tree_json(&self.files)
    }

    /// {"name","placement":{...},"sure","conflicts":[...],"refused":[{"path","why"}],"tree":[...]}
    pub fn to_json(&self) -> String {
        format!(
            "{{\"name\":{},\"placement\":{},\"sure\":{},\"conflicts\":[{}],\"refused\":[{}],\"tree\":{}}}",
            quote(&self.name),
            basic::placement_json(&self.placement),
            self.sure,
            self.conflicts.iter().map(|c| quote(c)).collect::<Vec<_>>().join(","),
            self.refused.iter().map(|(p, w)| format!("{{\"path\":{},\"why\":{}}}", quote(p), quote(w))).collect::<Vec<_>>().join(","),
            self.archive_tree_json()
        )
    }
}

/// A mod added (or updated), with notes for the activity log (installer files that were missing, files copied from
/// the game, and so on).
#[derive(Debug, Clone, PartialEq)]
pub struct Installed {
    pub item: Mod,
    pub notes: Vec<String>,
    /// Basic games: where the files went.
    pub placed: Option<PlaceInfo>,
}

/// What adding an archive led to.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Import {
    Added(Installed),
    /// The mod has an installer with choices: drive `Pending::session`, then call `Store::finish_install` (or
    /// `Store::cancel_install`).
    NeedsChoices(Box<Pending>),
    /// A Basic game's mod needs the player: where its files go, or whether it may replace the game's files. Call
    /// `Store::finish_place` (or `Store::cancel_place`).
    NeedsPlace(Box<PendingPlace>),
}

/// An installer waiting for the player's choices. Its unpacked files wait in the store's tmp folder.
#[derive(Debug)]
pub struct Pending {
    pub session: fomod::Session,
    /// What the installer's info.xml says (name, author, version, website, description).
    pub info: fomod::Info,
    /// The name the mod will get.
    pub name: String,
    target: Target,
    unpacked: PathBuf,
}

impl Pending {
    /// The real file for a path inside the archive (an option's image: `fomod::Session::archive_path`), if it's there.
    pub fn image_file(&self, archive_path: &str) -> Option<PathBuf> {
        let rel = zip::safe_relative(archive_path).ok()?;
        let p = self.unpacked.join(rel);
        p.is_file().then_some(p)
    }

    /// The installer as JSON for the interface (see `fomod::Session::to_json`), with "name" added.
    pub fn to_json(&self) -> String {
        let s = self.session.to_json();
        format!("{{\"name\":{},{}", quote(&self.name), &s[1..])
    }
}

/// Where an import goes, worked out before unpacking.
#[derive(Debug, Clone)]
struct Target {
    id: String,
    /// Index in `mods` of the mod being replaced.
    existing: Option<usize>,
    name: String,
    version: Option<String>,
    file_name: String,
    shown: String,
    source: SourceInfo,
    format_label: String,
    /// Set by `replace_mod`: keep the mod's own name.
    keep_name: bool,
}


/// Does a game path go through a link (a folder on the way that is a link to somewhere else)?
fn through_link(game: &Path, rel: &str) -> bool {
    let parts: Vec<&str> = rel.split('/').collect();
    let mut dir = game.to_path_buf();
    for part in &parts[..parts.len().saturating_sub(1)] {
        dir.push(part);
        match fs::symlink_metadata(&dir) {
            Ok(m) if m.file_type().is_symlink() => return true,
            Ok(m) if m.is_dir() => {}
            _ => return false,
        }
    }
    false
}

/// Where an installer's files land, each once (the last mention wins), in order.
fn unique_dests(map: &[(String, String)]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (_, to) in map {
        if let Some(i) = out.iter().position(|d| d.eq_ignore_ascii_case(to)) {
            out.remove(i);
        }
        out.push(to.clone());
    }
    out
}

/// `Store::place_conflicts` for a store kept in `store_root` (the interface asks while the store is busy elsewhere).
pub fn conflicts_in(game: &Game, store_root: &Path, files: &[String], from: &str, to: &str) -> Result<Conflicts, String> {
    if !(to.is_empty() || crate::gamedata::safe_path(to)) {
        return Err(format!("{to} isn't a plain folder inside the game folder"));
    }
    let map = basic::map_placement(files, from, to)?;
    let record = Record::load(&store_root.join("deploy").join("record.json")).unwrap_or_default();
    // A file Turbine put over a game file (another mod, with the player's consent) still counts as the game's:
    // replacing it needs this mod's own consent too.
    let ours: HashSet<String> = record.files.iter().filter(|p| p.backup.is_none()).map(|p| p.rel.to_lowercase()).collect();
    let mut conflicts = Vec::new();
    let mut refused = Vec::new();
    for (_, dest) in map {
        let exists = rules::find_ci(&game.install_dir, &dest).is_some();
        let not_ours = exists && !ours.contains(&dest.to_lowercase());
        if let Some(why) = basic::refused(&dest, not_ours) {
            if !refused.iter().any(|(r, _): &(String, &str)| r.eq_ignore_ascii_case(&dest)) {
                refused.push((dest, why));
            }
        } else if not_ours && !conflicts.iter().any(|c: &String| c.eq_ignore_ascii_case(&dest)) {
            conflicts.push(dest);
        }
    }
    Ok((conflicts, refused))
}

/// Folders Turbine created in the game (from the store's record), lowercased.
pub fn made_dirs(store_root: &Path) -> HashSet<String> {
    Record::load(&store_root.join("deploy").join("record.json")).unwrap_or_default().dirs.iter().map(|d| d.to_lowercase()).collect()
}

/// A rule name kept in state.json as the static name the placement uses (unknown names read as "remembered").
fn leak_rule(rule: &str) -> &'static str {
    const RULES: &[&str] = &[
        "data-file-hint", "mods-folder", "bepinex", "bepinex-plugin", "mirrors-game", "data-folder", "data-contents", "game-folder",
        "unreal-paks", "loose-files", "you",
    ];
    RULES.iter().find(|r| **r == rule).copied().unwrap_or("remembered")
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
    /// Where each mod (installed or in Recently removed) came from, by mod ID.
    sources: HashMap<String, SourceInfo>,
    /// Each mod's installer choices ("step/group/option"), by mod ID, to start from when it's updated.
    choices: HashMap<String, Vec<String>>,
    /// Basic games: where each mod's files went, by mod ID.
    placements: HashMap<String, PlaceInfo>,
    setup_list: Vec<Setup>,
    saved: Vec<SavedSetup>,
    last_build: Option<String>,
    clock: Option<u64>,
    /// The 7z/rar reader; None means the real bsdtar.
    tar: Option<Box<dyn archives::Tar + Send>>,
    /// Each staged folder's files (game-relative), by folder, with the folder's inode and time when read: a mod's
    /// folder is made anew when its files change, so the list is read again only then.
    files_cache: std::sync::Mutex<FilesCache>,
}

/// A staged folder's files by folder, with its inode and modification time when read.
type FilesCache = HashMap<PathBuf, (u64, i64, Vec<String>)>;

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

/// A Nexus download's mod ID and version from its file name ("Fast Travel from Anywhere-324-1-0-5-1699999999.zip"
/// gives (324, "1.0.5")), when the name has Nexus's pattern: name, mod ID, version parts, upload time.
pub fn nexus_ids_from_name(file_name: &str) -> Option<(u64, String)> {
    let stem = file_name.rsplit_once('.').map(|(s, _)| s).unwrap_or(file_name);
    let parts: Vec<&str> = stem.split('-').collect();
    let last = parts.len().checked_sub(1)?;
    if parts.len() < 3 || !all_digits(parts[last]) || parts[last].len() < 9 {
        return None;
    }
    let i = (1..last).find(|&i| all_digits(parts[i]))?;
    let id: u64 = parts[i].parse().ok().filter(|x| *x > 0)?;
    Some((id, parts[i + 1..last].join(".")))
}

/// Where a game's store goes: a hidden folder at the top of the game's library, for every library (internal storage
/// too: device numbers can match while the folders are still separate mounts, and links between mounts fail). Only
/// when the library can't be written does it go in the data folder. The device numbers are no longer used.
pub fn choose_root(game_key: &str, library: &Path, _library_dev: Option<u64>, _home_dev: Option<u64>, library_writable: bool, data_home: &Path) -> PathBuf {
    if library_writable {
        library.join(DRIVE_FOLDER).join(game_key)
    } else {
        data_home.join("turbine-mod-manager").join(game_key)
    }
}

/// Can files be made in `dir` (made if missing)? Tried for real.
fn writable(dir: &Path) -> bool {
    if fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(format!(".turbine-write-test-{}", std::process::id()));
    let ok = File::create(&probe).is_ok();
    let _ = fs::remove_file(&probe);
    ok
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

fn source_from(v: &Json) -> SourceInfo {
    SourceInfo {
        kind: v.str_at("kind").unwrap_or("zip").to_string(),
        id: v.str_at("id").unwrap_or("").to_string(),
        version: v.str_at("version").map(String::from),
        url: v.str_at("url").map(String::from),
        sha256: v.str_at("sha256").map(String::from),
        needs: v.get("needs").map(Json::as_array).unwrap_or(&[]).iter().filter_map(|n| n.as_str().map(String::from)).collect(),
    }
}

/// A source as JSON (also used by share files).
pub fn source_json(s: &SourceInfo) -> String {
    format!(
        "{{\"kind\":{},\"id\":{},\"version\":{},\"url\":{},\"sha256\":{},\"needs\":[{}]}}",
        quote(&s.kind),
        quote(&s.id),
        opt(&s.version),
        opt(&s.url),
        opt(&s.sha256),
        s.needs.iter().map(|n| quote(n)).collect::<Vec<_>>().join(",")
    )
}

/// Plain words for where a mod came from.
fn source_label(kind: &str, format: &str) -> String {
    match kind {
        "thunderstore" => "Thunderstore".into(),
        "nexus" => "Nexus Mods".into(),
        "loader" => "Official download".into(),
        _ => match format {
            "7z" => "7z file".into(),
            "rar" => "Rar file".into(),
            _ => "Zip file".into(),
        },
    }
}

fn join_rel(root: &str, rest: &str) -> String {
    match (root.is_empty(), rest.is_empty()) {
        (true, _) => rest.to_string(),
        (_, true) => root.to_string(),
        _ => format!("{root}/{rest}"),
    }
}

fn rel_of(p: &Path) -> String {
    p.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/")
}

/// Largest installer file read (ModuleConfig.xml, info.xml).
const MAX_INSTALLER_FILE: u64 = 8 << 20;

fn read_small(path: &Path) -> Result<Vec<u8>, String> {
    let m = fs::metadata(path).map_err(|e| e.to_string())?;
    if m.len() > MAX_INSTALLER_FILE {
        return Err("its installer file is far too large".into());
    }
    fs::read(path).map_err(|e| e.to_string())
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
        let on_library = choose_root(&key, &game.library, None, None, true, data_home);
        let in_data = choose_root(&key, &game.library, None, None, false, data_home);
        // A store already in the data folder stays there: its record lists the files it placed in the game.
        if in_data.join("state.json").exists() && !on_library.join("state.json").exists() {
            return in_data;
        }
        // Card out: keep the library's place (opening it then says the drive is missing) rather than splitting the store.
        if !game.library.is_dir() || writable(&on_library) {
            on_library
        } else {
            in_data
        }
    }

    /// Opens the store kept in `root`.
    pub fn open_at(game: &Game, root: &Path, rules: Box<dyn ModRules>) -> io::Result<Store> {
        if root.starts_with(&game.library) && !game.library.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("The drive with {} isn't there. If it's on an SD card, put the card back in.", game.name),
            ));
        }
        for d in ["mods", "removed", "deploy"] {
            fs::create_dir_all(root.join(d))?;
        }
        // A swap of a mod's folder that stopped halfway (the power went out between its two renames): the mod's
        // files are still in tmp as "<id>.old". Put them back before tmp is cleared.
        if let Ok(rd) = fs::read_dir(root.join("tmp")) {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if let Some(id) = name.strip_suffix(".old") {
                    let dir = root.join("mods").join(id);
                    if !id.is_empty() && !id.contains(['/', '\\']) && !dir.exists() {
                        fs::rename(e.path(), &dir)?;
                    }
                }
            }
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
            sources: HashMap::new(),
            choices: HashMap::new(),
            placements: HashMap::new(),
            setup_list: Vec::new(),
            saved: Vec::new(),
            last_build: None,
            clock: None,
            tar: None,
            files_cache: std::sync::Mutex::new(HashMap::new()),
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

    /// Uses another 7z/rar reader instead of bsdtar, for tests. Not used by the app.
    #[doc(hidden)]
    pub fn set_tar(&mut self, tar: Box<dyn archives::Tar + Send>) {
        self.tar = Some(tar);
    }

    fn unpack(&self, archive: &Path, dest: &Path) -> Result<Vec<String>, String> {
        match &self.tar {
            Some(t) => archives::unpack_with(archive, dest, &Limits::default(), t.as_ref()),
            None => archives::unpack(archive, dest, &Limits::default()),
        }
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
                setup: Setup {
                    id: id.to_string(),
                    saved_at: num(s.get("savedAt")),
                    mods_on: num(s.get("modsOn")) as usize,
                    played: s.get("played").and_then(Json::as_bool).unwrap_or(false),
                },
                order,
            });
        }
        for (id, src) in v.get("sources").map(Json::entries).unwrap_or(&[]) {
            self.sources.insert(id.clone(), source_from(src));
        }
        for (id, list) in v.get("choices").map(Json::entries).unwrap_or(&[]) {
            self.choices.insert(id.clone(), list.as_array().iter().filter_map(|c| c.as_str().map(String::from)).collect());
        }
        for (id, p) in v.get("placements").map(Json::entries).unwrap_or(&[]) {
            let (Some(from), Some(to)) = (p.str_at("from"), p.str_at("to")) else { continue };
            // Read back only what is still a plain path inside the game folder.
            if !(to.is_empty() || crate::gamedata::safe_path(to)) || !(from.is_empty() || zip::safe_relative(from).is_ok()) {
                continue;
            }
            self.placements.insert(
                id.clone(),
                PlaceInfo {
                    from: from.to_string(),
                    to: to.to_string(),
                    rule: p.str_at("rule").unwrap_or("").to_string(),
                    replace: p.get("replace").and_then(Json::as_bool).unwrap_or(false),
                    skipped: p.get("skipped").map(Json::as_array).unwrap_or(&[]).iter().filter_map(|x| x.as_str().map(String::from)).collect(),
                },
            );
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
                    "{{\"id\":{},\"savedAt\":{},\"modsOn\":{},\"played\":{},\"order\":[{}]}}",
                    quote(&s.setup.id),
                    s.setup.saved_at,
                    s.setup.mods_on,
                    s.setup.played,
                    order.join(",")
                )
            })
            .collect();
        // Sources and choices only for mods that are still somewhere.
        let known: HashSet<&str> =
            self.mods.iter().map(|m| m.id.as_str()).chain(self.removed_extra.iter().map(|r| r.removed.item.id.as_str())).collect();
        let mut sources: Vec<String> = self
            .sources
            .iter()
            .filter(|(id, _)| known.contains(id.as_str()))
            .map(|(id, src)| format!("{}:{}", quote(id), source_json(src)))
            .collect();
        sources.sort();
        let mut choices: Vec<String> = self
            .choices
            .iter()
            .filter(|(id, _)| known.contains(id.as_str()))
            .map(|(id, list)| format!("{}:[{}]", quote(id), list.iter().map(|c| quote(c)).collect::<Vec<_>>().join(",")))
            .collect();
        choices.sort();
        let mut placements: Vec<String> = self
            .placements
            .iter()
            .filter(|(id, _)| known.contains(id.as_str()))
            .map(|(id, p)| format!("{}:{}", quote(id), p.to_json()))
            .collect();
        placements.sort();
        let text = format!(
            "{{\"version\":1,\n\"mods\":[\n{}\n],\n\"removed\":[\n{}\n],\n\"setups\":[\n{}\n],\n\"sources\":{{\n{}\n}},\n\"choices\":{{\n{}\n}},\n\"placements\":{{\n{}\n}},\n\"lastBuild\":{}}}\n",
            mods.join(",\n"),
            removed.join(",\n"),
            setups.join(",\n"),
            sources.join(",\n"),
            choices.join(",\n"),
            placements.join(",\n"),
            opt(&self.last_build)
        );
        write_atomic(&self.state_path(), &text)
    }

    /// Every mod's files in the game (game-relative), in Turbine's order, then the Recently removed ones (turned off),
    /// for the checks before Play (rules::mod_checks) and The Witcher 3's mods.settings.
    pub fn mod_files(&self) -> Vec<rules::ModFiles> {
        let mut out: Vec<rules::ModFiles> = self
            .mods
            .iter()
            .map(|m| rules::ModFiles { id: m.id.clone(), name: m.name.clone(), enabled: m.enabled, files: self.staged_files(&self.mod_dir(&m.id)) })
            .collect();
        for r in &self.removed_extra {
            let m = &r.removed.item;
            let files = self.staged_files(&self.removed_dir(&m.id));
            out.push(rules::ModFiles { id: m.id.clone(), name: m.name.clone(), enabled: false, files });
        }
        out
    }

    /// The files in one staged folder (game-relative), read again only when the folder was made anew.
    fn staged_files(&self, dir: &Path) -> Vec<String> {
        let Ok(meta) = fs::metadata(dir) else { return Vec::new() };
        let stamp = (meta.ino(), meta.mtime() * 1_000_000_000 + meta.mtime_nsec());
        let mut cache = self.files_cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((ino, mtime, list)) = cache.get(dir) {
            if (*ino, *mtime) == stamp {
                return list.clone();
            }
        }
        let list: Vec<String> = walk_files(dir).into_iter().map(|(rel, _)| rel).collect();
        cache.insert(dir.to_path_buf(), (stamp.0, stamp.1, list.clone()));
        list
    }

    /// Who wins when turned-on mods change the same file (later in the list wins).
    pub fn overlaps(&self) -> Vec<crate::overlaps::Overlap> {
        crate::overlaps::overlaps(&self.mod_files())
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
    /// A mod with an installer is installed with the installer's own suggestions (or the choices made last time);
    /// use `import_archive` to let the player choose.
    pub fn import_zip(&mut self, archive: &Path) -> Result<Mod, String> {
        let source = SourceInfo { kind: "zip".into(), ..SourceInfo::default() };
        match self.import_archive(archive, source)? {
            Import::Added(done) => Ok(done.item),
            Import::NeedsChoices(p) => self.finish_install(*p).map(|d| d.item),
            Import::NeedsPlace(p) => self.place_unasked(*p).map(|d| d.item),
        }
    }

    /// Adds a mod from an archive (zip, 7z or rar) and records where it came from.
    /// The same download (same file name, or the same package from the same source) added again replaces the old
    /// one's files and keeps its place and on/off state.
    pub fn import_archive(&mut self, archive: &Path, source: SourceInfo) -> Result<Import, String> {
        let target = self.target_for(archive, source, None)?;
        self.begin(archive, target)
    }

    /// Replaces a mod's files with a new download (an update), keeping its place, on/off state and name.
    /// Installer choices made last time are the starting point if the new version has an installer.
    pub fn replace_mod(&mut self, id: &str, archive: &Path, source: SourceInfo) -> Result<Import, String> {
        self.index_of(id).map_err(|e| e.to_string())?;
        let target = self.target_for(archive, source, Some(id))?;
        self.begin(archive, target)
    }

    /// Where each installed mod came from, in order, for the update checker.
    pub fn installed_sources(&self) -> Vec<(String, SourceInfo)> {
        self.mods.iter().map(|m| (m.id.clone(), self.source_of(&m.id))).collect()
    }

    /// Where a mod came from (a zip file with no ID when Turbine doesn't know).
    pub fn source_of(&self, id: &str) -> SourceInfo {
        self.sources.get(id).cloned().unwrap_or_else(|| SourceInfo { kind: "zip".into(), ..SourceInfo::default() })
    }

    /// Stages the files an installer's choices pick and adds (or updates) the mod. In a game Turbine places by
    /// looking at the files (Basic support), the installer's output goes through the same placement and
    /// protections as any other mod; when the player would be asked, nobody is here to answer, so it goes to the
    /// suggested folder and never replaces the game's own files (use `finish_install_import` to ask).
    pub fn finish_install(&mut self, pending: Pending) -> Result<Installed, String> {
        match self.finish_install_import(pending)? {
            Import::Added(done) => Ok(done),
            Import::NeedsChoices(p) => {
                self.cancel_install(*p);
                Err("The installer still has choices to make.".into())
            }
            Import::NeedsPlace(p) => self.place_unasked(*p),
        }
    }

    /// A Basic game's waiting mod with no one to ask: where detection put it, never replacing the game's own
    /// files. When detection couldn't tell where it goes, it isn't added (the player adds it from Turbine).
    fn place_unasked(&mut self, p: PendingPlace) -> Result<Installed, String> {
        let (from, to) = match &p.placement {
            Placement::Sure { from, to, .. } | Placement::Guess { from, to, .. } => (from.clone(), to.clone()),
            Placement::Ask { .. } => {
                let e = format!("{} couldn't be added: Turbine can't tell where its files go. Add it in Turbine to choose.", p.target.shown);
                self.cancel_place(p);
                return Err(e);
            }
        };
        self.finish_place(p, &from, &to, false)
    }

    /// `finish_install` that hands back "Where do these files go?" (Import::NeedsPlace) in a Basic game.
    pub fn finish_install_import(&mut self, pending: Pending) -> Result<Import, String> {
        let Pending { session, target, unpacked, .. } = pending;
        let fail = |what: String| format!("{} couldn't be added: {what}.", target.shown);
        let result = (|| {
            let done = session.finish().map_err(|e| fail(e.trim_end_matches('.').to_string()))?;
            let root = self.rules.mod_root();
            let map: Vec<(String, String)> = done.files.iter().map(|(from, to)| (from.clone(), join_rel(&root, to))).collect();
            if map.is_empty() {
                return Err(fail("the choices made don't install any files".into()));
            }
            let mut notes: Vec<String> = done.missing.iter().map(|m| format!("The installer names {m}, which isn't in the archive.")).collect();
            if !done.flags.is_empty() {
                notes.push(format!(
                    "Installer choices: {}",
                    done.flags.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(", ")
                ));
            }
            let choices = session.choices();
            let dests = unique_dests(&map);
            if self.rules.place(&dests, &target.name, &self.game).is_some() {
                // Basic support: lay the installer's output out as an archive of its own, then place it like
                // any other mod (placement, the refused table and the overwrite question all apply).
                let out = unpacked.with_extension("output");
                let _ = remove_dir(&out);
                let laid = (|| -> io::Result<()> {
                    for (from, to) in &map {
                        let src = unpacked.join(zip::safe_relative(from).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?);
                        let dest = out.join(zip::safe_relative(to).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?);
                        if !src.is_file() {
                            continue;
                        }
                        if let Some(p) = dest.parent() {
                            fs::create_dir_all(p)?;
                        }
                        let _ = fs::remove_file(&dest);
                        fs::copy(&src, &dest)?;
                    }
                    Ok(())
                })();
                if let Err(e) = laid {
                    let _ = remove_dir(&out);
                    return Err(fail(format!("couldn't read or write a file ({e})")));
                }
                remove_dir(&unpacked).map_err(|e| fail(format!("couldn't read or write a file ({e})")))?;
                fs::rename(&out, &unpacked).map_err(|e| fail(format!("couldn't read or write a file ({e})")))?;
                let files: Vec<String> = walk_files(&unpacked).into_iter().map(|(rel, _)| rel).collect();
                return self.place_or_ask(&unpacked.clone(), &files, &target, &unpacked, notes, Some(choices)).unwrap_or_else(|| Err(fail("its files have nowhere to go".into())));
            }
            let mut out = self.stage_and_commit(&unpacked, &map, &target, notes)?;
            self.keep_choices(&out.item.id, Some(choices));
            self.save().map_err(|e| fail(format!("couldn't read or write a file ({e})")))?;
            out.notes.retain(|n| !n.is_empty());
            Ok(Import::Added(out))
        })();
        if !matches!(result, Ok(Import::NeedsPlace(_))) {
            let _ = remove_dir(&unpacked);
        }
        result
    }

    /// Drops an installer the player backed out of.
    pub fn cancel_install(&mut self, pending: Pending) {
        let _ = remove_dir(&pending.unpacked);
    }

    /// Game files a placement would land on that Turbine didn't place (the game's own), and files it may never
    /// place (basic::refused, with why).
    pub fn place_conflicts(&self, files: &[String], from: &str, to: &str) -> Result<Conflicts, String> {
        conflicts_in(&self.game, &self.root, files, from, to)
    }

    /// The pending mod's conflicts for a folder the player is looking at (the overwrite sheet after "Put them here").
    pub fn pending_conflicts(&self, pending: &PendingPlace, from: &str, to: &str) -> Result<Conflicts, String> {
        self.place_conflicts(&pending.files, from, to)
    }

    /// Stages a Basic game's mod at its placement and remembers the placement for its updates.
    #[allow(clippy::too_many_arguments)]
    fn place_and_commit(
        &mut self,
        work: &Path,
        files: &[String],
        target: &Target,
        mut info: PlaceInfo,
        conflicts: Vec<String>,
        refused: Vec<(String, &'static str)>,
        mut notes: Vec<String>,
    ) -> Result<Installed, String> {
        let fail = |what: String| format!("{} couldn't be added: {what}.", target.shown);
        let map = basic::map_placement(files, &info.from, &info.to).map_err(fail)?;
        let map: Vec<(String, String)> = map.into_iter().filter(|(_, d)| !refused.iter().any(|(r, _)| r.eq_ignore_ascii_case(d))).collect();
        for (r, why) in &refused {
            notes.push(format!("Refused {r}: {why}. Turbine never places it."));
        }
        if map.is_empty() {
            return Err(fail("every file in it is one Turbine never places".into()));
        }
        let mut out = self.stage_and_commit(work, &map, target, notes)?;
        info.skipped = if info.replace { Vec::new() } else { conflicts };
        self.placements.insert(out.item.id.clone(), info.clone());
        self.save().map_err(|e| fail(format!("couldn't read or write a file ({e})")))?;
        out.placed = Some(info);
        Ok(out)
    }

    /// Adds a Basic game's waiting mod with its files from the archive folder `from` going into the game folder
    /// `to`. `replace`: it may replace files the game already has (kept and put back when it goes).
    pub fn finish_place(&mut self, pending: PendingPlace, from: &str, to: &str, replace: bool) -> Result<Installed, String> {
        let fail = |what: String| format!("{} couldn't be added: {what}.", pending.target.shown);
        let result = (|| {
            if !basic::archive_has_folder(&pending.files, from) {
                return Err(fail(format!("the archive has no folder {from}")));
            }
            let (conflicts, refused) = self.place_conflicts(&pending.files, from, to).map_err(fail)?;
            let rule = match &pending.placement {
                Placement::Sure { from: f, to: t, rule } | Placement::Guess { from: f, to: t, rule, .. } if f == from && t == to => rule.to_string(),
                _ => "you".to_string(),
            };
            let info = PlaceInfo { from: from.to_string(), to: to.to_string(), rule, replace, skipped: Vec::new() };
            let out = self.place_and_commit(&pending.unpacked, &pending.files, &pending.target, info, conflicts, refused, pending.notes.clone())?;
            self.keep_choices(&out.item.id, pending.choices.clone());
            Ok(out)
        })();
        let _ = remove_dir(&pending.unpacked);
        result
    }

    /// Drops a mod the player backed out of on "Where do these files go?".
    pub fn cancel_place(&mut self, pending: PendingPlace) {
        let _ = remove_dir(&pending.unpacked);
    }

    /// Where a Basic game's mod went, if it is one.
    pub fn placement(&self, id: &str) -> Option<&PlaceInfo> {
        self.placements.get(id)
    }

    /// Moves a Basic game's mod to another folder of the game: its staged files move (no archive needed), every
    /// file keeping its path below the folder. Applied to the game with the next apply.
    pub fn change_placement(&mut self, id: &str, to: &str) -> io::Result<()> {
        let bad = |m: String| io::Error::new(io::ErrorKind::InvalidInput, m);
        if !(to.is_empty() || crate::gamedata::safe_path(to)) {
            return Err(bad(format!("{to} isn't a plain folder inside the game folder")));
        }
        self.index_of(id)?;
        let mut info = self.placements.get(id).cloned().ok_or_else(|| bad("That mod wasn't placed by folder, so it can't be moved.".into()))?;
        let dir = self.mod_dir(id);
        let tmp = self.root.join("tmp");
        fs::create_dir_all(&tmp)?;
        let staged = tmp.join(format!("{id}.moving"));
        remove_dir(&staged)?;
        fs::create_dir_all(&staged)?;
        let old: Vec<String> = info.to.split('/').filter(|p| !p.is_empty()).map(str::to_lowercase).collect();
        let moved = (|| -> io::Result<()> {
            for (rel, src) in walk_files(&dir) {
                // Compared part by part (never by byte length: a lowercased name can be longer or shorter).
                let parts: Vec<&str> = rel.split('/').collect();
                let under = parts.len() > old.len() && parts.iter().zip(&old).all(|(a, b)| a.to_lowercase() == *b);
                let new_rel = if under { join_rel(to, &parts[old.len()..].join("/")) } else { rel.clone() };
                if !deploy::safe_rel(&new_rel) {
                    return Err(bad(format!("{new_rel} isn't a plain path inside the game folder")));
                }
                let dest = staged.join(&new_rel);
                if let Some(p) = dest.parent() {
                    fs::create_dir_all(p)?;
                }
                // A link (or copy) into the new folder: the mod's folder stays whole until the swap.
                if fs::hard_link(&src, &dest).is_err() {
                    fs::copy(&src, &dest)?;
                }
            }
            Ok(())
        })();
        if let Err(e) = moved {
            remove_dir(&staged)?;
            return Err(e);
        }
        let old_dir = tmp.join(format!("{id}.old"));
        remove_dir(&old_dir)?;
        fs::rename(&dir, &old_dir)?;
        fs::rename(&staged, &dir)?;
        remove_dir(&old_dir)?;
        info.to = to.to_string();
        info.rule = "you".into();
        info.skipped.clear();
        self.placements.insert(id.to_string(), info);
        self.save()
    }

    /// Lets a Basic game's mod replace the game's own files (or not, the default).
    pub fn set_replace(&mut self, id: &str, replace: bool) -> io::Result<()> {
        match self.placements.get_mut(id) {
            Some(p) if p.replace != replace => {
                p.replace = replace;
                self.save()
            }
            _ => Ok(()),
        }
    }

    /// The game's folders right in `rel`, for "Where do these files go?" (folders Turbine made are marked).
    pub fn game_folders_json(&self, rel: &str) -> String {
        let index = basic::GameIndex::cached(&self.game.install_dir);
        let record = Record::load(&self.deploy_paths().record).unwrap_or_default();
        let made: HashSet<String> = record.dirs.iter().map(|d| d.to_lowercase()).collect();
        basic::game_folders_json(&index, rel, &made)
    }

    fn target_for(&self, archive: &Path, source: SourceInfo, replace: Option<&str>) -> Result<Target, String> {
        let file_name = archive.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let shown = if file_name.is_empty() { archive.to_string_lossy().into_owned() } else { file_name.clone() };
        let format = archives::detect(archive)
            .map_err(|e| format!("{shown} couldn't be added: couldn't read the file ({e})."))?
            .ok_or_else(|| format!("{shown} couldn't be added: it isn't a zip, 7z or rar archive."))?;
        let (mut name, mut version) = clean_name(&file_name);
        if source.version.is_some() {
            version = source.version.clone();
        }
        if source.kind == "thunderstore" && !source.id.is_empty() {
            // "Owner-Package_Name": the package part reads better.
            name = source.id.rsplit_once('-').map(|(_, n)| n).unwrap_or(&source.id).replace('_', " ");
        }
        let existing = match replace {
            Some(id) => self.mods.iter().position(|m| m.id == id),
            None => {
                let same_source = (!source.id.is_empty() && source.kind != "zip")
                    .then(|| self.mods.iter().position(|m| self.sources.get(&m.id).is_some_and(|s| s.kind == source.kind && s.id == source.id)))
                    .flatten();
                same_source.or_else(|| if file_name.is_empty() { None } else { self.archives.iter().position(|a| *a == file_name) })
            }
        };
        let id = match existing {
            Some(i) => self.mods[i].id.clone(),
            None => self.new_id(&name),
        };
        Ok(Target {
            id,
            existing,
            name,
            version,
            file_name,
            shown,
            source,
            format_label: match format {
                Format::Zip => "zip",
                Format::SevenZip => "7z",
                Format::Rar => "rar",
            }
            .into(),
            keep_name: replace.is_some(),
        })
    }

    /// Plugins the player has and whether they're on, for installer conditions: the game's Data folder (with the
    /// load order) and the plugins of the mods turned on now.
    fn plugin_env(&self) -> fomod::Env {
        let mut list: Vec<(String, bool)> = Vec::new();
        if self.rules.has_plugins() {
            if let Ok(order) = plugins::LoadOrder::read(&self.game) {
                list.extend(order.states());
            }
            let root = self.rules.mod_root();
            for m in self.mods.iter().filter(|m| m.enabled) {
                let dir = if root.is_empty() { self.mod_dir(&m.id) } else { self.mod_dir(&m.id).join(&root) };
                if let Ok(rd) = fs::read_dir(dir) {
                    for e in rd.flatten() {
                        let n = e.file_name().to_string_lossy().into_owned();
                        if plugins::is_plugin_name(&n) && !list.iter().any(|(x, _)| x.eq_ignore_ascii_case(&n)) {
                            list.push((n, true));
                        }
                    }
                }
            }
        }
        // The game's version isn't known here, so version conditions count as met.
        fomod::Env::with_plugins(&list)
    }

    /// Unpacks and works out where the files go; stages them, or hands back an installer waiting for choices.
    fn begin(&mut self, archive: &Path, target: Target) -> Result<Import, String> {
        let fail = |what: String| format!("{} couldn't be added: {what}.", target.shown);
        let tmp = self.root.join("tmp");
        fs::create_dir_all(&tmp).map_err(|e| fail(format!("couldn't read or write a file ({e})")))?;
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let unpacked = tmp.join(format!("{}-{stamp}.unpacked", target.id));
        let _ = remove_dir(&unpacked);
        let result = (|| -> Result<Import, String> {
            let mut files = self.unpack(archive, &unpacked).map_err(|e| fail(e.trim_end_matches('.').to_string()))?;
            if files.is_empty() {
                return Err(fail(format!("the {} file is empty", target.format_label)));
            }
            let mut work = unpacked.clone();
            let mut notes = Vec::new();

            // An installer that carries its files in an archive of their own (SMAPI): open that one instead.
            if let Some(inner) = self.rules.inner_archive(&files) {
                let inner_path = unpacked.join(zip::safe_relative(&inner).map_err(|e| fail(e.to_string()))?);
                let inner_dir = unpacked.join(".inner");
                files = self.unpack(&inner_path, &inner_dir)
                    .map_err(|e| fail(format!("the files inside its installer can't be opened ({})", e.trim_end_matches('.'))))?;
                work = inner_dir;
                let map = self.rules.map_inner(&files).map_err(|e| fail(e.trim_end_matches('.').to_string()))?;
                notes.push(format!("Set up from {inner} inside the installer; the installer itself was not run."));
                return self.stage_and_commit(&work, &map, &target, notes).map(Import::Added);
            }

            // A FOMOD installer.
            if let Some(found) = fomod::find(&files) {
                let info = match &found.info {
                    Some(p) => fomod::parse_info(&read_small(&work.join(zip::safe_relative(p).map_err(|e| fail(e.to_string()))?)).unwrap_or_default()),
                    None => fomod::Info::default(),
                };
                if let Some(cfg_path) = &found.config {
                    let bytes = read_small(&work.join(zip::safe_relative(cfg_path).map_err(|e| fail(e.to_string()))?))
                        .map_err(|e| fail(format!("its installer file can't be read ({e})")))?;
                    let config = fomod::parse(&bytes).map_err(fail)?;
                    let mut session = fomod::Session::new(config, self.plugin_env(), &files, &found.root);
                    if let Some(prev) = self.choices.get(&target.id) {
                        session.apply_choices(prev);
                    }
                    let mut target = target.clone();
                    if target.existing.is_none() {
                        if let Some(n) = info.name.clone().filter(|n| !n.trim().is_empty()).or_else(|| {
                            Some(session.module_name().to_string()).filter(|n| !n.trim().is_empty())
                        }) {
                            target.name = n.trim().to_string();
                        }
                    }
                    if target.version.is_none() {
                        target.version = info.version.clone();
                    }
                    let name = if target.keep_name { self.mods[target.existing.unwrap_or(0)].name.clone() } else { target.name.clone() };
                    let pending = Pending { session, info, name, target, unpacked: unpacked.clone() };
                    if pending.session.has_choices() {
                        return Ok(Import::NeedsChoices(Box::new(pending)));
                    }
                    // No steps: only fixed files.
                    return self.finish_install_import(pending);
                }
            }

            // Basic support: where the files go is worked out from the files and the game folder (or asked).
            if let Some(r) = self.place_or_ask(&work, &files, &target, &unpacked, notes.clone(), None) {
                return r;
            }
            let map = self.rules.map_archive(&files, &folder_safe(&target.name)).map_err(|e| fail(e.trim_end_matches('.').to_string()))?;
            notes.extend(self.rules.install_notes(&files));
            self.stage_and_commit(&work, &map, &target, notes).map(Import::Added)
        })();
        // An installer waiting for choices (or a mod waiting for where it goes) keeps its files; everything else is
        // cleaned up.
        if !matches!(result, Ok(Import::NeedsChoices(_)) | Ok(Import::NeedsPlace(_))) {
            let _ = remove_dir(&unpacked);
        }
        result
    }

    /// Basic support's placement for files in `work` (paths `files`): the remembered placement of the mod being
    /// updated, or detection's sure answer, or the player is asked ("Where do these files go?", or only whether it
    /// may replace the game's files). None when the game has rules of its own for where files go.
    fn place_or_ask(
        &mut self,
        work: &Path,
        files: &[String],
        target: &Target,
        unpacked: &Path,
        notes: Vec<String>,
        choices: Option<Vec<String>>,
    ) -> Option<Result<Import, String>> {
        let fail = |what: String| format!("{} couldn't be added: {what}.", target.shown);
        let answer = self.rules.place(files, &target.name, &self.game)?;
        Some((|| {
            let answer = answer.map_err(|e| fail(e.trim_end_matches('.').to_string()))?;
            let remembered = target
                .existing
                .and_then(|i| self.placements.get(&self.mods[i].id))
                .filter(|p| basic::archive_has_folder(files, &p.from))
                .cloned();
            let name = if target.keep_name { self.mods[target.existing.unwrap_or(0)].name.clone() } else { target.name.clone() };
            let (from, to, rule, replace) = match (&remembered, &answer) {
                (Some(p), _) => (p.from.clone(), p.to.clone(), p.rule.clone(), Some(p.replace)),
                (None, Placement::Sure { from, to, rule }) => (from.clone(), to.clone(), rule.to_string(), None),
                _ => {
                    let pending = PendingPlace {
                        name,
                        placement: answer,
                        sure: false,
                        conflicts: Vec::new(),
                        refused: Vec::new(),
                        files: files.to_vec(),
                        target: target.clone(),
                        unpacked: unpacked.to_path_buf(),
                        notes,
                        choices,
                    };
                    return Ok(Import::NeedsPlace(Box::new(pending)));
                }
            };
            let (conflicts, refused) = self.place_conflicts(files, &from, &to).map_err(|e| fail(e.trim_end_matches('.').to_string()))?;
            if replace.is_none() && !conflicts.is_empty() {
                let pending = PendingPlace {
                    name,
                    placement: Placement::Sure { from, to, rule: leak_rule(&rule) },
                    sure: true,
                    conflicts,
                    refused,
                    files: files.to_vec(),
                    target: target.clone(),
                    unpacked: unpacked.to_path_buf(),
                    notes,
                    choices,
                };
                return Ok(Import::NeedsPlace(Box::new(pending)));
            }
            let info = PlaceInfo { from, to, rule, replace: replace.unwrap_or(false), skipped: Vec::new() };
            let out = self.place_and_commit(work, files, target, info, conflicts, refused, notes)?;
            self.keep_choices(&out.item.id, choices);
            Ok(Import::Added(out))
        })())
    }

    /// Remembers an installer's choices for the mod's next update (None: it had no installer).
    fn keep_choices(&mut self, id: &str, choices: Option<Vec<String>>) {
        match choices {
            None => {}
            Some(c) if c.is_empty() => {
                self.choices.remove(id);
            }
            Some(c) => {
                self.choices.insert(id.to_string(), c);
            }
        }
        let _ = self.save();
    }

    /// Moves the mapped files from `work` into the mod's folder (replacing an earlier copy) and records the mod.
    fn stage_and_commit(&mut self, work: &Path, map: &[(String, String)], target: &Target, mut notes: Vec<String>) -> Result<Installed, String> {
        let shown = &target.shown;
        let fail = |what: String| format!("{shown} couldn't be added: {what}.");
        let io_fail = |e: io::Error| fail(format!("couldn't read or write a file ({e})"));
        if map.is_empty() {
            return Err(fail("none of its files is something Turbine knows where to put".into()));
        }
        let id = target.id.clone();
        let tmp = self.root.join("tmp");
        let staged = tmp.join(format!("{id}.staged"));
        let _ = remove_dir(&staged);
        let result = (|| -> Result<(Vec<String>, usize), String> {
            fs::create_dir_all(&staged).map_err(io_fail)?;
            let mut game_files = Vec::new();
            // A source used twice (a loader that's also the launcher, an installer file in two places) is copied
            // from where it was first put.
            let mut moved: HashMap<String, PathBuf> = HashMap::new();
            for (from, to) in map {
                let from_rel = zip::safe_relative(from).map_err(|e| fail(e.to_string()))?;
                let to_rel = zip::safe_relative(to).map_err(|e| fail(e.to_string()))?;
                if to_rel.as_os_str().is_empty() {
                    continue;
                }
                let dest = staged.join(&to_rel);
                if let Some(p) = dest.parent() {
                    fs::create_dir_all(p).map_err(io_fail)?;
                }
                let key = from_rel.to_string_lossy().to_lowercase();
                if let Some(first) = moved.get(&key) {
                    if first != &dest {
                        fs::copy(first, &dest).map_err(io_fail)?;
                    }
                } else {
                    let src = work.join(&from_rel);
                    if !fs::symlink_metadata(&src).map(|m| m.is_file()).unwrap_or(false) {
                        continue;
                    }
                    if dest.exists() {
                        fs::remove_file(&dest).map_err(io_fail)?;
                    }
                    fs::rename(&src, &dest).map_err(io_fail)?;
                    moved.insert(key, dest.clone());
                }
                let rel = rel_of(&to_rel);
                if !game_files.iter().any(|g: &String| g.eq_ignore_ascii_case(&rel)) {
                    game_files.push(rel);
                }
            }
            // Files the mod needs from the game's own folder.
            for (game_rel, to) in self.rules.game_copies(&game_files) {
                let to_rel = zip::safe_relative(&to).map_err(|e| fail(e.to_string()))?;
                match rules::find_ci(&self.game.install_dir, &game_rel).filter(|p| p.is_file()) {
                    Some(src) => {
                        let dest = staged.join(&to_rel);
                        fs::copy(&src, &dest).map_err(io_fail)?;
                        game_files.push(rel_of(&to_rel));
                        notes.push(format!("Copied the game's {game_rel} as {to}."));
                    }
                    None => notes.push(format!("The game's {game_rel} wasn't found, so {to} wasn't made.")),
                }
            }
            for exe in self.rules.executables(&game_files) {
                if let Ok(rel) = zip::safe_relative(&exe) {
                    let p = staged.join(rel);
                    if p.is_file() {
                        use std::os::unix::fs::PermissionsExt;
                        fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).map_err(io_fail)?;
                    }
                }
            }
            let count = walk_files(&staged).len();
            if count == 0 {
                return Err(fail("none of its files is something Turbine knows where to put".into()));
            }
            Ok((game_files, count))
        })();
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
        let label = source_label(&target.source.kind, &target.format_label);
        let mut source = target.source.clone();
        if source.kind.is_empty() {
            source.kind = "zip".into();
        }
        if source.version.is_none() {
            source.version = target.version.clone();
        }
        let item = match target.existing {
            Some(i) => {
                let m = &mut self.mods[i];
                if !target.keep_name {
                    m.name = target.name.clone();
                }
                m.version = target.version.clone();
                m.files = count;
                m.needs_loader = needs_loader;
                m.added = now;
                m.source = label;
                if !target.file_name.is_empty() {
                    self.archives[i] = target.file_name.clone();
                }
                m.clone()
            }
            None => {
                let m = Mod { id: id.clone(), name: target.name.clone(), enabled: true, version: target.version.clone(), source: label, files: count, needs_loader, added: now };
                self.mods.push(m.clone());
                self.archives.push(target.file_name.clone());
                m
            }
        };
        self.sources.insert(id, source);
        self.save().map_err(io_fail)?;
        Ok(Installed { item, notes, placed: None })
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

    /// Deletes one mod from Recently removed for good (its files in Turbine's own store only; the game folder lost
    /// them when it was removed). It can't be put back afterwards.
    pub fn purge_removed(&mut self, id: &str) -> io::Result<()> {
        let r = self.removed_extra.iter().position(|r| r.removed.item.id == id).ok_or_else(|| not_found(id))?;
        remove_dir(&self.removed_dir(id))?;
        self.removed_extra.remove(r);
        self.save()
    }

    /// Empties Recently removed for good. Returns how many mods were deleted.
    pub fn purge_all_removed(&mut self) -> io::Result<usize> {
        // One at a time: each entry leaves the list (and the list is saved) right after its folder is gone, so a
        // failure partway leaves only the mods that are really still there.
        let ids: Vec<String> = self.removed_extra.iter().map(|r| r.removed.item.id.clone()).collect();
        let mut done = 0;
        for id in &ids {
            remove_dir(&self.removed_dir(id))?;
            self.removed_extra.retain(|r| r.removed.item.id != *id);
            self.save()?;
            done += 1;
        }
        Ok(done)
    }

    /// Every file the enabled mods put in the game, in order (later ones win).
    fn wanted(&self) -> Vec<Wanted> {
        self.wanted_except(&[])
    }

    /// Every file of the turned-on mods (but `skip`), in order. A Basic game's mod may only replace the game's own
    /// files when the player said so for it, and never places a file in the refused table (basic::refused).
    fn wanted_except(&self, skip: &[String]) -> Vec<Wanted> {
        let record = Record::load(&self.deploy_paths().record).unwrap_or_default();
        let ours: HashSet<String> = record.files.iter().map(|p| p.rel.to_lowercase()).collect();
        let guarded = self.rules.guarded();
        let mut out = Vec::new();
        for m in self.mods.iter().filter(|m| m.enabled && !skip.contains(&m.id)) {
            let place = self.placements.get(&m.id);
            for (rel, source) in walk_files(&self.mod_dir(&m.id)) {
                // A guarded game's mod with no placement (laid out by a player file's rules) may not replace.
                let unplaced = PlaceInfo::default();
                let place = place.or(if guarded { Some(&unplaced) } else { None });
                match place {
                    Some(p) => {
                        let exists_not_ours = !ours.contains(&rel.to_lowercase()) && rules::find_ci(&self.game.install_dir, &rel).is_some();
                        if basic::refused(&rel, exists_not_ours).is_some() || through_link(&self.game.install_dir, &rel) {
                            continue;
                        }
                        out.push(Wanted { rel, source, may_replace: p.replace });
                    }
                    None => out.push(Wanted { rel, source, may_replace: true }),
                }
            }
        }
        out
    }

    /// Files of turned-on Basic mods that are refused (never placed), with why.
    pub fn refused_files(&self) -> Vec<(String, &'static str)> {
        let record = Record::load(&self.deploy_paths().record).unwrap_or_default();
        let ours: HashSet<String> = record.files.iter().map(|p| p.rel.to_lowercase()).collect();
        let mut out = Vec::new();
        let guarded = self.rules.guarded();
        for m in self.mods.iter().filter(|m| m.enabled && (guarded || self.placements.contains_key(&m.id))) {
            for (rel, _) in walk_files(&self.mod_dir(&m.id)) {
                let exists_not_ours = !ours.contains(&rel.to_lowercase()) && rules::find_ci(&self.game.install_dir, &rel).is_some();
                if let Some(why) = basic::refused(&rel, exists_not_ours) {
                    out.push((rel, why));
                } else if through_link(&self.game.install_dir, &rel) {
                    out.push((rel, "goes through a link in the game folder, so it would land outside the game"));
                }
            }
        }
        out
    }

    /// Makes the game folder match the enabled mods in order. Saves a setup when it changed something.
    pub fn apply(&mut self) -> io::Result<Report> {
        self.apply_stopping(None)
    }

    /// `apply` that stops after `stop_after` steps as if the power went out (for tests).
    #[doc(hidden)]
    pub fn apply_stopping(&mut self, stop_after: Option<usize>) -> io::Result<Report> {
        let report = self.deploy(self.wanted(), stop_after)?;
        let skipped_changed = self.note_skipped(&report);
        if report.added + report.removed > 0 {
            self.save_setup();
            self.last_build = self.game.build.clone();
            self.save()?;
        } else if skipped_changed {
            self.save()?;
        }
        Ok(report)
    }

    /// Writes down which files each Basic mod had skipped this time. True when something changed.
    fn note_skipped(&mut self, report: &Report) -> bool {
        if self.placements.is_empty() {
            return false;
        }
        let skipped: HashSet<String> = report.skipped_existing.iter().map(|r| r.to_lowercase()).collect();
        let mut changed = false;
        let ids: Vec<String> = self.mods.iter().filter(|m| m.enabled && self.placements.contains_key(&m.id)).map(|m| m.id.clone()).collect();
        for id in ids {
            let files: Vec<String> = self.staged_files(&self.mod_dir(&id)).into_iter().filter(|f| skipped.contains(&f.to_lowercase())).collect();
            if let Some(p) = self.placements.get_mut(&id) {
                if p.skipped != files {
                    p.skipped = files;
                    changed = true;
                }
            }
        }
        changed
    }

    /// Runs deploy. Files in the game whose staged copy was replaced since (the same download added again) are
    /// taken out first, so the new copy goes in.
    fn deploy(&self, wanted: Vec<Wanted>, stop_after: Option<usize>) -> io::Result<Report> {
        let paths = self.deploy_paths();
        let recovered = deploy::recover(&paths)?.is_some();
        let record = Record::load(&paths.record)?;
        let stale: HashSet<String> = record
            .files
            .iter()
            .filter(|p| match fs::metadata(&p.source) {
                // The staged file as it was when placed is written down: any change means a new copy.
                Ok(m) if p.src_ino != 0 || p.src_size != 0 || p.src_mtime != 0 => {
                    m.ino() != p.src_ino || m.len() != p.src_size || m.mtime() * 1_000_000_000 + m.mtime_nsec() != p.src_mtime
                }
                Ok(m) if p.copy => m.len() != p.size || m.mtime() * 1_000_000_000 + m.mtime_nsec() != p.mtime,
                Ok(m) => m.ino() != p.ino,
                Err(_) => false,
            })
            .map(|p| p.rel.to_lowercase())
            .collect();
        let mut first = Report::default();
        if !stale.is_empty() {
            let without: Vec<Wanted> = wanted.iter().filter(|w| !stale.contains(&w.rel.to_lowercase())).cloned().collect();
            first = deploy::apply_guarded(&paths, &without, stop_after)?;
        }
        let mut report = deploy::apply_guarded(&paths, &wanted, stop_after)?;
        report.refused = self.refused_files().into_iter().map(|(r, _)| r).collect();
        report.added += first.added;
        report.removed += first.removed;
        report.copied_not_linked += first.copied_not_linked;
        report.recovered |= recovered || first.recovered;
        fn before(a: Vec<String>, b: &mut Vec<String>) {
            let mut a = a;
            a.append(b);
            *b = a;
        }
        before(first.changed_by_others, &mut report.changed_by_others);
        before(first.put_back, &mut report.put_back);
        before(first.player_changes_saved, &mut report.player_changes_saved);
        before(first.player_changes_restored, &mut report.player_changes_restored);
        before(first.bad_entries, &mut report.bad_entries);
        before(first.left_in_backups, &mut report.left_in_backups);
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
        self.saved.insert(0, SavedSetup { setup: Setup { id, saved_at: now, mods_on, played: false }, order });
        // The newest played setup is always kept: it's what "Restore last working setup" goes back to.
        let keep_played = self.saved.iter().position(|s| s.setup.played).filter(|&i| i >= KEEP_SETUPS);
        let played = keep_played.map(|i| self.saved.remove(i));
        self.saved.truncate(if played.is_some() { KEEP_SETUPS - 1 } else { KEEP_SETUPS });
        self.saved.extend(played);
        self.sync_lists();
    }

    /// The current on/off and order, as a setup stores it.
    fn current_order(&self) -> Vec<(String, bool)> {
        self.mods.iter().map(|m| (m.id.clone(), m.enabled)).collect()
    }

    /// The game was started with the mods as they are now (the app calls this when Play goes through): the setup is
    /// marked as played, so "Restore last working setup" goes back to it.
    pub fn mark_played(&mut self) -> io::Result<()> {
        let order = self.current_order();
        let on = |o: &[(String, bool)]| o.iter().filter(|(_, e)| *e).cloned().collect::<Vec<_>>();
        let i = match self.saved.iter().position(|s| on(&s.order) == on(&order)) {
            Some(i) => i,
            None => {
                self.save_setup();
                0
            }
        };
        if !self.saved[i].setup.played {
            self.saved[i].setup.played = true;
            self.sync_lists();
            self.save()?;
        }
        Ok(())
    }

    /// The game closed right after starting with the mods as they are now (the check after Play): this setup no
    /// longer counts as one that worked, so "the setup that worked" is the one before it. Returns true when a
    /// setup was unmarked.
    pub fn mark_not_working(&mut self) -> io::Result<bool> {
        let on = |o: &[(String, bool)]| o.iter().filter(|(_, e)| *e).cloned().collect::<Vec<_>>();
        let now = on(&self.current_order());
        let Some(i) = self.saved.iter().position(|s| s.setup.played && on(&s.order) == now) else { return Ok(false) };
        self.saved[i].setup.played = false;
        self.sync_lists();
        self.save()?;
        Ok(true)
    }

    /// The turned-on mods of a saved setup, in its order (for "since it last worked, you added ...").
    pub fn setup_on(&self, setup_id: &str) -> Vec<String> {
        self.saved
            .iter()
            .find(|s| s.setup.id == setup_id)
            .map(|s| s.order.iter().filter(|(_, on)| *on).map(|(id, _)| id.clone()).collect())
            .unwrap_or_default()
    }

    /// Puts the turned-on mods in the game except `skip` (Play without the script extender: the loader and the mods
    /// that need it stay out for this one run). Nothing about the mods changes and no setup is saved; the next
    /// apply puts everything back.
    pub fn apply_except(&mut self, skip: &[String]) -> io::Result<Report> {
        let wanted = self.wanted_except(skip);
        self.deploy(wanted, None)
    }

    /// The mods that are the game's script loader itself (their files hold everything the loader is detected by).
    pub fn loader_mods(&self, rules: &GameRules) -> Vec<String> {
        let Some(l) = &rules.loader else { return Vec::new() };
        self.mods
            .iter()
            .filter(|m| {
                let files: Vec<String> = self.staged_files(&self.mod_dir(&m.id)).iter().map(|f| f.to_lowercase()).collect();
                l.detect.iter().all(|d| {
                    d.split('|').any(|choice| {
                        let c = choice.to_lowercase();
                        files.iter().any(|f| *f == c || f.starts_with(&format!("{c}/")))
                    })
                })
            })
            .map(|m| m.id.clone())
            .collect()
    }

    /// What "Restore last working setup" goes back to: the newest setup the game was played with.
    pub fn restore_target(&self) -> Option<&Setup> {
        self.setup_list.iter().find(|s| s.played)
    }

    /// Takes every mod file out of the game (nothing is deleted; mods stay on/off as they were).
    pub fn play_original(&mut self) -> io::Result<Report> {
        self.deploy(Vec::new(), None)
    }

    /// Goes back to a saved setup and applies it. Mods that are gone are skipped; mods added since are turned off and
    /// kept after the others.
    /// An empty `setup_id` means the newest played setup (`restore_target`).
    pub fn restore(&mut self, setup_id: &str) -> io::Result<Report> {
        let setup_id = if setup_id.is_empty() {
            self.restore_target()
                .map(|s| s.id.clone())
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "The game hasn't been played with mods yet, so there's no working setup to go back to."))?
        } else {
            setup_id.to_string()
        };
        let setup_id = setup_id.as_str();
        let s = self
            .saved
            .iter()
            .find(|s| s.setup.id == setup_id)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "That saved setup isn't there any more."))?;
        let order = s.order.clone();
        self.set_order(&order)?;
        self.apply()
    }

    /// The mods in order with their on/off state.
    pub fn order(&self) -> Vec<(String, bool)> {
        self.current_order()
    }

    /// Puts the mods in this order with this on/off state (profiles use it). Mods not listed are turned off and kept
    /// after the others; IDs that aren't here are skipped. The game changes at the next `apply`.
    pub fn set_order(&mut self, order: &[(String, bool)]) -> io::Result<()> {
        let mut mods = Vec::new();
        let mut archives = Vec::new();
        for (id, on) in order {
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
        self.save()
    }

    /// The game this store is for.
    pub fn game(&self) -> &Game {
        &self.game
    }

    /// The way back when something went wrong: takes out every mod file Turbine knows it placed and puts the game's
    /// own files back from the backup folder (see `deploy::reset_from_backups`). Mods stay listed, all turned off.
    pub fn reset_game(&mut self) -> io::Result<Report> {
        let report = deploy::reset_from_backups(&self.deploy_paths())?;
        for m in &mut self.mods {
            m.enabled = false;
        }
        self.save()?;
        Ok(report)
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
        let mods: Vec<String> = self
            .mods
            .iter()
            .map(|m| {
                let src = self.source_of(&m.id);
                format!("{{{},\"sourceKind\":{},\"sourceId\":{}}}", mod_json(m), quote(&src.kind), quote(&src.id))
            })
            .collect();
        let removed: Vec<String> = self
            .removed_list
            .iter()
            .map(|r| format!("{{\"id\":{},\"name\":{},\"removedAt\":{}}}", quote(&r.item.id), quote(&r.item.name), r.removed_at))
            .collect();
        let setups: Vec<String> = self
            .setup_list
            .iter()
            .map(|s| {
                let on: Vec<String> = self.setup_on(&s.id).iter().map(|id| quote(id)).collect();
                format!("{{\"id\":{},\"savedAt\":{},\"modsOn\":{},\"played\":{},\"on\":[{}]}}", quote(&s.id), s.saved_at, s.mods_on, s.played, on.join(","))
            })
            .collect();
        let mut placements: Vec<String> = self.placements.iter().map(|(id, p)| format!("{}:{}", quote(id), p.to_json())).collect();
        placements.sort();
        format!(
            "{{\"mods\":[{}],\"removed\":[{}],\"setups\":[{}],\"placements\":{{{}}},\"lastBuild\":{}}}",
            mods.join(","),
            removed.join(","),
            setups.join(","),
            placements.join(","),
            opt(&self.last_build)
        )
    }
}

/// Mod archives in Downloads and in each Downloads/Game Mods/<game> folder (where the browser helper saves Nexus
/// downloads), newest first, each with its Game Mods folder name ("" for Downloads itself and Game Mods itself).
/// Links are not followed.
pub fn downloads_all(home: &Path) -> Vec<(PathBuf, String)> {
    let dl = home.join("Downloads");
    let gm = dl.join(crate::watch::GAME_MODS);
    let mut dirs: Vec<(PathBuf, String)> = vec![(dl.clone(), String::new()), (gm.clone(), String::new())];
    if let Ok(rd) = fs::read_dir(&gm) {
        for e in rd.flatten() {
            if e.file_type().is_ok_and(|t| t.is_dir()) {
                dirs.push((e.path(), e.file_name().to_string_lossy().into_owned()));
            }
        }
    }
    let mut found: Vec<(SystemTime, PathBuf, String)> = Vec::new();
    for (d, folder) in dirs {
        let Ok(rd) = fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if !p.file_name().is_some_and(|n| archives::is_archive_name(&n.to_string_lossy())) {
                continue;
            }
            let Ok(m) = fs::symlink_metadata(&p) else { continue };
            if m.is_file() {
                found.push((m.modified().unwrap_or(UNIX_EPOCH), p, folder.clone()));
            }
        }
    }
    found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    found.into_iter().map(|(_, p, f)| (p, f)).collect()
}

/// Mod archives (zip, 7z, rar) in the Downloads folder, newest first.
pub fn downloads(home: &Path) -> Vec<PathBuf> {
    let Ok(rd) = fs::read_dir(home.join("Downloads")) else { return Vec::new() };
    let mut found: Vec<(SystemTime, PathBuf)> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().is_some_and(|n| archives::is_archive_name(&n.to_string_lossy())))
        .filter_map(|p| {
            let m = fs::metadata(&p).ok()?;
            m.is_file().then(|| (m.modified().unwrap_or(UNIX_EPOCH), p))
        })
        .collect();
    found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    found.into_iter().map(|(_, p)| p).collect()
}
