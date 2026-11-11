//! The rail's order: pinned games, then the last played, then the rest; hidden ones left off.

use turbine_engine::games::{rail_order, rail_order_json};
use turbine_engine::json;

fn ids(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn pinned_then_played_then_the_rest() {
    let all = ids(&["a", "b", "c", "d", "e"]);
    assert_eq!(rail_order(&all, &[], &[], &[], ""), [0, 1, 2, 3, 4]);
    assert_eq!(rail_order(&all, &ids(&["d", "b", "x"]), &[], &[], ""), [3, 1, 0, 2, 4]);
    assert_eq!(rail_order(&all, &ids(&["d", "b"]), &ids(&["e", "b"]), &[], ""), [4, 1, 3, 0, 2]);
}

#[test]
fn hidden_games_are_left_off_unless_asked_for() {
    let all = ids(&["a", "b", "c"]);
    assert_eq!(rail_order(&all, &[], &[], &ids(&["b"]), ""), [0, 2]);
    assert_eq!(rail_order(&all, &[], &[], &ids(&["b"]), "b"), [0, 1, 2]);
    assert!(rail_order(&all, &[], &ids(&["a"]), &all, "").is_empty(), "every game hidden");
    let j = json::parse(r#"{"ids":["a","b","c"],"played":["c"],"pinned":[],"hidden":["a"],"always":""}"#).unwrap();
    assert_eq!(rail_order_json(&j), "[2,1]");
}
