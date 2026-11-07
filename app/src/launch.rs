//! Opening a web page for the player. In Game Mode gamescope only shows windows that belong to the Steam app in
//! front, so a browser started the usual way (Qt.openUrlExternally, through the portal) opens where nobody can see
//! it. There the browser is started on the host with Turbine's own SteamGameId and SteamAppId (Steam sets them when
//! it starts Turbine), so gamescope treats its window as Turbine's. If Turbine is still in front 8 seconds later,
//! the app offers Steam's own browser (steam://openurl). Desktop Mode keeps the normal way. Each open is written to
//! the activity log with the way it was done. Fixed arguments, never a shell.

use std::process::{Command, Stdio};
use turbine_engine::browser;
use turbine_engine::json::{self, quote};

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

/// Turbine's own Steam ID (set by Steam when it starts Turbine), if any.
pub fn steam_game_id() -> Option<String> {
    ["SteamGameId", "SteamAppId"]
        .iter()
        .filter_map(|k| std::env::var(k).ok())
        .find(|v| !v.is_empty() && v != "0" && v.bytes().all(|b| b.is_ascii_digit()))
}

/// Opens `url`. `browser_json` is the browser check's answer ({"result":{"found","name","how"}}). Answers
/// {"how": "gamemode" | "desktop" | "portal", "name": the browser's name, "error"}: "desktop" and "portal" mean the
/// app opens it with Qt.openUrlExternally itself.
pub fn open(url: &str, game_mode: bool, browser_json: &str) -> String {
    let check = json::parse(browser_json).ok();
    let result = check.as_ref().and_then(|c| c.get("result"));
    let name = result.and_then(|r| r.str_at("name")).unwrap_or("").to_string();
    let how = result.and_then(|r| r.str_at("how")).unwrap_or("").to_string();
    let answer = |how: &str, error: &str| format!("{{\"how\":{},\"name\":{},\"error\":{}}}", quote(how), quote(&name), quote(error));
    if !game_mode {
        crate::log::line("browser", &format!("Opened Nexus with: the desktop's default browser ({})", if name.is_empty() { "not checked yet" } else { &name }));
        return answer("desktop", "");
    }
    let id = steam_game_id();
    match browser::game_mode_command(&name, &how, url, id.as_deref()) {
        Some(args) => match spawn(&args) {
            Ok(()) => {
                crate::log::line(
                    "browser",
                    &format!("Opened Nexus with: {name} on the host with Turbine's SteamGameId {}", id.as_deref().unwrap_or("(none set)")),
                );
                answer("gamemode", "")
            }
            Err(e) => {
                crate::log::line("browser", &format!("Opened Nexus with: the portal, because {name} couldn't be started ({e})"));
                answer("portal", &e)
            }
        },
        None => {
            crate::log::line("browser", &format!("Opened Nexus with: the portal ({} isn't a browser Turbine knows how to start)", if name.is_empty() { "no browser found" } else { &name }));
            answer("portal", "")
        }
    }
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
