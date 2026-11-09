//! 0.59: Steam library art. Grid files are named by the shortcut's appid; the player's own art is never replaced;
//! Remove takes out only what Turbine wrote.

use std::fs;
use std::path::PathBuf;
use turbine_engine::steam_art::{self, ArtOutcome};
use turbine_engine::{shortcuts, vdf};

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("turbine-steamart-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn grid_names_from_the_appid() {
    assert_eq!(steam_art::grid_name(3_871_208_806, "capsule").unwrap(), "3871208806p.png");
    assert_eq!(steam_art::grid_name(3_871_208_806, "wide").unwrap(), "3871208806.png");
    assert_eq!(steam_art::grid_name(3_871_208_806, "hero").unwrap(), "3871208806_hero.png");
    assert_eq!(steam_art::grid_name(3_871_208_806, "logo").unwrap(), "3871208806_logo.png");
    assert_eq!(steam_art::grid_name(3_871_208_806, "icon").unwrap(), "3871208806_icon.png");
    assert_eq!(steam_art::grid_name(1, "banner"), None);
    assert_eq!(steam_art::TURBINE_PARTS, ["capsule", "wide", "hero", "logo"]);
}

fn src(d: &std::path::Path, tag: &str) -> PathBuf {
    let s = d.join(format!("src-{tag}"));
    fs::create_dir_all(&s).unwrap();
    for p in ["capsule", "wide", "hero", "logo"] {
        fs::write(s.join(format!("{p}.png")), format!("{p} {tag}")).unwrap();
    }
    s
}

#[test]
fn never_over_the_players_art_and_remove_only_turbines() {
    let d = tmp("own");
    let grid = d.join("grid");
    let record = d.join("data/steam-art.json");
    fs::create_dir_all(&grid).unwrap();
    // The player already picked a hero picture.
    fs::write(grid.join("77_hero.png"), "mine").unwrap();
    fs::write(grid.join("77_icon.png"), "mine too").unwrap();
    fs::write(grid.join("78p.png"), "another game's").unwrap();
    let v1 = src(&d, "v1");
    let got = steam_art::add_art(&v1, &grid, 77, steam_art::TURBINE_PARTS, &record).unwrap();
    let outcome = |n: &str| got.iter().find(|(p, _)| p.ends_with(n)).map(|(_, o)| o.clone()).unwrap();
    assert_eq!(outcome("77p.png"), ArtOutcome::Written);
    assert_eq!(outcome("77.png"), ArtOutcome::Written);
    assert_eq!(outcome("77_logo.png"), ArtOutcome::Written);
    assert_eq!(outcome("77_hero.png"), ArtOutcome::PlayersOwn);
    assert_eq!(fs::read_to_string(grid.join("77_hero.png")).unwrap(), "mine");
    // Again: nothing to write.
    let got = steam_art::add_art(&v1, &grid, 77, steam_art::TURBINE_PARTS, &record).unwrap();
    assert!(got.iter().all(|(p, o)| *o == ArtOutcome::Unchanged || p.ends_with("77_hero.png")), "{got:?}");
    // After an update Turbine's own files are renewed; the player replaced the wide one meanwhile: kept.
    fs::write(grid.join("77.png"), "player's wide").unwrap();
    let v2 = src(&d, "v2");
    let got = steam_art::add_art(&v2, &grid, 77, steam_art::TURBINE_PARTS, &record).unwrap();
    assert_eq!(got.iter().find(|(p, _)| p.ends_with("77p.png")).unwrap().1, ArtOutcome::Written);
    assert_eq!(fs::read_to_string(grid.join("77p.png")).unwrap(), "capsule v2");
    assert_eq!(got.iter().find(|(p, _)| p.ends_with("77.png")).unwrap().1, ArtOutcome::PlayersOwn);
    assert_eq!(fs::read_to_string(grid.join("77.png")).unwrap(), "player's wide");
    // Remove: only Turbine's own files.
    let removed = steam_art::remove_art(&grid, 77, &record).unwrap();
    let names: Vec<String> = removed.iter().map(|p| p.file_name().unwrap().to_string_lossy().to_string()).collect();
    assert_eq!(names, ["77p.png", "77_logo.png"]);
    for (n, want) in [("77_hero.png", "mine"), ("77_icon.png", "mine too"), ("77.png", "player's wide"), ("78p.png", "another game's")] {
        assert_eq!(fs::read_to_string(grid.join(n)).unwrap(), want, "{n}");
    }
    assert!(steam_art::remove_art(&grid, 77, &record).unwrap().is_empty());
    // A missing source piece is skipped.
    let empty = d.join("none");
    let got = steam_art::add_art(&empty, &grid, 5, steam_art::TURBINE_PARTS, &record).unwrap();
    assert!(got.iter().all(|(_, o)| *o == ArtOutcome::NoSource));
    let _ = fs::remove_dir_all(&d);
}

/// Turbine's own shortcut is found in shortcuts.vdf (read only) by its Flatpak run line.
#[test]
fn finds_turbines_own_shortcut() {
    let d = tmp("find");
    let file = d.join(".local/share/Steam/userdata/42/config/shortcuts.vdf");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    let mut doc = vdf::BinDoc { items: vec![vdf::BinItem::obj("shortcuts", vec![])], end: Some(8), tail: vec![] };
    let own = vdf::BinItem::obj(
        "0",
        vec![
            vdf::BinItem::int("appid", 0x9000_0042),
            vdf::BinItem::text("AppName", "Turbine Mod Manager"),
            vdf::BinItem::text("Exe", "\"/usr/bin/flatpak\""),
            vdf::BinItem::text("LaunchOptions", "run --branch=stable --arch=x86_64 --command=turbine-mod-manager io.github.ravenor907.TurbineModManager"),
        ],
    );
    if let vdf::BinValue::Obj(list, _) = &mut doc.items[0].value {
        list.push(own);
    }
    let bytes = vdf::write_binary_doc(&doc);
    fs::write(&file, &bytes).unwrap();
    // The co-op tile next to it doesn't count as Turbine itself.
    let tile = shortcuts::coop_tile("ELDEN RING", "steam:1245620", "io.github.ravenor907.TurbineModManager");
    shortcuts::add_shortcut(&file, &tile, &shortcuts::coop_marker("steam:1245620"), false).unwrap();
    let (f, appid) = shortcuts::find_turbine_file(&d).unwrap();
    assert_eq!(f, file);
    assert_eq!(appid, 0x9000_0042);
    assert_eq!(shortcuts::find_turbine(&d).unwrap().0, 0x9000_0042);
    assert_eq!(steam_art::grid_dir(&f), file.with_file_name("grid"));
    let _ = fs::remove_dir_all(&d);
}
