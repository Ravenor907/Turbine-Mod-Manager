//! Opening archives: zip with Turbine's own reader, 7z and rar through bsdtar (a stand-in here), always checked.

mod support;

use std::fs;
use support::{write_zip, FakeTar, TempDir, RAR, SEVEN_Z};
use turbine_engine::archives::{self, check_listing, detect, parse_listing, unpack_with, Format, Listed};
use turbine_engine::zip::Limits;

#[test]
fn formats_come_from_the_first_bytes() {
    let t = TempDir::new("fmt");
    let z = t.path("a.7z");
    write_zip(&z, &[("a.txt", b"x")]);
    assert_eq!(detect(&z).unwrap(), Some(Format::Zip), "a zip named .7z is still a zip");
    assert_eq!(detect(&t.write("b.zip", SEVEN_Z)).unwrap(), Some(Format::SevenZip));
    assert_eq!(detect(&t.write("c.rar", RAR)).unwrap(), Some(Format::Rar));
    assert_eq!(detect(&t.write("d.zip", b"hello")).unwrap(), None);
    assert_eq!(detect(&t.write("e.zip", b"")).unwrap(), None);
    assert!(archives::is_archive_name("Mod.RAR") && archives::is_archive_name("x.7z") && !archives::is_archive_name("x.tar"));
}

#[test]
fn reads_bsdtar_listings() {
    let text = "drwxr-xr-x  0 0      0           0 Jan  1  2024 Data/\n\
                -rw-r--r--  0 0      0        1234 Mar 15 12:30 Data/My Mod  Name.esp\n\
                -rw-r--r--  0 root   root        5 Dec 31  1999 Data/caf\\303\\251\\\\x.txt\n";
    let l = parse_listing(text).unwrap();
    assert_eq!(l.len(), 3);
    assert!(l[0].is_dir);
    assert_eq!(l[1], Listed { name: "Data/My Mod  Name.esp".into(), is_dir: false, size: 1234 });
    assert_eq!(l[2].name, "Data/caf\u{e9}\\x.txt");
    // Links, hard links and special files are refused.
    for bad in [
        "lrwxrwxrwx  0 0 0 0 Jan  1  2024 evil -> /etc/passwd",
        "-rw-r--r--  0 0 0 0 Jan  1  2024 a link to b",
        "hrw-r--r--  0 0 0 0 Jan  1  2024 a",
        "crw-r--r--  0 0 0 0 Jan  1  2024 dev",
        "prw-r--r--  0 0 0 0 Jan  1  2024 fifo",
        "garbage",
    ] {
        assert!(parse_listing(bad).is_err(), "{bad}");
    }
}

fn entry(name: &str, size: u64) -> Listed {
    Listed { name: name.into(), is_dir: name.ends_with('/'), size }
}

#[test]
fn listing_checks() {
    let limits = Limits::default();
    let files = check_listing(&[entry("A/", 0), entry("A/b.esp", 3), entry("./A/c.txt", 1)], &limits).unwrap();
    assert_eq!(files, vec!["A/b.esp", "A/c.txt"]);
    assert!(check_listing(&[entry("../x", 1)], &limits).is_err());
    assert!(check_listing(&[entry("/etc/x", 1)], &limits).is_err());
    assert!(check_listing(&[entry("C:\\x", 1)], &limits).is_err());
    assert!(check_listing(&[entry("a/../../x", 1)], &limits).is_err());
    // Same file twice, ignoring case.
    assert!(check_listing(&[entry("Data/A.esp", 1), entry("data/a.ESP", 1)], &limits).unwrap_err().contains("twice"));
    // A file where a folder is, either way round.
    assert!(check_listing(&[entry("x", 1), entry("X/y", 1)], &limits).is_err());
    assert!(check_listing(&[entry("x/y", 1), entry("X", 1)], &limits).is_err());
    assert!(check_listing(&[entry("x/y", 1), entry("X/", 0)], &limits).is_ok(), "folders may repeat");
    // Size.
    let small = Limits { total: 10, ratio: 400 };
    assert!(check_listing(&[entry("a", 6), entry("b", 6)], &small).is_err());
    assert!(check_listing(&[entry("a", u64::MAX), entry("b", 2)], &limits).is_err());
}

#[test]
fn seven_zip_unpacks_through_the_reader() {
    let t = TempDir::new("7z");
    let a = t.write("Mod.7z", SEVEN_Z);
    let tar = FakeTar::of(&[("Mod/Data/a.esp", b"esp"), ("Mod/readme.txt", b"hi")]);
    let dest = t.path("out");
    let files = unpack_with(&a, &dest, &Limits::default(), &tar).unwrap();
    assert_eq!(files, vec!["Mod/Data/a.esp", "Mod/readme.txt"]);
    assert_eq!(fs::read(dest.join("Mod/Data/a.esp")).unwrap(), b"esp");
    // Nothing left beside it.
    assert_eq!(fs::read_dir(&t.0).unwrap().count(), 2);
    // The destination must be new.
    assert!(unpack_with(&a, &dest, &Limits::default(), &tar).is_err());
}

#[test]
fn rar_with_bad_names_is_refused_before_unpacking() {
    let t = TempDir::new("rar");
    let a = t.write("Mod.rar", RAR);
    let mut tar = FakeTar::of(&[("ok.txt", b"x")]);
    tar.listing.push_str("-rw-r--r--  0 0 0 1 Jan  1  2024 ../../escape.txt\n");
    let dest = t.path("out");
    assert!(unpack_with(&a, &dest, &Limits::default(), &tar).is_err());
    assert!(!dest.exists() && !t.path(".out.unpacking").exists());
}

#[test]
fn what_was_unpacked_is_checked_again() {
    let t = TempDir::new("rewalk");
    let a = t.write("Mod.7z", SEVEN_Z);
    // The listing looks fine, but the reader makes a link anyway.
    let mut tar = FakeTar::of(&[("ok.txt", b"x")]);
    tar.links.push(("sneaky".into(), "/etc/passwd".into()));
    let err = unpack_with(&a, &t.path("out"), &Limits::default(), &tar).unwrap_err();
    assert!(err.contains("link"), "{err}");
    assert!(!t.path("out").exists() && !t.path(".out.unpacking").exists());
    // A file the listing didn't mention.
    let mut tar = FakeTar::of(&[("ok.txt", b"x")]);
    tar.files.push(("extra.dll".into(), b"y".to_vec()));
    tar.listing = FakeTar::of(&[("ok.txt", b"x")]).listing;
    assert!(unpack_with(&a, &t.path("out2"), &Limits::default(), &tar).unwrap_err().contains("didn't list"));
    // Bigger than the listing said.
    let mut tar = FakeTar::of(&[("big.bin", b"x")]);
    tar.files[0].1 = vec![0; 64];
    let small = Limits { total: 16, ratio: 400 };
    assert!(unpack_with(&a, &t.path("out3"), &small, &tar).is_err());
}

#[test]
fn zip_goes_through_the_own_reader() {
    let t = TempDir::new("zip");
    let z = t.path("Mod.zip");
    write_zip(&z, &[("Mod/b.esp", b"b"), ("Mod/a.esp", b"a")]);
    // A reader that would fail: zip never uses it.
    struct Never;
    impl archives::Tar for Never {
        fn list(&self, _: &std::path::Path) -> Result<String, String> {
            Err("no".into())
        }
        fn extract(&self, _: &std::path::Path, _: &std::path::Path) -> Result<(), String> {
            Err("no".into())
        }
    }
    let files = unpack_with(&z, &t.path("out"), &Limits::default(), &Never).unwrap();
    assert_eq!(files, vec!["Mod/a.esp", "Mod/b.esp"]);
    // The repo's unsafe test zips are refused here too.
    let data = format!("{}/tests/data", env!("CARGO_MANIFEST_DIR"));
    assert!(unpack_with(std::path::Path::new(&format!("{data}/evil.zip")), &t.path("evil"), &Limits::default(), &Never).is_err());
    assert!(unpack_with(&t.write("not.zip", b"nothing"), &t.path("n"), &Limits::default(), &Never).unwrap_err().contains("isn't a zip, 7z or rar"));
}

#[test]
fn missing_bsdtar_is_a_plain_error() {
    let t = TempDir::new("nobsdtar");
    let a = t.write("Mod.7z", SEVEN_Z);
    let tar = archives::Bsdtar { program: t.path("no-such-program") };
    let err = unpack_with(&a, &t.path("out"), &Limits::default(), &tar).unwrap_err();
    assert!(err.contains("bsdtar"), "{err}");
}
