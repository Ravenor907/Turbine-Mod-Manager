//! Cyberpunk 2077: where each kind of mod goes, the frameworks and what needs them, the Proton DLL settings, REDmod
//! and the update warning. The framework file lists are the ones each project's own packaging makes (see the notes at
//! the top of engine/src/cyberpunk.rs for the files and commits they come from).

mod support;

use std::collections::BTreeMap;
use std::fs;
use support::{game, write_zip, TempDir};
use turbine_engine::cyberpunk::{self, REDMOD_TITLE};
use turbine_engine::games::Store as Launcher;
use turbine_engine::proton;
use turbine_engine::rules::{self, mod_checks_with, pre_play_with, Family, GameRules, Kind, ModFiles};
use turbine_engine::store::{Import, SourceInfo, Store};
use turbine_engine::{Game, Runtime};

fn cp_game(dir: &std::path::Path) -> Game {
    let mut g = game(Launcher::Steam, "1091500", "Cyberpunk 2077", dir, None);
    g.runtime = Runtime::Windows;
    g
}

fn cp() -> GameRules {
    GameRules::for_game(&cp_game(std::path::Path::new("/games/Cyberpunk 2077")))
}

fn list(files: &[&str]) -> Vec<String> {
    files.iter().map(|s| s.to_string()).collect()
}

fn map(files: &[&str], name: &str) -> BTreeMap<String, String> {
    cp().map_archive(&list(files), name).unwrap().into_iter().collect()
}

fn dests(files: &[&str], name: &str) -> Vec<String> {
    let mut d: Vec<String> = map(files, name).into_values().collect();
    d.sort();
    d
}

// Release archives of the frameworks, as their packaging scripts lay them out.
const CET: &[&str] = &[
    "bin/x64/global.ini",
    "bin/x64/version.dll",
    "bin/x64/LICENSE",
    "bin/x64/plugins/cyber_engine_tweaks.asi",
    "bin/x64/plugins/cyber_engine_tweaks/ThirdParty_LICENSES",
    "bin/x64/plugins/cyber_engine_tweaks/fonts/NotoSans-Regular.ttf",
    "bin/x64/plugins/cyber_engine_tweaks/scripts/json/json.lua",
    "bin/x64/plugins/cyber_engine_tweaks/tweakdb/usedhashes.kark",
];
const RED4EXT: &[&str] = &["bin/x64/winmm.dll", "red4ext/RED4ext.dll", "red4ext/LICENSE.txt", "red4ext/THIRD_PARTY_LICENSES.txt"];
const ARCHIVEXL: &[&str] = &[
    "red4ext/plugins/ArchiveXL/ArchiveXL.dll",
    "red4ext/plugins/ArchiveXL/Bundle/PlayerCustomizationHairFix.xl",
    "red4ext/plugins/ArchiveXL/Scripts/ArchiveXL.reds",
    "r6/config/redsUserHints/ArchiveXL.toml",
    "red4ext/plugins/ArchiveXL/LICENSE",
];
const TWEAKXL: &[&str] = &["red4ext/plugins/TweakXL/TweakXL.dll", "red4ext/plugins/TweakXL/Data/ExtraFlats.dat", "red4ext/plugins/TweakXL/Scripts/TweakXL.reds"];
const REDSCRIPT: &[&str] = &["engine/config/base/scripts.ini", "engine/tools/scc.exe", "engine/tools/scc_lib.dll", "r6/config/cybercmd/scc.toml"];

#[test]
fn ids_and_rules() {
    for (store, id) in [(Launcher::Steam, "1091500"), (Launcher::Gog, "1423049311")] {
        let d = rules::find_def(store, id).unwrap();
        assert_eq!((d.name, d.family), ("Cyberpunk 2077", Family::Cyberpunk));
    }
    let r = cp();
    assert_eq!((r.kind, r.family, r.ordered, r.plugins, r.mod_root.as_str()), (Kind::Generic, Family::Cyberpunk, true, false, ""));
    assert!(r.loader.is_none());
    assert_eq!(r.support, "Full support");
}

#[test]
fn framework_archives_go_in_as_they_are() {
    for files in [CET, RED4EXT, ARCHIVEXL, TWEAKXL, REDSCRIPT] {
        let m = map(files, "x");
        for f in files.iter() {
            assert_eq!(m.get(*f).map(String::as_str), Some(*f), "{f}");
        }
    }
    // Wrapped in a folder with the version in its name.
    let d = dests(&["cet_1.36.0/bin/x64/version.dll", "cet_1.36.0/bin/x64/plugins/cyber_engine_tweaks.asi", "cet_1.36.0/readme.md"], "x");
    assert_eq!(d, vec!["bin/x64/plugins/cyber_engine_tweaks.asi", "bin/x64/version.dll"]);
}

#[test]
fn archive_mods_go_to_archive_pc_mod() {
    assert_eq!(dests(&["SomeOutfit.archive", "SomeOutfit.archive.xl", "readme.txt", "preview.png"], "x"), vec![
        "archive/pc/mod/SomeOutfit.archive",
        "archive/pc/mod/SomeOutfit.archive.xl"
    ]);
    assert_eq!(dests(&["Outfit v2/archive/pc/mod/Outfit.archive"], "x"), vec!["archive/pc/mod/Outfit.archive"]);
    assert_eq!(dests(&["Wrapper/Inner/archive/pc/mod/a.archive"], "x"), vec!["archive/pc/mod/a.archive"]);
    // A plain folder of .archive files.
    assert_eq!(dests(&["Files/a.archive", "Files/b.archive"], "x"), vec!["archive/pc/mod/a.archive", "archive/pc/mod/b.archive"]);
    // One folder per option: Turbine can't choose, and says so.
    let err = cp().map_archive(&list(&["Option Red/car.archive", "Option Blue/car.archive"]), "x").unwrap_err();
    assert!(err.contains("more than one version of car.archive") && err.contains("Option Red") && err.contains("Option Blue"), "{err}");
}

#[test]
fn cet_mods_redscript_tweaks_and_red4ext_plugins() {
    // A Lua mod as a bare folder, and the same wrapped in the game's path.
    assert_eq!(dests(&["MyLuaMod/init.lua", "MyLuaMod/data/settings.json"], "x"), vec![
        "bin/x64/plugins/cyber_engine_tweaks/mods/MyLuaMod/data/settings.json",
        "bin/x64/plugins/cyber_engine_tweaks/mods/MyLuaMod/init.lua"
    ]);
    assert_eq!(dests(&["Pack 1.0/bin/x64/plugins/cyber_engine_tweaks/mods/MyLuaMod/init.lua"], "x"), vec![
        "bin/x64/plugins/cyber_engine_tweaks/mods/MyLuaMod/init.lua"
    ]);
    // Flat: init.lua at the top takes the mod's name.
    assert_eq!(dests(&["init.lua", "modules/a.lua"], "Better Lua"), vec![
        "bin/x64/plugins/cyber_engine_tweaks/mods/Better Lua/init.lua",
        "bin/x64/plugins/cyber_engine_tweaks/mods/Better Lua/modules/a.lua"
    ]);
    // redscript: loose .reds, a folder of them, or r6/scripts as given.
    assert_eq!(dests(&["fix.reds"], "Fix"), vec!["r6/scripts/Fix/fix.reds"]);
    assert_eq!(dests(&["MyScripts/a.reds", "MyScripts/sub/b.reds"], "x"), vec!["r6/scripts/MyScripts/a.reds", "r6/scripts/MyScripts/sub/b.reds"]);
    assert_eq!(dests(&["r6/scripts/Thing/thing.reds"], "x"), vec!["r6/scripts/Thing/thing.reds"]);
    // TweakXL tweaks.
    assert_eq!(dests(&["prices.yaml"], "Prices"), vec!["r6/tweaks/Prices/prices.yaml"]);
    assert_eq!(dests(&["r6/tweaks/Prices/prices.yaml"], "x"), vec!["r6/tweaks/Prices/prices.yaml"]);
    // A RED4ext plugin.
    assert_eq!(dests(&["red4ext/plugins/Thing/Thing.dll"], "x"), vec!["red4ext/plugins/Thing/Thing.dll"]);
    // Nothing usable.
    assert!(cp().map_archive(&list(&["readme.txt", "screenshot.png"]), "x").is_err());
}

#[test]
fn redmod_mods_go_to_mods() {
    let want = vec!["mods/CoolRedmod/archives/cool.archive", "mods/CoolRedmod/info.json", "mods/CoolRedmod/scripts/a.script"];
    assert_eq!(dests(&["mods/CoolRedmod/info.json", "mods/CoolRedmod/archives/cool.archive", "mods/CoolRedmod/scripts/a.script"], "x"), want);
    assert_eq!(dests(&["CoolRedmod/info.json", "CoolRedmod/archives/cool.archive", "CoolRedmod/scripts/a.script"], "x"), want);
    assert_eq!(dests(&["Download v3/CoolRedmod/info.json", "Download v3/CoolRedmod/archives/cool.archive", "Download v3/CoolRedmod/scripts/a.script"], "x"), want);
    // Flat: info.json at the top, under the mod's name.
    assert_eq!(dests(&["info.json", "archives/cool.archive"], "CoolRedmod"), vec!["mods/CoolRedmod/archives/cool.archive", "mods/CoolRedmod/info.json"]);
    assert!(cyberpunk::has_redmod(&list(&["mods/CoolRedmod/info.json"])));
    assert!(!cyberpunk::has_redmod(&list(&["mods/CoolRedmod/sub/info.json"])));
}

#[test]
fn what_needs_which_framework() {
    let needs = |files: &[&str]| {
        let mut v = cyberpunk::needs(&list(files));
        v.sort();
        v
    };
    assert_eq!(needs(&["bin/x64/plugins/cyber_engine_tweaks/mods/A/init.lua"]), vec!["cet", "red4ext"], "CET needs RED4ext");
    assert_eq!(needs(&["red4ext/plugins/A/A.dll"]), vec!["red4ext"]);
    assert_eq!(needs(&["r6/scripts/A/a.reds"]), vec!["redscript"]);
    assert_eq!(needs(&["r6/tweaks/A/a.yaml"]), vec!["red4ext", "tweakxl"]);
    assert_eq!(needs(&["archive/pc/mod/a.archive", "archive/pc/mod/a.archive.xl"]), vec!["archivexl", "red4ext"]);
    assert_eq!(needs(&["mods/A/info.json", "mods/A/archives/a.xl"]), vec!["archivexl", "red4ext"]);
    assert!(needs(&["archive/pc/mod/a.archive"]).is_empty());
    // The frameworks themselves: what they are isn't needed, what they build on is.
    assert!(needs(RED4EXT).is_empty());
    assert!(needs(REDSCRIPT).is_empty());
    assert_eq!(needs(CET), vec!["red4ext"]);
    assert_eq!(needs(ARCHIVEXL), vec!["red4ext"]);
    assert_eq!(needs(TWEAKXL), vec!["red4ext"]);
    assert_eq!(cyberpunk::provides(&list(CET)), vec!["cet"]);
    assert_eq!(cyberpunk::provides(&list(&["RED4EXT/RED4ext.dll"])), vec!["red4ext"]);
    // needs_loader follows the same rules.
    let r = cp();
    assert!(r.needs_loader(&list(&["r6/scripts/A/a.reds"])));
    assert!(!r.needs_loader(&list(&["archive/pc/mod/a.archive"])));
    assert!(!r.needs_loader(&list(RED4EXT)));
}

fn m(name: &str, files: &[&str]) -> ModFiles {
    ModFiles { id: name.to_lowercase(), name: name.into(), enabled: true, files: list(files) }
}

#[test]
fn checks_before_play_name_what_is_missing() {
    let r = cp();
    let lua = m("Better Lua", &["bin/x64/plugins/cyber_engine_tweaks/mods/BL/init.lua"]);
    let outfit = m("Outfit", &["archive/pc/mod/o.archive", "archive/pc/mod/o.archive.xl"]);
    let checks = mod_checks_with(&r, &[lua.clone(), outfit.clone()], &[]);
    let titles: Vec<&str> = checks.iter().map(|c| c.title.as_str()).collect();
    assert_eq!(titles, vec!["RED4ext is missing", "Cyber Engine Tweaks is missing", "ArchiveXL is missing"]);
    assert!(checks.iter().all(|c| c.level == "warn"));
    assert!(checks[0].detail.starts_with("Better Lua and Outfit need it."), "{}", checks[0].detail);
    assert!(checks[1].detail.contains("https://www.nexusmods.com/cyberpunk2077/mods/107"));
    // With the frameworks turned on (or already in the game folder) there is nothing to say.
    let all = [lua.clone(), outfit, m("CET", CET), m("ArchiveXL", ARCHIVEXL)];
    assert_eq!(mod_checks_with(&r, &all, &list(&["red4ext/RED4ext.dll"])), vec![]);
    // A turned-off framework doesn't count.
    let mut off = m("CET", CET);
    off.enabled = false;
    let c = mod_checks_with(&r, &[lua, off], &list(&["red4ext/red4ext.dll"]));
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].title, "Cyber Engine Tweaks is missing");
}

#[test]
fn redmod_gets_the_launcher_explanation_once_per_check() {
    let r = cp();
    let redmod = m("Cool REDmod", &["mods/CoolRedmod/info.json", "mods/CoolRedmod/archives/a.archive"]);
    let c = mod_checks_with(&r, std::slice::from_ref(&redmod), &list(&["tools/redmod/bin/redMod.exe"]));
    assert_eq!(c.len(), 1);
    assert_eq!((c[0].level.as_str(), c[0].title.as_str()), ("warn", REDMOD_TITLE));
    assert!(c[0].detail.contains("Enable mods") && !c[0].detail.contains("isn't installed"));
    // Without the REDmod DLC, the detail says to install it first.
    let c = mod_checks_with(&r, &[redmod], &[]);
    assert!(c[0].detail.starts_with("REDmod isn't installed."), "{}", c[0].detail);
    // The note in the activity log when such a mod is added.
    assert!(r.install_notes(&list(&["CoolRedmod/info.json", "CoolRedmod/archives/a.archive"]))[0].contains("REDmod"));
    assert!(r.install_notes(&list(&["a.archive"])).is_empty());
}

#[test]
fn game_update_warns_about_cet_and_red4ext() {
    let mut g = cp_game(std::path::Path::new("/games/cp"));
    g.build = Some("200".into());
    let r = cp();
    let c = pre_play_with(&g, &r, Some("100"), true, None, Some("ext4"), true);
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].level, "warn", "a warning: Play anyway stays possible");
    assert!(c[0].detail.contains("Cyber Engine Tweaks and RED4ext"), "{}", c[0].detail);
    // Without such mods, the plain wording.
    let c = pre_play_with(&g, &r, Some("100"), false, None, Some("ext4"), true);
    assert!(!c[0].detail.contains("RED4ext"));
    assert_eq!(pre_play_with(&g, &r, Some("200"), true, None, Some("ext4"), true)[0].level, "ok");
}

#[test]
fn proton_dll_overrides_follow_what_is_on() {
    let dir = TempDir::new("cp-proton");
    let g = cp_game(&dir.path("game"));
    let r = cp();
    let mut files = list(CET);
    assert_eq!(proton::wanted_overrides(&g, &r, &files), vec![("version", "native,builtin")]);
    files.extend(list(RED4EXT));
    let mut want = proton::wanted_overrides(&g, &r, &files);
    want.sort();
    assert_eq!(want, vec![("version", "native,builtin"), ("winmm", "native,builtin")]);
    assert!(proton::wanted_overrides(&g, &r, &list(&["archive/pc/mod/a.archive"])).is_empty());
    let mut native = g.clone();
    native.runtime = Runtime::Native;
    assert!(proton::wanted_overrides(&native, &r, &files).is_empty());

    // Written into the prefix, and only once.
    let pfx = dir.path("pfx");
    fs::create_dir_all(&pfx).unwrap();
    fs::write(pfx.join("user.reg"), "WINE REGISTRY Version 2\n;; All keys relative to \\\\User\\\\S-1-5-21-0-0-0-1000\n\n#arch=win64\n").unwrap();
    let mut changed = proton::ensure_overrides(&pfx, &want).unwrap();
    changed.sort();
    assert_eq!(changed, vec!["version", "winmm"]);
    assert_eq!(proton::read_override(&pfx, "winmm").unwrap().as_deref(), Some("native,builtin"));
    assert_eq!(proton::read_override(&pfx, "version").unwrap().as_deref(), Some("native,builtin"));
    assert!(proton::ensure_overrides(&pfx, &want).unwrap().is_empty());
    assert!(pfx.join(proton::BACKUP_NAME).exists());
}

#[test]
fn installing_and_checking_through_the_store() {
    let t = TempDir::new("cp-store");
    let dir = t.path("lib/Cyberpunk 2077");
    fs::create_dir_all(&dir).unwrap();
    let g = cp_game(&dir);
    let mut s = Store::open(&g, &t.path("data")).unwrap();
    let a = t.path("dl/Better Lua.zip");
    fs::create_dir_all(a.parent().unwrap()).unwrap();
    write_zip(&a, &[("BetterLua/init.lua", b"-- lua"), ("BetterLua/readme.txt", b"hi")]);
    let Import::Added(done) = s.import_archive(&a, SourceInfo { kind: "zip".into(), ..SourceInfo::default() }).unwrap() else { panic!() };
    assert!(done.item.needs_loader);
    let b = t.path("dl/red4ext-1.29.0.zip");
    write_zip(&b, &[("bin/x64/winmm.dll", b"winmm"), ("red4ext/RED4ext.dll", b"r4e"), ("red4ext/LICENSE.txt", b"l")]);
    let Import::Added(r4e) = s.import_archive(&b, SourceInfo { kind: "zip".into(), ..SourceInfo::default() }).unwrap() else { panic!() };
    assert!(!r4e.item.needs_loader);
    s.apply().unwrap();
    assert_eq!(fs::read(dir.join("bin/x64/plugins/cyber_engine_tweaks/mods/BetterLua/init.lua")).unwrap(), b"-- lua");
    assert_eq!(fs::read(dir.join("bin/x64/winmm.dll")).unwrap(), b"winmm");

    let mods = s.mod_files();
    assert_eq!(mods.len(), 2);
    let checks = rules::mod_checks(&g, &GameRules::for_game(&g), &mods);
    assert_eq!(checks.iter().map(|c| c.title.as_str()).collect::<Vec<_>>(), vec!["Cyber Engine Tweaks is missing"]);
    // What the game folder has counts too: CET put there by hand.
    fs::create_dir_all(dir.join("bin/x64/plugins")).unwrap();
    fs::write(dir.join("bin/x64/plugins/Cyber_Engine_Tweaks.asi"), b"asi").unwrap();
    assert!(rules::present_files(&g, &GameRules::for_game(&g)).contains(&"bin/x64/plugins/cyber_engine_tweaks.asi".to_string()));
    assert!(rules::mod_checks(&g, &GameRules::for_game(&g), &mods).is_empty());
}
