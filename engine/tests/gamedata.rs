//! Game rules as data files: every built-in file is valid, the nine launch games behave exactly as they did with
//! 0.55's tables (a snapshot of those tables is kept below), and players' own files can add games but never change
//! Turbine's.

mod support;

use std::fs;
use std::path::PathBuf;
use support::scratch as temp;
use turbine_engine::gamedata::{self, parse_game};
use turbine_engine::games::{Game, Runtime, Store};
use turbine_engine::rules::{self, Family, GameRules, Kind, Support};

/// 0.55's tables as they were (engine/src/rules.rs before the data files), kept only for this comparison.
#[allow(dead_code)]
mod old {
    use turbine_engine::games::Store;
    use turbine_engine::rules::{Family, Kind};

    /// One known game. Data only.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct OldGame {
        pub name: &'static str,
        /// A shorter name for the page header; None when `name` is already short.
        pub short_name: Option<&'static str>,
        /// Store IDs the game is known by (Steam app ID, GOG product ID, Heroic's Epic app name).
        pub ids: &'static [(Store, &'static str)],
        pub kind: Kind,
        pub mod_root: &'static str,
        pub plugins: bool,
        pub ordered: bool,
        /// Key into LOADERS.
        pub loader: Option<&'static str>,
        /// Where this game's build of the loader comes from, when it differs from the loader's own page.
        pub loader_source: Option<&'static str>,
        /// For the game's native Linux build: the Launch Option its loader needs, and why.
        pub native_launch: Option<(&'static str, &'static str)>,
        pub family: Family,
    }

    /// One script loader. Data only.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct OldLoader {
        pub key: &'static str,
        pub name: &'static str,
        pub explain: &'static str,
        pub detect: &'static [&'static str],
        pub source: &'static str,
        /// How it hooks into the game, in words.
        pub hook: &'static str,
        /// (loader file, launcher file): the loader is also installed under the launcher's name (Steam only).
        pub launcher_swap: Option<(&'static str, &'static str)>,
    }

    pub const GAMES: &[OldGame] = &[
        OldGame {
            name: "Skyrim Special Edition",
            short_name: Some("Skyrim Special Edition"),
            // GOG sells it as Skyrim Anniversary Edition: product 1711230643 is "The Elder Scrolls V: Skyrim Special
            // Edition", type Game, with Windows builds (gogdb.org/product/1711230643, checked Nov 6 2026).
            ids: &[(Store::Steam, "489830"), (Store::Gog, "1711230643")],
            kind: Kind::Bethesda,
            mod_root: "Data",
            plugins: true,
            ordered: true,
            loader: Some("skse"),
            loader_source: None,
            native_launch: None,
            family: Family::Standard,
        },
        OldGame {
            name: "Stardew Valley",
            short_name: None,
            ids: &[(Store::Steam, "413150"), (Store::Gog, "1453375253")],
            kind: Kind::Smapi,
            mod_root: "Mods",
            plugins: false,
            ordered: false,
            loader: Some("smapi"),
            loader_source: None,
            native_launch: None,
            family: Family::Standard,
        },
        OldGame {
            name: "Valheim",
            short_name: None,
            // Steam ships a native Linux build; its BepInEx pack starts through start_game_bepinex.sh there, and through
            // winhttp.dll under Proton. The pack holds both, so detection is the same.
            ids: &[(Store::Steam, "892970")],
            kind: Kind::BepInEx,
            mod_root: "BepInEx/plugins",
            plugins: false,
            ordered: false,
            loader: Some("bepinex"),
            loader_source: Some("https://thunderstore.io/c/valheim/p/denikson/BepInExPack_Valheim/"),
            // The pack's start_game_bepinex.sh starts the game with BepInEx (Doorstop) loaded; Steam only runs it when the
            // Launch Option points at it.
            native_launch: Some((
                "./start_game_bepinex.sh %command%",
                "Valheim runs its Linux version here, and BepInEx (the mod loader) starts through a script that comes with \
                 it. Steam only runs that script when you ask it to: in Steam, open Valheim's Properties, and under Launch \
                 Options type exactly the line below. You only do this once. Without it, the game starts without mods.",
            )),
            family: Family::Standard,
        },
        OldGame {
            name: "Lethal Company",
            short_name: None,
            ids: &[(Store::Steam, "1966720")],
            kind: Kind::BepInEx,
            mod_root: "BepInEx/plugins",
            plugins: false,
            ordered: false,
            loader: Some("bepinex"),
            loader_source: Some("https://thunderstore.io/c/lethal-company/p/BepInEx/BepInExPack/"),
            native_launch: None,
            family: Family::Standard,
        },
        OldGame {
            name: "Risk of Rain 2",
            short_name: None,
            ids: &[(Store::Steam, "632360")],
            kind: Kind::BepInEx,
            mod_root: "BepInEx/plugins",
            plugins: false,
            ordered: false,
            loader: Some("bepinex"),
            loader_source: Some("https://thunderstore.io/c/riskofrain2/p/bbepis/BepInExPack/"),
            native_launch: None,
            family: Family::Standard,
        },
        OldGame {
            name: "Cyberpunk 2077",
            short_name: None,
            // Steam 1091500 (store.steampowered.com/app/1091500, "Cyberpunk 2077"); GOG 1423049311 (gogdb.org, "Cyberpunk
            // 2077", type Game). Both checked Nov 6 2026.
            ids: &[(Store::Steam, "1091500"), (Store::Gog, "1423049311")],
            kind: Kind::Generic,
            mod_root: "",
            plugins: false,
            ordered: true,
            // Several frameworks instead of one loader: see cyberpunk::FRAMEWORKS and mod_checks.
            loader: None,
            loader_source: None,
            native_launch: None,
            family: Family::Cyberpunk,
        },
        OldGame {
            name: "The Witcher 3",
            short_name: Some("The Witcher 3"),
            // Steam 292030 (store.steampowered.com/app/292030, "The Witcher 3: Wild Hunt"). On GOG, 1207664663 is the
            // Complete Edition package (gogdb.org, type Package, no builds); the installable games are 1207664643 ("The
            // Witcher 3: Wild Hunt", type Game, the one that package holds) and 1495134320 (the Game of the Year
            // Edition, type Game). Checked Nov 6 2026.
            ids: &[(Store::Steam, "292030"), (Store::Gog, "1207664643"), (Store::Gog, "1495134320")],
            kind: Kind::Generic,
            mod_root: "",
            plugins: false,
            ordered: true,
            loader: None,
            loader_source: None,
            native_launch: None,
            family: Family::Witcher3,
        },
        OldGame {
            name: "Kingdom Come: Deliverance",
            short_name: None,
            // Steam 379430 (store.steampowered.com/app/379430, "Kingdom Come: Deliverance"); GOG 1719198803 (gogdb.org,
            // "Kingdom Come: Deliverance", type Game, Windows builds; 1182943037 is the Royal Edition package). Checked
            // Nov 7 2026. Mods: Mods/<mod>/mod.manifest and Data/*.pak, order in Mods/mod_order.txt (kcd.rs has sources).
            ids: &[(Store::Steam, "379430"), (Store::Gog, "1719198803")],
            kind: Kind::Generic,
            mod_root: "Mods",
            plugins: false,
            ordered: true,
            loader: None,
            loader_source: None,
            native_launch: None,
            family: Family::Kcd,
        },
        OldGame {
            name: "Kingdom Come: Deliverance II",
            short_name: None,
            // Steam 1771300 (store.steampowered.com/app/1771300, "Kingdom Come: Deliverance II"); GOG 1248083010
            // (gogdb.org, "Kingdom Come: Deliverance II", type Game, Windows builds; 1076704248 is its package and
            // 1141344806 the Expansion Pass package). Checked Nov 7 2026. Mods: Warhorse's own guide (see kcd.rs).
            ids: &[(Store::Steam, "1771300"), (Store::Gog, "1248083010")],
            kind: Kind::Generic,
            mod_root: "Mods",
            plugins: false,
            ordered: true,
            loader: None,
            loader_source: None,
            native_launch: None,
            family: Family::Kcd2,
        },
    ];

    pub const LOADERS: &[OldLoader] = &[
        OldLoader {
            key: "skse",
            name: "SKSE (Skyrim Script Extender)",
            explain: "SKSE lets Skyrim load mods that add new code, like SkyUI and many popular fixes.",
            detect: &["skse64_loader.exe"],
            source: "https://skse.silverlock.org/",
            // Steam starts SkyrimSELauncher.exe, not the loader, so a later build swaps the launcher for the loader
            // (keeping the original in the backup folder).
            hook: "launcher swap: SkyrimSELauncher.exe is replaced by skse64_loader.exe",
            launcher_swap: Some(("skse64_loader.exe", "SkyrimSELauncher.exe")),
        },
        OldLoader {
            key: "bepinex",
            name: "BepInEx",
            explain: "BepInEx lets Unity games load mods that add new code; most mods for this game need it.",
            detect: &["winhttp.dll", "BepInEx/core"],
            source: "https://github.com/BepInEx/BepInEx/releases",
            hook: "Windows builds: winhttp.dll override in the prefix; native builds: the pack's start script",
            launcher_swap: None,
        },
        OldLoader {
            key: "smapi",
            name: "SMAPI (Stardew Modding API)",
            explain: "SMAPI is the mod loader for Stardew Valley; mods in the Mods folder only work through it.",
            detect: &["StardewModdingAPI|StardewModdingAPI.exe"],
            source: "https://smapi.io/",
            hook: "installed from the installer's own bundle (never run); its launcher takes the game's launcher's place",
            launcher_swap: None,
        },
    ];

    /// The known game with this store ID, if any.
    pub fn find_def(store: Store, store_id: &str) -> Option<&'static OldGame> {
        GAMES.iter().find(|d| d.ids.iter().any(|(s, id)| *s == store && *id == store_id))
    }

    pub fn find_loader(key: &str) -> Option<&'static OldLoader> {
        LOADERS.iter().find(|l| l.key == key)
    }

}

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

/// 0.55's GameRules::for_game, field by field, from the snapshot.
#[allow(clippy::type_complexity)]
fn old_rules(store: Store, id: &str, runtime: Runtime) -> (Kind, String, bool, bool, Option<(String, String, Vec<String>, String)>, Option<String>, Option<(String, String)>, Option<String>, Option<String>, Family) {
    let def = old::find_def(store, id).unwrap();
    let l = def.loader.and_then(old::find_loader);
    let loader = l.map(|l| (l.name.to_string(), l.explain.to_string(), l.detect.iter().map(|s| s.to_string()).collect(), def.loader_source.unwrap_or(l.source).to_string()));
    let mut swap = l.and_then(|l| l.launcher_swap).map(|(a, b)| (a.to_string(), b.to_string()));
    let mut option = def.native_launch.map(|(o, _)| o.to_string());
    let mut help = def.native_launch.map(|(_, h)| h.to_string());
    if store != Store::Steam {
        swap = None;
    }
    if runtime == Runtime::Windows || store != Store::Steam {
        option = None;
        help = None;
    }
    // 0.6: Heroic starts SMAPI through its own link (rules::launch_url_with), so the Steam wording is for Steam only.
    if def.kind == Kind::Smapi && runtime == Runtime::Windows && store == Store::Steam {
        help = Some("This copy of Stardew Valley is the Windows version. Turbine sets SMAPI up from its Windows files, but Steam still starts the game itself: follow SMAPI's own guide at smapi.io/install for the Launch Option that starts StardewModdingAPI.exe instead.".into());
    }
    (def.kind, def.mod_root.into(), def.plugins, def.ordered, loader, def.short_name.map(Into::into), swap, option, help, def.family)
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
fn golden_rules_unchanged() {
    let mut n = 0;
    for def in old::GAMES {
        for (store, id) in def.ids {
            for runtime in [Runtime::Windows, Runtime::Native, Runtime::Unknown] {
                let new = GameRules::for_game(&game(*store, id, runtime));
                let got = (
                    new.kind,
                    new.mod_root.clone(),
                    new.plugins,
                    new.ordered,
                    new.loader.as_ref().map(|l| (l.name.clone(), l.explain.clone(), l.detect.clone(), l.source.clone())),
                    new.short_name.clone(),
                    new.launcher_swap.clone(),
                    new.launch_option.clone(),
                    new.launch_help.clone(),
                    new.family,
                );
                assert_eq!(got, old_rules(*store, id, runtime), "{} {id} {runtime:?}", def.name);
                assert_eq!(new.runtime, runtime);
                // The one intended change: the support words.
                assert_eq!((new.support.as_str(), new.support_level), ("Full support", Support::Full));
                n += 1;
            }
        }
    }
    assert_eq!(n, 16 * 3, "every store ID of the nine games");
    // The loader table too (hook words are shown in Labs).
    for l in old::LOADERS {
        let new = rules::find_loader(l.key).unwrap();
        assert_eq!((new.name, new.explain, new.detect, new.source, new.hook, new.launcher_swap), (l.name, l.explain, l.detect, l.source, l.hook, l.launcher_swap));
    }
    assert_eq!(rules::loaders().len(), old::LOADERS.len());
    // Same Nexus and Thunderstore answers as the old tables.
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
