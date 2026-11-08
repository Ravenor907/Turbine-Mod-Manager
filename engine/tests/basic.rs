//! Basic support: where a mod for a game without rules of its own goes (basic::place), the refused table, and the
//! store's placement flow (asking, remembering, moving, skipping the game's own files).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use turbine_engine::basic::{self, GameIndex, Placement};
use turbine_engine::games::{Game, Runtime, Store as Launcher};
use turbine_engine::rules::{GameRules, Support};
use turbine_engine::store::{Import, Store};
use turbine_engine::zip::crc32;

struct TempDir(PathBuf);
impl TempDir {
    fn new(tag: &str) -> TempDir {
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let p = std::env::temp_dir().join(format!("turbine-basic-{tag}-{}-{n}", std::process::id()));
        fs::create_dir_all(&p).unwrap();
        TempDir(p)
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write_zip(path: &Path, files: &[(&str, &str)]) {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in files {
        let offset = out.len() as u32;
        let crc = crc32(data.as_bytes());
        let len = data.len() as u32;
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&0x0800u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data.as_bytes());
        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&0x0800u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&[0; 4]);
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&len.to_le_bytes());
        central.extend_from_slice(&len.to_le_bytes());
        central.extend_from_slice(&(name.len() as u16).to_le_bytes());
        central.extend_from_slice(&[0; 8]);
        central.extend_from_slice(&0u32.to_le_bytes());
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
    }
    let cd_offset = out.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, out).unwrap();
}

/// A game folder with these folders and files (a path ending in '/' is a folder).
fn game_at(root: &Path, name: &str, folder: &str, store_id: &str, entries: &[&str]) -> Game {
    let dir = root.join("library").join(folder);
    fs::create_dir_all(&dir).unwrap();
    for e in entries {
        let p = dir.join(e.trim_end_matches('/'));
        if e.ends_with('/') {
            fs::create_dir_all(&p).unwrap();
        } else {
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, format!("game file {e}")).unwrap();
        }
    }
    Game {
        store: Launcher::Steam,
        store_id: store_id.into(),
        name: name.into(),
        install_dir: dir,
        library: root.join("library"),
        build: Some("1".into()),
        runtime: Runtime::Windows,
        cover: None,
        header: None,
        prefix: None,
    }
}

fn files(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn place(game: &Game, list: &[&str]) -> Placement {
    let rules = GameRules::for_game(game);
    let ix = GameIndex::build(&game.install_dir);
    basic::place(&files(list), "Cool Mod", game, &rules, &ix).unwrap()
}

fn sure(p: &Placement) -> (&str, &str, &str) {
    match p {
        Placement::Sure { from, to, rule } => (from, to, rule),
        other => panic!("not sure: {other:?}"),
    }
}

#[test]
fn unknown_games_get_basic_support() {
    let t = TempDir::new("support");
    let g = game_at(&t.0, "Brass Pier", "BrassPier", "3999902", &["Data/"]);
    let r = GameRules::for_game(&g);
    assert_eq!(r.support_level, Support::Basic);
    assert_eq!(r.support, "Basic support");
    assert!(r.places_by_detection());
}

#[test]
fn data_folder_mirror() {
    let t = TempDir::new("mirror");
    let g = game_at(&t.0, "Brass Pier", "BrassPier", "3999902", &["Data/", "BrassPier.exe"]);
    let p = place(&g, &["MyMod/Data/textures/a.dds", "MyMod/readme.txt"]);
    assert_eq!(sure(&p), ("MyMod", "", "data-folder"));
}

#[test]
fn data_contents() {
    let t = TempDir::new("contents");
    let g = game_at(&t.0, "Brass Pier", "BrassPier", "3999902", &["Data/meshes/"]);
    assert_eq!(sure(&place(&g, &["meshes/x.nif"])), ("", "Data", "data-contents"));
}

#[test]
fn bepinex_pack_to_root() {
    let t = TempDir::new("bep");
    let g = game_at(&t.0, "Some Unity Game", "SUG", "3999903", &["SUG_Data/"]);
    let p = place(&g, &["Pack/BepInEx/core/BepInEx.dll", "Pack/winhttp.dll", "Pack/doorstop_config.ini"]);
    assert_eq!(sure(&p), ("Pack", "", "bepinex"));
}

#[test]
fn bepinex_dll_only_needs_core() {
    let t = TempDir::new("bepdll");
    let g = game_at(&t.0, "Some Unity Game", "SUG", "3999903", &["SUG_Data/"]);
    assert!(matches!(place(&g, &["CoolMod.dll"]), Placement::Ask { .. }));
    let g2 = game_at(&t.0, "Other Unity Game", "OUG", "3999904", &["BepInEx/core/", "BepInEx/plugins/"]);
    assert_eq!(sure(&place(&g2, &["CoolMod.dll"])), ("", "BepInEx/plugins/Cool Mod", "bepinex-plugin"));
}

#[test]
fn top_folder_named_after_game() {
    let t = TempDir::new("named");
    let g = game_at(&t.0, "Hades II", "Hades II", "3999905", &["Content/"]);
    assert_eq!(sure(&place(&g, &["Hades II/Content/x.lua"])), ("Hades II", "", "mirrors-game"));
}

#[test]
fn game_name_match_ignores_punctuation() {
    assert_eq!(basic::squash("Baldur's Gate 3"), basic::squash("BaldursGate3"));
    let t = TempDir::new("punct");
    let g = game_at(&t.0, "Baldur's Gate 3", "Baldurs Gate 3", "3999906", &["Data/"]);
    assert_eq!(sure(&place(&g, &["BaldursGate3/Data/x.pak"])), ("BaldursGate3", "", "data-folder"));
}

#[test]
fn unwrap_keeps_meaningful_folders() {
    let t = TempDir::new("keep");
    let g = game_at(&t.0, "Brass Pier", "BrassPier", "3999902", &["Data/"]);
    // Data alone is never taken off: it mirrors the game's top.
    assert_eq!(sure(&place(&g, &["Data/x.esp"])), ("", "", "data-folder"));
}

#[test]
fn nested_match_unique() {
    let t = TempDir::new("nested1");
    let g = game_at(&t.0, "Deep Game", "Deep", "3999907", &["Game/Assets/Textures/", "Game/Assets/Sounds/"]);
    assert_eq!(sure(&place(&g, &["Textures/a.png.dds", "Sounds/b.ogg"])), ("", "Game/Assets", "game-folder"));
}

#[test]
fn nested_match_several_is_guess() {
    let t = TempDir::new("nested2");
    let g = game_at(&t.0, "Deep Game", "Deep", "3999907", &["Game/Assets/Textures/", "Game/Old/Textures/", "Alt/Textures/"]);
    match place(&g, &["Textures/a.dds"]) {
        Placement::Guess { to, others, rule, .. } => {
            assert_eq!(rule, "game-folder");
            assert_eq!(to, "Alt");
            assert_eq!(others, vec!["Game/Assets".to_string(), "Game/Old".to_string()]);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn unreal_paks_loose() {
    let t = TempDir::new("pak1");
    let g = game_at(&t.0, "Lantern Hollow", "LanternHollow", "3999901", &["Lantern/Content/Paks/", "Lantern/Binaries/Win64/Lantern.exe"]);
    assert_eq!(sure(&place(&g, &["WandGlow_P.pak", "WandGlow_P.utoc", "WandGlow_P.ucas", "readme.txt"])), ("", "Lantern/Content/Paks/~mods", "unreal-paks"));
}

#[test]
fn unreal_paks_logicmods() {
    let t = TempDir::new("pak2");
    let g = game_at(&t.0, "Lantern Hollow", "LanternHollow", "3999901", &["Lantern/Content/Paks/"]);
    assert_eq!(sure(&place(&g, &["LogicMods/Trick.pak"])), ("LogicMods", "Lantern/Content/Paks/LogicMods", "unreal-paks"));
}

#[test]
fn unreal_two_paks_folders_is_ask() {
    let t = TempDir::new("pak3");
    let g = game_at(&t.0, "Two Paks", "TwoPaks", "3999908", &["A/Content/Paks/", "B/Content/Paks/"]);
    assert!(matches!(place(&g, &["x.pak"]), Placement::Ask { .. }));
}

#[test]
fn loose_files_existing_once_is_guess() {
    let t = TempDir::new("loose1");
    let g = game_at(&t.0, "Old Game", "Old", "3999909", &["bin/settings.cfg", "bin/other.cfg", "Old.exe"]);
    match place(&g, &["settings.cfg"]) {
        Placement::Guess { to, rule, .. } => assert_eq!((to.as_str(), rule), ("bin", "loose-files")),
        other => panic!("{other:?}"),
    }
}

#[test]
fn loose_files_unknown_is_ask() {
    let t = TempDir::new("loose2");
    let g = game_at(&t.0, "Old Game", "Old", "3999909", &["Binaries/Win64/Old.exe", "Binaries/Win64/small.exe", "_CommonRedist/vcredist_big.exe"]);
    fs::write(g.install_dir.join("Binaries/Win64/Old.exe"), vec![0u8; 5000]).unwrap();
    fs::write(g.install_dir.join("_CommonRedist/vcredist_big.exe"), vec![0u8; 90000]).unwrap();
    assert_eq!(place(&g, &["mystery.bin"]), Placement::Ask { suggest_to: "Binaries/Win64".into() });
}

#[test]
fn mods_folder_keeps_mod_folder() {
    let t = TempDir::new("mods");
    let g = game_at(&t.0, "Brass Pier", "BrassPier", "3999902", &["Data/", "Mods/OldMod/", "BrassPier.exe"]);
    match place(&g, &["CoolMod/mod.json", "CoolMod/assets/a.png"]) {
        Placement::Guess { from, to, rule, .. } => assert_eq!((from.as_str(), to.as_str(), rule), ("CoolMod", "Mods/CoolMod", "mods-folder")),
        other => panic!("{other:?}"),
    }
}

#[test]
fn hint_beats_rules() {
    let t = TempDir::new("hint");
    let mut g = game_at(&t.0, "Hogwarts Legacy", "Hogwarts Legacy", "990080", &["Phoenix/Content/Paks/", "Other/Content/Paks/"]);
    g.store_id = "990080".into();
    let r = GameRules::for_game(&g);
    assert_eq!(r.support_level, Support::Basic);
    assert_eq!(r.nexus.as_deref(), Some("hogwartslegacy"));
    // Two Paks folders: the Unreal rule alone would ask; the data file's hint is sure.
    assert_eq!(sure(&place(&g, &["Wand_P.pak", "Wand_P.sig"])), ("", "Phoenix/Content/Paks/~mods", "data-file-hint"));
}

#[test]
fn readme_and_images_dropped_at_top_only() {
    let map = basic::map_placement(&files(&["readme.txt", "shot.png", "Mod/docs/readme.txt", "Mod/a.dat"]), "", "X").unwrap();
    let dests: Vec<&str> = map.iter().map(|(_, d)| d.as_str()).collect();
    assert_eq!(dests, vec!["X/Mod/docs/readme.txt", "X/Mod/a.dat"]);
}

#[test]
fn climbing_paths_refused() {
    let t = TempDir::new("climb");
    let g = game_at(&t.0, "Brass Pier", "BrassPier", "3999902", &["Data/"]);
    let rules = GameRules::for_game(&g);
    let ix = GameIndex::build(&g.install_dir);
    assert!(basic::place(&files(&["../x"]), "m", &g, &rules, &ix).is_err());
    assert!(basic::place(&files(&["/etc/passwd"]), "m", &g, &rules, &ix).is_err());
    assert!(basic::map_placement(&files(&["a.txt"]), "", "../up").is_err());
    let bad = r#"{"schema":1,"name":"X","ids":{"steam":["1"]},"layout":[{"match":"ext","values":["pak"],"to":"../"}]}"#;
    assert!(turbine_engine::gamedata::parse_game("x.json", bad, false).is_err());
}

#[test]
fn index_caps() {
    let t = TempDir::new("cap");
    let g = game_at(&t.0, "Huge Game", "Huge", "3999910", &[]);
    for i in 0..300 {
        for j in 0..100 {
            fs::create_dir_all(g.install_dir.join(format!("d{i}/e{j}"))).unwrap();
        }
    }
    let start = Instant::now();
    let ix = GameIndex::build(&g.install_dir);
    let took = start.elapsed();
    assert!(ix.capped);
    assert!(ix.dirs.len() + ix.files.len() <= basic::INDEX_CAP);
    assert!(took.as_millis() <= 350, "{took:?}");
    let rules = GameRules::for_game(&g);
    assert!(matches!(basic::place(&files(&["mystery.bin"]), "m", &g, &rules, &ix).unwrap(), Placement::Ask { .. }));
}

#[test]
fn refused_targets() {
    assert!(basic::refused("steam_api64.dll", false).is_some());
    assert!(basic::refused("bin/steam_api.dll", false).is_some());
    assert!(basic::refused("steamclient64.dll", false).is_some());
    assert!(basic::refused("EOSSDK-Win64-Shipping.dll", false).is_some());
    assert!(basic::refused("EasyAntiCheat/x.cfg", false).is_some());
    assert!(basic::refused("easyanticheat_eos/y", false).is_some());
    assert!(basic::refused("BattlEye/z.dll", false).is_some());
    assert!(basic::refused("start_protected_game.exe", false).is_some());
    assert!(basic::refused("Game.exe", true).is_some());
    assert!(basic::refused("NewTool.exe", false).is_none());
    assert!(basic::refused("Data/x.esp", true).is_none());

    // In the store: even with Replace them, refused files never go in, and the game's own exe stays.
    let t = TempDir::new("refused");
    let g = game_at(&t.0, "Brass Pier", "BrassPier", "3999902", &["Data/", "BrassPier.exe", "EasyAntiCheat/eac.cfg"]);
    let zip = t.0.join("dl/Overwrite 1.0.zip");
    write_zip(&zip, &[("BrassPier.exe", "mod exe"), ("steam_api64.dll", "x"), ("EasyAntiCheat/eac.cfg", "y"), ("Data/new.txt", "new")]);
    let mut s = Store::open(&g, &t.0.join("data")).unwrap();
    let pending = match s.import_archive(&zip, Default::default()).unwrap() {
        Import::NeedsPlace(p) => p,
        other => panic!("{other:?}"),
    };
    // steam_api64.dll isn't at the game's top, so this one asks; the page's check lists the refused files.
    assert!(!pending.sure);
    let (_, refused) = s.pending_conflicts(&pending, "", "").unwrap();
    let refused: Vec<&str> = refused.iter().map(|(r, _)| r.as_str()).collect();
    assert!(refused.contains(&"BrassPier.exe") && refused.contains(&"steam_api64.dll") && refused.contains(&"EasyAntiCheat/eac.cfg"), "{refused:?}");
    let done = s.finish_place(*pending, "", "", true).unwrap();
    assert!(done.notes.iter().any(|n| n.starts_with("Refused BrassPier.exe")));
    s.apply().unwrap();
    assert_eq!(fs::read_to_string(g.install_dir.join("BrassPier.exe")).unwrap(), "game file BrassPier.exe");
    assert_eq!(fs::read_to_string(g.install_dir.join("EasyAntiCheat/eac.cfg")).unwrap(), "game file EasyAntiCheat/eac.cfg");
    assert!(!g.install_dir.join("steam_api64.dll").exists());
    assert_eq!(fs::read_to_string(g.install_dir.join("Data/new.txt")).unwrap(), "new");
}

#[test]
fn skip_existing_then_replace_and_put_back() {
    let t = TempDir::new("skip");
    let g = game_at(&t.0, "Brass Pier", "BrassPier", "3999902", &["Data/brass.ini", "BrassPier.exe"]);
    let zip = t.0.join("dl/Brass Tweaks 1.0.zip");
    write_zip(&zip, &[("Data/brass.ini", "modded"), ("Data/extra.txt", "extra")]);
    let mut s = Store::open(&g, &t.0.join("data")).unwrap();
    let pending = match s.import_archive(&zip, Default::default()).unwrap() {
        Import::NeedsPlace(p) => p,
        other => panic!("{other:?}"),
    };
    assert_eq!(pending.conflicts, vec!["Data/brass.ini".to_string()]);
    let done = s.finish_place(*pending, "", "", false).unwrap();
    let id = done.item.id.clone();
    let r = s.apply().unwrap();
    assert_eq!(r.skipped_existing, vec!["Data/brass.ini".to_string()]);
    assert_eq!(fs::read_to_string(g.install_dir.join("Data/brass.ini")).unwrap(), "game file Data/brass.ini");
    assert_eq!(s.placement(&id).unwrap().skipped, vec!["Data/brass.ini".to_string()]);
    // Replace them: the game's file goes to the backups; turning the mod off puts it back byte for byte.
    s.set_replace(&id, true).unwrap();
    s.apply().unwrap();
    assert_eq!(fs::read_to_string(g.install_dir.join("Data/brass.ini")).unwrap(), "modded");
    s.remove(&id).unwrap();
    s.apply().unwrap();
    assert_eq!(fs::read_to_string(g.install_dir.join("Data/brass.ini")).unwrap(), "game file Data/brass.ini");
    assert!(!g.install_dir.join("Data/extra.txt").exists());
}

#[test]
fn remembered_placement_reused() {
    let t = TempDir::new("remember");
    let g = game_at(&t.0, "Odd Game", "Odd", "3999911", &["Odd.exe", "stuff/"]);
    let zip = t.0.join("dl/Weird Mod 1.0.zip");
    write_zip(&zip, &[("Weird/thing.bin", "1")]);
    let mut s = Store::open(&g, &t.0.join("data")).unwrap();
    let pending = match s.import_archive(&zip, Default::default()).unwrap() {
        Import::NeedsPlace(p) => p,
        other => panic!("{other:?}"),
    };
    assert!(!pending.sure);
    let done = s.finish_place(*pending, "Weird", "stuff/weird", false).unwrap();
    assert_eq!(done.placed.as_ref().unwrap().rule, "you");
    let id = done.item.id.clone();
    // The same layout again: no question.
    write_zip(&zip, &[("Weird/thing.bin", "2"), ("Weird/more.bin", "3")]);
    match s.replace_mod(&id, &zip, Default::default()).unwrap() {
        Import::Added(d) => assert_eq!(d.placed.unwrap().to, "stuff/weird"),
        other => panic!("{other:?}"),
    }
    assert!(s.root().join("mods").join(&id).join("stuff/weird/more.bin").is_file());
    // Its folder is gone in the new version: ask again.
    write_zip(&zip, &[("Other/thing.bin", "4")]);
    assert!(matches!(s.replace_mod(&id, &zip, Default::default()).unwrap(), Import::NeedsPlace(_)));
    // Remembered across opening the store again.
    drop(s);
    let s2 = Store::open(&g, &t.0.join("data")).unwrap();
    assert_eq!(s2.placement(&id).unwrap().from, "Weird");
}

#[test]
fn change_placement_moves_staged_files() {
    let t = TempDir::new("move");
    let g = game_at(&t.0, "Odd Game", "Odd", "3999911", &["Odd.exe", "A/", "B/"]);
    let zip = t.0.join("dl/Mover 1.0.zip");
    write_zip(&zip, &[("x/one.bin", "1"), ("x/deep/two.bin", "2")]);
    let mut s = Store::open(&g, &t.0.join("data")).unwrap();
    let pending = match s.import_archive(&zip, Default::default()).unwrap() {
        Import::NeedsPlace(p) => p,
        other => panic!("{other:?}"),
    };
    let id = s.finish_place(*pending, "", "A", false).unwrap().item.id;
    s.apply().unwrap();
    assert!(g.install_dir.join("A/x/deep/two.bin").is_file());
    s.change_placement(&id, "B/C").unwrap();
    let dir = s.root().join("mods").join(&id);
    assert_eq!(fs::read_to_string(dir.join("B/C/x/one.bin")).unwrap(), "1");
    assert_eq!(fs::read_to_string(dir.join("B/C/x/deep/two.bin")).unwrap(), "2");
    assert!(!dir.join("A").exists());
    s.apply().unwrap();
    assert!(g.install_dir.join("B/C/x/deep/two.bin").is_file());
    assert!(!g.install_dir.join("A/x").exists());
    assert!(s.change_placement(&id, "../out").is_err());
}

#[test]
fn sure_placement_adds_at_once() {
    let t = TempDir::new("sure");
    let g = game_at(&t.0, "Lantern Hollow", "LanternHollow", "3999901", &["Lantern/Content/Paks/", "Lantern/Binaries/Win64/Lantern.exe", "EasyAntiCheat/"]);
    let zip = t.0.join("dl/Wand Glow 1.0.zip");
    write_zip(&zip, &[("WandGlow_P.pak", "p"), ("readme.txt", "r")]);
    let mut s = Store::open(&g, &t.0.join("data")).unwrap();
    match s.import_archive(&zip, Default::default()).unwrap() {
        Import::Added(d) => {
            let p = d.placed.unwrap();
            assert_eq!((p.to.as_str(), p.rule.as_str()), ("Lantern/Content/Paks/~mods", "unreal-paks"));
        }
        other => panic!("{other:?}"),
    }
    let ix = GameIndex::build(&g.install_dir);
    assert_eq!(ix.anti_cheat(), Some("Easy Anti-Cheat"));
}

#[test]
fn folder_lists_for_the_page() {
    let t = TempDir::new("lists");
    let g = game_at(&t.0, "Odd Game", "Odd", "3999911", &["Odd.exe", "b/", "A/x/"]);
    let s = Store::open(&g, &t.0.join("data")).unwrap();
    let j = s.game_folders_json("");
    assert!(j.starts_with("[{\"name\":\"A\""), "{j}");
    assert!(j.contains("\"name\":\"b\""));
    let tree = basic::archive_tree_json(&files(&["M/a.txt", "M/sub/b.txt", "c.txt"]));
    assert!(tree.contains("{\"path\":\"\",\"files\":3}") && tree.contains("{\"path\":\"M\",\"files\":2}"), "{tree}");
}
