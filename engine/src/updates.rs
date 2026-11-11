//! Update checks: compares installed mods with the Thunderstore index and Nexus's per-game update lists. Pure
//! functions; the app fetches the index (thunderstore::fetch_index) and Nexus lists (Nexus::updated, Nexus::files)
//! and passes them in, so one Nexus request covers every mod of a game.

use crate::nexus::{self, FileList};
use crate::thunderstore::Package;
use std::cmp::Ordering;
use std::collections::HashMap;

/// Compares two mod versions part by part: numbers as numbers (1.10 is after 1.9), missing parts as zero (1.2 equals
/// 1.2.0), a leading "v" ignored, and a pre-release tag ("1.0-beta") before the plain release.
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    fn split(v: &str) -> (Vec<&str>, Option<&str>) {
        let v = v.trim();
        let v = v.strip_prefix(['v', 'V']).unwrap_or(v);
        let v = v.split('+').next().unwrap_or(v);
        let (core, pre) = match v.split_once('-') {
            Some((c, p)) => (c, Some(p)),
            None => (v, None),
        };
        (core.split(['.', '_']).filter(|p| !p.is_empty()).collect(), pre)
    }
    fn part(p: &str) -> (u128, &str) {
        let digits = p.bytes().take_while(u8::is_ascii_digit).count();
        let n = p[..digits].parse::<u128>().unwrap_or(0);
        (n, &p[digits..])
    }
    let (pa, pre_a) = split(a);
    let (pb, pre_b) = split(b);
    for i in 0..pa.len().max(pb.len()) {
        let (na, ra) = part(pa.get(i).copied().unwrap_or("0"));
        let (nb, rb) = part(pb.get(i).copied().unwrap_or("0"));
        let o = na.cmp(&nb).then_with(|| match (ra.is_empty(), rb.is_empty()) {
            // A bare number is the release: "2" comes after "2rc".
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            _ => ra.to_lowercase().cmp(&rb.to_lowercase()),
        });
        if o != Ordering::Equal {
            return o;
        }
    }
    match (pre_a, pre_b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(x), Some(y)) => compare_versions(x, y),
    }
}

/// One installed mod, as the store records it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Installed {
    /// SourceInfo.kind: "thunderstore", "nexus", "zip" or "loader".
    pub kind: String,
    /// SourceInfo.id: "Owner-Name" for Thunderstore, "<domain>:<mod>:<file>" for Nexus.
    pub id: String,
    pub version: Option<String>,
    /// When it was installed (Unix seconds). Nexus checks use it to skip mods with no newer file.
    pub installed_at: u64,
}

/// An update that can be installed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Update {
    pub kind: String,
    pub id: String,
    pub installed_version: Option<String>,
    pub new_version: String,
    /// Thunderstore: the "Owner-Name-1.2.3" to resolve. Nexus: the new file's id as text.
    pub target: String,
    /// The package was marked deprecated by its author (shown as a note, still offered).
    pub deprecated: bool,
}

/// Thunderstore mods with a newer version in the index.
pub fn thunderstore_updates(installed: &[Installed], index: &[Package]) -> Vec<Update> {
    let by_name: HashMap<String, &Package> = index.iter().map(|p| (p.full_name.to_lowercase(), p)).collect();
    let mut out = Vec::new();
    for m in installed.iter().filter(|m| m.kind == "thunderstore") {
        let Some(p) = by_name.get(&m.id.to_lowercase()) else { continue };
        let Some(latest) = p.latest() else { continue };
        let newer = match &m.version {
            Some(v) => compare_versions(&latest.number, v) == Ordering::Greater,
            None => true,
        };
        if newer {
            out.push(Update {
                kind: "thunderstore".into(),
                id: p.full_name.clone(),
                installed_version: m.version.clone(),
                new_version: latest.number.clone(),
                target: format!("{}-{}", p.full_name, latest.number),
                deprecated: p.is_deprecated,
            });
        }
    }
    out
}

/// The Nexus mod of an installed mod's source id ("<domain>:<mod>:<file>" or "<domain>:<mod>"), for this domain.
fn nexus_mod_of(id: &str, domain: &str) -> Option<u64> {
    let mut parts = id.split(':');
    let (d, m) = (parts.next()?, parts.next()?);
    (d == domain).then(|| m.parse().ok()).flatten().filter(|m| *m > 0)
}

/// The Nexus mod ids of one game's installed Nexus mods, each once (what `Nexus::mods_by_id` is asked about).
pub fn nexus_mod_ids(domain: &str, installed: &[Installed]) -> Vec<u64> {
    let mut out: Vec<u64> = Vec::new();
    for m in installed.iter().filter(|m| m.kind == "nexus") {
        if let Some(id) = nexus_mod_of(&m.id, domain) && !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

/// Without a key: Nexus mods whose page changed after they were installed, or whose page gives a newer version,
/// from `Nexus::mods_by_id`. Returns (mod id, installed entry, Nexus's game number for the files query); the app then
/// reads each one's files (`Nexus::files_public`) and calls `nexus_update_from_files`.
pub fn nexus_stamp_candidates<'a>(domain: &str, installed: &'a [Installed], stamps: &[nexus::ModStamp]) -> Vec<(u64, &'a Installed, u64)> {
    let mut out: Vec<(u64, &Installed, u64)> = Vec::new();
    for m in installed.iter().filter(|m| m.kind == "nexus") {
        let Some(mod_id) = nexus_mod_of(&m.id, domain) else { continue };
        let Some(st) = stamps.iter().find(|s| s.mod_id == mod_id) else { continue };
        let newer_page = st.updated > m.installed_at;
        let newer_version = match (&m.version, st.version.is_empty()) {
            (Some(v), false) => compare_versions(&st.version, v) == Ordering::Greater,
            _ => false,
        };
        if (newer_page || newer_version) && !out.iter().any(|(id, _, _)| *id == mod_id) {
            out.push((mod_id, m, st.game_id));
        }
    }
    out
}

/// Whether a Nexus mod's file list has something newer than the installed file: first the author's "replaces"
/// links, then a newer version among the current main files.
pub fn nexus_update_from_files(installed: &Installed, files: &FileList) -> Option<Update> {
    // "<domain>:<mod>" (a download caught without its file id) counts as file 0: only a newer main version updates it.
    let (domain, mod_id, file_id) = nexus::parse_source_id(&installed.id).or_else(|| {
        let (d, m) = installed.id.split_once(':')?;
        Some((d.to_string(), m.parse().ok()?, 0))
    })?;
    let current = files.files.iter().find(|f| f.file_id == file_id);
    let candidate = files.newest_replacement(file_id).or_else(|| {
        let main = files.main_file()?;
        if main.file_id == file_id {
            return None;
        }
        let have = installed.version.as_deref().or(current.map(|c| c.version.as_str())).unwrap_or("");
        let newer_version = !main.version.is_empty() && !have.is_empty() && compare_versions(&main.version, have) == Ordering::Greater;
        // Only offer a different main file if the installed one was a main file too (not an optional add-on).
        let was_main = current.map(|c| c.category == "MAIN" || c.category == "OLD_VERSION" || c.is_primary).unwrap_or(true);
        (newer_version && was_main).then_some(main)
    })?;
    Some(Update {
        kind: "nexus".into(),
        id: nexus::source_id(&domain, mod_id, candidate.file_id),
        installed_version: installed.version.clone(),
        new_version: candidate.version.clone(),
        target: candidate.file_id.to_string(),
        deprecated: false,
    })
}
