//! Helpers shared by the test files: temp folders, test data, small archives, fake games and plugins.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use turbine_engine::archives::Tar;
use turbine_engine::games::{Game, Runtime, Store as Launcher};
use turbine_engine::zip::crc32;

/// A folder that is removed again at the end of the test.
pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new(tag: &str) -> TempDir {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let d = std::env::temp_dir().join(format!("turbine-t-{tag}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        TempDir(d)
    }
    pub fn path(&self, rel: &str) -> PathBuf {
        self.0.join(rel)
    }
    pub fn write(&self, rel: &str, data: &[u8]) -> PathBuf {
        let p = self.0.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, data).unwrap();
        p
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A zip with stored (uncompressed) entries, as bytes.
pub fn zip_bytes(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in files {
        let offset = out.len() as u32;
        let crc = crc32(data);
        let len = data.len() as u32;
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&0x0800u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);

        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&0x0800u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&[0; 4]);
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&len.to_le_bytes());
        central.extend_from_slice(&len.to_le_bytes());
        central.extend_from_slice(&(name.len() as u16).to_le_bytes());
        central.extend_from_slice(&[0; 8]);
        central.extend_from_slice(&0u32.to_le_bytes());
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
    }
    let cd_offset = out.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

pub fn write_zip(path: &Path, files: &[(&str, &[u8])]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, zip_bytes(files)).unwrap();
}

/// `write_zip` for text entries.
pub fn write_zip_text(path: &Path, files: &[(&str, &str)]) {
    let bytes: Vec<(&str, &[u8])> = files.iter().map(|(n, d)| (*n, d.as_bytes())).collect();
    write_zip(path, &bytes);
}

/// A fresh empty folder under the system temp folder. Not removed afterwards.
pub fn scratch(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let p = std::env::temp_dir().join(format!("turbine-s-{tag}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

/// A file in tests/data.
pub fn data(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data").join(name)
}

/// The text of a file in tests/data.
pub fn data_text(name: &str) -> String {
    fs::read_to_string(data(name)).unwrap()
}

pub fn has_bash() -> bool {
    std::process::Command::new("bash").arg("--version").output().is_ok()
}

/// The first bytes of a 7z file; the rest doesn't matter to the fake reader.
pub const SEVEN_Z: &[u8] = &[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C, 0, 4, 1, 2, 3];
pub const RAR: &[u8] = b"Rar!\x1a\x07\x01\x00rest";

/// A stand-in for bsdtar: gives a fixed listing and "extracts" by writing the given files (and whatever else a test
/// wants to sneak in).
pub struct FakeTar {
    pub listing: String,
    pub files: Vec<(String, Vec<u8>)>,
    /// Links to make while extracting: (path, target).
    pub links: Vec<(String, String)>,
}

impl FakeTar {
    /// A listing in bsdtar's `-tv` style for these files (folders listed too).
    pub fn of(files: &[(&str, &[u8])]) -> FakeTar {
        let mut listing = String::new();
        let mut dirs: Vec<String> = Vec::new();
        for (name, data) in files {
            let parts: Vec<&str> = name.split('/').collect();
            for i in 1..parts.len() {
                let d = format!("{}/", parts[..i].join("/"));
                if !dirs.contains(&d) {
                    listing.push_str(&format!("drwxr-xr-x  0 0      0           0 Jan  1  2024 {d}\n"));
                    dirs.push(d);
                }
            }
            listing.push_str(&format!("-rw-r--r--  0 0      0      {:>6} Mar 15 12:30 {name}\n", data.len()));
        }
        FakeTar { listing, files: files.iter().map(|(n, d)| (n.to_string(), d.to_vec())).collect(), links: Vec::new() }
    }
}

impl Tar for FakeTar {
    fn list(&self, _archive: &Path) -> Result<String, String> {
        Ok(self.listing.clone())
    }
    fn extract(&self, _archive: &Path, into: &Path) -> Result<(), String> {
        for (name, data) in &self.files {
            let p = into.join(name);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, data).unwrap();
        }
        for (name, target) in &self.links {
            let p = into.join(name);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::os::unix::fs::symlink(target, p).unwrap();
        }
        Ok(())
    }
}

pub fn game(store: Launcher, id: &str, name: &str, dir: &Path, prefix: Option<PathBuf>) -> Game {
    Game {
        store,
        store_id: id.into(),
        name: name.into(),
        install_dir: dir.to_path_buf(),
        library: dir.parent().unwrap().to_path_buf(),
        build: Some("100".into()),
        runtime: Runtime::Native,
        cover: None,
        header: None,
        prefix,
    }
}

/// A Skyrim SE plugin: TES4 header with these flags and masters.
pub fn plugin_bytes(flags: u32, masters: &[&str]) -> Vec<u8> {
    let mut sub = Vec::new();
    let mut add = |kind: &[u8; 4], data: &[u8]| {
        sub.extend_from_slice(kind);
        sub.extend_from_slice(&(data.len() as u16).to_le_bytes());
        sub.extend_from_slice(data);
    };
    let mut hedr = Vec::new();
    hedr.extend_from_slice(&1.71f32.to_le_bytes());
    hedr.extend_from_slice(&0u32.to_le_bytes());
    hedr.extend_from_slice(&0x800u32.to_le_bytes());
    add(b"HEDR", &hedr);
    add(b"CNAM", b"Someone\0");
    for m in masters {
        let mut z = m.as_bytes().to_vec();
        z.push(0);
        add(b"MAST", &z);
        add(b"DATA", &0u64.to_le_bytes());
    }
    let mut out = Vec::new();
    out.extend_from_slice(b"TES4");
    out.extend_from_slice(&(sub.len() as u32).to_le_bytes());
    out.extend_from_slice(&flags.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&44u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&sub);
    out.extend_from_slice(b"GRUP\x18\0\0\0rest of the file");
    out
}

/// Text as UTF-16 LE with a BOM.
pub fn utf16le(text: &str) -> Vec<u8> {
    let mut out = vec![0xFF, 0xFE];
    for u in text.encode_utf16() {
        out.extend_from_slice(&u.to_le_bytes());
    }
    out
}
