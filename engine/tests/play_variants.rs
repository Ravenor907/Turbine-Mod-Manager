//! Play without the script extender (one run without the loader and the mods that need it), and a setup that
//! closed right after starting no longer counting as one that worked.

mod support;

use std::fs;
use support::{game, write_zip, TempDir};
use turbine_engine::games::Store as Launcher;
use turbine_engine::rules::GameRules;
use turbine_engine::store::{Import, SourceInfo, Store};

#[test]
fn without_the_loader_for_one_run() {
    let t = TempDir::new("play-variants");
    let dir = t.path("lib/Skyrim Special Edition");
    fs::create_dir_all(dir.join("Data")).unwrap();
    fs::write(dir.join("SkyrimSELauncher.exe"), b"launcher").unwrap();
    let g = game(Launcher::Steam, "489830", "Skyrim Special Edition", &dir, None);
    let rules = GameRules::for_game(&g);
    let mut s = Store::open(&g, &t.path("data")).unwrap();
    let src = || SourceInfo { kind: "zip".into(), ..SourceInfo::default() };
    let skse = t.path("dl/skse64_2_02_06.zip");
    write_zip(&skse, &[("skse64_2_02_06/skse64_loader.exe", b"loader"), ("skse64_2_02_06/skse64_1_6_1170.dll", b"dll")]);
    let plugin = t.path("dl/Glow.zip");
    write_zip(&plugin, &[("Data/SKSE/Plugins/Glow.dll", b"p")]);
    let plain = t.path("dl/Lanterns.zip");
    write_zip(&plain, &[("textures/lantern.dds", b"t")]);
    let Import::Added(a) = s.import_archive(&skse, src()).unwrap() else { panic!() };
    let Import::Added(b) = s.import_archive(&plugin, src()).unwrap() else { panic!() };
    let Import::Added(c) = s.import_archive(&plain, src()).unwrap() else { panic!() };
    s.apply().unwrap();
    assert_eq!(fs::read(dir.join("SkyrimSELauncher.exe")).unwrap(), b"loader", "Steam's Play starts the loader");
    assert_eq!(s.loader_mods(&rules), vec![a.item.id.clone()]);
    assert!(b.item.needs_loader && !c.item.needs_loader);

    let skip: Vec<String> = s.loader_mods(&rules).into_iter().chain(s.mods().iter().filter(|m| m.needs_loader).map(|m| m.id.clone())).collect();
    let setups = s.setups().len();
    s.apply_except(&skip).unwrap();
    assert_eq!(fs::read(dir.join("SkyrimSELauncher.exe")).unwrap(), b"launcher", "the game's own launcher is back");
    assert!(!dir.join("Data/SKSE/Plugins/Glow.dll").exists());
    assert!(dir.join("Data/textures/lantern.dds").exists());
    assert!(s.mods().iter().all(|m| m.enabled), "nothing about the mods changed");
    assert_eq!(s.setups().len(), setups, "no setup saved for the one run");
    // The next apply puts everything back.
    s.apply().unwrap();
    assert_eq!(fs::read(dir.join("SkyrimSELauncher.exe")).unwrap(), b"loader");
    assert!(dir.join("Data/SKSE/Plugins/Glow.dll").exists());
}

#[test]
fn a_setup_that_closed_early_is_not_the_one_that_worked() {
    let t = TempDir::new("closed-early");
    let dir = t.path("lib/Game");
    // A Basic game: its own text folder makes the placement sure (mirrors the game's top).
    fs::create_dir_all(dir.join("text")).unwrap();
    let g = game(Launcher::Steam, "99999", "Some Game", &dir, None);
    let mut s = Store::open(&g, &t.path("data")).unwrap();
    let src = || SourceInfo { kind: "zip".into(), ..SourceInfo::default() };
    let a = t.path("dl/A.zip");
    write_zip(&a, &[("text/a.txt", b"a")]);
    let Import::Added(ma) = s.import_archive(&a, src()).unwrap() else { panic!() };
    s.set_clock(1_000);
    s.apply().unwrap();
    s.mark_played().unwrap();
    let worked = s.restore_target().unwrap().id.clone();
    assert_eq!(s.setup_on(&worked), vec![ma.item.id.clone()]);
    let b = t.path("dl/B.zip");
    write_zip(&b, &[("text/b.txt", b"b")]);
    s.set_clock(2_000);
    let Import::Added(mb) = s.import_archive(&b, src()).unwrap() else { panic!() };
    s.apply().unwrap();
    s.mark_played().unwrap();
    assert_ne!(s.restore_target().unwrap().id, worked);
    // It closed after a few seconds: the setup with B no longer counts.
    assert!(s.mark_not_working().unwrap());
    assert_eq!(s.restore_target().unwrap().id, worked);
    let added: Vec<String> = s.mods().iter().filter(|m| m.enabled && !s.setup_on(&worked).contains(&m.id)).map(|m| m.id.clone()).collect();
    assert_eq!(added, vec![mb.item.id.clone()]);
    assert!(!s.mark_not_working().unwrap(), "nothing more to unmark");
}
