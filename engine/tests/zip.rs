//! The archives in tests/data were made with Python's zipfile and zlib, an independent implementation.

use std::fs;
use std::path::{Path, PathBuf};
use turbine_engine::zip::{Limits, Zip, ZipError};

fn data(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data").join(name)
}

fn scratch(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("turbine-zip-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn unpacks_every_kind_of_entry() {
    let out = scratch("good");
    let dest = out.join("mod");
    let written = Zip::open(&data("good.zip")).unwrap().extract_all(&dest, &Limits::default()).unwrap();
    assert_eq!(written.len(), 7);
    let readme = fs::read(dest.join("Data/readme.txt")).unwrap();
    assert_eq!(readme, "Skyrim mod readme line. ".repeat(400).into_bytes());
    assert_eq!(fs::read(dest.join("Data/stored.esp")).unwrap(), b"TES4 plugin bytes");
    assert_eq!(fs::read(dest.join("Data/tiny.txt")).unwrap(), b"hi");
    assert_eq!(fs::read(dest.join("Data/empty.txt")).unwrap(), b"");
    assert_eq!(fs::read(dest.join("Data/\u{dc}n\u{ef}code name.txt")).unwrap(), b"unicode");
    assert_eq!(fs::metadata(dest.join("Data/SKSE/Plugins/test.dll")).unwrap().len(), 70000);
    assert!(fs::metadata(dest.join("Data/mixed.txt")).unwrap().len() > 100_000);
    assert!(!out.join(".mod.unpacking").exists());
    let _ = fs::remove_dir_all(&out);
}

#[test]
fn reads_zip64() {
    let out = scratch("z64");
    let written = Zip::open(&data("zip64.zip")).unwrap().extract_all(&out.join("m"), &Limits::default()).unwrap();
    assert_eq!(written.len(), 2);
    assert_eq!(fs::read(out.join("m/small.txt")).unwrap(), b"small");
    let _ = fs::remove_dir_all(&out);
}

#[test]
fn refuses_escaping_paths() {
    assert!(matches!(Zip::open(&data("evil.zip")), Err(ZipError::UnsafePath(_))));
}

#[test]
fn skips_links() {
    let out = scratch("link");
    let written = Zip::open(&data("link.zip")).unwrap().extract_all(&out.join("m"), &Limits::default()).unwrap();
    assert_eq!(written, vec![PathBuf::from("real.txt")]);
    assert!(!out.join("m/link").exists());
    let _ = fs::remove_dir_all(&out);
}

#[test]
fn notices_damage_and_leaves_nothing_behind() {
    let out = scratch("crc");
    let r = Zip::open(&data("badcrc.zip")).unwrap().extract_all(&out.join("m"), &Limits::default());
    assert!(matches!(r, Err(ZipError::Checksum(_))), "{r:?}");
    assert!(!out.join("m").exists() && !out.join(".m.unpacking").exists());
    let _ = fs::remove_dir_all(&out);
}

#[test]
fn refuses_bombs() {
    let out = scratch("bomb");
    let r = Zip::open(&data("bomb.zip")).unwrap().extract_all(&out.join("m"), &Limits::default());
    assert!(matches!(r, Err(ZipError::TooBig(_))), "{r:?}");
    // And with a lower total cap, before reading anything.
    let small = Limits { total: 1 << 20, ratio: 400 };
    let r = Zip::open(&data("bomb.zip")).unwrap().extract_all(&out.join("m"), &small);
    assert!(matches!(r, Err(ZipError::TooBig(_))));
    let _ = fs::remove_dir_all(&out);
}

#[test]
fn not_a_zip() {
    assert!(matches!(Zip::open(&data("../scan.rs")), Err(ZipError::NotZip)));
}
