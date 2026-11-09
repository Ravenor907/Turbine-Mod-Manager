//! 0.6: the Steam menu plugin's requests (quick.rs) and Mods off / Mods on in the store.

mod support;

use std::fs;
use support::{game, write_zip, TempDir};
use turbine_engine::games::Store as Launcher;
use turbine_engine::quick::{self, Action, Extra, Launch, Peek, Request};
use turbine_engine::store::{ModRules, Store};

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn req(list: &[&str]) -> Request {
    quick::parse_args(&args(list)).expect("a request").expect("readable")
}

#[test]
fn reads_each_request() {
    assert_eq!(quick::parse_args(&args(&[])), None);
    assert_eq!(quick::parse_args(&args(&["nxm://skyrimspecialedition/mods/1/files/2"])), None);
    assert_eq!(quick::parse_args(&args(&["--screenshot=x.png"])), None);

    let r = req(&["--status", "489830", "--json"]);
    assert_eq!((r.action, r.game.as_str(), r.json, r.anyway), (Action::Status, "489830", true, false));
    assert_eq!(req(&["--mods", "off", "steam:489830", "--json"]).action, Action::ModsOff);
    assert_eq!(req(&["--mods", "ON", "steam:489830"]).action, Action::ModsOn);
    assert_eq!(req(&["--mods=on", "--game=steam:489830"]).game, "steam:489830");
    assert!(req(&["--play", "1245620", "--anyway", "--json"]).anyway);
    assert_eq!(req(&["--play-coop", "1245620"]).action, Action::PlayCoop);
    assert_eq!(req(&["--play-plain", "1245620"]).action, Action::PlayPlain);
    assert_eq!(req(&["--open", "steam:413150"]).action, Action::Open);
    assert_eq!(req(&["--report", "steam:413150"]).action, Action::Report);
    // A name in several words (as the shell passes it).
    assert_eq!(req(&["--status", "Stardew", "Valley", "--json"]).game, "Stardew Valley");
    assert_eq!(req(&["--status=Elden Ring"]).game, "Elden Ring");
}

#[test]
fn refuses_requests_it_cant_read() {
    for bad in [&["--status"][..], &["--mods", "maybe", "1"], &["--mods"], &["--play", "--json"], &["--play", "1", "--open", "2"]] {
        let r = quick::parse_args(&args(bad));
        assert!(matches!(r, Some(Err(_))), "{bad:?} gave {r:?}");
    }
    let long = "x".repeat(300);
    assert!(matches!(quick::parse_args(&args(&["--status", &long])), Some(Err(_))));
    assert!(matches!(quick::parse_args(&args(&["--status", "a\u{7}b"])), Some(Err(_))));
}

#[test]
fn only_changes_need_the_lock() {
    assert!(!Action::Status.changes() && !Action::Open.changes() && !Action::Report.changes());
    assert!(Action::Play.changes() && Action::PlayCoop.changes() && Action::PlayPlain.changes());
    assert!(Action::ModsOn.changes() && Action::ModsOff.changes());
}

#[test]
fn finds_the_game() {
    let t = TempDir::new("quick-find");
    let games = vec![
        game(Launcher::Steam, "489830", "Skyrim Special Edition", &t.path("lib/a"), None),
        game(Launcher::Steam, "1245620", "ELDEN RING", &t.path("lib/b"), None),
        game(Launcher::Gog, "1453375253", "Stardew Valley", &t.path("lib/c"), None),
        game(Launcher::Epic, "Fortnite", "Hades", &t.path("lib/d"), None),
    ];
    let shortcuts = vec![(3_000_000_001u32, "Stardew Valley".to_string()), (3_000_000_002, "Hades".to_string())];
    let id = |q: &str| quick::find_game(&games, q, &shortcuts).map(|g| g.id());
    assert_eq!(id("steam:489830").as_deref(), Some("steam:489830"));
    assert_eq!(id("1245620").as_deref(), Some("steam:1245620"));
    assert_eq!(id("Elden Ring").as_deref(), Some("steam:1245620"));
    assert_eq!(id("elden-ring").as_deref(), Some("steam:1245620"));
    assert_eq!(id("gog:1453375253").as_deref(), Some("gog:1453375253"));
    // A Heroic game added to Steam: Steam runs it under the shortcut's app id (or its 64-bit game id).
    assert_eq!(id("3000000001").as_deref(), Some("gog:1453375253"));
    let big = turbine_engine::shortcuts::game_id(3_000_000_002).to_string();
    assert_eq!(id(&big).as_deref(), Some("epic:Fortnite"));
    assert_eq!(id("999"), None);
    assert_eq!(id("Some Other Game"), None);
    assert_eq!(id(""), None);
    assert_eq!(id("!!!"), None);
}

#[test]
fn socket_lines_go_both_ways() {
    let line = quick::socket_line(Action::PlayCoop, "steam:1245620");
    assert_eq!(line, "turbine:quick play-coop steam:1245620");
    assert_eq!(quick::parse_socket_line(&line), Some((Action::PlayCoop, "steam:1245620".to_string())));
    let odd = quick::socket_line(Action::Open, "sideload:my game/1");
    assert_eq!(odd.matches(' ').count(), 2, "{odd}");
    assert_eq!(quick::parse_socket_line(&odd), Some((Action::Open, "sideload:my game/1".to_string())));
    for a in [Action::ModsOn, Action::ModsOff, Action::Play, Action::PlayPlain, Action::Report] {
        assert_eq!(quick::parse_socket_line(&quick::socket_line(a, "steam:1")).map(|x| x.0), Some(a));
    }
    // Status is never sent; anything else is dropped.
    assert_eq!(quick::parse_socket_line(&quick::socket_line(Action::Status, "steam:1")), None);
    for bad in ["turbine:show", "turbine:quick", "turbine:quick play", "turbine:quick run steam:1", "turbine:quick play nogame", "turbine:quick play steam:1 extra", "turbine:quick play %ZZ", "nxm://x"] {
        assert_eq!(quick::parse_socket_line(bad), None, "{bad}");
    }
    assert_eq!(quick::decode("a%20b"), Some("a b".into()));
    assert_eq!(quick::decode("a%2"), None);
}

#[test]
fn a_request_waits_for_the_next_turbine() {
    let t = TempDir::new("quick-request");
    let dir = t.path("data");
    quick::write_request(&dir, Action::Report, "steam:489830", 1000).unwrap();
    assert_eq!(quick::take_request(&dir, 1010).as_deref(), Some("turbine:quick report steam:489830"));
    // Taken once.
    assert_eq!(quick::take_request(&dir, 1010), None);
    // Too old: dropped, and the file goes.
    quick::write_request(&dir, Action::Open, "steam:489830", 1000).unwrap();
    assert_eq!(quick::take_request(&dir, 1000 + quick::REQUEST_MAX_AGE + 1), None);
    assert!(!dir.join(quick::REQUEST_FILE).exists());
    // Damaged: dropped.
    fs::write(dir.join(quick::REQUEST_FILE), "soon\nturbine:quick open steam:1\n").unwrap();
    assert_eq!(quick::take_request(&dir, 1000), None);
}

#[test]
fn the_last_launch_is_kept() {
    let t = TempDir::new("quick-launch");
    let root = t.path("store");
    fs::create_dir_all(&root).unwrap();
    assert_eq!(quick::read_launch(&root), None);
    // No launch on record: an update does nothing.
    quick::update_launch(&root, "closed", Some(3)).unwrap();
    assert_eq!(quick::read_launch(&root), None);
    let l = Launch { at: 1_700_000_000, how: "coop".into(), result: "started".into(), seconds: None, words: "Ready to play".into() };
    quick::write_launch(&root, &l).unwrap();
    assert_eq!(quick::read_launch(&root), Some(l.clone()));
    quick::update_launch(&root, "closed", Some(12)).unwrap();
    let back = quick::read_launch(&root).unwrap();
    assert_eq!((back.result.as_str(), back.seconds, back.how.as_str()), ("closed", Some(12), "coop"));
}

struct Flat;
impl ModRules for Flat {
    fn map_archive(&self, files: &[String], _mod_name: &str) -> Result<Vec<(String, String)>, String> {
        Ok(files.iter().map(|f| (f.clone(), format!("Data/{f}"))).collect())
    }
    fn needs_loader(&self, _game_files: &[String]) -> bool {
        false
    }
}

#[test]
fn mods_off_then_on_brings_back_exactly_those() {
    let t = TempDir::new("quick-hold");
    let install = t.path("library/steamapps/common/Game");
    fs::create_dir_all(install.join("Data")).unwrap();
    let g = game(Launcher::Steam, "4242", "Test Game", &install, None);
    let data = t.path("data");
    let mut s = Store::open_with(&g, &data, Box::new(Flat)).unwrap();
    let mut add = |name: &str, file: &str| {
        let z = t.path(&format!("Downloads/{name}.zip"));
        write_zip(&z, &[(file, &b"x"[..])]);
        s.import_zip(&z).unwrap().id
    };
    let a = add("Alpha", "a.txt");
    let b = add("Beta", "b.txt");
    let c = add("Gamma", "c.txt");
    s.set_enabled(&a, true).unwrap();
    s.set_enabled(&b, true).unwrap();
    s.set_enabled(&c, false).unwrap();
    s.apply().unwrap();
    assert!(install.join("Data/a.txt").exists());

    // Off: everything off, the two that were on are kept, and the game folder is clean after apply.
    assert_eq!(s.hold_all_off().unwrap(), 2);
    assert!(s.mods().iter().all(|m| !m.enabled));
    assert_eq!(s.held_off(), vec![a.clone(), b.clone()]);
    s.apply().unwrap();
    assert!(!install.join("Data/a.txt").exists() && !install.join("Data/b.txt").exists());
    // Off again with nothing on keeps the list.
    assert_eq!(s.hold_all_off().unwrap(), 0);
    assert_eq!(s.held_off().len(), 2);

    // The plugin's read-only look sees the same.
    let p = quick::peek(s.root());
    assert_eq!(p, Peek { mods: 3, on: 0, held: 2, seamless: false, seamless_on: false });

    // Reopened (as the next run sees it): still held.
    drop(s);
    let mut s = Store::open_with(&g, &data, Box::new(Flat)).unwrap();
    assert_eq!(s.held_off().len(), 2);

    // On: exactly those two, Gamma stays off.
    assert_eq!(s.release_held().unwrap(), 2);
    let on: Vec<&str> = s.mods().iter().filter(|m| m.enabled).map(|m| m.id.as_str()).collect();
    assert_eq!(on, vec![a.as_str(), b.as_str()]);
    assert!(s.held_off().is_empty());
    s.apply().unwrap();
    assert!(install.join("Data/a.txt").exists() && !install.join("Data/c.txt").exists());

    // A mod turned on by hand ends Mods off (the old list would undo the player's choice).
    s.hold_all_off().unwrap();
    s.set_enabled(&c, true).unwrap();
    assert!(s.held_off().is_empty());
    assert_eq!(s.release_held().unwrap(), 0);
    let on: Vec<&str> = s.mods().iter().filter(|m| m.enabled).map(|m| m.id.as_str()).collect();
    assert_eq!(on, vec![c.as_str()]);
}

#[test]
fn status_says_what_the_panel_shows() {
    let t = TempDir::new("quick-status");
    let g = game(Launcher::Steam, "1245620", "ELDEN RING", &t.path("lib/er"), None);
    let root = t.path("store");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("state.json"),
        r#"{"version":1,"mods":[{"id":"m1","name":"A","enabled":true},{"id":"sc","name":"Seamless Co-op","enabled":false},{"id":"m2","name":"B","enabled":false}],"sources":{"sc":{"kind":"loader","id":"seamless"}}}"#,
    )
    .unwrap();
    let p = quick::peek(&root);
    assert_eq!(p, Peek { mods: 3, on: 1, held: 0, seamless: true, seamless_on: false });
    let x = Extra { has_store: true, running: Some(false), turbine_open: false, turbine_url: "steam://rungameid/1".into(), anti_cheat: "me3".into(), coop_game: true, words: String::new() };
    let l = Launch { at: 5, how: "mods".into(), result: "closed".into(), seconds: Some(8), words: String::new() };
    let j = turbine_engine::json::parse(&quick::status_json(&g, &p, Some(&l), &x)).unwrap();
    let b = |k: &str| j.get(k).and_then(|v| v.as_bool());
    assert_eq!(b("ok"), Some(true));
    assert_eq!(b("modded"), Some(true));
    assert_eq!(b("modsOn"), Some(true));
    assert_eq!(b("coop"), Some(true));
    assert_eq!(b("refused"), Some(false));
    assert_eq!(b("running"), Some(false));
    assert_eq!(j.get("game").and_then(|g| g.str_at("id")), Some("steam:1245620"));
    assert_eq!(j.get("lastLaunch").and_then(|l| l.str_at("result")), Some("closed"));

    // Online-only anti-cheat: never co-op, never "modded" words beyond the count.
    let x2 = Extra { anti_cheat: "refuse".into(), coop_game: true, has_store: true, ..Extra::default() };
    let j = turbine_engine::json::parse(&quick::status_json(&g, &p, None, &x2)).unwrap();
    assert_eq!(j.get("coop").and_then(|v| v.as_bool()), Some(false));
    assert_eq!(j.get("refused").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(j.get("running"), Some(&turbine_engine::json::Json::Null));
    assert_eq!(j.get("lastLaunch"), Some(&turbine_engine::json::Json::Null));

    // A game with no store yet: not modded.
    let none = quick::peek(&t.path("nothing-here"));
    assert_eq!(none, Peek::default());
    let j = turbine_engine::json::parse(&quick::status_json(&g, &none, None, &Extra::default())).unwrap();
    assert_eq!(j.get("modded").and_then(|v| v.as_bool()), Some(false));

    let nf = turbine_engine::json::parse(&quick::not_found_json("12\"3")).unwrap();
    assert_eq!(nf.get("found").and_then(|v| v.as_bool()), Some(false));
    assert_eq!(nf.str_at("query"), Some("12\"3"));
    let a = turbine_engine::json::parse(&quick::answer_json(false, "Close it first.", ",\"busy\":true")).unwrap();
    assert_eq!(a.get("busy").and_then(|v| v.as_bool()), Some(true));
}
