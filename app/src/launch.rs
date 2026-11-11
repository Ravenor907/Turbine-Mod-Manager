//! Opening a web page for the player. In Game Mode gamescope only shows what Steam itself starts, so a browser
//! started any other way opens where nobody can see it. There Turbine finds the player's Chrome-family browser among
//! their non-Steam games (shortcuts.vdf, read only) and the app opens steam://rungameid/<id> with
//! Qt.openUrlExternally, exactly the way Play starts a game (and Steam's keyboard opens), so Steam starts it where
//! the player sees it. The browser helper then opens the page in a new tab: sent at once when it is connected, or
//! kept until it connects. A helper too old to know "open" doesn't make the page wait: Steam's own browser opens it
//! right away. With no browser in Steam the app shows how to add one (once), and Steam's own browser is the fallback
//! (steam://openurl, opened the same way). Desktop Mode keeps the normal way. Each step is written to the activity
//! log; nothing here starts a program.

use crate::bridge;
use turbine_engine::browser;
use turbine_engine::json::{self, quote};
use turbine_engine::shortcuts;

/// Opens `url`. `browser_json` is the browser check's answer ({"result":{"found","name","how"}}). Answers
/// {"how", "name", "error", ...}:
/// - "desktop": the app opens it with Qt.openUrlExternally itself;
/// - "shortcut": Game Mode, a browser is among the player's non-Steam games; also "browser", "steam" (the
///   steam://rungameid link the app opens now), "helper" (a helper has paired) and "sent" (the page went to the
///   helper now, rather than waiting for it to connect);
/// - "oldhelper": Game Mode, the helper that last connected is too old to open pages: "steam" is Steam's browser
///   (steam://openurl) with the page, which the app opens right away;
/// - "noshortcut": Game Mode, no Chrome-family browser among the player's non-Steam games.
pub fn open(url: &str, game_mode: bool, browser_json: &str) -> String {
    // The same page asked for again while the browser is still starting (A pressed twice because nothing seemed to
    // happen): nothing is started a second time.
    if let Some(ago) = recently_opened(url, std::time::Instant::now()) {
        crate::log::line("browser", &format!("Already opening this page ({ago} ms ago): not starting the browser again"));
        return "{\"how\":\"guarded\",\"name\":\"\",\"error\":\"\"}".into();
    }
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
                    format!("The helper isn't connected yet: it opens {url} when it connects (within {} seconds)", bridge::OPEN_WAIT.as_secs())
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
        "{{\"how\":\"shortcut\",\"name\":{},\"browser\":{},\"steam\":{},\"helper\":{helper},\"sent\":{sent},\"error\":\"\"}}",
        quote(&sc.browser),
        quote(&sc.name),
        quote(&link)
    )
}

/// How long the same page isn't opened again: the browser can take this long to come up in Game Mode.
pub const RELAUNCH_GUARD: std::time::Duration = std::time::Duration::from_secs(8);

static LAST_OPEN: std::sync::Mutex<Vec<(String, std::time::Instant)>> = std::sync::Mutex::new(Vec::new());

/// Some(milliseconds since) when `url` was opened less than `RELAUNCH_GUARD` ago; otherwise remembers it as opened
/// now and answers None. Each page is remembered on its own (A, B, then A again within 8 seconds opens A once).
/// Steam's browser (the player's own choice after a press) is never held back by this.
pub(crate) fn recently_opened(url: &str, now: std::time::Instant) -> Option<u128> {
    let mut last = LAST_OPEN.lock().unwrap_or_else(|e| e.into_inner());
    crate::bridge::page_guard(&mut last, url, now, RELAUNCH_GUARD)
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

/// Steam's own browser (the safety net in Game Mode): the steam://openurl link the app opens with
/// Qt.openUrlExternally, or "" when the address can't be opened that way.
pub fn open_in_steam(url: &str) -> String {
    match browser::steam_browser_link(url) {
        Some(link) => {
            crate::log::line("browser", "Asked Steam's browser to open Nexus (steam://openurl)");
            link
        }
        None => {
            crate::log::line("browser", "Steam's browser can't open that address");
            String::new()
        }
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
