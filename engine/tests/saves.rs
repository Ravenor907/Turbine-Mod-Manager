//! Save backups before Play: copies only, newest ten kept, links never followed, and putting one back first keeps
//! the saves as they were.

mod support;

use std::fs;
use support::{game, TempDir};
use turbine_engine::gamedata::SavesFile;
use turbine_engine::games::{Runtime, Store as Launcher};
use turbine_engine::saves;

#[test]
fn folders_follow_the_runtime() {
    let t = TempDir::new("saves-folders");
    let dir = t.path("lib/Game");
    let pfx = t.path("pfx");
    fs::create_dir_all(pfx.join("drive_c/users/steamuser/Documents/My Games/G/Saves")).unwrap();
    fs::create_dir_all(t.path("home/.config/G")).unwrap();
    fs::create_dir_all(dir.join("save")).unwrap();
    let s = SavesFile { proton: vec!["Documents/My Games/G/Saves".into()], native: vec![".config/G".into()], game: vec!["save".into(), "missing".into()] };
    let mut g = game(Launcher::Steam, "1", "G", &dir, Some(pfx.clone()));
    g.runtime = Runtime::Windows;
    let f = saves::folders(&g, &s, &t.path("home"));
    assert_eq!(f, vec![pfx.join("drive_c/users/steamuser/Documents/My Games/G/Saves"), dir.join("save")]);
    g.runtime = Runtime::Native;
    assert_eq!(saves::folders(&g, &s, &t.path("home")), vec![t.path("home/.config/G"), dir.join("save")]);
}

#[test]
fn backup_copies_and_keeps_ten() {
    let t = TempDir::new("saves-keep");
    let save = t.path("save");
    fs::create_dir_all(save.join("slot1")).unwrap();
    fs::write(save.join("slot1/a.sav"), b"one").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink("/etc", save.join("link")).unwrap();
    let root = t.path("backups");
    assert!(saves::backup(&[], &root, 1).unwrap().is_none(), "no save folders, nothing kept");
    let s = saves::backup(std::slice::from_ref(&save), &root, 1_794_000_000).unwrap().unwrap();
    assert_eq!((s.files, s.bytes), (1, 3), "links are never followed");
    assert_eq!(fs::read(root.join(&s.id).join("0/slot1/a.sav")).unwrap(), b"one");
    for i in 1..=12 {
        saves::backup(std::slice::from_ref(&save), &root, 1_794_000_000 + i * 60).unwrap();
    }
    let all = saves::list(&root);
    assert_eq!(all.len(), saves::KEEP);
    assert_eq!(all[0].when, 1_794_000_000 + 12 * 60, "newest first");
    assert_eq!(fs::read(save.join("slot1/a.sav")).unwrap(), b"one", "the saves themselves are untouched");
}

#[test]
fn restore_keeps_the_saves_first() {
    let t = TempDir::new("saves-restore");
    let save = t.path("save");
    fs::create_dir_all(&save).unwrap();
    fs::write(save.join("a.sav"), b"old").unwrap();
    let root = t.path("backups");
    let old = saves::backup(std::slice::from_ref(&save), &root, 100).unwrap().unwrap();
    fs::write(save.join("a.sav"), b"new").unwrap();
    fs::write(save.join("b.sav"), b"only now").unwrap();
    let before = saves::restore(&old.id, std::slice::from_ref(&save), &root, 200).unwrap().unwrap();
    assert_eq!(fs::read(save.join("a.sav")).unwrap(), b"old");
    assert_eq!(fs::read(save.join("b.sav")).unwrap(), b"only now", "files the backup doesn't have stay");
    assert_eq!(fs::read(root.join(&before.id).join("0/a.sav")).unwrap(), b"new", "the saves as they were are kept first");
    assert!(saves::restore("../x", std::slice::from_ref(&save), &root, 300).is_err());
    assert!(saves::restore("nope", &[save], &root, 300).is_err());
}
