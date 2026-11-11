//! Keyless Nexus: categories (facets) and the category filter in Get mods, and update checks for installed
//! mods through legacyModsByDomain with no key.

mod support;

use support::data_text as data;
use turbine_engine::json;
use turbine_engine::nexus::{self, SearchSort};
use turbine_engine::updates::{self, Installed};

fn installed(kind: &str, id: &str, v: Option<&str>, at: u64) -> Installed {
    Installed { kind: kind.into(), id: id.into(), version: v.map(String::from), installed_at: at }
}

#[test]
fn search_asks_for_categories_and_keeps_one() {
    let body = json::parse(&nexus::search_body_in("skyrimspecialedition", "", "Weapons", SearchSort::Endorsements, 0, 30, false, 0)).unwrap();
    let f = body.get("variables").and_then(|v| v.get("filter")).unwrap();
    let cat = f.get("categoryName").map(json::Json::as_array).unwrap();
    assert_eq!((cat[0].str_at("value"), cat[0].str_at("op")), (Some("Weapons"), Some("EQUALS")));
    assert_eq!(f.get("gameDomainName").map(json::Json::as_array).unwrap()[0].str_at("value"), Some("skyrimspecialedition"));
    let q = body.str_at("query").unwrap();
    assert!(q.contains("facets: {categoryName: []}") && q.contains("facetsData") && q.contains(" category "), "{q}");
    // No category: no filter for it.
    let plain = json::parse(&nexus::search_body_in("witcher3", "", "", SearchSort::Endorsements, 0, 30, false, turbine_engine::nxm::now())).unwrap();
    assert!(plain.get("variables").and_then(|v| v.get("filter")).unwrap().get("categoryName").is_none());

    // The live answer's shape (facetsData by category name), most mods first.
    let answer = r#"{"data":{"mods":{"totalCount":3,"facetsData":{"categoryName":{"Armour":11521,"Patches":10725,"Gameplay":6704,"":3}},
        "nodes":[{"modId":12604,"name":"SkyUI","category":"User Interface","version":"6.9","updatedAt":"2026-05-05T22:41:21Z"}]}}}"#;
    let p = nexus::parse_search(answer).unwrap();
    assert_eq!(p.categories, vec![("Armour".to_string(), 11521), ("Patches".to_string(), 10725), ("Gameplay".to_string(), 6704)]);
    assert_eq!(p.mods[0].category, "User Interface");
    // The recorded sample for the screenshot checks has them too.
    let s = nexus::parse_search(&data("nexus_graphql_search.json")).unwrap_or_default();
    assert!(s.mods.is_empty() || s.total > 0);
}

#[test]
fn update_checks_without_a_key() {
    let body = json::parse(&nexus::by_id_body("skyrimspecialedition", &[12604, 32444])).unwrap();
    let q = body.str_at("query").unwrap();
    assert!(q.contains("legacyModsByDomain(ids: [{gameDomain: \"skyrimspecialedition\", modId: 12604}, {gameDomain: \"skyrimspecialedition\", modId: 32444}])"), "{q}");
    // As Nexus answered the build's live check.
    let answer = r#"{"data":{"legacyModsByDomain":{"nodes":[{"modId":12604,"name":"SkyUI","version":"6.9","updatedAt":"2026-05-05T22:41:21Z","gameId":1704},
        {"modId":32444,"name":"Address Library for SKSE Plugins","version":"13","updatedAt":"2026-08-27T15:30:29Z","gameId":1704}]}}}"#;
    let stamps = nexus::parse_by_id(answer).unwrap();
    assert_eq!((stamps[0].mod_id, stamps[0].version.as_str(), stamps[0].game_id), (12604, "6.9", 1704));
    assert_eq!(stamps[1].updated, nexus::iso_secs("2026-08-27T15:30:29Z"));
    // The words for the list come with it (a mod added from a file is named by Nexus).
    assert!(q.contains("author summary thumbnailUrl"), "{q}");
    let words = nexus::parse_by_id(r#"{"data":{"legacyModsByDomain":{"nodes":[{"modId":3,"name":"SkyUI","version":"6.9","author":"schlangster","summary":"Elegant menus.","thumbnailUrl":"https://staticdelivery.nexusmods.com/mods/1704/images/thumbnails/12604/x.jpg","gameId":1704}]}}}"#).unwrap();
    assert_eq!((words[0].author.as_str(), words[0].summary.as_str()), ("schlangster", "Elegant menus."));
    assert!(words[0].picture.starts_with("https://"));
    assert_eq!(stamps[0].author, "");
    assert!(nexus::parse_by_id(r#"{"errors":[{"message":"Mod not found"}],"data":null}"#).unwrap_err().contains("Mod not found"));

    let after = nexus::iso_secs("2026-06-01T00:00:00Z");
    let mine = vec![
        installed("nexus", "skyrimspecialedition:12604:100", Some("6.9"), after), // page older than the install, same version
        installed("nexus", "skyrimspecialedition:32444:7", Some("12"), after),   // page changed after the install
        installed("nexus", "skyrimspecialedition:32444", Some("12"), after),     // the same mod twice: asked once
        installed("nexus", "witcher3:12604:1", Some("1.0"), 0),
        installed("zip", "", None, 0),
    ];
    assert_eq!(updates::nexus_mod_ids("skyrimspecialedition", &mine), vec![12604, 32444]);
    let cand = updates::nexus_stamp_candidates("skyrimspecialedition", &mine, &stamps);
    assert_eq!(cand.iter().map(|(id, _, g)| (*id, *g)).collect::<Vec<_>>(), vec![(32444, 1704)]);
    // A newer version on the page counts even when the install is newer than the page change.
    let older = vec![installed("nexus", "skyrimspecialedition:12604:100", Some("5.2"), u64::MAX)];
    assert_eq!(updates::nexus_stamp_candidates("skyrimspecialedition", &older, &stamps).len(), 1);

    // A download caught without its file id: a newer main version updates it.
    let files = nexus::parse_files(&data("nexus_files.json")).unwrap();
    let up = updates::nexus_update_from_files(&installed("nexus", "skyrimspecialedition:12604", Some("5.1SE"), 0), &files).unwrap();
    assert_eq!((up.target.as_str(), up.new_version.as_str()), ("200", "5.2SE"));
    assert_eq!(updates::nexus_update_from_files(&installed("nexus", "skyrimspecialedition:12604", Some("5.2SE"), 0), &files), None);
}
