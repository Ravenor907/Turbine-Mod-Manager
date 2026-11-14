//! Saved state that went wrong: state.json, profiles.json and seen.json cut short, damaged, the wrong shape, not
//! readable, or with IDs that point outside the store. The mod list must never be lost without a word, and nothing
//! outside the store may be deleted.

mod support;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use support::write_zip_text as write_zip;
use turbine_engine::games::{Game, Runtime, Store as Launcher};
use turbine_engine::profiles::Profiles;
use turbine_engine::seen::Seen;
use turbine_engine::store::{prev_copy, safe_id, ModRules, Store, KEEP_DAMAGED};

struct R;

impl ModRules for R {
    fn map_archive(&self, files: &[String], _mod_name: &str) -> Result<Vec<(String, String)>, String> {
        Ok(files.iter().map(|f| (f.clone(), format!("Data/{f}"))).collect())
    }

    fn needs_loader(&self, _game_files: &[String]) -> bool {
        false
    }
}

struct World {
    root: PathBuf,
    game: Game,
    data_home: PathBuf,
}

impl Drop for World {
    fn drop(&mut self) {
        // Folders a test made read-only are opened up again so they can be cleared.
        for p in [self.root.join("library/.turbine-mod-manager/steam-1"), self.root.clone()] {
            let _ = fs::set_permissions(&p, fs::Permissions::from_mode(0o755));
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl World {
    fn open(&self) -> std::io::Result<Store> {
        Store::open_with(&self.game, &self.data_home, Box::new(R))
    }

    fn state(&self) -> PathBuf {
        Store::root_for(&self.game, &self.data_home).join("state.json")
    }
}

fn world(tag: &str) -> World {
    let root = std::env::temp_dir().join(format!("turbine-saved-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let library = root.join("library");
    let install = library.join("steamapps/common/G");
    fs::create_dir_all(install.join("Data")).unwrap();
    fs::write(install.join("G.exe"), "x").unwrap();
    let game = Game {
        store: Launcher::Steam,
        store_id: "1".into(),
        name: "G".into(),
        install_dir: install,
        library,
        build: None,
        runtime: Runtime::Windows,
        cover: None,
        header: None,
        prefix: None,
    };
    let data_home = root.join("data");
    World { root, game, data_home }
}

/// Three mods: beta (on) then alpha (on), and gamma in Recently removed.
fn with_mods(tag: &str) -> World {
    let w = world(tag);
    let mut s = w.open().unwrap();
    for n in ["Alpha", "Beta", "Gamma"] {
        let z = w.root.join(format!("dl/{n}.zip"));
        write_zip(&z, &[(&format!("{n}.esp"), "x")]);
        s.import_zip(&z).unwrap();
    }
    s.set_enabled("alpha", true).unwrap();
    s.set_enabled("beta", true).unwrap();
    s.move_to("beta", 0).unwrap();
    s.remove("gamma").unwrap();
    assert!(s.opened_note().is_none());
    w
}

/// One way to damage a file.
type Damage<'a> = Box<dyn Fn(&Path) + 'a>;

fn order(s: &Store) -> Vec<(String, bool)> {
    s.mods().iter().map(|m| (m.id.clone(), m.enabled)).collect()
}

fn names_in(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    v.sort();
    v
}

/// Running as root reads files whatever their permissions; the permission tests mean nothing then.
fn can_lock(dir: &Path) -> bool {
    let p = dir.join(".lock-test");
    fs::write(&p, "x").unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o000)).unwrap();
    let locked = fs::read(&p).is_err();
    let _ = fs::remove_file(&p);
    locked
}

#[test]
fn mod_ids_must_be_plain_folder_names() {
    for bad in ["", ".", "..", "../x", "a/b", "a\\b", "a\0b", ".hidden"] {
        assert!(!safe_id(bad), "{bad:?}");
    }
    for good in ["skyui", "mod-2", "Some Mod v1.2"] {
        assert!(safe_id(good), "{good:?}");
    }
}

#[test]
fn ids_pointing_outside_the_store_delete_nothing() {
    let w = with_mods("trav");
    let state = w.state();
    let victim = w.root.join("victim");
    fs::create_dir_all(victim.join("keep")).unwrap();
    fs::write(victim.join("keep/file.txt"), "precious").unwrap();
    fs::write(
        &state,
        "{\"version\":1,\"mods\":[{\"id\":\"../../../victim\",\"name\":\"x\"},{\"id\":\"alpha\",\"name\":\"Alpha\",\"enabled\":true}],\
         \"removed\":[{\"id\":\"../../../../victim\",\"name\":\"x\",\"removedAt\":0,\"position\":0},{\"id\":\"..\",\"name\":\"y\",\"removedAt\":0}]}",
    )
    .unwrap();
    let mut s = w.open().unwrap();
    assert_eq!(fs::read_to_string(victim.join("keep/file.txt")).unwrap(), "precious");
    assert!(s.mods().iter().all(|m| safe_id(&m.id)));
    assert!(s.mods().iter().any(|m| m.id == "alpha" && m.enabled));
    assert!(s.opened_note().unwrap().contains("3 entries"), "{:?}", s.opened_note());
    // The file as it was is kept, and the list on disk no longer has the bad entries.
    let kept = s.root().join("state.damaged.json");
    assert!(fs::read_to_string(kept).unwrap().contains("victim"));
    assert!(!fs::read_to_string(&state).unwrap().contains("victim"));
    // Asking for such an ID does nothing outside the store either.
    assert!(s.purge_removed("../../../../victim").is_err());
    assert!(s.remove("../../../victim").is_err());
    s.purge_all_removed().unwrap();
    assert!(victim.join("keep/file.txt").exists());
}

#[test]
fn a_damaged_list_comes_back_from_its_saved_copy() {
    let w = with_mods("copy");
    let state = w.state();
    let good = fs::read_to_string(&state).unwrap();
    assert_eq!(fs::read_to_string(prev_copy(&state)).unwrap(), good, "every save keeps the second copy");
    // Cut short (a full disk, a card pulled out), then empty, then not text, then a folder, then the wrong shape.
    let wrong_shape = good.replacen("\"mods\":[", "\"mods\":{\"list\":[", 1).replacen("],\n\"removed\"", "]},\n\"removed\"", 1);
    let damages: Vec<Damage<'_>> = vec![
        Box::new(|p: &Path| fs::write(p, &good[..good.len() / 2]).unwrap()),
        Box::new(|p: &Path| fs::write(p, "").unwrap()),
        Box::new(|p: &Path| {
            let mut b = good.clone().into_bytes();
            b[5] = 0xff;
            fs::write(p, b).unwrap()
        }),
        Box::new(|p: &Path| {
            fs::remove_file(p).unwrap();
            fs::create_dir(p).unwrap();
        }),
        Box::new(|p: &Path| fs::write(p, &wrong_shape).unwrap()),
        Box::new(|p: &Path| fs::write(p, good.replacen("\"version\":1", "\"version\":2", 1)).unwrap()),
        Box::new(|p: &Path| fs::remove_file(p).unwrap()),
    ];
    for (n, damage) in damages.iter().enumerate() {
        damage(&state);
        let s = w.open().unwrap();
        assert_eq!(order(&s), [("beta".to_string(), true), ("alpha".to_string(), true)], "damage {n}");
        assert_eq!(s.removed().len(), 1, "damage {n}");
        assert!(s.opened_note().unwrap().contains("saved copy"), "damage {n}");
        assert_eq!(fs::read_to_string(&state).unwrap(), good, "damage {n}: written back whole");
        assert!(s.root().join("removed/gamma").is_dir(), "damage {n}: Recently removed is still there");
    }
    // Every damaged copy has a name of its own; only the newest few are kept.
    let damaged: Vec<String> = names_in(state.parent().unwrap()).into_iter().filter(|n| n.starts_with("state.damaged")).collect();
    assert_eq!(damaged.len(), KEEP_DAMAGED, "{damaged:?}");
    assert!(damaged.contains(&"state.damaged-6.json".to_string()), "{damaged:?}");
    assert!(!damaged.contains(&"state.damaged.json".to_string()), "the oldest went first: {damaged:?}");
}

#[test]
fn with_both_copies_damaged_the_mods_and_recently_removed_stay() {
    let w = with_mods("both");
    let state = w.state();
    let good = fs::read_to_string(&state).unwrap();
    fs::write(&state, &good[..good.len() / 2]).unwrap();
    fs::write(prev_copy(&state), "{\"mods\":[").unwrap();
    let s = w.open().unwrap();
    let mut ids: Vec<String> = s.mods().iter().map(|m| m.id.clone()).collect();
    ids.sort();
    assert_eq!(ids, ["alpha", "beta"]);
    assert!(s.mods().iter().all(|m| !m.enabled));
    assert_eq!(s.removed().iter().map(|r| r.item.id.as_str()).collect::<Vec<_>>(), ["gamma"], "not deleted for good");
    assert!(s.root().join("removed/gamma").is_dir());
    let note = s.opened_note().unwrap();
    assert!(note.contains("all off") && note.contains("state.damaged.json"), "{note}");
    let root = s.root().to_path_buf();
    drop(s);
    // Both damaged copies are kept; a second time adds to them instead of writing over the first.
    assert_eq!(fs::read_to_string(root.join("state.damaged.json")).unwrap(), good[..good.len() / 2]);
    assert_eq!(fs::read_to_string(root.join("state.damaged-2.json")).unwrap(), "{\"mods\":[");
    fs::write(&state, "garbage").unwrap();
    fs::write(prev_copy(&state), "garbage").unwrap();
    let s = w.open().unwrap();
    assert_eq!(s.mods().len(), 2);
    assert_eq!(fs::read_to_string(root.join("state.damaged.json")).unwrap(), good[..good.len() / 2]);
    assert!(root.join("state.damaged-4.json").exists());
    // A list that opened fine says nothing.
    drop(s);
    assert!(w.open().unwrap().opened_note().is_none());
}

#[test]
fn an_unreadable_list_comes_back_from_its_saved_copy() {
    let w = with_mods("perm");
    let state = w.state();
    if !can_lock(state.parent().unwrap()) {
        return;
    }
    fs::set_permissions(&state, fs::Permissions::from_mode(0o000)).unwrap();
    let s = w.open().unwrap();
    assert_eq!(order(&s), [("beta".to_string(), true), ("alpha".to_string(), true)]);
    assert!(s.root().join("state.damaged.json").exists(), "the unreadable file is kept");
}

#[test]
fn a_read_only_store_on_the_card_is_reported_not_replaced() {
    let w = with_mods("ro");
    let store = Store::root_for(&w.game, &w.data_home);
    if !can_lock(&store) {
        return;
    }
    fs::set_permissions(&store, fs::Permissions::from_mode(0o555)).unwrap();
    assert_eq!(Store::root_for(&w.game, &w.data_home), store, "no new empty store elsewhere");
    let err = w.open().err().expect("a read-only store can't be opened for changes");
    assert!(err.to_string().contains("can only be read"), "{err}");
    assert!(!w.data_home.join("turbine-mod-manager").exists());
    fs::set_permissions(&store, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(w.open().unwrap().mods().len(), 2);
}

#[test]
fn damaged_loadouts_come_back_from_their_saved_copy() {
    let w = with_mods("prof");
    let s = w.open().unwrap();
    let mut p = Profiles::open(&s).unwrap();
    p.create(&s, "Coop").unwrap();
    p.create(&s, "Solo run").unwrap();
    let path = s.root().join("profiles.json");
    let good = fs::read_to_string(&path).unwrap();
    fs::write(&path, &good[..good.len() - 5]).unwrap();
    let mut p = Profiles::open(&s).unwrap();
    assert_eq!(p.names(), ["Default", "Coop", "Solo run"]);
    p.create(&s, "New").unwrap();
    assert!(fs::read_to_string(&path).unwrap().contains("Solo run"));
    assert!(s.root().join("profiles.damaged.json").exists());

    // Unreadable and can't be set aside: an error, never "only Default" saved over the real list.
    if can_lock(s.root()) {
        let before = fs::read_to_string(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
        fs::set_permissions(s.root(), fs::Permissions::from_mode(0o555)).unwrap();
        assert!(Profiles::open(&s).is_err());
        fs::set_permissions(s.root(), fs::Permissions::from_mode(0o755)).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), before);
    }

    // Both copies damaged: said once, and both are kept.
    fs::write(&path, "{").unwrap();
    fs::write(prev_copy(&path), "{").unwrap();
    let err = Profiles::open(&s).unwrap_err();
    assert!(err.to_string().contains("damaged"), "{err}");
    assert_eq!(Profiles::open(&s).unwrap().names(), ["Default"]);
}

#[test]
fn damaged_seen_comes_back_from_its_saved_copy() {
    let w = world("seen");
    fs::create_dir_all(&w.root).unwrap();
    let path = w.root.join("seen.json");
    let mut s = Seen::default();
    s.mark_launch("steam:1");
    s.save(&path).unwrap();
    fs::write(&path, "{\"launchDone\":[\"a\"").unwrap();
    assert!(Seen::load(&path).launch_seen("steam:1"));
    assert!(w.root.join("seen.damaged.json").exists());
}
