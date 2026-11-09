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
    let h = "/home/deck/.var/app/x/data/steam-launch/turbine-launch.sh";
    assert_eq!(opt, format!("if [ -x \"{h}\" ]; then \"{h}\" %command%; else %command%; fi"));
    assert!(anticheat::launch_option_ok(&opt, data));
    assert!(anticheat::launch_option_ok(&format!("PROTON_LOG=1 {opt}"), data));
    assert!(!anticheat::launch_option_ok("", data));
    assert!(!anticheat::launch_option_ok("%command% /home/deck/.var/app/x/data/steam-launch/turbine-launch.sh", data));
    let spaced = anticheat::launch_option(Path::new("/home/a b/data"));
    assert!(spaced.contains("\"/home/a b/data/steam-launch/turbine-launch.sh\" %command%"), "{spaced}");
}

#[test]
fn helper_runs_steam_or_me3() {
    if Command::new("bash").arg("--version").output().is_err() {
        return;
    }
    let d = tmp("helper");
    let data = d.join("data");
    assert!(anticheat::write_helper(&data, None).unwrap());
    assert!(!anticheat::write_helper(&data, None).unwrap(), "unchanged: not written again");
    let out = d.join("out.txt");
    let reaper = recorder(&d, "reaper", &out);
    let wrapper = d.join("steam-launch-wrapper");
    let game = recorder(&d, "proton", &out);
    let bin = data.join("me3/0.13.0/bin");
    fs::create_dir_all(&bin).unwrap();
    let me3 = recorder(&bin, "me3", &out);
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

/// 0.58 break test: the Launch Options line starts the game normally when the helper is gone; the helper uses only
/// Turbine's own me3; the next-Play-only file is used once; a leftover Play without mods never outlives a new state;
/// Turbine uninstalled (its data kept) means Steam's own command.
#[test]
fn line_falls_back_and_helper_keeps_to_turbine() {
    if Command::new("bash").arg("--version").output().is_err() {
        return;
    }
    let d = tmp("fallback");
    let data = d.join("data");
    let out = d.join("out.txt");
    let game = recorder(&d, "proton", &out);
    // Steam runs the line through its shell with %command% put in.
    let line = anticheat::launch_option(&data).replace("%command%", &format!("'{}' run start_protected_game.exe", game.display()));
    let st = Command::new("sh").arg("-c").arg(&line).env("SteamAppId", "1245620").status().unwrap();
    assert!(st.success());
    assert_eq!(fs::read_to_string(&out).unwrap(), "proton|run|start_protected_game.exe|", "no helper: Steam's own command");

    anticheat::write_helper(&data, None).unwrap();
    let bin = data.join("me3/0.13.0/bin");
    fs::create_dir_all(&bin).unwrap();
    let me3 = recorder(&bin, "me3", &out);
    let solo = vec![me3.display().to_string(), "-p".into(), "solo.me3".into()];
    let coop = vec![me3.display().to_string(), "-p".into(), "coop.me3".into()];
    anticheat::arm_with(&data, "1245620", Some(&solo), "mods on: 2 mods, solo").unwrap();
    assert!(fs::read_to_string(anticheat::armed_file(&data, "1245620")).unwrap().starts_with("# mods on: 2 mods, solo\n"));
    let run_line = |out: &Path| {
        let st = Command::new("sh").arg("-c").arg(&line).env("SteamAppId", "1245620").status().unwrap();
        assert!(st.success());
        fs::read_to_string(out).unwrap()
    };
    assert_eq!(run_line(&out), "me3|-p|solo.me3|");
    // Seamless Co-op for the next Play only.
    anticheat::arm_once(&data, "1245620", &coop, "Seamless Co-op, this Play only").unwrap();
    assert_eq!(run_line(&out), "me3|-p|coop.me3|");
    assert_eq!(run_line(&out), "me3|-p|solo.me3|", "back to the normal state");
    // A Play without mods waiting, then mods on again: the new state wins.
    anticheat::arm_plain_once(&data, "1245620").unwrap();
    anticheat::arm(&data, "1245620", Some(&solo)).unwrap();
    assert!(anticheat::plain_file(&data, "1245620").exists(), "the state written again keeps a waiting Play without mods");
    anticheat::clear_plain(&data, "1245620").unwrap();
    assert!(!anticheat::plain_file(&data, "1245620").exists());
    // Off clears a waiting next-Play-only file too.
    anticheat::arm_once(&data, "1245620", &coop, "co-op").unwrap();
    anticheat::arm(&data, "1245620", None).unwrap();
    assert!(!anticheat::once_file(&data, "1245620").exists());
    assert_eq!(run_line(&out), "proton|run|start_protected_game.exe|");
    // Any other program in the file: Steam's own command.
    let other = recorder(&d, "me3", &d.join("other.txt"));
    anticheat::arm(&data, "1245620", Some(&[other.display().to_string(), "-p".into(), "x".into()])).unwrap();
    assert_eq!(run_line(&out), "proton|run|start_protected_game.exe|");
    assert!(!d.join("other.txt").exists());
    // Turbine uninstalled but its data kept: Steam's own command.
    if Command::new("flatpak").arg("--version").output().is_ok() {
        anticheat::write_helper(&data, Some("io.example.NotInstalled")).unwrap();
        anticheat::arm(&data, "1245620", Some(&solo)).unwrap();
        assert_eq!(run_line(&out), "proton|run|start_protected_game.exe|");
    }
    // The original Launch Options are kept once, never Turbine's own line.
    anticheat::remember_original(&data, "1245620", "PROTON_LOG=1 %command%").unwrap();
    anticheat::remember_original(&data, "1245620", &anticheat::launch_option(&data)).unwrap();
    assert_eq!(anticheat::original_options(&data, "1245620"), "PROTON_LOG=1 %command%");
    let _ = fs::remove_dir_all(&d);
}

/// 0.58 break test H1: every way of turning mods off clears what Steam's Play starts.
#[test]
fn every_off_path_clears_the_helper() {
    use anticheat::helper_state;
    assert!(helper_state(2, true, false, false, true).is_ok());
    // No mods on (Reset game files, the last mod removed, a loadout with none on, every mod off).
    assert_eq!(helper_state(0, true, false, false, true), Err("no mods on"));
    // Anti-cheat back on in Game details.
    assert!(helper_state(2, false, false, false, true).unwrap_err().contains("anti-cheat"));
    // Held after a game update.
    assert!(helper_state(2, true, true, false, true).is_err());
    // A failed apply.
    assert!(helper_state(2, true, false, true, true).is_err());
    assert!(helper_state(2, true, false, false, false).is_err());

    // An old Play without mods (over ten minutes) is ignored.
    if Command::new("bash").arg("--version").output().is_err() || Command::new("touch").arg("--version").output().is_err() {
        return;
    }
    let d = tmp("oldplain");
    let data = d.join("data");
    anticheat::write_helper(&data, None).unwrap();
    let out = d.join("out.txt");
    let bin = data.join("me3/0.13.0/bin");
    fs::create_dir_all(&bin).unwrap();
    let me3 = recorder(&bin, "me3", &out);
    let game = recorder(&d, "proton", &out);
    anticheat::arm(&data, "1245620", Some(&[me3.display().to_string(), "-p".into(), "solo".into()])).unwrap();
    anticheat::arm_plain_once(&data, "1245620").unwrap();
    Command::new("touch").arg("-d").arg("20 minutes ago").arg(anticheat::plain_file(&data, "1245620")).status().unwrap();
    assert!(run(&data, &[game.to_str().unwrap(), "x"]));
    assert_eq!(fs::read_to_string(&out).unwrap(), "me3|-p|solo|");
    assert!(!anticheat::plain_file(&data, "1245620").exists());
    let _ = fs::remove_dir_all(&d);
}
