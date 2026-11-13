//! Deploy safety: the game folder always ends up exactly as planned, a stop at any step is recovered, and turning
//! everything off gives back the original game folder byte for byte.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use turbine_engine::deploy::{apply, recover, Paths, Record};

fn write(p: &Path, text: &str) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

/// Every file and folder under `root` (relative path -> contents, folders as "<dir>").
fn snapshot(root: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
        for e in fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            let rel = p.strip_prefix(root).unwrap().to_string_lossy().into_owned();
            if p.is_dir() {
                out.insert(rel, "<dir>".into());
                walk(root, &p, out);
            } else {
                out.insert(rel, fs::read_to_string(&p).unwrap());
            }
        }
    }
    walk(root, root, &mut out);
    out
}

struct World {
    root: PathBuf,
    paths: Paths,
    staged: PathBuf,
}

impl Drop for World {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn read(p: &Path) -> String {
    fs::read_to_string(p).unwrap_or_else(|e| format!("<missing: {e}>"))
}

fn world(tag: &str) -> World {
    let root = std::env::temp_dir().join(format!("turbine-deploy-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let game = root.join("game");
    write(&game.join("Data/Skyrim.esm"), "vanilla esm");
    write(&game.join("Data/Textures/sky.dds"), "vanilla sky");
    write(&game.join("readme.txt"), "vanilla readme");
    fs::create_dir_all(game.join("Data/EmptyGameFolder")).unwrap();
    let staged = root.join("staging");
    write(&staged.join("A/Data/textures/Sky.dds"), "A sky");
    write(&staged.join("A/Data/meshes/a.nif"), "A mesh");
    write(&staged.join("A/Data/ModA.esp"), "A plugin");
    write(&staged.join("A/Data/EmptyGameFolder/inside.txt"), "A inside");
    write(&staged.join("B/Data/Meshes/A.nif"), "B mesh");
    write(&staged.join("B/Data/ModB.esp"), "B plugin");
    write(&staged.join("B/Data/New/Deep/b.txt"), "B deep");
    let paths = Paths { game, backups: root.join("backups"), record: root.join("record.json"), plan: root.join("plan.json") };
    World { root, paths, staged }
}

impl World {
    /// Stages one file for a mod and returns (rel, staged path).
    fn stage(&self, m: &str, rel: &str, text: &str) -> (String, PathBuf) {
        let p = self.staged.join(m).join(rel);
        write(&p, text);
        (rel.to_string(), p)
    }
}

fn files_of(w: &World, m: &str) -> Vec<(String, PathBuf)> {
    let base = w.staged.join(m);
    let mut out = Vec::new();
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
    walk(&base, &base, &mut out);
    out
}

fn wanted(w: &World, mods: &[&str]) -> Vec<(String, PathBuf)> {
    mods.iter().flat_map(|m| files_of(w, m)).collect()
}

/// Finds a path ignoring case and checks there is only one spelling of it on disk.
fn ci(root: &Path, rel: &str) -> PathBuf {
    let mut dir = root.to_path_buf();
    for part in rel.split('/') {
        let hits: Vec<PathBuf> = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.file_name().unwrap().to_string_lossy().eq_ignore_ascii_case(part))
            .collect();
        assert_eq!(hits.len(), 1, "{part} in {}: {hits:?}", dir.display());
        dir = hits[0].clone();
    }
    dir
}

fn expect_ab(game: &Path) {
    // B wins the mesh (one spelling of the folder only), A's sky replaces the game's under the game's own casing.
    assert_eq!(fs::read_to_string(ci(game, "data/meshes/a.nif")).unwrap(), "B mesh");
    assert_eq!(fs::read_to_string(game.join("Data/Textures/sky.dds")).unwrap(), "A sky");
    assert_eq!(fs::read_to_string(game.join("Data/ModA.esp")).unwrap(), "A plugin");
    assert_eq!(fs::read_to_string(game.join("Data/New/Deep/b.txt")).unwrap(), "B deep");
    assert_eq!(fs::read_to_string(game.join("Data/Skyrim.esm")).unwrap(), "vanilla esm");
}

fn expect_a(game: &Path) {
    assert_eq!(fs::read_to_string(ci(game, "data/meshes/a.nif")).unwrap(), "A mesh");
    assert!(!game.join("Data/ModB.esp").exists());
    assert!(!game.join("Data/New").exists(), "folders Turbine made go when empty");
    assert_eq!(fs::read_to_string(game.join("Data/Textures/sky.dds")).unwrap(), "A sky");
}

#[test]
fn deploys_switches_and_restores_exactly() {
    let w = world("basic");
    let original = snapshot(&w.paths.game);
    let r = apply(&w.paths, &wanted(&w, &["A", "B"]), None).unwrap();
    assert_eq!(r.added, 7 - 1); // A's mesh is overridden by B's, so 6 files land
    expect_ab(&w.paths.game);
    // Turbine's mod files are links to the staged copies, except the one replacing a game file (a copy).
    let rec = Record::load(&w.paths.record).unwrap();
    let sky = rec.files.iter().find(|p| p.rel == "Data/Textures/sky.dds").unwrap();
    assert!(sky.copy && sky.backup.is_some());
    assert!(!rec.files.iter().find(|p| p.rel == "Data/ModA.esp").unwrap().copy);

    let r = apply(&w.paths, &wanted(&w, &["A"]), None).unwrap();
    assert_eq!((r.removed, r.added), (3, 1));
    expect_a(&w.paths.game);

    // Same request again: nothing to do.
    let r = apply(&w.paths, &wanted(&w, &["A"]), None).unwrap();
    assert_eq!((r.added, r.removed, r.kept), (0, 0, 4));

    apply(&w.paths, &[], None).unwrap();
    assert_eq!(snapshot(&w.paths.game), original, "turning everything off gives back the original folder");
    assert!(Record::load(&w.paths.record).unwrap().files.is_empty());
}

#[test]
fn never_touches_files_it_did_not_place() {
    let w = world("others");
    let original = snapshot(&w.paths.game);
    apply(&w.paths, &wanted(&w, &["A", "B"]), None).unwrap();
    // A loader writes its own config inside a folder Turbine made, and a game update rewrites a replaced file.
    write(&w.paths.game.join("Data/New/config.ini"), "made by the game");
    fs::write(w.paths.game.join("Data/Textures/sky.dds"), "updated by Steam").unwrap();
    let r = apply(&w.paths, &[], None).unwrap();
    assert_eq!(r.changed_by_others, vec!["Data/Textures/sky.dds".to_string()]);
    assert_eq!(fs::read_to_string(w.paths.game.join("Data/New/config.ini")).unwrap(), "made by the game");
    assert_eq!(fs::read_to_string(w.paths.game.join("Data/Textures/sky.dds")).unwrap(), "updated by Steam");
    // Everything else is back as it was.
    let now = snapshot(&w.paths.game);
    for (k, v) in &original {
        if k != "Data/Textures/sky.dds" {
            assert_eq!(now.get(k), Some(v), "{k}");
        }
    }
}

#[test]
fn recovers_from_a_stop_at_every_step() {
    // How many steps does each switch take?
    let probe = world("probe");
    apply(&probe.paths, &wanted(&probe, &["A", "B"]), None).unwrap();
    let r = apply(&probe.paths, &wanted(&probe, &["A"]), None).unwrap();
    let switch_steps = r.added + r.removed;
    drop(probe);

    type Case<'a> = (&'a [&'a str], &'a [&'a str], fn(&Path));
    let cases: [Case; 3] = [(&[], &["A", "B"], expect_ab), (&["A", "B"], &["A"], expect_a), (&["A"], &["A", "B"], expect_ab)];
    for (first, second, check) in cases {
        for stop in 0..=switch_steps.max(7) {
            let w = world(&format!("stop{stop}"));
            let original = snapshot(&w.paths.game);
            apply(&w.paths, &wanted(&w, first), None).unwrap();
            let _ = apply(&w.paths, &wanted(&w, second), Some(stop));
            // Next start: recovery, then the same request again.
            recover(&w.paths).unwrap();
            assert!(!w.paths.plan.exists());
            apply(&w.paths, &wanted(&w, second), None).unwrap();
            check(&w.paths.game);
            apply(&w.paths, &[], None).unwrap();
            assert_eq!(snapshot(&w.paths.game), original, "stop at step {stop}");
        }
    }
}

#[test]
fn a_cut_off_plan_changes_nothing() {
    let w = world("cutplan");
    apply(&w.paths, &wanted(&w, &["A"]), None).unwrap();
    let before = snapshot(&w.paths.game);
    fs::write(&w.paths.plan, "{\"ops\":[{\"op\":\"add\",\"rel\":").unwrap();
    recover(&w.paths).unwrap();
    assert_eq!(snapshot(&w.paths.game), before);
    assert!(!w.paths.plan.exists());
}

#[test]
fn same_path_different_case_two_mods_one_file() {
    let w = world("case2");
    let original = snapshot(&w.paths.game);
    let a = w.stage("A", "Data/Foo.esp", "A foo");
    let b = w.stage("B", "data/FOO.ESP", "B foo");
    let r = apply(&w.paths, &[a, b], None).unwrap();
    assert_eq!(r.added, 1);
    let names: Vec<String> = fs::read_dir(w.paths.game.join("Data")).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(names.iter().filter(|n| n.eq_ignore_ascii_case("foo.esp")).count(), 1, "{names:?}");
    // The game's folder casing is used, the later mod wins.
    assert!(!w.paths.game.join("data").exists());
    let foo = names.iter().find(|n| n.eq_ignore_ascii_case("foo.esp")).unwrap();
    assert_eq!(read(&w.paths.game.join("Data").join(foo)), "B foo");
    apply(&w.paths, &[], None).unwrap();
    assert_eq!(snapshot(&w.paths.game), original);
}

#[test]
fn new_folder_case_shared_within_plan() {
    let w = world("newcase");
    let original = snapshot(&w.paths.game);
    let a = w.stage("A", "BepInEx/plugins/a.dll", "a");
    let b = w.stage("B", "bepinex/Plugins/b.dll", "b");
    apply(&w.paths, &[a, b], None).unwrap();
    let tops: Vec<String> = fs::read_dir(&w.paths.game).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(tops.iter().filter(|n| n.eq_ignore_ascii_case("bepinex")).count(), 1, "{tops:?}");
    apply(&w.paths, &[], None).unwrap();
    assert_eq!(snapshot(&w.paths.game), original);
}

#[test]
fn spaces_and_unicode_round_trip() {
    let w = world("unicode");
    write(&w.paths.game.join("Data/Ünïcode Gäme Fïle.txt"), "vanilla unicode");
    let original = snapshot(&w.paths.game);
    let wanted = vec![
        w.stage("A", "Data/My Mod Folder/名前 with spaces.esp", "A plugin"),
        w.stage("A", "Data/Ünïcode Gäme Fïle.txt", "A replaces unicode"),
        w.stage("A", "Data/ trailing and leading .txt", "spaces"),
        w.stage("A", "Data/quote\"and'apostrophe.txt", "quotes"),
    ];
    apply(&w.paths, &wanted, None).unwrap();
    assert_eq!(read(&w.paths.game.join("Data/My Mod Folder/名前 with spaces.esp")), "A plugin");
    assert_eq!(read(&w.paths.game.join("Data/Ünïcode Gäme Fïle.txt")), "A replaces unicode");
    // The record must read back exactly.
    let rec = Record::load(&w.paths.record).unwrap();
    assert_eq!(rec.files.len(), 4);
    apply(&w.paths, &[], None).unwrap();
    assert_eq!(snapshot(&w.paths.game), original);
}

/// Mod A and mod B share most paths (some replace game files), so switching A -> B removes and adds the same paths.
/// Over 200 files so the record is also saved partway. Stops at many steps, then recovers.
#[test]
fn stop_at_every_step_remove_and_add_same_paths() {
    fn build(w: &World, m: &str, n: usize) -> Vec<(String, PathBuf)> {
        let mut v = vec![
            w.stage(m, "Data/Textures/sky.dds", &format!("{m} sky")),
            w.stage(m, "readme.txt", &format!("{m} readme")),
            w.stage(m, "Data/Only/only.txt", &format!("{m} only")),
        ];
        for i in 0..n {
            v.push(w.stage(m, &format!("Data/Many/f{i:03}.txt"), &format!("{m} {i}")));
        }
        v
    }
    let n = 230;
    let probe = world("stopprobe");
    let a = build(&probe, "A", n);
    let b = build(&probe, "B", n);
    apply(&probe.paths, &a, None).unwrap();
    let r = apply(&probe.paths, &b, None).unwrap();
    let steps = r.added + r.removed;
    drop(probe);
    assert!(steps > 400, "{steps}");
    let mut stops: Vec<usize> = vec![0, 1, 2, 3, 4, 5, 198, 199, 200, 201, 202, 229, 230, 231, 232, 233, 234, 235, 398, 399, 400, 401];
    stops.extend([steps - 3, steps - 2, steps - 1, steps]);
    for stop in stops {
        let w = world(&format!("stopsame{stop}"));
        let original = snapshot(&w.paths.game);
        let a = build(&w, "A", n);
        let b = build(&w, "B", n);
        apply(&w.paths, &a, None).unwrap();
        let _ = apply(&w.paths, &b, Some(stop));
        recover(&w.paths).unwrap();
        assert!(!w.paths.plan.exists());
        apply(&w.paths, &b, None).unwrap();
        assert_eq!(read(&w.paths.game.join("Data/Textures/sky.dds")), "B sky", "stop {stop}");
        assert_eq!(read(&w.paths.game.join("readme.txt")), "B readme", "stop {stop}");
        assert_eq!(read(&w.paths.game.join("Data/Many/f117.txt")), "B 117", "stop {stop}");
        let rec = Record::load(&w.paths.record).unwrap();
        assert_eq!(rec.files.len(), n + 3, "stop {stop}: record must list every placed file once");
        apply(&w.paths, &[], None).unwrap();
        assert_eq!(snapshot(&w.paths.game), original, "stop {stop}");
        assert!(Record::load(&w.paths.record).unwrap().files.is_empty());
    }
}

/// Power loss inside one remove: the mod file was deleted but the game's file was not moved back yet.
#[test]
fn stop_inside_remove_before_backup_restore() {
    let w = world("insiderm");
    let original = snapshot(&w.paths.game);
    let sky = w.stage("A", "Data/Textures/sky.dds", "A sky");
    apply(&w.paths, &[sky], None).unwrap();
    let _ = apply(&w.paths, &[], Some(0));
    fs::remove_file(w.paths.game.join("Data/Textures/sky.dds")).unwrap();
    recover(&w.paths).unwrap();
    assert_eq!(snapshot(&w.paths.game), original);
}

/// A file shows up (the game or the player made it) after Turbine planned to put a mod file there.
#[test]
fn file_appearing_after_plan_is_kept() {
    let w = world("appear");
    let cfg = w.stage("A", "BepInEx/config/mod.cfg", "A config");
    let _ = apply(&w.paths, std::slice::from_ref(&cfg), Some(0));
    write(&w.paths.game.join("BepInEx/config/mod.cfg"), "made by the game");
    recover(&w.paths).unwrap();
    assert_eq!(read(&w.paths.game.join("BepInEx/config/mod.cfg")), "made by the game");
    apply(&w.paths, std::slice::from_ref(&cfg), None).unwrap();
    assert_eq!(read(&w.paths.game.join("BepInEx/config/mod.cfg")), "A config");
    apply(&w.paths, &[], None).unwrap();
    assert_eq!(read(&w.paths.game.join("BepInEx/config/mod.cfg")), "made by the game", "the game's own file comes back");
}

#[test]
fn empty_path_is_refused() {
    let w = world("emptyrel");
    let original = snapshot(&w.paths.game);
    let f = w.stage("A", "x.txt", "mod");
    let r = apply(&w.paths, &[(String::new(), f.1.clone())], None);
    let game_is_dir = w.paths.game.is_dir();
    assert!(r.is_err(), "an empty path must be refused");
    assert!(game_is_dir && snapshot(&w.paths.game) == original, "the game folder must be untouched");
}

#[test]
fn stale_backup_does_not_collide() {
    let w = world("bakstale");
    write(&w.paths.game.join("x.txt"), "vanilla x");
    write(&w.paths.game.join("x.txt.1"), "vanilla x.1");
    let original = snapshot(&w.paths.game);
    // A backup left over from an earlier run (a game update replaced a modded file, so its old backup stays).
    write(&w.paths.backups.join("x.txt"), "stale");
    apply(&w.paths, &[w.stage("A", "x.txt", "A x"), w.stage("A", "x.txt.1", "A x.1")], None).unwrap();
    apply(&w.paths, &[], None).unwrap();
    assert_eq!(snapshot(&w.paths.game), original, "a game file was overwritten in the backup folder");
}

#[test]
fn kept_file_gone_is_placed_again() {
    let w = world("missing");
    let a = w.stage("A", "BepInEx/plugins/a.dll", "A dll");
    apply(&w.paths, std::slice::from_ref(&a), None).unwrap();
    fs::remove_file(w.paths.game.join("BepInEx/plugins/a.dll")).unwrap();
    apply(&w.paths, std::slice::from_ref(&a), None).unwrap();
    assert_eq!(read(&w.paths.game.join("BepInEx/plugins/a.dll")), "A dll", "Turbine shows the mod on but its file is gone");
}

#[test]
fn game_file_restored_by_steam_is_modded_again() {
    let w = world("verify");
    let sky = w.stage("A", "Data/Textures/sky.dds", "A sky");
    apply(&w.paths, std::slice::from_ref(&sky), None).unwrap();
    // Steam's "verify files" or an update writes the game's own file back.
    fs::remove_file(w.paths.game.join("Data/Textures/sky.dds")).unwrap();
    write(&w.paths.game.join("Data/Textures/sky.dds"), "vanilla sky v2");
    let r = apply(&w.paths, std::slice::from_ref(&sky), None).unwrap();
    assert_eq!(read(&w.paths.game.join("Data/Textures/sky.dds")), "A sky", "report: {r:?}");
}

#[test]
fn file_replaced_by_game_then_new_mod_deploys() {
    let w = world("othersnew");
    let a = w.stage("A", "BepInEx/plugins/shared.dll", "A");
    apply(&w.paths, &[a], None).unwrap();
    // The game (or the player) replaces the file with a new one.
    let t = w.paths.game.join("BepInEx/plugins/shared.dll");
    fs::remove_file(&t).unwrap();
    write(&t, "replaced by the game");
    let b = w.stage("B", "BepInEx/plugins/shared.dll", "B");
    let r = apply(&w.paths, &[b], None);
    assert!(r.is_ok(), "deploy failed: {r:?}");
}

#[test]
fn damaged_record_entry_leaves_nothing_untracked() {
    let w = world("dropentry");
    let original = snapshot(&w.paths.game);
    apply(&w.paths, &[w.stage("A", "Data/a.esp", "A"), w.stage("A", "Data/b.esp", "B")], None).unwrap();
    let text = fs::read_to_string(&w.paths.record).unwrap();
    // Damage one entry (a bad edit, a bit flip): its "source" key no longer reads.
    let damaged = text.replacen("\"source\"", "\"sourc\"", 1);
    assert_ne!(damaged, text);
    fs::write(&w.paths.record, damaged).unwrap();
    let r = apply(&w.paths, &[], None);
    assert!(r.is_err() || snapshot(&w.paths.game) == original, "a mod file is now untracked in the game folder: {r:?}");
}

#[test]
fn in_place_write_does_not_change_staging() {
    let w = world("inplace");
    let cfg = w.stage("A", "BepInEx/config/BepInEx.cfg", "shipped config");
    apply(&w.paths, std::slice::from_ref(&cfg), None).unwrap();
    // BepInEx rewrites its config on first run by truncating the existing file (same inode).
    fs::write(w.paths.game.join("BepInEx/config/BepInEx.cfg"), "rewritten by the loader").unwrap();
    assert_eq!(read(&cfg.1), "shipped config", "the staged mod file changed through the hard link");
}

/// A mod file that is gone from Turbine's own folder (deleted, or its drive is out) is named in the error, with
/// what to do, and the game's folder comes back as it was.
#[test]
fn a_missing_mod_file_is_named_in_the_error() {
    let w = world("gone-source");
    let before = snapshot(&w.paths.game);
    let gone = w.staged.join("A/Data/meshes/never-there.nif");
    let e = apply(&w.paths, &[("Data/meshes/never-there.nif".to_string(), gone.clone())], None).unwrap_err();
    assert_eq!(e.kind(), std::io::ErrorKind::NotFound);
    let said = e.to_string();
    assert!(said.contains("never-there.nif") && said.contains(&gone.display().to_string()), "{said}");
    assert!(said.contains("add it again") && !said.contains("os error"), "{said}");
    apply(&w.paths, &[], None).unwrap();
    assert_eq!(snapshot(&w.paths.game), before);
}
