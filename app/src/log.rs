//! The activity log and the crash log.
//!
//! Activity: every user action, engine call and result, scan, apply, launch, controller and window change and Qt
//! message, one timestamped line each. The last 2,000 lines stay in memory for Labs reports; all of them also go to
//! activity.log in Turbine's data folder, which starts over at 2 MB (the previous one is kept as activity.log.1).
//!
//! Crash: panics, Qt's fatal errors and runs that ended without closing go to crash.log beside it (kept under 200 KB).
//! Labs > Report a bug attaches its last lines only when the player ticks "Include crash log".
//!
//! Nothing secret is ever written: a key registered with `secret` is replaced by "[hidden]" in every line.

use std::collections::VecDeque;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

const RING: usize = 2000;
const FILE_CAP: u64 = 2 * 1024 * 1024;
const CRASH_CAP: u64 = 200 * 1024;

struct State {
    ring: VecDeque<String>,
    dir: Option<PathBuf>,
    file: Option<File>,
    size: u64,
    secrets: Vec<String>,
}

fn state() -> MutexGuard<'static, State> {
    static S: OnceLock<Mutex<State>> = OnceLock::new();
    let m = S.get_or_init(|| {
        Mutex::new(State { ring: VecDeque::with_capacity(RING), dir: None, file: None, size: 0, secrets: Vec::new() })
    });
    // A panic while the lock was held must not silence the log for the rest of the run.
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// The folder for activity.log and crash.log (made if needed). Call once at start.
pub fn start(dir: PathBuf) {
    let _ = fs::create_dir_all(&dir);
    let mark = dir.join("running.mark");
    let unclean = mark.exists();
    let _ = fs::write(&mark, b"");
    {
        let mut s = state();
        let path = dir.join("activity.log");
        s.size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        s.file = OpenOptions::new().create(true).append(true).open(&path).ok();
        s.dir = Some(dir);
    }
    trim_crash_log();
    if unclean {
        crash("The last run ended without closing (a crash, or it was stopped).");
    }
    install_panic_hook();
}

/// This run is closing normally.
pub fn clean_exit() {
    line("app", "Closed");
    if let Some(dir) = state().dir.clone() {
        let _ = fs::remove_file(dir.join("running.mark"));
    }
}

/// Never written as it is: every line has it replaced by "[hidden]" (the Nexus key, for one).
pub fn secret(text: &str) {
    let t = text.trim();
    if t.len() >= 6 {
        let mut s = state();
        if !s.secrets.iter().any(|x| x == t) {
            s.secrets.push(t.to_string());
        }
    }
}

fn scrub(s: &State, text: &str) -> String {
    let mut out = text.replace(['\n', '\r'], " ");
    for k in &s.secrets {
        if out.contains(k.as_str()) {
            out = out.replace(k.as_str(), "[hidden]");
        }
    }
    out
}

/// One activity line: `kind` says what it is about ("tap", "pad", "page", "engine", "qml" ...).
pub fn line(kind: &str, text: &str) {
    let mut s = state();
    let text = scrub(&s, text);
    let entry = format!("{} {:<7} {}", now_stamp(), kind, text);
    if s.ring.len() >= RING {
        s.ring.pop_front();
    }
    s.ring.push_back(entry.clone());
    let bytes = entry.len() as u64 + 1;
    if s.size + bytes > FILE_CAP {
        rotate(&mut s);
    }
    if let Some(f) = s.file.as_mut() {
        if writeln!(f, "{entry}").is_ok() {
            s.size += bytes;
        }
    }
}

fn rotate(s: &mut State) {
    let Some(dir) = s.dir.clone() else { return };
    s.file = None;
    let path = dir.join("activity.log");
    let _ = fs::rename(&path, dir.join("activity.log.1"));
    s.file = OpenOptions::new().create(true).write(true).truncate(true).open(&path).ok();
    s.size = 0;
}

/// The last `n` activity lines, oldest first.
pub fn tail(n: usize) -> Vec<String> {
    let s = state();
    s.ring.iter().skip(s.ring.len().saturating_sub(n)).cloned().collect()
}

fn crash_path() -> Option<PathBuf> {
    state().dir.as_ref().map(|d| d.join("crash.log"))
}

/// One line in crash.log (and in the activity log).
pub fn crash(text: &str) {
    line("crash", text);
    let Some(path) = crash_path() else { return };
    let text = {
        let s = state();
        scrub(&s, text)
    };
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "{} {}", now_stamp(), text);
    }
}

fn trim_crash_log() {
    let Some(path) = crash_path() else { return };
    let Ok(meta) = fs::metadata(&path) else { return };
    if meta.len() <= CRASH_CAP {
        return;
    }
    if let Ok(all) = fs::read(&path) {
        let keep = &all[all.len() - (CRASH_CAP as usize / 2)..];
        let start = keep.iter().position(|b| *b == b'\n').map(|i| i + 1).unwrap_or(0);
        let _ = fs::write(&path, &keep[start..]);
    }
}

pub fn has_crash_log() -> bool {
    crash_path().and_then(|p| fs::metadata(p).ok()).map(|m| m.len() > 0).unwrap_or(false)
}

/// crash.log's last lines, at most `max` characters.
pub fn crash_tail(max: usize) -> String {
    let Some(path) = crash_path() else { return String::new() };
    let text = fs::read_to_string(&path).unwrap_or_default();
    if text.len() <= max {
        return text;
    }
    let mut start = text.len() - max;
    while !text.is_char_boundary(start) {
        start += 1;
    }
    let cut = &text[start..];
    cut.find('\n').map(|i| cut[i + 1..].to_string()).unwrap_or_else(|| cut.to_string())
}

fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current().name().unwrap_or("unnamed").to_string();
        let what = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "(no message)".into());
        let place = info.location().map(|l| format!(" at {}:{}", l.file(), l.line())).unwrap_or_default();
        crash(&format!("panic in thread {thread}{place}: {what}"));
        previous(info);
    }));
}

/// A message from Qt or QML (installed in main through qtglue.cpp). kind: 0 debug, 1 warning, 2 critical, 3 fatal,
/// 4 info.
pub extern "C" fn qt_message(kind: i32, place: *const std::os::raw::c_char, text: *const std::os::raw::c_char) {
    let read = |p: *const std::os::raw::c_char| {
        if p.is_null() {
            String::new()
        } else {
            // SAFETY: qtglue.cpp passes NUL-terminated UTF-8 that lives for the whole call.
            unsafe { std::ffi::CStr::from_ptr(p) }.to_string_lossy().into_owned()
        }
    };
    let place = read(place);
    let text = read(text);
    let from_qml = place.contains(".qml") || text.contains(".qml:") || place == "qml" || place == "js";
    let label = match kind {
        1 => "warning",
        2 => "critical",
        3 => "fatal",
        _ if from_qml => "console",
        _ => return, // Qt's own chatter stays out of the log
    };
    let msg = if place.is_empty() { format!("{label}: {text}") } else { format!("{label}: {text} ({place})") };
    if kind == 3 {
        crash(&format!("Qt {msg}"));
    } else {
        line("qt", &msg);
    }
}

// ---------------------------------------------------------------------------------------------------- local time

#[repr(C)]
struct Tm {
    sec: i32,
    min: i32,
    hour: i32,
    mday: i32,
    mon: i32,
    year: i32,
    wday: i32,
    yday: i32,
    isdst: i32,
    gmtoff: i64,
    zone: *const std::os::raw::c_char,
}

extern "C" {
    fn localtime_r(t: *const i64, out: *mut Tm) -> *mut Tm;
}

/// Local date and time: (year, month, day, hour, minute, second, millisecond).
pub fn local_now() -> (i32, i32, i32, i32, i32, i32, u32) {
    let d = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = d.as_secs() as i64;
    let mut tm = Tm { sec: 0, min: 0, hour: 0, mday: 1, mon: 0, year: 70, wday: 0, yday: 0, isdst: 0, gmtoff: 0, zone: std::ptr::null() };
    // SAFETY: both pointers are valid for the call; glibc's struct tm on 64-bit Linux matches Tm.
    let ok = !unsafe { localtime_r(&secs, &mut tm) }.is_null();
    if !ok {
        return (1970, 1, 1, 0, 0, 0, 0);
    }
    (tm.year + 1900, tm.mon + 1, tm.mday, tm.hour, tm.min, tm.sec, d.subsec_millis())
}

fn now_stamp() -> String {
    let (y, mo, d, h, mi, s, ms) = local_now();
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}.{ms:03}")
}

/// For file names: 2026-11-06_14-03-22.
pub fn file_stamp() -> String {
    let (y, mo, d, h, mi, s, _) = local_now();
    format!("{y:04}-{mo:02}-{d:02}_{h:02}-{mi:02}-{s:02}")
}

/// Where activity.log and crash.log live.
pub fn folder(data_home: &Path) -> PathBuf {
    data_home.join("turbine-mod-manager")
}

#[cfg(test)]
mod tests {
    #[test]
    fn hides_secrets_and_keeps_lines() {
        super::secret("abcdef123456");
        super::line("test", "key is abcdef123456\nnext");
        let t = super::tail(1);
        assert!(t[0].contains("[hidden]") && !t[0].contains("abcdef") && !t[0].contains('\n'));
    }

    #[test]
    fn local_time_is_sane() {
        let (y, mo, d, ..) = super::local_now();
        assert!(y >= 2024 && (1..=12).contains(&mo) && (1..=31).contains(&d));
    }
}
