//! 0.6: the one Steam restart that changes Steam's own settings. The host script runs against stub Steam processes:
//! it closes Steam, makes the change only once Steam is gone, starts Steam again (or lets Game Mode's session do it)
//! and brings Turbine back. Also the job file, applying it, and the busy check.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};
use turbine_engine::steam_restart::{self, Change, Job};
use turbine_engine::{shortcuts, steam, steam_setup};

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("turbine-restart-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn script(p: &Path, body: &str) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, format!("#!/bin/sh\n{body}")).unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(p, fs::Permissions::from_mode(0o755)).unwrap();
}

fn tools_there() -> bool {
    ["bash", "pgrep", "pkill", "setsid", "sleep"].iter().all(|t| Command::new("sh").arg("-c").arg(format!("command -v {t}")).output().is_ok_and(|o| o.status.success()))
}

/// A stub Steam: a process with its own name that stands for Steam, and a `steam` command that closes it
/// (-shutdown), starts it (no arguments) and records links.
struct FakeSteam {
    d: PathBuf,
    proc_name: String,
    steam_cmd: PathBuf,
    trace: PathBuf,
}

impl FakeSteam {
    fn new(d: &Path, tag: &str) -> FakeSteam {
        let proc_name = format!("fs{}{tag}", std::process::id() % 100_000);
        let bin = d.join("bin");
        fs::create_dir_all(&bin).unwrap();
        let sleep = String::from_utf8(Command::new("sh").arg("-c").arg("command -v sleep").output().unwrap().stdout).unwrap().trim().to_string();
        let proc_bin = bin.join(&proc_name);
        fs::copy(&sleep, &proc_bin).unwrap();
        let trace = d.join("trace.txt");
        let steam_cmd = bin.join("steam");
        script(
            &steam_cmd,
            &format!(
                "printf 'steam %s\\n' \"$*\" >> '{t}'\ncase \"$1\" in\n  -shutdown) sleep 0.3; pkill -x '{p}' ;;\n  '') setsid '{b}' 300 < /dev/null > /dev/null 2>&1 & ;;\nesac\n",
                t = trace.display(),
                p = proc_name,
                b = proc_bin.display()
            ),
        );
        FakeSteam { d: d.to_path_buf(), proc_name, steam_cmd, trace }
    }

    fn start(&self) {
        Command::new(&self.steam_cmd).status().unwrap();
        self.wait(true);
    }

    fn up(&self) -> bool {
        Command::new("pgrep").arg("-x").arg(&self.proc_name).output().is_ok_and(|o| o.status.success())
    }

    fn wait(&self, up: bool) {
        let t = Instant::now();
        while self.up() != up && t.elapsed() < Duration::from_secs(10) {
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn trace(&self) -> String {
        fs::read_to_string(&self.trace).unwrap_or_default()
    }

    /// Runs the script the way Turbine does (it returns at once) and waits for its result.
    fn run(&self, data: &Path, mode: &str, back: &str, apply: &Path) -> (String, String) {
        let s = steam_restart::write_script(data).unwrap();
        let mut args = steam_restart::script_args(data, mode == "gamemode", back, &[apply.display().to_string()]);
        args.insert(0, s.display().to_string());
        let t = Instant::now();
        let st = Command::new("bash")
            .args(&args)
            .env("TURBINE_STEAM", &self.steam_cmd)
            .env("TURBINE_STEAM_PROC", &self.proc_name)
            .env("TURBINE_PROC", "no-such-turbine")
            .env("TURBINE_POLL", "0.1")
            .env("TURBINE_WAIT", "40")
            .status()
            .unwrap();
        assert!(st.success());
        assert!(t.elapsed() < Duration::from_secs(2), "the first run returns at once (it starts itself apart)");
        let t = Instant::now();
        loop {
            if let Some((state, words)) = steam_restart::read_result(data) {
                if state != "running" {
                    // The link back to Turbine comes right after the result.
                    std::thread::sleep(Duration::from_millis(400));
                    return (state, words);
                }
            }
            assert!(t.elapsed() < Duration::from_secs(30), "no result: {}", fs::read_to_string(steam_restart::log_file(data)).unwrap_or_default());
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn stop(&self) {
        let _ = Command::new("pkill").arg("-x").arg(&self.proc_name).status();
        let _ = fs::remove_dir_all(&self.d);
    }
}

/// The apply stub: records whether Steam was running when it was called.
fn apply_stub(d: &Path, fake: &FakeSteam, fail: bool) -> PathBuf {
    let p = d.join("bin/apply");
    script(
        &p,
        &format!(
            "if pgrep -x '{n}' > /dev/null; then echo 'apply while Steam runs' >> '{t}'; else echo 'apply with Steam closed' >> '{t}'; fi\n{}",
            if fail { "echo \"Steam's settings changed meanwhile\" >&2; exit 1" } else { "echo 'Launch Options for 1245620 set'" },
            n = fake.proc_name,
            t = fake.trace.display()
        ),
    );
    p
}

#[test]
fn desktop_restart_closes_changes_and_starts_steam() {
    if !tools_there() {
        return;
    }
    let d = tmp("desktop");
    let fake = FakeSteam::new(&d, "d");
    fake.start();
    let data = d.join("data");
    let apply = apply_stub(&d, &fake, false);
    let (state, words) = fake.run(&data, "desktop", "steam://rungameid/123", &apply);
    assert_eq!(state, "done", "{words}");
    let t = fake.trace();
    let at = |s: &str| t.find(s).unwrap_or_else(|| panic!("{s} in {t}"));
    assert!(at("steam -shutdown") < at("apply with Steam closed"));
    assert!(!t.contains("apply while Steam runs"), "{t}");
    assert!(at("apply with Steam closed") < t.rfind("steam \n").unwrap(), "Steam started again after the change: {t}");
    assert!(t.contains("steam steam://rungameid/123"), "Turbine brought back: {t}");
    assert!(fake.up(), "Steam runs again");
    let log = fs::read_to_string(steam_restart::log_file(&data)).unwrap();
    for want in ["Asking Steam to close", "Steam is closed", "Changed: Launch Options for 1245620 set", "Starting Steam", "Steam is running again", "Result: done"] {
        assert!(log.contains(want), "{want} in {log}");
    }
    fake.stop();
}

#[test]
fn game_mode_lets_the_session_start_steam() {
    if !tools_there() {
        return;
    }
    let d = tmp("gamemode");
    let fake = FakeSteam::new(&d, "g");
    fake.start();
    // Game Mode's session: starts Steam again once it has gone.
    let session = d.join("bin/session");
    script(
        &session,
        &format!("while pgrep -x '{n}' > /dev/null; do sleep 0.05; done\nsleep 0.3\n'{s}'\n", n = fake.proc_name, s = fake.steam_cmd.display()),
    );
    let mut sess = Command::new(&session).spawn().unwrap();
    let data = d.join("data");
    let apply = apply_stub(&d, &fake, false);
    let (state, _) = fake.run(&data, "gamemode", "", &apply);
    let _ = sess.wait();
    assert_eq!(state, "done");
    let log = fs::read_to_string(steam_restart::log_file(&data)).unwrap();
    assert!(log.contains("Waiting for Game Mode to start Steam again") && !log.contains("Starting Steam"), "{log}");
    assert_eq!(fake.trace().matches("steam \n").count(), 2, "started once by the test, once by the session: {}", fake.trace());
    assert!(fake.up());
    fake.stop();
}

#[test]
fn a_failed_change_still_starts_steam_and_says_so() {
    if !tools_there() {
        return;
    }
    let d = tmp("fail");
    let fake = FakeSteam::new(&d, "f");
    fake.start();
    let data = d.join("data");
    let apply = apply_stub(&d, &fake, true);
    let (state, words) = fake.run(&data, "desktop", "", &apply);
    assert_eq!(state, "failed");
    assert!(words.contains("Steam's settings changed meanwhile"), "{words}");
    assert!(fake.up(), "Steam is started again anyway");
    fake.stop();
}

#[test]
fn steam_that_wont_close_changes_nothing() {
    if !tools_there() {
        return;
    }
    let d = tmp("stuck");
    let fake = FakeSteam::new(&d, "s");
    fake.start();
    // This Steam ignores -shutdown.
    script(&fake.steam_cmd, &format!("printf 'steam %s\\n' \"$*\" >> '{}'\n", fake.trace.display()));
    let data = d.join("data");
    let apply = apply_stub(&d, &fake, false);
    let (state, words) = fake.run(&data, "desktop", "", &apply);
    assert_eq!(state, "failed");
    assert!(words.contains("didn't close"), "{words}");
    assert!(!fake.trace().contains("apply"), "{}", fake.trace());
    fake.stop();
}

const CONFIG: &str = "\"UserLocalConfigStore\"\n{\n\t\"Software\"\n\t{\n\t\t\"Valve\"\n\t\t{\n\t\t\t\"Steam\"\n\t\t\t{\n\t\t\t\t\"apps\"\n\t\t\t\t{\n\t\t\t\t\t\"1245620\"\n\t\t\t\t\t{\n\t\t\t\t\t\t\"LastPlayed\"\t\t\"1\"\n\t\t\t\t\t}\n\t\t\t\t}\n\t\t\t}\n\t\t}\n\t}\n}\n";

#[test]
fn the_job_round_trips_and_applies() {
    let d = tmp("job");
    let data = d.join("data");
    let cfg = d.join("userdata/42/config");
    fs::create_dir_all(&cfg).unwrap();
    let local = cfg.join("localconfig.vdf");
    fs::write(&local, CONFIG).unwrap();
    let sc = cfg.join("shortcuts.vdf");
    let art = d.join("art");
    fs::create_dir_all(&art).unwrap();
    fs::write(art.join("icon.png"), "i").unwrap();
    let tile = shortcuts::coop_tile("ELDEN RING", "steam:1245620", "io.github.ravenor907.TurbineModManager");
    let marker = shortcuts::coop_marker("steam:1245620");
    let record = data.join("steam-art.json");
    let line = "if [ -x \"/d/turbine-launch.sh\" ]; then \"/d/turbine-launch.sh\" %command%; else %command%; fi";
    let job = Job {
        changes: vec![
            Change::Options { file: local.clone(), app_id: "1245620".into(), value: line.into() },
            Change::Tile { file: sc.clone(), tile: tile.clone(), marker: marker.clone(), art: art.clone(), record: record.clone() },
        ],
        resume: "play steam:1245620 solo".into(),
        what: "ELDEN RING".into(),
        at: 1_762_700_000,
    };
    job.save(&data).unwrap();
    assert_eq!(Job::load(&data).unwrap(), job);
    let lines = steam_restart::apply_job(&data, &job).unwrap();
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert_eq!(steam::launch_options_in(&fs::read_to_string(&local).unwrap(), "1245620").unwrap(), line);
    assert_eq!(shortcuts::parse(&fs::read(&sc).unwrap()).unwrap()[0].name, "ELDEN RING Co-op");
    assert_eq!(steam_setup::pending(&data).len(), 2, "both noted for the check after Steam starts again");
    // Undo the same way: the old (empty) value back, the tile removed.
    let undo = Job {
        changes: vec![Change::Options { file: local.clone(), app_id: "1245620".into(), value: String::new() }, Change::NoTile { file: sc.clone(), marker, record }],
        resume: String::new(),
        what: String::new(),
        at: 1,
    };
    steam_restart::apply_job(&data, &undo).unwrap();
    assert_eq!(steam::launch_options_in(&fs::read_to_string(&local).unwrap(), "1245620").unwrap(), "");
    assert!(shortcuts::parse(&fs::read(&sc).unwrap()).unwrap().is_empty());
    fs::write(steam_restart::result_file(&data), "done \n").unwrap();
    assert_eq!(steam_restart::read_result(&data), Some(("done".into(), String::new())));
    steam_restart::finish(&data);
    assert!(Job::load(&data).is_none() && steam_restart::read_result(&data).is_none());
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn busy_when_a_game_runs_or_steam_downloads() {
    let d = tmp("busy");
    let reg = |running: &str, apps: &str| format!("\"Registry\"\n{{\n\"HKCU\"\n{{\n\"Software\"\n{{\n\"Valve\"\n{{\n\"Steam\"\n{{\n\"RunningAppID\"\t\"{running}\"\n\"apps\"\n{{\n{apps}\n}}\n}}\n}}\n}}\n}}\n}}\n");
    fs::create_dir_all(d.join(".steam")).unwrap();
    fs::write(d.join(".steam/registry.vdf"), reg("0", "")).unwrap();
    assert_eq!(steam_restart::busy(&d, &[], None), None);
    fs::write(d.join(".steam/registry.vdf"), reg("1245620", "")).unwrap();
    assert!(steam_restart::busy(&d, &[], None).unwrap().contains("is running. Close it first"));
    // Turbine's own entry running is fine.
    fs::write(d.join(".steam/registry.vdf"), reg("3871208806", "")).unwrap();
    assert_eq!(steam_restart::busy(&d, &[], Some(3_871_208_806)), None);
    fs::write(d.join(".steam/registry.vdf"), reg("0", "\"489830\"\n{\n\"Updating\"\t\"1\"\n}")).unwrap();
    assert!(steam_restart::busy(&d, &[], None).unwrap().contains("downloading"));
    fs::write(d.join(".steam/registry.vdf"), reg("0", "")).unwrap();
    let lib = d.join(".local/share/Steam/steamapps");
    fs::create_dir_all(&lib).unwrap();
    fs::write(lib.join("appmanifest_489830.acf"), "\"AppState\"\n{\n\"appid\"\t\"489830\"\n\"name\"\t\"Skyrim\"\n\"StateFlags\"\t\"1026\"\n}\n").unwrap();
    assert_eq!(steam_restart::busy(&d, &[], None).unwrap(), "Steam is downloading Skyrim. Let it finish (or pause it), then try again.");
    // Only queued (update required) is not a download under way.
    fs::write(lib.join("appmanifest_489830.acf"), "\"AppState\"\n{\n\"appid\"\t\"489830\"\n\"name\"\t\"Skyrim\"\n\"StateFlags\"\t\"6\"\n}\n").unwrap();
    assert_eq!(steam_restart::busy(&d, &[], None), None);
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn apply_command_and_args() {
    assert_eq!(
        steam_restart::apply_command(Some("io.github.ravenor907.TurbineModManager"), Path::new("/x")),
        ["flatpak", "run", "--command=turbine-mod-manager", "io.github.ravenor907.TurbineModManager", "--apply-steam-setup"]
    );
    assert_eq!(steam_restart::apply_command(None, Path::new("/usr/bin/turbine")), ["/usr/bin/turbine", "--apply-steam-setup"]);
    let a = steam_restart::script_args(Path::new("/d"), true, "steam://rungameid/1", &["x".into()]);
    assert_eq!(a, ["/d", "gamemode", "steam://rungameid/1", "--", "x"]);
}
