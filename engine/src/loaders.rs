//! Script loaders from their official sources, and how each one goes into a game.
//!
//! - SKSE: skse.silverlock.org lists the current builds. The GOG build is a .7z there; the Steam Anniversary Edition
//!   build is now published only on Nexus (the page links to Nexus mod 30379), so it comes through the Nexus module
//!   (Premium) or, for free accounts, a plain "download it in Desktop Mode" message. A .7z is unpacked by the
//!   archive module (bsdtar); this module returns the downloaded file.
//! - BepInEx: the BepInExPack for each game from Thunderstore.
//! - SMAPI: the latest release zip from GitHub (Pathoschild/SMAPI).
//!
//! Nothing from these archives is run by Turbine. Each `InstallPlan` is data for the install step.

use crate::json::{self, Json};
use crate::net::{self, Client, Download, Expect, NetError, Progress, Request, SourceInfo};
use crate::nexus::{Account, Nexus};
use crate::sha256;
use crate::thunderstore::{self, Package};

pub const SKSE_PAGE: &str = "https://skse.silverlock.org/";
pub const SKSE_NEXUS_DOMAIN: &str = "skyrimspecialedition";
pub const SKSE_NEXUS_MOD: u64 = 30379;
pub const SMAPI_LATEST: &str = "https://api.github.com/repos/Pathoschild/SMAPI/releases/latest";

/// Known SHA-256 of loader files by file name. A download whose name is listed must match. Files not listed are
/// accepted after the size checks, and their hash is recorded in SourceInfo (and `LoaderDownload::pinned` is false)
/// so the app can log that the file wasn't pinned. SMAPI's hash comes from GitHub's published asset digest.
pub const KNOWN_SHA256: &[(&str, &str)] = &[];

pub fn known_sha256(file_name: &str) -> Option<&'static str> {
    KNOWN_SHA256.iter().find(|(n, _)| n.eq_ignore_ascii_case(file_name)).map(|(_, h)| *h)
}

// ---- Install plans (data for the install step) ----

/// How a loader hooks into the game once its files are in place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Hook {
    /// Put `replacement` where `original` is, keeping the original in the backup folder (Steam starts `original`).
    LauncherSwap { original: String, replacement: String },
    /// Proton/Wine must load the game folder's copy of this DLL first (WINEDLLOVERRIDES "<dll>=n,b").
    DllOverride { dll: String },
    /// A Steam Launch Option the player sets once, shown with a guide.
    LaunchOption { text: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallPlan {
    /// "skse", "bepinex" or "smapi".
    pub loader: String,
    /// The download holds another archive with the real files (SMAPI's installer zip holds install.dat); unpack
    /// this path from it first, then use that.
    pub inner_archive: Option<String>,
    /// Drop this many leading folders from every path (the top folder of the archive).
    pub strip_folders: usize,
    /// Only take files under this folder (after stripping), e.g. "BepInExPack". Empty means everything.
    pub take_from: String,
    /// Paths (after stripping) to leave out, compared case-insensitively as prefixes.
    pub skip: Vec<String>,
    /// Files go to this folder relative to the game folder ("" is the game folder itself).
    pub target: String,
    /// Copies inside the game folder after placing files: (from, to).
    pub copies: Vec<(String, String)>,
    /// Folders to create in the game folder.
    pub make_dirs: Vec<String>,
    /// Files to mark executable (native Linux launchers).
    pub executable: Vec<String>,
    pub hooks: Vec<Hook>,
    /// Plain notes for the activity log or a guide.
    pub notes: Vec<String>,
}

impl InstallPlan {
    fn new(loader: &str) -> InstallPlan {
        InstallPlan {
            loader: loader.into(),
            inner_archive: None,
            strip_folders: 0,
            take_from: String::new(),
            skip: Vec::new(),
            target: String::new(),
            copies: Vec::new(),
            make_dirs: Vec::new(),
            executable: Vec::new(),
            hooks: Vec::new(),
            notes: Vec::new(),
        }
    }

    /// Where an archive path goes inside the game folder, or None if it's left out. Pure, for the install step.
    pub fn map_path(&self, archive_path: &str) -> Option<String> {
        let clean = archive_path.replace('\\', "/");
        let parts: Vec<&str> = clean.split('/').filter(|p| !p.is_empty() && *p != ".").collect();
        if parts.len() <= self.strip_folders {
            return None;
        }
        let mut rest = parts[self.strip_folders..].join("/");
        if !self.take_from.is_empty() {
            let pre = format!("{}/", self.take_from);
            if rest.len() <= pre.len() || !rest.to_lowercase().starts_with(&pre.to_lowercase()) {
                return None;
            }
            rest = rest[pre.len()..].to_string();
        }
        let low = rest.to_lowercase();
        if self.skip.iter().any(|s| {
            let s = s.to_lowercase();
            low == s || low.starts_with(&format!("{s}/"))
        }) {
            return None;
        }
        Some(if self.target.is_empty() { rest } else { format!("{}/{rest}", self.target) })
    }
}

/// SKSE: the archive's top folder (skse64_x_yy_zz) is dropped; the loader and its DLL go into the game folder and
/// Data/Scripts into Data. The source code and text files are left out. On Steam Deck, Steam starts
/// SkyrimSELauncher.exe, so the SKSE loader takes its place.
pub fn skse_plan() -> InstallPlan {
    let mut p = InstallPlan::new("skse");
    p.strip_folders = 1;
    p.skip = vec!["src".into(), "readme.txt".into(), "skse64_readme.txt".into(), "skse64_whatsnew.txt".into()];
    p.hooks.push(Hook::LauncherSwap { original: "SkyrimSELauncher.exe".into(), replacement: "skse64_loader.exe".into() });
    p.notes.push("SKSE starts in place of Skyrim's launcher; Play without mods puts the launcher back for that run.".into());
    p
}

/// The install plan for a loader published on Nexus, by its Nexus mod: SKSE (skyrimspecialedition 30379) whether
/// it came through Mod Manager Download or a Slow download picked up from Downloads. rules::GameRules::map_archive
/// lays SKSE's archive out with this same plan.
pub fn plan_for_nexus(domain: &str, mod_id: u64) -> Option<InstallPlan> {
    (domain == SKSE_NEXUS_DOMAIN && mod_id == SKSE_NEXUS_MOD).then(skse_plan)
}

/// Which Thunderstore pack holds BepInEx for a community, and the folder inside it with the files.
pub fn bepinex_pack(community: &str) -> Option<(&'static str, &'static str)> {
    match community {
        "valheim" => Some(("denikson-BepInExPack_Valheim", "BepInExPack_Valheim")),
        "lethal-company" => Some(("BepInEx-BepInExPack", "BepInExPack")),
        "riskofrain2" => Some(("bbepis-BepInExPack", "BepInExPack")),
        _ => None,
    }
}

/// BepInEx: the pack's inner folder goes into the game folder. Windows games under Proton need winhttp.dll loaded
/// from the game folder; native Linux builds start through the pack's script instead.
pub fn bepinex_plan(inner_folder: &str, native_linux: bool) -> InstallPlan {
    let mut p = InstallPlan::new("bepinex");
    p.take_from = inner_folder.into();
    if native_linux {
        let script = if inner_folder == "BepInExPack_Valheim" { "start_game_bepinex.sh" } else { "run_bepinex.sh" };
        p.executable.push(script.into());
        p.hooks.push(Hook::LaunchOption { text: format!("./{script} %command%") });
        p.notes.push(format!("This game runs natively, so BepInEx starts through {script} (set once as a Launch Option)."));
    } else {
        p.hooks.push(Hook::DllOverride { dll: "winhttp".into() });
    }
    p
}

/// SMAPI: the release zip holds an installer; the real files are in internal/<linux|windows>/install.dat (a zip).
/// Its contents go into the game folder, the game's deps.json is copied for SMAPI, and Mods is created. On the
/// native Linux build SMAPI's launcher replaces the game's "StardewValley" script (the original is kept); under
/// Proton, Steam needs a Launch Option that starts StardewModdingAPI.exe.
pub fn smapi_plan(installer_folder: &str, native_linux: bool) -> InstallPlan {
    let mut p = InstallPlan::new("smapi");
    let platform = if native_linux { "linux" } else { "windows" };
    p.inner_archive = Some(format!("{installer_folder}/internal/{platform}/install.dat"));
    p.copies.push(("Stardew Valley.deps.json".into(), "StardewModdingAPI.deps.json".into()));
    p.make_dirs.push("Mods".into());
    if native_linux {
        p.hooks.push(Hook::LauncherSwap { original: "StardewValley".into(), replacement: "unix-launcher.sh".into() });
        p.executable.extend(["StardewValley".into(), "StardewModdingAPI".into()]);
    } else {
        // {game_dir} is filled in by the app with the game's folder.
        p.hooks.push(Hook::LaunchOption { text: "\"{game_dir}/StardewModdingAPI.exe\" %command%".into() });
        p.notes.push("Under Proton, Steam needs a Launch Option so it starts StardewModdingAPI.exe.".into());
    }
    p
}

// ---- SKSE page ----

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkseEdition {
    /// The current Steam release (Anniversary Edition, game 1.6.640 and later).
    SteamAe,
    GogAe,
    /// The old 1.5.97 build for downgraded games.
    SpecialEdition,
    Vr,
    Classic,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkseBuild {
    pub edition: SkseEdition,
    /// "2.3.1".
    pub version: String,
    /// The game version it is for, when the page says.
    pub game_version: Option<String>,
    /// The download link that follows the build's line on the page.
    pub link: Option<String>,
}

fn decode_entities(s: &str) -> String {
    s.replace("&nbsp;", " ").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'").replace("&amp;", "&")
}

/// Splits HTML into its text (tags removed) and the links with their position in that text.
fn text_and_links(html: &str) -> (String, Vec<(usize, String)>) {
    let mut text = String::new();
    let mut links = Vec::new();
    let mut rest = html;
    while let Some(lt) = rest.find('<') {
        text.push_str(&decode_entities(&rest[..lt]));
        let Some(gt) = rest[lt..].find('>') else {
            rest = "";
            break;
        };
        let tag = &rest[lt + 1..lt + gt];
        let lower = tag.to_ascii_lowercase();
        if lower.starts_with("a ") || lower.starts_with("a\n") || lower.starts_with("a\t") {
            if let Some(i) = lower.find("href") {
                let after = tag[i + 4..].trim_start();
                if let Some(v) = after.strip_prefix('=') {
                    let v = v.trim_start();
                    let href = match v.chars().next() {
                        Some(q @ ('"' | '\'')) => v[1..].split(q).next().unwrap_or(""),
                        _ => v.split(|c: char| c.is_whitespace()).next().unwrap_or(""),
                    };
                    links.push((text.len(), decode_entities(href)));
                }
            }
        }
        if lower.starts_with("br") || lower.starts_with("p") || lower.starts_with("/p") || lower.starts_with("li") || lower.starts_with("div") {
            text.push('\n');
        }
        rest = &rest[lt + gt + 1..];
    }
    text.push_str(&decode_entities(rest));
    (text, links)
}

/// Makes a page link absolute (relative links resolve against the SKSE site).
fn absolute(href: &str) -> String {
    let h = href.trim();
    if h.starts_with("https://") || h.starts_with("http://") {
        h.to_string()
    } else if let Some(p) = h.strip_prefix('/') {
        format!("{SKSE_PAGE}{p}")
    } else {
        format!("{SKSE_PAGE}{h}")
    }
}

fn version_token(s: &str) -> Option<String> {
    let t: String = s.trim_start().chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    let t = t.trim_end_matches('.').to_string();
    (t.contains('.') && t.split('.').all(|p| !p.is_empty())).then_some(t)
}

/// Reads the SKSE page: every "Current ... build X (game version Y)" line and the first link after it.
pub fn parse_skse_page(html: &str) -> Vec<SkseBuild> {
    let (text, links) = text_and_links(html);
    let lower = text.to_lowercase();
    let starts: Vec<usize> = lower.match_indices("current ").map(|(i, _)| i).collect();
    let mut out = Vec::new();
    for (k, &start) in starts.iter().enumerate() {
        let end = starts.get(k + 1).copied().unwrap_or(text.len());
        let seg = &lower[start..end];
        let Some(b) = seg.find(" build ") else { continue };
        let label = seg["current ".len()..b].trim();
        let edition = if label.contains("gog") {
            SkseEdition::GogAe
        } else if label.contains("anniversary") {
            SkseEdition::SteamAe
        } else if label.contains("special edition") {
            SkseEdition::SpecialEdition
        } else if label.contains("vr") {
            SkseEdition::Vr
        } else if label.contains("classic") {
            SkseEdition::Classic
        } else {
            continue;
        };
        let Some(version) = version_token(&seg[b + " build ".len()..]) else { continue };
        let game_version = seg.find("game version").and_then(|g| version_token(&seg[g + "game version".len()..]));
        let link = links
            .iter()
            .find(|(pos, href)| *pos >= start + b && *pos < end && !href.starts_with('#') && !href.starts_with("mailto:"))
            .map(|(_, href)| absolute(href));
        out.push(SkseBuild { edition, version, game_version, link });
    }
    out
}

/// Where to get SKSE for an edition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SkseSource {
    /// A .7z on the SKSE site.
    Direct { url: String, version: String },
    /// Published on Nexus: download through the Nexus module (Premium) or in a browser.
    Nexus { domain: String, mod_id: u64, version: String },
}

/// Reads "https://www.nexusmods.com/<domain>/mods/<id>..." into (domain, id).
pub fn nexus_mod_link(url: &str) -> Option<(String, u64)> {
    let host = net::https_host(url)?;
    if host != "www.nexusmods.com" && host != "nexusmods.com" {
        return None;
    }
    let path = url.split_once(&host)?.1;
    let parts: Vec<&str> = path.split(['/', '?', '#']).filter(|p| !p.is_empty()).collect();
    match parts.as_slice() {
        [d, "mods", id, ..] => Some((d.to_string(), id.parse().ok()?)),
        _ => None,
    }
}

/// Picks the source for an edition from the parsed page. A direct link must be a .7z on the SKSE site.
pub fn skse_source(builds: &[SkseBuild], edition: SkseEdition) -> Result<SkseSource, String> {
    let b = builds.iter().find(|b| b.edition == edition).ok_or("The SKSE page doesn't list a build for this game")?;
    let link = b.link.as_deref().ok_or("The SKSE page lists the build but no download link")?;
    if let Some((domain, mod_id)) = nexus_mod_link(link) {
        return Ok(SkseSource::Nexus { domain, mod_id, version: b.version.clone() });
    }
    if net::https_host(link).as_deref() == Some("skse.silverlock.org") && link.to_ascii_lowercase().ends_with(".7z") {
        return Ok(SkseSource::Direct { url: link.to_string(), version: b.version.clone() });
    }
    Err(format!("The SKSE page links somewhere Turbine doesn't download from ({link})"))
}

/// Free accounts (or no Nexus key) get this for the Steam build of SKSE.
pub fn skse_browser_message(version: &str) -> String {
    format!(
        "SKSE {version} for the Steam version is published on Nexus. Download it in Desktop Mode's browser from \
nexusmods.com/skyrimspecialedition/mods/30379, then add it from Downloads."
    )
}

/// A downloaded loader, checked, with its record and how to install it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoaderDownload {
    pub download: Download,
    pub source: SourceInfo,
    pub plan: InstallPlan,
    /// True when the file was matched against a SHA-256 published by its source or pinned in Turbine.
    pub pinned: bool,
}

fn loader_source(id: &str, version: &str, url: &str, sha: &str) -> SourceInfo {
    SourceInfo {
        kind: "loader".into(),
        id: id.into(),
        version: Some(version.into()),
        url: Some(url.into()),
        sha256: Some(sha.into()),
        needs: Vec::new(),
        title: None,
        picture: None,
    }
}

fn file_name_of(url: &str) -> &str {
    url.split(['?', '#']).next().unwrap_or(url).trim_end_matches('/').rsplit('/').next().unwrap_or("")
}

/// Downloads SKSE for an edition. `nexus` is the player's Nexus session and account, when they have one.
pub fn fetch_skse(
    client: &Client,
    edition: SkseEdition,
    nexus: Option<(&Nexus, &Account)>,
    progress: Option<Progress>,
) -> Result<LoaderDownload, NetError> {
    let page = client.get_text(SKSE_PAGE, &[])?;
    let builds = parse_skse_page(&page);
    let source = skse_source(&builds, edition).map_err(|e| NetError::Failed(format!("{e}.")))?;
    match source {
        SkseSource::Direct { url, version } => {
            let name = file_name_of(&url).to_string();
            let known = known_sha256(&name);
            let check = Expect { max_bytes: 64 << 20, size: None, sha256: known.map(str::to_string) };
            let d = client.download(&url, &check, progress)?;
            Ok(LoaderDownload { source: loader_source("skse", &version, &url, &d.sha256), download: d, plan: skse_plan(), pinned: known.is_some() })
        }
        SkseSource::Nexus { domain, mod_id, version } => {
            let Some((nx, account)) = nexus else { return Err(NetError::Failed(skse_browser_message(&version))) };
            if !account.is_premium {
                return Err(NetError::Failed(skse_browser_message(&version)));
            }
            let files = nx.files(&domain, mod_id)?;
            let file = files
                .files
                .iter()
                .filter(|f| f.is_current() && f.category == "MAIN")
                .find(|f| f.version == version || f.name.contains(&version))
                .or_else(|| files.main_file())
                .cloned()
                .ok_or_else(|| NetError::Failed("Nexus lists no main file for SKSE.".into()))?;
            let (d, _) = nx.download(account, &domain, mod_id, &file, progress)?;
            if let Some(want) = known_sha256(&file.file_name) {
                if !sha256::same_digest(want, &d.sha256) {
                    let _ = std::fs::remove_file(&d.path);
                    return Err(NetError::Mismatch("its SHA-256 is different".into()));
                }
            }
            let url = crate::nexus::mod_page_url(&domain, mod_id);
            Ok(LoaderDownload {
                source: loader_source("skse", &version, &url, &d.sha256),
                pinned: known_sha256(&file.file_name).is_some(),
                download: d,
                plan: skse_plan(),
            })
        }
    }
}

/// Downloads the BepInExPack for a Thunderstore community from its index (the pack alone, no dependencies).
pub fn fetch_bepinex(
    client: &Client,
    index: &[Package],
    community: &str,
    native_linux: bool,
    progress: Option<Progress>,
) -> Result<LoaderDownload, NetError> {
    let (pack, folder) =
        bepinex_pack(community).ok_or_else(|| NetError::Failed("Turbine doesn't know which BepInEx this game uses.".into()))?;
    let plan = thunderstore::resolve(index, &[pack], &[], false);
    let pick = plan
        .install
        .iter()
        .find(|p| p.full_name.eq_ignore_ascii_case(pack))
        .ok_or_else(|| NetError::Failed(format!("Thunderstore doesn't list {pack} right now.")))?;
    let (d, mut info) = thunderstore::download(client, pick, progress)?;
    let known = known_sha256(&format!("{}-{}.zip", pick.full_name, pick.version));
    if let Some(want) = known {
        if !sha256::same_digest(want, &d.sha256) {
            let _ = std::fs::remove_file(&d.path);
            return Err(NetError::Mismatch("its SHA-256 is different".into()));
        }
    }
    info.kind = "loader".into();
    info.needs.clear();
    Ok(LoaderDownload { download: d, source: info, plan: bepinex_plan(folder, native_linux), pinned: known.is_some() })
}

/// Seamless Co-op's releases (the repository moved from LukeYui to yuiamoroll, which GitHub redirects).
pub const SEAMLESS_LATEST: &str = "https://api.github.com/repos/yuiamoroll/EldenRingSeamlessCoopRelease/releases/latest";

/// Seamless Co-op's newest release as GitHub lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeamlessRelease {
    pub version: String,
    pub url: String,
    pub size: u64,
    /// GitHub's published digest for the file, when present.
    pub sha256: Option<String>,
}

/// Reads GitHub's latest-release answer for Seamless Co-op: its one zip (the first .zip file listed), from GitHub
/// itself. The version is the release's tag without a leading "v".
pub fn parse_seamless_release(text: &str) -> Result<SeamlessRelease, String> {
    let j = json::parse(text).map_err(|e| format!("GitHub's answer can't be read ({e})"))?;
    let tag = j.str_at("tag_name").ok_or("GitHub's answer has no release")?;
    let version = tag.trim().trim_start_matches(['v', 'V']).to_string();
    if version.is_empty() || !version.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-') || !version.as_bytes()[0].is_ascii_digit() {
        return Err(format!("Seamless Co-op's release has a version Turbine can't read ({tag})"));
    }
    for a in j.get("assets").map(Json::as_array).unwrap_or(&[]) {
        let Some(name) = a.str_at("name") else { continue };
        if !name.to_ascii_lowercase().ends_with(".zip") {
            continue;
        }
        let url = a.str_at("browser_download_url").ok_or("GitHub's answer has no download address")?;
        if crate::net::https_host(url).as_deref() != Some("github.com") {
            return Err("Seamless Co-op's release points somewhere other than GitHub".to_string());
        }
        let size = match a.get("size") {
            Some(Json::Num(n)) if *n > 0.0 => *n as u64,
            _ => 0,
        };
        let sha256 = a.str_at("digest").and_then(|d| d.strip_prefix("sha256:")).filter(|h| sha256::is_hex_digest(h)).map(str::to_ascii_lowercase);
        return Ok(SeamlessRelease { version, url: url.to_string(), size, sha256 });
    }
    Err(format!("Seamless Co-op release {tag} has no zip file"))
}

/// What "Get Seamless Co-op" should do, given the version installed (if any) and the newest release Turbine can
/// download itself. GitHub's release can lag behind the mod's Nexus page (0.62: GitHub had 2.0.1 while Nexus had
/// 2.0.2), so a copy that is as new or newer is never touched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SeamlessGet {
    /// Installed and as new or newer than what Turbine can download: nothing to do. The words say so.
    HaveIt(String),
    /// Download this release (not installed, or the installed copy is older).
    Download,
}

pub fn seamless_get(have: Option<&str>, release: &SeamlessRelease) -> SeamlessGet {
    match have {
        Some(h) if crate::updates::compare_versions(h, &release.version) != std::cmp::Ordering::Less => SeamlessGet::HaveIt(if crate::updates::compare_versions(h, &release.version) == std::cmp::Ordering::Equal {
            format!("You have Seamless Co-op {h}, the newest Turbine can download itself. Nothing to do.")
        } else {
            format!("You have Seamless Co-op {h}, which is newer than the {} Turbine can download itself. Nothing to do.", release.version)
        }),
        _ => SeamlessGet::Download,
    }
}

/// The words while Turbine downloads Seamless Co-op itself: which version, and that its Nexus page may be ahead.
pub fn seamless_download_words(release: &SeamlessRelease) -> String {
    format!("Getting Seamless Co-op {} from its GitHub release. Its Nexus page may have a newer one.", release.version)
}

/// Seamless Co-op's newest release as its GitHub repository lists it (no download).
pub fn seamless_latest(client: &Client) -> Result<SeamlessRelease, NetError> {
    let text = client.send(&Request::get(SEAMLESS_LATEST).header("Accept", "application/vnd.github+json").max_bytes(4 << 20), None)?.take_text()?;
    parse_seamless_release(&text).map_err(|e| NetError::Failed(format!("{e}.")))
}

/// Downloads one Seamless Co-op release, checked against the size and digest GitHub lists for it. The record keeps
/// the version, the address and the SHA-256 of what was downloaded.
pub fn fetch_seamless_release(client: &Client, rel: &SeamlessRelease, progress: Option<Progress>) -> Result<(Download, SourceInfo), NetError> {
    let check = Expect { max_bytes: if rel.size > 0 { rel.size } else { 64 << 20 }, size: (rel.size > 0).then_some(rel.size), sha256: rel.sha256.clone() };
    let d = client.download(&rel.url, &check, progress)?;
    let mut source = loader_source("seamless", &rel.version, &rel.url, &d.sha256);
    source.title = Some("Seamless Co-op".into());
    Ok((d, source))
}

/// Downloads Seamless Co-op's newest release (`seamless_latest`, then `fetch_seamless_release`).
pub fn fetch_seamless(client: &Client, progress: Option<Progress>) -> Result<(Download, SourceInfo), NetError> {
    let rel = seamless_latest(client)?;
    fetch_seamless_release(client, &rel, progress)
}

/// SMAPI's newest release as GitHub lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SmapiRelease {
    pub version: String,
    pub url: String,
    pub size: u64,
    /// GitHub's published digest for the asset, when present.
    pub sha256: Option<String>,
    /// "SMAPI 4.1.10 installer": the folder inside the zip.
    pub installer_folder: String,
}

/// Reads GitHub's latest-release answer for SMAPI: the "SMAPI-<version>-installer.zip" asset (not the one for
/// developers).
pub fn parse_smapi_release(text: &str) -> Result<SmapiRelease, String> {
    let j = json::parse(text).map_err(|e| format!("GitHub's answer can't be read ({e})"))?;
    let tag = j.str_at("tag_name").ok_or("GitHub's answer has no release")?;
    for a in j.get("assets").map(Json::as_array).unwrap_or(&[]) {
        let Some(name) = a.str_at("name") else { continue };
        let Some(ver) = name.strip_prefix("SMAPI-").and_then(|r| r.strip_suffix("-installer.zip")) else { continue };
        if ver.is_empty() || !ver.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
            continue;
        }
        let url = a.str_at("browser_download_url").ok_or("GitHub's answer has no download address")?;
        let size = match a.get("size") {
            Some(Json::Num(n)) if *n > 0.0 => *n as u64,
            _ => 0,
        };
        let sha256 = a.str_at("digest").and_then(|d| d.strip_prefix("sha256:")).filter(|h| sha256::is_hex_digest(h)).map(str::to_ascii_lowercase);
        return Ok(SmapiRelease { version: ver.to_string(), url: url.to_string(), size, sha256, installer_folder: format!("SMAPI {ver} installer") });
    }
    Err(format!("SMAPI release {tag} has no installer zip"))
}

/// Downloads the latest SMAPI release zip, checked against GitHub's size and digest.
pub fn fetch_smapi(client: &Client, native_linux: bool, progress: Option<Progress>) -> Result<LoaderDownload, NetError> {
    let text = client.send(&Request::get(SMAPI_LATEST).header("Accept", "application/vnd.github+json").max_bytes(4 << 20), None)?.take_text()?;
    let rel = parse_smapi_release(&text).map_err(|e| NetError::Failed(format!("{e}.")))?;
    let known = rel.sha256.clone().or_else(|| known_sha256(file_name_of(&rel.url)).map(str::to_string));
    let check = Expect {
        max_bytes: if rel.size > 0 { rel.size } else { 128 << 20 },
        size: (rel.size > 0).then_some(rel.size),
        sha256: known.clone(),
    };
    let d = client.download(&rel.url, &check, progress)?;
    Ok(LoaderDownload {
        source: loader_source("smapi", &rel.version, &rel.url, &d.sha256),
        download: d,
        plan: smapi_plan(&rel.installer_folder, native_linux),
        pinned: known.is_some(),
    })
}
