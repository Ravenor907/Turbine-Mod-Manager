//! 0.57 anti-cheat games: the data files, the store deploying a me3 game's mods into its Turbine-owned me3 profile
//! (never the game folder), the game folder made original again for the normal launch, and online-only games
//! refusing mods.

use std::fs;
use std::path::{Path, PathBuf};
use turbine_engine::anticheat::{self, AcState, Mode};
use turbine_engine::games::{Game, Runtime, Store as Launcher};
use turbine_engine::gamedata::{self, parse_game};
use turbine_engine::rules::GameRules;
use turbine_engine::store::{ModRules, Store};

fn write(p: &Path, text: &str) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

/// A zip with stored entries (enough for the store's reader).
fn write_zip(path: &Path, files: &[(&str, &str)]) {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in files {
        let offset = out.len() as u32;
        let crc = turbine_engine::zip::crc32(data.as_bytes());
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

/// Rules like Elden Ring's: files go where the archive has them; me3 starts the game.
struct Me3Rules(Option<&'static str>, bool);

impl ModRules for Me3Rules {
    fn map_archive(&self, files: &[String], _mod_name: &str) -> Result<Vec<(String, String)>, String> {
        Ok(files.iter().map(|f| (f.clone(), f.clone())).collect())
    }
    fn needs_loader(&self, _game_files: &[String]) -> bool {
        false
    }
    fn me3_game(&self) -> Option<String> {
        self.0.map(str::to_string)
    }
    fn refuses_mods(&self) -> Option<String> {
        self.1.then(|| anticheat::REFUSE_WORDS.to_string())
    }
}

struct World {
    root: PathBuf,
    game: Game,
}

fn world(tag: &str) -> World {
    let root = std::env::temp_dir().join(format!("turbine-ac-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let library = root.join("library");
    let install = library.join("steamapps/common/ELDEN RING");
    write(&install.join("Game/eldenring.exe"), "the game");
    write(&install.join("Game/start_protected_game.exe"), "the anti-cheat starter");
    write(&install.join("Game/regulation.bin"), "the game's own regulation");
    let game = Game {
        store: Launcher::Steam,
        store_id: "1245620".into(),
        name: "ELDEN RING".into(),
        install_dir: install,
        library,
        build: Some("100".into()),
        runtime: Runtime::Windows,
        cover: None,
        header: None,
        prefix: None,
    };
    World { root, game }
}

fn files_under(dir: &Path) -> Vec<String> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
        for e in fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(root, &p, out);
            } else {
                out.push(p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut out = Vec::new();
    if dir.is_dir() {
        walk(dir, dir, &mut out);
    }
    out.sort();
    out
}

#[test]
fn me3_mods_never_go_into_the_game_folder() {
    let w = world("profile");
    let before = files_under(&w.game.install_dir);
    let mut store = Store::open_at(&w.game, &w.root.join("store"), Box::new(Me3Rules(Some("eldenring"), false))).unwrap();
    let zip = w.root.join("dl/Better Regulation 1.0.zip");
    write_zip(&zip, &[("Game/regulation.bin", "modded regulation"), ("Game/SkipIntro.dll", "dll")]);
    store.import_zip(&zip).unwrap();
    store.apply().unwrap();
    // The game folder is exactly as it was; the mod is in the profile's package folder, without the Game/ prefix.
    assert_eq!(files_under(&w.game.install_dir), before);
    assert_eq!(fs::read_to_string(w.game.install_dir.join("Game/regulation.bin")).unwrap(), "the game's own regulation");
    let mods = anticheat::profile_mods(store.root());
    assert_eq!(files_under(&mods), vec!["SkipIntro.dll".to_string(), "regulation.bin".to_string()]);
    let (profile, natives) = anticheat::write_profile(store.root(), "eldenring").unwrap();
    assert_eq!(natives, vec!["mods/SkipIntro.dll".to_string()]);
    assert!(fs::read_to_string(profile).unwrap().contains("savefile = 'TurbineModded.sl2'"));
    // Play without mods leaves the profile alone and the game folder original.
    store.play_original().unwrap();
    assert_eq!(files_under(&w.game.install_dir), before);
    assert_eq!(store.files_in_game_folder(), 0);
    let _ = fs::remove_dir_all(&w.root);
}

#[test]
fn files_placed_before_come_out_of_the_game_folder() {
    let w = world("leftover");
    let before = files_under(&w.game.install_dir);
    let store_root = w.root.join("store");
    // Before 0.57 the game was a Basic game: the mod went into the game folder.
    {
        let mut old = Store::open_at(&w.game, &store_root, Box::new(Me3Rules(None, false))).unwrap();
        let zip = w.root.join("dl/Old Mod 1.0.zip");
        write_zip(&zip, &[("Game/oldmod.txt", "old")]);
        old.import_zip(&zip).unwrap();
        old.apply().unwrap();
        assert!(w.game.install_dir.join("Game/oldmod.txt").is_file());
        assert_eq!(old.files_in_game_folder(), 1);
    }
    let ac = turbine_engine::rules::AntiCheat {
        name: "Easy Anti-Cheat".into(),
        level: "warn".into(),
        mode: "me3".into(),
        me3: Some("eldenring".into()),
        starter: Some("Game/start_protected_game.exe".into()),
        game_exe: Some("Game/eldenring.exe".into()),
        ..Default::default()
    };
    let mut store = Store::open_at(&w.game, &store_root, Box::new(Me3Rules(Some("eldenring"), false))).unwrap();
    assert_eq!(anticheat::state(&w.game.install_dir, &ac, None, Some("100"), store.files_in_game_folder()), AcState::TurbineFilesLeft(1));
    // The next apply takes it out of the game folder and puts the mod in the profile instead.
    store.apply().unwrap();
    assert!(!w.game.install_dir.join("Game/oldmod.txt").exists());
    assert_eq!(files_under(&w.game.install_dir), before);
    assert_eq!(files_under(&anticheat::profile_mods(store.root())), vec!["oldmod.txt".to_string()]);
    let rec = anticheat::record_now(&w.game.install_dir, &ac, Some("100")).unwrap();
    assert_eq!(anticheat::state(&w.game.install_dir, &ac, Some(&rec), Some("100"), store.files_in_game_folder()), AcState::Original);
    let _ = fs::remove_dir_all(&w.root);
}

#[test]
fn elden_ring_rules_place_into_the_profile() {
    let w = world("realrules");
    write(&w.game.install_dir.join("Game/parts/am_f_0000.partsbnd.dcx"), "parts");
    let before = files_under(&w.game.install_dir);
    let rules = GameRules::for_game(&w.game);
    let mut store = Store::open_at(&w.game, &w.root.join("store"), Box::new(rules)).unwrap();
    let armor = w.root.join("dl/Golden Armor 1.0.zip");
    write_zip(&armor, &[("parts/am_f_9000.partsbnd.dcx", "armor")]);
    store.import_zip(&armor).unwrap();
    // Seamless Co-op's own zip layout: its folder and its launcher at the top.
    let coop = w.root.join("dl/Seamless Co-op 2.0.1.zip");
    write_zip(&coop, &[("SeamlessCoop/ersc.dll", "dll"), ("SeamlessCoop/ersc_settings.ini", "ini"), ("ersc_launcher.exe", "exe")]);
    store.import_zip(&coop).unwrap();
    store.apply().unwrap();
    assert_eq!(files_under(&w.game.install_dir), before, "the game folder must not change");
    let got = files_under(&anticheat::profile_mods(store.root()));
    assert!(got.contains(&"parts/am_f_9000.partsbnd.dcx".to_string()), "{got:?}");
    assert!(got.contains(&"SeamlessCoop/ersc.dll".to_string()), "{got:?}");
    let (profile, natives) = anticheat::write_profile(store.root(), "eldenring").unwrap();
    assert_eq!(natives, vec!["mods/SeamlessCoop/ersc.dll".to_string()]);
    // Seamless Co-op on: its own .co2 saves, no separate save of Turbine's.
    assert!(!fs::read_to_string(profile).unwrap().contains("savefile"));
    let _ = fs::remove_dir_all(&w.root);
}

#[test]
fn online_only_games_take_no_mods() {
    let w = world("refuse");
    let mut store = Store::open_at(&w.game, &w.root.join("store"), Box::new(Me3Rules(None, true))).unwrap();
    let zip = w.root.join("dl/Anything 1.0.zip");
    write_zip(&zip, &[("a.txt", "a")]);
    assert!(store.import_zip(&zip).unwrap_err().contains("won't add mods"));
    assert!(store.mods().is_empty());
    let _ = fs::remove_dir_all(&w.root);
}

#[test]
fn builtin_anticheat_games() {
    let g = |id: &str| {
        GameRules::for_game(&Game {
            store: Launcher::Steam,
            store_id: id.into(),
            name: String::new(),
            install_dir: PathBuf::from("/nowhere"),
            library: PathBuf::from("/nowhere"),
            build: None,
            runtime: Runtime::Windows,
            cover: None,
            header: None,
            prefix: None,
        })
    };
    for (id, me3) in [("1245620", "eldenring"), ("2622380", "nightreign"), ("1888160", "armoredcore6"), ("374320", "darksouls3"), ("814380", "sekiro")] {
        let r = g(id);
        let ac = r.anti_cheat.as_ref().unwrap_or_else(|| panic!("{id} has no antiCheat"));
        assert_eq!(anticheat::mode(ac), Mode::Me3, "{id}");
        assert_eq!(ac.me3_game(), Some(me3));
        assert_eq!(ModRules::me3_game(&r).as_deref(), Some(me3));
    }
    assert!(g("374320").anti_cheat.unwrap().offline_setting.unwrap().contains("Start offline"));
    let halo = g("976730").anti_cheat.unwrap();
    assert_eq!(anticheat::mode(&halo), Mode::Official);
    assert!(halo.official_choice.unwrap().contains("Anti-Cheat Disabled"));
    for id in ["381210", "1172470", "1085660"] {
        let r = g(id);
        assert_eq!(anticheat::mode(r.anti_cheat.as_ref().unwrap()), Mode::Refuse, "{id}");
        assert!(ModRules::refuses_mods(&r).is_some());
    }
    // Games without anti-cheat are unchanged.
    assert!(ModRules::me3_game(&g("489830")).is_none());
    assert!(ModRules::refuses_mods(&g("489830")).is_none());
}

#[test]
fn data_file_checks() {
    let me3 = r#"{"schema":1,"name":"X","ids":{"steam":["1"]},"antiCheat":{"name":"EAC","mode":"me3","me3":"eldenring"}}"#;
    assert!(parse_game("x.json", me3, true).is_ok());
    // A player's file can't make Turbine start a program on the host.
    assert!(parse_game("x.json", me3, false).unwrap_err().contains("only Turbine's own files"));
    let bad = r#"{"schema":1,"name":"X","ids":{"steam":["1"]},"antiCheat":{"name":"EAC","mode":"me3","me3":"tetris"}}"#;
    assert!(parse_game("x.json", bad, true).is_err());
    let off = r#"{"schema":1,"name":"X","ids":{"steam":["1"]},"antiCheat":{"name":"EAC","mode":"official"}}"#;
    assert!(parse_game("x.json", off, true).unwrap_err().contains("officialChoice"));
    let climb = r#"{"schema":1,"name":"X","ids":{"steam":["1"]},"antiCheat":{"name":"EAC","starter":"../x.exe"}}"#;
    assert!(parse_game("x.json", climb, true).is_err());
    let refuse = r#"{"schema":1,"name":"X","ids":{"steam":["1"]},"antiCheat":{"name":"BattlEye","level":"refuse"}}"#;
    assert_eq!(parse_game("x.json", refuse, false).unwrap().anti_cheat.unwrap().mode, "refuse");
    let half = r#"{"schema":1,"name":"X","ids":{"steam":["1"]},"antiCheat":{"name":"BattlEye","level":"warn","mode":"refuse"}}"#;
    assert!(parse_game("x.json", half, false).is_err());
    let unknown = r#"{"schema":1,"name":"X","ids":{"steam":["1"]},"antiCheat":{"name":"EAC","swap":true}}"#;
    assert!(parse_game("x.json", unknown, true).unwrap_err().contains("unknown key"));
    assert!(gamedata::builtin().problems.is_empty(), "{:?}", gamedata::builtin().problems);
}

// ---- 0.57 break test fixes

#[test]
fn halo_mods_in_its_folder_never_block_play() {
    let root = std::env::temp_dir().join(format!("turbine-ac-halo-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let library = root.join("library");
    let install = library.join("steamapps/common/Halo The Master Chief Collection");
    write(&install.join("mcclauncher.exe"), "launcher");
    write(&install.join("halo1/maps/a.map"), "m");
    let game = Game {
        store: Launcher::Steam,
        store_id: "976730".into(),
        name: "Halo: The Master Chief Collection".into(),
        install_dir: install,
        library,
        build: Some("1".into()),
        runtime: Runtime::Windows,
        cover: None,
        header: None,
        prefix: None,
    };
    let rules = GameRules::for_game(&game);
    let ac = rules.anti_cheat.clone().unwrap();
    let mut store = Store::open_at(&game, &root.join("store"), Box::new(rules)).unwrap();
    let zip = root.join("dl/Halo Mod 1.0.zip");
    write_zip(&zip, &[("halo1/maps/b.map", "x")]);
    store.import_zip(&zip).unwrap();
    store.apply().unwrap();
    // Its mods are in its folder by design (the player picks Steam's Play choice without anti-cheat): the folder
    // check that would stop Play is only for games me3 starts.
    assert!(store.files_in_game_folder() > 0);
    assert!(!anticheat::folder_checked(&ac));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn online_only_games_never_get_files_by_any_path() {
    let w = world("refusepaths");
    let before = files_under(&w.game.install_dir);
    let store_root = w.root.join("store");
    {
        // An earlier version placed two mods (one later removed).
        let mut old = Store::open_at(&w.game, &store_root, Box::new(Me3Rules(None, false))).unwrap();
        let zip = w.root.join("dl/Skin 1.0.zip");
        write_zip(&zip, &[("Game/skin.txt", "skin")]);
        old.import_zip(&zip).unwrap();
        let zip2 = w.root.join("dl/Other 1.0.zip");
        write_zip(&zip2, &[("Game/other.txt", "o")]);
        let m2 = old.import_zip(&zip2).unwrap();
        old.remove(&m2.id).unwrap();
        old.apply().unwrap();
        assert!(w.game.install_dir.join("Game/skin.txt").exists());
    }
    let mut store = Store::open_at(&w.game, &store_root, Box::new(Me3Rules(None, true))).unwrap();
    // The next start takes the earlier files out.
    assert!(store.refused_cleanup().unwrap() > 0);
    assert_eq!(files_under(&w.game.install_dir), before);
    assert_eq!(store.files_in_game_folder(), 0);
    // Apply, undo, an update and a saved setup place nothing.
    store.apply().unwrap();
    let removed: Vec<String> = store.removed().iter().map(|m| m.item.id.clone()).collect();
    assert!(store.undo_remove(&removed[0]).is_err());
    let upd = w.root.join("dl/Skin 1.1.zip");
    write_zip(&upd, &[("Game/skin2.txt", "skin2")]);
    let id = store.mods()[0].id.clone();
    assert!(store.replace_mod(&id, &upd, Default::default()).is_err());
    let _ = store.restore("");
    store.set_order(&[(id, true)]).unwrap();
    store.apply().unwrap();
    assert_eq!(files_under(&w.game.install_dir), before);
    assert_eq!(store.refused_cleanup().unwrap(), 0);
    let _ = fs::remove_dir_all(&w.root);
}

#[test]
fn a_first_deploy_cut_short_is_cleaned_for_a_me3_game() {
    let w = world("crash");
    let store_root = w.root.join("store");
    let before = files_under(&w.game.install_dir);
    {
        let mut old = Store::open_at(&w.game, &store_root, Box::new(Me3Rules(None, false))).unwrap();
        let zip = w.root.join("dl/Big 1.0.zip");
        write_zip(&zip, &[("Game/a.txt", "a"), ("Game/b.txt", "b"), ("Game/c.txt", "c")]);
        old.import_zip(&zip).unwrap();
        let _ = old.apply_stopping(Some(1));
    }
    let mut store = Store::open_at(&w.game, &store_root, Box::new(Me3Rules(Some("eldenring"), false))).unwrap();
    // The plan is still there: the folder isn't counted as original.
    assert!(store.files_in_game_folder() > 0);
    store.play_original().unwrap();
    assert_eq!(files_under(&w.game.install_dir), before);
    assert_eq!(store.files_in_game_folder(), 0);
    store.apply().unwrap();
    assert_eq!(files_under(&w.game.install_dir), before);
    let _ = fs::remove_dir_all(&w.root);
}

/// 0.58: an online-only game owned on Epic or GOG (through Heroic) is refused like its Steam copy: by its store id
/// where the data file has it, else by its name.
#[test]
fn heroic_copies_of_online_only_games_are_refused() {
    let g = |store: Launcher, id: &str, name: &str| Game {
        name: name.into(),
        store,
        store_id: id.into(),
        install_dir: PathBuf::from("/nowhere"),
        library: PathBuf::from("/"),
        build: None,
        runtime: Runtime::Windows,
        cover: None,
        header: None,
        prefix: None,
    };
    for (store, id, name) in [
        (Launcher::Epic, "Fortnite", "Fortnite"),
        (Launcher::Epic, "0a2d9f6403244d12969e11da6713137b", "Fall Guys"),
        (Launcher::Epic, "abc", "Dead by Daylight"),
        (Launcher::Gog, "1234", "DEAD BY DAYLIGHT"),
        (Launcher::Epic, "x", "Destiny 2"),
        (Launcher::Epic, "y", "Tom Clancy's Rainbow Six Siege X"),
    ] {
        let r = GameRules::for_game(&g(store, id, name));
        assert_eq!(r.anti_cheat.as_ref().map(anticheat::mode), Some(Mode::Refuse), "{name} on {store:?}");
    }
    // Other games stay as they were.
    let r = GameRules::for_game(&g(Launcher::Epic, "y", "A Short Hike"));
    assert!(r.anti_cheat.is_none());
    let r = GameRules::for_game(&g(Launcher::Epic, "z", "Destiny"));
    assert!(r.anti_cheat.is_none(), "only the whole name matches");
    let r = GameRules::for_game(&g(Launcher::Gog, "1", "Fallout 2"));
    assert!(r.anti_cheat.is_none());
}

/// 0.58: a me3 game's own files (regulation.bin) never make the "game already has this file" question: the mod goes
/// into Turbine's me3 profile, not the game folder.
#[test]
fn me3_games_ask_nothing_about_their_own_files() {
    let w = world("ownfiles");
    write(&w.game.install_dir.join("Game/regulation.bin"), "the game's own");
    let files = vec!["regulation.bin".to_string(), "Game/regulation.bin".to_string()];
    let (conflicts, _) = turbine_engine::store::conflicts_in(&w.game, &w.root.join("store"), &files, "", "").unwrap();
    assert!(conflicts.is_empty(), "{conflicts:?}");
    let (conflicts, _) = turbine_engine::store::conflicts_in(&w.game, &w.root.join("store"), &files, "", "Game").unwrap();
    assert!(conflicts.is_empty(), "{conflicts:?}");
    let _ = fs::remove_dir_all(&w.root);
}

/// 0.62: every me3 game's profile turns the game's code protection off (me3's top-level `disable_arxan`). Without
/// it Elden Ring 1.17.2 started with mods and closed by itself after 20 to 28 seconds on the owner's device. The
/// key has to sit above the first table, or TOML reads it as part of that table and me3 never sees it.
#[test]
fn every_me3_profile_turns_the_code_protection_off() {
    let top_level = |text: &str| {
        let head = text.split("\n[[").next().unwrap_or("");
        head.lines().any(|l| l.trim() == "disable_arxan = true")
    };
    for game in gamedata::ME3_GAMES {
        let d = std::env::temp_dir().join(format!("turbine-arxan-{}-{game}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        let mods = anticheat::profile_mods(&d);
        write(&mods.join("some.dll"), "x");
        let (file, _) = anticheat::write_profile(&d, game).unwrap();
        let t = fs::read_to_string(&file).unwrap();
        assert!(top_level(&t), "{game}: {t}");
        assert_eq!(t.matches("disable_arxan").count(), 1, "{game}: {t}");
        assert!(t.contains(&format!("game = \"{game}\"")), "{game}: {t}");
        let (file, _) = anticheat::write_profile_for(&d, game, anticheat::Coop::Solo).unwrap();
        assert!(top_level(&fs::read_to_string(&file).unwrap()), "{game} solo");
        let _ = fs::remove_dir_all(&d);
    }
    // The co-op profile (Elden Ring) carries it too.
    let d = std::env::temp_dir().join(format!("turbine-arxan-{}-coop", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    write(&anticheat::profile_mods(&d).join("SeamlessCoop/ersc.dll"), "x");
    write(&anticheat::profile_mods(&d).join("SeamlessCoop/ersc_settings.ini"), "[PASSWORD]\ncooppassword = dragon\n");
    let (file, _) = anticheat::write_profile_for(&d, "eldenring", anticheat::Coop::Seamless).unwrap();
    assert!(top_level(&fs::read_to_string(&file).unwrap()));
    let _ = fs::remove_dir_all(&d);
}

/// 0.62 (the owner's device): Play Seamless Co-op started with an empty session password and the mod stopped the
/// game. The password is read from the file the mod itself loads under me3 (next to its ersc.dll in Turbine's me3
/// profile), and no co-op profile or once file is written while it is empty.
#[test]
fn coop_needs_a_session_password_in_the_file_the_mod_loads() {
    use anticheat::{Coop, CoopPassword};
    // The key in its section, in any letter case, with spaces and quotes.
    assert_eq!(anticheat::coop_password_in("[PASSWORD]\r\ncooppassword = dragon\r\n").as_deref(), Some("dragon"));
    assert_eq!(anticheat::coop_password_in("\u{feff}[password]\n  CoopPassword=dragon  \n").as_deref(), Some("dragon"));
    assert_eq!(anticheat::coop_password_in("[ Password ]\ncooppassword\t=\t\"two words\"\n").as_deref(), Some("two words"));
    assert_eq!(anticheat::coop_password_in("[PASSWORD]\ncooppassword = 'x'\n").as_deref(), Some("x"));
    assert_eq!(anticheat::coop_password_in("[PASSWORD]\ncooppassword = \n").as_deref(), Some(""));
    assert_eq!(anticheat::coop_password_in("[PASSWORD]\ncooppassword =\"\"\n").as_deref(), Some(""));
    assert_eq!(anticheat::coop_password_in("[PASSWORD]\ncooppassword = \"  \"\n").as_deref(), Some(""));
    // A commented line is no password; the mod's own help text above the key doesn't count either.
    assert_eq!(anticheat::coop_password_in("[PASSWORD]\n; cooppassword = old\n# cooppassword = older\ncooppassword =\n").as_deref(), Some(""));
    // The [PASSWORD] section wins over the same key elsewhere; without the section the key still counts.
    assert_eq!(anticheat::coop_password_in("[GAMEPLAY]\ncooppassword = wrong\n[PASSWORD]\ncooppassword = right\n").as_deref(), Some("right"));
    assert_eq!(anticheat::coop_password_in("cooppassword = bare\n").as_deref(), Some("bare"));
    assert_eq!(anticheat::coop_password_in("[GAMEPLAY]\nallow_invaders = 1\n"), None);
    // A password with an = in it stays whole.
    assert_eq!(anticheat::coop_password_in("[PASSWORD]\ncooppassword = a=b\n").as_deref(), Some("a=b"));

    let d = std::env::temp_dir().join(format!("turbine-cooppw-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    let store = d.join("store");
    let data = d.join("data");
    let mods = anticheat::profile_mods(&store);
    // No Seamless Co-op in the profile.
    fs::create_dir_all(&mods).unwrap();
    assert_eq!(anticheat::coop_password(&store), CoopPassword::NoMod);
    // The mod without its settings file.
    write(&mods.join("SeamlessCoop/ersc.dll"), "x");
    assert!(matches!(anticheat::coop_password(&store), CoopPassword::NoFile(_)));
    assert!(anticheat::write_profile_for(&store, "eldenring", Coop::Seamless).is_err());
    // An edited copy somewhere else in the store doesn't count: only the file next to the mod does.
    write(&store.join("settings/seamless/ersc_settings.ini"), "[PASSWORD]\ncooppassword = elsewhere\n");
    assert!(matches!(anticheat::coop_password(&store), CoopPassword::NoFile(_)));
    // The file with an empty password (any letter case in its name): refused everywhere, with the plain words.
    let ini = mods.join("SeamlessCoop/ERSC_Settings.ini");
    write(&ini, "[GAMEPLAY]\r\nallow_invaders = 1\r\n\r\n[PASSWORD]\r\n; Session password\r\ncooppassword = \r\n");
    assert_eq!(anticheat::coop_password(&store), CoopPassword::Empty(ini.clone()));
    assert_eq!(anticheat::coop_password(&store).word(), "empty");
    let e = anticheat::write_profile_for(&store, "eldenring", Coop::Seamless).unwrap_err();
    assert_eq!(e.to_string(), anticheat::COOP_NEEDS_PASSWORD);
    assert!(!anticheat::profile_file_for(&store, Coop::Seamless).exists(), "no co-op profile is written");
    // The once file (what the helper and the Steam tool read for Play Seamless Co-op): not written, and one left
    // from before goes.
    let args = vec!["/x/me3.exe".to_string(), "launch".to_string()];
    anticheat::arm_once(&data, "1245620", &args, "old").unwrap();
    let e = anticheat::arm_coop_once(&data, &store, "1245620", &args).unwrap_err();
    assert_eq!(e, anticheat::COOP_NEEDS_PASSWORD);
    assert!(!anticheat::once_file(&data, "1245620").exists());
    // Solo Play is not held up by it.
    assert!(anticheat::write_profile_for(&store, "eldenring", Coop::Solo).is_ok());
    // With a password: the profile and the once file are written.
    write(&ini, "[PASSWORD]\r\ncooppassword = dragon\r\n");
    assert_eq!(anticheat::coop_password(&store), CoopPassword::Set);
    assert!(anticheat::coop_password(&store).ready().is_ok());
    let (profile, natives) = anticheat::write_profile_for(&store, "eldenring", Coop::Seamless).unwrap();
    assert!(profile.exists() && natives.len() == 1);
    anticheat::arm_coop_once(&data, &store, "1245620", &args).unwrap();
    assert!(anticheat::once_file(&data, "1245620").exists());
    assert!(anticheat::arm_coop_once(&data, &store, "12a", &args).is_err());
    let _ = fs::remove_dir_all(&d);
}
