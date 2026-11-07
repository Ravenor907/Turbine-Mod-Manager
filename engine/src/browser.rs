//! Is there a web browser on this system? Turbine opens Nexus pages in the player's own browser (Chrome, Firefox and
//! so on); it has none of its own. SteamOS ships without one, so the app checks first and, when none is found, shows
//! plain steps to install one.
//!
//! Three places are asked, on the host (Turbine runs as a Flatpak, so through flatpak-spawn --host, with fixed
//! arguments and never a shell):
//! - the desktop's default browser: `xdg-settings get default-web-browser` prints a desktop file name such as
//!   "org.mozilla.firefox.desktop", or nothing when none is set;
//! - Flatpak apps: `flatpak list --app --columns=application` prints one application ID per line (Discover installs
//!   browsers as Flatpaks on SteamOS);
//! - browsers installed with the system: `ls -1 -d --` on a fixed list of program paths prints the ones that exist.
//!
//! Parsing is pure and tested with recorded outputs; `check` runs the commands.

use std::path::Path;
use std::process::{Command, Stdio};

/// Browsers from Flathub, by application ID, with the name the player knows them by.
pub const FLATPAK_BROWSERS: &[(&str, &str)] = &[
    ("com.google.Chrome", "Google Chrome"),
    ("org.mozilla.firefox", "Firefox"),
    ("org.chromium.Chromium", "Chromium"),
    ("com.brave.Browser", "Brave"),
    ("com.microsoft.Edge", "Microsoft Edge"),
    ("io.gitlab.librewolf-community", "LibreWolf"),
    ("com.vivaldi.Vivaldi", "Vivaldi"),
    ("com.opera.Opera", "Opera"),
];

/// Browsers installed with the system, by program path.
pub const HOST_BROWSERS: &[(&str, &str)] = &[
    ("/usr/bin/firefox", "Firefox"),
    ("/usr/bin/chromium", "Chromium"),
    ("/usr/bin/chromium-browser", "Chromium"),
    ("/usr/bin/google-chrome", "Google Chrome"),
    ("/usr/bin/google-chrome-stable", "Google Chrome"),
    ("/usr/bin/brave", "Brave"),
    ("/usr/bin/brave-browser", "Brave"),
    ("/usr/bin/microsoft-edge", "Microsoft Edge"),
    ("/usr/bin/microsoft-edge-stable", "Microsoft Edge"),
    ("/usr/bin/librewolf", "LibreWolf"),
    ("/usr/bin/vivaldi", "Vivaldi"),
    ("/usr/bin/vivaldi-stable", "Vivaldi"),
    ("/usr/bin/opera", "Opera"),
    ("/usr/bin/falkon", "Falkon"),
    ("/usr/bin/epiphany", "GNOME Web"),
];

/// Desktop file names (without ".desktop") of system packages, for naming the default browser.
const DESKTOP_NAMES: &[(&str, &str)] = &[
    ("firefox", "Firefox"),
    ("chromium", "Chromium"),
    ("chromium-browser", "Chromium"),
    ("google-chrome", "Google Chrome"),
    ("brave-browser", "Brave"),
    ("microsoft-edge", "Microsoft Edge"),
    ("librewolf", "LibreWolf"),
    ("vivaldi-stable", "Vivaldi"),
    ("opera", "Opera"),
    ("org.kde.falkon", "Falkon"),
    ("org.gnome.epiphany", "GNOME Web"),
];

/// What the check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserCheck {
    pub found: bool,
    /// The browser's name ("Firefox"), empty when none was found.
    pub name: String,
    /// How it was found: "default" (the desktop's default browser), "flatpak", "system", or "" when none was.
    pub how: String,
}

impl BrowserCheck {
    fn none() -> BrowserCheck {
        BrowserCheck { found: false, name: String::new(), how: String::new() }
    }
    fn some(name: &str, how: &str) -> BrowserCheck {
        BrowserCheck { found: true, name: name.into(), how: how.into() }
    }

    /// {"found":true,"name":"Firefox","how":"flatpak"} for the app.
    pub fn to_json(&self) -> String {
        format!(
            "{{\"found\":{},\"name\":{},\"how\":{}}}",
            self.found,
            crate::json::quote(&self.name),
            crate::json::quote(&self.how)
        )
    }
}

/// Title of the warning shown when no browser is found.
pub const NO_BROWSER_TITLE: &str = "No web browser found";
/// Why it matters, in one sentence.
pub const NO_BROWSER_TEXT: &str =
    "Turbine opens Nexus Mods in your web browser, and this system doesn't have one yet. Installing one takes a minute.";
/// The steps, one per line, in the order the player does them.
pub const INSTALL_STEPS: &[&str] = &[
    "Switch to Desktop Mode: press the Steam button, open Power, and pick Switch to Desktop.",
    "In Desktop Mode, open Discover, search for Google Chrome or Firefox, and press Install.",
    "Optional, so it also opens in Game Mode: open the app menu, right-click the browser, and pick Add to Steam.",
    "Switch back to Game Mode and open Turbine again.",
];

/// The desktop file name xdg-settings printed, without ".desktop"; None when no default is set.
pub fn parse_default_browser(out: &str) -> Option<String> {
    let line = out.lines().map(str::trim).find(|l| !l.is_empty())?;
    let id = line.strip_suffix(".desktop").unwrap_or(line).trim();
    // An error message or anything that isn't a file name is not an answer.
    if id.is_empty() || id.contains(char::is_whitespace) || id.contains('/') {
        return None;
    }
    Some(id.to_string())
}

/// A plain name for a desktop file name ("org.mozilla.firefox" is "Firefox"); unknown ones read from their last part.
pub fn browser_name(desktop_id: &str) -> String {
    if let Some((_, n)) = FLATPAK_BROWSERS.iter().find(|(id, _)| id.eq_ignore_ascii_case(desktop_id)) {
        return n.to_string();
    }
    if let Some((_, n)) = DESKTOP_NAMES.iter().find(|(id, _)| id.eq_ignore_ascii_case(desktop_id)) {
        return n.to_string();
    }
    let last = desktop_id.rsplit('.').next().unwrap_or(desktop_id);
    let mut c = last.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => desktop_id.to_string(),
    }
}

/// Application IDs from `flatpak list --app --columns=application`.
pub fn parse_flatpak_apps(out: &str) -> Vec<String> {
    out.lines()
        .map(|l| l.split_whitespace().next().unwrap_or("").to_string())
        .filter(|l| !l.is_empty() && l.contains('.') && !l.eq_ignore_ascii_case("application"))
        .collect()
}

/// The first known browser among installed Flatpak apps, in FLATPAK_BROWSERS order.
pub fn known_flatpak_browser(apps: &[String]) -> Option<(&'static str, &'static str)> {
    FLATPAK_BROWSERS.iter().copied().find(|(id, _)| apps.iter().any(|a| a.eq_ignore_ascii_case(id)))
}

/// The first known browser among the program paths `ls` printed.
pub fn known_host_browser(ls_out: &str) -> Option<(&'static str, &'static str)> {
    let found: Vec<&str> = ls_out.lines().map(str::trim).filter(|l| l.starts_with('/')).collect();
    HOST_BROWSERS.iter().copied().find(|(p, _)| found.contains(p))
}

/// Puts the three answers together. Each is the command's output, or None when it couldn't run.
/// A default browser counts only when it is a browser Turbine can confirm (one of the lists, or found installed);
/// a default left pointing at an app that was removed would otherwise hide the warning.
pub fn decide(default_out: Option<&str>, flatpak_out: Option<&str>, host_out: Option<&str>) -> BrowserCheck {
    let apps = flatpak_out.map(parse_flatpak_apps).unwrap_or_default();
    let flatpak = known_flatpak_browser(&apps);
    let host = host_out.and_then(known_host_browser);
    if let Some(id) = default_out.and_then(parse_default_browser) {
        let is_flatpak_app = apps.iter().any(|a| a.eq_ignore_ascii_case(&id));
        let known_flatpak = FLATPAK_BROWSERS.iter().any(|(f, _)| f.eq_ignore_ascii_case(&id));
        let known_system = DESKTOP_NAMES.iter().any(|(d, _)| d.eq_ignore_ascii_case(&id));
        // A Flathub browser named as the default must also be installed; a system one is trusted when a system
        // browser was seen (or the program list couldn't be read).
        let confirmed = (known_flatpak && (is_flatpak_app || flatpak_out.is_none()))
            || (known_system && (host.is_some() || host_out.is_none()))
            || (!known_flatpak && is_flatpak_app);
        if confirmed {
            return BrowserCheck::some(&browser_name(&id), "default");
        }
    }
    if let Some((_, name)) = flatpak {
        return BrowserCheck::some(name, "flatpak");
    }
    if let Some((_, name)) = host {
        return BrowserCheck::some(name, "system");
    }
    BrowserCheck::none()
}

/// The commands `check` runs, as argument lists (each starts with the program). Inside the Flatpak each is prefixed
/// with flatpak-spawn --host.
pub fn commands() -> [Vec<&'static str>; 3] {
    let mut ls = vec!["ls", "-1", "-d", "--"];
    ls.extend(HOST_BROWSERS.iter().map(|(p, _)| *p));
    [vec!["xdg-settings", "get", "default-web-browser"], vec!["flatpak", "list", "--app", "--columns=application"], ls]
}

/// Runs one command on the host and returns what it printed, even when it ended with an error (ls does when some of
/// the paths are missing). None when it couldn't start at all.
pub fn host_output(args: &[&str]) -> Option<String> {
    let (prog, rest) = args.split_first()?;
    let mut cmd = if Path::new("/.flatpak-info").exists() {
        let mut c = Command::new("flatpak-spawn");
        c.arg("--host").arg(prog);
        c
    } else {
        Command::new(prog)
    };
    let out = cmd.args(rest).stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Checks this system with the given way of running commands (tests pass recorded outputs).
pub fn check_with(run: &dyn Fn(&[&str]) -> Option<String>) -> BrowserCheck {
    let [d, f, h] = commands();
    let default_out = run(&d);
    let flatpak_out = run(&f);
    let host_out = run(&h);
    decide(default_out.as_deref(), flatpak_out.as_deref(), host_out.as_deref())
}

/// Checks this system for a web browser.
pub fn check() -> BrowserCheck {
    check_with(&host_output)
}

/// The program and arguments that open `url` in the browser the check found, run on the host (Game Mode: gamescope
/// shows only windows of the focused Steam app, so the browser is started with Turbine's own SteamGameId and
/// SteamAppId, and its window counts as Turbine's). Fixed arguments, never a shell. None when the browser isn't one
/// Turbine knows how to start, or the address isn't https.
pub fn game_mode_command(name: &str, how: &str, url: &str, steam_game_id: Option<&str>) -> Option<Vec<String>> {
    if !url.starts_with("https://") || url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return None;
    }
    let mut out = vec!["flatpak-spawn".to_string(), "--host".to_string()];
    if let Some(id) = steam_game_id.filter(|i| !i.is_empty() && i.bytes().all(|b| b.is_ascii_digit())) {
        out.push(format!("--env=SteamGameId={id}"));
        out.push(format!("--env=SteamAppId={id}"));
    }
    let flatpak = FLATPAK_BROWSERS.iter().find(|(_, n)| n.eq_ignore_ascii_case(name)).map(|(id, _)| *id);
    let host = HOST_BROWSERS.iter().find(|(_, n)| n.eq_ignore_ascii_case(name)).map(|(p, _)| *p);
    match (how, flatpak, host) {
        ("system", _, Some(p)) | ("default", None, Some(p)) => out.push(p.to_string()),
        (_, Some(id), _) => out.extend(["flatpak".to_string(), "run".to_string(), id.to_string()]),
        (_, None, Some(p)) => out.push(p.to_string()),
        _ => return None,
    }
    out.push(url.to_string());
    Some(out)
}

/// Steam's own browser: steam://openurl/<url>, opened on the host.
pub fn steam_browser_command(url: &str) -> Option<Vec<String>> {
    if !url.starts_with("https://") || url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return None;
    }
    Some(vec!["flatpak-spawn".into(), "--host".into(), "xdg-open".into(), format!("steam://openurl/{url}")])
}
