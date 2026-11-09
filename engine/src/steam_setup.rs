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
    files.into_iter().map(|(_, p)| p).collect()
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
