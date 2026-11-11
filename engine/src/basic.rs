//! Basic support: mods for games Turbine has no rules of its own for.
//!
//! `place` looks at a mod archive's files and the game's folder (`GameIndex`) and decides where the files go:
//! Sure (placed at once, the rule named in the activity log), Guess (the player checks it on "Where do these files
//! go?") or Ask (no idea; the page opens on the folder with the game's program). The rules run in a fixed order and
//! the first Sure wins (see `place`). Every answer stays inside the game folder.
//!
//! The refused table (`refused`) lists files a Basic mod never places or replaces, whatever the player says:
//! anti-cheat, Steam's and Epic's own libraries, and the game's programs.

use crate::games::Game;
use crate::rules::{is_doc, is_image, GameRules};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Where a mod's files go: `from` is the archive folder whose contents go in ("" for the archive's top), `to` the
/// game-relative folder.
#[derive(Debug, Clone, PartialEq)]
pub enum Placement {
    Sure { from: String, to: String, rule: &'static str },
    Guess { from: String, to: String, rule: &'static str, others: Vec<String> },
    Ask { suggest_to: String },
}

/// Deepest level of the game folder the index reads.
pub const INDEX_DEPTH: usize = 4;
/// Most entries (folders and files) the index holds.
pub const INDEX_CAP: usize = 20_000;
/// Longest the index may take to read.
pub const INDEX_TIME: Duration = Duration::from_millis(300);

/// A game folder's folders and files, down to `INDEX_DEPTH`, names only (links are kept apart, never followed).
#[derive(Debug, Default, Clone)]
pub struct GameIndex {
    /// Game-relative folders, as on disk.
    pub dirs: Vec<String>,
    /// Game-relative files, as on disk.
    pub files: Vec<String>,
    /// Sizes of the .exe files (for the game's main program).
    exe_sizes: HashMap<String, u64>,
    dirs_lc: HashMap<String, String>,
    files_lc: HashMap<String, String>,
    /// Links (to folders or files), lowercase: never counted as the game's folders or files, never gone through.
    links_lc: HashSet<String>,
    /// The cap or the time limit stopped the walk early.
    pub capped: bool,
}

impl GameIndex {
    /// Reads the folder (breadth first, so a cap still has the top levels).
    pub fn build(root: &Path) -> GameIndex {
        let start = Instant::now();
        let mut ix = GameIndex::default();
        let mut queue: std::collections::VecDeque<(PathBuf, String, usize)> = std::collections::VecDeque::new();
        queue.push_back((root.to_path_buf(), String::new(), 0));
        'walk: while let Some((dir, rel, depth)) = queue.pop_front() {
            let Ok(rd) = std::fs::read_dir(&dir) else { continue };
            let mut entries: Vec<_> = rd.flatten().collect();
            entries.sort_by_key(|e| e.file_name());
            for e in entries {
                if ix.dirs.len() + ix.files.len() >= INDEX_CAP || start.elapsed() > INDEX_TIME {
                    ix.capped = true;
                    break 'walk;
                }
                let name = e.file_name().to_string_lossy().into_owned();
                let r = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
                let Ok(ft) = e.file_type() else { continue };
                if ft.is_symlink() {
                    ix.links_lc.insert(r.to_lowercase());
                    continue;
                }
                if ft.is_dir() {
                    ix.dirs_lc.insert(r.to_lowercase(), r.clone());
                    ix.dirs.push(r.clone());
                    if depth + 1 < INDEX_DEPTH {
                        queue.push_back((e.path(), r, depth + 1));
                    }
                } else {
                    if name.to_lowercase().ends_with(".exe") && ft.is_file() {
                        ix.exe_sizes.insert(r.clone(), e.metadata().map(|m| m.len()).unwrap_or(0));
                    }
                    ix.files_lc.insert(r.to_lowercase(), r.clone());
                    ix.files.push(r);
                }
            }
        }
        ix
    }

    /// The index of a game folder, read again only when the folder's top changed (kept per folder).
    pub fn cached(root: &Path) -> Arc<GameIndex> {
        type Cache = Mutex<HashMap<PathBuf, (i128, Arc<GameIndex>)>>;
        static CACHE: std::sync::OnceLock<Cache> = std::sync::OnceLock::new();
        let stamp = std::fs::metadata(root)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos() as i128)
            .unwrap_or(-1);
        let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        let mut c = cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((s, ix)) = c.get(root) && *s == stamp {
            return ix.clone();
        }
        let ix = Arc::new(GameIndex::build(root));
        c.insert(root.to_path_buf(), (stamp, ix.clone()));
        ix
    }

    /// The folder as it is on disk, matched ignoring case ("" is the game folder itself).
    pub fn dir(&self, rel: &str) -> Option<String> {
        if rel.is_empty() {
            return Some(String::new());
        }
        self.dirs_lc.get(&rel.to_lowercase()).cloned()
    }

    pub fn has_dir(&self, rel: &str) -> bool {
        rel.is_empty() || self.dirs_lc.contains_key(&rel.to_lowercase())
    }

    /// Is this game path (or a folder on the way to it) a link?
    pub fn through_link(&self, rel: &str) -> bool {
        let lc = rel.to_lowercase();
        let mut acc = String::new();
        for part in lc.split('/') {
            if !acc.is_empty() {
                acc.push('/');
            }
            acc.push_str(part);
            if self.links_lc.contains(&acc) {
                return true;
            }
        }
        false
    }

    /// Folder or file directly in the game folder?
    fn at_top(&self, name: &str) -> bool {
        let n = name.to_lowercase();
        self.dirs_lc.contains_key(&n) || self.files_lc.contains_key(&n)
    }

    /// The folders directly inside `rel` (as on disk), A to Z ignoring case.
    pub fn children(&self, rel: &str) -> Vec<String> {
        let base = rel.trim_matches('/').to_lowercase();
        let mut out: Vec<String> = self
            .dirs
            .iter()
            .filter(|d| match d.rsplit_once('/') {
                Some((parent, _)) => parent.to_lowercase() == base,
                None => base.is_empty(),
            })
            .map(|d| d.rsplit('/').next().unwrap_or(d).to_string())
            .collect();
        out.sort_by_key(|a| a.to_lowercase());
        out
    }

    /// The folder holding the game's main program: the largest .exe outside redistributable, support and
    /// anti-cheat folders ("" when there is none).
    pub fn main_exe_folder(&self) -> String {
        let skip = |p: &str| {
            p.to_lowercase().split('/').any(|part| {
                matches!(part, "redist" | "redists" | "support" | "_commonredist" | "commonredist" | "battleye") || part.starts_with("easyanticheat")
            })
        };
        let mut best: Option<(&String, u64)> = None;
        for (p, size) in &self.exe_sizes {
            if skip(p) {
                continue;
            }
            if best.is_none_or(|(bp, bs)| *size > bs || (*size == bs && p < bp)) {
                best = Some((p, *size));
            }
        }
        best.and_then(|(p, _)| p.rsplit_once('/').map(|(d, _)| d.to_string())).unwrap_or_default()
    }

    /// Anti-cheat in the game's top two levels, by name ("Easy Anti-Cheat", "BattlEye").
    pub fn anti_cheat(&self) -> Option<&'static str> {
        let near = |p: &String| p.matches('/').count() < 2;
        let names = self.dirs.iter().filter(|p| near(p)).chain(self.files.iter().filter(|p| near(p)));
        let mut found = None;
        for p in names {
            let last = p.rsplit('/').next().unwrap_or(p).to_lowercase();
            if last == "easyanticheat" || last == "easyanticheat_eos" || last == "start_protected_game.exe" {
                found = Some("Easy Anti-Cheat");
            } else if last == "battleye" || last.starts_with("beservice") {
                return Some("BattlEye");
            }
        }
        found
    }
}

/// Why a Basic mod may never place or replace this game-relative file, if it may not. `exists_not_ours`: the file
/// is in the game already and Turbine's record doesn't list it.
pub fn refused(rel: &str, exists_not_ours: bool) -> Option<&'static str> {
    let lc = rel.replace('\\', "/").to_lowercase();
    let parts: Vec<&str> = lc.split('/').collect();
    let name = parts.last().copied().unwrap_or("");
    if parts.iter().any(|p| p.starts_with("easyanticheat") || *p == "battleye") || name == "start_protected_game.exe" {
        return Some("part of the game's anti-cheat");
    }
    if name == "steam_api.dll" || name == "steam_api64.dll" || (name.starts_with("steamclient") && name.ends_with(".dll")) {
        return Some("part of Steam");
    }
    if name.starts_with("eossdk-") && name.ends_with(".dll") {
        return Some("part of Epic Online Services");
    }
    if name.ends_with(".exe") && exists_not_ours {
        return Some("the game's own program");
    }
    if parts.iter().any(|p| p.is_empty() || *p == "." || *p == "..") || lc.starts_with('/') {
        return Some("outside the game folder");
    }
    None
}

/// The refused table in words, for Game details and the overwrite sheet.
pub const REFUSED_WORDS: &str = "Turbine never places or replaces anti-cheat files (EasyAntiCheat, BattlEye, \
start_protected_game.exe), Steam's and Epic's own libraries (steam_api.dll, steam_api64.dll, steamclient, EOSSDK), \
the game's own programs, or anything outside the game folder.";

#[derive(Debug, Clone)]
struct E {
    src: String,
    parts: Vec<String>,
}

impl E {
    fn top_lc(&self) -> String {
        self.parts[0].to_lowercase()
    }
    fn is_top_file(&self) -> bool {
        self.parts.len() == 1
    }
    fn name_lc(&self) -> String {
        self.parts.last().map(|s| s.to_lowercase()).unwrap_or_default()
    }
    fn is_extra(&self) -> bool {
        let n = self.name_lc();
        is_doc(&n) || is_image(&n)
    }
}

fn normalize(files: &[String]) -> Result<Vec<E>, String> {
    Ok(crate::rules::archive_paths(files, true)?.into_iter().map(|parts| E { src: parts.join("/"), parts }).collect())
}

/// The entries that count at this level: readmes and images at the top are left out when there is anything else.
fn content(es: &[E]) -> Vec<&E> {
    let real: Vec<&E> = es.iter().filter(|e| !(e.is_top_file() && e.is_extra())).collect();
    if real.is_empty() {
        es.iter().collect()
    } else {
        real
    }
}

fn top_folders(es: &[&E]) -> Vec<String> {
    let mut v: Vec<String> = es.iter().filter(|e| !e.is_top_file()).map(|e| e.parts[0].clone()).collect();
    v.sort_by_key(|a| a.to_lowercase());
    v.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    v
}

fn strip(es: &[E], folder_lc: &str) -> Vec<E> {
    es.iter()
        .filter(|e| e.parts.len() > 1 && e.top_lc() == folder_lc)
        .map(|e| E { src: e.src.clone(), parts: e.parts[1..].to_vec() })
        .collect()
}

fn join(a: &str, b: &str) -> String {
    match (a.is_empty(), b.is_empty()) {
        (true, _) => b.to_string(),
        (_, true) => a.to_string(),
        _ => format!("{a}/{b}"),
    }
}

/// Lowercase letters and digits only ("Baldur's Gate 3" and "BaldursGate3" are the same).
pub fn squash(name: &str) -> String {
    name.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

/// Folder names that mean something when they are at an archive's top, so they are never taken off as wrappers.
const MEANINGFUL: &[&str] = &["data", "bepinex", "mods", "plugins", "content", "bin", "~mods", "logicmods"];

const PAK_EXTS: &[&str] = &["pak", "utoc", "ucas", "sig"];

fn ext(name_lc: &str) -> &str {
    name_lc.rsplit_once('.').map(|(_, e)| e).unwrap_or("")
}

struct Ctx<'a> {
    index: &'a GameIndex,
    rules: &'a GameRules,
    game_names: Vec<String>,
    mod_name: String,
}

/// Where a Basic game's mod goes. `files` are the archive's paths ('/' separated).
///
/// Rules, in order (the first Sure wins): 1 the data file's hints; 2 the game's Mods folder (each mod a folder in
/// it); 3 a top folder named after the game is taken off; 4 wrapper folders are taken off; 5 BepInEx; 6 the archive
/// mirrors the game's top; 7 the contents of the game's Data folder; 8 folders the game has deeper down; 9 Unreal
/// .pak files; 10 loose files the game already has (a Guess); 11 Ask, suggesting the folder of the game's program.
pub fn place(files: &[String], mod_name: &str, game: &Game, rules: &GameRules, index: &GameIndex) -> Result<Placement, String> {
    let es = normalize(files)?;
    if es.is_empty() {
        return Err("The archive is empty.".into());
    }
    let folder = game.install_dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let ctx = Ctx { index, rules, game_names: vec![squash(&game.name), squash(&folder)], mod_name: crate::store::folder_safe(mod_name) };
    let p = decide(&ctx, es, String::new(), 0);
    // Safety: every answer stays inside the game folder.
    let ok = |to: &str| to.is_empty() || crate::gamedata::safe_path(to);
    Ok(match p {
        Placement::Sure { ref to, .. } | Placement::Guess { ref to, .. } if !ok(to) => Placement::Ask { suggest_to: index.main_exe_folder() },
        other => other,
    })
}

fn unwrap_all(ctx: &Ctx, mut es: Vec<E>, mut from: String) -> (Vec<E>, String) {
    for _ in 0..8 {
        let view = content(&es);
        if view.iter().any(|e| e.is_top_file()) {
            break;
        }
        let tops = top_folders(&view);
        if tops.len() != 1 {
            break;
        }
        let t = &tops[0];
        let tl = t.to_lowercase();
        if MEANINGFUL.contains(&tl.as_str()) || ctx.index.at_top(t) || ctx.index.dirs.iter().any(|d| d.rsplit('/').next().is_some_and(|n| n.eq_ignore_ascii_case(t))) {
            break;
        }
        es = strip(&es, &tl);
        from = join(&from, t);
    }
    (es, from)
}

fn decide(ctx: &Ctx, es: Vec<E>, from: String, depth: usize) -> Placement {
    let ix = ctx.index;
    // 1. The data file's hints, on the archive with its wrapper folders taken off.
    let (unwrapped, ufrom) = unwrap_all(ctx, es.clone(), from.clone());
    if let Some(p) = hints(ctx, &unwrapped, &ufrom) {
        return p;
    }
    // 2. The game keeps each mod in a folder of its Mods folder: the archive's one folder goes in there as it is.
    if let Some(mods) = ix.dir("Mods").filter(|m| !m.is_empty()) {
        let inside = ix.children(&mods);
        let view = content(&es);
        let tops = top_folders(&view);
        if !inside.is_empty() && !view.iter().any(|e| e.is_top_file()) && tops.len() == 1 && !ix.at_top(&tops[0]) {
            let t = &tops[0];
            if !ctx.game_names.contains(&squash(t)) && !MEANINGFUL.contains(&t.to_lowercase().as_str()) {
                return Placement::Guess { from: join(&from, t), to: format!("{mods}/{t}"), rule: "mods-folder", others: Vec::new() };
            }
        }
    }
    // 3. A top folder named after the game: taken off, then the rules again.
    {
        let view = content(&es);
        let tops = top_folders(&view);
        if depth < 3 && tops.len() == 1 && !view.iter().any(|e| e.is_top_file()) && ctx.game_names.iter().any(|n| !n.is_empty() && *n == squash(&tops[0])) {
            let t = tops[0].clone();
            return decide(ctx, strip(&es, &t.to_lowercase()), join(&from, &t), depth + 1);
        }
    }
    // 4. Wrapper folders off.
    let (es, from) = (unwrapped, ufrom);
    let view = content(&es);
    let top_files: Vec<&&E> = view.iter().filter(|e| e.is_top_file()).collect();
    let tops = top_folders(&view);
    // 5. BepInEx.
    let has_top_file = |n: &str| top_files.iter().any(|e| e.name_lc() == n);
    if tops.iter().any(|t| t.eq_ignore_ascii_case("bepinex")) || (has_top_file("winhttp.dll") && has_top_file("doorstop_config.ini")) {
        return Placement::Sure { from, to: String::new(), rule: "bepinex" };
    }
    if view.iter().all(|e| e.name_lc().ends_with(".dll")) && ix.has_dir("BepInEx/core") {
        let base = ix.dir("BepInEx/plugins").unwrap_or_else(|| format!("{}/plugins", ix.dir("BepInEx").unwrap_or_else(|| "BepInEx".into())));
        return Placement::Sure { from, to: format!("{base}/{}", ctx.mod_name), rule: "bepinex-plugin" };
    }
    // 6. Every top entry is also at the game's top.
    let mut top_names: Vec<String> = tops.clone();
    top_names.extend(top_files.iter().map(|e| e.parts[0].clone()));
    // A top entry that is a link in the game folder: files would land outside the game, so the player decides.
    if top_names.iter().any(|n| ix.through_link(n)) {
        return Placement::Ask { suggest_to: ix.main_exe_folder() };
    }
    if !top_names.is_empty() && top_names.iter().all(|n| ix.at_top(n)) {
        let rule = if tops.iter().any(|t| t.eq_ignore_ascii_case("data")) { "data-folder" } else { "mirrors-game" };
        return Placement::Sure { from, to: String::new(), rule };
    }
    // 7. The contents of the game's Data folder.
    if let Some(data) = ix.dir("Data").filter(|d| !d.is_empty()) && top_files.is_empty() && !tops.is_empty() && tops.iter().all(|t| ix.has_dir(&format!("{data}/{t}"))) {
        return Placement::Sure { from, to: data, rule: "data-contents" };
    }
    // 8. Folders the game has deeper down: every top folder inside one game folder.
    if top_files.is_empty() && !tops.is_empty() {
        let mut found: Vec<&String> = ix
            .dirs
            .iter()
            .filter(|p| p.matches('/').count() < INDEX_DEPTH - 1)
            .filter(|p| tops.iter().all(|t| ix.has_dir(&format!("{p}/{t}"))))
            .collect();
        found.sort_by(|a, b| a.matches('/').count().cmp(&b.matches('/').count()).then(a.to_lowercase().cmp(&b.to_lowercase())));
        match found.len() {
            0 => {}
            1 => return Placement::Sure { from, to: found[0].clone(), rule: "game-folder" },
            _ => {
                return Placement::Guess {
                    from,
                    to: found[0].clone(),
                    rule: "game-folder",
                    others: found[1..].iter().map(|s| s.to_string()).collect(),
                }
            }
        }
    }
    // 9. Unreal .pak mods: loose, or in one ~mods or LogicMods folder, and the game has one Content/Paks folder.
    let all_paks = view.iter().all(|e| PAK_EXTS.contains(&ext(&e.name_lc())));
    if all_paks {
        let paks: Vec<&String> = ix.dirs.iter().filter(|d| d.to_lowercase().ends_with("content/paks")).collect();
        let into = if top_files.len() == view.len() {
            Some((from.clone(), "~mods"))
        } else if top_files.is_empty() && tops.len() == 1 && view.iter().all(|e| e.parts.len() == 2) {
            let t = &tops[0];
            match t.to_lowercase().as_str() {
                "~mods" => Some((join(&from, t), "~mods")),
                "logicmods" => Some((join(&from, t), "LogicMods")),
                _ => None,
            }
        } else {
            None
        };
        if let (Some((f, sub)), 1) = (into, paks.len()) {
            let existing = ix.dir(&format!("{}/{sub}", paks[0]));
            let to = existing.unwrap_or_else(|| format!("{}/{sub}", paks[0]));
            return Placement::Sure { from: f, to, rule: "unreal-paks" };
        }
    }
    // 10. Loose files the game already has, all in the same one folder: replacements (a Guess).
    if tops.is_empty() && !top_files.is_empty() {
        let mut parent: Option<String> = None;
        let mut ok = true;
        for e in &top_files {
            let n = e.name_lc();
            let places: Vec<&String> = ix.files.iter().filter(|f| f.rsplit('/').next().is_some_and(|x| x.to_lowercase() == n)).collect();
            if places.len() != 1 {
                ok = false;
                break;
            }
            let p = places[0].rsplit_once('/').map(|(d, _)| d.to_string()).unwrap_or_default();
            match &parent {
                None => parent = Some(p),
                Some(x) if *x == p => {}
                Some(_) => {
                    ok = false;
                    break;
                }
            }
        }
        if let (true, Some(p)) = (ok, parent) {
            return Placement::Guess { from, to: p, rule: "loose-files", others: Vec::new() };
        }
    }
    // 11. No idea: ask, starting at the folder of the game's program.
    let _ = ctx.rules;
    Placement::Ask { suggest_to: ix.main_exe_folder() }
}

fn hints(ctx: &Ctx, es: &[E], from: &str) -> Option<Placement> {
    let view = content(es);
    for h in &ctx.rules.layout {
        let to = h.to.clone();
        if !to.is_empty() && !crate::gamedata::safe_path(&to) {
            continue;
        }
        let hit = match h.matches.as_str() {
            "ext" => !view.is_empty() && view.iter().all(|e| h.values.iter().any(|v| *v == ext(&e.name_lc()))),
            "folder" => top_folders(&view).iter().any(|t| h.values.iter().any(|v| t.eq_ignore_ascii_case(v))),
            "file" => es.iter().any(|e| h.values.iter().any(|v| e.name_lc() == *v)),
            _ => false,
        };
        if hit {
            // Use the game's casing for a folder it has already.
            let to = ctx.index.dir(&to).unwrap_or(to);
            return Some(Placement::Sure { from: from.to_string(), to, rule: "data-file-hint" });
        }
    }
    None
}

/// The archive's files under `from` mapped into `to` (archive path, game-relative path). Readmes and images right
/// in `from` are left out when there is anything else.
pub fn map_placement(files: &[String], from: &str, to: &str) -> Result<Vec<(String, String)>, String> {
    let es = normalize(files)?;
    let mut cur = es;
    for part in from.split('/').filter(|p| !p.is_empty()) {
        cur = strip(&cur, &part.to_lowercase());
    }
    let view = content(&cur);
    let out: Vec<(String, String)> = view.iter().map(|e| (e.src.clone(), join(to, &e.parts.join("/")))).collect();
    if out.is_empty() {
        return Err(format!("the archive has no files in {}", if from.is_empty() { "it" } else { from }));
    }
    if let Some((_, bad)) = out.iter().find(|(_, d)| !crate::deploy::safe_rel(d)) {
        return Err(format!("{bad} isn't a plain path inside the game folder"));
    }
    Ok(out)
}

/// Is `from` a folder in this archive ("" always is)?
pub fn archive_has_folder(files: &[String], from: &str) -> bool {
    if from.is_empty() {
        return true;
    }
    let f = format!("{}/", from.to_lowercase().trim_matches('/'));
    files.iter().any(|p| p.replace('\\', "/").trim_start_matches('/').to_lowercase().starts_with(&f))
}

/// The archive as folders with their file counts, for the "Mod files" pane: [{"path","files"}], "" first.
pub fn archive_tree_json(files: &[String]) -> String {
    let es = normalize(files).unwrap_or_default();
    let mut counts: std::collections::BTreeMap<String, (String, usize)> = std::collections::BTreeMap::new();
    for e in &es {
        let mut acc = String::new();
        counts.entry(String::new()).or_insert((String::new(), 0)).1 += 1;
        for part in &e.parts[..e.parts.len() - 1] {
            acc = join(&acc, part);
            counts.entry(acc.to_lowercase()).or_insert((acc.clone(), 0)).1 += 1;
        }
    }
    let items: Vec<String> = counts
        .values()
        .map(|(path, n)| format!("{{\"path\":{},\"files\":{n}}}", crate::json::quote(path)))
        .collect();
    format!("[{}]", items.join(","))
}

/// Folders right in the game's `rel` folder, for the "Game folder" pane: [{"name","path","turbine"}]. `made`: the
/// folders Turbine created (from its record), lowercased.
pub fn game_folders_json(index: &GameIndex, rel: &str, made: &HashSet<String>) -> String {
    let base = index.dir(rel).unwrap_or_default();
    let items: Vec<String> = index
        .children(&base)
        .iter()
        .map(|n| {
            let path = join(&base, n);
            format!(
                "{{\"name\":{},\"path\":{},\"turbine\":{}}}",
                crate::json::quote(n),
                crate::json::quote(&path),
                made.contains(&path.to_lowercase())
            )
        })
        .collect();
    format!("[{}]", items.join(","))
}

/// The Placement as JSON for the interface: {"state":"sure"|"guess"|"ask","from","to","rule","others"}.
pub fn placement_json(p: &Placement) -> String {
    use crate::json::quote;
    match p {
        Placement::Sure { from, to, rule } => format!("{{\"state\":\"sure\",\"from\":{},\"to\":{},\"rule\":{},\"others\":[]}}", quote(from), quote(to), quote(rule)),
        Placement::Guess { from, to, rule, others } => format!(
            "{{\"state\":\"guess\",\"from\":{},\"to\":{},\"rule\":{},\"others\":[{}]}}",
            quote(from),
            quote(to),
            quote(rule),
            others.iter().map(|o| quote(o)).collect::<Vec<_>>().join(",")
        ),
        Placement::Ask { suggest_to } => format!("{{\"state\":\"ask\",\"from\":\"\",\"to\":{},\"rule\":\"\",\"others\":[]}}", quote(suggest_to)),
    }
}
