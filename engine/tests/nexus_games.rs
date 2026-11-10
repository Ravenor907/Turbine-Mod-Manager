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

/// 0.62: the owner's six games that ended as "ask (4 candidates)" and were never asked about. Names that differ
/// from Nexus's only by an edition or by punctuation are matched for sure; the rest get candidates that really
/// share the name's words (or none), never four games that only share "The".
#[test]
fn editions_and_punctuation_no_longer_end_in_a_question() {
    let g = |id: u64, name: &str, domain: &str, mods: u64| nexus_games::NexusGame { id, name: name.into(), domain: domain.into(), mods };
    let list = vec![
        g(1, "Slay the Princess", "slaytheprincess", 40),
        g(2, "The Outer Worlds", "theouterworlds", 900),
        g(3, "The Outer Worlds 2", "theouterworlds2", 300),
        g(4, "Fall Guys: Ultimate Knockout", "fallguys", 20),
        g(5, "Diablo", "diablo", 60),
        g(6, "Diablo II", "diablo2", 700),
        g(7, "Diablo II: Resurrected", "diablo2resurrected", 2000),
        g(8, "The Witcher 3", "witcher3", 7000),
        g(9, "The Sims 4", "thesims4", 5000),
        g(10, "The Long Dark", "thelongdark", 800),
        g(11, "Dark Souls", "darksouls", 3000),
        g(12, "Dark Souls Remastered", "darksoulsremastered", 2500),
        g(13, "A Hat in Time", "ahatintime", 100),
        g(14, "Trail Out", "trailout", 5),
    ];
    let sure = |name: &str| match nexus_games::match_name(name, &list) {
        Match::Sure(g) => g.domain,
        Match::Ask(c) => panic!("{name}: asked, with {:?}", c.iter().map(|g| g.name.clone()).collect::<Vec<_>>()),
    };
    let ask = |name: &str| match nexus_games::match_name(name, &list) {
        Match::Ask(c) => c.iter().map(|g| g.domain.clone()).collect::<Vec<_>>(),
        Match::Sure(g) => panic!("{name}: took {} for sure", g.name),
    };
    // An edition after a dash or a colon.
    assert_eq!(sure("Slay the Princess \u{2014} The Pristine Cut"), "slaytheprincess");
    assert_eq!(sure("Slay the Princess - The Pristine Cut"), "slaytheprincess");
    assert_eq!(sure("The Outer Worlds: Spacer's Choice Edition"), "theouterworlds");
    // Nexus's name carries a subtitle the store's doesn't: a question with that one game (it could be a sequel).
    assert_eq!(ask("Fall Guys"), vec!["fallguys".to_string()]);
    // Trailing edition words without a separator, and Game of the Year.
    assert_eq!(sure("The Outer Worlds Definitive Edition"), "theouterworlds");
    assert_eq!(sure("The Witcher 3 - Game of the Year Edition"), "witcher3");
    assert_eq!(sure("Slay the Princess (Complete Edition)"), "slaytheprincess");
    // An exact name still wins, also when an edition of it is a game of its own on Nexus.
    assert_eq!(sure("Dark Souls"), "darksouls");
    assert_eq!(sure("DARK SOULS\u{2122}: REMASTERED"), "darksoulsremastered");
    assert_eq!(sure("Diablo II"), "diablo2");
    // Two games that differ only by edition, and the store's name fits neither exactly: asked, those two first.
    assert_eq!(ask("Dark Souls: Prepare to Die Edition"), vec!["darksouls".to_string(), "darksoulsremastered".to_string()]);
    // A number is part of the name, never an edition.
    assert_eq!(nexus_games::base_name("The Outer Worlds 2"), "the outer worlds 2");
    assert_eq!(nexus_games::base_name("Diablo II: Resurrected"), "diablo ii resurrected");
    assert_eq!(nexus_games::base_name("Definitive Edition"), "definitive edition", "a name that is all edition words stays whole");
    // Not on Nexus under that name: candidates share the telling words, or there are none.
    assert_eq!(ask("Diablo + Hellfire"), vec!["diablo".to_string(), "diablo2".to_string(), "diablo2resurrected".to_string()], "the closest name first, whatever its mod count");
    assert!(ask("A Short Hike").is_empty(), "nothing shares Short or Hike; A doesn't count");
    assert_eq!(ask("The Oregon Trail"), vec!["trailout".to_string()], "The alone is not a match");
}
