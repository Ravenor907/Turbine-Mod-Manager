//! Skyrim SE plugins: headers, the load order files in the Proton prefix, sorting and missing masters.

mod support;

use std::fs;
use support::{game, plugin_bytes, TempDir};
use turbine_engine::games::Store as Launcher;
use turbine_engine::plugins::{self, parse_header, Files, LoadOrder, FLAG_LIGHT, FLAG_MASTER};
use turbine_engine::rules::{self, pre_play_with, GameRules};

#[test]
fn reads_plugin_headers() {
    let h = parse_header(&plugin_bytes(FLAG_MASTER | FLAG_LIGHT, &["Skyrim.esm", "Update.esm"])).unwrap();
    assert_eq!(h.flags & FLAG_MASTER, FLAG_MASTER);
    assert_eq!(h.flags & FLAG_LIGHT, FLAG_LIGHT);
    assert_eq!(h.masters, vec!["Skyrim.esm", "Update.esm"]);
    assert_eq!(h.author.as_deref(), Some("Someone"));
    assert!(parse_header(b"TES3xxxxxxxxxxxxxxxxxxxxxxxxxx").is_err());
    // A header that says it's longer than the file.
    let mut b = plugin_bytes(0, &["Skyrim.esm"]);
    b[4] = 0xFF;
    b[5] = 0xFF;
    assert!(parse_header(&b).is_err());
    // A subrecord running past the header.
    let mut b = plugin_bytes(0, &[]);
    let n = 24 + 4;
    b[n] = 0xFF;
    assert!(parse_header(&b).is_err());
}

#[test]
fn xxxx_gives_the_next_size() {
    let mut sub = Vec::new();
    sub.extend_from_slice(b"XXXX");
    sub.extend_from_slice(&4u16.to_le_bytes());
    sub.extend_from_slice(&11u32.to_le_bytes());
    sub.extend_from_slice(b"MAST");
    sub.extend_from_slice(&0u16.to_le_bytes());
    sub.extend_from_slice(b"Skyrim.esm\0");
    let mut b = b"TES4".to_vec();
    b.extend_from_slice(&(sub.len() as u32).to_le_bytes());
    b.extend_from_slice(&[0; 16]);
    b.extend_from_slice(&sub);
    assert_eq!(parse_header(&b).unwrap().masters, vec!["Skyrim.esm"]);
}

struct Setup {
    t: TempDir,
}

impl Setup {
    fn new() -> Setup {
        let t = TempDir::new("plugins");
        for base in plugins::BASE_PLUGINS {
            t.write(&format!("library/Skyrim/Data/{base}"), &plugin_bytes(FLAG_MASTER, if *base == "Skyrim.esm" { &[] } else { &["Skyrim.esm"] }));
        }
        fs::create_dir_all(t.path("pfx/drive_c/users/steamuser")).unwrap();
        Setup { t }
    }
    fn plugin(&self, name: &str, flags: u32, masters: &[&str]) {
        self.t.write(&format!("library/Skyrim/Data/{name}"), &plugin_bytes(flags, masters));
    }
    fn settings(&self) -> std::path::PathBuf {
        self.t.path("pfx/drive_c/users/steamuser/AppData/Local/Skyrim Special Edition")
    }
    fn game(&self) -> turbine_engine::Game {
        let mut g = game(Launcher::Steam, "489830", "Skyrim Special Edition", &self.t.path("library/Skyrim"), Some(self.t.path("pfx")));
        g.runtime = turbine_engine::Runtime::Windows;
        g
    }
}

#[test]
fn load_order_from_data_ccc_and_plugins_txt() {
    let s = Setup::new();
    s.plugin("ccBGSSSE001-Fish.esm", FLAG_MASTER | FLAG_LIGHT, &["Skyrim.esm"]);
    s.plugin("ccQDRSSE001-SurvivalMode.esl", FLAG_MASTER | FLAG_LIGHT, &["Skyrim.esm"]);
    s.t.write("library/Skyrim/Skyrim.ccc", b"ccQDRSSE001-SurvivalMode.esl\r\nccBGSSSE001-Fish.esm\r\nccNotHere.esl\r\n");
    s.plugin("SkyUI_SE.esp", 0, &["Skyrim.esm"]);
    s.plugin("Lib.esm", FLAG_MASTER, &["Skyrim.esm"]);
    s.plugin("Patch.esp", 0, &["SkyUI_SE.esp", "Lib.esm"]);
    s.plugin("Light.esp", FLAG_LIGHT, &["Skyrim.esm"]);
    s.plugin("New.esp", 0, &[]);
    fs::create_dir_all(s.settings()).unwrap();
    // The player's file: CRLF, a comment, a plugin that's gone, odd case.
    fs::write(s.settings().join("plugins.txt"), b"# comment\r\n*patch.esp\r\n*SkyUI_SE.esp\r\nLight.esp\r\n*Gone.esp\r\n*Lib.esm\r\n").unwrap();

    let lo = LoadOrder::read(&s.game()).unwrap();
    let names: Vec<&str> = lo.plugins().iter().map(|p| p.name.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "Skyrim.esm",
            "Update.esm",
            "Dawnguard.esm",
            "HearthFires.esm",
            "Dragonborn.esm",
            "ccQDRSSE001-SurvivalMode.esl",
            "ccBGSSSE001-Fish.esm",
            // Masters go before the rest whatever plugins.txt says.
            "Lib.esm",
            "Patch.esp",
            "SkyUI_SE.esp",
            "Light.esp",
            // Not in plugins.txt yet: at the end, turned on.
            "New.esp",
        ]
    );
    let p = |n: &str| lo.plugins().iter().find(|p| p.name == n).unwrap().clone();
    assert!(p("Skyrim.esm").fixed && p("Skyrim.esm").active);
    assert!(p("ccBGSSSE001-Fish.esm").fixed);
    assert!(p("Light.esp").light && !p("Light.esp").master && !p("Light.esp").active);
    assert!(p("ccQDRSSE001-SurvivalMode.esl").master);
    assert!(p("New.esp").active);

    // Patch.esp loads before its master SkyUI_SE.esp: sorting fixes that and keeps the rest.
    let mut lo2 = lo.clone();
    assert!(lo2.sort());
    let names: Vec<&str> = lo2.plugins().iter().skip(7).map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["Lib.esm", "SkyUI_SE.esp", "Patch.esp", "Light.esp", "New.esp"]);
    assert!(!lo2.sort(), "sorting again changes nothing");

    // Writing: plugins.txt without the always-loaded ones, '*' for on, the gone plugin kept; a backup the first time.
    lo2.save().unwrap();
    let txt = fs::read_to_string(s.settings().join("plugins.txt")).unwrap();
    assert_eq!(txt, "*Lib.esm\r\n*SkyUI_SE.esp\r\n*Patch.esp\r\nLight.esp\r\n*New.esp\r\n*Gone.esp\r\n");
    let backup = s.settings().join("plugins.txt.before-turbine");
    assert!(fs::read_to_string(&backup).unwrap().starts_with("# comment"));
    let order = fs::read_to_string(s.settings().join("loadorder.txt")).unwrap();
    assert!(order.starts_with("Skyrim.esm\r\nUpdate.esm\r\n") && order.ends_with("New.esp\r\n"));
    lo2.save().unwrap();
    assert!(fs::read_to_string(&backup).unwrap().starts_with("# comment"), "the backup is only made once");
    assert!(!s.settings().join("plugins.turbine-tmp").exists());

    // Reading back gives the same order.
    let again = LoadOrder::read(&s.game()).unwrap();
    assert_eq!(again.plugins_txt(), lo2.plugins_txt());

    // The gone plugin comes back where the list ends, turned on.
    s.plugin("Gone.esp", 0, &[]);
    let back = LoadOrder::read(&s.game()).unwrap();
    assert!(back.plugins().iter().any(|p| p.name == "Gone.esp" && p.active));
}

#[test]
fn move_toggle_and_missing_masters() {
    let s = Setup::new();
    s.plugin("A.esp", 0, &["Skyrim.esm"]);
    s.plugin("B.esp", 0, &["A.esp"]);
    s.plugin("M.esm", FLAG_MASTER, &[]);
    s.plugin("Needs.esp", 0, &["Missing.esm", "Skyrim.esm"]);
    let mut lo = LoadOrder::read(&s.game()).unwrap();
    // Can't move or turn off the game's own plugins.
    assert!(lo.move_to("Skyrim.esm", 3).is_err());
    assert!(lo.set_active("Update.esm", false).is_err());
    // A plugin can't go above the masters, a master can't go below the plugins.
    assert_eq!(lo.move_to("B.esp", 0).unwrap(), 6);
    assert_eq!(lo.move_to("M.esm", 100).unwrap(), 5);
    assert!(lo.move_to("Nope.esp", 1).is_err());

    let miss = lo.missing_masters();
    assert_eq!(miss.len(), 1);
    assert_eq!((miss[0].plugin.as_str(), miss[0].master.as_str(), miss[0].turned_off), ("Needs.esp", "Missing.esm", false));
    lo.set_active("A.esp", false).unwrap();
    let miss = lo.missing_masters();
    assert!(miss.iter().any(|m| m.plugin == "B.esp" && m.master == "A.esp" && m.turned_off));
    lo.set_active("Needs.esp", false).unwrap();
    lo.set_active("B.esp", false).unwrap();
    assert!(lo.missing_masters().is_empty());

    let json = lo.to_json();
    assert!(turbine_engine::json::parse(&json).is_ok());
    assert!(json.contains("\"name\":\"Needs.esp\",\"active\":false"));
}

#[test]
fn missing_master_stops_play() {
    let s = Setup::new();
    s.plugin("Needs.esp", 0, &["Missing.esm"]);
    s.plugin("Off.esm", FLAG_MASTER, &[]);
    s.plugin("UsesOff.esp", 0, &["Off.esm"]);
    fs::create_dir_all(s.settings()).unwrap();
    fs::write(s.settings().join("plugins.txt"), "Off.esm\n*Needs.esp\n*UsesOff.esp\n").unwrap();
    let g = s.game();
    let r = GameRules::for_game(&g);
    let checks = rules::pre_play(&g, &r, Some("100"), false, Some(100 << 30));
    let stops: Vec<_> = checks.iter().filter(|c| c.title == "Missing master").collect();
    assert_eq!(stops.len(), 2, "{checks:?}");
    assert!(stops.iter().all(|c| c.level == "stop"));
    assert!(stops[0].detail.contains("Needs.esp needs Missing.esm, which isn't in the game's Data folder"));
    assert!(stops[1].detail.contains("UsesOff.esp needs Off.esm, which is turned off"));
    assert!(!checks.iter().any(|c| c.level == "ok"));
    // The plain check without plugins is unchanged.
    let plain = pre_play_with(&g, &r, Some("100"), false, Some(100 << 30), Some("ext4"), true);
    assert_eq!(plain[0].level, "ok");
}

#[test]
fn no_prefix_means_no_writing() {
    let s = Setup::new();
    let mut g = s.game();
    g.prefix = None;
    let lo = LoadOrder::read(&g).unwrap();
    assert!(lo.save().unwrap_err().to_string().contains("Start it once"));
    // A prefix path that isn't there yet: refused, nothing made.
    g.prefix = Some(s.t.path("nope/pfx"));
    let lo = LoadOrder::read(&g).unwrap();
    assert!(lo.save().is_err());
    assert!(!s.t.path("nope").exists());
    // GOG keeps its settings in its own folder.
    let mut gog = s.game();
    gog.store = Launcher::Gog;
    assert!(Files::for_game(&gog).settings.unwrap().ends_with("Skyrim Special Edition GOG"));
    // A missing Data folder is an error, not an empty list.
    let mut gone = s.game();
    gone.install_dir = s.t.path("nowhere");
    assert!(LoadOrder::read(&gone).is_err());
}

#[test]
fn sync_writes_only_when_needed() {
    let s = Setup::new();
    s.plugin("A.esp", 0, &[]);
    let lo = plugins::sync(&s.game()).unwrap();
    assert_eq!(lo.plugins().len(), 6);
    let path = s.settings().join("plugins.txt");
    assert_eq!(fs::read_to_string(&path).unwrap(), "*A.esp\r\n");
    let before = fs::metadata(&path).unwrap().modified().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    plugins::sync(&s.game()).unwrap();
    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), before);
}

#[test]
fn windows_1252_names_round_trip() {
    let s = Setup::new();
    s.plugin("Café.esp", 0, &[]);
    fs::create_dir_all(s.settings()).unwrap();
    fs::write(s.settings().join("plugins.txt"), b"Caf\xe9.esp\r\n").unwrap();
    let lo = LoadOrder::read(&s.game()).unwrap();
    let p = lo.plugins().iter().find(|p| p.name == "Café.esp").unwrap();
    assert!(!p.active, "read from the 1252 file as off");
    assert_eq!(lo.plugins_txt(), b"Caf\xe9.esp\r\n".to_vec());
}
