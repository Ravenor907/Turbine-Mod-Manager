//! Facts about the system Turbine runs on.

use std::fs;

/// SteamOS, read from the host's os-release (inside the Flatpak /etc/os-release is the runtime's own).
pub fn is_steamos() -> bool {
    for path in ["/run/host/os-release", "/etc/os-release"] {
        if let Ok(text) = fs::read_to_string(path) {
            return os_release_is_steamos(&text);
        }
    }
    false
}

fn os_release_is_steamos(text: &str) -> bool {
    text.lines().any(|l| {
        let l = l.trim();
        l.strip_prefix("ID=").map(|v| v.trim_matches('"') == "steamos").unwrap_or(false)
    })
}

/// Running inside Game Mode (gamescope) rather than the desktop.
pub fn in_game_mode() -> bool {
    let var = |k: &str| std::env::var(k).unwrap_or_default();
    var("XDG_CURRENT_DESKTOP").to_ascii_lowercase().contains("gamescope")
        || !var("GAMESCOPE_WAYLAND_DISPLAY").is_empty()
        || var("SteamGamepadUI") == "1"
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_os_release() {
        assert!(super::os_release_is_steamos("NAME=\"SteamOS\"\nID=steamos\nID_LIKE=arch\n"));
        assert!(!super::os_release_is_steamos("ID=arch\n"));
        // Only SteamOS itself counts; distributions that look like it get the "not supported" notice.
        assert!(!super::os_release_is_steamos("ID=bazzite\nID_LIKE=\"fedora steamos\"\n"));
        assert!(super::os_release_is_steamos("ID=\"steamos\"\n"));
    }
}
