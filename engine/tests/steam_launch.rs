//! Modded launches go through Steam. The helper keeps Steam's whole command in Steam's own order (the
//! launch wrapper with its options, the reaper with the app id, the runtime, Proton) and swaps only the game's program
//! for me3's Windows build. These tests run the helper behind a stub of Steam's chain: each stub is a real program that
//! writes its arguments to a trace file and passes control on the way Steam's own do.

mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use support::{has_bash, scratch as tmp};
use turbine_engine::anticheat::{self, Coop};

const APP: &str = "1245620";

fn script(p: &Path, body: &str) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    // Written by a child process, never by this one. A file this process holds open for writing is inherited, for
    // a moment, by any child another test's thread starts just then; running the script in that moment fails with
    // "Text file busy" (the build on GitHub hit exactly that). This process never holds it, so nothing inherits it.
    use std::io::Write;
    let mut c = Command::new("sh").arg("-c").arg("cat > \"$1\" && chmod 755 \"$1\"").arg("sh").arg(p).stdin(std::process::Stdio::piped()).spawn().unwrap();
    c.stdin.take().unwrap().write_all(format!("#!/bin/sh\n{body}").as_bytes()).unwrap();
    assert!(c.wait().unwrap().success(), "couldn't write {}", p.display());
}

/// Writes `name|arg|arg...` as one line to the trace.
fn trace_line(name: &str, trace: &Path) -> String {
    format!("{{ printf '%s' '{name}'; for a in \"$@\"; do printf '|%s' \"$a\"; done; printf '\\n'; }} >> '{}'\n", trace.display())
}

/// Steam's chain as stubs, the game's own programs, Turbine's data folder with me3's Windows build. Paths hold
/// spaces as on the device ("Proton 11.0", "ELDEN RING").
struct Chain {
    d: PathBuf,
    data: PathBuf,
    trace: PathBuf,
    wrapper: PathBuf,
    reaper: PathBuf,
    runtime: PathBuf,
    proton: PathBuf,
    starter: PathBuf,
    game_exe: PathBuf,
    me3: PathBuf,
    me3_exe: PathBuf,
    profile: PathBuf,
}

impl Chain {
    fn new(name: &str) -> Chain {
        let d = tmp(name);
        let trace = d.join("trace.txt");
        let steam = d.join("Steam Root");
        let wrapper = steam.join("ubuntu12_32/steam-launch-wrapper");
        let reaper = steam.join("ubuntu12_32/reaper");
        let runtime = steam.join("steamapps/common/SteamLinuxRuntime_4/_v2-entry-point");
        let proton = steam.join("steamapps/common/Proton 11.0/proton");
        let game = steam.join("steamapps/common/ELDEN RING/Game");
        let starter = game.join("start_protected_game.exe");
        let game_exe = game.join("eldenring.exe");
        // The wrapper takes its own options up to "--"; the reaper and the runtime take theirs up to "--".
        let skip = "while [ $# -gt 0 ] && [ \"$1\" != -- ]; do shift; done; shift; exec \"$@\"\n";
        script(&wrapper, &format!("{}{skip}", trace_line("wrapper", &trace)));
        script(&reaper, &format!("{}{skip}", trace_line("reaper", &trace)));
        script(&runtime, &format!("{}{skip}", trace_line("runtime", &trace)));
        // Proton: the verb, then the program it runs (a Windows program here is a stub that runs as it is).
        script(
            &proton,
            &format!(
                "{}[ -n \"$PROTON_LOG_DIR\" ] && printf 'proton-env|PROTON_LOG=%s|PROTON_LOG_DIR=%s\\n' \"$PROTON_LOG\" \"$PROTON_LOG_DIR\" >> '{t}'\nshift; exec \"$@\"\n",
                trace_line("proton", &trace),
                t = trace.display()
            ),
        );
        script(&starter, &trace_line("game", &trace));
        script(&game_exe, &trace_line("eldenring", &trace));
        let data = d.join("data dir");
        let me3 = anticheat::me3_dir(&data);
        let me3_exe = me3.join("bin/win64/me3.exe");
        // me3.exe: exits with the code in "me3.fail" next to the trace, after an error line, when that file is there.
        script(
            &me3_exe,
            &format!(
                "{}if [ -f '{f}' ]; then echo 'ERROR run: me3_launcher: an error occurred while loading me3' >&2; sleep 1; exit \"$(cat '{f}')\"; fi\necho 'me3 started the game'\n",
                trace_line("me3.exe", &trace),
                f = d.join("me3.fail").display()
            ),
        );
        script(&me3.join("bin/me3"), &trace_line("me3", &trace));
        let store = d.join("Steam Root/.turbine-mod-manager/steam-1245620");
        let (profile, _) = anticheat::write_profile_for(&store, "eldenring", Coop::Solo).unwrap();
        Chain { d, data, trace, wrapper, reaper, runtime, proton, starter, game_exe, me3, me3_exe, profile }
    }

    /// Steam's command today: the launch wrapper (with an option) first, then the reaper.
    fn steam_now(&self) -> Vec<String> {
        let mut v = vec![self.wrapper.display().to_string(), "--oom-score-adjust".into(), "900".into(), "--".into()];
        v.extend(self.reaper_part());
        v.extend(self.runtime_part());
        v
    }

    /// Older Steam: the reaper first, then the launch wrapper.
    fn steam_old(&self) -> Vec<String> {
        let mut v = self.reaper_part();
        v.extend([self.wrapper.display().to_string(), "--".into()]);
        v.extend(self.runtime_part());
        v
    }

    fn reaper_part(&self) -> Vec<String> {
        vec![self.reaper.display().to_string(), "SteamLaunch".into(), format!("AppId={APP}"), "--".into()]
    }

    fn runtime_part(&self) -> Vec<String> {
        vec![
            self.runtime.display().to_string(),
            "--verb=waitforexitandrun".into(),
            "--".into(),
            self.proton.display().to_string(),
            "waitforexitandrun".into(),
            self.starter.display().to_string(),
        ]
    }

    fn arm_windows(&self) -> Vec<String> {
        let a = anticheat::helper_args_for(&self.me3, &self.profile, Some(&self.game_exe));
        anticheat::arm(&self.data, APP, Some(&a)).unwrap();
        a
    }

    /// Runs Turbine's Launch Options line the way Steam does: through the shell, with %command% put in (quoted).
    /// Answers the exit code and the trace (one Vec per program, its name first).
    fn play(&self, steam: &[String], env: &[(&str, &str)]) -> (i32, Vec<Vec<String>>) {
        let _ = fs::remove_file(&self.trace);
        let cmd: Vec<String> = steam.iter().map(|a| format!("'{a}'")).collect();
        let line = anticheat::launch_option(&self.data).replace("%command%", &cmd.join(" "));
        let mut c = Command::new("sh");
        c.arg("-c").arg(&line).env("SteamAppId", APP).env_remove("PROTON_LOG").env_remove("PROTON_LOG_DIR");
        for (k, v) in env {
            c.env(k, v);
        }
        let mut code = c.status().unwrap().code().unwrap_or(-1);
        // 126 with nothing run: the script Turbine had just written was still held open for writing by a child of
        // another test's thread ("Text file busy"). Nothing was used up; once more after a moment.
        for _ in 0..3 {
            if code != 126 || fs::read_to_string(&self.trace).is_ok_and(|t| !t.is_empty()) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
            code = c.status().unwrap().code().unwrap_or(-1);
        }
        let t = fs::read_to_string(&self.trace).unwrap_or_default();
        (code, t.lines().map(|l| l.split('|').map(str::to_string).collect()).collect())
    }

    fn log(&self) -> String {
        fs::read_to_string(anticheat::helper_log(&self.data)).unwrap_or_default()
    }
}

fn names(t: &[Vec<String>]) -> Vec<&str> {
    t.iter().map(|l| l[0].as_str()).filter(|n| *n != "proton-env").collect()
}

fn line<'a>(t: &'a [Vec<String>], name: &str) -> &'a [String] {
    t.iter().find(|l| l[0] == name).unwrap_or_else(|| panic!("no {name} in {t:?}"))
}

/// The process chain holds Steam's reaper with the app id, and `program` runs under Steam's Proton.
fn assert_steam_chain(c: &Chain, t: &[Vec<String>], program: &Path) {
    let reaper = line(t, "reaper");
    assert_eq!(&reaper[1..4], ["SteamLaunch", &format!("AppId={APP}"), "--"], "{t:?}");
    let runtime = line(t, "runtime");
    assert_eq!(runtime[1], "--verb=waitforexitandrun", "{t:?}");
    assert_eq!(runtime[3], c.proton.display().to_string(), "Steam's own Proton: {t:?}");
    let proton = line(t, "proton");
    assert_eq!(proton[1], "waitforexitandrun", "{t:?}");
    assert_eq!(proton[2], program.display().to_string(), "{t:?}");
}

#[test]
fn launch_option_text_and_check() {
    let data = Path::new("/home/deck/.var/app/x/data");
    let opt = anticheat::launch_option(data);
    let h = "/home/deck/.var/app/x/data/steam-launch/turbine-launch.sh";
    assert_eq!(opt, format!("if [ -x \"{h}\" ]; then \"{h}\" %command%; else %command%; fi"));
    assert!(anticheat::launch_option_ok(&opt, data));
    // A bare assignment in front of `if` is a shell syntax error: Steam would start nothing.
    assert!(!anticheat::launch_option_ok(&format!("PROTON_LOG=1 {opt}"), data));
    assert!(!anticheat::launch_option_ok(&format!("PROTON_LOG=1 DXVK_HUD=1 {opt}"), data));
    // `export NAME=value;` statements in front are fine.
    assert!(anticheat::launch_option_ok(&format!("export PROTON_LOG=1; {opt}"), data));
    assert!(anticheat::launch_option_ok(&format!("export PROTON_LOG=1;{opt}"), data));
    assert!(anticheat::launch_option_ok(&format!("export PROTON_LOG=1 DXVK_HUD=1; export X=2; {opt}"), data));
    assert!(!anticheat::launch_option_ok(&format!("export PROTON_LOG=1 {opt}"), data), "no ; before if");
    assert!(!anticheat::launch_option_ok(&format!("gamemoderun; {opt}"), data));
    assert!(!anticheat::launch_option_ok(&format!("export 1X=2; {opt}"), data));
    assert!(!anticheat::launch_option_ok("", data));
    assert!(!anticheat::launch_option_ok("%command% /home/deck/.var/app/x/data/steam-launch/turbine-launch.sh", data));
    // The doctor's fix for the bare form.
    assert_eq!(anticheat::launch_option_fix(&format!("PROTON_LOG=1 {opt}"), data), Some(format!("export PROTON_LOG=1; {opt}")));
    assert_eq!(anticheat::launch_option_fix(&format!("export PROTON_LOG=1; {opt}"), data), None, "already right");
    assert_eq!(anticheat::launch_option_fix("PROTON_LOG=1 %command%", data), None, "not Turbine's line");
    let spaced = anticheat::launch_option(Path::new("/home/a b/data"));
    assert!(spaced.contains("\"/home/a b/data/steam-launch/turbine-launch.sh\" %command%"), "{spaced}");
}

/// Every accepted form parses in the shell and reaches Steam's chain (the bare form doesn't even parse).
#[test]
fn accepted_lines_run_in_the_shell() {
    if !has_bash() {
        return;
    }
    let c = Chain::new("shellforms");
    anticheat::write_helper(&c.data, None).unwrap();
    let cmd: Vec<String> = c.steam_now().iter().map(|a| format!("'{a}'")).collect();
    let opt = anticheat::launch_option(&c.data);
    for (form, ok) in [
        (opt.clone(), true),
        (format!("export PROTON_LOG=1; {opt}"), true),
        (format!("export PROTON_LOG=1 DXVK_HUD=1; export X=2; {opt}"), true),
        (format!("PROTON_LOG=1 {opt}"), false),
    ] {
        assert_eq!(anticheat::launch_option_ok(&form, &c.data), ok, "{form}");
        let _ = fs::remove_file(&c.trace);
        // The broken form is broken on purpose: its shell error stays out of the test output.
        let err = if ok { std::process::Stdio::inherit() } else { std::process::Stdio::null() };
        let st = Command::new("sh").arg("-c").arg(form.replace("%command%", &cmd.join(" "))).env("SteamAppId", APP).stderr(err).status().unwrap();
        let t = fs::read_to_string(&c.trace).unwrap_or_default();
        assert_eq!(st.success() && t.lines().any(|l| l == "game" || l.starts_with("game|")), ok, "{form}: {t}");
    }
    let _ = fs::remove_dir_all(&c.d);
}

#[test]
fn windows_paths_and_args() {
    assert_eq!(anticheat::windows_path(Path::new("/home/deck/x y/ELDEN RING/Game/eldenring.exe")), "Z:\\home\\deck\\x y\\ELDEN RING\\Game\\eldenring.exe");
    let a = anticheat::helper_args_for(Path::new("/d/me3/0.13.0"), Path::new("/p/turbine.me3"), Some(Path::new("/g/Game/eldenring.exe")));
    assert_eq!(a, ["/d/me3/0.13.0/bin/win64/me3.exe", "launch", "--exe", "Z:\\g\\Game\\eldenring.exe", "-p", "Z:\\p\\turbine.me3"]);
    let l = anticheat::helper_args_for(Path::new("/d/me3/0.13.0"), Path::new("/p/turbine.me3"), None);
    assert_eq!(l, anticheat::helper_args(Path::new("/d/me3/0.13.0"), Path::new("/p/turbine.me3")));
    assert_eq!(l[0], "/d/me3/0.13.0/bin/me3");
}

/// The device's case: Steam's command today, launch wrapper first. Mods on: Steam's whole chain runs in its own
/// order and me3.exe takes the game's place under Steam's own Proton.
#[test]
fn steam_order_now_keeps_the_whole_chain() {
    if !has_bash() {
        return;
    }
    let c = Chain::new("now");
    assert!(anticheat::write_helper(&c.data, None).unwrap());
    assert!(!anticheat::write_helper(&c.data, None).unwrap(), "unchanged: not written again");

    // Mods off: Steam's own command, the game's starter under Proton.
    let (code, t) = c.play(&c.steam_now(), &[]);
    assert_eq!(code, 0);
    assert_eq!(names(&t), ["wrapper", "reaper", "runtime", "proton", "game"], "{t:?}");
    assert_steam_chain(&c, &t, &c.starter);
    assert!(c.log().contains("No mods on in Turbine"), "{}", c.log());

    // Mods on.
    let args = c.arm_windows();
    let (code, t) = c.play(&c.steam_now(), &[]);
    assert_eq!(code, 0, "{}", c.log());
    assert_eq!(names(&t), ["wrapper", "reaper", "runtime", "proton", "me3.exe"], "{t:?}\n{}", c.log());
    assert_eq!(&line(&t, "wrapper")[1..4], ["--oom-score-adjust", "900", "--"], "the wrapper keeps its options");
    assert_steam_chain(&c, &t, &c.me3_exe);
    let proton = line(&t, "proton");
    assert_eq!(&proton[3..], &args[1..], "me3.exe's arguments after Steam's verb");
    let me3 = line(&t, "me3.exe");
    assert_eq!(me3[3], format!("Z:{}", c.game_exe.display().to_string().replace('/', "\\")));
    assert!(me3[3].contains("ELDEN RING"), "{me3:?}");
    assert_eq!(me3[5], format!("Z:{}", c.profile.display().to_string().replace('/', "\\")));
    assert!(!t.iter().any(|l| l[0] == "game"), "the starter with its anti-cheat never runs: {t:?}");
    let log = c.log();
    assert!(log.contains("Kept in front: "), "{log}");
    assert!(log.contains(&format!("{} SteamLaunch AppId={APP} --", c.reaper.display())), "{log}");
    assert!(log.contains("Steam's Proton for this game: "), "{log}");
    assert!(log.contains("me3 ended with exit code 0 after"), "{log}");
    assert!(log.contains("me3: me3 started the game"), "{log}");
    let _ = fs::remove_dir_all(&c.d);
}

/// Older Steam: the reaper first, then the launch wrapper. Same result.
#[test]
fn steam_order_old_keeps_the_whole_chain() {
    if !has_bash() {
        return;
    }
    let c = Chain::new("old");
    anticheat::write_helper(&c.data, None).unwrap();
    c.arm_windows();
    let (code, t) = c.play(&c.steam_old(), &[]);
    assert_eq!(code, 0, "{}", c.log());
    assert_eq!(names(&t), ["reaper", "wrapper", "runtime", "proton", "me3.exe"], "{t:?}");
    assert_steam_chain(&c, &t, &c.me3_exe);
    let _ = fs::remove_dir_all(&c.d);
}

/// No reaper or no AppId in front: the normal game, with the reason in the log.
#[test]
fn without_the_reaper_the_game_starts_normally() {
    if !has_bash() {
        return;
    }
    let c = Chain::new("noreaper");
    anticheat::write_helper(&c.data, None).unwrap();
    c.arm_windows();
    // Only the runtime and Proton (the launch options inside Steam's command, or another launcher).
    let bare: Vec<String> = c.runtime_part();
    let (code, t) = c.play(&bare, &[]);
    assert_eq!(code, 0);
    assert_eq!(names(&t), ["runtime", "proton", "game"], "{t:?}");
    assert!(c.log().contains("Steam's reaper with AppId=1245620 isn't in front of the game (reaper: no, AppId: no"), "{}", c.log());
    // The reaper for another app.
    let mut other = c.steam_now();
    for a in other.iter_mut() {
        if a.starts_with("AppId=") {
            *a = "AppId=99".into();
        }
    }
    let (_, t) = c.play(&other, &[]);
    assert_eq!(names(&t), ["wrapper", "reaper", "runtime", "proton", "game"], "{t:?}");
    assert!(c.log().contains("(reaper: yes, AppId: no"), "{}", c.log());
    let _ = fs::remove_dir_all(&c.d);
}

/// The helper gone (Turbine's data deleted): the line runs Steam's own command.
#[test]
fn missing_helper_runs_steams_command() {
    if !has_bash() {
        return;
    }
    let c = Chain::new("nohelper");
    let (code, t) = c.play(&c.steam_now(), &[]);
    assert_eq!(code, 0);
    assert_eq!(names(&t), ["wrapper", "reaper", "runtime", "proton", "game"], "{t:?}");
    assert_steam_chain(&c, &t, &c.starter);
    let _ = fs::remove_dir_all(&c.d);
}

/// Turbine uninstalled (its data kept): Steam's own command. The check runs without Steam's library path, so that
/// can't make it fail while Turbine is installed.
#[test]
fn turbine_uninstalled_and_steams_library_path() {
    if !has_bash() || Command::new("env").arg("-u").arg("X").arg("true").status().map(|s| !s.success()).unwrap_or(true) {
        return;
    }
    let c = Chain::new("uninstalled");
    let bin = c.d.join("fakebin");
    let installed = c.d.join("installed");
    script(
        &bin.join("flatpak"),
        &format!(
            "[ -n \"$LD_LIBRARY_PATH\" ] && exit 3\n[ -n \"$LD_PRELOAD\" ] && exit 4\n[ \"$1\" = info ] && [ \"$2\" = io.example.Turbine ] && [ -f '{}' ] && exit 0\nexit 1\n",
            installed.display()
        ),
    );
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap_or_default());
    anticheat::write_helper(&c.data, Some("io.example.Turbine")).unwrap();
    c.arm_windows();
    // A library that exists stands in for Steam's overlay (preloading it changes nothing).
    let preload = ["/lib/x86_64-linux-gnu/libc.so.6", "/usr/lib/x86_64-linux-gnu/libc.so.6", "/lib64/libc.so.6", "/usr/lib/libc.so.6"]
        .into_iter()
        .find(|p| Path::new(p).exists())
        .unwrap_or("");
    let env = [("PATH", path.as_str()), ("LD_LIBRARY_PATH", "/x/steam-runtime/lib"), ("LD_PRELOAD", preload)];
    // Installed, with Steam's library path and overlay set: me3.
    fs::write(&installed, "").unwrap();
    let (_, t) = c.play(&c.steam_now(), &env);
    assert_eq!(names(&t), ["wrapper", "reaper", "runtime", "proton", "me3.exe"], "{t:?}\n{}", c.log());
    // Uninstalled: the normal game.
    fs::remove_file(&installed).unwrap();
    let (_, t) = c.play(&c.steam_now(), &env);
    assert_eq!(names(&t), ["wrapper", "reaper", "runtime", "proton", "game"], "{t:?}");
    assert!(c.log().contains("isn't installed any more (flatpak info answered 1)"), "{}", c.log());
    let _ = fs::remove_dir_all(&c.d);
}

/// me3 missing, the game's program missing, or another program in the file: the normal game.
#[test]
fn me3_or_game_missing_runs_the_normal_game() {
    if !has_bash() {
        return;
    }
    let c = Chain::new("missing");
    anticheat::write_helper(&c.data, None).unwrap();
    c.arm_windows();
    fs::rename(&c.game_exe, c.d.join("moved.exe")).unwrap();
    let (_, t) = c.play(&c.steam_now(), &[]);
    assert_eq!(names(&t), ["wrapper", "reaper", "runtime", "proton", "game"], "{t:?}");
    assert!(c.log().contains("Not found: "), "{}", c.log());
    fs::rename(c.d.join("moved.exe"), &c.game_exe).unwrap();

    fs::remove_file(&c.me3_exe).unwrap();
    let (_, t) = c.play(&c.steam_now(), &[]);
    assert_eq!(names(&t), ["wrapper", "reaper", "runtime", "proton", "game"], "{t:?}");
    assert!(c.log().contains("me3 isn't where Turbine put it"), "{}", c.log());

    let other = c.d.join("me3.exe");
    script(&other, &trace_line("other", &c.trace));
    let mut a = anticheat::helper_args_for(&c.me3, &c.profile, Some(&c.game_exe));
    a[0] = other.display().to_string();
    anticheat::arm(&c.data, APP, Some(&a)).unwrap();
    let (_, t) = c.play(&c.steam_now(), &[]);
    assert_eq!(names(&t), ["wrapper", "reaper", "runtime", "proton", "game"], "{t:?}");
    assert!(c.log().contains("doesn't name Turbine's own me3"), "{}", c.log());
    let _ = fs::remove_dir_all(&c.d);
}

/// me3 fails: its exit code, run time and last lines go to the helper's log; the helper ends with the same code.
/// Proton's log goes into the helper's folder when Turbine's setting is on, and Report a problem gets all of it.
#[test]
fn me3_failure_is_logged_and_reported() {
    if !has_bash() {
        return;
    }
    let c = Chain::new("fail");
    anticheat::write_helper(&c.data, None).unwrap();
    c.arm_windows();
    fs::write(c.d.join("me3.fail"), "1").unwrap();
    anticheat::set_proton_log(&c.data, true).unwrap();
    let (code, t) = c.play(&c.steam_now(), &[]);
    assert_eq!(code, 1);
    let env = line(&t, "proton-env");
    assert_eq!(env[1], "PROTON_LOG=1");
    assert_eq!(env[2], format!("PROTON_LOG_DIR={}", anticheat::launch_dir(&c.data).display()));
    let log = c.log();
    assert!(log.contains("Proton log on: "), "{log}");
    let ended = log.lines().find(|l| l.contains("me3 ended with exit code 1 after ")).unwrap_or_else(|| panic!("{log}"));
    let secs: u64 = ended.rsplit("after ").next().unwrap().trim_end_matches(" s").parse().unwrap();
    assert!(secs <= 30, "{ended}");
    assert!(log.contains("me3: ERROR run: me3_launcher: an error occurred while loading me3"), "{log}");
    // Off again: no Proton log.
    anticheat::set_proton_log(&c.data, false).unwrap();
    anticheat::set_proton_log(&c.data, false).unwrap();
    let (_, t) = c.play(&c.steam_now(), &[]);
    assert!(!t.iter().any(|l| l[0] == "proton-env"), "{t:?}");
    // The player's own `export PROTON_LOG=1;` also puts it in the helper's folder.
    let (_, t) = c.play(&c.steam_now(), &[("PROTON_LOG", "1")]);
    assert!(t.iter().any(|l| l[0] == "proton-env"), "{t:?}");
    // Report a problem.
    fs::write(anticheat::launch_dir(&c.data).join(format!("steam-{APP}.log")), "Proton: 11.0\nwine: Unhandled page fault\n").unwrap();
    let r = anticheat::launch_report_lines(&c.data).join("\n");
    for want in [
        "-- steam-launch.log (last ",
        "me3 ended with exit code 1",
        &format!("-- me3-{APP}.log (last "),
        "ERROR run: me3_launcher",
        &format!("-- steam-{APP}.log (last 2 lines) --"),
        "wine: Unhandled page fault",
    ] {
        assert!(r.contains(want), "{want} in {r}");
    }
    let _ = fs::remove_dir_all(&c.d);
}

/// The Linux me3 (an armed file without the game's program, used when that isn't known): everything in front of
/// Steam's runtime stays, reaper included.
#[test]
fn linux_me3_keeps_the_prefix() {
    if !has_bash() {
        return;
    }
    let c = Chain::new("linux");
    anticheat::write_helper(&c.data, None).unwrap();
    let a = anticheat::helper_args(&c.me3, &c.profile);
    anticheat::arm(&c.data, APP, Some(&a)).unwrap();
    for (steam, order) in [(c.steam_now(), ["wrapper", "reaper", "me3"]), (c.steam_old(), ["reaper", "wrapper", "me3"])] {
        let (code, t) = c.play(&steam, &[]);
        assert_eq!(code, 0, "{}", c.log());
        assert_eq!(names(&t), order, "{t:?}\n{}", c.log());
        assert_eq!(&line(&t, "me3")[1..], &a[1..]);
        assert_eq!(&line(&t, "reaper")[1..3], ["SteamLaunch", &format!("AppId={APP}")]);
    }
    let _ = fs::remove_dir_all(&c.d);
}

/// The next-Play-only file is used once; Play without mods once; a leftover Play without mods never outlives a new
/// state; Off clears everything.
#[test]
fn once_files_and_plain_play() {
    if !has_bash() {
        return;
    }
    let c = Chain::new("once");
    anticheat::write_helper(&c.data, None).unwrap();
    let store = c.d.join("Steam Root/.turbine-mod-manager/steam-1245620");
    let solo = anticheat::helper_args_for(&c.me3, &c.profile, Some(&c.game_exe));
    let (coop_profile, _) = {
        let mods = anticheat::profile_mods(&store);
        fs::create_dir_all(mods.join("SeamlessCoop")).unwrap();
        fs::write(mods.join("SeamlessCoop/ersc.dll"), "x").unwrap();
        fs::write(mods.join("SeamlessCoop/ersc_settings.ini"), "[PASSWORD]\ncooppassword = dragon\n").unwrap();
        anticheat::write_profile_for(&store, "eldenring", Coop::Seamless).unwrap()
    };
    let coop = anticheat::helper_args_for(&c.me3, &coop_profile, Some(&c.game_exe));
    anticheat::arm_with(&c.data, APP, Some(&solo), "mods on: 2 mods, solo").unwrap();
    assert!(fs::read_to_string(anticheat::armed_file(&c.data, APP)).unwrap().starts_with("# mods on: 2 mods, solo\n"));
    let profile_of = |t: &[Vec<String>]| line(t, "me3.exe")[5].clone();
    let (_, t) = c.play(&c.steam_now(), &[]);
    assert!(profile_of(&t).ends_with("turbine.me3"), "{t:?}");
    anticheat::arm_once(&c.data, APP, &coop, "Seamless Co-op, this Play only").unwrap();
    let (_, t) = c.play(&c.steam_now(), &[]);
    assert!(profile_of(&t).ends_with("turbine-coop.me3"), "{t:?}");
    assert!(c.log().contains("This Play only: # Seamless Co-op, this Play only"), "{}", c.log());
    let (_, t) = c.play(&c.steam_now(), &[]);
    assert!(profile_of(&t).ends_with("turbine.me3"), "back to the normal state: {t:?}");
    // Play without mods once, then mods again.
    anticheat::arm_plain_once(&c.data, APP).unwrap();
    let (_, t) = c.play(&c.steam_now(), &[]);
    assert_eq!(names(&t), ["wrapper", "reaper", "runtime", "proton", "game"], "{t:?}");
    assert!(!anticheat::plain_file(&c.data, APP).exists());
    assert!(c.log().contains("Play without mods (once)"), "{}", c.log());
    let (_, t) = c.play(&c.steam_now(), &[]);
    assert!(names(&t).contains(&"me3.exe"), "{t:?}");
    // A Play without mods waiting, then mods on again: the new state wins.
    anticheat::arm_plain_once(&c.data, APP).unwrap();
    anticheat::arm(&c.data, APP, Some(&solo)).unwrap();
    assert!(anticheat::plain_file(&c.data, APP).exists(), "the state written again keeps a waiting Play without mods");
    anticheat::clear_plain(&c.data, APP).unwrap();
    assert!(!anticheat::plain_file(&c.data, APP).exists());
    // Off clears a waiting next-Play-only file too.
    anticheat::arm_once(&c.data, APP, &coop, "co-op").unwrap();
    anticheat::arm(&c.data, APP, None).unwrap();
    anticheat::arm(&c.data, APP, None).unwrap();
    assert!(!anticheat::once_file(&c.data, APP).exists());
    assert!(!anticheat::armed_file(&c.data, APP).exists());
    let (_, t) = c.play(&c.steam_now(), &[]);
    assert_eq!(names(&t), ["wrapper", "reaper", "runtime", "proton", "game"], "{t:?}");
    assert!(anticheat::arm(&c.data, "12a", None).is_err());
    assert!(anticheat::arm(&c.data, "1", Some(&["a\nb".to_string()])).is_err());
    assert_eq!(anticheat::helper_log_since(&c.data, 0).len(), c.log().lines().count().min(40));
    // The original Launch Options are kept once, never Turbine's own line.
    anticheat::remember_original(&c.data, APP, "PROTON_LOG=1 %command%").unwrap();
    anticheat::remember_original(&c.data, APP, &anticheat::launch_option(&c.data)).unwrap();
    assert_eq!(anticheat::original_options(&c.data, APP), "PROTON_LOG=1 %command%");
    let _ = fs::remove_dir_all(&c.d);
}

/// Every way of turning mods off clears what Steam's Play starts.
#[test]
fn every_off_path_clears_the_helper() {
    let helper_state = |a, b, c, d, e| anticheat::helper_state_undone(a, b, c, d, e, false);
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
    if !has_bash() || Command::new("touch").arg("--version").output().is_err() {
        return;
    }
    let c = Chain::new("oldplain");
    anticheat::write_helper(&c.data, None).unwrap();
    c.arm_windows();
    anticheat::arm_plain_once(&c.data, APP).unwrap();
    Command::new("touch").arg("-d").arg("20 minutes ago").arg(anticheat::plain_file(&c.data, APP)).status().unwrap();
    let (_, t) = c.play(&c.steam_now(), &[]);
    assert!(names(&t).contains(&"me3.exe"), "{t:?}");
    assert!(!anticheat::plain_file(&c.data, APP).exists());
    let _ = fs::remove_dir_all(&c.d);
}

#[test]
fn solo_and_coop_profiles() {
    let d = tmp("profiles");
    let mods = anticheat::profile_mods(&d);
    fs::create_dir_all(mods.join("SeamlessCoop")).unwrap();
    fs::write(mods.join("SeamlessCoop/ersc.dll"), "x").unwrap();
    fs::write(mods.join("SeamlessCoop/ersc_settings.ini"), "[PASSWORD]\ncooppassword = dragon\n").unwrap();
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

/// The older line (the helper alone) is outdated, so Play asks again for the line that
/// falls back to Steam's own command.
#[test]
fn the_older_line_is_outdated() {
    let data = Path::new("/home/deck/.var/app/x/data");
    let h = "/home/deck/.var/app/x/data/steam-launch/turbine-launch.sh";
    assert!(!anticheat::launch_option_ok(&format!("{h} %command%"), data), "the ae53371 line");
    assert!(!anticheat::launch_option_ok(&format!("\"{h}\" %command%"), data));
    assert!(anticheat::launch_option_ok(&anticheat::launch_option(data), data));
    // Steam may keep it with other spacing.
    let spaced = anticheat::launch_option(data).replace("; ", ";  ");
    assert!(anticheat::launch_option_ok(&spaced, data));
    // The older line still counts as Turbine's (Undo shows it), and is never kept as the player's original.
    assert!(anticheat::launch_option_uses_helper(&format!("{h} %command%"), data));
}

/// Undo sticks. Nothing arms again (Steam's Play stays normal) until the line is set
/// again or the player says to use it again; the first original is never replaced.
#[test]
fn undo_sticks_until_the_line_is_set_again() {
    let d = tmp("undo");
    let data = d.join("data");
    let line = anticheat::launch_option(&data);
    anticheat::arm(&data, "1245620", Some(&["/x/me3".to_string(), "-p".into(), "solo".into()])).unwrap();
    anticheat::mark_undone(&data, "1245620", &line).unwrap();
    assert!(!anticheat::armed_file(&data, "1245620").exists());
    // Opening the game, a toggle: still undone while the Launch Options are as they were at Undo.
    assert!(anticheat::still_undone(&data, "1245620", Some(&line), false));
    assert!(anticheat::helper_state_undone(2, true, false, false, true, true).is_err());
    assert!(anticheat::still_undone(&data, "1245620", Some(""), false), "the old text pasted back: still undone");
    assert!(anticheat::still_undone(&data, "1245620", None, false));
    // The line set again (changed since Undo, and right): no longer undone.
    assert!(!anticheat::still_undone(&data, "1245620", Some(&format!("{line} ")), false));
    assert!(!anticheat::undone_file(&data, "1245620").exists());
    // Or the player says so in Turbine.
    anticheat::mark_undone(&data, "1245620", &line).unwrap();
    assert!(!anticheat::still_undone(&data, "1245620", Some(&line), true));
    // The newest options without Turbine's line are the ones Undo puts back ("A", Undo, the player sets
    // "B", Turbine's line again, Undo: "B" comes back), and Turbine's own line never counts as the player's.
    anticheat::remember_original(&data, "1245620", "A %command%").unwrap();
    anticheat::remember_original(&data, "1245620", "B %command%").unwrap();
    anticheat::remember_original(&data, "1245620", &anticheat::launch_option(&data)).unwrap();
    assert_eq!(anticheat::original_options(&data, "1245620"), "B %command%");
    anticheat::remember_original(&data, "1245620", "").unwrap();
    assert_eq!(anticheat::original_options(&data, "1245620"), "");
    let _ = fs::remove_dir_all(&d);
}

/// The Launch Options kept for Undo are written whole or not at all: a write that fails leaves the ones kept
/// before, and no half-written file takes their place.
#[test]
fn a_failed_write_keeps_the_launch_options_kept_for_undo() {
    let d = tmp("orig-atomic");
    let data = d.join("data");
    anticheat::remember_original(&data, "1245620", "A %command%").unwrap();
    assert_eq!(anticheat::original_options(&data, "1245620"), "A %command%");
    // Something is in the way of the new copy: the write fails before the kept file is touched.
    let f = anticheat::original_file(&data, "1245620");
    let blocker = f.with_file_name(".1245620.original.turbine-new");
    std::fs::create_dir_all(&blocker).unwrap();
    assert!(anticheat::remember_original(&data, "1245620", "B %command%").is_err());
    assert_eq!(anticheat::original_options(&data, "1245620"), "A %command%");
    std::fs::remove_dir(&blocker).unwrap();
    anticheat::remember_original(&data, "1245620", "B %command%").unwrap();
    assert_eq!(anticheat::original_options(&data, "1245620"), "B %command%");
    assert!(!blocker.exists(), "nothing left beside it");
}
