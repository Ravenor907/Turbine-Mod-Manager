//! Facts about the system Turbine runs on.

use std::fs;
use std::process::{Command, Stdio};

/// Turbine runs as a Flatpak.
pub fn in_flatpak() -> bool {
    std::path::Path::new("/.flatpak-info").exists()
}

/// A program run on the host: through flatpak-spawn --host inside the Flatpak (which can't see or change the host by
/// itself), else directly.
pub fn host(program: &str) -> Command {
    if in_flatpak() {
        let mut c = Command::new("flatpak-spawn");
        c.arg("--host").arg(program);
        c
    } else {
        Command::new(program)
    }
}

/// How long a quick question to the host (pgrep, ps, test, grep) may take before Turbine stops waiting for it.
pub const HOST_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

/// Waits for a started program, at most `limit`: one that takes longer is stopped and None is the answer, so a
/// program that hangs can't hold up whoever asked.
pub fn wait_limited(child: &mut std::process::Child, limit: std::time::Duration) -> Option<std::process::ExitStatus> {
    let until = std::time::Instant::now() + limit;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) if std::time::Instant::now() < until => std::thread::sleep(std::time::Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

/// Reads everything a program writes, on its own thread (a full pipe would stop the program).
fn drain<R: std::io::Read + Send + 'static>(pipe: Option<R>) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut p) = pipe {
            let _ = p.read_to_end(&mut buf);
        }
        buf
    })
}

/// Command::output with a time limit: what the program wrote and how it ended, or an error of the kind TimedOut
/// when it was still running after `limit` (Turbine stops waiting for it then).
pub fn output_limited(cmd: &mut Command, limit: std::time::Duration) -> std::io::Result<std::process::Output> {
    let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    let out = drain(child.stdout.take());
    let err = drain(child.stderr.take());
    match wait_limited(&mut child, limit) {
        Some(status) => Ok(std::process::Output { status, stdout: out.join().unwrap_or_default(), stderr: err.join().unwrap_or_default() }),
        None => Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "it took too long")),
    }
}

/// How long an installer run on the host (Turbine's own update, the Visual C++ runtime) may take.
pub const INSTALL_WAIT: std::time::Duration = std::time::Duration::from_secs(600);

/// Runs a yes-or-no command quietly: exit code 0 is yes, 1 (or any code in `no`) is no. None when it couldn't run or
/// said something else.
fn answer(cmd: &mut Command, no: &[i32]) -> Option<bool> {
    let mut child = cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().ok()?;
    let status = wait_limited(&mut child, HOST_WAIT)?;
    match status.code()? {
        0 => Some(true),
        c if c == 1 || no.contains(&c) => Some(false),
        _ => None,
    }
}

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

/// The developer switch: TURBINE_DEVELOPER=1 (tools/dev-build.sh sets it) lets Turbine run on a development PC as if
/// it were supported, without the "not supported" notice. It changes nothing else.
pub const DEVELOPER_ENV: &str = "TURBINE_DEVELOPER";

pub fn developer() -> bool {
    std::env::var(DEVELOPER_ENV).is_ok_and(|v| v == "1")
}

/// SteamOS, or the developer switch: what the "not supported" notice asks.
pub fn supported() -> bool {
    is_steamos() || developer()
}

/// Running under WSL (Windows' Linux): its kernel names itself "microsoft". WSL has no GPU that the Flatpak's Mesa
/// can use.
pub fn in_wsl() -> bool {
    fs::read_to_string("/proc/sys/kernel/osrelease").is_ok_and(|r| kernel_is_wsl(&r))
}

fn kernel_is_wsl(release: &str) -> bool {
    let r = release.to_ascii_lowercase();
    r.contains("microsoft") || r.contains("wsl")
}

/// Is there a GPU to draw with (a render node in /dev/dri)? On a device without one Mesa only prints errors before it
/// falls back by itself.
pub fn has_render_node() -> bool {
    fs::read_dir("/dev/dri").is_ok_and(|d| d.flatten().any(|e| e.file_name().to_string_lossy().starts_with("renderD")))
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
    unsafe extern "C" {
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
    let pattern = turbine_engine::proton::running_pattern(dir);
    answer(host("pgrep").arg("-f").arg("--").arg(&pattern), &[])
}

/// Is Steam running on the host? None when it can't be asked.
pub fn steam_running() -> Option<bool> {
    answer(host("pgrep").arg("-x").arg("steam"), &[])
}

/// The desktop file Turbine is installed with (it says it opens nxm:// links).
pub const DESKTOP_FILE: &str = "io.github.ravenor907.TurbineModManager.desktop";
const NXM_TYPE: &str = "x-scheme-handler/nxm";

/// xdg-mime on the host (the Flatpak can't change the host's defaults by itself), with fixed arguments.
fn xdg_mime(args: &[&str]) -> Result<String, String> {
    let out = output_limited(host("xdg-mime").args(args).stdin(Stdio::null()), HOST_WAIT).map_err(|e| format!("xdg-mime couldn't start: {e}"))?;
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

/// Where the Steam menu plugin (decky/) is downloaded from: always the newest release's.
pub const DECKY_PLUGIN_URL: &str = "https://github.com/Ravenor907/Turbine-Mod-Manager/releases/latest/download/Turbine-Decky.zip";

/// Is there a file or folder at this path on the host (`test -e`, fixed arguments)? None when it can't be asked.
fn host_exists(path: &std::path::Path) -> Option<bool> {
    answer(host("test").arg("-e").arg(path), &[])
}

/// Is Turbine's plugin among Decky's plugins (whatever its folder is called)? Its package.json names it
/// "turbine-decky". Asked with grep on the host, fixed arguments, only package.json files read.
fn host_has_plugin(plugins: &std::path::Path) -> Option<bool> {
    // grep says 2 when the folder isn't there.
    answer(host("grep").args(["-r", "-l", "-s", "-F", "--include=package.json", "\"name\": \"turbine-decky\"", "--"]).arg(plugins), &[2])
}

/// Is Turbine's Steam menu plugin installed in Decky (for the Co-op tile, which the plugin adds with Steam's
/// own call)? Read only.
pub fn decky_plugin_installed() -> bool {
    host_has_plugin(&turbine_engine::home_dir().join("homebrew/plugins")) == Some(true)
}

/// Settings > Help > Steam menu plugin: is Decky Loader on this device, and Turbine's plugin in it?
/// {"checked":true,"decky":true|false|null,"plugin":bool,"url"}. Read only.
pub fn decky_json() -> String {
    use turbine_engine::json::quote;
    let home = turbine_engine::home_dir();
    let decky = host_exists(&home.join("homebrew/services/PluginLoader"));
    let plugin = host_has_plugin(&home.join("homebrew/plugins")) == Some(true);
    crate::log::line("app", &format!("Steam menu plugin: Decky {}, Turbine's plugin {}", match decky { Some(true) => "found", Some(false) => "not found", None => "unknown" }, if plugin { "installed" } else { "not installed" }));
    format!(
        "{{\"checked\":true,\"decky\":{},\"plugin\":{plugin},\"url\":{}}}",
        decky.map(|d| d.to_string()).unwrap_or_else(|| "null".into()),
        quote(DECKY_PLUGIN_URL)
    )
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
    fn reads_wsl_kernel() {
        assert!(super::kernel_is_wsl("6.18.58.1-microsoft-standard-WSL2\n"));
        assert!(super::kernel_is_wsl("4.4.0-19041-Microsoft"));
        assert!(!super::kernel_is_wsl("6.11.11-valve24-2-neptune-611\n"));
    }

    #[test]
    fn reads_free_space() {
        assert!(super::free_bytes(std::path::Path::new("/")).is_some());
        assert!(super::free_bytes(std::path::Path::new("/no/such/folder")).is_none());
    }
}
