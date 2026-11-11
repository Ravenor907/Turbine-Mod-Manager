//! 0.58: Edit settings. A mod's (or the game's) config files as a page of plain controls: toggles, numbers (a slider
//! where the file says the range, otherwise steps), lists, and text typed with Steam's keyboard, with the file's own
//! comments as the help. Anything the reader can't make sense of still has the plain text fallback (line by line).
//!
//! Reading never changes a file. Editing changes only the value's own characters, so comments, order, spacing and
//! everything else stay exactly as the mod's author wrote them.
//!
//! Where edits live (a mod's file): Turbine's own copy in the store, `settings/<mod id>/<path>`, never the mod's staged
//! file itself. The store deploys the copy in place of the mod's file (`effective`). Beside it: `.base` (the mod's file
//! the edit was made from) and `.backup` (the copy before the last save). Reset to default takes the copy away (kept
//! as `.backup`). When a mod update brings a new file, the player's changed values are carried into it (`merge_report`).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The file types the page understands. .cfg and .conf read like .ini.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Ini,
    Toml,
    Json,
    Yaml,
    Xml,
}

impl Format {
    pub fn key(self) -> &'static str {
        match self {
            Format::Ini => "ini",
            Format::Toml => "toml",
            Format::Json => "json",
            Format::Yaml => "yaml",
            Format::Xml => "xml",
        }
    }
}

/// The format from a file name, or None when it isn't a config file.
pub fn format_of(name: &str) -> Option<Format> {
    let l = name.to_ascii_lowercase();
    let ext = l.rsplit('.').next().unwrap_or("");
    match ext {
        "ini" | "cfg" | "conf" => Some(Format::Ini),
        "toml" => Some(Format::Toml),
        "json" => Some(Format::Json),
        "yaml" | "yml" => Some(Format::Yaml),
        "xml" => Some(Format::Xml),
        _ => None,
    }
}

/// Files bigger than this are left to the plain text fallback's limits (and never parsed).
pub const MAX_BYTES: u64 = 512 * 1024;

/// What kind of control a setting gets.
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    /// On and off, with the file's own words for each ("True"/"False", "1"/"0").
    Toggle { on: String, off: String },
    /// A number: a slider when the file says the range, otherwise steps.
    Number { min: Option<f64>, max: Option<f64>, step: f64 },
    /// One of a few values: (value, label).
    List(Vec<(String, String)>),
    Text,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub id: usize,
    /// Which one of the settings with this section and name it is (0 for the first): a file may repeat a name.
    pub occ: usize,
    /// "[GAMEPLAY]", "General", "a.b" (JSON and YAML paths): where it is, for the page's headings.
    pub section: String,
    pub key: String,
    /// The value as the player sees it (a string without its quotes).
    pub value: String,
    pub kind: Kind,
    /// The comment above it (and after it on its line), without the comment marks.
    pub help: String,
    /// Bytes of the value in the file (inside the quotes for a quoted string).
    pub span: (usize, usize),
    /// The value is a quoted string (an edit is escaped for the format).
    pub quoted: bool,
    /// 1-based line number.
    pub line: usize,
}

// ---- Reading ----

fn strip_comment_marks(l: &str) -> String {
    let t = l.trim_start();
    let t = t.trim_start_matches([';', '#', '/']).trim_start_matches(['/', ';']);
    t.trim().to_string()
}

fn is_number(v: &str) -> bool {
    let v = v.trim();
    !v.is_empty() && v.parse::<f64>().is_ok() && !v.eq_ignore_ascii_case("nan") && !v.to_ascii_lowercase().contains("inf")
}

fn decimals(v: &str) -> usize {
    v.split_once('.').map(|(_, d)| d.trim_end_matches(|c: char| !c.is_ascii_digit()).len()).unwrap_or(0)
}

/// "0 = Normal | 1 = None | 2 = Ping" or "0=FALSE 1=TRUE": the numbered choices a comment lists.
fn numbered_choices(help: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let b = help.as_bytes();
    let mut i = 0;
    while i < b.len() {
        // A number at a word start, then "=", then words up to the next "<number> =" or "|".
        let start_ok = i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'.');
        if start_ok && (b[i].is_ascii_digit() || (b[i] == b'-' && i + 1 < b.len() && b[i + 1].is_ascii_digit())) {
            let mut j = i + 1;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            let mut k = j;
            while k < b.len() && b[k] == b' ' {
                k += 1;
            }
            if k < b.len() && b[k] == b'=' {
                let num = help[i..j].to_string();
                let mut e = k + 1;
                let label_start = e;
                // The label ends before the next "<digits> =" or a "|".
                loop {
                    if e >= b.len() || b[e] == b'|' {
                        break;
                    }
                    if b[e].is_ascii_digit() && (e == 0 || b[e - 1] == b' ') {
                        let mut d = e;
                        while d < b.len() && b[d].is_ascii_digit() {
                            d += 1;
                        }
                        let mut s = d;
                        while s < b.len() && b[s] == b' ' {
                            s += 1;
                        }
                        if s < b.len() && b[s] == b'=' {
                            break;
                        }
                    }
                    e += 1;
                }
                let label = help[label_start..e].trim().trim_end_matches(['.', ',', ';']).trim().to_string();
                if !label.is_empty() && !out.iter().any(|(v, _)| *v == num) {
                    out.push((num, label));
                }
                i = e;
                continue;
            }
            i = j;
            continue;
        }
        i += 1;
    }
    out
}

fn find_after<'a>(help: &'a str, label: &str) -> Option<&'a str> {
    let l = help.to_ascii_lowercase();
    let i = l.find(&label.to_ascii_lowercase())?;
    Some(help[i + label.len()..].trim_start_matches([':', ' ']))
}

/// The control for a value with its help.
pub fn kind_for(value: &str, help: &str, quoted: bool) -> Kind {
    let v = value.trim();
    let lower = v.to_ascii_lowercase();
    let hl = help.to_ascii_lowercase();
    // The file's own list of values (a config generator's "Acceptable values: A, B, C").
    if let Some(rest) = find_after(help, "acceptable values") {
        let line = rest.lines().next().unwrap_or("");
        let opts: Vec<(String, String)> = line.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).map(|s| (s.clone(), s)).collect();
        if opts.len() >= 2 && opts.iter().any(|(o, _)| o == v) {
            return Kind::List(opts);
        }
    }
    if !quoted {
        for (on, off) in [("true", "false"), ("yes", "no"), ("on", "off")] {
            if lower == on || lower == off {
                let case = |w: &str| {
                    if v.chars().all(|c| c.is_ascii_uppercase()) {
                        w.to_ascii_uppercase()
                    } else if v.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
                        let mut s = w.to_string();
                        s[..1].make_ascii_uppercase();
                        s
                    } else {
                        w.to_string()
                    }
                };
                return Kind::Toggle { on: case(on), off: case(off) };
            }
        }
    }
    if is_number(v) && !quoted {
        let choices = numbered_choices(help);
        let is_bool_words = |a: &str, b: &str| {
            let (a, b) = (a.to_ascii_lowercase(), b.to_ascii_lowercase());
            matches!((a.as_str(), b.as_str()), ("false", "true") | ("off", "on") | ("no", "yes") | ("disabled", "enabled"))
        };
        if (v == "0" || v == "1")
            && (choices.len() == 2 && choices[0].0 == "0" && choices[1].0 == "1" && is_bool_words(&choices[0].1, &choices[1].1)
                || hl.contains("setting type: boolean")
                || hl.contains("0=false")
                || hl.contains("0 = false")
                || hl.contains("1 = true")
                || hl.contains("1=true"))
        {
            return Kind::Toggle { on: "1".into(), off: "0".into() };
        }
        if choices.len() >= 2 && choices.iter().any(|(c, _)| c == v) {
            return Kind::List(choices);
        }
        let step = if decimals(v) > 0 { 10f64.powi(-(decimals(v).min(4) as i32)) } else { 1.0 };
        // "From 0 to 100" (a config generator's range), or the two ends a comment names ("0 = MUTE 10 = MAX").
        if let Some(rest) = find_after(help, "acceptable value range") {
            let nums: Vec<f64> = rest.split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-')).filter_map(|s| s.parse().ok()).collect();
            if nums.len() >= 2 && nums[0] < nums[1] {
                return Kind::Number { min: Some(nums[0]), max: Some(nums[1]), step };
            }
        }
        if choices.len() == 2 {
            let (a, b): (f64, f64) = (choices[0].0.parse().unwrap_or(0.0), choices[1].0.parse().unwrap_or(0.0));
            let x: f64 = v.parse().unwrap_or(0.0);
            if a < b && x >= a && x <= b {
                return Kind::Number { min: Some(a), max: Some(b), step };
            }
        }
        // A percent with a default ("Amount ... (%) (Default: 35)"): steps of 5.
        let step = if step == 1.0 && (hl.contains('%') || hl.contains("percent")) { 5.0 } else { step };
        return Kind::Number { min: if v.starts_with('-') { None } else { Some(0.0) }, max: None, step };
    }
    Kind::Text
}

struct Pending {
    help: Vec<String>,
}

impl Pending {
    fn take(&mut self) -> String {
        let h = self.help.join(" ");
        self.help.clear();
        h
    }
}

fn line_starts(text: &str) -> Vec<usize> {
    let mut v = vec![0];
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            v.push(i + 1);
        }
    }
    v
}

/// Files bigger than this are not read as settings (the plain text is still shown).
pub const PARSE_MAX: usize = 256 * 1024;

/// The settings in a file, in file order, with their values as the player reads them (no escapes) and which
/// occurrence of their name each one is.
pub fn parse(text: &str, fmt: Format) -> Vec<Field> {
    if text.len() > PARSE_MAX {
        return Vec::new();
    }
    let mut fields = match fmt {
        Format::Ini | Format::Toml => parse_ini(text, fmt == Format::Toml),
        Format::Yaml => parse_yaml(text),
        Format::Json => parse_json(text),
        Format::Xml => parse_xml(text),
    };
    let mut seen: std::collections::HashMap<(String, String), usize> = std::collections::HashMap::new();
    for f in fields.iter_mut() {
        let n = seen.entry((f.section.clone(), f.key.clone())).or_insert(0);
        f.occ = *n;
        *n += 1;
        f.value = match (fmt, quote_of(text, f, fmt)) {
            (Format::Toml | Format::Yaml, Some('"')) => unescape_basic(&f.value),
            (Format::Yaml, Some('\'')) => f.value.replace("''", "'"),
            (Format::Xml, _) => unescape_xml(&f.value),
            _ => f.value.clone(),
        };
    }
    fields
}

/// The quote around a field's value in the file, if any (XML text has none).
fn quote_of(text: &str, f: &Field, fmt: Format) -> Option<char> {
    if !f.quoted || fmt == Format::Xml {
        return None;
    }
    text.get(f.span.0.checked_sub(1)?..f.span.0).and_then(|c| c.chars().next())
}

fn unescape_basic(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    let mut it = v.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some('u') => {
                let h: String = it.by_ref().take(4).collect();
                match u32::from_str_radix(&h, 16).ok().and_then(char::from_u32) {
                    Some(ch) => out.push(ch),
                    None => {
                        out.push_str("\\u");
                        out.push_str(&h);
                    }
                }
            }
            Some(o) => {
                out.push('\\');
                out.push(o);
            }
            None => out.push('\\'),
        }
    }
    out
}

fn unescape_xml(v: &str) -> String {
    v.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

fn unquote_span(text: &str, vs: usize, ve: usize) -> (usize, usize, bool) {
    let v = &text[vs..ve];
    let b = v.as_bytes();
    if b.len() >= 2 && ((b[0] == b'"' && b[b.len() - 1] == b'"') || (b[0] == b'\'' && b[b.len() - 1] == b'\'')) {
        (vs + 1, ve - 1, true)
    } else {
        (vs, ve, false)
    }
}

fn parse_ini(text: &str, toml: bool) -> Vec<Field> {
    let mut out = Vec::new();
    let mut section = String::new();
    let mut pending = Pending { help: Vec::new() };
    let starts = line_starts(text);
    for (n, line) in text.split('\n').enumerate() {
        let mut base = starts[n];
        let mut line_nr = line.trim_end_matches('\r');
        // A byte order mark before the first line isn't part of it.
        if let Some(rest) = line_nr.strip_prefix('\u{feff}') {
            base += 3;
            line_nr = rest;
        }
        let t = line_nr.trim();
        if t.is_empty() {
            pending.help.clear();
            continue;
        }
        if t.starts_with(';') || t.starts_with('#') || t.starts_with("//") {
            let c = strip_comment_marks(t);
            // A config generator's "Setting type" and "Default value" lines aren't help (but Boolean says toggle).
            let keep = (!c.is_empty() && !c.starts_with("Setting type") && !c.starts_with("Default value")) || c.starts_with("Setting type: Boolean");
            if keep {
                pending.help.push(c);
            }
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') {
            section = t.trim_matches(['[', ']']).trim().to_string();
            pending.help.clear();
            continue;
        }
        let Some(eq) = line_nr.find('=') else {
            pending.help.clear();
            continue;
        };
        let key = line_nr[..eq].trim();
        if key.is_empty() {
            continue;
        }
        let after = &line_nr[eq + 1..];
        let lead = after.len() - after.trim_start().len();
        let mut vs = eq + 1 + lead;
        let mut ve = line_nr.len();
        let mut inline_help = String::new();
        let rest = &line_nr[vs..];
        if toml && (rest.starts_with('"') || rest.starts_with('\'')) {
            let q = rest.as_bytes()[0];
            // The closing quote (a basic string's \" doesn't count).
            let mut j = 1;
            let rb = rest.as_bytes();
            while j < rb.len() && !(rb[j] == q && (q == b'\'' || rb[j - 1] != b'\\')) {
                j += 1;
            }
            if j >= rb.len() {
                pending.help.clear();
                continue; // a multi-line string: the plain text fallback has it
            }
            let tail = &rest[j + 1..];
            if let Some(h) = tail.find('#') {
                inline_help = tail[h + 1..].trim().to_string();
            }
            ve = vs + j + 1;
        } else {
            // An inline comment: " ;" in .ini (never "#": colors are written "#FFFFFF"), " #" in TOML.
            let mark = if toml { " #" } else { " ;" };
            if let Some(h) = rest.find(mark) {
                inline_help = strip_comment_marks(&rest[h + 1..]);
                ve = vs + h;
            }
            ve = vs + line_nr[vs..ve].trim_end().len();
            if toml && (rest.starts_with('[') || rest.starts_with('{')) {
                // Arrays and inline tables: edited as they are written.
                let (s0, e0) = (vs, ve);
                let mut help = pending.take();
                if !inline_help.is_empty() {
                    help = format!("{help} {inline_help}").trim().to_string();
                }
                out.push(Field {
                    id: out.len(), occ: 0,
                    section: section.clone(),
                    key: key.trim_matches('"').to_string(),
                    value: line_nr[s0..e0].to_string(),
                    kind: Kind::Text,
                    help,
                    span: (base + s0, base + e0),
                    quoted: false,
                    line: n + 1,
                });
                continue;
            }
        }
        if ve < vs {
            ve = vs;
        }
        let (s0, e0, quoted) = if toml { unquote_span(line_nr, vs, ve) } else { (vs, ve, false) };
        vs = s0;
        let value = line_nr[vs..e0].to_string();
        let mut help = pending.take();
        if !inline_help.is_empty() {
            help = format!("{help} {inline_help}").trim().to_string();
        }
        let kind = kind_for(&value, &help, quoted);
        out.push(Field {
            id: out.len(), occ: 0,
            section: section.clone(),
            key: key.trim_matches('"').to_string(),
            value,
            kind,
            help,
            span: (base + vs, base + e0),
            quoted,
            line: n + 1,
        });
    }
    out
}

fn parse_yaml(text: &str) -> Vec<Field> {
    let mut out = Vec::new();
    let mut stack: Vec<(usize, String)> = Vec::new();
    let mut pending = Pending { help: Vec::new() };
    let starts = line_starts(text);
    for (n, raw) in text.split('\n').enumerate() {
        let mut base = starts[n];
        let mut line = raw.trim_end_matches('\r');
        if let Some(rest) = line.strip_prefix('\u{feff}') {
            base += 3;
            line = rest;
        }
        let t = line.trim();
        if t.is_empty() || t == "---" {
            pending.help.clear();
            continue;
        }
        if t.starts_with('#') {
            pending.help.push(strip_comment_marks(t));
            continue;
        }
        if t.starts_with("- ") || t == "-" {
            pending.help.clear();
            continue; // list items: the plain text fallback
        }
        let indent = line.len() - line.trim_start().len();
        let Some(colon) = find_yaml_colon(line) else {
            pending.help.clear();
            continue;
        };
        let key = line[indent..colon].trim().trim_matches(['"', '\'']).to_string();
        while stack.last().is_some_and(|(i, _)| *i >= indent) {
            stack.pop();
        }
        let after = &line[colon + 1..];
        let rest = after.trim();
        if rest.is_empty() || rest.starts_with('#') {
            stack.push((indent, key));
            pending.help.clear();
            continue;
        }
        if rest.starts_with('|') || rest.starts_with('>') || rest.starts_with('[') || rest.starts_with('{') || rest.starts_with('&') || rest.starts_with('*') {
            pending.help.clear();
            continue;
        }
        let lead = after.len() - after.trim_start().len();
        let vs = colon + 1 + lead;
        let mut ve = line.len();
        let mut inline_help = String::new();
        let r = &line[vs..];
        if r.starts_with('"') || r.starts_with('\'') {
            let q = r.as_bytes()[0];
            let rb = r.as_bytes();
            let mut j = 1;
            while j < rb.len() && !(rb[j] == q && (q == b'\'' || rb[j - 1] != b'\\')) {
                j += 1;
            }
            // YAML writes a ' inside single quotes as ''.
            while q == b'\'' && j + 1 < rb.len() && rb[j + 1] == b'\'' {
                j += 2;
                while j < rb.len() && rb[j] != q {
                    j += 1;
                }
            }
            if j >= rb.len() {
                pending.help.clear();
                continue;
            }
            if let Some(h) = r[j + 1..].find('#') {
                inline_help = r[j + 1 + h + 1..].trim().to_string();
            }
            ve = vs + j + 1;
        } else if let Some(h) = r.find(" #") {
            inline_help = r[h + 2..].trim().to_string();
            ve = vs + h;
        }
        ve = vs + line[vs..ve].trim_end().len();
        let (s0, e0, quoted) = unquote_span(line, vs, ve);
        let value = line[s0..e0].to_string();
        let mut help = pending.take();
        if !inline_help.is_empty() {
            help = format!("{help} {inline_help}").trim().to_string();
        }
        let section = stack.iter().map(|(_, k)| k.as_str()).collect::<Vec<_>>().join(".");
        let kind = if value == "null" || value == "~" { Kind::Text } else { kind_for(&value, &help, quoted) };
        out.push(Field { id: out.len(), occ: 0, section, key, value, kind, help, span: (base + s0, base + e0), quoted, line: n + 1 });
    }
    out
}

/// The colon that ends a YAML key (not one inside quotes or a URL's value).
fn find_yaml_colon(line: &str) -> Option<usize> {
    let b = line.as_bytes();
    let mut q: Option<u8> = None;
    for i in 0..b.len() {
        match q {
            Some(c) if b[i] == c => q = None,
            Some(_) => {}
            None if b[i] == b'"' || b[i] == b'\'' => q = Some(b[i]),
            None if b[i] == b'#' => return None,
            None if b[i] == b':' && (i + 1 == b.len() || b[i + 1] == b' ') => return Some(i),
            None => {}
        }
    }
    None
}

fn parse_json(text: &str) -> Vec<Field> {
    struct P<'a> {
        b: &'a [u8],
        i: usize,
        out: Vec<Field>,
        starts: Vec<usize>,
        depth: usize,
    }
    impl P<'_> {
        fn ws(&mut self) {
            while self.i < self.b.len() && (self.b[self.i] as char).is_whitespace() {
                self.i += 1;
            }
        }
        fn string(&mut self) -> Option<(usize, usize)> {
            if self.b.get(self.i) != Some(&b'"') {
                return None;
            }
            let s = self.i + 1;
            let mut j = s;
            while j < self.b.len() && self.b[j] != b'"' {
                if self.b[j] == b'\\' {
                    j += 1;
                }
                j += 1;
            }
            if j >= self.b.len() {
                return None;
            }
            self.i = j + 1;
            Some((s, j))
        }
        fn line_of(&self, at: usize) -> usize {
            self.starts.partition_point(|&s| s <= at)
        }
        fn value(&mut self, path: &[String], key: &str) -> Option<()> {
            self.ws();
            self.depth += 1;
            if self.depth > 32 {
                return None;
            }
            let c = *self.b.get(self.i)?;
            match c {
                b'{' => {
                    self.i += 1;
                    loop {
                        self.ws();
                        if self.b.get(self.i) == Some(&b'}') {
                            self.i += 1;
                            break;
                        }
                        let (ks, ke) = self.string()?;
                        let k = String::from_utf8_lossy(&self.b[ks..ke]).to_string();
                        self.ws();
                        if self.b.get(self.i) != Some(&b':') {
                            return None;
                        }
                        self.i += 1;
                        let mut p = path.to_vec();
                        if !key.is_empty() {
                            p.push(key.to_string());
                        }
                        self.value(&p, &k)?;
                        self.ws();
                        match self.b.get(self.i) {
                            Some(b',') => self.i += 1,
                            Some(b'}') => {
                                self.i += 1;
                                break;
                            }
                            _ => return None,
                        }
                    }
                }
                b'[' => {
                    self.i += 1;
                    let mut n = 0;
                    loop {
                        self.ws();
                        if self.b.get(self.i) == Some(&b']') {
                            self.i += 1;
                            break;
                        }
                        let mut p = path.to_vec();
                        if !key.is_empty() {
                            p.push(key.to_string());
                        }
                        self.value(&p, &n.to_string())?;
                        n += 1;
                        self.ws();
                        match self.b.get(self.i) {
                            Some(b',') => self.i += 1,
                            Some(b']') => {
                                self.i += 1;
                                break;
                            }
                            _ => return None,
                        }
                    }
                }
                b'"' => {
                    let (s, e) = self.string()?;
                    let raw = String::from_utf8_lossy(&self.b[s..e]).to_string();
                    let value = crate::json::parse(&format!("\"{raw}\"")).ok().and_then(|j| j.as_str().map(str::to_string)).unwrap_or(raw);
                    let line = self.line_of(s);
                    self.out.push(Field {
                        id: self.out.len(), occ: 0,
                        section: path.join("."),
                        key: key.to_string(),
                        value,
                        kind: Kind::Text,
                        help: String::new(),
                        span: (s, e),
                        quoted: true,
                        line,
                    });
                }
                _ => {
                    let s = self.i;
                    while self.i < self.b.len() && !matches!(self.b[self.i], b',' | b'}' | b']') && !(self.b[self.i] as char).is_whitespace() {
                        self.i += 1;
                    }
                    let v = String::from_utf8_lossy(&self.b[s..self.i]).to_string();
                    if v.is_empty() {
                        return None;
                    }
                    let line = self.line_of(s);
                    let kind = if v == "null" { Kind::Text } else { kind_for(&v, "", false) };
                    self.out.push(Field { id: self.out.len(), occ: 0, section: path.join("."), key: key.to_string(), value: v, kind, help: String::new(), span: (s, self.i), quoted: false, line });
                }
            }
            self.depth -= 1;
            Some(())
        }
    }
    let mut p = P { b: text.as_bytes(), i: 0, out: Vec::new(), starts: line_starts(text), depth: 0 };
    if p.value(&[], "").is_none() {
        return Vec::new();
    }
    p.out
}

fn parse_xml(text: &str) -> Vec<Field> {
    // Leaf elements only: <name>value</name>, with the comment before it as help.
    let mut out = Vec::new();
    let starts = line_starts(text);
    let b = text.as_bytes();
    let mut stack: Vec<String> = Vec::new();
    let mut help = String::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'<' {
            i += 1;
            continue;
        }
        if text[i..].starts_with("<!--") {
            let end = text[i..].find("-->").map(|e| i + e + 3).unwrap_or(b.len());
            help = text[i + 4..end.saturating_sub(3).max(i + 4)].trim().to_string();
            i = end;
            continue;
        }
        if text[i..].starts_with("<?") || text[i..].starts_with("<!") {
            i = text[i..].find('>').map(|e| i + e + 1).unwrap_or(b.len());
            continue;
        }
        let Some(close) = text[i..].find('>').map(|e| i + e) else { break };
        let tag = &text[i + 1..close];
        if let Some(name) = tag.strip_prefix('/') {
            let name = name.trim();
            if let Some(p) = stack.iter().rposition(|s| s == name) {
                stack.truncate(p);
            }
            i = close + 1;
            continue;
        }
        let self_closing = tag.ends_with('/');
        let name = tag.split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("").to_string();
        i = close + 1;
        if self_closing || name.is_empty() {
            continue;
        }
        // A leaf: text up to its own closing tag, with no other tag in between.
        let rest = &text[i..];
        let end_tag = format!("</{name}>");
        // Only the next tag is looked at (never a search to the end of the file for each element).
        let lt = rest.find('<');
        match (lt, lt.filter(|&l| rest[l..].starts_with(&end_tag))) {
            (Some(lt), Some(et)) if lt == et => {
                let (s, e) = (i, i + et);
                let raw = &text[s..e];
                let lead = raw.len() - raw.trim_start().len();
                let (s, e) = (s + lead, s + lead + raw.trim().len());
                let value = text[s..e].to_string();
                if !value.contains("<![CDATA[") {
                    let kind = kind_for(&value, &help, false);
                    out.push(Field {
                        id: out.len(), occ: 0,
                        section: stack.join("."),
                        key: name.clone(),
                        value,
                        kind,
                        help: std::mem::take(&mut help),
                        span: (s, e),
                        quoted: true,
                        line: starts.partition_point(|&st| st <= s),
                    });
                }
                i += et + end_tag.len();
            }
            _ => {
                stack.push(name);
                help.clear();
            }
        }
    }
    out
}

// ---- Writing ----

/// The value as it goes into the file, and whether the quotes around it change too (a TOML literal string that now
/// holds a ' becomes a basic string).
fn escape(value: &str, fmt: Format, field: &Field, quote: Option<char>) -> Result<(String, bool), String> {
    if value.contains('\n') || value.contains('\r') {
        return Err(format!("{}: a value can't have a line break", field.key));
    }
    let basic = |v: &str| v.replace('\\', "\\\\").replace('"', "\\\"");
    Ok(match (fmt, quote) {
        (Format::Json, _) if field.quoted => {
            let q = crate::json::quote(value);
            (q[1..q.len() - 1].to_string(), false)
        }
        (Format::Toml | Format::Yaml, Some('"')) => (basic(value), false),
        (Format::Toml, Some('\'')) if value.contains('\'') => (format!("\"{}\"", basic(value)), true),
        (Format::Yaml, Some('\'')) => (value.replace('\'', "''"), false),
        (Format::Xml, _) => (value.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;"), false),
        _ => (value.to_string(), false),
    })
}

/// The file with these settings changed (by field id, as `parse` numbered them in this same text). Only the values'
/// own characters change.
pub fn set(text: &str, fmt: Format, edits: &[(usize, String)]) -> Result<String, String> {
    let fields = parse(text, fmt);
    let mut spans: Vec<((usize, usize), String)> = Vec::new();
    for (id, v) in edits {
        let f = fields.iter().find(|f| f.id == *id).ok_or_else(|| format!("setting {id} isn't in the file any more"))?;
        let q = quote_of(text, f, fmt);
        let (mut val, requote) = escape(v, fmt, f, q)?;
        let mut span = f.span;
        if requote {
            span = (span.0 - 1, span.1 + 1);
        }
        // A JSON number or bool stays bare; text into a bare JSON value that isn't one gets quotes.
        if fmt == Format::Json && !f.quoted && !(is_number(v) || v == "true" || v == "false" || v == "null") {
            val = crate::json::quote(v);
        }
        spans.push((span, val));
    }
    spans.sort_by_key(|s| std::cmp::Reverse(s.0 .0));
    let mut out = text.to_string();
    for ((s, e), v) in spans {
        out.replace_range(s..e, &v);
    }
    Ok(out)
}

/// One change from the page, by the setting's identity (never a position): its section, name and occurrence, the
/// value the page showed, and the new value.
#[derive(Clone, Debug, PartialEq)]
pub struct Edit {
    pub section: String,
    pub key: String,
    pub occ: usize,
    pub was: String,
    pub value: String,
}

/// The file with these changes, matched again by identity in the file as it is now. A setting that is gone, or that
/// changed in the file since the page showed it (the game or a mod update rewrote it), is refused.
pub fn set_by_key(text: &str, fmt: Format, edits: &[Edit]) -> Result<String, String> {
    let fields = parse(text, fmt);
    let mut ids = Vec::new();
    for e in edits {
        let f = fields
            .iter()
            .find(|f| f.section == e.section && f.key == e.key && f.occ == e.occ)
            .ok_or_else(|| format!("{} isn't in the file any more. Open it again.", e.key))?;
        if f.value != e.was {
            return Err(format!("{} changed in the file since this page opened (now {}). Open it again.", e.key, f.value));
        }
        ids.push((f.id, e.value.clone()));
    }
    set(text, fmt, &ids)
}

/// The plain text fallback: one line replaced (1-based).
pub fn set_line(text: &str, line: usize, new: &str) -> Result<String, String> {
    if new.contains('\n') {
        return Err("one line at a time".into());
    }
    let mut lines: Vec<&str> = text.split('\n').collect();
    if line == 0 || line > lines.len() {
        return Err(format!("line {line} isn't in the file"));
    }
    let cr = lines[line - 1].ends_with('\r');
    let owned = if cr { format!("{new}\r") } else { new.to_string() };
    lines[line - 1] = &owned;
    Ok(lines.join("\n"))
}

/// What carrying the player's changes into a mod's new file did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MergeReport {
    pub text: String,
    /// Changed settings Turbine couldn't place for sure in the new file (the player should look at them again), as
    /// the page names them ("[General] Volume", "list 2: on").
    pub unsure: Vec<String>,
    /// Changed settings the new file doesn't have any more.
    pub gone: Vec<String>,
}

/// Keys that name a list item (its identity across versions), in the order they are tried.
const ITEM_IDS: &[&str] = &["id", "uid", "guid", "key", "name", "title", "label", "type"];

/// How the page names a setting to the player.
pub fn setting_name(section: &str, key: &str, occ: usize) -> String {
    let mut n = if section.is_empty() { key.to_string() } else { format!("{section}: {key}") };
    if occ > 0 {
        n.push_str(&format!(" (number {})", occ + 1));
    }
    n
}

/// For each item of `old`, the item of `new` it is in a longest common subsequence. `late`: of several equally
/// good matches, the last one (else the first). None for items left out.
fn align(old: &[&str], new: &[&str], late: bool) -> Vec<Option<usize>> {
    if late {
        let ro: Vec<&str> = old.iter().rev().copied().collect();
        let rn: Vec<&str> = new.iter().rev().copied().collect();
        let m = align(&ro, &rn, false);
        return m.into_iter().rev().map(|j| j.map(|j| new.len() - 1 - j)).collect();
    }
    let (a, b) = (old.len(), new.len());
    let mut dp = vec![vec![0u32; b + 1]; a + 1];
    for i in (0..a).rev() {
        for j in (0..b).rev() {
            dp[i][j] = if old[i] == new[j] { dp[i + 1][j + 1] + 1 } else { dp[i + 1][j].max(dp[i][j + 1]) };
        }
    }
    let mut out = vec![None; a];
    let (mut i, mut j) = (0, 0);
    while i < a && j < b {
        if old[i] == new[j] && dp[i][j] == dp[i + 1][j + 1] + 1 {
            out[i] = Some(j);
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    out
}

/// Where occurrence `occ` of a repeated setting went: matched by the old default's values in order, only when the
/// match is the same whichever way it's read. Err(()) when that can't be told for sure.
fn place_occurrence(old_vals: &[&str], new_vals: &[&str], occ: usize) -> Result<Option<usize>, ()> {
    if old_vals == new_vals {
        return Ok(Some(occ));
    }
    if old_vals.len() > 2000 || new_vals.len() > 2000 {
        return Err(());
    }
    let early = align(old_vals, new_vals, false);
    let late = align(old_vals, new_vals, true);
    match (early.get(occ).copied().flatten(), late.get(occ).copied().flatten()) {
        (Some(x), Some(y)) if x == y => Ok(Some(x)),
        _ => Err(()),
    }
}

/// A list item's path split around its index: ("list", 2, ".rest") for "list.2.rest".
fn split_item(section: &str) -> Option<(String, usize, String)> {
    let parts: Vec<&str> = section.split('.').collect();
    let i = parts.iter().position(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))?;
    let idx = parts[i].parse().ok()?;
    let rest: String = parts[i + 1..].iter().map(|p| format!(".{p}")).collect();
    Some((parts[..i].join("."), idx, rest))
}

/// The identity of item `idx` of list `prefix`: (the identity key, its value), from the first ITEM_IDS key the item
/// has.
fn item_identity(list: &[Field], prefix: &str, idx: usize) -> Option<(String, String)> {
    let sec = format!("{prefix}.{idx}");
    ITEM_IDS.iter().find_map(|k| list.iter().find(|f| f.section == sec && f.key.eq_ignore_ascii_case(k)).map(|f| (f.key.clone(), f.value.clone())))
}

/// Carries the player's changes (`edited` against `old_default`) into `new_default`. Each changed setting is
/// matched by section, name and occurrence; a repeated name by the old default's values in order; a list item by
/// its identity (an id or name key) when the list changed. Anything that can't be placed for sure is left as the
/// new file has it and listed in `unsure`. Running it again changes nothing (values are compared and written
/// unescaped).
pub fn merge_report(fmt: Format, new_default: &str, old_default: &str, edited: &str) -> MergeReport {
    let old = parse(old_default, fmt);
    let mine = parse(edited, fmt);
    let new = parse(new_default, fmt);
    let vals = |list: &[Field], s: &str, k: &str| -> Vec<String> { list.iter().filter(|f| f.section == s && f.key == k).map(|f| f.value.clone()).collect() };
    let items = |list: &[Field], prefix: &str| -> Vec<(String, String, String)> {
        list.iter()
            .filter(|f| f.section == prefix || f.section.starts_with(&format!("{prefix}.")))
            .map(|f| (f.section.clone(), f.key.clone(), f.value.clone()))
            .collect()
    };
    let mut report = MergeReport::default();
    let mut edits: Vec<(usize, String)> = Vec::new();
    for m in &mine {
        let was = old.iter().find(|o| o.section == m.section && o.key == m.key && o.occ == m.occ);
        let Some(was) = was else { continue };
        if was.value == m.value {
            continue;
        }
        let name = setting_name(&m.section, &m.key, m.occ);
        // Where is this setting in the new file?
        let mut section = m.section.clone();
        let key = m.key.clone();
        if let Some((prefix, idx, rest)) = split_item(&m.section) {
            if items(&old, &prefix) != items(&new, &prefix) {
                let Some((id_key, id_val)) = item_identity(&old, &prefix, idx) else {
                    report.unsure.push(name);
                    continue;
                };
                if id_key.eq_ignore_ascii_case(&m.key) && rest.is_empty() {
                    // The player changed the item's identity itself.
                    report.unsure.push(name);
                    continue;
                }
                let same: Vec<usize> = (0..10_000)
                    .take_while(|j| new.iter().any(|f| f.section == format!("{prefix}.{j}") || f.section.starts_with(&format!("{prefix}.{j}."))))
                    .filter(|j| item_identity(&new, &prefix, *j).is_some_and(|(k, v)| k.eq_ignore_ascii_case(&id_key) && v == id_val))
                    .collect();
                match same.as_slice() {
                    [j] => section = format!("{prefix}.{j}{rest}"),
                    [] => {
                        report.gone.push(name);
                        continue;
                    }
                    _ => {
                        report.unsure.push(name);
                        continue;
                    }
                }
            }
        } else if m.section.is_empty() || !m.key.bytes().all(|b| b.is_ascii_digit()) {
            // Not an item of a list of plain values: nothing to remap.
        } else if items(&old, &m.section) != items(&new, &m.section) {
            // A list of plain values that changed: its items have no identity.
            report.unsure.push(name);
            continue;
        }
        let old_vals = vals(&old, &m.section, &m.key);
        let new_vals = vals(&new, &section, &key);
        if new_vals.is_empty() {
            report.gone.push(name);
            continue;
        }
        let ov: Vec<&str> = old_vals.iter().map(String::as_str).collect();
        let nv: Vec<&str> = new_vals.iter().map(String::as_str).collect();
        let occ = if section != m.section {
            // A list item that moved: its own settings keep their occurrence inside it.
            if old_vals.len() == new_vals.len() { Ok(Some(m.occ)) } else { place_occurrence(&ov, &nv, m.occ) }
        } else {
            place_occurrence(&ov, &nv, m.occ)
        };
        let occ = match occ {
            Ok(Some(o)) => o,
            Ok(None) | Err(()) => {
                report.unsure.push(name);
                continue;
            }
        };
        if let Some(n) = new.iter().find(|n| n.section == section && n.key == key && n.occ == occ) {
            if n.value != m.value {
                edits.push((n.id, m.value.clone()));
            }
        }
    }
    report.text = set(new_default, fmt, &edits).unwrap_or_else(|_| new_default.to_string());
    report
}

// ---- A mod's files: Turbine's own copies in the store ----

fn settings_root(store_root: &Path) -> PathBuf {
    store_root.join("settings")
}

/// 0.62: Turbine's copies of one mod's settings files (with what belongs to them) carried over to another mod's
/// entry, when that one has none yet: a new copy of the same mod under a new entry keeps the player's settings.
/// The new mod's own files then count as an update of the old ones (`effective` merges). Answers whether it copied.
pub fn carry_edits(store_root: &Path, from_id: &str, to_id: &str) -> io::Result<bool> {
    fn copy_tree(from: &Path, to: &Path, depth: usize) -> io::Result<()> {
        fs::create_dir_all(to)?;
        for e in fs::read_dir(from)?.flatten() {
            let t = e.file_type()?;
            if t.is_dir() && depth < 12 {
                copy_tree(&e.path(), &to.join(e.file_name()), depth + 1)?;
            } else if t.is_file() {
                fs::copy(e.path(), to.join(e.file_name()))?;
            }
        }
        Ok(())
    }
    if !safe_rel(from_id) || !safe_rel(to_id) || from_id == to_id {
        return Ok(false);
    }
    let (from, to) = (settings_root(store_root).join(from_id), settings_root(store_root).join(to_id));
    if !from.is_dir() || to.exists() {
        return Ok(false);
    }
    copy_tree(&from, &to, 0)?;
    Ok(true)
}

/// Turbine's copy of a mod's config file.
pub fn edited_path(store_root: &Path, mod_id: &str, rel: &str) -> PathBuf {
    settings_root(store_root).join(mod_id).join(rel)
}

fn with_ext(p: &Path, ext: &str) -> PathBuf {
    let mut s = p.as_os_str().to_owned();
    s.push(ext);
    PathBuf::from(s)
}

pub fn base_path(store_root: &Path, mod_id: &str, rel: &str) -> PathBuf {
    with_ext(&edited_path(store_root, mod_id, rel), ".base")
}

pub fn backup_path(store_root: &Path, mod_id: &str, rel: &str) -> PathBuf {
    with_ext(&edited_path(store_root, mod_id, rel), ".backup")
}

/// 0.62: the file as the game or the mod itself last changed it in the game's folder, kept when its changes were
/// merged into Turbine's copy (so nothing is lost without a trace).
pub fn in_place_path(store_root: &Path, mod_id: &str, rel: &str) -> PathBuf {
    with_ext(&edited_path(store_root, mod_id, rel), ".in-place")
}

/// 0.62: Turbine's copy as it was right after in-place changes were taken into it: the baseline for the player's
/// changes in Turbine until the next apply.
pub fn synced_path(store_root: &Path, mod_id: &str, rel: &str) -> PathBuf {
    with_ext(&edited_path(store_root, mod_id, rel), ".synced")
}

/// 0.62: a settings file that was changed where it is deployed (by the game, the mod or by hand) merged with
/// Turbine's own copy. `placed` is the file as Turbine last put it there. A setting the player changed in Turbine
/// since then keeps Turbine's value (also when it was changed in place too); every other change made in place is
/// taken over. Answers the text Turbine's copy should have now. When one of Turbine's changes has no place in the
/// in-place file (its setting is gone there), Turbine's copy stays whole.
pub fn merge_in_place(fmt: Format, in_place: &str, placed: &str, turbine_copy: &str) -> String {
    if in_place_is_damaged(fmt, in_place, placed) {
        return turbine_copy.to_string();
    }
    let (was, mine, theirs) = (parse(placed, fmt), parse(turbine_copy, fmt), parse(in_place, fmt));
    let same = |a: &Field, b: &Field| a.section == b.section && a.key == b.key && a.occ == b.occ;
    let mut edits: Vec<(usize, String)> = Vec::new();
    for m in &mine {
        // Not changed in Turbine since it was placed: the in-place file's value stands.
        if was.iter().find(|w| same(w, m)).is_some_and(|w| w.value == m.value) {
            continue;
        }
        match theirs.iter().find(|t| same(t, m)) {
            Some(t) if t.value == m.value => {}
            Some(t) => edits.push((t.id, m.value.clone())),
            None => return turbine_copy.to_string(),
        }
    }
    set(in_place, fmt, &edits).unwrap_or_else(|_| turbine_copy.to_string())
}

/// 0.62: is the file in the game's folder damaged rather than changed? Empty, or with no settings at all, or with
/// fewer than half the settings Turbine placed (a crash while the game wrote it, a full disk). Such a file is
/// never taken over: Turbine's copy stays and goes back in.
pub fn in_place_is_damaged(fmt: Format, in_place: &str, placed: &str) -> bool {
    let (theirs, was) = (parse(in_place, fmt).len(), parse(placed, fmt).len());
    was > 0 && (in_place.trim().is_empty() || theirs == 0 || theirs * 2 < was)
}

/// A path inside a mod: plain parts only.
pub fn safe_rel(rel: &str) -> bool {
    !rel.is_empty() && !rel.starts_with('/') && rel.split('/').all(|p| !p.is_empty() && p != "." && p != "..") && !rel.contains('\\') && !rel.contains('\0')
}

/// The file the store deploys for a mod's file: Turbine's edited copy when the player changed it (brought up to date
/// with a new default from a mod update first), otherwise the mod's own.
pub fn effective(store_root: &Path, mod_id: &str, rel: &str, default_src: &Path) -> PathBuf {
    let edited = edited_path(store_root, mod_id, rel);
    if !edited.is_file() || format_of(rel).is_none() {
        return default_src.to_path_buf();
    }
    let base = base_path(store_root, mod_id, rel);
    if let (Ok(new_default), Ok(old_default)) = (fs::read_to_string(default_src), fs::read_to_string(&base)) {
        if new_default != old_default {
            if let (Some(fmt), Ok(mine)) = (format_of(rel), fs::read_to_string(&edited)) {
                let report = merge_report(fmt, &new_default, &old_default, &mine);
                let _ = fs::copy(&edited, backup_path(store_root, mod_id, rel));
                // Anything not carried over for sure: the player's whole file before the update is kept, and the
                // settings to look at again are listed for the page.
                let review = review_path(store_root, mod_id, rel);
                if report.unsure.is_empty() && report.gone.is_empty() {
                    let _ = fs::remove_file(&review);
                } else {
                    let _ = fs::copy(&edited, before_update_path(store_root, mod_id, rel));
                    let lines: Vec<String> = report.unsure.iter().map(|n| format!("look\t{n}")).chain(report.gone.iter().map(|n| format!("gone\t{n}"))).collect();
                    let _ = fs::write(&review, lines.join("\n") + "\n");
                }
                let tmp = with_ext(&edited, ".tmp");
                if fs::write(&tmp, report.text).and_then(|_| fs::rename(&tmp, &edited)).is_ok() {
                    let _ = fs::write(&base, new_default);
                }
            }
        }
    }
    edited
}

/// The player's file as it was before a mod update, kept when not every change could be carried over for sure.
pub fn before_update_path(store_root: &Path, mod_id: &str, rel: &str) -> PathBuf {
    with_ext(&edited_path(store_root, mod_id, rel), ".before-update")
}

/// The list of settings to look at again after a mod update (one "look\t<name>" or "gone\t<name>" per line).
pub fn review_path(store_root: &Path, mod_id: &str, rel: &str) -> PathBuf {
    with_ext(&edited_path(store_root, mod_id, rel), ".review")
}

/// The settings to look at again after the mod's last update: (settings Turbine couldn't place for sure, settings the
/// new version doesn't have). Both empty when there is nothing to look at.
pub fn review_notes(store_root: &Path, mod_id: &str, rel: &str) -> (Vec<String>, Vec<String>) {
    let text = fs::read_to_string(review_path(store_root, mod_id, rel)).unwrap_or_default();
    let mut look = Vec::new();
    let mut gone = Vec::new();
    for l in text.lines() {
        match l.split_once('\t') {
            Some(("look", n)) => look.push(n.to_string()),
            Some(("gone", n)) => gone.push(n.to_string()),
            _ => {}
        }
    }
    (look, gone)
}

/// The words for the page and the mod's row: which settings need a second look after the mod updated.
pub fn review_words(mod_name: &str, look: &[String], gone: &[String]) -> Option<String> {
    if look.is_empty() && gone.is_empty() {
        return None;
    }
    let mut t = format!("{mod_name} updated. Turbine kept your changes where it was sure.");
    if !look.is_empty() {
        t.push_str(&format!(" Please check these settings again, the new version moved things around: {}.", look.join(", ")));
    }
    if !gone.is_empty() {
        t.push_str(&format!(" The new version no longer has: {}.", gone.join(", ")));
    }
    t.push_str(" Your file from before the update is kept.");
    Some(t)
}

/// The player looked at them (or saved the page): the list goes.
pub fn clear_review(store_root: &Path, mod_id: &str, rel: &str) {
    let _ = fs::remove_file(review_path(store_root, mod_id, rel));
}

/// Saves the player's version of a mod's file (the copy before it kept as .backup).
pub fn save_edit(store_root: &Path, mod_id: &str, rel: &str, default_src: &Path, text: &str) -> io::Result<()> {
    if !safe_rel(rel) || !safe_rel(mod_id) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a plain path"));
    }
    let edited = edited_path(store_root, mod_id, rel);
    if let Some(p) = edited.parent() {
        fs::create_dir_all(p)?;
    }
    let before = if edited.is_file() { edited.clone() } else { default_src.to_path_buf() };
    let _ = fs::copy(&before, backup_path(store_root, mod_id, rel));
    let tmp = with_ext(&edited, ".tmp");
    fs::write(&tmp, text)?;
    fs::rename(&tmp, &edited)?;
    fs::copy(default_src, base_path(store_root, mod_id, rel))?;
    Ok(())
}

/// Reset to default: the mod's own file again (the player's version kept as .backup).
pub fn reset(store_root: &Path, mod_id: &str, rel: &str) -> io::Result<bool> {
    let edited = edited_path(store_root, mod_id, rel);
    if !edited.is_file() {
        return Ok(false);
    }
    fs::rename(&edited, backup_path(store_root, mod_id, rel))?;
    let _ = fs::remove_file(base_path(store_root, mod_id, rel));
    Ok(true)
}

/// The config files in a folder (a mod's staged files or the game's), at most `limit`, small enough to read.
pub fn config_files(dir: &Path, max_depth: usize, limit: usize) -> Vec<String> {
    fn walk(base: &Path, dir: &Path, depth: usize, max: usize, limit: usize, out: &mut Vec<String>) {
        if out.len() >= limit || depth > max {
            return;
        }
        let Ok(rd) = fs::read_dir(dir) else { return };
        let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            let Ok(m) = fs::symlink_metadata(&p) else { continue };
            if m.is_dir() {
                walk(base, &p, depth + 1, max, limit, out);
            } else if m.is_file() && m.len() <= MAX_BYTES {
                let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                if format_of(&name).is_some() {
                    if let Ok(rel) = p.strip_prefix(base) {
                        let rel: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
                        out.push(rel.join("/"));
                    }
                }
            }
            if out.len() >= limit {
                return;
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, 0, max_depth, limit, &mut out);
    out
}

// ---- The game's own config files (Game menu, Edit settings) ----
//
// Edited in place, only when the player saves; the first original is kept in the store (`settings-game/<path>.original`)
// and the copy before each save as `.backup`. Reset to default puts the original back.

pub fn game_original(store_root: &Path, rel: &str) -> PathBuf {
    with_ext(&store_root.join("settings-game").join(rel), ".original")
}

pub fn game_backup(store_root: &Path, rel: &str) -> PathBuf {
    with_ext(&store_root.join("settings-game").join(rel), ".backup")
}

pub fn save_game_file(store_root: &Path, game_dir: &Path, rel: &str, text: &str) -> io::Result<()> {
    if !safe_rel(rel) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a plain path"));
    }
    let file = game_dir.join(rel);
    let orig = game_original(store_root, rel);
    if let Some(p) = orig.parent() {
        fs::create_dir_all(p)?;
    }
    if !orig.exists() {
        fs::copy(&file, &orig)?;
    }
    fs::copy(&file, game_backup(store_root, rel))?;
    let tmp = with_ext(&file, ".turbine-tmp");
    fs::write(&tmp, text)?;
    fs::rename(&tmp, &file)
}

fn build_file(store_root: &Path, rel: &str) -> PathBuf {
    with_ext(&store_root.join("settings-game").join(rel), ".build")
}

/// `save_game_file`, noting the game's build the original was kept at.
pub fn save_game_file_at(store_root: &Path, game_dir: &Path, rel: &str, text: &str, build: Option<&str>) -> io::Result<()> {
    let first = !game_original(store_root, rel).exists();
    save_game_file(store_root, game_dir, rel, text)?;
    if first {
        let _ = fs::write(build_file(store_root, rel), build.unwrap_or(""));
    }
    Ok(())
}

/// `reset_game_file`, refused when the game updated since the original was kept (it would put an old file back):
/// Steam's Verify integrity is the way then.
pub fn reset_game_file_at(store_root: &Path, game_dir: &Path, rel: &str, build: Option<&str>) -> io::Result<bool> {
    if let (Ok(then), Some(now)) = (fs::read_to_string(build_file(store_root, rel)), build) {
        if !then.is_empty() && then != now {
            return Err(io::Error::other(format!(
                "the game updated since Turbine kept its original {rel}, so putting that back could break it. Use Steam's Verify integrity of game files instead"
            )));
        }
    }
    reset_game_file(store_root, game_dir, rel)
}

pub fn reset_game_file(store_root: &Path, game_dir: &Path, rel: &str) -> io::Result<bool> {
    let orig = game_original(store_root, rel);
    if !safe_rel(rel) || !orig.is_file() {
        return Ok(false);
    }
    let file = game_dir.join(rel);
    let _ = fs::copy(&file, game_backup(store_root, rel));
    fs::copy(&orig, &file)?;
    Ok(true)
}

// ---- For the page ----

fn num_json(v: Option<f64>) -> String {
    v.map(|x| format!("{x}")).unwrap_or_else(|| "null".into())
}

/// The page's JSON: [{"id","section","key","value","kind","on","off","min","max","step","options":[{"value","label"}],
/// "help","line"}].
pub fn fields_json(fields: &[Field]) -> String {
    use crate::json::quote;
    let items: Vec<String> = fields
        .iter()
        .map(|f| {
            let (kind, extra) = match &f.kind {
                Kind::Toggle { on, off } => ("toggle", format!(",\"on\":{},\"off\":{}", quote(on), quote(off))),
                Kind::Number { min, max, step } => ("number", format!(",\"min\":{},\"max\":{},\"step\":{}", num_json(*min), num_json(*max), step)),
                Kind::List(o) => (
                    "list",
                    format!(",\"options\":[{}]", o.iter().map(|(v, l)| format!("{{\"value\":{},\"label\":{}}}", quote(v), quote(l))).collect::<Vec<_>>().join(",")),
                ),
                Kind::Text => ("text", String::new()),
            };
            format!(
                "{{\"id\":{},\"occ\":{},\"section\":{},\"key\":{},\"value\":{},\"kind\":\"{kind}\"{extra},\"help\":{},\"line\":{}}}",
                f.id,
                f.occ,
                quote(&f.section),
                quote(&f.key),
                quote(&f.value),
                quote(&f.help),
                f.line
            )
        })
        .collect();
    format!("[{}]", items.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ERSC: &str = "[GAMEPLAY]\r\n\r\n; Invaders are other players that will join your world uninvited and try to kill you and your party.  0=FALSE  1=TRUE\r\nallow_invaders = 1\r\n\r\n; 0 = Normal | 1 = None | 2 = Display player ping | 3 = Display player soul level | 4 = Display player death count | 5 = Soul level AND ping\r\noverhead_player_display = 0\r\n\r\n; 0 = Spectate only during boss fights and invasion encounters | 1 = Always spectate other players on death until party wipeout or bonfire rest\r\nalways_spectate_on_death = 0 \r\n\r\n; Game volume before initial save load. 0 = MUTE 10 = MAX\r\ndefault_boot_master_volume = 5\r\n\r\n[SCALING]\r\n\r\n; Amount of enemy health (%) per player for each enemy. (Default: 35 = 35% more enemy health per player)\r\nenemy_health_scaling = 35\r\n\r\n[PASSWORD]\r\n\r\n; Session password\r\ncooppassword = \r\n\r\n[SAVE]\r\n\r\n;Your save file extension (in the vanilla game this is .sl2). Use any alphanumeric characters (limit = 120)\r\nsave_file_extension = co2\r\n";

    fn by_key<'a>(f: &'a [Field], k: &str) -> &'a Field {
        f.iter().find(|x| x.key == k).unwrap_or_else(|| panic!("{k} in {f:?}"))
    }

    #[test]
    fn seamless_settings() {
        let f = parse(ERSC, Format::Ini);
        assert_eq!(by_key(&f, "allow_invaders").kind, Kind::Toggle { on: "1".into(), off: "0".into() });
        assert_eq!(by_key(&f, "allow_invaders").section, "GAMEPLAY");
        match &by_key(&f, "overhead_player_display").kind {
            Kind::List(o) => {
                assert_eq!(o.len(), 6, "{o:?}");
                assert_eq!(o[2], ("2".to_string(), "Display player ping".to_string()));
            }
            k => panic!("{k:?}"),
        }
        assert!(matches!(&by_key(&f, "always_spectate_on_death").kind, Kind::List(o) if o.len() == 2), "{:?}", by_key(&f, "always_spectate_on_death"));
        assert_eq!(by_key(&f, "always_spectate_on_death").value, "0");
        assert_eq!(by_key(&f, "default_boot_master_volume").kind, Kind::Number { min: Some(0.0), max: Some(10.0), step: 1.0 });
        assert_eq!(by_key(&f, "enemy_health_scaling").kind, Kind::Number { min: Some(0.0), max: None, step: 5.0 });
        let pw = by_key(&f, "cooppassword");
        assert_eq!((pw.value.as_str(), &pw.kind, pw.section.as_str()), ("", &Kind::Text, "PASSWORD"));
        assert_eq!(by_key(&f, "save_file_extension").kind, Kind::Text);
        assert!(by_key(&f, "allow_invaders").help.starts_with("Invaders are other players"));

        // Edits change only the values; everything else (CRLF, comments, the trailing space) stays.
        let t = set(ERSC, Format::Ini, &[(pw.id, "dragon".into()), (by_key(&f, "allow_invaders").id, "0".into()), (by_key(&f, "default_boot_master_volume").id, "7".into())]).unwrap();
        assert!(t.contains("cooppassword = dragon\r\n"), "{t}");
        assert!(t.contains("allow_invaders = 0\r\n"));
        assert!(t.contains("default_boot_master_volume = 7\r\n"));
        assert!(t.contains("always_spectate_on_death = 0 \r\n"));
        assert_eq!(t.len(), ERSC.len() + "dragon".len());
        assert_eq!(parse(&t, Format::Ini).len(), f.len());
        assert!(set(ERSC, Format::Ini, &[(pw.id, "a\nb".into())]).is_err());
    }

    #[test]
    fn config_generator_cfg() {
        let cfg = "## Settings file was created by plugin Bigger Pockets v1.0.3\n\n[General]\n\n## Turn the extra rows on\n# Setting type: Boolean\n# Default value: true\nEnabled = true\n\n## How many rows\n# Setting type: Int32\n# Default value: 2\n# Acceptable value range: From 1 to 6\nRows = 2\n\n## Where\n# Setting type: Where\n# Default value: Left\n# Acceptable values: Left, Right, Top\nSide = Left\n\nColor = #FFAA00\n";
        let f = parse(cfg, Format::Ini);
        assert_eq!(by_key(&f, "Enabled").kind, Kind::Toggle { on: "true".into(), off: "false".into() });
        assert_eq!(by_key(&f, "Rows").kind, Kind::Number { min: Some(1.0), max: Some(6.0), step: 1.0 });
        assert!(matches!(&by_key(&f, "Side").kind, Kind::List(o) if o.len() == 3));
        assert_eq!(by_key(&f, "Color").value, "#FFAA00");
        assert_eq!(by_key(&f, "Rows").help, "How many rows Acceptable value range: From 1 to 6");
    }

    #[test]
    fn toml_yaml_json_xml() {
        let toml = "# Speed of the thing\nspeed = 1.5 # times\nname = \"Ro\\\"ach\"\nlist = [1, 2]\n[ui]\nshow = false\n";
        let f = parse(toml, Format::Toml);
        assert_eq!(by_key(&f, "speed").kind, Kind::Number { min: Some(0.0), max: None, step: 0.1 });
        assert_eq!(by_key(&f, "speed").help, "Speed of the thing times");
        assert_eq!(by_key(&f, "name").value, "Ro\"ach");
        assert_eq!(by_key(&f, "show").section, "ui");
        assert_eq!(by_key(&f, "list").kind, Kind::Text);
        let t = set(toml, Format::Toml, &[(by_key(&f, "name").id, "say \"hi\"".into()), (by_key(&f, "show").id, "true".into())]).unwrap();
        assert!(t.contains("name = \"say \\\"hi\\\"\"\n"), "{t}");
        assert!(t.contains("show = true\n"));

        let yaml = "# top\nwindow:\n  # Width in pixels\n  width: 1280\n  title: \"Hi: there\"\n  full: yes\nitems:\n  - a\n";
        let f = parse(yaml, Format::Yaml);
        assert_eq!(by_key(&f, "width").section, "window");
        assert_eq!(by_key(&f, "width").help, "Width in pixels");
        assert_eq!(by_key(&f, "title").value, "Hi: there");
        assert_eq!(by_key(&f, "full").kind, Kind::Toggle { on: "yes".into(), off: "no".into() });
        let t = set(yaml, Format::Yaml, &[(by_key(&f, "width").id, "800".into())]).unwrap();
        assert!(t.contains("  width: 800\n"));

        let json = "{\n  \"volume\": 0.8,\n  \"name\": \"a\\\"b\",\n  \"keys\": {\"jump\": \"Space\", \"on\": true},\n  \"list\": [1, 2]\n}";
        let f = parse(json, Format::Json);
        assert_eq!(f.len(), 6, "{f:?}");
        assert_eq!(by_key(&f, "jump").section, "keys");
        assert_eq!(by_key(&f, "name").value, "a\"b");
        assert_eq!(by_key(&f, "on").kind, Kind::Toggle { on: "true".into(), off: "false".into() });
        let t = set(json, Format::Json, &[(by_key(&f, "name").id, "c\"d".into()), (by_key(&f, "volume").id, "1".into())]).unwrap();
        assert!(crate::json::parse(&t).is_ok(), "{t}");
        assert!(t.contains("\"name\": \"c\\\"d\""));
        assert!(t.contains("\"volume\": 1,"));
        assert!(parse("{ broken", Format::Json).is_empty());

        let xml = "<?xml version=\"1.0\"?>\n<config>\n  <!-- Draw distance -->\n  <distance>300</distance>\n  <name>A &amp; B</name>\n  <group><on>true</on></group>\n</config>\n";
        let f = parse(xml, Format::Xml);
        assert_eq!(by_key(&f, "distance").help, "Draw distance");
        assert_eq!(by_key(&f, "on").section, "config.group");
        let t = set(xml, Format::Xml, &[(by_key(&f, "distance").id, "500".into()), (by_key(&f, "name").id, "C<D".into())]).unwrap();
        assert!(t.contains("<distance>500</distance>") && t.contains("<name>C&lt;D</name>"), "{t}");
    }

    #[test]
    fn values_carry_over_to_a_new_default() {
        let old = "[a]\nx = 1\ny = 2\nz = 3\n";
        let mine = "[a]\nx = 1\ny = 9\nz = 3\n";
        let new = "[a]\n; new help\nx = 5\ny = 2\nw = 4\n";
        assert_eq!(merge_report(Format::Ini, new, old, mine).text, "[a]\n; new help\nx = 5\ny = 9\nw = 4\n");
    }

    #[test]
    fn files_and_reset() {
        let d = std::env::temp_dir().join(format!("turbine-settings-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        let store = d.join("store");
        let def = d.join("mods/m1/BepInEx/config/a.cfg");
        fs::create_dir_all(def.parent().unwrap()).unwrap();
        fs::write(&def, "[a]\nx = 1\ny = 2\n").unwrap();
        fs::write(def.with_file_name("readme.txt"), "x").unwrap();
        assert_eq!(config_files(&d.join("mods/m1"), 6, 40), vec!["BepInEx/config/a.cfg".to_string()]);
        let rel = "BepInEx/config/a.cfg";
        assert_eq!(effective(&store, "m1", rel, &def), def);
        save_edit(&store, "m1", rel, &def, "[a]\nx = 1\ny = 7\n").unwrap();
        assert_eq!(effective(&store, "m1", rel, &def), edited_path(&store, "m1", rel));
        assert_eq!(fs::read_to_string(backup_path(&store, "m1", rel)).unwrap(), "[a]\nx = 1\ny = 2\n");
        // A mod update: the player's y carries over into the new file.
        fs::write(&def, "[a]\nx = 1\ny = 2\nz = 3\n").unwrap();
        let p = effective(&store, "m1", rel, &def);
        assert_eq!(fs::read_to_string(&p).unwrap(), "[a]\nx = 1\ny = 7\nz = 3\n");
        assert!(reset(&store, "m1", rel).unwrap());
        assert_eq!(effective(&store, "m1", rel, &def), def);
        assert!(!reset(&store, "m1", rel).unwrap());
        assert!(save_edit(&store, "../x", rel, &def, "").is_err());
        assert!(!safe_rel("a/../b") && safe_rel("a/b.ini"));
        // The game's own file: the first original kept, Reset puts it back.
        let game = d.join("game");
        fs::create_dir_all(&game).unwrap();
        fs::write(game.join("game.ini"), "a = 1\n").unwrap();
        save_game_file(&store, &game, "game.ini", "a = 2\n").unwrap();
        save_game_file(&store, &game, "game.ini", "a = 3\n").unwrap();
        assert_eq!(fs::read_to_string(game_original(&store, "game.ini")).unwrap(), "a = 1\n");
        assert_eq!(fs::read_to_string(game_backup(&store, "game.ini")).unwrap(), "a = 2\n");
        assert!(reset_game_file(&store, &game, "game.ini").unwrap());
        assert_eq!(fs::read_to_string(game.join("game.ini")).unwrap(), "a = 1\n");
        // An original kept at another build isn't put back.
        fs::write(game.join("g2.ini"), "b = 1\n").unwrap();
        save_game_file_at(&store, &game, "g2.ini", "b = 2\n", Some("100")).unwrap();
        assert!(reset_game_file_at(&store, &game, "g2.ini", Some("101")).is_err());
        assert!(reset_game_file_at(&store, &game, "g2.ini", Some("100")).unwrap());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn page_json_parses() {
        let f = parse(ERSC, Format::Ini);
        let j = crate::json::parse(&fields_json(&f)).unwrap();
        assert_eq!(j.as_array().len(), f.len());
    }

    #[test]
    fn set_line_keeps_the_rest() {
        assert_eq!(set_line("a\r\nb\r\n", 2, "c").unwrap(), "a\r\nc\r\n");
        assert!(set_line("a", 3, "x").is_err());
    }
}

#[cfg(test)]
mod break_tests {
    use super::*;

    fn key<'a>(f: &'a [Field], k: &str, occ: usize) -> &'a Field {
        f.iter().find(|x| x.key == k && x.occ == occ).unwrap()
    }

    #[test]
    fn no_double_escaping() {
        let toml = "path = \"C:\\\\Games\"\nlit = 'abc'\n";
        let f = parse(toml, Format::Toml);
        assert_eq!(key(&f, "path", 0).value, "C:\\Games");
        let t = set(toml, Format::Toml, &[(key(&f, "path", 0).id, "D:\\Games".into()), (key(&f, "lit", 0).id, "it's".into())]).unwrap();
        assert_eq!(t, "path = \"D:\\\\Games\"\nlit = \"it's\"\n");
        let f2 = parse(&t, Format::Toml);
        assert_eq!((key(&f2, "path", 0).value.as_str(), key(&f2, "lit", 0).value.as_str()), ("D:\\Games", "it's"));
        let xml = "<a><n>Tom &amp; Jerry</n></a>";
        let f = parse(xml, Format::Xml);
        assert_eq!(f[0].value, "Tom & Jerry");
        let t = set(xml, Format::Xml, &[(f[0].id, "Tom & Jerry x".into())]).unwrap();
        assert_eq!(t, "<a><n>Tom &amp; Jerry x</n></a>");
        let yaml = "a: 'it''s'\nb: \"x\\\\y\"\n";
        let f = parse(yaml, Format::Yaml);
        assert_eq!((f[0].value.as_str(), f[1].value.as_str()), ("it's", "x\\y"));
        assert_eq!(set(yaml, Format::Yaml, &[(f[0].id, "it's".into()), (f[1].id, "x\\y".into())]).unwrap(), yaml);
        // Carrying values over twice changes nothing more.
        let old = "p = \"C:\\\\A\"\n";
        let mine = "p = \"C:\\\\B&\"\n";
        let new = "# new\np = \"C:\\\\A\"\n";
        let once = merge_report(Format::Toml, new, old, mine).text;
        assert_eq!(once, "# new\np = \"C:\\\\B&\"\n");
        assert_eq!(merge_report(Format::Toml, new, old, &once).text, once);
        let xold = "<a><n>x</n></a>";
        let xmine = "<a><n>A &amp; B</n></a>";
        let m1 = merge_report(Format::Xml, "<a><n>x</n><m>1</m></a>", xold, xmine).text;
        assert_eq!(m1, "<a><n>A &amp; B</n><m>1</m></a>");
        assert_eq!(merge_report(Format::Xml, "<a><n>x</n><m>1</m></a>", xold, &m1).text, m1);
    }

    #[test]
    fn identity_not_position() {
        // Repeated names: the player's second plugin stays the second.
        let old = "[p]\nplugin = a\nplugin = b\n";
        let mine = "[p]\nplugin = a\nplugin = z\n";
        let new = "[p]\n; help\nplugin = a\nplugin = b\n";
        assert_eq!(merge_report(Format::Ini, new, old, mine).text, "[p]\n; help\nplugin = a\nplugin = z\n");
        // A JSON array that changed shape: the player's value isn't moved onto another item.
        let old = "{\"l\": [{\"n\": \"x\", \"on\": true}, {\"n\": \"y\", \"on\": true}]}";
        let mine = "{\"l\": [{\"n\": \"x\", \"on\": true}, {\"n\": \"y\", \"on\": false}]}";
        let new = "{\"l\": [{\"n\": \"w\", \"on\": true}, {\"n\": \"x\", \"on\": true}, {\"n\": \"y\", \"on\": true}]}";
        assert_eq!(merge_report(Format::Json, new, old, mine).text, new);
        // The same array: carried.
        let new2 = "{\"l\": [{\"n\": \"x\", \"on\": true}, {\"n\": \"y\", \"on\": true}], \"v\": 2}";
        assert!(merge_report(Format::Json, new2, old, mine).text.contains("{\"n\": \"y\", \"on\": false}"));
        // Saves match by name again and refuse a value that changed meanwhile.
        let text = "[a]\nx = 1\ny = 2\n";
        let e = |k: &str, was: &str, v: &str| Edit { section: "a".into(), key: k.into(), occ: 0, was: was.into(), value: v.into() };
        assert_eq!(set_by_key(text, Format::Ini, &[e("y", "2", "5")]).unwrap(), "[a]\nx = 1\ny = 5\n");
        let moved = "[a]\nnew = 0\nx = 1\ny = 2\n";
        assert_eq!(set_by_key(moved, Format::Ini, &[e("y", "2", "5")]).unwrap(), "[a]\nnew = 0\nx = 1\ny = 5\n");
        assert!(set_by_key("[a]\nx = 1\ny = 3\n", Format::Ini, &[e("y", "2", "5")]).unwrap_err().contains("changed"));
        assert!(set_by_key("[a]\nx = 1\n", Format::Ini, &[e("y", "2", "5")]).unwrap_err().contains("isn't in the file"));
    }

    #[test]
    fn bom_and_big_files() {
        let f = parse("\u{feff}[GAMEPLAY]\na = 1\n", Format::Ini);
        assert_eq!(f[0].section, "GAMEPLAY");
        let t = set("\u{feff}[G]\na = 1\n", Format::Ini, &[(0, "2".into())]).unwrap();
        assert_eq!(t, "\u{feff}[G]\na = 2\n");
        // Deeply nested XML is read in linear time; past the limit nothing is parsed.
        let deep = "<b>".repeat(60_000) + &"</b>".repeat(60_000);
        let t0 = std::time::Instant::now();
        let _ = parse(&deep[..PARSE_MAX.min(deep.len())], Format::Xml);
        assert!(t0.elapsed().as_secs() < 3, "{:?}", t0.elapsed());
        assert!(parse(&"a = 1\n".repeat(60_000), Format::Ini).is_empty());
    }
}
