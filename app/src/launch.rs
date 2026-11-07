//! Opening a web page for the player. In Game Mode gamescope only shows what Steam itself starts, so a browser
//! started any other way (Qt.openUrlExternally, or on the host with Turbine's SteamGameId) opens where nobody can
//! see it. There Turbine finds the player's Chrome-family browser among their non-Steam games (shortcuts.vdf, read
//! only), starts it through Steam with steam://rungameid/<id> (the same steam:// way Steam's own browser opens),
//! and asks the browser helper to open the page in a new tab: sent at once when the helper is connected, or kept
//! until it connects. With no browser in Steam the app shows how to add one (once), and Steam's own browser is the
//! fallback. Desktop Mode keeps the normal way. Each step is written to the activity log. Fixed arguments, never a
//! shell.

use std::process::{Command, Stdio};
use crate::bridge;
use turbine_engine::browser;
use turbine_engine::json::{self, quote};
use turbine_engine::shortcuts;

fn spawn(args: &[String]) -> Result<(), String> {
    let (prog, rest) = args.split_first().ok_or("nothing to run")?;
    let mut child = Command::new(prog)
        .args(rest)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("{prog} couldn't start: {e}"))?;
    // Reaped in the background so it never lingers as a zombie.
    let _ = std::thread::Builder::new().name("turbine-open".into()).spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

/// Opens `url`. `browser_json` is the browser check's answer ({"result":{"found","name","how"}}). Answers
/// {"how", "name", "error", ...}:
/// - "desktop": the app opens it with Qt.openUrlExternally itself;
/// - "shortcut": Game Mode, the browser was started through Steam; also "browser", "steam" (the steam:// link, which
///   the app opens itself when "error" says starting it failed), "helper" (a helper has paired) and "sent" (the page
///   went to the helper now, rather than waiting for it to connect);
/// - "noshortcut": Game Mode, no Chrome-family browser among the player's non-Steam games.
pub fn open(url: &str, game_mode: bool, browser_json: &str) -> String {
    let check = json::parse(browser_json).ok();
    let result = check.as_ref().and_then(|c| c.get("result"));
    let name = result.and_then(|r| r.str_at("name")).unwrap_or("").to_string();
    if !game_mode {
        crate::log::line("browser", &format!("Opened Nexus with: the desktop's default browser ({})", if name.is_empty() { "not checked yet" } else { &name }));
        return format!("{{\"how\":\"desktop\",\"name\":{},\"error\":\"\"}}", quote(&name));
    }
    open_through_steam(url)
}

/// Game Mode: the player's browser through Steam, then the page through the helper.
pub fn open_through_steam(url: &str) -> String {
    crate::log::line("browser", "Game Mode: looking for a Chrome-family browser among Steam's non-Steam games (shortcuts.vdf, read only)");
    let Some(sc) = shortcuts::find_browser(&turbine_engine::home_dir()) else {
        crate::log::line("browser", "No Chrome-family browser among Steam's non-Steam games: Game Mode can't show one Turbine starts");
        return "{\"how\":\"noshortcut\",\"name\":\"\",\"error\":\"\"}".into();
    };
    crate::log::line(
        "browser",
        &format!("Found {} in Steam as \"{}\" (appid {}, game id {}, from {})", sc.browser, sc.name, sc.appid, shortcuts::game_id(sc.appid), sc.file.display()),
    );
    let link = sc.url();
    let error = match open_steam_link(&link) {
        Ok(()) => {
            crate::log::line("browser", &format!("Started {} through Steam: {link}", sc.browser));
            String::new()
        }
        Err(e) => {
            crate::log::line("browser", &format!("Couldn't start {link} on the host ({e}); the app tries the steam:// link itself"));
            e
        }
    };
    let helper = bridge::paired();
    let sent = match bridge::open_tab(url) {
        Ok(true) => {
            crate::log::line("browser", &format!("The helper is connected: asked it to open {url} in a new tab"));
            true
        }
        Ok(false) => {
            crate::log::line(
                "browser",
                &if helper {
                    format!("Waiting up to {} seconds for the helper to connect, then it opens {url}", bridge::OPEN_WAIT.as_secs())
                } else {
                    "The browser helper isn't set up: the browser opens, but not on the page".to_string()
                },
            );
            false
        }
        Err(e) => {
            crate::log::line("browser", &format!("Didn't ask the helper to open {url}: {e}"));
            false
        }
    };
    format!(
        "{{\"how\":\"shortcut\",\"name\":{},\"browser\":{},\"steam\":{},\"helper\":{helper},\"sent\":{sent},\"error\":{}}}",
        quote(&sc.browser),
        quote(&sc.name),
        quote(&link),
        quote(&error)
    )
}

/// Labs and Recheck: is there a browser among Steam's non-Steam games? {"found":false} or the shortcut's JSON.
pub fn browser_shortcut_json() -> String {
    match shortcuts::find_browser(&turbine_engine::home_dir()) {
        Some(sc) => {
            crate::log::line("browser", &format!("Recheck: found {} in Steam as \"{}\" ({})", sc.browser, sc.name, sc.url()));
            sc.to_json()
        }
        None => {
            let files = shortcuts::shortcut_files(&turbine_engine::home_dir());
            crate::log::line("browser", &format!("Recheck: no Chrome-family browser among Steam's non-Steam games ({} shortcuts file(s) read)", files.len()));
            "{\"found\":false}".into()
        }
    }
}

/// Opens a steam:// link on the host (Steam handles it, so Game Mode shows what it starts).
fn open_steam_link(link: &str) -> Result<(), String> {
    if !link.starts_with("steam://") || link.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err("not a steam:// link".into());
    }
    let args: Vec<String> = if std::path::Path::new("/.flatpak-info").exists() {
        vec!["flatpak-spawn".into(), "--host".into(), "xdg-open".into(), link.into()]
    } else {
        vec!["xdg-open".into(), link.into()]
    };
    spawn(&args)
}

/// Steam's own browser (the safety net in Game Mode). Answers "" or what went wrong.
pub fn open_in_steam(url: &str) -> String {
    match browser::steam_browser_command(url) {
        Some(args) => match spawn(&args) {
            Ok(()) => {
                crate::log::line("browser", "Opened Nexus with: Steam's browser (steam://openurl)");
                String::new()
            }
            Err(e) => {
                crate::log::line("browser", &format!("Steam's browser couldn't be opened: {e}"));
                e
            }
        },
        None => "That address can't be opened.".into(),
    }
}

/// An external display is connected (HDMI or DisplayPort, read from /sys/class/drm).
pub fn external_display() -> bool {
    let Ok(rd) = std::fs::read_dir("/sys/class/drm") else { return false };
    rd.flatten().any(|e| {
        let n = e.file_name().to_string_lossy().into_owned();
        let port = n.starts_with("card") && (n.contains("-HDMI-") || n.contains("-DP-"));
        port && std::fs::read_to_string(e.path().join("status")).map(|s| s.trim() == "connected").unwrap_or(false)
    })
}
