//! The check before Play, made ready to show: lines already read or answered come out, mods that are on but miss
//! what they need become one warning, web addresses come out of the words, and the answer says whether anything is
//! in the way and whether these same warnings were played through before.

use crate::json::{Json, quote};
use crate::needs::{self, Links, Shown};
use crate::seen::Seen;

/// The loader's one-time Steam Launch Options line (the app's check adds it).
pub const LAUNCH_TITLE: &str = "One-time Steam setting for the mod loader";
/// Mods that are on miss what they need.
pub const REQ_TITLE: &str = "Mods need others that aren't on";

/// What the check is shaped with: the game and its mods as the interface shows them, and what was seen before.
#[derive(Debug, Clone, Default)]
pub struct PlayCtx {
    /// The game's id ("" when no game is open).
    pub game: String,
    pub mods: Vec<Shown>,
    pub links: Links,
    /// The game's Nexus domain (for requirements with none of their own).
    pub domain: String,
    /// Steam's Launch Options read back: Some(true) when they have the loader's line, Some(false) when they
    /// don't, None when they couldn't be read.
    pub launch_set: Option<bool>,
    /// The Launch Options line was seen before (stands in when they can't be read).
    pub launch_seen: bool,
    /// Titles of this game's read-once lines that were read.
    pub once_seen: Vec<String>,
    /// The game updated (since mods were last on) and its warning was answered already: the mods are held until
    /// they update, or the warning for this build was shown.
    pub update_answered: bool,
    /// Warnings played through before ("<game>|<mods on>|<warnings>").
    pub played_warn: Vec<String>,
}

impl PlayCtx {
    /// From the interface's {"game","mods","links","domain","launchSet","update","hold"} and what was seen.
    /// "update" is the build the game updated to (null when it didn't), "hold" whether mods are held for it.
    pub fn read(j: &Json, seen: &Seen) -> PlayCtx {
        let game = j.str_at("game").unwrap_or_default().to_string();
        let update = j.str_at("update");
        let hold = j.get("hold").is_some_and(Json::truthy);
        PlayCtx {
            mods: needs::shown_from(j.get("mods")),
            links: needs::links_from(j.get("links")),
            domain: j.str_at("domain").unwrap_or_default().to_string(),
            launch_set: j.get("launchSet").and_then(Json::as_bool),
            launch_seen: seen.launch_seen(&game),
            once_seen: seen.once_seen(&game),
            update_answered: !game.is_empty() && update.is_some_and(|b| hold || seen.update_warned_for(&game, b)),
            played_warn: seen.played_warn.clone(),
            game,
        }
    }
}

/// The check ready to show.
#[derive(Debug, Clone, PartialEq)]
pub struct Shaped {
    pub checks: Vec<Json>,
    /// Something stops play or warns.
    pub trouble: bool,
    /// Something stops play.
    pub stops: bool,
    /// Nothing stops play and these warnings were played through with these same mods before.
    pub known: bool,
    /// What Play anyway remembers for these warnings with these mods.
    pub key: String,
}

impl Shaped {
    pub fn to_json(&self) -> String {
        let checks: Vec<String> = self.checks.iter().map(Json::to_text).collect();
        format!(
            "{{\"checks\":[{}],\"trouble\":{},\"stops\":{},\"known\":{},\"key\":{}}}",
            checks.join(","),
            self.trouble,
            self.stops,
            self.known,
            quote(&self.key)
        )
    }
}

/// `String(v)` for a field that may be missing, the way the words were read: missing, null, false, 0 and "" read
/// as "".
fn text_of(v: Option<&Json>) -> String {
    match v {
        Some(Json::Str(s)) => s.clone(),
        Some(j) if j.truthy() => j.to_text(),
        _ => String::new(),
    }
}

fn level_is(x: &Json, level: &str) -> bool {
    x.str_at("level") == Some(level)
}

fn set_field(x: &mut Json, key: &str, value: Json) {
    if let Json::Obj(items) = x {
        match items.iter_mut().find(|(k, _)| k == key) {
            Some(slot) => slot.1 = value,
            None => items.push((key.to_string(), value)),
        }
    }
}

/// Shapes the raw check (a JSON list of {"level","title","detail", ...}).
pub fn shape(raw: &Json, ctx: &PlayCtx) -> Shaped {
    let mut list: Vec<Json> = raw.as_array().to_vec();
    // The loader's one-time Steam setting goes only once Steam's Launch Options really have the line (read back
    // from Steam); when they can't be read, once it has been seen.
    if ctx.launch_set.unwrap_or(ctx.launch_seen) {
        list.retain(|x| x.str_at("title") != Some(LAUNCH_TITLE));
    }
    // Other read-once lines (REDmod) go once read.
    list.retain(|x| {
        let once = x.get("once").is_some_and(Json::truthy);
        let title = x.str_at("title");
        !(once && title != Some(LAUNCH_TITLE) && ctx.once_seen.iter().any(|t| Some(t.as_str()) == title))
    });
    // Mods that are on but miss what they need: one warning naming them.
    let need = needs::missing_on(&ctx.mods, &ctx.links, &ctx.domain);
    if !need.words.is_empty() {
        list.retain(|x| !level_is(x, "ok"));
        list.push(Json::Obj(vec![
            ("level".into(), Json::Str("warn".into())),
            ("title".into(), Json::Str(REQ_TITLE.into())),
            ("detail".into(), Json::Str(format!("{} Without them those mods may not load, or stop the game.", need.words))),
        ]));
    }
    // One warning for one game update: once it was answered, the check doesn't ask again about the same update.
    if ctx.update_answered {
        list.retain(|x| !text_of(x.get("title")).ends_with(" updated this game"));
    }
    if list.is_empty() {
        list.push(Json::Obj(vec![
            ("level".into(), Json::Str("ok".into())),
            ("title".into(), Json::Str("Ready to play".into())),
            ("detail".into(), Json::Str("Everything checks out.".into())),
        ]));
    }
    // Plain words: web addresses come out of the text (the game menu and the loader's own row get there), and
    // "Missing master" says what it means.
    for x in &mut list {
        let detail = text_of(x.get("detail"));
        let url = first_url(&detail);
        let words = plain_words(&detail, !url.is_empty());
        if x.str_at("title") == Some("Missing master") {
            set_field(x, "title", Json::Str("Needs a mod that isn't on".into()));
        }
        set_field(x, "detail", Json::Str(words));
        set_field(x, "url", Json::Str(url));
    }
    let stops = list.iter().any(|x| level_is(x, "stop"));
    let trouble = stops || list.iter().any(|x| level_is(x, "warn"));
    let key = play_key(&ctx.game, &ctx.mods, &list);
    let known = !stops && ctx.played_warn.contains(&key);
    Shaped { checks: list, trouble, stops, known, key }
}

/// "<game>|<ids of the mods on, sorted>|<warning titles, sorted>": the same warnings with the same mods.
pub fn play_key(game: &str, mods: &[Shown], checks: &[Json]) -> String {
    let sorted = |mut v: Vec<String>| {
        v.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
        v.join(",")
    };
    let on: Vec<String> = mods.iter().filter(|m| m.enabled).map(|m| m.id.clone()).collect();
    let warns: Vec<String> = checks.iter().filter(|x| level_is(x, "warn")).map(|x| x.str_at("title").unwrap_or_default().to_string()).collect();
    format!("{game}|{}|{}", sorted(on), sorted(warns))
}

/// A space as the words' patterns count one: Unicode's spaces and line breaks, and the byte order mark, but not
/// the next-line control.
fn is_space(c: char) -> bool {
    (c.is_whitespace() && c != '\u{85}') || c == '\u{feff}'
}

/// At `i`, "http://" or "https://" and one or more characters up to a space or ")": where that ends.
fn url_end(s: &[char], i: usize) -> Option<usize> {
    let starts = |at: usize, word: &str| word.chars().enumerate().all(|(k, c)| s.get(at + k) == Some(&c));
    if !starts(i, "http") {
        return None;
    }
    let mut j = i + 4;
    if s.get(j) == Some(&'s') && starts(j + 1, "://") {
        j += 1;
    }
    if !starts(j, "://") {
        return None;
    }
    j += 3;
    let from = j;
    while j < s.len() && !is_space(s[j]) && s[j] != ')' {
        j += 1;
    }
    (j > from).then_some(j)
}

/// The first web address in the text, without a full stop or comma at its end.
pub fn first_url(text: &str) -> String {
    let s: Vec<char> = text.chars().collect();
    for i in 0..s.len() {
        if let Some(end) = url_end(&s, i) {
            let mut url: String = s[i..end].iter().collect();
            if url.ends_with('.') || url.ends_with(',') {
                url.pop();
            }
            return url;
        }
    }
    String::new()
}

/// Every match of a pattern made of leading spaces then `at` (a test at the place after the spaces giving where
/// the match ends), replaced by `with`.
fn replace_all(s: &[char], with: &str, at: impl Fn(&[char], usize) -> Option<usize>) -> Vec<char> {
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        let mut j = i;
        while j < s.len() && is_space(s[j]) {
            j += 1;
        }
        match at(s, j) {
            Some(end) => {
                out.extend(with.chars());
                i = end;
            }
            None => {
                out.push(s[i]);
                i += 1;
            }
        }
    }
    out
}

/// The check's words without web addresses: "Get it from <address>" says Turbine shows where it comes from (when
/// there is an address), other addresses (with their brackets) go, and spaces are tidied (a run of spaces with a
/// line break in it stays one line break, so a list in the words keeps its lines).
pub fn plain_words(text: &str, has_url: bool) -> String {
    let s: Vec<char> = text.chars().collect();
    let get_it = |s: &[char], j: usize| -> Option<usize> {
        let lead = "Get it from ";
        if !lead.chars().enumerate().all(|(k, c)| s.get(j + k) == Some(&c)) {
            return None;
        }
        let end = url_end(s, j + lead.chars().count())?;
        Some(if matches!(s.get(end), Some('.') | Some(',')) { end + 1 } else { end })
    };
    let s = replace_all(&s, if has_url { " Turbine shows where it comes from." } else { "" }, get_it);
    let bracketed = |s: &[char], j: usize| -> Option<usize> {
        let end = if s.get(j) == Some(&'(') { url_end(s, j + 1)? } else { url_end(s, j)? };
        Some(if s.get(end) == Some(&')') { end + 1 } else { end })
    };
    let s = replace_all(&s, "", bracketed);
    let mut out = String::with_capacity(s.len());
    let mut space = false;
    let mut line = false;
    for c in s {
        if is_space(c) {
            space = true;
            line |= c == '\n';
        } else {
            if space && !out.is_empty() {
                out.push(if line { '\n' } else { ' ' });
            }
            space = false;
            line = false;
            out.push(c);
        }
    }
    out
}
