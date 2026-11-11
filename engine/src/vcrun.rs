//! 0.64: Microsoft's Visual C++ 2022 runtime in a game's Proton prefix.
//!
//! Cyberpunk 2077's RED4ext.dll and Cyber Engine Tweaks' cyber_engine_tweaks.asi stop at load with Windows error 998
//! (ERROR_NOACCESS) when the prefix has no real Visual C++ 2022 runtime: Wine's own msvcp140/vcruntime140 and the
//! 2019 runtime (14.29) Steam installs at a game's first start are not enough (wiki.redmodding.org, "Modding on
//! Linux", Troubleshooting, Error 998; github.com/wopss/RED4ext issue 76). The owner's device confirmed the fix
//! (`protontricks 1091500 --force vcrun2022`) on 2026-11-10.
//!
//! What winetricks' vcrun2022 does, and so what Turbine does the same way:
//! - downloads Microsoft's installer from https://aka.ms/vs/17/release/vc_redist.x64.exe (Microsoft's own short link;
//!   it redirects to download.visualstudio.microsoft.com);
//! - sets the DLL overrides "native,builtin" for the runtime's DLLs (concrt140, msvcp140 and its parts, vcamp140,
//!   vccorlib140, vcomp140, vcruntime140, vcruntime140_1): without them Wine keeps loading its own copies even after
//!   the installer has put Microsoft's in system32;
//! - runs the installer in the prefix, unattended.
//!
//! winetricks pins a SHA-256 for the installer, which goes stale whenever Microsoft updates it. Turbine instead reads
//! the SHA-256 Microsoft's download server names in the file's own address
//! (download.visualstudio.microsoft.com/download/pr/<id>/<SHA-256>/VC_redist.x64.exe) and checks the file against
//! it; a file whose address names no SHA-256 is refused.
//!
//! Turbine runs it as `<the runtime Proton needs> <proton> runinprefix vc_redist.x64.exe /install /quiet /norestart`
//! with STEAM_COMPAT_DATA_PATH set to the game's compatdata folder, on the host, with fixed arguments.

use crate::net::{self, Client, Download, Expect, NetError, Progress, Request};
use crate::proton;
use std::fs;
use std::path::{Path, PathBuf};

pub const REDIST_URL: &str = "https://aka.ms/vs/17/release/vc_redist.x64.exe";
/// The installer is about 25 MB; anything far bigger is stopped.
pub const MAX_REDIST: u64 = 64 << 20;
/// The DLLs winetricks' vcrun2022 sets to "native,builtin".
pub const DLLS: &[&str] = &[
    "concrt140",
    "msvcp140",
    "msvcp140_1",
    "msvcp140_2",
    "msvcp140_atomic_wait",
    "msvcp140_codecvt_ids",
    "vcamp140",
    "vccorlib140",
    "vcomp140",
    "vcruntime140",
    "vcruntime140_1",
];
/// The 2022 runtime's files are version 14.30 and up (14.29 is the 2019 one).
pub const FIRST_2022: (u16, u16) = (14, 30);

/// Said before Play when it is missing.
pub const CHECK_TITLE: &str = "The Visual C++ runtime is missing";

/// What the prefix has.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// Microsoft's 2022 runtime, and Wine told to use it.
    Ready(String),
    /// No vcruntime140.dll in system32.
    Missing,
    /// Only Wine's own copy.
    WineOnly,
    /// Microsoft's, but older than 2022 (Steam's 2019 one).
    Old(String),
    /// Microsoft's 2022 runtime is there, but Wine still loads its own.
    NotUsed(String),
    /// No prefix yet (the game never started).
    NoPrefix,
}

impl State {
    pub fn needs_install(&self) -> bool {
        !matches!(self, State::Ready(_) | State::NoPrefix)
    }

    /// One plain sentence for the log and the check's detail.
    pub fn words(&self) -> String {
        match self {
            State::Ready(v) => format!("Microsoft's Visual C++ runtime {v} is installed for this game."),
            State::Missing => "This game's Proton setup has no Visual C++ runtime.".into(),
            State::WineOnly => "This game's Proton setup has only Wine's stand-in for the Visual C++ runtime.".into(),
            State::Old(v) => format!("This game's Proton setup has an older Visual C++ runtime ({v}), not the 2022 one."),
            State::NotUsed(v) => format!("Microsoft's Visual C++ runtime {v} is there, but Proton is set to use its own copy."),
            State::NoPrefix => "The game hasn't started through Proton yet.".into(),
        }
    }
}

/// A PE file's VS_FIXEDFILEINFO (signature 0xFEEF04BD): its structure version and the file version's two words
/// (most and least significant).
pub(crate) fn fixed_file_info(bytes: &[u8]) -> Option<(u32, u32, u32)> {
    let sig = [0xBD, 0x04, 0xEF, 0xFE];
    let at = bytes.windows(4).position(|w| w == sig)?;
    let word = |o: usize| -> Option<u32> { bytes.get(at + o..at + o + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])) };
    Some((word(4)?, word(8)?, word(12)?))
}

/// A PE file's version from its VS_FIXEDFILEINFO: (major, minor, build, revision).
pub fn dll_version(bytes: &[u8]) -> Option<(u16, u16, u16, u16)> {
    let (_, ms, ls) = fixed_file_info(bytes)?;
    Some(((ms >> 16) as u16, (ms & 0xFFFF) as u16, (ls >> 16) as u16, (ls & 0xFFFF) as u16))
}

/// Wine marks its own DLLs (and the placeholders it puts in system32) in the DOS stub.
pub fn is_wine_dll(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(4096)];
    [&b"Wine builtin DLL"[..], &b"Wine placeholder DLL"[..]].iter().any(|m| head.windows(m.len()).any(|w| w == *m))
}

/// What a prefix has (`prefix` is the pfx folder).
pub fn state(prefix: Option<&Path>) -> State {
    let Some(prefix) = prefix.filter(|p| p.join("drive_c").is_dir()) else { return State::NoPrefix };
    let dll = prefix.join("drive_c/windows/system32/vcruntime140.dll");
    let Ok(bytes) = fs::read(&dll) else { return State::Missing };
    if is_wine_dll(&bytes) {
        return State::WineOnly;
    }
    let Some(v) = dll_version(&bytes) else { return State::Missing };
    let words = format!("{}.{}.{}", v.0, v.1, v.2);
    if (v.0, v.1) < FIRST_2022 {
        return State::Old(words);
    }
    let used = ["vcruntime140", "msvcp140"].iter().all(|d| {
        proton::read_override(prefix, d).ok().flatten().is_some_and(|o| o.replace(' ', "").to_ascii_lowercase().starts_with("native"))
    });
    if used {
        State::Ready(words)
    } else {
        State::NotUsed(words)
    }
}

/// The SHA-256 Microsoft's download server names in a file's address (a path part of 64 hex digits).
pub fn sha_from_url(url: &str) -> Option<String> {
    let path = url.split(['?', '#']).next()?;
    path.split('/').find(|p| p.len() == 64 && crate::sha256::is_hex_digest(p)).map(str::to_ascii_lowercase)
}

/// Downloads the installer and checks it against the SHA-256 in the address Microsoft sent it from.
pub fn download(client: &Client, progress: Option<Progress>) -> Result<Download, NetError> {
    let resp = client.send(&Request::get(REDIST_URL).max_bytes(MAX_REDIST).max_time(0), progress)?;
    let Some(sha) = sha_from_url(&resp.effective_url) else {
        let _ = fs::remove_file(&resp.body);
        return Err(NetError::Failed(
            "Microsoft's download didn't say its checksum, so Turbine didn't use it. Try again later.".into(),
        ));
    };
    let d = net::verify_file(&resp.body, &Expect { max_bytes: MAX_REDIST, size: None, sha256: Some(sha) }).inspect_err(|_| {
        let _ = fs::remove_file(&resp.body);
    })?;
    let head = fs::read(&d.path).map(|b| b.into_iter().take(2).collect::<Vec<u8>>()).unwrap_or_default();
    if head != b"MZ" {
        let _ = fs::remove_file(&d.path);
        return Err(NetError::Mismatch("it isn't a Windows installer".into()));
    }
    Ok(d)
}

/// The value of `"key" "value"` on a line of a Valve key-value file.
fn value_of(text: &str, key: &str) -> Option<String> {
    let want = format!("\"{key}\"");
    text.lines().find_map(|l| {
        let l = l.trim();
        let rest = l.strip_prefix(&want)?.trim();
        Some(rest.strip_prefix('"')?.strip_suffix('"')?.to_string())
    })
}

/// The Proton to run it with: the one Turbine has on record for the game, else the newest numbered Proton in
/// Steam's libraries, else Proton Experimental.
pub fn find_proton(libs: &[PathBuf], recorded: Option<&Path>) -> Option<PathBuf> {
    if let Some(r) = recorded.filter(|r| crate::compat_tool::is_proton(r)) {
        return Some(r.to_path_buf());
    }
    let mut numbered: Vec<(Vec<u32>, PathBuf)> = Vec::new();
    for l in libs {
        let Ok(rd) = fs::read_dir(l.join("steamapps/common")) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let Some(num) = name.strip_prefix("Proton ") else { continue };
            if !num.chars().all(|c| c.is_ascii_digit() || c == '.') || !crate::compat_tool::is_proton(&e.path()) {
                continue;
            }
            numbered.push((num.split('.').filter_map(|p| p.parse().ok()).collect(), e.path()));
        }
    }
    numbered.sort();
    if let Some((_, p)) = numbered.pop() {
        return Some(p);
    }
    libs.iter().map(|l| l.join("steamapps/common/Proton - Experimental")).find(|p| crate::compat_tool::is_proton(p))
}

/// The command of the Steam Linux Runtime a Proton names (`require_tool_appid` in its toolmanifest), for the verb
/// runinprefix. Some(empty) when the Proton needs none; None when the one it needs isn't installed.
pub fn runtime_command(libs: &[PathBuf], proton: &Path) -> Option<Vec<String>> {
    let manifest = fs::read_to_string(proton.join("toolmanifest.vdf")).unwrap_or_default();
    let Some(need) = value_of(&manifest, "require_tool_appid") else { return Some(Vec::new()) };
    for l in libs {
        let Ok(acf) = fs::read_to_string(l.join(format!("steamapps/appmanifest_{need}.acf"))) else { continue };
        let Some(dir) = value_of(&acf, "installdir") else { continue };
        let runtime = l.join("steamapps/common").join(dir);
        let Ok(rt) = fs::read_to_string(runtime.join("toolmanifest.vdf")) else { continue };
        let line = value_of(&rt, "commandline").unwrap_or_else(|| "/_v2-entry-point --verb=%verb% --".into()).replace("%verb%", "runinprefix");
        let mut parts: Vec<String> = line.split_whitespace().map(str::to_string).collect();
        if parts.is_empty() {
            continue;
        }
        parts[0] = format!("{}{}", runtime.display(), parts[0]);
        if Path::new(&parts[0]).is_file() {
            return Some(parts);
        }
    }
    None
}

/// The whole command, run on the host (never through a shell). `compatdata` is the pfx folder's parent.
pub fn install_args(compatdata: &Path, steam_root: &Path, runtime: &[String], proton: &Path, installer: &Path) -> Vec<String> {
    let mut a: Vec<String> = vec![
        "flatpak-spawn".into(),
        "--host".into(),
        format!("--env=STEAM_COMPAT_DATA_PATH={}", compatdata.display()),
        format!("--env=STEAM_COMPAT_CLIENT_INSTALL_PATH={}", steam_root.display()),
        "--env=WINEDEBUG=-all".into(),
    ];
    a.extend(runtime.iter().cloned());
    a.push(proton.join("proton").display().to_string());
    a.push("runinprefix".into());
    a.push(installer.display().to_string());
    a.extend(["/install", "/quiet", "/norestart"].map(String::from));
    a
}

/// The installer's own exit codes that mean it is in: 0, 3010 (a restart would finish it; Wine needs none) and
/// 1638 (this version or a newer one is installed already).
pub fn installer_ok(code: i32) -> bool {
    matches!(code, 0 | 3010 | 1638)
}

/// Plain words for an installer exit code that isn't a success.
pub fn installer_words(code: i32) -> String {
    match code {
        1618 => "Another installation was running in this game's Proton setup. Close the game and try again.".into(),
        1602 | 1223 => "The installer was stopped before it finished. Try again.".into(),
        c => format!("Microsoft's installer stopped with code {c}. The details are in the activity log."),
    }
}

/// Tells Wine to use Microsoft's runtime (set before the installer runs, so the prefix keeps it when it closes).
pub fn set_overrides(prefix: &Path) -> Result<Vec<String>, String> {
    let wanted: Vec<(&str, &str)> = DLLS.iter().map(|d| (*d, proton::NATIVE_FIRST)).collect();
    proton::ensure_overrides(prefix, &wanted)
}
