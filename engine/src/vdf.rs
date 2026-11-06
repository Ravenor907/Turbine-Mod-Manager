//! Valve's text KeyValues format ("VDF"), as used by libraryfolders.vdf and appmanifest_*.acf.
//!
//! Keys are case-insensitive in Steam's own reader, so lookups here are too.

#[derive(Debug, Clone, PartialEq)]
pub enum Vdf {
    Str(String),
    Obj(Vec<(String, Vdf)>),
}

impl Vdf {
    /// The child with this key (case-insensitive), if this is an object.
    pub fn get(&self, key: &str) -> Option<&Vdf> {
        match self {
            Vdf::Obj(items) => items.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v),
            Vdf::Str(_) => None,
        }
    }

    /// Walks a path of keys: `v.path(&["AppState", "installdir"])`.
    pub fn path(&self, keys: &[&str]) -> Option<&Vdf> {
        keys.iter().try_fold(self, |v, k| v.get(k))
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Vdf::Str(s) => Some(s),
            Vdf::Obj(_) => None,
        }
    }

    pub fn str_at(&self, keys: &[&str]) -> Option<&str> {
        self.path(keys).and_then(Vdf::as_str)
    }

    /// The (key, value) pairs of an object, in file order.
    pub fn entries(&self) -> &[(String, Vdf)] {
        match self {
            Vdf::Obj(items) => items,
            Vdf::Str(_) => &[],
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct VdfError {
    pub line: usize,
    pub message: &'static str,
}

impl std::fmt::Display for VdfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for VdfError {}

/// Parses a whole file. The result is an object holding the top-level keys.
pub fn parse(text: &str) -> Result<Vdf, VdfError> {
    let mut p = Parser { chars: text.chars().collect(), pos: 0, line: 1 };
    let items = p.object_body(false)?;
    Ok(Vdf::Obj(items))
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
    line: usize,
}

enum Tok {
    Str(String),
    Open,
    Close,
    End,
}

impl Parser {
    fn err(&self, message: &'static str) -> VdfError {
        VdfError { line: self.line, message }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied();
        if let Some(ch) = c {
            self.pos += 1;
            if ch == '\n' {
                self.line += 1;
            }
        }
        c
    }

    fn skip_space_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() || c == '\u{feff}' => {
                    self.bump();
                }
                Some('/') if self.chars.get(self.pos + 1) == Some(&'/') => {
                    while let Some(c) = self.bump() {
                        if c == '\n' {
                            break;
                        }
                    }
                }
                _ => return,
            }
        }
    }

    fn token(&mut self) -> Result<Tok, VdfError> {
        self.skip_space_and_comments();
        match self.peek() {
            None => Ok(Tok::End),
            Some('{') => {
                self.bump();
                Ok(Tok::Open)
            }
            Some('}') => {
                self.bump();
                Ok(Tok::Close)
            }
            Some('"') => {
                self.bump();
                let mut s = String::new();
                loop {
                    match self.bump() {
                        None => return Err(self.err("unterminated string")),
                        Some('"') => break,
                        Some('\\') => match self.bump() {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some('\\') => s.push('\\'),
                            Some('"') => s.push('"'),
                            Some(other) => {
                                s.push('\\');
                                s.push(other);
                            }
                            None => return Err(self.err("unterminated string")),
                        },
                        Some(c) => s.push(c),
                    }
                }
                // Conditionals like [$WIN32] after a value are skipped.
                self.skip_conditional();
                Ok(Tok::Str(s))
            }
            Some(_) => {
                // Unquoted token: runs to whitespace or a brace.
                let mut s = String::new();
                while let Some(c) = self.peek() {
                    if c.is_whitespace() || c == '{' || c == '}' || c == '"' {
                        break;
                    }
                    s.push(c);
                    self.bump();
                }
                self.skip_conditional();
                Ok(Tok::Str(s))
            }
        }
    }

    fn skip_conditional(&mut self) {
        let save = (self.pos, self.line);
        while let Some(c) = self.peek() {
            if c == ' ' || c == '\t' {
                self.bump();
            } else {
                break;
            }
        }
        if self.peek() == Some('[') {
            while let Some(c) = self.bump() {
                if c == ']' || c == '\n' {
                    return;
                }
            }
        } else {
            self.pos = save.0;
            self.line = save.1;
        }
    }

    fn object_body(&mut self, nested: bool) -> Result<Vec<(String, Vdf)>, VdfError> {
        let mut items = Vec::new();
        loop {
            let key = match self.token()? {
                Tok::Str(s) => s,
                Tok::Close if nested => return Ok(items),
                Tok::Close => return Err(self.err("unexpected '}'")),
                Tok::End if !nested => return Ok(items),
                Tok::End => return Err(self.err("missing '}'")),
                Tok::Open => return Err(self.err("expected a key, found '{'")),
            };
            let value = match self.token()? {
                Tok::Str(s) => Vdf::Str(s),
                Tok::Open => Vdf::Obj(self.object_body(true)?),
                Tok::Close | Tok::End => return Err(self.err("key without a value")),
            };
            items.push((key, value));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIBRARY: &str = r#"
"libraryfolders"
{
	"0"
	{
		"path"		"/home/deck/.local/share/Steam"
		"label"		""
		"apps"
		{
			"228980"		"0"
			"489830"		"14958233"
		}
	}
	// SD card
	"1"
	{
		"path"		"/run/media/deck/SD \"Card\""
		"apps"
		{
			"892970"		"1234"
		}
	}
}
"#;

    #[test]
    fn reads_library_folders() {
        let v = parse(LIBRARY).unwrap();
        let lf = v.get("LibraryFolders").unwrap();
        assert_eq!(lf.entries().len(), 2);
        assert_eq!(lf.str_at(&["0", "path"]), Some("/home/deck/.local/share/Steam"));
        assert_eq!(lf.str_at(&["1", "path"]), Some("/run/media/deck/SD \"Card\""));
        let apps: Vec<&str> = lf.path(&["0", "apps"]).unwrap().entries().iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(apps, ["228980", "489830"]);
    }

    #[test]
    fn skips_conditionals_and_unquoted() {
        let v = parse("root { key value [$WIN32] other \"x\" }").unwrap();
        assert_eq!(v.str_at(&["root", "key"]), Some("value"));
        assert_eq!(v.str_at(&["root", "other"]), Some("x"));
    }

    #[test]
    fn reports_broken_files() {
        assert!(parse("\"a\" { \"b\" \"c\"").is_err());
        assert!(parse("\"a\"").is_err());
        assert!(parse("}").is_err());
    }
}
