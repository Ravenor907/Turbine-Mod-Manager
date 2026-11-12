//! The check before Play made ready to show: read-once lines, the requirements warning, one warning per game update,
//! web addresses out of the words, and the same warnings played through before.

use turbine_engine::json::{self, Json};
use turbine_engine::precheck::{self, PlayCtx, LAUNCH_TITLE, REQ_TITLE};
use turbine_engine::seen::Seen;

fn raw(text: &str) -> Json {
    json::parse(text).unwrap()
}

fn ctx(input: &str, seen: &Seen) -> PlayCtx {
    PlayCtx::read(&raw(input), seen)
}

fn titles(s: &precheck::Shaped) -> Vec<String> {
    s.checks.iter().map(|x| format!("{} {}", x.str_at("level").unwrap_or(""), x.str_at("title").unwrap_or(""))).collect()
}

const GAME: &str = r#""game":"steam:1","mods":[{"id":"a","name":"Alpha","enabled":true},{"id":"b","name":"Beta","enabled":false}]"#;

#[test]
fn nothing_found_is_ready_to_play() {
    let s = precheck::shape(&raw("[]"), &ctx(&format!("{{{GAME}}}"), &Seen::default()));
    assert_eq!(titles(&s), ["ok Ready to play"]);
    assert_eq!(s.checks[0].str_at("detail"), Some("Everything checks out."));
    assert_eq!(s.checks[0].str_at("url"), Some(""));
    assert!(!s.trouble && !s.stops && !s.known);
    assert_eq!(s.key, "steam:1|a|");
}

#[test]
fn the_launch_line_goes_once_steam_has_it_or_once_seen() {
    let list = raw(&format!(r#"[{{"level":"warn","title":"{LAUNCH_TITLE}","detail":"x","once":true}}]"#));
    let mut seen = Seen::default();
    // Not read back and not seen: it shows.
    assert_eq!(titles(&precheck::shape(&list, &ctx(&format!("{{{GAME}}}"), &seen))), [format!("warn {LAUNCH_TITLE}")]);
    // Read back with the line set: it goes.
    assert_eq!(titles(&precheck::shape(&list, &ctx(&format!("{{{GAME},\"launchSet\":true}}"), &seen))), ["ok Ready to play"]);
    seen.mark_launch("steam:1");
    // Seen, and Steam's options can't be read: it goes.
    assert_eq!(titles(&precheck::shape(&list, &ctx(&format!("{{{GAME},\"launchSet\":null}}"), &seen))), ["ok Ready to play"]);
    // Seen, but read back without the line: it stays.
    assert_eq!(titles(&precheck::shape(&list, &ctx(&format!("{{{GAME},\"launchSet\":false}}"), &seen))), [format!("warn {LAUNCH_TITLE}")]);
    // Seen for another game only.
    assert_eq!(titles(&precheck::shape(&list, &ctx(r#"{"game":"steam:2"}"#, &seen))).len(), 1);
}

#[test]
fn read_once_lines_go_after_they_were_read() {
    let list = raw(r#"[{"level":"info","title":"REDmod","detail":"d","once":true},{"level":"warn","title":"Other","detail":"e"}]"#);
    let mut seen = Seen::default();
    assert_eq!(titles(&precheck::shape(&list, &ctx(&format!("{{{GAME}}}"), &seen))), ["info REDmod", "warn Other"]);
    seen.mark_once("steam:1", &["REDmod".into(), "Other".into()]);
    // Only lines marked once go.
    assert_eq!(titles(&precheck::shape(&list, &ctx(&format!("{{{GAME}}}"), &seen))), ["warn Other"]);
    assert_eq!(titles(&precheck::shape(&list, &ctx(r#"{"game":"steam:9"}"#, &seen))), ["info REDmod", "warn Other"]);
}

#[test]
fn missing_requirements_become_one_warning_and_ok_lines_go() {
    let input = r#"{"game":"g","domain":"skyrim","mods":[{"id":"a","name":"Alpha","enabled":true},{"id":"b","name":"Beta","enabled":false}],
        "links":{"a":{"needs":[{"name":"Beta","id":"b","domain":"","modId":0},{"name":"Lib","id":null,"domain":"","modId":7}],"neededBy":[]}}}"#;
    let s = precheck::shape(&raw(r#"[{"level":"ok","title":"Fine","detail":""},{"level":"info","title":"Note","detail":""}]"#), &ctx(input, &Seen::default()));
    assert_eq!(titles(&s), ["info Note".to_string(), format!("warn {REQ_TITLE}")]);
    assert_eq!(s.checks[1].str_at("detail"), Some("Alpha needs Beta and Lib. Without them those mods may not load, or stop the game."));
    assert!(s.trouble && !s.stops);
    assert_eq!(s.key, format!("g|a|{REQ_TITLE}"));
}

#[test]
fn a_game_update_warns_once() {
    let list = raw(r#"[{"level":"warn","title":"Skyrim updated this game","detail":""},{"level":"warn","title":"Other","detail":""}]"#);
    let mut seen = Seen::default();
    let with = |extra: &str, seen: &Seen| titles(&precheck::shape(&list, &ctx(&format!("{{{GAME}{extra}}}"), seen)));
    assert_eq!(with(r#","update":"20""#, &seen).len(), 2);
    // Held until the mods update: the warning goes.
    assert_eq!(with(r#","update":"20","hold":true"#, &seen), ["warn Other"]);
    // Warned for this build (also while it waited in Steam): it goes; not for another build or with no update.
    seen.mark_update_warned("steam:1|20|waiting");
    assert_eq!(with(r#","update":"20""#, &seen), ["warn Other"]);
    assert_eq!(with(r#","update":"21""#, &seen).len(), 2);
    assert_eq!(with(r#","hold":true"#, &seen).len(), 2);
}

#[test]
fn web_addresses_come_out_of_the_words() {
    let list = raw(r#"[
        {"level":"stop","title":"Missing master","detail":"Needs X. Get it from https://www.nexusmods.com/skyrim/mods/1.  Then play."},
        {"level":"warn","title":"T","detail":"See the page (https://example.com/a,b) for more,\n and https://x.org/y."},
        {"level":"warn","title":"U","detail":"Get it from"},
        {"level":"warn","title":"V","offer":{"mod":"a"},"paths":["p"]}]"#);
    let s = precheck::shape(&list, &ctx(&format!("{{{GAME}}}"), &Seen::default()));
    let at = |i: usize, k: &str| s.checks[i].str_at(k).unwrap().to_string();
    assert_eq!(at(0, "title"), "Needs a mod that isn't on");
    assert_eq!(at(0, "detail"), "Needs X. Turbine shows where it comes from. Then play.");
    assert_eq!(at(0, "url"), "https://www.nexusmods.com/skyrim/mods/1");
    assert_eq!(at(1, "detail"), "See the page for more,\nand");
    assert_eq!(at(1, "url"), "https://example.com/a,b");
    assert_eq!(at(2, "detail"), "Get it from");
    assert_eq!(at(2, "url"), "");
    // Other fields stay as they were.
    assert_eq!(at(3, "detail"), "");
    assert_eq!(s.checks[3].get("offer").unwrap().str_at("mod"), Some("a"));
    assert!(s.stops && s.trouble && !s.known);
}

#[test]
fn the_same_warnings_played_through_are_known() {
    let list = raw(r#"[{"level":"warn","title":"B warn","detail":""},{"level":"warn","title":"A warn","detail":""},{"level":"info","title":"I","detail":""}]"#);
    let input = r#"{"game":"g","mods":[{"id":"z","name":"Z","enabled":true},{"id":"m","name":"M","enabled":true},{"id":"o","name":"O","enabled":false}]}"#;
    let mut seen = Seen::default();
    let s = precheck::shape(&list, &ctx(input, &seen));
    assert_eq!(s.key, "g|m,z|A warn,B warn");
    assert!(!s.known);
    seen.mark_played(&s.key);
    assert!(precheck::shape(&list, &ctx(input, &seen)).known);
    // Another mod on: not the same.
    let more = input.replace(r#""enabled":false"#, r#""enabled":true"#);
    assert!(!precheck::shape(&list, &ctx(&more, &seen)).known);
    // Something that stops play is never played through.
    let stop = raw(r#"[{"level":"stop","title":"S","detail":""}]"#);
    let s2 = precheck::shape(&stop, &ctx(input, &seen));
    seen.mark_played(&s2.key);
    assert!(!precheck::shape(&stop, &ctx(input, &seen)).known);
}

#[test]
fn shaped_as_json() {
    let s = precheck::shape(&raw(r#"[{"level":"warn","title":"W","detail":"d","once":false}]"#), &ctx(r#"{"game":"g"}"#, &Seen::default()));
    assert_eq!(s.to_json(), r#"{"checks":[{"level":"warn","title":"W","detail":"d","once":false,"url":""}],"trouble":true,"stops":false,"known":false,"key":"g||W"}"#);
}

#[test]
fn words_and_addresses() {
    assert_eq!(precheck::first_url("a http://x.y/z, b https://q"), "http://x.y/z");
    assert_eq!(precheck::first_url("httpx://no https:/no"), "");
    assert_eq!(precheck::plain_words("  a\t\tb  (https://x)  c ", false), "a b c");
    assert_eq!(precheck::plain_words("Get it from https://x.", false), "");
}
