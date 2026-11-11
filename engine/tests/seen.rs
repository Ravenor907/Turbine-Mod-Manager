//! What the player has seen and answered, kept between runs, and the interface's earlier values read in once.

use turbine_engine::json;
use turbine_engine::needs::{GetItem, ReqQueue};
use turbine_engine::seen::{Seen, KEEP};

#[test]
fn round_trips_through_its_file() {
    let dir = std::env::temp_dir().join(format!("turbine-seen-{}", std::process::id()));
    let path = dir.join("seen.json");
    let mut s = Seen::load(&path);
    assert_eq!(s, Seen::default(), "no file yet");
    s.mark_launch("g");
    s.mark_once("g", &["REDmod".into()]);
    s.mark_played("g|a|W");
    s.mark_update_warned("g|20");
    s.set_dep_move("g", "m1", true);
    s.set_dep_move("g", "m2", false);
    s.req_queue = Some(ReqQueue::new("g", "Alpha", vec![GetItem { name: "Lib".into(), mod_id: 7, domain: "d".into() }]));
    s.save(&path).unwrap();
    assert_eq!(Seen::load(&path), s);
    assert_eq!(s.moves_json("g"), r#"{"m1":"in","m2":"out"}"#);
    assert_eq!(s.moves_json("other"), "{}");
    std::fs::remove_dir_all(&dir).unwrap();
    // A damaged file reads as nothing seen.
    assert_eq!(Seen::parse("{nope"), Seen::default());
}

#[test]
fn played_warnings_keep_one_per_game_and_the_last_fifty() {
    let mut s = Seen::default();
    s.mark_played("g|a|W1");
    s.mark_played("h|a|W1");
    s.mark_played("g|a,b|W2");
    assert_eq!(s.played_warn, ["h|a|W1", "g|a,b|W2"]);
    for i in 0..60 {
        s.mark_played(&format!("game{i}|x|y"));
        s.mark_update_warned(&format!("game{i}|1"));
    }
    assert_eq!(s.played_warn.len(), KEEP);
    assert_eq!(s.played_warn[0], "game10|x|y");
    assert_eq!(s.update_warned.len(), KEEP);
    assert!(s.update_warned("game59|1") && !s.update_warned("game9|1"));
    assert!(s.update_warned_for("game59", "1") && !s.update_warned_for("game59", "2"));
}

#[test]
fn moves_and_read_once_lines() {
    let mut s = Seen::default();
    s.set_dep_move("g", "m", true);
    s.set_dep_move("g", "m", false);
    assert_eq!(s.dep_moves("g"), [("m".to_string(), false)]);
    s.mark_once("g", &["A".into(), "B|C".into()]);
    s.mark_once("h", &["A".into()]);
    assert_eq!(s.once_seen("g"), ["A", "B|C"]);
    assert!(s.once_seen("").is_empty() && !s.launch_seen(""));
}

#[test]
fn the_interfaces_old_values_are_read_in_once() {
    let old = json::parse(
        r#"{"launchDone":"[\"g\"]","onceDone":"[\"g|REDmod\"]","playedWarn":"[\"g|a|W\"]","updateWarned":"[\"g|20\"]",
            "reqQueue":"{\"game\":\"g\",\"forName\":\"Alpha\",\"items\":[{\"name\":\"Lib\",\"modId\":7,\"domain\":\"d\"}],\"index\":1}",
            "depMoves":"{\"g\":{\"m\":\"in\",\"n\":\"out\",\"x\":\"other\"}}"}"#,
    )
    .unwrap();
    let mut s = Seen::default();
    s.mark_launch("h");
    s.set_dep_move("g", "n", true);
    assert!(s.adopt(&old));
    assert_eq!(s.launch_done, ["g", "h"]);
    assert_eq!(s.once_seen("g"), ["REDmod"]);
    assert_eq!(s.played_warn, ["g|a|W"]);
    assert!(s.update_warned_for("g", "20"));
    let q = s.req_queue.clone().unwrap();
    assert_eq!((q.game.as_str(), q.for_name.as_str(), q.index, q.items.len()), ("g", "Alpha", 1, 1));
    // A move made here already wins over the old one.
    assert_eq!(s.moves_json("g"), r#"{"n":"in","m":"in"}"#);
    // Only once.
    assert!(!s.adopt(&json::parse(r#"{"launchDone":"[\"z\"]"}"#).unwrap()));
    assert!(!s.launch_seen("z"));
    // Empty or damaged old values read as nothing.
    let mut t = Seen::default();
    assert!(t.adopt(&json::parse(r#"{"launchDone":"[]","reqQueue":"","depMoves":"{oops"}"#).unwrap()));
    assert_eq!(t, Seen { adopted: true, ..Seen::default() });
}
