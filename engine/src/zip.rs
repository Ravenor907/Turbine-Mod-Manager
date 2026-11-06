//! Reads zip archives (including ZIP64) and unpacks them safely: nothing outside the target folder, no links, no
//! archive bombs, every file checked against its CRC-32. Nothing inside an archive is ever run.

use crate::inflate::{inflate, InflateError};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};

#[derive(Debug)]
pub enum ZipError {
    Io(std::io::Error),
    NotZip,
    Damaged(&'static str),
    Unsupported(String),
    UnsafePath(String),
    TooBig(String),
    Inflate(String, InflateError),
    Checksum(String),
}

impl std::fmt::Display for ZipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ZipError::Io(e) => write!(f, "couldn't read or write a file ({e})"),
            ZipError::NotZip => f.write_str("this isn't a zip file"),
            ZipError::Damaged(what) => write!(f, "the zip file is damaged ({what})"),
            ZipError::Unsupported(what) => write!(f, "this zip uses something Turbine can't open yet ({what})"),
            ZipError::UnsafePath(p) => write!(f, "the zip tries to put a file outside its folder ({p}), so it wasn't opened"),
            ZipError::TooBig(p) => write!(f, "{p} unpacks to far more than it should, so the zip wasn't opened"),
            ZipError::Inflate(p, e) => write!(f, "{p}: {e}"),
            ZipError::Checksum(p) => write!(f, "{p} doesn't match its checksum, so the download is probably damaged"),
        }
    }
}

impl std::error::Error for ZipError {}

impl From<std::io::Error> for ZipError {
    fn from(e: std::io::Error) -> Self {
        ZipError::Io(e)
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    /// Path inside the archive, '/' separated, already checked to be safe.
    pub name: String,
    pub is_dir: bool,
    method: u16,
    crc: u32,
    pub compressed: u64,
    pub size: u64,
    local_offset: u64,
    is_link: bool,
}

pub struct Limits {
    /// Most bytes all files together may unpack to.
    pub total: u64,
    /// Most a single file may grow compared to its compressed size, once it is larger than 64 MiB.
    pub ratio: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Limits { total: 200 << 30, ratio: 400 }
    }
}

fn u16le(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}
fn u32le(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}
fn u64le(b: &[u8], at: usize) -> u64 {
    let mut a = [0u8; 8];
    a.copy_from_slice(&b[at..at + 8]);
    u64::from_le_bytes(a)
}

/// Code page 437, which zips use for names when they don't say UTF-8 (upper half only).
const CP437: &str = "ÇüéâäàåçêëèïîìÄÅÉæÆôöòûùÿÖÜ¢£¥₧ƒáíóúñÑªº¿⌐¬½¼¡«»░▒▓│┤╡╢╖╕╣║╗╝╜╛┐└┴┬├─┼╞╟╚╔╩╦╠═╬╧╨╤╥╙╘╒╓╫╪┘┌█▄▌▐▀αßΓπΣσµτΦΘΩδ∞φε∩≡±≥≤⌠⌡÷≈°∙·√ⁿ²■\u{a0}";

fn decode_name(raw: &[u8], utf8: bool) -> String {
    if utf8 {
        return String::from_utf8_lossy(raw).into_owned();
    }
    if let Ok(s) = std::str::from_utf8(raw) {
        return s.to_string(); // plain ASCII, or UTF-8 without the flag (common)
    }
    let table: Vec<char> = CP437.chars().collect();
    raw.iter().map(|&b| if b < 0x80 { b as char } else { table[(b - 0x80) as usize] }).collect()
}

/// Turns an archive path into a safe relative path, or refuses it.
pub fn safe_relative(name: &str) -> Result<PathBuf, ZipError> {
    let clean = name.replace('\\', "/");
    if clean.contains('\0') || clean.starts_with('/') || clean.chars().nth(1) == Some(':') {
        return Err(ZipError::UnsafePath(name.to_string()));
    }
    let mut out = PathBuf::new();
    for part in clean.split('/') {
        match part {
            "" | "." => {}
            ".." => return Err(ZipError::UnsafePath(name.to_string())),
            p => out.push(p),
        }
    }
    for c in out.components() {
        if !matches!(c, Component::Normal(_)) {
            return Err(ZipError::UnsafePath(name.to_string()));
        }
    }
    Ok(out)
}

pub struct Zip {
    file: File,
    pub entries: Vec<Entry>,
}

impl Zip {
    pub fn open(path: &Path) -> Result<Zip, ZipError> {
        let mut file = File::open(path)?;
        let len = file.seek(SeekFrom::End(0))?;
        if len < 22 {
            return Err(ZipError::NotZip);
        }
        // The end record is in the last 64 KiB + 22 bytes (the comment can be up to 65535 bytes).
        let tail_len = len.min(65_557) as usize;
        let mut tail = vec![0u8; tail_len];
        file.seek(SeekFrom::Start(len - tail_len as u64))?;
        file.read_exact(&mut tail)?;
        let eocd = (0..=tail_len - 22).rev().find(|&i| u32le(&tail, i) == 0x0605_4b50).ok_or(ZipError::NotZip)?;
        let mut count = u16le(&tail, eocd + 10) as u64;
        let mut cd_size = u32le(&tail, eocd + 12) as u64;
        let mut cd_off = u32le(&tail, eocd + 16) as u64;
        // ZIP64: a locator sits just before the end record.
        if eocd >= 20 && u32le(&tail, eocd - 20) == 0x0706_4b50 {
            let rec = u64le(&tail, eocd - 12);
            let mut r = [0u8; 56];
            file.seek(SeekFrom::Start(rec))?;
            file.read_exact(&mut r)?;
            if u32le(&r, 0) != 0x0606_4b50 {
                return Err(ZipError::Damaged("ZIP64 end record"));
            }
            count = u64le(&r, 32);
            cd_size = u64le(&r, 40);
            cd_off = u64le(&r, 48);
        }
        if cd_off.checked_add(cd_size).map(|e| e > len).unwrap_or(true) || count > 2_000_000 {
            return Err(ZipError::Damaged("directory"));
        }
        let mut cd = vec![0u8; cd_size as usize];
        file.seek(SeekFrom::Start(cd_off))?;
        file.read_exact(&mut cd)?;
        let mut entries = Vec::with_capacity(count as usize);
        let mut p = 0usize;
        for _ in 0..count {
            if p + 46 > cd.len() || u32le(&cd, p) != 0x0201_4b50 {
                return Err(ZipError::Damaged("directory entry"));
            }
            let made_by_os = cd[p + 5];
            let flags = u16le(&cd, p + 8);
            let method = u16le(&cd, p + 10);
            let crc = u32le(&cd, p + 16);
            let mut compressed = u32le(&cd, p + 20) as u64;
            let mut size = u32le(&cd, p + 24) as u64;
            let name_len = u16le(&cd, p + 28) as usize;
            let extra_len = u16le(&cd, p + 30) as usize;
            let comment_len = u16le(&cd, p + 32) as usize;
            let ext_attr = u32le(&cd, p + 38);
            let mut local_offset = u32le(&cd, p + 42) as u64;
            let end = p + 46 + name_len + extra_len + comment_len;
            if end > cd.len() {
                return Err(ZipError::Damaged("directory entry"));
            }
            let raw_name = &cd[p + 46..p + 46 + name_len];
            let extra = &cd[p + 46 + name_len..p + 46 + name_len + extra_len];
            // ZIP64 sizes and offset, in that order, only for fields that are 0xFFFFFFFF.
            let mut q = 0;
            while q + 4 <= extra.len() {
                let id = u16le(extra, q);
                let sz = u16le(extra, q + 2) as usize;
                if q + 4 + sz > extra.len() {
                    break;
                }
                if id == 0x0001 {
                    let mut f = q + 4;
                    let lim = q + 4 + sz;
                    if size == 0xFFFF_FFFF && f + 8 <= lim {
                        size = u64le(extra, f);
                        f += 8;
                    }
                    if compressed == 0xFFFF_FFFF && f + 8 <= lim {
                        compressed = u64le(extra, f);
                        f += 8;
                    }
                    if local_offset == 0xFFFF_FFFF && f + 8 <= lim {
                        local_offset = u64le(extra, f);
                    }
                }
                q += 4 + sz;
            }
            let name = decode_name(raw_name, flags & 0x0800 != 0);
            if flags & 0x0001 != 0 {
                return Err(ZipError::Unsupported(format!("password protection on {name}")));
            }
            let rel = safe_relative(&name)?;
            let is_dir = name.ends_with('/') || name.ends_with('\\');
            // Unix-made zips keep the file type in the high bits: 0o120000 is a symbolic link.
            let is_link = made_by_os == 3 && (ext_attr >> 16) & 0o170000 == 0o120000;
            entries.push(Entry {
                name: rel.to_string_lossy().replace('\\', "/"),
                is_dir,
                method,
                crc,
                compressed,
                size,
                local_offset,
                is_link,
            });
            p = end;
        }
        Ok(Zip { file, entries })
    }

    /// Reads one file's contents.
    pub fn read(&mut self, e: &Entry, limits: &Limits) -> Result<Vec<u8>, ZipError> {
        if e.size > (64 << 20) && e.size / e.compressed.max(1) > limits.ratio {
            return Err(ZipError::TooBig(e.name.clone()));
        }
        let mut h = [0u8; 30];
        self.file.seek(SeekFrom::Start(e.local_offset))?;
        self.file.read_exact(&mut h)?;
        if u32le(&h, 0) != 0x0403_4b50 {
            return Err(ZipError::Damaged("file header"));
        }
        let skip = u16le(&h, 26) as u64 + u16le(&h, 28) as u64;
        self.file.seek(SeekFrom::Start(e.local_offset + 30 + skip))?;
        let mut data = vec![0u8; usize::try_from(e.compressed).map_err(|_| ZipError::TooBig(e.name.clone()))?];
        self.file.read_exact(&mut data)?;
        let out = match e.method {
            0 => data,
            8 => {
                let limit = usize::try_from(e.size).map_err(|_| ZipError::TooBig(e.name.clone()))?;
                let out = inflate(&data, limit).map_err(|err| ZipError::Inflate(e.name.clone(), err))?;
                if out.len() as u64 != e.size {
                    return Err(ZipError::Damaged("file size"));
                }
                out
            }
            m => return Err(ZipError::Unsupported(format!("compression method {m} on {}", e.name))),
        };
        if crc32(&out) != e.crc {
            return Err(ZipError::Checksum(e.name.clone()));
        }
        Ok(out)
    }

    /// Unpacks everything into `dest` (which must not exist yet). Unpacks into a temporary folder next to it first and
    /// renames it at the end, so `dest` is either complete or absent. Links are skipped. Returns the files written.
    pub fn extract_all(&mut self, dest: &Path, limits: &Limits) -> Result<Vec<PathBuf>, ZipError> {
        let total: u64 = self.entries.iter().filter(|e| !e.is_dir).map(|e| e.size).sum();
        if total > limits.total {
            return Err(ZipError::TooBig("The archive".into()));
        }
        let parent = dest.parent().ok_or(ZipError::Damaged("destination"))?;
        fs::create_dir_all(parent)?;
        let tmp = parent.join(format!(
            ".{}.unpacking",
            dest.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp)?;
        let result = (|| {
            let mut written = Vec::new();
            let entries = self.entries.clone();
            for e in &entries {
                let rel = safe_relative(&e.name)?;
                if rel.as_os_str().is_empty() {
                    continue;
                }
                let target = tmp.join(&rel);
                if e.is_dir {
                    fs::create_dir_all(&target)?;
                    continue;
                }
                if e.is_link {
                    continue;
                }
                let data = self.read(e, limits)?;
                if let Some(p) = target.parent() {
                    fs::create_dir_all(p)?;
                }
                let mut f = File::create(&target)?;
                f.write_all(&data)?;
                f.sync_all()?;
                written.push(rel);
            }
            Ok(written)
        })();
        match result {
            Ok(written) => {
                fs::rename(&tmp, dest)?;
                Ok(written)
            }
            Err(e) => {
                let _ = fs::remove_dir_all(&tmp);
                Err(e)
            }
        }
    }
}

/// CRC-32 (IEEE), as zip uses it.
pub fn crc32(data: &[u8]) -> u32 {
    static TABLE: std::sync::OnceLock<[u32; 256]> = std::sync::OnceLock::new();
    let t = TABLE.get_or_init(|| {
        let mut t = [0u32; 256];
        for (i, v) in t.iter_mut().enumerate() {
            let mut c = i as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            }
            *v = c;
        }
        t
    });
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c = t[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_matches_known_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn refuses_unsafe_paths() {
        for bad in ["../x", "a/../../x", "/etc/passwd", "C:/x", "a\\..\\..\\x"] {
            assert!(safe_relative(bad).is_err(), "{bad}");
        }
        assert_eq!(safe_relative("Data\\meshes/./a.nif").unwrap(), PathBuf::from("Data/meshes/a.nif"));
    }

    #[test]
    fn decodes_cp437_names() {
        assert_eq!(decode_name(&[b'a', 0x81, b'b'], false), "aüb");
    }
}
