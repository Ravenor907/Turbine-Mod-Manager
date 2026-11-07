//! Finding the player's web browser, from recorded outputs of xdg-settings, flatpak list and ls.

use std::cell::RefCell;
use turbine_engine::browser::{
    browser_name, check_with, commands, decide, known_flatpak_browser, known_host_browser, parse_default_browser,
    parse_flatpak_apps, BrowserCheck, INSTALL_STEPS, NO_BROWSER_TEXT, NO_BROWSER_TITLE,
};

/// A SteamOS 3.7 Flatpak list with Firefox from Discover.
const FLATPAK_LIST: &str = "org.kde.kdenlive\norg.mozilla.firefox\ncom.valvesoftware.Steam.CompatibilityTool.Proton-GE\nio.github.ravenor907.TurbineModManager\n";
const FLATPAK_NONE: &str = "org.kde.kdenlive\nio.github.ravenor907.TurbineModManager\n";

#[test]
fn default_browser_output() {
    assert_eq!(parse_default_browser("org.mozilla.firefox.desktop\n").as_deref(), Some("org.mozilla.firefox"));
    assert_eq!(parse_default_browser("\n  firefox.desktop  \n").as_deref(), Some("firefox"));
    assert_eq!(parse_default_browser(""), None);
    assert_eq!(parse_default_browser("xdg-settings: unknown desktop environment\n"), None);
    assert_eq!(browser_name("com.google.Chrome"), "Google Chrome");
    assert_eq!(browser_name("firefox"), "Firefox");
    assert_eq!(browser_name("org.example.Webby"), "Webby");
}

#[test]
fn flatpak_and_system_lists() {
    let apps = parse_flatpak_apps(FLATPAK_LIST);
    assert_eq!(apps.len(), 4);
    assert_eq!(known_flatpak_browser(&apps), Some(("org.mozilla.firefox", "Firefox")));
    assert_eq!(known_flatpak_browser(&parse_flatpak_apps(FLATPAK_NONE)), None);
    // A header line some versions print is ignored.
    assert_eq!(parse_flatpak_apps("Application ID\ncom.brave.Browser\n"), vec!["com.brave.Browser".to_string()]);
    // Every browser the owner asked for is known.
    for id in ["com.google.Chrome", "org.mozilla.firefox", "org.chromium.Chromium", "com.brave.Browser", "com.microsoft.Edge",
        "io.gitlab.librewolf-community", "com.vivaldi.Vivaldi", "com.opera.Opera"] {
        assert!(known_flatpak_browser(&[id.to_string()]).is_some(), "{id}");
    }
    assert_eq!(known_host_browser("/usr/bin/chromium\n"), Some(("/usr/bin/chromium", "Chromium")));
    assert_eq!(known_host_browser(""), None);
}

#[test]
fn putting_the_answers_together() {
    // The default is a browser that is installed.
    assert_eq!(decide(Some("org.mozilla.firefox.desktop"), Some(FLATPAK_LIST), Some("")), BrowserCheck {
        found: true,
        name: "Firefox".into(),
        how: "default".into()
    });
    // No default, a Flatpak browser.
    let c = decide(Some(""), Some(FLATPAK_LIST), Some(""));
    assert_eq!((c.found, c.name.as_str(), c.how.as_str()), (true, "Firefox", "flatpak"));
    // A default pointing at a Flatpak browser that was removed doesn't count.
    let c = decide(Some("com.google.Chrome.desktop"), Some(FLATPAK_NONE), Some(""));
    assert!(!c.found, "{c:?}");
    // A system browser.
    let c = decide(None, Some(FLATPAK_NONE), Some("/usr/bin/firefox\n"));
    assert_eq!((c.found, c.name.as_str(), c.how.as_str()), (true, "Firefox", "system"));
    // Nothing at all: the warning.
    let c = decide(Some(""), Some(FLATPAK_NONE), Some(""));
    assert_eq!(c, BrowserCheck { found: false, name: String::new(), how: String::new() });
    assert_eq!(c.to_json(), "{\"found\":false,\"name\":\"\",\"how\":\"\"}");
    assert!(NO_BROWSER_TITLE.contains("browser") && NO_BROWSER_TEXT.contains("Nexus"));
    assert!(INSTALL_STEPS.iter().any(|s| s.contains("Discover") && s.contains("Google Chrome or Firefox") && s.contains("Install")));
    assert!(INSTALL_STEPS.iter().any(|s| s.contains("Add to Steam")));
}

#[test]
fn check_runs_the_three_commands_with_fixed_arguments() {
    let asked = RefCell::new(Vec::new());
    let run = |args: &[&str]| -> Option<String> {
        asked.borrow_mut().push(args.join(" "));
        match args[0] {
            "xdg-settings" => Some(String::new()),
            "flatpak" => Some("com.brave.Browser\n".into()),
            _ => None, // ls couldn't start
        }
    };
    let c = check_with(&run);
    assert_eq!((c.found, c.name.as_str(), c.how.as_str()), (true, "Brave", "flatpak"));
    let asked = asked.into_inner();
    assert_eq!(asked[0], "xdg-settings get default-web-browser");
    assert_eq!(asked[1], "flatpak list --app --columns=application");
    assert!(asked[2].starts_with("ls -1 -d -- /usr/bin/firefox"));
    // Nothing a shell would read is in any of them.
    for cmd in commands() {
        assert!(cmd.iter().all(|a| !a.contains(';') && !a.contains('|') && !a.contains('$')));
    }
}

#[test]
fn game_mode_launch_uses_turbines_steam_id() {
    use turbine_engine::browser::{game_mode_command, steam_browser_command};
    let url = "https://www.nexusmods.com/witcher3/mods/5?tab=files&file_id=12";
    let c = game_mode_command("Google Chrome", "flatpak", url, Some("3141592653")).unwrap();
    assert_eq!(
        c,
        ["flatpak-spawn", "--host", "--env=SteamGameId=3141592653", "--env=SteamAppId=3141592653", "flatpak", "run", "com.google.Chrome", url]
    );
    let f = game_mode_command("Firefox", "system", url, None).unwrap();
    assert_eq!(f, ["flatpak-spawn", "--host", "/usr/bin/firefox", url]);
    assert_eq!(game_mode_command("Firefox", "default", url, None).unwrap()[2..5], ["flatpak", "run", "org.mozilla.firefox"]);
    assert!(game_mode_command("Unknown Browser", "default", url, None).is_none());
    assert!(game_mode_command("Firefox", "flatpak", "javascript:alert(1)", None).is_none());
    assert!(game_mode_command("Firefox", "flatpak", "https://a b", None).is_none());
    // A SteamGameId that isn't a number is left out.
    assert_eq!(game_mode_command("Firefox", "flatpak", url, Some("x; rm")).unwrap().len(), 6);
    assert_eq!(steam_browser_command(url).unwrap()[3], format!("steam://openurl/{url}"));
}
