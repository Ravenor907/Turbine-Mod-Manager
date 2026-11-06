//! Puts mod files into a game's folder and takes them out again, safely.
//!
//! Rules:
//! - Every file Turbine places is written down in a record (with its inode, or size and time for copies), so Turbine
//!   only ever removes files it placed. Files a player or the game made are never touched.
//! - A game file a mod replaces is moved to a backup folder first, and put back when the mod goes.
//! - Mods are hard-linked where possible. A mod file that replaces a game file is copied instead, so a game update
//!   writing to that file can never change the mod's own copy.
//! - Before any change, the whole plan is written to disk. If Turbine stops halfway (crash, power loss), the next start
//!   reads the plan, looks at what's really on disk, and finishes the record so nothing is lost or left untracked.

use crate::json::{self, quote, Json};
use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File};
use std::io::{self, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
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
}

/// Everything Turbine has placed in one game folder.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Record {
    pub files: Vec<Placed>,
    /// Folders Turbine created (removed again only when empty).
    pub dirs: Vec<String>,
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
}

#[derive(Debug, Clone, PartialEq)]
enum Op {
    Add { rel: String, source: PathBuf, copy: bool, backup: Option<PathBuf> },
    Remove { placed: Placed },
}

fn lower(s: &str) -> String {
    s.to_lowercase()
}

fn mtime_of(m: &fs::Metadata) -> i64 {
    m.mtime() * 1_000_000_000 + m.mtime_nsec()
}

/// Is the file at `target` still the one Turbine placed?
fn is_ours(p: &Placed, target: &Path) -> bool {
    match fs::symlink_metadata(target) {
        Ok(m) if m.is_file() => {
            if p.copy {
                m.len() == p.size && mtime_of(&m) == p.mtime
            } else {
                m.ino() == p.ino
            }
        }
        _ => false,
    }
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

fn placed_json(p: &Placed) -> String {
    format!(
        "{{\"rel\":{},\"source\":{},\"copy\":{},\"ino\":\"{}\",\"size\":\"{}\",\"mtime\":\"{}\",\"backup\":{}}}",
        quote(&p.rel),
        quote(&p.source.to_string_lossy()),
        p.copy,
        p.ino,
        p.size,
        p.mtime,
        p.backup.as_ref().map(|b| quote(&b.to_string_lossy())).unwrap_or_else(|| "null".into())
    )
}

fn num(v: Option<&Json>) -> i64 {
    match v {
        Some(Json::Num(n)) => *n as i64,
        // Large numbers are stored as strings so they survive exactly.
        Some(Json::Str(s)) => s.parse().unwrap_or(0),
        _ => 0,
    }
}

fn placed_from(v: &Json) -> Option<Placed> {
    Some(Placed {
        rel: v.str_at("rel")?.to_string(),
        source: PathBuf::from(v.str_at("source")?),
        copy: v.get("copy").and_then(Json::as_bool).unwrap_or(false),
        ino: num(v.get("ino")) as u64,
        size: num(v.get("size")) as u64,
        mtime: num(v.get("mtime")),
        backup: v.str_at("backup").map(PathBuf::from),
    })
}

fn write_atomic(path: &Path, text: &str) -> io::Result<()> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    let tmp = path.with_extension("tmp");
    {
        let mut f = File::create(&tmp)?;
        f.write_all(text.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)
}

impl Record {
    pub fn load(path: &Path) -> io::Result<Record> {
        let text = match fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Record::default()),
            Err(e) => return Err(e),
        };
        let v = json::parse(&text).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        let files = v.get("files").map(Json::as_array).unwrap_or(&[]).iter().filter_map(placed_from).collect();
        let dirs = v.get("dirs").map(Json::as_array).unwrap_or(&[]).iter().filter_map(|d| d.as_str().map(String::from)).collect();
        Ok(Record { files, dirs })
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        let files: Vec<String> = self.files.iter().map(placed_json).collect();
        let dirs: Vec<String> = self.dirs.iter().map(|d| quote(d)).collect();
        write_atomic(path, &format!("{{\"files\":[\n{}\n],\"dirs\":[{}]}}\n", files.join(",\n"), dirs.join(",")))
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
        Op::Remove { placed } => format!("{{\"op\":\"remove\",\"placed\":{}}}", placed_json(placed)),
    }
}

fn op_from(v: &Json) -> Option<Op> {
    match v.str_at("op")? {
        "add" => Some(Op::Add {
            rel: v.str_at("rel")?.to_string(),
            source: PathBuf::from(v.str_at("source")?),
            copy: v.get("copy").and_then(Json::as_bool).unwrap_or(false),
            backup: v.str_at("backup").map(PathBuf::from),
        }),
        "remove" => Some(Op::Remove { placed: placed_from(v.get("placed")?)? }),
        _ => None,
    }
}

// ------------------------------------------------------------------ file moves

/// Moves a file, copying across drives when a rename isn't possible.
fn move_file(from: &Path, to: &Path) -> io::Result<()> {
    if let Some(p) = to.parent() {
        fs::create_dir_all(p)?;
    }
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(e) if e.raw_os_error() == Some(18) => {
            // EXDEV: different drives.
            fs::copy(from, to)?;
            File::open(to)?.sync_all()?;
            fs::remove_file(from)
        }
        Err(e) => Err(e),
    }
}

fn unique_backup(backups: &Path, rel: &str) -> PathBuf {
    let base = backups.join(rel);
    if !base.exists() {
        return base;
    }
    (1..).map(|i| PathBuf::from(format!("{}.{}", base.display(), i))).find(|p| !p.exists()).unwrap()
}

fn same_drive(a: &Path, b: &Path) -> bool {
    match (fs::metadata(a), fs::metadata(b)) {
        (Ok(x), Ok(y)) => x.dev() == y.dev(),
        _ => false,
    }
}

// ------------------------------------------------------------------ planning and applying

/// Works out what has to change so the game folder holds exactly `wanted` (path inside the game folder, staged file),
/// later entries winning over earlier ones with the same path.
fn plan(paths: &Paths, record: &Record, wanted: &[(String, PathBuf)]) -> (Vec<Op>, usize, Vec<String>) {
    // Final choice per path, case-insensitive, in order (later wins).
    let mut chosen: BTreeMap<String, (String, PathBuf)> = BTreeMap::new();
    for (rel, src) in wanted {
        chosen.insert(lower(rel), (rel.clone(), src.clone()));
    }
    let current: HashMap<String, &Placed> = record.files.iter().map(|p| (lower(&p.rel), p)).collect();
    let mut ops = Vec::new();
    let mut kept = 0;
    // Removals first: anything placed that is no longer wanted, or now comes from a different file.
    for p in &record.files {
        match chosen.get(&lower(&p.rel)) {
            Some((_, src)) if *src == p.source => kept += 1,
            _ => ops.push(Op::Remove { placed: p.clone() }),
        }
    }
    let mut case = CaseMap::new(&paths.game);
    let mut planned: HashMap<String, String> = HashMap::new();
    for (key, (rel, src)) in &chosen {
        if let Some(p) = current.get(key) {
            if p.source == *src {
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
        let target = paths.game.join(&actual);
        // A game file is here, or will be once the mod file being replaced puts it back: it gets backed up.
        let replaces_game_file = match current.get(&lk) {
            Some(p) => p.backup.is_some(),
            None => target.exists(),
        };
        let backup = replaces_game_file.then(|| unique_backup(&paths.backups, &actual));
        let copy = replaces_game_file || !same_drive(src, &paths.game);
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
                if !paths.game.join(&acc).exists() && !new_dirs.contains(&acc) {
                    new_dirs.push(acc.clone());
                }
            }
        }
    }
    (ops, kept, new_dirs)
}

fn do_add(paths: &Paths, rel: &str, source: &Path, copy: bool, backup: &Option<PathBuf>, record: &mut Record) -> io::Result<()> {
    let target = paths.game.join(rel);
    // Folders on the way that don't exist yet are Turbine's.
    let mut acc = String::new();
    let parts: Vec<&str> = rel.split('/').collect();
    for part in &parts[..parts.len().saturating_sub(1)] {
        if !acc.is_empty() {
            acc.push('/');
        }
        acc.push_str(part);
        let d = paths.game.join(&acc);
        if !d.exists() {
            fs::create_dir(&d)?;
            if !record.dirs.contains(&acc) {
                record.dirs.push(acc.clone());
            }
        }
    }
    match backup {
        Some(b) if target.exists() => move_file(&target, b)?,
        None if fs::symlink_metadata(&target).is_ok() => {
            // Something appeared here after planning. Never overwrite a file Turbine didn't plan for.
            return Err(io::Error::new(io::ErrorKind::AlreadyExists, format!("{rel} appeared while applying changes")));
        }
        _ => {}
    }
    let use_copy = copy || fs::hard_link(source, &target).is_err();
    if use_copy {
        let tmp = target.with_extension("turbine-tmp");
        fs::copy(source, &tmp)?;
        let f = File::options().write(true).open(&tmp)?;
        f.set_modified(fs::metadata(source)?.modified().unwrap_or(SystemTime::now()))?;
        f.sync_all()?;
        fs::rename(&tmp, &target)?;
    }
    let m = fs::symlink_metadata(&target)?;
    record.files.push(Placed {
        rel: rel.to_string(),
        source: source.to_path_buf(),
        copy: use_copy,
        ino: m.ino(),
        size: m.len(),
        mtime: mtime_of(&m),
        backup: backup.clone(),
    });
    Ok(())
}

fn do_remove(paths: &Paths, p: &Placed, record: &mut Record, report: &mut Report) -> io::Result<()> {
    let target = paths.game.join(&p.rel);
    if is_ours(p, &target) {
        fs::remove_file(&target)?;
        if let Some(b) = &p.backup {
            if b.exists() {
                move_file(b, &target)?;
            }
        }
    } else if target.exists() {
        // Something else changed it (often a game update). Leave it; the old backup is no longer the game's file.
        report.changed_by_others.push(p.rel.clone());
    } else if let Some(b) = &p.backup {
        if b.exists() {
            move_file(b, &target)?;
        }
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
        let empty = fs::read_dir(&p).map(|mut r| r.next().is_none()).unwrap_or(false);
        if empty && fs::remove_dir(&p).is_ok() || !p.exists() {
            record.dirs.retain(|x| *x != d);
        }
    }
}

/// Makes the game folder hold exactly `wanted`. `stop_after` exists for the crash tests: it stops after that many
/// steps as if the power went out.
pub fn apply(paths: &Paths, wanted: &[(String, PathBuf)], stop_after: Option<usize>) -> io::Result<Report> {
    let mut report = Report { recovered: recover(paths)?.is_some(), ..Report::default() };
    let mut record = Record::load(&paths.record)?;
    let (ops, kept, new_dirs) = plan(paths, &record, wanted);
    report.kept = kept;
    if ops.is_empty() {
        return Ok(report);
    }
    let lines: Vec<String> = ops.iter().map(op_json).collect();
    let dirs: Vec<String> = new_dirs.iter().map(|d| quote(d)).collect();
    write_atomic(&paths.plan, &format!("{{\"ops\":[\n{}\n],\n\"new_dirs\":[{}]}}\n", lines.join(",\n"), dirs.join(",")))?;
    for (i, op) in ops.iter().enumerate() {
        if stop_after == Some(i) {
            return Err(io::Error::other("stopped on purpose (test)"));
        }
        match op {
            Op::Remove { placed } => {
                do_remove(paths, placed, &mut record, &mut report)?;
                report.removed += 1;
            }
            Op::Add { rel, source, copy, backup } => {
                do_add(paths, rel, source, *copy, backup, &mut record)?;
                report.added += 1;
            }
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

/// Finishes a deploy that stopped halfway. Returns None when there was nothing to recover.
pub fn recover(paths: &Paths) -> io::Result<Option<Report>> {
    let text = match fs::read_to_string(&paths.plan) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    // A plan cut off while being written can't be read; nothing was changed yet then.
    let parsed = json::parse(&text).ok();
    let ops: Vec<Op> = parsed.as_ref().and_then(|v| v.get("ops")).map(Json::as_array).unwrap_or(&[]).iter().filter_map(op_from).collect();
    let new_dirs: Vec<String> =
        parsed.as_ref().and_then(|v| v.get("new_dirs")).map(Json::as_array).unwrap_or(&[]).iter().filter_map(|d| d.as_str().map(String::from)).collect();
    let mut record = Record::load(&paths.record)?;
    let mut report = Report { recovered: true, ..Report::default() };
    for op in ops {
        match op {
            Op::Remove { placed } => {
                let target = paths.game.join(&placed.rel);
                let still_recorded = record.files.iter().any(|p| p.rel == placed.rel);
                if is_ours(&placed, &target) {
                    // Not removed yet: finish it.
                    do_remove(paths, &placed, &mut record, &mut report)?;
                    report.removed += 1;
                } else if still_recorded {
                    // Already removed; put the game's file back if that step didn't happen.
                    if !target.exists() {
                        if let Some(b) = &placed.backup {
                            if b.exists() {
                                move_file(b, &target)?;
                            }
                        }
                    }
                    record.files.retain(|p| p.rel != placed.rel);
                    report.removed += 1;
                }
            }
            Op::Add { rel, source, copy, backup } => {
                let target = paths.game.join(&rel);
                if record.files.iter().any(|p| p.rel == rel) {
                    continue;
                }
                let src_meta = fs::metadata(&source).ok();
                let placed_here = match (fs::symlink_metadata(&target), &src_meta) {
                    (Ok(t), Some(s)) => t.ino() == s.ino() || (copy && t.len() == s.len() && t.mtime() == s.mtime()),
                    _ => false,
                };
                if placed_here {
                    let m = fs::symlink_metadata(&target)?;
                    record.files.push(Placed {
                        rel: rel.clone(),
                        source,
                        copy: m.ino() != src_meta.map(|s| s.ino()).unwrap_or(0),
                        ino: m.ino(),
                        size: m.len(),
                        mtime: mtime_of(&m),
                        backup,
                    });
                    report.added += 1;
                } else {
                    // Never finished: undo the half that happened (the game file moved to backups).
                    let tmp = target.with_extension("turbine-tmp");
                    let _ = fs::remove_file(&tmp);
                    if let Some(b) = &backup {
                        if b.exists() && !target.exists() {
                            move_file(b, &target)?;
                        }
                    }
                }
            }
        }
    }
    // Folders the plan was going to create are Turbine's; empty ones go.
    for d in new_dirs {
        if paths.game.join(&d).is_dir() && !record.dirs.contains(&d) {
            record.dirs.push(d);
        }
    }
    tidy_dirs(paths, &mut record);
    record.save(&paths.record)?;
    fs::remove_file(&paths.plan)?;
    Ok(Some(report))
}
