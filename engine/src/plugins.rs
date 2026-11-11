//! Skyrim Special Edition plugins (.esm, .esp, .esl): reading their headers, the load order the game uses, and the
//! Load order tab (list, move, turn on and off, sort).
//!
//! Facts used, from the public file format and how the game behaves:
//! - A plugin starts with a TES4 record: "TES4", data size (u32), flags (u32), form ID, version control info, form
//!   version and an unknown u16, so the record header is 24 bytes. Its subrecords are a 4 letter type and a u16 size.
//!   Flag 0x1 marks a master (ESM), 0x200 a light plugin (ESL). Each MAST subrecord names one master (text ending in
//!   a zero byte). An XXXX subrecord gives the size of the next subrecord when it is larger than a u16.
//! - .esm and .esl files are masters whatever their flags; .esl files are always light. An ESL-flagged .esp is light
//!   but not a master, so it loads among the other plugins.
//! - The game always loads Skyrim.esm, Update.esm, Dawnguard.esm, HearthFires.esm and Dragonborn.esm first, then the
//!   Creation Club plugins listed in Skyrim.ccc (in the game folder), and masters before every other plugin.
//! - The order and on/off state live in plugins.txt (an asterisk before a name means on), in the Proton prefix at
//!   drive_c/users/steamuser/AppData/Local/Skyrim Special Edition/ (the GOG build uses "Skyrim Special Edition GOG").
//!   The game's own files leave out the always-loaded plugins. Turbine also writes loadorder.txt (every plugin, in
//!   order) for tools that read it.
//!
//! Sorting here only puts masters first and every plugin after its masters, keeping the order otherwise. Sorting by
//! LOOT's community masterlist is done by the app (app/src/loot.rs).

use crate::games::{Game, Store};
use crate::json::quote;
use crate::xml::cp1252;
use crate::write_atomic;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

/// The game's own masters, always loaded first in this order.
pub const BASE_PLUGINS: &[&str] = &["Skyrim.esm", "Update.esm", "Dawnguard.esm", "HearthFires.esm", "Dragonborn.esm"];

pub const FLAG_MASTER: u32 = 0x1;
pub const FLAG_LIGHT: u32 = 0x200;

/// What a plugin's header says.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Header {
    pub flags: u32,
    pub masters: Vec<String>,
    pub author: Option<String>,
    pub description: Option<String>,
}

fn zstring(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    let b = &b[..end];
    match std::str::from_utf8(b) {
        Ok(s) => s.to_string(),
        Err(_) => b.iter().map(|&c| cp1252(c)).collect(),
    }
}

/// Reads a plugin header from the start of the file's bytes.
pub fn parse_header(b: &[u8]) -> Result<Header, String> {
    if b.len() < 24 || &b[..4] != b"TES4" {
        return Err("this isn't a Skyrim plugin (no TES4 header)".into());
    }
    let u32le = |at: usize| u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]);
    let size = u32le(4) as usize;
    let flags = u32le(8);
    let end = 24usize.checked_add(size).filter(|&e| e <= b.len()).ok_or("the plugin's header is cut off")?;
    let mut h = Header { flags, ..Header::default() };
    let mut i = 24;
    let mut big: Option<usize> = None;
    while i + 6 <= end {
        let kind = &b[i..i + 4];
        let mut len = u16::from_le_bytes([b[i + 4], b[i + 5]]) as usize;
        if let Some(n) = big.take() {
            len = n;
        }
        let start = i + 6;
        let stop = start.checked_add(len).filter(|&s| s <= end).ok_or("the plugin's header is damaged")?;
        let data = &b[start..stop];
        match kind {
            b"XXXX" if len >= 4 => big = Some(u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize),
            b"MAST" => h.masters.push(zstring(data)),
            b"CNAM" => h.author = Some(zstring(data)).filter(|s| !s.is_empty()),
            b"SNAM" => h.description = Some(zstring(data)).filter(|s| !s.is_empty()),
            _ => {}
        }
        i = stop;
    }
    Ok(h)
}

/// Reads a plugin file's header (only the header is read, never the whole file).
pub fn read_header(path: &Path) -> Result<Header, String> {
    let mut f = File::open(path).map_err(|e| e.to_string())?;
    let mut head = [0u8; 24];
    f.read_exact(&mut head).map_err(|_| "the plugin is too short".to_string())?;
    if &head[..4] != b"TES4" {
        return Err("this isn't a Skyrim plugin (no TES4 header)".into());
    }
    let size = u32::from_le_bytes([head[4], head[5], head[6], head[7]]) as usize;
    if size > 16 << 20 {
        return Err("the plugin's header is impossibly large".into());
    }
    let mut buf = head.to_vec();
    buf.resize(24 + size, 0);
    f.read_exact(&mut buf[24..]).map_err(|_| "the plugin's header is cut off".to_string())?;
    parse_header(&buf)
}

pub fn is_plugin_name(name: &str) -> bool {
    let lc = name.to_ascii_lowercase();
    lc.ends_with(".esp") || lc.ends_with(".esm") || lc.ends_with(".esl")
}

#[derive(Debug, Clone, PartialEq)]
pub struct Plugin {
    /// File name as on disk.
    pub name: String,
    pub active: bool,
    /// Loads with the masters (ESM flag, or .esm/.esl file).
    pub master: bool,
    /// Light plugin (ESL flag or .esl file).
    pub light: bool,
    pub masters: Vec<String>,
    /// Always loaded by the game (base game and Creation Club): can't be moved or turned off.
    pub fixed: bool,
    /// The header couldn't be read (shown, but it can't be checked).
    pub unreadable: bool,
}

/// A plugin whose master isn't there or is turned off.
#[derive(Debug, Clone, PartialEq)]
pub struct MissingMaster {
    pub plugin: String,
    pub master: String,
    /// true: the file is there but turned off; false: the file isn't in Data at all.
    pub turned_off: bool,
}

/// Where the load order files live for a game.
#[derive(Debug, Clone, PartialEq)]
pub struct Files {
    pub data: PathBuf,
    /// The folder holding plugins.txt and loadorder.txt (None when the game has no prefix yet).
    pub settings: Option<PathBuf>,
    /// Skyrim.ccc in the game folder.
    pub ccc: PathBuf,
    /// The Proton prefix (pfx folder); load order files are only written when it exists.
    pub prefix: Option<PathBuf>,
}

impl Files {
    /// The files for a game: Data in the game folder, the settings folder in its Proton prefix.
    pub fn for_game(game: &Game) -> Files {
        let folder = if game.store == Store::Gog { "Skyrim Special Edition GOG" } else { "Skyrim Special Edition" };
        Files {
            data: crate::rules::find_ci(&game.install_dir, "Data").unwrap_or_else(|| game.install_dir.join("Data")),
            settings: game
                .prefix
                .as_ref()
                .map(|p| p.join("drive_c/users/steamuser/AppData/Local").join(folder)),
            ccc: game.install_dir.join("Skyrim.ccc"),
            prefix: game.prefix.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoadOrder {
    files: Files,
    plugins: Vec<Plugin>,
    /// Plugins listed in plugins.txt that aren't in Data now (their mod is turned off): written back after the
    /// others with their on/off state, so a plugin comes back turned on when its mod does. (name, active)
    absent: Vec<(String, bool)>,
}

/// Reads a text file of plugin names: UTF-8 when valid, Windows-1252 otherwise. Missing file: empty.
fn read_lines(path: &Path) -> io::Result<Vec<String>> {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let text = match String::from_utf8(bytes.clone()) {
        Ok(s) => s,
        Err(_) => bytes.iter().map(|&c| cp1252(c)).collect(),
    };
    Ok(text.trim_start_matches('\u{feff}').lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty() && !l.starts_with('#')).collect())
}

/// Text to Windows-1252 bytes (what plugins.txt uses); characters it can't hold become '?'.
fn to_cp1252(s: &str) -> Vec<u8> {
    s.chars()
        .map(|c| {
            let n = c as u32;
            if n < 0x80 || (0xA0..=0xFF).contains(&n) {
                n as u8
            } else {
                (0x80u8..0xA0).find(|&b| cp1252(b) == c).unwrap_or(b'?')
            }
        })
        .collect()
}

/// Keeps a copy of the player's own file the first time Turbine changes it.
fn backup_once(path: &Path) -> io::Result<()> {
    let backup = path.with_file_name(format!("{}.before-turbine", path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()));
    if path.is_file() && fs::symlink_metadata(&backup).is_err() {
        fs::copy(path, &backup)?;
    }
    Ok(())
}

impl LoadOrder {
    /// Reads the plugins in the game's Data folder and the order in plugins.txt.
    pub fn read(game: &Game) -> io::Result<LoadOrder> {
        LoadOrder::read_at(Files::for_game(game))
    }

    pub fn read_at(files: Files) -> io::Result<LoadOrder> {
        if !files.data.is_dir() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "The game's Data folder isn't there."));
        }
        // Plugins on disk, by lowercase name.
        let mut on_disk: HashMap<String, String> = HashMap::new();
        for e in fs::read_dir(&files.data)?.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if is_plugin_name(&name) && e.path().is_file() {
                on_disk.insert(name.to_lowercase(), name);
            }
        }
        let mut plugins: Vec<Plugin> = Vec::new();
        let mut taken: HashSet<String> = HashSet::new();
        let make = |name: &str, active: bool, fixed: bool| -> Plugin {
            let lc = name.to_ascii_lowercase();
            let (header, unreadable) = match read_header(&files.data.join(name)) {
                Ok(h) => (h, false),
                Err(_) => (Header::default(), true),
            };
            Plugin {
                name: name.to_string(),
                active: active || fixed,
                master: header.flags & FLAG_MASTER != 0 || lc.ends_with(".esm") || lc.ends_with(".esl"),
                light: header.flags & FLAG_LIGHT != 0 || lc.ends_with(".esl"),
                masters: header.masters,
                fixed,
                unreadable,
            }
        };
        // Always loaded: base game, then Creation Club in Skyrim.ccc's order.
        let mut fixed_names: Vec<String> = BASE_PLUGINS.iter().map(|s| s.to_string()).collect();
        fixed_names.extend(read_lines(&files.ccc)?);
        for n in fixed_names {
            let lc = n.to_lowercase();
            if let Some(real) = on_disk.get(&lc) {
                if taken.insert(lc) {
                    plugins.push(make(real, true, true));
                }
            }
        }
        // plugins.txt order.
        let mut absent = Vec::new();
        let listed = match &files.settings {
            Some(dir) => read_lines(&dir.join("plugins.txt"))?,
            None => Vec::new(),
        };
        for line in listed {
            let (active, name) = match line.strip_prefix('*') {
                Some(n) => (true, n.trim().to_string()),
                None => (false, line.clone()),
            };
            let lc = name.to_lowercase();
            if taken.contains(&lc) || !is_plugin_name(&name) {
                continue;
            }
            match on_disk.get(&lc) {
                Some(real) => {
                    taken.insert(lc);
                    plugins.push(make(real, active, false));
                }
                None => {
                    if !absent.iter().any(|(n, _): &(String, bool)| n.eq_ignore_ascii_case(&name)) {
                        absent.push((name, active));
                    }
                }
            }
        }
        // New plugins (just deployed): at the end, turned on, in name order.
        let mut new: Vec<&String> = on_disk.iter().filter(|(lc, _)| !taken.contains(*lc)).map(|(_, n)| n).collect();
        new.sort_by_key(|n| n.to_lowercase());
        for n in new {
            plugins.push(make(n, true, false));
        }
        let mut lo = LoadOrder { files, plugins, absent };
        lo.keep_masters_first();
        Ok(lo)
    }

    pub fn plugins(&self) -> &[Plugin] {
        &self.plugins
    }

    pub fn files(&self) -> &Files {
        &self.files
    }

    fn index(&self, name: &str) -> Result<usize, String> {
        self.plugins.iter().position(|p| p.name.eq_ignore_ascii_case(name)).ok_or_else(|| format!("{name} isn't in the Data folder."))
    }

    /// Masters before other plugins (the game loads them that way whatever the file says), order kept otherwise.
    fn keep_masters_first(&mut self) {
        let (fixed, rest): (Vec<Plugin>, Vec<Plugin>) = self.plugins.drain(..).partition(|p| p.fixed);
        let (masters, others): (Vec<Plugin>, Vec<Plugin>) = rest.into_iter().partition(|p| p.master);
        self.plugins = fixed.into_iter().chain(masters).chain(others).collect();
    }

    /// Turns a plugin on or off.
    pub fn set_active(&mut self, name: &str, on: bool) -> Result<(), String> {
        let i = self.index(name)?;
        if self.plugins[i].fixed && !on {
            return Err(format!("{} is part of the game and always loads.", self.plugins[i].name));
        }
        self.plugins[i].active = on;
        Ok(())
    }

    /// Moves a plugin to a position in the list (0 = first). It stays among its kind (masters with masters) and
    /// below the always-loaded plugins; returns where it ended up.
    pub fn move_to(&mut self, name: &str, index: usize) -> Result<usize, String> {
        let i = self.index(name)?;
        if self.plugins[i].fixed {
            return Err(format!("{} is part of the game and always loads first.", self.plugins[i].name));
        }
        let p = self.plugins.remove(i);
        let fixed = self.plugins.iter().filter(|q| q.fixed).count();
        let masters = self.plugins.iter().filter(|q| !q.fixed && q.master).count();
        let (lo, hi) = if p.master { (fixed, fixed + masters) } else { (fixed + masters, self.plugins.len()) };
        let at = index.clamp(lo, hi);
        self.plugins.insert(at, p);
        Ok(at)
    }

    /// Sorts: always-loaded first, then masters, then the rest, each plugin after its own masters; otherwise the
    /// current order is kept. Returns true when something moved.
    pub fn sort(&mut self) -> bool {
        let before: Vec<String> = self.plugins.iter().map(|p| p.name.clone()).collect();
        self.keep_masters_first();
        let fixed = self.plugins.iter().filter(|p| p.fixed).count();
        let masters = self.plugins.iter().filter(|p| !p.fixed && p.master).count();
        let all = self.plugins.clone();
        let mut out: Vec<Plugin> = all[..fixed].to_vec();
        for range in [fixed..fixed + masters, fixed + masters..all.len()] {
            let mut left: Vec<Plugin> = all[range].to_vec();
            let names: HashSet<String> = left.iter().map(|p| p.name.to_lowercase()).collect();
            while !left.is_empty() {
                let placed: HashSet<String> = out.iter().map(|p| p.name.to_lowercase()).collect();
                // First plugin whose masters from this part are all placed; a loop of masters takes the first.
                let pick = left
                    .iter()
                    .position(|p| p.masters.iter().all(|m| !names.contains(&m.to_lowercase()) || placed.contains(&m.to_lowercase())))
                    .unwrap_or(0);
                out.push(left.remove(pick));
            }
        }
        self.plugins = out;
        self.plugins.iter().map(|p| &p.name).ne(before.iter())
    }

    /// Active plugins whose masters are missing or turned off.
    pub fn missing_masters(&self) -> Vec<MissingMaster> {
        let state: HashMap<String, bool> = self.plugins.iter().map(|p| (p.name.to_lowercase(), p.active)).collect();
        let mut out = Vec::new();
        for p in self.plugins.iter().filter(|p| p.active) {
            for m in &p.masters {
                match state.get(&m.to_lowercase()) {
                    Some(true) => {}
                    Some(false) => out.push(MissingMaster { plugin: p.name.clone(), master: m.clone(), turned_off: true }),
                    None => out.push(MissingMaster { plugin: p.name.clone(), master: m.clone(), turned_off: false }),
                }
            }
        }
        out
    }

    /// Plugins and their on/off state, for FOMOD conditions.
    pub fn states(&self) -> Vec<(String, bool)> {
        self.plugins.iter().map(|p| (p.name.clone(), p.active)).collect()
    }

    /// plugins.txt as the game reads it (CRLF lines, Windows-1252): every plugin that isn't always loaded,
    /// '*' for on, followed by the plugins that are away right now.
    pub fn plugins_txt(&self) -> Vec<u8> {
        let mut text = String::new();
        for p in self.plugins.iter().filter(|p| !p.fixed) {
            text.push_str(&format!("{}{}\r\n", if p.active { "*" } else { "" }, p.name));
        }
        for (name, active) in &self.absent {
            text.push_str(&format!("{}{}\r\n", if *active { "*" } else { "" }, name));
        }
        to_cp1252(&text)
    }

    /// loadorder.txt: every plugin in Data in load order, always-loaded ones included (UTF-8).
    pub fn loadorder_txt(&self) -> Vec<u8> {
        let mut text = String::new();
        for p in &self.plugins {
            text.push_str(&p.name);
            text.push_str("\r\n");
        }
        text.into_bytes()
    }

    /// Writes plugins.txt and loadorder.txt into the prefix, each atomically, keeping a copy of the player's own
    /// files the first time. Does nothing (and says so) when the game has no prefix yet: start it once from Steam.
    pub fn save(&self) -> io::Result<()> {
        let Some(dir) = &self.files.settings else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "The game hasn't been started yet, so its load order can't be written. Start it once from Steam.",
            ));
        };
        // The prefix itself must exist (Turbine never makes one); the settings folder inside it may not yet.
        if self.files.prefix.as_ref().is_some_and(|p| !p.is_dir()) {
            return Err(io::Error::new(io::ErrorKind::NotFound, "The game's Proton folder isn't there yet. Start the game once from Steam."));
        }
        fs::create_dir_all(dir)?;
        for (file, bytes) in [("plugins.txt", self.plugins_txt()), ("loadorder.txt", self.loadorder_txt())] {
            let path = dir.join(file);
            backup_once(&path)?;
            write_atomic(&path, &bytes)?;
        }
        Ok(())
    }

    /// The Load order tab as JSON: [{"name","active","master","light","fixed","position","masters":[..],"missing":[..]}].
    pub fn to_json(&self) -> String {
        let missing = self.missing_masters();
        let items: Vec<String> = self
            .plugins
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let ms: Vec<String> = p.masters.iter().map(|m| quote(m)).collect();
                let miss: Vec<String> = missing.iter().filter(|m| m.plugin == p.name).map(|m| quote(&m.master)).collect();
                format!(
                    "{{\"name\":{},\"active\":{},\"master\":{},\"light\":{},\"fixed\":{},\"unreadable\":{},\"position\":{},\"masters\":[{}],\"missing\":[{}]}}",
                    quote(&p.name),
                    p.active,
                    p.master,
                    p.light,
                    p.fixed,
                    p.unreadable,
                    i,
                    ms.join(","),
                    miss.join(",")
                )
            })
            .collect();
        format!("[{}]", items.join(","))
    }
}

/// Reads the load order and writes it back when the files on disk differ (new plugins after an apply).
/// Returns the load order.
pub fn sync(game: &Game) -> io::Result<LoadOrder> {
    let lo = LoadOrder::read(game)?;
    if let Some(dir) = &lo.files.settings {
        let now = fs::read(dir.join("plugins.txt")).unwrap_or_default();
        if now != lo.plugins_txt() && lo.files.prefix.as_ref().is_some_and(|p| p.is_dir()) {
            lo.save()?;
        }
    }
    Ok(lo)
}
