//! Finding a game's Nexus Mods domain: Nexus's public game list, exact name matches, the player's picks, and the
//! GraphQL game search behind "Which game on Nexus is this?".

use std::path::PathBuf;
use turbine_engine::games::{Game, Runtime, Store};
use turbine_engine::json;
use turbine_engine::nexus_games::{self, Answer, Match, Picks};

fn list() -> Vec<nexus_games::NexusGame> {
    nexus_games::parse_list(include_str!("data/nexus_games_list.json")).unwrap()
}

fn game(id: &str, name: &str) -> Game {
    Game {
        store: Store::Steam,
        store_id: id.into(),
        name: name.into(),
        install_dir: PathBuf::from("/nonexistent"),
        library: PathBuf::from("/nonexistent"),
        build: None,
        runtime: Runtime::Windows,
        cover: None,
        header: None,
        prefix: None,
    }
}

fn temp(tag: &str) -> PathBuf {
    let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let p = std::env::temp_dir().join(format!("turbine-nexusgames-{tag}-{n}"));
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn parse_static_list() {
    let l = list();
    assert_eq!(l.len(), 8, "entries without a usable domain are left out");
    assert_eq!(l[0].domain, "skyrimspecialedition");
    assert_eq!((l[0].id, l[0].mods), (1704, 110000));
    assert!(nexus_games::parse_list("{}").is_err());
}

#[test]
fn normalize_names() {
    assert_eq!(nexus_games::normalize("Knights & Dragons\u{2122}"), "knights and dragons");
    assert_eq!(nexus_games::normalize("The Witcher 3: Wild Hunt"), "the witcher 3 wild hunt");
    assert_eq!(nexus_games::normalize("  Baldur's   Gate 3 "), "baldurs gate 3");
    assert_eq!(nexus_games::normalize("Skyrim\u{00AE} Special Edition"), "skyrim special edition");
}

#[test]
fn exact_match_sure() {
    match nexus_games::match_name("Brass Pier", &list()) {
        Match::Sure(g) => assert_eq!(g.domain, "brasspier"),
        other => panic!("{other:?}"),
    }
    assert!(matches!(nexus_games::match_name("KNIGHTS AND DRAGONS", &list()), Match::Sure(_)));
}

#[test]
fn subtitle_only_is_ask() {
    match nexus_games::match_name("The Witcher 3: Wild Hunt", &list()) {
        Match::Ask(c) => assert_eq!(c[0].domain, "witcher3"),
        other => panic!("{other:?}"),
    }
    match nexus_games::match_name("Lantern Hollow", &list()) {
        // Both share two words; the one with more mods first.
        Match::Ask(c) => assert_eq!(c.iter().map(|g| g.domain.as_str()).collect::<Vec<_>>(), vec!["lanternhollow", "lanternhollowremastered"]),
        other => panic!("{other:?}"),
    }
}

#[test]
fn two_exact_matches_is_ask() {
    match nexus_games::match_name("Twin Game", &list()) {
        Match::Ask(c) => assert_eq!(c.len(), 2),
        other => panic!("{other:?}"),
    }
}

#[test]
fn saved_pick_wins() {
    let dir = temp("picks");
    let mut p = Picks::load(&dir);
    p.set("steam:3999901", Some("lanternhollow"));
    p.set("steam:3999905", None);
    p.save(&dir).unwrap();
    let p = Picks::load(&dir);
    let l = list();
    match nexus_games::domain_for_game(&game("3999901", "Lantern Hollow"), &p, Some(&l)) {
        Answer::Sure { domain, how, .. } => assert_eq!((domain.as_str(), how), ("lanternhollow", "your pick")),
        other => panic!("{other:?}"),
    }
    assert_eq!(nexus_games::domain_for_game(&game("3999905", "Brass Pier"), &p, Some(&l)), Answer::NotOnNexus);
    match nexus_games::domain_for_game(&game("3999902", "Brass Pier"), &p, Some(&l)) {
        Answer::Sure { domain, how, .. } => assert_eq!((domain.as_str(), how), ("brasspier", "name match")),
        other => panic!("{other:?}"),
    }
    assert_eq!(nexus_games::domain_for_game(&game("3999902", "Brass Pier"), &p, None), Answer::Unknown);
    // A bad domain in the file is never read back.
    std::fs::write(Picks::path(&dir), r#"{"steam:1":"../evil","steam:2":"ok"}"#).unwrap();
    assert_eq!(Picks::load(&dir).items, vec![("steam:2".to_string(), Some("ok".to_string()))]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn data_file_domain_wins_over_pick() {
    let mut p = Picks::default();
    p.set("steam:990080", Some("somethingelse"));
    match nexus_games::domain_for_game(&game("990080", "Hogwarts Legacy"), &p, Some(&list())) {
        Answer::Sure { domain, how, .. } => assert_eq!((domain.as_str(), how), ("hogwartslegacy", "data file")),
        other => panic!("{other:?}"),
    }
}

#[test]
fn graphql_games_search_body() {
    let body = nexus_games::games_search_body("lantern");
    let v = json::parse(&body).unwrap();
    let q = v.str_at("query").unwrap();
    assert!(q.contains("games(filter: $f, count: $count)") && q.contains("domainName"));
    let f = v.get("variables").unwrap().get("f").unwrap().get("name").unwrap().as_array();
    assert_eq!((f[0].str_at("value"), f[0].str_at("op")), (Some("lantern"), Some("WILDCARD")));
    let answer = r#"{"data":{"games":{"nodes":[{"id":9002,"name":"Lantern Hollow: The Long Night","domainName":"lanternhollow","modCount":17}]}}}"#;
    assert_eq!(nexus_games::parse_games_search(answer).unwrap()[0].domain, "lanternhollow");
    assert!(nexus_games::parse_games_search(r#"{"errors":[{"message":"no"}]}"#).is_err());
}

#[test]
fn reverse_lookup_domain_to_installed_game() {
    let games = vec![game("489830", "The Elder Scrolls V: Skyrim Special Edition"), game("3999902", "Brass Pier")];
    let l = list();
    let p = Picks::default();
    assert_eq!(nexus_games::game_for_domain("brasspier", &games, &p, Some(&l)).unwrap().store_id, "3999902");
    assert_eq!(nexus_games::game_for_domain("skyrimspecialedition", &games, &p, Some(&l)).unwrap().store_id, "489830");
    assert!(nexus_games::game_for_domain("witcher3", &games, &p, Some(&l)).is_none());
    assert!(nexus_games::answer_json(&Answer::Unknown).contains("\"state\":\"unknown\""));
}
