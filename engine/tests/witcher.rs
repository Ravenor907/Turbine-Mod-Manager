//! The Witcher 3 (next-gen): where mods go, mods.settings from Turbine's order, script conflicts and the notes about
//! settings files Turbine never edits.

mod support;

use std::fs;
use support::{game, write_zip, TempDir};
use turbine_engine::games::Store as Launcher;
use turbine_engine::rules::{self, mod_checks_with, Family, GameRules, ModFiles};
use turbine_engine::store::{Import, SourceInfo, Store};
use turbine_engine::witcher::{self, BACKUP_NAME, MODS_SETTINGS};
use turbine_engine::{Game, Runtime};

fn w3_game(dir: &std::path::Path, prefix: Option<std::path::PathBuf>) -> Game {
    let mut g = game(Launcher::Steam, "292030", "The Witcher 3: Wild Hunt", dir, prefix);
    g.runtime = Runtime::Windows;
    g
}

fn w3() -> GameRules {
    GameRules::for_game(&w3_game(std::path::Path::new("/games/The Witcher 3"), None))
}

fn list(files: &[&str]) -> Vec<String> {
    files.iter().map(|s| s.to_string()).collect()
}

fn dests(files: &[&str], name: &str) -> Vec<String> {
    let mut d: Vec<String> = w3().map_archive(&list(files), name).unwrap().into_iter().map(|(_, d)| d).collect();
    d.sort();
    d
}

#[test]
fn ids_and_rules() {
    for (store, id) in [(Launcher::Steam, "292030"), (Launcher::Gog, "1207664643"), (Launcher::Gog, "1495134320")] {
        let d = rules::find_def(store, id).unwrap();
        assert_eq!((d.name, d.family), ("The Witcher 3", Family::Witcher3), "{id}");
    }
    // The Complete Edition package isn't something that gets installed.
    assert!(rules::find_def(Launcher::Gog, "1207664663").is_none());
    let r = w3();
    assert_eq!(r.short_name.as_deref(), Some("The Witcher 3"));
    assert!(r.ordered && r.loader.is_none());
    assert!(!r.needs_loader(&list(&["mods/modA/content/scripts/game/a.ws"])));
}

#[test]
fn mod_layouts() {
    let want = vec!["mods/modFastTravel/content/scripts/game/player/r4Player.ws"];
    assert_eq!(dests(&["modFastTravel/content/scripts/game/player/r4Player.ws"], "x"), want);
    assert_eq!(dests(&["Mods/modFastTravel/content/scripts/game/player/r4Player.ws"], "x"), want);
    assert_eq!(dests(&["Fast Travel 1.2/mods/modFastTravel/content/scripts/game/player/r4Player.ws", "Fast Travel 1.2/readme.txt"], "x"), want);
    assert_eq!(dests(&["The Witcher 3/Mods/modFastTravel/content/scripts/game/player/r4Player.ws"], "x"), want);
    // A mod folder whose name doesn't start with "mod" would be ignored by the game.
    assert_eq!(dests(&["Better Icons/content/blob0.bundle"], "x"), vec!["mods/modBetterIcons/content/blob0.bundle"]);
    assert_eq!(dests(&["mods/BetterIcons/content/blob0.bundle"], "x"), vec!["mods/modBetterIcons/content/blob0.bundle"]);
    // Only content: the mod's name makes the folder.
    assert_eq!(dests(&["content/blob0.bundle", "content/metadata.store"], "Buff Icons"), vec![
        "mods/modBuffIcons/content/blob0.bundle",
        "mods/modBuffIcons/content/metadata.store"
    ]);
    // DLC-style mods and the game's own content0 folder.
    assert_eq!(dests(&["dlcHDRP/content/blob0.bundle"], "x"), vec!["dlc/dlcHDRP/content/blob0.bundle"]);
    assert_eq!(dests(&["DLC/dlcHDRP/content/blob0.bundle"], "x"), vec!["DLC/dlcHDRP/content/blob0.bundle"]);
    assert_eq!(dests(&["content/content0/scripts/game/a.ws"], "x"), vec!["content/content0/scripts/game/a.ws"]);
    // Menu files, in their folder or on their own; settings lines and readmes stay out.
    assert_eq!(
        dests(
            &[
                "bin/config/r4game/user_config_matrix/pc/modFastTravel.xml",
                "mods/modFastTravel/content/scripts/a.ws",
                "input.settings.txt",
                "Readme.txt",
            ],
            "x"
        ),
        vec!["bin/config/r4game/user_config_matrix/pc/modFastTravel.xml", "mods/modFastTravel/content/scripts/a.ws"]
    );
    assert_eq!(dests(&["modMenu.xml", "modMenu/content/a.ws"], "x"), vec![
        "bin/config/r4game/user_config_matrix/pc/modMenu.xml",
        "mods/modMenu/content/a.ws"
    ]);
    assert!(w3().map_archive(&list(&["readme.txt"]), "x").is_err());
}

#[test]
fn notes_for_settings_files_turbine_never_edits() {
    let r = w3();
    let notes = r.install_notes(&list(&[
        "mods/modX/content/a.ws",
        "Add to input.settings.txt",
        "dx12user.settings",
        "bin/config/r4game/user_config_matrix/pc/modX.xml",
    ]));
    assert_eq!(notes.len(), 3, "{notes:?}");
    assert!(notes[0].contains("input.settings") && notes[0].contains("never changes"));
    assert!(notes[1].contains("dx12user.settings"));
    assert!(notes[2].contains("dx12filelist.txt") && notes[2].contains("modX.xml;"));
    assert!(r.install_notes(&list(&["mods/modX/content/a.ws"])).is_empty());
}

#[test]
fn mods_settings_text() {
    // Turbine's order: later wins, so the last one gets the lowest number (the highest priority).
    let order = list(&["modA", "modB", "modC"]);
    let t = witcher::render_mods_settings("", &order, &order);
    assert_eq!(t, "[modA]\r\nEnabled=1\r\nPriority=3\r\n\r\n[modB]\r\nEnabled=1\r\nPriority=2\r\n\r\n[modC]\r\nEnabled=1\r\nPriority=1\r\n");
    // Sections Turbine didn't make stay as they were; turned-off Turbine mods drop out; old Turbine sections are
    // replaced.
    let existing = "[modHandMade]\nEnabled=0\nPriority=7\n\n[modB]\nEnabled=1\nPriority=50\n";
    let t = witcher::render_mods_settings(existing, &list(&["modB", "modA"]), &list(&["modA", "modB", "modOff"]));
    assert_eq!(t, "[modHandMade]\nEnabled=0\nPriority=7\n\n[modB]\nEnabled=1\nPriority=2\n\n[modA]\nEnabled=1\nPriority=1\n");
    // Names match ignoring case; a folder listed twice counts where it last appears.
    let t = witcher::render_mods_settings("[MODA]\nEnabled=1\nPriority=9\n", &list(&["modA", "modB", "moda"]), &[]);
    assert_eq!(t, "[modB]\nEnabled=1\nPriority=2\n\n[moda]\nEnabled=1\nPriority=1\n");
    assert_eq!(witcher::mod_folders(&list(&["mods/modA/content/x", "mods/modA/content/y", "mods/modB/content/z", "dlc/d/content/q", "mods/top.txt"])), list(&["modA", "modB"]));
}

#[test]
fn mods_settings_file_backup_once_and_atomic() {
    let t = TempDir::new("w3-settings");
    let docs = t.path("Documents/The Witcher 3");
    assert_eq!(witcher::write_mods_settings(&docs, &list(&["modA"]), &[]).unwrap_err(), witcher::NOT_STARTED);
    fs::create_dir_all(&docs).unwrap();
    // No file yet and nothing on: nothing written.
    assert!(!witcher::write_mods_settings(&docs, &[], &[]).unwrap());
    assert!(!docs.join(MODS_SETTINGS).exists());
    // The player's own file: kept as a backup before the first change, never overwritten after.
    fs::write(docs.join(MODS_SETTINGS), "[modMine]\r\nEnabled=1\r\nPriority=1\r\n").unwrap();
    assert!(witcher::write_mods_settings(&docs, &list(&["modA"]), &list(&["modA"])).unwrap());
    assert_eq!(fs::read_to_string(docs.join(BACKUP_NAME)).unwrap(), "[modMine]\r\nEnabled=1\r\nPriority=1\r\n");
    assert_eq!(fs::read_to_string(docs.join(MODS_SETTINGS)).unwrap(), "[modMine]\r\nEnabled=1\r\nPriority=1\r\n\r\n[modA]\r\nEnabled=1\r\nPriority=1\r\n");
    assert!(!witcher::write_mods_settings(&docs, &list(&["modA"]), &list(&["modA"])).unwrap(), "unchanged");
    assert!(witcher::write_mods_settings(&docs, &[], &list(&["modA"])).unwrap());
    assert_eq!(fs::read_to_string(docs.join(MODS_SETTINGS)).unwrap(), "[modMine]\r\nEnabled=1\r\nPriority=1\r\n");
    assert_eq!(fs::read_to_string(docs.join(BACKUP_NAME)).unwrap(), "[modMine]\r\nEnabled=1\r\nPriority=1\r\n");
    // No temporary files left behind.
    let names: Vec<String> = fs::read_dir(&docs).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(names.len(), 2, "{names:?}");
}

#[test]
fn settings_folder_in_the_prefix() {
    let t = TempDir::new("w3-prefix");
    let pfx = t.path("pfx");
    assert_eq!(witcher::documents_dir(&pfx), pfx.join(witcher::DOCS_IN_PREFIX));
    fs::create_dir_all(pfx.join("drive_c/users/deck/Documents/The Witcher 3")).unwrap();
    assert_eq!(witcher::documents_dir(&pfx), pfx.join("drive_c/users/deck/Documents/The Witcher 3"));
    fs::create_dir_all(pfx.join(witcher::DOCS_IN_PREFIX)).unwrap();
    assert_eq!(witcher::documents_dir(&pfx), pfx.join(witcher::DOCS_IN_PREFIX));
    assert_eq!(witcher::sync_mods_settings(None, &[]).unwrap_err(), witcher::NOT_STARTED);
}

fn m(name: &str, files: &[&str]) -> ModFiles {
    ModFiles { id: name.to_lowercase(), name: name.into(), enabled: true, files: list(files) }
}

#[test]
fn script_conflicts_warn_before_play() {
    let r = w3();
    let a = m("Auto Oils", &["mods/modAutoOils/content/scripts/game/player/r4Player.ws", "mods/modAutoOils/content/scripts/game/gui/x.ws"]);
    let b = m("Fast Travel", &["mods/modFastTravel/content/scripts/game/player/r4Player.ws", "mods/modFastTravel/content/scripts/game/gui/x.ws"]);
    let c = m("Icons", &["mods/modIcons/content/blob0.bundle"]);
    let checks = mod_checks_with(&r, &[a.clone(), b.clone(), c.clone()], &[]);
    assert_eq!(checks.len(), 1);
    assert_eq!((checks[0].level.as_str(), checks[0].title.as_str()), ("warn", "Scripts need merging"));
    assert!(checks[0].detail.starts_with("Auto Oils and Fast Travel change the same game script (game/player/r4Player.ws and 1 more)"), "{}", checks[0].detail);
    assert!(checks[0].detail.contains("Script Merger"));
    // Turned off: no conflict. Merged already (Script Merger's folder has the file): no warning.
    let mut off = b.clone();
    off.enabled = false;
    assert!(mod_checks_with(&r, &[a.clone(), off], &[]).is_empty());
    let merged = m("Merged", &["mods/mod0000_MergedFiles/content/scripts/game/player/r4Player.ws", "mods/mod0000_MergedFiles/content/scripts/game/gui/x.ws"]);
    assert!(mod_checks_with(&r, &[a, b, merged], &[]).is_empty());
}

#[test]
fn installing_through_the_store_then_writing_mods_settings() {
    let t = TempDir::new("w3-store");
    let dir = t.path("lib/The Witcher 3");
    fs::create_dir_all(&dir).unwrap();
    let pfx = t.path("compatdata/292030/pfx");
    fs::create_dir_all(pfx.join(witcher::DOCS_IN_PREFIX)).unwrap();
    let g = w3_game(&dir, Some(pfx.clone()));
    let mut s = Store::open(&g, &t.path("data")).unwrap();
    let src = || SourceInfo { kind: "zip".into(), ..SourceInfo::default() };
    let a = t.path("dl/Fast Travel.zip");
    fs::create_dir_all(a.parent().unwrap()).unwrap();
    write_zip(&a, &[("modFastTravel/content/scripts/game/a.ws", b"ws"), ("input.settings.txt", b"[Exploration]\nIK_NumPad0=(Action=Teleport)\n")]);
    let Import::Added(ft) = s.import_archive(&a, src()).unwrap() else { panic!() };
    assert!(ft.notes.iter().any(|n| n.contains("input.settings")), "{:?}", ft.notes);
    let b = t.path("dl/Icons.zip");
    write_zip(&b, &[("Icons/content/blob0.bundle", b"bundle")]);
    let Import::Added(icons) = s.import_archive(&b, src()).unwrap() else { panic!() };
    s.apply().unwrap();
    assert_eq!(fs::read(dir.join("mods/modFastTravel/content/scripts/game/a.ws")).unwrap(), b"ws");
    assert!(dir.join("mods/modIcons/content/blob0.bundle").exists());

    let files = s.mod_files();
    let pairs: Vec<(&[String], bool)> = files.iter().map(|m| (m.files.as_slice(), m.enabled)).collect();
    assert!(witcher::sync_mods_settings(g.prefix.as_deref(), &pairs).unwrap());
    let text = fs::read_to_string(pfx.join(witcher::DOCS_IN_PREFIX).join(MODS_SETTINGS)).unwrap();
    assert_eq!(text, "[modFastTravel]\r\nEnabled=1\r\nPriority=2\r\n\r\n[modIcons]\r\nEnabled=1\r\nPriority=1\r\n");
    // Turning one off and moving the other: written again from the new order.
    s.set_enabled(&icons.item.id, false).unwrap();
    s.remove(&ft.item.id).unwrap();
    let files = s.mod_files();
    assert_eq!(files.iter().filter(|m| !m.enabled).count(), 2, "removed mods still count as Turbine's");
    let pairs: Vec<(&[String], bool)> = files.iter().map(|m| (m.files.as_slice(), m.enabled)).collect();
    assert!(witcher::sync_mods_settings(g.prefix.as_deref(), &pairs).unwrap());
    assert_eq!(fs::read_to_string(pfx.join(witcher::DOCS_IN_PREFIX).join(MODS_SETTINGS)).unwrap(), "");
}
