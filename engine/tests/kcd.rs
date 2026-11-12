//! Kingdom Come: Deliverance I and II: their ids, where mods go, mod_order.txt from Turbine's order, and the checks
//! before Play.

mod support;

use std::fs;
use support::{game, write_zip, TempDir};
use turbine_engine::games::Store as Launcher;
use turbine_engine::kcd::{self, BACKUP_NAME, MADE_MARK, ORDER_FILE, SEEN_NAME};
use turbine_engine::nexus;
use turbine_engine::rules::{self, mod_checks_with, Family, GameRules, ModFiles};
use turbine_engine::store::{Import, SourceInfo, Store};
use turbine_engine::{Game, Runtime};

fn kcd_game(id: &str, dir: &std::path::Path) -> Game {
    let mut g = game(Launcher::Steam, id, "Kingdom Come", dir, None);
    g.runtime = Runtime::Windows;
    g
}

fn kcd(id: &str) -> GameRules {
    GameRules::for_game(&kcd_game(id, std::path::Path::new("/games/KingdomComeDeliverance")))
}

fn list(files: &[&str]) -> Vec<String> {
    files.iter().map(|s| s.to_string()).collect()
}

fn dests(id: &str, files: &[&str], name: &str) -> Vec<String> {
    let mut d: Vec<String> = kcd(id).map_archive(&list(files), name).unwrap().into_iter().map(|(_, d)| d).collect();
    d.sort();
    d
}

#[test]
fn ids_domains_and_rules() {
    for (store, id, family, name, domain) in [
        (Launcher::Steam, "379430", Family::Kcd, "Kingdom Come: Deliverance", "kingdomcomedeliverance"),
        (Launcher::Gog, "1719198803", Family::Kcd, "Kingdom Come: Deliverance", "kingdomcomedeliverance"),
        (Launcher::Steam, "1771300", Family::Kcd2, "Kingdom Come: Deliverance II", "kingdomcomedeliverance2"),
        (Launcher::Gog, "1248083010", Family::Kcd2, "Kingdom Come: Deliverance II", "kingdomcomedeliverance2"),
    ] {
        let d = rules::find_def(store, id).unwrap();
        assert_eq!((d.name, d.family), (name, family), "{id}");
        assert_eq!(nexus::domain_for_store_id(id), Some(domain));
    }
    // The packages (Royal Edition, the second game's package and Expansion Pass) aren't installed games.
    for id in ["1182943037", "1076704248", "1141344806"] {
        assert!(rules::find_def(Launcher::Gog, id).is_none());
    }
    let r = kcd("379430");
    assert!(r.ordered && !r.plugins && r.loader.is_none());
    assert_eq!(r.mod_root, "Mods");
    assert!(!r.needs_loader(&list(&["Mods/A/Data/a.pak"])));
}

#[test]
fn mod_layouts() {
    let want = vec!["Mods/BetterLockpicking/Data/lock.pak", "Mods/BetterLockpicking/mod.manifest"];
    // The mod folder itself, inside Mods, or inside a wrapper folder.
    assert_eq!(dests("379430", &["BetterLockpicking/mod.manifest", "BetterLockpicking/Data/lock.pak"], "x"), want);
    assert_eq!(dests("379430", &["Mods/BetterLockpicking/mod.manifest", "Mods/BetterLockpicking/Data/lock.pak"], "x"), want);
    assert_eq!(dests("379430", &["Better Lockpicking-12-1-0/BetterLockpicking/mod.manifest", "Better Lockpicking-12-1-0/BetterLockpicking/Data/lock.pak", "Better Lockpicking-12-1-0/readme.txt"], "x"), want);
    // Its contents: the mod's name makes the folder (letters and digits only).
    assert_eq!(dests("1771300", &["mod.manifest", "Data/lock.pak", "readme.txt"], "Better Lockpicking"), want);
    assert_eq!(dests("1771300", &["lock.pak"], "Better Lockpicking"), vec!["Mods/BetterLockpicking/Data/lock.pak"]);
    // Text paks, mod.cfg, and two mod folders in one archive.
    assert_eq!(
        dests("1771300", &["A/mod.manifest", "A/mod.cfg", "A/Localization/english_xml.pak", "B/Data/b.pak", "Screens/1.jpg"], "x"),
        vec!["Mods/A/Localization/english_xml.pak", "Mods/A/mod.cfg", "Mods/A/mod.manifest", "Mods/B/Data/b.pak"]
    );
    // A user.cfg of game settings goes in the game's main folder, with a note about -devmode.
    assert_eq!(dests("1771300", &["user.cfg"], "Tweaks"), vec!["user.cfg"]);
    assert!(kcd("1771300").install_notes(&list(&["user.cfg"]))[0].contains("-devmode +exec user.cfg"));
    assert!(kcd("379430").install_notes(&list(&["user.cfg"])).is_empty());
    assert!(kcd("379430").map_archive(&list(&["readme.txt", "pics/a.png"]), "x").is_err());
    assert_eq!(kcd::folder_name("Better Lockpicking 1.2 (fixed)"), "BetterLockpicking12fixed");
}

#[test]
fn mod_order_text() {
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<String>>();
    // No file yet: the player's own folders first (alphabetically, as the game loaded them), then Turbine's order.
    assert_eq!(kcd::render_mod_order_adding(None, &s(&["zeta", "Alpha", "B"]), &s(&["B"]), &s(&["B", "C"]), &[]), "Alpha\nzeta\nB\n");
    // A file: its other lines stay as they were, Turbine's come after; turned-off ones leave.
    assert_eq!(kcd::render_mod_order_adding(Some("C\r\nOwn\r\nB\r\n"), &[], &s(&["B"]), &s(&["B", "C"]), &[]), "Own\r\nB\r\n");
    assert_eq!(kcd::render_mod_order_adding(Some("Own\n"), &[], &[], &s(&["B"]), &[]), "Own\n");
    assert_eq!(kcd::render_mod_order_adding(None, &s(&["B"]), &[], &s(&["B"]), &[]), "");
    assert_eq!(rules::mod_folders(&s(&["mods/A/Data/a.pak", "Mods/A/mod.manifest", "Mods/B/Data/b.pak", "user.cfg"])), s(&["A", "B"]));
}

#[test]
fn mod_order_file_backup_and_cleanup() {
    let t = TempDir::new("kcd-order");
    let game_dir = t.path("KingdomComeDeliverance");
    let keep = t.path("turbine");
    fs::create_dir_all(game_dir.join("mods/Own")).unwrap();
    fs::create_dir_all(game_dir.join("mods/A")).unwrap();
    let a = vec!["Mods/A/Data/a.pak".to_string()];
    // No file: Turbine makes one (the player's folder kept loading) and remembers it made it.
    assert!(kcd::sync_mod_order(&game_dir, &keep, &[(&a, true)]).unwrap());
    assert_eq!(fs::read_to_string(game_dir.join("mods").join(ORDER_FILE)).unwrap(), "Own\nA\n");
    assert!(keep.join(MADE_MARK).exists() && !keep.join(BACKUP_NAME).exists());
    assert!(!kcd::sync_mod_order(&game_dir, &keep, &[(&a, true)]).unwrap(), "nothing changed");
    // All of Turbine's off: still lists the player's own.
    assert!(kcd::sync_mod_order(&game_dir, &keep, &[(&a, false)]).unwrap());
    assert_eq!(fs::read_to_string(game_dir.join("mods").join(ORDER_FILE)).unwrap(), "Own\n");

    // The player's own file: copied once into Turbine's folder before the first change.
    let t2 = TempDir::new("kcd-order2");
    let g2 = t2.path("KCD2");
    let keep2 = t2.path("turbine");
    t2.write("KCD2/Mods/mod_order.txt", b"Own\n");
    assert!(kcd::sync_mod_order(&g2, &keep2, &[(&a, true)]).unwrap());
    assert_eq!(fs::read_to_string(g2.join("Mods/mod_order.txt")).unwrap(), "Own\nA\n");
    assert_eq!(fs::read_to_string(keep2.join(BACKUP_NAME)).unwrap(), "Own\n");
    // A file Turbine made with nothing left to list goes again.
    let t3 = TempDir::new("kcd-order3");
    let g3 = t3.path("KCD");
    fs::create_dir_all(g3.join("Mods/A")).unwrap();
    let keep3 = t3.path("turbine");
    assert!(kcd::sync_mod_order(&g3, &keep3, &[(&a, true)]).unwrap());
    fs::remove_dir_all(g3.join("Mods/A")).unwrap();
    assert!(kcd::sync_mod_order(&g3, &keep3, &[(&a, false)]).unwrap());
    assert!(!g3.join("Mods/mod_order.txt").exists() && !keep3.join(MADE_MARK).exists());
    // No Mods folder at all: nothing to do.
    assert!(!kcd::sync_mod_order(&t3.path("none"), &keep3, &[(&a, false)]).unwrap());
}

#[test]
fn folders_added_by_hand_later_are_listed() {
    let a = vec!["Mods/A/Data/a.pak".to_string()];
    // Turbine made the file: a folder the player drops in later is listed too (it would not load otherwise).
    let t = TempDir::new("kcd-later");
    let g = t.path("KCD2");
    let keep = t.path("turbine");
    fs::create_dir_all(g.join("Mods/Own")).unwrap();
    assert!(kcd::sync_mod_order(&g, &keep, &[(&a, true)]).unwrap());
    assert_eq!(fs::read_to_string(g.join("Mods/mod_order.txt")).unwrap(), "Own\nA\n");
    fs::create_dir_all(g.join("Mods/Zebra")).unwrap();
    fs::create_dir_all(g.join("Mods/Bridge")).unwrap();
    assert!(kcd::sync_mod_order(&g, &keep, &[(&a, true)]).unwrap());
    assert_eq!(fs::read_to_string(g.join("Mods/mod_order.txt")).unwrap(), "Own\nBridge\nZebra\nA\n");
    assert!(keep.join(SEEN_NAME).exists());

    // The player's own file: a folder they left out stays out; one added after Turbine's last write goes in.
    let t2 = TempDir::new("kcd-later2");
    let g2 = t2.path("KCD");
    let keep2 = t2.path("turbine");
    fs::create_dir_all(g2.join("Mods/Left")).unwrap();
    t2.write("KCD/Mods/Own/mod.manifest", b"<kcd_mod/>");
    t2.write("KCD/Mods/mod_order.txt", b"Own\n");
    assert!(kcd::sync_mod_order(&g2, &keep2, &[(&a, true)]).unwrap());
    assert_eq!(fs::read_to_string(g2.join("Mods/mod_order.txt")).unwrap(), "Own\nA\n", "Left stays out");
    fs::create_dir_all(g2.join("Mods/Newer")).unwrap();
    assert!(kcd::sync_mod_order(&g2, &keep2, &[(&a, true)]).unwrap());
    assert_eq!(fs::read_to_string(g2.join("Mods/mod_order.txt")).unwrap(), "Own\nNewer\nA\n");
    assert!(!kcd::sync_mod_order(&g2, &keep2, &[(&a, true)]).unwrap(), "nothing new");
}

#[test]
fn the_players_mod_order_can_be_put_back() {
    let a = vec!["Mods/A/Data/a.pak".to_string()];
    let t = TempDir::new("kcd-restore");
    let g = t.path("KCD");
    let keep = t.path("turbine");
    assert!(!kcd::can_restore(&keep));
    t.write("KCD/Mods/mod_order.txt", b"Own\r\nSecond\r\n");
    assert!(kcd::sync_mod_order(&g, &keep, &[(&a, true)]).unwrap());
    assert!(kcd::can_restore(&keep));
    // Broken by hand afterwards; put back as it was before Turbine, byte for byte, and the copy stays.
    fs::write(g.join("Mods/mod_order.txt"), b"garbage\n").unwrap();
    assert!(kcd::restore_mod_order(&g, &keep).unwrap());
    assert_eq!(fs::read(g.join("Mods/mod_order.txt")).unwrap(), b"Own\r\nSecond\r\n");
    assert!(keep.join(BACKUP_NAME).exists() && kcd::can_restore(&keep));
    // The next write adds Turbine's line again and keeps the copy as it was.
    assert!(kcd::sync_mod_order(&g, &keep, &[(&a, true)]).unwrap());
    assert_eq!(fs::read_to_string(g.join("Mods/mod_order.txt")).unwrap(), "Own\r\nSecond\r\nA\r\n");
    assert_eq!(fs::read(keep.join(BACKUP_NAME)).unwrap(), b"Own\r\nSecond\r\n");

    // A file Turbine made: putting it back means no file, and the mark goes.
    let t2 = TempDir::new("kcd-restore2");
    let g2 = t2.path("KCD2");
    let keep2 = t2.path("turbine");
    fs::create_dir_all(g2.join("Mods/A")).unwrap();
    assert!(kcd::sync_mod_order(&g2, &keep2, &[(&a, true)]).unwrap());
    assert!(kcd::restore_mod_order(&g2, &keep2).unwrap());
    assert!(!g2.join("Mods/mod_order.txt").exists() && !keep2.join(MADE_MARK).exists() && !kcd::can_restore(&keep2));
    assert!(!kcd::restore_mod_order(&g2, &keep2).unwrap(), "nothing left to put back");
}

#[test]
fn row_badges_say_what_the_check_says() {
    let good = list(&["Mods/HerbPicking/mod.manifest", "Mods/HerbPicking/Data/herbs.pak"]);
    let bare = list(&["Mods/FastTravel/Data/travel.pak"]);
    assert_eq!(kcd::mod_badge("Herb Picking II 1.5", &good, true, false), None);
    assert_eq!(kcd::mod_badge("Fast Travel KCD1 2.0", &good, true, false).as_deref(), Some("Made for the first Kingdom Come: Deliverance"));
    assert_eq!(kcd::mod_badge("Herb Picking", &good, true, true).as_deref(), Some("Made for the first Kingdom Come: Deliverance"));
    assert_eq!(kcd::mod_badge("Better Saving KCD2", &good, false, false).as_deref(), Some("Made for Kingdom Come: Deliverance II"));
    assert_eq!(kcd::mod_badge("Fast Travel", &bare, true, false).as_deref(), Some("May not load"));
    assert_eq!(kcd::mod_badge("Fast Travel", &bare, false, false), None, "the first game loads paks without a manifest");
    assert_eq!(kcd::mod_badge("Readme", &list(&["Mods/Readme/readme.txt"]), false, false).as_deref(), Some("May not load"));
}

fn m(name: &str, files: &[&str]) -> ModFiles {
    ModFiles { id: name.into(), name: name.into(), enabled: true, files: list(files) }
}

#[test]
fn checks_before_play() {
    let one = kcd("379430");
    let two = kcd("1771300");
    let good = m("Good", &["Mods/Good/mod.manifest", "Mods/Good/Data/g.pak"]);
    let paks_only = m("Paks", &["Mods/Paks/Data/p.pak"]);
    let empty = m("Empty", &["Mods/Empty/readme.md"]);
    assert!(mod_checks_with(&one, &[good.clone(), paks_only.clone()], &[]).is_empty());
    let c = mod_checks_with(&one, &[empty], &[]);
    assert_eq!((c.len(), c[0].level.as_str(), c[0].title.as_str()), (1, "warn", "Empty may not load"));
    // The second game needs the manifest.
    let c = mod_checks_with(&two, &[paks_only], &[]);
    assert_eq!(c[0].title, "Paks has no mod.manifest");
    // A mod named for the other game.
    let c = mod_checks_with(&one, &[m("Fast Travel KCD2", &["Mods/FastTravel/mod.manifest"])], &[]);
    assert_eq!(c[0].title, "Fast Travel KCD2 is made for Kingdom Come: Deliverance II");
    assert!(kcd::name_says_other("KCD1_Remaster", true) && !kcd::name_says_other("KCD2 Tweaks", true));
    let d = kcd::devmode_check(true);
    assert!(d.detail.ends_with("\n-devmode") && d.detail.contains("Launch Options"));
    assert!(kcd::devmode_check(false).detail.contains("Heroic"));
}

#[test]
fn installing_through_the_store() {
    let t = TempDir::new("kcd-store");
    let dir = t.path("lib/KingdomComeDeliverance2");
    fs::create_dir_all(dir.join("mods")).unwrap();
    let g = kcd_game("1771300", &dir);
    let mut s = Store::open(&g, &t.path("data")).unwrap();
    let src = || SourceInfo { kind: "zip".into(), ..SourceInfo::default() };
    let a = t.path("dl/Lockpicking.zip");
    write_zip(&a, &[("Lockpicking/mod.manifest", b"<kcd_mod/>"), ("Lockpicking/Data/lock.pak", b"pak")]);
    let Import::Added(_) = s.import_archive(&a, src()).unwrap() else { panic!() };
    let b = t.path("dl/Stash.zip");
    write_zip(&b, &[("mod.manifest", b"<kcd_mod/>"), ("Data/stash.pak", b"pak")]);
    let Import::Added(stash) = s.import_archive(&b, src()).unwrap() else { panic!() };
    s.apply().unwrap();
    // The game's own lower-case mods folder is used, not a second one.
    assert_eq!(fs::read(dir.join("mods/Lockpicking/Data/lock.pak")).unwrap(), b"pak");
    assert!(dir.join("mods/Stash/mod.manifest").exists());
    assert!(!dir.join("Mods").exists());
    let files = s.mod_files();
    let pairs: Vec<(&[String], bool)> = files.iter().map(|m| (m.files.as_slice(), m.enabled)).collect();
    assert!(kcd::sync_mod_order(&dir, s.root(), &pairs).unwrap());
    assert_eq!(fs::read_to_string(dir.join("mods/mod_order.txt")).unwrap(), "Lockpicking\nStash\n");
    s.move_to(&stash.item.id, 0).unwrap();
    let files = s.mod_files();
    let pairs: Vec<(&[String], bool)> = files.iter().map(|m| (m.files.as_slice(), m.enabled)).collect();
    assert!(kcd::sync_mod_order(&dir, s.root(), &pairs).unwrap());
    assert_eq!(fs::read_to_string(dir.join("mods/mod_order.txt")).unwrap(), "Stash\nLockpicking\n");
}
