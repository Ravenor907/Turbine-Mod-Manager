//! Puts mod files into a game's folder and takes them out again, safely.
//!
//! Rules:
//! - Every file Turbine places is written down in a record (with its inode, size and time), so Turbine only ever
//!   removes files it placed. Files a player or the game made are never touched.
//! - A game file a mod replaces is moved to a backup folder first, and put back when the mod goes. Backup names are
//!   never reused within a plan and a move never lands on a path that already exists.
//! - Mods are hard-linked where a real link works. A mod file that replaces a game file, and any settings-like file
//!   (.cfg, .ini, .json, .toml, .yaml, .yml, .xml, .txt, or anything in a folder named config) is copied instead, so
//!   the game writing to it can never change the mod's own copy. A settings copy the player changed is kept when the
//!   mod goes and put back the next time it goes in.
//! - Only plain relative paths are used, and nothing is ever written through a linked folder inside the game. A mod
//!   file never replaces a game folder.
//! - Before any change, the whole plan is written to disk. If Turbine stops halfway (crash, power loss), the next start
//!   reads the plan, looks at what's really on disk, and finishes the record so nothing is lost or left untracked.
//!   Every path read back from the plan or the record is checked first; entries that fail are never acted on.
//! - One deploy at a time per game (a lock file next to the record). A missing game folder (SD card out) is refused.
//! - The record is kept twice (record.json and record.json.prev). `reset_from_backups` is always a way back.

use crate::json::{self, quote, Json};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ffi::OsString;
use std::fs::{self, File, TryLockError};
use std::io::{self, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
use std::time::SystemTime;

/// One file Turbine placed in the game folder.
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    /// Path inside the game folder, with the casing actually used on disk.
    pub rel: String,
    /// The staged file it came from.
    pub source: PathBuf,
    pub copy: bool,
    pub ino: u64,
    pub size: u64,
    pub mtime: i64,
    /// Where the game's own file went, if this one replaced it.
    pub backup: Option<PathBuf>,
    /// The staged file as it was when placed (inode, size, time); all 0 when not known.
    pub src_ino: u64,
    pub src_size: u64,
    pub src_mtime: i64,
}

/// Everything Turbine has placed in one game folder.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Record {
    pub files: Vec<Placed>,
    /// Folders Turbine created (removed again only when empty).
    pub dirs: Vec<String>,
    /// Entries that couldn't be read safely, kept as they were (JSON text). Never acted on by a deploy; only
    /// `reset_from_backups` looks at them.
    pub quarantine: Vec<String>,
}

/// Where a game's deploy bookkeeping lives.
#[derive(Debug, Clone)]
pub struct Paths {
    pub game: PathBuf,
    pub backups: PathBuf,
    pub record: PathBuf,
    pub plan: PathBuf,
}

#[derive(Debug, Default, PartialEq)]
pub struct Report {
    pub added: usize,
    pub removed: usize,
    pub kept: usize,
    /// Files Turbine placed that something else changed since; they were left alone and dropped from the record.
    pub changed_by_others: Vec<String>,
    /// True when a stopped deploy was found and finished first.
    pub recovered: bool,
    /// Files that were meant to be hard links but had to be copied (the staging folder and the game are on different
    /// drives or mounts). Each one uses its full size on disk again.
    pub copied_not_linked: usize,
    /// Mod files that were missing or replaced (Steam checking the game's files, an update) and were put back.
    pub put_back: Vec<String>,
    /// Settings files the player changed while the mod was in; kept to go back in next time.
    pub player_changes_saved: Vec<String>,
    /// Settings files put back in with the player's own changes.
    pub player_changes_restored: Vec<String>,
    /// Entries in the record or plan that couldn't be read safely, found this time. They were set aside, not used.
    pub bad_entries: Vec<String>,
    /// How many set-aside record entries are waiting (only `reset_from_backups` deals with them).
    pub quarantined: usize,
    /// Game files still in the backup folder because something else is now at their place (reset only).
    pub left_in_backups: Vec<String>,
    /// Files a mod has that the game already has (and Turbine didn't place), left alone because that mod may not
    /// replace the game's files (`Wanted::may_replace`).
    pub skipped_existing: Vec<String>,
    /// Files never placed whatever the player chose (anti-cheat, Steam's own files; see basic::refused). Filled in by
    /// the store, which knows which mods are Basic.
    pub refused: Vec<String>,
}

/// One file a caller wants in the game folder.
#[derive(Debug, Clone, PartialEq)]
pub struct Wanted {
    /// Path inside the game folder.
    pub rel: String,
    /// The staged file.
    pub source: PathBuf,
    /// May it replace a file the game already has (one Turbine didn't place)? The game's file is kept in the backup
    /// folder and put back when the mod goes. When false, such a file is skipped and listed in
    /// `Report::skipped_existing`.
    pub may_replace: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum Op {
    Add { rel: String, source: PathBuf, copy: bool, backup: Option<PathBuf> },
    /// `again`: the same file goes back in right after (it went missing or was replaced).
    Remove { placed: Placed, again: bool },
}

fn plain(kind: io::ErrorKind, msg: impl Into<String>) -> io::Error {
    io::Error::new(kind, msg.into())
}

fn lower(s: &str) -> String {
    s.to_lowercase()
}

fn mtime_of(m: &fs::Metadata) -> i64 {
    m.mtime() * 1_000_000_000 + m.mtime_nsec()
}

fn exists(p: &Path) -> bool {
    fs::symlink_metadata(p).is_ok()
}

fn is_file(p: &Path) -> bool {
    fs::symlink_metadata(p).map(|m| m.is_file()).unwrap_or(false)
}

/// A file or a link (a game can have links as files; they are moved as they are, never followed).
fn is_file_or_link(p: &Path) -> bool {
    fs::symlink_metadata(p).map(|m| m.is_file() || m.file_type().is_symlink()).unwrap_or(false)
}

/// A plain relative path inside a folder: not empty, not absolute, no empty, "." or ".." parts.
pub fn safe_rel(rel: &str) -> bool {
    !rel.is_empty() && !rel.starts_with('/') && rel.split('/').all(|p| !p.is_empty() && p != "." && p != ".." && !p.contains('\0'))
}

/// Is `b` a path inside the backups folder (and not the folder itself)?
fn backup_ok(paths: &Paths, b: &Path) -> bool {
    match b.strip_prefix(&paths.backups) {
        Ok(r) => r.components().next().is_some() && r.components().all(|c| matches!(c, Component::Normal(_))),
        Err(_) => false,
    }
}

/// Settings-like files are always copied, never linked: games and loaders write to them in place.
pub fn config_like(rel: &str) -> bool {
    let l = lower(rel);
    let parts: Vec<&str> = l.split('/').collect();
    let (name, dirs) = parts.split_last().unwrap_or((&"", &[]));
    let ext = name.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
    dirs.contains(&"config") || matches!(ext, "cfg" | "ini" | "json" | "toml" | "yaml" | "yml" | "xml" | "txt")
}

/// Is the file at `target` still the one Turbine placed?
fn is_ours(p: &Placed, target: &Path) -> bool {
    match fs::symlink_metadata(target) {
        Ok(m) if m.is_file() => {
            let same = m.len() == p.size && mtime_of(&m) == p.mtime;
            if p.copy {
                same
            } else {
                same && m.ino() == p.ino
            }
        }
        _ => false,
    }
}

/// Checks every folder between the game folder and `rel`'s file. Ok(true): all there as real folders. Ok(false):
/// some are missing (the rest are fine). An error when one is a link or isn't a folder: nothing may go through it.
/// Folders in `going` (lowercase paths of files this plan removes first) count as missing.
fn check_dirs(game: &Path, rel: &str, going: Option<&HashSet<String>>) -> io::Result<bool> {
    let parts: Vec<&str> = rel.split('/').collect();
    let mut dir = game.to_path_buf();
    let mut acc = String::new();
    for part in &parts[..parts.len().saturating_sub(1)] {
        dir.push(part);
        if !acc.is_empty() {
            acc.push('/');
        }
        acc.push_str(part);
        match fs::symlink_metadata(&dir) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(plain(
                    io::ErrorKind::InvalidInput,
                    format!("Turbine won't put files through {acc} in the game's folder: it's a link to another place, so files would land outside the game."),
                ))
            }
            Ok(m) if m.is_dir() => {}
            Ok(_) if going.is_some_and(|g| g.contains(&lower(&acc))) => return Ok(false),
            Ok(_) => {
                return Err(plain(
                    io::ErrorKind::InvalidInput,
                    format!("{rel} can't go in: {acc} is a file in the game's folder, not a folder."),
                ))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(e),
        }
    }
    Ok(true)
}

/// Creates the missing folders on the way to `rel` (never through a link). Returns the ones it made.
fn make_dirs(game: &Path, rel: &str) -> io::Result<Vec<String>> {
    check_dirs(game, rel, None)?;
    let parts: Vec<&str> = rel.split('/').collect();
    let mut made = Vec::new();
    let mut acc = String::new();
    for part in &parts[..parts.len().saturating_sub(1)] {
        if !acc.is_empty() {
            acc.push('/');
        }
        acc.push_str(part);
        let d = game.join(&acc);
        match fs::symlink_metadata(&d) {
            Ok(m) if m.is_dir() => {}
            Ok(_) => return Err(plain(io::ErrorKind::InvalidInput, format!("{acc} in the game's folder isn't a folder any more."))),
            Err(_) => {
                fs::create_dir(&d)?;
                made.push(acc.clone());
            }
        }
    }
    Ok(made)
}

/// Finds the casing already used on disk for each part of `rel` (Windows mods ignore case; Linux doesn't).
struct CaseMap<'a> {
    root: &'a Path,
    cache: HashMap<PathBuf, HashMap<String, String>>,
}

impl<'a> CaseMap<'a> {
    fn new(root: &'a Path) -> Self {
        CaseMap { root, cache: HashMap::new() }
    }

    fn resolve(&mut self, rel: &str, planned: &HashMap<String, String>) -> String {
        let mut out = String::new();
        let mut dir = self.root.to_path_buf();
        for part in rel.split('/').filter(|p| !p.is_empty()) {
            let key_prefix = if out.is_empty() { lower(part) } else { format!("{}/{}", lower(&out), lower(part)) };
            let names = self.cache.entry(dir.clone()).or_insert_with(|| {
                let mut m = HashMap::new();
                if let Ok(rd) = fs::read_dir(&dir) {
                    for e in rd.flatten() {
                        let n = e.file_name().to_string_lossy().into_owned();
                        m.entry(lower(&n)).or_insert(n);
                    }
                }
                m
            });
            // On disk first, then a casing already chosen earlier in this same plan.
            let actual = names
                .get(&lower(part))
                .cloned()
                .or_else(|| planned.get(&key_prefix).map(|full| full.rsplit('/').next().unwrap_or(part).to_string()))
                .unwrap_or_else(|| part.to_string());
            if !out.is_empty() {
                out.push('/');
            }
            out.push_str(&actual);
            dir.push(&actual);
        }
        out
    }
}

// ------------------------------------------------------------------ record and plan files

/// JSON text for a value read earlier (quarantined entries are written back as they were).
fn to_text(v: &Json) -> String {
    match v {
        Json::Null => "null".into(),
        Json::Bool(b) => b.to_string(),
        Json::Num(n) if n.is_finite() => format!("{n}"),
        Json::Num(_) => "null".into(),
        Json::Str(s) => quote(s),
        Json::Arr(items) => format!("[{}]", items.iter().map(to_text).collect::<Vec<_>>().join(",")),
        Json::Obj(items) => format!("{{{}}}", items.iter().map(|(k, v)| format!("{}:{}", quote(k), to_text(v))).collect::<Vec<_>>().join(",")),
    }
}

fn placed_json(p: &Placed) -> String {
    format!(
        "{{\"rel\":{},\"source\":{},\"copy\":{},\"ino\":\"{}\",\"size\":\"{}\",\"mtime\":\"{}\",\"backup\":{},\"srcIno\":\"{}\",\"srcSize\":\"{}\",\"srcMtime\":\"{}\"}}",
        quote(&p.rel),
        quote(&p.source.to_string_lossy()),
        p.copy,
        p.ino,
        p.size,
        p.mtime,
        p.backup.as_ref().map(|b| quote(&b.to_string_lossy())).unwrap_or_else(|| "null".into()),
        p.src_ino,
        p.src_size,
        p.src_mtime
    )
}

/// A whole number written as a JSON number or a string (large numbers are stored as strings to survive exactly).
fn int(v: Option<&Json>) -> Option<i128> {
    match v? {
        Json::Num(n) if n.fract() == 0.0 && n.abs() < 9.0e15 => Some(*n as i128),
        Json::Str(s) if !s.is_empty() && s.trim() == s => s.parse::<i128>().ok(),
        _ => None,
    }
}

fn int_u64(v: Option<&Json>) -> Option<u64> {
    int(v).and_then(|n| u64::try_from(n).ok())
}

fn int_i64(v: Option<&Json>) -> Option<i64> {
    int(v).and_then(|n| i64::try_from(n).ok())
}

/// An optional number: absent or null is 0, anything else must read.
fn opt_u64(v: Option<&Json>) -> Option<u64> {
    match v {
        None | Some(Json::Null) => Some(0),
        _ => int_u64(v),
    }
}

fn opt_i64(v: Option<&Json>) -> Option<i64> {
    match v {
        None | Some(Json::Null) => Some(0),
        _ => int_i64(v),
    }
}

/// Reads one record entry strictly. The error says why it can't be used.
fn placed_from(v: &Json, backups: Option<&Paths>) -> Result<Placed, String> {
    let rel = v.str_at("rel").ok_or("no path")?;
    if !safe_rel(rel) {
        return Err(format!("path {rel:?} isn't a plain path inside the game's folder"));
    }
    let source = match v.get("source") {
        Some(Json::Str(s)) => PathBuf::from(s),
        None | Some(Json::Null) => PathBuf::new(),
        _ => return Err("its source can't be read".into()),
    };
    let copy = match v.get("copy") {
        Some(Json::Bool(b)) => *b,
        None => false,
        _ => return Err("its copy flag can't be read".into()),
    };
    let ino = int_u64(v.get("ino")).ok_or("its inode can't be read")?;
    let size = int_u64(v.get("size")).ok_or("its size can't be read")?;
    let mtime = int_i64(v.get("mtime")).ok_or("its time can't be read")?;
    let backup = match v.get("backup") {
        None | Some(Json::Null) => None,
        Some(Json::Str(s)) => Some(PathBuf::from(s)),
        _ => return Err("its backup can't be read".into()),
    };
    if let (Some(b), Some(paths)) = (&backup, backups) {
        if !backup_ok(paths, b) {
            return Err(format!("its backup {} is outside Turbine's backup folder", b.display()));
        }
    }
    let src_ino = opt_u64(v.get("srcIno")).ok_or("its source inode can't be read")?;
    let src_size = opt_u64(v.get("srcSize")).ok_or("its source size can't be read")?;
    let src_mtime = opt_i64(v.get("srcMtime")).ok_or("its source time can't be read")?;
    Ok(Placed { rel: rel.to_string(), source, copy, ino, size, mtime, backup, src_ino, src_size, src_mtime })
}

fn describe(v: &Json, why: &str) -> String {
    match v.str_at("rel") {
        Some(rel) => format!("{rel}: {why}"),
        None => format!("an entry without a usable path: {why}"),
    }
}

fn write_atomic(path: &Path, text: &str) -> io::Result<()> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    let mut name = path.file_name().map(OsString::from).unwrap_or_default();
    name.push(".tmp");
    let tmp = path.with_file_name(name);
    {
        let mut f = File::create(&tmp)?;
        f.write_all(text.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    // Make the rename itself last through a power cut (matters on SD cards).
    if let Some(p) = path.parent() {
        if let Ok(d) = File::open(p) {
            let _ = d.sync_all();
        }
    }
    Ok(())
}

fn with_suffix(p: &Path, suffix: &str) -> PathBuf {
    let mut s = p.as_os_str().to_os_string();
    s.push(suffix);
    PathBuf::from(s)
}

/// The second copy of the record, kept in step with it.
pub fn prev_path(record: &Path) -> PathBuf {
    with_suffix(record, ".prev")
}

fn damaged(e: impl std::fmt::Display) -> io::Error {
    plain(io::ErrorKind::InvalidData, e.to_string())
}

/// Reads a record. With `paths`, backups must be inside its backup folder. Bad entries go to `quarantine` and are
/// described in `bad`. A file that isn't a record at all is an error (InvalidData).
fn parse_record(text: &str, paths: Option<&Paths>, bad: &mut Vec<String>) -> io::Result<Record> {
    let v = json::parse(text).map_err(damaged)?;
    if !matches!(v, Json::Obj(_)) {
        return Err(damaged("the record isn't an object"));
    }
    let files = match v.get("files") {
        Some(Json::Arr(items)) => items.as_slice(),
        None => &[],
        _ => return Err(damaged("the record's file list can't be read")),
    };
    let mut rec = Record::default();
    let mut seen = HashSet::new();
    for f in files {
        match placed_from(f, paths) {
            Ok(p) if seen.insert(lower(&p.rel)) => rec.files.push(p),
            Ok(p) => {
                bad.push(format!("{}: listed twice", p.rel));
                rec.quarantine.push(to_text(f));
            }
            Err(why) => {
                bad.push(describe(f, &why));
                rec.quarantine.push(to_text(f));
            }
        }
    }
    for d in v.get("dirs").map(Json::as_array).unwrap_or(&[]) {
        match d.as_str() {
            Some(s) if safe_rel(s) => {
                if !rec.dirs.iter().any(|x| x == s) {
                    rec.dirs.push(s.to_string());
                }
            }
            _ => bad.push(format!("a folder entry {} isn't a plain path inside the game's folder", to_text(d))),
        }
    }
    // Entries set aside earlier stay set aside.
    for q in v.get("quarantine").map(Json::as_array).unwrap_or(&[]) {
        rec.quarantine.push(to_text(q));
    }
    Ok(rec)
}

impl Record {
    /// Reads a record file. A missing file is an empty record; a damaged one is an error (InvalidData). Entries that
    /// can't be read safely are left out of `files` and kept in `quarantine`.
    pub fn load(path: &Path) -> io::Result<Record> {
        let text = match fs::read(path) {
            Ok(b) => String::from_utf8(b).map_err(damaged)?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Record::default()),
            Err(e) => return Err(e),
        };
        parse_record(&text, None, &mut Vec::new())
    }

    fn text(&self) -> String {
        let files: Vec<String> = self.files.iter().map(placed_json).collect();
        let dirs: Vec<String> = self.dirs.iter().map(|d| quote(d)).collect();
        format!(
            "{{\"files\":[\n{}\n],\"dirs\":[{}],\n\"quarantine\":[{}]}}\n",
            files.join(",\n"),
            dirs.join(","),
            self.quarantine.join(",\n")
        )
    }

    /// Writes the record and its second copy (record.json.prev).
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let text = self.text();
        write_atomic(path, &text)?;
        write_atomic(&prev_path(path), &text)
    }
}

/// Loads the game's record for a deploy. A damaged record is set aside (record.json.damaged) and the second copy
/// takes its place, but this deploy still stops so nothing is done on a guess; the next one goes ahead. With no usable
/// second copy, every deploy stops until `reset_from_backups`.
fn load_record(paths: &Paths, report: &mut Report) -> io::Result<Record> {
    let prev = prev_path(&paths.record);
    let read = |p: &Path, bad: &mut Vec<String>| -> io::Result<Option<Record>> {
        match fs::read(p) {
            Ok(b) => {
                let text = String::from_utf8(b).map_err(damaged)?;
                parse_record(&text, Some(paths), bad).map(Some)
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    };
    let mut bad = Vec::new();
    match read(&paths.record, &mut bad) {
        Ok(Some(r)) => {
            report.bad_entries.append(&mut bad);
            report.quarantined = r.quarantine.len();
            Ok(r)
        }
        Ok(None) if !exists(&prev) => Ok(Record::default()),
        Ok(None) | Err(_) => {
            let first = read(&paths.record, &mut Vec::new());
            if let Err(e) = &first {
                if e.kind() != io::ErrorKind::InvalidData {
                    return Err(first.unwrap_err());
                }
            }
            match fs::read(&prev).ok().and_then(|b| String::from_utf8(b).ok()) {
                Some(text) if parse_record(&text, Some(paths), &mut Vec::new()).is_ok() => {
                    if exists(&paths.record) {
                        fs::rename(&paths.record, with_suffix(&paths.record, ".damaged"))?;
                    }
                    write_atomic(&paths.record, &text)?;
                    Err(plain(
                        io::ErrorKind::InvalidData,
                        "Turbine's list of the files it put in this game was damaged. It went back to its saved copy; try again.",
                    ))
                }
                _ => Err(plain(
                    io::ErrorKind::InvalidData,
                    "Turbine's list of the files it put in this game is damaged, and so is its saved copy. Use Reset to put the game's own files back.",
                )),
            }
        }
    }
}

fn op_json(op: &Op) -> String {
    match op {
        Op::Add { rel, source, copy, backup } => format!(
            "{{\"op\":\"add\",\"rel\":{},\"source\":{},\"copy\":{},\"backup\":{}}}",
            quote(rel),
            quote(&source.to_string_lossy()),
            copy,
            backup.as_ref().map(|b| quote(&b.to_string_lossy())).unwrap_or_else(|| "null".into())
        ),
        Op::Remove { placed, again } => format!("{{\"op\":\"remove\",\"again\":{again},\"placed\":{}}}", placed_json(placed)),
    }
}

/// Reads one plan step strictly; the error says why it can't be used.
fn op_from(v: &Json, paths: &Paths) -> Result<Op, String> {
    match v.str_at("op") {
        Some("add") => {
            let rel = v.str_at("rel").ok_or("no path")?;
            if !safe_rel(rel) {
                return Err(format!("path {rel:?} isn't a plain path inside the game's folder"));
            }
            let source = PathBuf::from(v.str_at("source").ok_or("no source")?);
            let copy = match v.get("copy") {
                Some(Json::Bool(b)) => *b,
                None => false,
                _ => return Err("its copy flag can't be read".into()),
            };
            let backup = match v.get("backup") {
                None | Some(Json::Null) => None,
                Some(Json::Str(s)) => Some(PathBuf::from(s)),
                _ => return Err("its backup can't be read".into()),
            };
            if let Some(b) = &backup {
                if !backup_ok(paths, b) {
                    return Err(format!("its backup {} is outside Turbine's backup folder", b.display()));
                }
            }
            Ok(Op::Add { rel: rel.to_string(), source, copy, backup })
        }
        Some("remove") => {
            let placed = placed_from(v.get("placed").ok_or("nothing to remove")?, Some(paths))?;
            Ok(Op::Remove { placed, again: v.get("again").and_then(Json::as_bool).unwrap_or(false) })
        }
        _ => Err("not a step Turbine knows".into()),
    }
}

// ------------------------------------------------------------------ file moves

/// Moves a file, copying across drives when a rename isn't possible. Never replaces anything at `to`.
fn move_file(from: &Path, to: &Path) -> io::Result<()> {
    if exists(to) {
        return Err(plain(io::ErrorKind::AlreadyExists, format!("{} is already there; Turbine won't write over it.", to.display())));
    }
    if let Some(p) = to.parent() {
        fs::create_dir_all(p)?;
    }
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(e) if e.raw_os_error() == Some(18) && fs::symlink_metadata(from)?.file_type().is_symlink() => {
            std::os::unix::fs::symlink(fs::read_link(from)?, to)?;
            fs::remove_file(from)
        }
        Err(e) if e.raw_os_error() == Some(18) => {
            // EXDEV: different drives or mounts.
            let tmp = temp_name(to);
            let _ = remove_if_file(&tmp);
            let r = (|| {
                fs::copy(from, &tmp)?;
                let f = File::options().write(true).open(&tmp)?;
                if let Ok(t) = fs::metadata(from).and_then(|m| m.modified()) {
                    f.set_modified(t)?;
                }
                f.sync_all()?;
                if exists(to) {
                    return Err(plain(io::ErrorKind::AlreadyExists, format!("{} appeared; Turbine won't write over it.", to.display())));
                }
                fs::rename(&tmp, to)
            })();
            if r.is_err() {
                let _ = remove_if_file(&tmp);
            }
            r?;
            fs::remove_file(from)
        }
        Err(e) => Err(e),
    }
}

fn remove_if_file(p: &Path) -> io::Result<()> {
    match fs::symlink_metadata(p) {
        Ok(m) if m.is_file() => fs::remove_file(p),
        _ => Ok(()),
    }
}

/// A short temporary name next to `target` (a long file name plus an ending would not fit).
fn temp_name(target: &Path) -> PathBuf {
    target.with_file_name(format!(".turbine-tmp-{}", std::process::id()))
}

/// Removes temporary files a stopped deploy may have left next to `target`: only names Turbine makes.
fn clean_temp(target: &Path) {
    let _ = remove_if_file(&target.with_extension("turbine-tmp"));
    let Some(dir) = target.parent() else { return };
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let n = e.file_name().to_string_lossy().into_owned();
        if let Some(num) = n.strip_prefix(".turbine-tmp-") {
            if !num.is_empty() && num.bytes().all(|b| b.is_ascii_digit()) {
                let _ = remove_if_file(&e.path());
            }
        }
    }
}

/// Removes link tests left at the top of the game folder by a stop.
fn clean_link_tests(game: &Path) {
    let Ok(rd) = fs::read_dir(game) else { return };
    for e in rd.flatten() {
        let n = e.file_name().to_string_lossy().into_owned();
        if let Some(num) = n.strip_prefix(".turbine-link-test-") {
            if !num.is_empty() && num.bytes().all(|b| b.is_ascii_digit()) {
                let _ = remove_if_file(&e.path());
            }
        }
    }
}

/// Can a staged file be hard linked into the game folder? Tried for real: inside a Flatpak, two folders on the same
/// drive can still be separate mounts, and links between them fail.
fn try_link(source: &Path, game: &Path) -> Option<bool> {
    let t = game.join(format!(".turbine-link-test-{}", std::process::id()));
    let _ = remove_if_file(&t);
    match fs::hard_link(source, &t) {
        Ok(()) => {
            let _ = fs::remove_file(&t);
            Some(true)
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(_) => Some(false),
    }
}

/// Where the player's own version of a settings file is kept while its mod is out.
fn changes_path(paths: &Paths, source: &Path) -> Option<PathBuf> {
    if !source.is_absolute() {
        return None;
    }
    let mut out = paths.record.parent()?.join("player-changes");
    for c in source.components() {
        match c {
            Component::RootDir => {}
            Component::Normal(n) => out.push(n),
            _ => return None,
        }
    }
    Some(out)
}

// ------------------------------------------------------------------ planning and applying

/// Hands out backup names: never one given out earlier in the same plan, never one already on disk.
struct BackupNames {
    given: HashSet<PathBuf>,
    /// Folders on the way to the names given out.
    under: HashSet<PathBuf>,
}

impl BackupNames {
    fn new() -> Self {
        BackupNames { given: HashSet::new(), under: HashSet::new() }
    }

    fn reserve(&mut self, base: &Path, p: &Path) {
        self.given.insert(p.to_path_buf());
        let mut a = p.parent();
        while let Some(d) = a {
            if d == base || !d.starts_with(base) {
                break;
            }
            self.under.insert(d.to_path_buf());
            a = d.parent();
        }
    }

    fn free(&self, base: &Path, cand: &Path) -> bool {
        if exists(cand) || self.given.contains(cand) || self.under.contains(cand) {
            return false;
        }
        let mut a = cand.parent();
        while let Some(d) = a {
            if d == base || !d.starts_with(base) {
                break;
            }
            if self.given.contains(d) {
                return false;
            }
            if let Ok(m) = fs::symlink_metadata(d) {
                if !m.is_dir() {
                    return false;
                }
            }
            a = d.parent();
        }
        true
    }

    fn take(&mut self, base: &Path, rel: &str) -> PathBuf {
        let mut n = 0u32;
        loop {
            let cand = if n == 0 { base.join(rel) } else { base.join(format!(".turbine-{n}")).join(rel) };
            if self.free(base, &cand) {
                self.reserve(base, &cand);
                return cand;
            }
            n += 1;
        }
    }
}

struct Plan {
    ops: Vec<Op>,
    kept: usize,
    new_dirs: Vec<String>,
    copied_not_linked: usize,
    put_back: Vec<String>,
}

/// Checks the paths a caller wants placed: plain relative paths only, and no file that is also a folder.
fn check_wanted(wanted: &[(String, PathBuf)]) -> io::Result<()> {
    let mut folders = HashSet::new();
    let mut files = HashSet::new();
    for (rel, _) in wanted {
        if !safe_rel(rel) {
            return Err(plain(io::ErrorKind::InvalidInput, format!("Turbine won't place {rel:?}: it isn't a plain path inside the game's folder.")));
        }
        let l = lower(rel);
        let parts: Vec<&str> = l.split('/').collect();
        for i in 1..parts.len() {
            folders.insert(parts[..i].join("/"));
        }
        files.insert(l);
    }
    if let Some(both) = files.iter().find(|f| folders.contains(*f)) {
        return Err(plain(io::ErrorKind::InvalidInput, format!("Two mods disagree: one has a file called {both}, another has a folder with that name.")));
    }
    Ok(())
}

/// Works out what has to change so the game folder holds exactly `wanted` (path inside the game folder, staged file),
/// later entries winning over earlier ones with the same path.
fn plan(paths: &Paths, record: &Record, wanted: &[(String, PathBuf)]) -> io::Result<Plan> {
    check_wanted(wanted)?;
    // Final choice per path, case-insensitive, in order (later wins).
    let mut chosen: BTreeMap<String, (String, PathBuf)> = BTreeMap::new();
    for (rel, src) in wanted {
        chosen.insert(lower(rel), (rel.clone(), src.clone()));
    }
    let current: HashMap<String, &Placed> = record.files.iter().map(|p| (lower(&p.rel), p)).collect();
    let mut names = BackupNames::new();
    for p in &record.files {
        if let Some(b) = &p.backup {
            names.reserve(&paths.backups, b);
        }
    }
    let mut ops = Vec::new();
    let mut kept = 0;
    let mut put_back = Vec::new();
    // Removals first: anything placed that is no longer wanted, now comes from a different file, or is no longer
    // there as Turbine left it (missing, or replaced by a new file).
    let mut again: HashSet<String> = HashSet::new();
    for p in &record.files {
        let key = lower(&p.rel);
        match chosen.get(&key) {
            Some((_, src)) if *src == p.source => {
                let target = paths.game.join(&p.rel);
                // A settings file the player or the loader changed: their edit, left as it is.
                let edited_settings = p.copy && p.backup.is_none() && config_like(&p.rel) && is_file(&target);
                if is_ours(p, &target) || edited_settings {
                    kept += 1;
                } else {
                    put_back.push(p.rel.clone());
                    again.insert(key);
                    ops.push(Op::Remove { placed: p.clone(), again: true });
                }
            }
            _ => ops.push(Op::Remove { placed: p.clone(), again: false }),
        }
    }
    let going: HashSet<String> = ops.iter().filter_map(|o| if let Op::Remove { placed, .. } = o { Some(lower(&placed.rel)) } else { None }).collect();
    let mut case = CaseMap::new(&paths.game);
    let mut planned: HashMap<String, String> = HashMap::new();
    let mut can_link: HashMap<(u64, u64), bool> = HashMap::new();
    let game_dev = fs::metadata(&paths.game)?.dev();
    let mut copied_not_linked = 0;
    for (key, (rel, src)) in &chosen {
        if let Some(p) = current.get(key) {
            if p.source == *src && !again.contains(key) {
                continue;
            }
        }
        let actual = case.resolve(rel, &planned);
        let lk = lower(&actual);
        // Remember the chosen casing of every folder on the way, for later files in the same plan.
        let mut acc = String::new();
        for part in actual.split('/') {
            if !acc.is_empty() {
                acc.push('/');
            }
            acc.push_str(part);
            planned.entry(lower(&acc)).or_insert_with(|| acc.clone());
        }
        let all_dirs_there = check_dirs(&paths.game, &actual, Some(&going))?;
        let target = paths.game.join(&actual);
        let meta = if all_dirs_there { fs::symlink_metadata(&target).ok() } else { None };
        if meta.as_ref().is_some_and(|m| m.is_dir()) {
            return Err(plain(
                io::ErrorKind::InvalidInput,
                format!("{actual} can't go in: the game has a folder with that name, and Turbine never moves the game's folders."),
            ));
        }
        // A game file is here, or will be once the removals before this step are done: it gets backed up.
        let replaces_game_file = match current.get(&lk) {
            Some(p) if is_ours(p, &target) => p.backup.as_ref().is_some_and(|b| exists(b)),
            Some(p) => meta.is_some() || p.backup.as_ref().is_some_and(|b| exists(b)),
            None => meta.is_some(),
        };
        let backup = replaces_game_file.then(|| names.take(&paths.backups, &actual));
        let mut copy = replaces_game_file || config_like(&actual);
        if !copy {
            let linkable = match fs::metadata(src) {
                Ok(m) => match can_link.get(&(m.dev(), game_dev)) {
                    Some(v) => *v,
                    None => match try_link(src, &paths.game) {
                        Some(v) => {
                            can_link.insert((m.dev(), game_dev), v);
                            v
                        }
                        None => true,
                    },
                },
                // Missing: the add fails and says so.
                Err(_) => true,
            };
            if !linkable {
                copy = true;
                copied_not_linked += 1;
            }
        }
        ops.push(Op::Add { rel: actual, source: src.clone(), copy, backup });
    }
    // Folders the adds will create, so a stopped deploy knows exactly which folders are Turbine's.
    let mut new_dirs: Vec<String> = Vec::new();
    for op in &ops {
        if let Op::Add { rel, .. } = op {
            let parts: Vec<&str> = rel.split('/').collect();
            let mut acc = String::new();
            for part in &parts[..parts.len().saturating_sub(1)] {
                if !acc.is_empty() {
                    acc.push('/');
                }
                acc.push_str(part);
                if !exists(&paths.game.join(&acc)) && !new_dirs.contains(&acc) {
                    new_dirs.push(acc.clone());
                }
            }
        }
    }
    Ok(Plan { ops, kept, new_dirs, copied_not_linked, put_back })
}

/// Puts one file in place. On failure the game's own file is moved back before the error is returned.
fn do_add(paths: &Paths, rel: &str, source: &Path, copy: bool, backup: &Option<PathBuf>, record: &mut Record, report: &mut Report) -> io::Result<()> {
    let target = paths.game.join(rel);
    // Folders on the way that don't exist yet are Turbine's.
    for d in make_dirs(&paths.game, rel)? {
        if !record.dirs.contains(&d) {
            record.dirs.push(d);
        }
    }
    match fs::symlink_metadata(&target) {
        Ok(m) if m.is_dir() => {
            return Err(plain(io::ErrorKind::InvalidInput, format!("{rel} can't go in: the game has a folder with that name.")));
        }
        Ok(_) => match backup {
            Some(b) => move_file(&target, b)?,
            // Something appeared here after planning. Never overwrite a file Turbine didn't plan for.
            None => return Err(plain(io::ErrorKind::AlreadyExists, format!("{rel} appeared while applying changes"))),
        },
        Err(_) => {}
    }
    let placed = place(paths, rel, source, copy, &target, report);
    match placed {
        Ok(p) => {
            record.files.push(Placed { backup: backup.clone(), ..p });
            Ok(())
        }
        Err(e) => {
            if let Some(b) = backup {
                if !exists(&target) && is_file_or_link(b) {
                    let _ = move_file(b, &target);
                }
            }
            Err(e)
        }
    }
}

fn place(paths: &Paths, rel: &str, source: &Path, copy: bool, target: &Path, report: &mut Report) -> io::Result<Placed> {
    let src_meta = fs::metadata(source)?;
    // The player's own version of this settings file, kept from last time.
    let saved = changes_path(paths, source).filter(|p| is_file(p));
    let from = saved.as_deref().unwrap_or(source);
    let mut use_copy = copy || saved.is_some();
    if !use_copy && fs::hard_link(source, target).is_err() {
        use_copy = true;
        report.copied_not_linked += 1;
    }
    if use_copy {
        let tmp = temp_name(target);
        let _ = remove_if_file(&tmp);
        let r = (|| {
            fs::copy(from, &tmp)?;
            let f = File::options().write(true).open(&tmp)?;
            f.set_modified(fs::metadata(from)?.modified().unwrap_or(SystemTime::now()))?;
            f.sync_all()?;
            if exists(target) {
                return Err(plain(io::ErrorKind::AlreadyExists, format!("{rel} appeared while applying changes")));
            }
            fs::rename(&tmp, target)
        })();
        if let Err(e) = r {
            let _ = remove_if_file(&tmp);
            return Err(e);
        }
        if saved.is_some() {
            report.player_changes_restored.push(rel.to_string());
        }
    }
    let m = fs::symlink_metadata(target)?;
    Ok(Placed {
        rel: rel.to_string(),
        source: source.to_path_buf(),
        copy: use_copy,
        ino: m.ino(),
        size: m.len(),
        mtime: mtime_of(&m),
        backup: None,
        src_ino: src_meta.ino(),
        src_size: src_meta.len(),
        src_mtime: mtime_of(&src_meta),
    })
}

/// Puts the game's file back from its backup, if the backup is there and the place is free.
fn restore_backup(paths: &Paths, rel: &str, b: &Path) -> io::Result<bool> {
    let target = paths.game.join(rel);
    if !backup_ok(paths, b) || !is_file_or_link(b) || exists(&target) {
        return Ok(false);
    }
    make_dirs(&paths.game, rel)?;
    move_file(b, &target)?;
    Ok(true)
}

fn do_remove(paths: &Paths, p: &Placed, again: bool, record: &mut Record, report: &mut Report) -> io::Result<()> {
    let target = paths.game.join(&p.rel);
    if check_dirs(&paths.game, &p.rel, None).is_err() {
        // A folder on the way became a link or a file: never go through it. The backup stays in the backup folder.
        report.changed_by_others.push(p.rel.clone());
        record.files.retain(|x| x.rel != p.rel);
        return Ok(());
    }
    if is_ours(p, &target) {
        fs::remove_file(&target)?;
        if let Some(b) = &p.backup {
            restore_backup(paths, &p.rel, b)?;
        }
    } else if exists(&target) {
        let settings = p.copy && p.backup.is_none() && config_like(&p.rel) && is_file(&target);
        match changes_path(paths, &p.source).filter(|_| settings) {
            Some(keep) => {
                // The player (or the game) changed this mod's settings file: keep their version for next time.
                let _ = remove_if_file(&keep);
                move_file(&target, &keep)?;
                report.player_changes_saved.push(p.rel.clone());
            }
            // Something else changed it (often a game update). Leave it; the old backup is no longer the game's file.
            None if !again => report.changed_by_others.push(p.rel.clone()),
            None => {}
        }
    } else if let Some(b) = &p.backup {
        restore_backup(paths, &p.rel, b)?;
    }
    record.files.retain(|x| x.rel != p.rel);
    Ok(())
}

/// Removes folders Turbine created that are now empty, deepest first.
fn tidy_dirs(paths: &Paths, record: &mut Record) {
    let mut dirs = record.dirs.clone();
    dirs.sort_by_key(|d| std::cmp::Reverse(d.matches('/').count()));
    for d in dirs {
        let p = paths.game.join(&d);
        match fs::symlink_metadata(&p) {
            Ok(m) if m.is_dir() => {
                let empty = fs::read_dir(&p).map(|mut r| r.next().is_none()).unwrap_or(false);
                if empty && check_dirs(&paths.game, &d, None).is_ok() && fs::remove_dir(&p).is_ok() {
                    record.dirs.retain(|x| *x != d);
                }
            }
            Ok(_) => {}
            Err(_) => record.dirs.retain(|x| *x != d),
        }
    }
}

/// Holds the game's deploy lock until dropped.
struct Lock(#[allow(dead_code)] File);

fn lock(paths: &Paths) -> io::Result<Lock> {
    if let Some(d) = paths.record.parent() {
        fs::create_dir_all(d)?;
    }
    let f = File::options().create(true).truncate(false).write(true).open(with_suffix(&paths.record, ".lock"))?;
    // A short wait first: a program being started at that moment can hold a copy of the lock for a few milliseconds.
    for _ in 0..50 {
        match f.try_lock() {
            Ok(()) => return Ok(Lock(f)),
            Err(TryLockError::WouldBlock) => std::thread::sleep(std::time::Duration::from_millis(20)),
            Err(TryLockError::Error(e)) => return Err(e),
        }
    }
    Err(plain(io::ErrorKind::WouldBlock, "Turbine is already applying changes to this game. Try again when it's done."))
}

/// The game's folder must be there (an SD card can be out); Turbine never creates it.
fn check_game(paths: &Paths) -> io::Result<()> {
    match fs::metadata(&paths.game) {
        Ok(m) if m.is_dir() => Ok(()),
        _ => Err(plain(
            io::ErrorKind::NotFound,
            format!(
                "The game's folder isn't there ({}). If the game is on an SD card or another drive, put it back in and try again.",
                paths.game.display()
            ),
        )),
    }
}

/// Makes the game folder hold exactly `wanted`. `stop_after` exists for the crash tests: it stops after that many
/// steps as if the power went out.
pub fn apply(paths: &Paths, wanted: &[(String, PathBuf)], stop_after: Option<usize>) -> io::Result<Report> {
    let all: Vec<Wanted> = wanted.iter().map(|(rel, source)| Wanted { rel: rel.clone(), source: source.clone(), may_replace: true }).collect();
    apply_guarded(paths, &all, stop_after)
}

/// `apply` where each file says whether it may replace a file the game already has. A file that may not, and whose
/// place holds a file Turbine didn't place, is left out (`Report::skipped_existing`); one Turbine placed earlier over
/// a game file (with its consent then) comes out again, so the game's own file goes back.
pub fn apply_guarded(paths: &Paths, wanted: &[Wanted], stop_after: Option<usize>) -> io::Result<Report> {
    check_game(paths)?;
    let _lock = lock(paths)?;
    let mut report = Report::default();
    if let Some(r) = recover_locked(paths)? {
        report.recovered = true;
        report.bad_entries = r.bad_entries;
    }
    clean_link_tests(&paths.game);
    let mut record = load_record(paths, &mut report)?;
    let placed: HashMap<String, &Placed> = record.files.iter().map(|p| (lower(&p.rel), p)).collect();
    // Later entries win. When the one that would land may not replace what is there (a game file, or a file
    // Turbine put over a game file with another mod's consent, or one the game has replaced since), the latest
    // earlier mod that may replace it wins instead; with none, the path is left as it is (`skipped_existing`).
    let mut order: Vec<String> = Vec::new();
    let mut by_key: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, w) in wanted.iter().enumerate() {
        let k = lower(&w.rel);
        if !by_key.contains_key(&k) {
            order.push(k.clone());
        }
        by_key.entry(k).or_default().push(i);
    }
    let mut upto: HashMap<String, usize> = HashMap::new();
    let mut skipped: HashSet<String> = HashSet::new();
    for k in &order {
        let idxs = &by_key[k];
        let last = *idxs.last().unwrap_or(&0);
        let w = &wanted[last];
        if w.may_replace {
            continue;
        }
        let blocked = match placed.get(k) {
            Some(p) => {
                let target = paths.game.join(&p.rel);
                let edited_settings = p.copy && p.backup.is_none() && config_like(&p.rel) && is_file(&target);
                // Over a game file before (with consent then), or the game put its own file here since.
                p.backup.is_some() || (!is_ours(p, &target) && !edited_settings && exists(&target))
            }
            None => safe_rel(&w.rel) && game_has(&paths.game, &w.rel),
        };
        if !blocked {
            continue;
        }
        match idxs.iter().rev().find(|&&j| wanted[j].may_replace) {
            Some(&j) => {
                upto.insert(k.clone(), j);
            }
            None => {
                skipped.insert(k.clone());
                report.skipped_existing.push(w.rel.clone());
            }
        }
    }
    let mut list: Vec<(String, PathBuf)> = Vec::with_capacity(wanted.len());
    for (i, w) in wanted.iter().enumerate() {
        let k = lower(&w.rel);
        if skipped.contains(&k) || upto.get(&k).is_some_and(|&j| i > j) {
            continue;
        }
        list.push((w.rel.clone(), w.source.clone()));
    }
    drop(placed);
    let plan = plan(paths, &record, &list)?;
    report.kept = plan.kept;
    report.copied_not_linked = plan.copied_not_linked;
    report.put_back = plan.put_back;
    if plan.ops.is_empty() {
        return Ok(report);
    }
    let lines: Vec<String> = plan.ops.iter().map(op_json).collect();
    let dirs: Vec<String> = plan.new_dirs.iter().map(|d| quote(d)).collect();
    write_atomic(&paths.plan, &format!("{{\"ops\":[\n{}\n],\n\"new_dirs\":[{}]}}\n", lines.join(",\n"), dirs.join(",")))?;
    for (i, op) in plan.ops.iter().enumerate() {
        if stop_after == Some(i) {
            return Err(io::Error::other("stopped on purpose (test)"));
        }
        let step = match op {
            Op::Remove { placed, again } => do_remove(paths, placed, *again, &mut record, &mut report).map(|()| report.removed += 1),
            Op::Add { rel, source, copy, backup } => do_add(paths, rel, source, *copy, backup, &mut record, &mut report).map(|()| report.added += 1),
        };
        if let Err(e) = step {
            // Keep what's done on record; the plan stays for the next start to finish.
            let _ = record.save(&paths.record);
            return Err(e);
        }
        // Saving every so often keeps recovery short without slowing big deploys.
        if i % 200 == 199 {
            record.save(&paths.record)?;
        }
    }
    tidy_dirs(paths, &mut record);
    record.save(&paths.record)?;
    fs::remove_file(&paths.plan)?;
    Ok(report)
}

/// Is there a file or folder at `rel` in the game (matched ignoring case, no links followed on the way)?
fn game_has(game: &Path, rel: &str) -> bool {
    let mut cur = game.to_path_buf();
    for part in rel.split('/') {
        let exact = cur.join(part);
        if fs::symlink_metadata(&exact).is_ok() {
            cur = exact;
            continue;
        }
        let Ok(rd) = fs::read_dir(&cur) else { return false };
        match rd.flatten().find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(part)) {
            Some(e) => cur = e.path(),
            None => return false,
        }
    }
    true
}

/// What Turbine's record says against what is in the game folder now ("Check game folder"): files Turbine placed
/// that are gone, and ones something else changed since. Read only.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Verify {
    pub ok: usize,
    pub missing: Vec<String>,
    pub changed: Vec<String>,
    /// Entries in the record that couldn't be read safely (only Reset game files deals with them).
    pub unreadable: usize,
}

pub fn verify(paths: &Paths) -> io::Result<Verify> {
    check_game(paths)?;
    let mut report = Report::default();
    let record = load_record(paths, &mut report)?;
    let mut v = Verify { unreadable: record.quarantine.len() + report.bad_entries.len(), ..Verify::default() };
    for p in &record.files {
        let target = paths.game.join(&p.rel);
        if fs::symlink_metadata(&target).is_err() {
            v.missing.push(p.rel.clone());
        } else if is_ours(p, &target) || (p.copy && config_like(&p.rel)) {
            v.ok += 1;
        } else {
            v.changed.push(p.rel.clone());
        }
    }
    Ok(v)
}

/// Finishes a deploy that stopped halfway. Returns None when there was nothing to recover.
pub fn recover(paths: &Paths) -> io::Result<Option<Report>> {
    check_game(paths)?;
    let _lock = lock(paths)?;
    recover_locked(paths)
}

fn recover_locked(paths: &Paths) -> io::Result<Option<Report>> {
    let text = match fs::read(&paths.plan) {
        Ok(t) => String::from_utf8_lossy(&t).into_owned(),
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let mut report = Report { recovered: true, ..Report::default() };
    // A plan cut off while being written can't be read; nothing was changed yet then.
    let parsed = json::parse(&text).ok();
    let mut ops = Vec::new();
    for v in parsed.as_ref().and_then(|v| v.get("ops")).map(Json::as_array).unwrap_or(&[]) {
        match op_from(v, paths) {
            Ok(op) => ops.push(op),
            Err(why) => report.bad_entries.push(format!("plan step {}", describe(v, &why))),
        }
    }
    let new_dirs: Vec<String> =
        parsed.as_ref().and_then(|v| v.get("new_dirs")).map(Json::as_array).unwrap_or(&[]).iter().filter_map(|d| d.as_str()).filter(|d| safe_rel(d)).map(String::from).collect();
    let mut record = load_record(paths, &mut report)?;
    for op in ops {
        match op {
            Op::Remove { placed, .. } => {
                let target = paths.game.join(&placed.rel);
                let still_recorded = record.files.iter().any(|p| p.rel == placed.rel);
                if check_dirs(&paths.game, &placed.rel, None).is_err() {
                    record.files.retain(|p| p.rel != placed.rel);
                    continue;
                }
                if is_ours(&placed, &target) {
                    // Not removed yet: finish it.
                    do_remove(paths, &placed, true, &mut record, &mut report)?;
                    report.removed += 1;
                } else if still_recorded {
                    // Already removed; put the game's file back if that step didn't happen.
                    if !exists(&target) {
                        if let Some(b) = &placed.backup {
                            restore_backup(paths, &placed.rel, b)?;
                        }
                    }
                    record.files.retain(|p| p.rel != placed.rel);
                    report.removed += 1;
                }
            }
            Op::Add { rel, source, copy: _, backup } => {
                if record.files.iter().any(|p| p.rel == rel) {
                    continue;
                }
                if check_dirs(&paths.game, &rel, None).is_err() {
                    continue;
                }
                let target = paths.game.join(&rel);
                clean_temp(&target);
                let backup_there = backup.as_ref().map(|b| is_file_or_link(b));
                let t = match fs::symlink_metadata(&target) {
                    Ok(t) => t,
                    Err(_) => {
                        // Never finished: undo the half that happened (the game file moved to backups).
                        if let Some(b) = &backup {
                            restore_backup(paths, &rel, b)?;
                        }
                        continue;
                    }
                };
                if !t.is_file() || backup_there == Some(false) {
                    // Not started: what's there is the game's own (or isn't a file at all).
                    continue;
                }
                let src_meta = fs::metadata(&source).ok();
                let saved_meta = changes_path(paths, &source).and_then(|p| fs::metadata(p).ok());
                let matches = |s: &fs::Metadata| t.ino() == s.ino() || (t.len() == s.len() && t.mtime() == s.mtime() && t.mtime_nsec() == s.mtime_nsec());
                let placed_here = match &src_meta {
                    Some(s) => matches(s) || saved_meta.as_ref().is_some_and(matches),
                    // The staged file is gone (the mod was removed since): what's at the target can only be Turbine's.
                    None => true,
                };
                if placed_here {
                    let linked = src_meta.as_ref().is_some_and(|s| s.ino() == t.ino());
                    record.files.push(Placed {
                        rel: rel.clone(),
                        source,
                        copy: !linked,
                        ino: t.ino(),
                        size: t.len(),
                        mtime: mtime_of(&t),
                        backup,
                        src_ino: src_meta.as_ref().map(|s| s.ino()).unwrap_or(0),
                        src_size: src_meta.as_ref().map(|s| s.len()).unwrap_or(0),
                        src_mtime: src_meta.as_ref().map(mtime_of).unwrap_or(0),
                    });
                    report.added += 1;
                }
            }
        }
    }
    // Folders the plan was going to create are Turbine's; empty ones go.
    for d in new_dirs {
        let real_dir = fs::symlink_metadata(paths.game.join(&d)).map(|m| m.is_dir()).unwrap_or(false);
        if real_dir && !record.dirs.contains(&d) {
            record.dirs.push(d);
        }
    }
    tidy_dirs(paths, &mut record);
    record.save(&paths.record)?;
    fs::remove_file(&paths.plan)?;
    Ok(Some(report))
}

/// A quarantined entry read as far as it goes; each part only when it reads safely.
struct Loose {
    rel: String,
    backup: Option<PathBuf>,
    ino: Option<u64>,
    size: Option<u64>,
    mtime: Option<i64>,
}

fn loose_entry(paths: &Paths, text: &str) -> Option<Loose> {
    let v = json::parse(text).ok()?;
    let rel = v.str_at("rel").filter(|r| safe_rel(r))?.to_string();
    let backup = v.str_at("backup").map(PathBuf::from).filter(|b| backup_ok(paths, b));
    Some(Loose { rel, backup, ino: int_u64(v.get("ino")).filter(|n| *n != 0), size: int_u64(v.get("size")), mtime: int_i64(v.get("mtime")) })
}

/// Every file under the backup folder: (path inside the game it belongs to, backup path).
fn leftover_backups(paths: &Paths) -> Vec<(String, PathBuf)> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = fs::read_dir(dir) else { return };
        let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            match fs::symlink_metadata(&p) {
                Ok(m) if m.is_dir() => walk(&p, out),
                Ok(m) if m.is_file() || m.file_type().is_symlink() => out.push(p),
                _ => {}
            }
        }
    }
    let mut files = Vec::new();
    walk(&paths.backups, &mut files);
    let mut out = Vec::new();
    for f in files {
        let Ok(r) = f.strip_prefix(&paths.backups) else { continue };
        let mut parts: Vec<String> = r.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
        if parts.len() > 1 && parts[0].strip_prefix(".turbine-").is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())) {
            parts.remove(0);
        }
        let rel = parts.join("/");
        if safe_rel(&rel) {
            out.push((rel, f));
        }
    }
    out
}

/// The way back when anything went wrong: takes out every file Turbine knows it placed, puts every game file in the
/// backup folder back where its place is free, and starts a fresh, empty record. Works with a damaged record (it uses
/// the saved copy, or nothing) and with set-aside entries (only files that still match what was written down go).
/// Files it can't be sure of are left alone and listed.
pub fn reset_from_backups(paths: &Paths) -> io::Result<Report> {
    check_game(paths)?;
    let _lock = lock(paths)?;
    let mut report = Report::default();
    match recover_locked(paths) {
        Ok(Some(_)) => report.recovered = true,
        Ok(None) => {}
        Err(_) => {
            // The stopped deploy can't be finished (its record is damaged): set its plan aside.
            if exists(&paths.plan) {
                fs::rename(&paths.plan, with_suffix(&paths.plan, ".damaged"))?;
            }
        }
    }
    let read = |p: &Path, bad: &mut Vec<String>| -> Option<Record> {
        let text = String::from_utf8(fs::read(p).ok()?).ok()?;
        parse_record(&text, Some(paths), bad).ok()
    };
    let mut bad = Vec::new();
    let mut record = match read(&paths.record, &mut bad) {
        Some(r) => r,
        None => {
            bad.clear();
            if exists(&paths.record) {
                fs::rename(&paths.record, with_suffix(&paths.record, ".damaged"))?;
            }
            read(&prev_path(&paths.record), &mut bad).unwrap_or_default()
        }
    };
    report.bad_entries = bad;
    for p in record.files.clone() {
        do_remove(paths, &p, false, &mut record, &mut report)?;
        report.removed += 1;
    }
    for q in std::mem::take(&mut record.quarantine) {
        let Some(Loose { rel, backup, ino, size, mtime }) = loose_entry(paths, &q) else { continue };
        if check_dirs(&paths.game, &rel, None).is_err() {
            continue;
        }
        let target = paths.game.join(&rel);
        if let Ok(m) = fs::symlink_metadata(&target) {
            let same_ino = ino.is_some_and(|i| i == m.ino());
            let same_stamp = size == Some(m.len()) && mtime == Some(mtime_of(&m));
            if m.is_file() && (same_ino || same_stamp) {
                fs::remove_file(&target)?;
                report.removed += 1;
            } else {
                report.changed_by_others.push(rel.clone());
            }
        }
        if let Some(b) = backup {
            restore_backup(paths, &rel, &b)?;
        }
    }
    for (rel, b) in leftover_backups(paths) {
        if check_dirs(&paths.game, &rel, None).is_err() || exists(&paths.game.join(&rel)) {
            report.left_in_backups.push(rel);
            continue;
        }
        restore_backup(paths, &rel, &b)?;
    }
    tidy_dirs(paths, &mut record);
    let fresh = Record { files: Vec::new(), dirs: record.dirs, quarantine: Vec::new() };
    fresh.save(&paths.record)?;
    Ok(report)
}
