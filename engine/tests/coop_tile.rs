//! The optional co-op tile in Steam's library. shortcuts.vdf is written back byte for byte apart from Turbine's
//! own tile, with a backup first, only while Steam is closed, and never with a second copy of the tile.

mod support;

use std::fs;
use support::scratch as tmp;
use turbine_engine::shortcuts::{self, TileChange};
use turbine_engine::vdf;

fn s(out: &mut Vec<u8>, k: &str, v: &str) {
    out.push(1);
    out.extend_from_slice(k.as_bytes());
    out.push(0);
    out.extend_from_slice(v.as_bytes());
    out.push(0);
}

fn i(out: &mut Vec<u8>, k: &str, v: u32) {
    out.push(2);
    out.extend_from_slice(k.as_bytes());
    out.push(0);
    out.extend_from_slice(&v.to_le_bytes());
}

fn open(out: &mut Vec<u8>, k: &str) {
    out.push(0);
    out.extend_from_slice(k.as_bytes());
    out.push(0);
}

/// A shortcuts.vdf as Steam writes it: two games, one with tags and a play time, the other an older entry without
/// an appid.
fn steam_file() -> Vec<u8> {
    let mut b = Vec::new();
    open(&mut b, "shortcuts");
    open(&mut b, "0");
    i(&mut b, "appid", 0xC0FF_EE01);
    s(&mut b, "AppName", "Google Chrome");
    s(&mut b, "Exe", "\"/usr/bin/flatpak\"");
    s(&mut b, "StartDir", "\"/usr/bin/\"");
    s(&mut b, "icon", "");
    s(&mut b, "LaunchOptions", "run --branch=stable --arch=x86_64 --command=/app/bin/chrome com.google.Chrome");
    i(&mut b, "IsHidden", 0);
    i(&mut b, "LastPlayTime", 1_762_000_000);
    open(&mut b, "tags");
    s(&mut b, "0", "favorite");
    b.push(8);
    b.push(8);
    open(&mut b, "1");
    s(&mut b, "AppName", "Game");
    s(&mut b, "Exe", "\"C:\\Games\\game.exe\"");
    open(&mut b, "tags");
    b.push(8);
    b.push(8);
    b.push(8);
    b.push(8);
    b
}

#[test]
fn binary_vdf_round_trip() {
    let b = steam_file();
    let doc = vdf::parse_binary_doc(&b).unwrap();
    assert_eq!(vdf::write_binary_doc(&doc), b);
    // Without the last end byte, and with bytes after it: still kept as they were.
    let short = &b[..b.len() - 1];
    assert_eq!(vdf::write_binary_doc(&vdf::parse_binary_doc(short).unwrap()), short);
    let mut extra = b.clone();
    extra.extend_from_slice(b"\x08junk");
    assert_eq!(vdf::write_binary_doc(&vdf::parse_binary_doc(&extra).unwrap()), extra);
    // Broken files are refused, never a panic.
    for n in 1..b.len() - 2 {
        let _ = vdf::parse_binary_doc(&b[..n]);
    }
    assert!(vdf::parse_binary_doc(b"\x00shortcuts\x00\x09x\x00").is_err());
}

#[test]
fn appid_like_steam() {
    assert_eq!(turbine_engine::zip::crc32(b"123456789"), 0xCBF4_3926, "the standard CRC-32 check value");
    // Worked out separately (Python's zlib.crc32 of exe + name, top bit set).
    assert_eq!(shortcuts::legacy_appid("\"C:\\Games\\game.exe\"", "Game"), 3_806_792_068);
    let tile = shortcuts::coop_tile("ELDEN RING", "steam:1245620", "io.github.ravenor907.TurbineModManager");
    assert_eq!(tile.name, "ELDEN RING Co-op");
    assert_eq!(tile.appid(), 3_871_208_806);
    assert_eq!(shortcuts::rungameid_url(tile.appid()), format!("steam://rungameid/{}", (3_871_208_806u64 << 32) | 0x0200_0000));
    assert!(tile.launch_options.ends_with("io.github.ravenor907.TurbineModManager --play-coop steam:1245620 --from-tile"));
    // Turbine's own reading of those arguments finds the game.
    let args: Vec<String> = tile.launch_options.split(' ').skip(4).map(str::to_string).collect();
    assert_eq!(shortcuts::play_coop_arg(&args).as_deref(), Some("steam:1245620"));
    // The old reader gives the second game (no appid stored) the same rule.
    let list = shortcuts::parse(&steam_file()).unwrap();
    assert_eq!(list[1].appid, 3_806_792_068);
}

#[test]
fn add_and_remove_the_tile() {
    let d = tmp("add");
    let file = d.join("userdata/123/config/shortcuts.vdf");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    let orig = steam_file();
    fs::write(&file, &orig).unwrap();
    let tile = shortcuts::coop_tile("ELDEN RING", "steam:1245620", "io.github.ravenor907.TurbineModManager");
    let marker = shortcuts::coop_marker("steam:1245620");

    // Steam running: nothing written.
    assert!(shortcuts::add_shortcut(&file, &tile, &marker, true).unwrap_err().contains("Close Steam first"));
    assert_eq!(fs::read(&file).unwrap(), orig);
    assert!(!shortcuts::backup_file(&file).exists());

    assert_eq!(shortcuts::add_shortcut(&file, &tile, &marker, false).unwrap(), TileChange::Added(tile.appid()));
    assert_eq!(fs::read(shortcuts::backup_file(&file)).unwrap(), orig, "the backup is the file as it was");
    assert_eq!(fs::read(shortcuts::original_file(&file)).unwrap(), orig);
    let after = fs::read(&file).unwrap();
    assert!(after.starts_with(&orig[..orig.len() - 2]), "the other games are untouched, byte for byte");
    let list = shortcuts::parse(&after).unwrap();
    assert_eq!(list.len(), 3);
    assert_eq!(list[2].appid, tile.appid());
    assert_eq!(list[2].name, "ELDEN RING Co-op");
    assert_eq!(list[2].exe, "\"/usr/bin/flatpak\"");
    assert!(shortcuts::is_turbine(&list[2]));
    assert_eq!(list[0].appid, 0xC0FF_EE01);
    let doc = vdf::parse_binary(&after).unwrap();
    assert_eq!(doc.str_at(&["shortcuts", "2", "AllowOverlay"]), Some("1"));
    assert_eq!(doc.str_at(&["shortcuts", "2", "FlatpakAppID"]), Some("io.github.ravenor907.TurbineModManager"));

    // No duplicates, even with Steam running (nothing to write).
    assert_eq!(shortcuts::add_shortcut(&file, &tile, &marker, false).unwrap(), TileChange::Already(tile.appid()));
    assert_eq!(shortcuts::add_shortcut(&file, &tile, &marker, true).unwrap(), TileChange::Already(tile.appid()));
    assert_eq!(fs::read(&file).unwrap(), after);
    assert_eq!(shortcuts::parse(&fs::read(&file).unwrap()).unwrap().len(), 3);

    // Remove: only the tile goes, and the file is exactly as Steam had it.
    assert!(shortcuts::remove_shortcut(&file, &marker, true).is_err());
    assert_eq!(shortcuts::remove_shortcut(&file, &marker, false).unwrap(), TileChange::Removed(tile.appid()));
    assert_eq!(fs::read(&file).unwrap(), orig);
    assert_eq!(fs::read(shortcuts::backup_file(&file)).unwrap(), after);
    assert_eq!(fs::read(shortcuts::original_file(&file)).unwrap(), orig, "the first original is kept");
    assert_eq!(shortcuts::remove_shortcut(&file, &marker, false).unwrap(), TileChange::NotThere);
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn tile_in_a_new_file_and_renumbering() {
    let d = tmp("new");
    let cfg = d.join(".local/share/Steam/userdata/456/config");
    fs::create_dir_all(&cfg).unwrap();
    let file = shortcuts::tile_file(&d).unwrap();
    assert_eq!(file, cfg.join("shortcuts.vdf"));
    let a = shortcuts::coop_tile("ELDEN RING", "steam:1245620", "io.github.ravenor907.TurbineModManager");
    let b = shortcuts::coop_tile("ELDEN RING NIGHTREIGN", "steam:2622380", "io.github.ravenor907.TurbineModManager");
    assert_eq!(shortcuts::add_shortcut(&file, &a, &shortcuts::coop_marker("steam:1245620"), false).unwrap(), TileChange::Added(a.appid()));
    assert!(!shortcuts::backup_file(&file).exists(), "nothing to back up");
    assert_eq!(shortcuts::add_shortcut(&file, &b, &shortcuts::coop_marker("steam:2622380"), false).unwrap(), TileChange::Added(b.appid()));
    assert_eq!(shortcuts::remove_shortcut(&file, &shortcuts::coop_marker("steam:1245620"), false).unwrap(), TileChange::Removed(a.appid()));
    let doc = vdf::parse_binary(&fs::read(&file).unwrap()).unwrap();
    assert_eq!(doc.str_at(&["shortcuts", "0", "AppName"]), Some("ELDEN RING NIGHTREIGN Co-op"), "renumbered from 0");
    // A file that isn't a shortcuts list is refused, never replaced.
    fs::write(&file, b"\x00other\x00\x08\x08").unwrap();
    assert!(shortcuts::add_shortcut(&file, &a, &shortcuts::coop_marker("steam:1245620"), false).is_err());
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn tile_with_its_art_and_icon() {
    let d = tmp("art");
    let art = d.join("coop");
    fs::create_dir_all(&art).unwrap();
    for p in ["capsule", "wide", "hero", "logo", "icon"] {
        fs::write(art.join(format!("{p}.png")), p).unwrap();
    }
    let file = d.join("userdata/9/config/shortcuts.vdf");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, steam_file()).unwrap();
    let record = d.join("data/steam-art.json");
    let tile = shortcuts::coop_tile("ELDEN RING", "steam:1245620", "io.github.ravenor907.TurbineModManager");
    let marker = shortcuts::coop_marker("steam:1245620");
    // Steam running: neither the tile nor its art.
    assert!(shortcuts::add_tile_with_art(&file, &tile, &marker, &art, &record, true).is_err());
    assert!(!d.join("userdata/9/config/grid").exists());
    assert_eq!(shortcuts::add_tile_with_art(&file, &tile, &marker, &art, &record, false).unwrap(), TileChange::Added(3_871_208_806));
    let grid = d.join("userdata/9/config/grid");
    for n in ["3871208806p.png", "3871208806.png", "3871208806_hero.png", "3871208806_logo.png", "3871208806_icon.png"] {
        assert!(grid.join(n).is_file(), "{n}");
    }
    let doc = vdf::parse_binary(&fs::read(&file).unwrap()).unwrap();
    assert_eq!(doc.str_at(&["shortcuts", "2", "icon"]), Some(grid.join("3871208806_icon.png").to_str().unwrap()));
    // Turbine's own shortcut is never taken for the tile, and the tile never for Turbine.
    assert!(shortcuts::find_turbine_file(&d).is_none());
    // Remove: the tile and Turbine's art go; the player's own picture for it stays.
    fs::write(grid.join("3871208806_hero.png"), "the player's own").unwrap();
    assert_eq!(shortcuts::remove_tile_with_art(&file, &marker, &record, false).unwrap(), TileChange::Removed(3_871_208_806));
    assert!(!grid.join("3871208806p.png").exists());
    assert!(!grid.join("3871208806_icon.png").exists());
    assert_eq!(fs::read_to_string(grid.join("3871208806_hero.png")).unwrap(), "the player's own");
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn play_coop_argument() {
    let a = |v: &[&str]| shortcuts::play_coop_arg(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    assert_eq!(a(&["turbine", "--play-coop", "steam:1245620"]), Some("steam:1245620".into()));
    assert_eq!(a(&["turbine"]), None);
    assert_eq!(a(&["turbine", "--play-coop"]), None);
    assert_eq!(a(&["turbine", "--play-coop", "epic:x"]), None);
    assert_eq!(a(&["turbine", "--play-coop", "steam:12;rm"]), None);
}

#[test]
fn the_signed_in_account_comes_first() {
    use std::fs;
    let home = std::env::temp_dir().join(format!("turbine-accounts-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    let root = home.join(".local/share/Steam");
    for acc in ["111", "222"] {
        let cfg = root.join("userdata").join(acc).join("config");
        fs::create_dir_all(&cfg).unwrap();
        fs::write(cfg.join("shortcuts.vdf"), b"\x00shortcuts\x00\x08\x08").unwrap();
    }
    // 222 changed last, but 111 is the account signed in (steamid64 = 76561197960265728 + 111).
    std::thread::sleep(std::time::Duration::from_millis(20));
    fs::write(root.join("userdata/222/config/shortcuts.vdf"), b"\x00shortcuts\x00\x08\x08").unwrap();
    fs::create_dir_all(root.join("config")).unwrap();
    fs::write(
        root.join("config/loginusers.vdf"),
        "\"users\"\n{\n\t\"76561197960265839\"\n\t{\n\t\t\"AccountName\"\t\t\"a\"\n\t\t\"MostRecent\"\t\t\"1\"\n\t}\n\t\"76561197960265950\"\n\t{\n\t\t\"AccountName\"\t\t\"b\"\n\t\t\"MostRecent\"\t\t\"0\"\n\t}\n}\n",
    )
    .unwrap();
    assert_eq!(turbine_engine::steam::signed_in_account(&home).as_deref(), Some("111"));
    let files = turbine_engine::shortcuts::shortcut_files(&home);
    assert!(files[0].to_string_lossy().contains("/userdata/111/"), "{files:?}");
    assert!(turbine_engine::shortcuts::tile_file(&home).unwrap().to_string_lossy().contains("/userdata/111/"));
    // The signed-in account has no non-Steam games yet: its file is where the tile goes, not 222's.
    fs::remove_file(root.join("userdata/111/config/shortcuts.vdf")).unwrap();
    let t = turbine_engine::shortcuts::tile_file(&home).unwrap();
    assert!(t.to_string_lossy().contains("/userdata/111/config/shortcuts.vdf"), "{t:?}");
    // localconfig.vdf: 222 changed last, but 111's own file is the one picked.
    fs::create_dir_all(root.join("steamapps")).unwrap();
    for acc in ["111", "222"] {
        fs::write(root.join("userdata").join(acc).join("config/localconfig.vdf"), "\"UserLocalConfigStore\"\n{\n}\n").unwrap();
    }
    let files = turbine_engine::steam_setup::localconfig_files(&home);
    let f = turbine_engine::steam::signed_in_file(&home, &files).unwrap();
    assert!(f.to_string_lossy().contains("/userdata/111/"), "{f:?}");
    let _ = fs::remove_dir_all(&home);
}
