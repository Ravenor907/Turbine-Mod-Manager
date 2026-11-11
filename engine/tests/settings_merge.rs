//! 0.6: a player's settings changes survive a mod update: repeated names by their place among the old values, list
//! items by their identity; what can't be placed for sure is listed for a second look, with the old file kept.

use std::fs;
use turbine_engine::settings::{self, Format};

#[test]
fn repeated_keys_follow_their_values() {
    let old = "[Paths]\nPath = a\nPath = b\nPath = c\n";
    let mine = "[Paths]\nPath = a\nPath = B2\nPath = c\n";
    // The new version adds one in front: the player's change still lands on what was "b".
    let new = "[Paths]\nPath = z\nPath = a\nPath = b\nPath = c\n";
    let r = settings::merge_report(Format::Ini, new, old, mine);
    assert_eq!(r.text, "[Paths]\nPath = z\nPath = a\nPath = B2\nPath = c\n");
    assert!(r.unsure.is_empty() && r.gone.is_empty(), "{r:?}");
    // The same value twice and one removed: which one is meant can't be told.
    let old = "[s]\nk = 1\nk = 1\n";
    let mine = "[s]\nk = 1\nk = 5\n";
    let new = "[s]\nk = 1\n";
    let r = settings::merge_report(Format::Ini, new, old, mine);
    assert_eq!(r.text, new);
    assert_eq!(r.unsure, ["s: k (number 2)"]);
    // The old value gone from the new file: left as the new file has it, and listed.
    let r = settings::merge_report(Format::Ini, "[s]\nk = 9\nk = 2\n", "[s]\nk = 1\nk = 2\n", "[s]\nk = 7\nk = 2\n");
    assert_eq!(r.text, "[s]\nk = 9\nk = 2\n");
    assert_eq!(r.unsure, ["s: k"]);
    // The old behaviour stays for the simple case.
    assert_eq!(settings::merge_report(Format::Ini, "[a]\n; new help\nx = 5\ny = 2\nw = 4\n", "[a]\nx = 1\ny = 2\nz = 3\n", "[a]\nx = 1\ny = 9\nz = 3\n").text, "[a]\n; new help\nx = 5\ny = 9\nw = 4\n");
}

#[test]
fn list_items_follow_their_identity() {
    let old = r#"{"plugins":[{"id":"alpha","on":true},{"id":"beta","on":true}],"volume":5}"#;
    let mine = r#"{"plugins":[{"id":"alpha","on":true},{"id":"beta","on":false}],"volume":8}"#;
    // The new version puts a new plugin first: beta's "on" follows beta.
    let new = r#"{"plugins":[{"id":"gamma","on":true},{"id":"alpha","on":true},{"id":"beta","on":true}],"volume":5}"#;
    let r = settings::merge_report(Format::Json, new, old, mine);
    assert_eq!(r.text, r#"{"plugins":[{"id":"gamma","on":true},{"id":"alpha","on":true},{"id":"beta","on":false}],"volume":8}"#);
    assert!(r.unsure.is_empty() && r.gone.is_empty(), "{r:?}");
    // beta removed: listed as gone.
    let new = r#"{"plugins":[{"id":"alpha","on":true}],"volume":5}"#;
    let r = settings::merge_report(Format::Json, new, old, mine);
    assert_eq!(r.gone, ["plugins.1: on"]);
    assert_eq!(r.text, r#"{"plugins":[{"id":"alpha","on":true}],"volume":8}"#);
    // Items without an identity in a list that changed: unsure.
    let old = r#"{"slots":[{"on":true},{"on":true}]}"#;
    let mine = r#"{"slots":[{"on":true},{"on":false}]}"#;
    let new = r#"{"slots":[{"on":true},{"on":true},{"on":true}]}"#;
    let r = settings::merge_report(Format::Json, new, old, mine);
    assert_eq!(r.text, new);
    assert_eq!(r.unsure, ["slots.1: on"]);
    // A list of plain values that changed: unsure; unchanged: carried.
    let r = settings::merge_report(Format::Json, r#"{"n":[1,2,3,4]}"#, r#"{"n":[1,2,3]}"#, r#"{"n":[1,9,3]}"#);
    assert_eq!(r.unsure, ["n: 1"]);
    let r = settings::merge_report(Format::Json, r#"{"n":[1,2,3],"x":1}"#, r#"{"n":[1,2,3]}"#, r#"{"n":[1,9,3]}"#);
    assert_eq!(r.text, r#"{"n":[1,9,3],"x":1}"#);
    // The same identity twice in the new list: unsure.
    let r = settings::merge_report(
        Format::Json,
        r#"{"p":[{"name":"a","v":1},{"name":"a","v":1},{"name":"b","v":1}]}"#,
        r#"{"p":[{"name":"a","v":1},{"name":"b","v":1}]}"#,
        r#"{"p":[{"name":"a","v":2},{"name":"b","v":1}]}"#,
    );
    assert_eq!(r.unsure, ["p.0: v"]);
}

#[test]
fn a_mod_update_keeps_the_old_file_and_lists_what_to_check() {
    let d = std::env::temp_dir().join(format!("turbine-settingsmerge-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    let store = d.join("store");
    let def = d.join("mods/m1/a.ini");
    fs::create_dir_all(def.parent().unwrap()).unwrap();
    fs::write(&def, "[s]\nk = 1\nk = 1\nv = 2\n").unwrap();
    let rel = "a.ini";
    settings::save_edit(&store, "m1", rel, &def, "[s]\nk = 1\nk = 5\nv = 3\n").unwrap();
    // The update: one of the two k goes.
    fs::write(&def, "[s]\nk = 1\nv = 2\nw = 0\n").unwrap();
    let p = settings::effective(&store, "m1", rel, &def);
    assert_eq!(fs::read_to_string(&p).unwrap(), "[s]\nk = 1\nv = 3\nw = 0\n", "what was sure is carried");
    assert_eq!(fs::read_to_string(settings::before_update_path(&store, "m1", rel)).unwrap(), "[s]\nk = 1\nk = 5\nv = 3\n");
    let (look, gone) = settings::review_notes(&store, "m1", rel);
    assert_eq!(look, ["s: k (number 2)"]);
    assert!(gone.is_empty());
    let words = settings::review_words("Seamless Co-op", &look, &gone).unwrap();
    assert!(words.contains("Please check these settings again") && words.contains("s: k (number 2)") && words.contains("kept"), "{words}");
    settings::clear_review(&store, "m1", rel);
    assert_eq!(settings::review_notes(&store, "m1", rel), (vec![], vec![]));
    assert_eq!(settings::review_words("x", &[], &[]), None);
    // A clean update leaves no list behind.
    fs::write(&def, "[s]\nk = 1\nv = 2\nw = 1\n").unwrap();
    settings::effective(&store, "m1", rel, &def);
    assert_eq!(settings::review_notes(&store, "m1", rel), (vec![], vec![]));
    let _ = fs::remove_dir_all(&d);
}
