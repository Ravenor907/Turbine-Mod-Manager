//! My mods as a list: Find narrows it to the mods whose name has every word typed, Show to on, off, needing
//! attention, new or with an update, and Sort orders it (the load order, A to Z, or the newest first). Mods that
//! are mainly what other mods need (another installed mod lists it, or it is a known framework or loader) go into
//! one Dependencies row at the bottom; the player can move a mod in or out of it. Only the list changes: the load
//! order stays exactly as it is.

use crate::json::{Json, quote};
use crate::needs::{self, Links, Shown};

/// An installed mod as the list needs it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ListMod {
    pub id: String,
    /// The name shown.
    pub name: String,
    pub enabled: bool,
    /// When it was added (seconds since 1970).
    pub added: f64,
    pub needs_loader: bool,
}

/// What the player chose for the list.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Filter {
    /// Find's words ("" for none).
    pub find: String,
    /// all, on, off, attention, new, update.
    pub show: String,
    /// order, az, recent.
    pub sort: String,
    /// A mod just turned on or off under Show: it stays in its row until the focus moves ("" for none).
    pub kept: String,
    /// Dependencies are grouped (not while moving, selecting or finding).
    pub grouped: bool,
    /// The Dependencies row is open.
    pub open: bool,
    /// Mods added after this (seconds since 1970) are new.
    pub last_played_at: f64,
    /// Mods with an update ready.
    pub updates: Vec<String>,
}

/// What makes a mod a dependency.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Deps {
    /// Frameworks and loaders.
    pub core: Vec<String>,
    /// The player's moves: (mod id, true for into Dependencies).
    pub moves: Vec<(String, bool)>,
}

/// What, beyond missing requirements, makes a mod need attention.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Attention {
    /// The game's script loader isn't installed (mods that need it need attention).
    pub loader_missing: bool,
    /// Mods whose files are all covered by others.
    pub hidden: Vec<String>,
    /// Mods with a warning of their own.
    pub warned: Vec<String>,
}

/// One row of the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    /// The mod at this place in the mods given; `dep` when it shows inside the open Dependencies row.
    Mod { index: usize, dep: bool },
    /// The Dependencies row.
    Deps,
}

/// The list: the mods shown (places in the mods given, in the order shown), the ones in Dependencies, the rows,
/// and every mod that counts as a dependency (shown or not).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Listed {
    pub shown: Vec<usize>,
    pub deps: Vec<usize>,
    pub rows: Vec<Row>,
    pub dep_all: Vec<usize>,
}

/// A dependency: moved in (or out) by the player, a framework or loader, or needed by another installed mod.
pub fn is_dep(m: &ListMod, mods: &[ListMod], links: &Links, deps: &Deps) -> bool {
    if let Some((_, into)) = deps.moves.iter().find(|(id, _)| *id == m.id) {
        return *into;
    }
    if deps.core.contains(&m.id) {
        return true;
    }
    links.get(&m.id).is_some_and(|l| l.needed_by.iter().any(|id| mods.iter().any(|x| x.id == *id)))
}

/// Needs attention: needs the missing loader, or is on and misses a requirement, has all its files covered or has
/// a warning.
pub fn needs_attention(m: &ListMod, shown: &[Shown], links: &Links, attention: &Attention) -> bool {
    (m.needs_loader && attention.loader_missing)
        || (m.enabled && (!needs::missing(shown, links, &m.id).is_empty() || attention.hidden.contains(&m.id) || attention.warned.contains(&m.id)))
}

pub fn list(mods: &[ListMod], links: &Links, filter: &Filter, deps: &Deps, attention: &Attention) -> Listed {
    let shown_mods: Vec<Shown> = mods.iter().map(|m| Shown { id: m.id.clone(), name: m.name.clone(), enabled: m.enabled }).collect();
    let find = filter.find.to_lowercase();
    let words: Vec<&str> = find.split(|c: char| c.is_whitespace()).filter(|w| !w.is_empty()).collect();
    let mut shown: Vec<usize> = (0..mods.len())
        .filter(|&i| {
            let m = &mods[i];
            if !filter.kept.is_empty() && filter.kept == m.id {
                return true;
            }
            let n = m.name.to_lowercase();
            if !words.iter().all(|w| n.contains(w)) {
                return false;
            }
            match filter.show.as_str() {
                "on" => m.enabled,
                "off" => !m.enabled,
                "attention" => needs_attention(m, &shown_mods, links, attention),
                "new" => m.added > filter.last_played_at,
                "update" => filter.updates.contains(&m.id),
                _ => true,
            }
        })
        .collect();
    match filter.sort.as_str() {
        "az" => shown.sort_by_cached_key(|&i| mods[i].name.to_lowercase()),
        "recent" => shown.sort_by(|&a, &b| mods[b].added.total_cmp(&mods[a].added)),
        _ => {}
    }
    let dep_all: Vec<usize> = (0..mods.len()).filter(|&i| is_dep(&mods[i], mods, links, deps)).collect();
    let in_deps: Vec<usize> = if filter.grouped { shown.iter().copied().filter(|i| dep_all.contains(i)).collect() } else { Vec::new() };
    let mut rows: Vec<Row> = shown.iter().filter(|i| !in_deps.contains(i)).map(|&index| Row::Mod { index, dep: false }).collect();
    if !in_deps.is_empty() {
        rows.push(Row::Deps);
        if filter.open {
            rows.extend(in_deps.iter().map(|&index| Row::Mod { index, dep: true }));
        }
    }
    Listed { shown, deps: in_deps, rows, dep_all }
}

fn texts(j: Option<&Json>) -> Vec<String> {
    j.map(Json::as_array).unwrap_or(&[]).iter().filter_map(|x| x.as_str().map(str::to_string)).collect()
}

/// The list from the interface's {"mods": [{"id","name","enabled","added","needsLoader"}], "links", "find",
/// "show", "sort", "kept", "grouped", "open", "lastPlayedAt", "updates", "core", "moves": {"<id>": "in" | "out"},
/// "loaderMissing", "hidden", "warned"}, as {"shown": [i], "deps": [i], "rows": [{"i"[, "dep"]} | {"deps": true}],
/// "depAll": [ids]}.
pub fn list_json(input: &Json) -> String {
    let text = |k: &str| input.str_at(k).unwrap_or_default().to_string();
    let flag = |k: &str| input.get(k).is_some_and(Json::truthy);
    let mods: Vec<ListMod> = input
        .get("mods")
        .map(Json::as_array)
        .unwrap_or(&[])
        .iter()
        .map(|m| ListMod {
            id: m.str_at("id").unwrap_or_default().into(),
            name: m.str_at("name").unwrap_or_default().into(),
            enabled: m.get("enabled").is_some_and(Json::truthy),
            added: m.get("added").and_then(Json::as_f64).unwrap_or(0.0),
            needs_loader: m.get("needsLoader").is_some_and(Json::truthy),
        })
        .collect();
    let links = needs::links_from(input.get("links"));
    let filter = Filter {
        find: text("find"),
        show: text("show"),
        sort: text("sort"),
        kept: text("kept"),
        grouped: flag("grouped"),
        open: flag("open"),
        last_played_at: input.get("lastPlayedAt").and_then(Json::as_f64).unwrap_or(0.0),
        updates: texts(input.get("updates")),
    };
    let moves = input
        .get("moves")
        .map(Json::entries)
        .unwrap_or(&[])
        .iter()
        .filter_map(|(id, v)| match v.as_str() {
            Some("in") => Some((id.clone(), true)),
            Some("out") => Some((id.clone(), false)),
            _ => None,
        })
        .collect();
    let deps = Deps { core: texts(input.get("core")), moves };
    let attention = Attention { loader_missing: flag("loaderMissing"), hidden: texts(input.get("hidden")), warned: texts(input.get("warned")) };
    let l = list(&mods, &links, &filter, &deps, &attention);
    let nums = |v: &[usize]| v.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
    let rows: Vec<String> = l
        .rows
        .iter()
        .map(|r| match r {
            Row::Mod { index, dep: false } => format!("{{\"i\":{index}}}"),
            Row::Mod { index, dep: true } => format!("{{\"i\":{index},\"dep\":true}}"),
            Row::Deps => "{\"deps\":true}".into(),
        })
        .collect();
    let dep_ids: Vec<String> = l.dep_all.iter().map(|&i| quote(&mods[i].id)).collect();
    format!("{{\"shown\":[{}],\"deps\":[{}],\"rows\":[{}],\"depAll\":[{}]}}", nums(&l.shown), nums(&l.deps), rows.join(","), dep_ids.join(","))
}
