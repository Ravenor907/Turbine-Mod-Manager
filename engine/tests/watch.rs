//! The Downloads watcher for Nexus's Slow download: made-up folders and clocks for the matching rules, and real
//! folders for the polling helpers.

mod support;

use std::cell::Cell;
use std::fs;
use std::path::PathBuf;
use support::TempDir;
use turbine_engine::nexus;
use turbine_engine::watch::{
    arrived_secs, catch_up_since, decide, default_folders, ignore_key, parse_user_dirs, same_download_name, scan, wait, Arrival, Basis,
    CatchState, Catcher, Confidence, Event, ExpectedFile, Expectation, Seen, Skip, Watcher, FIRST_RUN_SECS, MAX_IGNORED, WINDOW_SECS,
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

#[test]
fn one_downloads_folder_as_the_sandbox_sees_it() {
    use turbine_engine::watch::downloads_folder_from;
    let t = TempDir::new("dlfolder");
    // Flatpak's copy of the host's file sits in the app's own config folder.
    t.write("app-config/user-dirs.dirs", b"XDG_DOWNLOAD_DIR=\"$HOME/Telechargements\"\n");
    t.write(".config/user-dirs.dirs", b"XDG_DOWNLOAD_DIR=\"$HOME/Other\"\n");
    let configs = [t.path("app-config"), t.path(".config")];
    assert_eq!(downloads_folder_from(&t.0, None, &configs), t.path("Telechargements"));
    assert_eq!(downloads_folder_from(&t.0, Some("/mnt/sd/dl"), &configs), PathBuf::from("/mnt/sd/dl"));
    // A relative or home-only value is ignored.
    assert_eq!(downloads_folder_from(&t.0, Some("dl"), &configs), t.path("Telechargements"));
    assert_eq!(downloads_folder_from(&t.0, None, &[t.path(".config")]), t.path("Other"));
    assert_eq!(downloads_folder_from(&t.0, None, &[t.path("none")]), t.path("Downloads"));
}

// ------------------------------------------------------------------------------------------------ always-on catching

fn ready(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Ready(a) => Some(a.path.file_name().unwrap().to_string_lossy().into_owned()),
            _ => None,
        })
        .collect()
}

fn skips(events: &[Event]) -> Vec<(String, Skip)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Skipped { path, why } => Some((path.file_name().unwrap().to_string_lossy().into_owned(), *why)),
            _ => None,
        })
        .collect()
}

#[test]
fn file_time_is_the_later_of_modified_and_changed() {
    // A browser that keeps the server's date: modified in 2015, written into Downloads now.
    assert_eq!(arrived_secs(1_420_000_000, 1_800_000_000), 1_800_000_000);
    assert_eq!(arrived_secs(1_800_000_100, 1_800_000_000), 1_800_000_100);
    assert_eq!(arrived_secs(-5, -1), 0);
}

#[test]
fn real_file_with_an_old_modified_date_still_counts_as_new() {
    let t = TempDir::new("ctime");
    let p = t.write("Downloads/Old Date Mod.zip", b"zip bytes");
    let old = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000); // 2001
    fs::File::options().write(true).open(&p).unwrap().set_modified(old).unwrap();
    let meta = fs::metadata(&p).unwrap();
    assert_eq!(meta.modified().unwrap(), old, "the file really says 2001");
    let listing = scan(&[t.path("Downloads")]);
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    assert!(listing[0].modified + 60 >= now, "counted from when it landed, not the server's date: {}", listing[0].modified);

    // The catcher takes it even though Turbine started (and looked) after 2001.
    let mut c = Catcher::new(vec![t.path("Downloads")], now - 30, now - 30);
    assert!(ready(&c.poll()).is_empty(), "first look: size seen once");
    assert_eq!(ready(&c.poll()), vec!["Old Date Mod.zip"]);
    assert_eq!(fs::read(&p).unwrap(), b"zip bytes", "never touched");
}

#[test]
fn catcher_waits_for_finished_stable_archives_and_says_why_it_skips() {
    let mut c = Catcher::new(vec![], T0, T0);
    let a = "Cyber Engine Tweaks-107-1-36-0.zip";
    // Firefox: the empty final file and the .part beside it.
    let e = c.step(&[seen(a, 0, T0 + 5), seen(&format!("{a}.part"), 1000, T0 + 5), seen("notes.pdf", 10, T0 + 5), seen("old.zip", 10, T0 - 100)]);
    assert_eq!(e[0], Event::Downloading { name: a.into(), size: 1000 });
    assert_eq!(skips(&e), vec![(a.to_string(), Skip::Unfinished), ("notes.pdf".to_string(), Skip::NotArchive)]);
    assert!(ready(&e).is_empty());
    // Said once only.
    let e = c.step(&[seen(a, 0, T0 + 6), seen(&format!("{a}.part"), 9000, T0 + 6), seen("notes.pdf", 10, T0 + 5)]);
    assert!(skips(&e).is_empty());
    // Done, but its size is seen for the first time at this size: still being written.
    let e = c.step(&[seen(a, 23_000, T0 + 7)]);
    assert_eq!(skips(&e), vec![(a.to_string(), Skip::Growing)]);
    let e = c.step(&[seen(a, 23_000, T0 + 7)]);
    assert_eq!(e, vec![Event::Ready(Arrival { path: PathBuf::from("/home/deck/Downloads").join(a), size: 23_000, arrived: T0 + 7, catch_up: false })]);
    // Ready only once.
    assert!(c.step(&[seen(a, 23_000, T0 + 7)]).is_empty());
    // Chrome's and other browsers' unfinished files; Chromium's "Unconfirmed" name.
    let e = c.step(&[seen("B.7z", 50, T0 + 9), seen("B.7z.crdownload", 50, T0 + 9), seen("Unconfirmed 4411.crdownload", 70, T0 + 10)]);
    assert_eq!(e[0], Event::Downloading { name: "a file".into(), size: 70 });
    assert_eq!(skips(&e), vec![("B.7z".to_string(), Skip::Unfinished)]);
    let e = c.step(&[seen("C.rar", 0, T0 + 9)]);
    assert_eq!(skips(&e), vec![("C.rar".to_string(), Skip::Empty)]);
    assert!(c.step(&[seen("C.rar", 0, T0 + 9)]).is_empty());
}

#[test]
fn catcher_needs_more_looks_when_asked() {
    let mut c = Catcher::new(vec![], T0, T0);
    c.stable_looks = 3;
    let l = [seen("Mod.zip", 9, T0 + 1)];
    assert!(ready(&c.step(&l)).is_empty());
    assert!(ready(&c.step(&l)).is_empty());
    assert!(ready(&c.step(&l)).is_empty());
    assert_eq!(ready(&c.step(&l)), vec!["Mod.zip"]);
}

#[test]
fn ignored_files_are_never_ready_again_and_new_contents_are() {
    let mut c = Catcher::new(vec![], T0, T0);
    let p = PathBuf::from("/home/deck/Downloads/Wallpaper Pack.zip");
    c.ignore(&ignore_key(&p, 400));
    let l = [seen("Wallpaper Pack.zip", 400, T0 + 2)];
    c.step(&l);
    let e = c.step(&l);
    assert_eq!(skips(&e), vec![("Wallpaper Pack.zip".to_string(), Skip::Ignored)]);
    assert!(ready(&e).is_empty());
    assert!(c.step(&l).is_empty(), "said once");
    // Downloaded again with other contents (another size): asked about again.
    let l = [seen("Wallpaper Pack.zip", 500, T0 + 9)];
    c.step(&l);
    assert_eq!(ready(&c.step(&l)), vec!["Wallpaper Pack.zip"]);

    // The saved list keeps the newest MAX_IGNORED, and reads back.
    let mut st = CatchState { last_seen: T0, ignored: vec![], pending: vec!["/home/deck/Downloads/Waiting.zip".into()] };
    for i in 0..MAX_IGNORED + 3 {
        st.ignore(&ignore_key(&PathBuf::from(format!("/d/{i}.zip")), 1));
    }
    st.ignore(&ignore_key(&PathBuf::from("/d/5.zip"), 1));
    assert_eq!(st.ignored.len(), MAX_IGNORED);
    assert_eq!(st.ignored.last().unwrap(), "1|/d/5.zip");
    assert!(!st.ignored.contains(&"1|/d/0.zip".to_string()));
    let back = CatchState::parse(&st.to_json()).unwrap();
    assert_eq!(back, st);
    assert_eq!(CatchState::parse("{}"), None);
    assert_eq!(CatchState::parse("not json"), None);

    let t = TempDir::new("catchstate");
    let path = t.path("turbine-mod-manager/catch.json");
    assert_eq!(CatchState::load(&path), None);
    st.save(&path).unwrap();
    assert_eq!(CatchState::load(&path), Some(st));
}

#[test]
fn catch_up_window_and_catch_up_files() {
    // First run: the last 24 hours, and Turbine asks.
    assert_eq!(catch_up_since(None, T0), (T0 - FIRST_RUN_SECS, true));
    assert_eq!(catch_up_since(Some(&CatchState::default()), T0), (T0 - FIRST_RUN_SECS, true));
    // Later: from when it last looked (2 seconds of slack), never in the future.
    let st = CatchState { last_seen: T0 - 3600, ..Default::default() };
    assert_eq!(catch_up_since(Some(&st), T0), (T0 - 3602, false));
    let ahead = CatchState { last_seen: T0 + 99, ..Default::default() };
    assert_eq!(catch_up_since(Some(&ahead), T0), (T0 - 2, false));

    // Started at T0, last looked an hour before: files from that hour are catch-up files, older ones are left alone.
    let (since, _) = catch_up_since(Some(&st), T0);
    let mut c = Catcher::new(vec![], since, T0);
    let l = [seen("Before.zip", 5, T0 - 7200), seen("While Closed.zip", 5, T0 - 60), seen("Now.zip", 5, T0 + 4)];
    c.step(&l);
    let e = c.step(&l);
    let arrivals: Vec<(String, bool)> = e
        .iter()
        .filter_map(|e| match e {
            Event::Ready(a) => Some((a.path.file_name().unwrap().to_string_lossy().into_owned(), a.catch_up)),
            _ => None,
        })
        .collect();
    assert_eq!(arrivals, vec![("While Closed.zip".to_string(), true), ("Now.zip".to_string(), false)]);

    // A question left unanswered last time is asked again, however old the file is.
    let mut c = Catcher::new(vec![], since, T0);
    c.ask_again(&PathBuf::from("/home/deck/Downloads/Before.zip"));
    c.step(&l);
    assert_eq!(ready(&c.step(&l)), vec!["Before.zip", "While Closed.zip", "Now.zip"]);
}

#[test]
fn deciding_the_game_goes_in_order() {
    let d = decide(Some("steam:1091500"), Some("steam:489830"), Some(("steam:292030", Confidence::Name)), Some("steam:892970"), false);
    assert_eq!((d.game.as_deref(), d.basis, d.sure), (Some("steam:1091500"), Basis::Helper, true));
    let d = decide(None, Some("steam:489830"), Some(("steam:292030", Confidence::Name)), Some("steam:892970"), false);
    assert_eq!((d.game.as_deref(), d.basis, d.sure), (Some("steam:489830"), Basis::Md5, true));
    let d = decide(None, None, Some(("steam:292030", Confidence::Size)), Some("steam:892970"), false);
    assert_eq!((d.game.as_deref(), d.basis, d.sure), (Some("steam:292030"), Basis::Page(Confidence::Size), true));
    let d = decide(None, None, Some(("steam:292030", Confidence::Guess)), Some("steam:892970"), false);
    assert_eq!((d.game.as_deref(), d.sure), (Some("steam:292030"), false), "a guess asks");
    let d = decide(None, None, None, Some("steam:892970"), false);
    assert_eq!((d.game.as_deref(), d.basis, d.sure), (Some("steam:892970"), Basis::LastGame, false));
    let d = decide(None, None, None, None, false);
    assert_eq!((d.game, d.basis, d.sure), (None, Basis::Nothing, false));
    // The first run's catch-up always asks, even when sure.
    let d = decide(Some("steam:1091500"), None, None, None, true);
    assert_eq!((d.game.as_deref(), d.sure), (Some("steam:1091500"), false));
    for b in [Basis::Helper, Basis::Md5, Basis::Page(Confidence::Guess), Basis::LastGame, Basis::Nothing] {
        assert!(!b.words().is_empty());
    }
}

#[test]
fn expectation_matches_a_single_file() {
    let e = expect(&[("Mod-1-2.zip", 5000)]);
    assert_eq!(e.match_file("Mod-1-2 (1).zip", 1).unwrap().1, Confidence::Name);
    assert_eq!(e.match_file("renamed.zip", 5100).unwrap().1, Confidence::Size);
    assert_eq!(e.match_file("other.zip", 99_999), None);
    let none = Expectation::new("steam:1", "x", 1, T0);
    assert_eq!(none.match_file("anything.zip", 3), Some((None, Confidence::Guess)));
}

#[test]
fn real_folders_with_the_catcher() {
    let t = TempDir::new("catcher");
    let dl = t.path("Downloads");
    t.write("Downloads/Real Mod.7z", b"7z bytes");
    t.write("Downloads/Real Mod 2.zip.part", b"partial");
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let mut c = Catcher::new(vec![dl.clone()], now - 10, now + 100);
    let e = c.poll();
    assert!(e.contains(&Event::Downloading { name: "Real Mod 2.zip".into(), size: 7 }));
    let e = c.poll();
    let a = e.iter().find_map(|e| if let Event::Ready(a) = e { Some(a.clone()) } else { None }).unwrap();
    assert_eq!(a.path, dl.join("Real Mod 7z").with_file_name("Real Mod.7z"));
    assert!(a.catch_up, "arrived before this Turbine started");
}
