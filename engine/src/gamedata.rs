//! Game and loader rules as data files (schema version 1).
//!
//! Built in: engine/data/games/*.json and engine/data/loaders/*.json, made part of the engine by build.rs (a new
//! game is a new file, no code). The player's own: <TURBINE_DATA>/games/*.json, read at start with the same checks.
//! A player's file may add a game that has no built-in file; it never replaces a built-in one, never sets a code
//! handler and is always shown as "Added by you". A file that fails a check is left out with one line saying why.
//!
//! Keys are camelCase; an unknown key is an error (it catches typos). See `parse_game` for the keys and defaults.

use crate::games::Store;
use crate::json::{self, Json};
use std::collections::HashMap;
use std::path::Path;

include!(concat!(env!("OUT_DIR"), "/builtin.rs"));

/// The only schema version this Turbine reads.
pub const SCHEMA: u32 = 1;

/// One placement hint for a game without full rules (checked before the built-in detection rules).
#[derive(Debug, Clone, PartialEq)]
pub struct HintFile {
    /// "ext", "folder" or "file".
    pub matches: String,
    pub values: Vec<String>,
    /// Game-relative folder the files go into.
    pub to: String,
}

/// A good first mod for a game (Get mods' "Good first mods" row). Each press is still the player's own download.
#[derive(Debug, Clone, PartialEq)]
pub struct StarterFile {
    /// "nexus" (a mod ID) or "thunderstore" (Owner-Name).
    pub source: String,
    pub id: String,
    pub name: String,
    /// One line on what it does.
    pub what: String,
}

/// Where a game keeps its saves, for the save backups before Play.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SavesFile {
    /// Under Proton: relative to the prefix's user folder (pfx/drive_c/users/steamuser).
    pub proton: Vec<String>,
    /// Native Linux builds: relative to the home folder.
    pub native: Vec<String>,
    /// Inside the game folder itself.
    pub game: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AntiCheatFile {
    pub name: String,
    /// "warn" or "stop".
    pub level: String,
    pub note: String,
}

/// One game's data file.
#[derive(Debug, Clone, PartialEq)]
pub struct GameFile {
    /// The file it came from (for problem lines).
    pub file: String,
    pub builtin: bool,
    pub name: String,
    pub short_name: Option<String>,
    pub ids: Vec<(Store, String)>,
    /// "full" or "basic" (a player's file is always shown as "Added by you").
    pub support: String,
    /// "bethesda", "bepinex", "smapi" or "generic".
    pub kind: String,
    /// null, "cyberpunk", "witcher3", "kcd" or "kcd2".
    pub handler: Option<String>,
    pub mod_root: String,
    pub plugins: bool,
    pub ordered: bool,
    pub loader: Option<String>,
    pub loader_source: Option<String>,
    pub native_launch: Option<(String, String)>,
    pub nexus: Option<String>,
    pub thunderstore: Option<String>,
    pub anti_cheat: Option<AntiCheatFile>,
    pub layout: Vec<HintFile>,
    pub starters: Vec<StarterFile>,
    pub saves: SavesFile,
    pub notes: Vec<String>,
}

/// One script loader's data file.
#[derive(Debug, Clone, PartialEq)]
pub struct LoaderFile {
    pub file: String,
    pub key: String,
    pub name: String,
    pub explain: String,
    pub detect: Vec<String>,
    pub source: String,
    pub hook: String,
    pub launcher_swap: Option<(String, String)>,
}

fn no_dashes(what: &str, s: &str) -> Result<(), String> {
    if s.contains('\u{2014}') || s.contains('\u{2013}') {
        return Err(format!("{what} has an em or en dash"));
    }
    Ok(())
}

/// A game-relative path in a data file: plain, relative, nothing that climbs out.
pub fn safe_path(s: &str) -> bool {
    !s.contains("..") && !s.contains('\\') && crate::zip::safe_relative(s).is_ok() && !s.starts_with('/')
}

fn check_keys(v: &Json, allowed: &[&str], what: &str) -> Result<(), String> {
    match v {
        Json::Obj(items) => {
            for (k, _) in items {
                if !allowed.contains(&k.as_str()) {
                    return Err(format!("{what} has an unknown key \"{k}\""));
                }
            }
            Ok(())
        }
        _ => Err(format!("{what} isn't an object")),
    }
}

fn opt_str(v: &Json, key: &str) -> Result<Option<String>, String> {
    match v.get(key) {
        None | Some(Json::Null) => Ok(None),
        Some(Json::Str(s)) => {
            no_dashes(key, s)?;
            Ok(Some(s.clone()))
        }
        Some(_) => Err(format!("\"{key}\" must be text or null")),
    }
}

fn req_str(v: &Json, key: &str) -> Result<String, String> {
    match opt_str(v, key)? {
        Some(s) if !s.trim().is_empty() => Ok(s),
        _ => Err(format!("\"{key}\" is missing or empty")),
    }
}

fn opt_bool(v: &Json, key: &str, default: bool) -> Result<bool, String> {
    match v.get(key) {
        None => Ok(default),
        Some(Json::Bool(b)) => Ok(*b),
        Some(_) => Err(format!("\"{key}\" must be true or false")),
    }
}

fn str_list(v: Option<&Json>, key: &str) -> Result<Vec<String>, String> {
    match v {
        None | Some(Json::Null) => Ok(Vec::new()),
        Some(Json::Arr(items)) => items
            .iter()
            .map(|i| match i {
                Json::Str(s) => {
                    no_dashes(key, s)?;
                    Ok(s.clone())
                }
                _ => Err(format!("\"{key}\" must be a list of text")),
            })
            .collect(),
        Some(_) => Err(format!("\"{key}\" must be a list")),
    }
}

/// Reads one game file. `builtin`: it comes with Turbine (may say "full" and name a handler).
///
/// Keys (defaults in brackets): schema (1, required), name (required), shortName [null], ids (required, at least
/// one; steam, gog, epic, sideload: lists of text; Steam and GOG digits only), support ("full" or "basic")
/// ["basic"], kind ("bethesda", "bepinex", "smapi", "generic") ["generic"], handler (null, "cyberpunk", "witcher3",
/// "kcd", "kcd2") [null], modRoot [""], plugins [false], ordered [true], loader ({"use", "source"}) [null],
/// nativeLaunch ({"option", "help"}) [null], nexus [null], thunderstore [null], antiCheat ({"name", "level",
/// "note"}) [null], layout ([{"match", "values", "to"}]) [[]], starters ([{"source", "id", "name", "what"}]) [[]],
/// saves ({"proton", "native", "game"}: lists of paths) [none], notes (list of text, never shown) [[]].
pub fn parse_game(file: &str, text: &str, builtin: bool) -> Result<GameFile, String> {
    let v = json::parse(text).map_err(|e| format!("isn't valid JSON ({} at byte {})", e.message, e.offset))?;
    check_keys(
        &v,
        &[
            "schema", "name", "shortName", "ids", "support", "kind", "handler", "modRoot", "plugins", "ordered", "loader",
            "nativeLaunch", "nexus", "thunderstore", "antiCheat", "layout", "starters", "saves", "notes",
        ],
        "the file",
    )?;
    match v.get("schema") {
        Some(Json::Num(n)) if *n == SCHEMA as f64 => {}
        Some(Json::Num(n)) => return Err(format!("is schema {n}; this Turbine reads schema {SCHEMA}")),
        _ => return Err("\"schema\" is missing".into()),
    }
    let name = req_str(&v, "name")?;
    let short_name = opt_str(&v, "shortName")?;
    let ids_v = v.get("ids").ok_or("\"ids\" is missing")?;
    check_keys(ids_v, &["steam", "gog", "epic", "sideload"], "\"ids\"")?;
    let mut ids = Vec::new();
    for (key, store) in [("steam", Store::Steam), ("gog", Store::Gog), ("epic", Store::Epic), ("sideload", Store::Sideload)] {
        for id in str_list(ids_v.get(key), key)? {
            if id.trim().is_empty() {
                return Err(format!("an ID under \"{key}\" is empty"));
            }
            if matches!(store, Store::Steam | Store::Gog) && !id.bytes().all(|b| b.is_ascii_digit()) {
                return Err(format!("{key} ID \"{id}\" isn't digits only"));
            }
            ids.push((store, id));
        }
    }
    if ids.is_empty() {
        return Err("\"ids\" has no IDs".into());
    }
    let support = opt_str(&v, "support")?.unwrap_or_else(|| "basic".into());
    if support != "full" && support != "basic" {
        return Err(format!("\"support\" is \"{support}\"; it can be \"full\" or \"basic\""));
    }
    if support == "full" && !builtin {
        return Err("only Turbine's own files can say \"support\": \"full\"".into());
    }
    let kind = opt_str(&v, "kind")?.unwrap_or_else(|| "generic".into());
    if !["bethesda", "bepinex", "smapi", "generic"].contains(&kind.as_str()) {
        return Err(format!("\"kind\" is \"{kind}\"; it can be bethesda, bepinex, smapi or generic"));
    }
    let handler = opt_str(&v, "handler")?;
    if let Some(h) = &handler {
        if !builtin {
            return Err("only Turbine's own files can name a \"handler\"".into());
        }
        if !["cyberpunk", "witcher3", "kcd", "kcd2"].contains(&h.as_str()) {
            return Err(format!("\"handler\" is \"{h}\"; it can be cyberpunk, witcher3, kcd or kcd2"));
        }
    }
    let mod_root = opt_str(&v, "modRoot")?.unwrap_or_default();
    if !mod_root.is_empty() && !safe_path(&mod_root) {
        return Err(format!("\"modRoot\" \"{mod_root}\" isn't a plain path inside the game folder"));
    }
    let (loader, loader_source) = match v.get("loader") {
        None | Some(Json::Null) => (None, None),
        Some(l) => {
            check_keys(l, &["use", "source"], "\"loader\"")?;
            (Some(req_str(l, "use")?), opt_str(l, "source")?)
        }
    };
    if let Some(src) = &loader_source {
        if !src.starts_with("https://") {
            return Err("\"loader\".\"source\" must be an https:// address".into());
        }
    }
    let native_launch = match v.get("nativeLaunch") {
        None | Some(Json::Null) => None,
        Some(n) => {
            check_keys(n, &["option", "help"], "\"nativeLaunch\"")?;
            Some((req_str(n, "option")?, req_str(n, "help")?))
        }
    };
    let nexus = opt_str(&v, "nexus")?;
    if let Some(d) = &nexus {
        if d.is_empty() || !d.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()) {
            return Err(format!("\"nexus\" \"{d}\" isn't a Nexus game domain (lowercase letters and digits)"));
        }
    }
    let thunderstore = opt_str(&v, "thunderstore")?;
    if let Some(c) = &thunderstore {
        if c.is_empty() || !c.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-') {
            return Err(format!("\"thunderstore\" \"{c}\" isn't a Thunderstore community name"));
        }
    }
    let anti_cheat = match v.get("antiCheat") {
        None | Some(Json::Null) => None,
        Some(a) => {
            check_keys(a, &["name", "level", "note"], "\"antiCheat\"")?;
            let level = opt_str(a, "level")?.unwrap_or_else(|| "warn".into());
            if level != "warn" && level != "stop" {
                return Err("\"antiCheat\".\"level\" can be \"warn\" or \"stop\"".into());
            }
            Some(AntiCheatFile { name: req_str(a, "name")?, level, note: opt_str(a, "note")?.unwrap_or_default() })
        }
    };
    let mut layout = Vec::new();
    for h in v.get("layout").map(Json::as_array).unwrap_or(&[]) {
        check_keys(h, &["match", "values", "to"], "a \"layout\" hint")?;
        let matches = req_str(h, "match")?;
        if !["ext", "folder", "file"].contains(&matches.as_str()) {
            return Err(format!("a \"layout\" hint's \"match\" is \"{matches}\"; it can be ext, folder or file"));
        }
        let values = str_list(h.get("values"), "values")?;
        if values.is_empty() {
            return Err("a \"layout\" hint has no \"values\"".into());
        }
        let to = opt_str(h, "to")?.unwrap_or_default();
        if matches == "ext" && to.is_empty() {
            return Err("an \"ext\" hint needs a \"to\" folder".into());
        }
        if !to.is_empty() && !safe_path(&to) {
            return Err(format!("a \"layout\" hint's \"to\" \"{to}\" isn't a plain path inside the game folder"));
        }
        layout.push(HintFile { matches, values: values.into_iter().map(|s| s.to_lowercase()).collect(), to });
    }
    let mut starters = Vec::new();
    for s in v.get("starters").map(Json::as_array).unwrap_or(&[]) {
        check_keys(s, &["source", "id", "name", "what"], "a \"starters\" entry")?;
        let source = req_str(s, "source")?;
        let id = req_str(s, "id")?;
        match source.as_str() {
            "nexus" if id.bytes().all(|b| b.is_ascii_digit()) => {}
            "thunderstore" if id.split_once('-').is_some_and(|(o, n)| !o.is_empty() && !n.is_empty()) => {}
            _ => return Err(format!("starter \"{id}\" isn't a Nexus mod number or a Thunderstore Owner-Name")),
        }
        starters.push(StarterFile { source, id, name: req_str(s, "name")?, what: req_str(s, "what")? });
    }
    let saves = match v.get("saves") {
        None | Some(Json::Null) => SavesFile::default(),
        Some(s) => {
            check_keys(s, &["proton", "native", "game"], "\"saves\"")?;
            let f = SavesFile {
                proton: str_list(s.get("proton"), "proton")?,
                native: str_list(s.get("native"), "native")?,
                game: str_list(s.get("game"), "game")?,
            };
            if let Some(bad) = f.proton.iter().chain(&f.native).chain(&f.game).find(|p| !safe_path(p)) {
                return Err(format!("save folder \"{bad}\" isn't a plain relative path"));
            }
            f
        }
    };
    let notes = str_list(v.get("notes"), "notes")?;
    Ok(GameFile {
        file: file.to_string(),
        builtin,
        name,
        short_name,
        ids,
        support,
        kind,
        handler,
        mod_root,
        plugins: opt_bool(&v, "plugins", false)?,
        ordered: opt_bool(&v, "ordered", true)?,
        loader,
        loader_source,
        native_launch,
        nexus,
        thunderstore,
        anti_cheat,
        layout,
        starters,
        saves,
        notes,
    })
}

/// Reads one loader file.
pub fn parse_loader(file: &str, text: &str) -> Result<LoaderFile, String> {
    let v = json::parse(text).map_err(|e| format!("isn't valid JSON ({} at byte {})", e.message, e.offset))?;
    check_keys(&v, &["key", "name", "explain", "detect", "source", "hook", "launcherSwap", "notes"], "the file")?;
    let detect = str_list(v.get("detect"), "detect")?;
    if detect.is_empty() {
        return Err("\"detect\" is empty".into());
    }
    for d in &detect {
        if d.split('|').any(|c| !safe_path(c)) {
            return Err(format!("\"detect\" entry \"{d}\" isn't a plain path"));
        }
    }
    let launcher_swap = match v.get("launcherSwap") {
        None | Some(Json::Null) => None,
        Some(a) => {
            let l = str_list(Some(a), "launcherSwap")?;
            if l.len() != 2 || l.iter().any(|p| !safe_path(p)) {
                return Err("\"launcherSwap\" must be two plain file names".into());
            }
            Some((l[0].clone(), l[1].clone()))
        }
    };
    let source = req_str(&v, "source")?;
    if !source.starts_with("https://") {
        return Err("\"source\" must be an https:// address".into());
    }
    str_list(v.get("notes"), "notes")?;
    Ok(LoaderFile {
        file: file.to_string(),
        key: req_str(&v, "key")?,
        name: req_str(&v, "name")?,
        explain: req_str(&v, "explain")?,
        detect,
        source,
        hook: req_str(&v, "hook")?,
        launcher_swap,
    })
}

/// Checks across files: IDs unique per store, every loader named exists. Returns the problems (file: why).
pub fn validate(games: &[GameFile], loaders: &[LoaderFile]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut seen: HashMap<(Store, String), &str> = HashMap::new();
    for g in games {
        for (store, id) in &g.ids {
            if let Some(other) = seen.insert((*store, id.clone()), &g.file) {
                problems.push(format!("{}: {} ID {id} is also in {other}", g.file, store.key()));
            }
        }
        if let Some(l) = &g.loader {
            if !loaders.iter().any(|x| &x.key == l) {
                problems.push(format!("{}: loader \"{l}\" has no file in data/loaders", g.file));
            }
        }
    }
    let mut keys: Vec<&str> = loaders.iter().map(|l| l.key.as_str()).collect();
    keys.sort();
    if keys.windows(2).any(|w| w[0] == w[1]) {
        problems.push("two loader files have the same key".into());
    }
    problems
}

/// Turbine's own data files, read once. Problems (a broken built-in file) are listed instead of stopping Turbine;
/// the build's tests make sure there are none.
pub struct Builtin {
    pub games: Vec<GameFile>,
    pub loaders: Vec<LoaderFile>,
    pub problems: Vec<String>,
}

pub fn builtin() -> &'static Builtin {
    static B: std::sync::OnceLock<Builtin> = std::sync::OnceLock::new();
    B.get_or_init(|| {
        let mut problems = Vec::new();
        let mut loaders = Vec::new();
        for (file, text) in BUILTIN_LOADERS {
            match parse_loader(file, text) {
                Ok(l) => loaders.push(l),
                Err(e) => problems.push(format!("{file}: {e}")),
            }
        }
        let mut games = Vec::new();
        for (file, text) in BUILTIN_GAMES {
            match parse_game(file, text, true) {
                Ok(g) => games.push(g),
                Err(e) => problems.push(format!("{file}: {e}")),
            }
        }
        problems.extend(validate(&games, &loaders));
        Builtin { games, loaders, problems }
    })
}

/// The built-in files as (file name, text), for the build's checks.
/// Data files as (file name, text).
pub type Sources = &'static [(&'static str, &'static str)];

pub fn builtin_sources() -> (Sources, Sources) {
    (BUILTIN_GAMES, BUILTIN_LOADERS)
}

/// Reads the player's own game files from `dir` (missing folder: none). A file is left out when it fails a check,
/// names a store ID a built-in file has, or repeats another player file's ID; each such file gets a problem line.
pub fn load_player_dir(dir: &Path) -> (Vec<GameFile>, Vec<String>) {
    let mut out: Vec<GameFile> = Vec::new();
    let mut problems = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else { return (out, problems) };
    let mut files: Vec<_> = rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "json")).collect();
    files.sort();
    let b = builtin();
    for p in files {
        let name = p.file_name().unwrap_or_default().to_string_lossy().into_owned();
        let text = match std::fs::metadata(&p) {
            Ok(m) if m.len() > 256 * 1024 => {
                problems.push(format!("{name}: left out: the file is over 256 KB"));
                continue;
            }
            _ => match std::fs::read_to_string(&p) {
                Ok(t) => t,
                Err(e) => {
                    problems.push(format!("{name}: left out: couldn't read it ({e})"));
                    continue;
                }
            },
        };
        let g = match parse_game(&name, &text, false) {
            Ok(g) => g,
            Err(e) => {
                problems.push(format!("{name}: left out: {e}"));
                continue;
            }
        };
        if let Some((s, id)) = g.ids.iter().find(|(s, id)| b.games.iter().any(|x| x.ids.iter().any(|(s2, id2)| s2 == s && id2 == id))) {
            problems.push(format!("{name}: left out: Turbine already has rules for {} ID {id}; your file can't replace them", s.key()));
            continue;
        }
        if let Some((s, id)) = g.ids.iter().find(|(s, id)| out.iter().any(|x| x.ids.iter().any(|(s2, id2)| s2 == s && id2 == id))) {
            problems.push(format!("{name}: left out: {} ID {id} is already in another of your files", s.key()));
            continue;
        }
        if let Some(l) = &g.loader {
            if !b.loaders.iter().any(|x| &x.key == l) {
                problems.push(format!("{name}: left out: there is no loader called \"{l}\""));
                continue;
            }
        }
        out.push(g);
    }
    (out, problems)
}
