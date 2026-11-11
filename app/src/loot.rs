//! LOOT sorting for Skyrim Special Edition, through LOOT's own library (libloot) and its community masterlist.
//!
//! The masterlist and the prelude it uses come from LOOT's repositories on GitHub (through the engine's download
//! client, so only allowed hosts and checked downloads), at most once a day. A new copy replaces the old one only
//! after LOOT itself has loaded it without errors, so a bad download never takes away the last good copy.
//!
//! Sorting reads the plugins with LOOT, asks it for the order, and the engine's load order code writes it
//! (plugins.txt and loadorder.txt in the game's Proton folder, with the same backups as every other change). LOOT's
//! messages for each plugin (dirty plugins, incompatibilities, missing requirements, notes) are shown in the Load
//! order tab.

use libloot::metadata::{select_message_content, MessageType};
use libloot::{EvalMode, GameType, MergeMode};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use turbine_engine::net::{Client, NetError, Request};
use turbine_engine::plugins::{Files, LoadOrder};
use turbine_engine::Game;

/// The masterlist branch for the metadata syntax this libloot reads (v0.29), through GitHub's API so the file comes
/// from an allowed host. "raw" answers with the file itself.
pub const MASTERLIST_URL: &str = "https://api.github.com/repos/loot/skyrimse/contents/masterlist.yaml?ref=v0.29";
pub const PRELUDE_URL: &str = "https://api.github.com/repos/loot/prelude/contents/prelude.yaml?ref=v0.29";
/// Checked for a newer masterlist at most this often.
pub const REFRESH_AFTER: Duration = Duration::from_secs(24 * 3600);
/// The masterlist is about 1 MB; anything far bigger isn't it.
const MAX_LIST_BYTES: u64 = 16 << 20;

/// The masterlist and prelude on this device.
#[derive(Clone, Debug)]
pub struct Lists {
    pub masterlist: PathBuf,
    pub prelude: PathBuf,
    /// When this copy was downloaded (seconds since 1970), 0 for the recorded samples.
    pub fetched: u64,
}

/// One of LOOT's messages for a plugin. level: "say", "warn" or "error".
#[derive(Clone, Debug, PartialEq)]
pub struct Msg {
    pub level: &'static str,
    pub text: String,
}

/// What a LOOT sort found.
#[derive(Clone, Debug, Default)]
pub struct Sorted {
    /// Every plugin LOOT read, in the order it wants.
    pub order: Vec<String>,
    /// Plugins LOOT couldn't read (they keep their place after the others).
    pub unread: Vec<String>,
    pub messages: HashMap<String, Vec<Msg>>,
    pub general: Vec<Msg>,
}

/// Where the masterlist is kept, in Turbine's cache folder.
pub fn folder(cache: &Path) -> PathBuf {
    cache.join("loot").join("skyrimse")
}

fn stamp_file(dir: &Path) -> PathBuf {
    dir.join("fetched")
}

/// The copy on this device, if there is a complete one.
pub fn on_device(dir: &Path) -> Option<Lists> {
    let masterlist = dir.join("masterlist.yaml");
    let prelude = dir.join("prelude.yaml");
    if !masterlist.is_file() || !prelude.is_file() {
        return None;
    }
    let fetched = fs::read_to_string(stamp_file(dir)).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
    Some(Lists { masterlist, prelude, fetched })
}

/// The recorded samples (the build's screenshot checks): loot_masterlist.yaml and loot_prelude.yaml.
pub fn samples(dir: &Path) -> Option<Lists> {
    let masterlist = dir.join("loot_masterlist.yaml");
    let prelude = dir.join("loot_prelude.yaml");
    (masterlist.is_file() && prelude.is_file()).then_some(Lists { masterlist, prelude, fetched: 0 })
}

/// Loads a masterlist and prelude with LOOT, to know they work before they replace the copy in use.
fn check_lists(masterlist: &Path, prelude: &Path, scratch: &Path) -> Result<(), String> {
    let game_dir = scratch.join("game");
    fs::create_dir_all(game_dir.join("Data")).map_err(|e| format!("couldn't make a check folder ({e})"))?;
    let game = libloot::Game::with_local_path(GameType::SkyrimSE, &game_dir, &scratch.join("local"))
        .map_err(|e| format!("LOOT couldn't start ({e})"))?;
    let db = game.database();
    let mut db = db.write().map_err(|_| "LOOT's data was left unusable".to_string())?;
    db.load_masterlist_with_prelude(masterlist, prelude).map_err(|e| format!("LOOT couldn't read it ({e})"))
}

/// 0.63: fetches one list. With the copy from before and the tag GitHub gave for it, GitHub is asked only whether it
/// changed (If-None-Match): "not changed" (304) keeps that copy and doesn't count against GitHub's hourly limit. A
/// limit from GitHub gets its own words. Returns the tag of what is now in `to`.
fn fetch(client: &Client, url: &str, to: &Path, have: &Path) -> Result<Option<String>, String> {
    let tag_file = etag_file(have);
    let known = fs::read_to_string(&tag_file).ok().map(|t| t.trim().to_string()).filter(|t| !t.is_empty() && have.is_file());
    let mut req = Request::get(url).header("Accept", "application/vnd.github.raw").max_bytes(MAX_LIST_BYTES).max_time(120);
    if let Some(tag) = &known {
        req = req.header("If-None-Match", tag);
    }
    let resp = match client.send(&req, None) {
        Ok(r) => r,
        Err(NetError::Http(304)) if known.is_some() => {
            fs::copy(have, to).map_err(|e| format!("couldn't keep it ({e})"))?;
            return Ok(known);
        }
        Err(NetError::GitHubLimit(m)) => {
            return Err(format!("GitHub is limiting how often the list can be fetched, try again in about {}", turbine_engine::net::minutes_words(m)))
        }
        Err(e) => return Err(e.to_string()),
    };
    let tag = resp.header("etag").map(str::to_string);
    let len = fs::metadata(&resp.body).map(|m| m.len()).unwrap_or(0);
    if len < 16 {
        let _ = fs::remove_file(&resp.body);
        return Err("the answer was empty".into());
    }
    fs::rename(&resp.body, to).or_else(|_| fs::copy(&resp.body, to).map(|_| ())).map_err(|e| format!("couldn't keep it ({e})"))?;
    let _ = fs::remove_file(&resp.body);
    Ok(tag)
}

/// Where the tag GitHub gave for a list is kept, beside the list.
fn etag_file(list: &Path) -> PathBuf {
    let mut name = list.file_name().unwrap_or_default().to_os_string();
    name.push(".etag");
    list.with_file_name(name)
}

/// The masterlist to sort with: the copy on this device when it's under a day old (or `force` is false and there is
/// one); otherwise a fresh download, loaded by LOOT before it replaces the old copy. The second value says what
/// happened when something didn't work (the last good copy is used then, if there is one).
pub fn lists(client: &Client, dir: &Path, force: bool) -> (Option<Lists>, String) {
    let have = on_device(dir);
    if let Some(l) = &have {
        let age = crate::now_secs().saturating_sub(l.fetched);
        if !force && age < REFRESH_AFTER.as_secs() {
            return (have, String::new());
        }
    }
    let result = (|| -> Result<Lists, String> {
        let new = dir.join("new");
        let _ = fs::remove_dir_all(&new);
        fs::create_dir_all(&new).map_err(|e| format!("couldn't make its folder ({e})"))?;
        let (ml, pre) = (new.join("masterlist.yaml"), new.join("prelude.yaml"));
        let ml_tag = fetch(client, MASTERLIST_URL, &ml, &dir.join("masterlist.yaml"))?;
        let pre_tag = fetch(client, PRELUDE_URL, &pre, &dir.join("prelude.yaml"))?;
        check_lists(&ml, &pre, &new.join("check"))?;
        for (from, name, tag) in [(&ml, "masterlist.yaml", ml_tag), (&pre, "prelude.yaml", pre_tag)] {
            fs::rename(from, dir.join(name)).map_err(|e| format!("couldn't keep it ({e})"))?;
            let _ = fs::write(etag_file(&dir.join(name)), tag.unwrap_or_default());
        }
        let at = crate::now_secs();
        let _ = fs::write(stamp_file(dir), at.to_string());
        let _ = fs::remove_dir_all(&new);
        on_device(dir).ok_or_else(|| "it went missing after the download".to_string())
    })();
    match result {
        Ok(l) => {
            crate::log::line("loot", &format!("Masterlist updated ({} bytes)", fs::metadata(&l.masterlist).map(|m| m.len()).unwrap_or(0)));
            (Some(l), String::new())
        }
        Err(e) => {
            crate::log::line("loot", &format!("Masterlist not updated: {e}"));
            let note = match &have {
                Some(_) => format!("The community sort list couldn't be updated ({e}), so the copy from before was used."),
                None => format!("The community sort list couldn't be downloaded ({e})."),
            };
            (have, note)
        }
    }
}

/// A LOOT handle for the game, with the masterlist loaded.
fn handle(game: &Game, lists: &Lists) -> Result<libloot::Game, String> {
    let files = Files::for_game(game);
    // LOOT reads the active plugins from the same folder the game uses (in the Proton prefix). Without a prefix yet
    // it gets a folder that doesn't exist, which it reads as "nothing active".
    let local = files.settings.clone().unwrap_or_else(|| game.install_dir.join(".turbine-no-settings"));
    let mut g = libloot::Game::with_local_path(GameType::SkyrimSE, &game.install_dir, &local)
        .map_err(|e| format!("LOOT couldn't open the game ({e})"))?;
    {
        let db = g.database();
        let mut db = db.write().map_err(|_| "LOOT's data was left unusable".to_string())?;
        db.load_masterlist_with_prelude(&lists.masterlist, &lists.prelude)
            .map_err(|e| format!("LOOT couldn't read its masterlist ({e})"))?;
    }
    if let Err(e) = g.load_current_load_order_state() {
        // Conditions about active plugins then read as "not active"; sorting still works.
        crate::log::line("loot", &format!("Couldn't read the current load order state: {e}"));
    }
    Ok(g)
}

/// Makes LOOT's message text plain: links become their words, emphasis marks go.
pub fn plain(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        let after = &rest[open + 1..];
        match after.find("](").and_then(|close| after[close + 2..].find(')').map(|end| (close, close + 2 + end))) {
            Some((close, end)) if !after[..close].contains('[') => {
                out.push_str(&rest[..open]);
                out.push_str(&after[..close]);
                rest = &after[end + 1..];
            }
            _ => {
                out.push_str(&rest[..open + 1]);
                rest = after;
            }
        }
    }
    out.push_str(rest);
    let out = out.replace("**", "").replace("__", "").replace('`', "");
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn level(t: MessageType) -> &'static str {
    match t {
        MessageType::Say => "say",
        MessageType::Warn => "warn",
        MessageType::Error => "error",
    }
}

fn text_of(content: &[libloot::metadata::MessageContent]) -> String {
    select_message_content(content, "en").map(|c| plain(c.text())).unwrap_or_default()
}

/// Is a file LOOT names (a plugin or another file under Data) there? Case doesn't matter, as on Windows.
fn present(data: &Path, plugins: &HashMap<String, bool>, name: &str) -> bool {
    let lc = name.to_lowercase();
    if let Some(on) = plugins.get(&lc) {
        return *on;
    }
    !name.contains("..") && turbine_engine::rules::find_ci(data, &name.replace('\\', "/")).is_some()
}

/// LOOT's messages for each plugin (names lowercased), from the loaded masterlist.
fn plugin_messages(g: &libloot::Game, lo: &LoadOrder) -> Result<HashMap<String, Vec<Msg>>, String> {
    let data = lo.files().data.clone();
    let state: HashMap<String, bool> = lo.plugins().iter().map(|p| (p.name.to_lowercase(), p.active)).collect();
    let db = g.database();
    let db = db.read().map_err(|_| "LOOT's data was left unusable".to_string())?;
    let mut out = HashMap::new();
    for p in lo.plugins() {
        let meta = match db.plugin_metadata(&p.name, MergeMode::WithUserMetadata, EvalMode::Evaluate) {
            Ok(Some(m)) => m,
            Ok(None) => continue,
            Err(e) => {
                crate::log::line("loot", &format!("Couldn't read LOOT's notes for {}: {e}", p.name));
                continue;
            }
        };
        let mut list = Vec::new();
        for m in meta.messages() {
            let text = text_of(m.content());
            if !text.is_empty() {
                list.push(Msg { level: level(m.message_type()), text });
            }
        }
        for d in meta.dirty_info() {
            let mut parts = Vec::new();
            if d.itm_count() > 0 {
                parts.push(format!("{} records identical to the game's", d.itm_count()));
            }
            if d.deleted_reference_count() > 0 {
                parts.push(format!("{} deleted references", d.deleted_reference_count()));
            }
            if d.deleted_navmesh_count() > 0 {
                parts.push(format!("{} deleted navmeshes", d.deleted_navmesh_count()));
            }
            let mut text = String::from("Dirty: it has leftover edits");
            if !parts.is_empty() {
                text.push_str(&format!(" ({})", parts.join(", ")));
            }
            text.push_str(&format!(". The community list suggests cleaning it with {}.", plain(d.cleaning_utility())));
            let more = text_of(d.detail());
            if !more.is_empty() {
                text.push(' ');
                text.push_str(&more);
            }
            list.push(Msg { level: "warn", text });
        }
        if p.active {
            for f in meta.incompatibilities() {
                let name = f.name().as_str();
                if present(&data, &state, name) {
                    let shown = f.display_name().map(plain).unwrap_or_else(|| name.to_string());
                    let more = text_of(f.detail());
                    list.push(Msg { level: "error", text: format!("Doesn't work with {shown}, which is on.{}", if more.is_empty() { String::new() } else { format!(" {more}") }) });
                }
            }
            for f in meta.requirements() {
                let name = f.name().as_str();
                if !present(&data, &state, name) {
                    let shown = f.display_name().map(plain).unwrap_or_else(|| name.to_string());
                    let more = text_of(f.detail());
                    list.push(Msg { level: "error", text: format!("Needs {shown}, which isn't there or is off.{}", if more.is_empty() { String::new() } else { format!(" {more}") }) });
                }
            }
        }
        if !list.is_empty() {
            out.insert(p.name.to_lowercase(), list);
        }
    }
    Ok(out)
}

fn general(g: &libloot::Game) -> Vec<Msg> {
    let db = g.database();
    let Ok(db) = db.read() else { return Vec::new() };
    db.general_messages(MergeMode::WithUserMetadata, EvalMode::Evaluate)
        .map(|v| v.iter().map(|m| Msg { level: level(m.message_type()), text: text_of(m.content()) }).filter(|m| !m.text.is_empty()).collect())
        .unwrap_or_default()
}

fn paths(lo: &LoadOrder) -> Vec<PathBuf> {
    lo.plugins().iter().map(|p| PathBuf::from(&p.name)).collect()
}

/// LOOT's messages for the plugins as they are (reads only the plugins' headers, so it's quick).
pub fn messages(game: &Game, lo: &LoadOrder, lists: &Lists) -> Result<HashMap<String, Vec<Msg>>, String> {
    let mut g = handle(game, lists)?;
    let p = paths(lo);
    let refs: Vec<&Path> = p.iter().map(PathBuf::as_path).collect();
    g.load_plugin_headers(&refs).map_err(|e| format!("LOOT couldn't read the plugins ({e})"))?;
    plugin_messages(&g, lo)
}

/// Sorts with LOOT: reads every plugin in full (that takes a moment for the game's own masters), then asks LOOT for
/// the order. Nothing is written here.
pub fn sort(game: &Game, lo: &LoadOrder, lists: &Lists) -> Result<Sorted, String> {
    let at = std::time::Instant::now();
    let mut g = handle(game, lists)?;
    let p = paths(lo);
    let refs: Vec<&Path> = p.iter().map(PathBuf::as_path).collect();
    g.load_plugins(&refs).map_err(|e| format!("LOOT couldn't read the plugins ({e})"))?;
    let (read, unread): (Vec<String>, Vec<String>) =
        lo.plugins().iter().map(|p| p.name.clone()).partition(|n| g.plugin(n).is_some());
    for u in &unread {
        crate::log::line("loot", &format!("LOOT couldn't read {u}; it keeps its place after the others"));
    }
    let names: Vec<&str> = read.iter().map(String::as_str).collect();
    let order = g.sort_plugins(&names).map_err(|e| format!("LOOT couldn't sort ({e})"))?;
    let messages = plugin_messages(&g, lo)?;
    crate::log::line("loot", &format!("Sorted {} plugins in {} ms", order.len(), at.elapsed().as_millis()));
    Ok(Sorted { order, unread, messages, general: general(&g) })
}

/// Puts a load order in the order LOOT gave (plugins it didn't read keep their order after the others). The game's
/// own plugins stay where they always load.
pub fn apply(lo: &mut LoadOrder, sorted: &Sorted) {
    let fixed = lo.plugins().iter().filter(|p| p.fixed).count();
    let mut at = fixed;
    for name in sorted.order.iter().chain(sorted.unread.iter()) {
        let is_fixed = lo.plugins().iter().any(|p| p.fixed && p.name.eq_ignore_ascii_case(name));
        if is_fixed {
            continue;
        }
        if let Ok(i) = lo.move_to(name, at) {
            at = i + 1;
        }
    }
}

/// Messages as JSON for the interface: {"plugin name lowercased": [{"level","text"}]}.
pub fn messages_json(m: &HashMap<String, Vec<Msg>>) -> String {
    use turbine_engine::json::quote;
    let mut keys: Vec<&String> = m.keys().collect();
    keys.sort();
    let items: Vec<String> = keys
        .iter()
        .map(|k| {
            let list: Vec<String> =
                m[*k].iter().map(|x| format!("{{\"level\":{},\"text\":{}}}", quote(x.level), quote(&x.text))).collect();
            format!("{}:[{}]", quote(k), list.join(","))
        })
        .collect();
    format!("{{{}}}", items.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn makes_messages_plain() {
        assert_eq!(plain("Needs [SKSE](https://skse.silverlock.org/) **1.6**."), "Needs SKSE 1.6.");
        assert_eq!(plain("a [b] c"), "a [b] c");
        assert_eq!(plain("Use `xEdit`\n to  clean"), "Use xEdit to clean");
    }
}
