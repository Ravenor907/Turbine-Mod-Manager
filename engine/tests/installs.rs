//! Adding mods through the store: FOMOD installers, 7z and rar, SMAPI's installer, the SKSE launcher swap, BepInEx
//! start scripts, sources and updates.

mod support;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use support::{game, utf16le, write_zip, zip_bytes, FakeTar, TempDir, SEVEN_Z};
use turbine_engine::games::Store as Launcher;
use turbine_engine::rules::GameRules;
use turbine_engine::store::{Import, SourceInfo, Store};
use turbine_engine::Runtime;

fn skyui_xml() -> Vec<u8> {
    fs::read(format!("{}/tests/data/fomod/skyui_like.xml", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

fn zip_src() -> SourceInfo {
    SourceInfo { kind: "zip".into(), ..SourceInfo::default() }
}

struct Env {
    t: TempDir,
}

impl Env {
    fn new(tag: &str) -> Env {
        Env { t: TempDir::new(tag) }
    }
    fn skyrim(&self) -> turbine_engine::Game {
        let dir = self.t.path("lib/Skyrim");
        fs::create_dir_all(dir.join("Data")).unwrap();
        let mut g = game(Launcher::Steam, "489830", "Skyrim Special Edition", &dir, None);
        g.runtime = Runtime::Windows;
        g
    }
    fn data(&self) -> std::path::PathBuf {
        self.t.path("data")
    }
}

fn skyui_zip(path: &std::path::Path, esp: &[u8]) {
    let xml = skyui_xml();
    write_zip(
        path,
        &[
            ("SkyUI_5_2_SE/fomod/ModuleConfig.xml", &xml),
            ("SkyUI_5_2_SE/fomod/info.xml", b"<fomod><Name>SkyUI</Name><Version>5.2SE</Version></fomod>"),
            ("SkyUI_5_2_SE/fomod/images/full.png", b"png"),
            ("SkyUI_5_2_SE/Interface/skyui/a.swf", b"swf"),
            ("SkyUI_5_2_SE/SkyUI_SE.bsa", b"bsa"),
            ("SkyUI_5_2_SE/Plugins/SkyUI_SE.esp", esp),
            ("SkyUI_5_2_SE/Plugins/SkyUI_SE_NoMCM.esp", b"nomcm"),
        ],
    );
}

#[test]
fn fomod_install_with_choices_then_update_keeps_them() {
    let e = Env::new("fomod-store");
    let g = e.skyrim();
    let mut s = Store::open(&g, &e.data()).unwrap();
    let a = e.t.path("Downloads/SkyUI_5_2_SE-12604-5-2SE-1573517013.zip");
    skyui_zip(&a, b"full");

    let Import::NeedsChoices(mut p) = s.import_archive(&a, zip_src()).unwrap() else { panic!("expected choices") };
    assert_eq!(p.name, "SkyUI");
    assert_eq!(p.info.version.as_deref(), Some("5.2SE"));
    // The option's image is a real file the app can show.
    let json = p.to_json();
    assert!(json.contains("\"image\":\"SkyUI_5_2_SE/fomod/images/full.png\""), "{json}");
    assert!(p.image_file("SkyUI_5_2_SE/fomod/images/full.png").is_some());
    assert!(turbine_engine::json::parse(&json).is_ok());
    p.session.toggle(0, 1).unwrap();
    let done = s.finish_install(*p).unwrap();
    assert_eq!(done.item.name, "SkyUI");
    let dir = s.root().join("mods").join(&done.item.id);
    assert_eq!(fs::read(dir.join("Data/SkyUI_SE.esp")).unwrap(), b"nomcm");
    assert!(dir.join("Data/Interface/skyui/a.swf").is_file() && dir.join("Data/SkyUI_SE.bsa").is_file());
    assert!(!dir.join("Data/fomod").exists());
    assert_eq!(fs::read_dir(s.root().join("tmp")).unwrap().count(), 0, "nothing left in tmp");

    // Another mod, then the update: same place, same on/off state, last choices preselected.
    let other = e.t.path("Downloads/Other.zip");
    write_zip(&other, &[("Other.esp", b"o")]);
    s.import_zip(&other).unwrap();
    s.set_enabled(&done.item.id, false).unwrap();
    let v2 = e.t.path("Downloads/SkyUI_5_3.zip");
    skyui_zip(&v2, b"full2");
    let src = SourceInfo { kind: "nexus".into(), id: "12604".into(), version: Some("5.3".into()), ..SourceInfo::default() };
    let Import::NeedsChoices(p) = s.replace_mod(&done.item.id, &v2, src.clone()).unwrap() else { panic!() };
    let st = p.session.current().unwrap();
    assert!(st.groups[0].options[1].selected, "last time's choice");
    let up = s.finish_install(*p).unwrap();
    assert_eq!(up.item.id, done.item.id);
    assert_eq!(s.mods()[0].id, done.item.id);
    assert!(!s.mods()[0].enabled);
    assert_eq!(s.mods()[0].version.as_deref(), Some("5.3"));
    assert_eq!(s.mods()[0].source, "Nexus Mods");
    assert_eq!(fs::read(s.root().join("mods").join(&up.item.id).join("Data/SkyUI_SE.esp")).unwrap(), b"nomcm");

    // Sources survive a reopen; the update checker sees them.
    let s = Store::open(&g, &e.data()).unwrap();
    let sources = s.installed_sources();
    assert_eq!(sources[0], (done.item.id.clone(), src));
    assert_eq!(sources[1].1.kind, "zip");
    assert!(s.to_json().contains("\"sourceKind\":\"nexus\""));
}

#[test]
fn import_zip_uses_the_installer_defaults_and_cancel_cleans_up() {
    let e = Env::new("fomod-default");
    let g = e.skyrim();
    let mut s = Store::open(&g, &e.data()).unwrap();
    let a = e.t.path("SkyUI.zip");
    skyui_zip(&a, b"full");
    let m = s.import_zip(&a).unwrap();
    assert_eq!(fs::read(s.root().join("mods").join(&m.id).join("Data/SkyUI_SE.esp")).unwrap(), b"full");

    let b = e.t.path("SkyUI copy.zip");
    skyui_zip(&b, b"full");
    let Import::NeedsChoices(p) = s.import_archive(&b, zip_src()).unwrap() else { panic!() };
    assert_eq!(fs::read_dir(s.root().join("tmp")).unwrap().count(), 1);
    s.cancel_install(*p);
    assert_eq!(fs::read_dir(s.root().join("tmp")).unwrap().count(), 0);
    assert_eq!(s.mods().len(), 1);
}

#[test]
fn utf16_installer_inside_an_archive() {
    let e = Env::new("fomod-utf16");
    let g = e.skyrim();
    let mut s = Store::open(&g, &e.data()).unwrap();
    let xml = fs::read(format!("{}/tests/data/fomod/utf16_le_bom.xml", env!("CARGO_MANIFEST_DIR"))).unwrap();
    let a = e.t.path("Nord.zip");
    write_zip(&a, &[("fomod/ModuleConfig.xml", &xml), ("Klein/a.dds", b"k"), ("Gross/a.dds", b"g")]);
    let Import::NeedsChoices(p) = s.import_archive(&a, zip_src()).unwrap() else { panic!() };
    assert_eq!(p.name, "Rüstung der Nord \u{b7} Édition");
    let done = s.finish_install(*p).unwrap();
    assert_eq!(fs::read(s.root().join("mods").join(&done.item.id).join("Data/textures/a.dds")).unwrap(), b"g");
}

#[test]
fn installer_without_steps_installs_right_away() {
    let e = Env::new("fomod-nosteps");
    let g = e.skyrim();
    let mut s = Store::open(&g, &e.data()).unwrap();
    let xml = utf16le("<config><moduleName>Fixed</moduleName><requiredInstallFiles><file source=\"core\\a.esp\" destination=\"a.esp\"/><file source=\"nothere.esp\"/></requiredInstallFiles></config>");
    let a = e.t.path("Fixed.zip");
    write_zip(&a, &[("fomod/ModuleConfig.xml", &xml), ("core/a.esp", b"a")]);
    let Import::Added(done) = s.import_archive(&a, zip_src()).unwrap() else { panic!() };
    assert_eq!(done.item.name, "Fixed");
    assert!(done.notes.iter().any(|n| n.contains("nothere.esp")), "{:?}", done.notes);
    // A broken installer file is a plain error.
    let b = e.t.path("Broken.zip");
    write_zip(&b, &[("fomod/ModuleConfig.xml", b"<config><installSteps>"), ("a.esp", b"a")]);
    // Cut off at the end: read as far as it goes, which installs nothing.
    let err = s.import_archive(&b, zip_src()).unwrap_err();
    assert!(err.contains("don't install any files"), "{err}");
    let c = e.t.path("Broken2.zip");
    write_zip(&c, &[("fomod/ModuleConfig.xml", b"<config><a></b></config>"), ("a.esp", b"a")]);
    let err = s.import_archive(&c, zip_src()).unwrap_err();
    assert!(err.starts_with("Broken2.zip couldn't be added: its installer file can't be read"), "{err}");
}

#[test]
fn seven_zip_and_rar_go_through_the_reader() {
    let e = Env::new("7z-store");
    let g = e.skyrim();
    let mut s = Store::open(&g, &e.data()).unwrap();
    s.set_tar(Box::new(FakeTar::of(&[("Mod 1.2/Data/Cool.esp", b"c"), ("Mod 1.2/readme.txt", b"r")])));
    let a = e.t.write("Downloads/Cool Mod-1.2.7z", SEVEN_Z);
    let Import::Added(done) = s.import_archive(&a, zip_src()).unwrap() else { panic!() };
    assert_eq!(done.item.source, "7z file");
    assert_eq!(done.item.version.as_deref(), Some("1.2"));
    assert!(s.root().join("mods").join(&done.item.id).join("Data/Cool.esp").is_file());
    // A listing with a link stops the import with words.
    let mut bad = FakeTar::of(&[("a.esp", b"a")]);
    bad.listing.push_str("lrwxrwxrwx  0 0 0 0 Jan  1  2024 evil -> /home\n");
    s.set_tar(Box::new(bad));
    let b = e.t.write("Downloads/Evil.rar", support::RAR);
    let err = s.import_archive(&b, zip_src()).unwrap_err();
    assert!(err.starts_with("Evil.rar couldn't be added: the archive has a link"), "{err}");
    // Not an archive at all.
    let c = e.t.write("Downloads/x.zip", b"hello");
    assert!(s.import_archive(&c, zip_src()).unwrap_err().contains("isn't a zip, 7z or rar"));
    // Downloads lists every kind.
    let names: Vec<String> = turbine_engine::store::downloads(&e.t.0).iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
    assert_eq!(names.len(), 3);
}

#[test]
fn skse_goes_in_as_the_launcher_and_comes_out_again() {
    let e = Env::new("skse");
    let g = e.skyrim();
    fs::write(g.install_dir.join("SkyrimSELauncher.exe"), b"vanilla launcher").unwrap();
    let mut s = Store::open(&g, &e.data()).unwrap();
    let a = e.t.path("skse64_2_02_06.7z.zip");
    write_zip(
        &a,
        &[
            ("skse64_2_02_06/skse64_loader.exe", b"skse loader"),
            ("skse64_2_02_06/skse64_1_6_1170.dll", b"dll"),
            ("skse64_2_02_06/Data/Scripts/Actor.pex", b"pex"),
        ],
    );
    let m = s.import_zip(&a).unwrap();
    let dir = s.root().join("mods").join(&m.id);
    assert_eq!(fs::read(dir.join("SkyrimSELauncher.exe")).unwrap(), b"skse loader");
    assert_eq!(fs::read(dir.join("skse64_loader.exe")).unwrap(), b"skse loader");
    s.apply().unwrap();
    assert_eq!(fs::read(g.install_dir.join("SkyrimSELauncher.exe")).unwrap(), b"skse loader");
    assert!(GameRules::for_game(&g).loader_installed(&g));
    s.play_original().unwrap();
    assert_eq!(fs::read(g.install_dir.join("SkyrimSELauncher.exe")).unwrap(), b"vanilla launcher");
    assert!(!g.install_dir.join("skse64_loader.exe").exists());
    // Not on GOG: there, nothing replaces a launcher.
    let mut gog = g.clone();
    gog.store = Launcher::Gog;
    gog.store_id = "1711230643".into();
    assert!(GameRules::for_game(&gog).launcher_swap.is_none());
}

fn stardew(e: &Env, runtime: Runtime) -> turbine_engine::Game {
    let dir = e.t.path("lib/Stardew Valley");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("StardewValley"), b"#!/bin/sh\nvanilla").unwrap();
    fs::write(dir.join("Stardew Valley.deps.json"), b"{\"deps\":1}").unwrap();
    fs::write(dir.join("Stardew Valley.dll"), b"game").unwrap();
    let mut g = game(Launcher::Steam, "413150", "Stardew Valley", &dir, None);
    g.runtime = runtime;
    g
}

fn smapi_installer(path: &std::path::Path) {
    let linux = zip_bytes(&[
        ("StardewModdingAPI", b"elf"),
        ("StardewModdingAPI.dll", b"smapi"),
        ("StardewModdingAPI.runtimeconfig.json", b"{}"),
        ("unix-launcher.sh", b"#!/usr/bin/env bash\n./StardewModdingAPI \"$@\"\n"),
        ("smapi-internal/config.json", b"{}"),
        ("smapi-internal/i18n/de.json", b"{}"),
        ("Mods/ConsoleCommands/manifest.json", b"{\"UniqueID\":\"SMAPI.ConsoleCommands\"}"),
        ("Mods/ConsoleCommands/ConsoleCommands.dll", b"cc"),
        ("Mods/SaveBackup/manifest.json", b"{}"),
        ("Mods/SaveBackup/SaveBackup.dll", b"sb"),
    ]);
    let windows = zip_bytes(&[
        ("StardewModdingAPI.exe", b"exe"),
        ("StardewModdingAPI.dll", b"smapi"),
        ("smapi-internal/config.json", b"{}"),
        ("Mods/SaveBackup/manifest.json", b"{}"),
    ]);
    write_zip(
        path,
        &[
            ("SMAPI 4.3.2 installer/install on Linux.sh", b"#!/bin/sh"),
            ("SMAPI 4.3.2 installer/install on Windows.bat", b"@echo"),
            ("SMAPI 4.3.2 installer/README.txt", b"read me"),
            ("SMAPI 4.3.2 installer/internal/linux/SMAPI.Installer.dll", b"installer"),
            ("SMAPI 4.3.2 installer/internal/linux/install.dat", &linux),
            ("SMAPI 4.3.2 installer/internal/windows/install.dat", &windows),
        ],
    );
}

#[test]
fn smapi_is_set_up_from_its_bundle_without_running_anything() {
    let e = Env::new("smapi");
    let g = stardew(&e, Runtime::Native);
    let mut s = Store::open(&g, &e.data()).unwrap();
    let a = e.t.path("SMAPI 4.3.2 installer.zip");
    smapi_installer(&a);
    let Import::Added(done) = s.import_archive(&a, SourceInfo { kind: "loader".into(), id: "smapi".into(), ..SourceInfo::default() }).unwrap() else { panic!() };
    let dir = s.root().join("mods").join(&done.item.id);
    let mut files: Vec<String> = Vec::new();
    fn walk(base: &std::path::Path, d: &std::path::Path, out: &mut Vec<String>) {
        for e in fs::read_dir(d).unwrap().flatten() {
            if e.path().is_dir() {
                walk(base, &e.path(), out);
            } else {
                out.push(e.path().strip_prefix(base).unwrap().to_string_lossy().into_owned());
            }
        }
    }
    walk(&dir, &dir, &mut files);
    files.sort();
    assert_eq!(
        files,
        vec![
            "Mods/ConsoleCommands/ConsoleCommands.dll",
            "Mods/ConsoleCommands/manifest.json",
            "Mods/SaveBackup/SaveBackup.dll",
            "Mods/SaveBackup/manifest.json",
            "StardewModdingAPI",
            "StardewModdingAPI.deps.json",
            "StardewModdingAPI.dll",
            "StardewModdingAPI.runtimeconfig.json",
            "StardewValley",
            "smapi-internal/config.json",
            "smapi-internal/i18n/de.json",
        ]
    );
    assert_eq!(fs::read(dir.join("StardewModdingAPI.deps.json")).unwrap(), b"{\"deps\":1}");
    assert!(fs::read_to_string(dir.join("StardewValley")).unwrap().contains("./StardewModdingAPI"));
    for exe in ["StardewValley", "StardewModdingAPI"] {
        assert_eq!(fs::metadata(dir.join(exe)).unwrap().permissions().mode() & 0o777, 0o755, "{exe}");
    }
    assert_eq!(fs::metadata(dir.join("StardewModdingAPI.dll")).unwrap().permissions().mode() & 0o111, 0);
    assert!(done.notes.iter().any(|n| n.contains("not run")));
    assert_eq!(done.item.source, "Official download");

    // Deployed: the launcher is SMAPI's (still executable), the game's own is kept and comes back.
    s.apply().unwrap();
    let launcher = g.install_dir.join("StardewValley");
    assert!(fs::read_to_string(&launcher).unwrap().contains("./StardewModdingAPI"));
    assert_eq!(fs::metadata(&launcher).unwrap().permissions().mode() & 0o111, 0o111);
    assert!(GameRules::for_game(&g).loader_installed(&g));
    s.play_original().unwrap();
    assert_eq!(fs::read(&launcher).unwrap(), b"#!/bin/sh\nvanilla");
    assert!(!g.install_dir.join("StardewModdingAPI").exists());
}

#[test]
fn smapi_for_the_windows_build_uses_the_windows_bundle() {
    let e = Env::new("smapi-win");
    let g = stardew(&e, Runtime::Windows);
    let mut s = Store::open(&g, &e.data()).unwrap();
    let a = e.t.path("SMAPI 4.3.2 installer.zip");
    smapi_installer(&a);
    let m = s.import_zip(&a).unwrap();
    let dir = s.root().join("mods").join(&m.id);
    assert!(dir.join("StardewModdingAPI.exe").is_file());
    assert!(!dir.join("StardewValley").exists());
    let r = GameRules::for_game(&g);
    assert!(r.launch_option.is_none());
    assert!(r.launch_help.unwrap().contains("smapi.io/install"));
}

#[test]
fn valheim_pack_scripts_are_executable_and_launch_option_is_given() {
    let e = Env::new("valheim");
    let dir = e.t.path("lib/Valheim");
    fs::create_dir_all(&dir).unwrap();
    let g = game(Launcher::Steam, "892970", "Valheim", &dir, None);
    let mut s = Store::open(&g, &e.data()).unwrap();
    let a = e.t.path("denikson-BepInExPack_Valheim-5.4.2202.zip");
    write_zip(
        &a,
        &[
            ("manifest.json", b"{}"),
            ("icon.png", b"png"),
            ("BepInExPack_Valheim/winhttp.dll", b"w"),
            ("BepInExPack_Valheim/doorstop_config.ini", b"[General]"),
            ("BepInExPack_Valheim/start_game_bepinex.sh", b"#!/bin/sh\nexec \"$@\""),
            ("BepInExPack_Valheim/start_server_bepinex.sh", b"#!/bin/sh"),
            ("BepInExPack_Valheim/BepInEx/core/BepInEx.dll", b"core"),
        ],
    );
    let src = SourceInfo { kind: "thunderstore".into(), id: "denikson-BepInExPack_Valheim".into(), version: Some("5.4.2202".into()), ..SourceInfo::default() };
    let Import::Added(done) = s.import_archive(&a, src.clone()).unwrap() else { panic!() };
    assert_eq!(done.item.name, "BepInExPack Valheim");
    let mdir = s.root().join("mods").join(&done.item.id);
    assert_eq!(fs::metadata(mdir.join("start_game_bepinex.sh")).unwrap().permissions().mode() & 0o777, 0o755);
    assert_eq!(fs::metadata(mdir.join("winhttp.dll")).unwrap().permissions().mode() & 0o111, 0);
    let r = GameRules::for_game(&g);
    assert_eq!(r.launch_option.as_deref(), Some("./start_game_bepinex.sh %command%"));
    assert!(r.launch_help.unwrap().contains("Launch Options"));
    // Under Proton the pack loads through winhttp.dll: nothing to type.
    let mut win = g.clone();
    win.runtime = Runtime::Windows;
    assert!(GameRules::for_game(&win).launch_option.is_none());

    // The same Thunderstore package again (another file name) replaces it in place.
    let b = e.t.path("download-1234.zip");
    fs::copy(&a, &b).unwrap();
    let src2 = SourceInfo { version: Some("5.4.2300".into()), ..src };
    let Import::Added(again) = s.import_archive(&b, src2).unwrap() else { panic!() };
    assert_eq!(again.item.id, done.item.id);
    assert_eq!(s.mods().len(), 1);
    assert_eq!(s.installed_sources()[0].1.version.as_deref(), Some("5.4.2300"));
}

#[test]
fn replace_mod_needs_a_known_mod() {
    let e = Env::new("replace-missing");
    let g = e.skyrim();
    let mut s = Store::open(&g, &e.data()).unwrap();
    let a = e.t.path("x.zip");
    write_zip(&a, &[("x.esp", b"x")]);
    assert!(s.replace_mod("nope", &a, zip_src()).is_err());
}
