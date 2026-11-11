//! One Turbine at a time, and Nexus Mods' "Mod Manager Download" links (nxm://).
//!
//! The first Turbine listens on a Unix socket in its runtime folder (only the player can open it). A second launch
//! with an nxm:// link (the browser opens one when "Mod Manager Download" is pressed) sends the link to the running
//! Turbine and exits; with no Turbine running, it starts normally and handles the link itself. Only nxm:// lines that
//! read as a Nexus download link are accepted from the socket (and "turbine:show", from a second launch with no link:
//! the running Turbine comes to the front, and 0.6's "turbine:quick" requests from Steam's menu); anything else is
//! dropped.
//!
//! In the Flatpak the folder is $XDG_RUNTIME_DIR/app/<app id>/, which Flatpak shares between every copy of the same
//! app (the rest of the runtime folder is private to each copy); outside it, $XDG_RUNTIME_DIR/<app id>/.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Longest line accepted from the socket (a Nexus link is well under 1 KB).
const MAX_LINE: u64 = 4096;
/// What a second launch with no link sends: the running Turbine comes to the front.
pub const SHOW: &str = "turbine:show";

/// Where the socket lives. None when there is no runtime folder.
pub fn socket_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_RUNTIME_DIR").filter(|v| !v.is_empty()).map(PathBuf::from)?;
    let dir = if crate::system::in_flatpak() { base.join("app").join(crate::APP_ID) } else { base.join(crate::APP_ID) };
    Some(dir.join("turbine.sock"))
}

/// Is this a Nexus download link Turbine can handle? Checked again in full when it's used.
pub fn plausible_link(line: &str) -> bool {
    let l = line.trim();
    l.len() <= MAX_LINE as usize
        && l.get(..6).is_some_and(|p| p.eq_ignore_ascii_case("nxm://"))
        && l.chars().all(|c| c.is_ascii_graphic())
        && turbine_engine::nxm::parse(l).is_ok()
}

/// Links waiting for the interface (it asks a few times a second).
fn queue() -> std::sync::MutexGuard<'static, std::collections::VecDeque<String>> {
    static Q: std::sync::OnceLock<std::sync::Mutex<std::collections::VecDeque<String>>> = std::sync::OnceLock::new();
    Q.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner())
}

pub fn push(link: String) {
    let mut q = queue();
    if q.len() < 20 {
        q.push_back(link);
    }
}

pub fn take() -> Option<String> {
    queue().pop_front()
}

/// The nxm:// link among the program's arguments, if any.
pub fn link_from_args() -> Option<String> {
    std::env::args().skip(1).find(|a| a.get(..6).is_some_and(|p| p.eq_ignore_ascii_case("nxm://")))
}

/// What starting found.
pub enum Start {
    /// This is the only Turbine: it listens for links from later launches.
    First(UnixListener),
    /// Another Turbine is running and took the link: this one exits.
    Forwarded,
    /// Couldn't tell (no runtime folder, or the socket can't be made): start without listening.
    Alone(String),
}

fn send(path: &Path, link: &str) -> std::io::Result<()> {
    let mut s = UnixStream::connect(path)?;
    s.set_write_timeout(Some(Duration::from_secs(3)))?;
    s.write_all(link.trim().as_bytes())?;
    s.write_all(b"\n")?;
    s.flush()?;
    // Wait for the running Turbine to say it took the line ("ok"; "no" when it was refused).
    s.set_read_timeout(Some(Duration::from_secs(3)))?;
    let mut ok = [0u8; 2];
    let n = s.read(&mut ok).unwrap_or(0);
    if n == 2 && &ok == b"no" {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Turbine didn't take that request"));
    }
    Ok(())
}

/// 0.6: hands one request from Steam's menu to the Turbine that is open (Err when none answers).
pub fn send_request(line: &str) -> std::io::Result<()> {
    let path = socket_path().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "there is no runtime folder"))?;
    send(&path, line)
}

/// Finds out whether another Turbine runs. With `link`, a running Turbine gets it.
pub fn start(link: Option<&str>) -> Start {
    let Some(path) = socket_path() else { return Start::Alone("there is no runtime folder".into()) };
    let Some(dir) = path.parent() else { return Start::Alone("the runtime folder has no parent".into()) };
    if UnixStream::connect(&path).is_ok() {
        // Another Turbine answers.
        return match link {
            Some(l) => match send(&path, l) {
                Ok(()) => Start::Forwarded,
                Err(e) => Start::Alone(format!("the running Turbine didn't take the link ({e})")),
            },
            // No link: the running one comes to the front, and this one ends (it never shares the running one's
            // partial downloads or browser profile).
            None => match send(&path, SHOW) {
                Ok(()) => Start::Forwarded,
                Err(e) => Start::Alone(format!("another Turbine is running but didn't answer ({e})")),
            },
        };
    }
    if let Err(e) = fs::create_dir_all(dir) {
        return Start::Alone(format!("its folder couldn't be made ({e})"));
    }
    let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o700));
    // Nothing answers, so a socket file left here is from a Turbine that didn't close: it goes.
    if fs::symlink_metadata(&path).is_ok() {
        let _ = fs::remove_file(&path);
    }
    match UnixListener::bind(&path) {
        Ok(l) => {
            let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
            Start::First(l)
        }
        Err(e) => Start::Alone(format!("the socket couldn't be made ({e})")),
    }
}

/// A line Turbine takes on its socket: bring it to the front, a Nexus link, or a request from Steam's menu.
fn accepted(line: &str) -> bool {
    line == SHOW || plausible_link(line) || turbine_engine::quick::parse_socket_line(line).is_some()
}

/// Reads one line from a connection: a line `accepted` takes, or nothing.
fn read_link(stream: UnixStream) -> Option<String> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
    let mut writer = stream.try_clone().ok();
    let mut line = String::new();
    let mut r = BufReader::new(stream.take(MAX_LINE + 1));
    r.read_line(&mut line).ok()?;
    let line = line.trim().to_string();
    // 0.6: a request from Steam's menu (quick.rs) for the game Turbine has open. The answer says whether the line
    // was taken, after it is checked (break L2): "ok" or "no".
    let taken = accepted(&line);
    if let Some(w) = writer.as_mut() {
        let _ = w.write_all(if taken { b"ok" } else { b"no" });
    }
    if taken {
        Some(line)
    } else {
        crate::log::line("nxm", &format!("Dropped a line from the socket that isn't a Nexus download link ({} bytes)", line.len()));
        None
    }
}

/// Listens on its own thread; every link that arrives goes to `got`.
pub fn listen(listener: UnixListener, got: impl Fn(String) + Send + 'static) {
    let spawned = std::thread::Builder::new().name("turbine-links".into()).spawn(move || {
        for conn in listener.incoming() {
            match conn {
                Ok(s) => {
                    if let Some(link) = read_link(s) {
                        got(link);
                    }
                }
                Err(e) => {
                    crate::log::line("nxm", &format!("Socket error: {e}"));
                    std::thread::sleep(Duration::from_millis(200));
                }
            }
        }
    });
    if let Err(e) = spawned {
        crate::log::line("nxm", &format!("Couldn't listen for links: {e}"));
    }
}

/// Removes the socket when Turbine closes.
pub fn stop() {
    if let Some(p) = socket_path() {
        let _ = fs::remove_file(p);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn checks_links() {
        assert!(super::plausible_link("nxm://skyrimspecialedition/mods/266/files/12345?key=abc&expires=1700000000&user_id=1"));
        assert!(!super::plausible_link("https://example.com"));
        assert!(!super::plausible_link("nxm://bad link with spaces"));
    }

    #[test]
    fn only_lines_it_takes_are_answered_ok() {
        assert!(super::accepted(super::SHOW));
        assert!(super::accepted("nxm://skyrimspecialedition/mods/266/files/12345?key=abc&expires=1700000000&user_id=1"));
        assert!(!super::accepted("hello"));
        assert!(!super::accepted(""));
    }
}
