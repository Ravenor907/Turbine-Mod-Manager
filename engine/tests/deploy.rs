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
