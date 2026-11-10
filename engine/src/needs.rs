//! Requirements both ways between installed mods: what each mod needs (and whether it is installed), and which mods
//! need it. Read from each mod's recorded source (SourceInfo::needs): Thunderstore dependency strings
//! ("Owner-Name-1.2.3"), and for Nexus mods "nexus:<domain>:<mod id>|<name>", recorded when Turbine reads the mod's
//! requirements from Nexus as it is added.

use crate::json::quote;
use crate::net::SourceInfo;

/// One thing a mod needs: its name, and the installed mod that provides it (None when it isn't installed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Need {
    pub name: String,
    pub id: Option<String>,
    /// 0.64: for a Nexus requirement, its game's domain and mod ID, so it can be got (empty and 0 otherwise).
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
        for n in &src.needs {
            let Some((key, name, domain, mod_id)) = parse_need(n) else { continue };
            if out[i].1.iter().any(|x: &Need| x.name == name) {
                continue;
            }
            let provider = keys.iter().position(|k| k.as_deref() == Some(key.as_str())).filter(|&j| j != i);
            out[i].1.push(Need { name, id: provider.map(|j| mods[j].0.clone()), domain, mod_id });
            if let Some(j) = provider {
                if !out[j].2.contains(id) {
                    out[j].2.push(id.clone());
                }
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
