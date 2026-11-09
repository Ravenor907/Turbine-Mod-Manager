//! 0.59: Turbine sets a game's Steam Launch Options itself, so the player never pastes Turbine's line into Steam's
//! Properties. They live in userdata/<account>/config/localconfig.vdf (text VDF), under
//! UserLocalConfigStore/Software/Valve/Steam/apps/<app id>/LaunchOptions. Steam keeps that file in memory while it
//! runs, writes it back when it closes and reads it when it starts, so Turbine changes it only while Steam is closed
//! (the app's own check), with a backup first, and checks right before replacing it that nobody changed it since it
//! was read. Only that one value changes: every other byte of the file stays as Steam wrote it, and the result is
//! read back and compared with the old file before it is used.

use crate::vdf::{self, Vdf};
use std::fs;
use std::path::{Path, PathBuf};

/// The path to a game's settings in localconfig.vdf.
const APPS: &[&str] = &["UserLocalConfigStore", "Software", "Valve", "Steam", "apps"];

#[derive(Debug, Clone)]
enum Val {
    /// The value token's byte range (quotes included).
    Leaf(usize, usize),
    /// The byte offset of '}', and the children.
    Obj(usize, Vec<Node>),
}

#[derive(Debug, Clone)]
struct Node {
    key: String,
    val: Val,
}

struct Lexer<'a> {
    t: &'a str,
    pos: usize,
}

enum Tok {
    Str(String, usize, usize),
    Open,
    Close(usize),
    End,
}

impl Lexer<'_> {
    fn skip(&mut self) {
        let b = self.t.as_bytes();
        while self.pos < b.len() {
            if b[self.pos].is_ascii_whitespace() {
                self.pos += 1;
            } else if b[self.pos] == b'/' && b.get(self.pos + 1) == Some(&b'/') {
                while self.pos < b.len() && b[self.pos] != b'\n' {
                    self.pos += 1;
                }
            } else if self.t[self.pos..].starts_with('\u{feff}') {
                self.pos += 3;
            } else {
                break;
            }
        }
    }

    fn next(&mut self) -> Result<Tok, String> {
        self.skip();
        let b = self.t.as_bytes();
        let start = self.pos;
        match b.get(self.pos) {
            None => Ok(Tok::End),
            Some(b'{') => {
                self.pos += 1;
                Ok(Tok::Open)
            }
            Some(b'}') => {
                self.pos += 1;
                Ok(Tok::Close(start))
            }
            Some(b'"') => {
                self.pos += 1;
                let mut s = String::new();
                loop {
                    let c = self.t[self.pos..].chars().next().ok_or("a text without its closing quote")?;
                    self.pos += c.len_utf8();
                    match c {
                        '"' => break,
                        '\\' => {
                            let e = self.t[self.pos..].chars().next().ok_or("a text without its closing quote")?;
                            self.pos += e.len_utf8();
                            match e {
                                'n' => s.push('\n'),
                                't' => s.push('\t'),
                                '\\' => s.push('\\'),
                                '"' => s.push('"'),
                                o => {
                                    s.push('\\');
                                    s.push(o);
                                }
                            }
                        }
                        c => s.push(c),
                    }
                }
                Ok(Tok::Str(s, start, self.pos))
            }
            Some(_) => {
                while let Some(&c) = b.get(self.pos) {
                    if c.is_ascii_whitespace() || c == b'{' || c == b'}' || c == b'"' {
                        break;
                    }
                    self.pos += 1;
                }
                Ok(Tok::Str(self.t[start..self.pos].to_string(), start, self.pos))
            }
        }
    }

    fn body(&mut self, depth: usize) -> Result<(Vec<Node>, usize), String> {
        let mut out = Vec::new();
        loop {
            let key = match self.next()? {
                Tok::End if depth == 0 => return Ok((out, self.t.len())),
                Tok::End => return Err("an object without its end".into()),
                Tok::Close(at) if depth > 0 => return Ok((out, at)),
                Tok::Str(k, ..) => k,
                _ => return Err("a brace where a key should be".into()),
            };
            let val = match self.next()? {
                Tok::Str(_, s, e) => Val::Leaf(s, e),
                Tok::Open if depth < vdf::MAX_DEPTH => {
                    let (children, close) = self.body(depth + 1)?;
                    Val::Obj(close, children)
                }
                _ => return Err("a key without a value".into()),
            };
            out.push(Node { key, val });
        }
    }
}

fn find<'a>(nodes: &'a [Node], key: &str) -> Option<&'a Node> {
    nodes.iter().find(|n| n.key.eq_ignore_ascii_case(key))
}

fn quoted(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// The leading spaces and tabs of the line `at` is on, and where that line starts.
fn line_indent(t: &str, at: usize) -> (usize, String) {
    let start = t[..at].rfind('\n').map_or(0, |n| n + 1);
    let indent: String = t[start..at].chars().take_while(|c| *c == '\t' || *c == ' ').collect();
    (start, indent)
}

/// Sets a value in a parsed tree (to compare the edited file with).
fn set_in(v: &mut Vdf, path: &[&str], value: &str) {
    let Vdf::Obj(items) = v else { return };
    let Some((first, rest)) = path.split_first() else { return };
    let at = match items.iter().position(|(k, _)| k.eq_ignore_ascii_case(first)) {
        Some(i) => i,
        None => {
            items.push((first.to_string(), if rest.is_empty() { Vdf::Str(String::new()) } else { Vdf::Obj(Vec::new()) }));
            items.len() - 1
        }
    };
    if rest.is_empty() {
        items[at].1 = Vdf::Str(value.to_string());
    } else {
        set_in(&mut items[at].1, rest, value);
    }
}

/// The localconfig.vdf text with the game's Launch Options set to `value`, every other byte unchanged. Errors when
/// the file can't be read as Steam's settings or has no games list.
pub fn set_launch_options_in(text: &str, app_id: &str, value: &str) -> Result<String, String> {
    if app_id.is_empty() || !app_id.bytes().all(|b| b.is_ascii_digit()) {
        return Err("not a Steam app id".into());
    }
    let (nodes, _) = Lexer { t: text, pos: 0 }.body(0)?;
    let mut level: &[Node] = &nodes;
    let mut apps = None;
    for (i, k) in APPS.iter().enumerate() {
        let n = find(level, k).ok_or("Steam's settings have no games list yet (start the game once from Steam)")?;
        let Val::Obj(close, children) = &n.val else { return Err("Steam's settings aren't as expected".into()) };
        if i == APPS.len() - 1 {
            apps = Some((*close, children));
        }
        level = children;
    }
    let (apps_close, apps_children) = apps.ok_or("Steam's settings aren't as expected")?;
    let out = match find(apps_children, app_id) {
        Some(Node { val: Val::Obj(close, children), .. }) => match find(children, "LaunchOptions") {
            Some(Node { val: Val::Leaf(s, e), .. }) => format!("{}{}{}", &text[..*s], quoted(value), &text[*e..]),
            Some(_) => return Err("the game's Launch Options aren't a text in Steam's settings".into()),
            None => {
                let (ls, indent) = line_indent(text, *close);
                format!("{}{indent}\t\"LaunchOptions\"\t\t{}\n{}", &text[..ls], quoted(value), &text[ls..])
            }
        },
        Some(_) => return Err("the game's entry in Steam's settings isn't as expected".into()),
        None => {
            let (ls, indent) = line_indent(text, apps_close);
            format!(
                "{}{indent}\t\"{app_id}\"\n{indent}\t{{\n{indent}\t\t\"LaunchOptions\"\t\t{}\n{indent}\t}}\n{}",
                &text[..ls],
                quoted(value),
                &text[ls..]
            )
        }
    };
    // Read back: the new file must be the old one with exactly this value changed.
    let mut want = vdf::parse(text).map_err(|e| format!("Steam's settings can't be read ({e})"))?;
    let mut path: Vec<&str> = APPS.to_vec();
    path.push(app_id);
    path.push("LaunchOptions");
    set_in(&mut want, &path, value);
    let got = vdf::parse(&out).map_err(|e| format!("the changed settings wouldn't read back ({e}): nothing written"))?;
    if got != want {
        return Err("the changed settings didn't read back as expected: nothing written".into());
    }
    Ok(out)
}

/// The localconfig.vdf of each Steam account in this home, the most recently changed first (the account signed in
/// last).
pub fn localconfig_files(home: &Path) -> Vec<PathBuf> {
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    for root in crate::steam::steam_roots(home) {
        let Ok(rd) = fs::read_dir(root.join("userdata")) else { continue };
        for e in rd.flatten() {
            let f = e.path().join("config").join("localconfig.vdf");
            if let Ok(m) = fs::metadata(&f) {
                if m.is_file() && !files.iter().any(|(_, p)| *p == f) {
                    files.push((m.modified().unwrap_or(std::time::UNIX_EPOCH), f));
                }
            }
        }
    }
    files.sort_by_key(|f| std::cmp::Reverse(f.0));
    let mut list: Vec<PathBuf> = files.into_iter().map(|(_, p)| p).collect();
    crate::steam::signed_in_first(home, &mut list);
    list
}

/// The backup Turbine writes before each change (the file as it was just before).
pub fn backup_file(file: &Path) -> PathBuf {
    file.with_file_name("localconfig.vdf.turbine-backup")
}

/// The file as it was before Turbine's first change, kept for good.
pub fn original_file(file: &Path) -> PathBuf {
    file.with_file_name("localconfig.vdf.turbine-original")
}

/// Sets the game's Launch Options in this localconfig.vdf. Only while Steam is closed (`steam_running` from the
/// app's own check); a backup first; nothing written when the file changed meanwhile. Answers the Launch Options
/// as they were (None when the game had none), for Undo. The same value again writes nothing.
pub fn write_launch_options(file: &Path, app_id: &str, value: &str, steam_running: bool) -> Result<Option<String>, String> {
    let before = fs::read(file).map_err(|e| format!("Steam's settings can't be read ({e})"))?;
    if before.len() > 64 << 20 {
        return Err("Steam's settings file is too big to be one".into());
    }
    let text = String::from_utf8(before.clone()).map_err(|_| "Steam's settings aren't text".to_string())?;
    let was = crate::steam::launch_options_in(&text, app_id).filter(|s| !s.is_empty());
    if was.as_deref() == Some(value) {
        return Ok(was);
    }
    if steam_running {
        return Err("Close Steam first: Steam writes its settings back over any change while it runs".into());
    }
    let out = set_launch_options_in(&text, app_id, value)?;
    let orig = original_file(file);
    if !orig.exists() {
        fs::write(&orig, &before).map_err(|e| format!("Couldn't keep a copy of Steam's settings first ({e})"))?;
    }
    fs::write(backup_file(file), &before).map_err(|e| format!("Couldn't keep a copy of Steam's settings first ({e})"))?;
    let tmp = file.with_file_name("localconfig.vdf.turbine-tmp");
    fs::write(&tmp, out.as_bytes()).map_err(|e| e.to_string())?;
    if fs::read(file).ok().as_deref() != Some(&before[..]) {
        let _ = fs::remove_file(&tmp);
        return Err("Steam's settings changed meanwhile (is Steam running?): nothing written".into());
    }
    fs::rename(&tmp, file).map_err(|e| e.to_string())?;
    Ok(was)
}

// ---- Changes waiting for Steam to start again ----
//
// Steam reads shortcuts.vdf and localconfig.vdf when it starts and writes its own copy back when it closes. A change
// Turbine made while Steam was closed shows once Steam starts; one made while Steam was in fact running (the app's
// check missed it) is lost when Steam closes. So each change is noted here, checked once Steam has started again,
// and made again (while Steam is closed) when it didn't hold.

/// One change Turbine made to Steam's files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SteamChange {
    /// A tile added (shortcuts.vdf, by its marker).
    Tile { file: PathBuf, marker: String },
    /// A tile removed.
    NoTile { file: PathBuf, marker: String },
    /// A game's Launch Options set (localconfig.vdf).
    Options { file: PathBuf, app_id: String, value: String },
}

impl SteamChange {
    fn to_json(&self) -> String {
        use crate::json::quote;
        match self {
            SteamChange::Tile { file, marker } => format!("{{\"kind\":\"tile\",\"file\":{},\"marker\":{}}}", quote(&file.to_string_lossy()), quote(marker)),
            SteamChange::NoTile { file, marker } => format!("{{\"kind\":\"notile\",\"file\":{},\"marker\":{}}}", quote(&file.to_string_lossy()), quote(marker)),
            SteamChange::Options { file, app_id, value } => format!(
                "{{\"kind\":\"options\",\"file\":{},\"app\":{},\"value\":{}}}",
                quote(&file.to_string_lossy()),
                quote(app_id),
                quote(value)
            ),
        }
    }

    fn from_json(v: &crate::json::Json) -> Option<SteamChange> {
        let file = PathBuf::from(v.str_at("file")?);
        Some(match v.str_at("kind")? {
            "tile" => SteamChange::Tile { file, marker: v.str_at("marker")?.to_string() },
            "notile" => SteamChange::NoTile { file, marker: v.str_at("marker")?.to_string() },
            "options" => SteamChange::Options { file, app_id: v.str_at("app")?.to_string(), value: v.str_at("value")?.to_string() },
            _ => return None,
        })
    }

    /// Does Steam's file hold this change now?
    /// Did a Steam started at `started` read this change (its file was written before then)? Unknown start: yes.
    pub fn read_by(&self, started: Option<std::time::SystemTime>) -> bool {
        let file = match self {
            SteamChange::Tile { file, .. } | SteamChange::NoTile { file, .. } | SteamChange::Options { file, .. } => file,
        };
        match (started, fs::metadata(file).and_then(|m| m.modified())) {
            (Some(s), Ok(m)) => m <= s,
            _ => true,
        }
    }

    pub fn holds(&self) -> bool {
        match self {
            SteamChange::Tile { file, marker } | SteamChange::NoTile { file, marker } => {
                let there = fs::read(file).ok().and_then(|b| crate::shortcuts::parse(&b).ok()).is_some_and(|l| {
                    l.iter().any(|s| crate::shortcuts::is_turbine(s) && s.launch_options.contains(marker.as_str()))
                });
                there == matches!(self, SteamChange::Tile { .. })
            }
            SteamChange::Options { file, app_id, value } => {
                fs::read_to_string(file).ok().and_then(|t| crate::steam::launch_options_in(&t, app_id)).as_deref() == Some(value.as_str())
            }
        }
    }
}

/// Turbine's list of changes waiting to be checked after Steam starts again.
pub fn pending_file(data: &Path) -> PathBuf {
    data.join("steam-setup").join("pending.json")
}

/// The changes waiting to be checked.
pub fn pending(data: &Path) -> Vec<SteamChange> {
    let Ok(t) = fs::read_to_string(pending_file(data)) else { return Vec::new() };
    let Ok(v) = crate::json::parse(&t) else { return Vec::new() };
    v.get("changes").map(crate::json::Json::as_array).unwrap_or(&[]).iter().filter_map(SteamChange::from_json).collect()
}

fn save_pending(data: &Path, list: &[SteamChange]) -> std::io::Result<()> {
    let f = pending_file(data);
    if list.is_empty() {
        return match fs::remove_file(&f) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        };
    }
    fs::create_dir_all(f.parent().unwrap_or(data))?;
    let items: Vec<String> = list.iter().map(SteamChange::to_json).collect();
    let tmp = f.with_extension("tmp");
    fs::write(&tmp, format!("{{\"changes\":[{}]}}\n", items.join(",")))?;
    fs::rename(tmp, f)
}

/// Notes a change just made (a later one for the same tile or game replaces an earlier one).
pub fn note_change(data: &Path, change: SteamChange) -> std::io::Result<()> {
    let same = |a: &SteamChange, b: &SteamChange| match (a, b) {
        (SteamChange::Tile { file: f1, marker: m1 } | SteamChange::NoTile { file: f1, marker: m1 }, SteamChange::Tile { file: f2, marker: m2 } | SteamChange::NoTile { file: f2, marker: m2 }) => {
            f1 == f2 && m1 == m2
        }
        (SteamChange::Options { file: f1, app_id: a1, .. }, SteamChange::Options { file: f2, app_id: a2, .. }) => f1 == f2 && a1 == a2,
        _ => false,
    };
    let mut list = pending(data);
    list.retain(|c| !same(c, &change));
    list.push(change);
    save_pending(data, &list)
}

/// After Steam started again: the changes that held are done and leave the list; the ones that didn't are answered
/// so the app makes them again the next time Steam is closed (and tells the player). Answers (held, lost).
pub fn check_after_restart(data: &Path) -> (Vec<SteamChange>, Vec<SteamChange>) {
    check_after_restart_since(data, None)
}

/// 0.6 (break M3): the same, with when the running Steam started. A change only counts as held when its file was
/// written before Steam started (Steam then read it); written after, Steam is running on the old copy and writes
/// it back when it closes, so it is lost even though the file has it now.
pub fn check_after_restart_since(data: &Path, steam_started: Option<std::time::SystemTime>) -> (Vec<SteamChange>, Vec<SteamChange>) {
    let (held, lost): (Vec<SteamChange>, Vec<SteamChange>) = pending(data).into_iter().partition(|c| c.holds() && c.read_by(steam_started));
    let _ = save_pending(data, &lost);
    (held, lost)
}
