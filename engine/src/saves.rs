//! Save backups before Play: before a game starts with mods, Turbine copies its save folders into a dated
//! snapshot in its own data folder, and keeps the newest ten (or fewer, so all of them fit in 2 GB). Copy only:
//! the game's saves are never changed, except when the player puts a backup back, and that first takes a snapshot
//! of the saves as they are. Links are never followed and nothing outside the save folders is read.
//!
//! Every snapshot keeps the path each folder came from (`from`, one per line, the folder named "0" is line one),
//! and a putting back goes only to that same path. A snapshot is whole or not there at all: it is copied into a
//! hidden `.part` folder first and only renamed into place when every file made it, and a failed copy is removed.
//! Before copying, the size is measured: a save that is too big or has too many files is refused with a plain
//! reason (never cut off), and so is a copy the drive has no room for. Nothing is copied when the saves haven't
//! changed since the newest snapshot. Trimming never removes the snapshot just taken or the one being put back.
//!
//! Where a game keeps its saves comes from its data file (`saves`: under Proton, relative to the prefix's user
//! folder; native builds, relative to the home folder; or inside the game folder).

use crate::gamedata::SavesFile;
use crate::games::{Game, Runtime};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Snapshots kept per game.
pub const KEEP: usize = 10;
/// The most all of a game's snapshots may take together.
pub const MAX_BYTES: u64 = 2 * 1024 * 1024 * 1024;
/// The most one snapshot may take (half the total, so a new one never pushes out every older one).
pub const MAX_ONE: u64 = MAX_BYTES / 2;
/// Files one snapshot may hold at most. Over this, nothing is copied and the player is told why.
pub const MAX_FILES: usize = 20_000;
/// Room left free on the drive after a snapshot.
const SPARE: u64 = 256 * 1024 * 1024;

/// One snapshot: its folder name (the time it was taken, "20261108-213012"), and its size.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub id: String,
    pub when: u64,
    pub bytes: u64,
    pub files: usize,
}

/// The game's save folders that exist now.
pub fn folders(game: &Game, saves: &SavesFile, home: &Path) -> Vec<PathBuf> {
    locations(game, saves, home).into_iter().filter(|p| fs::symlink_metadata(p).map(|m| m.is_dir()).unwrap_or(false)).collect()
}

/// Every place the game's data file says it keeps saves, there or not (a backup may go back into one the game or
/// the player deleted).
pub fn locations(game: &Game, saves: &SavesFile, home: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let user = game.prefix.as_ref().map(|p| p.join("drive_c").join("users").join("steamuser"));
    let native = game.runtime == Runtime::Native;
    if let Some(u) = user.as_ref().filter(|_| !native) {
        out.extend(saves.proton.iter().map(|r| u.join(r)));
    }
    if native || user.is_none() {
        out.extend(saves.native.iter().map(|r| home.join(r)));
    }
    out.extend(saves.game.iter().map(|r| game.install_dir.join(r)));
    out
}

fn stamp(now: u64) -> String {
    // Days and seconds since 1970 as a civil date (UTC), so the folder names sort by time.
    let days = (now / 86_400) as i64;
    let secs = now % 86_400;
    let (y, m, d) = civil(days);
    format!("{y:04}{m:02}{d:02}-{:02}{:02}{:02}", secs / 3600, secs / 60 % 60, secs % 60)
}

fn civil(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

fn other(msg: String) -> io::Error {
    io::Error::other(msg)
}

/// Copies `from` into `to` (made if needed): plain files and folders only, links skipped. Adds (files, bytes).
fn copy_tree(from: &Path, to: &Path, count: &mut (usize, u64)) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for e in fs::read_dir(from)? {
        let e = e?;
        let ft = e.file_type()?;
        let dest = to.join(e.file_name());
        if ft.is_dir() {
            copy_tree(&e.path(), &dest, count)?;
        } else if ft.is_file() {
            count.1 += fs::copy(e.path(), &dest)?;
            count.0 += 1;
        }
    }
    Ok(())
}

/// Every plain file under `dir` (links not followed), as "relative path\tsize\tmodified" lines, sorted. Stops
/// early once over `cap` files.
fn index_of(dir: &Path, cap: usize) -> (Vec<String>, u64) {
    let mut out = Vec::new();
    let mut bytes = 0u64;
    let mut todo = vec![(dir.to_path_buf(), String::new())];
    while let Some((d, rel)) = todo.pop() {
        let Ok(rd) = fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let Ok(m) = fs::symlink_metadata(e.path()) else { continue };
            let name = e.file_name().to_string_lossy().into_owned();
            let r = if rel.is_empty() { name } else { format!("{rel}/{name}") };
            if m.is_dir() {
                todo.push((e.path(), r));
            } else if m.is_file() {
                let t = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_nanos()).unwrap_or(0);
                out.push(format!("{r}\t{}\t{t}", m.len()));
                bytes += m.len();
                if out.len() > cap {
                    return (out, bytes);
                }
            }
        }
    }
    out.sort();
    (out, bytes)
}

fn size_of(dir: &Path) -> (usize, u64) {
    let (files, bytes) = index_of(dir, usize::MAX);
    (files.len(), bytes)
}

/// Every snapshot in `root`, newest first (half-made `.part` folders are not snapshots).
pub fn list(root: &Path) -> Vec<Snapshot> {
    let Ok(rd) = fs::read_dir(root) else { return Vec::new() };
    let mut out: Vec<Snapshot> = rd
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .filter_map(|e| {
            let id = e.file_name().to_string_lossy().into_owned();
            let when = fs::read_to_string(e.path().join("when")).ok()?.trim().parse().ok()?;
            let (files, bytes) = size_of(&e.path());
            // The snapshot's own small files ("when", "from", "index") aren't saves.
            let own: Vec<u64> = ["when", "from", "index"].iter().filter_map(|f| fs::metadata(e.path().join(f)).ok()).map(|m| m.len()).collect();
            Some(Snapshot { id, when, bytes: bytes - own.iter().sum::<u64>(), files: files - own.len() })
        })
        .collect();
    out.sort_by(|a, b| b.when.cmp(&a.when).then(b.id.cmp(&a.id)));
    out
}

/// The save folders a snapshot was taken from, in order (line i is kept in the snapshot's folder "i").
pub fn sources(root: &Path, id: &str) -> io::Result<Vec<PathBuf>> {
    let from = fs::read_to_string(root.join(id).join("from"))?;
    Ok(from.lines().filter(|l| !l.is_empty()).map(PathBuf::from).collect())
}

/// Free space (bytes) on the drive holding `path` (or the nearest folder above it that exists). None where it
/// can't be read.
#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
pub fn free_bytes(path: &Path) -> Option<u64> {
    use std::ffi::CString;
    use std::os::raw::{c_char, c_int};
    use std::os::unix::ffi::OsStrExt;
    unsafe extern "C" {
        // The C library's statvfs. On 64-bit Linux its struct starts with f_bsize, f_frsize, f_blocks, f_bfree,
        // f_bavail, all 64-bit; the buffer below is larger than the whole struct (112 bytes).
        fn statvfs(path: *const c_char, buf: *mut u64) -> c_int;
    }
    let at = path.ancestors().find(|p| p.exists())?;
    let c = CString::new(at.as_os_str().as_bytes()).ok()?;
    let mut buf = [0u64; 32];
    // SAFETY: c is a valid C string and buf is 256 bytes, more than struct statvfs needs.
    let r = unsafe { statvfs(c.as_ptr(), buf.as_mut_ptr()) };
    if r != 0 {
        return None;
    }
    Some(buf[1].saturating_mul(buf[4]))
}

#[cfg(not(all(target_os = "linux", target_pointer_width = "64")))]
pub fn free_bytes(_path: &Path) -> Option<u64> {
    None
}

fn gb(bytes: u64) -> String {
    format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
}

/// Takes a snapshot of `folders` into `root` (a new folder named for `now`), then trims old ones. None when there
/// are no save folders (nothing to keep). When the saves haven't changed since the newest snapshot, that snapshot
/// is returned and nothing is copied. Refused (an error saying why, nothing copied) when the saves are too big or
/// have too many files to keep, or the drive has no room.
pub fn backup(folders: &[PathBuf], root: &Path, now: u64) -> io::Result<Option<Snapshot>> {
    backup_with(folders, root, now, free_bytes(root), &[])
}

/// backup with the drive's free space given (None: unknown, not checked) and snapshots trimming must keep.
pub fn backup_with(folders: &[PathBuf], root: &Path, now: u64, free: Option<u64>, protect: &[&str]) -> io::Result<Option<Snapshot>> {
    if folders.is_empty() {
        return Ok(None);
    }
    fs::create_dir_all(root)?;
    // Half-made snapshots left by a crash are never snapshots: clear them.
    if let Ok(rd) = fs::read_dir(root) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().into_owned();
            if n.starts_with('.') && n.ends_with(".part") {
                let _ = fs::remove_dir_all(e.path());
            }
        }
    }
    // Measure first: nothing is copied when it can't all be copied.
    let mut index = String::new();
    let mut files = 0usize;
    let mut bytes = 0u64;
    for (i, f) in folders.iter().enumerate() {
        let (lines, b) = index_of(f, MAX_FILES);
        files += lines.len();
        bytes += b;
        if files > MAX_FILES {
            return Err(other(format!("the saves have too many files to back up (more than {MAX_FILES})")));
        }
        for l in lines {
            index.push_str(&format!("{i}/{l}\n"));
        }
    }
    if bytes > MAX_ONE {
        return Err(other(format!("the saves are too big to back up ({}; the most is {})", gb(bytes), gb(MAX_ONE))));
    }
    let from = folders.iter().map(|f| f.to_string_lossy().into_owned()).collect::<Vec<_>>().join("\n");
    // Unchanged since the newest snapshot: that one already holds these saves.
    if let Some(newest) = list(root).into_iter().next() {
        let same_from = fs::read_to_string(root.join(&newest.id).join("from")).map(|s| s == from).unwrap_or(false);
        let same_files = fs::read_to_string(root.join(&newest.id).join("index")).map(|s| s == index).unwrap_or(false);
        if same_from && same_files {
            return Ok(Some(newest));
        }
    }
    if let Some(free) = free && free < bytes.saturating_add(SPARE) {
        return Err(other(format!("not enough free space to back up the saves ({} needed, {} free)", gb(bytes + SPARE), gb(free))));
    }
    let mut id = stamp(now);
    let mut n = 1;
    while root.join(&id).exists() {
        n += 1;
        id = format!("{}-{n}", stamp(now));
    }
    let dir = root.join(&id);
    let tmp = root.join(format!(".{id}.part"));
    let made = (|| -> io::Result<(usize, u64)> {
        let mut count = (0usize, 0u64);
        for (i, f) in folders.iter().enumerate() {
            copy_tree(f, &tmp.join(i.to_string()), &mut count)?;
        }
        fs::write(tmp.join("when"), now.to_string())?;
        fs::write(tmp.join("from"), &from)?;
        fs::write(tmp.join("index"), &index)?;
        fs::rename(&tmp, &dir)?;
        Ok(count)
    })();
    let count = match made {
        Ok(c) => c,
        Err(e) => {
            let _ = fs::remove_dir_all(&tmp);
            return Err(e);
        }
    };
    let mut keep = vec![id.as_str()];
    keep.extend_from_slice(protect);
    trim(root, &keep)?;
    Ok(Some(Snapshot { id, when: now, bytes: count.1, files: count.0 }))
}

/// Keeps the newest KEEP snapshots, and drops the oldest while all together are over MAX_BYTES (never one in
/// `keep`).
fn trim(root: &Path, keep: &[&str]) -> io::Result<()> {
    let all = list(root);
    let mut total: u64 = all.iter().map(|s| s.bytes).sum();
    for (i, s) in all.iter().enumerate().rev() {
        if keep.contains(&s.id.as_str()) {
            continue;
        }
        if i >= KEEP || total > MAX_BYTES {
            fs::remove_dir_all(root.join(&s.id))?;
            total = total.saturating_sub(s.bytes);
        }
    }
    Ok(())
}

/// Puts a snapshot back: first a snapshot of the saves as they are now (returned; trimming never removes the one
/// being put back), then each kept folder's files copied over the folder it was taken from (files the snapshot
/// doesn't have are left alone). `folders` are the game's save locations (`locations`): a folder that is gone is
/// made again. Refused, with nothing changed, when a folder the snapshot came from is not one of them, is a link
/// or a file, or the snapshot is incomplete.
pub fn restore(id: &str, folders: &[PathBuf], root: &Path, now: u64) -> io::Result<Option<Snapshot>> {
    restore_with(id, folders, root, now, free_bytes(root))
}

/// restore with the drive's free space given (see backup_with).
pub fn restore_with(id: &str, folders: &[PathBuf], root: &Path, now: u64, free: Option<u64>) -> io::Result<Option<Snapshot>> {
    put_back(id, folders, root, now, free, true)
}

/// Puts a snapshot back without first keeping the saves as they are now: only when the player asked for it
/// after `restore` couldn't keep them (too big, too many files, no room). The saves now are replaced for good.
pub fn restore_without_keeping(id: &str, folders: &[PathBuf], root: &Path) -> io::Result<()> {
    put_back(id, folders, root, 0, None, false).map(|_| ())
}

/// Is this error from `restore` about keeping the current saves first (so `restore_without_keeping` could do it)?
pub fn couldnt_keep(e: &io::Error) -> bool {
    e.to_string().starts_with(KEEP_FAILED)
}

const KEEP_FAILED: &str = "the saves as they are now couldn't be kept first";

fn put_back(id: &str, folders: &[PathBuf], root: &Path, now: u64, free: Option<u64>, keep: bool) -> io::Result<Option<Snapshot>> {
    if id.is_empty() || id.contains('/') || id.contains('\\') || id.starts_with('.') {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a backup"));
    }
    let snap = root.join(id);
    if !snap.join("when").is_file() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "that backup is gone"));
    }
    let from = sources(root, id).map_err(|_| other("that backup doesn't say where its saves came from".into()))?;
    if from.is_empty() {
        return Err(other("that backup has no saves in it".into()));
    }
    for (i, f) in from.iter().enumerate() {
        if !folders.contains(f) {
            return Err(other(format!("it came from a folder that isn't one of this game's save folders: {}", f.display())));
        }
        if fs::symlink_metadata(f).is_ok_and(|m| !m.is_dir()) {
            return Err(other(format!("{} isn't a plain folder", f.display())));
        }
        if !snap.join(i.to_string()).is_dir() {
            return Err(other("that backup is incomplete".into()));
        }
    }
    let there: Vec<PathBuf> = folders.iter().filter(|p| fs::symlink_metadata(p).map(|m| m.is_dir()).unwrap_or(false)).cloned().collect();
    let before = if keep { backup_with(&there, root, now, free, &[id]).map_err(|e| other(format!("{KEEP_FAILED}: {e}")))? } else { None };
    if !snap.join("when").is_file() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "that backup is gone"));
    }
    for (i, f) in from.iter().enumerate() {
        let mut count = (0usize, 0u64);
        fs::create_dir_all(f)?;
        copy_tree(&snap.join(i.to_string()), f, &mut count)
            .map_err(|e| other(format!("copying stopped partway ({e}); the saves from just before are kept as a backup")))?;
    }
    Ok(before)
}


// ---- Elden Ring's three kinds of save ----
//
// The normal game keeps ER0000.sl2; Turbine's me3 profile its own TurbineModded.sl2 (anticheat::ME3_SAVEFILE); and
// Seamless Co-op ER0000.co2. They are listed apart, and each can be put back from a backup on its own (the saves as
// they are now are kept first), so a modded or co-op save never lands on the online one by mistake.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Solo,
    Modded,
    Coop,
}

impl Kind {
    pub fn key(self) -> &'static str {
        match self {
            Kind::Solo => "solo",
            Kind::Modded => "modded",
            Kind::Coop => "coop",
        }
    }
    pub fn from_key(k: &str) -> Option<Kind> {
        match k {
            "solo" => Some(Kind::Solo),
            "modded" => Some(Kind::Modded),
            "coop" => Some(Kind::Coop),
            _ => None,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Kind::Solo => "Normal game (online)",
            Kind::Modded => "With mods (Mod Engine 3, offline)",
            Kind::Coop => "Seamless Co-op",
        }
    }
}

/// Which kind a save file is, by its name (the game's own .bak copies count with their save). `coop_ext` is
/// Seamless Co-op's save extension ("co2" unless changed in its settings, save_file_extension).
pub fn kind_of_with(name: &str, coop_ext: &str) -> Option<Kind> {
    let l = name.to_ascii_lowercase();
    let l = l.strip_suffix(".bak").unwrap_or(&l);
    let coop = format!(".{}", coop_ext.trim().trim_start_matches('.').to_ascii_lowercase());
    if coop.len() > 1 && coop != ".sl2" && l.ends_with(&coop) || l.ends_with(".co2") {
        Some(Kind::Coop)
    } else if l == crate::anticheat::ME3_SAVEFILE.to_ascii_lowercase() {
        Some(Kind::Modded)
    } else if l.ends_with(".sl2") {
        Some(Kind::Solo)
    } else {
        None
    }
}

/// One save file now: its path inside the save folder, size and time changed.
#[derive(Debug, Clone, PartialEq)]
pub struct SaveFile {
    pub rel: String,
    pub bytes: u64,
    pub modified: u64,
}

/// An index line's path ("0/7656/ER0000.sl2\t1234\t...").
fn index_rel(l: &str) -> &str {
    l.split('\t').next().unwrap_or(l)
}

/// The save files of one kind in the save folders now.
pub fn files_of_with(folders: &[PathBuf], kind: Kind, coop_ext: &str) -> Vec<SaveFile> {
    let mut out = Vec::new();
    for f in folders {
        let (lines, _) = index_of(f, MAX_FILES);
        for line in lines {
            let rel = index_rel(&line).to_string();
            let name = rel.rsplit('/').next().unwrap_or(&rel);
            if kind_of_with(name, coop_ext) == Some(kind) {
                let m = fs::metadata(f.join(&rel)).ok();
                let modified = m.as_ref().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
                out.push(SaveFile { rel, bytes: m.map(|m| m.len()).unwrap_or(0), modified });
            }
        }
    }
    out
}

/// How many files of one kind a backup holds.
pub fn count_in_with(root: &Path, id: &str, kind: Kind, coop_ext: &str) -> usize {
    fs::read_to_string(root.join(id).join("index"))
        .map(|t| t.lines().map(index_rel).filter(|l| kind_of_with(l.rsplit('/').next().unwrap_or(l), coop_ext) == Some(kind)).count())
        .unwrap_or(0)
}

/// Puts back only one kind's files from a backup (the saves as they are now are kept first, as a new backup).
/// Answers how many files went back.
pub fn restore_kind_with(id: &str, kind: Kind, folders: &[PathBuf], root: &Path, now: u64, coop_ext: &str) -> io::Result<usize> {
    if id.is_empty() || id.contains('/') || id.contains('\\') || id.starts_with('.') {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a backup"));
    }
    let snap = root.join(id);
    let from = sources(root, id)?;
    for f in &from {
        if !folders.contains(f) {
            return Err(other(format!("{} isn't one of the game's save folders", f.display())));
        }
    }
    let index = fs::read_to_string(snap.join("index"))?;
    let wanted: Vec<&str> = index.lines().map(index_rel).filter(|l| kind_of_with(l.rsplit('/').next().unwrap_or(l), coop_ext) == Some(kind)).collect();
    if wanted.is_empty() {
        return Ok(0);
    }
    let existing: Vec<PathBuf> = folders.iter().filter(|p| p.is_dir()).cloned().collect();
    backup_with(&existing, root, now, free_bytes(root), &[id]).map_err(|e| other(format!("{KEEP_FAILED}: {e}")))?;
    let mut n = 0;
    for line in wanted {
        let Some((i, rel)) = line.split_once('/') else { continue };
        let Ok(i) = i.parse::<usize>() else { continue };
        let Some(dest_root) = from.get(i) else { continue };
        if rel.split('/').any(|p| p.is_empty() || p == "." || p == "..") {
            continue;
        }
        let src = snap.join(i.to_string()).join(rel);
        let dest = dest_root.join(rel);
        if fs::symlink_metadata(&dest).is_ok_and(|m| m.file_type().is_symlink()) {
            continue;
        }
        if let Some(p) = dest.parent() {
            fs::create_dir_all(p)?;
        }
        fs::copy(&src, &dest)?;
        n += 1;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamps_sort_by_time() {
        assert_eq!(stamp(0), "19700101-000000");
        assert_eq!(stamp(1_794_000_000), "20261106-212000");
    }

    #[test]
    fn kinds_apart() {
        assert_eq!(kind_of_with("ER0000.sl2", "co2"), Some(Kind::Solo));
        assert_eq!(kind_of_with("ER0000.sl2.bak", "co2"), Some(Kind::Solo));
        assert_eq!(kind_of_with("TurbineModded.sl2", "co2"), Some(Kind::Modded));
        assert_eq!(kind_of_with("ER0000.co2", "co2"), Some(Kind::Coop));
        assert_eq!(kind_of_with("steam_autocloud.vdf", "co2"), None);
        assert_eq!(kind_of_with("ER0000.myco", "myco"), Some(Kind::Coop));
        assert_eq!(kind_of_with("ER0000.sl2", "sl2"), Some(Kind::Solo), "never the normal save");
        let d = std::env::temp_dir().join(format!("turbine-savekinds-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        let saves = d.join("EldenRing/7656");
        fs::create_dir_all(&saves).unwrap();
        for (n, t) in [("ER0000.sl2", "solo1"), ("TurbineModded.sl2", "mod1"), ("ER0000.co2", "coop1")] {
            fs::write(saves.join(n), t).unwrap();
        }
        let folders = vec![d.join("EldenRing")];
        let root = d.join("backups");
        let snap = backup_with(&folders, &root, 1_000, None, &[]).unwrap().unwrap();
        assert_eq!(files_of_with(&folders, Kind::Coop, "co2").len(), 1);
        assert_eq!(count_in_with(&root, &snap.id, Kind::Modded, "co2"), 1);
        for (n, t) in [("ER0000.sl2", "solo2"), ("TurbineModded.sl2", "mod2"), ("ER0000.co2", "coop2")] {
            fs::write(saves.join(n), t).unwrap();
        }
        // Only the co-op save goes back; the others stay as they are now.
        assert_eq!(restore_kind_with(&snap.id, Kind::Coop, &folders, &root, 2_000, "co2").unwrap(), 1);
        assert_eq!(fs::read_to_string(saves.join("ER0000.co2")).unwrap(), "coop1");
        assert_eq!(fs::read_to_string(saves.join("ER0000.sl2")).unwrap(), "solo2");
        assert_eq!(fs::read_to_string(saves.join("TurbineModded.sl2")).unwrap(), "mod2");
        // The saves from just before were kept.
        assert_eq!(list(&root).len(), 2);
        assert!(restore_kind_with("../x", Kind::Solo, &folders, &root, 3_000, "co2").is_err());
        let _ = fs::remove_dir_all(&d);
    }
}
