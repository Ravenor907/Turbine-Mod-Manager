//! Game rules: archive layouts for every kind of game, loader detection, the check before Play and launch links.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use turbine_engine::rules::{self, pre_play_with, GameRules, Kind};
use turbine_engine::{Game, Runtime, Store};

fn game(store: Store, id: &str, name: &str, dir: &Path) -> Game {
    Game {
        store,
        store_id: id.into(),
        name: name.into(),
        install_dir: dir.to_path_buf(),
        library: dir.to_path_buf(),
        build: None,
        runtime: Runtime::Windows,
        cover: None,
        header: None,
        prefix: None,
    }
}

fn rules_for(store: Store, id: &str) -> GameRules {
    GameRules::for_game(&game(store, id, "Test", Path::new("/nowhere")))
}

fn skyrim() -> GameRules {
    rules_for(Store::Steam, "489830")
}
fn valheim() -> GameRules {
    rules_for(Store::Steam, "892970")
}
fn stardew() -> GameRules {
    rules_for(Store::Steam, "413150")
}
fn generic() -> GameRules {
    rules_for(Store::Steam, "70")
}

fn list(files: &[&str]) -> Vec<String> {
    files.iter().map(|s| s.to_string()).collect()
}

/// Archive path -> game path, for easy comparing.
fn map(rules: &GameRules, files: &[&str], name: &str) -> BTreeMap<String, String> {
    rules.map_archive(&list(files), name).unwrap().into_iter().collect()
}

fn dests(rules: &GameRules, files: &[&str], name: &str) -> Vec<String> {
    let mut d: Vec<String> = map(rules, files, name).into_values().collect();
    d.sort();
    d
}

struct TempDir(PathBuf);
impl TempDir {
    fn new(tag: &str) -> TempDir {
        let d = std::env::temp_dir().join(format!("turbine-rules-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        TempDir(d)
    }
    fn touch(&self, rel: &str) {
        let p = self.0.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, "x").unwrap();
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

// ----- game table ----------------------------------------------------------------------------------------------

#[test]
fn known_games_get_their_rules() {
    let s = skyrim();
    assert_eq!((s.kind, s.mod_root.as_str(), s.plugins, s.ordered), (Kind::Bethesda, "Data", true, true));
    assert_eq!(s.support, "Full support");
    assert!(s.loader.as_ref().unwrap().name.starts_with("SKSE"));
    assert_eq!(s.loader.as_ref().unwrap().source, "https://skse.silverlock.org/");

    let sv = stardew();
    assert_eq!((sv.kind, sv.mod_root.as_str()), (Kind::Smapi, "Mods"));
    assert_eq!(rules_for(Store::Gog, "1453375253").kind, Kind::Smapi);
    assert_eq!(sv.loader.as_ref().unwrap().source, "https://smapi.io/");

    for id in ["892970", "1966720", "632360"] {
        let r = rules_for(Store::Steam, id);
        assert_eq!(r.kind, Kind::BepInEx, "{id}");
        assert_eq!(r.mod_root, "BepInEx/plugins");
        assert!(!r.plugins && !r.ordered);
        assert!(r.loader.as_ref().unwrap().source.contains("thunderstore.io"), "{id}");
    }
    assert!(valheim().loader.unwrap().source.contains("BepInExPack_Valheim"));
}

#[test]
fn unknown_games_work_generically() {
    let g = generic();
    assert_eq!(g.kind, Kind::Generic);
    assert_eq!(g.mod_root, "");
    assert_eq!(g.support, "Basic support");
    assert!(g.loader.is_none());
    assert!(!g.plugins);
    // Same number under another store is a different game.
    assert_eq!(rules_for(Store::Epic, "489830").kind, Kind::Generic);
    assert_eq!(rules_for(Store::Gog, "413150").kind, Kind::Generic);
}

#[test]
fn every_table_loader_exists() {
    for g in rules::games() {
        if let Some(l) = g.loader {
            assert!(rules::find_loader(l).is_some(), "{} uses unknown loader {l}", g.name);
        }
    }
}

// ----- Bethesda --------------------------------------------------------------------------------------------------

#[test]
fn skyui_style_loose_files_go_to_data() {
    let m = map(
        &skyrim(),
        &[
            "SkyUI_5_2_SE/SkyUI_SE.bsa",
            "SkyUI_5_2_SE/SkyUI_SE.esp",
            "SkyUI_5_2_SE/interface/skyui/config.txt",
            "SkyUI_5_2_SE/SKSE/Plugins/SkyUI.ini",
            "SkyUI_5_2_SE/readme.txt",
            "SkyUI_5_2_SE/Changelog.md",
            "SkyUI_5_2_SE/screenshot1.jpg",
            "SkyUI_5_2_SE/",
        ],
        "SkyUI",
    );
    let want: BTreeMap<String, String> = [
        ("SkyUI_5_2_SE/SkyUI_SE.bsa", "Data/SkyUI_SE.bsa"),
        ("SkyUI_5_2_SE/SkyUI_SE.esp", "Data/SkyUI_SE.esp"),
        ("SkyUI_5_2_SE/interface/skyui/config.txt", "Data/interface/skyui/config.txt"),
        ("SkyUI_5_2_SE/SKSE/Plugins/SkyUI.ini", "Data/SKSE/Plugins/SkyUI.ini"),
    ]
    .iter()
    .map(|(a, b)| (a.to_string(), b.to_string()))
    .collect();
    assert_eq!(m, want);
}

#[test]
fn data_wrapped_mod_any_case() {
    assert_eq!(
        dests(&skyrim(), &["Cool Mod/data/meshes/armor/a.nif", "Cool Mod/data/CoolMod.esp", "Cool Mod/data/Textures/a.dds"], "x"),
        vec!["Data/CoolMod.esp", "Data/Textures/a.dds", "Data/meshes/armor/a.nif"]
    );
    assert_eq!(dests(&skyrim(), &["DATA/Scripts/a.pex"], "x"), vec!["Data/Scripts/a.pex"]);
}

#[test]
fn skse_package_splits_game_folder_and_data() {
    let files = list(&[
        "skse64_2_02_06/skse64_loader.exe",
        "skse64_2_02_06/skse64_1_6_1170.dll",
        "skse64_2_02_06/skse64_readme.txt",
        "skse64_2_02_06/skse64_whatsnew.txt",
        "skse64_2_02_06/Data/Scripts/Actor.pex",
        "skse64_2_02_06/Data/Scripts/Source/Actor.psc",
        "skse64_2_02_06/src/skse64/skse64/main.cpp",
    ]);
    let mut d: Vec<String> = skyrim().map_archive(&files, "SKSE").unwrap().into_iter().map(|(_, to)| to).collect();
    d.sort();
    // The loader also goes in as the launcher Steam's Play starts.
    assert_eq!(
        d,
        vec![
            "Data/Scripts/Actor.pex",
            "Data/Scripts/Source/Actor.psc",
            "SkyrimSELauncher.exe",
            "skse64_1_6_1170.dll",
            "skse64_loader.exe"
        ]
    );
}

#[test]
fn skse_plugin_with_dll_needs_loader() {
    let r = skyrim();
    let d = dests(&r, &["Engine Fixes/SKSE/Plugins/EngineFixes.dll", "Engine Fixes/SKSE/Plugins/EngineFixes.toml"], "x");
    assert_eq!(d, vec!["Data/SKSE/Plugins/EngineFixes.dll", "Data/SKSE/Plugins/EngineFixes.toml"]);
    assert!(r.needs_loader(&d));
    assert!(r.needs_loader(&list(&["data/skse/plugins/x.DLL"])));
    assert!(!r.needs_loader(&list(&["Data/SkyUI_SE.esp", "Data/SKSE/Plugins/SkyUI.ini"])));
}

#[test]
fn enb_style_binaries_go_to_game_folder() {
    let d = dests(&skyrim(), &["d3d11.dll", "d3dcompiler_46e.dll", "enblocal.ini", "enbseries/effect.txt"], "ENB");
    assert_eq!(d, vec!["d3d11.dll", "d3dcompiler_46e.dll", "enblocal.ini", "enbseries/effect.txt"]);
}

#[test]
fn fomod_only_archive_is_a_clear_error() {
    let err = skyrim()
        .map_archive(
            &list(&[
                "fomod/ModuleConfig.xml",
                "fomod/info.xml",
                "fomod/images/a.png",
                "00 Core/meshes/a.nif",
                "01 Option A/textures/a.dds",
                "readme.txt",
            ]),
            "x",
        )
        .unwrap_err();
    // The store opens installers before mapping; only an installer folder without its file gets here.
    assert!(err.contains("installer") && !err.contains("later build"), "{err}");
    // Wrapped in a folder: the same.
    assert!(skyrim().map_archive(&list(&["Mod/fomod/ModuleConfig.xml", "Mod/Core/a.esp"]), "x").is_err());
}

#[test]
fn fomod_with_default_content_installs_that() {
    let d = dests(
        &skyrim(),
        &["fomod/ModuleConfig.xml", "Mod.esp", "meshes/a.nif", "Optional/textures/b.dds", "Data/scripts/c.pex"],
        "x",
    );
    assert_eq!(d, vec!["Data/Mod.esp", "Data/meshes/a.nif", "Data/scripts/c.pex"]);
}

#[test]
fn bethesda_drops_screenshots_and_keeps_unknown_data_folders() {
    let d = dests(&skyrim(), &["Mod.esp", "Screenshots/1.png", "CalienteTools/BodySlide/x.xml", "MyThing/a.json"], "x");
    assert_eq!(d, vec!["Data/CalienteTools/BodySlide/x.xml", "Data/Mod.esp", "Data/MyThing/a.json"]);
}

// ----- BepInEx ---------------------------------------------------------------------------------------------------

#[test]
fn thunderstore_bepinexpack_goes_to_game_folder() {
    let d = dests(
        &valheim(),
        &[
            "manifest.json",
            "icon.png",
            "README.md",
            "BepInExPack_Valheim/winhttp.dll",
            "BepInExPack_Valheim/doorstop_config.ini",
            "BepInExPack_Valheim/changelog.txt",
            "BepInExPack_Valheim/start_game_bepinex.sh",
            "BepInExPack_Valheim/BepInEx/core/BepInEx.dll",
            "BepInExPack_Valheim/BepInEx/config/BepInEx.cfg",
            "BepInExPack_Valheim/doorstop_libs/libdoorstop_x64.so",
        ],
        "denikson-BepInExPack_Valheim",
    );
    assert_eq!(
        d,
        vec![
            "BepInEx/config/BepInEx.cfg",
            "BepInEx/core/BepInEx.dll",
            "doorstop_config.ini",
            "doorstop_libs/libdoorstop_x64.so",
            "start_game_bepinex.sh",
            "winhttp.dll",
        ]
    );
}

#[test]
fn bepinex_release_zip_goes_as_is() {
    let d = dests(
        &valheim(),
        &["winhttp.dll", "doorstop_config.ini", ".doorstop_version", "changelog.txt", "BepInEx/core/BepInEx.dll"],
        "BepInEx",
    );
    assert_eq!(d, vec![".doorstop_version", "BepInEx/core/BepInEx.dll", "doorstop_config.ini", "winhttp.dll"]);
}

#[test]
fn thunderstore_plugin_package() {
    let r = rules_for(Store::Steam, "1966720");
    let m = map(
        &r,
        &[
            "manifest.json",
            "icon.png",
            "README.md",
            "CHANGELOG.md",
            "plugins/MoreCompany/MoreCompany.dll",
            "plugins/MoreCompany/assets/bundle",
            "patchers/Fixer.dll",
            "config/MoreCompany.cfg",
            "core/Lib.dll",
        ],
        "notnotnotswipez-MoreCompany",
    );
    let d: Vec<&str> = m.values().map(String::as_str).collect();
    assert_eq!(
        d,
        vec![
            "BepInEx/config/MoreCompany.cfg",
            "BepInEx/core/notnotnotswipez-MoreCompany/Lib.dll",
            "BepInEx/patchers/notnotnotswipez-MoreCompany/Fixer.dll",
            "BepInEx/plugins/notnotnotswipez-MoreCompany/MoreCompany/MoreCompany.dll",
            "BepInEx/plugins/notnotnotswipez-MoreCompany/MoreCompany/assets/bundle",
        ]
    );
    let all: Vec<String> = m.into_values().collect();
    assert!(r.needs_loader(&all));
}

#[test]
fn thunderstore_loose_dll_package() {
    let d = dests(
        &rules_for(Store::Steam, "632360"),
        &["manifest.json", "icon.png", "README.md", "R2API.dll", "R2API.xml", "Hook.mm.dll", "My.cfg"],
        "tristanmcpherson-R2API",
    );
    assert_eq!(
        d,
        vec![
            "BepInEx/config/My.cfg",
            "BepInEx/monomod/tristanmcpherson-R2API/Hook.mm.dll",
            "BepInEx/plugins/tristanmcpherson-R2API/R2API.dll",
            "BepInEx/plugins/tristanmcpherson-R2API/R2API.xml",
        ]
    );
}

#[test]
fn nexus_bepinex_zips() {
    // Loose dll in a wrapper folder.
    assert_eq!(dests(&valheim(), &["PlantEverything/PlantEverything.dll", "PlantEverything/readme.txt"], "PlantEverything"), vec![
        "BepInEx/plugins/PlantEverything/PlantEverything.dll"
    ]);
    // Full BepInEx path in the archive: merged into the game folder.
    assert_eq!(
        dests(&valheim(), &["ValheimPlus/BepInEx/plugins/ValheimPlus.dll", "ValheimPlus/BepInEx/config/valheim_plus.cfg"], "VP"),
        vec!["BepInEx/config/valheim_plus.cfg", "BepInEx/plugins/ValheimPlus.dll"]
    );
    // Case of the archive kept below the BepInEx folder.
    assert_eq!(dests(&valheim(), &["bepinex/Plugins/Foo/Foo.dll"], "Foo"), vec!["BepInEx/Plugins/Foo/Foo.dll"]);
}

#[test]
fn bepinex_needs_loader() {
    let r = valheim();
    assert!(r.needs_loader(&list(&["BepInEx/plugins/x/x.dll"])));
    assert!(r.needs_loader(&list(&["bepinex/patchers/x.dll"])));
    assert!(!r.needs_loader(&list(&["BepInEx/config/x.cfg"])));
}

// ----- SMAPI -----------------------------------------------------------------------------------------------------

#[test]
fn smapi_mod_folder() {
    let d = dests(
        &stardew(),
        &["Lookup Anything 1.40/LookupAnything/manifest.json", "Lookup Anything 1.40/LookupAnything/LookupAnything.dll", "Lookup Anything 1.40/LookupAnything/i18n/de.json", "Lookup Anything 1.40/readme.txt"],
        "Lookup Anything",
    );
    assert_eq!(d, vec!["Mods/LookupAnything/LookupAnything.dll", "Mods/LookupAnything/i18n/de.json", "Mods/LookupAnything/manifest.json"]);
}

#[test]
fn smapi_multi_mod_zip() {
    let d = dests(
        &stardew(),
        &[
            "[CP] Seasonal Outfits/manifest.json",
            "[CP] Seasonal Outfits/content.json",
            "[CP] Seasonal Outfits/assets/a.png",
            "Seasonal Outfits Config/manifest.json",
            "Seasonal Outfits Config/SOC.dll",
            "Seasonal Outfits Config/data/manifest.json",
            "README.txt",
        ],
        "Seasonal",
    );
    assert_eq!(
        d,
        vec![
            "Mods/Seasonal Outfits Config/SOC.dll",
            "Mods/Seasonal Outfits Config/data/manifest.json",
            "Mods/Seasonal Outfits Config/manifest.json",
            "Mods/[CP] Seasonal Outfits/assets/a.png",
            "Mods/[CP] Seasonal Outfits/content.json",
            "Mods/[CP] Seasonal Outfits/manifest.json",
        ]
    );
}

#[test]
fn smapi_manifest_at_top_uses_mod_name() {
    let d = dests(&stardew(), &["manifest.json", "Mod.dll", "assets/x.png"], "Better Crafting");
    assert_eq!(d, vec!["Mods/Better Crafting/Mod.dll", "Mods/Better Crafting/assets/x.png", "Mods/Better Crafting/manifest.json"]);
    // A Mods folder inside the archive.
    assert_eq!(dests(&stardew(), &["Mods/Foo/manifest.json"], "x"), vec!["Mods/Foo/manifest.json"]);
}

#[test]
fn smapi_installer_is_refused() {
    let err = stardew()
        .map_archive(
            &list(&[
                "SMAPI 4.1.10 installer/install on Linux.sh",
                "SMAPI 4.1.10 installer/install on Windows.bat",
                "SMAPI 4.1.10 installer/install on macOS.command",
                "SMAPI 4.1.10 installer/internal/linux/install.dat",
                "SMAPI 4.1.10 installer/internal/linux/SMAPI.Installer.dll",
                "SMAPI 4.1.10 installer/README.txt",
            ]),
            "SMAPI",
        )
        .unwrap_err();
    assert!(err.contains("SMAPI") && err.contains("installer"), "{err}");
}

#[test]
fn smapi_without_manifest_is_an_error() {
    assert!(stardew().map_archive(&list(&["Content/Portraits/Abigail.xnb"]), "x").is_err());
}

#[test]
fn smapi_needs_loader() {
    assert!(stardew().needs_loader(&list(&["Mods/Foo/manifest.json"])));
    assert!(!stardew().needs_loader(&list(&["Content/x.xnb"])));
}

// ----- Generic and safety ----------------------------------------------------------------------------------------

#[test]
fn generic_unwraps_one_wrapper() {
    assert_eq!(dests(&generic(), &["MyMod-1.2/bin/x.pak", "MyMod-1.2/readme.txt"], "x"), vec!["bin/x.pak", "readme.txt"]);
    assert_eq!(dests(&generic(), &["Outer/Inner/x.pak"], "x"), vec!["Inner/x.pak"]);
    assert_eq!(dests(&generic(), &["a.pak", "Sub/b.pak"], "x"), vec!["Sub/b.pak", "a.pak"]);
    assert!(!generic().needs_loader(&list(&["anything.dll"])));
}

#[test]
fn paths_are_cleaned_and_never_escape() {
    assert!(generic().map_archive(&list(&["a/../../etc/passwd"]), "x").is_err());
    assert!(skyrim().map_archive(&list(&["..\\x.esp"]), "x").is_err());
    let d = dests(&generic(), &["Mod\\sub\\a.txt", "./Mod/b.txt", "__MACOSX/Mod/._a.txt", "Mod/.DS_Store"], "x");
    assert_eq!(d, vec!["b.txt", "sub/a.txt"]);
    assert!(generic().map_archive(&list(&["__MACOSX/x"]), "x").is_err());
    // A mod name with a slash can't make a path.
    let d = dests(&valheim(), &["a.dll"], "../evil/x");
    assert_eq!(d, vec!["BepInEx/plugins/_evil_x/a.dll"]);
}

// ----- Loaders ---------------------------------------------------------------------------------------------------

#[test]
fn loader_detection_ignores_case() {
    let t = TempDir::new("skse");
    let g = game(Store::Steam, "489830", "Skyrim Special Edition", &t.0);
    let r = GameRules::for_game(&g);
    assert!(!r.loader_installed(&g));
    t.touch("SKSE64_Loader.EXE");
    assert!(r.loader_installed(&g));

    let t = TempDir::new("bepinex");
    let g = game(Store::Steam, "892970", "Valheim", &t.0);
    let r = GameRules::for_game(&g);
    t.touch("winhttp.dll");
    assert!(!r.loader_installed(&g), "core folder missing");
    t.touch("bepinex/Core/BepInEx.dll");
    assert!(r.loader_installed(&g));

    let t = TempDir::new("smapi");
    let g = game(Store::Gog, "1453375253", "Stardew Valley", &t.0);
    let r = GameRules::for_game(&g);
    t.touch("Stardew Valley");
    assert!(!r.loader_installed(&g));
    t.touch("StardewModdingAPI");
    assert!(r.loader_installed(&g));
    fs::remove_file(t.0.join("StardewModdingAPI")).unwrap();
    t.touch("StardewModdingAPI.exe");
    assert!(r.loader_installed(&g));

    // No loader: nothing to install.
    let g = game(Store::Steam, "70", "Half-Life", &t.0);
    assert!(GameRules::for_game(&g).loader_installed(&g));
}

// ----- Before Play -----------------------------------------------------------------------------------------------

fn with_build(mut g: Game, b: &str) -> Game {
    g.build = Some(b.into());
    g
}

const GB: u64 = 1024 * 1024 * 1024;

#[test]
fn all_fine_is_ready_to_play() {
    let g = with_build(game(Store::Steam, "489830", "Skyrim Special Edition", Path::new("/g")), "100");
    let c = pre_play_with(&g, &skyrim(), Some("100"), true, Some(10 * GB), Some("ext4"), true);
    assert_eq!(c.len(), 1);
    assert_eq!((c[0].level.as_str(), c[0].title.as_str()), ("ok", "Ready to play"));
    // Unknown build or space: nothing to say.
    let c = pre_play_with(&game(Store::Steam, "70", "X", Path::new("/g")), &generic(), None, false, None, None, true);
    assert_eq!(c[0].level, "ok");
}

#[test]
fn skyrim_update_with_skse_mods_stops() {
    let g = with_build(game(Store::Steam, "489830", "Skyrim Special Edition", Path::new("/g")), "200");
    let c = pre_play_with(&g, &skyrim(), Some("100"), true, Some(10 * GB), Some("ext4"), true);
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].level, "stop");
    assert_eq!(c[0].title, "Steam updated this game");
    assert!(c[0].detail.contains("Play without mods is safe"));
    // Without script mods it's only a warning.
    let c = pre_play_with(&g, &skyrim(), Some("100"), false, None, None, true);
    assert_eq!(c[0].level, "warn");
}

#[test]
fn bepinex_and_smapi_updates_warn() {
    let g = with_build(game(Store::Steam, "892970", "Valheim", Path::new("/g")), "2");
    let c = pre_play_with(&g, &valheim(), Some("1"), true, None, None, true);
    assert_eq!((c.len(), c[0].level.as_str()), (1, "warn"));
    let g = with_build(game(Store::Gog, "1453375253", "Stardew Valley", Path::new("/g")), "1.6.15");
    let c = pre_play_with(&g, &stardew(), Some("1.6.14"), true, None, None, true);
    assert_eq!(c[0].level, "warn");
    assert_eq!(c[0].title, "Heroic updated this game");
}

#[test]
fn missing_loader() {
    let g = game(Store::Steam, "489830", "Skyrim Special Edition", Path::new("/g"));
    let c = pre_play_with(&g, &skyrim(), None, true, None, None, false);
    assert_eq!(c[0].level, "stop");
    assert!(c[0].title.contains("SKSE") && c[0].detail.contains("skse.silverlock.org"));
    // Not needed by any mod: fine.
    assert_eq!(pre_play_with(&g, &skyrim(), None, false, None, None, false)[0].level, "ok");

    let c = pre_play_with(&g, &stardew(), None, true, None, None, false);
    assert_eq!(c[0].level, "stop");
    let c = pre_play_with(&g, &valheim(), None, true, None, None, false);
    assert_eq!(c[0].level, "warn");
}

#[test]
fn low_space_and_exfat_warn_in_order() {
    let g = with_build(game(Store::Steam, "489830", "Skyrim Special Edition", Path::new("/g")), "2");
    let c = pre_play_with(&g, &skyrim(), Some("1"), true, Some(300 * 1024 * 1024), Some("exfat"), false);
    let levels: Vec<(&str, &str)> = c.iter().map(|c| (c.level.as_str(), c.title.as_str())).collect();
    assert_eq!(levels.len(), 4);
    assert_eq!(levels[0], ("stop", "Steam updated this game"));
    assert_eq!(levels[1].0, "stop");
    assert_eq!(levels[2], ("warn", "Low space on the game's drive"));
    assert_eq!(levels[3], ("warn", "Game is on an exFAT drive"));
    assert!(c[2].detail.contains("300 MB"));
    // Exactly 1 GB free is enough.
    assert_eq!(pre_play_with(&g, &skyrim(), Some("2"), false, Some(GB), None, true)[0].level, "ok");
}

#[test]
fn pre_play_reads_real_system() {
    // Runs against this machine's /proc/mounts; only checks it doesn't fail.
    let g = game(Store::Steam, "70", "X", Path::new("/"));
    assert!(!rules::pre_play(&g, &generic(), None, false, Some(10 * GB)).is_empty());
}

const MOUNTS: &str = "\
/dev/nvme0n1p8 / btrfs ro,relatime 0 0
proc /proc proc rw,nosuid 0 0
/dev/nvme0n1p8 /home ext4 rw,relatime 0 0
/dev/mmcblk0p1 /run/media/deck/SD\\040Card exfat rw,nosuid,nodev,relatime,uid=1000 0 0
/dev/sda1 /run/media/deck/SD fuseblk rw 0 0
/dev/sdb1 /mnt/games ext4 rw 0 0
/dev/sdc1 /mnt/games/exfat fuse.exfat rw 0 0
";

#[test]
fn exfat_from_proc_mounts() {
    let sd = "/run/media/deck/SD Card/steamapps/common/Skyrim Special Edition";
    assert_eq!(rules::fs_type_for(MOUNTS, sd).as_deref(), Some("exfat"));
    assert!(rules::is_exfat(MOUNTS, sd));
    assert!(!rules::is_exfat(MOUNTS, "/home/deck/.local/share/Steam/steamapps/common/Valheim"));
    assert_eq!(rules::fs_type_for(MOUNTS, "/home/deck").as_deref(), Some("ext4"));
    // "/run/media/deck/SD" must not swallow "/run/media/deck/SDX".
    assert_eq!(rules::fs_type_for(MOUNTS, "/run/media/deck/SDX/x").as_deref(), Some("btrfs"));
    assert_eq!(rules::fs_type_for(MOUNTS, "/run/media/deck/SD/x").as_deref(), Some("fuseblk"));
    assert!(rules::is_exfat(MOUNTS, "/mnt/games/exfat/Game"));
    assert!(!rules::is_exfat(MOUNTS, "/mnt/games/exfatter"));
    assert_eq!(rules::fs_type_for("", "/x"), None);
}

// ----- Launching -------------------------------------------------------------------------------------------------

#[test]
fn launch_urls() {
    let p = Path::new("/g");
    assert_eq!(rules::launch_url(&game(Store::Steam, "489830", "S", p)).unwrap(), "steam://rungameid/489830");
    assert_eq!(
        rules::launch_url(&game(Store::Epic, "Fortnite", "F", p)).unwrap(),
        "heroic://launch?appName=Fortnite&runner=legendary"
    );
    assert_eq!(
        rules::launch_url(&game(Store::Gog, "1453375253", "SV", p)).unwrap(),
        "heroic://launch?appName=1453375253&runner=gog"
    );
    assert_eq!(
        rules::launch_url(&game(Store::Sideload, "a b&c", "X", p)).unwrap(),
        "heroic://launch?appName=a%20b%26c&runner=sideload"
    );
    assert_eq!(rules::launch_url(&game(Store::Steam, "", "X", p)), None);
}

// ----- Names -----------------------------------------------------------------------------------------------------

#[test]
fn short_names_for_the_header() {
    // Steam calls it "The Elder Scrolls V: Skyrim Special Edition", too long for a handheld's header.
    assert_eq!(skyrim().short_name.as_deref(), Some("Skyrim Special Edition"));
    assert_eq!(valheim().short_name, None);
    assert_eq!(generic().short_name, None);
}
