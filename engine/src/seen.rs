//! What the player has already seen and answered, kept between runs in one small file: check lines read once, the
//! warnings played through, the game updates warned about, the requirements being got, and the mods moved into or
//! out of Dependencies in My mods.

use crate::json::{self, Json, quote};
use crate::needs::ReqQueue;
use std::path::Path;

/// How many played-through warnings and game update warnings are kept (the oldest go first).
pub const KEEP: usize = 50;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Seen {
    /// The interface's earlier saved values were read in (once).
    pub adopted: bool,
    /// Games whose one-time Steam Launch Option line was seen.
    pub launch_done: Vec<String>,
    /// "<game id>|<title>": check lines read once.
    pub once_done: Vec<String>,
    /// "<game>|<mods on>|<warnings>": warnings played through with Play anyway.
    pub played_warn: Vec<String>,
    /// "<game>|<build>" (and "|waiting"): the game update warning was shown.
    pub update_warned: Vec<String>,
    pub req_queue: Option<ReqQueue>,
    /// Per game, mods the player moved into ("in", true) or out of ("out", false) Dependencies.
    pub dep_moves: Vec<(String, Vec<(String, bool)>)>,
}

fn texts(j: Option<&Json>) -> Vec<String> {
    j.map(Json::as_array).unwrap_or(&[]).iter().filter_map(|x| x.as_str().map(str::to_string)).collect()
}

fn texts_json(v: &[String]) -> String {
    format!("[{}]", v.iter().map(|x| quote(x)).collect::<Vec<_>>().join(","))
}

/// {"<game>": {"<mod>": "in" | "out"}}; other values are left out.
fn moves_from(j: Option<&Json>) -> Vec<(String, Vec<(String, bool)>)> {
    j.map(Json::entries)
        .unwrap_or(&[])
        .iter()
        .map(|(g, m)| {
            let moves = m
                .entries()
                .iter()
                .filter_map(|(id, v)| match v.as_str() {
                    Some("in") => Some((id.clone(), true)),
                    Some("out") => Some((id.clone(), false)),
                    _ => None,
                })
                .collect();
            (g.clone(), moves)
        })
        .collect()
}

fn keep_last(v: &mut Vec<String>) {
    if v.len() > KEEP {
        v.drain(..v.len() - KEEP);
    }
}

impl Seen {
    pub fn parse(text: &str) -> Seen {
        let Ok(j) = json::parse(text) else { return Seen::default() };
        Seen {
            adopted: j.get("adopted").is_some_and(Json::truthy),
            launch_done: texts(j.get("launchDone")),
            once_done: texts(j.get("onceDone")),
            played_warn: texts(j.get("playedWarn")),
            update_warned: texts(j.get("updateWarned")),
            req_queue: j.get("reqQueue").and_then(ReqQueue::from_json),
            dep_moves: moves_from(j.get("depMoves")),
        }
    }

    pub fn to_json(&self) -> String {
        format!(
            "{{\"adopted\":{},\"launchDone\":{},\"onceDone\":{},\"playedWarn\":{},\"updateWarned\":{},\"reqQueue\":{},\"depMoves\":{}}}\n",
            self.adopted,
            texts_json(&self.launch_done),
            texts_json(&self.once_done),
            texts_json(&self.played_warn),
            texts_json(&self.update_warned),
            self.req_queue.as_ref().map(ReqQueue::to_json).unwrap_or_else(|| "null".into()),
            self.moves_all_json()
        )
    }

    /// The saved state; empty when there is none yet or it can't be read.
    pub fn load(path: &Path) -> Seen {
        std::fs::read_to_string(path).map(|t| Seen::parse(&t)).unwrap_or_default()
    }

    /// Writes the state whole or not at all.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        crate::write_atomic(path, self.to_json().as_bytes())
    }

    /// Reads in the values the interface kept itself before (each one JSON text, as it saved them): {"launchDone",
    /// "onceDone", "playedWarn", "updateWarned", "reqQueue", "depMoves"}. Only once: returns false when that was
    /// done already. What is here already stays; the old values go first in each list.
    pub fn adopt(&mut self, old: &Json) -> bool {
        if self.adopted {
            return false;
        }
        self.adopted = true;
        let inner = |key: &str| old.str_at(key).and_then(|t| json::parse(t).ok());
        let merge = |mine: &mut Vec<String>, from: Vec<String>| {
            let mut all: Vec<String> = from;
            for x in mine.drain(..) {
                if !all.contains(&x) {
                    all.push(x);
                }
            }
            *mine = all;
        };
        merge(&mut self.launch_done, texts(inner("launchDone").as_ref()));
        merge(&mut self.once_done, texts(inner("onceDone").as_ref()));
        merge(&mut self.played_warn, texts(inner("playedWarn").as_ref()));
        merge(&mut self.update_warned, texts(inner("updateWarned").as_ref()));
        keep_last(&mut self.played_warn);
        keep_last(&mut self.update_warned);
        if self.req_queue.is_none() {
            self.req_queue = inner("reqQueue").as_ref().and_then(ReqQueue::from_json);
        }
        for (game, moves) in moves_from(inner("depMoves").as_ref()) {
            for (id, into) in moves {
                if !self.dep_moves.iter().any(|(g, m)| *g == game && m.iter().any(|(x, _)| *x == id)) {
                    self.set_dep_move(&game, &id, into);
                }
            }
        }
        true
    }

    pub fn launch_seen(&self, game: &str) -> bool {
        !game.is_empty() && self.launch_done.iter().any(|g| g == game)
    }

    pub fn mark_launch(&mut self, game: &str) {
        if !self.launch_done.iter().any(|g| g == game) {
            self.launch_done.push(game.to_string());
        }
    }

    /// The titles of this game's read-once check lines that were read.
    pub fn once_seen(&self, game: &str) -> Vec<String> {
        if game.is_empty() {
            return Vec::new();
        }
        let head = format!("{game}|");
        self.once_done.iter().filter_map(|k| k.strip_prefix(&head).map(str::to_string)).collect()
    }

    pub fn mark_once(&mut self, game: &str, titles: &[String]) {
        for t in titles {
            let k = format!("{game}|{t}");
            if !self.once_done.contains(&k) {
                self.once_done.push(k);
            }
        }
    }

    /// Play anyway with these warnings (`key` from precheck): it replaces the game's earlier one.
    pub fn mark_played(&mut self, key: &str) {
        let game = key.split('|').next().unwrap_or_default();
        let head = format!("{game}|");
        self.played_warn.retain(|k| !k.starts_with(&head));
        self.played_warn.push(key.to_string());
        keep_last(&mut self.played_warn);
    }

    pub fn update_warned(&self, key: &str) -> bool {
        self.update_warned.iter().any(|k| k == key)
    }

    /// A warning for this game and build was shown (waiting in Steam or installed).
    pub fn update_warned_for(&self, game: &str, build: &str) -> bool {
        let head = format!("{game}|{build}");
        self.update_warned.iter().any(|k| k.starts_with(&head))
    }

    pub fn mark_update_warned(&mut self, key: &str) {
        self.update_warned.push(key.to_string());
        keep_last(&mut self.update_warned);
    }

    /// This game's moves: (mod id, true for into Dependencies).
    pub fn dep_moves(&self, game: &str) -> &[(String, bool)] {
        self.dep_moves.iter().find(|(g, _)| g == game).map(|(_, m)| m.as_slice()).unwrap_or(&[])
    }

    pub fn set_dep_move(&mut self, game: &str, id: &str, into: bool) {
        let at = match self.dep_moves.iter().position(|(g, _)| g == game) {
            Some(i) => i,
            None => {
                self.dep_moves.push((game.to_string(), Vec::new()));
                self.dep_moves.len() - 1
            }
        };
        let moves = &mut self.dep_moves[at].1;
        match moves.iter_mut().find(|(x, _)| x == id) {
            Some(m) => m.1 = into,
            None => moves.push((id.to_string(), into)),
        }
    }

    /// One game's moves as the interface reads them: {"<mod id>": "in" | "out"}.
    pub fn moves_json(&self, game: &str) -> String {
        let items: Vec<String> = self.dep_moves(game).iter().map(|(id, into)| format!("{}:\"{}\"", quote(id), if *into { "in" } else { "out" })).collect();
        format!("{{{}}}", items.join(","))
    }

    fn moves_all_json(&self) -> String {
        let items: Vec<String> = self.dep_moves.iter().map(|(g, _)| format!("{}:{}", quote(g), self.moves_json(g))).collect();
        format!("{{{}}}", items.join(","))
    }
}
