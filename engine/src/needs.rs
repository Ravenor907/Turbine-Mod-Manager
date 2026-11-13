//! Requirements both ways between installed mods: what each mod needs (and whether it is installed), and which mods
//! need it. Read from each mod's recorded source (SourceInfo::needs): Thunderstore dependency strings
//! ("Owner-Name-1.2.3"), and for Nexus mods "nexus:<domain>:<mod id>|<name>", recorded when Turbine reads the mod's
//! requirements from Nexus as it is added.

use crate::json::{Json, quote};
use std::collections::HashMap;
use crate::net::SourceInfo;

/// One thing a mod needs: its name, and the installed mod that provides it (None when it isn't installed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Need {
    pub name: String,
    pub id: Option<String>,
    /// For a Nexus requirement, its game's domain and mod ID, so it can be got (empty and 0 otherwise).
    pub domain: String,
    pub mod_id: u64,
}

/// The needs entry Turbine records for a Nexus requirement.
pub fn nexus_need(domain: &str, mod_id: u64, name: &str) -> String {
    format!("nexus:{domain}:{mod_id}|{}", name.replace('|', " "))
}

/// What an installed mod provides, as a key its dependents' needs match.
fn key_of(src: &SourceInfo) -> Option<String> {
    match src.kind.as_str() {
        "thunderstore" if !src.id.is_empty() => Some(format!("ts:{}", src.id.to_lowercase())),
        "nexus" => crate::nexus::source_mod(src).map(|(_, m)| format!("nexus:{m}")),
        _ => None,
    }
}

/// A recorded need as (key, name shown, Nexus domain, Nexus mod ID). Thunderstore's loader pack is left out:
/// Turbine sets the loader up itself.
fn parse_need(n: &str) -> Option<(String, String, String, u64)> {
    if let Some(rest) = n.strip_prefix("nexus:") {
        let (ids, name) = rest.split_once('|').unwrap_or((rest, ""));
        let m = ids.rsplit(':').next()?.parse::<u64>().ok().filter(|m| *m > 0)?;
        let domain = ids.rsplit_once(':').map(|(d, _)| d.to_string()).unwrap_or_default();
        let name = if name.trim().is_empty() { format!("Nexus mod {m}") } else { name.trim().to_string() };
        return Some((format!("nexus:{m}"), name, domain, m));
    }
    // "Owner-Name-1.2.3": the version is the last part.
    let full = match n.rsplit_once('-') {
        Some((head, v)) if v.chars().next().is_some_and(|c| c.is_ascii_digit()) => head,
        _ => n,
    };
    let (owner, name) = full.split_once('-')?;
    if owner.is_empty() || name.is_empty() || name.to_lowercase().starts_with("bepinexpack") {
        return None;
    }
    Some((format!("ts:{}", full.to_lowercase()), name.replace('_', " "), String::new(), 0))
}

/// For each mod (id, its source), what it needs and which mods need it. Same order as `mods`.
pub fn links(mods: &[(String, SourceInfo)]) -> Vec<(String, Vec<Need>, Vec<String>)> {
    let keys: Vec<Option<String>> = mods.iter().map(|(_, s)| key_of(s)).collect();
    let mut out: Vec<(String, Vec<Need>, Vec<String>)> = mods.iter().map(|(id, _)| (id.clone(), Vec::new(), Vec::new())).collect();
    for (i, (id, src)) in mods.iter().enumerate() {
        // Each requirement once, told apart by what it is and not by its name: two different mods that happen to
        // share a name are both needed.
        let mut had: Vec<String> = Vec::new();
        for n in &src.needs {
            let Some((key, name, domain, mod_id)) = parse_need(n) else { continue };
            if had.contains(&key) {
                continue;
            }
            had.push(key.clone());
            let provider = keys.iter().position(|k| k.as_deref() == Some(key.as_str())).filter(|&j| j != i);
            out[i].1.push(Need { name, id: provider.map(|j| mods[j].0.clone()), domain, mod_id });
            if let Some(j) = provider && !out[j].2.contains(id) {
                out[j].2.push(id.clone());
            }
        }
    }
    out
}

/// The links as JSON for the interface: {"<mod id>": {"needs": [{"name","id"}], "neededBy": [ids]}}, only mods
/// with something to say.
pub fn links_json(list: &[(String, Vec<Need>, Vec<String>)]) -> String {
    let items: Vec<String> = list
        .iter()
        .filter(|(_, n, by)| !n.is_empty() || !by.is_empty())
        .map(|(id, n, by)| {
            let needs: Vec<String> = n
                .iter()
                .map(|x| {
                    format!(
                        "{{\"name\":{},\"id\":{},\"domain\":{},\"modId\":{}}}",
                        quote(&x.name),
                        x.id.as_deref().map(quote).unwrap_or_else(|| "null".into()),
                        quote(&x.domain),
                        x.mod_id
                    )
                })
                .collect();
            let by: Vec<String> = by.iter().map(|b| quote(b)).collect();
            format!("{}:{{\"needs\":[{}],\"neededBy\":[{}]}}", quote(id), needs.join(","), by.join(","))
        })
        .collect();
    format!("{{{}}}", items.join(","))
}

// ---------------------------------------------------------------- what the interface asks about requirements


/// An installed mod as the interface shows it: its id, the name shown, and whether it is on.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Shown {
    pub id: String,
    pub name: String,
    pub enabled: bool,
}

/// One mod's entry in the links the interface holds (`links_json`): what it needs, and which mods need it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Link {
    pub needs: Vec<Need>,
    pub needed_by: Vec<String>,
}

pub type Links = HashMap<String, Link>;

/// A requirement to get: its name, Nexus mod ID and game domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetItem {
    pub name: String,
    pub mod_id: u64,
    pub domain: String,
}

/// What a mod needs that isn't on: installed mods that are off (their ids), and mods to get.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Split {
    pub off: Vec<String>,
    pub get: Vec<GetItem>,
}

/// Every mod that is on together: what they miss (each once), who misses something, and words naming who needs what.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OnAll {
    pub off: Vec<String>,
    pub get: Vec<GetItem>,
    pub names: Vec<String>,
    pub words: String,
}

fn text_at(j: &Json, key: &str) -> String {
    j.str_at(key).unwrap_or_default().to_string()
}

fn list_of_text(j: Option<&Json>) -> Vec<String> {
    j.map(Json::as_array).unwrap_or(&[]).iter().filter_map(|x| x.as_str().map(str::to_string)).collect()
}

/// The mods from the interface's list ([{"id","name","enabled"}]).
pub fn shown_from(j: Option<&Json>) -> Vec<Shown> {
    j.map(Json::as_array)
        .unwrap_or(&[])
        .iter()
        .map(|m| Shown { id: text_at(m, "id"), name: text_at(m, "name"), enabled: m.get("enabled").is_some_and(Json::truthy) })
        .collect()
}

/// One need as the interface writes it ({"name","id","domain","modId"}); an empty or null id is no provider.
fn need_from(n: &Json) -> Need {
    Need {
        name: text_at(n, "name"),
        id: n.str_at("id").filter(|s| !s.is_empty()).map(str::to_string),
        domain: text_at(n, "domain"),
        mod_id: n.get("modId").and_then(Json::as_f64).filter(|m| *m > 0.0).map(|m| m as u64).unwrap_or(0),
    }
}

/// The links from the interface ({"<mod id>": {"needs": [...], "neededBy": [ids]}}).
pub fn links_from(j: Option<&Json>) -> Links {
    j.map(Json::entries)
        .unwrap_or(&[])
        .iter()
        .map(|(id, l)| {
            let needs = l.get("needs").map(Json::as_array).unwrap_or(&[]).iter().map(need_from).collect();
            (id.clone(), Link { needs, needed_by: list_of_text(l.get("neededBy")) })
        })
        .collect()
}

/// A need as JSON, the way `links_json` writes it.
pub fn need_json(n: &Need) -> String {
    format!(
        "{{\"name\":{},\"id\":{},\"domain\":{},\"modId\":{}}}",
        quote(&n.name),
        n.id.as_deref().map(quote).unwrap_or_else(|| "null".into()),
        quote(&n.domain),
        n.mod_id
    )
}

/// What mod `id` needs that isn't on: needs with no installed provider, or whose provider is off.
pub fn missing<'a>(mods: &[Shown], links: &'a Links, id: &str) -> Vec<&'a Need> {
    let Some(l) = links.get(id) else { return Vec::new() };
    l.needs.iter().filter(|n| n.id.as_ref().is_none_or(|p| !mods.iter().any(|x| &x.id == p && x.enabled))).collect()
}

/// What mod `id` misses, split: installed but off (turned on by Get them all), and not installed with a Nexus mod
/// ID (got). A need with no domain of its own takes the game's `domain`.
pub fn split(mods: &[Shown], links: &Links, id: &str, domain: &str) -> Split {
    let miss = missing(mods, links, id);
    let off = miss.iter().filter_map(|n| n.id.as_ref()).filter(|p| mods.iter().any(|x| &x.id == *p)).cloned().collect();
    let get = miss
        .iter()
        .filter(|n| n.id.is_none() && n.mod_id > 0)
        .map(|n| GetItem { name: n.name.clone(), mod_id: n.mod_id, domain: if n.domain.is_empty() { domain.to_string() } else { n.domain.clone() } })
        .collect();
    Split { off, get }
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// Every mod that is on, in list order: what they miss together (each mod to turn on and each mod to get once),
/// their names, and words for the first three ("A needs B and C.") with "And N more mods." after.
pub fn missing_on(mods: &[Shown], links: &Links, domain: &str) -> OnAll {
    let mut all = OnAll::default();
    let mut lines: Vec<String> = Vec::new();
    for m in mods.iter().filter(|m| m.enabled) {
        let miss = missing(mods, links, &m.id);
        if miss.is_empty() {
            continue;
        }
        all.names.push(m.name.clone());
        if lines.len() < 3 {
            let names: Vec<&str> = miss.iter().map(|n| n.name.as_str()).collect();
            lines.push(format!("{} needs {}.", m.name, names.join(" and ")));
        }
        let r = split(mods, links, &m.id, domain);
        for x in r.off {
            if !all.off.contains(&x) {
                all.off.push(x);
            }
        }
        for x in r.get {
            if !all.get.iter().any(|y| y.mod_id == x.mod_id) {
                all.get.push(x);
            }
        }
    }
    if all.names.len() > 3 {
        lines.push(format!("And {}.", plural(all.names.len() - 3, "more mod", "more mods")));
    }
    all.words = lines.join(" ");
    all
}

impl GetItem {
    pub fn from_json(j: &Json) -> GetItem {
        GetItem {
            name: text_at(j, "name"),
            mod_id: j.get("modId").and_then(Json::as_f64).filter(|m| *m > 0.0).map(|m| m as u64).unwrap_or(0),
            domain: text_at(j, "domain"),
        }
    }

    pub fn to_json(&self) -> String {
        format!("{{\"name\":{},\"modId\":{},\"domain\":{}}}", quote(&self.name), self.mod_id, quote(&self.domain))
    }
}

pub fn items_from(j: Option<&Json>) -> Vec<GetItem> {
    j.map(Json::as_array).unwrap_or(&[]).iter().map(GetItem::from_json).collect()
}

pub fn items_json(items: &[GetItem]) -> String {
    format!("[{}]", items.iter().map(GetItem::to_json).collect::<Vec<_>>().join(","))
}

fn ids_json(ids: &[String]) -> String {
    format!("[{}]", ids.iter().map(|i| quote(i)).collect::<Vec<_>>().join(","))
}

impl Split {
    pub fn to_json(&self) -> String {
        format!("{{\"off\":{},\"get\":{}}}", ids_json(&self.off), items_json(&self.get))
    }
}

impl OnAll {
    pub fn to_json(&self) -> String {
        format!("{{\"off\":{},\"get\":{},\"names\":{},\"words\":{}}}", ids_json(&self.off), items_json(&self.get), ids_json(&self.names), quote(&self.words))
    }
}

/// The interface's view of requirements in one answer, from {"mods","links","domain"}: for each mod that misses
/// something, its missing needs and their split; and what the mods that are on miss together.
/// {"missing": {"<id>": [need]}, "split": {"<id>": {"off","get"}}, "all": {"off","get","names","words"}}
pub fn view_json(input: &Json) -> String {
    let mods = shown_from(input.get("mods"));
    let links = links_from(input.get("links"));
    let domain = input.str_at("domain").unwrap_or_default();
    let mut missing_parts = Vec::new();
    let mut split_parts = Vec::new();
    for m in &mods {
        let miss = missing(&mods, &links, &m.id);
        if miss.is_empty() {
            continue;
        }
        missing_parts.push(format!("{}:[{}]", quote(&m.id), miss.iter().map(|n| need_json(n)).collect::<Vec<_>>().join(",")));
        split_parts.push(format!("{}:{}", quote(&m.id), split(&mods, &links, &m.id, domain).to_json()));
    }
    format!(
        "{{\"missing\":{{{}}},\"split\":{{{}}},\"all\":{}}}",
        missing_parts.join(","),
        split_parts.join(","),
        missing_on(&mods, &links, domain).to_json()
    )
}

/// The requirements being got, one after another: for which mod, the list, and how far along it is. Kept between
/// runs, so My mods can go on where it left off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReqQueue {
    pub game: String,
    pub for_name: String,
    pub items: Vec<GetItem>,
    pub index: usize,
}

impl ReqQueue {
    pub fn new(game: &str, for_name: &str, items: Vec<GetItem>) -> ReqQueue {
        ReqQueue { game: game.into(), for_name: for_name.into(), items, index: 0 }
    }

    /// The one being got now (None once every one is done).
    pub fn current(&self) -> Option<&GetItem> {
        self.items.get(self.index)
    }

    /// What a mod got along the way needs: the ones not already in the list go right after the current place,
    /// before the rest. Returns the ones added.
    pub fn insert_next(&mut self, got: &[GetItem]) -> Vec<GetItem> {
        let mut fresh: Vec<GetItem> = Vec::new();
        for x in got {
            // Each one once, even when the same requirement is named twice in what came in.
            if !self.items.iter().chain(fresh.iter()).any(|y| y.mod_id == x.mod_id) {
                fresh.push(x.clone());
            }
        }
        let at = self.index.min(self.items.len());
        self.items.splice(at..at, fresh.iter().cloned());
        fresh
    }

    /// Moves on to the next one. Returns the one that was current.
    pub fn advance(&mut self) -> Option<GetItem> {
        let done = self.current().cloned();
        // A saved list with a wild index must not wrap around to the start and get everything again.
        self.index = self.index.saturating_add(1);
        done
    }

    /// Reads a saved list; None without an "items" list.
    pub fn from_json(j: &Json) -> Option<ReqQueue> {
        let items = j.get("items").filter(|i| matches!(i, Json::Arr(_)))?;
        let items = items_from(Some(items));
        let index = j.get("index").and_then(Json::as_f64).filter(|i| *i > 0.0).map(|i| i as usize).unwrap_or(0);
        Some(ReqQueue { game: text_at(j, "game"), for_name: text_at(j, "forName"), items, index })
    }

    pub fn to_json(&self) -> String {
        format!("{{\"game\":{},\"forName\":{},\"items\":{},\"index\":{}}}", quote(&self.game), quote(&self.for_name), items_json(&self.items), self.index)
    }
}
