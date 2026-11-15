//! Turning mods off and Reset game files always give the game its own files back: a replaced settings file the
//! player edited, a file that can't be removed, a game writing into a mod's file, a stopped deploy whose mod is gone.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use turbine_engine::deploy::{apply, recover, reset_from_backups, reset_with_staged, Paths, Record};

fn write(p: &Path, text: &str) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

fn read(p: &Path) -> String {
    fs::read_to_string(p).unwrap_or_else(|e| format!("<missing: {e}>"))
}

struct World {
    root: PathBuf,
    paths: Paths,
}

impl Drop for World {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn world(tag: &str) -> World {
    let root = std::env::temp_dir().join(format!("turbine-undo-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("game")).unwrap();
    let paths = Paths { game: root.join("game"), backups: root.join("backups"), record: root.join("rec/record.json"), plan: root.join("rec/plan.json") };
    World { root, paths }
}

impl World {
    fn g(&self, rel: &str) -> PathBuf {
        self.paths.game.join(rel)
    }

    /// Stages one file for a mod and returns (rel, staged path).
    fn stage(&self, m: &str, rel: &str, text: &str) -> (String, PathBuf) {
        let p = self.root.join("staging").join(m).join(rel);
        write(&p, text);
        (rel.to_string(), p)
    }
}

/// Something writes the file in place with a newer time, as a game or its launcher would.
fn edit(p: &Path, text: &str) {
    std::thread::sleep(std::time::Duration::from_millis(20));
    fs::write(p, text).unwrap();
    let f = fs::OpenOptions::new().write(true).open(p).unwrap();
    f.set_modified(std::time::SystemTime::now()).unwrap();
}

/// Every file under `d` (backups included), so a test can say no copy of a file is stranded anywhere.
fn files_under(d: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir(d) {
        for e in rd.flatten() {
            let p = e.path();
            if fs::symlink_metadata(&p).map(|m| m.is_dir()).unwrap_or(false) {
                out.extend(files_under(&p));
            } else {
                out.push(p);
            }
        }
    }
    out
}

#[test]
fn replaced_settings_file_edited_then_applied_again_gives_the_original_back() {
    let w = world("ini-again");
    write(&w.g("Engine.ini"), "ORIGINAL");
    let a = w.stage("A", "Engine.ini", "MOD");
    apply(&w.paths, std::slice::from_ref(&a), None).unwrap();
    assert_eq!(read(&w.g("Engine.ini")), "MOD");
    edit(&w.g("Engine.ini"), "MOD + player edit");
    // Play again: the player's edit stays while the mod is in.
    let r = apply(&w.paths, std::slice::from_ref(&a), None).unwrap();
    assert!(r.put_back.is_empty(), "{r:?}");
    assert_eq!(read(&w.g("Engine.ini")), "MOD + player edit");
    // Mod off: the game's own file comes back and nothing is left in backups.
    let r = apply(&w.paths, &[], None).unwrap();
    assert_eq!(read(&w.g("Engine.ini")), "ORIGINAL");
    assert_eq!(r.player_changes_saved, vec!["Engine.ini".to_string()]);
    assert!(files_under(&w.paths.backups).is_empty(), "{:?}", files_under(&w.paths.backups));
    // Mod on again: the player's edit goes back in, over the game's file, which is backed up again.
    apply(&w.paths, std::slice::from_ref(&a), None).unwrap();
    assert_eq!(read(&w.g("Engine.ini")), "MOD + player edit");
    let r = reset_from_backups(&w.paths).unwrap();
    assert!(r.left_in_backups.is_empty(), "{r:?}");
    assert_eq!(read(&w.g("Engine.ini")), "ORIGINAL");
}

#[test]
fn replaced_settings_file_edited_then_mod_off_or_reset_gives_the_original_back() {
    for reset in [false, true] {
        let w = world(if reset { "ini-reset" } else { "ini-off" });
        write(&w.g("bin/config/base/Engine.ini"), "ORIGINAL");
        let a = w.stage("A", "bin/config/base/Engine.ini", "MOD");
        apply(&w.paths, &[a], None).unwrap();
        edit(&w.g("bin/config/base/Engine.ini"), "MOD + edit");
        let r = if reset { reset_from_backups(&w.paths).unwrap() } else { apply(&w.paths, &[], None).unwrap() };
        assert_eq!(read(&w.g("bin/config/base/Engine.ini")), "ORIGINAL", "reset {reset}: {r:?}");
        assert!(r.left_in_backups.is_empty() && r.changed_by_others.is_empty(), "{r:?}");
        assert!(files_under(&w.paths.backups).is_empty());
        // The player's edit is kept on the side, not lost.
        let kept: Vec<String> = files_under(&w.root.join("rec/player-changes")).iter().map(|p| read(p)).collect();
        assert_eq!(kept, vec!["MOD + edit".to_string()]);
    }
}

#[test]
fn replaced_settings_file_edited_then_another_mod_takes_its_place() {
    let w = world("ini-switch");
    write(&w.g("Engine.ini"), "ORIGINAL");
    let a = w.stage("A", "Engine.ini", "A");
    let b = w.stage("B", "Engine.ini", "B");
    apply(&w.paths, &[a], None).unwrap();
    edit(&w.g("Engine.ini"), "A + edit");
    apply(&w.paths, &[b], None).unwrap();
    assert_eq!(read(&w.g("Engine.ini")), "B");
    apply(&w.paths, &[], None).unwrap();
    assert_eq!(read(&w.g("Engine.ini")), "ORIGINAL");
    assert!(files_under(&w.paths.backups).is_empty());
}

/// The game writes into a mod's file in place (a cache, a script, a plugin a tool cleans): the mod's own copy stays as
/// it came. When the mod goes, what was written leaves the game's folder too and is kept on the side.
#[test]
fn a_game_writing_into_a_mod_file_never_changes_the_mods_copy() {
    for reset in [false, true] {
        let w = world(if reset { "inplace-reset" } else { "inplace" });
        write(&w.g("Data/A.esp"), "ORIGINAL ESP");
        write(&w.g("Data/own.dat"), "GAME'S OWN");
        let files = vec![w.stage("A", "Data/cache.dat", "MOD DATA"), w.stage("A", "gamedata/scripts/a.ltx", "MOD LTX"), w.stage("A", "Data/A.esp", "MOD ESP")];
        apply(&w.paths, &files, None).unwrap();
        for (rel, _) in &files {
            let mut f = fs::OpenOptions::new().write(true).truncate(true).open(w.g(rel)).unwrap();
            std::io::Write::write_all(&mut f, format!("GAME WROTE {rel}").as_bytes()).unwrap();
        }
        // A file the game made by itself next to them, which no record lists.
        write(&w.g("Data/made.dat"), "GAME MADE");
        assert_eq!(read(&files[0].1), "MOD DATA");
        assert_eq!(read(&files[1].1), "MOD LTX");
        assert_eq!(read(&files[2].1), "MOD ESP");
        let r = if reset { reset_from_backups(&w.paths).unwrap() } else { apply(&w.paths, &[], None).unwrap() };
        assert_eq!(r.moved_aside.len(), 3, "{r:?}");
        assert!(r.changed_by_others.is_empty() && r.left_in_backups.is_empty(), "{r:?}");
        // The game's folder is as it was before the mod, plus what the game made by itself.
        let mut left: Vec<String> = files_under(&w.paths.game).iter().map(|p| p.strip_prefix(&w.paths.game).unwrap().to_string_lossy().into_owned()).collect();
        left.sort();
        assert_eq!(left, vec!["Data/A.esp", "Data/made.dat", "Data/own.dat"]);
        assert_eq!(read(&w.g("Data/A.esp")), "ORIGINAL ESP");
        assert_eq!(read(&w.g("Data/own.dat")), "GAME'S OWN");
        assert_eq!(read(&w.g("Data/made.dat")), "GAME MADE");
        assert!(!w.g("gamedata").exists(), "folders Turbine made go when empty");
        assert!(files_under(&w.paths.backups).is_empty());
        // Nothing is lost: what the game wrote is kept, and the mod's own files are as they came.
        let mut kept: Vec<String> = files_under(&w.root.join("rec/player-changes")).iter().map(|p| read(p)).collect();
        kept.sort();
        assert_eq!(kept, vec!["GAME WROTE Data/A.esp", "GAME WROTE Data/cache.dat", "GAME WROTE gamedata/scripts/a.ltx"]);
        assert_eq!(read(&files[0].1), "MOD DATA");
        // Mod on again: the mod's own files go in, not the kept ones.
        let r = apply(&w.paths, &files, None).unwrap();
        assert!(r.player_changes_restored.is_empty(), "{r:?}");
        assert_eq!(read(&w.g("Data/cache.dat")), "MOD DATA");
        assert_eq!(read(&w.g("Data/A.esp")), "MOD ESP");
        apply(&w.paths, &[], None).unwrap();
        assert_eq!(read(&w.g("Data/A.esp")), "ORIGINAL ESP");
    }
}

/// A file the record doesn't list is never touched, whatever it looks like: here one with a mod file's name, in the
/// game's folder after the record forgot it.
#[test]
fn a_file_the_record_does_not_list_is_never_moved() {
    let w = world("unlisted");
    let a = w.stage("A", "Data/cache.dat", "MOD DATA");
    let b = w.stage("A", "Data/b.dat", "MOD B");
    apply(&w.paths, &[a, b], None).unwrap();
    // The record loses Data/cache.dat (as if Turbine had never placed it).
    let mut rec = Record::load(&w.paths.record).unwrap();
    rec.files.retain(|p| p.rel != "Data/cache.dat");
    rec.save(&w.paths.record).unwrap();
    edit(&w.g("Data/cache.dat"), "CHANGED");
    edit(&w.g("Data/b.dat"), "CHANGED B");
    let r = apply(&w.paths, &[], None).unwrap();
    assert_eq!(r.moved_aside, vec!["Data/b.dat".to_string()]);
    assert_eq!(read(&w.g("Data/cache.dat")), "CHANGED");
    assert!(!w.g("Data/b.dat").exists());
    let r = reset_from_backups(&w.paths).unwrap();
    assert!(r.moved_aside.is_empty(), "{r:?}");
    assert_eq!(read(&w.g("Data/cache.dat")), "CHANGED");
}

/// Something makes a file of its own where a mod's file was (a game update does): that file is not Turbine's and
/// stays, when the mod goes and on Reset.
#[test]
fn a_file_made_anew_where_a_mod_file_was_is_left_alone() {
    for reset in [false, true] {
        let w = world(if reset { "anew-reset" } else { "anew" });
        let a = w.stage("A", "Data/new.bin", "MOD");
        apply(&w.paths, &[a], None).unwrap();
        fs::remove_file(w.g("Data/new.bin")).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        write(&w.g("Data/new.bin"), "GAME UPDATE");
        let r = if reset { reset_from_backups(&w.paths).unwrap() } else { apply(&w.paths, &[], None).unwrap() };
        assert_eq!(r.changed_by_others, vec!["Data/new.bin".to_string()], "{r:?}");
        assert!(r.moved_aside.is_empty(), "{r:?}");
        assert_eq!(read(&w.g("Data/new.bin")), "GAME UPDATE");
    }
}

/// Packed archives and textures are still linked (a Deck's drive is small), and one written into anyway is still
/// known as the mod's file: it comes out with the mod and is never taken for a game file.
#[test]
fn a_linked_archive_written_into_is_still_the_mods_file() {
    use std::os::unix::fs::MetadataExt;
    let w = world("linked");
    let a = w.stage("A", "Data/A.bsa", "MOD ARCHIVE");
    apply(&w.paths, std::slice::from_ref(&a), None).unwrap();
    assert_eq!(fs::metadata(w.g("Data/A.bsa")).unwrap().ino(), fs::metadata(&a.1).unwrap().ino(), "archives are linked");
    edit(&w.g("Data/A.bsa"), "WRITTEN THROUGH");
    let r = apply(&w.paths, std::slice::from_ref(&a), None).unwrap();
    assert!(r.put_back.is_empty() && r.added == 0, "{r:?}");
    apply(&w.paths, &[], None).unwrap();
    assert!(!w.g("Data/A.bsa").exists());
    assert!(files_under(&w.paths.backups).is_empty());
}

/// A deploy stopped (power out) before its file went in; the mod's own file is gone since and the game made a file of
/// the same name. Finishing the stopped deploy never claims that file, so turning mods off never deletes it.
#[test]
fn a_stopped_deploy_whose_mod_is_gone_never_claims_a_game_file() {
    let w = world("recover-gone");
    let a = w.stage("A", "Data/settings_cache.dat", "A");
    assert!(apply(&w.paths, std::slice::from_ref(&a), Some(0)).is_err());
    fs::remove_file(&a.1).unwrap();
    write(&w.g("Data/settings_cache.dat"), "GAME MADE THIS");
    recover(&w.paths).unwrap();
    assert!(Record::load(&w.paths.record).unwrap().files.is_empty());
    apply(&w.paths, &[], None).unwrap();
    assert_eq!(read(&w.g("Data/settings_cache.dat")), "GAME MADE THIS");
}

/// A mod that may not replace the game's files is kept off them whatever letters their names use.
#[test]
fn a_mod_that_may_not_replace_game_files_skips_them_in_any_letters() {
    use turbine_engine::deploy::{apply_guarded, Wanted};
    let w = world("unicode-case");
    write(&w.g("Data/Ärger.esp"), "GAME");
    write(&w.g("Data/Plain.esp"), "GAME");
    write(&w.g("Данные/Мод.esp"), "GAME");
    let wanted: Vec<Wanted> = [w.stage("A", "data/ärger.esp", "MOD"), w.stage("A", "data/plain.esp", "MOD"), w.stage("A", "данные/мод.esp", "MOD")]
        .into_iter()
        .map(|(rel, source)| Wanted { rel, source, may_replace: false })
        .collect();
    let r = apply_guarded(&w.paths, &wanted, None).unwrap();
    assert_eq!(r.skipped_existing.len(), 3, "{r:?}");
    assert_eq!(read(&w.g("Data/Ärger.esp")), "GAME");
    assert_eq!(read(&w.g("Данные/Мод.esp")), "GAME");
    assert!(files_under(&w.paths.backups).is_empty());
}

#[test]
fn finding_a_game_file_ignores_case_in_any_letters() {
    let w = world("find-ci");
    write(&w.g("Данные/Ärger.esp"), "GAME");
    assert_eq!(turbine_engine::rules::find_ci(&w.paths.game, "данные/äRGER.ESP"), Some(w.g("Данные/Ärger.esp")));
    assert_eq!(turbine_engine::rules::find_ci(&w.paths.game, "данные/other.esp"), None);
}

/// A folder in the game that can't be written to (an SD card gone read-only, say) holds one of the mod's files.
fn stuck_world(tag: &str) -> (World, Vec<(String, PathBuf)>) {
    let w = world(tag);
    fs::create_dir_all(w.g("Aaa")).unwrap();
    write(&w.g("Data/x.esp"), "ORIGINAL X");
    let files = vec![w.stage("A", "Aaa/new.dll", "A dll"), w.stage("A", "Data/x.esp", "A x"), w.stage("A", "Zzz/z.txt", "A z")];
    apply(&w.paths, &files, None).unwrap();
    fs::set_permissions(w.g("Aaa"), fs::Permissions::from_mode(0o555)).unwrap();
    (w, files)
}

fn unstick(w: &World) {
    fs::set_permissions(w.g("Aaa"), fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn one_file_that_cannot_go_does_not_stop_play_without_mods() {
    let (w, _) = stuck_world("stuck-off");
    let msg = apply(&w.paths, &[], None).unwrap_err().to_string();
    assert!(msg.contains("Aaa/new.dll") && !msg.contains("x.esp"), "{msg}");
    // Everything else went back.
    assert_eq!(read(&w.g("Data/x.esp")), "ORIGINAL X");
    assert!(!w.g("Zzz").exists());
    // The next try, once the folder can be written to, finishes the rest.
    unstick(&w);
    apply(&w.paths, &[], None).unwrap();
    assert!(!w.g("Aaa/new.dll").exists());
    assert!(files_under(&w.paths.backups).is_empty());
    assert!(!w.paths.plan.exists());
}

#[test]
fn one_file_that_cannot_go_does_not_stop_reset() {
    let (w, _) = stuck_world("stuck-reset");
    let msg = reset_from_backups(&w.paths).unwrap_err().to_string();
    assert!(msg.contains("Aaa/new.dll"), "{msg}");
    assert_eq!(read(&w.g("Data/x.esp")), "ORIGINAL X");
    assert!(!w.g("Zzz").exists());
    unstick(&w);
    let r = reset_from_backups(&w.paths).unwrap();
    assert_eq!(r.removed, 1, "{r:?}");
    assert!(!w.g("Aaa/new.dll").exists());
}

#[test]
fn one_file_that_cannot_go_while_switching_mods() {
    let (w, files) = stuck_world("stuck-switch");
    // The mod's dll goes, its other files stay, and another mod's file goes in.
    let b = w.stage("B", "Data/b.esp", "B");
    let msg = apply(&w.paths, &[files[1].clone(), files[2].clone(), b], None).unwrap_err().to_string();
    assert!(msg.contains("Aaa/new.dll"), "{msg}");
    assert_eq!(read(&w.g("Data/b.esp")), "B");
    unstick(&w);
    // The stopped plan is finished first, then everything goes back.
    apply(&w.paths, &[], None).unwrap();
    assert!(!w.g("Aaa/new.dll").exists() && !w.g("Data/b.esp").exists());
    assert_eq!(read(&w.g("Data/x.esp")), "ORIGINAL X");
}

/// The record and its second copy are both gone. Reset tells the mods' files by their own copies.
#[test]
fn reset_without_any_record_uses_the_mods_own_files() {
    let w = world("no-record");
    write(&w.g("Data/x.esp"), "ORIGINAL X");
    write(&w.g("Data/same.esp"), "SAME BYTES");
    write(&w.g("Data/Big.bsa"), "ORIGINAL ARCHIVE");
    let files = vec![
        w.stage("A", "data/x.esp", "A x"),
        // A mod file that is the same as the game's own: the game's comes back and stays.
        w.stage("A", "Data/same.esp", "SAME BYTES"),
        w.stage("A", "Data/Big.bsa", "A archive"),
        w.stage("A", "Data/Textures/a.dds", "A texture"),
        w.stage("A", "Mods/A/a.dll", "A dll"),
    ];
    // Another mod has a file for the same path; the one in the game is the first mod's.
    let other = w.stage("B", "Data/x.esp", "B x");
    apply(&w.paths, &files, None).unwrap();
    assert_eq!(read(&w.g("Data/x.esp")), "A x");
    fs::remove_file(&w.paths.record).unwrap();
    fs::remove_file(w.root.join("rec/record.json.prev")).unwrap();
    // A file with a mod file's name but not its bytes, and one no mod has: neither is Turbine's to take.
    write(&w.g("Mods/A/b.dll"), "not the mod's");
    let lookalike = w.stage("B", "Mods/A/b.dll", "B dll");
    write(&w.g("Data/save.dat"), "GAME MADE");

    let mut staged = files.clone();
    staged.push(other);
    staged.push(lookalike);
    let r = reset_with_staged(&w.paths, &staged).unwrap();
    assert!(r.left_in_backups.is_empty() && r.moved_aside.is_empty() && r.changed_by_others.is_empty(), "{r:?}");
    assert_eq!(r.removed, 5, "{r:?}");
    let mut left: Vec<(String, String)> = files_under(&w.paths.game).iter().map(|p| (p.strip_prefix(&w.paths.game).unwrap().to_string_lossy().into_owned(), read(p))).collect();
    left.sort();
    let want = [("Data/Big.bsa", "ORIGINAL ARCHIVE"), ("Data/same.esp", "SAME BYTES"), ("Data/save.dat", "GAME MADE"), ("Data/x.esp", "ORIGINAL X"), ("Mods/A/b.dll", "not the mod's")];
    assert_eq!(left, want.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect::<Vec<_>>());
    assert!(!w.g("Data/Textures").exists(), "a folder left empty goes");
    assert!(files_under(&w.paths.backups).is_empty());
    // The mods' own files are all still there.
    assert_eq!(read(&files[0].1), "A x");
    assert_eq!(read(&files[3].1), "A texture");
    // Again changes nothing.
    let r = reset_with_staged(&w.paths, &staged).unwrap();
    assert_eq!(r.removed, 0, "{r:?}");
    assert_eq!(read(&w.g("Data/same.esp")), "SAME BYTES");
}

/// With a record, a place where something else made a file anew over a mod's file keeps that file on Reset (the
/// game's older file waits in the backup folder and is said so), mods' files known or not.
#[test]
fn reset_with_a_record_leaves_a_file_made_anew() {
    let w = world("anew-backup");
    write(&w.g("Data/x.esp"), "ORIGINAL X");
    let a = w.stage("A", "Data/x.esp", "A x");
    apply(&w.paths, std::slice::from_ref(&a), None).unwrap();
    fs::remove_file(w.g("Data/x.esp")).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    write(&w.g("Data/x.esp"), "GAME UPDATE");
    let r = reset_with_staged(&w.paths, &[a]).unwrap();
    assert_eq!(r.left_in_backups, vec!["Data/x.esp".to_string()], "{r:?}");
    assert_eq!(read(&w.g("Data/x.esp")), "GAME UPDATE");
}
