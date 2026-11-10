//! Loaders, offline. tests/data/skse_page.html is rebuilt from the live SKSE page's text and links (its raw HTML
//! couldn't be fetched from the build machine); github_smapi_latest.json follows GitHub's release format with the
//! asset names SMAPI's build scripts produce.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use turbine_engine::loaders::{self, Hook, SkseEdition, SkseSource};
use turbine_engine::net::{Client, NetError};
use turbine_engine::nexus::{Account, Nexus};
use turbine_engine::thunderstore;

fn data(name: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data").join(name)).unwrap()
}

fn scratch(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("turbine-loaders-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn reads_the_skse_page() {
    let builds = loaders::parse_skse_page(&data("skse_page.html"));
    let ed: Vec<(SkseEdition, &str)> = builds.iter().map(|b| (b.edition, b.version.as_str())).collect();
    assert_eq!(
        ed,
        vec![
            (SkseEdition::SteamAe, "2.3.1"),
            (SkseEdition::GogAe, "2.2.6"),
            (SkseEdition::SpecialEdition, "2.0.20"),
            (SkseEdition::Classic, "1.7.3"),
            (SkseEdition::Vr, "2.0.12"),
        ]
    );
    assert_eq!(builds[0].game_version.as_deref(), Some("1.7.104"));
    assert_eq!(builds[2].link.as_deref(), Some("https://skse.silverlock.org/beta/skse64_2_00_20.7z"), "relative links resolve");
    // The Steam build is published on Nexus now; the GOG build is a .7z on the SKSE site.
    assert_eq!(
        loaders::skse_source(&builds, SkseEdition::SteamAe),
        Ok(SkseSource::Nexus { domain: "skyrimspecialedition".into(), mod_id: 30379, version: "2.3.1".into() })
    );
    assert_eq!(
        loaders::skse_source(&builds, SkseEdition::GogAe),
        Ok(SkseSource::Direct { url: "https://skse.silverlock.org/beta/skse64_2_02_06_gog.7z".into(), version: "2.2.6".into() })
    );
    // The classic build's first link is the Steam store: not a download Turbine takes.
    assert!(loaders::skse_source(&builds, SkseEdition::Classic).is_err());
    assert!(loaders::skse_source(&[], SkseEdition::SteamAe).is_err());
}

#[test]
fn skse_page_variants() {
    // A direct AE link (as the page had before) is used directly; entities and single quotes are handled.
    let html = "<P>Current Anniversary Edition build 2.2.6 (game version 1.6.1170):&nbsp;<A HREF='https://skse.silverlock.org/beta/skse64_2_02_06.7z'>Download&amp;go</A></P>";
    let b = loaders::parse_skse_page(html);
    assert_eq!(loaders::skse_source(&b, SkseEdition::SteamAe), Ok(SkseSource::Direct { url: "https://skse.silverlock.org/beta/skse64_2_02_06.7z".into(), version: "2.2.6".into() }));
    // A link to anywhere else is refused.
    let html = "<p>Current Anniversary Edition build 9.9.9: <a href=\"https://evil.example.com/skse.7z\">x</a></p>";
    assert!(loaders::skse_source(&loaders::parse_skse_page(html), SkseEdition::SteamAe).unwrap_err().contains("evil.example.com"));
    let html = "<p>Current Anniversary Edition build 9.9.9: <a href=\"http://skse.silverlock.org/x.7z\">x</a></p>";
    assert!(loaders::skse_source(&loaders::parse_skse_page(html), SkseEdition::SteamAe).is_err(), "plain http is refused");
    assert!(loaders::parse_skse_page("<p>nothing here</p><a").is_empty());
    assert_eq!(loaders::nexus_mod_link("https://www.nexusmods.com/skyrimspecialedition/mods/30379?tab=files"), Some(("skyrimspecialedition".into(), 30379)));
    assert_eq!(loaders::nexus_mod_link("https://evil.com/skyrimspecialedition/mods/30379"), None);
}

#[test]
fn install_plans_map_paths() {
    let p = loaders::skse_plan();
    assert_eq!(p.map_path("skse64_2_03_01/skse64_loader.exe").as_deref(), Some("skse64_loader.exe"));
    assert_eq!(p.map_path("skse64_2_03_01/skse64_1_7_104.dll").as_deref(), Some("skse64_1_7_104.dll"));
    assert_eq!(p.map_path("skse64_2_03_01/Data/Scripts/Actor.pex").as_deref(), Some("Data/Scripts/Actor.pex"));
    assert_eq!(p.map_path("skse64_2_03_01/src/skse64/main.cpp"), None);
    assert_eq!(p.map_path("skse64_2_03_01/skse64_readme.txt"), None);
    assert_eq!(p.map_path("skse64_2_03_01"), None);
    assert_eq!(p.hooks, vec![Hook::LauncherSwap { original: "SkyrimSELauncher.exe".into(), replacement: "skse64_loader.exe".into() }]);

    let (pack, folder) = loaders::bepinex_pack("valheim").unwrap();
    assert_eq!(pack, "denikson-BepInExPack_Valheim");
    let b = loaders::bepinex_plan(folder, false);
    assert_eq!(b.map_path("BepInExPack_Valheim/winhttp.dll").as_deref(), Some("winhttp.dll"));
    assert_eq!(b.map_path("BepInExPack_Valheim/BepInEx/core/BepInEx.dll").as_deref(), Some("BepInEx/core/BepInEx.dll"));
    assert_eq!(b.map_path("manifest.json"), None);
    assert_eq!(b.map_path("icon.png"), None);
    assert_eq!(b.hooks, vec![Hook::DllOverride { dll: "winhttp".into() }]);
    let native = loaders::bepinex_plan(folder, true);
    assert_eq!(native.executable, vec!["start_game_bepinex.sh"]);
    assert_eq!(loaders::bepinex_pack("lethal-company"), Some(("BepInEx-BepInExPack", "BepInExPack")));
    assert_eq!(loaders::bepinex_pack("riskofrain2"), Some(("bbepis-BepInExPack", "BepInExPack")));
    assert_eq!(loaders::bepinex_pack("stardew"), None);

    let s = loaders::smapi_plan("SMAPI 4.5.2 installer", true);
    assert_eq!(s.inner_archive.as_deref(), Some("SMAPI 4.5.2 installer/internal/linux/install.dat"));
    assert_eq!(s.copies, vec![("Stardew Valley.deps.json".to_string(), "StardewModdingAPI.deps.json".to_string())]);
    assert_eq!(s.hooks, vec![Hook::LauncherSwap { original: "StardewValley".into(), replacement: "unix-launcher.sh".into() }]);
    assert_eq!(s.map_path("smapi-internal/SMAPI.Toolkit.dll").as_deref(), Some("smapi-internal/SMAPI.Toolkit.dll"));
    let w = loaders::smapi_plan("SMAPI 4.5.2 installer", false);
    assert_eq!(w.inner_archive.as_deref(), Some("SMAPI 4.5.2 installer/internal/windows/install.dat"));
    assert!(matches!(&w.hooks[0], Hook::LaunchOption { text } if text.contains("StardewModdingAPI.exe")));
}

#[test]
fn reads_smapi_release() {
    let r = loaders::parse_smapi_release(&data("github_smapi_latest.json")).unwrap();
    assert_eq!(r.version, "4.5.2");
    assert_eq!(r.url, "https://github.com/Pathoschild/SMAPI/releases/download/4.5.2/SMAPI-4.5.2-installer.zip");
    assert_eq!(r.size, 29000000);
    assert_eq!(r.sha256.as_deref(), Some("ab".repeat(32).as_str()));
    assert_eq!(r.installer_folder, "SMAPI 4.5.2 installer");
    assert!(loaders::parse_smapi_release("{\"tag_name\":\"1\",\"assets\":[{\"name\":\"SMAPI-1-beta-installer.zip\"}]}").is_err());
    assert!(loaders::parse_smapi_release("[]").is_err());
}

// ---- Through a stand-in curl ----

const FAKE_CURL: &str = r#"#!/bin/sh
SERVED='@SERVED@'
out=; hdr=; url=
while [ $# -gt 0 ]; do
  case "$1" in
    --output) out=$2; shift;;
    --dump-header) hdr=$2; shift;;
    --) url=$2; shift;;
  esac
  shift
done
printf '%s\n' "$url" >> "$SERVED/requests.log"
key=$(printf '%s' "$url" | tr -c 'A-Za-z0-9' '_')
if [ ! -f "$SERVED/$key" ]; then printf 'HTTP/2 404\r\n\r\n' > "$hdr"; printf '404 %s' "$url"; exit 22; fi
printf 'HTTP/2 200\r\n\r\n' > "$hdr"
cp "$SERVED/$key" "$out"
printf '200 %s' "$url"
"#;

fn fake(tag: &str) -> (PathBuf, Client) {
    let dir = scratch(tag);
    fs::create_dir_all(dir.join("served")).unwrap();
    let script = dir.join("curl");
    fs::write(&script, FAKE_CURL.replace("@SERVED@", &dir.join("served").display().to_string())).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    let mut c = Client::new("0.3", &dir.join("tmp"));
    c.program = script;
    (dir, c)
}

fn serve(dir: &Path, url: &str, body: &[u8]) {
    let key: String = url.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
    fs::write(dir.join("served").join(key), body).unwrap();
}

#[test]
fn skse_downloads_or_explains() {
    let (dir, client) = fake("skse");
    serve(&dir, loaders::SKSE_PAGE, data("skse_page.html").as_bytes());
    serve(&dir, "https://skse.silverlock.org/beta/skse64_2_02_06_gog.7z", b"7z\xbc\xaf\x27\x1c pretend");
    let got = loaders::fetch_skse(&client, SkseEdition::GogAe, None, None).unwrap();
    assert_eq!(got.source.kind, "loader");
    assert_eq!(got.source.id, "skse");
    assert_eq!(got.source.version.as_deref(), Some("2.2.6"));
    assert_eq!(got.source.sha256.as_deref(), Some(got.download.sha256.as_str()));
    assert!(!got.pinned);
    assert!(got.download.path.exists());

    // The Steam build without a Nexus Premium session: a plain message, no download.
    let e = loaders::fetch_skse(&client, SkseEdition::SteamAe, None, None).unwrap_err();
    assert_eq!(e, NetError::Failed(loaders::skse_browser_message("2.3.1")));
    assert!(e.to_string().contains("Desktop Mode"));
    let free = Account { user_id: 1, name: "x".into(), is_premium: false, is_supporter: false };
    let nx = Nexus::new(client.clone(), "abcdefghijklmnopqrstuvwxyz0123456789", "0.3");
    assert_eq!(loaders::fetch_skse(&client, SkseEdition::SteamAe, Some((&nx, &free)), None).unwrap_err(), e);
}

#[test]
fn bepinex_and_smapi_download() {
    let (dir, client) = fake("bep");
    let (index, _) = thunderstore::parse_index(fs::File::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/thunderstore_v1_lethal.json")).unwrap()).unwrap();
    serve(&dir, "https://thunderstore.io/package/download/BepInEx/BepInExPack/5.4.2100/", &vec![1u8; 637381]);
    let got = loaders::fetch_bepinex(&client, &index, "lethal-company", false, None).unwrap();
    assert_eq!(got.source.kind, "loader");
    assert_eq!(got.source.id, "BepInEx-BepInExPack");
    assert_eq!(got.source.version.as_deref(), Some("5.4.2100"));
    assert_eq!(got.plan.take_from, "BepInExPack");
    assert!(loaders::fetch_bepinex(&client, &index, "valheim", false, None).unwrap_err().to_string().contains("doesn't list"));

    let mut rel = data("github_smapi_latest.json");
    let body = vec![9u8; 1000];
    let sha = turbine_engine::sha256::digest_hex(&body);
    rel = rel.replace("29000000", "1000").replace(&"AB".repeat(32), &sha);
    serve(&dir, loaders::SMAPI_LATEST, rel.as_bytes());
    serve(&dir, "https://github.com/Pathoschild/SMAPI/releases/download/4.5.2/SMAPI-4.5.2-installer.zip", &body);
    let got = loaders::fetch_smapi(&client, true, None).unwrap();
    assert!(got.pinned, "GitHub's digest was checked");
    assert_eq!(got.source.id, "smapi");
    assert_eq!(got.plan.inner_archive.as_deref(), Some("SMAPI 4.5.2 installer/internal/linux/install.dat"));
    // A file that doesn't match GitHub's digest is refused.
    serve(&dir, "https://github.com/Pathoschild/SMAPI/releases/download/4.5.2/SMAPI-4.5.2-installer.zip", &vec![8u8; 1000]);
    assert!(matches!(loaders::fetch_smapi(&client, true, None), Err(NetError::Mismatch(_))));
    // 0.63: when GitHub limits its API, the release page's address names the version and its installer.
    let r = loaders::smapi_release_from_page("https://github.com/Pathoschild/SMAPI/releases/tag/4.5.2").unwrap();
    assert_eq!(r.url, "https://github.com/Pathoschild/SMAPI/releases/download/4.5.2/SMAPI-4.5.2-installer.zip");
    assert_eq!(r.installer_folder, "SMAPI 4.5.2 installer");
    assert!(loaders::smapi_release_from_page("https://github.com/Pathoschild/SMAPI/releases").is_none());
    assert!(loaders::smapi_release_from_page("https://github.com/Someone/Else/releases/tag/4.5.2").is_none());
}
