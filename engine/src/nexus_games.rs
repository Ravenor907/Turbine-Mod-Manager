//! Which Nexus Mods game a Steam or Heroic game is, for games without a Nexus domain in their data file.
//!
//! No public Nexus API links a store ID to a Nexus game, so the answer is found in this order: the game's data file,
//! the player's earlier pick (<TURBINE_DATA>/nexus/picks.json), then an exact name match against Nexus's public game
//! list (the file Nexus's own bot reads, cached in <TURBINE_DATA>/nexus/games.json and fetched again at most once a
//! week). Anything less than one exact match asks the player once ("Which game on Nexus is this?"); the answer is
//! kept per store and ID. GraphQL `games` with a wildcard name filter is the search behind that question.

use crate::games::{Game, Store};
use crate::json::{self, quote, Json};
use crate::net::{self, Client, NetError, Request};
use std::path::{Path, PathBuf};

/// Nexus's public game list (no key; only the application headers).
pub const LIST_URL: &str = "https://data.nexusmods.com/file/nexus-data/games.json";
/// Largest list accepted.
pub const LIST_MAX: u64 = 16 << 20;
/// The cached list is fetched again after a week.
pub const LIST_MAX_AGE_SECS: u64 = 7 * 24 * 60 * 60;
/// The build's checks pass a recorded list through this variable (no network there).
pub const FIXTURE_ENV: &str = "TURBINE_NEXUS_GAMES";

#[derive(Debug, Clone, PartialEq)]
pub struct NexusGame {
    pub id: u64,
    pub name: String,
    pub domain: String,
    pub mods: u64,
}

fn num(v: Option<&Json>) -> u64 {
    match v {
        Some(Json::Num(n)) if *n >= 0.0 => *n as u64,
        Some(Json::Str(s)) => s.parse().unwrap_or(0),
        _ => 0,
    }
}

fn good_domain(d: &str) -> bool {
    !d.is_empty() && d.len() <= 64 && d.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

/// Reads the public game list: an array of {id, name, domain_name, mods, ...}. Entries without a usable domain are
/// left out.
pub fn parse_list(text: &str) -> Result<Vec<NexusGame>, String> {
    let v = json::parse(text).map_err(|e| format!("Nexus's game list isn't valid JSON ({})", e.message))?;
    let Json::Arr(items) = v else { return Err("Nexus's game list isn't a list".into()) };
    Ok(items
        .iter()
        .filter_map(|g| {
            let domain = g.str_at("domain_name")?.to_string();
            let name = g.str_at("name")?.trim().to_string();
            (good_domain(&domain) && !name.is_empty()).then(|| NexusGame { id: num(g.get("id")), name, domain, mods: num(g.get("mods")) })
        })
        .collect())
}

/// Names compared the same way on both sides: lowercase, trademark and copyright signs off, "&" as "and",
/// punctuation as spaces, single spaces.
pub fn normalize(name: &str) -> String {
    let mut s = String::new();
    for c in name.chars() {
        match c {
            '\u{2122}' | '\u{00AE}' | '\u{00A9}' => {}
            '&' => s.push_str(" and "),
            c if c.is_alphanumeric() => s.extend(c.to_lowercase()),
            '\'' | '\u{2019}' => {}
            _ => s.push(' '),
        }
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[derive(Debug, Clone, PartialEq)]
pub enum Match {
    /// Exactly one game with the same name.
    Sure(NexusGame),
    /// Up to four games to pick from (most shared words first, then most mods); may be empty.
    Ask(Vec<NexusGame>),
}

/// Matches a store's game name against the list. Only one exact (normalized) name is sure.
pub fn match_name(name: &str, list: &[NexusGame]) -> Match {
    let want = normalize(name);
    let exact: Vec<&NexusGame> = list.iter().filter(|g| normalize(&g.name) == want).collect();
    if exact.len() == 1 {
        return Match::Sure(exact[0].clone());
    }
    let words: Vec<&str> = want.split(' ').filter(|w| !w.is_empty()).collect();
    let mut scored: Vec<(usize, &NexusGame)> = list
        .iter()
        .map(|g| {
            let n = normalize(&g.name);
            let theirs: Vec<&str> = n.split(' ').collect();
            (words.iter().filter(|w| theirs.contains(w)).count(), g)
        })
        .filter(|(n, _)| *n > 0)
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.mods.cmp(&a.1.mods)).then(a.1.name.cmp(&b.1.name)));
    Match::Ask(scored.into_iter().take(4).map(|(_, g)| g.clone()).collect())
}

/// The key a game's answer is kept under: "steam:990080".
pub fn key(store: Store, store_id: &str) -> String {
    format!("{}:{store_id}", store.key())
}

/// The player's picks: key to a domain, or None for "It isn't on Nexus".
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Picks {
    pub items: Vec<(String, Option<String>)>,
}

impl Picks {
    pub fn path(data_dir: &Path) -> PathBuf {
        data_dir.join("nexus").join("picks.json")
    }

    pub fn load(data_dir: &Path) -> Picks {
        let Ok(text) = std::fs::read_to_string(Picks::path(data_dir)) else { return Picks::default() };
        let Ok(v) = json::parse(&text) else { return Picks::default() };
        let items = v
            .entries()
            .iter()
            .filter_map(|(k, v)| match v {
                Json::Null => Some((k.clone(), None)),
                Json::Str(d) if good_domain(d) => Some((k.clone(), Some(d.clone()))),
                _ => None,
            })
            .collect();
        Picks { items }
    }

    /// Some(None): the player said the game isn't on Nexus.
    pub fn get(&self, key: &str) -> Option<Option<String>> {
        self.items.iter().find(|(k, _)| k == key).map(|(_, d)| d.clone())
    }

    pub fn set(&mut self, key: &str, domain: Option<&str>) {
        let d = domain.filter(|d| good_domain(d)).map(String::from);
        match self.items.iter_mut().find(|(k, _)| k == key) {
            Some(e) => e.1 = d,
            None => self.items.push((key.to_string(), d)),
        }
    }

    pub fn save(&self, data_dir: &Path) -> std::io::Result<()> {
        let path = Picks::path(data_dir);
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p)?;
        }
        let body: Vec<String> = self.items.iter().map(|(k, d)| format!("{}:{}", quote(k), d.as_deref().map(quote).unwrap_or_else(|| "null".into()))).collect();
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, format!("{{{}}}\n", body.join(",")))?;
        std::fs::rename(&tmp, &path)
    }
}

/// GraphQL `games` with a wildcard name filter (the "Search Nexus" row).
pub fn games_search_body(text: &str) -> String {
    let q = "query TurbineGames($f: GamesSearchFilter, $count: Int) { games(filter: $f, count: $count) { nodes { id name domainName modCount } } }";
    format!(
        "{{\"query\":{},\"variables\":{{\"f\":{{\"name\":[{{\"value\":{},\"op\":\"WILDCARD\"}}]}},\"count\":10}}}}",
        quote(q),
        quote(text.trim())
    )
}

pub fn parse_games_search(text: &str) -> Result<Vec<NexusGame>, String> {
    let v = json::parse(text).map_err(|e| format!("Nexus's answer isn't valid JSON ({})", e.message))?;
    if let Some(e) = v.get("errors").map(Json::as_array).and_then(|e| e.first()) {
        return Err(format!("Nexus couldn't search its games ({})", e.str_at("message").unwrap_or("unknown error")));
    }
    let nodes = v.get("data").and_then(|d| d.get("games")).and_then(|g| g.get("nodes")).map(Json::as_array).unwrap_or(&[]);
    Ok(nodes
        .iter()
        .filter_map(|g| {
            let domain = g.str_at("domainName")?.to_string();
            let name = g.str_at("name")?.to_string();
            good_domain(&domain).then(|| NexusGame { id: num(g.get("id")), name, domain, mods: num(g.get("modCount")) })
        })
        .collect())
}

/// What Turbine knows about a game's Nexus domain.
#[derive(Debug, Clone, PartialEq)]
pub enum Answer {
    /// `how`: "data file", "your pick" or "name match".
    Sure { domain: String, name: String, how: &'static str },
    Ask(Vec<NexusGame>),
    NotOnNexus,
    /// No list and nothing else to go on.
    Unknown,
}

impl Answer {
    pub fn state(&self) -> &'static str {
        match self {
            Answer::Sure { .. } => "sure",
            Answer::Ask(_) => "ask",
            Answer::NotOnNexus => "none",
            Answer::Unknown => "unknown",
        }
    }
}

/// The cached game list (or the build's recorded one), if there is one.
pub fn cached_list(data_dir: &Path) -> Option<Vec<NexusGame>> {
    let path = std::env::var_os(FIXTURE_ENV).map(PathBuf::from).unwrap_or_else(|| data_dir.join("nexus").join("games.json"));
    let text = std::fs::read_to_string(path).ok()?;
    parse_list(&text).ok()
}

/// Is the cached list missing or older than a week (and not the build's recorded one)?
pub fn list_is_old(data_dir: &Path, now: u64) -> bool {
    if std::env::var_os(FIXTURE_ENV).is_some() {
        return false;
    }
    let path = data_dir.join("nexus").join("games.json");
    match std::fs::metadata(&path).and_then(|m| m.modified()) {
        Ok(t) => t.duration_since(std::time::UNIX_EPOCH).map(|d| now.saturating_sub(d.as_secs()) > LIST_MAX_AGE_SECS).unwrap_or(true),
        Err(_) => true,
    }
}

/// Fetches the game list again; the old copy is kept when anything goes wrong. Returns how many games it has.
pub fn refresh_list(client: &Client, data_dir: &Path, app_version: &str) -> Result<usize, NetError> {
    let mut req = Request::get(LIST_URL).max_bytes(LIST_MAX).max_time(120);
    for (k, v) in crate::nexus::headers_public(app_version) {
        req = req.header(&k, &v);
    }
    let text = client.send(&req, None)?.take_text()?;
    let list = parse_list(&text).map_err(NetError::Failed)?;
    if list.len() < 10 {
        return Err(NetError::Failed("Nexus's game list came back nearly empty, so the old copy was kept.".into()));
    }
    let dir = data_dir.join("nexus");
    std::fs::create_dir_all(&dir).map_err(|e| NetError::Failed(e.to_string()))?;
    let tmp = net::temp_path(&dir, "games");
    std::fs::write(&tmp, text).map_err(|e| NetError::Failed(e.to_string()))?;
    std::fs::rename(&tmp, dir.join("games.json")).map_err(|e| NetError::Failed(e.to_string()))?;
    Ok(list.len())
}

/// A game's Nexus domain: its data file, then the player's pick, then an exact name match in `list`.
pub fn domain_for_game(game: &Game, picks: &Picks, list: Option<&[NexusGame]>) -> Answer {
    if let Some(d) = crate::rules::find_def(game.store, &game.store_id).and_then(|d| d.data.nexus.as_deref()) {
        let name = crate::rules::find_def(game.store, &game.store_id).map(|d| d.name).unwrap_or(&game.name);
        return Answer::Sure { domain: d.to_string(), name: name.to_string(), how: "data file" };
    }
    match picks.get(&key(game.store, &game.store_id)) {
        Some(Some(d)) => {
            let name = list.and_then(|l| l.iter().find(|g| g.domain == d)).map(|g| g.name.clone()).unwrap_or_else(|| game.name.clone());
            return Answer::Sure { domain: d, name, how: "your pick" };
        }
        Some(None) => return Answer::NotOnNexus,
        None => {}
    }
    match list {
        None => Answer::Unknown,
        Some(l) => match match_name(&game.name, l) {
            Match::Sure(g) => Answer::Sure { domain: g.domain, name: g.name, how: "name match" },
            Match::Ask(c) => Answer::Ask(c),
        },
    }
}

/// The installed game a Nexus domain belongs to (for links and downloads from a Basic game's Nexus pages).
pub fn game_for_domain<'a>(domain: &str, games: &'a [Game], picks: &Picks, list: Option<&[NexusGame]>) -> Option<&'a Game> {
    games.iter().find(|g| matches!(domain_for_game(g, picks, list), Answer::Sure { domain: ref d, .. } if d == domain))
}

/// The answer as JSON for the interface: {"state","domain","name","how","candidates":[{"name","domain","mods"}]}.
pub fn answer_json(a: &Answer) -> String {
    let cands = |c: &[NexusGame]| {
        c.iter().map(|g| format!("{{\"name\":{},\"domain\":{},\"mods\":{}}}", quote(&g.name), quote(&g.domain), g.mods)).collect::<Vec<_>>().join(",")
    };
    match a {
        Answer::Sure { domain, name, how } => {
            format!("{{\"state\":\"sure\",\"domain\":{},\"name\":{},\"how\":{},\"candidates\":[]}}", quote(domain), quote(name), quote(how))
        }
        Answer::Ask(c) => format!("{{\"state\":\"ask\",\"domain\":\"\",\"name\":\"\",\"how\":\"\",\"candidates\":[{}]}}", cands(c)),
        Answer::NotOnNexus => "{\"state\":\"none\",\"domain\":\"\",\"name\":\"\",\"how\":\"your pick\",\"candidates\":[]}".into(),
        Answer::Unknown => "{\"state\":\"unknown\",\"domain\":\"\",\"name\":\"\",\"how\":\"\",\"candidates\":[]}".into(),
    }
}

/// Candidates as JSON (the search's results).
pub fn games_json(list: &[NexusGame]) -> String {
    format!(
        "[{}]",
        list.iter().map(|g| format!("{{\"name\":{},\"domain\":{},\"mods\":{}}}", quote(&g.name), quote(&g.domain), g.mods)).collect::<Vec<_>>().join(",")
    )
}
