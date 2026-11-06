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
