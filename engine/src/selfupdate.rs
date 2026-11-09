//! Turbine's own updates from its GitHub releases: find the newest release (pre-releases only when Labs mode opts
//! in), compare one-dot versions, download TurbineModManager.flatpak and check its size and SHA-256. Installing is
//! done by the app with `install_args`.

use crate::json::{self, Json};
use crate::net::{Client, Download, Expect, NetError, Progress, Request};
use crate::sha256;
use std::cmp::Ordering;
use std::path::Path;

pub const REPO: &str = "Ravenor907/Turbine-Mod-Manager";
pub const LATEST_URL: &str = "https://api.github.com/repos/Ravenor907/Turbine-Mod-Manager/releases/latest";
pub const LIST_URL: &str = "https://api.github.com/repos/Ravenor907/Turbine-Mod-Manager/releases?per_page=30";
pub const ASSET: &str = "TurbineModManager.flatpak";
/// No Turbine bundle comes near this; anything bigger is stopped.
pub const MAX_BUNDLE: u64 = 1 << 30;

/// Compares Turbine versions written with one dot ("0.3", "0.31", "1.0"; a leading "v" is ignored). The part after
/// the dot is read as decimal digits, so 0.3 < 0.31 < 0.4 < 1.0 (0.31 is "thirty-one hundredths", above 0.3 and
/// below 0.4). None if either isn't a one-dot version.
pub fn compare(a: &str, b: &str) -> Option<Ordering> {
    let (wa, fa) = parse_version(a)?;
    let (wb, fb) = parse_version(b)?;
    Some(wa.cmp(&wb).then_with(|| {
        // Compare fractions digit by digit, padding the shorter with zeros ("3" vs "31" is "30" vs "31").
        let len = fa.len().max(fb.len());
        let pad = |f: &str| format!("{f:0<len$}");
        pad(fa).cmp(&pad(fb))
    }))
}

/// Splits "v0.31" into (0, "31"). Trailing zeros of the fraction are dropped so "0.30" equals "0.3".
fn parse_version(v: &str) -> Option<(u64, &str)> {
    let v = v.trim();
    let v = v.strip_prefix(['v', 'V']).unwrap_or(v);
    let (whole, frac) = v.split_once('.').unwrap_or((v, ""));
    if whole.is_empty() || whole.len() > 9 || !whole.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if frac.len() > 9 || !frac.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((whole.parse().ok()?, frac.trim_end_matches('0')))
}

/// Is `candidate` newer than `current`?
pub fn is_newer(candidate: &str, current: &str) -> bool {
    compare(candidate, current) == Some(Ordering::Greater)
}

/// A release that can be installed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    /// "0.31" (the tag without its "v").
    pub version: String,
    pub tag: String,
    pub name: String,
    pub prerelease: bool,
    pub notes: String,
    pub asset_url: String,
    pub asset_size: u64,
    /// SHA-256 from GitHub's asset digest or the release notes, when present.
    pub sha256: Option<String>,
    /// Address of a TurbineModManager.flatpak.sha256 asset, when the release has one.
    pub sha256_asset_url: Option<String>,
    pub page_url: String,
}

/// Finds a SHA-256 for the bundle in release notes: a 64 hex digit run on a line that names the bundle or says
/// SHA-256, or the only such run in the notes.
pub fn sha256_from_notes(notes: &str) -> Option<String> {
    let runs = |line: &str| -> Vec<String> {
        line.split(|c: char| !c.is_ascii_hexdigit()).filter(|w| w.len() == 64).map(|w| w.to_ascii_lowercase()).collect()
    };
    for line in notes.lines() {
        let l = line.to_ascii_lowercase();
        if l.contains("sha256") || l.contains("sha-256") || l.contains(&ASSET.to_ascii_lowercase()) {
            if let Some(h) = runs(line).into_iter().next() {
                return Some(h);
            }
        }
    }
    let all: Vec<String> = notes.lines().flat_map(runs).collect();
    (all.len() == 1).then(|| all[0].clone())
}

/// Reads the hash out of a .sha256 file ("<hash>  TurbineModManager.flatpak" or just the hash).
pub fn parse_sha256_file(text: &str) -> Option<String> {
    let first = text.split_whitespace().next()?;
    sha256::is_hex_digest(first).then(|| first.to_ascii_lowercase())
}

/// Reads one release object from GitHub's API. None for drafts, releases without the bundle, or tags that aren't
/// one-dot versions.
pub fn release_from_json(r: &Json) -> Option<Release> {
    if r.get("draft").and_then(Json::as_bool) == Some(true) {
        return None;
    }
    let tag = r.str_at("tag_name")?.to_string();
    let version = tag.trim_start_matches(['v', 'V']).to_string();
    parse_version(&version)?;
    let assets = r.get("assets").map(Json::as_array).unwrap_or(&[]);
    let bundle = assets.iter().find(|a| a.str_at("name") == Some(ASSET))?;
    let asset_url = bundle.str_at("browser_download_url")?.to_string();
    let asset_size = match bundle.get("size") {
        Some(Json::Num(n)) if *n > 0.0 => *n as u64,
        _ => 0,
    };
    let notes = r.str_at("body").unwrap_or("").to_string();
    let digest = bundle
        .str_at("digest")
        .and_then(|d| d.strip_prefix("sha256:"))
        .filter(|h| sha256::is_hex_digest(h))
        .map(str::to_ascii_lowercase);
    let sha_asset = format!("{ASSET}.sha256");
    let sha256_asset_url = assets
        .iter()
        .find(|a| a.str_at("name").is_some_and(|n| n.eq_ignore_ascii_case(&sha_asset)))
        .and_then(|a| a.str_at("browser_download_url"))
        .map(str::to_string);
    Some(Release {
        sha256: digest.or_else(|| sha256_from_notes(&notes)),
        version,
        tag,
        name: r.str_at("name").unwrap_or("").to_string(),
        prerelease: r.get("prerelease").and_then(Json::as_bool).unwrap_or(false),
        notes,
        asset_url,
        asset_size,
        sha256_asset_url,
        page_url: r.str_at("html_url").unwrap_or("").to_string(),
    })
}

/// From GitHub's release list: the newest usable release. Pre-releases count only when `labs` is on.
pub fn pick_release(list_json: &str, labs: bool) -> Result<Option<Release>, String> {
    let j = json::parse(list_json).map_err(|e| format!("GitHub's answer can't be read ({e})"))?;
    let items: Vec<Release> = match &j {
        Json::Arr(a) => a.iter().filter_map(release_from_json).collect(),
        obj @ Json::Obj(_) => release_from_json(obj).into_iter().collect(),
        _ => return Err("GitHub's answer isn't a release list".into()),
    };
    Ok(items
        .into_iter()
        .filter(|r| labs || !r.prerelease)
        .max_by(|a, b| compare(&a.version, &b.version).unwrap_or(Ordering::Equal)))
}

/// What a check found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Check {
    UpToDate,
    Available(Box<Release>),
}

/// Decides from a release list and the running version (pure).
pub fn decide(list_json: &str, current: &str, labs: bool) -> Result<Check, String> {
    Ok(match pick_release(list_json, labs)? {
        Some(r) if is_newer(&r.version, current) => Check::Available(Box::new(r)),
        _ => Check::UpToDate,
    })
}

/// Asks GitHub for the newest release. Without Labs mode only full releases count (the "latest" endpoint, which
/// answers 404 while there are only pre-releases); with Labs mode the release list is read so pre-releases count.
pub fn check(client: &Client, current: &str, labs: bool) -> Result<Check, NetError> {
    let url = if labs { LIST_URL } else { LATEST_URL };
    let req = Request::get(url).header("Accept", "application/vnd.github+json").max_bytes(8 << 20).max_time(30);
    let text = match client.send(&req, None) {
        Ok(r) => r.take_text()?,
        Err(NetError::NotFound) if !labs => return Ok(Check::UpToDate),
        Err(e) => return Err(e),
    };
    decide(&text, current, labs).map_err(|e| NetError::Failed(format!("{e}.")))
}

/// Downloads the bundle to a temp file and checks it: size from the release, SHA-256 from the asset digest, the
/// notes, or a .sha256 asset. A release without any published SHA-256 is refused, so nothing unchecked is installed.
pub fn download(client: &Client, release: &Release, progress: Option<Progress>) -> Result<Download, NetError> {
    let mut sha = release.sha256.clone();
    if sha.is_none() {
        if let Some(url) = &release.sha256_asset_url {
            let text = client.send(&Request::get(url).max_bytes(4096).max_time(30), None)?.take_text()?;
            sha = parse_sha256_file(&text);
        }
    }
    let Some(sha) = sha else {
        return Err(NetError::Failed("This release doesn't publish a SHA-256 for its bundle, so Turbine won't install it.".into()));
    };
    let check = Expect {
        max_bytes: if release.asset_size > 0 { release.asset_size } else { MAX_BUNDLE },
        size: (release.asset_size > 0).then_some(release.asset_size),
        sha256: Some(sha),
    };
    client.download(&release.asset_url, &check, progress)
}

/// The program and arguments that install a downloaded bundle on the host (run by the app, never through a
/// shell). --reinstall lets a bundle replace the installed copy of the same app.
pub fn install_args(bundle: &Path) -> Vec<String> {
    install_args_to(bundle, "--user")
}

/// 0.61 (integration I12): where the running Turbine is installed, from the sandbox's /.flatpak-info
/// (`[Instance] app-path`): "--system" for the system installation (/var/lib/flatpak), "--user" otherwise.
pub fn install_scope(flatpak_info: &str) -> &'static str {
    let mut in_instance = false;
    for line in flatpak_info.lines().map(str::trim) {
        if line.starts_with('[') {
            in_instance = line == "[Instance]";
        } else if let (true, Some(path)) = (in_instance, line.strip_prefix("app-path=")) {
            return if path.trim().starts_with("/var/lib/flatpak/") { "--system" } else { "--user" };
        }
    }
    "--user"
}

/// The same, into the installation `scope` names (install_scope), so an update never adds a second copy.
pub fn install_args_to(bundle: &Path, scope: &str) -> Vec<String> {
    let scope = if scope == "--system" { "--system" } else { "--user" };
    ["flatpak-spawn", "--host", "flatpak", "install", scope, "-y", "--reinstall", "--bundle"]
        .iter()
        .map(|s| s.to_string())
        .chain(std::iter::once(bundle.display().to_string()))
        .collect()
}
