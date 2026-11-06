//! The Downloads watcher for Nexus's Slow download: made-up folders and clocks for the matching rules, and real
//! folders for the polling helpers.

mod support;

use std::cell::Cell;
use std::fs;
use std::path::PathBuf;
use support::TempDir;
use turbine_engine::nexus;
use turbine_engine::watch::{
    default_folders, parse_user_dirs, same_download_name, scan, wait, Confidence, ExpectedFile, Expectation, Seen, Watcher,
    WINDOW_SECS,
};

const T0: u64 = 1_800_000_000;

fn seen(name: &str, size: u64, modified: u64) -> Seen {
    Seen { path: PathBuf::from("/home/deck/Downloads").join(name), size, modified }
}

fn skyui() -> Expectation {
    let list = nexus::parse_files(&fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/nexus_files.json")).unwrap()).unwrap();
    Expectation::new("steam:489830", "skyrimspecialedition", 12604, T0).with_files(&list)
}

fn expect(files: &[(&str, u64)]) -> Expectation {
    let mut e = Expectation::new("steam:1091500", "cyberpunk2077", 107, T0);
    e.files = files
        .iter()
        .enumerate()
        .map(|(i, (n, s))| ExpectedFile { file_id: i as u64 + 1, file_name: n.to_string(), size_bytes: *s, version: "1".into() })
        .collect();
    e
}

#[test]
fn names_browsers_give_copies() {
    assert!(same_download_name("Mod-1-2.zip", "Mod-1-2.zip"));
    assert!(same_download_name("mod-1-2.ZIP", "Mod-1-2.zip"));
    assert!(same_download_name("Mod-1-2 (1).zip", "Mod-1-2.zip"));
    assert!(same_download_name("Mod-1-2(3).zip", "Mod-1-2.zip"));
    assert!(!same_download_name("Mod-1-2.7z", "Mod-1-2.zip"));
    assert!(!same_download_name("Mod-1-3.zip", "Mod-1-2.zip"));
    assert!(!same_download_name("Mod-1-2 (x).zip", "Mod-1-2.zip"));
    assert!(!same_download_name("anything.zip", ""));
}

#[test]
fn a_finished_download_matches_by_name_after_two_looks() {
    let mut w = Watcher::new(expect(&[("Cyber Engine Tweaks-107-1-36-0.zip", 23_606_784)]), vec![]);
    let name = "Cyber Engine Tweaks-107-1-36-0.zip";
    // Firefox: the empty final file and the .part next to it while it downloads.
    assert_eq!(w.step(&[seen(name, 0, T0 + 5), seen(&format!("{name}.part"), 1000, T0 + 5)]), None);
    assert_eq!(w.step(&[seen(name, 0, T0 + 6), seen(&format!("{name}.part"), 9000, T0 + 6)]), None);
    // Done: the .part is gone, but the size has only been seen once.
    assert_eq!(w.step(&[seen(name, 23_606_784, T0 + 7)]), None);
    let f = w.step(&[seen(name, 23_606_784, T0 + 7)]).unwrap();
    assert_eq!(f.confidence, Confidence::Name);
    assert_eq!(f.confidence.label(), "high");
    assert_eq!(f.file.unwrap().file_id, 1);
    assert_eq!(f.path, PathBuf::from("/home/deck/Downloads").join(name));
}

#[test]
fn chrome_and_other_browsers_partial_files_hold_it_back() {
    let name = "Mod.7z";
    for suffix in [".crdownload", ".download", ".PART"] {
        let mut w = Watcher::new(expect(&[(name, 5000)]), vec![]);
        let listing = [seen(name, 5000, T0 + 1), seen(&format!("{name}{suffix}"), 10, T0 + 1)];
        assert_eq!(w.step(&listing), None);
        assert_eq!(w.step(&listing), None, "{suffix} still there");
    }
}

#[test]
fn a_still_growing_file_waits() {
    let mut w = Watcher::new(expect(&[("Big.zip", 0)]), vec![]);
    assert_eq!(w.step(&[seen("Big.zip", 100, T0 + 1)]), None);
    assert_eq!(w.step(&[seen("Big.zip", 200, T0 + 2)]), None);
    assert_eq!(w.step(&[seen("Big.zip", 300, T0 + 3)]), None);
    assert!(w.step(&[seen("Big.zip", 300, T0 + 3)]).is_some());
}

#[test]
fn size_match_and_older_files_and_others_are_left_alone() {
    let mut w = Watcher::new(expect(&[("Real Name.zip", 4096)]), vec![]);
    let listing = [
        seen("renamed by the player.zip", 4096, T0 + 10),
        seen("Real Name.zip", 4096, T0 - 60),     // there before the page was opened
        seen("something else.zip", 777, T0 + 20), // another download
        seen("notes.txt", 4096, T0 + 20),         // not an archive
    ];
    assert_eq!(w.step(&listing), None);
    let f = w.step(&listing).unwrap();
    assert_eq!(f.confidence, Confidence::Size);
    assert!(f.path.ends_with("renamed by the player.zip"));
    // Nexus sizes in whole KB still match.
    let mut w = Watcher::new(expect(&[("x.zip", 2048)]), vec![]);
    let l = [seen("y.zip", 2048 + 1000, T0 + 1)];
    w.step(&l);
    assert_eq!(w.step(&l).unwrap().confidence, Confidence::Size);
    // When the file list is known, a download that matches nothing is never taken.
    let mut w = Watcher::new(expect(&[("x.zip", 2048)]), vec![]);
    let l = [seen("other.zip", 999_999, T0 + 1)];
    w.step(&l);
    assert_eq!(w.step(&l), None);
}

#[test]
fn name_wins_over_size_and_newer_wins_over_older() {
    let mut w = Watcher::new(expect(&[("A.zip", 10), ("B.zip", 20)]), vec![]);
    let l = [seen("whatever.zip", 10, T0 + 50), seen("B (1).zip", 20, T0 + 5), seen("B.zip", 20, T0 + 9)];
    w.step(&l);
    let f = w.step(&l).unwrap();
    assert_eq!(f.confidence, Confidence::Name);
    assert!(f.path.ends_with("B.zip"));
}

#[test]
fn without_a_file_list_the_newest_within_30_minutes_is_a_guess() {
    let mut w = Watcher::new(Expectation::new("gog:1423049311", "cyberpunk2077", 4198, T0), vec![]);
    let l = [seen("old.zip", 5, T0 + 100), seen("new.7z", 5, T0 + 200), seen("late.rar", 5, T0 + WINDOW_SECS + 1)];
    w.step(&l);
    let f = w.step(&l).unwrap();
    assert_eq!((f.confidence, f.file.clone()), (Confidence::Guess, None));
    assert_eq!(f.confidence.label(), "low");
    assert!(f.path.ends_with("new.7z"));
    assert!(w.expectation.expired_at(T0 + WINDOW_SECS + 1));
    assert!(!w.expectation.expired_at(T0 + WINDOW_SECS));
}

#[test]
fn expectation_from_nexus_file_list() {
    let e = skyui();
    assert_eq!(e.mod_id, 12604);
    assert!(!e.files.is_empty());
    assert!(e.files.iter().all(|f| !f.file_name.is_empty()));
    let first = e.files[0].clone();
    let mut w = Watcher::new(e, vec![]);
    let l = [seen(&first.file_name, first.size_bytes.max(1), T0 + 3)];
    w.step(&l);
    assert_eq!(w.step(&l).unwrap().file.unwrap().file_id, first.file_id);
}

#[test]
fn real_folders_poll_and_wait() {
    let t = TempDir::new("watch");
    let dl = t.path("Downloads");
    fs::create_dir_all(&dl).unwrap();
    fs::create_dir_all(dl.join("sub")).unwrap();
    t.write("Downloads/sub/inside.zip", b"not looked at");
    t.write("Downloads/Mod.zip", b"zip bytes");
    #[cfg(unix)]
    std::os::unix::fs::symlink(dl.join("Mod.zip"), dl.join("link.zip")).unwrap();
    let listing = scan(std::slice::from_ref(&dl));
    assert_eq!(listing.iter().map(|s| s.path.file_name().unwrap().to_string_lossy().into_owned()).collect::<Vec<_>>(), vec!["Mod.zip"]);

    let now = listing[0].modified;
    let mut w = Watcher::new(expect(&[("Mod.zip", 9)]), vec![dl.clone()]);
    w.expectation.started_at = now;
    let sleeps = Cell::new(0);
    let found = wait(&mut w, &|| now, &|_| sleeps.set(sleeps.get() + 1), &|| false).unwrap();
    assert_eq!(found.confidence, Confidence::Name);
    assert_eq!(sleeps.get(), 1, "one second between the two looks");
    assert_eq!(fs::read(dl.join("Mod.zip")).unwrap(), b"zip bytes", "the file is left as it was");

    // Stopped, or past the 30 minutes: nothing.
    let mut w = Watcher::new(expect(&[("Mod.zip", 9)]), vec![dl.clone()]);
    w.expectation.started_at = now;
    assert_eq!(wait(&mut w, &|| now, &|_| {}, &|| true), None);
    assert_eq!(wait(&mut w, &|| now + WINDOW_SECS + 5, &|_| {}, &|| false), None);
}

#[test]
fn downloads_folder_from_user_dirs() {
    let home = std::path::Path::new("/home/deck");
    let text = "# comment\nXDG_DESKTOP_DIR=\"$HOME/Desktop\"\nXDG_DOWNLOAD_DIR=\"$HOME/Stuff/Downloads\"\n";
    assert_eq!(parse_user_dirs(text, home), Some(PathBuf::from("/home/deck/Stuff/Downloads")));
    assert_eq!(parse_user_dirs("XDG_DOWNLOAD_DIR=\"/mnt/sd/dl\"\n", home), Some(PathBuf::from("/mnt/sd/dl")));
    assert_eq!(parse_user_dirs("XDG_DOWNLOAD_DIR=\"$HOME/\"\n", home), None);
    assert_eq!(parse_user_dirs("", home), None);
    let t = TempDir::new("userdirs");
    t.write(".config/user-dirs.dirs", b"XDG_DOWNLOAD_DIR=\"$HOME/Telechargements\"\n");
    assert_eq!(default_folders(&t.0), vec![t.path("Downloads"), t.path("Telechargements")]);
}
