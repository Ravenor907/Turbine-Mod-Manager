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
        Need { name: "SkyUI".into(), id: Some("skyui".into()) },
        Need { name: "Address Library".into(), id: Some("addr".into()) },
        Need { name: "Missing Thing".into(), id: None },
    ]);
    assert_eq!(l[0].2, vec!["hud".to_string()]);
    assert_eq!(l[1].2, vec!["hud".to_string()]);
    // Thunderstore: the loader pack left out, versions dropped, case ignored.
    assert_eq!(l[3].1, vec![Need { name: "LethalLib".into(), id: Some("lib".into()) }, Need { name: "More Suits Core".into(), id: None }]);
    assert_eq!(l[4].2, vec!["suits".to_string()]);
    let j = needs::links_json(&l);
    assert!(j.contains("\"hud\":{\"needs\":[{\"name\":\"SkyUI\",\"id\":\"skyui\"}") && j.contains("\"neededBy\":[\"hud\"]"), "{j}");
    assert!(!j.contains("\"zip\""));
}
