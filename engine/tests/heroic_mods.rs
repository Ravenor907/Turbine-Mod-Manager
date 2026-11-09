//! 0.6 (integration audit B4): Heroic games get the Wine or Proton DLL overrides their mod frameworks need, from
//! Heroic's own game settings, and a Heroic game with a script loader starts the loader through Heroic's link.

use std::fs;
use std::path::{Path, PathBuf};
use turbine_engine::games::{Runtime, Scan, Store};
use turbine_engine::{heroic, proton, rules};

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("turbine-heroicmods-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

const USER_REG: &str = "WINE REGISTRY Version 2\n;; All keys relative to \\\\User\\\\S-1-5-21-0-0-0-1000\n\n#arch=win64\n\n[Software\\\\Wine] 1762650000\n#time=1dc5180a1b2c3d4\n\"Version\"=\"win10\"\n";

/// A fake Heroic (native config folder) with GOG Cyberpunk 2077 and GOG Stardew Valley (Windows builds).
fn fake_heroic(home: &Path, wine_type: &str) -> PathBuf {
    let root = home.join(".config/heroic");
    fs::create_dir_all(root.join("gog_store")).unwrap();
    fs::create_dir_all(root.join("GamesConfig")).unwrap();
    let cp = home.join("Games/Heroic/Cyberpunk 2077");
    let sv = home.join("Games/Heroic/Stardew Valley");
    fs::create_dir_all(cp.join("bin/x64")).unwrap();
    fs::create_dir_all(&sv).unwrap();
    fs::write(cp.join("bin/x64/Cyberpunk2077.exe"), "x").unwrap();
    fs::write(sv.join("Stardew Valley.exe"), "x").unwrap();
    fs::write(
        root.join("gog_store/installed.json"),
        format!(
            r#"{{"installed":[{{"appName":"1423049311","install_path":"{}","platform":"windows","version":"2.3"}},{{"appName":"1453375253","install_path":"{}","platform":"windows","version":"1.6"}}]}}"#,
            cp.display(),
            sv.display()
        ),
    )
    .unwrap();
    fs::write(root.join("gog_store/library.json"), r#"{"games":[{"app_name":"1423049311","title":"Cyberpunk 2077"},{"app_name":"1453375253","title":"Stardew Valley"}]}"#).unwrap();
    let prefix = home.join("Games/Heroic/Prefixes/default/Cyberpunk 2077");
    let reg_dir = if wine_type == "proton" { prefix.join("pfx") } else { prefix.clone() };
    fs::create_dir_all(&reg_dir).unwrap();
    fs::write(reg_dir.join("user.reg"), USER_REG).unwrap();
    fs::write(
        root.join("GamesConfig/1423049311.json"),
        format!(r#"{{"1423049311":{{"winePrefix":"{}","wineVersion":{{"type":"{wine_type}","name":"x"}}}},"version":"v0","explicit":true}}"#, prefix.display()),
    )
    .unwrap();
    reg_dir
}

#[test]
fn heroic_prefix_gets_the_overrides() {
    for wine_type in ["proton", "wine"] {
        let home = tmp(wine_type);
        let reg_dir = fake_heroic(&home, wine_type);
        let mut scan = Scan::default();
        heroic::scan(&home, &mut scan);
        let cp = scan.games.iter().find(|g| g.store_id == "1423049311").expect("Cyberpunk found");
        assert_eq!(cp.store, Store::Gog);
        assert_eq!(cp.runtime, Runtime::Windows);
        assert_eq!(cp.prefix.as_deref(), Some(reg_dir.as_path()), "{wine_type}");
        // CET and RED4ext on: winmm and version go native,builtin in that prefix's user.reg.
        let r = rules::GameRules::for_game(cp);
        let files = vec!["bin/x64/winmm.dll".to_string(), "bin/x64/version.dll".to_string(), "red4ext/RED4ext.dll".to_string(), "bin/x64/plugins/cyber_engine_tweaks.asi".to_string()];
        let wanted = proton::wanted_overrides(cp, &r, &files);
        assert!(!wanted.is_empty(), "{wanted:?}");
        proton::ensure_overrides(cp.prefix.as_ref().unwrap(), &wanted).unwrap();
        for (dll, _) in &wanted {
            assert_eq!(proton::read_override(&reg_dir, dll).unwrap().as_deref(), Some("native,builtin"), "{dll}");
        }
        let reg = fs::read_to_string(reg_dir.join("user.reg")).unwrap();
        assert!(reg.contains("\"Version\"=\"win10\""), "the rest of the registry stays");
        assert!(reg_dir.join(proton::BACKUP_NAME).is_file(), "a copy kept first");
        let _ = fs::remove_dir_all(&home);
    }
}

#[test]
fn heroic_default_prefix_and_none() {
    let home = tmp("default");
    let root = home.join(".config/heroic");
    fs::create_dir_all(&root).unwrap();
    // No game settings file: Heroic's default folder by title, when it is there.
    assert_eq!(heroic::prefix_for(&root, &home, "123", "Some Game"), None);
    let d = home.join("Games/Heroic/Prefixes/default/Some Game");
    fs::create_dir_all(&d).unwrap();
    assert_eq!(heroic::prefix_for(&root, &home, "123", "Some Game"), Some(d.clone()));
    fs::write(root.join("config.json"), r#"{"defaultSettings":{"defaultWinePrefix":"~/P","wineVersion":{"type":"proton"}}}"#).unwrap();
    fs::create_dir_all(home.join("P/Some Game")).unwrap();
    assert_eq!(heroic::prefix_for(&root, &home, "123", "Some Game"), Some(home.join("P/Some Game/pfx")));
    assert_eq!(heroic::prefix_for(&root, &home, "../x", "Some Game"), None);
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn heroic_link_starts_the_loader() {
    let home = tmp("altexe");
    fake_heroic(&home, "proton");
    let mut scan = Scan::default();
    heroic::scan(&home, &mut scan);
    let sv = scan.games.iter().find(|g| g.store_id == "1453375253").expect("Stardew found");
    let r = rules::GameRules::for_game(sv);
    assert!(r.launch_help.is_none(), "no Steam Launch Option wording for a Heroic game: {:?}", r.launch_help);
    // Without SMAPI: the plain link.
    assert_eq!(rules::heroic_loader_exe(sv, &r), None);
    assert_eq!(rules::launch_url(sv).unwrap(), "heroic://launch?appName=1453375253&runner=gog");
    // With SMAPI in the game folder: Heroic starts it.
    let smapi = sv.install_dir.join("StardewModdingAPI.exe");
    fs::write(&smapi, "x").unwrap();
    let exe = rules::heroic_loader_exe(sv, &r).unwrap();
    assert_eq!(exe, smapi);
    let url = rules::launch_url_with(sv, Some(&exe)).unwrap();
    assert!(url.starts_with("heroic://launch?appName=1453375253&runner=gog&altExe=%2F"), "{url}");
    assert!(url.ends_with("Stardew%20Valley%2FStardewModdingAPI.exe"), "{url}");
    // Play original keeps the plain link.
    assert_eq!(rules::launch_url(sv).unwrap(), "heroic://launch?appName=1453375253&runner=gog");
    let _ = fs::remove_dir_all(&home);
}
