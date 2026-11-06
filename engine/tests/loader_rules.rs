//! Game rules added for loaders: SMAPI's bundle layout, executables, the SKSE launcher swap data, launch guidance,
//! and the missing master check lines.

use std::path::Path;
use turbine_engine::plugins::MissingMaster;
use turbine_engine::rules::{self, GameRules, SMAPI_LAUNCHER};
use turbine_engine::{Game, Runtime, Store};

fn game(store: Store, id: &str, runtime: Runtime) -> Game {
    Game {
        store,
        store_id: id.into(),
        name: "Test".into(),
        install_dir: "/nowhere".into(),
        library: "/".into(),
        build: None,
        runtime,
        cover: None,
        header: None,
        prefix: None,
    }
}

fn list(files: &[&str]) -> Vec<String> {
    files.iter().map(|s| s.to_string()).collect()
}

const INSTALLER: &[&str] = &[
    "SMAPI 4.3.2 installer/install on Linux.sh",
    "SMAPI 4.3.2 installer/internal/linux/install.dat",
    "SMAPI 4.3.2 installer/internal/linux/SMAPI.Installer.dll",
    "SMAPI 4.3.2 installer/internal/windows/install.dat",
    "SMAPI 4.3.2 installer/internal/macOS/install.dat",
];

#[test]
fn smapi_bundle_is_picked_for_the_build_that_runs() {
    let native = GameRules::for_game(&game(Store::Steam, "413150", Runtime::Native));
    assert_eq!(native.smapi_bundle(&list(INSTALLER)).as_deref(), Some("SMAPI 4.3.2 installer/internal/linux/install.dat"));
    let unknown = GameRules::for_game(&game(Store::Gog, "1453375253", Runtime::Unknown));
    assert_eq!(unknown.smapi_bundle(&list(INSTALLER)).as_deref(), Some("SMAPI 4.3.2 installer/internal/linux/install.dat"));
    let windows = GameRules::for_game(&game(Store::Steam, "413150", Runtime::Windows));
    assert_eq!(windows.smapi_bundle(&list(INSTALLER)).as_deref(), Some("SMAPI 4.3.2 installer/internal/windows/install.dat"));
    // A normal mod, or another game: no bundle.
    assert!(native.smapi_bundle(&list(&["Mod/manifest.json", "Mod/a.dll"])).is_none());
    let valheim = GameRules::for_game(&game(Store::Steam, "892970", Runtime::Native));
    assert!(valheim.smapi_bundle(&list(INSTALLER)).is_none());
}

#[test]
fn smapi_bundle_layout() {
    let r = GameRules::for_game(&game(Store::Steam, "413150", Runtime::Native));
    let files = list(&[
        "StardewModdingAPI",
        "StardewModdingAPI.dll",
        "unix-launcher.sh",
        "mcs/link",
        "smapi-internal/config.json",
        "Mods/SaveBackup/manifest.json",
    ]);
    let mut map = r.map_smapi_bundle(&files).unwrap();
    map.sort();
    assert_eq!(
        map,
        vec![
            ("Mods/SaveBackup/manifest.json".to_string(), "Mods/SaveBackup/manifest.json".to_string()),
            ("StardewModdingAPI".into(), "StardewModdingAPI".into()),
            ("StardewModdingAPI.dll".into(), "StardewModdingAPI.dll".into()),
            ("smapi-internal/config.json".into(), "smapi-internal/config.json".into()),
            ("unix-launcher.sh".into(), "StardewValley".into()),
        ]
    );
    assert_eq!(SMAPI_LAUNCHER, ("unix-launcher.sh", "StardewValley"));
    assert!(r.map_smapi_bundle(&list(&["readme.txt"])).is_err());
    let game_files = list(&["StardewValley", "StardewModdingAPI", "StardewModdingAPI.dll", "Mods/x/manifest.json"]);
    assert_eq!(r.executables(&game_files), list(&["StardewValley", "StardewModdingAPI"]));
    assert_eq!(r.game_copies(&game_files), vec![("Stardew Valley.deps.json".to_string(), "StardewModdingAPI.deps.json".to_string())]);
    // A normal SMAPI mod: nothing executable, nothing copied.
    assert!(r.executables(&list(&["Mods/Foo/StardewValley"])).is_empty());
    assert!(r.game_copies(&list(&["Mods/Foo/manifest.json"])).is_empty());
    // SMAPI itself doesn't "need SMAPI"; its bundled mods don't make it need itself.
    assert!(!r.needs_loader(&game_files));
    assert!(r.needs_loader(&list(&["Mods/Foo/manifest.json"])));
}

#[test]
fn loaders_never_need_themselves() {
    let sky = GameRules::for_game(&game(Store::Steam, "489830", Runtime::Windows));
    assert!(!sky.needs_loader(&list(&["skse64_loader.exe", "Data/SKSE/Plugins/x.dll"])));
    let val = GameRules::for_game(&game(Store::Steam, "892970", Runtime::Windows));
    assert!(!val.needs_loader(&list(&["winhttp.dll", "BepInEx/core/BepInEx.dll", "BepInEx/plugins/x.dll"])));
    assert!(val.needs_loader(&list(&["BepInEx/plugins/x.dll"])));
}

#[test]
fn launcher_swap_and_launch_guidance_come_from_the_tables() {
    let sky = GameRules::for_game(&game(Store::Steam, "489830", Runtime::Windows));
    assert_eq!(sky.launcher_swap, Some(("skse64_loader.exe".to_string(), "SkyrimSELauncher.exe".to_string())));
    assert!(sky.launch_option.is_none());
    let val = GameRules::for_game(&game(Store::Steam, "892970", Runtime::Native));
    assert_eq!(val.launch_option.as_deref(), Some("./start_game_bepinex.sh %command%"));
    assert!(val.launch_help.as_deref().unwrap().contains("once"));
    assert_eq!(val.executables(&list(&["start_game_bepinex.sh", "BepInEx/core/a.dll"])), list(&["start_game_bepinex.sh"]));
    let lc = GameRules::for_game(&game(Store::Steam, "1966720", Runtime::Windows));
    assert!(lc.launch_option.is_none() && lc.launcher_swap.is_none());
    let generic = GameRules::for_game(&game(Store::Steam, "70", Runtime::Native));
    assert!(generic.launch_help.is_none());
    assert!(!Path::new("/nowhere").exists());
}

#[test]
fn missing_master_lines() {
    let m = |p: &str, mm: &str, off: bool| MissingMaster { plugin: p.into(), master: mm.into(), turned_off: off };
    let checks = rules::master_checks(&[m("A.esp", "X.esm", false), m("A.esp", "Y.esm", false), m("A.esp", "Z.esp", true), m("B.esp", "X.esm", false)]);
    assert_eq!(checks.len(), 2);
    assert_eq!((checks[0].level.as_str(), checks[0].title.as_str()), ("stop", "Missing master"));
    assert_eq!(
        checks[0].detail,
        "A.esp needs X.esm and Y.esm, which aren't in the game's Data folder. Add the mod that has them, or turn off A.esp. \
         A.esp needs Z.esp, which is turned off. Turn it on in the Load order tab, or turn off A.esp."
    );
    assert!(rules::master_checks(&[]).is_empty());
    let mut list = vec![rules::Check { level: "ok".into(), title: "Ready to play".into(), detail: String::new() }];
    rules::add_checks(&mut list, checks);
    assert_eq!(list.len(), 2);
}
