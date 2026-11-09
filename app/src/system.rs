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

/// Free space (bytes) on the drive holding this folder, for the check before Play. None where it can't be read.
#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
pub fn free_bytes(path: &std::path::Path) -> Option<u64> {
    use std::ffi::CString;
    use std::os::raw::{c_char, c_int};
    use std::os::unix::ffi::OsStrExt;
    extern "C" {
        // glibc's statvfs. On 64-bit Linux its struct starts with f_bsize, f_frsize, f_blocks, f_bfree, f_bavail,
        // all 64-bit; the buffer below is larger than the whole struct (112 bytes).
        fn statvfs(path: *const c_char, buf: *mut u64) -> c_int;
    }
    let c = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut buf = [0u64; 32];
    // SAFETY: c is a valid C string and buf is 256 bytes, more than struct statvfs needs.
    let r = unsafe { statvfs(c.as_ptr(), buf.as_mut_ptr()) };
    if r != 0 {
        return None;
    }
    Some(buf[1].saturating_mul(buf[4]))
}

#[cfg(not(all(target_os = "linux", target_pointer_width = "64")))]
pub fn free_bytes(_path: &std::path::Path) -> Option<u64> {
    None
}

/// Is a process running whose command line holds this game folder (the game, or Proton starting it)? Asked on the
/// host through flatpak-spawn, since the Flatpak can't see the game's processes by itself. None when it can't tell.
pub fn game_running(dir: &std::path::Path) -> Option<bool> {
    use std::process::{Command, Stdio};
    let pattern = turbine_engine::proton::running_pattern(dir);
    let mut cmd = if std::path::Path::new("/.flatpak-info").exists() {
        let mut c = Command::new("flatpak-spawn");
        c.arg("--host").arg("pgrep");
        c
    } else {
        Command::new("pgrep")
    };
    let status = cmd.arg("-f").arg("--").arg(&pattern).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status().ok()?;
    match status.code() {
        Some(0) => Some(true),
        Some(1) => Some(false),
        _ => None,
    }
}

/// Is Steam running on the host? None when it can't be asked.
pub fn steam_running() -> Option<bool> {
    use std::process::{Command, Stdio};
    let mut cmd = if std::path::Path::new("/.flatpak-info").exists() {
        let mut c = Command::new("flatpak-spawn");
        c.arg("--host").arg("pgrep");
        c
    } else {
        Command::new("pgrep")
    };
    let status = cmd.arg("-x").arg("steam").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status().ok()?;
    match status.code() {
        Some(0) => Some(true),
        Some(1) => Some(false),
        _ => None,
    }
}

/// The desktop file Turbine is installed with (it says it opens nxm:// links).
pub const DESKTOP_FILE: &str = "io.github.ravenor907.TurbineModManager.desktop";
const NXM_TYPE: &str = "x-scheme-handler/nxm";

/// xdg-mime on the host (the Flatpak can't change the host's defaults by itself), with fixed arguments.
fn xdg_mime(args: &[&str]) -> Result<String, String> {
    use std::process::{Command, Stdio};
    let mut cmd = if std::path::Path::new("/.flatpak-info").exists() {
        let mut c = Command::new("flatpak-spawn");
        c.arg("--host").arg("xdg-mime");
        c
    } else {
        Command::new("xdg-mime")
    };
    let out = cmd.args(args).stdin(Stdio::null()).output().map_err(|e| format!("xdg-mime couldn't start: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() { "xdg-mime stopped with an error".into() } else { err });
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Settings > Accounts: {"state": "yes" | "no" | "unknown", "current": the program now used for Nexus links,
/// "message"}.
pub fn nxm_handler_json() -> String {
    use turbine_engine::json::quote;
    match xdg_mime(&["query", "default", NXM_TYPE]) {
        Ok(now) => {
            let yes = now.eq_ignore_ascii_case(DESKTOP_FILE);
            let shown = now.trim_end_matches(".desktop").rsplit('.').next().unwrap_or(&now).to_string();
            format!("{{\"state\":{},\"current\":{},\"message\":\"\"}}", quote(if yes { "yes" } else { "no" }), quote(&shown))
        }
        Err(e) => {
            crate::log::line("nxm", &format!("Couldn't ask which program opens Nexus links: {e}"));
            format!("{{\"state\":\"unknown\",\"current\":\"\",\"message\":{}}}", quote(&e))
        }
    }
}

/// Makes Turbine the program for Nexus's Mod Manager Download links.
pub fn set_nxm_handler() -> Result<(), String> {
    let r = xdg_mime(&["default", DESKTOP_FILE, NXM_TYPE]).map(|_| ());
    crate::log::line("nxm", &format!("Set as the Nexus link handler: {}", match &r { Ok(()) => "done".to_string(), Err(e) => e.clone() }));
    r
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

    #[test]
    fn reads_free_space() {
        assert!(super::free_bytes(std::path::Path::new("/")).is_some());
        assert!(super::free_bytes(std::path::Path::new("/no/such/folder")).is_none());
    }
}

/// 0.6: where the Steam menu plugin (decky/) is downloaded from: always the newest release's.
pub const DECKY_PLUGIN_URL: &str = "https://github.com/Ravenor907/Turbine-Mod-Manager/releases/latest/download/Turbine-Decky.zip";

/// Is there a file or folder at this path on the host (`test -e`, fixed arguments)? None when it can't be asked.
fn host_exists(path: &std::path::Path) -> Option<bool> {
    use std::process::{Command, Stdio};
    let mut cmd = if std::path::Path::new("/.flatpak-info").exists() {
        let mut c = Command::new("flatpak-spawn");
        c.arg("--host").arg("test");
        c
    } else {
        Command::new("test")
    };
    let status = cmd.arg("-e").arg(path).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status().ok()?;
    match status.code() {
        Some(0) => Some(true),
        Some(1) => Some(false),
        _ => None,
    }
}

/// 0.6, Settings > Help > Steam menu plugin: is Decky Loader on this device, and Turbine's plugin in it?
/// {"checked":true,"decky":true|false|null,"plugin":bool,"url"}. Read only.
pub fn decky_json() -> String {
    use turbine_engine::json::quote;
    let home = turbine_engine::home_dir();
    let decky = host_exists(&home.join("homebrew/services/PluginLoader"));
    let plugin = host_exists(&home.join("homebrew/plugins/Turbine/plugin.json")) == Some(true);
    crate::log::line("app", &format!("Steam menu plugin: Decky {}, Turbine's plugin {}", match decky { Some(true) => "found", Some(false) => "not found", None => "unknown" }, if plugin { "installed" } else { "not installed" }));
    format!(
        "{{\"checked\":true,\"decky\":{},\"plugin\":{plugin},\"url\":{}}}",
        decky.map(|d| d.to_string()).unwrap_or_else(|| "null".into()),
        quote(DECKY_PLUGIN_URL)
    )
}
