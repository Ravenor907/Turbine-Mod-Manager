//! Who wins when two mods change the same file: one pass over the turned-on mods' files in Turbine's order. The
//! later mod in the list wins a file (the same rule deploy follows), paths compared ignoring case (the games run
//! under Proton, where case doesn't matter). Reads only Turbine's own list of each mod's files.

use crate::rules::ModFiles;
use std::collections::{HashMap, HashSet};

/// One other mod this one shares files with, and how many.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shared {
    pub id: String,
    pub name: String,
    pub count: usize,
    /// Up to `SAMPLE` of the shared files (game-relative), for the Overlaps sheet.
    pub files: Vec<String>,
}

/// One turned-on mod's overlaps.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Overlap {
    pub id: String,
    /// Mods it wins files over (they come earlier in the list), most files first.
    pub wins: Vec<Shared>,
    /// Mods it loses files to (they come later), most files first.
    pub loses: Vec<Shared>,
    /// Every one of its files is replaced by a later mod: it changes nothing in the game.
    pub hidden: bool,
}

pub const SAMPLE: usize = 12;

fn add(list: &mut Vec<Shared>, id: &str, name: &str, file: &str) {
    match list.iter_mut().find(|s| s.id == id) {
        Some(s) => {
            s.count += 1;
            if s.files.len() < SAMPLE {
                s.files.push(file.to_string());
            }
        }
        None => list.push(Shared { id: id.into(), name: name.into(), count: 1, files: vec![file.to_string()] }),
    }
}

/// The overlaps of every turned-on mod that shares at least one file, in list order. Turned-off mods take no part.
pub fn overlaps(mods: &[ModFiles]) -> Vec<Overlap> {
    let on: Vec<&ModFiles> = mods.iter().filter(|m| m.enabled).collect();
    // For each file (lowercase), the mods that have it, in order, with the path as the first one spells it.
    let mut by_file: HashMap<String, (String, Vec<usize>)> = HashMap::new();
    let mut keys: Vec<String> = Vec::new();
    for (i, m) in on.iter().enumerate() {
        let mut seen: HashSet<String> = HashSet::new();
        for f in &m.files {
            let k = f.replace('\\', "/").trim_start_matches('/').to_lowercase();
            if !seen.insert(k.clone()) {
                continue;
            }
            let e = by_file.entry(k.clone()).or_insert_with(|| {
                keys.push(k.clone());
                (f.clone(), Vec::new())
            });
            e.1.push(i);
        }
    }
    let mut out: Vec<Overlap> = on.iter().map(|m| Overlap { id: m.id.clone(), ..Overlap::default() }).collect();
    let mut won = vec![0usize; on.len()];
    let mut total = vec![0usize; on.len()];
    for k in &keys {
        let (path, who) = &by_file[k];
        for &i in who {
            total[i] += 1;
        }
        let Some(&winner) = who.last() else { continue };
        won[winner] += 1;
        for &loser in &who[..who.len() - 1] {
            add(&mut out[winner].wins, &on[loser].id, &on[loser].name, path);
            add(&mut out[loser].loses, &on[winner].id, &on[winner].name, path);
        }
    }
    for (i, o) in out.iter_mut().enumerate() {
        o.hidden = total[i] > 0 && won[i] == 0;
        o.wins.sort_by_key(|s| std::cmp::Reverse(s.count));
        o.loses.sort_by_key(|s| std::cmp::Reverse(s.count));
    }
    out.retain(|o| !o.wins.is_empty() || !o.loses.is_empty());
    out
}

/// The overlaps as JSON for the interface: {"<mod id>": {"wins": [...], "loses": [...], "hidden": bool}}.
pub fn to_json(list: &[Overlap]) -> String {
    use crate::json::quote;
    let shared = |v: &[Shared]| {
        v.iter()
            .map(|s| {
                format!(
                    "{{\"id\":{},\"name\":{},\"count\":{},\"files\":[{}]}}",
                    quote(&s.id),
                    quote(&s.name),
                    s.count,
                    s.files.iter().map(|f| quote(f)).collect::<Vec<_>>().join(",")
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    };
    let items: Vec<String> = list
        .iter()
        .map(|o| format!("{}:{{\"wins\":[{}],\"loses\":[{}],\"hidden\":{}}}", quote(&o.id), shared(&o.wins), shared(&o.loses), o.hidden))
        .collect();
    format!("{{{}}}", items.join(","))
}
