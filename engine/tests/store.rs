//! The mod store: importing zips, on/off and order, Recently removed, setups, and applying to a game folder.
//! Rules here are made by hand so these tests don't depend on any game's real rules.

mod support;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use support::write_zip_text as write_zip;
use turbine_engine::games::{Game, Runtime, Store as Launcher};
use turbine_engine::json;
use turbine_engine::store::{choose_root, clean_name, downloads, ModRules, Store, DRIVE_FOLDER, KEEP_REMOVED_SECS, KEEP_SETUPS};

// ------------------------------------------------------------------ helpers

/// Bethesda-like rules: everything goes under Data, a top-level readme is left out, SKSE plugins need the loader.
struct TestRules;

impl ModRules for TestRules {
    fn map_archive(&self, files: &[String], _mod_name: &str) -> Result<Vec<(String, String)>, String> {
        if files.iter().any(|f| f == "wrong-game.txt") {
            return Err("This mod is for a different game.".into());
        }
        Ok(files
            .iter()
            .filter(|f| !f.eq_ignore_ascii_case("readme.txt"))
            .map(|f| {
                let to = if f.to_lowercase().starts_with("data/") { f.clone() } else { format!("Data/{f}") };
                (f.clone(), to)
            })
            .collect())
    }

    fn needs_loader(&self, game_files: &[String]) -> bool {
        game_files.iter().any(|f| f.to_lowercase().contains("/skse/"))
    }
}

struct World {
    root: PathBuf,
    game: Game,
    data_home: PathBuf,
}

impl Drop for World {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl World {
    fn open(&self) -> Store {
        Store::open_with(&self.game, &self.data_home, Box::new(TestRules)).unwrap()
    }

    fn zip(&self, name: &str, files: &[(&str, &str)]) -> PathBuf {
        let p = self.root.join("Downloads").join(name);
        write_zip(&p, files);
        p
    }

    fn read(&self, rel: &str) -> Option<String> {
        fs::read_to_string(self.game.install_dir.join(rel)).ok()
    }
}

fn write(p: &Path, text: &str) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

fn world(tag: &str) -> World {
    let root = std::env::temp_dir().join(format!("turbine-store-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let library = root.join("library");
    let install = library.join("steamapps/common/Skyrim");
    write(&install.join("Data/Skyrim.esm"), "vanilla esm");
    write(&install.join("Data/textures/sky.dds"), "vanilla sky");
    write(&install.join("SkyrimSE.exe"), "vanilla exe");
    let game = Game {
        store: Launcher::Steam,
        store_id: "489830".into(),
        name: "Skyrim Special Edition".into(),
        install_dir: install,
        library,
        build: Some("100".into()),
        runtime: Runtime::Windows,
        cover: None,
        header: None,
        prefix: None,
    };
    let data_home = root.join("data");
    World { root, game, data_home }
}

/// Every file and folder under `root` (relative path -> contents, folders as "<dir>").
fn snapshot(root: &Path) -> BTreeMap<String, String> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
        for e in fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            let rel = p.strip_prefix(root).unwrap().to_string_lossy().into_owned();
            if p.is_dir() {
                out.insert(rel, "<dir>".into());
                walk(root, &p, out);
            } else {
                out.insert(rel, fs::read_to_string(&p).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn ids(s: &Store) -> Vec<String> {
    s.mods().iter().map(|m| m.id.clone()).collect()
}

// ------------------------------------------------------------------ tests

#[test]
fn names_come_from_the_file_name() {
    assert_eq!(clean_name("SkyUI_5_2_SE-12604-5-2SE-1573517013.zip"), ("SkyUI 5 2 SE".into(), Some("5.2SE".into())));
    assert_eq!(clean_name("Cool_Mod-1234-5-6-1700000000.zip"), ("Cool Mod".into(), Some("5.6".into())));
    assert_eq!(clean_name("Cool_Mod-1234-1700000000.zip"), ("Cool Mod".into(), None));
    assert_eq!(clean_name("Author-Cool_Mod-1.2.3.zip"), ("Author-Cool Mod".into(), Some("1.2.3".into())));
    assert_eq!(clean_name("Better Lights v2.zip"), ("Better Lights".into(), Some("2".into())));
    assert_eq!(clean_name("Better_Lights-v2.ZIP"), ("Better Lights".into(), Some("2".into())));
    assert_eq!(clean_name("plain.zip"), ("Plain".into(), None));
    assert_eq!(clean_name("__.zip").0, "Mod");
}

#[test]
fn imports_and_lays_out_relative_to_the_game_folder() {
    let w = world("import");
    let mut s = w.open();
    assert_eq!(s.root(), w.game.library.join(".turbine-mod-manager/steam-489830"), "on the game's own library");
    let z = w.zip("Cool_Mod-1234-1-2-1700000000.zip", &[("readme.txt", "read me"), ("textures/a.dds", "A tex"), ("Data/b.esp", "B esp")]);
    let m = s.import_zip(&z).unwrap();
    assert_eq!(m.name, "Cool Mod");
    assert_eq!(m.version.as_deref(), Some("1.2"));
    assert_eq!(m.id, "cool-mod");
    assert_eq!((m.files, m.enabled, m.needs_loader), (2, true, false));
    assert_eq!(m.source, "Zip file");
    assert!(m.added > 1_600_000_000);
    let staged = s.root().join("mods/cool-mod");
    assert_eq!(fs::read_to_string(staged.join("Data/textures/a.dds")).unwrap(), "A tex");
    assert_eq!(fs::read_to_string(staged.join("Data/b.esp")).unwrap(), "B esp");
    assert!(!staged.join("readme.txt").exists() && !staged.join("Data/readme.txt").exists());
    assert!(fs::read_dir(s.root().join("tmp")).map(|r| r.count()).unwrap_or(0) == 0, "nothing left in tmp");
    assert!(w.read("Data/b.esp").is_none(), "importing doesn't touch the game");

    let z2 = w.zip("Script_Thing.zip", &[("SKSE/Plugins/x.dll", "dll")]);
    let m2 = s.import_zip(&z2).unwrap();
    assert!(m2.needs_loader);
    assert_eq!(ids(&s), ["cool-mod", "script-thing"], "new mods go last");

    // A different download with the same name gets its own ID.
    let z3 = w.zip("Cool Mod.zip", &[("c.esp", "c")]);
    assert_eq!(s.import_zip(&z3).unwrap().id, "cool-mod-2");
}

#[test]
fn import_errors_are_sentences_and_leave_nothing_behind() {
    let w = world("errors");
    let mut s = w.open();
    let junk = w.root.join("Downloads/junk.zip");
    write(&junk, "this is not a zip file at all, just text");
    let e = s.import_zip(&junk).unwrap_err();
    assert!(e.starts_with("junk.zip couldn't be added: ") && e.ends_with('.'), "{e}");

    let wrong = w.zip("wrong.zip", &[("wrong-game.txt", "x")]);
    let e = s.import_zip(&wrong).unwrap_err();
    assert_eq!(e, "wrong.zip couldn't be added: This mod is for a different game.");

    let only_readme = w.zip("readme-only.zip", &[("readme.txt", "x")]);
    assert!(s.import_zip(&only_readme).unwrap_err().contains("knows where to put"));

    let empty = w.zip("empty.zip", &[]);
    assert!(s.import_zip(&empty).unwrap_err().contains("empty"));

    let missing = s.import_zip(&w.root.join("Downloads/nope.zip")).unwrap_err();
    assert!(missing.starts_with("nope.zip couldn't be added"), "{missing}");

    assert!(s.mods().is_empty());
    assert_eq!(fs::read_dir(s.root().join("mods")).unwrap().count(), 0);
    assert_eq!(fs::read_dir(s.root().join("tmp")).map(|r| r.count()).unwrap_or(0), 0);
}

#[test]
fn order_decides_who_wins_and_off_mods_stay_out() {
    let w = world("order");
    let mut s = w.open();
    let original = snapshot(&w.game.install_dir);
    s.import_zip(&w.zip("Alpha.zip", &[("shared.esp", "alpha"), ("textures/sky.dds", "alpha sky")])).unwrap();
    s.import_zip(&w.zip("Beta.zip", &[("shared.esp", "beta"), ("beta.esp", "beta only")])).unwrap();
    let r = s.apply().unwrap();
    assert_eq!(r.added, 3);
    assert_eq!(w.read("Data/shared.esp").as_deref(), Some("beta"), "later in the order wins");
    assert_eq!(w.read("Data/textures/sky.dds").as_deref(), Some("alpha sky"));

    s.move_to("beta", 0).unwrap();
    assert_eq!(ids(&s), ["beta", "alpha"]);
    s.apply().unwrap();
    assert_eq!(w.read("Data/shared.esp").as_deref(), Some("alpha"));

    s.move_to("beta", 99).unwrap();
    assert_eq!(ids(&s), ["alpha", "beta"], "past the end means last");
    s.set_enabled("beta", false).unwrap();
    assert!(!s.mods()[1].enabled);
    s.apply().unwrap();
    assert_eq!(w.read("Data/shared.esp").as_deref(), Some("alpha"));
    assert!(w.read("Data/beta.esp").is_none());

    s.set_enabled("alpha", false).unwrap();
    s.apply().unwrap();
    assert_eq!(snapshot(&w.game.install_dir), original, "all off gives back the original game folder");

    assert_eq!(s.set_enabled("nope", true).unwrap_err().kind(), std::io::ErrorKind::NotFound);
    assert!(s.move_to("nope", 0).is_err());
    assert!(s.remove("nope").is_err());
    assert!(s.undo_remove("nope").is_err());
}

#[test]
fn same_download_again_replaces_files_and_keeps_place_and_state() {
    let w = world("reimport");
    let mut s = w.open();
    s.import_zip(&w.zip("First.zip", &[("first.esp", "first")])).unwrap();
    let z = w.zip("Tweak-1.0.zip", &[("tweak.esp", "old"), ("old-only.txt", "old")]);
    s.import_zip(&z).unwrap();
    s.import_zip(&w.zip("Last.zip", &[("last.esp", "last")])).unwrap();
    s.move_to("tweak", 0).unwrap();
    s.set_enabled("tweak", false).unwrap();
    s.set_enabled("tweak", true).unwrap();
    s.apply().unwrap();
    assert_eq!(w.read("Data/tweak.esp").as_deref(), Some("old"));

    write_zip(&z, &[("tweak.esp", "new"), ("new-only.txt", "new")]);
    s.set_enabled("tweak", false).unwrap();
    let m = s.import_zip(&z).unwrap();
    assert_eq!((m.id.as_str(), m.enabled, m.files), ("tweak", false, 2));
    assert_eq!(ids(&s), ["tweak", "first", "last"], "keeps its place");
    s.set_enabled("tweak", true).unwrap();

    let r = s.apply().unwrap();
    assert!(r.added >= 2, "{r:?}");
    assert_eq!(w.read("Data/tweak.esp").as_deref(), Some("new"), "the new copy reaches the game");
    assert_eq!(w.read("Data/new-only.txt").as_deref(), Some("new"));
    assert!(w.read("Data/old-only.txt").is_none());
    assert_eq!(s.apply().unwrap().added, 0, "settled");
}

#[test]
fn remove_undo_and_purge() {
    let w = world("remove");
    let mut s = w.open();
    for n in ["A", "B", "C"] {
        s.import_zip(&w.zip(&format!("{n}.zip"), &[(&format!("{n}.esp"), n)])).unwrap();
    }
    s.apply().unwrap();
    s.set_clock(1_800_000_000);
    s.remove("b").unwrap();
    assert_eq!(ids(&s), ["a", "c"]);
    assert_eq!(s.removed().len(), 1);
    assert_eq!((s.removed()[0].item.id.as_str(), s.removed()[0].removed_at), ("b", 1_800_000_000));
    assert!(!s.root().join("mods/b").exists());
    assert_eq!(fs::read_to_string(s.root().join("removed/b/Data/B.esp")).unwrap(), "B");
    s.apply().unwrap();
    assert!(w.read("Data/B.esp").is_none(), "taken out of the game");

    s.undo_remove("b").unwrap();
    assert_eq!(ids(&s), ["a", "b", "c"], "back where it was");
    assert!(s.removed().is_empty());
    s.apply().unwrap();
    assert_eq!(w.read("Data/B.esp").as_deref(), Some("B"));

    // Old position past the end now: goes last.
    s.remove("c").unwrap();
    s.remove("a").unwrap();
    s.remove("b").unwrap();
    assert_eq!(s.removed().iter().map(|r| r.item.id.as_str()).collect::<Vec<_>>(), ["b", "a", "c"], "newest first");
    s.undo_remove("c").unwrap();
    assert_eq!(ids(&s), ["c"]);
    s.undo_remove("b").unwrap();
    assert_eq!(ids(&s), ["b", "c"]);
    s.remove("b").unwrap();

    // a and b were removed at 1_800_000_000; make b recent and reopen: a is purged, b stays.
    drop(s);
    let state = w.game.library.join(".turbine-mod-manager/steam-489830/state.json");
    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
    let text = fs::read_to_string(&state).unwrap();
    let old = now - KEEP_REMOVED_SECS - 10;
    let text = text.replacen("\"removedAt\":1800000000", &format!("\"removedAt\":{}", now - 60), 1);
    let text = text.replacen("\"removedAt\":1800000000", &format!("\"removedAt\":{old}"), 1);
    fs::write(&state, text).unwrap();
    let s = w.open();
    assert_eq!(s.removed().iter().map(|r| r.item.id.as_str()).collect::<Vec<_>>(), ["b"]);
    assert!(!s.root().join("removed/a").exists(), "purged files are deleted");
    assert!(s.root().join("removed/b").exists());
}

#[test]
fn play_original_and_restore() {
    let w = world("restore");
    let mut s = w.open();
    let original = snapshot(&w.game.install_dir);
    s.import_zip(&w.zip("A.zip", &[("a.esp", "a"), ("textures/sky.dds", "a sky")])).unwrap();
    s.import_zip(&w.zip("B.zip", &[("b.esp", "b")])).unwrap();
    s.set_clock(1000);
    s.apply().unwrap();
    assert_eq!(s.setups().len(), 1);
    assert_eq!((s.setups()[0].id.as_str(), s.setups()[0].saved_at, s.setups()[0].mods_on), ("1000", 1000, 2));
    assert_eq!(s.last_build().as_deref(), Some("100"));

    // Play original: game folder back to how it was, mods still on, no setup saved.
    let r = s.play_original().unwrap();
    assert_eq!(r.removed, 3);
    assert_eq!(snapshot(&w.game.install_dir), original);
    assert!(s.mods().iter().all(|m| m.enabled));
    assert_eq!(s.setups().len(), 1);
    s.apply().unwrap();
    assert_eq!(w.read("Data/b.esp").as_deref(), Some("b"));
    assert_eq!(s.setups().len(), 2);
    assert_eq!(s.setups()[1].id, "1000");
    assert_eq!(s.setups()[0].id, "1000-2", "same second: still unique, newest first");
    let both = s.setups()[0].id.clone();

    s.set_clock(2000);
    s.set_enabled("a", false).unwrap();
    s.move_to("b", 0).unwrap();
    s.apply().unwrap();
    assert_eq!(s.setups()[0].mods_on, 1);
    s.import_zip(&w.zip("C.zip", &[("c.esp", "c")])).unwrap();
    s.remove("b").unwrap();

    // Back to "both on": b is gone and skipped, c came later and is turned off.
    s.set_clock(3000);
    let r = s.restore(&both).unwrap();
    assert!(r.added > 0);
    assert_eq!(ids(&s), ["a", "c"]);
    assert_eq!(s.mods().iter().map(|m| m.enabled).collect::<Vec<_>>(), [true, false]);
    assert_eq!(w.read("Data/a.esp").as_deref(), Some("a"));
    assert!(w.read("Data/c.esp").is_none());
    assert!(w.read("Data/b.esp").is_none());
    assert!(s.restore("no-such-setup").is_err());
}

#[test]
fn setups_only_when_something_changed_and_at_most_ten() {
    let w = world("setups");
    let mut s = w.open();
    s.import_zip(&w.zip("A.zip", &[("a.esp", "a")])).unwrap();
    for i in 0..15u64 {
        s.set_clock(10_000 + i);
        s.set_enabled("a", i % 2 == 0).unwrap();
        s.apply().unwrap();
    }
    assert_eq!(s.setups().len(), KEEP_SETUPS);
    assert_eq!(s.setups()[0].saved_at, 10_014);
    assert_eq!(s.setups()[9].saved_at, 10_005);
    s.set_clock(20_000);
    let r = s.apply().unwrap();
    assert_eq!((r.added, r.removed), (0, 0));
    assert_eq!(s.setups()[0].saved_at, 10_014, "nothing changed: no new setup");

    // The update guard's build is the one at the last apply that changed something.
    drop(s);
    let mut w2 = w;
    w2.game.build = Some("200".into());
    let mut s = w2.open();
    assert_eq!(s.last_build().as_deref(), Some("100"));
    s.set_enabled("a", false).unwrap();
    s.apply().unwrap();
    assert_eq!(s.last_build().as_deref(), Some("200"));
}

#[test]
fn state_survives_reopening_and_json_parses() {
    let w = world("reopen");
    let mut s = w.open();
    s.import_zip(&w.zip("Quote \"Mod\"-2.0.zip", &[("q.esp", "q")])).unwrap();
    s.import_zip(&w.zip("Second.zip", &[("SKSE/Plugins/s.dll", "s")])).unwrap();
    s.import_zip(&w.zip("Third.zip", &[("t.esp", "t")])).unwrap();
    s.set_enabled("second", false).unwrap();
    s.move_to("third", 0).unwrap();
    s.apply().unwrap();
    s.remove("quote-mod").unwrap();
    let before = s.to_json();
    let mods = s.mods().to_vec();
    let removed = s.removed().to_vec();
    let setups = s.setups().to_vec();
    drop(s);

    let s = w.open();
    assert_eq!(s.mods(), &mods[..]);
    assert_eq!(s.removed(), &removed[..]);
    assert_eq!(s.setups(), &setups[..]);
    assert_eq!(s.to_json(), before);
    assert!(!s.root().join("state.json.tmp").exists());

    let v = json::parse(&before).unwrap();
    let m = &v.get("mods").unwrap().as_array()[1];
    assert_eq!(m.str_at("id"), Some("second"));
    assert_eq!(m.get("enabled").and_then(|b| b.as_bool()), Some(false));
    assert_eq!(m.get("needsLoader").and_then(|b| b.as_bool()), Some(true));
    assert_eq!(m.get("version"), Some(&json::Json::Null));
    for key in ["id", "name", "enabled", "version", "source", "files", "needsLoader", "added"] {
        assert!(m.get(key).is_some(), "{key}");
    }
    let r = &v.get("removed").unwrap().as_array()[0];
    assert_eq!(r.str_at("name"), Some("Quote \"Mod\""));
    assert!(matches!(r.get("removedAt"), Some(json::Json::Num(_))));
    let st = &v.get("setups").unwrap().as_array()[0];
    assert!(st.str_at("id").is_some() && st.get("savedAt").is_some() && st.get("modsOn").is_some());
    assert_eq!(v.str_at("lastBuild"), Some("100"));
}

#[test]
fn damaged_state_keeps_the_mods() {
    let w = world("damaged");
    let mut s = w.open();
    s.import_zip(&w.zip("Keep Me.zip", &[("k.esp", "k")])).unwrap();
    let root = s.root().to_path_buf();
    drop(s);
    fs::write(root.join("state.json"), "{\"mods\":[").unwrap();
    let s = w.open();
    assert_eq!(ids(&s), ["keep-me"]);
    assert!(!s.mods()[0].enabled, "picked up again, off until the player turns it on");
    assert_eq!(s.mods()[0].files, 1);
    assert!(root.join("state.damaged.json").exists());
}

#[test]
fn a_stop_mid_apply_is_finished_by_the_next_apply() {
    let w = world("stop");
    let mut s = w.open();
    let original = snapshot(&w.game.install_dir);
    s.import_zip(&w.zip("A.zip", &[("a.esp", "a"), ("textures/sky.dds", "a sky"), ("meshes/m.nif", "m")])).unwrap();
    assert!(s.apply_stopping(Some(2)).is_err());
    assert!(s.root().join("deploy/plan.json").exists());
    assert!(s.setups().is_empty());
    let r = s.apply().unwrap();
    assert!(r.recovered);
    assert_eq!(w.read("Data/a.esp").as_deref(), Some("a"));
    assert_eq!(w.read("Data/textures/sky.dds").as_deref(), Some("a sky"));
    assert_eq!(w.read("Data/meshes/m.nif").as_deref(), Some("m"));
    assert!(!s.root().join("deploy/plan.json").exists());
    s.play_original().unwrap();
    assert_eq!(snapshot(&w.game.install_dir), original);
}

#[test]
fn store_goes_on_the_game_drive() {
    let home = Path::new("/home/deck/.var/app/x/data");
    let sd = Path::new("/run/media/deck/SD/SteamLibrary");
    let key = "steam-489830";
    // Every library gets the hidden folder at its top (same device or not: separate mounts can't link).
    assert_eq!(choose_root(key, sd, Some(2), Some(1), true, home), sd.join(DRIVE_FOLDER).join(key));
    let internal = Path::new("/home/deck/.local/share/Steam");
    assert_eq!(choose_root(key, internal, Some(1), Some(1), true, home), internal.join(DRIVE_FOLDER).join(key));
    assert_eq!(choose_root(key, internal, None, None, true, home), internal.join(DRIVE_FOLDER).join(key));
    // Not writable: back to the data folder.
    assert_eq!(choose_root(key, sd, Some(2), Some(1), false, home), home.join("turbine-mod-manager").join(key));

    // A real game: its library is writable here.
    let w = world("root");
    assert_eq!(Store::root_for(&w.game, &w.data_home), w.game.library.join(".turbine-mod-manager/steam-489830"));
}

#[test]
fn downloads_lists_zips_newest_first() {
    let w = world("downloads");
    let dl = w.root.join("Downloads");
    let base = SystemTime::now() - Duration::from_secs(10_000);
    for (name, age) in [("old.zip", 300u64), ("new.ZIP", 10), ("mid.zip", 100), ("notes.txt", 0), ("archive.7z", 0)] {
        let p = dl.join(name);
        write(&p, "x");
        let f = fs::File::options().write(true).open(&p).unwrap();
        f.set_modified(base - Duration::from_secs(age)).unwrap();
    }
    fs::create_dir_all(dl.join("folder.zip")).unwrap();
    write(&dl.join("sub/deep.zip"), "x");
    let names: Vec<String> = downloads(&w.root).iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
    // 7z and rar are offered too (0.3).
    assert_eq!(names, ["archive.7z", "new.ZIP", "mid.zip", "old.zip"]);
    assert!(downloads(&w.root.join("nobody")).is_empty());
}

#[test]
fn downloads_all_includes_game_mods_folders() {
    let w = world("downloads-all");
    let dl = w.root.join("Downloads");
    let base = SystemTime::now() - Duration::from_secs(10_000);
    for (name, age) in [("top.zip", 50u64), ("Game Mods/The Witcher 3/aqom.zip", 10), ("Game Mods/Skyrim Special Edition/sky.7z", 30),
                        ("Game Mods/loose.zip", 40), ("Game Mods/The Witcher 3/aqom.zip.crdownload", 0), ("Game Mods/x/y/deep.zip", 0)] {
        let p = dl.join(name);
        write(&p, "x");
        let f = fs::File::options().write(true).open(&p).unwrap();
        f.set_modified(base - Duration::from_secs(age)).unwrap();
    }
    let got: Vec<(String, String)> = turbine_engine::store::downloads_all(&w.root)
        .into_iter()
        .map(|(p, f)| (p.file_name().unwrap().to_string_lossy().into_owned(), f))
        .collect();
    let want = [("aqom.zip", "The Witcher 3"), ("sky.7z", "Skyrim Special Edition"), ("loose.zip", ""), ("top.zip", "")];
    assert_eq!(got, want.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect::<Vec<_>>());
    // The plain list still only looks in Downloads itself.
    assert_eq!(downloads(&w.root).len(), 1);
}

#[test]
fn removed_mods_can_be_deleted_for_good() {
    let w = world("purge");
    let mut s = w.open();
    for n in ["one", "two", "three"] {
        s.import_zip(&w.zip(&format!("{n}.zip"), &[(&format!("{n}.esp"), n)])).unwrap();
    }
    let ids: Vec<String> = s.mods().iter().map(|m| m.id.clone()).collect();
    s.remove(&ids[0]).unwrap();
    s.remove(&ids[1]).unwrap();
    assert_eq!(s.removed().len(), 2);
    let removed_dir = s.root().join("removed");
    assert!(removed_dir.join(&ids[0]).exists());
    s.purge_removed(&ids[0]).unwrap();
    assert!(!removed_dir.join(&ids[0]).exists(), "its files are gone from Turbine's store");
    assert_eq!(s.removed().len(), 1);
    assert!(s.undo_remove(&ids[0]).is_err(), "it can't be put back");
    assert!(s.purge_removed("nope").is_err());
    // Still gone after opening again.
    drop(s);
    let mut s = w.open();
    assert_eq!(s.removed().len(), 1);
    assert_eq!(s.purge_all_removed().unwrap(), 1);
    assert!(s.removed().is_empty());
    assert_eq!(s.mods().len(), 1, "mods in the list are untouched");
    assert!(w.root.join("Downloads/one.zip").exists(), "the player's archive is never touched");
}

/// 0.58, Edit settings: the player's copy of a mod's config file goes into the game in place of the mod's own, a mod
/// update keeps the player's values, and Reset to default puts the mod's file back.
#[test]
fn edited_settings_reach_the_game_and_survive_updates() {
    use turbine_engine::settings;
    let w = world("settings");
    let mut s = w.open();
    let z = w.zip("Lanterns-1.0.zip", &[("lanterns.ini", "[a]\nbright = 1\nradius = 2\n")]);
    let m = s.import_zip(&z).unwrap();
    s.apply().unwrap();
    assert_eq!(w.read("Data/lanterns.ini").as_deref(), Some("[a]\nbright = 1\nradius = 2\n"));
    let staged = turbine_engine::store::staged_dir(s.root(), &m.id).join("Data/lanterns.ini");
    assert_eq!(settings::config_files(&turbine_engine::store::staged_dir(s.root(), &m.id), 6, 40), vec!["Data/lanterns.ini".to_string()]);
    let text = fs::read_to_string(&staged).unwrap();
    let f = settings::parse(&text, settings::Format::Ini);
    let edited = settings::set(&text, settings::Format::Ini, &[(f[1].id, "9".into())]).unwrap();
    settings::save_edit(s.root(), &m.id, "Data/lanterns.ini", &staged, &edited).unwrap();
    s.apply().unwrap();
    assert_eq!(w.read("Data/lanterns.ini").as_deref(), Some("[a]\nbright = 1\nradius = 9\n"));
    assert_eq!(fs::read_to_string(&staged).unwrap(), text, "the mod's own file is untouched");

    // An update with a new setting: the player's radius stays.
    write_zip(&z, &[("lanterns.ini", "[a]\nbright = 1\nradius = 2\ncolor = warm\n")]);
    s.import_zip(&z).unwrap();
    s.apply().unwrap();
    assert_eq!(w.read("Data/lanterns.ini").as_deref(), Some("[a]\nbright = 1\nradius = 9\ncolor = warm\n"));

    assert!(settings::reset(s.root(), &m.id, "Data/lanterns.ini").unwrap());
    s.apply().unwrap();
    assert_eq!(w.read("Data/lanterns.ini").as_deref(), Some("[a]\nbright = 1\nradius = 2\ncolor = warm\n"));
}

/// 0.62: Add from Downloads looks in the desktop's Downloads folder (the player moved it, or the desktop names it in
/// another language) as well as ~/Downloads, and in each one's Game Mods folders.
#[test]
fn downloads_follow_the_desktops_downloads_folder() {
    let d = std::env::temp_dir().join(format!("turbine-dl-moved-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    let home = d.join("home");
    let moved = d.join("sd card/Telechargements");
    for (p, text) in [
        (home.join("Downloads/Old 1.0.zip"), "PK"),
        (moved.join("New 2.0.zip"), "PK"),
        (moved.join("notes.txt"), "x"),
        (moved.join("Game Mods/Elden Ring/Armor 1.0.7z"), "7z"),
    ] {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }
    // The desktop says where Downloads is (user-dirs.dirs, as the sandbox sees it).
    fs::create_dir_all(home.join(".config")).unwrap();
    fs::write(home.join(".config/user-dirs.dirs"), format!("XDG_DOWNLOAD_DIR=\"{}\"\n", moved.display())).unwrap();
    let folder = turbine_engine::watch::downloads_folder_from(&home, None, &[home.join(".config")]);
    assert_eq!(folder, moved);
    let folders = vec![home.join("Downloads"), folder];
    let mut all: Vec<(String, String)> = turbine_engine::store::downloads_all_in(&folders).into_iter().map(|(p, f)| (p.file_name().unwrap().to_string_lossy().into_owned(), f)).collect();
    all.sort();
    assert_eq!(all, vec![("Armor 1.0.7z".to_string(), "Elden Ring".to_string()), ("New 2.0.zip".to_string(), String::new()), ("Old 1.0.zip".to_string(), String::new())]);
    let mut plain: Vec<String> = turbine_engine::store::downloads_in(&folders).iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
    plain.sort();
    assert_eq!(plain, vec!["New 2.0.zip".to_string(), "Old 1.0.zip".to_string()]);
    // One folder listed twice (the usual case: the desktop's folder is ~/Downloads) gives each file once.
    assert_eq!(turbine_engine::watch::default_folders(&d.join("nobody")).len(), 1);
    let _ = fs::remove_dir_all(&d);
}
