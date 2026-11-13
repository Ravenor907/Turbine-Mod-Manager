//! FOMOD installers: reading ModuleConfig.xml and info.xml, and the step by step session.

mod support;

use std::fs;
use support::utf16le;
use turbine_engine::fomod::{self, Env, FileState, GroupKind, PluginType, Session};

fn sample(name: &str) -> Vec<u8> {
    fs::read(format!("{}/tests/data/fomod/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

fn list(files: &[&str]) -> Vec<String> {
    files.iter().map(|s| s.to_string()).collect()
}

const SKYUI_FILES: &[&str] = &[
    "SkyUI_5_2_SE/fomod/ModuleConfig.xml",
    "SkyUI_5_2_SE/fomod/info.xml",
    "SkyUI_5_2_SE/fomod/images/full.png",
    "SkyUI_5_2_SE/Interface/skyui/a.swf",
    "SkyUI_5_2_SE/Interface/Translations/skyui_english.txt",
    "SkyUI_5_2_SE/SkyUI_SE.bsa",
    "SkyUI_5_2_SE/plugins/SkyUI_SE.esp",
    "SkyUI_5_2_SE/Plugins/SkyUI_SE_NoMCM.esp",
];

#[test]
fn finds_the_installer_under_a_wrapper() {
    let f = fomod::find(&list(SKYUI_FILES)).unwrap();
    assert_eq!(f.root, "SkyUI_5_2_SE/");
    assert_eq!(f.config.as_deref(), Some("SkyUI_5_2_SE/fomod/ModuleConfig.xml"));
    assert_eq!(f.info.as_deref(), Some("SkyUI_5_2_SE/fomod/info.xml"));
    let f = fomod::find(&list(&["FOMOD/moduleconfig.XML", "a.esp", "Deeper/fomod/ModuleConfig.xml"])).unwrap();
    assert_eq!((f.root.as_str(), f.config.as_deref()), ("", Some("FOMOD/moduleconfig.XML")));
    assert!(fomod::find(&list(&["a.esp", "fomodish/ModuleConfig.xml"])).is_none());
    // Only info.xml: found, but nothing to run.
    let f = fomod::find(&list(&["fomod/info.xml", "a.esp"])).unwrap();
    assert!(f.config.is_none());
}

#[test]
fn skyui_like_installer() {
    let cfg = fomod::parse(&sample("skyui_like.xml")).unwrap();
    assert_eq!(cfg.module_name, "SkyUI");
    assert_eq!(cfg.module_image.as_deref(), Some("fomod\\images\\header.png"));
    assert_eq!(cfg.required.len(), 2);
    let g = &cfg.steps[0].groups[0];
    assert_eq!(g.kind, GroupKind::ExactlyOne);
    // Explicit order is kept.
    assert_eq!(g.plugins[0].name, "Full (recommended)");
    assert_eq!(g.plugins[0].description, "Everything, including the mod configuration menu & <its> settings.");
    assert_eq!(g.plugins[1].description, "Only the new menus. Tom & Jerry's \u{2764} test &unknown; stays.");

    let env = Env::with_plugins(&[("Skyrim.esm".into(), true)]);
    let mut s = Session::new(cfg, env, &list(SKYUI_FILES), "SkyUI_5_2_SE/");
    assert!(s.has_choices());
    assert!(s.requirement_warning().is_none());
    let step = s.current().unwrap();
    assert!(step.is_last && !step.can_go_back);
    assert_eq!(step.groups[0].options[0].kind, PluginType::Recommended);
    assert!(step.groups[0].options[0].selected && !step.groups[0].options[1].selected);
    // The image path resolves to the archive's own casing.
    assert_eq!(s.archive_path("fomod/images/full.png").as_deref(), Some("SkyUI_5_2_SE/fomod/images/full.png"));

    let done = s.finish().unwrap();
    let mut files = done.files.clone();
    files.sort();
    assert_eq!(
        files,
        vec![
            ("SkyUI_5_2_SE/Interface/Translations/skyui_english.txt".to_string(), "Interface/Translations/skyui_english.txt".to_string()),
            ("SkyUI_5_2_SE/Interface/skyui/a.swf".into(), "Interface/skyui/a.swf".into()),
            ("SkyUI_5_2_SE/SkyUI_SE.bsa".into(), "SkyUI_SE.bsa".into()),
            // Source written as Plugins\SkyUI_SE.esp, archive has plugins/: matched ignoring case.
            ("SkyUI_5_2_SE/plugins/SkyUI_SE.esp".into(), "SkyUI_SE.esp".into()),
        ]
    );
    assert!(done.missing.is_empty());

    // Pick the other one: it replaces the first (pick one), and can't be unpicked.
    s.toggle(0, 1).unwrap();
    let step = s.current().unwrap();
    assert!(!step.groups[0].options[0].selected && step.groups[0].options[1].selected);
    assert!(s.deselect(0, 1).is_err());
    let done = s.finish().unwrap();
    assert!(done.files.contains(&("SkyUI_5_2_SE/Plugins/SkyUI_SE_NoMCM.esp".into(), "SkyUI_SE.esp".into())));
    assert!(!done.files.iter().any(|(f, _)| f.ends_with("/SkyUI_SE.esp")));
    assert!(!s.next().unwrap(), "no step after the last");

    // The mod's own requirement isn't met without Skyrim.esm turned on.
    let cfg = fomod::parse(&sample("skyui_like.xml")).unwrap();
    let s = Session::new(cfg, Env::default(), &list(SKYUI_FILES), "SkyUI_5_2_SE/");
    assert!(s.requirement_warning().unwrap().contains("Skyrim.esm"));
    let json = s.to_json();
    assert!(json.contains("\"module\":\"SkyUI\"") && json.contains("\"hint\":\"Pick one\""), "{json}");
    assert!(turbine_engine::json::parse(&json).is_ok());
}

const BAP_FILES: &[&str] = &[
    "fomod/ModuleConfig.xml",
    "fomod/unp.jpg",
    "00 Core/BAP.esp",
    "00 Core/meshes/armor/bap/shared.nif",
    "10 Heels/sound/heels.wav",
    "11 Physics/BAP Physics.esp",
    "11 Physics/readme physics.txt",
    "20 Patches/BAP - USSEP.esp",
    "20 Patches/BAP - Lists.esp",
    "30 Bodies/CBBE/cuirass.nif",
    "30 Bodies/CBBE/meshes_extra/boots.nif",
    "30 Bodies/UNP/cuirass.nif",
    "40 Textures/4K/armor.dds",
    "40 Textures/2K/armor.dds",
];

fn bap(env: Env) -> Session {
    let cfg = fomod::parse(&sample("multistep.xml")).unwrap();
    Session::new(cfg, env, &list(BAP_FILES), "")
}

fn sorted(mut v: Vec<(String, String)>) -> Vec<(String, String)> {
    v.sort();
    v
}

#[test]
fn multistep_parse_orders_and_doctype() {
    let cfg = fomod::parse(&sample("multistep.xml")).unwrap();
    // The DOCTYPE's entity is never expanded.
    assert_eq!(cfg.module_name, "Better Armor Pack &xxe;");
    // Groups without an order are sorted by name (CBBE before UNP).
    assert_eq!(cfg.steps[0].groups[0].plugins[0].name, "CBBE");
    assert_eq!(cfg.steps.len(), 3);
    assert!(cfg.steps[1].visible.is_some());
    assert_eq!(cfg.conditional.len(), 4);
}

#[test]
fn multistep_cbbe_path_with_flags_and_conditional_installs() {
    let env = Env::with_plugins(&[("Unofficial Skyrim Special Edition Patch.esp".into(), true), ("CBPC.esp".into(), false)]);
    let mut s = bap(env);
    let st = s.current().unwrap();
    assert_eq!((st.name.as_str(), st.number), ("Body", 1));
    // Pick one with nothing recommended: the first is chosen.
    assert!(st.groups[0].options[0].selected);
    assert_eq!(st.groups[0].options[0].name, "CBBE");
    assert!(!st.is_last);

    assert!(s.next().unwrap());
    let st = s.current().unwrap();
    assert_eq!((st.name.as_str(), st.number), ("CBBE extras", 2));
    let heels = &st.groups[0].options[0];
    assert!(heels.selected && heels.can_change && heels.kind == PluginType::Recommended);
    // Physics needs CBPC or FSMP turned on: CBPC is only installed.
    let physics = &st.groups[0].options[1];
    assert_eq!(physics.kind, PluginType::NotUsable);
    assert!(!physics.can_change && !physics.selected);
    assert!(s.select(0, 1).is_err());

    assert!(s.next().unwrap());
    let st = s.current().unwrap();
    assert_eq!(st.name, "Patches");
    assert!(st.is_last && st.can_go_back);
    let patches = &st.groups[0];
    // Sorted by name: "Leveled lists" then "Unofficial Patch".
    assert_eq!(patches.options[0].name, "Leveled lists");
    assert_eq!(patches.options[0].kind, PluginType::Required);
    assert!(patches.options[0].selected && !patches.options[0].can_change);
    // Recommended because the unofficial patch is on (file names matched ignoring case).
    assert_eq!(patches.options[1].kind, PluginType::Recommended);
    assert!(patches.options[1].selected);
    assert!(s.deselect(0, 0).is_err(), "required");
    // At most one: none chosen is fine.
    assert!(!st.groups[1].options[0].selected);

    let done = s.finish().unwrap();
    assert_eq!(
        sorted(done.files.clone()),
        sorted(vec![
            ("00 Core/BAP.esp".into(), "BAP.esp".into()),
            ("00 Core/meshes/armor/bap/shared.nif".into(), "meshes/armor/bap/shared.nif".into()),
            ("10 Heels/sound/heels.wav".into(), "sound/heels.wav".into()),
            // alwaysInstall from an option that wasn't chosen, into a folder destination.
            ("11 Physics/readme physics.txt".into(), "docs/readme physics.txt".into()),
            ("20 Patches/BAP - Lists.esp".into(), "BAP - Lists.esp".into()),
            ("20 Patches/BAP - USSEP.esp".into(), "BAP - USSEP.esp".into()),
            ("30 Bodies/CBBE/cuirass.nif".into(), "meshes/armor/bap/cuirass.nif".into()),
            ("30 Bodies/CBBE/meshes_extra/boots.nif".into(), "meshes/armor/bap/meshes_extra/boots.nif".into()),
            // tex flag never set: the empty-value condition matches.
            ("40 Textures/4K/armor.dds".into(), "textures/bap/armor.dds".into()),
        ])
    );
    assert!(done.flags.contains(&("body".into(), "CBBE".into())) && done.flags.contains(&("heels".into(), "On".into())));

    // Choosing 2K: the conditional install for 2K wins the same destination; 4K's condition no longer holds.
    s.select(1, 0).unwrap();
    let done = s.finish().unwrap();
    assert!(done.files.contains(&("40 Textures/2K/armor.dds".into(), "textures/bap/armor.dds".into())));
    assert!(!done.files.iter().any(|(f, _)| f.contains("4K")));
    // Choices can be replayed later.
    let choices = s.choices();
    assert!(choices.contains(&"patches/texture size/2k".to_string()), "{choices:?}");
}

#[test]
fn multistep_unp_skips_the_hidden_step_and_back_works() {
    let mut s = bap(Env::default());
    s.select(0, 1).unwrap(); // UNP
    assert!(s.next().unwrap());
    // "CBBE extras" is hidden for UNP.
    let st = s.current().unwrap();
    assert_eq!((st.name.as_str(), st.number), ("Patches", 2));
    // The unofficial patch is missing: its patch can't be used. installIfUsable files of the required option go in.
    assert_eq!(st.groups[0].options[1].kind, PluginType::NotUsable);
    let done = s.finish().unwrap();
    let dests: Vec<&str> = done.files.iter().map(|(_, d)| d.as_str()).collect();
    assert!(dests.contains(&"meshes/armor/bap/cuirass.nif"));
    assert!(done.files.contains(&("30 Bodies/UNP/cuirass.nif".into(), "meshes/armor/bap/cuirass.nif".into())));
    assert!(!dests.contains(&"BAP - USSEP.esp"));
    assert!(!dests.contains(&"sound/heels.wav"), "the hidden step's options don't count");
    // No 4K texture: that pattern needs CBBE.
    assert!(!dests.contains(&"textures/bap/armor.dds"));

    // Back to the first step, choices kept; switch to CBBE and the extras step shows again.
    assert!(s.back());
    assert!(!s.back());
    let st = s.current().unwrap();
    assert!(st.groups[0].options[1].selected);
    s.select(0, 0).unwrap();
    s.next().unwrap();
    assert_eq!(s.current().unwrap().name, "CBBE extras");
}

#[test]
fn earlier_choices_are_the_starting_point() {
    let mut first = bap(Env::default());
    first.select(0, 1).unwrap();
    first.next().unwrap();
    let choices = first.choices();
    let mut again = bap(Env::default());
    again.apply_choices(&choices);
    assert!(again.current().unwrap().groups[0].options[1].selected, "UNP again");
}

#[test]
fn files_that_are_not_in_the_archive_are_reported() {
    let cfg = fomod::parse(&sample("skyui_like.xml")).unwrap();
    let s = Session::new(cfg, Env::default(), &list(&["fomod/ModuleConfig.xml", "Interface/x.swf"]), "");
    let done = s.finish().unwrap();
    assert_eq!(done.files, vec![("Interface/x.swf".to_string(), "Interface/x.swf".to_string())]);
    assert_eq!(done.missing, vec!["SkyUI_SE.bsa".to_string(), "Plugins\\SkyUI_SE.esp".to_string()]);
}

#[test]
fn utf16_installer_and_info() {
    let bytes = sample("utf16_le_bom.xml");
    assert_eq!(&bytes[..2], &[0xFF, 0xFE]);
    let cfg = fomod::parse(&bytes).unwrap();
    assert_eq!(cfg.module_name, "Rüstung der Nord \u{b7} Édition");
    let g = &cfg.steps[0].groups[0];
    assert_eq!((g.name.as_str(), g.kind), ("Größe", GroupKind::AtLeastOne));
    // Sorted by name: Groß before Klein.
    assert_eq!(g.plugins[0].name, "Groß");
    let files = list(&["fomod/ModuleConfig.xml", "Klein/armor.dds", "Gross/armor.dds"]);
    let mut s = Session::new(cfg, Env::default(), &files, "");
    // At least one: the first is picked, and the last one chosen can't be unpicked.
    assert!(s.current().unwrap().groups[0].options[0].selected);
    assert!(s.deselect(0, 0).is_err());
    s.select(0, 1).unwrap();
    s.deselect(0, 0).unwrap();
    let done = s.finish().unwrap();
    assert_eq!(done.files, vec![("Klein/armor.dds".to_string(), "textures/armor.dds".to_string())]);

    let info = fomod::parse_info(&sample("info_utf16.xml"));
    assert_eq!(info.name.as_deref(), Some("Rüstung der Nord"));
    assert_eq!(info.version.as_deref(), Some("1.2"));
    assert_eq!(info.description.as_deref(), Some("Text & mehr"));
    assert_eq!(fomod::parse_info(b"not xml"), fomod::Info::default());
}

#[test]
fn unsafe_paths_and_odd_files() {
    let xml = r#"<config><requiredInstallFiles><file source="..\..\evil.dll" destination="x.dll"/></requiredInstallFiles></config>"#;
    let s = Session::new(fomod::parse(xml.as_bytes()).unwrap(), Env::default(), &list(&["a.esp"]), "");
    assert!(s.finish().unwrap_err().contains("outside"));
    let xml = r#"<config><requiredInstallFiles><file source="a.esp" destination="/etc/x"/></requiredInstallFiles></config>"#;
    let s = Session::new(fomod::parse(xml.as_bytes()).unwrap(), Env::default(), &list(&["a.esp"]), "");
    assert!(s.finish().is_err());
    // No steps at all: nothing to choose, finish right away; destination missing means "same as source".
    let xml = r#"<config><moduleName>Fixed</moduleName><requiredInstallFiles><file source="Sub\a.esp"/></requiredInstallFiles></config>"#;
    let s = Session::new(fomod::parse(xml.as_bytes()).unwrap(), Env::default(), &list(&["sub/A.esp"]), "");
    assert!(!s.has_choices() && s.current().is_none());
    assert_eq!(s.finish().unwrap().files, vec![("sub/A.esp".to_string(), "Sub/a.esp".to_string())]);
    assert!(fomod::parse(b"<notconfig/>").unwrap_err().contains("isn't a FOMOD"));
    assert!(fomod::parse(&utf16le("<config><moduleName>x</moduleName></config>")).is_ok());
}

#[test]
fn priority_decides_between_files_at_the_same_place() {
    let xml = r#"<config><requiredInstallFiles>
        <file source="b.esp" destination="out.esp" priority="5"/>
        <file source="a.esp" destination="out.esp" priority="1"/>
        <file source="c.esp" destination="OUT.esp" priority="5"/>
    </requiredInstallFiles></config>"#;
    let s = Session::new(fomod::parse(xml.as_bytes()).unwrap(), Env::default(), &list(&["a.esp", "b.esp", "c.esp"]), "");
    // Highest priority wins; at the same priority the later one does.
    assert_eq!(s.finish().unwrap().files, vec![("c.esp".to_string(), "OUT.esp".to_string())]);
}

#[test]
fn select_all_and_at_most_one_groups() {
    let xml = r#"<config><installSteps><installStep name="S"><optionalFileGroups>
        <group name="All" type="SelectAll"><plugins><plugin name="a"><description/><typeDescriptor><type name="Optional"/></typeDescriptor></plugin><plugin name="b"><description/><typeDescriptor><type name="Optional"/></typeDescriptor></plugin></plugins></group>
        <group name="One or none" type="SelectAtMostOne"><plugins><plugin name="x"><description/><typeDescriptor><type name="Recommended"/></typeDescriptor></plugin><plugin name="y"><description/><typeDescriptor><type name="Recommended"/></typeDescriptor></plugin></plugins></group>
    </optionalFileGroups></installStep></installSteps></config>"#;
    let mut s = Session::new(fomod::parse(xml.as_bytes()).unwrap(), Env::default(), &[], "");
    let st = s.current().unwrap();
    assert!(st.groups[0].options.iter().all(|o| o.selected && !o.can_change));
    // Two recommended in a one-or-none group: only the first.
    assert!(st.groups[1].options[0].selected && !st.groups[1].options[1].selected);
    assert!(s.deselect(0, 0).is_err());
    s.toggle(1, 0).unwrap();
    assert!(!s.current().unwrap().groups[1].options[0].selected);
    let _ = FileState::Active;
}

/// A folder whose name changes length when lowercased (a letter outside English) must not stop the installer,
/// and the files below it keep their full names.
#[test]
fn a_folder_name_that_changes_length_in_lowercase() {
    let xml = "<config><requiredInstallFiles><folder source=\"\u{130}\" destination=\"out\"/></requiredInstallFiles></config>";
    let s = Session::new(fomod::parse(xml.as_bytes()).unwrap(), Env::default(), &list(&["\u{130}/\u{e9}.esp", "\u{130}/sub/b.esp"]), "");
    assert_eq!(sorted(s.finish().unwrap().files), sorted(vec![
        ("\u{130}/\u{e9}.esp".to_string(), "out/\u{e9}.esp".to_string()),
        ("\u{130}/sub/b.esp".to_string(), "out/sub/b.esp".to_string()),
    ]));
}
