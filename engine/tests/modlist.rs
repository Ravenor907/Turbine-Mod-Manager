//! My mods as a list: Find, Show, Sort and the Dependencies row.

use turbine_engine::json;
use turbine_engine::modlist::{self, Row};

/// Five mods in load order: a framework (core), a library another mod needs, a mod that needs it, one that is off,
/// and one added last. `extra` (",key":value...) goes first, so it wins over the defaults after it.
fn input(extra: &str) -> String {
    let first = match extra.strip_prefix(',') {
        Some(e) => format!("{e},"),
        None => String::new(),
    };
    format!(
        r#"{{{first}"mods":[
            {{"id":"core","name":"Script Extender","enabled":true,"added":10,"needsLoader":false}},
            {{"id":"lib","name":"Address Library","enabled":true,"added":20}},
            {{"id":"hud","name":"Better HUD","enabled":true,"added":30,"needsLoader":true}},
            {{"id":"off","name":"zebra Skins","enabled":false,"added":40}},
            {{"id":"new","name":"Ancient Armor","enabled":true,"added":50}}],
          "links":{{"hud":{{"needs":[{{"name":"Address Library","id":"lib","domain":"","modId":0}}],"neededBy":[]}},
                   "lib":{{"needs":[],"neededBy":["hud"]}}}},
          "core":["core"],"show":"all","sort":"order","find":"","kept":"","grouped":true,"open":false,"lastPlayedAt":35}}"#
    )
}

fn run(extra: &str) -> modlist::Listed {
    list_of(&json::parse(&input(extra)).unwrap())
}

/// The list through its JSON answer, read back.
fn list_of(j: &json::Json) -> modlist::Listed {
    let text = modlist::list_json(j);
    let back = json::parse(&text).unwrap();
    let nums = |k: &str| -> Vec<usize> { back.get(k).unwrap().as_array().iter().map(|x| x.as_f64().unwrap() as usize).collect() };
    let rows = back
        .get("rows")
        .unwrap()
        .as_array()
        .iter()
        .map(|r| match r.get("i") {
            Some(i) => Row::Mod { index: i.as_f64().unwrap() as usize, dep: r.get("dep").is_some() },
            None => Row::Deps,
        })
        .collect();
    let ids: Vec<String> = j.get("mods").unwrap().as_array().iter().map(|m| m.str_at("id").unwrap().to_string()).collect();
    let dep_all = back.get("depAll").unwrap().as_array().iter().map(|x| ids.iter().position(|i| Some(i.as_str()) == x.as_str()).unwrap()).collect();
    modlist::Listed { shown: nums("shown"), deps: nums("deps"), rows, dep_all }
}

#[test]
fn dependencies_go_into_one_row_at_the_bottom() {
    let l = run("");
    assert_eq!(l.shown, [0, 1, 2, 3, 4]);
    assert_eq!(l.deps, [0, 1], "the framework and the library another mod needs");
    assert_eq!(l.rows, [Row::Mod { index: 2, dep: false }, Row::Mod { index: 3, dep: false }, Row::Mod { index: 4, dep: false }, Row::Deps]);
    // Open, they show under it.
    let l = run(r#","open":true"#);
    assert_eq!(&l.rows[3..], [Row::Deps, Row::Mod { index: 0, dep: true }, Row::Mod { index: 1, dep: true }]);
    // Not grouped (moving, selecting, finding): every mod in its place.
    let l = run(r#","grouped":false"#);
    assert!(l.deps.is_empty() && l.rows.len() == 5);
    assert_eq!(l.dep_all, [0, 1], "still counted as dependencies");
}

#[test]
fn the_players_moves_win() {
    let l = run(r#","moves":{"lib":"out","off":"in","core":"other"}"#);
    assert_eq!(l.deps, [0, 3]);
    // Needed by a mod that isn't installed: not a dependency.
    let j = json::parse(&input("").replace(r#""neededBy":["hud"]"#, r#""neededBy":["gone"]"#)).unwrap();
    assert_eq!(list_of(&j).deps, [0]);
}

#[test]
fn find_matches_every_word_in_any_case() {
    let l = run(r#","find":"  ADDRESS   lib ","grouped":false"#);
    assert_eq!(l.shown, [1]);
    let l = run(r#","find":"armor x","grouped":false"#);
    assert!(l.shown.is_empty());
    // The mod just turned on or off keeps its row.
    let l = run(r#","find":"armor x","kept":"hud","grouped":false"#);
    assert_eq!(l.shown, [2]);
}

#[test]
fn show_narrows_the_list() {
    let shown = |extra: &str| run(&format!(r#",{extra},"grouped":false"#)).shown;
    assert_eq!(shown(r#""show":"on""#), [0, 1, 2, 4]);
    assert_eq!(shown(r#""show":"off""#), [3]);
    assert_eq!(shown(r#""show":"new""#), [3, 4], "added after the last Play");
    assert_eq!(shown(r#""show":"update","updates":["lib"]"#), [1]);
    // Needs attention: nothing yet; the loader missing (mods that need it, on or off); a mod on whose need is off;
    // a mod with its files covered or a warning (only while on).
    assert!(shown(r#""show":"attention""#).is_empty());
    assert_eq!(shown(r#""show":"attention","loaderMissing":true"#), [2]);
    assert_eq!(shown(r#""show":"attention","hidden":["off","new"],"warned":["core"]"#), [0, 4]);
    let j = json::parse(&input(r#","show":"attention","grouped":false"#).replace(r#""name":"Address Library","enabled":true"#, r#""name":"Address Library","enabled":false"#)).unwrap();
    assert_eq!(list_of(&j).shown, [2]);
}

#[test]
fn sort_orders_it() {
    assert_eq!(run(r#","sort":"az","grouped":false"#).shown, [1, 4, 2, 0, 3], "A to Z ignores case");
    assert_eq!(run(r#","sort":"recent","grouped":false"#).shown, [4, 3, 2, 1, 0]);
    // Grouped, the sorted list loses its dependencies to the row.
    let l = run(r#","sort":"az""#);
    assert_eq!(l.rows[..3], [Row::Mod { index: 4, dep: false }, Row::Mod { index: 2, dep: false }, Row::Mod { index: 3, dep: false }]);
    assert_eq!(l.deps, [1, 0], "in the sorted order");
}

#[test]
fn equal_names_and_times_keep_the_load_order() {
    let j = json::parse(
        r#"{"mods":[{"id":"a","name":"Same","added":5},{"id":"b","name":"same","added":5},{"id":"c","name":"Same","added":5}],"sort":"az"}"#,
    )
    .unwrap();
    assert_eq!(list_of(&j).shown, [0, 1, 2]);
    let j = json::parse(&j.to_text().replace("\"az\"", "\"recent\"")).unwrap();
    assert_eq!(list_of(&j).shown, [0, 1, 2]);
}
