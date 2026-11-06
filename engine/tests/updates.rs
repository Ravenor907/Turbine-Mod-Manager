//! Update checks, offline, against the Thunderstore and Nexus samples in tests/data.

use std::cmp::Ordering::*;
use std::fs;
use std::path::Path;
use turbine_engine::nexus;
use turbine_engine::thunderstore;
use turbine_engine::updates::{self, compare_versions, Installed};

fn data(name: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data").join(name)).unwrap()
}

#[test]
fn versions_compare_by_number() {
    assert_eq!(compare_versions("1.10.0", "1.9.9"), Greater);
    assert_eq!(compare_versions("1.2", "1.2.0"), Equal);
    assert_eq!(compare_versions("v2.0", "1.99"), Greater);
    assert_eq!(compare_versions("1.0.0-beta", "1.0.0"), Less);
    assert_eq!(compare_versions("1.0.0-beta.2", "1.0.0-beta.10"), Less);
    assert_eq!(compare_versions("5.2SE", "5.1SE"), Greater);
    assert_eq!(compare_versions("5.2", "5.2SE"), Greater);
    assert_eq!(compare_versions("0.0.5", "0.0.5"), Equal);
    assert_eq!(compare_versions("1.0.0+build7", "1.0.0"), Equal);
    assert_eq!(compare_versions("", "0"), Equal);
    assert_eq!(compare_versions("99999999999999999999999.1", "1"), Greater);
}

fn installed(kind: &str, id: &str, v: Option<&str>, at: u64) -> Installed {
    Installed { kind: kind.into(), id: id.into(), version: v.map(Into::into), installed_at: at }
}

#[test]
fn thunderstore_updates_from_the_index() {
    let (index, _) = thunderstore::parse_index(data("thunderstore_v1_lethal.json").as_bytes()).unwrap();
    let mine = vec![
        installed("thunderstore", "x753-More_Suits", Some("1.4.3"), 0),
        installed("thunderstore", "evaisa-lethallib", Some("0.16.1"), 0),
        installed("thunderstore", "Oldie-OldThing", Some("1.0.0"), 0),
        installed("thunderstore", "Gone-Away", Some("1.0.0"), 0),
        installed("zip", "x753-More_Suits", Some("0.1"), 0),
        installed("thunderstore", "Evaisa-HookGenPatcher", None, 0),
    ];
    let ups = updates::thunderstore_updates(&mine, &index);
    let got: Vec<(&str, &str)> = ups.iter().map(|u| (u.id.as_str(), u.new_version.as_str())).collect();
    assert_eq!(got, vec![("x753-More_Suits", "1.10.0"), ("Oldie-OldThing", "2.0.0"), ("Evaisa-HookGenPatcher", "0.0.5")]);
    assert_eq!(ups[0].target, "x753-More_Suits-1.10.0");
    assert_eq!(ups[0].installed_version.as_deref(), Some("1.4.3"));
    assert!(ups[1].deprecated);
}

#[test]
fn nexus_updates_one_list_per_game_then_files() {
    let updated = nexus::parse_updated(&data("nexus_updated.json")).unwrap();
    let mine = vec![
        installed("nexus", "skyrimspecialedition:12604:100", Some("5.1SE"), 1_500_000_000),
        installed("nexus", "skyrimspecialedition:555:1", Some("1.0"), 1_500_000_000),
        installed("nexus", "skyrimspecialedition:99:5", Some("1.0"), 1_800_000_000),
        installed("nexus", "stardewvalley:12604:100", Some("1.0"), 0),
        installed("thunderstore", "skyrimspecialedition:12604:100", None, 0),
        installed("nexus", "broken-id", None, 0),
    ];
    let cand = updates::nexus_candidates("skyrimspecialedition", &mine, &updated);
    // Only 12604 changed after it was installed (99 changed before its install; 555 isn't in the list).
    assert_eq!(cand.iter().map(|(id, _)| *id).collect::<Vec<_>>(), vec![12604]);

    let files = nexus::parse_files(&data("nexus_files.json")).unwrap();
    let up = updates::nexus_update_from_files(cand[0].1, &files).unwrap();
    assert_eq!(up.id, "skyrimspecialedition:12604:200");
    assert_eq!(up.new_version, "5.2SE");
    assert_eq!(up.target, "200");

    // Already on the newest main file: nothing.
    assert_eq!(updates::nexus_update_from_files(&installed("nexus", "skyrimspecialedition:12604:200", Some("5.2SE"), 0), &files), None);
    // An optional add-on isn't "updated" to the main file.
    assert_eq!(updates::nexus_update_from_files(&installed("nexus", "skyrimspecialedition:12604:300", Some("1.0"), 0), &files), None);
    // Without a replaces link, a newer main version still counts.
    let mut no_links = files.clone();
    no_links.updates.clear();
    let up = updates::nexus_update_from_files(&installed("nexus", "skyrimspecialedition:12604:100", Some("5.1SE"), 0), &no_links).unwrap();
    assert_eq!(up.target, "200");
}
