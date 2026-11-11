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
    let same = saves::backup(std::slice::from_ref(&save), &root, 1_794_000_030).unwrap().unwrap();
    assert_eq!(same.id, s.id, "nothing changed, nothing copied");
    for i in 1..=12 {
        fs::write(save.join("slot1/a.sav"), format!("one {i}")).unwrap();
        saves::backup(std::slice::from_ref(&save), &root, 1_794_000_000 + i * 60).unwrap();
    }
    fs::write(save.join("slot1/a.sav"), b"one").unwrap();
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

#[test]
fn restoring_the_oldest_never_deletes_it() {
    // Ten snapshots, put back the oldest. Its pre-restore snapshot is the eleventh, and trimming
    // must not remove the one being put back.
    let t = TempDir::new("saves-oldest");
    let save = t.path("save");
    fs::create_dir_all(&save).unwrap();
    let root = t.path("bk");
    for i in 0..10u64 {
        fs::write(save.join("slot1.sav"), format!("v{i}")).unwrap();
        saves::backup(std::slice::from_ref(&save), &root, 1_000_000 + i * 100).unwrap();
    }
    let list = saves::list(&root);
    assert_eq!(list.len(), 10);
    let oldest = list.last().unwrap().id.clone();
    fs::write(save.join("slot1.sav"), "current").unwrap();
    saves::restore(&oldest, std::slice::from_ref(&save), &root, 2_000_000).unwrap();
    assert_eq!(fs::read_to_string(save.join("slot1.sav")).unwrap(), "v0", "the oldest backup really is back");
    assert!(root.join(&oldest).is_dir(), "the backup put back is still there");
    assert!(saves::list(&root).len() <= saves::KEEP + 1);
}

#[test]
fn restore_goes_to_the_folder_each_came_from() {
    // The backup was taken when only B existed; A appears later. B's saves must go back to B.
    let t = TempDir::new("saves-shift");
    let (a, b) = (t.path("A"), t.path("B"));
    fs::create_dir_all(&b).unwrap();
    fs::write(b.join("b.sav"), "B data").unwrap();
    let root = t.path("bk");
    let s = saves::backup(std::slice::from_ref(&b), &root, 1_000_000).unwrap().unwrap();
    assert_eq!(saves::sources(&root, &s.id).unwrap(), vec![b.clone()]);
    fs::create_dir_all(&a).unwrap();
    fs::write(b.join("b.sav"), "changed").unwrap();
    saves::restore(&s.id, &[a.clone(), b.clone()], &root, 1_000_100).unwrap();
    assert!(!a.join("b.sav").exists(), "A never gets B's saves");
    assert_eq!(fs::read_to_string(b.join("b.sav")).unwrap(), "B data");
    // A folder it came from that isn't a save folder now: refused, nothing changed.
    fs::write(b.join("b.sav"), "changed again").unwrap();
    assert!(saves::restore(&s.id, std::slice::from_ref(&a), &root, 1_000_200).is_err());
    assert_eq!(fs::read_to_string(b.join("b.sav")).unwrap(), "changed again");
}

#[cfg(unix)]
#[test]
fn a_failed_backup_leaves_nothing_behind() {
    // An unreadable file stops the copy; no half-made folder is left.
    use std::os::unix::fs::PermissionsExt;
    let t = TempDir::new("saves-part");
    let save = t.path("save");
    fs::create_dir_all(save.join("sub")).unwrap();
    fs::write(save.join("ok.sav"), "x").unwrap();
    fs::write(save.join("sub/locked.sav"), "y").unwrap();
    fs::set_permissions(save.join("sub/locked.sav"), fs::Permissions::from_mode(0o000)).unwrap();
    let root = t.path("bk");
    let readable = fs::read(save.join("sub/locked.sav")).is_ok(); // running as root reads anything
    let r = saves::backup(std::slice::from_ref(&save), &root, 1_000_000);
    fs::set_permissions(save.join("sub/locked.sav"), fs::Permissions::from_mode(0o644)).unwrap();
    if !readable {
        assert!(r.is_err());
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0, "no .part folder left");
    }
    // A leftover from a crash is cleared by the next backup.
    fs::create_dir_all(root.join(".19700101-000000.part/0")).unwrap();
    saves::backup(std::slice::from_ref(&save), &root, 1_000_100).unwrap();
    assert!(!root.join(".19700101-000000.part").exists());
}

#[test]
fn backups_check_space_size_and_count_first() {
    // No room, too big, too many files: refused with a plain reason, nothing copied, older
    // backups kept.
    let t = TempDir::new("saves-space");
    let save = t.path("save");
    fs::create_dir_all(&save).unwrap();
    fs::write(save.join("a.sav"), vec![7u8; 4096]).unwrap();
    let root = t.path("bk");
    let first = saves::backup_with(std::slice::from_ref(&save), &root, 100, None, &[]).unwrap().unwrap();
    fs::write(save.join("a.sav"), vec![8u8; 4096]).unwrap();
    let e = saves::backup_with(std::slice::from_ref(&save), &root, 200, Some(1000), &[]).unwrap_err();
    assert!(e.to_string().contains("not enough free space"), "{e}");
    assert_eq!(saves::list(&root).len(), 1);
    // Too many files: nothing is cut off, the backup is refused.
    let many = t.path("many");
    fs::create_dir_all(&many).unwrap();
    for i in 0..=saves::MAX_FILES {
        fs::write(many.join(format!("{i}")), "").unwrap();
    }
    let e = saves::backup(std::slice::from_ref(&many), &root, 300).unwrap_err();
    assert!(e.to_string().contains("too many files"), "{e}");
    assert_eq!(saves::list(&root), vec![saves::list(&root)[0].clone()]);
    assert_eq!(saves::list(&root)[0].id, first.id, "the older backup is kept");
    // Putting back with no room refuses before touching the saves.
    let e = saves::restore_with(&first.id, std::slice::from_ref(&save), &root, 400, Some(10)).unwrap_err();
    assert!(e.to_string().contains("not enough free space"), "{e}");
    assert_eq!(fs::read(save.join("a.sav")).unwrap(), vec![8u8; 4096]);
    // The player can still put it back, without keeping the saves as they are now.
    assert!(saves::couldnt_keep(&e));
    saves::restore_without_keeping(&first.id, std::slice::from_ref(&save), &root).unwrap();
    assert_eq!(fs::read(save.join("a.sav")).unwrap(), vec![7u8; 4096]);
}

#[test]
fn restore_makes_a_deleted_save_folder_again() {
    // The whole save folder is gone; the backup goes back into it.
    let t = TempDir::new("saves-gone");
    let save = t.path("Saves");
    fs::create_dir_all(&save).unwrap();
    fs::write(save.join("a.sav"), b"kept").unwrap();
    let root = t.path("bk");
    let s = saves::backup(std::slice::from_ref(&save), &root, 100).unwrap().unwrap();
    fs::remove_dir_all(&save).unwrap();
    let before = saves::restore(&s.id, std::slice::from_ref(&save), &root, 200).unwrap();
    assert!(before.is_none(), "nothing there to keep first");
    assert_eq!(fs::read(save.join("a.sav")).unwrap(), b"kept");
    // A place that isn't one of the game's save locations is still refused.
    fs::remove_dir_all(&save).unwrap();
    assert!(saves::restore(&s.id, &[t.path("Other")], &root, 300).is_err());
    assert!(!save.exists());
}
