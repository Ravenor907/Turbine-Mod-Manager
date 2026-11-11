//! Requirements both ways between installed mods.

use turbine_engine::needs::{self, Need};
use turbine_engine::net::SourceInfo;

fn src(kind: &str, id: &str, needs: &[&str]) -> SourceInfo {
    SourceInfo { kind: kind.into(), id: id.into(), needs: needs.iter().map(|s| s.to_string()).collect(), ..SourceInfo::default() }
}

#[test]
fn needs_and_needed_by() {
    let mods = vec![
        ("skyui".to_string(), src("nexus", "skyrimspecialedition:12604:100", &[])),
        ("addr".to_string(), src("nexus", "skyrimspecialedition:32444", &[])),
        ("hud".to_string(), src("nexus", "skyrimspecialedition:500:1", &[&needs::nexus_need("skyrimspecialedition", 12604, "SkyUI"), &needs::nexus_need("skyrimspecialedition", 32444, "Address Library"), "nexus:skyrimspecialedition:777|Missing Thing"])),
        ("suits".to_string(), src("thunderstore", "x753-More_Suits", &["BepInEx-BepInExPack-5.4.2100", "Evaisa-LethalLib-0.16.0", "x753-More_Suits_Core-1.0.0"])),
        ("lib".to_string(), src("thunderstore", "Evaisa-LethalLib", &[])),
        ("zip".to_string(), src("zip", "", &[])),
    ];
    let l = needs::links(&mods);
    assert_eq!(l[2].1, vec![
        Need { name: "SkyUI".into(), id: Some("skyui".into()), domain: "skyrimspecialedition".into(), mod_id: 12604 },
        Need { name: "Address Library".into(), id: Some("addr".into()), domain: "skyrimspecialedition".into(), mod_id: 32444 },
        Need { name: "Missing Thing".into(), id: None, domain: "skyrimspecialedition".into(), mod_id: 777 },
    ]);
    assert_eq!(l[0].2, vec!["hud".to_string()]);
    assert_eq!(l[1].2, vec!["hud".to_string()]);
    // Thunderstore: the loader pack left out, versions dropped, case ignored.
    assert_eq!(l[3].1, vec![
        Need { name: "LethalLib".into(), id: Some("lib".into()), domain: String::new(), mod_id: 0 },
        Need { name: "More Suits Core".into(), id: None, domain: String::new(), mod_id: 0 }
    ]);
    assert_eq!(l[4].2, vec!["suits".to_string()]);
    let j = needs::links_json(&l);
    assert!(j.contains("\"hud\":{\"needs\":[{\"name\":\"SkyUI\",\"id\":\"skyui\",\"domain\":\"skyrimspecialedition\",\"modId\":12604}") && j.contains("\"neededBy\":[\"hud\"]"), "{j}");
    // A missing Nexus requirement says where to get it.
    assert!(j.contains("{\"name\":\"Missing Thing\",\"id\":null,\"domain\":\"skyrimspecialedition\",\"modId\":777}"), "{j}");
    assert!(!j.contains("\"zip\""));
}

// ---------------------------------------------------------------- what the interface asks

use turbine_engine::json;
use turbine_engine::needs::{GetItem, ReqQueue};

/// Alpha (on) needs Beta (installed, off), Gamma (installed, on) and Lib (not installed, Nexus 7); Delta (on)
/// needs Lib too and Other (not installed, on another game's Nexus); Beta (off) needs Lib.
fn world() -> json::Json {
    json::parse(
        r#"{"domain":"skyrim","mods":[
            {"id":"a","name":"Alpha","enabled":true},{"id":"b","name":"Beta","enabled":false},
            {"id":"g","name":"Gamma","enabled":true},{"id":"d","name":"Delta","enabled":true}],
          "links":{
            "a":{"needs":[{"name":"Beta","id":"b","domain":"","modId":0},{"name":"Gamma","id":"g","domain":"","modId":0},{"name":"Lib","id":null,"domain":"","modId":7}],"neededBy":[]},
            "b":{"needs":[{"name":"Lib","id":"","domain":"","modId":7}],"neededBy":["a"]},
            "d":{"needs":[{"name":"Lib","id":null,"domain":"","modId":7},{"name":"Other","id":null,"domain":"fallout4","modId":9},{"name":"No page","id":null,"domain":"","modId":0}],"neededBy":[]}}}"#,
    )
    .unwrap()
}

#[test]
fn missing_split_and_together() {
    let w = world();
    let mods = needs::shown_from(w.get("mods"));
    let links = needs::links_from(w.get("links"));
    let names = |id: &str| needs::missing(&mods, &links, id).iter().map(|n| n.name.clone()).collect::<Vec<_>>();
    assert_eq!(names("a"), ["Beta", "Lib"], "Gamma is on");
    assert_eq!(names("b"), ["Lib"], "an empty id is no provider");
    assert!(names("g").is_empty());
    let s = needs::split(&mods, &links, "a", "skyrim");
    assert_eq!(s.off, ["b"]);
    assert_eq!(s.get, [GetItem { name: "Lib".into(), mod_id: 7, domain: "skyrim".into() }]);
    let s = needs::split(&mods, &links, "d", "skyrim");
    assert_eq!(s.get.iter().map(|g| (g.name.as_str(), g.domain.as_str())).collect::<Vec<_>>(), [("Lib", "skyrim"), ("Other", "fallout4")], "no Nexus ID: nothing to get");
    // Together: only the mods that are on, each missing mod once.
    let all = needs::missing_on(&mods, &links, "skyrim");
    assert_eq!(all.names, ["Alpha", "Delta"]);
    assert_eq!(all.off, ["b"]);
    assert_eq!(all.get.iter().map(|g| g.mod_id).collect::<Vec<_>>(), [7, 9]);
    assert_eq!(all.words, "Alpha needs Beta and Lib. Delta needs Lib and Other and No page.");
}

#[test]
fn more_than_three_are_counted() {
    let mods: Vec<String> = (0..5).map(|i| format!(r#"{{"id":"m{i}","name":"M{i}","enabled":true}}"#)).collect();
    let links: Vec<String> = (0..5).map(|i| format!(r#""m{i}":{{"needs":[{{"name":"L","id":null,"domain":"","modId":1}}],"neededBy":[]}}"#)).collect();
    let w = json::parse(&format!(r#"{{"mods":[{}],"links":{{{}}}}}"#, mods.join(","), links.join(","))).unwrap();
    let all = needs::missing_on(&needs::shown_from(w.get("mods")), &needs::links_from(w.get("links")), "");
    assert_eq!(all.words, "M0 needs L. M1 needs L. M2 needs L. And 2 more mods.");
    let four = json::parse(&w.to_text().replace(r#",{"id":"m4","name":"M4","enabled":true}"#, "")).unwrap();
    assert!(needs::missing_on(&needs::shown_from(four.get("mods")), &needs::links_from(four.get("links")), "").words.ends_with("And 1 more mod."));
}

#[test]
fn the_view_in_one_answer() {
    let v = json::parse(&needs::view_json(&world())).unwrap();
    let missing = v.get("missing").unwrap();
    assert_eq!(missing.entries().iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(), ["a", "b", "d"]);
    assert_eq!(missing.get("a").unwrap().as_array()[0].str_at("id"), Some("b"));
    assert_eq!(v.get("split").unwrap().get("a").unwrap().get("off").unwrap().as_array().len(), 1);
    assert_eq!(v.get("all").unwrap().str_at("words"), Some("Alpha needs Beta and Lib. Delta needs Lib and Other and No page."));
}

#[test]
fn the_queue_goes_one_after_another() {
    let item = |name: &str, id: u64| GetItem { name: name.into(), mod_id: id, domain: "d".into() };
    let mut q = ReqQueue::new("g", "Alpha", vec![item("A", 1), item("B", 2)]);
    assert_eq!(q.current(), Some(&item("A", 1)));
    assert_eq!(q.advance(), Some(item("A", 1)));
    // B needs C and A (A is in the list already): C goes before B.
    assert_eq!(q.insert_next(&[item("C", 3), item("A again", 1)]), [item("C", 3)]);
    assert_eq!(q.items.iter().map(|i| i.mod_id).collect::<Vec<_>>(), [1, 3, 2]);
    assert_eq!(q.current(), Some(&item("C", 3)));
    q.advance();
    q.advance();
    assert_eq!(q.current(), None);
    assert_eq!(q.advance(), None, "past the end it still counts on");
    assert_eq!(q.index, 4);
    let back = ReqQueue::from_json(&json::parse(&q.to_json()).unwrap()).unwrap();
    assert_eq!(back, q);
    assert!(ReqQueue::from_json(&json::parse(r#"{"game":"g"}"#).unwrap()).is_none(), "no list: no queue");
}
