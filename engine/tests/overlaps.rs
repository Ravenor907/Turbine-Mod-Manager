//! Who wins a file two mods both change: later in the list wins, case ignored, turned-off mods left out.

mod support;

use support::{game, write_zip, TempDir};
use turbine_engine::games::Store as Launcher;
use turbine_engine::overlaps::{overlaps, to_json};
use turbine_engine::rules::ModFiles;
use turbine_engine::store::{Import, SourceInfo, Store};

fn m(id: &str, on: bool, files: &[&str]) -> ModFiles {
    ModFiles { id: id.into(), name: format!("Mod {id}"), enabled: on, files: files.iter().map(|s| s.to_string()).collect() }
}

#[test]
fn later_wins_and_hidden() {
    let list = vec![
        m("a", true, &["Data/a.esp", "Data/meshes/x.nif", "Data/textures/t.dds"]),
        m("b", true, &["Data/Meshes/X.nif", "Data/b.esp"]),
        m("c", true, &["data/textures/t.dds", "data/meshes/x.nif"]),
        m("off", false, &["Data/meshes/x.nif"]),
        m("alone", true, &["Data/alone.esp"]),
    ];
    let o = overlaps(&list);
    assert_eq!(o.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(), vec!["a", "b", "c"], "only mods that share files, in order");
    let a = &o[0];
    assert!(a.wins.is_empty());
    // A file is lost to the mod whose copy the game gets (the last one), not to every later one.
    assert_eq!(a.loses.iter().map(|s| (s.id.as_str(), s.count)).collect::<Vec<_>>(), vec![("c", 2)]);
    assert!(!a.hidden, "a.esp is still its own");
    let b = &o[1];
    assert!(b.wins.is_empty() && !b.hidden);
    assert_eq!((b.loses[0].id.as_str(), b.loses[0].count), ("c", 1));
    assert_eq!(b.loses[0].files, vec!["Data/meshes/x.nif".to_string()], "spelled as the first mod has it");
    let c = &o[2];
    assert_eq!(c.wins.iter().map(|s| (s.id.as_str(), s.count)).collect::<Vec<_>>(), vec![("a", 2), ("b", 1)]);
    // Every file replaced: hidden.
    let o = overlaps(&[m("x", true, &["a.txt", "b.txt"]), m("y", true, &["A.txt", "B.TXT", "c.txt"])]);
    assert!(o[0].hidden && !o[1].hidden);
    let j = to_json(&o);
    assert!(j.starts_with("{\"x\":{\"wins\":[],\"loses\":[{\"id\":\"y\",\"name\":\"Mod y\",\"count\":2"), "{j}");
    assert!(j.contains("\"hidden\":true"));
    assert_eq!(to_json(&overlaps(&[m("x", true, &["a"]), m("y", false, &["a"])])), "{}");
}

#[test]
fn from_the_store_after_a_move() {
    let t = TempDir::new("overlaps");
    let dir = t.path("lib/Game");
    std::fs::create_dir_all(&dir).unwrap();
    let g = game(Launcher::Steam, "99999", "Some Game", &dir, None);
    let mut s = Store::open(&g, &t.path("data")).unwrap();
    let src = || SourceInfo { kind: "zip".into(), ..SourceInfo::default() };
    let a = t.path("dl/A.zip");
    write_zip(&a, &[("textures/rock.dds", b"a"), ("textures/sand.dds", b"a")]);
    let b = t.path("dl/B.zip");
    write_zip(&b, &[("textures/rock.dds", b"b")]);
    let Import::Added(ma) = s.import_archive(&a, src()).unwrap() else { panic!() };
    let Import::Added(mb) = s.import_archive(&b, src()).unwrap() else { panic!() };
    let o = s.overlaps();
    assert_eq!((o[0].id.as_str(), o[0].loses[0].id.as_str()), (ma.item.id.as_str(), mb.item.id.as_str()));
    // "Make this one win": B moved above A, so A (now later) wins, and B changes nothing any more.
    s.move_to(&mb.item.id, 0).unwrap();
    let o = s.overlaps();
    assert_eq!(o[0].id, mb.item.id);
    assert!(o[0].hidden);
    assert_eq!(o[1].wins[0].id, mb.item.id);
    s.set_enabled(&ma.item.id, false).unwrap();
    assert!(s.overlaps().is_empty());
}
