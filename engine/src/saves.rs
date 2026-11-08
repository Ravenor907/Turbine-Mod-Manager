//! Save backups before Play (0.56): before a game starts with mods, Turbine copies its save folders into a dated
//! snapshot in its own data folder, and keeps the newest ten (or fewer, so all of them fit in 2 GB). Copy only:
//! the game's saves are never changed, except when the player puts a backup back, and that first takes a snapshot
//! of the saves as they are. Links are never followed and nothing outside the save folders is read.
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
/// Files copied per snapshot at most (a runaway folder is cut off, and the snapshot says so).
const MAX_FILES: usize = 20_000;

/// One snapshot: its folder name (the time it was taken, "20261108-213012"), and its size.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub id: String,
    pub when: u64,
    pub bytes: u64,
    pub files: usize,
}

/// The game's save folders that exist now, with the name each is kept under in a snapshot ("0", "1", ...).
pub fn folders(game: &Game, saves: &SavesFile, home: &Path) -> Vec<PathBuf> {
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
    out.into_iter().filter(|p| fs::symlink_metadata(p).map(|m| m.is_dir()).unwrap_or(false)).collect()
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

/// Copies `from` into `to` (made fresh): plain files and folders only, links skipped. Returns (files, bytes).
fn copy_tree(from: &Path, to: &Path, count: &mut (usize, u64)) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for e in fs::read_dir(from)? {
        let e = e?;
        let ft = e.file_type()?;
        let dest = to.join(e.file_name());
        if ft.is_dir() {
            copy_tree(&e.path(), &dest, count)?;
        } else if ft.is_file() {
            if count.0 >= MAX_FILES {
                return Ok(());
            }
            count.1 += fs::copy(e.path(), &dest)?;
            count.0 += 1;
        }
    }
    Ok(())
}

fn size_of(dir: &Path) -> (usize, u64) {
    let mut n = (0usize, 0u64);
    let mut todo = vec![dir.to_path_buf()];
    while let Some(d) = todo.pop() {
        let Ok(rd) = fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let Ok(m) = fs::symlink_metadata(e.path()) else { continue };
            if m.is_dir() {
                todo.push(e.path());
            } else if m.is_file() {
                n.0 += 1;
                n.1 += m.len();
            }
        }
    }
    n
}

/// Every snapshot in `root`, newest first.
pub fn list(root: &Path) -> Vec<Snapshot> {
    let Ok(rd) = fs::read_dir(root) else { return Vec::new() };
    let mut out: Vec<Snapshot> = rd
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| {
            let id = e.file_name().to_string_lossy().into_owned();
            let when = fs::read_to_string(e.path().join("when")).ok()?.trim().parse().ok()?;
            let (files, bytes) = size_of(&e.path());
            Some(Snapshot { id, when, bytes, files })
        })
        .collect();
    out.sort_by(|a, b| b.when.cmp(&a.when).then(b.id.cmp(&a.id)));
    out
}

/// Takes a snapshot of `folders` into `root` (a new folder named for `now`), then trims old ones. None when there
/// are no save folders (nothing to keep).
pub fn backup(folders: &[PathBuf], root: &Path, now: u64) -> io::Result<Option<Snapshot>> {
    if folders.is_empty() {
        return Ok(None);
    }
    let mut id = stamp(now);
    let mut n = 1;
    while root.join(&id).exists() {
        n += 1;
        id = format!("{}-{n}", stamp(now));
    }
    let dir = root.join(&id);
    let tmp = root.join(format!(".{id}.part"));
    let _ = fs::remove_dir_all(&tmp);
    let mut count = (0usize, 0u64);
    for (i, f) in folders.iter().enumerate() {
        copy_tree(f, &tmp.join(i.to_string()), &mut count)?;
    }
    fs::write(tmp.join("when"), now.to_string())?;
    fs::write(tmp.join("from"), folders.iter().map(|f| f.to_string_lossy().into_owned()).collect::<Vec<_>>().join("\n"))?;
    fs::rename(&tmp, &dir)?;
    trim(root, &id)?;
    Ok(Some(Snapshot { id, when: now, bytes: count.1, files: count.0 }))
}

/// Keeps the newest KEEP snapshots, and drops the oldest while all together are over MAX_BYTES (never `keep`).
fn trim(root: &Path, keep: &str) -> io::Result<()> {
    let all = list(root);
    let mut total: u64 = all.iter().map(|s| s.bytes).sum();
    for (i, s) in all.iter().enumerate().rev() {
        if s.id == keep {
            continue;
        }
        if i >= KEEP || total > MAX_BYTES {
            fs::remove_dir_all(root.join(&s.id))?;
            total = total.saturating_sub(s.bytes);
        }
    }
    Ok(())
}

/// Puts a snapshot back: first a snapshot of the saves as they are now (returned), then each kept folder's files
/// copied over the save folder (files the snapshot doesn't have are left alone). The folders must be the ones the
/// snapshot was taken from, in the same order.
pub fn restore(id: &str, folders: &[PathBuf], root: &Path, now: u64) -> io::Result<Option<Snapshot>> {
    if id.is_empty() || id.contains('/') || id.contains('\\') || id.starts_with('.') {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a backup"));
    }
    let snap = root.join(id);
    if !snap.join("when").is_file() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "that backup is gone"));
    }
    let before = backup(folders, root, now)?;
    for (i, f) in folders.iter().enumerate() {
        let kept = snap.join(i.to_string());
        if kept.is_dir() {
            let mut count = (0usize, 0u64);
            copy_tree(&kept, f, &mut count)?;
        }
    }
    Ok(before)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamps_sort_by_time() {
        assert_eq!(stamp(0), "19700101-000000");
        assert_eq!(stamp(1_794_000_000), "20261106-212000");
    }
}
