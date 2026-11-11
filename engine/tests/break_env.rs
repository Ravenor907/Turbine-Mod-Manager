//! Break tests: the messy situations real SteamOS handhelds produce, thrown at game detection and deploys.
//!
//! Tests that rely on file permissions say so when they run as root, because root ignores permissions and such a
//! test would prove nothing.

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::{symlink, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use turbine_engine::deploy::{apply, prev_path, recover, reset_from_backups, Paths, Record};
use turbine_engine::{json, scan_games, Store};

// ------------------------------------------------------------------ helpers

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A scratch folder removed again at the end, even if a test made parts of it read-only or immutable.
struct Tmp(PathBuf);

impl Tmp {
    fn new(tag: &str) -> Tmp {
        Tmp::under(&std::env::temp_dir(), tag)
    }

    fn under(base: &Path, tag: &str) -> Tmp {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let p = base.join(format!("turbine-break-{tag}-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        Tmp(p)
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::process::Command::new("chattr").arg("-R").arg("-i").arg(&self.0).output();
        unlock(&self.0);
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn unlock(p: &Path) {
    if let Ok(m) = fs::symlink_metadata(p) {
        if m.is_dir() {
            let _ = fs::set_permissions(p, fs::Permissions::from_mode(0o755));
            if let Ok(rd) = fs::read_dir(p) {
                for e in rd.flatten() {
                    unlock(&e.path());
                }
            }
        } else if m.is_file() {
            let _ = fs::set_permissions(p, fs::Permissions::from_mode(0o644));
        }
    }
}

fn is_root() -> bool {
    fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| s.lines().find(|l| l.starts_with("Uid:")).map(|l| l.split_whitespace().nth(1) == Some("0")))
        .unwrap_or(false)
}

fn write(p: &Path, text: &str) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

/// Every file, folder and link under `root`, without following links (relative path -> what it is).
fn snapshot(root: &Path) -> BTreeMap<String, String> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
        let Ok(rd) = fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            let rel = p.strip_prefix(root).unwrap().to_string_lossy().into_owned();
            let m = fs::symlink_metadata(&p).unwrap();
            if m.file_type().is_symlink() {
                out.insert(rel, format!("<link {}>", fs::read_link(&p).unwrap().display()));
            } else if m.is_dir() {
                out.insert(rel, "<dir>".into());
                walk(root, &p, out);
            } else {
                out.insert(rel, String::from_utf8_lossy(&fs::read(&p).unwrap()).into_owned());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn same_dev(a: &Path, b: &Path) -> bool {
    fs::metadata(a).unwrap().dev() == fs::metadata(b).unwrap().dev()
}

// ------------------------------------------------------------------ detection helpers

fn vdf_quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn manifest(app_id: &str, name: &str, dir: Option<&str>) -> String {
    let dir = dir.map(|d| format!("\t\"installdir\"\t\t{}\n", vdf_quote(d))).unwrap_or_default();
    format!(
        "\"AppState\"\n{{\n\t\"appid\"\t\t\"{app_id}\"\n\t\"name\"\t\t{}\n\t\"StateFlags\"\t\t\"4\"\n{dir}\t\"buildid\"\t\t\"1\"\n}}\n",
        vdf_quote(name)
    )
}

/// A Steam folder under `home` with one game per (app id, name, folder) and the given libraryfolders.vdf text.
fn steam_home(home: &Path, games: &[(&str, &str, &str)]) -> PathBuf {
    let sapps = home.join(".local/share/Steam/steamapps");
    for (id, name, dir) in games {
        write(&sapps.join(format!("appmanifest_{id}.acf")), &manifest(id, name, Some(dir)));
        write(&sapps.join("common").join(dir).join("game.exe"), "");
    }
    fs::create_dir_all(&sapps).unwrap();
    sapps
}

fn library_vdf(paths: &[String]) -> String {
    let mut s = String::from("\"libraryfolders\"\n{\n");
    for (i, p) in paths.iter().enumerate() {
        s.push_str(&format!("\t\"{i}\"\n\t{{\n\t\t\"path\"\t\t{}\n\t}}\n", vdf_quote(p)));
    }
    s.push_str("}\n");
    s
}

fn names(home: &Path) -> Vec<String> {
    scan_games(home).games.into_iter().map(|g| g.name).collect()
}

// ------------------------------------------------------------------ detection

#[test]
fn library_list_with_missing_file_self_twice_and_symlinked_roots() {
    let t = Tmp::new("libs");
    let home = &t.0;
    let steam = home.join(".local/share/Steam");
    let sapps = steam_home(home, &[("10", "Alpha", "Alpha")]);
    // ~/.steam/steam and ~/.steam/root point at ~/.local/share/Steam, as on SteamOS.
    fs::create_dir_all(home.join(".steam")).unwrap();
    symlink(&steam, home.join(".steam/steam")).unwrap();
    symlink(&steam, home.join(".steam/root")).unwrap();
    // An SD card library, also reachable through a symlink.
    let sd = home.join("run/media/deck/SD Card");
    write(&sd.join("steamapps/appmanifest_20.acf"), &manifest("20", "Beta", Some("Beta")));
    write(&sd.join("steamapps/common/Beta/beta.x86_64"), "");
    symlink(&sd, home.join("sdlink")).unwrap();
    // A file where a library should be, and a library folder whose steamapps is a file.
    write(&home.join("not-a-folder"), "x");
    write(&home.join("weird/steamapps"), "x");
    let list = [
        home.join(".steam/steam").display().to_string(),
        steam.display().to_string(),
        steam.display().to_string(),
        "/nowhere/missing card".to_string(),
        home.join("not-a-folder").display().to_string(),
        home.join("weird").display().to_string(),
        sd.display().to_string(),
        home.join("sdlink").display().to_string(),
        format!("{}/", sd.display()),
        String::new(),
    ];
    write(&sapps.join("libraryfolders.vdf"), &library_vdf(&list));
    assert_eq!(names(home), ["Alpha", "Beta"]);

    // The old format: "LibraryFolders" { "TimeNextStatsReport" "..." "ContentStatsID" "..." "1" "/path" }.
    write(
        &sapps.join("libraryfolders.vdf"),
        &format!(
            "\"LibraryFolders\"\n{{\n\t\"TimeNextStatsReport\"\t\t\"1700000000\"\n\t\"ContentStatsID\"\t\t\"-123\"\n\t\"1\"\t\t{}\n\t\"2\"\t\t{}\n}}\n",
            vdf_quote(&sd.display().to_string()),
            vdf_quote(&home.join("sdlink").display().to_string())
        ),
    );
    assert_eq!(names(home), ["Alpha", "Beta"]);

    // A libraryfolders.vdf that lists its own library inside a broken file: the internal games are still found.
    write(&sapps.join("libraryfolders.vdf"), "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\" \"");
    let scan = scan_games(home);
    assert_eq!(scan.games.len(), 1);
    assert!(!scan.problems.is_empty(), "a broken libraryfolders.vdf should be reported");
}

#[test]
fn two_thousand_manifests() {
    let t = Tmp::new("many");
    let home = &t.0;
    let sapps = home.join(".local/share/Steam/steamapps");
    for i in 0..2000 {
        let id = 100_000 + i;
        write(&sapps.join(format!("appmanifest_{id}.acf")), &manifest(&id.to_string(), &format!("Game {i:04}"), Some(&format!("G{i}"))));
        fs::create_dir_all(sapps.join(format!("common/G{i}"))).unwrap();
    }
    let start = Instant::now();
    let scan = scan_games(home);
    let took = start.elapsed();
    assert_eq!(scan.games.len(), 2000);
    assert!(scan.problems.is_empty());
    assert!(took.as_secs() < 5, "scanning 2,000 manifests took {took:?}");
    eprintln!("2,000 manifests scanned in {took:?}");
}

#[test]
fn unicode_emoji_quotes_and_backslashes_in_names() {
    let t = Tmp::new("names");
    let home = &t.0;
    let tricky = [
        ("1", "\u{014c}kami HD \u{5927}\u{795e} \u{1f43a}", "\u{5927}\u{795e} \u{1f43a}"),
        ("2", "The \"Quoted\" Game", "Quoted \"Dir\""),
        ("3", "Back\\slash\\Game \\\"", "Back\\slash"),
        ("4", "Tab\tand {braces} // not a comment", "Braces {x}"),
    ];
    steam_home(home, &tricky);
    let scan = scan_games(home);
    let mut got: Vec<(String, String)> =
        scan.games.iter().map(|g| (g.name.clone(), g.install_dir.file_name().unwrap().to_string_lossy().into_owned())).collect();
    got.sort();
    let mut want: Vec<(String, String)> = tricky.iter().map(|(_, n, d)| (n.to_string(), d.to_string())).collect();
    want.sort();
    assert_eq!(got, want);
    // The JSON handed to the interface must read back to the same names.
    let v = json::parse(&scan.games_json()).expect("games_json must be valid JSON");
    let mut back: Vec<String> = v.as_array().iter().map(|g| g.str_at("name").unwrap().to_string()).collect();
    back.sort();
    let mut want_names: Vec<String> = tricky.iter().map(|(_, n, _)| n.to_string()).collect();
    want_names.sort();
    assert_eq!(back, want_names);
}

#[test]
fn manifests_missing_fields_or_broken_are_skipped() {
    let t = Tmp::new("missing");
    let home = &t.0;
    let sapps = steam_home(home, &[("10", "Good", "Good")]);
    write(&sapps.join("appmanifest_11.acf"), &manifest("11", "No Dir", None));
    write(&sapps.join("appmanifest_12.acf"), "\"AppState\"\n{\n\t\"appid\"\t\"12\"\n\t\"installdir\"\t\"Good\"\n}\n");
    write(&sapps.join("appmanifest_13.acf"), "\"AppState\"\n{\n\t\"appid\"\t\"13\"\n\t\"name\"\t\"Cut");
    write(&sapps.join("appmanifest_14.acf"), "");
    write(&sapps.join("appmanifest_15.acf"), "\"AppState\" \"not an object\"");
    fs::write(sapps.join("appmanifest_16.acf"), [0xff, 0xfe, 0x00, 0x22, 0x80]).unwrap();
    let scan = scan_games(home);
    assert_eq!(scan.games.iter().map(|g| g.name.as_str()).collect::<Vec<_>>(), ["Good"]);
    assert!(scan.problems.iter().any(|p| p.contains("appmanifest_13")), "a cut-off manifest should be reported");
}

/// installdir comes straight from a file on disk. It must name a folder inside steamapps/common and nothing else.
#[test]
fn installdir_cannot_escape_steamapps_common() {
    let t = Tmp::new("escape");
    let home = &t.0;
    let sapps = steam_home(home, &[("10", "Good", "Good")]);
    fs::create_dir_all(home.join("outside")).unwrap();
    write(&sapps.join("appmanifest_20.acf"), &manifest("20", "Dot Dot", Some("../../../../../outside")));
    write(&sapps.join("appmanifest_21.acf"), &manifest("21", "Root", Some("/")));
    write(&sapps.join("appmanifest_22.acf"), &manifest("22", "Empty", Some("")));
    write(&sapps.join("appmanifest_23.acf"), &manifest("23", "Up One", Some("..")));
    write(&sapps.join("appmanifest_24.acf"), &manifest("24", "Inner Dot Dot", Some("Good/../../..")));
    let common = fs::canonicalize(sapps.join("common")).unwrap();
    let bad: Vec<String> = scan_games(home)
        .games
        .iter()
        .filter(|g| {
            let real = fs::canonicalize(&g.install_dir).unwrap();
            !real.starts_with(&common) || real == common
        })
        .map(|g| format!("{} -> {}", g.name, g.install_dir.display()))
        .collect();
    assert!(bad.is_empty(), "games whose folder is outside steamapps/common: {bad:?}");
}

#[test]
fn game_folder_that_is_a_symlink_is_found() {
    let t = Tmp::new("gamelink");
    let home = &t.0;
    let sapps = steam_home(home, &[]);
    write(&home.join("elsewhere/Real Game/game.exe"), "");
    write(&sapps.join("appmanifest_30.acf"), &manifest("30", "Linked", Some("Linked")));
    fs::create_dir_all(sapps.join("common")).unwrap();
    symlink(home.join("elsewhere/Real Game"), sapps.join("common/Linked")).unwrap();
    let scan = scan_games(home);
    assert_eq!(scan.games.len(), 1);
    assert_eq!(scan.games[0].runtime, turbine_engine::Runtime::Windows);
}

#[test]
fn unreadable_files_do_not_stop_the_scan() {
    let t = Tmp::new("unreadable");
    let home = &t.0;
    let sapps = steam_home(home, &[("10", "Good", "Good"), ("11", "Locked", "Locked")]);
    fs::set_permissions(sapps.join("appmanifest_11.acf"), fs::Permissions::from_mode(0o000)).unwrap();
    fs::set_permissions(sapps.join("common/Locked"), fs::Permissions::from_mode(0o000)).unwrap();
    write(&sapps.join("libraryfolders.vdf"), &library_vdf(&[]));
    fs::set_permissions(sapps.join("libraryfolders.vdf"), fs::Permissions::from_mode(0o000)).unwrap();
    let got = names(home);
    assert!(got.contains(&"Good".to_string()));
    if is_root() {
        eprintln!("NOTE: running as root, so chmod 000 files stay readable here; this only proves the scan doesn't stop");
    } else {
        assert_eq!(got, ["Good"]);
    }
}

/// A library list or manifest that can't be read hides games (often a whole SD card) with no word why.
/// Reading a folder in place of the file gives the same read error as chmod 000, and works even as root.
#[test]
fn unreadable_library_list_is_reported() {
    let t = Tmp::new("unreadable2");
    let home = &t.0;
    let sapps = steam_home(home, &[("10", "Good", "Good")]);
    fs::create_dir_all(sapps.join("libraryfolders.vdf")).unwrap();
    fs::create_dir_all(sapps.join("appmanifest_11.acf")).unwrap();
    let scan = scan_games(home);
    assert_eq!(scan.games.len(), 1);
    assert!(scan.problems.iter().any(|p| p.contains("libraryfolders")), "problems: {:?}", scan.problems);
    assert!(scan.problems.iter().any(|p| p.contains("appmanifest_11")), "problems: {:?}", scan.problems);
}

#[test]
fn heroic_files_with_unexpected_types() {
    let t = Tmp::new("heroic-types");
    let home = &t.0;
    let flat = home.join(".var/app/com.heroicgameslauncher.hgl/config/heroic");
    let native = home.join(".config/heroic");
    let g1 = home.join("Games/Epic One");
    let g2 = home.join("Games/Side One");
    fs::create_dir_all(&g1).unwrap();
    fs::create_dir_all(&g2).unwrap();
    write(
        &flat.join("legendaryConfig/legendary/installed.json"),
        &format!(
            r#"{{"a": 5, "b": null, "c": [1, 2], "d": {{"title": 7, "install_path": ["x"]}},
               "e": {{"title": null, "install_path": {}, "is_dlc": "no", "version": 3, "platform": 9}},
               "f": {{"title": "Dlc", "install_path": {}, "is_dlc": true}}}}"#,
            json::quote(&g1.to_string_lossy()),
            json::quote(&g1.to_string_lossy())
        ),
    );
    write(&flat.join("gog_store/installed.json"), r#"{"installed": {"oops": 1}}"#);
    write(&flat.join("gog_store/library.json"), r#"{"games": "nope"}"#);
    write(&flat.join("store_cache/gog_library.json"), r#"[{"app_name": 1, "title": 2}]"#);
    write(
        &flat.join("sideload_apps/library.json"),
        &format!(
            r#"{{"games": [null, 5, "x", {{"app_name": null}}, {{"app_name": "s1", "title": ["t"], "folder_name": {}, "install": 5, "is_installed": "yes"}}]}}"#,
            json::quote(&g2.to_string_lossy())
        ),
    );
    // The native Heroic's files are whole-file oddities.
    write(&native.join("legendaryConfig/legendary/installed.json"), "null");
    write(&native.join("gog_store/installed.json"), r#"{"installed": [null, 5, {"appName": 1453375253, "install_path": 7}, {"appName": "", "install_path": "/"}]}"#);
    write(&native.join("sideload_apps/library.json"), "42");
    let scan = scan_games(home);
    let mut got: Vec<(Store, String, String)> = scan.games.iter().map(|g| (g.store, g.store_id.clone(), g.name.clone())).collect();
    got.sort_by(|a, b| a.1.cmp(&b.1));
    assert_eq!(
        got,
        [(Store::Epic, "e".to_string(), "Epic One".to_string()), (Store::Sideload, "s1".to_string(), "Side One".to_string())]
    );
}

/// A game "installed" at / or at the home folder would make Turbine deploy mod files into it.
#[test]
fn heroic_install_path_at_root_or_home_is_rejected() {
    let t = Tmp::new("heroic-root");
    let home = &t.0;
    let root = home.join(".config/heroic");
    write(
        &root.join("legendaryConfig/legendary/installed.json"),
        &format!(
            r#"{{"rootgame": {{"title": "At Root", "install_path": "/"}},
               "homegame": {{"title": "At Home", "install_path": {}}},
               "homeslash": {{"title": "At Home Slash", "install_path": {}}}}}"#,
            json::quote(&home.to_string_lossy()),
            json::quote(&format!("{}/", home.display()))
        ),
    );
    write(&root.join("sideload_apps/library.json"), r#"{"games": [{"app_name": "x", "title": "Root Sideload", "folder_name": "/"}]}"#);
    let bad: Vec<String> = scan_games(home).games.iter().map(|g| format!("{} -> {}", g.name, g.install_dir.display())).collect();
    assert!(bad.is_empty(), "accepted as game folders: {bad:?}");
}

// ------------------------------------------------------------------ deploy helpers

struct World {
    _dirs: Vec<Tmp>,
    paths: Paths,
    staged: PathBuf,
    outside: PathBuf,
}

/// A game folder, staging folder and Turbine's own folder, each on the given base (so they can sit on different drives).
fn world_on(tag: &str, game_base: &Path, stage_base: &Path, book_base: &Path) -> World {
    let g = Tmp::under(game_base, &format!("{tag}-game"));
    let s = Tmp::under(stage_base, &format!("{tag}-stage"));
    let b = Tmp::under(book_base, &format!("{tag}-book"));
    let o = Tmp::under(game_base, &format!("{tag}-outside"));
    let game = g.0.join("Game");
    write(&game.join("Data/Skyrim.esm"), "vanilla esm");
    write(&game.join("Data/Textures/sky.dds"), "vanilla sky");
    write(&game.join("readme.txt"), "vanilla readme");
    let staged = s.0.clone();
    write(&staged.join("A/Data/Skyrim.esm"), "A esm");
    write(&staged.join("A/Data/ModA.esp"), "A plugin");
    write(&staged.join("A/Data/New/Deep/a.txt"), "A deep");
    write(&o.0.join("victim.txt"), "outside victim");
    write(&o.0.join("secret.txt"), "outside secret");
    let paths = Paths { game, backups: b.0.join("backups"), record: b.0.join("record.json"), plan: b.0.join("plan.json") };
    let outside = o.0.clone();
    World { _dirs: vec![g, s, b, o], paths, staged, outside }
}

fn world(tag: &str) -> World {
    let tmp = std::env::temp_dir();
    world_on(tag, &tmp, &tmp, &tmp)
}

fn mod_files(w: &World, m: &str) -> Vec<(String, PathBuf)> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) {
        let mut entries: Vec<_> = fs::read_dir(dir).unwrap().flatten().map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                walk(base, &p, out);
            } else {
                out.push((p.strip_prefix(base).unwrap().to_string_lossy().into_owned(), p));
            }
        }
    }
    let base = w.staged.join(m);
    let mut out = Vec::new();
    walk(&base, &base, &mut out);
    out
}

fn one(w: &World, rel: &str, text: &str) -> (String, PathBuf) {
    let src = w.staged.join("single").join(rel.replace('/', "_"));
    write(&src, text);
    (rel.to_string(), src)
}

// ------------------------------------------------------------------ deploy

/// Staging on one drive, the game on another: hard links fail and copies are used. Backups also cross drives.
#[test]
fn cross_device_staging_and_backups() {
    let shm = Path::new("/dev/shm");
    let tmp = std::env::temp_dir();
    if !shm.is_dir() || same_dev(shm, &tmp) {
        eprintln!("NOTE: /dev/shm and {} are the same drive here; cross-device test not meaningful", tmp.display());
        return;
    }
    for (game_base, other) in [(tmp.as_path(), shm), (shm, tmp.as_path())] {
        let w = world_on("xdev", game_base, other, other);
        let before = snapshot(&w.paths.game);
        let wanted = mod_files(&w, "A");
        let r = apply(&w.paths, &wanted, None).unwrap();
        assert_eq!(r.added, 3);
        assert_eq!(fs::read_to_string(w.paths.game.join("Data/Skyrim.esm")).unwrap(), "A esm");
        assert_eq!(fs::read_to_string(w.paths.game.join("Data/New/Deep/a.txt")).unwrap(), "A deep");
        // ModA.esp would be a link; the replaced Skyrim.esm and the settings-like a.txt are copies anyway.
        assert_eq!(r.copied_not_linked, 1, "{r:?}");
        let rec = Record::load(&w.paths.record).unwrap();
        assert!(rec.files.iter().all(|p| p.copy), "every file must be a copy across drives: {rec:?}");
        let r = apply(&w.paths, &wanted, None).unwrap();
        assert_eq!((r.added, r.removed, r.kept), (0, 0, 3));
        apply(&w.paths, &[], None).unwrap();
        assert_eq!(snapshot(&w.paths.game), before);
        assert!(snapshot(&w.paths.backups).values().all(|v| v == "<dir>"), "a game file was left in backups");
    }
}

/// A read-only folder inside the game. As root, permissions don't apply, so the folder is made immutable instead
/// (chattr +i stops even root); if that's not possible either, the test says so and proves nothing.
#[test]
fn read_only_game_subfolder_fails_without_losing_anything() {
    let w = world("ro");
    let data = w.paths.game.join("Data");
    let locked = if is_root() {
        std::process::Command::new("chattr").arg("+i").arg(&data).status().map(|s| s.success()).unwrap_or(false)
    } else {
        fs::set_permissions(&data, fs::Permissions::from_mode(0o555)).is_ok()
    };
    if !locked {
        eprintln!("NOTE: running as root and chattr +i is unavailable; the read-only test proves nothing here");
        return;
    }
    let before = snapshot(&w.paths.game);
    let wanted = mod_files(&w, "A");
    let r = apply(&w.paths, &wanted, None);
    assert!(r.is_err(), "writing into a read-only folder should fail: {r:?}");
    assert_eq!(snapshot(&w.paths.game), before);
    // Turning the mod off again must work and leave the game as it was.
    apply(&w.paths, &[], None).unwrap();
    assert_eq!(snapshot(&w.paths.game), before);
    assert!(!w.paths.plan.exists());
    let _ = std::process::Command::new("chattr").arg("-i").arg(&data).status();
}

/// A mod file named like one of the game's folders. Moving a whole game folder away is far more than "backing up a
/// file": the game loses every file in it while the mod is on, and a backups folder on another drive can't take it.
#[test]
fn mod_file_where_game_has_a_folder_leaves_the_folder() {
    let w = world("filedir");
    let wanted = vec![one(&w, "Data/Textures", "a file, not a folder")];
    let _ = apply(&w.paths, &wanted, None);
    assert_eq!(
        fs::read_to_string(w.paths.game.join("Data/Textures/sky.dds")).ok().as_deref(),
        Some("vanilla sky"),
        "the game's Data/Textures folder was moved away"
    );
}

#[test]
fn mod_file_where_game_has_a_folder_is_restored_when_off() {
    let w = world("filedir2");
    let before = snapshot(&w.paths.game);
    let wanted = vec![one(&w, "Data/Textures", "a file, not a folder")];
    let _ = apply(&w.paths, &wanted, None);
    let _ = recover(&w.paths);
    apply(&w.paths, &[], None).unwrap();
    assert_eq!(snapshot(&w.paths.game), before);
}

#[test]
fn mod_folder_where_game_has_a_file() {
    let w = world("dirfile");
    let before = snapshot(&w.paths.game);
    let mut wanted = mod_files(&w, "A");
    wanted.push(one(&w, "Data/Skyrim.esm/inner.txt", "inside a file?"));
    wanted.push(one(&w, "readme.txt/x/y.txt", "deeper"));
    let r = apply(&w.paths, &wanted, None);
    assert!(r.is_err(), "a folder can't be made where a game file is: {r:?}");
    assert_eq!(snapshot(&w.paths.game).get("readme.txt").map(String::as_str), Some("vanilla readme"));
    apply(&w.paths, &[], None).unwrap();
    assert_eq!(snapshot(&w.paths.game), before);
}

/// A player moved part of the game to the SD card and left a symlink. Turbine must not write outside the game folder.
#[test]
fn symlinked_folder_inside_game_pointing_outside() {
    let w = world("symdir");
    symlink(&w.outside, w.paths.game.join("Data/Linked")).unwrap();
    let outside_before = snapshot(&w.outside);
    let wanted = vec![one(&w, "Data/Linked/evil.dll", "mod dll"), one(&w, "Data/linked/VICTIM.txt", "mod victim")];
    let _ = apply(&w.paths, &wanted, None);
    let _ = recover(&w.paths);
    assert_eq!(snapshot(&w.outside), outside_before, "files outside the game folder were changed");
}

#[test]
fn symlinked_file_inside_game_is_replaced_not_followed() {
    let w = world("symfile");
    symlink(w.outside.join("victim.txt"), w.paths.game.join("Data/Link.txt")).unwrap();
    let before = snapshot(&w.paths.game);
    let outside_before = snapshot(&w.outside);
    apply(&w.paths, &[one(&w, "Data/Link.txt", "mod text")], None).unwrap();
    assert_eq!(snapshot(&w.outside), outside_before);
    assert_eq!(fs::read_to_string(w.paths.game.join("Data/Link.txt")).unwrap(), "mod text");
    apply(&w.paths, &[], None).unwrap();
    assert_eq!(snapshot(&w.paths.game), before);
    assert_eq!(snapshot(&w.outside), outside_before);
}

/// The copy path writes "<name>.turbine-tmp" first; a long file name that fits on disk no longer fits with that ending.
#[test]
fn long_file_name_replacing_a_game_file() {
    let w = world("long");
    let name = format!("Data/{}.pak", "n".repeat(246));
    write(&w.paths.game.join(&name), "vanilla long");
    let wanted = vec![one(&w, &name, "mod long")];
    let r = apply(&w.paths, &wanted, None);
    assert!(r.is_ok(), "{r:?}");
    assert_eq!(fs::read_to_string(w.paths.game.join(&name)).unwrap(), "mod long");
}

#[test]
fn very_deep_path_fails_cleanly() {
    let w = world("deep");
    let before = snapshot(&w.paths.game);
    let deep = format!("Data/{}/x.txt", vec!["d".repeat(200); 25].join("/"));
    let src = w.staged.join("deep.txt");
    write(&src, "deep");
    let r = apply(&w.paths, &[(deep, src)], None);
    if r.is_ok() {
        apply(&w.paths, &[], None).unwrap();
    } else {
        let _ = recover(&w.paths);
        apply(&w.paths, &[], None).unwrap();
    }
    assert_eq!(snapshot(&w.paths.game), before);
}

/// The staged file is gone when its turn comes (Recently removed was emptied, staging drive pulled).
/// The error is fine; the game's own file must not be left in backups when apply returns.
#[test]
fn staged_file_gone_leaves_game_file_in_place() {
    let w = world("gone");
    let wanted = mod_files(&w, "A");
    fs::remove_file(w.staged.join("A/Data/Skyrim.esm")).unwrap();
    let r = apply(&w.paths, &wanted, None);
    assert!(r.is_err());
    assert_eq!(
        fs::read_to_string(w.paths.game.join("Data/Skyrim.esm")).ok().as_deref(),
        Some("vanilla esm"),
        "the game's Skyrim.esm is missing from the game folder after the failed apply"
    );
}

#[test]
fn staged_file_gone_then_recover_puts_everything_back() {
    let w = world("gone2");
    let before = snapshot(&w.paths.game);
    let wanted = mod_files(&w, "A");
    fs::remove_file(w.staged.join("A/Data/Skyrim.esm")).unwrap();
    assert!(apply(&w.paths, &wanted, None).is_err());
    let r = recover(&w.paths).unwrap().unwrap();
    assert!(r.recovered);
    apply(&w.paths, &[], None).unwrap();
    assert_eq!(snapshot(&w.paths.game), before);
}

/// Turbine stops halfway, then the mod is removed (its staged files move or are deleted) before the next start.
/// Recovery must still bring back the game's own files.
#[test]
fn stop_then_staged_source_gone_strands_game_file() {
    let w = world("strand");
    let before = snapshot(&w.paths.game);
    let wanted = mod_files(&w, "A");
    // Ops are sorted: Data/ModA.esp, Data/New/Deep/a.txt, Data/Skyrim.esm. Stop after the two new files are placed.
    assert!(apply(&w.paths, &wanted, Some(2)).is_err());
    fs::remove_dir_all(w.staged.join("A")).unwrap();
    // And a game file replaced, then a stop.
    let w2 = world("strand2");
    let before2 = snapshot(&w2.paths.game);
    let wanted2 = vec![one(&w2, "Data/Skyrim.esm", "mod esm"), one(&w2, "Data/zzz.esp", "mod zzz")];
    assert!(apply(&w2.paths, &wanted2, Some(1)).is_err());
    fs::remove_dir_all(w2.staged.join("single")).unwrap();
    for (w, before) in [(&w, &before), (&w2, &before2)] {
        recover(&w.paths).unwrap();
        apply(&w.paths, &[], None).unwrap();
        assert_eq!(&snapshot(&w.paths.game), before, "game folder not back to the original after recovery and turning off");
    }
}

/// Stops in the middle of one step, not just between steps.
#[test]
fn stop_inside_a_step_is_recovered() {
    // (1) the game file was moved to backups, the mod file not placed yet.
    // (2) the copy was written to its temporary name but not renamed into place.
    // (3) the copy was put in place but not recorded.
    for stage in 1..=3 {
        let w = world(&format!("midstep{stage}"));
        let before = snapshot(&w.paths.game);
        let wanted = vec![one(&w, "Data/Skyrim.esm", "mod esm"), one(&w, "Data/zzz.esp", "mod zzz")];
        assert!(apply(&w.paths, &wanted, Some(0)).is_err());
        let plan = json::parse(&fs::read_to_string(&w.paths.plan).unwrap()).unwrap();
        let op = &plan.get("ops").unwrap().as_array()[0];
        let backup = PathBuf::from(op.str_at("backup").expect("replacing Skyrim.esm needs a backup"));
        let target = w.paths.game.join("Data/Skyrim.esm");
        fs::create_dir_all(backup.parent().unwrap()).unwrap();
        fs::rename(&target, &backup).unwrap();
        let src = w.staged.join("single/Data_Skyrim.esm");
        if stage == 2 {
            fs::copy(&src, target.with_extension("turbine-tmp")).unwrap();
        }
        if stage == 3 {
            fs::copy(&src, &target).unwrap();
            let f = fs::File::options().write(true).open(&target).unwrap();
            f.set_modified(fs::metadata(&src).unwrap().modified().unwrap()).unwrap();
        }
        recover(&w.paths).unwrap();
        // A copy already in place is kept; anything less is undone.
        if stage < 3 {
            assert_eq!(snapshot(&w.paths.game), before, "stage {stage}");
        }
        assert!(!target.with_extension("turbine-tmp").exists(), "stage {stage}");
        apply(&w.paths, &wanted, None).unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "mod esm", "stage {stage}");
        apply(&w.paths, &[], None).unwrap();
        assert_eq!(snapshot(&w.paths.game), before, "stage {stage}");
    }
}

/// The SD card holding the game is pulled, the player turns a mod off, then puts the card back. The game's path must
/// not be recreated on internal storage (on SteamOS /run/media is a RAM disk), and the game must come back whole.
#[test]
fn sd_card_pulled_between_deploys() {
    let w = world("sdpull");
    let before = snapshot(&w.paths.game);
    apply(&w.paths, &mod_files(&w, "A"), None).unwrap();
    let away = w.paths.game.with_file_name("Game-card-out");
    fs::rename(&w.paths.game, &away).unwrap();
    let _ = apply(&w.paths, &[], None);
    let recreated = w.paths.game.exists();
    let _ = fs::remove_dir_all(&w.paths.game);
    fs::rename(&away, &w.paths.game).unwrap();
    assert!(!recreated, "the game folder was recreated while the card was out");
    let _ = recover(&w.paths);
    apply(&w.paths, &[], None).unwrap();
    assert_eq!(snapshot(&w.paths.game), before, "after the card came back, the game isn't back to the original");
}

/// record.json cut off or zeroed (power loss on a card, a bad sector). Turbine must not guess, but it must not be
/// stuck forever either: there's then no way to turn mods off or restore the game.
#[test]
fn damaged_record_never_changes_the_game() {
    for damage in ["half", "empty", "garbage"] {
        let w = world(&format!("rec-{damage}"));
        apply(&w.paths, &mod_files(&w, "A"), None).unwrap();
        let text = fs::read(&w.paths.record).unwrap();
        let bad: Vec<u8> = match damage {
            "half" => text[..text.len() / 2].to_vec(),
            "empty" => Vec::new(),
            _ => vec![0, 0, 0xff, 0xfe],
        };
        fs::write(&w.paths.record, &bad).unwrap();
        let modded = snapshot(&w.paths.game);
        let r = apply(&w.paths, &[], None);
        assert!(r.is_err(), "{damage}: a damaged record must be refused, not treated as empty: {r:?}");
        assert_eq!(snapshot(&w.paths.game), modded, "{damage}");
    }
}

#[test]
fn damaged_record_has_a_way_back() {
    let w = world("rec-way");
    let before = snapshot(&w.paths.game);
    apply(&w.paths, &mod_files(&w, "A"), None).unwrap();
    let text = fs::read(&w.paths.record).unwrap();
    fs::write(&w.paths.record, &text[..text.len() / 2]).unwrap();
    let _ = apply(&w.paths, &[], None);
    let r = apply(&w.paths, &[], None);
    assert!(r.is_ok(), "still stuck: {r:?}");
    assert_eq!(snapshot(&w.paths.game), before);
}

#[test]
fn record_with_bogus_numbers_never_deletes() {
    let w = world("rec-bogus");
    write(&w.paths.game.join("Data/player.ini"), "player made");
    write(
        &w.paths.record,
        r#"{"files":[{"rel":"Data/player.ini","source":"/x","copy":false,"ino":"abc","size":null,"mtime":[1]},
                     {"rel":"readme.txt","source":"/x","copy":true,"ino":"1","size":"-5","mtime":"9e99"},
                     {"source":"/no-rel"}, 7, null],"dirs":["Data", "", "/", "../.."]}"#,
    );
    let before = snapshot(&w.paths.game);
    apply(&w.paths, &[], None).unwrap();
    assert_eq!(snapshot(&w.paths.game), before);
}

fn plan_text(ops: &[String]) -> String {
    format!("{{\"ops\":[\n{}\n],\n\"new_dirs\":[]}}\n", ops.join(",\n"))
}

/// A plan.json that was tampered with (or damaged in just the wrong way) must not delete or move anything outside
/// the game folder.
#[test]
fn tampered_plan_cannot_touch_outside() {
    let w = world("plan-tamper");
    let victim = w.outside.join("victim.txt");
    let m = fs::metadata(&victim).unwrap();
    let rel_victim = format!("../../{}/victim.txt", w.outside.file_name().unwrap().to_string_lossy());
    let remove = format!(
        "{{\"op\":\"remove\",\"placed\":{{\"rel\":{},\"source\":\"/x\",\"copy\":false,\"ino\":\"{}\",\"size\":\"0\",\"mtime\":\"0\",\"backup\":null}}}}",
        json::quote(&rel_victim),
        m.ino()
    );
    let steal = format!(
        "{{\"op\":\"add\",\"rel\":\"Data/stolen.txt\",\"source\":\"/nonexistent\",\"copy\":true,\"backup\":{}}}",
        json::quote(&w.outside.join("secret.txt").to_string_lossy())
    );
    let abs = format!(
        "{{\"op\":\"remove\",\"placed\":{{\"rel\":{},\"source\":\"/x\",\"copy\":false,\"ino\":\"{}\",\"size\":\"0\",\"mtime\":\"0\",\"backup\":null}}}}",
        json::quote(&victim.to_string_lossy()),
        m.ino()
    );
    write(&w.paths.plan, &plan_text(&[remove, steal, abs]));
    // Recovery needs the remove ops to be "still recorded" or "ours"; "ours" is a matching inode.
    let outside_before = snapshot(&w.outside);
    let _ = recover(&w.paths);
    assert_eq!(snapshot(&w.outside), outside_before, "files outside the game folder were deleted or moved");
}

#[test]
fn tampered_record_cannot_touch_outside() {
    let w = world("rec-tamper");
    let victim = w.outside.join("victim.txt");
    let m = fs::metadata(&victim).unwrap();
    write(
        &w.paths.record,
        &format!(
            "{{\"files\":[{{\"rel\":{},\"source\":\"/x\",\"copy\":false,\"ino\":\"{}\",\"size\":\"0\",\"mtime\":\"0\",\"backup\":{}}}],\"dirs\":[]}}",
            json::quote(&victim.to_string_lossy()),
            m.ino(),
            json::quote(&w.outside.join("secret.txt").to_string_lossy())
        ),
    );
    let outside_before = snapshot(&w.outside);
    let _ = apply(&w.paths, &[], None);
    assert_eq!(snapshot(&w.outside), outside_before, "files outside the game folder were deleted or moved");
}

/// Paths handed to apply() with ".." or a leading "/" are refused.
#[test]
fn wanted_paths_cannot_leave_the_game_folder() {
    let w = world("wanted-escape");
    let name = w.outside.file_name().unwrap().to_string_lossy().into_owned();
    let wanted = vec![
        one(&w, &format!("../../{name}/dotdot.txt"), "escaped"),
        one(&w, &format!("{}/abs.txt", w.outside.display()), "escaped"),
    ];
    let outside_before = snapshot(&w.outside);
    let r = apply(&w.paths, &wanted, None);
    assert!(r.is_err(), "paths outside the game folder must be refused");
    assert_eq!(snapshot(&w.outside), outside_before, "files were written outside the game folder");
}

/// Backup names are chosen when planning, so two in one plan must not get the same name. Game files "x" and "x.1"
/// with an older backup of "x" still there: each needs its own backup.
#[test]
fn backup_names_never_collide() {
    let w = world("collide");
    write(&w.paths.game.join("Data/a.bsa"), "game a");
    write(&w.paths.game.join("Data/a.bsa.1"), "game a1");
    let before = snapshot(&w.paths.game);
    let m1 = w.staged.join("m1/a.bsa");
    write(&m1, "mod1 a");
    apply(&w.paths, &[("Data/a.bsa".into(), m1)], None).unwrap();
    let m2a = w.staged.join("m2/a.bsa");
    let m2b = w.staged.join("m2/a.bsa.1");
    write(&m2a, "mod2 a");
    write(&m2b, "mod2 a1");
    apply(&w.paths, &[("Data/a.bsa".into(), m2a), ("Data/a.bsa.1".into(), m2b)], None).unwrap();
    apply(&w.paths, &[], None).unwrap();
    assert_eq!(snapshot(&w.paths.game), before);
}

/// Two Turbine processes (the app and a second copy, or the auto-apply timer racing Play in a future version)
/// applying to the same game. One of them may be turned away with "already applying" (the lock); nothing else may go
/// wrong. This runs the race a few times and reports what broke.
#[test]
fn two_turbines_at_once() {
    let w = world("race");
    for i in 0..300 {
        write(&w.paths.game.join(format!("Data/g{i:03}.pak")), &format!("game {i}"));
        write(&w.staged.join(format!("R/Data/g{i:03}.pak")), &format!("mod {i}"));
        write(&w.staged.join(format!("R/Data/new/n{i:03}.txt")), &format!("new {i}"));
    }
    let before = snapshot(&w.paths.game);
    let wanted = mod_files(&w, "R");
    let mut problems = Vec::new();
    for round in 0..6 {
        let errors: Vec<String> = std::thread::scope(|s| {
            let a = s.spawn(|| apply(&w.paths, &wanted, None).err().map(|e| e.to_string()));
            // Even rounds: both apply the same mods. Odd rounds: one turns the mods on while the other turns them off.
            let b = s.spawn(|| {
                let other: &[(String, PathBuf)] = if round % 2 == 0 { &wanted } else { &[] };
                apply(&w.paths, other, None).err().map(|e| e.to_string())
            });
            [a.join().unwrap(), b.join().unwrap()].into_iter().flatten().filter(|e| !e.contains("already applying changes")).collect()
        });
        if !errors.is_empty() {
            problems.push(format!("round {round} apply errors: {errors:?}"));
        }
        let _ = recover(&w.paths);
        let _ = apply(&w.paths, &wanted, None);
        let wrong = (0..300).filter(|i| fs::read_to_string(w.paths.game.join(format!("Data/g{i:03}.pak"))).ok() != Some(format!("mod {i}"))).count();
        if wrong > 0 {
            problems.push(format!("round {round}: {wrong} mod files not in place after settling"));
        }
        let _ = apply(&w.paths, &[], None);
        let after = snapshot(&w.paths.game);
        if after != before {
            let diff = before.iter().filter(|(k, v)| after.get(*k) != Some(*v)).count()
                + after.keys().filter(|k| !before.contains_key(*k)).count();
            let examples: Vec<String> = before
                .iter()
                .filter(|(k, v)| after.get(*k) != Some(*v))
                .map(|(k, v)| format!("{k}: was {v:?}, now {:?}", after.get(k)))
                .chain(after.keys().filter(|k| !before.contains_key(*k)).map(|k| format!("{k}: left behind")))
                .take(4)
                .collect();
            let originals_in_backups = snapshot(&w.paths.backups).values().filter(|v| v.starts_with("game ")).count();
            problems.push(format!(
                "round {round}: {diff} paths differ from the original game after turning off \
                 ({originals_in_backups} original game files still findable in backups), e.g. {examples:?}"
            ));
            break;
        }
    }
    assert!(problems.is_empty(), "{problems:#?}");
}

// ------------------------------------------------------------------ fixes: lock, reset, settings files, setups

#[test]
fn a_held_lock_refuses_with_a_plain_message() {
    let w = world("lock");
    let before = snapshot(&w.paths.game);
    fs::create_dir_all(w.paths.record.parent().unwrap()).unwrap();
    let mut lock_path = w.paths.record.clone().into_os_string();
    lock_path.push(".lock");
    let held = fs::File::create(&lock_path).unwrap();
    held.try_lock().unwrap();
    let e = apply(&w.paths, &mod_files(&w, "A"), None).unwrap_err();
    assert!(e.to_string().contains("Turbine is already applying changes to this game"), "{e}");
    assert!(recover(&w.paths).is_err());
    assert!(reset_from_backups(&w.paths).is_err());
    assert_eq!(snapshot(&w.paths.game), before);
    drop(held);
    apply(&w.paths, &mod_files(&w, "A"), None).unwrap();
    apply(&w.paths, &[], None).unwrap();
    assert_eq!(snapshot(&w.paths.game), before);
}

#[test]
fn missing_game_folder_is_refused_by_recover_too() {
    let w = world("nogame");
    apply(&w.paths, &mod_files(&w, "A"), None).unwrap();
    let _ = apply(&w.paths, &[], Some(1));
    let away = w.paths.game.with_file_name("Game-away");
    fs::rename(&w.paths.game, &away).unwrap();
    assert!(recover(&w.paths).is_err());
    assert!(reset_from_backups(&w.paths).is_err());
    assert!(!w.paths.game.exists(), "the game folder was created");
    fs::rename(&away, &w.paths.game).unwrap();
    recover(&w.paths).unwrap();
}

#[test]
fn record_has_a_second_copy() {
    let w = world("prev");
    apply(&w.paths, &mod_files(&w, "A"), None).unwrap();
    assert_eq!(fs::read(&w.paths.record).unwrap(), fs::read(prev_path(&w.paths.record)).unwrap());
}

#[test]
fn reset_from_backups_gives_back_the_original_game() {
    let w = world("reset");
    let before = snapshot(&w.paths.game);
    apply(&w.paths, &mod_files(&w, "A"), None).unwrap();
    let r = reset_from_backups(&w.paths).unwrap();
    assert_eq!(r.removed, 3, "{r:?}");
    assert_eq!(snapshot(&w.paths.game), before);
    assert!(Record::load(&w.paths.record).unwrap().files.is_empty());
    // And again after a stop halfway.
    apply(&w.paths, &mod_files(&w, "A"), None).unwrap();
    let _ = apply(&w.paths, &[], Some(1));
    reset_from_backups(&w.paths).unwrap();
    assert_eq!(snapshot(&w.paths.game), before);
    assert!(!w.paths.plan.exists());
}

/// One record entry can't be read: it's set aside and reported, the deploy goes on without it, and Reset deals
/// with it (the file still matches what was written down, so it goes and the game's file comes back).
#[test]
fn bad_record_entry_is_quarantined_and_reset_handles_it() {
    let w = world("quar");
    let before = snapshot(&w.paths.game);
    apply(&w.paths, &mod_files(&w, "A"), None).unwrap();
    let text = fs::read_to_string(&w.paths.record).unwrap();
    let line = text.lines().find(|l| l.contains("Data/Skyrim.esm")).unwrap().to_string();
    let bad = line.replacen("\"mtime\":\"", "\"mtime\":\"x", 1);
    assert_ne!(bad, line);
    let damaged = text.replace(&line, &bad);
    fs::write(&w.paths.record, &damaged).unwrap();
    fs::write(prev_path(&w.paths.record), &damaged).unwrap();
    let r = apply(&w.paths, &[], None).unwrap();
    assert_eq!(r.bad_entries.len(), 1, "{r:?}");
    assert!(r.bad_entries[0].contains("Data/Skyrim.esm"), "{r:?}");
    assert_eq!(r.quarantined, 1);
    assert_eq!(fs::read_to_string(w.paths.game.join("Data/Skyrim.esm")).unwrap(), "A esm", "a set-aside entry is never acted on");
    assert_eq!(Record::load(&w.paths.record).unwrap().quarantine.len(), 1, "kept for Reset");
    reset_from_backups(&w.paths).unwrap();
    assert_eq!(snapshot(&w.paths.game), before);
    assert!(Record::load(&w.paths.record).unwrap().quarantine.is_empty());
}

/// Both copies of the record are gone to garbage: Reset still puts back every game file whose place is free and
/// lists the ones it can't be sure of.
#[test]
fn reset_works_with_no_usable_record() {
    let w = world("reset-norec");
    apply(&w.paths, &mod_files(&w, "A"), None).unwrap();
    fs::write(&w.paths.record, "garbage").unwrap();
    fs::write(prev_path(&w.paths.record), "garbage").unwrap();
    assert!(apply(&w.paths, &[], None).is_err());
    assert!(apply(&w.paths, &[], None).is_err(), "still refused: nothing to go on");
    let r = reset_from_backups(&w.paths).unwrap();
    assert_eq!(r.left_in_backups, vec!["Data/Skyrim.esm".to_string()], "{r:?}");
    // The player deletes the mod's copy; the next Reset puts the game's own file back.
    fs::remove_file(w.paths.game.join("Data/Skyrim.esm")).unwrap();
    let r = reset_from_backups(&w.paths).unwrap();
    assert!(r.left_in_backups.is_empty(), "{r:?}");
    assert_eq!(fs::read_to_string(w.paths.game.join("Data/Skyrim.esm")).unwrap(), "vanilla esm");
    apply(&w.paths, &[], None).unwrap();
}

/// Settings files are copies; a player's change is kept when the mod goes and comes back with it.
#[test]
fn player_settings_survive_turning_a_mod_off_and_on() {
    let w = world("settings");
    let before = snapshot(&w.paths.game);
    let cfg = w.staged.join("C/BepInEx/config/mod.cfg");
    let dll = w.staged.join("C/BepInEx/plugins/mod.dll");
    write(&cfg, "shipped");
    write(&dll, "dll");
    let wanted = vec![("BepInEx/config/mod.cfg".to_string(), cfg.clone()), ("BepInEx/plugins/mod.dll".to_string(), dll)];
    apply(&w.paths, &wanted, None).unwrap();
    let rec = Record::load(&w.paths.record).unwrap();
    assert!(rec.files.iter().find(|p| p.rel.ends_with("mod.cfg")).unwrap().copy, "settings files are copied");
    let target = w.paths.game.join("BepInEx/config/mod.cfg");
    fs::write(&target, "the player's settings, longer").unwrap();
    assert_eq!(fs::read_to_string(&cfg).unwrap(), "shipped", "the staged copy never changes");
    // Still on: left as the player made it.
    let r = apply(&w.paths, &wanted, None).unwrap();
    assert_eq!((r.added, r.removed, r.kept), (0, 0, 2), "{r:?}");
    let r = apply(&w.paths, &[], None).unwrap();
    assert_eq!(r.player_changes_saved, vec!["BepInEx/config/mod.cfg".to_string()]);
    assert_eq!(snapshot(&w.paths.game), before);
    let r = apply(&w.paths, &wanted, None).unwrap();
    assert_eq!(r.player_changes_restored, vec!["BepInEx/config/mod.cfg".to_string()]);
    assert_eq!(fs::read_to_string(&target).unwrap(), "the player's settings, longer");
    apply(&w.paths, &[], None).unwrap();
    assert_eq!(snapshot(&w.paths.game), before);
}

#[test]
fn steam_game_folder_linking_to_home_is_skipped() {
    let t = Tmp::new("linkhome");
    let home = &t.0;
    let sapps = steam_home(home, &[("10", "Good", "Good")]);
    write(&sapps.join("appmanifest_31.acf"), &manifest("31", "Home Link", Some("HomeLink")));
    symlink(home, sapps.join("common/HomeLink")).unwrap();
    assert_eq!(names(home), ["Good"]);
}

/// A store with two mod folders it picks up on open (off until turned on), on a game in `w`.
fn store_world(tag: &str) -> (Tmp, turbine_engine::Game, PathBuf) {
    let t = Tmp::new(tag);
    let library = t.0.join("library");
    let install = library.join("steamapps/common/G");
    write(&install.join("game.exe"), "exe");
    let game = turbine_engine::Game {
        store: Store::Steam,
        store_id: "10".into(),
        name: "G".into(),
        install_dir: install,
        library,
        build: Some("1".into()),
        runtime: turbine_engine::Runtime::Windows,
        cover: None,
        header: None,
        prefix: None,
    };
    let data = t.0.join("data");
    (t, game, data)
}

struct NoRules;

impl turbine_engine::store::ModRules for NoRules {
    fn map_archive(&self, files: &[String], _mod_name: &str) -> Result<Vec<(String, String)>, String> {
        Ok(files.iter().map(|f| (f.clone(), f.clone())).collect())
    }
    fn needs_loader(&self, _game_files: &[String]) -> bool {
        false
    }
}

#[test]
fn store_stages_on_the_library_and_restore_goes_to_the_played_setup() {
    use turbine_engine::store::Store as Mods;
    let (_t, game, data) = store_world("played");
    let root = Mods::root_for(&game, &data);
    assert!(root.starts_with(game.library.join(".turbine-mod-manager")), "{}", root.display());
    write(&root.join("mods/a/a.dll"), "a");
    write(&root.join("mods/b/b.dll"), "b");
    let mut s = Mods::open_with(&game, &data, Box::new(NoRules)).unwrap();
    s.set_clock(1000);
    assert!(s.restore("").is_err(), "nothing played yet");
    s.set_enabled("a", true).unwrap();
    let r = s.apply().unwrap();
    assert_eq!(r.copied_not_linked, 0, "same mount: hard links");
    s.mark_played().unwrap();
    assert_eq!(s.restore_target().map(|x| x.id.as_str()), Some("1000"));
    s.set_clock(2000);
    s.set_enabled("b", true).unwrap();
    s.apply().unwrap();
    assert!(!s.setups()[0].played && s.setups()[1].played);
    let v = json::parse(&s.to_json()).unwrap();
    let played: Vec<bool> = v.get("setups").unwrap().as_array().iter().map(|x| x.get("played").and_then(json::Json::as_bool).unwrap()).collect();
    assert_eq!(played, [false, true]);
    // Many applies later the played setup is still there.
    for i in 0..15u64 {
        s.set_clock(3000 + i);
        s.set_enabled("b", i % 2 == 0).unwrap();
        s.apply().unwrap();
    }
    assert_eq!(s.restore_target().map(|x| x.id.as_str()), Some("1000"));
    s.restore("").unwrap();
    assert!(s.mods().iter().find(|m| m.id == "a").unwrap().enabled);
    assert!(!s.mods().iter().find(|m| m.id == "b").unwrap().enabled);
    assert!(game.install_dir.join("a.dll").exists() && !game.install_dir.join("b.dll").exists());
    // Survives reopening; Reset takes everything out.
    drop(s);
    let mut s = Mods::open_with(&game, &data, Box::new(NoRules)).unwrap();
    assert!(s.restore_target().is_some());
    s.reset_game().unwrap();
    assert!(!game.install_dir.join("a.dll").exists());
    assert!(s.mods().iter().all(|m| !m.enabled));
}
