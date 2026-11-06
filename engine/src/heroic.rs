//! Finds Heroic games (Epic, GOG and games added to Heroic by hand), for both the Flatpak and the native Heroic.

use crate::games::{Game, Runtime, Scan, Store};
use crate::json::{self, Json};
use crate::steam::runtime_of;
use std::fs;
use std::path::{Path, PathBuf};

/// Heroic's config folders that exist: the Flatpak's first (the usual one on SteamOS), then a native install.
pub fn heroic_roots(home: &Path) -> Vec<PathBuf> {
    [".var/app/com.heroicgameslauncher.hgl/config/heroic", ".config/heroic"]
        .iter()
        .map(|r| home.join(r))
        .filter(|p| p.is_dir())
        .collect()
}

fn read_json(path: &Path, scan: &mut Scan) -> Option<Json> {
    let text = fs::read_to_string(path).ok()?;
    match json::parse(&text) {
        Ok(v) => Some(v),
        Err(e) => {
            scan.problems.push(format!("Couldn't read {} ({e}).", path.display()));
            None
        }
    }
}

fn runtime_from_platform(platform: Option<&str>, install_dir: &Path) -> Runtime {
    match platform.map(str::to_ascii_lowercase).as_deref() {
        Some("linux") => Runtime::Native,
        Some("windows" | "win32" | "win64") => Runtime::Windows,
        _ => runtime_of(install_dir, None),
    }
}

fn push(scan: &mut Scan, store: Store, id: &str, name: &str, dir: &str, build: Option<&str>, platform: Option<&str>) {
    if id.is_empty() || dir.is_empty() {
        return;
    }
    let install_dir = PathBuf::from(dir);
    if !install_dir.is_dir() {
        return;
    }
    if scan.games.iter().any(|g| g.store == store && g.store_id == id) {
        return;
    }
    let name = if name.is_empty() {
        install_dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| id.to_string())
    } else {
        name.to_string()
    };
    let runtime = runtime_from_platform(platform, &install_dir);
    scan.games.push(Game {
        store,
        store_id: id.to_string(),
        name,
        library: install_dir.parent().map(Path::to_path_buf).unwrap_or_default(),
        install_dir,
        build: build.map(str::to_string),
        runtime,
        cover: None,
        header: None,
        prefix: None,
    });
}

/// Looks up a GOG game's title in Heroic's library caches (installed.json doesn't carry it).
fn gog_titles(root: &Path, scan: &mut Scan) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for rel in ["store_cache/gog_library.json", "gog_store/library.json"] {
        let Some(v) = read_json(&root.join(rel), scan) else { continue };
        let list = v.get("games").map(Json::as_array).unwrap_or(&[]);
        for g in list {
            if let (Some(id), Some(t)) = (g.str_at("app_name"), g.str_at("title")) {
                out.push((id.to_string(), t.to_string()));
            }
        }
    }
    out
}

pub fn scan(home: &Path, scan: &mut Scan) {
    for root in heroic_roots(home) {
        // Epic, through Heroic's copy of Legendary.
        if let Some(v) = read_json(&root.join("legendaryConfig/legendary/installed.json"), scan) {
            for (id, g) in v.entries() {
                if g.get("is_dlc").and_then(Json::as_bool) == Some(true) {
                    continue;
                }
                push(
                    scan,
                    Store::Epic,
                    id,
                    g.str_at("title").unwrap_or(""),
                    g.str_at("install_path").unwrap_or(""),
                    g.str_at("version"),
                    g.str_at("platform"),
                );
            }
        }
        // GOG.
        let titles = gog_titles(&root, scan);
        if let Some(v) = read_json(&root.join("gog_store/installed.json"), scan) {
            for g in v.get("installed").map(Json::as_array).unwrap_or(&[]) {
                if g.get("is_dlc").and_then(Json::as_bool) == Some(true) {
                    continue;
                }
                let id = g.str_at("appName").unwrap_or("");
                let title = titles.iter().find(|(i, _)| i == id).map(|(_, t)| t.as_str()).unwrap_or("");
                push(
                    scan,
                    Store::Gog,
                    id,
                    title,
                    g.str_at("install_path").unwrap_or(""),
                    g.str_at("version"),
                    g.str_at("platform"),
                );
            }
        }
        // Games added to Heroic by hand.
        if let Some(v) = read_json(&root.join("sideload_apps/library.json"), scan) {
            for g in v.get("games").map(Json::as_array).unwrap_or(&[]) {
                if g.get("is_installed").and_then(Json::as_bool) == Some(false) {
                    continue;
                }
                let platform = g.get("install").and_then(|i| i.str_at("platform"));
                push(
                    scan,
                    Store::Sideload,
                    g.str_at("app_name").unwrap_or(""),
                    g.str_at("title").unwrap_or(""),
                    g.str_at("folder_name").unwrap_or(""),
                    None,
                    platform,
                );
            }
        }
    }
}
