//! Skyrim Special Edition: Address Library for SKSE plugins, and SKSE's archive from Nexus (Mod Manager Download or
//! a Slow download from Downloads) laid out with the same plan as when Turbine fetches SKSE itself.

mod support;

use std::collections::BTreeMap;
use std::fs;
use support::{game, write_zip, TempDir};
use turbine_engine::games::Store as Launcher;
use turbine_engine::loaders::{self, Hook};
use turbine_engine::nexus;
use turbine_engine::rules::{self, is_address_library_file, mod_checks_with, GameRules, ModFiles};
use turbine_engine::store::{Import, Store};
use turbine_engine::Runtime;

fn list(files: &[&str]) -> Vec<String> {
    files.iter().map(|s| s.to_string()).collect()
}

fn skyrim(store: Launcher) -> GameRules {
    let id = if store == Launcher::Steam { "489830" } else { "1711230643" };
    let mut g = game(store, id, "Skyrim Special Edition", std::path::Path::new("/games/Skyrim"), None);
    g.runtime = Runtime::Windows;
    GameRules::for_game(&g)
}

fn m(name: &str, files: &[&str], enabled: bool) -> ModFiles {
    ModFiles { id: name.to_lowercase(), name: name.into(), enabled, files: list(files) }
}

#[test]
fn address_library_files() {
    assert!(is_address_library_file("Data/SKSE/Plugins/versionlib-1-6-1170-0.bin"));
    assert!(is_address_library_file("data\\skse\\plugins\\version-1-5-97-0.bin"));
    assert!(!is_address_library_file("Data/SKSE/Plugins/EngineFixes.dll"));
    assert!(!is_address_library_file("Data/SKSE/Plugins/sub/version-1.bin"));
    assert!(!is_address_library_file("Data/version-1-5-97-0.bin"));
}

#[test]
fn skse_plugins_without_address_library_get_a_warning() {
    let r = skyrim(Launcher::Steam);
    let tweaks = m("powerofthree's Tweaks", &["Data/SKSE/Plugins/po3_Tweaks.dll", "Data/SKSE/Plugins/po3_Tweaks.ini"], true);
    let ui = m("SkyUI", &["Data/SkyUI_SE.esp", "Data/SkyUI_SE.bsa"], true);
    let c = mod_checks_with(&r, &[tweaks.clone(), ui.clone()], &[]);
    assert_eq!(c.len(), 1);
    assert_eq!((c[0].level.as_str(), c[0].title.as_str()), ("warn", "Address Library for SKSE Plugins is missing"));
    assert!(c[0].detail.starts_with("powerofthree's Tweaks adds code to the game"), "{}", c[0].detail);
    assert!(c[0].detail.contains("nexusmods.com/skyrimspecialedition/mods/32444"));
    // Turned on as a mod, or already in the game's Data folder: fine.
    let al = m("Address Library", &["Data/SKSE/Plugins/versionlib-1-6-1170-0.bin"], true);
    assert!(mod_checks_with(&r, &[tweaks.clone(), al.clone()], &[]).is_empty());
    assert!(mod_checks_with(&r, std::slice::from_ref(&tweaks), &list(&["Data/SKSE/Plugins/versionlib-1-6-1170-0.bin"])).is_empty());
    // Turned off: it doesn't count; and a turned-off plugin doesn't need it.
    let mut al_off = al;
    al_off.enabled = false;
    assert_eq!(mod_checks_with(&r, &[tweaks.clone(), al_off], &[]).len(), 1);
    let mut tweaks_off = tweaks;
    tweaks_off.enabled = false;
    assert!(mod_checks_with(&r, &[tweaks_off, ui], &[]).is_empty());
}

#[test]
fn address_library_seen_in_the_game_folder() {
    let t = TempDir::new("al-game");
    let dir = t.path("Skyrim");
    fs::create_dir_all(dir.join("data/SKSE/plugins")).unwrap();
    fs::write(dir.join("data/SKSE/plugins/versionlib-1-6-1170-0.bin"), b"db").unwrap();
    let mut g = game(Launcher::Steam, "489830", "Skyrim Special Edition", &dir, None);
    g.runtime = Runtime::Windows;
    let r = GameRules::for_game(&g);
    assert_eq!(rules::present_files(&g, &r), list(&["Data/SKSE/Plugins/versionlib-1-6-1170-0.bin"]));
    let tweaks = m("Tweaks", &["Data/SKSE/Plugins/po3_Tweaks.dll"], true);
    assert!(rules::mod_checks(&g, &r, &[tweaks]).is_empty());
}

/// SKSE 2.2.6's archive as published on Nexus (skse64_2_02_06.7z).
const SKSE: &[&str] = &[
    "skse64_2_02_06/skse64_loader.exe",
    "skse64_2_02_06/skse64_1_6_1170.dll",
    "skse64_2_02_06/skse64_readme.txt",
    "skse64_2_02_06/skse64_whatsnew.txt",
    "skse64_2_02_06/Data/Scripts/Actor.pex",
    "skse64_2_02_06/Data/Scripts/Source/Actor.psc",
    "skse64_2_02_06/src/skse64/skse64/main.cpp",
];

#[test]
fn skse_from_nexus_uses_the_loader_install_plan() {
    assert_eq!(nexus::domain_for_store_id("489830"), Some(loaders::SKSE_NEXUS_DOMAIN));
    let plan = loaders::skse_plan();
    // map_archive gives what the plan gives, plus the launcher swap the plan's hook asks for (Steam only).
    let mut from_plan: Vec<String> = SKSE.iter().filter_map(|f| plan.map_path(f)).collect();
    assert!(plan.hooks.iter().any(|h| matches!(h, Hook::LauncherSwap { original, .. } if original == "SkyrimSELauncher.exe")));
    from_plan.push("SkyrimSELauncher.exe".into());
    from_plan.sort();
    let steam: BTreeMap<String, String> = skyrim(Launcher::Steam).map_archive(&list(SKSE), "SKSE").unwrap().into_iter().map(|(s, d)| (d, s)).collect();
    assert_eq!(steam.keys().cloned().collect::<Vec<_>>(), from_plan);
    assert_eq!(steam["SkyrimSELauncher.exe"], "skse64_2_02_06/skse64_loader.exe");
    let gog: Vec<String> = skyrim(Launcher::Gog).map_archive(&list(SKSE), "SKSE").unwrap().into_iter().map(|(_, d)| d).collect();
    assert!(!gog.contains(&"SkyrimSELauncher.exe".to_string()));
    assert!(gog.contains(&"skse64_loader.exe".to_string()));
    // Repacked without the top folder, or with "data" in lower case: the same places.
    let flat = list(&["skse64_loader.exe", "skse64_1_6_1170.dll", "data/Scripts/Actor.pex", "skse64_readme.txt"]);
    let mut d: Vec<String> = skyrim(Launcher::Steam).map_archive(&flat, "SKSE").unwrap().into_iter().map(|(_, d)| d).collect();
    d.sort();
    assert_eq!(d, list(&["Data/Scripts/Actor.pex", "SkyrimSELauncher.exe", "skse64_1_6_1170.dll", "skse64_loader.exe"]));
}

#[test]
fn skse_picked_up_from_downloads_goes_in_with_its_launcher_swap() {
    let t = TempDir::new("skse-dl");
    let dir = t.path("lib/Skyrim");
    fs::create_dir_all(dir.join("Data")).unwrap();
    fs::write(dir.join("SkyrimSELauncher.exe"), b"vanilla").unwrap();
    let mut g = game(Launcher::Steam, "489830", "Skyrim Special Edition", &dir, None);
    g.runtime = Runtime::Windows;
    let mut s = Store::open(&g, &t.path("data")).unwrap();
    let a = t.path("Downloads/skse64_2_02_06.zip");
    fs::create_dir_all(a.parent().unwrap()).unwrap();
    write_zip(&a, &[("skse64_2_02_06/skse64_loader.exe", b"loader"), ("skse64_2_02_06/skse64_1_6_1170.dll", b"dll"), ("skse64_2_02_06/src/a.cpp", b"src")]);
    // As the watcher hands it over: a Nexus source for mod 30379.
    let file = nexus::FileInfo {
        file_id: 1,
        name: "SKSE".into(),
        version: "2.2.6".into(),
        category: "MAIN".into(),
        file_name: "skse64_2_02_06.zip".into(),
        size_bytes: 0,
        uploaded_timestamp: 0,
        is_primary: true,
        description: String::new(),
    };
    let src = nexus::source_info("skyrimspecialedition", 30379, &file, None);
    let Import::Added(done) = s.import_archive(&a, src).unwrap() else { panic!() };
    assert_eq!(done.item.version.as_deref(), Some("2.2.6"));
    s.apply().unwrap();
    assert_eq!(fs::read(dir.join("SkyrimSELauncher.exe")).unwrap(), b"loader");
    assert!(!dir.join("src").exists());
    s.play_original().unwrap();
    assert_eq!(fs::read(dir.join("SkyrimSELauncher.exe")).unwrap(), b"vanilla");
}
