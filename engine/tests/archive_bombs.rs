//! Archives that would fill the drive: refused before unpacking when their sizes give them away, and stopped while
//! unpacking when they don't. Small inputs only: the archives here claim big sizes rather than hold them.

mod support;

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};
use support::{write_zip, FakeTar, TempDir, SEVEN_Z};
use turbine_engine::archives::{self, unpack_with};
use turbine_engine::zip::{crc32, pretend_free_space, Limits, Zip, ZipError, ROOM_MARGIN};

/// A zip whose deflated entries claim `size` bytes each but hold `data` (never inflated: the claims are refused
/// first).
fn claiming_zip(entries: &[(&str, u64, &[u8])]) -> Vec<u8> {
    claiming_zip_crc(entries, None)
}

/// `claiming_zip` with the CRC-32 given (of what the data really inflates to).
fn claiming_zip_crc(entries: &[(&str, u64, &[u8])], crc: Option<u32>) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, size, data) in entries {
        let offset = out.len() as u32;
        let mut head = Vec::new();
        head.extend_from_slice(&20u16.to_le_bytes());
        head.extend_from_slice(&0u16.to_le_bytes());
        head.extend_from_slice(&8u16.to_le_bytes());
        head.extend_from_slice(&[0; 4]);
        head.extend_from_slice(&crc.unwrap_or_else(|| crc32(data)).to_le_bytes());
        head.extend_from_slice(&(data.len() as u32).to_le_bytes());
        head.extend_from_slice(&(*size as u32).to_le_bytes());
        head.extend_from_slice(&(name.len() as u16).to_le_bytes());
        head.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&head);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&head);
        central.extend_from_slice(&[0; 6]); // comment length, disk, internal attributes
        central.extend_from_slice(&0u32.to_le_bytes());
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
    }
    let cd_offset = out.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

/// Deflate (fixed codes) of `1 + 258 * runs` zero bytes: a blank file, packed about 160 to 1.
fn deflated_zeros(runs: usize) -> (Vec<u8>, usize) {
    let mut out = Vec::new();
    let (mut acc, mut n) = (0u64, 0u32);
    let mut put = |bits: u64, len: u32, out: &mut Vec<u8>| {
        acc |= bits << n;
        n += len;
        while n >= 8 {
            out.push(acc as u8);
            acc >>= 8;
            n -= 8;
        }
    };
    let code = |c: u64, len: u32| (0..len).fold(0u64, |r, i| r | (((c >> i) & 1) << (len - 1 - i)));
    put(0b011, 3, &mut out); // last block, fixed codes
    put(code(0x30, 8), 8, &mut out); // literal 0
    for _ in 0..runs {
        put(code(0xC5, 8), 8, &mut out); // length 258
        put(0, 5, &mut out); // distance 1
    }
    put(0, 7, &mut out); // end of block
    put(0, 7, &mut out); // flush
    (out, 1 + 258 * runs)
}

/// What is left in `dir` besides `keep`.
fn others(dir: &Path, keep: &str) -> Vec<String> {
    fs::read_dir(dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n != keep).collect()
}

#[test]
fn many_small_entries_that_add_up_to_a_bomb_are_refused_before_writing() {
    // The break test's shape: many entries just under the one-file check, together far over the archive's ratio.
    let t = TempDir::new("bomb-many");
    pretend_free_space(Some(1 << 50));
    let names: Vec<String> = (0..60).map(|i| format!("Data/f{i}.bin")).collect();
    let entries: Vec<(&str, u64, &[u8])> = names.iter().map(|n| (n.as_str(), 20u64 << 20, &[0u8; 16][..])).collect();
    let z = t.write("bomb.zip", &claiming_zip(&entries));
    let r = Zip::open(&z).unwrap().extract_all(&t.path("out"), &Limits::default());
    assert!(matches!(r, Err(ZipError::TooBig(_))), "{r:?}");
    assert_eq!(others(&t.0, "bomb.zip"), Vec::<String>::new(), "nothing is written");
}

#[test]
fn one_middle_sized_entry_is_held_to_the_ratio() {
    let t = TempDir::new("bomb-one");
    pretend_free_space(Some(1 << 50));
    let z = t.write("one.zip", &claiming_zip(&[("big.bin", 40 << 20, &[0u8; 1024])]));
    let r = Zip::open(&z).unwrap().extract_all(&t.path("out"), &Limits::default());
    assert!(matches!(r, Err(ZipError::TooBig(ref n)) if n == "big.bin"), "{r:?}");
    assert!(!t.path("out").exists() && !t.path(".out.unpacking").exists());
}

#[test]
fn blank_files_that_pack_well_still_unpack() {
    // A small mod of blank textures packs far better than the archive ratio; under the size those ratios start at,
    // that's fine.
    let t = TempDir::new("blank");
    pretend_free_space(Some(1 << 50));
    let (data, size) = deflated_zeros(16_000);
    let crc = crc32(&vec![0u8; size]);
    let z = t.write("blank.zip", &claiming_zip_crc(&[("textures/blank.dds", size as u64, &data)], Some(crc)));
    let written = Zip::open(&z).unwrap().extract_all(&t.path("out"), &Limits::default()).unwrap();
    assert_eq!(written.len(), 1);
    assert_eq!(fs::metadata(t.path("out/textures/blank.dds")).unwrap().len(), size as u64);
}

#[test]
fn a_zip_that_doesnt_fit_the_drive_is_refused_with_both_sizes() {
    let t = TempDir::new("noroom");
    let z = t.path("Mod.zip");
    write_zip(&z, &[("Mod/a.esp", &[7u8; 4096])]);
    pretend_free_space(Some(100 << 20));
    let err = archives::unpack(&z, &t.path("out"), &Limits::default()).unwrap_err();
    assert!(err.contains("257 MB") && err.contains("100 MB free"), "{err}");
    assert!(!t.path("out").exists() && !t.path(".out.unpacking").exists());
    // With room it goes through.
    pretend_free_space(Some(ROOM_MARGIN + (1 << 20)));
    assert_eq!(archives::unpack(&z, &t.path("out"), &Limits::default()).unwrap(), vec!["Mod/a.esp"]);
    // Unknown free space leaves only the total limit.
    pretend_free_space(None);
    assert!(archives::unpack(&z, &t.path("out2"), &Limits::default()).is_ok());
}

#[test]
fn a_7z_that_doesnt_fit_the_drive_is_refused_before_bsdtar_runs() {
    let t = TempDir::new("noroom7z");
    let a = t.write("Mod.7z", SEVEN_Z);
    let tar = FakeTar::of(&[("Mod/a.esp", &[1u8; 5000])]);
    pretend_free_space(Some(ROOM_MARGIN + 1000));
    let err = unpack_with(&a, &t.path("out"), &Limits::default(), &tar).unwrap_err();
    assert!(err.contains("free"), "{err}");
    assert_eq!(others(&t.0, "Mod.7z"), Vec::<String>::new());
}

#[test]
fn a_7z_that_writes_more_than_it_listed_is_held_to_the_free_space() {
    // Sizes the listing leaves out (rar can) show as 0: what bsdtar really writes is what counts.
    let t = TempDir::new("unknown7z");
    let a = t.write("Mod.7z", SEVEN_Z);
    let mut tar = FakeTar::of(&[("Mod/a.esp", b"")]);
    tar.files[0].1 = vec![0; 5000];
    pretend_free_space(Some(ROOM_MARGIN + 1000));
    let err = unpack_with(&a, &t.path("out"), &Limits::default(), &tar).unwrap_err();
    assert!(err.contains("far more"), "{err}");
    assert_eq!(others(&t.0, "Mod.7z"), Vec::<String>::new());
}

#[test]
fn a_7z_whose_listing_is_far_bigger_than_the_file_is_refused() {
    let t = TempDir::new("ratio7z");
    let a = t.write("Mod.7z", SEVEN_Z);
    pretend_free_space(Some(1 << 50));
    let mut tar = FakeTar::of(&[("Mod/a.bin", b"x")]);
    tar.listing = tar.listing.replace("     1 Mar", &format!("{} Mar", 2u64 << 30));
    let err = unpack_with(&a, &t.path("out"), &Limits::default(), &tar).unwrap_err();
    assert!(err.contains("far more"), "{err}");
}

#[test]
fn bsdtar_is_stopped_while_it_writes_too_much() {
    if !Path::new("/bin/sh").exists() {
        return;
    }
    let t = TempDir::new("watch");
    let a = t.write("Mod.7z", SEVEN_Z);
    // A stand-in program with bsdtar's arguments: lists one file of unknown size, then writes 1 MiB at a time (at
    // most 64 MiB) and waits. Stopping it is the only way this ends quickly.
    let script = t.write(
        "fake-bsdtar",
        b"#!/bin/sh\ncase \"$1\" in\n-tvf) echo '-rw-r--r--  0 0      0           0 Mar 15 12:30 big.bin' ;;\n\
          -xf) i=0; while [ $i -lt 64 ]; do head -c 1048576 /dev/zero >> \"$4/big.bin\"; i=$((i+1)); done; exec sleep 30 ;;\nesac\n",
    );
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    let tar = archives::Bsdtar { program: script };
    pretend_free_space(Some(ROOM_MARGIN + (4 << 20)));
    let started = Instant::now();
    let err = unpack_with(&a, &t.path("out"), &Limits::default(), &tar).unwrap_err();
    assert!(err.contains("far more"), "{err}");
    assert!(started.elapsed() < Duration::from_secs(25), "bsdtar ran to the end ({:?})", started.elapsed());
    assert_eq!(others(&t.0, "Mod.7z").into_iter().filter(|n| n != "fake-bsdtar").collect::<Vec<_>>(), Vec::<String>::new());
}

/// A stand-in bsdtar whose unpacking step is `write` (sh, with the folder to write into as "$4").
fn writing_tar(t: &TempDir, write: &str) -> archives::Bsdtar {
    use std::os::unix::fs::PermissionsExt;
    let script = t.write("fake-bsdtar", format!("#!/bin/sh\ncase \"$1\" in\n-xf) {write} ;;\nesac\n").as_bytes());
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    archives::Bsdtar { program: script }
}

fn bytes_in(dir: &Path) -> u64 {
    fs::read_dir(dir).unwrap().flatten().map(|e| e.metadata().unwrap().len()).sum()
}

#[test]
fn one_file_bsdtar_writes_can_never_pass_the_budget() {
    use archives::Tar;
    if !Path::new("/bin/sh").exists() || !cfg!(all(target_os = "linux", target_pointer_width = "64")) {
        return;
    }
    // One file written as fast as the drive takes it, 256 MiB if nothing stops it: the system's own cap ends it one
    // byte past the budget at most, however long it is between two looks.
    let t = TempDir::new("cap");
    let tar = writing_tar(&t, "head -c 268435456 /dev/zero > \"$4/big.bin\"; exec sleep 30");
    fs::create_dir(t.path("into")).unwrap();
    let budget = 4 << 20;
    let started = Instant::now();
    let err = tar.extract_within(Path::new("Mod.7z"), &t.path("into"), budget).unwrap_err();
    assert!(err.contains("far more"), "{err}");
    assert!(bytes_in(&t.path("into")) <= budget + 1, "{} bytes written", bytes_in(&t.path("into")));
    assert!(started.elapsed() < Duration::from_secs(25));
}

#[test]
fn many_files_bsdtar_writes_pass_the_budget_by_little() {
    use archives::Tar;
    if !Path::new("/bin/sh").exists() {
        return;
    }
    // 256 files of 1 MiB, each under the cap on one file: near the budget the folder is looked at often enough that
    // only a few more land (four looks a second would let most of them through).
    let t = TempDir::new("many");
    let tar = writing_tar(&t, "i=0; while [ $i -lt 256 ]; do head -c 1048576 /dev/zero > \"$4/f$i.bin\"; i=$((i+1)); done; exec sleep 30");
    fs::create_dir(t.path("into")).unwrap();
    let budget = 8 << 20;
    let err = tar.extract_within(Path::new("Mod.7z"), &t.path("into"), budget).unwrap_err();
    assert!(err.contains("far more"), "{err}");
    let wrote = bytes_in(&t.path("into"));
    assert!(wrote <= budget + (32 << 20), "{} MiB written with a budget of 8", wrote >> 20);
}

#[test]
fn bsdtar_far_under_the_budget_is_left_to_finish() {
    use archives::Tar;
    if !Path::new("/bin/sh").exists() {
        return;
    }
    let t = TempDir::new("calm");
    let tar = writing_tar(&t, "head -c 1048576 /dev/zero > \"$4/a.bin\"");
    fs::create_dir(t.path("into")).unwrap();
    tar.extract_within(Path::new("Mod.7z"), &t.path("into"), 8 << 30).unwrap();
    assert_eq!(bytes_in(&t.path("into")), 1 << 20);
    // No budget at all (plain extract) works as before.
    fs::create_dir(t.path("into2")).unwrap();
    tar.extract(Path::new("Mod.7z"), &t.path("into2")).unwrap();
    assert_eq!(bytes_in(&t.path("into2")), 1 << 20);
}

#[test]
fn an_archive_inside_an_archive_shares_the_allowance() {
    let t = TempDir::new("inner");
    t.write("outer/Installer/payload.bin", &[0u8; 3000]);
    let left = archives::limits_inside(&t.path("outer"), &Limits { total: 5000, ratio: 400 });
    assert_eq!(left.total, 2000);
    assert_eq!(archives::limits_inside(&t.path("outer"), &Limits { total: 100, ratio: 400 }).total, 0);
    // And the shared allowance is what the inner archive is held to.
    let a = t.write("outer/Installer/inner.7z", SEVEN_Z);
    pretend_free_space(Some(1 << 50));
    let tar = FakeTar::of(&[("x.dll", &[1u8; 10])]);
    let files = archives::unpack_inside(&a, &t.path("outer/.inner"), &t.path("outer"), Some(&tar)).unwrap();
    assert_eq!(files, vec!["x.dll"]);
}
