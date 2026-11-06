//! Opens mod archives of every kind Turbine accepts: zip (read by Turbine's own reader) and 7z or rar (read by
//! `bsdtar` from the Flatpak runtime). `unpack` is the one way in, used by the store for every format.
//!
//! bsdtar is only ever run with fixed arguments (never through a shell), and only on an archive that was listed and
//! checked first: every name must be a plain relative path (`zip::safe_relative`), no links, devices or other special
//! entries, no two entries with the same path ignoring case, and the total size within the limits. It then unpacks
//! into a fresh temporary folder without owners or permissions from the archive, and the result is walked again:
//! anything that isn't a plain file or folder, or that wasn't in the listing, stops the import.
//! Nothing inside an archive is ever run.
//!
//! The listing format is read as libarchive's tar/util.c list_item_verbose writes it (commit eb57ffe, Nov 2026):
//! mode, link count, owner, group, size, a three part date (or "-- -- ----"), one space, then the name with
//! unprintable bytes escaped (safe_fputs and bsdtar_expand_char: "\\", "\n" style letters, "\ooo" octal); hard links
//! end in " link to <target>" and symbolic links in " -> <target>". The two extract options are in tar/cmdline.c
//! lines 113 and 114.

use crate::zip::{self, Limits, Zip};
use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Zip,
    SevenZip,
    Rar,
}

impl Format {
    pub fn label(self) -> &'static str {
        match self {
            Format::Zip => "zip",
            Format::SevenZip => "7z",
            Format::Rar => "rar",
        }
    }
}

/// The archive's format from its first bytes (the name can lie). None: not an archive Turbine can open.
pub fn detect(path: &Path) -> io::Result<Option<Format>> {
    let mut head = [0u8; 8];
    let mut f = File::open(path)?;
    let mut n = 0;
    while n < head.len() {
        match f.read(&mut head[n..])? {
            0 => break,
            k => n += k,
        }
    }
    let head = &head[..n];
    Ok(if head.starts_with(b"PK\x03\x04") || head.starts_with(b"PK\x05\x06") {
        Some(Format::Zip)
    } else if head.starts_with(&[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C]) {
        Some(Format::SevenZip)
    } else if head.starts_with(b"Rar!\x1a\x07") {
        Some(Format::Rar)
    } else {
        None
    })
}

/// Is this file name one of the archive kinds Turbine offers to add?
pub fn is_archive_name(name: &str) -> bool {
    let lc = name.to_ascii_lowercase();
    lc.ends_with(".zip") || lc.ends_with(".7z") || lc.ends_with(".rar")
}

/// The program that reads 7z and rar archives. The real one runs bsdtar; tests give their own.
pub trait Tar {
    /// The output of `bsdtar -tvf <archive>`.
    fn list(&self, archive: &Path) -> Result<String, String>;
    /// Runs `bsdtar -xf <archive> -C <into> --no-same-owner --no-same-permissions`; `into` exists and is empty.
    fn extract(&self, archive: &Path, into: &Path) -> Result<(), String>;
}

/// bsdtar from the Flatpak runtime (found on PATH), with a fixed, clean environment.
pub struct Bsdtar {
    pub program: PathBuf,
}

impl Default for Bsdtar {
    fn default() -> Self {
        Bsdtar { program: "bsdtar".into() }
    }
}

impl Bsdtar {
    fn run(&self, args: Vec<OsString>) -> Result<Vec<u8>, String> {
        let out = Command::new(&self.program)
            .args(args)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_else(|| "/usr/bin:/bin".into()))
            .env("LC_ALL", "C.UTF-8")
            .stdin(Stdio::null())
            .output()
            .map_err(|e| {
                if e.kind() == io::ErrorKind::NotFound {
                    "the program that opens 7z and rar files (bsdtar) isn't available".to_string()
                } else {
                    format!("the program that opens 7z and rar files couldn't start ({e})")
                }
            })?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            let line = err.lines().find(|l| !l.trim().is_empty()).unwrap_or("it stopped with an error").trim();
            return Err(format!("the archive couldn't be read ({})", line.trim_start_matches("bsdtar: ")));
        }
        Ok(out.stdout)
    }
}

impl Tar for Bsdtar {
    fn list(&self, archive: &Path) -> Result<String, String> {
        let out = self.run(vec!["-tvf".into(), archive.into()])?;
        Ok(String::from_utf8_lossy(&out).into_owned())
    }

    fn extract(&self, archive: &Path, into: &Path) -> Result<(), String> {
        self.run(vec![
            "-xf".into(),
            archive.into(),
            "-C".into(),
            into.into(),
            "--no-same-owner".into(),
            "--no-same-permissions".into(),
        ])
        .map(|_| ())
    }
}

/// One entry of a bsdtar listing.
#[derive(Debug, Clone, PartialEq)]
pub struct Listed {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

/// Undoes bsdtar's escapes in names (`\\`, `\n` style letters and `\ooo` octal bytes).
fn unescape_name(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' && i + 1 < b.len() {
            let c = b[i + 1];
            if i + 3 < b.len() && (b'0'..=b'3').contains(&c) && b[i + 2..i + 4].iter().all(|d| (b'0'..=b'7').contains(d)) {
                out.push((c - b'0') * 64 + (b[i + 2] - b'0') * 8 + (b[i + 3] - b'0'));
                i += 4;
                continue;
            }
            let mapped = match c {
                b'\\' => Some(b'\\'),
                b'a' => Some(7),
                b'b' => Some(8),
                b'f' => Some(12),
                b'n' => Some(b'\n'),
                b'r' => Some(b'\r'),
                b't' => Some(b'\t'),
                b'v' => Some(11),
                _ => None,
            };
            if let Some(m) = mapped {
                out.push(m);
                i += 2;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Splits off `n` whitespace-separated fields; returns them and the rest (leading spaces removed).
fn fields(line: &str, n: usize) -> Option<(Vec<&str>, &str)> {
    let mut rest = line;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        rest = rest.trim_start();
        let end = rest.find(char::is_whitespace)?;
        out.push(&rest[..end]);
        rest = &rest[end..];
    }
    // Exactly one space separates the date from the name; more belong to the name.
    let rest = rest.strip_prefix(' ').unwrap_or(rest);
    Some((out, rest))
}

/// Reads `bsdtar -tvf` output (an `ls -l` style line per entry: mode, links, owner, group, size, month, day,
/// year or time, name). Refuses anything that isn't a plain file or folder.
pub fn parse_listing(text: &str) -> Result<Vec<Listed>, String> {
    let mut out = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let (f, name) = fields(line, 8).ok_or_else(|| format!("the archive's file list can't be read ({})", line.trim()))?;
        let mode = f[0];
        let size: u64 = f[4].parse().map_err(|_| format!("the archive's file list can't be read ({})", line.trim()))?;
        let kind = mode.chars().next().unwrap_or('?');
        if name.is_empty() {
            return Err("the archive has an entry without a name".into());
        }
        match kind {
            '-' | 'd' => {}
            'l' | 'h' => return Err(format!("the archive has a link ({}), so it wasn't opened", unescape_name(name))),
            _ => return Err(format!("the archive has a special file ({}), so it wasn't opened", unescape_name(name))),
        }
        // Hard links show as "name link to target" with a plain file's mode.
        if name.contains(" link to ") || name.contains(" -> ") {
            return Err(format!("the archive has a link ({}), so it wasn't opened", unescape_name(name)));
        }
        let name = unescape_name(name);
        out.push(Listed { is_dir: kind == 'd' || name.ends_with('/'), name, size: if kind == 'd' { 0 } else { size } });
    }
    Ok(out)
}

/// Checks a listing: safe names, no duplicates ignoring case, total size within the limits.
/// Returns the files ('/' separated) it should produce.
pub fn check_listing(list: &[Listed], limits: &Limits) -> Result<Vec<String>, String> {
    let mut seen_files: HashSet<String> = HashSet::new();
    let mut seen_dirs: HashSet<String> = HashSet::new();
    let mut files = Vec::new();
    let mut total: u64 = 0;
    for e in list {
        let rel = zip::safe_relative(&e.name).map_err(|err| err.to_string())?;
        let parts: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
        if parts.is_empty() {
            continue;
        }
        let path = parts.join("/");
        let key = path.to_lowercase();
        // Every folder above it, too, so a file can't sit where a folder is.
        for i in 1..parts.len() {
            let dir = parts[..i].join("/").to_lowercase();
            if seen_files.contains(&dir) {
                return Err(format!("the archive has a file and a folder both called {}, so it wasn't opened", parts[..i].join("/")));
            }
            seen_dirs.insert(dir);
        }
        if e.is_dir {
            if seen_files.contains(&key) {
                return Err(format!("the archive has a file and a folder both called {path}, so it wasn't opened"));
            }
            seen_dirs.insert(key);
            continue;
        }
        if seen_dirs.contains(&key) {
            return Err(format!("the archive has a file and a folder both called {path}, so it wasn't opened"));
        }
        if !seen_files.insert(key) {
            return Err(format!("the archive has {path} twice (names differing only in capitals count), so it wasn't opened"));
        }
        total = total.checked_add(e.size).ok_or("the archive says its files are impossibly large")?;
        if total > limits.total {
            return Err("the archive unpacks to far more than it should, so it wasn't opened".into());
        }
        files.push(path);
    }
    Ok(files)
}

fn rel_string(p: &Path) -> String {
    p.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/")
}

/// Walks an unpacked folder: every entry must be a plain file or folder. Returns the files and their total size.
fn walk_plain(base: &Path) -> Result<(Vec<String>, u64), String> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<String>, total: &mut u64) -> Result<(), String> {
        let rd = fs::read_dir(dir).map_err(|e| format!("couldn't read what was unpacked ({e})"))?;
        for e in rd {
            let e = e.map_err(|e| format!("couldn't read what was unpacked ({e})"))?;
            let p = e.path();
            let m = fs::symlink_metadata(&p).map_err(|e| format!("couldn't read what was unpacked ({e})"))?;
            let rel = p.strip_prefix(base).map(rel_string).unwrap_or_default();
            if m.file_type().is_symlink() {
                return Err(format!("the archive made a link ({rel}), so it wasn't added"));
            } else if m.is_dir() {
                walk(base, &p, out, total)?;
            } else if m.is_file() {
                *total = total.saturating_add(m.len());
                out.push(rel);
            } else {
                return Err(format!("the archive made a special file ({rel}), so it wasn't added"));
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    let mut total = 0;
    walk(base, base, &mut out, &mut total)?;
    out.sort();
    Ok((out, total))
}

/// Unpacks any supported archive into `dest` (which must not exist yet) and returns its files, '/' separated,
/// sorted. `dest` is either complete or absent afterwards.
pub fn unpack(archive: &Path, dest: &Path, limits: &Limits) -> Result<Vec<String>, String> {
    unpack_with(archive, dest, limits, &Bsdtar::default())
}

/// `unpack` with the 7z/rar reader given (tests use a stand-in).
pub fn unpack_with(archive: &Path, dest: &Path, limits: &Limits, tar: &dyn Tar) -> Result<Vec<String>, String> {
    let format = detect(archive)
        .map_err(|e| format!("couldn't read the file ({e})"))?
        .ok_or("it isn't a zip, 7z or rar archive")?;
    if fs::symlink_metadata(dest).is_ok() {
        return Err("the folder to unpack into already exists".into());
    }
    if format == Format::Zip {
        let mut z = Zip::open(archive).map_err(|e| e.to_string())?;
        let written = z.extract_all(dest, limits).map_err(|e| e.to_string())?;
        let mut files: Vec<String> = written.iter().map(|p| rel_string(p)).collect();
        files.sort();
        return Ok(files);
    }

    let listing = tar.list(archive)?;
    let entries = parse_listing(&listing)?;
    let expected = check_listing(&entries, limits)?;

    let parent = dest.parent().ok_or("there's no folder to unpack into")?;
    fs::create_dir_all(parent).map_err(|e| format!("couldn't make a folder ({e})"))?;
    let tmp = parent.join(format!(".{}.unpacking", dest.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).map_err(|e| format!("couldn't make a folder ({e})"))?;
    let result = (|| -> Result<Vec<String>, String> {
        tar.extract(archive, &tmp)?;
        let (files, total) = walk_plain(&tmp)?;
        if total > limits.total {
            return Err("the archive unpacked to far more than it should, so it wasn't added".into());
        }
        let want: HashSet<String> = expected.iter().map(|f| f.to_lowercase()).collect();
        if let Some(extra) = files.iter().find(|f| !want.contains(&f.to_lowercase())) {
            return Err(format!("the archive unpacked a file it didn't list ({extra}), so it wasn't added"));
        }
        fs::rename(&tmp, dest).map_err(|e| format!("couldn't move the unpacked files ({e})"))?;
        Ok(files)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&tmp);
    }
    result
}
