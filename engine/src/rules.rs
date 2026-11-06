//! What Turbine knows about each game: where mods go, how archives are laid out, which script loader it uses, the
//! checks before Play, and how to launch it.
//!
//! CONTRACT (other modules and the app rely on these signatures; keep them stable).
//!
//! The known games and loaders are plain data in the GAMES and LOADERS tables below. They only hold strings, numbers
//! and switches, so a later version can read the same shape from data files and add games without new code.

use crate::games::{Game, Store};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Bethesda games: mods go in Data, plugins have a load order.
    Bethesda,
    /// Unity games using BepInEx (Thunderstore games).
    BepInEx,
    /// Stardew Valley with SMAPI: each mod is a folder in Mods.
    Smapi,
    /// Anything else: files go where the archive says, from the game folder.
    Generic,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Loader {
    /// Plain name, e.g. "SKSE (Skyrim Script Extender)".
    pub name: String,
    /// One sentence on what it is, for the "needs a loader" prompt.
    pub explain: String,
    /// Files (relative to the game folder) whose presence means the loader is installed. All entries must be there;
    /// an entry may list choices separated by '|' (any one of them counts). Names are matched ignoring case.
    pub detect: Vec<String>,
    /// Where it comes from (official page), shown to the player.
    pub source: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameRules {
    pub kind: Kind,
    /// Folder (relative to the game folder) most mods go into; "" for the game folder itself.
    pub mod_root: String,
    /// The game has a plugin load order (shows the Load order tab).
    pub plugins: bool,
    /// Mod order matters (shows order numbers and Y to reorder).
    pub ordered: bool,
    pub loader: Option<Loader>,
    /// Shown in Game details: "Verified" or "Works generically".
    pub support: String,
    /// A shorter name for tight places such as the page header, when the store's name is long
    /// ("Skyrim Special Edition" for "The Elder Scrolls V: Skyrim Special Edition"). None: use the game's own name.
    pub short_name: Option<String>,
}

/// One line of the check before Play.
#[derive(Debug, Clone, PartialEq)]
pub struct Check {
    /// "ok", "warn" (Play anyway allowed) or "stop" (Play original or Wait only).
    pub level: String,
    pub title: String,
    pub detail: String,
}

// ---------------------------------------------------------------------------------------------------------------
// Data tables
// ---------------------------------------------------------------------------------------------------------------

/// One known game. Data only.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameDef {
    pub name: &'static str,
    /// A shorter name for the page header; None when `name` is already short.
    pub short_name: Option<&'static str>,
    /// Store IDs the game is known by (Steam app ID, GOG product ID, Heroic's Epic app name).
    pub ids: &'static [(Store, &'static str)],
    pub kind: Kind,
    pub mod_root: &'static str,
    pub plugins: bool,
    pub ordered: bool,
    /// Key into LOADERS.
    pub loader: Option<&'static str>,
    /// Where this game's build of the loader comes from, when it differs from the loader's own page.
    pub loader_source: Option<&'static str>,
}

/// One script loader. Data only.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoaderDef {
    pub key: &'static str,
    pub name: &'static str,
    pub explain: &'static str,
    pub detect: &'static [&'static str],
    pub source: &'static str,
    /// How it hooks into the game, for the install step in a later build.
    pub hook: &'static str,
}

pub const GAMES: &[GameDef] = &[
    GameDef {
        name: "Skyrim Special Edition",
        short_name: Some("Skyrim Special Edition"),
        // GOG sells it as Skyrim Anniversary Edition. The GOG product ID below is from memory and not verified yet.
        ids: &[(Store::Steam, "489830"), (Store::Gog, "1711230643")],
        kind: Kind::Bethesda,
        mod_root: "Data",
        plugins: true,
        ordered: true,
        loader: Some("skse"),
        loader_source: None,
    },
    GameDef {
        name: "Stardew Valley",
        short_name: None,
        ids: &[(Store::Steam, "413150"), (Store::Gog, "1453375253")],
        kind: Kind::Smapi,
        mod_root: "Mods",
        plugins: false,
        ordered: false,
        loader: Some("smapi"),
        loader_source: None,
    },
    GameDef {
        name: "Valheim",
        short_name: None,
        // Steam ships a native Linux build; its BepInEx pack starts through start_game_bepinex.sh there, and through
        // winhttp.dll under Proton. The pack holds both, so detection is the same.
        ids: &[(Store::Steam, "892970")],
        kind: Kind::BepInEx,
        mod_root: "BepInEx/plugins",
        plugins: false,
        ordered: false,
        loader: Some("bepinex"),
        loader_source: Some("https://thunderstore.io/c/valheim/p/denikson/BepInExPack_Valheim/"),
    },
    GameDef {
        name: "Lethal Company",
        short_name: None,
        ids: &[(Store::Steam, "1966720")],
        kind: Kind::BepInEx,
        mod_root: "BepInEx/plugins",
        plugins: false,
        ordered: false,
        loader: Some("bepinex"),
        loader_source: Some("https://thunderstore.io/c/lethal-company/p/BepInEx/BepInExPack/"),
    },
    GameDef {
        name: "Risk of Rain 2",
        short_name: None,
        ids: &[(Store::Steam, "632360")],
        kind: Kind::BepInEx,
        mod_root: "BepInEx/plugins",
        plugins: false,
        ordered: false,
        loader: Some("bepinex"),
        loader_source: Some("https://thunderstore.io/c/riskofrain2/p/bbepis/BepInExPack/"),
    },
];

pub const LOADERS: &[LoaderDef] = &[
    LoaderDef {
        key: "skse",
        name: "SKSE (Skyrim Script Extender)",
        explain: "SKSE lets Skyrim load mods that add new code, like SkyUI and many popular fixes.",
        detect: &["skse64_loader.exe"],
        source: "https://skse.silverlock.org/",
        // Steam starts SkyrimSELauncher.exe, not the loader, so a later build swaps the launcher for the loader
        // (keeping the original in the backup folder).
        hook: "launcher swap: SkyrimSELauncher.exe is replaced by skse64_loader.exe",
    },
    LoaderDef {
        key: "bepinex",
        name: "BepInEx",
        explain: "BepInEx lets Unity games load mods that add new code; most mods for this game need it.",
        detect: &["winhttp.dll", "BepInEx/core"],
        source: "https://github.com/BepInEx/BepInEx/releases",
        hook: "Windows builds: winhttp.dll override in the prefix; native builds: the pack's start script",
    },
    LoaderDef {
        key: "smapi",
        name: "SMAPI (Stardew Modding API)",
        explain: "SMAPI is the mod loader for Stardew Valley; mods in the Mods folder only work through it.",
        detect: &["StardewModdingAPI|StardewModdingAPI.exe"],
        source: "https://smapi.io/",
        hook: "own installer, then the game starts StardewModdingAPI instead of the game",
    },
];

/// The known game with this store ID, if any.
pub fn find_def(store: Store, store_id: &str) -> Option<&'static GameDef> {
    GAMES.iter().find(|d| d.ids.iter().any(|(s, id)| *s == store && *id == store_id))
}

pub fn find_loader(key: &str) -> Option<&'static LoaderDef> {
    LOADERS.iter().find(|l| l.key == key)
}

impl GameRules {
    /// Rules for a detected game (known games by store ID; everything else Generic).
    pub fn for_game(game: &Game) -> GameRules {
        match find_def(game.store, &game.store_id) {
            Some(def) => GameRules::from_def(def),
            None => GameRules {
                kind: Kind::Generic,
                mod_root: String::new(),
                plugins: false,
                ordered: true,
                loader: None,
                support: "Works generically".into(),
                short_name: None,
            },
        }
    }

    /// Rules from one entry of the game table.
    pub fn from_def(def: &GameDef) -> GameRules {
        let loader = def.loader.and_then(find_loader).map(|l| Loader {
            name: l.name.into(),
            explain: l.explain.into(),
            detect: l.detect.iter().map(|s| s.to_string()).collect(),
            source: def.loader_source.unwrap_or(l.source).into(),
        });
        GameRules {
            kind: def.kind,
            mod_root: def.mod_root.into(),
            plugins: def.plugins,
            ordered: def.ordered,
            loader,
            support: "Verified".into(),
            short_name: def.short_name.map(Into::into),
        }
    }

    /// Maps the files of a mod archive (paths inside the archive, '/' separated) to paths inside the game folder.
    /// Files that don't belong (readme, images at the top of a Thunderstore package) may be left out.
    /// `mod_name` is a folder-safe name for games that keep each mod in its own folder.
    pub fn map_archive(&self, files: &[String], mod_name: &str) -> Result<Vec<(String, String)>, String> {
        let entries = normalize(files)?;
        let name = safe_name(mod_name);
        let out = match self.kind {
            Kind::Bethesda => map_bethesda(&entries, &self.mod_root)?,
            Kind::BepInEx => map_bepinex(&entries, &name),
            Kind::Smapi => map_smapi(&entries, &self.mod_root, &name)?,
            Kind::Generic => map_generic(&entries, &self.mod_root),
        };
        if out.is_empty() {
            return Err("This archive has no files Turbine knows where to put.".into());
        }
        Ok(out)
    }

    /// Does this mod (its game-relative files) need the game's script loader?
    pub fn needs_loader(&self, game_files: &[String]) -> bool {
        game_files.iter().any(|f| {
            let p = f.replace('\\', "/").trim_start_matches('/').to_lowercase();
            match self.kind {
                Kind::Bethesda => p.starts_with("data/skse/plugins/") && p.ends_with(".dll"),
                Kind::BepInEx => {
                    p.starts_with("bepinex/plugins/") || p.starts_with("bepinex/patchers/") || p.starts_with("bepinex/monomod/")
                }
                Kind::Smapi => p.starts_with("mods/") && p.len() > "mods/".len(),
                Kind::Generic => false,
            }
        })
    }

    /// Is the loader installed in this game folder?
    pub fn loader_installed(&self, game: &Game) -> bool {
        match &self.loader {
            None => true,
            Some(l) => l
                .detect
                .iter()
                .all(|entry| entry.split('|').any(|choice| find_ci(&game.install_dir, choice).is_some())),
        }
    }
}

/// Finds a path under `base`, matching each part of `rel` ignoring case (games under Proton don't care about case,
/// archives and players do). Returns the path with the casing on disk.
pub fn find_ci(base: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = base.to_path_buf();
    for part in rel.split('/').filter(|p| !p.is_empty()) {
        let exact = cur.join(part);
        if exact.symlink_metadata().is_ok() {
            cur = exact;
            continue;
        }
        let found = std::fs::read_dir(&cur)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(part))?;
        cur = found.path();
    }
    Some(cur)
}

// ---------------------------------------------------------------------------------------------------------------
// Archive mapping
// ---------------------------------------------------------------------------------------------------------------

/// A file in the archive, split into its parts.
#[derive(Debug, Clone)]
struct Entry {
    /// The path as given (after cleanup), used as the source in the result.
    src: String,
    /// Path parts below any wrapper folders that were taken off.
    parts: Vec<String>,
}

impl Entry {
    fn top(&self) -> &str {
        &self.parts[0]
    }
    fn top_lc(&self) -> String {
        self.parts[0].to_lowercase()
    }
    fn is_top_file(&self) -> bool {
        self.parts.len() == 1
    }
    fn rest(&self, skip: usize) -> String {
        self.parts[skip..].join("/")
    }
    fn file_lc(&self) -> String {
        self.parts.last().map(|s| s.to_lowercase()).unwrap_or_default()
    }
}

/// Cleans up archive paths: '/' separators, no folder entries, no system junk, nothing that climbs out.
fn normalize(files: &[String]) -> Result<Vec<Entry>, String> {
    let mut out = Vec::new();
    for f in files {
        let clean = f.replace('\\', "/");
        if clean.ends_with('/') {
            continue; // a folder entry
        }
        let mut parts = Vec::new();
        for p in clean.split('/') {
            match p {
                "" | "." => {}
                ".." => return Err(format!("This archive has a file that points outside its folder: {f}")),
                _ => parts.push(p.to_string()),
            }
        }
        if parts.is_empty() {
            continue;
        }
        let lc: Vec<String> = parts.iter().map(|p| p.to_lowercase()).collect();
        if lc.iter().any(|p| p == "__macosx") || matches!(lc.last().map(String::as_str), Some(".ds_store" | "thumbs.db" | "desktop.ini")) {
            continue;
        }
        let src = parts.join("/");
        out.push(Entry { src, parts });
    }
    Ok(out)
}

fn safe_name(name: &str) -> String {
    let n: String = name
        .chars()
        .map(|c| if c == '/' || c == '\\' || c.is_control() { '_' } else { c })
        .collect();
    let n = n.trim().trim_matches('.').to_string();
    if n.is_empty() {
        "mod".into()
    } else {
        n
    }
}

fn join(root: &str, rest: &str) -> String {
    if root.is_empty() {
        rest.to_string()
    } else if rest.is_empty() {
        root.to_string()
    } else {
        format!("{root}/{rest}")
    }
}

/// When every file sits in one folder, and `keep` doesn't say that folder means something, takes it off.
fn unwrap_once(entries: &mut [Entry], keep: &dyn Fn(&str) -> bool) -> bool {
    let Some(first) = entries.first() else { return false };
    if entries.iter().any(|e| e.is_top_file()) {
        return false;
    }
    let top = first.top_lc();
    if keep(&top) || entries.iter().any(|e| e.top_lc() != top) {
        return false;
    }
    for e in entries.iter_mut() {
        e.parts.remove(0);
    }
    true
}

fn ext(name_lc: &str) -> &str {
    name_lc.rsplit_once('.').map(|(_, e)| e).unwrap_or("")
}

/// Readme, changelog and license files that come with mods but aren't part of them.
fn is_doc(name_lc: &str) -> bool {
    let stem = name_lc.rsplit_once('.').map(|(s, _)| s).unwrap_or(name_lc);
    let doc_words = ["readme", "read me", "changelog", "change log", "changes", "whatsnew", "license", "licence", "credits"];
    let doc_ext = matches!(ext(name_lc), "txt" | "md" | "pdf" | "rtf" | "htm" | "html" | "");
    (doc_ext && doc_words.iter().any(|w| stem.contains(w))) || ext(name_lc) == "url"
}

fn is_image(name_lc: &str) -> bool {
    matches!(ext(name_lc), "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp")
}

// ----- Bethesda ---------------------------------------------------------------------------------------------

/// Folders that live inside Data. Anything else that isn't clearly a game-folder item also goes to Data.
const DATA_FOLDERS: &[&str] = &[
    "meshes", "textures", "scripts", "interface", "sound", "music", "skse", "seq", "strings", "video", "shadersfx",
    "lodsettings", "grass", "materials", "dyndolod", "calientetools", "nemesis_engine", "mcm", "platform", "source",
    "netscriptframework", "skyproc patchers", "lod", "terrain", "trees", "facegen", "dialogueviews", "shaders",
    "distantlod", "particles", "fonts", "vis",
];

const DATA_EXTS: &[&str] = &["esp", "esm", "esl", "bsa", "ba2", "bsl"];

/// Things that sit next to the game's exe (ENB, ReShade, script extender).
fn is_game_folder_item(e: &Entry) -> bool {
    let lc = e.top_lc();
    if e.is_top_file() {
        matches!(ext(&lc), "exe" | "dll" | "asi") || lc.starts_with("enb") || lc.starts_with("d3d") || lc.starts_with("reshade")
    } else {
        matches!(lc.as_str(), "enbseries" | "enbcache" | "reshade-shaders" | "reshade-presets")
    }
}

fn is_data_item(e: &Entry) -> bool {
    let lc = e.top_lc();
    if e.is_top_file() {
        DATA_EXTS.contains(&ext(&lc))
    } else {
        DATA_FOLDERS.contains(&lc.as_str())
    }
}

fn map_bethesda(entries: &[Entry], data_root: &str) -> Result<Vec<(String, String)>, String> {
    let mut entries = entries.to_vec();
    // Take off wrapper folders ("SkyUI_5_2_SE/SkyUI/...") until something meaningful is at the top.
    let keep = |top: &str| top == "data" || top == "fomod" || DATA_FOLDERS.contains(&top);
    for _ in 0..4 {
        if !unwrap_once(&mut entries, &keep) {
            break;
        }
    }
    let has_fomod = entries.iter().any(|e| e.top_lc() == "fomod" && !e.is_top_file());
    let has_data_folder = entries.iter().any(|e| e.top_lc() == "data" && !e.is_top_file());
    let has_loader_exe = entries.iter().any(|e| e.is_top_file() && e.top_lc().ends_with("_loader.exe"));
    let has_top_binary = entries.iter().any(|e| e.is_top_file() && matches!(ext(&e.top_lc()), "exe" | "dll" | "asi"));

    let mut out = Vec::new();
    for e in &entries {
        let lc = e.top_lc();
        if lc == "fomod" || lc == "screenshots" || lc == "screenshot" {
            continue;
        }
        if e.is_top_file() && (is_doc(&lc) || is_image(&lc)) {
            continue;
        }
        if lc == "src" && !e.is_top_file() && has_loader_exe {
            continue; // the script extender's source code
        }
        if lc == "data" && !e.is_top_file() {
            out.push((e.src.clone(), join(data_root, &e.rest(1))));
        } else if is_game_folder_item(e) || ((has_data_folder || has_top_binary) && e.is_top_file() && ext(&lc) == "ini") {
            out.push((e.src.clone(), e.rest(0)));
        } else if has_fomod && !is_data_item(e) {
            // Option folders of an installer: chosen in the installer, not installed by default.
            continue;
        } else if has_data_folder && !e.is_top_file() && !is_data_item(e) {
            // Next to a Data folder, other folders belong to the game folder.
            out.push((e.src.clone(), e.rest(0)));
        } else {
            out.push((e.src.clone(), join(data_root, &e.rest(0))));
        }
    }
    if has_fomod && out.is_empty() {
        return Err("This mod has an installer with choices. Installers come in a later build.".into());
    }
    Ok(out)
}

// ----- BepInEx ----------------------------------------------------------------------------------------------

const BEPINEX_SUBFOLDERS: &[(&str, &str)] =
    &[("plugins", "plugins"), ("patchers", "patchers"), ("core", "core"), ("monomod", "monomod")];

/// Files at the top of a Thunderstore package that describe it and aren't part of the mod.
fn is_package_file(name_lc: &str) -> bool {
    matches!(name_lc, "manifest.json" | "icon.png" | "readme.md" | "changelog.md" | "license" | "license.md" | "license.txt")
        || is_doc(name_lc)
}

/// The top folder of a BepInEx pack: holds winhttp.dll, doorstop_config.ini and a BepInEx folder.
fn bepinex_pack_folder(entries: &[Entry]) -> Option<String> {
    let mut tops: Vec<String> = entries.iter().filter(|e| e.parts.len() > 1).map(|e| e.top_lc()).collect();
    tops.sort();
    tops.dedup();
    tops.into_iter().find(|t| {
        let inside = |name: &str| entries.iter().any(|e| e.parts.len() == 2 && e.top_lc() == *t && e.file_lc() == name);
        inside("winhttp.dll")
            && inside("doorstop_config.ini")
            && entries.iter().any(|e| e.parts.len() > 2 && e.top_lc() == *t && e.parts[1].eq_ignore_ascii_case("bepinex"))
    })
}

fn map_bepinex(entries: &[Entry], name: &str) -> Vec<(String, String)> {
    let mut entries = entries.to_vec();
    let keep = |top: &str| top == "bepinex" || top == "config" || BEPINEX_SUBFOLDERS.iter().any(|(k, _)| *k == top);
    if !entries.iter().any(|e| e.is_top_file() && e.file_lc() == "manifest.json") && bepinex_pack_folder(&entries).is_none() {
        unwrap_once(&mut entries, &keep);
    }

    // The loader itself, as a Thunderstore pack: its folder's contents go in the game folder.
    if let Some(pack) = bepinex_pack_folder(&entries) {
        return entries
            .iter()
            .filter(|e| e.parts.len() > 1 && e.top_lc() == pack)
            .filter(|e| !(e.parts.len() == 2 && is_doc(&e.file_lc())))
            .map(|e| (e.src.clone(), e.rest(1)))
            .collect();
    }
    // The loader as a plain release zip (winhttp.dll and BepInEx/core at the top): as it is.
    let top_winhttp = entries.iter().any(|e| e.is_top_file() && e.file_lc() == "winhttp.dll");
    let top_core = entries.iter().any(|e| e.parts.len() > 2 && e.top_lc() == "bepinex" && e.parts[1].eq_ignore_ascii_case("core"));
    if top_winhttp && top_core {
        return entries
            .iter()
            .filter(|e| !(e.is_top_file() && is_doc(&e.file_lc())))
            .map(|e| (e.src.clone(), e.rest(0)))
            .collect();
    }

    let plugins = format!("BepInEx/plugins/{name}");
    let mut out = Vec::new();
    for e in &entries {
        let lc = e.top_lc();
        if e.is_top_file() {
            if is_package_file(&lc) {
                continue;
            }
            let dest = if lc.ends_with(".mm.dll") {
                format!("BepInEx/monomod/{name}/{}", e.top())
            } else if ext(&lc) == "cfg" {
                format!("BepInEx/config/{}", e.top())
            } else {
                format!("{plugins}/{}", e.top())
            };
            out.push((e.src.clone(), dest));
        } else if lc == "bepinex" {
            out.push((e.src.clone(), format!("BepInEx/{}", e.rest(1))));
        } else if lc == "config" {
            out.push((e.src.clone(), format!("BepInEx/config/{}", e.rest(1))));
        } else if let Some((_, folder)) = BEPINEX_SUBFOLDERS.iter().find(|(k, _)| *k == lc) {
            out.push((e.src.clone(), format!("BepInEx/{folder}/{name}/{}", e.rest(1))));
        } else {
            out.push((e.src.clone(), format!("{plugins}/{}", e.rest(0))));
        }
    }
    out
}

// ----- SMAPI ------------------------------------------------------------------------------------------------

fn is_smapi_installer(entries: &[Entry]) -> bool {
    entries.iter().any(|e| {
        let f = e.file_lc();
        (f.starts_with("install on ") && matches!(ext(&f), "sh" | "bat" | "command"))
            || (e.parts.iter().any(|p| p.eq_ignore_ascii_case("internal")) && f.starts_with("smapi.installer"))
    })
}

fn map_smapi(entries: &[Entry], mods_root: &str, name: &str) -> Result<Vec<(String, String)>, String> {
    if is_smapi_installer(entries) {
        return Err("This is SMAPI's own installer. For now, install SMAPI by running its installer (see smapi.io); \
                    Turbine will do this in a later build."
            .into());
    }
    // Every folder holding a manifest.json is one mod; a manifest inside another mod's folder is just its content.
    let mut folders: Vec<Vec<String>> = entries
        .iter()
        .filter(|e| e.file_lc() == "manifest.json")
        .map(|e| e.parts[..e.parts.len() - 1].to_vec())
        .collect();
    folders.sort_by_key(|f| f.len());
    let mut mods: Vec<Vec<String>> = Vec::new();
    for f in folders {
        if !mods.iter().any(|m| f.len() >= m.len() && f[..m.len()].iter().zip(m).all(|(a, b)| a.eq_ignore_ascii_case(b))) {
            mods.push(f);
        }
    }
    if mods.is_empty() {
        return Err("This doesn't look like a Stardew Valley mod: there is no manifest.json in it.".into());
    }
    let mut out = Vec::new();
    for e in entries {
        let owner = mods.iter().find(|m| {
            e.parts.len() > m.len() && e.parts[..m.len()].iter().zip(m.iter()).all(|(a, b)| a.eq_ignore_ascii_case(b))
        });
        let Some(m) = owner else { continue }; // readme next to the mod folders
        let folder = m.last().cloned().unwrap_or_else(|| name.to_string());
        out.push((e.src.clone(), join(mods_root, &format!("{folder}/{}", e.rest(m.len())))));
    }
    Ok(out)
}

// ----- Generic ----------------------------------------------------------------------------------------------

fn map_generic(entries: &[Entry], root: &str) -> Vec<(String, String)> {
    let mut entries = entries.to_vec();
    unwrap_once(&mut entries, &|_| false);
    entries.iter().map(|e| (e.src.clone(), join(root, &e.rest(0)))).collect()
}

// ---------------------------------------------------------------------------------------------------------------
// Before Play
// ---------------------------------------------------------------------------------------------------------------

const LOW_SPACE: u64 = 1024 * 1024 * 1024;

/// The check before Play. `last_build` is the game build recorded at the last deploy; `needs_loader` is whether any
/// enabled mod needs the loader; `free_bytes` is free space on the game's drive.
pub fn pre_play(game: &Game, rules: &GameRules, last_build: Option<&str>, needs_loader: bool, free_bytes: Option<u64>) -> Vec<Check> {
    let mounts = std::fs::read_to_string("/proc/mounts").unwrap_or_default();
    let dir = game.install_dir.canonicalize().unwrap_or_else(|_| game.install_dir.clone());
    let fs = fs_type_for(&mounts, &dir.to_string_lossy());
    let loader_ok = rules.loader.is_none() || rules.loader_installed(game);
    pre_play_with(game, rules, last_build, needs_loader, free_bytes, fs.as_deref(), loader_ok)
}

/// The same check with the drive's file system type and loader state given, so it can be tested without a real
/// drive or game folder.
pub fn pre_play_with(
    game: &Game,
    rules: &GameRules,
    last_build: Option<&str>,
    needs_loader: bool,
    free_bytes: Option<u64>,
    fs_type: Option<&str>,
    loader_ok: bool,
) -> Vec<Check> {
    let check = |level: &str, title: String, detail: String| Check { level: level.into(), title, detail };
    let mut checks = Vec::new();

    if let (Some(now), Some(then)) = (game.build.as_deref(), last_build) {
        if now != then {
            let launcher = game.store.launcher();
            let title = format!("{launcher} updated this game");
            if rules.kind == Kind::Bethesda && needs_loader {
                let loader = rules.loader.as_ref().map(|l| l.name.as_str()).unwrap_or("The script loader");
                checks.push(check(
                    "stop",
                    title,
                    format!(
                        "{} changed since your mods were last set up. {loader} and the mods that use it usually stop \
                         working after a game update until their makers release new versions. Play original is safe: \
                         it starts the game without mods.",
                        game.name
                    ),
                ));
            } else {
                checks.push(check(
                    "warn",
                    title,
                    format!(
                        "{} changed since your mods were last set up. Some mods may not work until they are updated. \
                         Play original is safe: it starts the game without mods.",
                        game.name
                    ),
                ));
            }
        }
    }

    if needs_loader && !loader_ok {
        if let Some(l) = &rules.loader {
            let level = if matches!(rules.kind, Kind::Bethesda | Kind::Smapi) { "stop" } else { "warn" };
            checks.push(check(
                level,
                format!("{} is not installed", l.name),
                format!("Some of your mods need it. {} Get it from {}", l.explain, l.source),
            ));
        }
    }

    if let Some(free) = free_bytes {
        if free < LOW_SPACE {
            checks.push(check(
                "warn",
                "Low space on the game's drive".into(),
                format!("Only {} MB is free. The game may fail to save or update.", free / (1024 * 1024)),
            ));
        }
    }

    if fs_type.is_some_and(is_exfat_type) {
        checks.push(check(
            "warn",
            "Game is on an exFAT drive".into(),
            "exFAT drives can't link files, so Turbine copies mods there, which is slower and uses more space. Some \
             games and loaders also have trouble on exFAT. Formatting the card in Game Mode (Settings, Storage) fixes \
             this, but erases it."
                .into(),
        ));
    }

    if checks.is_empty() {
        checks.push(check("ok", "Ready to play".into(), "Everything checks out.".into()));
    }
    checks
}

fn is_exfat_type(t: &str) -> bool {
    let t = t.to_ascii_lowercase();
    t == "exfat" || t == "fuse.exfat" || t == "fuse.exfat-fuse"
}

/// Undoes the octal escapes /proc/mounts uses for spaces and tabs in mount points ("\040").
fn unescape_mount(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\'
            && i + 3 < b.len()
            && (b'0'..=b'3').contains(&b[i + 1])
            && b[i + 2..i + 4].iter().all(|c| (b'0'..=b'7').contains(c))
        {
            out.push((b[i + 1] - b'0') * 64 + (b[i + 2] - b'0') * 8 + (b[i + 3] - b'0'));
            i += 4;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The file system type of the mount holding `path`, from the text of /proc/mounts (the longest matching mount
/// point wins, matched by whole folder names).
pub fn fs_type_for(mounts: &str, path: &str) -> Option<String> {
    let mut best: Option<(usize, String)> = None;
    for line in mounts.lines() {
        let mut f = line.split_whitespace();
        let (Some(_dev), Some(point), Some(kind)) = (f.next(), f.next(), f.next()) else { continue };
        let point = unescape_mount(point);
        let inside = point == "/"
            || path == point
            || (path.starts_with(&point) && path.as_bytes().get(point.trim_end_matches('/').len()) == Some(&b'/'));
        if inside && best.as_ref().is_none_or(|(len, _)| point.len() >= *len) {
            best = Some((point.len(), kind.to_string()));
        }
    }
    best.map(|(_, k)| k)
}

/// Is the folder on an exFAT drive, going by the text of /proc/mounts?
pub fn is_exfat(mounts: &str, path: &str) -> bool {
    fs_type_for(mounts, path).is_some_and(|t| is_exfat_type(&t))
}

// ---------------------------------------------------------------------------------------------------------------
// Launching
// ---------------------------------------------------------------------------------------------------------------

/// Heroic's runner name for a store, as its launch link expects.
pub fn heroic_runner(store: Store) -> Option<&'static str> {
    match store {
        Store::Epic => Some("legendary"),
        Store::Gog => Some("gog"),
        Store::Sideload => Some("sideload"),
        Store::Steam => None,
    }
}

fn percent_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// The URL that starts the game through its launcher (steam://rungameid/..., Heroic's own link).
///
/// Heroic's link, from Heroic Games Launcher's src/backend/protocol.ts (main branch, commit 4737fdb, Nov 2026):
/// lines 81 to 84 read the "new-style params URL" `heroic://launch?appName=<app name>&runner=<runner>` from the query
/// (line 15 lists the runners: legendary, gog, nile, sideload), and src/backend/shortcuts/shortcuts/shortcuts.ts
/// line 48 makes the same link for Heroic's own desktop shortcuts. The app name is the store ID Heroic uses (Epic app
/// name, GOG product ID, sideload ID), which is what Turbine stores in Game::store_id.
pub fn launch_url(game: &Game) -> Option<String> {
    if game.store_id.is_empty() {
        return None;
    }
    match heroic_runner(game.store) {
        None => Some(format!("steam://rungameid/{}", percent_encode(&game.store_id))),
        Some(runner) => Some(format!("heroic://launch?appName={}&runner={runner}", percent_encode(&game.store_id))),
    }
}
