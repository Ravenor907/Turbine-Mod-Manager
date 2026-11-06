//! A small, safe XML reader for installer files inside mod archives (FOMOD's ModuleConfig.xml and info.xml).
//!
//! It reads elements, attributes, text, the five predefined entities, numeric character references, CDATA and
//! comments. It never reads a DTD: a DOCTYPE declaration is skipped whole, and no entity it declares is ever expanded,
//! so nothing outside the file is ever fetched or read. Size, depth and node count are limited.
//! Mod authors write these files by hand, so it is forgiving where that is safe: an '&' that doesn't start a known
//! entity is kept as it is, and a missing closing tag at the very end closes the open elements.

use std::fmt;

/// Largest file accepted, in bytes (after decoding).
pub const MAX_SIZE: usize = 8 << 20;
/// Deepest nesting accepted.
pub const MAX_DEPTH: usize = 64;
/// Most elements and text pieces in one file.
pub const MAX_NODES: usize = 200_000;

#[derive(Debug, Clone, PartialEq)]
pub struct XmlError {
    pub what: String,
    /// Line in the file (from 1).
    pub line: usize,
}

impl fmt::Display for XmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (line {})", self.what, self.line)
    }
}

impl std::error::Error for XmlError {}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Element(Element),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Element {
    /// The name as written (with any prefix, e.g. "xsi:foo").
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Node>,
}

fn local(name: &str) -> &str {
    name.rsplit_once(':').map(|(_, l)| l).unwrap_or(name)
}

impl Element {
    /// Name without a namespace prefix.
    pub fn local_name(&self) -> &str {
        local(&self.name)
    }

    /// Is this element called `name` (namespace prefix and case ignored)?
    pub fn is(&self, name: &str) -> bool {
        self.local_name().eq_ignore_ascii_case(name)
    }

    /// An attribute's value (name matched ignoring case and prefix).
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.iter().find(|(k, _)| local(k).eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }

    /// Child elements, in order.
    pub fn elements(&self) -> impl Iterator<Item = &Element> {
        self.children.iter().filter_map(|n| match n {
            Node::Element(e) => Some(e),
            Node::Text(_) => None,
        })
    }

    /// Child elements with this name.
    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Element> + 'a {
        self.elements().filter(move |e| e.is(name))
    }

    /// The first child element with this name.
    pub fn child(&self, name: &str) -> Option<&Element> {
        self.elements().find(|e| e.is(name))
    }

    /// All text inside, joined (including text of child elements), trimmed.
    pub fn text(&self) -> String {
        fn collect(e: &Element, out: &mut String) {
            for n in &e.children {
                match n {
                    Node::Text(t) => out.push_str(t),
                    Node::Element(c) => collect(c, out),
                }
            }
        }
        let mut s = String::new();
        collect(self, &mut s);
        s.trim().to_string()
    }
}

/// Turns file bytes into text: UTF-8 (with or without BOM), UTF-16 LE or BE (with BOM, or detected from the
/// leading '<'), and Windows-1252 for old files that aren't valid UTF-8.
pub fn decode(bytes: &[u8]) -> Result<String, XmlError> {
    let err = |what: &str| XmlError { what: what.into(), line: 1 };
    if bytes.len() > MAX_SIZE * 2 {
        return Err(err("the file is too large"));
    }
    let utf16 = |b: &[u8], le: bool| -> Result<String, XmlError> {
        if !b.len().is_multiple_of(2) {
            return Err(err("the file is cut off (odd length for UTF-16)"));
        }
        let units: Vec<u16> =
            b.chunks_exact(2).map(|c| if le { u16::from_le_bytes([c[0], c[1]]) } else { u16::from_be_bytes([c[0], c[1]]) }).collect();
        String::from_utf16(&units).map_err(|_| err("the file has broken UTF-16 text"))
    };
    match bytes {
        [0xEF, 0xBB, 0xBF, rest @ ..] => Ok(String::from_utf8_lossy(rest).into_owned()),
        [0xFF, 0xFE, rest @ ..] => utf16(rest, true),
        [0xFE, 0xFF, rest @ ..] => utf16(rest, false),
        [b'<', 0, ..] => utf16(bytes, true),
        [0, b'<', ..] => utf16(bytes, false),
        _ => match std::str::from_utf8(bytes) {
            Ok(s) => Ok(s.to_string()),
            Err(_) => Ok(bytes.iter().map(|&b| cp1252(b)).collect()),
        },
    }
}

/// Windows-1252 to Unicode.
pub fn cp1252(b: u8) -> char {
    const HIGH: [u16; 32] = [
        0x20AC, 0x81, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039, 0x0152, 0x8D, 0x017D,
        0x8F, 0x90, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014, 0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0x9D,
        0x017E, 0x0178,
    ];
    if (0x80..0xA0).contains(&b) {
        char::from_u32(HIGH[(b - 0x80) as usize] as u32).unwrap_or('?')
    } else {
        b as char
    }
}

/// Reads bytes as XML (see `decode`) and returns the root element.
pub fn parse_bytes(bytes: &[u8]) -> Result<Element, XmlError> {
    parse(&decode(bytes)?)
}

/// Reads XML text and returns the root element.
pub fn parse(text: &str) -> Result<Element, XmlError> {
    if text.len() > MAX_SIZE {
        return Err(XmlError { what: "the file is too large".into(), line: 1 });
    }
    let mut p = Parser { s: text.as_bytes(), text, i: 0, nodes: 0 };
    p.document()
}

struct Parser<'a> {
    s: &'a [u8],
    text: &'a str,
    i: usize,
    nodes: usize,
}

fn is_name_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.' | b':') || c >= 0x80
}

impl Parser<'_> {
    fn err<T>(&self, what: impl Into<String>) -> Result<T, XmlError> {
        let line = 1 + self.s[..self.i.min(self.s.len())].iter().filter(|&&c| c == b'\n').count();
        Err(XmlError { what: what.into(), line })
    }

    fn starts(&self, pat: &str) -> bool {
        self.s[self.i..].starts_with(pat.as_bytes())
    }

    fn skip_ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    /// Moves past the next `end`, or fails.
    fn skip_past(&mut self, end: &str, what: &str) -> Result<(), XmlError> {
        match find(&self.s[self.i..], end.as_bytes()) {
            Some(at) => {
                self.i += at + end.len();
                Ok(())
            }
            None => self.err(format!("{what} is never closed")),
        }
    }

    fn count(&mut self) -> Result<(), XmlError> {
        self.nodes += 1;
        if self.nodes > MAX_NODES {
            return self.err("the file has too many parts");
        }
        Ok(())
    }

    /// Skips the prolog and anything that isn't content: declarations, comments, processing instructions, DOCTYPE.
    fn skip_misc(&mut self) -> Result<(), XmlError> {
        loop {
            self.skip_ws();
            if self.starts("<?") {
                self.skip_past("?>", "a processing instruction")?;
            } else if self.starts("<!--") {
                self.skip_past("-->", "a comment")?;
            } else if self.starts("<!DOCTYPE") || self.starts("<!doctype") {
                self.skip_doctype()?;
            } else {
                return Ok(());
            }
        }
    }

    /// Skips a DOCTYPE declaration, internal subset included, without reading anything in it.
    fn skip_doctype(&mut self) -> Result<(), XmlError> {
        let mut depth = 0usize;
        let mut quote: Option<u8> = None;
        while self.i < self.s.len() {
            let c = self.s[self.i];
            self.i += 1;
            match quote {
                Some(q) if c == q => quote = None,
                Some(_) => {}
                None => match c {
                    b'"' | b'\'' => quote = Some(c),
                    b'[' => depth += 1,
                    b']' => depth = depth.saturating_sub(1),
                    b'>' if depth == 0 => return Ok(()),
                    _ => {}
                },
            }
        }
        self.err("a DOCTYPE is never closed")
    }

    fn document(&mut self) -> Result<Element, XmlError> {
        self.skip_misc()?;
        if !self.starts("<") {
            return self.err("this isn't an XML file");
        }
        let root = self.element(0)?;
        self.skip_misc()?;
        Ok(root)
    }

    fn name(&mut self) -> Result<String, XmlError> {
        let start = self.i;
        while self.i < self.s.len() && is_name_char(self.s[self.i]) {
            self.i += 1;
        }
        if start == self.i {
            return self.err("a tag has no name");
        }
        Ok(self.text[start..self.i].to_string())
    }

    /// Reads an element starting at '<'.
    fn element(&mut self, depth: usize) -> Result<Element, XmlError> {
        if depth >= MAX_DEPTH {
            return self.err("the file is nested too deeply");
        }
        self.count()?;
        self.i += 1; // '<'
        let name = self.name()?;
        let mut el = Element { name, attrs: Vec::new(), children: Vec::new() };
        loop {
            self.skip_ws();
            if self.i >= self.s.len() {
                return self.err(format!("the tag <{}> is never closed", el.name));
            }
            if self.starts("/>") {
                self.i += 2;
                return Ok(el);
            }
            if self.s[self.i] == b'>' {
                self.i += 1;
                break;
            }
            let key = self.name()?;
            self.skip_ws();
            if self.i >= self.s.len() || self.s[self.i] != b'=' {
                // An attribute without a value (not valid XML, but harmless): keep it empty.
                el.attrs.push((key, String::new()));
                continue;
            }
            self.i += 1;
            self.skip_ws();
            let q = match self.s.get(self.i) {
                Some(&c @ (b'"' | b'\'')) => c,
                _ => return self.err(format!("the value of {key} isn't in quotes")),
            };
            self.i += 1;
            let start = self.i;
            while self.i < self.s.len() && self.s[self.i] != q {
                if self.s[self.i] == b'<' {
                    return self.err(format!("the value of {key} has a '<' in it"));
                }
                self.i += 1;
            }
            if self.i >= self.s.len() {
                return self.err(format!("the value of {key} is never closed"));
            }
            let raw = &self.text[start..self.i];
            self.i += 1;
            let value = unescape(raw);
            // Attribute values: line breaks and tabs read as spaces, as XML says.
            let value: String = value.chars().map(|c| if matches!(c, '\n' | '\r' | '\t') { ' ' } else { c }).collect();
            if !el.attrs.iter().any(|(k, _)| *k == key) {
                el.attrs.push((key, value));
            }
        }
        // Content.
        let mut text = String::new();
        loop {
            if self.i >= self.s.len() {
                // Forgiving: a file cut off at the end closes what's open.
                flush(&mut text, &mut el);
                return Ok(el);
            }
            if self.starts("</") {
                flush(&mut text, &mut el);
                self.i += 2;
                let close = self.name()?;
                self.skip_ws();
                if self.s.get(self.i) != Some(&b'>') {
                    return self.err(format!("the closing tag </{close}> is broken"));
                }
                self.i += 1;
                if !close.eq_ignore_ascii_case(&el.name) {
                    return self.err(format!("<{}> is closed by </{close}>", el.name));
                }
                return Ok(el);
            }
            if self.starts("<!--") {
                self.skip_past("-->", "a comment")?;
                continue;
            }
            if self.starts("<![CDATA[") {
                self.i += 9;
                let start = self.i;
                match find(&self.s[self.i..], b"]]>") {
                    Some(at) => {
                        text.push_str(&self.text[start..start + at]);
                        self.i = start + at + 3;
                    }
                    None => return self.err("a CDATA section is never closed"),
                }
                continue;
            }
            if self.starts("<?") {
                self.skip_past("?>", "a processing instruction")?;
                continue;
            }
            if self.starts("<!") {
                return self.err("declarations aren't allowed inside elements");
            }
            if self.s[self.i] == b'<' {
                flush(&mut text, &mut el);
                let child = self.element(depth + 1)?;
                el.children.push(Node::Element(child));
                continue;
            }
            let start = self.i;
            while self.i < self.s.len() && self.s[self.i] != b'<' {
                self.i += 1;
            }
            text.push_str(&unescape(&self.text[start..self.i]));
        }
    }
}

fn flush(text: &mut String, el: &mut Element) {
    if !text.is_empty() {
        el.children.push(Node::Text(std::mem::take(text)));
    }
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Expands the predefined entities and numeric references. Anything else starting with '&' stays as written.
pub fn unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let end = rest[1..].find(|c: char| c == ';' || c == '&' || c.is_whitespace()).map(|e| e + 1);
        let entity = match end {
            Some(e) if rest.as_bytes()[e] == b';' && e <= 12 => Some(&rest[1..e]),
            _ => None,
        };
        let ch = entity.and_then(|name| match name {
            "lt" => Some('<'),
            "gt" => Some('>'),
            "amp" => Some('&'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            n if n.starts_with("#x") || n.starts_with("#X") => u32::from_str_radix(&n[2..], 16).ok().and_then(char::from_u32),
            n if n.starts_with('#') => n[1..].parse::<u32>().ok().and_then(char::from_u32),
            _ => None,
        });
        match (ch, entity) {
            (Some(c), Some(name)) if c != '\0' => {
                out.push(c);
                rest = &rest[name.len() + 2..];
            }
            _ => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}
