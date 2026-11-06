//! QA checks for 0.2: tries to break deploy, zip and the file readers.
//!
//! Tests marked #[ignore = "QA defect ..."] show a known defect and fail today. Run them with:
//!   tools/test-engine.sh --test qa -- --ignored
//! When a defect is fixed, remove its #[ignore] so the test guards the fix from then on.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use turbine_engine::deploy::{apply, recover, Paths, Record};
use turbine_engine::inflate::inflate;
use turbine_engine::zip::{crc32, Limits, Zip};

// ------------------------------------------------------------------ helpers

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
    let root = std::env::temp_dir().join(format!("turbine-qa-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let game = root.join("game");
    write(&game.join("Data/Skyrim.esm"), "vanilla esm");
    write(&game.join("Data/Textures/sky.dds"), "vanilla sky");
    write(&game.join("readme.txt"), "vanilla readme");
    let staged = root.join("staging");
    fs::create_dir_all(&staged).unwrap();
    let paths = Paths { game, backups: root.join("backups"), record: root.join("record.json"), plan: root.join("plan.json") };
    World { root, paths, staged }
}

impl World {
    /// Stages a file for a mod and returns (rel, staged path).
    fn stage(&self, m: &str, rel: &str, text: &str) -> (String, PathBuf) {
        let p = self.staged.join(m).join(rel);
        write(&p, text);
        (rel.to_string(), p)
    }
}

fn read(p: &Path) -> String {
    fs::read_to_string(p).unwrap_or_else(|e| format!("<missing: {e}>"))
}

// ------------------------------------------------------------------ deploy: things that must keep working

#[test]
fn qa_same_path_different_case_two_mods_one_file() {
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
fn qa_new_folder_case_shared_within_plan() {
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
fn qa_spaces_and_unicode_round_trip() {
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
fn qa_stop_at_every_step_remove_and_add_same_paths() {
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

/// The plan's backup path for `rel`, read from the plan file left by a stopped deploy.
fn planned_backup(w: &World, rel: &str) -> PathBuf {
    let text = fs::read_to_string(&w.paths.plan).unwrap();
    let v = turbine_engine::json::parse(&text).unwrap();
    for op in v.get("ops").unwrap().as_array() {
        if op.str_at("op") == Some("add") && op.str_at("rel") == Some(rel) {
            return PathBuf::from(op.str_at("backup").expect("this add has a backup"));
        }
    }
    panic!("no add for {rel} in plan");
}

/// Power loss inside one add: the game file was moved to backups, then (a) nothing else, (b) a half-written temporary
/// copy, (c) the full copy was put in place but not recorded.
#[test]
fn qa_stop_inside_add_after_backup_move() {
    for case in ["moved", "half tmp", "placed"] {
        let w = world(&format!("insideadd{}", case.len()));
        let original = snapshot(&w.paths.game);
        let sky = w.stage("A", "Data/Textures/sky.dds", "A sky");
        let _ = apply(&w.paths, std::slice::from_ref(&sky), Some(0));
        let target = w.paths.game.join("Data/Textures/sky.dds");
        let b = planned_backup(&w, "Data/Textures/sky.dds");
        fs::create_dir_all(b.parent().unwrap()).unwrap();
        fs::rename(&target, &b).unwrap();
        match case {
            "half tmp" => fs::write(target.with_extension("turbine-tmp"), "A s").unwrap(),
            "placed" => {
                fs::copy(&sky.1, &target).unwrap();
                let f = fs::File::options().write(true).open(&target).unwrap();
                f.set_modified(fs::metadata(&sky.1).unwrap().modified().unwrap()).unwrap();
            }
            _ => {}
        }
        recover(&w.paths).unwrap();
        assert!(!w.paths.game.join("Data/Textures/sky.turbine-tmp").exists(), "{case}");
        apply(&w.paths, std::slice::from_ref(&sky), None).unwrap();
        assert_eq!(read(&target), "A sky", "{case}");
        apply(&w.paths, &[], None).unwrap();
        assert_eq!(snapshot(&w.paths.game), original, "{case}");
    }
}

/// Power loss inside one remove: the mod file was deleted but the game's file was not moved back yet.
#[test]
fn qa_stop_inside_remove_before_backup_restore() {
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
fn qa_file_appearing_after_plan_is_kept() {
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
fn qa_record_garbage_refuses_to_touch_anything() {
    let w = world("garbage");
    let a = w.stage("A", "Data/a.esp", "A");
    apply(&w.paths, std::slice::from_ref(&a), None).unwrap();
    let before = snapshot(&w.paths.game);
    for junk in ["", "{", "\u{0}\u{0}\u{0}", "{\"files\":[{\"rel\":\"Data/a.esp\""] {
        fs::write(&w.paths.record, junk).unwrap();
        assert!(apply(&w.paths, &[], None).is_err(), "{junk:?} must stop the deploy");
        assert_eq!(snapshot(&w.paths.game), before, "{junk:?}");
    }
}

// ------------------------------------------------------------------ deploy: defects

#[test]
fn qa_defect_rel_with_dotdot_escapes_game_folder() {
    let w = world("dotdot");
    let evil = w.stage("A", "x.txt", "escaped");
    let r = apply(&w.paths, &[("../outside.txt".into(), evil.1.clone())], None);
    let escaped = w.root.join("outside.txt").exists();
    assert!(r.is_err() && !escaped, "a '..' path must be refused, file written outside: {escaped}");
}

#[test]
fn qa_defect_empty_rel_moves_the_whole_game_folder() {
    let w = world("emptyrel");
    let original = snapshot(&w.paths.game);
    let f = w.stage("A", "x.txt", "mod");
    let r = apply(&w.paths, &[(String::new(), f.1.clone())], None);
    let game_is_dir = w.paths.game.is_dir();
    assert!(r.is_err(), "an empty path must be refused");
    assert!(game_is_dir && snapshot(&w.paths.game) == original, "the game folder must be untouched");
}

#[test]
fn qa_defect_mod_file_over_game_folder_moves_folder() {
    let w = world("dirclash");
    write(&w.paths.game.join("Mods/GameOwned/keep.txt"), "game file in a folder");
    let f = w.stage("A", "Mods", "a file named like the game's folder");
    let _ = apply(&w.paths, &[f], None);
    assert_eq!(read(&w.paths.game.join("Mods/GameOwned/keep.txt")), "game file in a folder", "a whole game folder was moved to backups");
}

#[test]
fn qa_defect_symlinked_folder_escapes_game_folder() {
    let w = world("symlink");
    let outside = w.root.join("outside");
    fs::create_dir_all(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, w.paths.game.join("linked")).unwrap();
    let f = w.stage("A", "linked/mod.txt", "mod");
    let _ = apply(&w.paths, &[f], None);
    assert!(!outside.join("mod.txt").exists(), "a mod file was written outside the game folder through a link");
}

#[test]
fn qa_defect_backup_name_collision_in_one_plan_loses_game_file() {
    let w = world("bakplan");
    write(&w.paths.game.join("x.txt"), "vanilla x");
    write(&w.paths.game.join("x.txt.1"), "vanilla x.1");
    let original = snapshot(&w.paths.game);
    // Mod A replaces x.txt (its backup is backups/x.txt).
    apply(&w.paths, &[w.stage("A", "x.txt", "A x")], None).unwrap();
    // Mod B replaces x.txt (backup name taken, so backups/x.txt.1) and also x.txt.1 (backups/x.txt.1 again).
    apply(&w.paths, &[w.stage("B", "x.txt", "B x"), w.stage("B", "x.txt.1", "B x.1")], None).unwrap();
    apply(&w.paths, &[], None).unwrap();
    assert_eq!(snapshot(&w.paths.game), original, "a game file was overwritten in the backup folder");
}

#[test]
fn qa_defect_stale_backup_collision_loses_game_file() {
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
fn qa_defect_missing_kept_file_not_restored() {
    let w = world("missing");
    let a = w.stage("A", "BepInEx/plugins/a.dll", "A dll");
    apply(&w.paths, std::slice::from_ref(&a), None).unwrap();
    fs::remove_file(w.paths.game.join("BepInEx/plugins/a.dll")).unwrap();
    apply(&w.paths, std::slice::from_ref(&a), None).unwrap();
    assert_eq!(read(&w.paths.game.join("BepInEx/plugins/a.dll")), "A dll", "Turbine shows the mod on but its file is gone");
}

#[test]
fn qa_defect_vanilla_restored_by_steam_not_remodded() {
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
fn qa_defect_changed_by_others_then_new_source_errors() {
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
fn qa_defect_record_entry_dropped_leaves_untracked_file() {
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
fn qa_defect_hard_link_in_place_write_changes_staging() {
    let w = world("inplace");
    let cfg = w.stage("A", "BepInEx/config/BepInEx.cfg", "shipped config");
    apply(&w.paths, std::slice::from_ref(&cfg), None).unwrap();
    // BepInEx rewrites its config on first run by truncating the existing file (same inode).
    fs::write(w.paths.game.join("BepInEx/config/BepInEx.cfg"), "rewritten by the loader").unwrap();
    assert_eq!(read(&cfg.1), "shipped config", "the staged mod file changed through the hard link");
}

#[test]
fn qa_defect_long_name_copy_fails() {
    let w = world("longname");
    let name = format!("{}.b", "n".repeat(251)); // 253 bytes, legal on ext4 and exFAT
    write(&w.paths.game.join(&name), "vanilla long");
    let f = w.stage("A", &name, "mod long");
    let r = apply(&w.paths, &[f], None);
    assert!(r.is_ok(), "{r:?}");
}

// ------------------------------------------------------------------ zip and inflate

/// A tiny zip writer for test archives. Each entry: (name, data, method 0 stored).
fn zip_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut cd = Vec::new();
    for (name, data) in entries {
        let off = out.len() as u32;
        let crc = crc32(data);
        let le16 = |v: &mut Vec<u8>, x: u16| v.extend_from_slice(&x.to_le_bytes());
        let le32 = |v: &mut Vec<u8>, x: u32| v.extend_from_slice(&x.to_le_bytes());
        le32(&mut out, 0x0403_4b50);
        le16(&mut out, 20);
        le16(&mut out, 0);
        le16(&mut out, 0);
        le32(&mut out, 0);
        le32(&mut out, crc);
        le32(&mut out, data.len() as u32);
        le32(&mut out, data.len() as u32);
        le16(&mut out, name.len() as u16);
        le16(&mut out, 0);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        le32(&mut cd, 0x0201_4b50);
        le16(&mut cd, 20 | (3 << 8));
        le16(&mut cd, 20);
        le16(&mut cd, 0);
        le16(&mut cd, 0);
        le32(&mut cd, 0);
        le32(&mut cd, crc);
        le32(&mut cd, data.len() as u32);
        le32(&mut cd, data.len() as u32);
        le16(&mut cd, name.len() as u16);
        le16(&mut cd, 0);
        le16(&mut cd, 0);
        le16(&mut cd, 0);
        le16(&mut cd, 0);
        le32(&mut cd, 0o100644 << 16);
        le32(&mut cd, off);
        cd.extend_from_slice(name.as_bytes());
    }
    finish_zip(out, cd, entries.len())
}

fn finish_zip(mut out: Vec<u8>, cd: Vec<u8>, count: usize) -> Vec<u8> {
    let cd_off = out.len() as u32;
    out.extend_from_slice(&cd);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&(count.min(0xFFFF) as u16).to_le_bytes());
    out.extend_from_slice(&(count.min(0xFFFF) as u16).to_le_bytes());
    out.extend_from_slice(&(cd.len() as u32).to_le_bytes());
    out.extend_from_slice(&cd_off.to_le_bytes());
    out.extend_from_slice(&[0, 0]);
    out
}

fn scratch(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("turbine-qa-zip-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

fn data(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data").join(name)
}

/// Opens and unpacks `bytes`, catching panics. Ok(true) when it unpacked, Ok(false) on a clean error.
fn try_zip(bytes: &[u8], dir: &Path) -> Result<bool, String> {
    let file = dir.join("t.zip");
    fs::write(&file, bytes).unwrap();
    let dest = dir.join("out");
    let _ = fs::remove_dir_all(&dest);
    let limits = Limits { total: 64 << 20, ratio: 400 };
    let r = std::panic::catch_unwind(|| match Zip::open(&file) {
        Ok(mut z) => z.extract_all(&dest, &limits).is_ok(),
        Err(_) => false,
    });
    // Nothing may land next to the destination (only t.zip and out/ are allowed in dir).
    for e in fs::read_dir(dir).unwrap().flatten() {
        let n = e.file_name().to_string_lossy().into_owned();
        if n != "t.zip" && n != "out" {
            return Err(format!("left behind or escaped: {n}"));
        }
    }
    r.map_err(|_| "panicked".to_string())
}

#[test]
fn qa_zip_every_truncation_is_an_error_not_a_panic() {
    let dir = scratch("trunc");
    for name in ["good.zip", "zip64.zip", "link.zip"] {
        let full = fs::read(data(name)).unwrap();
        assert_eq!(try_zip(&full, &dir), Ok(true), "{name} whole");
        // Every length near the ends, then a stride through the middle.
        let mut cuts: Vec<usize> = (0..full.len().min(400)).collect();
        cuts.extend((full.len().saturating_sub(400)..full.len()).filter(|c| *c >= 400));
        cuts.extend((400..full.len().saturating_sub(400)).step_by(97));
        for cut in cuts {
            let r = try_zip(&full[..cut], &dir);
            assert!(r == Ok(false), "{name} cut at {cut}: {r:?}");
        }
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn qa_zip_random_damage_never_panics_or_escapes() {
    let dir = scratch("damage");
    let mut seed = 0x9E37_79B9_7F4A_7C15u64;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for name in ["good.zip", "zip64.zip", "bomb.zip"] {
        let full = fs::read(data(name)).unwrap();
        for _ in 0..600 {
            let mut b = full.clone();
            for _ in 0..(1 + rnd() % 4) {
                let i = (rnd() as usize) % b.len();
                b[i] = rnd() as u8;
            }
            let r = try_zip(&b, &dir);
            assert!(r.is_ok(), "{name}: {r:?}");
        }
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn qa_inflate_random_input_never_panics() {
    let mut seed = 0x2545_F491_4F6C_DD1Du64;
    for len in 0..3000usize {
        let mut v = Vec::with_capacity(len % 300);
        for _ in 0..(len % 300) {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            v.push(seed as u8);
        }
        let r = std::panic::catch_unwind(|| inflate(&v, 1 << 20));
        assert!(r.is_ok(), "inflate panicked on {v:?}");
    }
}

#[test]
fn qa_zip_claimed_count_too_big_fails_fast() {
    let dir = scratch("count");
    let mut z = zip_bytes(&[("a.txt", b"a")]);
    let n = z.len();
    // Claim 65535 entries in a directory that holds one.
    z[n - 12..n - 10].copy_from_slice(&0xFFFFu16.to_le_bytes());
    z[n - 14..n - 12].copy_from_slice(&0xFFFFu16.to_le_bytes());
    let t = std::time::Instant::now();
    assert_eq!(try_zip(&z, &dir), Ok(false));
    assert!(t.elapsed().as_secs() < 2);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn qa_zip_many_entries_ok() {
    let dir = scratch("many");
    let names: Vec<String> = (0..20_000).map(|i| format!("BepInEx/plugins/p{}/f{i}.txt", i % 50)).collect();
    let entries: Vec<(&str, &[u8])> = names.iter().map(|n| (n.as_str(), b"x" as &[u8])).collect();
    let z = zip_bytes(&entries);
    let t = std::time::Instant::now();
    assert_eq!(try_zip(&z, &dir), Ok(true));
    assert!(t.elapsed().as_secs() < 60, "{:?}", t.elapsed());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn qa_zip_name_tricks_refused() {
    let dir = scratch("names");
    for bad in ["../x.txt", "a/../../x.txt", "/abs.txt", "C:/x.txt", "C:x.txt", "..\\x.txt", "a\\..\\..\\x.txt", "a\u{0}b.txt"] {
        let z = zip_bytes(&[(bad, b"evil")]);
        assert_eq!(try_zip(&z, &dir), Ok(false), "{bad:?}");
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn qa_defect_zip_overlapping_entries_accepted() {
    // One local file, many directory entries pointing at it: the archive stays tiny, the output grows per entry.
    let data = vec![0u8; 1 << 16];
    let one = zip_bytes(&[("k.bin", &data)]);
    let cd_start = 30 + "k.bin".len() + data.len();
    let one_cd = one[cd_start..one.len() - 22].to_vec();
    let mut cd = Vec::new();
    for i in 0..2000 {
        let mut e = one_cd.clone();
        let name = format!("{i:05}");
        e.truncate(46);
        e[28..30].copy_from_slice(&(name.len() as u16).to_le_bytes());
        e.extend_from_slice(name.as_bytes());
        cd.extend_from_slice(&e);
    }
    let z = finish_zip(one[..cd_start].to_vec(), cd, 2000);
    let dir = scratch("overlap");
    let file = dir.join("t.zip");
    fs::write(&file, &z).unwrap();
    // A 131 MB unpack from a 200 KB zip must be refused before anything is written.
    assert!(Zip::open(&file).is_err(), "overlapping entries accepted ({} byte zip)", z.len());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn qa_defect_zip_stored_size_mismatch_accepted() {
    let mut z = zip_bytes(&[("a.txt", b"0123456789")]);
    // Directory says the file is 3 bytes; its data is 10.
    let cd = z.len() - 22 - (46 + 5);
    z[cd + 24..cd + 28].copy_from_slice(&3u32.to_le_bytes());
    let dir = scratch("sizes");
    assert_eq!(try_zip(&z, &dir), Ok(false));
    let _ = fs::remove_dir_all(&dir);
}

// ------------------------------------------------------------------ launcher file readers

#[test]
fn qa_json_deep_nesting_is_an_error() {
    let deep = "[".repeat(200_000);
    assert!(turbine_engine::json::parse(&deep).is_err());
}

/// Runs in a child process because a stack overflow ends the whole test program.
#[test]
fn qa_vdf_deep_nesting_child() {
    if std::env::var_os("TURBINE_QA_CHILD").is_none() {
        return;
    }
    let deep = "a{".repeat(1_000_000);
    let _ = turbine_engine::vdf::parse(&deep);
}

#[test]
fn qa_defect_vdf_deep_nesting_crashes() {
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["qa_vdf_deep_nesting_child", "--exact", "--test-threads=1"])
        .env("TURBINE_QA_CHILD", "1")
        .output()
        .unwrap();
    assert!(out.status.success(), "child crashed: {:?}", out.status);
}
