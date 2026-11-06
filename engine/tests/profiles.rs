//! Profiles: named setups per game, switching, and share files.

mod support;

use std::fs;
use support::{game, write_zip, TempDir};
use turbine_engine::games::Store as Launcher;
use turbine_engine::profiles::{self, Profiles, DEFAULT_PROFILE};
use turbine_engine::store::{Import, ModRules, SourceInfo, Store};

/// Everything goes to the game folder as it is.
struct Plain;
impl ModRules for Plain {
    fn map_archive(&self, files: &[String], _: &str) -> Result<Vec<(String, String)>, String> {
        Ok(files.iter().map(|f| (f.clone(), f.clone())).collect())
    }
    fn needs_loader(&self, _: &[String]) -> bool {
        false
    }
}

struct World {
    t: TempDir,
    store: Store,
}

fn world() -> World {
    let t = TempDir::new("profiles");
    let dir = t.path("lib/Game");
    fs::create_dir_all(&dir).unwrap();
    let g = game(Launcher::Steam, "1966720", "Lethal Company", &dir, None);
    let mut store = Store::open_with(&g, &t.path("data"), Box::new(Plain)).unwrap();
    for (file, inside, src) in [
        ("A.zip", "a.txt", SourceInfo { kind: "thunderstore".into(), id: "Owner-Alpha".into(), version: Some("1.0.2".into()), ..SourceInfo::default() }),
        ("B.zip", "b.txt", SourceInfo { kind: "zip".into(), ..SourceInfo::default() }),
        ("C.zip", "c.txt", SourceInfo { kind: "nexus".into(), id: "4242".into(), version: Some("2.1".into()), ..SourceInfo::default() }),
    ] {
        let p = t.path(&format!("dl/{file}"));
        write_zip(&p, &[(inside, b"x")]);
        let Import::Added(_) = store.import_archive(&p, src).unwrap() else { panic!() };
    }
    World { t, store }
}

#[test]
fn switching_saves_and_loads_setups() {
    let mut w = world();
    let mut p = Profiles::open(&w.store).unwrap();
    assert_eq!(p.names(), vec![DEFAULT_PROFILE]);
    assert_eq!(p.active(), DEFAULT_PROFILE);
    w.store.apply().unwrap();
    let game_dir = w.store.game().install_dir.clone();
    assert!(game_dir.join("a.txt").exists());

    p.create(&w.store, "  Only B ").unwrap();
    assert!(p.create(&w.store, "only b").is_err(), "names are unique ignoring case");
    assert!(p.create(&w.store, "   ").is_err());

    // Switch, then change the setup there.
    p.switch(&mut w.store, "Only B").unwrap();
    assert_eq!(p.active(), "Only B");
    w.store.set_enabled("alpha", false).unwrap();
    w.store.set_enabled("c", false).unwrap();
    w.store.move_to("b", 0).unwrap();
    w.store.apply().unwrap();
    assert!(!game_dir.join("a.txt").exists());

    // Back to Default: everything on in the old order, and applied.
    let report = p.switch(&mut w.store, DEFAULT_PROFILE).unwrap();
    assert!(report.added > 0);
    assert_eq!(w.store.order(), vec![("alpha".to_string(), true), ("b".to_string(), true), ("c".to_string(), true)]);
    assert!(game_dir.join("a.txt").exists());

    // Saved to disk: reopening knows both, and "Only B" kept its setup.
    let p2 = Profiles::open(&w.store).unwrap();
    assert_eq!(p2.names(), vec![DEFAULT_PROFILE, "Only B"]);
    assert_eq!(p2.order_of(&w.store, "Only B").unwrap(), vec![("b".to_string(), true), ("alpha".to_string(), false), ("c".to_string(), false)]);
    assert!(turbine_engine::json::parse(&p2.to_json()).is_ok());

    // Rename and delete.
    let mut p3 = p2;
    assert!(p3.delete(DEFAULT_PROFILE).is_err(), "in use");
    p3.rename("only b", "Just B").unwrap();
    p3.delete("Just B").unwrap();
    assert_eq!(p3.names(), vec![DEFAULT_PROFILE]);
    assert!(p3.switch(&mut w.store, "Just B").is_err());
}

#[test]
fn share_file_round_trip() {
    let mut w = world();
    w.store.set_enabled("b", false).unwrap();
    let p = Profiles::open(&w.store).unwrap();
    let home = w.t.path("home");
    let path = profiles::export(&p, &w.store, DEFAULT_PROFILE, &home).unwrap();
    assert!(path.starts_with(home.join("Downloads")));
    assert_eq!(path.file_name().unwrap().to_string_lossy(), "Turbine profile - Lethal Company - Default.json");
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("\"fullName\":\"Owner-Alpha-1.0.2\""), "{text}");
    assert!(text.contains("can't be fetched automatically"));
    // A second export doesn't overwrite the first.
    let path2 = profiles::export(&p, &w.store, DEFAULT_PROFILE, &home).unwrap();
    assert!(path2.to_string_lossy().ends_with("Default (2).json"));

    let share = profiles::read_share(&path).unwrap();
    assert_eq!((share.game_store.as_str(), share.game_id.as_str(), share.profile.as_str()), ("steam", "1966720", DEFAULT_PROFILE));
    assert_eq!(share.mods.len(), 3);
    assert_eq!((share.mods[0].kind.as_str(), share.mods[0].id.as_str(), share.mods[0].version.as_deref()), ("thunderstore", "Owner-Alpha", Some("1.0.2")));
    assert_eq!((share.mods[1].kind.as_str(), share.mods[1].id.as_str(), share.mods[1].enabled), ("zip", "", false));

    // Here everything is installed already.
    let plan = profiles::plan_import(&share, &w.store).unwrap();
    assert_eq!(plan.installed.len(), 3);
    assert_eq!(profiles::share_order(&share, &w.store), vec![("alpha".into(), true), ("b".into(), false), ("c".into(), true)]);

    // On another device with nothing installed: what can be fetched and what can't.
    let t2 = TempDir::new("profiles-other");
    let dir = t2.path("lib/Game");
    fs::create_dir_all(&dir).unwrap();
    let g = game(Launcher::Steam, "1966720", "Lethal Company", &dir, None);
    let other = Store::open_with(&g, &t2.path("data"), Box::new(Plain)).unwrap();
    let plan = profiles::plan_import(&share, &other).unwrap();
    assert_eq!(plan.automatic.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), vec!["Owner-Alpha"]);
    assert_eq!(plan.nexus.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), vec!["4242"]);
    assert_eq!(plan.by_hand.len(), 1);
    assert!(plan.by_hand[0].note.as_deref().unwrap().contains("by hand"));
    assert!(turbine_engine::json::parse(&profiles::plan_json(&plan)).is_ok());

    // Another game's file is refused.
    let g2 = game(Launcher::Steam, "632360", "Risk of Rain 2", &dir, None);
    let ror = Store::open_with(&g2, &t2.path("data2"), Box::new(Plain)).unwrap();
    assert!(profiles::plan_import(&share, &ror).unwrap_err().contains("is for Lethal Company"));
}

#[test]
fn bad_share_files() {
    assert!(profiles::parse_share("{}").is_err());
    assert!(profiles::parse_share("not json").is_err());
    assert!(profiles::parse_share("{\"turbineProfile\":99,\"game\":{}}").unwrap_err().contains("newer"));
    // Unknown source kinds are treated as by hand.
    let s = profiles::parse_share("{\"turbineProfile\":1,\"game\":{\"name\":\"X\"},\"mods\":[{\"name\":\"m\",\"source\":\"weird\",\"id\":\"1\"},{\"nope\":1}]}").unwrap();
    assert_eq!(s.mods.len(), 1);
    assert_eq!(s.mods[0].kind, "zip");
}

fn nexus_entry(name: &str, id: &str, url: Option<&str>) -> profiles::ShareMod {
    profiles::ShareMod {
        name: name.into(),
        enabled: true,
        kind: "nexus".into(),
        id: id.into(),
        version: Some("1.0".into()),
        url: url.map(String::from),
        note: None,
    }
}

#[test]
fn nexus_entries_become_downloads_or_pages() {
    use profiles::{NexusAction, NexusStep};
    let plan = profiles::ImportPlan {
        nexus: vec![
            nexus_entry("Full", "lethalcompany:1:20", None),
            nexus_entry("Mod only", "lethalcompany:2", None),
            nexus_entry("Old record", "4242", None),
            nexus_entry("Other site", "site:9:10", None),
            nexus_entry("Address only", "", Some("https://www.nexusmods.com/lethalcompany/mods/77")),
            nexus_entry("Nothing", "x:y:z", Some("https://evil.example.com/")),
        ],
        ..Default::default()
    };
    let premium = profiles::nexus_steps(&plan, Some("lethalcompany"), true);
    let acts: Vec<&NexusAction> = premium.iter().map(|s| &s.action).collect();
    assert_eq!(
        acts,
        vec![
            &NexusAction::Fetch { domain: "lethalcompany".into(), mod_id: 1, file_id: Some(20) },
            &NexusAction::Fetch { domain: "lethalcompany".into(), mod_id: 2, file_id: None },
            &NexusAction::Fetch { domain: "lethalcompany".into(), mod_id: 4242, file_id: None },
            &NexusAction::OpenPage { url: "https://www.nexusmods.com/site/mods/9?tab=files&file_id=10".into() },
            &NexusAction::OpenPage { url: "https://www.nexusmods.com/lethalcompany/mods/77?tab=files".into() },
            &NexusAction::ByHand,
        ]
    );
    let free = profiles::nexus_steps(&plan, Some("lethalcompany"), false);
    assert_eq!(
        free[0],
        NexusStep {
            entry: plan.nexus[0].clone(),
            action: NexusAction::OpenPage { url: "https://www.nexusmods.com/lethalcompany/mods/1?tab=files&file_id=20".into() }
        }
    );
    assert_eq!(free[2].action, NexusAction::OpenPage { url: "https://www.nexusmods.com/lethalcompany/mods/4242?tab=files".into() });
    // Without the game's domain, a bare mod ID can't be placed.
    assert_eq!(profiles::nexus_steps(&plan, None, true)[2].action, NexusAction::ByHand);
    let j = turbine_engine::json::parse(&profiles::nexus_steps_json(&premium)).unwrap();
    let first = &j.as_array()[0];
    assert_eq!(first.str_at("action"), Some("fetch"));
    assert_eq!(first.get("fileId"), Some(&turbine_engine::json::Json::Num(20.0)));
    assert_eq!(first.get("mod").unwrap().str_at("name"), Some("Full"));
    assert_eq!(j.as_array()[1].get("fileId"), Some(&turbine_engine::json::Json::Null));
}

#[test]
fn a_newer_nexus_file_of_the_same_mod_counts_as_installed() {
    let t = TempDir::new("profiles-nexus");
    let dir = t.path("lib/Game");
    fs::create_dir_all(&dir).unwrap();
    let g = game(Launcher::Steam, "1966720", "Lethal Company", &dir, None);
    let mut store = Store::open_with(&g, &t.path("data"), Box::new(Plain)).unwrap();
    let p = t.path("dl/N.zip");
    write_zip(&p, &[("n.txt", b"x")]);
    let src = SourceInfo { kind: "nexus".into(), id: "lethalcompany:5:60".into(), ..SourceInfo::default() };
    let Import::Added(_) = store.import_archive(&p, src).unwrap() else { panic!() };
    let share = profiles::Share {
        game_name: "Lethal Company".into(),
        game_store: "steam".into(),
        game_id: "1966720".into(),
        profile: "P".into(),
        mods: vec![nexus_entry("N", "lethalcompany:5:50", None), nexus_entry("M", "lethalcompany:6:61", None)],
    };
    let plan = profiles::plan_import(&share, &store).unwrap();
    assert_eq!(plan.installed.len(), 1);
    assert_eq!(plan.nexus.len(), 1);
    assert_eq!(plan.nexus[0].name, "M");
    assert_eq!(profiles::share_order(&share, &store).len(), 1);
}
