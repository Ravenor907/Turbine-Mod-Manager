//! Game rules as data files: every built-in file is valid, the launch games' store IDs find their Nexus and
//! Thunderstore sites, and players' own files can add games but never change Turbine's.

mod support;

use std::fs;
use std::path::PathBuf;
use support::scratch as temp;
use turbine_engine::gamedata::{self, parse_game};
use turbine_engine::games::{Game, Runtime, Store};
use turbine_engine::rules::{self, GameRules, Support};

fn game(store: Store, id: &str, runtime: Runtime) -> Game {
    Game {
        store,
        store_id: id.into(),
        name: "x".into(),
        install_dir: PathBuf::from("/nonexistent/turbine-golden"),
        library: PathBuf::from("/nonexistent"),
        build: None,
        runtime,
        cover: None,
        header: None,
        prefix: None,
    }
}

#[test]
fn builtin_files_valid() {
    let b = gamedata::builtin();
    assert!(b.problems.is_empty(), "{:?}", b.problems);
    let (games, loaders) = gamedata::builtin_sources();
    assert_eq!(b.games.len(), games.len());
    assert_eq!(b.loaders.len(), loaders.len());
    assert!(games.len() >= 10, "the nine launch games and Hogwarts Legacy");
    for (file, text) in games.iter().chain(loaders.iter()) {
        assert!(!text.contains('\u{2014}') && !text.contains('\u{2013}'), "{file} has an em or en dash");
        assert!(file.ends_with(".json"));
    }
}

#[test]
fn store_ids_find_their_sites() {
    // Each launch game's store IDs lead to its Nexus domain, and the Thunderstore games to their community.
    for (id, domain) in [("489830", "skyrimspecialedition"), ("413150", "stardewvalley"), ("1453375253", "stardewvalley"), ("892970", "valheim"),
        ("1966720", "lethalcompany"), ("632360", "riskofrain2"), ("1091500", "cyberpunk2077"), ("1423049311", "cyberpunk2077"), ("292030", "witcher3"),
        ("1207664643", "witcher3"), ("1495134320", "witcher3"), ("379430", "kingdomcomedeliverance"), ("1719198803", "kingdomcomedeliverance"),
        ("1771300", "kingdomcomedeliverance2"), ("1248083010", "kingdomcomedeliverance2")] {
        assert_eq!(turbine_engine::nexus::domain_for_store_id(id), Some(domain), "{id}");
    }
    for (id, c) in [("1966720", "lethal-company"), ("892970", "valheim"), ("632360", "riskofrain2")] {
        assert_eq!(turbine_engine::thunderstore::community_for_steam_app(id), Some(c));
    }
    assert_eq!(turbine_engine::thunderstore::community_for_steam_app("413150"), None);
}

#[test]
fn player_file_cannot_override_builtin() {
    let dir = temp("override");
    fs::write(dir.join("mine.json"), r#"{"schema":1,"name":"My Skyrim","ids":{"steam":["489830"]},"modRoot":"Elsewhere"}"#).unwrap();
    fs::write(dir.join("ok.json"), r#"{"schema":1,"name":"Brass Pier","ids":{"steam":["3999902"]},"kind":"generic","nexus":"brasspier"}"#).unwrap();
    let problems = rules::load_player_games(&dir);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].starts_with("mine.json: left out"));
    let r = GameRules::for_game(&game(Store::Steam, "489830", Runtime::Windows));
    assert_eq!(r.mod_root, "Data");
    let mine = GameRules::for_game(&game(Store::Steam, "3999902", Runtime::Windows));
    assert_eq!((mine.support.as_str(), mine.support_level), ("Added by you", Support::Custom));
    assert_eq!(turbine_engine::nexus::domain_for(Store::Steam, "3999902"), Some("brasspier"));
    rules::load_player_games(&dir.join("none"));
    assert_eq!(GameRules::for_game(&game(Store::Steam, "3999902", Runtime::Windows)).support_level, Support::Basic);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn player_file_cannot_set_handler() {
    let e = parse_game("x.json", r#"{"schema":1,"name":"X","ids":{"steam":["1"]},"handler":"cyberpunk"}"#, false).unwrap_err();
    assert!(e.contains("handler"), "{e}");
    let e = parse_game("x.json", r#"{"schema":1,"name":"X","ids":{"steam":["1"]},"support":"full"}"#, false).unwrap_err();
    assert!(e.contains("full"), "{e}");
    assert!(parse_game("x.json", r#"{"schema":1,"name":"X","ids":{"steam":["1"]},"handler":"cyberpunk"}"#, true).is_ok());
}

#[test]
fn unknown_key_rejected() {
    let e = parse_game("x.json", r#"{"schema":1,"name":"X","ids":{"steam":["1"]},"modroot":"Data"}"#, true).unwrap_err();
    assert!(e.contains("modroot"), "{e}");
    assert!(parse_game("x.json", r#"{"schema":2,"name":"X","ids":{"steam":["1"]}}"#, true).is_err());
    assert!(parse_game("x.json", r#"{"schema":1,"name":"X","ids":{"steam":["1"],"origin":["2"]}}"#, true).is_err());
}

#[test]
fn duplicate_id_rejected() {
    let a = parse_game("a.json", r#"{"schema":1,"name":"A","ids":{"steam":["5"]}}"#, true).unwrap();
    let b = parse_game("b.json", r#"{"schema":1,"name":"B","ids":{"steam":["5"]}}"#, true).unwrap();
    let p = gamedata::validate(&[a.clone(), b], &[]);
    assert_eq!(p.len(), 1, "{p:?}");
    let c = parse_game("c.json", r#"{"schema":1,"name":"C","ids":{"gog":["5"]},"loader":{"use":"nope"}}"#, true).unwrap();
    let p = gamedata::validate(&[a, c], &[]);
    assert_eq!(p.len(), 1, "a GOG 5 isn't Steam 5, but the loader is unknown: {p:?}");
}

#[test]
fn dashes_rejected() {
    let text = format!(r#"{{"schema":1,"name":"Dash {} game","ids":{{"steam":["1"]}}}}"#, '\u{2014}');
    assert!(parse_game("x.json", &text, true).is_err());
    let text = format!(r#"{{"schema":1,"name":"X","ids":{{"steam":["1"]}},"notes":["a {} b"]}}"#, '\u{2013}');
    assert!(parse_game("x.json", &text, true).is_err());
}

#[test]
fn ids_digits_only() {
    assert!(parse_game("x.json", r#"{"schema":1,"name":"X","ids":{"steam":["12a"]}}"#, true).is_err());
    assert!(parse_game("x.json", r#"{"schema":1,"name":"X","ids":{"gog":["1-2"]}}"#, true).is_err());
    assert!(parse_game("x.json", r#"{"schema":1,"name":"X","ids":{"epic":["Fortnite"]}}"#, true).is_ok());
    assert!(parse_game("x.json", r#"{"schema":1,"name":"X","ids":{}}"#, true).is_err());
}

#[test]
fn paths_and_starters_checked() {
    assert!(parse_game("x.json", r#"{"schema":1,"name":"X","ids":{"steam":["1"]},"modRoot":"/abs"}"#, true).is_err());
    assert!(parse_game("x.json", r#"{"schema":1,"name":"X","ids":{"steam":["1"]},"saves":{"native":["../.ssh"]}}"#, true).is_err());
    assert!(parse_game("x.json", r#"{"schema":1,"name":"X","ids":{"steam":["1"]},"starters":[{"source":"nexus","id":"abc","name":"n","what":"w"}]}"#, true).is_err());
    let g = parse_game("x.json", r#"{"schema":1,"name":"X","ids":{"steam":["1"]},"starters":[{"source":"thunderstore","id":"Owner-Name","name":"n","what":"w"}]}"#, true).unwrap();
    assert_eq!(g.starters.len(), 1);
    // Every built-in game with a Nexus or Thunderstore page has starter mods of the right kind.
    for d in rules::games() {
        for s in &d.data.starters {
            assert!(s.source == "nexus" && d.data.nexus.is_some() || s.source == "thunderstore" && d.data.thunderstore.is_some(), "{}: {}", d.name, s.name);
        }
    }
}
