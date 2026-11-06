//! Proton settings for games whose script loader is a DLL placed next to the game (BepInEx's winhttp.dll, RED4ext's
//! winmm.dll and Cyber Engine Tweaks' version.dll for Cyberpunk 2077).
//!
//! Wine only loads a game folder's own winhttp.dll when the prefix says so: a value in
//! HKEY_CURRENT_USER\Software\Wine\DllOverrides, e.g. "winhttp"="native,builtin". Wine keeps that branch of the
//! registry in <prefix>/user.reg, a text file it writes itself. This module reads and changes that one value and
//! leaves every other byte of the file as it was.
//!
//! The file format, from Wine's own source (wine-mirror/wine, commit c42c1532, Nov 5 2026):
//! - server/registry.c, save_all_subkeys(): the first line is "WINE REGISTRY Version 2", then a ";; All keys relative
//!   to ..." comment and an optional "#arch=win32" or "#arch=win64" line.
//! - server/registry.c, save_subkeys(): each key is "\n[" + its path + "] " + modification time in seconds since 1970,
//!   then "#time=" (the time in 100 ns ticks since 1601, as %x%08x), optional "#class=" and "#link" lines, then one
//!   line per value. dump_path() joins the parts of the path with "\\\\" in C, which is two backslashes in the file.
//! - server/registry.c, dump_value(): a value is "\"name\"=" (or "@=" for the default value), then for REG_SZ a quoted
//!   string, "str(N):" before the quotes for other string types, "dword:%08x", or "hex:" / "hex(N):" bytes that may
//!   continue on the next lines after a trailing backslash.
//! - server/unicode.c, dump_strW(): in names and strings, backslash and the quote character get a backslash in front,
//!   characters under 32 become C escapes (\n, \t, ...) or octal, characters over 127 become \x escapes. parse_strW()
//!   reads them back (and also accepts plain UTF-8).
//! - server/registry.c, load_keys(): lines are read one at a time; a trailing "\r" is dropped; leading spaces are
//!   skipped; '[' starts a key, '"' or '@' a value, '#' an option, ';' a comment. Key and value names are matched
//!   ignoring case (find_subkey(), find_value() use a case-insensitive compare).
//!
//! Wine writes user.reg again when the prefix shuts down, so it must only be changed while the game isn't running.

use crate::games::{Game, Runtime};
use crate::rules::{Family, GameRules, Kind};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// The key under HKEY_CURRENT_USER that holds DLL overrides, as Wine reads it (single backslashes).
pub const OVERRIDES_KEY: &str = "Software\\Wine\\DllOverrides";
/// Name of the one-time copy kept next to user.reg before Turbine first changes it.
pub const BACKUP_NAME: &str = "user.reg.turbine-backup";
/// Said when the prefix has no user.reg yet (Proton makes it the first time the game starts).
pub const NOT_STARTED: &str = "Start the game once from Steam, then try again.";
/// What BepInEx needs: the game folder's winhttp.dll first, Wine's own if there is none.
pub const BEPINEX_VALUE: &str = "native,builtin";

/// Does this game need the winhttp override for its loader? Windows builds of BepInEx games do (under Proton);
/// native Linux builds start BepInEx with the pack's own script instead.
pub fn wants_winhttp(game: &Game, rules: &GameRules) -> bool {
    rules.kind == Kind::BepInEx && game.runtime == Runtime::Windows
}

/// What Wine needs for a DLL from the game folder to load before its own: the game folder's copy first, Wine's own if
/// there is none (so it does no harm when the mod is turned off again).
pub const NATIVE_FIRST: &str = "native,builtin";

/// The DLL overrides this game needs in its Proton prefix, as (dll, value), for what is turned on now.
/// `files` are game-relative files of the turned-on mods plus the relevant files already in the game folder
/// (rules::present_files). Only Windows builds run through Proton need them.
/// - BepInEx games: winhttp (as before, see `wants_winhttp`).
/// - Cyberpunk 2077: winmm when RED4ext is there (its loader is bin/x64/winmm.dll), version when Cyber Engine Tweaks
///   is (its ASI loader is bin/x64/version.dll). The RED modding wiki's Linux page gives the same pair as the launch
///   option WINEDLLOVERRIDES="winmm,version=n,b" (wiki.redmodding.org, Modding on Linux); see cyberpunk.rs for where
///   each file comes from.
pub fn wanted_overrides(game: &Game, rules: &GameRules, files: &[String]) -> Vec<(&'static str, &'static str)> {
    if game.runtime != Runtime::Windows {
        return Vec::new();
    }
    if wants_winhttp(game, rules) {
        return vec![("winhttp", BEPINEX_VALUE)];
    }
    if rules.family == Family::Cyberpunk {
        return crate::cyberpunk::wanted_dlls(files).into_iter().map(|d| (d, NATIVE_FIRST)).collect();
    }
    Vec::new()
}

/// Sets every override `wanted_overrides` names. Returns the DLLs whose setting was changed.
pub fn ensure_overrides(prefix: &Path, wanted: &[(&str, &str)]) -> Result<Vec<String>, String> {
    let mut changed = Vec::new();
    for (dll, value) in wanted {
        if ensure_override(prefix, dll, value)? {
            changed.push(dll.to_string());
        }
    }
    Ok(changed)
}

/// The value of one DLL override (e.g. "native,builtin"), or None when it isn't set.
pub fn read_override(prefix: &Path, dll: &str) -> Result<Option<String>, String> {
    let bytes = read_reg(prefix)?;
    let file = RegFile::parse(&bytes);
    let Some(sec) = file.section(OVERRIDES_KEY) else { return Ok(None) };
    match sec.find_value(dll) {
        None => Ok(None),
        Some(v) => match parse_string_value(&bytes[v.data_at..v.end]) {
            Some(s) => Ok(Some(s)),
            None => Err(format!("The {dll} setting in this game's Proton settings isn't text, so Turbine leaves it alone.")),
        },
    }
}

/// Sets one DLL override. Returns true when user.reg was changed, false when it already had this value.
pub fn ensure_override(prefix: &Path, dll: &str, value: &str) -> Result<bool, String> {
    let bytes = read_reg(prefix)?;
    let file = RegFile::parse(&bytes);
    let eol = file.eol();
    let line = value_line(dll, value, eol);
    let mut out = Vec::with_capacity(bytes.len() + line.len() + 128);
    match file.section(OVERRIDES_KEY) {
        Some(sec) => {
            if let Some(v) = sec.find_value(dll) {
                if parse_string_value(&bytes[v.data_at..v.end]).as_deref() == Some(value) {
                    return Ok(false);
                }
                out.extend_from_slice(&bytes[..v.start]);
                out.extend_from_slice(&line);
                out.extend_from_slice(&bytes[v.end..]);
            } else {
                // Wine keeps values sorted by name; the new one goes in its place among them.
                let at = sec.insert_point(dll);
                out.extend_from_slice(&bytes[..at]);
                if at > 0 && bytes[at - 1] != b'\n' {
                    out.extend_from_slice(eol);
                }
                out.extend_from_slice(&line);
                out.extend_from_slice(&bytes[at..]);
            }
        }
        None => {
            out.extend_from_slice(&bytes);
            if !bytes.ends_with(b"\n") {
                out.extend_from_slice(eol);
            }
            out.extend_from_slice(&key_header(OVERRIDES_KEY, eol));
            out.extend_from_slice(&line);
        }
    }
    write_reg(prefix, &bytes, &out)?;
    Ok(true)
}

/// Takes one DLL override out. Returns true when user.reg was changed, false when it wasn't set.
pub fn remove_override(prefix: &Path, dll: &str) -> Result<bool, String> {
    let bytes = read_reg(prefix)?;
    let file = RegFile::parse(&bytes);
    let Some(sec) = file.section(OVERRIDES_KEY) else { return Ok(false) };
    let Some(v) = sec.find_value(dll) else { return Ok(false) };
    let mut out = Vec::with_capacity(bytes.len());
    out.extend_from_slice(&bytes[..v.start]);
    out.extend_from_slice(&bytes[v.end..]);
    write_reg(prefix, &bytes, &out)?;
    Ok(true)
}

/// A pgrep -f pattern that matches any process whose command line holds this folder, written with / or \ between
/// its parts (Proton passes the game's .exe as a Linux path, Wine shows it as a Windows path). The folder should be
/// absolute: then the pattern starts with a bracket class, so it never matches the command line that carries it
/// (the pgrep call itself, or flatpak-spawn passing it on).
pub fn running_pattern(dir: &Path) -> String {
    let text = dir.to_string_lossy();
    let mut out = String::new();
    for ch in text.trim_end_matches('/').chars() {
        match ch {
            '/' => out.push_str("[/\\\\]"),
            '.' | '^' | '$' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------------------------
// Reading and writing the file
// ---------------------------------------------------------------------------------------------------------------

fn reg_path(prefix: &Path) -> PathBuf {
    prefix.join("user.reg")
}

fn read_reg(prefix: &Path) -> Result<Vec<u8>, String> {
    let path = reg_path(prefix);
    match fs::read(&path) {
        Ok(b) => {
            if !b.starts_with(b"WINE REGISTRY Version 2") {
                return Err("This game's Proton settings file isn't in a form Turbine knows, so it was left alone.".into());
            }
            Ok(b)
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Err(NOT_STARTED.into()),
        Err(e) => Err(format!("Couldn't read this game's Proton settings: {e}")),
    }
}

/// Keeps a copy of the file as it was before Turbine's first change, then puts the new text in place in one step.
fn write_reg(prefix: &Path, old: &[u8], new: &[u8]) -> Result<(), String> {
    let path = reg_path(prefix);
    let backup = prefix.join(BACKUP_NAME);
    let fail = |e: io::Error| format!("Couldn't save this game's Proton settings: {e}");
    if !backup.exists() {
        write_atomic(&backup, old, &path).map_err(fail)?;
    }
    write_atomic(&path, new, &path).map_err(fail)
}

fn write_atomic(path: &Path, bytes: &[u8], perms_from: &Path) -> io::Result<()> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp = path.with_file_name(format!(".{name}.turbine-new"));
    {
        let mut f = File::create(&tmp)?;
        f.write_all(bytes)?;
        if let Ok(meta) = fs::metadata(perms_from) {
            let _ = f.set_permissions(meta.permissions());
        }
        f.sync_all()?;
    }
    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    if let Some(dir) = path.parent() {
        if let Ok(d) = File::open(dir) {
            let _ = d.sync_all();
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------------
// The format
// ---------------------------------------------------------------------------------------------------------------

/// One line: where it starts, where its text ends (before "\r\n" or "\n") and where the next line starts.
#[derive(Clone, Copy)]
struct Line {
    start: usize,
    text_end: usize,
    end: usize,
}

struct RegFile<'a> {
    bytes: &'a [u8],
    lines: Vec<Line>,
}

/// One key: the index of its "[...]" line and of the first line of the next key (or the number of lines).
struct Section<'f, 'a> {
    file: &'f RegFile<'a>,
    header: usize,
    stop: usize,
}

/// One value: its bytes in the file, from the start of its first line to the end of its last line (line ending
/// included), and where its data starts (after "name"=).
struct Value {
    start: usize,
    end: usize,
    data_at: usize,
}

impl<'a> RegFile<'a> {
    fn parse(bytes: &'a [u8]) -> RegFile<'a> {
        let mut lines = Vec::new();
        let mut start = 0;
        while start < bytes.len() {
            let nl = bytes[start..].iter().position(|&b| b == b'\n').map(|p| start + p);
            let end = nl.map(|p| p + 1).unwrap_or(bytes.len());
            let mut text_end = nl.unwrap_or(bytes.len());
            if text_end > start && bytes[text_end - 1] == b'\r' {
                text_end -= 1;
            }
            lines.push(Line { start, text_end, end });
            start = end;
        }
        RegFile { bytes, lines }
    }

    /// The line ending the file uses: "\n" as Wine writes it, "\r\n" if something rewrote it that way.
    fn eol(&self) -> &'static [u8] {
        match self.lines.first() {
            Some(l) if l.end == l.text_end + 2 => b"\r\n",
            _ => b"\n",
        }
    }

    /// The text of a line after leading spaces (load_keys() skips them too).
    fn text(&self, i: usize) -> (&'a [u8], usize) {
        let l = self.lines[i];
        let mut s = l.start;
        while s < l.text_end && self.bytes[s].is_ascii_whitespace() {
            s += 1;
        }
        (&self.bytes[s..l.text_end], s)
    }

    fn is_key(&self, i: usize) -> bool {
        self.text(i).0.first() == Some(&b'[')
    }

    /// The section for this key path (compared ignoring case). If the key shows up twice, the last one is used,
    /// because its values are the ones Wine ends up with.
    fn section(&self, key: &str) -> Option<Section<'_, 'a>> {
        let want = fold(&utf16(key));
        let mut header = None;
        for i in 0..self.lines.len() {
            if !self.is_key(i) {
                continue;
            }
            let (t, _) = self.text(i);
            if let Some((name, _)) = parse_str(&t[1..], b']') {
                if fold(&name) == want {
                    header = Some(i);
                }
            }
        }
        let header = header?;
        let stop = (header + 1..self.lines.len()).find(|&i| self.is_key(i)).unwrap_or(self.lines.len());
        Some(Section { file: self, header, stop })
    }
}

impl Section<'_, '_> {
    /// Every value in the section: (name, value bytes).
    fn values(&self) -> Vec<(Vec<u16>, Value)> {
        let f = self.file;
        let mut out = Vec::new();
        let mut i = self.header + 1;
        while i < self.stop {
            let (t, at) = f.text(i);
            let first = i;
            // A hex value goes on over the next lines while a line ends with a backslash.
            let mut last = i;
            while last + 1 < self.stop && f.text(last).0.last() == Some(&b'\\') {
                last += 1;
            }
            i = last + 1;
            let parsed = match t.first() {
                Some(b'"') => parse_str(&t[1..], b'"').map(|(n, used)| (n, 1 + used + 1)),
                Some(b'@') => Some((Vec::new(), 1)),
                _ => None,
            };
            let Some((name, mut len)) = parsed else { continue };
            while len < t.len() && t[len].is_ascii_whitespace() {
                len += 1;
            }
            if t.get(len) != Some(&b'=') {
                continue;
            }
            len += 1;
            while len < t.len() && t[len].is_ascii_whitespace() {
                len += 1;
            }
            out.push((name, Value { start: f.lines[first].start, end: f.lines[last].end, data_at: at + len }));
        }
        out
    }

    fn find_value(&self, dll: &str) -> Option<Value> {
        let want = fold(&utf16(dll));
        // Like keys, the last copy of a name is the one Wine keeps.
        self.values().into_iter().filter(|(n, _)| !n.is_empty() && fold(n) == want).map(|(_, v)| v).next_back()
    }

    /// Where a new value goes: before the first value whose name sorts after it, or after the section's last
    /// value or option line (never after the blank line that ends the section).
    fn insert_point(&self, dll: &str) -> usize {
        let f = self.file;
        let want = fold(&utf16(dll));
        let values = self.values();
        if let Some((_, v)) = values.iter().find(|(n, _)| !n.is_empty() && fold(n) > want) {
            return v.start;
        }
        let mut at = f.lines[self.header].end;
        for i in self.header + 1..self.stop {
            if !f.text(i).0.is_empty() {
                at = f.lines[i].end;
            }
        }
        at
    }
}

/// Reads a string written by dump_strW() up to the closing `end` byte. Returns the UTF-16 text and how many bytes
/// were used (not counting the closing byte), or None if it isn't closed. Follows parse_strW() in server/unicode.c.
fn parse_str(src: &[u8], end: u8) -> Option<(Vec<u16>, usize)> {
    let mut out = Vec::new();
    let mut p = 0;
    let hex = |b: u8| (b as char).to_digit(16).map(|d| d as u16);
    while p < src.len() && src[p] != end {
        if src[p] == b'\\' {
            p += 1;
            if p >= src.len() {
                return None;
            }
            let c = src[p];
            let simple = match c {
                b'a' => Some(7),
                b'b' => Some(8),
                b'e' => Some(27),
                b'f' => Some(12),
                b'n' => Some(10),
                b'r' => Some(13),
                b't' => Some(9),
                b'v' => Some(11),
                _ => None,
            };
            if let Some(v) = simple {
                out.push(v);
                p += 1;
                continue;
            }
            if c == b'x' {
                p += 1;
                let mut v: u16 = 0;
                let mut n = 0;
                while n < 4 && p < src.len() {
                    let Some(d) = hex(src[p]) else { break };
                    v = v.wrapping_mul(16).wrapping_add(d);
                    p += 1;
                    n += 1;
                }
                out.push(if n == 0 { b'x' as u16 } else { v });
                continue;
            }
            if (b'0'..=b'7').contains(&c) {
                let mut v: u16 = 0;
                let mut n = 0;
                while n < 3 && p < src.len() && (b'0'..=b'7').contains(&src[p]) {
                    v = v * 8 + (src[p] - b'0') as u16;
                    p += 1;
                    n += 1;
                }
                out.push(v);
                continue;
            }
            // Any other character after a backslash stands for itself.
        }
        // Plain text is UTF-8.
        let rest = &src[p..];
        let len = utf8_len(rest[0]).min(rest.len());
        match std::str::from_utf8(&rest[..len]) {
            Ok(s) => out.extend(s.encode_utf16()),
            Err(_) => out.push(rest[0] as u16),
        }
        p += len.max(1);
    }
    if p >= src.len() {
        return None;
    }
    Some((out, p))
}

fn utf8_len(b: u8) -> usize {
    match b {
        0xF0..=0xF7 => 4,
        0xE0..=0xEF => 3,
        0xC0..=0xDF => 2,
        _ => 1,
    }
}

/// The text of a REG_SZ value (or another string type written as str(N):"..."), from the bytes of its line.
fn parse_string_value(data: &[u8]) -> Option<String> {
    let mut d = data;
    if d.starts_with(b"str(") {
        let close = d.iter().position(|&b| b == b')')?;
        if d.get(close + 1) != Some(&b':') {
            return None;
        }
        d = &d[close + 2..];
    }
    if d.first() != Some(&b'"') {
        return None;
    }
    let (text, _) = parse_str(&d[1..], b'"')?;
    Some(String::from_utf16_lossy(&text))
}

/// Writes text the way dump_strW() does; `quotes` are the extra characters that need a backslash.
fn dump_str(s: &str, quotes: &[u8; 2], out: &mut Vec<u8>) {
    const ESCAPES: &[u8; 32] = b".......abtnvfr.............e....";
    let units: Vec<u16> = s.encode_utf16().collect();
    for (i, &c) in units.iter().enumerate() {
        let next = units.get(i + 1).copied();
        if c > 127 {
            let next_hex = next.is_some_and(|n| n < 128 && (n as u8).is_ascii_hexdigit());
            if next_hex {
                out.extend_from_slice(format!("\\x{c:04x}").as_bytes());
            } else {
                out.extend_from_slice(format!("\\x{c:x}").as_bytes());
            }
            continue;
        }
        if c < 32 {
            let e = ESCAPES[c as usize];
            if e != b'.' {
                out.push(b'\\');
                out.push(e);
            } else if next.is_some_and(|n| (b'0' as u16..=b'7' as u16).contains(&n)) {
                out.extend_from_slice(format!("\\{c:03o}").as_bytes());
            } else {
                out.extend_from_slice(format!("\\{c:o}").as_bytes());
            }
            continue;
        }
        let b = c as u8;
        if b == b'\\' || quotes.contains(&b) {
            out.push(b'\\');
        }
        out.push(b);
    }
}

/// "name"="value" and a line ending, as dump_value() writes a REG_SZ value.
fn value_line(name: &str, value: &str, eol: &[u8]) -> Vec<u8> {
    let mut out = vec![b'"'];
    dump_str(name, b"\"\"", &mut out);
    out.extend_from_slice(b"\"=\"");
    dump_str(value, b"\"\"", &mut out);
    out.push(b'"');
    out.extend_from_slice(eol);
    out
}

/// A new key's first lines, as save_subkeys() writes them: a blank line, "[path] seconds", "#time=ticks".
fn key_header(path: &str, eol: &[u8]) -> Vec<u8> {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    // 100 ns ticks since 1601-01-01, which is 11644473600 seconds before 1970.
    let ticks = (secs + 11_644_473_600) * 10_000_000;
    let mut out = Vec::new();
    out.extend_from_slice(eol);
    out.push(b'[');
    for (i, part) in path.split('\\').enumerate() {
        if i > 0 {
            out.extend_from_slice(b"\\\\");
        }
        dump_str(part, b"[]", &mut out);
    }
    out.extend_from_slice(format!("] {secs}").as_bytes());
    out.extend_from_slice(eol);
    out.extend_from_slice(format!("#time={:x}{:08x}", (ticks >> 32) as u32, ticks as u32).as_bytes());
    out.extend_from_slice(eol);
    out
}

fn utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

/// Case folding for comparing names the way Wine does (its compare ignores case; ASCII is what matters here).
fn fold(s: &[u16]) -> Vec<u16> {
    s.iter().map(|&c| if (b'A' as u16..=b'Z' as u16).contains(&c) { c + 32 } else { c }).collect()
}
