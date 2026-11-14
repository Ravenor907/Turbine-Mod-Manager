//! Turning mods off and Reset game files always give the game its own files back: a replaced settings file the
//! player edited, a file that can't be removed, a game writing into a mod's file, a stopped deploy whose mod is gone.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use turbine_engine::deploy::{apply, reset_from_backups, Paths};

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
