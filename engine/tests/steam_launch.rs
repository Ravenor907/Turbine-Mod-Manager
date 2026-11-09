//! 0.58: modded launches go through Steam. The helper Turbine writes runs me3 only when Turbine armed the game, keeps
//! Steam's reaper and launch wrapper in front, runs Steam's own command otherwise, and logs every step.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use turbine_engine::anticheat::{self, Coop};

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("turbine-steamlaunch-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

/// A program that writes its own arguments to `out`.
fn recorder(dir: &Path, name: &str, out: &Path) -> PathBuf {
    let p = dir.join(name);
    fs::write(&p, format!("#!/bin/sh\nprintf '%s|' \"$(basename \"$0\")\" \"$@\" > '{}'\n", out.display())).unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
    p
}

fn run(data: &Path, args: &[&str]) -> bool {
    Command::new(anticheat::helper_path(data)).args(args).env("SteamAppId", "1245620").status().unwrap().success()
}

#[test]
fn launch_option_text_and_check() {
    let data = Path::new("/home/deck/.var/app/x/data");
    let opt = anticheat::launch_option(data);
    assert_eq!(opt, "/home/deck/.var/app/x/data/steam-launch/turbine-launch.sh %command%");
    assert!(anticheat::launch_option_ok(&opt, data));
    assert!(anticheat::launch_option_ok(&format!("PROTON_LOG=1 {opt}"), data));
    assert!(!anticheat::launch_option_ok("", data));
    assert!(!anticheat::launch_option_ok("%command% /home/deck/.var/app/x/data/steam-launch/turbine-launch.sh", data));
    let spaced = anticheat::launch_option(Path::new("/home/a b/data"));
    assert!(spaced.starts_with('"'), "{spaced}");
}

#[test]
fn helper_runs_steam_or_me3() {
    if Command::new("bash").arg("--version").output().is_err() {
        return;
    }
    let d = tmp("helper");
    let data = d.join("data");
    assert!(anticheat::write_helper(&data).unwrap());
    assert!(!anticheat::write_helper(&data).unwrap(), "unchanged: not written again");
    let out = d.join("out.txt");
    let reaper = recorder(&d, "reaper", &out);
    let wrapper = d.join("steam-launch-wrapper");
    let game = recorder(&d, "proton", &out);
    let me3 = recorder(&d, "me3", &out);
    let steam: Vec<String> = vec![
        reaper.display().to_string(),
        "SteamLaunch".into(),
        "AppId=1245620".into(),
        "--".into(),
        wrapper.display().to_string(),
        "--".into(),
        game.display().to_string(),
        "waitforexitandrun".into(),
        "start_protected_game.exe".into(),
    ];
    let steam_ref: Vec<&str> = steam.iter().map(String::as_str).collect();

    // Not armed: Steam's own command, unchanged.
    assert!(run(&data, &steam_ref));
    assert!(fs::read_to_string(&out).unwrap().starts_with("reaper|SteamLaunch|AppId=1245620|--|"));
    assert!(fs::read_to_string(&out).unwrap().contains("start_protected_game.exe"));

    // Armed: me3 with Turbine's arguments behind Steam's reaper and launch wrapper. The fake reaper records only
    // itself, so check what it was given: reaper, then the wrapper, then me3.
    let args = vec![me3.display().to_string(), "launch".into(), "-p".into(), "/x/turbine.me3".into()];
    anticheat::arm(&data, "1245620", Some(&args)).unwrap();
    assert!(run(&data, &steam_ref));
    let got = fs::read_to_string(&out).unwrap();
    assert!(got.starts_with("reaper|SteamLaunch|AppId=1245620|--|"), "{got}");
    assert!(got.contains(&format!("{}|--|{}|launch|-p|/x/turbine.me3|", wrapper.display(), me3.display())), "{got}");
    assert!(!got.contains("start_protected_game"), "{got}");

    // Steam's command without its reaper in front (launch options inside it): me3 alone.
    assert!(run(&data, &[game.to_str().unwrap(), "waitforexitandrun", "start_protected_game.exe"]));
    assert_eq!(fs::read_to_string(&out).unwrap(), "me3|launch|-p|/x/turbine.me3|");

    // One Play without mods: Steam's command once, then armed again.
    anticheat::arm_plain_once(&data, "1245620").unwrap();
    assert!(run(&data, &steam_ref));
    assert!(fs::read_to_string(&out).unwrap().contains("start_protected_game.exe"));
    assert!(!anticheat::plain_file(&data, "1245620").exists());
    assert!(run(&data, &steam_ref));
    assert!(!fs::read_to_string(&out).unwrap().contains("start_protected_game.exe"));

    // me3 gone: Steam's own command, and the log says why.
    fs::remove_file(&me3).unwrap();
    assert!(run(&data, &steam_ref));
    assert!(fs::read_to_string(&out).unwrap().contains("start_protected_game.exe"));
    let log = fs::read_to_string(anticheat::helper_log(&data)).unwrap();
    assert!(log.contains("No mods on in Turbine"), "{log}");
    assert!(log.contains("Mods on:"), "{log}");
    assert!(log.contains("Play without mods (once)"), "{log}");
    assert!(log.contains("isn't where Turbine put it"), "{log}");
    assert!(log.contains("Steam's Proton for this game"), "{log}");
    assert_eq!(anticheat::helper_log_since(&data, 0).len(), log.lines().count().min(40));

    // Off again.
    anticheat::arm(&data, "1245620", None).unwrap();
    anticheat::arm(&data, "1245620", None).unwrap();
    assert!(!anticheat::armed_file(&data, "1245620").exists());
    assert!(anticheat::arm(&data, "12a", None).is_err());
    assert!(anticheat::arm(&data, "1", Some(&["a\nb".to_string()])).is_err());
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn solo_and_coop_profiles() {
    let d = tmp("profiles");
    let mods = anticheat::profile_mods(&d);
    fs::create_dir_all(mods.join("SeamlessCoop")).unwrap();
    fs::write(mods.join("SeamlessCoop/ersc.dll"), "x").unwrap();
    fs::write(mods.join("other.dll"), "x").unwrap();
    let (solo, n) = anticheat::write_profile_for(&d, "eldenring", Coop::Solo).unwrap();
    assert_eq!(n, vec!["mods/other.dll".to_string()]);
    let t = fs::read_to_string(&solo).unwrap();
    assert!(t.contains(anticheat::ME3_SAVEFILE) && !t.contains("ersc"), "{t}");
    let (coop, n) = anticheat::write_profile_for(&d, "eldenring", Coop::Seamless).unwrap();
    assert_eq!(n.len(), 2);
    assert_ne!(solo, coop);
    let t = fs::read_to_string(&coop).unwrap();
    assert!(!t.contains("savefile") && t.contains("ersc.dll"), "{t}");
    fs::remove_dir_all(mods.join("SeamlessCoop")).unwrap();
    assert!(anticheat::write_profile_for(&d, "eldenring", Coop::Seamless).is_err());
    let _ = fs::remove_dir_all(&d);
}
