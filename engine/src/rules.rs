//! What Turbine knows about each game: where mods go, how archives are laid out, which script loader it uses, the
//! checks before Play, and how to launch it.
//!
//! CONTRACT (other modules and the app rely on these signatures; keep them stable).
//!
//! The known games and loaders are plain data in the GAMES and LOADERS tables below. They only hold strings, numbers
//! and switches, so a later version can read the same shape from data files and add games without new code.

use crate::games::{Game, Runtime, Store};
use crate::plugins::{self, MissingMaster};
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

/// Games with rules of their own beyond their Kind (where archives go, checks before Play). Kept apart from Kind so
/// code that matches on Kind keeps working: these games are Kind::Generic there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Family {
    #[default]
    Standard,
    /// Cyberpunk 2077: archive/pc/mod, REDmod, Cyber Engine Tweaks, RED4ext, redscript, TweakXL, ArchiveXL.
    Cyberpunk,
    /// The Witcher 3 (next-gen): mods/modXXX, dlc/, menu files, mods.settings priorities.
    Witcher3,
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
    /// How the detected copy of the game runs (Unknown for rules made from the table alone).
    pub runtime: Runtime,
    /// The loader's file and the game file it is also installed as, so the launcher's Play starts the loader
    /// (Skyrim on Steam: skse64_loader.exe also goes in as SkyrimSELauncher.exe; deploy keeps the original launcher
    /// in its backup folder and puts it back when the loader goes).
    pub launcher_swap: Option<(String, String)>,
    /// Text the player types once into the game's Launch Options in Steam for the loader to start, exactly as shown.
    pub launch_option: Option<String>,
    /// Plain words on why and how, shown with `launch_option` (or alone when there's nothing to type).
    pub launch_help: Option<String>,
    /// Games with rules of their own (Cyberpunk 2077, The Witcher 3); Standard for the rest.
    pub family: Family,
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
    /// For the game's native Linux build: the Launch Option its loader needs, and why.
    pub native_launch: Option<(&'static str, &'static str)>,
    pub family: Family,
}

/// One script loader. Data only.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoaderDef {
    pub key: &'static str,
    pub name: &'static str,
    pub explain: &'static str,
    pub detect: &'static [&'static str],
    pub source: &'static str,
    /// How it hooks into the game, in words.
    pub hook: &'static str,
    /// (loader file, launcher file): the loader is also installed under the launcher's name (Steam only).
    pub launcher_swap: Option<(&'static str, &'static str)>,
}

pub const GAMES: &[GameDef] = &[
    GameDef {
        name: "Skyrim Special Edition",
        short_name: Some("Skyrim Special Edition"),
        // GOG sells it as Skyrim Anniversary Edition: product 1711230643 is "The Elder Scrolls V: Skyrim Special
        // Edition", type Game, with Windows builds (gogdb.org/product/1711230643, checked Nov 6 2026).
        ids: &[(Store::Steam, "489830"), (Store::Gog, "1711230643")],
        kind: Kind::Bethesda,
        mod_root: "Data",
        plugins: true,
        ordered: true,
        loader: Some("skse"),
        loader_source: None,
        native_launch: None,
        family: Family::Standard,
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
        native_launch: None,
        family: Family::Standard,
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
        // The pack's start_game_bepinex.sh starts the game with BepInEx (Doorstop) loaded; Steam only runs it when the
        // Launch Option points at it.
        native_launch: Some((
            "./start_game_bepinex.sh %command%",
            "Valheim runs its Linux version here, and BepInEx (the mod loader) starts through a script that comes with \
             it. Steam only runs that script when you ask it to: in Steam, open Valheim's Properties, and under Launch \
             Options type exactly the line below. You only do this once. Without it, the game starts without mods.",
        )),
        family: Family::Standard,
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
        native_launch: None,
        family: Family::Standard,
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
        native_launch: None,
        family: Family::Standard,
    },
    GameDef {
        name: "Cyberpunk 2077",
        short_name: None,
        // Steam 1091500 (store.steampowered.com/app/1091500, "Cyberpunk 2077"); GOG 1423049311 (gogdb.org, "Cyberpunk
        // 2077", type Game). Both checked Nov 6 2026.
        ids: &[(Store::Steam, "1091500"), (Store::Gog, "1423049311")],
        kind: Kind::Generic,
        mod_root: "",
        plugins: false,
        ordered: true,
        // Several frameworks instead of one loader: see cyberpunk::FRAMEWORKS and mod_checks.
        loader: None,
        loader_source: None,
        native_launch: None,
        family: Family::Cyberpunk,
    },
    GameDef {
        name: "The Witcher 3",
        short_name: Some("The Witcher 3"),
        // Steam 292030 (store.steampowered.com/app/292030, "The Witcher 3: Wild Hunt"). On GOG, 1207664663 is the
        // Complete Edition package (gogdb.org, type Package, no builds); the installable games are 1207664643 ("The
        // Witcher 3: Wild Hunt", type Game, the one that package holds) and 1495134320 (the Game of the Year
        // Edition, type Game). Checked Nov 6 2026.
        ids: &[(Store::Steam, "292030"), (Store::Gog, "1207664643"), (Store::Gog, "1495134320")],
        kind: Kind::Generic,
        mod_root: "",
        plugins: false,
        ordered: true,
        loader: None,
        loader_source: None,
        native_launch: None,
        family: Family::Witcher3,
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
        launcher_swap: Some(("skse64_loader.exe", "SkyrimSELauncher.exe")),
    },
    LoaderDef {
        key: "bepinex",
        name: "BepInEx",
        explain: "BepInEx lets Unity games load mods that add new code; most mods for this game need it.",
        detect: &["winhttp.dll", "BepInEx/core"],
        source: "https://github.com/BepInEx/BepInEx/releases",
        hook: "Windows builds: winhttp.dll override in the prefix; native builds: the pack's start script",
        launcher_swap: None,
    },
    LoaderDef {
        key: "smapi",
        name: "SMAPI (Stardew Modding API)",
        explain: "SMAPI is the mod loader for Stardew Valley; mods in the Mods folder only work through it.",
        detect: &["StardewModdingAPI|StardewModdingAPI.exe"],
        source: "https://smapi.io/",
        hook: "installed from the installer's own bundle (never run); its launcher takes the game's launcher's place",
        launcher_swap: None,
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
            Some(def) => {
                let mut r = GameRules::from_def(def);
                r.runtime = game.runtime;
                // Only Steam's Play starts SkyrimSELauncher.exe; other stores' builds start differently.
                if game.store != Store::Steam {
                    r.launcher_swap = None;
                }
                let native = game.runtime != Runtime::Windows;
                if !native || !matches!(game.store, Store::Steam) {
                    r.launch_option = None;
                    r.launch_help = None;
                }
                if r.kind == Kind::Smapi && game.runtime == Runtime::Windows {
                    r.launch_help = Some(
                        "This copy of Stardew Valley is the Windows version. Turbine sets SMAPI up from its Windows \
                         files, but Steam still starts the game itself: follow SMAPI's own guide at smapi.io/install \
                         for the Launch Option that starts StardewModdingAPI.exe instead."
                            .into(),
                    );
                }
                r
            }
            None => GameRules {
                kind: Kind::Generic,
                mod_root: String::new(),
                plugins: false,
                ordered: true,
                loader: None,
                support: "Works generically".into(),
                short_name: None,
                runtime: game.runtime,
                launcher_swap: None,
                launch_option: None,
                launch_help: None,
                family: Family::Standard,
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
            runtime: Runtime::Unknown,
            launcher_swap: def
                .loader
                .and_then(find_loader)
                .and_then(|l| l.launcher_swap)
                .map(|(a, b)| (a.to_string(), b.to_string())),
            launch_option: def.native_launch.map(|(o, _)| o.to_string()),
            launch_help: def.native_launch.map(|(_, h)| h.to_string()),
            family: def.family,
        }
    }

    /// Maps the files of a mod archive (paths inside the archive, '/' separated) to paths inside the game folder.
    /// Files that don't belong (readme, images at the top of a Thunderstore package) may be left out.
    /// `mod_name` is a folder-safe name for games that keep each mod in its own folder.
    pub fn map_archive(&self, files: &[String], mod_name: &str) -> Result<Vec<(String, String)>, String> {
        let entries = normalize(files)?;
        let name = safe_name(mod_name);
        let out = match (self.family, self.kind) {
            (Family::Cyberpunk, _) => map_cyberpunk(&entries, &name)?,
            (Family::Witcher3, _) => map_witcher(&entries, &name),
            (_, kind) => match kind {
                Kind::Bethesda => match map_skse(&entries, &self.mod_root, self.launcher_swap.is_some()) {
                    Some(out) => out,
                    None => map_bethesda(&entries, &self.mod_root, self.launcher_swap.as_ref())?,
                },
                Kind::BepInEx => map_bepinex(&entries, &name),
                Kind::Smapi => map_smapi(&entries, &self.mod_root, &name)?,
                Kind::Generic => map_generic(&entries, &self.mod_root),
            },
        };
        if out.is_empty() {
            return Err("This archive has no files Turbine knows where to put.".into());
        }
        Ok(out)
    }

    /// Does this mod (its game-relative files) need the game's script loader?
    pub fn needs_loader(&self, game_files: &[String]) -> bool {
        match self.family {
            // Any framework it needs and doesn't bring itself (CET mods, RED4ext plugins, .reds, tweaks, .xl).
            Family::Cyberpunk => return !crate::cyberpunk::needs(game_files).is_empty(),
            Family::Witcher3 => return false,
            Family::Standard => {}
        }
        // The loader itself doesn't need the loader.
        if let Some(l) = &self.loader {
            let has = |name: &str| {
                let name = name.to_lowercase();
                game_files.iter().any(|f| {
                    let f = f.replace('\\', "/").trim_start_matches('/').to_lowercase();
                    f == name || f.starts_with(&format!("{name}/"))
                })
            };
            if l.detect.iter().all(|d| d.split('|').any(has)) {
                return false;
            }
        }
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

fn map_bethesda(entries: &[Entry], data_root: &str, swap: Option<&(String, String)>) -> Result<Vec<(String, String)>, String> {
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
            // The script extender's loader also goes in as the launcher Steam starts.
            if let Some((loader, launcher)) = swap {
                if e.is_top_file() && lc.eq_ignore_ascii_case(loader) {
                    out.push((e.src.clone(), launcher.clone()));
                }
            }
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
        // The store opens FOMOD installers before it gets here; this is only reached when the installer file itself
        // is missing or can't be read.
        return Err("This mod has an installer folder (fomod), but its installer file is missing, so Turbine can't tell which files to use.".into());
    }
    Ok(out)
}

/// SKSE's own archive (from skse.silverlock.org, Nexus Mod Manager Download, or a Slow download picked up from
/// Downloads): laid out with the same install plan as when Turbine fetches SKSE itself (loaders::skse_plan): the top
/// folder dropped, source code and readmes left out, the loader also in place of the launcher Steam starts.
/// None when the archive isn't SKSE.
fn map_skse(entries: &[Entry], data_root: &str, swap: bool) -> Option<Vec<(String, String)>> {
    let loader = entries.iter().find(|e| {
        let f = e.file_lc();
        f.starts_with("skse") && f.ends_with("_loader.exe")
    })?;
    let depth = loader.parts.len() - 1;
    if depth > 1 || entries.iter().any(|e| e.parts.len() <= depth || e.parts[..depth] != loader.parts[..depth]) {
        return None;
    }
    let mut plan = crate::loaders::skse_plan();
    plan.strip_folders = depth;
    let mut out = Vec::new();
    for e in entries {
        let Some(mut dest) = plan.map_path(&e.src) else { continue };
        let lc = dest.to_lowercase();
        if (e.parts.len() == depth + 1 && (is_doc(&e.file_lc()) || is_image(&e.file_lc()))) || lc.starts_with("src/") {
            continue;
        }
        if let Some(rest) = lc.strip_prefix("data/") {
            dest = join(data_root, &dest[dest.len() - rest.len()..]);
        }
        out.push((e.src.clone(), dest));
    }
    if swap {
        for hook in &plan.hooks {
            if let crate::loaders::Hook::LauncherSwap { original, replacement } = hook {
                if let Some((src, _)) = out.iter().find(|(_, d)| d.eq_ignore_ascii_case(replacement)).cloned() {
                    out.push((src, original.clone()));
                }
            }
        }
    }
    Some(out)
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
        // The store sets SMAPI up from the bundle inside its installer (see `smapi_bundle`); this is only reached
        // when that bundle is missing.
        return Err("This looks like SMAPI's installer, but the files it installs (install.dat) aren't in it.".into());
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

// ----- SMAPI's installer, without running it -------------------------------------------------------------------
//
// SMAPI's release zip ("SMAPI <version> installer") holds an installer program and, per system, the files it
// installs: internal/<linux|macOS|windows>/install.dat, a zip of the "bundle" (SMAPI's
// build/scripts/finalize-install-package.sh lines 43 to 50 zip each internal/<system>/bundle folder into
// install.dat). Turbine never runs the installer or any script; it reads install.dat and lays the files out the way
// the installer would, as ordinary mod files. From Pathoschild/SMAPI, src/SMAPI.Installer (commit e5b8bf3, Sep 2026):
// - Program.cs:34 and :45: the installer unzips install.dat from its own folder into a temporary bundle folder.
// - InteractiveInstaller.cs:378-387: every top-level entry of the bundle is copied into the game folder, except
//   what ShouldCopy (InteractiveInstaller.cs:889-897) leaves out: "mcs" (a macOS link) and "Mods" (handled apart).
// - InteractiveInstaller.cs:389-404 with Framework/InstallerPaths.cs:79-81: on Linux and macOS the game's launcher
//   "StardewValley" is moved to "StardewValley-original" and the bundle's "unix-launcher.sh" is moved into its
//   place, so Steam's Play starts SMAPI. Turbine installs unix-launcher.sh as "StardewValley"; deploy keeps the
//   game's own launcher in its backup folder and puts it back when SMAPI goes.
// - InteractiveInstaller.cs:406-421: "StardewValley" (the new launcher) and "StardewModdingAPI"
//   (InstallerPaths.cs:82) are made executable. Turbine sets the same two as executable in its staged copy.
// - InteractiveInstaller.cs:423-428: the game's "Stardew Valley.deps.json" is copied to "StardewModdingAPI.deps.json"
//   (needed to find native libraries). Turbine copies it from the game folder into the staged mod.
// - InteractiveInstaller.cs:437-500: the bundled mods (Mods/ConsoleCommands, Mods/SaveBackup; IDs at line 32) go
//   into the game's Mods folder. Turbine stages them under Mods/ as part of SMAPI.

/// SMAPI's launcher in its bundle, and the game launcher it replaces (InstallerPaths.cs:79-80).
pub const SMAPI_LAUNCHER: (&str, &str) = ("unix-launcher.sh", "StardewValley");

impl GameRules {
    /// For SMAPI's installer zip: the path (as in `files`) of the install.dat to install from, for this copy of the
    /// game (the Windows one when the game runs through Proton, the Linux one otherwise). None for anything else.
    pub fn smapi_bundle(&self, files: &[String]) -> Option<String> {
        if self.kind != Kind::Smapi {
            return None;
        }
        let entries = normalize(files).ok()?;
        if !is_smapi_installer(&entries) {
            return None;
        }
        let system = if self.runtime == Runtime::Windows { "windows" } else { "linux" };
        entries
            .iter()
            .find(|e| {
                let n = e.parts.len();
                n >= 3
                    && e.file_lc() == "install.dat"
                    && e.parts[n - 2].eq_ignore_ascii_case(system)
                    && e.parts[n - 3].eq_ignore_ascii_case("internal")
            })
            .map(|e| e.src.clone())
    }

    /// Lays out the files of SMAPI's bundle (the contents of install.dat) in the game folder, as its installer
    /// would (see the notes above).
    pub fn map_smapi_bundle(&self, files: &[String]) -> Result<Vec<(String, String)>, String> {
        let entries = normalize(files)?;
        if !entries.iter().any(|e| e.is_top_file() && e.file_lc() == "stardewmoddingapi.dll") {
            return Err("SMAPI's installer doesn't have the files it should (StardewModdingAPI.dll is missing).".into());
        }
        let mut out = Vec::new();
        for e in &entries {
            let lc = e.top_lc();
            if lc == "mcs" {
                continue;
            }
            if e.is_top_file() && lc == SMAPI_LAUNCHER.0 {
                if self.runtime != Runtime::Windows {
                    out.push((e.src.clone(), SMAPI_LAUNCHER.1.to_string()));
                }
                continue;
            }
            if lc == "mods" && !e.is_top_file() {
                out.push((e.src.clone(), join(&self.mod_root, &e.rest(1))));
                continue;
            }
            out.push((e.src.clone(), e.rest(0)));
        }
        Ok(out)
    }

    /// Files to copy from the game's own folder into a mod as it is added: (game file, path in the mod).
    /// SMAPI needs a copy of the game's deps.json under its own name.
    pub fn game_copies(&self, game_files: &[String]) -> Vec<(String, String)> {
        let has = |name: &str| game_files.iter().any(|f| f.eq_ignore_ascii_case(name));
        if self.kind == Kind::Smapi && has("StardewModdingAPI.dll") {
            return vec![("Stardew Valley.deps.json".into(), "StardewModdingAPI.deps.json".into())];
        }
        Vec::new()
    }

    /// Files of a mod (game-relative) that must be executable for the game's launcher to start them: SMAPI's
    /// launcher and program, BepInEx's Linux start scripts. Turbine only marks them; it never runs them.
    pub fn executables(&self, game_files: &[String]) -> Vec<String> {
        let wanted: &[&str] = match self.kind {
            Kind::Smapi if self.runtime != Runtime::Windows => &["StardewValley", "StardewModdingAPI"],
            Kind::BepInEx => &["start_game_bepinex.sh", "start_server_bepinex.sh", "run_bepinex.sh"],
            _ => &[],
        };
        if self.kind == Kind::Smapi && !game_files.iter().any(|f| f.eq_ignore_ascii_case("StardewModdingAPI.dll")) {
            return Vec::new();
        }
        game_files.iter().filter(|f| wanted.iter().any(|w| f.eq_ignore_ascii_case(w))).cloned().collect()
    }
}

// ----- Cyberpunk 2077 ---------------------------------------------------------------------------------------
//
// Folders at the top of the game folder that mods fill (see cyberpunk.rs for where each comes from): archive/pc/mod
// (.archive and ArchiveXL's .xl files), bin/x64 (Cyber Engine Tweaks and its Lua mods), engine and r6 (redscript,
// its .reds scripts in r6/scripts, TweakXL's r6/tweaks), red4ext (RED4ext and its plugins), mods (REDmod).

const CP_ROOTS: &[&str] = &["archive", "bin", "engine", "r6", "red4ext", "mods"];

fn cp_folder_holds(entries: &[Entry], top_lc: &str, test: &dyn Fn(&str) -> bool) -> bool {
    entries.iter().any(|e| e.parts.len() == 2 && e.top_lc() == top_lc && test(&e.file_lc()))
}

fn cp_is_redmod(name_lc: &str) -> bool {
    name_lc == "info.json"
}
fn cp_is_cet(name_lc: &str) -> bool {
    name_lc == "init.lua"
}
fn cp_is_script(name_lc: &str) -> bool {
    ext(name_lc) == "reds"
}
fn cp_is_tweak(name_lc: &str) -> bool {
    matches!(ext(name_lc), "yaml" | "yml" | "tweak")
}
fn cp_is_archive(name_lc: &str) -> bool {
    matches!(ext(name_lc), "archive" | "xl")
}

/// Every file in one top folder passes `test` (readmes aside).
fn cp_folder_all(entries: &[Entry], top_lc: &str, test: &dyn Fn(&str) -> bool) -> bool {
    entries.iter().filter(|x| x.top_lc() == top_lc).all(|x| test(&x.file_lc()) || is_doc(&x.file_lc()))
}

fn map_cyberpunk(entries: &[Entry], name: &str) -> Result<Vec<(String, String)>, String> {
    let mut entries = entries.to_vec();
    // Take off wrapper folders ("My Mod 1.2/archive/pc/mod/..."), but not a folder that is itself a REDmod, a Lua
    // mod or a script mod: its name is the mod's folder name.
    for _ in 0..4 {
        let snapshot = entries.clone();
        let own = |top: &str| {
            CP_ROOTS.contains(&top)
                || cp_folder_holds(&snapshot, top, &|f| cp_is_redmod(f) || cp_is_cet(f) || cp_is_script(f) || cp_is_tweak(f))
        };
        if !unwrap_once(&mut entries, &own) {
            break;
        }
    }
    let flat_redmod = entries.iter().any(|e| e.is_top_file() && cp_is_redmod(&e.top_lc()));
    let flat_cet = entries.iter().any(|e| e.is_top_file() && cp_is_cet(&e.top_lc()));
    let cet_mods = "bin/x64/plugins/cyber_engine_tweaks/mods";
    let mut out: Vec<(String, String)> = Vec::new();
    for e in &entries {
        let lc = e.top_lc();
        let file = e.file_lc();
        if e.is_top_file() && (is_doc(&lc) || is_image(&lc)) && !(flat_redmod || flat_cet) {
            continue;
        }
        if !e.is_top_file() && CP_ROOTS.contains(&lc.as_str()) {
            out.push((e.src.clone(), e.rest(0)));
        } else if flat_redmod {
            out.push((e.src.clone(), format!("mods/{name}/{}", e.rest(0))));
        } else if flat_cet {
            out.push((e.src.clone(), format!("{cet_mods}/{name}/{}", e.rest(0))));
        } else if e.is_top_file() {
            let dest = if cp_is_archive(&lc) {
                format!("archive/pc/mod/{}", e.top())
            } else if cp_is_script(&lc) {
                format!("r6/scripts/{name}/{}", e.top())
            } else if cp_is_tweak(&lc) {
                format!("r6/tweaks/{name}/{}", e.top())
            } else {
                continue; // nothing says where it goes
            };
            out.push((e.src.clone(), dest));
        } else if cp_folder_holds(&entries, &lc, &cp_is_redmod) {
            out.push((e.src.clone(), format!("mods/{}", e.rest(0))));
        } else if cp_folder_holds(&entries, &lc, &cp_is_cet) {
            out.push((e.src.clone(), format!("{cet_mods}/{}", e.rest(0))));
        } else if cp_is_script(&file) && cp_folder_all(&entries, &lc, &cp_is_script) {
            out.push((e.src.clone(), format!("r6/scripts/{}", e.rest(0))));
        } else if cp_is_tweak(&file) && cp_folder_all(&entries, &lc, &cp_is_tweak) {
            out.push((e.src.clone(), format!("r6/tweaks/{}", e.rest(0))));
        } else if cp_is_archive(&file) {
            // .archive files in a plain folder: they all go flat into archive/pc/mod.
            out.push((e.src.clone(), format!("archive/pc/mod/{}", e.parts.last().cloned().unwrap_or_default())));
        }
    }
    // Two files for the same place means the archive holds choices (one folder per option), which Turbine can't pick.
    let mut seen: Vec<(String, &str)> = Vec::new();
    for (src, dest) in &out {
        let key = dest.to_lowercase();
        if let Some((_, first)) = seen.iter().find(|(k, _)| *k == key) {
            let folder = |p: &str| p.split('/').next().unwrap_or("").to_string();
            return Err(format!(
                "This archive has more than one version of {} (in \"{}\" and \"{}\"), so it holds choices Turbine \
                 can't make. Pick the one you want and add it as its own zip",
                dest.rsplit('/').next().unwrap_or(dest),
                folder(first),
                folder(src)
            ));
        }
        seen.push((key, src));
    }
    Ok(out)
}

// ----- The Witcher 3 ----------------------------------------------------------------------------------------
//
// Game folder layout (see witcher.rs): mods/<modName>/content/..., dlc/<dlcName>/content/..., menu files in
// bin/config/r4game/user_config_matrix/pc. A mod folder's name must start with "mod" or the game ignores it.

fn w3_mod_folder(folder: &str) -> String {
    if folder.to_lowercase().starts_with("mod") {
        folder.to_string()
    } else {
        format!("mod{}", folder.replace(' ', ""))
    }
}

fn w3_holds_content(entries: &[Entry], top: &str) -> bool {
    entries.iter().any(|e| e.parts.len() > 2 && e.top_lc() == top && e.parts[1].eq_ignore_ascii_case("content"))
}

fn map_witcher(entries: &[Entry], name: &str) -> Vec<(String, String)> {
    let mut entries = entries.to_vec();
    for _ in 0..4 {
        let snapshot = entries.clone();
        let own = |top: &str| matches!(top, "mods" | "dlc" | "bin" | "content") || w3_holds_content(&snapshot, top);
        if !unwrap_once(&mut entries, &own) {
            break;
        }
    }
    let menu_dir = "bin/config/r4game/user_config_matrix/pc";
    let mut out = Vec::new();
    for e in &entries {
        let lc = e.top_lc();
        if e.is_top_file() {
            // Menu files on their own; readmes, pictures and settings lines (written to the log) stay out.
            if ext(&lc) == "xml" {
                out.push((e.src.clone(), format!("{menu_dir}/{}", e.top())));
            }
            continue;
        }
        match lc.as_str() {
            "mods" if e.parts.len() > 2 => {
                out.push((e.src.clone(), format!("mods/{}/{}", w3_mod_folder(&e.parts[1]), e.rest(2))));
            }
            "dlc" | "bin" => out.push((e.src.clone(), e.rest(0))),
            "content" => {
                // The game's own content0 folder (patches for the classic game ship it that way), or a mod's content
                // folder without its mod folder around it.
                let p1 = e.parts[1].to_lowercase();
                if e.parts.len() > 2 && p1.starts_with("content") && p1.len() > "content".len() {
                    out.push((e.src.clone(), e.rest(0)));
                } else {
                    out.push((e.src.clone(), format!("mods/{}/{}", w3_mod_folder(name), e.rest(0))));
                }
            }
            _ if w3_holds_content(&entries, &lc) => {
                if lc.starts_with("dlc") {
                    out.push((e.src.clone(), format!("dlc/{}", e.rest(0))));
                } else {
                    out.push((e.src.clone(), format!("mods/{}/{}", w3_mod_folder(e.top()), e.rest(1))));
                }
            }
            _ => {} // a folder the game has no use for (documents, screenshots, optional extras)
        }
    }
    out
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
    let mut checks = pre_play_with(game, rules, last_build, needs_loader, free_bytes, fs.as_deref(), loader_ok);
    if rules.plugins {
        if let Ok(order) = plugins::LoadOrder::read(game) {
            add_checks(&mut checks, master_checks(&order.missing_masters()));
        }
    }
    checks
}

/// Adds checks to a list, dropping "Ready to play" when there's something to say.
pub fn add_checks(checks: &mut Vec<Check>, more: Vec<Check>) {
    if more.is_empty() {
        return;
    }
    checks.retain(|c| c.level != "ok");
    checks.extend(more);
}

/// One "Missing master" stop line per plugin whose masters (plugins it is built on) are missing or turned off.
/// The game crashes at start with such a plugin turned on.
pub fn master_checks(missing: &[MissingMaster]) -> Vec<Check> {
    let mut plugins: Vec<&str> = Vec::new();
    for m in missing {
        if !plugins.contains(&m.plugin.as_str()) {
            plugins.push(&m.plugin);
        }
    }
    plugins
        .into_iter()
        .map(|p| {
            let gone: Vec<&str> = missing.iter().filter(|m| m.plugin == p && !m.turned_off).map(|m| m.master.as_str()).collect();
            let off: Vec<&str> = missing.iter().filter(|m| m.plugin == p && m.turned_off).map(|m| m.master.as_str()).collect();
            let mut detail = Vec::new();
            if !gone.is_empty() {
                let (is, it) = if gone.len() > 1 { ("aren't", "them") } else { ("isn't", "it") };
                detail.push(format!(
                    "{p} needs {}, which {is} in the game's Data folder. Add the mod that has {it}, or turn off {p}.",
                    gone.join(" and ")
                ));
            }
            if !off.is_empty() {
                let (is, it) = if off.len() > 1 { ("are", "them") } else { ("is", "it") };
                detail.push(format!(
                    "{p} needs {}, which {is} turned off. Turn {it} on in the Load order tab, or turn off {p}.",
                    off.join(" and ")
                ));
            }
            Check { level: "stop".into(), title: "Missing master".into(), detail: detail.join(" ") }
        })
        .collect()
}

/// One mod's files in the game folder (game-relative), for the checks that look at what each mod brings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModFiles {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub files: Vec<String>,
}

/// Address Library for SKSE Plugins: the version database file SKSE plugins built with CommonLibSSE read, in
/// Data/SKSE/Plugins. Special Edition's is version-<game version>.bin, Anniversary Edition's
/// versionlib-<game version>.bin (nexusmods.com/skyrimspecialedition/mods/32444: versiondb.h reads the first,
/// versionlibdb.h the second).
pub const ADDRESS_LIBRARY: &str = "Address Library for SKSE Plugins";
pub const ADDRESS_LIBRARY_URL: &str = "https://www.nexusmods.com/skyrimspecialedition/mods/32444";
pub const ADDRESS_LIBRARY_DIR: &str = "Data/SKSE/Plugins";

/// Is this game-relative file part of Address Library?
pub fn is_address_library_file(path: &str) -> bool {
    let p = path.replace('\\', "/").trim_start_matches('/').to_lowercase();
    let Some(file) = p.strip_prefix("data/skse/plugins/") else { return false };
    !file.contains('/') && (file.starts_with("version-") || file.starts_with("versionlib-")) && file.ends_with(".bin")
}

fn is_skse_plugin(path: &str) -> bool {
    let p = path.replace('\\', "/").trim_start_matches('/').to_lowercase();
    p.starts_with("data/skse/plugins/") && p.ends_with(".dll")
}

impl GameRules {
    /// Notes for the activity log about a mod's archive (paths inside it) that the player may want to act on:
    /// settings lines to add by hand, a menu file the game's menu list must name, REDmod needing the launcher.
    pub fn install_notes(&self, archive_files: &[String]) -> Vec<String> {
        match self.family {
            Family::Witcher3 => crate::witcher::install_notes(archive_files),
            Family::Cyberpunk => {
                let mapped = self.map_archive(archive_files, "mod").unwrap_or_default();
                let dests: Vec<String> = mapped.into_iter().map(|(_, d)| d).collect();
                if crate::cyberpunk::has_redmod(&dests) {
                    vec!["This is a REDmod mod: it loads after Cyberpunk's own launcher sets it up (Enable mods in the launcher's settings).".into()]
                } else {
                    Vec::new()
                }
            }
            Family::Standard => Vec::new(),
        }
    }

    /// Game-relative files the mod checks look for in the game folder (frameworks, tools, Address Library).
    /// Folders in the list ending in '/' mean "every file directly in this folder".
    pub fn files_to_look_for(&self) -> Vec<String> {
        match (self.family, self.kind) {
            (Family::Cyberpunk, _) => crate::cyberpunk::files_to_look_for().into_iter().map(String::from).collect(),
            (Family::Standard, Kind::Bethesda) => vec![format!("{ADDRESS_LIBRARY_DIR}/")],
            _ => Vec::new(),
        }
    }
}

/// The game folder's files that `files_to_look_for` asks about, game-relative, as found on disk (case ignored).
pub fn present_files(game: &Game, rules: &GameRules) -> Vec<String> {
    let mut out = Vec::new();
    for want in rules.files_to_look_for() {
        if let Some(dir) = want.strip_suffix('/') {
            if let Some(found) = find_ci(&game.install_dir, dir) {
                if let Ok(rd) = std::fs::read_dir(found) {
                    for e in rd.flatten() {
                        out.push(format!("{dir}/{}", e.file_name().to_string_lossy()));
                    }
                }
            }
        } else if find_ci(&game.install_dir, &want).is_some() {
            out.push(want);
        }
    }
    out.sort();
    out
}

/// The checks before Play that look at each turned-on mod's files: missing Cyberpunk frameworks and REDmod, Witcher 3
/// scripts that need merging, SKSE plugins without Address Library. Add them to `pre_play`'s list with `add_checks`.
pub fn mod_checks(game: &Game, rules: &GameRules, mods: &[ModFiles]) -> Vec<Check> {
    mod_checks_with(rules, mods, &present_files(game, rules))
}

/// `mod_checks` with the game folder's relevant files given (as `present_files` returns them), for tests.
pub fn mod_checks_with(rules: &GameRules, mods: &[ModFiles], game_files: &[String]) -> Vec<Check> {
    let on: Vec<(&str, &[String])> = mods.iter().filter(|m| m.enabled).map(|m| (m.name.as_str(), m.files.as_slice())).collect();
    match (rules.family, rules.kind) {
        (Family::Cyberpunk, _) => crate::cyberpunk::checks(&on, game_files),
        (Family::Witcher3, _) => crate::witcher::conflict_checks(&on),
        (Family::Standard, Kind::Bethesda) => {
            let users: Vec<&str> = on
                .iter()
                .filter(|(_, files)| files.iter().any(|f| is_skse_plugin(f)) && !files.iter().any(|f| is_address_library_file(f)))
                .map(|(n, _)| *n)
                .collect();
            let have = game_files.iter().any(|f| is_address_library_file(f))
                || on.iter().any(|(_, files)| files.iter().any(|f| is_address_library_file(f)));
            if users.is_empty() || have {
                return Vec::new();
            }
            let who = match users.len() {
                1 => format!("{} adds code to the game (an SKSE plugin)", users[0]),
                2 => format!("{} and {} add code to the game (SKSE plugins)", users[0], users[1]),
                n => format!("{}, {} and {} more add code to the game (SKSE plugins)", users[0], users[1], n - 2),
            };
            vec![Check {
                level: "warn".into(),
                title: format!("{ADDRESS_LIBRARY} is missing"),
                detail: format!(
                    "{who}, and most such plugins need {ADDRESS_LIBRARY} to find their way around the game. Without \
                     it they don't load, or the game closes at start. Get it from {ADDRESS_LIBRARY_URL} (the \
                     All in one file)."
                ),
            }]
        }
        _ => Vec::new(),
    }
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
            if rules.family == Family::Cyberpunk && needs_loader {
                checks.push(check(
                    "warn",
                    title,
                    format!(
                        "{} changed since your mods were last set up. Cyber Engine Tweaks and RED4ext (and the mods \
                         built on them) usually stop working after a game update until their makers release new \
                         versions. If the game crashes at start or mods are missing, update them, or use Play \
                         original: it starts the game without mods.",
                        game.name
                    ),
                ));
            } else if rules.kind == Kind::Bethesda && needs_loader {
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
