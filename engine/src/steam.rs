//! Finds Steam games: Steam's own folder, every library listed in libraryfolders.vdf (internal, SD cards,
//! other drives), each game's appmanifest, its Proton prefix and the art Steam already keeps on the device.

use crate::games::{Game, Runtime, Scan, Store};
use crate::vdf::{self, Vdf};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Steam's own folders on SteamOS, in the order Steam itself uses them.
pub fn steam_roots(home: &Path) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for rel in [".local/share/Steam", ".steam/steam", ".steam/root"] {
        let p = home.join(rel);
        if !p.join("steamapps").is_dir() {
            continue;
        }
        let real = fs::canonicalize(&p).unwrap_or(p);
        if seen.insert(real.clone()) {
            out.push(real);
        }
    }
    out
}

/// Steam's tools and runtimes show up as apps; they are never games.
fn is_tool(app_id: &str, name: &str) -> bool {
    const IDS: &[&str] = &["228980", "1070560", "1391110", "1628350", "1493710", "2180100", "1826330", "1580130"];
    const PREFIXES: &[&str] = &["Proton", "Steam Linux Runtime", "Steamworks Common", "SteamVR", "Steam Audio"];
    IDS.contains(&app_id) || PREFIXES.iter().any(|p| name.starts_with(p))
}

fn read_vdf(path: &Path, scan: &mut Scan) -> Option<Vdf> {
    let text = match fs::read(path) {
        Ok(b) => String::from_utf8_lossy(&b).into_owned(),
        Err(_) => return None,
    };
    match vdf::parse(&text) {
        Ok(v) => Some(v),
        Err(e) => {
            scan.problems.push(format!("Couldn't read {} ({e}).", path.display()));
            None
        }
    }
}

/// Library folders listed by Steam, with Steam's own folder first. Missing ones (an SD card that's out) are skipped.
pub fn libraries(root: &Path, scan: &mut Scan) -> Vec<PathBuf> {
    let mut out = vec![root.to_path_buf()];
    let file = root.join("steamapps/libraryfolders.vdf");
    if let Some(v) = read_vdf(&file, scan) {
        let list = v.get("libraryfolders").or_else(|| v.get("LibraryFolders"));
        for (_, entry) in list.map(Vdf::entries).unwrap_or(&[]) {
            let path = match entry {
                Vdf::Obj(_) => entry.str_at(&["path"]),
                Vdf::Str(s) => Some(s.as_str()), // older format: "1" "/path"
            };
            if let Some(p) = path {
                let p = PathBuf::from(p);
                if p.join("steamapps").is_dir() {
                    let real = fs::canonicalize(&p).unwrap_or(p);
                    if !out.contains(&real) {
                        out.push(real);
                    }
                }
            }
        }
    }
    out
}

/// Art Steam keeps in appcache/librarycache. Older Steam used flat files ("489830_library_600x900.jpg"); newer Steam
/// keeps a folder per game, sometimes with a hashed subfolder. Both are checked.
pub fn find_art(root: &Path, app_id: &str, kind: &str) -> Option<PathBuf> {
    let cache = root.join("appcache/librarycache");
    for ext in ["jpg", "png"] {
        let flat = cache.join(format!("{app_id}_{kind}.{ext}"));
        if flat.is_file() {
            return Some(flat);
        }
    }
    let dir = cache.join(app_id);
    find_named(&dir, kind, 2)
}

fn find_named(dir: &Path, stem: &str, depth: u32) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    let mut subdirs = Vec::new();
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            subdirs.push(p);
        } else if p.file_stem().and_then(|s| s.to_str()) == Some(stem)
            && matches!(p.extension().and_then(|s| s.to_str()), Some("jpg" | "png"))
        {
            return Some(p);
        }
    }
    if depth == 0 {
        return None;
    }
    subdirs.sort();
    subdirs.iter().find_map(|d| find_named(d, stem, depth - 1))
}

/// Windows or native, from what's on disk: a Proton prefix means it already ran under Proton; otherwise .exe files
/// at the top of its folder mean a Windows build.
pub fn runtime_of(install_dir: &Path, prefix: Option<&Path>) -> Runtime {
    if prefix.is_some() {
        return Runtime::Windows;
    }
    let Ok(entries) = fs::read_dir(install_dir) else { return Runtime::Unknown };
    let mut any = false;
    for e in entries.flatten() {
        any = true;
        let name = e.file_name();
        let name = name.to_string_lossy().to_ascii_lowercase();
        if name.ends_with(".exe") {
            return Runtime::Windows;
        }
    }
    if any {
        Runtime::Native
    } else {
        Runtime::Unknown
    }
}

/// Every installed Steam game, across all libraries.
pub fn scan(home: &Path, scan: &mut Scan) {
    let mut seen = HashSet::new();
    for root in steam_roots(home) {
        for lib in libraries(&root, scan) {
            let steamapps = lib.join("steamapps");
            let Ok(entries) = fs::read_dir(&steamapps) else { continue };
            let mut manifests: Vec<PathBuf> = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    let n = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    n.starts_with("appmanifest_") && n.ends_with(".acf")
                })
                .collect();
            manifests.sort();
            for m in manifests {
                let Some(v) = read_vdf(&m, scan) else { continue };
                let Some(app) = v.get("AppState") else { continue };
                let (Some(app_id), Some(name), Some(dir)) =
                    (app.str_at(&["appid"]), app.str_at(&["name"]), app.str_at(&["installdir"]))
                else {
                    continue;
                };
                if is_tool(app_id, name) || !seen.insert(app_id.to_string()) {
                    continue;
                }
                // StateFlags 4 = fully installed; anything else is downloading, updating or broken.
                let flags: u32 = app.str_at(&["StateFlags"]).and_then(|s| s.parse().ok()).unwrap_or(4);
                if flags & 4 == 0 {
                    continue;
                }
                let install_dir = steamapps.join("common").join(dir);
                if !install_dir.is_dir() {
                    continue;
                }
                let pfx = steamapps.join("compatdata").join(app_id).join("pfx");
                let prefix = pfx.is_dir().then_some(pfx);
                let runtime = runtime_of(&install_dir, prefix.as_deref());
                scan.games.push(Game {
                    store: Store::Steam,
                    store_id: app_id.to_string(),
                    name: name.to_string(),
                    install_dir,
                    library: lib.clone(),
                    build: app.str_at(&["buildid"]).map(str::to_string),
                    runtime,
                    cover: find_art(&root, app_id, "library_600x900"),
                    header: find_art(&root, app_id, "header"),
                    prefix,
                });
            }
        }
    }
}
