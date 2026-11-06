//! Labs mode: the bug report Turbine writes by itself, saving it (and its screenshot) to Downloads, and the numbers
//! for Labs > Performance check.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Instant;
use turbine_engine::store::Store;
use turbine_engine::Game;

/// The commit this build was made from (the build writes app/build-commit.txt), or "dev".
pub const COMMIT: &str = match option_env!("TURBINE_COMMIT") {
    Some(v) => v,
    None => "dev",
};

/// When Turbine started (set first thing in main).
pub fn started() -> Instant {
    static AT: OnceLock<Instant> = OnceLock::new();
    *AT.get_or_init(Instant::now)
}

fn read_trim(path: &str) -> String {
    fs::read_to_string(path).map(|s| s.trim().to_string()).unwrap_or_default()
}

/// "Jupiter (Valve)", from the firmware's own names.
pub fn device() -> String {
    let product = read_trim("/sys/devices/virtual/dmi/id/product_name");
    let vendor = read_trim("/sys/devices/virtual/dmi/id/sys_vendor");
    match (product.is_empty(), vendor.is_empty()) {
        (true, true) => "Not reported".into(),
        (false, true) => product,
        (true, false) => vendor,
        (false, false) => format!("{product} ({vendor})"),
    }
}

/// The host system's name and version from its os-release (the Flatpak's own /etc/os-release is the runtime's).
pub fn system() -> String {
    for path in ["/run/host/os-release", "/etc/os-release"] {
        if let Ok(text) = fs::read_to_string(path) {
            let get = |k: &str| {
                text.lines()
                    .find_map(|l| l.trim().strip_prefix(&format!("{k}=")).map(|v| v.trim_matches('"').to_string()))
                    .unwrap_or_default()
            };
            let mut s = get("PRETTY_NAME");
            if s.is_empty() {
                s = get("NAME");
            }
            for (k, label) in [("VERSION_ID", "version"), ("BUILD_ID", "build"), ("VARIANT_ID", "variant")] {
                let v = get(k);
                if !v.is_empty() {
                    s.push_str(&format!(", {label} {v}"));
                }
            }
            if path == "/etc/os-release" {
                s.push_str(" (the host's own wasn't readable)");
            }
            return s;
        }
    }
    "Not reported".into()
}

/// Processor time Turbine has used, in milliseconds (user and system, from /proc/self/stat).
pub fn cpu_ms() -> f64 {
    let Ok(stat) = fs::read_to_string("/proc/self/stat") else { return 0.0 };
    // Fields after the command name (which sits in parentheses and may hold spaces).
    let Some(rest) = stat.rfind(')').map(|i| &stat[i + 2..]) else { return 0.0 };
    let f: Vec<&str> = rest.split_whitespace().collect();
    // utime and stime are fields 14 and 15 of the whole line, so 11 and 12 here; clock ticks are 100 a second on Linux.
    let ticks = |i: usize| f.get(i).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    (ticks(11) + ticks(12)) * 10.0
}

/// Memory in use (resident), in MB.
pub fn memory_mb() -> f64 {
    let Ok(status) = fs::read_to_string("/proc/self/status") else { return 0.0 };
    status
        .lines()
        .find_map(|l| l.strip_prefix("VmRSS:"))
        .and_then(|v| v.split_whitespace().next())
        .and_then(|v| v.parse::<f64>().ok())
        .map(|kb| kb / 1024.0)
        .unwrap_or(0.0)
}

/// How many processor threads this machine has.
pub fn threads() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
}

/// Downloads, where reports and screenshots go.
pub fn downloads() -> PathBuf {
    turbine_engine::home_dir().join("Downloads")
}

/// Where the report screen's screenshot waits until Send (Turbine's cache folder).
pub fn shot_temp() -> PathBuf {
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| turbine_engine::home_dir().join(".cache"));
    let dir = cache.join("turbine-mod-manager");
    let _ = fs::create_dir_all(&dir);
    dir.join("report-screenshot.png")
}

fn runtime_words(g: &Game) -> String {
    format!("{:?}", g.runtime)
}

/// A game in a few lines.
pub fn game_lines(g: &Game) -> Vec<String> {
    vec![
        format!("Name: {} ({})", g.name, g.id()),
        format!("Store: {} through {}", g.store.label(), g.store.launcher()),
        format!("Runs as: {}", runtime_words(g)),
        format!("Stored on: {} ({})", g.drive_label(), g.library.display()),
        format!("Folder: {}", g.install_dir.display()),
        format!("Build: {}", g.build.as_deref().unwrap_or("not reported")),
        format!("Proton prefix: {}", g.prefix.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "none yet".into())),
        format!(
            "Art: cover {}, header {}",
            if g.cover.is_some() { "yes" } else { "no" },
            if g.header.is_some() { "yes" } else { "no" }
        ),
    ]
}

/// The store's mods in a few lines (at most 80 listed).
pub fn mods_lines(store: &Store) -> Vec<String> {
    let mods = store.mods();
    let on = mods.iter().filter(|m| m.enabled).count();
    let mut out = vec![format!(
        "{} installed, {} on, {} in Recently removed, {} saved setups, last build seen {}",
        mods.len(),
        on,
        store.removed().len(),
        store.setups().len(),
        store.last_build().unwrap_or_else(|| "none".into())
    )];
    for (i, m) in mods.iter().take(80).enumerate() {
        out.push(format!(
            "{:>3}. [{}] {}{} ({}, {} files{})",
            i + 1,
            if m.enabled { "on " } else { "off" },
            m.name,
            m.version.as_deref().map(|v| format!(" {v}")).unwrap_or_default(),
            m.source,
            m.files,
            if m.needs_loader { ", needs the loader" } else { "" }
        ));
    }
    if mods.len() > 80 {
        out.push(format!("... and {} more", mods.len() - 80));
    }
    out
}

/// Everything that goes into a report besides what Rust reads itself.
pub struct ReportInput {
    /// What the report is about (a Labs checklist item: title, how to try it, what should happen); may be empty.
    pub item: String,
    /// The player's own words.
    pub note: String,
    /// From the interface: page, focus, window size and scale, theme settings.
    pub interface: String,
    pub version: String,
    pub gamemode: bool,
    pub pads: bool,
    pub game: Vec<String>,
    pub mods: Vec<String>,
    pub last_report: String,
    pub problems: Vec<String>,
    pub perf: String,
    pub crash: String,
    pub shot: String,
}

fn section(out: &mut String, title: &str, body: &str) {
    out.push_str("\n== ");
    out.push_str(title);
    out.push_str(" ==\n");
    let body = body.trim_end();
    out.push_str(if body.is_empty() { "(nothing)" } else { body });
    out.push('\n');
}

/// The plain-text report.
pub fn build(r: &ReportInput) -> String {
    let (y, mo, d, h, mi, s, _) = crate::log::local_now();
    let mut out = format!("Turbine Mod Manager report\nMade {y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}\n");
    if !r.item.trim().is_empty() {
        section(&mut out, "What this is about", &r.item);
    }
    section(&mut out, "What went wrong (the player's words)", if r.note.trim().is_empty() { "(no note)" } else { &r.note });
    let turbine = format!(
        "Version: {} (build {})\nScreenshot: {}\nRunning for: {} s\nMemory: {:.0} MB",
        r.version,
        COMMIT,
        if r.shot.is_empty() { "none" } else { &r.shot },
        started().elapsed().as_secs(),
        memory_mb()
    );
    section(&mut out, "Turbine", &turbine);
    let dev = format!(
        "Device: {}\nSystem: {}\nMode: {}\nController: {}\nProcessor threads: {}",
        device(),
        system(),
        if r.gamemode { "Game Mode" } else { "Desktop" },
        if r.pads { "connected" } else { "none found" },
        threads()
    );
    section(&mut out, "Device", &dev);
    section(&mut out, "Interface", &r.interface);
    section(&mut out, "Current game", &r.game.join("\n"));
    section(&mut out, "Mods", &r.mods.join("\n"));
    section(&mut out, "Last engine report", &r.last_report);
    section(&mut out, "Recent problems", &r.problems.join("\n"));
    if !r.perf.trim().is_empty() {
        section(&mut out, "Performance check", &r.perf);
    }
    if !r.crash.trim().is_empty() {
        section(&mut out, "Crash log (last lines)", &r.crash);
    }
    section(&mut out, "Activity (last 300 lines)", &crate::log::tail(300).join("\n"));
    out
}

/// Saves the report (and copies the screenshot when there is one) into Downloads. Returns the names written.
pub fn save(text: &str, stamp: &str, shot_from: Option<&Path>) -> Result<(String, String), String> {
    let dir = downloads();
    fs::create_dir_all(&dir).map_err(|e| format!("Couldn't open Downloads: {e}"))?;
    let shot_name = match shot_from {
        Some(from) => {
            let name = format!("Turbine-screenshot-{stamp}.png");
            fs::copy(from, dir.join(&name)).map_err(|e| format!("Couldn't save the screenshot: {e}"))?;
            name
        }
        None => String::new(),
    };
    let name = format!("Turbine-report-{stamp}.txt");
    fs::write(dir.join(&name), text).map_err(|e| format!("Couldn't save the report: {e}"))?;
    Ok((name, shot_name))
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_processor_and_memory() {
        assert!(super::memory_mb() > 0.0);
        assert!(super::cpu_ms() >= 0.0);
    }

    #[test]
    fn builds_a_report() {
        let r = super::ReportInput {
            item: "Item: Play original".into(),
            note: String::new(),
            interface: "Page: game".into(),
            version: "0.2".into(),
            gamemode: true,
            pads: false,
            game: vec![],
            mods: vec![],
            last_report: String::new(),
            problems: vec![],
            perf: String::new(),
            crash: String::new(),
            shot: "Turbine-screenshot-x.png".into(),
        };
        let t = super::build(&r);
        assert!(t.contains("== What this is about ==\nItem: Play original"));
        assert!(t.contains("(no note)") && t.contains("Game Mode") && t.contains("Turbine-screenshot-x.png"));
        assert!(!t.contains("Performance check"));
    }
}
