//! 0.62: Turbine's Steam compatibility tool. The script runs here behind stubs of Steam's chain (launch wrapper and
//! reaper in front, as Steam puts them) with stub runtimes, Protons and me3: each stub writes its arguments to a
//! trace and passes control on the way the real one does. Paths hold spaces, as on the device.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use turbine_engine::anticheat::{self, Coop};
use turbine_engine::compat_tool::{self as tool, Route, TileWay};

const APP: &str = "1245620";

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("turbine-compattool-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn has_bash() -> bool {
    Command::new("bash").arg("--version").output().is_ok()
}

fn write(p: &Path, text: &str) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

fn script(p: &Path, body: &str) {
    write(p, &format!("#!/bin/sh\n{body}"));
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(p, fs::Permissions::from_mode(0o755)).unwrap();
}

fn trace_line(name: &str, trace: &Path) -> String {
    format!("{{ printf '%s' '{name}'; for a in \"$@\"; do printf '|%s' \"$a\"; done; printf '\\n'; }} >> '{}'\n", trace.display())
}

/// A Steam folder with a second library, two Protons with their runtimes, the game, Turbine's data folder with me3
/// and the helper, and the tool installed.
struct Steam {
    d: PathBuf,
    root: PathBuf,
    sd: PathBuf,
    data: PathBuf,
    trace: PathBuf,
    wrapper: PathBuf,
    reaper: PathBuf,
    proton11: PathBuf,
    proton9: PathBuf,
    slr4: PathBuf,
    sniper: PathBuf,
    starter: PathBuf,
    game_exe: PathBuf,
    me3: PathBuf,
    me3_exe: PathBuf,
    profile: PathBuf,
}

/// A Proton folder: its program (traces its arguments and the tool paths it was given, then runs the program after
/// the verb) and its manifest naming the runtime it needs.
fn proton(dir: &Path, trace: &Path, name: &str, needs: Option<&str>) {
    script(
        &dir.join("proton"),
        &format!(
            "{}printf '{name}-env|%s|%s\\n' \"$STEAM_COMPAT_TOOL_PATHS\" \"$STEAM_COMPAT_MOUNTS\" >> '{t}'\nshift; exec \"$@\"\n",
            trace_line(name, trace),
            t = trace.display()
        ),
    );
    let need = needs.map(|n| format!("  \"require_tool_appid\" \"{n}\"\n")).unwrap_or_default();
    write(&dir.join("toolmanifest.vdf"), &format!("\"manifest\"\n{{\n  \"version\" \"2\"\n  \"commandline\" \"/proton %verb%\"\n{need}  \"use_sessions\" \"1\"\n  \"compatmanager_layer_name\" \"proton\"\n}}\n"));
}

/// A runtime: its app manifest in the library, its folder with the entry point and its own manifest.
fn runtime(lib: &Path, trace: &Path, app: &str, dir: &str, name: &str) -> PathBuf {
    write(&lib.join(format!("steamapps/appmanifest_{app}.acf")), &format!("\"AppState\"\n{{\n\t\"appid\"\t\t\"{app}\"\n\t\"name\"\t\t\"{dir}\"\n\t\"installdir\"\t\t\"{dir}\"\n}}\n"));
    let folder = lib.join("steamapps/common").join(dir);
    let skip = "while [ $# -gt 0 ] && [ \"$1\" != -- ]; do shift; done; shift; exec \"$@\"\n";
    script(&folder.join("_v2-entry-point"), &format!("{}{skip}", trace_line(name, trace)));
    write(&folder.join("toolmanifest.vdf"), "\"manifest\"\n{\n  \"version\" \"2\"\n  \"commandline\" \"/_v2-entry-point --verb=%verb% --\"\n  \"compatmanager_layer_name\" \"container-runtime\"\n}\n");
    folder
}

impl Steam {
    fn new(name: &str) -> Steam {
        let d = tmp(name);
        let trace = d.join("trace.txt");
        let root = d.join("Steam Root");
        let sd = d.join("SD Card");
        let wrapper = root.join("steamrt64/steam-launch-wrapper");
        let reaper = root.join("steamrt64/reaper");
        let skip = "while [ $# -gt 0 ] && [ \"$1\" != -- ]; do shift; done; shift; ";
        script(&wrapper, &format!("{}{skip}exec \"$@\"\n", trace_line("wrapper", &trace)));
        // Steam's reaper stays as the parent of what it starts.
        script(&reaper, &format!("{}{skip}\"$@\"\ncode=$?\nexit $code\n", trace_line("reaper", &trace)));
        let proton11 = root.join("steamapps/common/Proton 11.0");
        proton(&proton11, &trace, "proton11", Some("4183110"));
        let slr4 = runtime(&root, &trace, "4183110", "SteamLinuxRuntime_4", "slr4");
        // The older Proton and its runtime are in the second library.
        let proton9 = sd.join("steamapps/common/Proton 9.0 (Beta)");
        proton(&proton9, &trace, "proton9", Some("1628350"));
        let sniper = runtime(&sd, &trace, "1628350", "SteamLinuxRuntime_sniper", "sniper");
        write(
            &root.join("steamapps/libraryfolders.vdf"),
            &format!("\"libraryfolders\"\n{{\n\t\"0\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n\t\"1\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n}}\n", root.display(), sd.display()),
        );
        let game = root.join("steamapps/common/ELDEN RING/Game");
        let starter = game.join("start_protected_game.exe");
        let game_exe = game.join("eldenring.exe");
        script(&starter, &trace_line("game", &trace));
        script(&game_exe, &trace_line("eldenring", &trace));
        let data = d.join("data dir");
        let me3 = anticheat::me3_dir(&data);
        let me3_exe = me3.join("bin/win64/me3.exe");
        script(&me3_exe, &format!("{}echo 'me3 started the game'\nexit \"$(cat '{}' 2> /dev/null || echo 0)\"\n", trace_line("me3.exe", &trace), d.join("me3.code").display()));
        let store = root.join(".turbine-mod-manager/steam-1245620");
        let (profile, _) = anticheat::write_profile_for(&store, "eldenring", Coop::Solo).unwrap();
        anticheat::write_helper(&data, None).unwrap();
        tool::install(&root, &data).unwrap();
        Steam { d, root, sd, data, trace, wrapper, reaper, proton11, proton9, slr4, sniper, starter, game_exe, me3, me3_exe, profile }
    }

    fn tool(&self) -> PathBuf {
        tool::tool_dir(&self.root).join(tool::SCRIPT)
    }

    /// Steam's command for a game that uses the tool: its launch wrapper, its reaper, then the tool with the verb
    /// and the game's program.
    fn steam_line(&self, verb: &str, program: &Path) -> Vec<String> {
        vec![
            self.wrapper.display().to_string(),
            "--oom-score-adjust".into(),
            "900".into(),
            "--".into(),
            self.reaper.display().to_string(),
            "SteamLaunch".into(),
            format!("AppId={APP}"),
            "--".into(),
            self.tool().display().to_string(),
            verb.into(),
            program.display().to_string(),
        ]
    }

    fn run(&self, line: &[String]) -> (i32, Vec<Vec<String>>) {
        let _ = fs::remove_file(&self.trace);
        let mut c = Command::new(&line[0]);
        c.args(&line[1..])
            .env("SteamAppId", APP)
            .env("STEAM_COMPAT_APP_ID", APP)
            .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", &self.root)
            .env("STEAM_COMPAT_LIBRARY_PATHS", format!("{}/steamapps", self.root.display()))
            .env("STEAM_COMPAT_TOOL_PATHS", tool::tool_dir(&self.root))
            .env_remove("STEAM_COMPAT_MOUNTS")
            .env_remove("TURBINE_TOOL_REAPER")
            .env_remove("PROTON_LOG")
            .env_remove("PROTON_LOG_DIR");
        let code = c.status().unwrap().code().unwrap_or(-1);
        let t = fs::read_to_string(&self.trace).unwrap_or_default();
        (code, t.lines().map(|l| l.split('|').map(str::to_string).collect()).collect())
    }

    /// A game launch through the tool.
    fn play(&self) -> (i32, Vec<Vec<String>>) {
        self.run(&self.steam_line("waitforexitandrun", &self.starter))
    }

    fn arm(&self) -> Vec<String> {
        let a = anticheat::helper_args_for(&self.me3, &self.profile, Some(&self.game_exe));
        anticheat::arm(&self.data, APP, Some(&a)).unwrap();
        a
    }

    fn log(&self) -> String {
        fs::read_to_string(anticheat::helper_log(&self.data)).unwrap_or_default()
    }

    fn log_lines(&self) -> Vec<String> {
        self.log().lines().map(str::to_string).collect()
    }

    fn done(self) {
        let _ = fs::remove_dir_all(&self.d);
    }
}

fn names(t: &[Vec<String>]) -> Vec<&str> {
    t.iter().map(|l| l[0].as_str()).filter(|n| !n.ends_with("-env")).collect()
}

fn line<'a>(t: &'a [Vec<String>], name: &str) -> &'a [String] {
    t.iter().find(|l| l[0] == name).unwrap_or_else(|| panic!("no {name} in {t:?}"))
}

#[test]
fn the_three_files() {
    let v = tool::compatibilitytool_vdf();
    let parsed = turbine_engine::vdf::parse(&v).unwrap();
    let t = parsed.path(&["compatibilitytools", "compat_tools", "turbine-mods"]).unwrap();
    assert_eq!(t.str_at(&["display_name"]), Some("Turbine (mods)"));
    assert_eq!(t.str_at(&["install_path"]), Some("."));
    assert_eq!((t.str_at(&["from_oslist"]), t.str_at(&["to_oslist"])), (Some("windows"), Some("linux")));
    let m = turbine_engine::vdf::parse(&tool::toolmanifest_vdf()).unwrap();
    assert_eq!(m.str_at(&["manifest", "version"]), Some("2"));
    assert_eq!(m.str_at(&["manifest", "commandline"]), Some("/turbine-tool %verb%"));
    // No required tool: the script starts the runtime the chosen Proton needs.
    assert_eq!(m.str_at(&["manifest", "require_tool_appid"]), None);
    let s = tool::script_text(Path::new("/home/deck/.var/app/x/data/steam-launch"));
    assert!(s.starts_with("#!/bin/bash\n"));
    assert!(s.contains("launch='/home/deck/.var/app/x/data/steam-launch'\n"));
    assert!(!s.contains("@LAUNCH_DIR@"));
    for text in [v, tool::toolmanifest_vdf(), s] {
        assert!(!text.contains('\u{2014}') && !text.contains('\u{2013}'));
    }
}

#[test]
fn install_writes_only_what_differs_and_never_goes_while_a_game_uses_it() {
    let d = tmp("install");
    let root = d.join("Steam");
    let data = d.join("data");
    assert!(tool::install(&root, &data).is_err(), "no Steam folder");
    fs::create_dir_all(&root).unwrap();
    let first = tool::install(&root, &data).unwrap();
    assert!(first.new_folder);
    assert_eq!(first.written.len(), 3);
    assert!(tool::is_installed(&root, &data));
    let dir = tool::tool_dir(&root);
    assert_eq!(dir, root.join("compatibilitytools.d/TurbineMods"));
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(fs::metadata(dir.join("turbine-tool")).unwrap().permissions().mode() & 0o111, 0o111);
    // Again: nothing written.
    let again = tool::install(&root, &data).unwrap();
    assert_eq!(again, tool::Installed { written: vec![], new_folder: false });
    // A changed script (an older Turbine's) is replaced; the others stay.
    fs::write(dir.join("turbine-tool"), "old").unwrap();
    assert!(!tool::is_installed(&root, &data));
    assert_eq!(tool::install(&root, &data).unwrap().written, vec!["turbine-tool".to_string()]);
    // Steam sees it after it starts again.
    let made = fs::metadata(dir.join("compatibilitytool.vdf")).unwrap().modified().unwrap();
    assert!(!tool::seen_by_steam(&root, None));
    assert!(!tool::seen_by_steam(&root, Some(made - std::time::Duration::from_secs(60))), "Steam started before the tool was written");
    assert!(tool::seen_by_steam(&root, Some(made + std::time::Duration::from_secs(60))));
    // config.vdf: read only.
    let config = "\"InstallConfigStore\"\n{\n\t\"Software\"\n\t{\n\t\t\"Valve\"\n\t\t{\n\t\t\t\"Steam\"\n\t\t\t{\n\t\t\t\t\"CompatToolMapping\"\n\t\t\t\t{\n\t\t\t\t\t\"0\"\n\t\t\t\t\t{\n\t\t\t\t\t\t\"name\"\t\t\"proton_experimental\"\n\t\t\t\t\t\t\"config\"\t\t\"\"\n\t\t\t\t\t\t\"priority\"\t\t\"75\"\n\t\t\t\t\t}\n\t\t\t\t\t\"1245620\"\n\t\t\t\t\t{\n\t\t\t\t\t\t\"name\"\t\t\"turbine-mods\"\n\t\t\t\t\t\t\"config\"\t\t\"\"\n\t\t\t\t\t\t\"priority\"\t\t\"250\"\n\t\t\t\t\t}\n\t\t\t\t\t\"814380\"\n\t\t\t\t\t{\n\t\t\t\t\t\t\"name\"\t\t\"GE-Proton10-4\"\n\t\t\t\t\t\t\"config\"\t\t\"\"\n\t\t\t\t\t\t\"priority\"\t\t\"250\"\n\t\t\t\t\t}\n\t\t\t\t}\n\t\t\t}\n\t\t}\n\t}\n}\n";
    assert_eq!(tool::chosen_in(config, "1245620").as_deref(), Some("turbine-mods"));
    assert_eq!(tool::chosen_in(config, "814380").as_deref(), Some("GE-Proton10-4"));
    assert_eq!(tool::chosen_in(config, "1888160"), None);
    assert_eq!(tool::chosen_in("not vdf {", "1245620"), None);
    assert_eq!(tool::games_using_in(config), vec!["1245620".to_string()]);
    write(&tool::config_file(&root), config);
    assert!(tool::is_chosen(&root, "1245620") && !tool::is_chosen(&root, "814380"));
    let before = fs::read(tool::config_file(&root)).unwrap();
    // The folder stays while a game names the tool.
    assert_eq!(tool::remove(&root), Err(vec!["1245620".to_string()]));
    assert!(tool::is_installed(&root, &data));
    assert!(tool::still_used_words(&["Elden Ring".to_string()]).contains("Elden Ring still uses it"));
    write(&tool::config_file(&root), &config.replace("turbine-mods", "proton_11"));
    assert_eq!(tool::remove(&root), Ok(true));
    assert!(!dir.exists());
    assert_eq!(tool::remove(&root), Ok(false));
    assert_eq!(before, config.as_bytes(), "config.vdf is never written by the tool code");
    let _ = fs::remove_dir_all(&d);
}

/// The owner's chain: Proton 11 with SteamLinuxRuntime_4, mods on.
#[test]
fn proton_11_with_its_runtime_and_mods_on() {
    if !has_bash() {
        return;
    }
    let s = Steam::new("p11");
    // Turbine kept the Proton Steam last used for the game.
    tool::write_proton_choice(&s.data, APP, &s.proton11).unwrap();
    let args = s.arm();
    let (code, t) = s.play();
    assert_eq!(code, 0, "{t:?}\n{}", s.log());
    assert_eq!(names(&t), ["wrapper", "reaper", "slr4", "proton11", "me3.exe"], "{t:?}");
    // The runtime's own command with the verb, then Proton with the verb, then me3.exe in place of the game.
    let rt = line(&t, "slr4");
    assert_eq!(&rt[1..], ["--verb=waitforexitandrun", "--", &s.proton11.join("proton").display().to_string(), "waitforexitandrun", &args[0], "launch", "--exe", &args[3], "-p", &args[5]], "{t:?}");
    assert_eq!(line(&t, "me3.exe")[1..], args[1..], "{t:?}");
    // Steam's tool paths as Steam sets them for a Proton launch: Proton first, then its runtime.
    let env = line(&t, "proton11-env");
    assert_eq!(env[1], format!("{}:{}", s.proton11.display(), s.slr4.display()));
    assert_eq!(env[2], env[1]);
    // Every step is in steam-launch.log, with me3's exit code and its last lines, exactly as the helper writes them.
    let log = s.log();
    assert!(log.contains(&format!("Turbine (mods): Steam started app {APP} (waitforexitandrun)")), "{log}");
    assert!(log.contains(&format!("Turbine (mods): Proton for this game: {} [why: the Proton Steam last used for this game, kept by Turbine]", s.proton11.display())), "{log}");
    assert!(log.contains(&format!("Turbine (mods): its runtime: {}", s.slr4.display())), "{log}");
    assert!(log.contains("handing over to Turbine's launch helper (Steam's reaper above: yes)"), "{log}");
    assert!(log.contains("Mods on (windows me3): "), "{log}");
    assert!(log.contains("me3 ended with exit code 0 after "), "{log}");
    assert!(log.contains(" me3: me3 started the game"), "{log}");
    assert!(tool::launched_through_tool_in(&log, APP));
    assert!(!tool::launched_through_tool_in(&log, "814380"));
    // me3's own exit code comes back to Steam.
    fs::write(s.d.join("me3.code"), "3").unwrap();
    let (code, _) = s.play();
    assert_eq!(code, 3);
    assert!(s.log().contains("me3 ended with exit code 3 after "));
    s.done();
}

#[test]
fn no_mods_on_runs_the_normal_game_through_the_same_line() {
    if !has_bash() {
        return;
    }
    let s = Steam::new("plain");
    let (code, t) = s.play();
    assert_eq!(code, 0, "{}", s.log());
    assert_eq!(names(&t), ["wrapper", "reaper", "slr4", "proton11", "game"], "{t:?}");
    assert_eq!(line(&t, "proton11")[1..], ["waitforexitandrun".to_string(), s.starter.display().to_string()]);
    assert!(s.log().contains("No mods on in Turbine: Steam's own command"));
    // Play without mods once, with mods on: the normal game, and the once file is used up by the helper.
    s.arm();
    anticheat::arm_plain_once(&s.data, APP).unwrap();
    let (_, t) = s.play();
    assert_eq!(names(&t), ["wrapper", "reaper", "slr4", "proton11", "game"], "{t:?}");
    assert!(!anticheat::plain_file(&s.data, APP).exists());
    let (_, t) = s.play();
    assert!(names(&t).contains(&"me3.exe"), "{t:?}");
    // Extra arguments Steam adds after the program stay after it.
    let mut l = s.steam_line("waitforexitandrun", &s.starter);
    l.push("-fullscreen".into());
    anticheat::arm(&s.data, APP, None).unwrap();
    let (_, t) = s.run(&l);
    assert_eq!(line(&t, "game")[1..], ["-fullscreen".to_string()]);
    s.done();
}

/// An older Proton (in a second library, a folder name with spaces and brackets) asks for sniper, not SLR 4.
#[test]
fn an_older_proton_gets_its_own_runtime() {
    if !has_bash() {
        return;
    }
    let s = Steam::new("p9");
    tool::write_proton_choice(&s.data, APP, &s.proton9).unwrap();
    let args = s.arm();
    let (code, t) = s.play();
    assert_eq!(code, 0, "{}", s.log());
    assert_eq!(names(&t), ["wrapper", "reaper", "sniper", "proton9", "me3.exe"], "{t:?}");
    assert_eq!(line(&t, "sniper")[1..4], ["--verb=waitforexitandrun".to_string(), "--".to_string(), s.proton9.join("proton").display().to_string()]);
    assert_eq!(line(&t, "proton9")[2], args[0]);
    assert_eq!(line(&t, "proton9-env")[1], format!("{}:{}", s.proton9.display(), s.sniper.display()));
    // A Proton that names no runtime (very old ones) runs as it is.
    let bare = s.root.join("steamapps/common/Proton 4.11");
    proton(&bare, &s.trace, "proton4", None);
    tool::write_proton_choice(&s.data, APP, &bare).unwrap();
    anticheat::arm(&s.data, APP, None).unwrap();
    let (code, t) = s.play();
    assert_eq!(code, 0);
    assert_eq!(names(&t), ["wrapper", "reaper", "proton4", "game"], "{t:?}");
    assert!(s.log().contains("this Proton names no runtime"));
    s.done();
}

/// No record of the Proton Steam used and several installed: the newest numbered one, and the log says which and why.
#[test]
fn without_a_record_the_newest_proton_is_picked_and_said() {
    if !has_bash() {
        return;
    }
    let s = Steam::new("pick");
    assert!(tool::proton_choice(&s.data, APP).is_none());
    // A Proton 10 too, so "11.0" must win over "9.0 (Beta)" and "10.0" by number, not by text.
    let p10 = s.sd.join("steamapps/common/Proton 10.0");
    proton(&p10, &s.trace, "proton10", Some("1628350"));
    proton(&s.root.join("steamapps/common/Proton - Experimental"), &s.trace, "protonexp", Some("4183110"));
    let (code, t) = s.play();
    assert_eq!(code, 0, "{}", s.log());
    assert_eq!(names(&t), ["wrapper", "reaper", "slr4", "proton11", "game"], "{t:?}");
    let lines = s.log_lines();
    let (dir, why) = tool::proton_used(&lines).unwrap();
    assert_eq!(dir, s.proton11.display().to_string());
    assert!(why.starts_with("the newest Proton installed (Turbine has no record of the one Steam used for this game; 3 numbered Proton found"), "{why}");
    assert_eq!(tool::proton_words(&lines).unwrap(), format!("Started with Proton 11.0: {why}."));
    assert!(tool::tool_lines(&lines).len() >= 4);
    // default.proton (Turbine's choice for every game) comes before the search.
    fs::write(anticheat::launch_dir(&s.data).join("default.proton"), format!("{}\r\n", p10.display())).unwrap();
    let (_, t) = s.play();
    assert_eq!(names(&t), ["wrapper", "reaper", "sniper", "proton10", "game"], "{t:?}");
    assert!(s.log().contains("[why: Turbine's choice for every game]"));
    // Only Experimental left: it is used, and the log says why.
    fs::remove_file(anticheat::launch_dir(&s.data).join("default.proton")).unwrap();
    for p in [&s.proton11, &s.proton9, &p10] {
        fs::remove_dir_all(p).unwrap();
    }
    let (code, t) = s.play();
    assert_eq!(code, 0);
    assert_eq!(names(&t), ["wrapper", "reaper", "slr4", "protonexp", "game"], "{t:?}");
    assert!(s.log().contains("[why: Proton Experimental (no numbered Proton could be used)]"));
    s.done();
}

#[test]
fn a_missing_runtime_moves_on_to_a_proton_that_can_run_or_says_so() {
    if !has_bash() {
        return;
    }
    let s = Steam::new("nort");
    tool::write_proton_choice(&s.data, APP, &s.proton9).unwrap();
    // The recorded Proton's runtime is gone: the next Proton that can run is used, and the log names both.
    fs::remove_dir_all(&s.sniper).unwrap();
    let (code, t) = s.play();
    assert_eq!(code, 0, "{}", s.log());
    assert_eq!(names(&t), ["wrapper", "reaper", "slr4", "proton11", "game"], "{t:?}");
    let log = s.log();
    assert!(log.contains(&format!("Turbine (mods): not used: {} (the Steam Linux Runtime it needs (Steam app 1628350) isn't installed)", s.proton9.display())), "{log}");
    assert!(log.contains(&format!("Proton for this game: {} [why: the newest Proton installed", s.proton11.display())), "{log}");
    // No runtime at all: nothing is started (never a Proton without the runtime it asks for), and the log says what to do.
    fs::remove_dir_all(&s.slr4).unwrap();
    s.arm();
    let (code, t) = s.play();
    assert_eq!(code, 1);
    assert_eq!(names(&t), ["wrapper", "reaper"], "{t:?}");
    let why = tool::nothing_started(&s.log_lines()).unwrap();
    assert!(why.starts_with("no Proton that can run was found in "), "{why}");
    assert!(why.contains("start any game with Proton once"), "{why}");
    assert!(anticheat::armed_file(&s.data, APP).exists(), "nothing of Turbine's state is used up");
    s.done();
}

#[test]
fn a_missing_proton_is_said_and_nothing_starts() {
    if !has_bash() {
        return;
    }
    let s = Steam::new("noproton");
    // The recorded Proton was uninstalled: the search goes on.
    tool::write_proton_choice(&s.data, APP, &s.d.join("gone/Proton 12.0")).unwrap();
    let (code, t) = s.play();
    assert_eq!(code, 0);
    assert!(names(&t).contains(&"proton11"), "{t:?}");
    assert!(s.log().contains("Proton 12.0 (no Proton there)"));
    // No Proton anywhere.
    fs::remove_dir_all(&s.proton11).unwrap();
    fs::remove_dir_all(&s.proton9).unwrap();
    let (code, t) = s.play();
    assert_eq!(code, 1);
    assert_eq!(names(&t), ["wrapper", "reaper"], "{t:?}");
    assert!(tool::nothing_started(&s.log_lines()).is_some());
    // A Proton from another tool folder (compatibilitytools.d) is the last resort.
    let ge = tool::tools_dir(&s.root).join("GE-Proton10-4");
    proton(&ge, &s.trace, "ge", Some("4183110"));
    let (code, t) = s.play();
    assert_eq!(code, 0, "{}", s.log());
    assert_eq!(names(&t), ["wrapper", "reaper", "slr4", "ge", "game"], "{t:?}");
    s.done();
}

#[test]
fn without_the_helper_the_game_still_starts() {
    if !has_bash() {
        return;
    }
    let s = Steam::new("nohelper");
    s.arm();
    fs::remove_file(anticheat::helper_path(&s.data)).unwrap();
    let (code, t) = s.play();
    assert_eq!(code, 0);
    assert_eq!(names(&t), ["wrapper", "reaper", "slr4", "proton11", "game"], "{t:?}");
    assert!(s.log().contains("Turbine's launch helper isn't there"));
    // Turbine's data folder gone altogether: the same, and the tool keeps its log in its own folder.
    fs::remove_dir_all(&s.data).unwrap();
    let (code, t) = s.play();
    assert_eq!(code, 0);
    assert_eq!(names(&t), ["wrapper", "reaper", "slr4", "proton11", "game"], "{t:?}");
    let own = fs::read_to_string(tool::tool_dir(&s.root).join("steam-launch.log")).unwrap();
    assert!(own.contains("the game starts the normal way, without mods"), "{own}");
    s.done();
}

/// Steam's set-up steps (install scripts) come with the verb `run`: they go through the same line and never to me3.
#[test]
fn the_run_verb_never_gets_mods() {
    if !has_bash() {
        return;
    }
    let s = Steam::new("run");
    tool::write_proton_choice(&s.data, APP, &s.proton11).unwrap();
    s.arm();
    let coop = vec![s.me3_exe.display().to_string(), "launch".to_string()];
    anticheat::arm_once(&s.data, APP, &coop, "co-op").unwrap();
    let setup = s.root.join("steamapps/common/Steamworks Shared/iscriptevaluator.exe");
    script(&setup, &trace_line("setup", &s.trace));
    let (code, t) = s.run(&s.steam_line("run", &setup));
    assert_eq!(code, 0, "{}", s.log());
    assert_eq!(names(&t), ["wrapper", "reaper", "slr4", "proton11", "setup"], "{t:?}");
    assert_eq!(line(&t, "slr4")[1], "--verb=run");
    assert_eq!(line(&t, "proton11")[1], "run");
    assert!(anticheat::once_file(&s.data, APP).exists(), "the this-Play-only file is for the game, not for a set-up step");
    assert!(s.log().contains("a set-up step of Steam's (run), never with mods"));
    // Anything else is not a launch Steam makes.
    let (code, t) = s.run(&s.steam_line("getcompatpath", &setup));
    assert_eq!(code, 1);
    assert_eq!(names(&t), ["wrapper", "reaper"]);
    // No program at all.
    let mut l = s.steam_line("waitforexitandrun", &setup);
    l.pop();
    assert_eq!(s.run(&l).0, 1);
    s.done();
}

#[test]
fn coop_once_and_the_reaper_rule() {
    if !has_bash() {
        return;
    }
    let s = Steam::new("once");
    let solo = s.arm();
    let mut coop = solo.clone();
    coop[5] = format!("{}-coop", solo[5]);
    // The co-op profile file must be there for the helper's check.
    fs::copy(&s.profile, format!("{}-coop", s.profile.display())).unwrap();
    anticheat::arm_once(&s.data, APP, &coop, "Seamless Co-op, this Play only").unwrap();
    let (_, t) = s.play();
    assert_eq!(line(&t, "me3.exe")[5], coop[5], "{t:?}");
    assert!(!anticheat::once_file(&s.data, APP).exists());
    let (_, t) = s.play();
    assert_eq!(line(&t, "me3.exe")[5], solo[5], "the next Play is the normal one again");
    // Without Steam's reaper above the tool (started by hand): the normal game, never me3.
    let (code, t) = s.run(&[s.tool().display().to_string(), "waitforexitandrun".into(), s.starter.display().to_string()]);
    assert_eq!(code, 0);
    assert_eq!(names(&t), ["slr4", "proton11", "game"], "{t:?}");
    assert!(s.log().contains("Steam's reaper with AppId=1245620 isn't in front of the game"));
    // A reaper for another app above it doesn't count either.
    let mut other = s.steam_line("waitforexitandrun", &s.starter);
    other[6] = "AppId=12456200".into();
    let (_, t) = s.run(&other);
    assert!(!names(&t).contains(&"me3.exe"), "{t:?}");
    s.done();
}

/// The Launch Options may still hold Turbine's line while the game uses the tool: Steam then runs the helper around
/// the tool. The outer helper must use nothing up (the this-Play-only files are for the helper the tool calls).
#[test]
fn launch_options_and_the_tool_together() {
    if !has_bash() {
        return;
    }
    let s = Steam::new("both");
    s.arm();
    anticheat::arm_plain_once(&s.data, APP).unwrap();
    let steam: Vec<String> = s.steam_line("waitforexitandrun", &s.starter).iter().map(|a| format!("'{a}'")).collect();
    let line_text = anticheat::launch_option(&s.data).replace("%command%", &steam.join(" "));
    let run = |text: &str| {
        let _ = fs::remove_file(&s.trace);
        let code = Command::new("sh")
            .arg("-c")
            .arg(text)
            .env("SteamAppId", APP)
            .env("STEAM_COMPAT_APP_ID", APP)
            .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", &s.root)
            .env_remove("STEAM_COMPAT_LIBRARY_PATHS")
            .env_remove("TURBINE_TOOL_REAPER")
            .status()
            .unwrap()
            .code()
            .unwrap_or(-1);
        let t = fs::read_to_string(&s.trace).unwrap_or_default();
        (code, t.lines().map(|l| l.split('|').map(str::to_string).collect::<Vec<String>>()).collect::<Vec<_>>())
    };
    // Play without mods once: honoured (by the inner helper), the normal game.
    let (code, t) = run(&line_text);
    assert_eq!(code, 0, "{}", s.log());
    assert_eq!(names(&t), ["wrapper", "reaper", "slr4", "proton11", "game"], "{t:?}");
    assert!(s.log().contains("Steam's command runs Turbine's Steam tool: it goes on from here"));
    assert!(s.log().contains("Play without mods (once)"));
    // The next Play: mods on, once.
    let (_, t) = run(&line_text);
    assert_eq!(names(&t), ["wrapper", "reaper", "slr4", "proton11", "me3.exe"], "{t:?}");
    s.done();
}

#[test]
fn the_proton_steam_last_used_is_kept_from_the_helpers_log() {
    // The owner's own line (Steam's command through the Launch Options helper).
    let log = "2026-11-09 17:05:58 Steam started app 1245620 in /home/deck/.local/share/Steam/steamapps/common/ELDEN RING/Game: /home/deck/.local/share/Steam/steamrt64/steam-launch-wrapper --oom-score-adjust 900 -- /home/deck/.local/share/Steam/steamrt64/reaper SteamLaunch AppId=1245620 -- /home/deck/.local/share/Steam/steamapps/common/SteamLinuxRuntime_4/_v2-entry-point --verb=waitforexitandrun -- /home/deck/.local/share/Steam/steamapps/common/Proton 11.0/proton waitforexitandrun /home/deck/.local/share/Steam/steamapps/common/ELDEN RING/Game/start_protected_game.exe\n2026-11-09 17:05:58 Kept in front: x\n";
    assert_eq!(tool::last_steam_proton_in(log, "1245620"), Some(PathBuf::from("/home/deck/.local/share/Steam/steamapps/common/Proton 11.0")));
    assert_eq!(tool::last_steam_proton_in(log, "814380"), None);
    assert!(!tool::launched_through_tool_in(log, "1245620"));
    // A launch through the tool is the tool's own pick, not Steam's: it is never taken as "what Steam used".
    let through = "2026-11-10 10:00:00 Turbine (mods): Steam started app 1245620 (waitforexitandrun) in /g: /g/start_protected_game.exe\n2026-11-10 10:00:00 Turbine (mods): Proton for this game: /s/Proton 9.0 (Beta) [why: the newest Proton installed]\n2026-11-10 10:00:00 Turbine (mods): handing over to Turbine's launch helper (Steam's reaper above: yes)\n2026-11-10 10:00:00 Steam started app 1245620 in /g: /s/SteamLinuxRuntime_sniper/_v2-entry-point --verb=waitforexitandrun -- /s/Proton 9.0 (Beta)/proton waitforexitandrun /g/start_protected_game.exe\n2026-11-10 10:00:00 No mods on in Turbine: Steam's own command\n";
    assert_eq!(tool::last_steam_proton_in(through, "1245620"), None);
    assert!(tool::launched_through_tool_in(through, "1245620"));
    let both = format!("{log}{through}");
    assert_eq!(tool::last_steam_proton_in(&both, "1245620"), Some(PathBuf::from("/home/deck/.local/share/Steam/steamapps/common/Proton 11.0")));
    // Back on plain Launch Options afterwards: the newest launch is not through the tool.
    let back = format!("{through}{log}");
    assert!(!tool::launched_through_tool_in(&back, "1245620"));

    // remember_proton writes it down only when that Proton is still there.
    let d = tmp("remember");
    let data = d.join("data");
    let p = d.join("lib/steamapps/common/Proton 11.0");
    write(&anticheat::helper_log(&data), &log.replace("/home/deck/.local/share/Steam/steamapps/common/Proton 11.0", &p.display().to_string()));
    assert_eq!(tool::remember_proton(&data, "1245620"), None, "not installed any more: nothing kept");
    write(&p.join("proton"), "x");
    write(&p.join("toolmanifest.vdf"), "x");
    assert_eq!(tool::remember_proton(&data, "1245620"), Some(p.clone()));
    assert_eq!(fs::read_to_string(tool::proton_file(&data, "1245620")).unwrap(), format!("{}\nsteam\n", p.display()));
    assert!(tool::write_proton_choice(&data, "12a", &p).is_err());
    assert!(tool::write_proton_choice(&data, "1", Path::new("relative/Proton")).is_err());
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn the_route_the_steps_and_the_tile_decision() {
    let d = tmp("route");
    let root = d.join("Steam");
    let data = d.join("data");
    fs::create_dir_all(&root).unwrap();
    let config = |name: &str| format!("\"InstallConfigStore\"\n{{\n\"Software\"\n{{\n\"valve\"\n{{\n\"Steam\"\n{{\n\"CompatToolMapping\"\n{{\n\"1245620\"\n{{\n\"name\"\t\"{name}\"\n\"config\"\t\"\"\n\"priority\"\t\"250\"\n}}\n}}\n}}\n}}\n}}\n}}\n");
    // Nothing set.
    assert_eq!(tool::route(&root, &data, "1245620", false), Route::NotSet);
    assert_eq!(tool::route(&root, &data, "1245620", true), Route::LaunchOptions);
    // config.vdf names the tool but its folder isn't there (deleted by hand): not counted as set.
    write(&tool::config_file(&root), &config("turbine-mods"));
    assert_eq!(tool::route(&root, &data, "1245620", false), Route::NotSet);
    tool::install(&root, &data).unwrap();
    assert_eq!(tool::route(&root, &data, "1245620", true), Route::Tool);
    assert_eq!(Route::Tool.word(), "tool");
    // Steam hasn't written config.vdf yet, but the last launch came through the tool.
    fs::remove_file(tool::config_file(&root)).unwrap();
    assert_eq!(tool::route(&root, &data, "1245620", false), Route::NotSet);
    write(&anticheat::helper_log(&data), "2026-11-10 10:00:00 Turbine (mods): Steam started app 1245620 (waitforexitandrun) in /g: /g/x.exe\n");
    assert_eq!(tool::route(&root, &data, "1245620", false), Route::Tool);
    // The player switched the game to another tool since: config.vdf wins over the old log.
    write(&tool::config_file(&root), &config("proton_11"));
    assert_eq!(tool::route(&root, &data, "1245620", false), Route::NotSet);

    let steps = tool::steps("Elden Ring", false);
    assert_eq!(steps.len(), 4);
    assert!(steps[0].starts_with("Restart Steam once"));
    let steps = tool::steps("Elden Ring", true);
    assert_eq!(steps.len(), 3, "the three steps");
    assert!(steps[0].contains("Elden Ring") && steps[0].contains("Properties") && steps[0].contains("Compatibility"));
    assert!(steps[2].contains("\"Turbine (mods)\""));

    // The Co-op tile: never by writing Steam's files in Game Mode.
    assert_eq!(tool::tile_way(true, false, true), TileWay::GameMode);
    assert_eq!(tool::tile_way(true, true, true), TileWay::GameModePlugin);
    assert_eq!(tool::tile_way(false, false, false), TileWay::DesktopSteamClosed);
    assert_eq!(tool::tile_way(false, true, false), TileWay::DesktopSteamClosed);
    assert_eq!(tool::tile_way(false, false, true), TileWay::DesktopRestart);
    assert_eq!((TileWay::GameMode.how(), TileWay::GameModePlugin.how()), ("later", "later"));
    assert_eq!((TileWay::DesktopSteamClosed.how(), TileWay::DesktopRestart.how()), ("direct", "restart"));
    // Nothing to press in Game Mode, and the words say where it can be done.
    for w in [TileWay::GameMode, TileWay::GameModePlugin] {
        assert_eq!((w.label(true), w.label(false)), ("", ""));
        assert!(w.words(true).contains("Desktop Mode") && w.words(false).contains("Desktop Mode"));
    }
    assert!(TileWay::GameModePlugin.words(true).contains("Steam menu"));
    assert!(!TileWay::GameMode.words(true).contains("Steam menu"), "no plugin, no promise of it");
    let j = turbine_engine::json::parse(&TileWay::DesktopSteamClosed.offer_json(true)).unwrap();
    assert_eq!((j.str_at("how"), j.str_at("label")), (Some("direct"), Some("Add it")));
    assert_eq!(turbine_engine::json::parse(tool::NO_TILE_JSON).unwrap().str_at("how"), Some("none"));
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn the_state_for_the_app_and_no_restart_in_game_mode() {
    // Game Mode: never close Steam to write its files (the session starts it again at once).
    assert_eq!(tool::restart_and_write_allowed(true, Some(true)), Err(tool::GAME_MODE_NO_WRITE));
    assert_eq!(tool::restart_and_write_allowed(true, None), Err(tool::GAME_MODE_NO_WRITE));
    assert!(tool::restart_and_write_allowed(false, Some(true)).is_ok(), "Desktop Mode keeps the one restart");
    assert!(tool::restart_and_write_allowed(false, Some(false)).is_ok());
    assert!(tool::restart_and_write_allowed(true, Some(false)).is_ok(), "Steam really closed: the file can be written");
    assert!(tool::GAME_MODE_NO_WRITE.contains("\"Turbine (mods)\""));

    let d = tmp("state");
    let root = d.join("Steam");
    let data = d.join("data");
    fs::create_dir_all(&root).unwrap();
    let ask = |started: Option<std::time::SystemTime>| {
        let a = tool::Ask { steam_root: &root, data: &data, app_id: APP, game_name: "Elden Ring", launch_options_ok: false, steam_started: started, game_mode: true, decky_plugin: false, steam_running: true };
        turbine_engine::json::parse(&tool::state_json(&a)).unwrap()
    };
    let b = |v: &turbine_engine::json::Json, k: &str| v.get(k).and_then(|x| x.as_bool()).unwrap();
    // Not written yet.
    let v = ask(None);
    assert!(!b(&v, "installed") && !b(&v, "listed") && b(&v, "restartFirst"));
    assert_eq!(v.str_at("route"), Some("none"));
    assert_eq!(v.str_at("tile"), Some("later"));
    // Written, Steam not started since: the first step is the restart.
    tool::install(&root, &data).unwrap();
    let made = fs::metadata(tool::tool_dir(&root).join("compatibilitytool.vdf")).unwrap().modified().unwrap();
    let v = ask(Some(made - std::time::Duration::from_secs(5)));
    assert!(b(&v, "installed") && !b(&v, "listed"));
    assert_eq!(v.get("steps").unwrap().as_array().len(), 4);
    // Steam started since: three steps.
    let v = ask(Some(made + std::time::Duration::from_secs(5)));
    assert!(b(&v, "listed") && !b(&v, "restartFirst") && !b(&v, "chosen"));
    assert_eq!(v.get("steps").unwrap().as_array().len(), 3);
    assert_eq!(v.str_at("toolName"), Some("Turbine (mods)"));
    assert_eq!(v.str_at("proton"), Some(""));
    // The Proton on record and the last launch's own words.
    let p = d.join("Proton 11.0");
    write(&p.join("proton"), "x");
    write(&p.join("toolmanifest.vdf"), "x");
    tool::write_proton_choice(&data, APP, &p).unwrap();
    write(&anticheat::helper_log(&data), &format!("2026-11-10 10:00:00 Turbine (mods): Steam started app {APP} (waitforexitandrun) in /g: /g/x.exe\n2026-11-10 10:00:00 Turbine (mods): Proton for this game: {} [why: the Proton Steam last used for this game, kept by Turbine]\n", p.display()));
    let v = ask(Some(made + std::time::Duration::from_secs(5)));
    assert_eq!(v.str_at("protonName"), Some("Proton 11.0"));
    assert_eq!(v.str_at("route"), Some("tool"));
    assert_eq!(v.str_at("lastProton"), Some("Started with Proton 11.0: the Proton Steam last used for this game, kept by Turbine."));
    let _ = fs::remove_dir_all(&d);
}

/// 0.62: Report a bug (both reports use anticheat::launch_report_lines) holds the Steam launch logs, and says first
/// which Proton the tool used and why.
#[test]
fn the_report_lines_hold_the_launch_logs_and_the_proton_pick() {
    let d = tmp("report");
    let data = d.join("data");
    assert!(anticheat::launch_report_lines(&data).is_empty());
    write(&anticheat::helper_log(&data), "2026-11-10 10:00:00 Turbine (mods): Steam started app 1245620 (waitforexitandrun) in /g: /g/x.exe\n2026-11-10 10:00:00 Turbine (mods): Proton for this game: /s/common/Proton 11.0 [why: the newest Proton installed (Turbine has no record of the one Steam used for this game; 2 numbered Proton found)]\n2026-11-10 10:00:20 me3 ended with exit code 0 after 20 s\n");
    write(&anticheat::launch_dir(&data).join("me3-1245620.log"), "me3 line\n");
    write(&anticheat::launch_dir(&data).join("me3-1245620-own.log"), "own line\n");
    write(&anticheat::launch_dir(&data).join("steam-1245620.log"), "proton line\n");
    let lines = anticheat::launch_report_lines(&data);
    assert_eq!(lines[0], "Turbine's Steam tool: Started with Proton 11.0: the newest Proton installed (Turbine has no record of the one Steam used for this game; 2 numbered Proton found).");
    assert_eq!(lines[1], "-- steam-launch.log (last 3 lines) --");
    for want in ["me3 ended with exit code 0 after 20 s", "-- me3-1245620.log (last 1 lines) --", "me3 line", "own line", "-- steam-1245620.log (last 1 lines) --", "proton line"] {
        assert!(lines.iter().any(|l| l.contains(want)), "{want}: {lines:?}");
    }
    // Nothing started: that sentence comes first.
    write(&anticheat::helper_log(&data), "2026-11-10 10:00:00 Turbine (mods): no Proton that can run was found in /s: nothing started. In Steam, start any game with Proton once\n");
    assert!(anticheat::launch_report_lines(&data)[0].starts_with("Turbine's Steam tool: no Proton that can run was found"));
    let _ = fs::remove_dir_all(&d);
}

/// 0.62 break test M5: choosing by itself, the tool never takes a Beta or Experimental while a finished numbered
/// Proton can run, unless that Beta is the one Steam last used for the game.
#[test]
fn a_beta_is_never_picked_over_a_finished_proton() {
    if !has_bash() {
        return;
    }
    let s = Steam::new("beta");
    // A newer Beta and Experimental beside the finished Proton 11.
    let beta = s.root.join("steamapps/common/Proton 12.0 (Beta)");
    proton(&beta, &s.trace, "proton12beta", Some("4183110"));
    proton(&s.root.join("steamapps/common/Proton - Experimental"), &s.trace, "protonexp", Some("4183110"));
    let (code, t) = s.play();
    assert_eq!(code, 0, "{}", s.log());
    assert_eq!(names(&t), ["wrapper", "reaper", "slr4", "proton11", "game"], "{t:?}");
    // The engine's own list agrees: finished ones first, then Betas, Experimental.
    let list = tool::installed_protons(&s.root);
    assert_eq!(list.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["Proton 11.0", "Proton 12.0 (Beta)", "Proton 9.0 (Beta)", "Proton - Experimental"]);
    assert_eq!(list.iter().map(|p| p.kind.word()).collect::<Vec<_>>(), ["stable", "beta", "beta", "experimental"]);
    assert!(list.iter().all(|p| p.usable));
    assert_eq!(tool::best_proton(&s.root).unwrap().name, "Proton 11.0");
    // The Beta is used when Steam itself last used it for this game.
    tool::write_proton_choice(&s.data, APP, &beta).unwrap();
    let (_, t) = s.play();
    assert_eq!(names(&t), ["wrapper", "reaper", "slr4", "proton12beta", "game"], "{t:?}");
    // No record and no finished Proton left: the newest Beta, and the log says it is one.
    fs::remove_file(tool::proton_file(&s.data, APP)).unwrap();
    fs::remove_dir_all(&s.proton11).unwrap();
    let (_, t) = s.play();
    assert_eq!(names(&t), ["wrapper", "reaper", "slr4", "proton12beta", "game"], "{t:?}");
    assert!(s.log().contains("[why: the newest Proton installed is a Beta (no finished Proton could be used; 2 numbered Proton found)]"), "{}", s.log());
    // A Proton whose runtime is missing is listed as not usable and never the best.
    fs::remove_dir_all(&s.slr4).unwrap();
    let list = tool::installed_protons(&s.root);
    assert_eq!(list.iter().find(|p| p.name == "Proton 12.0 (Beta)").map(|p| p.usable), Some(false));
    assert_eq!(tool::best_proton(&s.root).unwrap().name, "Proton 9.0 (Beta)");
    s.done();
}

/// 0.62 break test M5: the record moves with what the game really ran with, also through the tool; the player's
/// own pick is kept; Automatic removes it; default.proton is written.
#[test]
fn the_proton_record_follows_launches_and_the_players_pick() {
    if !has_bash() {
        return;
    }
    let s = Steam::new("record");
    // No record: the tool chooses (Proton 11), and Turbine keeps what the game ran with.
    assert_eq!(tool::proton_record(&s.data, APP), None);
    s.play();
    assert_eq!(tool::last_proton_seen_in(&s.log(), APP), Some((s.proton11.clone(), tool::ProtonHow::Tool)));
    assert_eq!(tool::remember_proton(&s.data, APP), Some(s.proton11.clone()));
    assert_eq!(tool::proton_record(&s.data, APP), Some((s.proton11.clone(), tool::ProtonHow::Tool)));
    s.play();
    assert!(s.log().contains("[why: the Proton this game last ran with, kept by Turbine]"), "{}", s.log());
    // The player picks another one in Turbine: used, said, and never replaced by what a launch shows.
    tool::set_proton_pick(&s.data, APP, Some(&s.proton9)).unwrap();
    let (_, t) = s.play();
    assert_eq!(names(&t), ["wrapper", "reaper", "sniper", "proton9", "game"], "{t:?}");
    assert!(s.log().contains("[why: your pick in Turbine for this game]"));
    assert_eq!(tool::remember_proton(&s.data, APP), Some(s.proton9.clone()));
    assert_eq!(tool::proton_record(&s.data, APP).unwrap().1, tool::ProtonHow::You);
    // A pick that isn't a Proton is refused; Automatic takes the record away.
    assert!(tool::set_proton_pick(&s.data, APP, Some(&s.d.join("nothing"))).is_err());
    tool::set_proton_pick(&s.data, APP, None).unwrap();
    assert_eq!(tool::proton_record(&s.data, APP), None);
    tool::set_proton_pick(&s.data, APP, None).unwrap();
    assert!(tool::set_proton_pick(&s.data, "12a", None).is_err());
    // Steam's own launch afterwards (the game back on a plain Proton with the Launch Options line): its Proton
    // becomes the record, over the tool's earlier one.
    s.play();
    tool::remember_proton(&s.data, APP);
    assert_eq!(tool::proton_record(&s.data, APP).unwrap().1, tool::ProtonHow::Tool);
    let own = format!("2026-11-10 12:00:00 Steam started app {APP} in /g: /s/steam-launch-wrapper -- /s/reaper SteamLaunch AppId={APP} -- {}/_v2-entry-point --verb=waitforexitandrun -- {}/proton waitforexitandrun /g/x.exe\n", s.sniper.display(), s.proton9.display());
    fs::write(anticheat::helper_log(&s.data), format!("{}{own}", s.log())).unwrap();
    assert_eq!(tool::remember_proton(&s.data, APP), Some(s.proton9.clone()));
    assert_eq!(tool::proton_record(&s.data, APP), Some((s.proton9.clone(), tool::ProtonHow::Steam)));

    // default.proton: the newest finished Proton, or Steam's own choice for all games when its settings name one.
    assert_eq!(tool::write_default_proton(&s.root, &s.data), Some(s.proton11.clone()));
    assert_eq!(fs::read_to_string(tool::default_proton_file(&s.data)).unwrap(), format!("{}\nturbine\n", s.proton11.display()));
    let config = |zero: &str, game: &str| format!("\"InstallConfigStore\"\n{{\n\"Software\"\n{{\n\"Valve\"\n{{\n\"Steam\"\n{{\n\"CompatToolMapping\"\n{{\n\"0\"\n{{\n\"name\"\t\"{zero}\"\n}}\n\"814380\"\n{{\n\"name\"\t\"{game}\"\n}}\n}}\n}}\n}}\n}}\n}}\n");
    let ge = tool::tools_dir(&s.root).join("GE-Proton10-4");
    proton(&ge, &s.trace, "ge", Some("4183110"));
    write(&ge.join("compatibilitytool.vdf"), "\"compatibilitytools\"\n{\n\"compat_tools\"\n{\n\"GE-Proton10-4\"\n{\n\"install_path\" \".\"\n\"display_name\" \"GE-Proton10-4\"\n}\n}\n}\n");
    write(&tool::config_file(&s.root), &config("proton_9", "GE-Proton10-4"));
    assert_eq!(tool::write_default_proton(&s.root, &s.data), Some(s.proton9.clone()), "Steam's own choice for all games");
    assert_eq!(fs::read_to_string(tool::default_proton_file(&s.data)).unwrap(), format!("{}\nsteam\n", s.proton9.display()));
    // A game with no record and no launch in the log: its own entry in Steam's settings (another tool's Proton).
    assert_eq!(tool::remember_proton_with(&s.root, &s.data, "814380"), Some(ge.clone()));
    assert_eq!(tool::proton_record(&s.data, "814380"), Some((ge.clone(), tool::ProtonHow::Steam)));
    // A game with neither runs with default.proton, and the log says whose choice that is.
    tool::set_proton_pick(&s.data, APP, None).unwrap();
    let (_, t) = s.play();
    assert_eq!(names(&t), ["wrapper", "reaper", "sniper", "proton9", "game"], "{t:?}");
    assert!(s.log().contains("[why: Steam's own choice for all games, kept by Turbine]"));

    // The Settings row: the Proton in use, why, and the list to pick from.
    let j = turbine_engine::json::parse(&tool::proton_json(&s.root, &s.data, APP)).unwrap();
    assert_eq!((j.str_at("name"), j.str_at("how"), j.str_at("auto")), (Some("Proton 9.0 (Beta)"), Some("auto"), Some("Proton 9.0 (Beta)")));
    let list = j.get("list").unwrap().as_array();
    assert_eq!(list.iter().map(|p| p.str_at("name").unwrap()).collect::<Vec<_>>(), ["Proton 11.0", "Proton 9.0 (Beta)", "GE-Proton10-4"]);
    assert_eq!(list.iter().filter(|p| p.get("current").and_then(|c| c.as_bool()) == Some(true)).count(), 1);
    tool::set_proton_pick(&s.data, APP, Some(&s.proton11)).unwrap();
    let j = turbine_engine::json::parse(&tool::proton_json(&s.root, &s.data, APP)).unwrap();
    assert_eq!((j.str_at("name"), j.str_at("how"), j.str_at("words")), (Some("Proton 11.0"), Some("you"), Some("your pick")));
    s.done();
}
