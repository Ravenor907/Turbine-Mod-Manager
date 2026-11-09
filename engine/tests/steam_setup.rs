//! 0.59: Turbine sets the game's Launch Options in Steam's localconfig.vdf itself (only while Steam is closed), so
//! the player never pastes the line. Only that value changes; the rest of the file stays byte for byte.

use std::fs;
use std::path::Path;
use turbine_engine::{anticheat, steam, steam_setup};

const CONFIG: &str = "\"UserLocalConfigStore\"\n{\n\t\"Software\"\n\t{\n\t\t\"Valve\"\n\t\t{\n\t\t\t\"Steam\"\n\t\t\t{\n\t\t\t\t\"apps\"\n\t\t\t\t{\n\t\t\t\t\t\"1245620\"\n\t\t\t\t\t{\n\t\t\t\t\t\t\"LastPlayed\"\t\t\"1762650000\"\n\t\t\t\t\t\t\"LaunchOptions\"\t\t\"PROTON_LOG=1 \\\"x\\\" %command%\"\n\t\t\t\t\t}\n\t\t\t\t\t\"489830\"\n\t\t\t\t\t{\n\t\t\t\t\t\t\"LastPlayed\"\t\t\"1\"\n\t\t\t\t\t}\n\t\t\t\t}\n\t\t\t\t\"ShaderCacheManager\"\n\t\t\t\t{\n\t\t\t\t\t\"HasCurrentBucket\"\t\t\"1\"\n\t\t\t\t}\n\t\t\t}\n\t\t}\n\t}\n\t\"friends\"\n\t{\n\t\t\"PersonaName\"\t\t\"Ravenor907\"\n\t}\n}\n";

fn line() -> String {
    anticheat::launch_option(Path::new("/home/deck/.var/app/io.github.ravenor907.TurbineModManager/data"))
}

#[test]
fn replaces_only_the_value() {
    assert_eq!(steam::launch_options_in(CONFIG, "1245620").unwrap(), "PROTON_LOG=1 \"x\" %command%");
    let out = steam_setup::set_launch_options_in(CONFIG, "1245620", &line()).unwrap();
    assert_eq!(steam::launch_options_in(&out, "1245620").unwrap(), line(), "Turbine's line, quotes and all, reads back");
    let (a, b) = CONFIG.split_once("\"PROTON_LOG").unwrap();
    assert!(out.starts_with(a));
    assert!(out.ends_with(b.split_once("%command%\"").unwrap().1), "the rest of the file is untouched");
    assert!(anticheat::launch_option_ok(&steam::launch_options_in(&out, "1245620").unwrap(), Path::new("/home/deck/.var/app/io.github.ravenor907.TurbineModManager/data")));
}

#[test]
fn adds_the_value_or_the_game() {
    // The game listed without Launch Options.
    let out = steam_setup::set_launch_options_in(CONFIG, "489830", "export A=1; %command%").unwrap();
    assert_eq!(steam::launch_options_in(&out, "489830").unwrap(), "export A=1; %command%");
    assert!(out.contains("\t\t\t\t\t\t\"LastPlayed\"\t\t\"1\"\n\t\t\t\t\t\t\"LaunchOptions\"\t\t\"export A=1; %command%\"\n\t\t\t\t\t}"), "{out}");
    // The game not listed yet.
    let out = steam_setup::set_launch_options_in(CONFIG, "1888160", "%command% -x").unwrap();
    assert_eq!(steam::launch_options_in(&out, "1888160").unwrap(), "%command% -x");
    assert_eq!(steam::launch_options_in(&out, "1245620").unwrap(), "PROTON_LOG=1 \"x\" %command%");
    // Broken or unexpected files are refused.
    assert!(steam_setup::set_launch_options_in("\"UserLocalConfigStore\"\n{\n", "1", "x").is_err());
    assert!(steam_setup::set_launch_options_in("\"UserLocalConfigStore\"\n{\n}\n", "1", "x").is_err());
    assert!(steam_setup::set_launch_options_in(CONFIG, "12a", "x").is_err());
}

#[test]
fn writes_only_while_steam_is_closed_with_a_backup() {
    let d = std::env::temp_dir().join(format!("turbine-steamsetup-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    let file = d.join(".local/share/Steam/userdata/42/config/localconfig.vdf");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::create_dir_all(d.join(".local/share/Steam/steamapps")).unwrap();
    fs::write(&file, CONFIG).unwrap();
    assert_eq!(steam_setup::localconfig_files(&d), vec![file.clone()]);
    assert!(steam_setup::write_launch_options(&file, "1245620", &line(), true).unwrap_err().contains("Close Steam first"));
    assert_eq!(fs::read_to_string(&file).unwrap(), CONFIG);
    let was = steam_setup::write_launch_options(&file, "1245620", &line(), false).unwrap();
    assert_eq!(was.as_deref(), Some("PROTON_LOG=1 \"x\" %command%"), "the old ones, for Undo");
    assert_eq!(fs::read_to_string(steam_setup::backup_file(&file)).unwrap(), CONFIG);
    assert_eq!(fs::read_to_string(steam_setup::original_file(&file)).unwrap(), CONFIG);
    assert_eq!(steam::launch_options(&d, "1245620").unwrap(), line());
    // The same again: nothing to write, even with Steam running.
    let now = fs::read_to_string(&file).unwrap();
    assert_eq!(steam_setup::write_launch_options(&file, "1245620", &line(), true).unwrap(), Some(line()));
    assert_eq!(fs::read_to_string(&file).unwrap(), now);
    // Undo: the old ones back; the first original stays.
    steam_setup::write_launch_options(&file, "1245620", "PROTON_LOG=1 \"x\" %command%", false).unwrap();
    assert_eq!(fs::read_to_string(&file).unwrap(), CONFIG, "back to exactly Steam's own file");
    assert_eq!(fs::read_to_string(steam_setup::original_file(&file)).unwrap(), CONFIG);
    let _ = fs::remove_dir_all(&d);
}
