//! Download names to the clean names players see (the owner's Seamless Co-op among them).
use turbine_engine::store::clean_name;

#[test]
fn download_names_to_clean_names() {
    let table: &[(&str, &str, Option<&str>)] = &[
        ("Seamless Co-op v2.0.2 510 2.0.2 2026-10-27T14-30Z gn8dtBhTy (2).zip", "Seamless Co-op", Some("2.0.2")),
        ("Seamless Co-op v2.0.2 510 2.0.2 2026-10-27T14-30Z gn8dtBhTy.zip", "Seamless Co-op", Some("2.0.2")),
        ("Seamless Co-op-510-2-0-2-1730039400.zip", "Seamless Co-op", Some("2.0.2")),
        ("Seamless Co-op.zip", "Seamless Co-op", None),
        ("SeamlessCoop_2.0.2_trg5df.zip", "SeamlessCoop", Some("2.0.2")),
        ("Better_Torches_1.0_a8Fk2LmQ.zip", "Better Torches", Some("1.0")),
        ("Better Torches_1.0.zip", "Better Torches", Some("1.0")),
        ("SkyUI_5_2_SE-12604-5-2SE-1573517013.zip", "SkyUI 5 2 SE", Some("5.2SE")),
        ("Fast Travel from Anywhere-324-1-0-5-1699999999.zip", "Fast Travel from Anywhere", Some("1.0.5")),
        ("All Quest Objectives On Map-5-1-0-5-1699999999.zip", "All Quest Objectives On Map", Some("1.0.5")),
        ("Author-Cool_Mod-1.2.3.zip", "Author-Cool Mod", Some("1.2.3")),
        ("Brighter Lanterns 1.2.zip", "Brighter Lanterns", Some("1.2")),
        ("Lantern Glow SKSE 2.0.zip", "Lantern Glow SKSE", Some("2.0")),
        ("Quiet Rooster 1.4.zip", "Quiet Rooster", Some("1.4")),
        ("TurbineTest-Bigger_Pockets-1.0.3.zip", "TurbineTest-Bigger Pockets", Some("1.0.3")),
        ("better-torches-main-file.zip", "Better Torches Main File", None),
        ("unofficial skyrim patch.7z", "Unofficial Skyrim Patch", None),
        ("SKSE64 2.2.6.7z", "SKSE64", Some("2.2.6")),
        ("Cyber Engine Tweaks 1.32.0 (3).zip", "Cyber Engine Tweaks", Some("1.32.0")),
        ("Lantern Pack Extra - Copy.zip", "Lantern Pack Extra", None),
        ("RED4ext v1.25.1 2026-07-01T09-00Z k3jD9xQp.zip", "RED4ext", Some("1.25.1")),
        ("Appearance Menu Mod-790-2-6-1-1699999999.zip", "Appearance Menu Mod", Some("2.6.1")),
        ("Odd Layout 1.0.zip", "Odd Layout", Some("1.0")),
        ("Pier Fix 1.0.zip", "Pier Fix", Some("1.0")),
        ("Golden Armor 1.0.zip", "Golden Armor", Some("1.0")),
        ("Wand Glow 1.0.zip", "Wand Glow", Some("1.0")),
        ("Neon Signs REDmod 1.1.zip", "Neon Signs REDmod", Some("1.1")),
        ("Faster Roach 2.0.zip", "Faster Roach", Some("2.0")),
        ("Herb Picking II 1.5.zip", "Herb Picking II", Some("1.5")),
        ("Better Lockpicking 1.0.zip", "Better Lockpicking", Some("1.0")),
        ("Rüstung der Nord v3.zip", "Rüstung der Nord", Some("3")),
        ("Immersive Armors 8.1 2026-06-20 Hq7tY2b.zip", "Immersive Armors", Some("8.1")),
        ("ImmersiveArmors_8.1_Hq7tY2b.7z", "ImmersiveArmors", Some("8.1")),
        ("Co-op Tweaks (12).rar", "Co-op Tweaks", None),
        ("mod.zip", "Mod", None),
    ];
    let mut bad = Vec::new();
    for (file, name, version) in table {
        let got = clean_name(file);
        if got.0 != *name || got.1.as_deref() != *version {
            bad.push(format!("{file}: got {got:?}, want ({name:?}, {version:?})"));
        }
    }
    assert!(table.len() >= 30);
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
