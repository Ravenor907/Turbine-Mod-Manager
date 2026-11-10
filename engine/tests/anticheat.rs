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

fn nexus(version: &str) -> turbine_engine::net::SourceInfo {
    turbine_engine::net::SourceInfo { kind: "nexus".into(), id: "510".into(), version: Some(version.into()), title: Some("Seamless Co-op (Elden Ring)".into()), ..Default::default() }
}

fn own_download(version: &str) -> turbine_engine::net::SourceInfo {
    turbine_engine::net::SourceInfo { kind: "loader".into(), id: "seamless".into(), version: Some(version.into()), title: Some("Seamless Co-op".into()), ..Default::default() }
}

fn added(r: Result<turbine_engine::store::Import, String>) -> turbine_engine::store::Mod {
    match r.unwrap() {
        turbine_engine::store::Import::Added(done) => done.item,
        _ => panic!("the mod wasn't simply added"),
    }
}

const INI_201: &str = "[GAMEPLAY]\r\nallow_invaders = 1\r\n\r\n[PASSWORD]\r\n; Session password\r\ncooppassword = \r\n";
const INI_202: &str = "[GAMEPLAY]\r\nallow_invaders = 1\r\nnew_in_202 = 5\r\n\r\n[PASSWORD]\r\n; Session password\r\ncooppassword = \r\n";

/// 0.62 (the owner's device): the list showed Seamless Co-op 2.0.2 from Nexus, but the game loaded 2.0.1 and the mod
/// stopped ("out of date"). Turbine only knew its own pinned download as Seamless Co-op, so that older copy was
/// added as a second mod and its files won. Now there is one Seamless Co-op per game, known by its files: a new
/// download takes the place of the old one and keeps the session password.
#[test]
fn a_newer_seamless_coop_replaces_the_older_copy_and_keeps_the_password() {
    let w = world("seamless-update");
    let mut store = Store::open_at(&w.game, &w.root.join("store"), Box::new(GameRules::for_game(&w.game))).unwrap();
    // Turbine's own older download first, with a password set through Edit settings.
    let old = w.root.join("dl/Seamless Co-op 2.0.1.zip");
    write_zip(&old, &[("SeamlessCoop/ersc.dll", "dll 2.0.1"), ("SeamlessCoop/ersc_settings.ini", INI_201), ("ersc_launcher.exe", "exe")]);
    let first = added(store.import_archive(&old, own_download("2.0.1")));
    let staged = turbine_engine::store::staged_dir(store.root(), &first.id);
    let rel = turbine_engine::settings::config_files(&staged, 8, 40).into_iter().find(|r| r.to_ascii_lowercase().ends_with("ersc_settings.ini")).unwrap();
    let with_pw = anticheat::coop_password_put(INI_201, "dragon").unwrap();
    assert_eq!(with_pw, INI_201.replace("cooppassword = \r\n", "cooppassword = dragon\r\n"));
    turbine_engine::settings::save_edit(store.root(), &first.id, &rel, &staged.join(&rel), &with_pw).unwrap();
    store.apply().unwrap();
    let mods = anticheat::profile_mods(store.root());
    assert_eq!(fs::read_to_string(mods.join("SeamlessCoop/ersc.dll")).unwrap(), "dll 2.0.1");
    assert_eq!(anticheat::coop_password(store.root()), anticheat::CoopPassword::Set);

    // The player gets 2.0.2 from Nexus: it replaces the copy in the list (no second mod), under the same entry.
    let new = w.root.join("dl/Seamless Co-op v2.0.2 510 2.0.2 2026-10-27T14-30Z gn8dtBhTy.zip");
    write_zip(&new, &[("SeamlessCoop/ersc.dll", "dll 2.0.2"), ("SeamlessCoop/ersc_settings.ini", INI_202), ("ersc_launcher.exe", "exe")]);
    let second = added(store.import_archive(&new, nexus("2.0.2")));
    assert_eq!(second.id, first.id, "the same entry in the list");
    assert_eq!(second.version.as_deref(), Some("2.0.2"));
    let all = store.seamless_mods();
    assert_eq!(all.len(), 1, "{all:?}");
    assert_eq!(all[0].source_kind, "nexus");
    assert_eq!(store.mods().iter().filter(|m| m.name.to_lowercase().contains("seamless")).count(), 1);
    // The older file is still in the game's mod folder until the apply: the check before a co-op Play says so.
    let before = store.coop_check();
    assert!(!before.profile_current);
    assert!(before.blocks().unwrap().contains("isn't the one in your list (Seamless Co-op (Elden Ring) 2.0.2)"), "{:?}", before.blocks());
    store.apply().unwrap();
    // The game's mod folder now holds 2.0.2, and the password came along with the new file's new setting.
    assert_eq!(fs::read_to_string(mods.join("SeamlessCoop/ersc.dll")).unwrap(), "dll 2.0.2");
    let ini = fs::read_to_string(mods.join("SeamlessCoop/ersc_settings.ini")).unwrap();
    assert!(ini.contains("cooppassword = dragon"), "{ini}");
    assert!(ini.contains("new_in_202 = 5"), "{ini}");
    let check = store.coop_check();
    assert!(check.profile_current && check.blocks().is_none() && check.warning().is_none(), "{check:?}");
    assert_eq!(check.list_version.as_deref(), Some("2.0.2"));
    let j = turbine_engine::json::parse(&check.to_json()).unwrap();
    assert_eq!((j.str_at("password"), j.str_at("listVersion")), (Some("set"), Some("2.0.2")));
    // The list marks it as the Seamless Co-op the game loads, whatever its source.
    let state = turbine_engine::json::parse(&store.to_json()).unwrap();
    let marked: Vec<&turbine_engine::json::Json> = state.get("mods").unwrap().as_array().iter().filter(|m| m.get("seamless").and_then(|s| s.as_bool()) == Some(true)).collect();
    assert_eq!(marked.len(), 1);
    assert_eq!(marked[0].str_at("sourceKind"), Some("nexus"));
    // Steam's menu (which reads the store without opening it) sees a Nexus copy as Seamless Co-op too.
    let peek = turbine_engine::quick::peek(store.root());
    assert!(peek.seamless && peek.seamless_on);
    let _ = fs::remove_dir_all(&w.root);
}

/// The state the owner's device is in now: two copies in the list, the older one later in the order. Only the
/// newer one's files go into the game's mod folder, the check names both, and tidying keeps one.
#[test]
fn with_two_copies_in_the_list_the_newer_one_is_loaded() {
    let w = world("seamless-two");
    let mut store = Store::open_at(&w.game, &w.root.join("store"), Box::new(Me3Rules(Some("eldenring"), false))).unwrap();
    // An entry that becomes the Nexus 2.0.2 (replaced in place below), then Turbine's own 2.0.1 after it.
    let other = w.root.join("dl/Placeholder 1.0.zip");
    write_zip(&other, &[("Game/other.bin", "x")]);
    let a = store.import_zip(&other).unwrap();
    let old = w.root.join("dl/Seamless Co-op 2.0.1.zip");
    write_zip(&old, &[("SeamlessCoop/ersc.dll", "dll 2.0.1"), ("SeamlessCoop/ersc_settings.ini", &INI_201.replace("cooppassword = ", "cooppassword = dragon")), ("ersc_launcher.exe", "exe")]);
    let b = added(store.import_archive(&old, own_download("2.0.1")));
    let new = w.root.join("dl/Seamless Co-op 2.0.2.zip");
    write_zip(&new, &[("SeamlessCoop/ersc.dll", "dll 2.0.2"), ("SeamlessCoop/ersc_settings.ini", INI_202), ("ersc_launcher.exe", "exe")]);
    added(store.replace_mod(&a.id, &new, nexus("2.0.2")));
    assert_eq!(store.seamless_mods().len(), 2);
    assert_ne!(a.id, b.id);
    // The older copy is later in the list (it would win by order alone): the newer version is the one loaded.
    assert_eq!(store.seamless().unwrap().id, a.id);
    store.apply().unwrap();
    let mods = anticheat::profile_mods(store.root());
    assert_eq!(fs::read_to_string(mods.join("SeamlessCoop/ersc.dll")).unwrap(), "dll 2.0.2");
    // Its own settings file is the one in the folder: the password typed into the older copy is not there, and
    // the check says exactly that instead of letting the mod stop the game.
    let check = store.coop_check();
    assert_eq!(check.password.word(), "empty");
    assert_eq!(check.blocks().as_deref(), Some(anticheat::COOP_NEEDS_PASSWORD));
    assert_eq!(check.others, vec!["Seamless Co-op 2.0.1".to_string()]);
    // Tidying keeps the newer copy, moves the older one to Removed mods and carries its password over.
    let gone = store.seamless_tidy().unwrap();
    assert_eq!(gone, vec!["Seamless Co-op 2.0.1".to_string()]);
    assert_eq!(store.seamless_mods().len(), 1);
    assert!(store.seamless_tidy().unwrap().is_empty(), "nothing more to tidy");
    store.apply().unwrap();
    assert_eq!(fs::read_to_string(mods.join("SeamlessCoop/ersc.dll")).unwrap(), "dll 2.0.2");
    let check = store.coop_check();
    assert_eq!(check.password, anticheat::CoopPassword::Set);
    assert!(check.blocks().is_none() && check.others.is_empty(), "{check:?}");
    // With the newer copy turned off, the older one (still on) would be the one loaded: here it is gone, so none.
    store.set_enabled(&a.id, false).unwrap();
    store.apply().unwrap();
    assert!(!mods.join("SeamlessCoop/ersc.dll").exists());
    let _ = fs::remove_dir_all(&w.root);
}

#[test]
fn the_dll_says_its_own_version() {
    // A version block: signature, structure version 1.0, file version 2.0.1.0.
    let mut bytes = vec![0u8; 64];
    bytes.extend_from_slice(&[0xBD, 0x04, 0xEF, 0xFE]);
    bytes.extend_from_slice(&0x0001_0000u32.to_le_bytes());
    bytes.extend_from_slice(&(2u32 << 16).to_le_bytes());
    bytes.extend_from_slice(&(1u32 << 16).to_le_bytes());
    bytes.extend_from_slice(&[0u8; 32]);
    assert_eq!(anticheat::dll_version_in(&bytes).as_deref(), Some("2.0.1"));
    assert_eq!(anticheat::dll_version_in(b"no version block here"), None);
    assert_eq!(anticheat::dll_version_in(&[0xBD, 0x04, 0xEF, 0xFE, 1, 2]), None, "cut short");
    assert!(anticheat::is_seamless_file("SeamlessCoop/ersc.dll") && anticheat::is_seamless_file("Game\\SeamlessCoop\\ERSC.DLL") && !anticheat::is_seamless_file("SeamlessCoop/ersc_settings.ini"));

    // The list says 2.0.2 and the file says 2.0.1: said plainly before a co-op Play.
    let w = world("seamless-version");
    let mut store = Store::open_at(&w.game, &w.root.join("store"), Box::new(GameRules::for_game(&w.game))).unwrap();
    let zip = w.root.join("dl/Seamless Co-op 2.0.2.zip");
    write_zip(&zip, &[("SeamlessCoop/ersc.dll", "placeholder"), ("SeamlessCoop/ersc_settings.ini", &INI_202.replace("cooppassword = ", "cooppassword = dragon")), ("ersc_launcher.exe", "exe")]);
    added(store.import_archive(&zip, nexus("2.0.2")));
    store.apply().unwrap();
    // The file the game loads carries the older number (written here straight into the game's mod folder; it is
    // the same file as the mod's own when Turbine linked it).
    fs::write(anticheat::coop_dll(store.root()).unwrap(), &bytes).unwrap();
    let check = store.coop_check();
    assert_eq!(check.file_version.as_deref(), Some("2.0.1"));
    assert_eq!(check.version_differs(), Some(("2.0.2".to_string(), "2.0.1".to_string())));
    let words = check.warning().unwrap();
    assert!(words.contains("Your list says Seamless Co-op 2.0.2, but its file says it is 2.0.1"), "{words}");
    if check.profile_current {
        assert!(check.blocks().is_none(), "a warning, not a stop: the file's own number may lag behind");
    }
    let _ = fs::remove_dir_all(&w.root);
}

#[test]
fn seamless_coop_is_the_newest_release_never_a_fixed_one() {
    use turbine_engine::loaders;
    let answer = r#"{"tag_name":"v2.0.2","name":"Seamless Co-op v2.0.2","assets":[
        {"name":"notes.txt","browser_download_url":"https://github.com/yuiamoroll/EldenRingSeamlessCoopRelease/releases/download/v2.0.2/notes.txt","size":10},
        {"name":"Seamless.Co-op.v2.0.2.zip","browser_download_url":"https://github.com/yuiamoroll/EldenRingSeamlessCoopRelease/releases/download/v2.0.2/Seamless.Co-op.v2.0.2.zip","size":8100000,"digest":"sha256:848AE27E1C77217590DAC401C012A17D9B4F05B9B9A24AA33AF54D8D7B8BA2A1"}]}"#;
    let r = loaders::parse_seamless_release(answer).unwrap();
    assert_eq!(r.version, "2.0.2");
    assert_eq!(r.url, "https://github.com/yuiamoroll/EldenRingSeamlessCoopRelease/releases/download/v2.0.2/Seamless.Co-op.v2.0.2.zip");
    assert_eq!(r.size, 8_100_000);
    assert_eq!(r.sha256.as_deref(), Some("848ae27e1c77217590dac401c012a17d9b4f05b9b9a24aa33af54d8d7b8ba2a1"));
    assert!(turbine_engine::net::allowed_url(loaders::SEAMLESS_LATEST) && turbine_engine::net::allowed_url(&r.url));
    // No digest listed: still the size; the SHA-256 of what was downloaded is recorded by the download itself.
    let no_digest = answer.replace(",\"digest\":\"sha256:848AE27E1C77217590DAC401C012A17D9B4F05B9B9A24AA33AF54D8D7B8BA2A1\"", "");
    assert_eq!(loaders::parse_seamless_release(&no_digest).unwrap().sha256, None);
    // A file from anywhere but GitHub is refused; so is a release with no zip or an unreadable version.
    assert!(loaders::parse_seamless_release(&answer.replace("https://github.com/yuiamoroll/EldenRingSeamlessCoopRelease/releases/download/v2.0.2/Seamless", "https://example.com/Seamless")).is_err());
    assert!(loaders::parse_seamless_release(r#"{"tag_name":"v2.0.3","assets":[]}"#).is_err());
    assert!(loaders::parse_seamless_release(r#"{"tag_name":"latest build!","assets":[]}"#).is_err());
    assert!(loaders::parse_seamless_release("{}").is_err());
}

fn touch_later(p: &Path, secs: u64) {
    let f = fs::File::options().write(true).open(p).unwrap();
    f.set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(secs)).unwrap();
}

/// 0.62: a settings file changed where it is deployed (by the game, the mod, or by hand) used to be kept over
/// Turbine's newer copy, so a password or any other setting changed in Turbine afterwards never reached the game.
/// Now the in-place changes are merged into Turbine's copy: what the player changed in Turbine since the last apply
/// wins for those settings, everything else changed in place is kept and shows in Edit settings, and nothing is
/// lost without a trace.
#[test]
fn settings_changed_in_place_are_merged_and_turbines_edits_reach_the_game() {
    use turbine_engine::settings;
    let w = world("in-place");
    let mut store = Store::open_at(&w.game, &w.root.join("store"), Box::new(GameRules::for_game(&w.game))).unwrap();
    let zip = w.root.join("dl/Seamless Co-op 2.0.2.zip");
    write_zip(&zip, &[("SeamlessCoop/ersc.dll", "dll"), ("SeamlessCoop/ersc_settings.ini", INI_202), ("ersc_launcher.exe", "exe")]);
    let m = added(store.import_archive(&zip, nexus("2.0.2")));
    store.apply().unwrap();
    let deployed = anticheat::profile_mods(store.root()).join("SeamlessCoop/ersc_settings.ini");
    let staged = turbine_engine::store::staged_dir(store.root(), &m.id);
    let rel = settings::config_files(&staged, 8, 40).into_iter().find(|r| r.to_ascii_lowercase().ends_with("ersc_settings.ini")).unwrap();
    let turbines = |store: &Store| fs::read_to_string(settings::effective(store.root(), &m.id, &rel, &staged.join(&rel))).unwrap();

    // 1. Changed in place only (never edited in Turbine): kept in the game, and Turbine's copy now shows it too.
    fs::write(&deployed, INI_202.replace("allow_invaders = 1", "allow_invaders = 0")).unwrap();
    touch_later(&deployed, 10);
    store.apply().unwrap();
    assert!(fs::read_to_string(&deployed).unwrap().contains("allow_invaders = 0"));
    assert!(turbines(&store).contains("allow_invaders = 0"), "Edit settings shows what the game has");
    assert!(settings::in_place_path(store.root(), &m.id, &rel).is_file(), "the in-place file is kept beside Turbine's copy");

    // 2. The owner's case: the file changes in place again, then the password is set in Turbine. Both reach the game.
    let now = fs::read_to_string(&deployed).unwrap();
    fs::write(&deployed, now.replace("new_in_202 = 5", "new_in_202 = 9")).unwrap();
    touch_later(&deployed, 20);
    let with_pw = anticheat::coop_password_put(&turbines(&store), "dragon").unwrap();
    settings::save_edit(store.root(), &m.id, &rel, &staged.join(&rel), &with_pw).unwrap();
    store.apply().unwrap();
    let got = fs::read_to_string(&deployed).unwrap();
    assert!(got.contains("cooppassword = dragon"), "the password set in Turbine is in the game: {got}");
    assert!(got.contains("new_in_202 = 9") && got.contains("allow_invaders = 0"), "the in-place changes stay: {got}");
    assert_eq!(turbines(&store), got, "Turbine's copy and the game's file are the same again");
    assert_eq!(anticheat::coop_password(store.root()), anticheat::CoopPassword::Set);

    // 3. A second change in Turbine (the part that never reached the game before): it does now.
    fs::write(&deployed, got.replace("new_in_202 = 9", "new_in_202 = 7")).unwrap();
    touch_later(&deployed, 30);
    settings::save_edit(store.root(), &m.id, &rel, &staged.join(&rel), &anticheat::coop_password_put(&turbines(&store), "wyvern").unwrap()).unwrap();
    store.apply().unwrap();
    let got = fs::read_to_string(&deployed).unwrap();
    assert!(got.contains("cooppassword = wyvern") && got.contains("new_in_202 = 7"), "{got}");

    // 4. Changed in place and in Turbine since the last apply: Turbine's value is the truth; the in-place file is
    //    kept beside Turbine's copy, so the other value isn't lost without a trace.
    fs::write(&deployed, got.replace("cooppassword = wyvern", "cooppassword = other")).unwrap();
    touch_later(&deployed, 40);
    settings::save_edit(store.root(), &m.id, &rel, &staged.join(&rel), &anticheat::coop_password_put(&turbines(&store), "drake").unwrap()).unwrap();
    store.apply().unwrap();
    let got = fs::read_to_string(&deployed).unwrap();
    assert!(got.contains("cooppassword = drake"), "{got}");
    assert!(fs::read_to_string(settings::in_place_path(store.root(), &m.id, &rel)).unwrap().contains("cooppassword = other"));
    // Changed in place only, a setting once set in Turbine: taken over, and Edit settings shows it (never a
    // silent difference between Turbine's copy and the game's file).
    fs::write(&deployed, got.replace("cooppassword = drake", "cooppassword = typed in the file")).unwrap();
    touch_later(&deployed, 50);
    store.apply().unwrap();
    let got = fs::read_to_string(&deployed).unwrap();
    assert!(got.contains("cooppassword = typed in the file"), "{got}");
    assert_eq!(turbines(&store), got);

    // 5. Nothing changed anywhere: an apply changes nothing.
    let r = store.apply().unwrap();
    assert_eq!((r.added, r.removed), (0, 0), "{r:?}");
    assert_eq!(fs::read_to_string(&deployed).unwrap(), got);
    let _ = fs::remove_dir_all(&w.root);
}

/// 0.62 break test M1: a setting changed in the game's folder, then the FIRST Edit settings save in Turbine. The
/// deploy record still names the mod's own file while the file to deploy is Turbine's new copy; the merge was
/// skipped and the in-place change was dropped without a trace.
#[test]
fn the_first_edit_in_turbine_keeps_changes_made_in_the_games_folder() {
    use turbine_engine::settings;
    let w = world("first-edit");
    let mut store = Store::open_at(&w.game, &w.root.join("store"), Box::new(GameRules::for_game(&w.game))).unwrap();
    let zip = w.root.join("dl/Seamless Co-op 2.0.2.zip");
    write_zip(&zip, &[("SeamlessCoop/ersc.dll", "dll"), ("SeamlessCoop/ersc_settings.ini", INI_202), ("ersc_launcher.exe", "exe")]);
    let m = added(store.import_archive(&zip, nexus("2.0.2")));
    store.apply().unwrap();
    let deployed = anticheat::profile_mods(store.root()).join("SeamlessCoop/ersc_settings.ini");
    let staged = turbine_engine::store::staged_dir(store.root(), &m.id);
    let rel = settings::config_files(&staged, 8, 40).into_iter().find(|r| r.to_ascii_lowercase().ends_with("ersc_settings.ini")).unwrap();
    // The game (or the player, by hand) changes a setting in place.
    fs::write(&deployed, INI_202.replace("allow_invaders = 1", "allow_invaders = 0")).unwrap();
    touch_later(&deployed, 10);
    // The first Edit settings save, made from the mod's defaults as the page showed them before 0.62 (no apply
    // in between): only the password is set.
    settings::save_edit(store.root(), &m.id, &rel, &staged.join(&rel), &anticheat::coop_password_put(INI_202, "dragon").unwrap()).unwrap();
    store.apply().unwrap();
    let got = fs::read_to_string(&deployed).unwrap();
    assert!(got.contains("cooppassword = dragon"), "{got}");
    assert!(got.contains("allow_invaders = 0"), "the in-place change is kept: {got}");
    let mine = fs::read_to_string(settings::effective(store.root(), &m.id, &rel, &staged.join(&rel))).unwrap();
    assert_eq!(mine, got, "and Edit settings shows it");
    assert!(fs::read_to_string(settings::in_place_path(store.root(), &m.id, &rel)).unwrap().contains("allow_invaders = 0"));
    // Reset to default afterwards gives the mod's own file, not a stale copy of the in-place one.
    settings::reset(store.root(), &m.id, &rel).unwrap();
    store.apply().unwrap();
    assert_eq!(fs::read_to_string(&deployed).unwrap(), INI_202);

    // Edit settings itself merges before it shows the file (no store open): the page shows what the game has.
    fs::write(&deployed, INI_202.replace("new_in_202 = 5", "new_in_202 = 8")).unwrap();
    touch_later(&deployed, 20);
    assert!(turbine_engine::store::sync_in_place(&w.game, store.root(), &m.id, &rel));
    let shown = fs::read_to_string(settings::effective(store.root(), &m.id, &rel, &staged.join(&rel))).unwrap();
    assert!(shown.contains("new_in_202 = 8"), "{shown}");
    assert!(!turbine_engine::store::sync_in_place(&w.game, store.root(), "../x", &rel));
    store.apply().unwrap();
    assert!(fs::read_to_string(&deployed).unwrap().contains("new_in_202 = 8"));
    let _ = fs::remove_dir_all(&w.root);
}

/// 0.62 break test M4: a settings file emptied or cut short in the game's folder (a crash while the game wrote it,
/// a full disk) never empties Turbine's copy: Turbine's values stay and go back in, and the report says so.
#[test]
fn a_damaged_settings_file_never_empties_turbines_copy() {
    use turbine_engine::settings::{self, Format};
    // The unit case from the break test.
    let placed = "[A]\nz = 0\npw = a\n";
    assert_eq!(settings::merge_in_place(Format::Ini, "", placed, placed), placed);
    assert_eq!(settings::merge_in_place(Format::Ini, "\n\n", placed, placed), placed);
    assert_eq!(settings::merge_in_place(Format::Ini, "[A]\n", "[A]\nz = 0\npw = a\nq = 1\n", "[A]\nz = 0\npw = a\nq = 1\n"), "[A]\nz = 0\npw = a\nq = 1\n", "cut short: most settings gone");
    assert!(settings::in_place_is_damaged(Format::Ini, "", placed));
    assert!(!settings::in_place_is_damaged(Format::Ini, "[A]\nz = 1\npw = a\n", placed), "a changed value is not damage");
    assert!(!settings::in_place_is_damaged(Format::Ini, "", ""), "an empty file that was placed empty");
    // One setting removed on purpose is still a change, taken over.
    assert_eq!(settings::merge_in_place(Format::Ini, "[A]\nz = 0\n", "[A]\nz = 0\npw = \n", "[A]\nz = 0\npw = \n"), "[A]\nz = 0\n");

    let w = world("damaged");
    let mut store = Store::open_at(&w.game, &w.root.join("store"), Box::new(GameRules::for_game(&w.game))).unwrap();
    let zip = w.root.join("dl/Seamless Co-op 2.0.2.zip");
    write_zip(&zip, &[("SeamlessCoop/ersc.dll", "dll"), ("SeamlessCoop/ersc_settings.ini", INI_202), ("ersc_launcher.exe", "exe")]);
    let m = added(store.import_archive(&zip, nexus("2.0.2")));
    let staged = turbine_engine::store::staged_dir(store.root(), &m.id);
    let rel = settings::config_files(&staged, 8, 40).into_iter().find(|r| r.to_ascii_lowercase().ends_with("ersc_settings.ini")).unwrap();
    let with_pw = anticheat::coop_password_put(INI_202, "dragon").unwrap();
    settings::save_edit(store.root(), &m.id, &rel, &staged.join(&rel), &with_pw).unwrap();
    store.apply().unwrap();
    let deployed = anticheat::profile_mods(store.root()).join("SeamlessCoop/ersc_settings.ini");
    assert_eq!(fs::read_to_string(&deployed).unwrap(), with_pw);
    // The file is emptied where it is deployed.
    fs::write(&deployed, "").unwrap();
    touch_later(&deployed, 10);
    let report = store.apply().unwrap();
    assert_eq!(fs::read_to_string(&deployed).unwrap(), with_pw, "Turbine's copy is back in the game");
    assert_eq!(fs::read_to_string(settings::effective(store.root(), &m.id, &rel, &staged.join(&rel))).unwrap(), with_pw, "and was never emptied");
    assert_eq!(report.settings_notes.len(), 1, "{:?}", report.settings_notes);
    assert!(report.settings_notes[0].contains("Turbine kept its own copy"), "{:?}", report.settings_notes);
    assert_eq!(anticheat::coop_password(store.root()), anticheat::CoopPassword::Set);
    assert_eq!(fs::read_to_string(settings::in_place_path(store.root(), &m.id, &rel)).unwrap(), "", "the damaged file is kept beside it");
    let _ = fs::remove_dir_all(&w.root);
}
