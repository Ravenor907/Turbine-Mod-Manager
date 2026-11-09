//! Self-update, offline. tests/data/github_turbine_releases.json is a recorded answer from GitHub's API for this
//! project's releases (one pre-release, 0.1, with GitHub's SHA-256 digest on the bundle).

use std::cmp::Ordering::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use turbine_engine::net::{Client, NetError};
use turbine_engine::selfupdate::{self as su, Check};

fn data(name: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data").join(name)).unwrap()
}

fn scratch(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("turbine-su-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn one_dot_versions_compare_as_decimals() {
    assert_eq!(su::compare("0.3", "0.31"), Some(Less));
    assert_eq!(su::compare("0.31", "0.4"), Some(Less));
    assert_eq!(su::compare("0.4", "1.0"), Some(Less));
    assert_eq!(su::compare("0.31", "0.3"), Some(Greater));
    assert_eq!(su::compare("0.30", "0.3"), Some(Equal));
    assert_eq!(su::compare("v1.0", "1"), Some(Equal));
    assert_eq!(su::compare("0.9", "0.10"), Some(Greater), "0.9 is above 0.10 read as decimals");
    assert_eq!(su::compare("2.0", "10.0"), Some(Less));
    assert_eq!(su::compare("0.2.0", "0.3"), None, "two dots isn't a Turbine version");
    assert_eq!(su::compare("0.3-beta", "0.3"), None);
    assert_eq!(su::compare("", "0.3"), None);
    assert!(su::is_newer("0.31", "0.3"));
    assert!(!su::is_newer("0.3", "0.3"));
    assert!(!su::is_newer("junk", "0.3"));
}

#[test]
fn reads_the_recorded_release_list() {
    let list = data("github_turbine_releases.json");
    // Without Labs mode the 0.1 pre-release doesn't count.
    assert_eq!(su::pick_release(&list, false).unwrap(), None);
    let r = su::pick_release(&list, true).unwrap().unwrap();
    assert_eq!(r.version, "0.1");
    assert_eq!(r.tag, "v0.1");
    assert!(r.prerelease);
    assert_eq!(r.asset_size, 705304);
    assert_eq!(r.asset_url, "https://github.com/Ravenor907/Turbine-Mod-Manager/releases/download/v0.1/TurbineModManager.flatpak");
    assert_eq!(r.sha256.as_deref(), Some("f40cccd57658730bc0732a299f2d3c38b312ee8f9634bd6d303c8effc7106de7"));
    assert_eq!(su::decide(&list, "0.1", true).unwrap(), Check::UpToDate);
    assert!(matches!(su::decide(&list, "0.05", true).unwrap(), Check::Available(r) if r.version == "0.1"));
    assert!(su::pick_release("{]", true).is_err());
}

fn release(tag: &str, pre: bool, extra_assets: &str, body: &str) -> String {
    format!(
        "{{\"tag_name\":\"{tag}\",\"name\":\"Turbine {tag}\",\"draft\":false,\"prerelease\":{pre},\"html_url\":\"https://github.com/Ravenor907/Turbine-Mod-Manager/releases/tag/{tag}\",\"body\":{},\"assets\":[{{\"name\":\"TurbineModManager.flatpak\",\"size\":10,\"browser_download_url\":\"https://github.com/Ravenor907/Turbine-Mod-Manager/releases/download/{tag}/TurbineModManager.flatpak\"}}{extra_assets}]}}",
        turbine_engine::json::quote(body)
    )
}

#[test]
fn picks_the_newest_and_finds_hashes() {
    let hash = "a".repeat(64);
    let list = format!(
        "[{},{},{},{}]",
        release("v0.4", true, "", ""),
        release("v0.31", false, "", &format!("SHA-256 of TurbineModManager.flatpak: {hash}")),
        release("v0.3", false, "", ""),
        release("v0.3.1", false, "", "")
    );
    let stable = su::pick_release(&list, false).unwrap().unwrap();
    assert_eq!(stable.version, "0.31");
    assert_eq!(stable.sha256.as_deref(), Some(hash.as_str()));
    assert_eq!(su::pick_release(&list, true).unwrap().unwrap().version, "0.4");
    // A draft never counts, nor a release without the bundle.
    let draft = release("v9.0", false, "", "").replace("\"draft\":false", "\"draft\":true");
    let nobundle = release("v8.0", false, "", "").replace("TurbineModManager.flatpak\",\"size", "Other.zip\",\"size");
    let list = format!("[{draft},{nobundle},{}]", release("v0.5", false, "", ""));
    assert_eq!(su::pick_release(&list, false).unwrap().unwrap().version, "0.5");
    // The "latest" endpoint answers with one object.
    assert_eq!(su::pick_release(&release("v1.0", false, "", ""), false).unwrap().unwrap().version, "1.0");

    assert_eq!(su::sha256_from_notes(&format!("Changes\n- stuff\n\nsha256: {}", hash.to_uppercase())), Some(hash.clone()));
    assert_eq!(su::sha256_from_notes(&hash), Some(hash.clone()));
    assert_eq!(su::sha256_from_notes(&format!("{hash}\n{}", "b".repeat(64))), None, "two loose hashes: unclear");
    assert_eq!(su::sha256_from_notes("no hash"), None);
    assert_eq!(su::parse_sha256_file(&format!("{hash}  TurbineModManager.flatpak\n")), Some(hash.clone()));
    assert_eq!(su::parse_sha256_file("nope"), None);

    let with_asset = release("v0.6", false, ",{\"name\":\"TurbineModManager.flatpak.sha256\",\"size\":90,\"browser_download_url\":\"https://github.com/Ravenor907/Turbine-Mod-Manager/releases/download/v0.6/TurbineModManager.flatpak.sha256\"}", "");
    let r = su::pick_release(&with_asset, false).unwrap().unwrap();
    assert_eq!(r.sha256, None);
    assert!(r.sha256_asset_url.unwrap().ends_with(".flatpak.sha256"));
}

#[test]
fn install_arguments_are_fixed() {
    assert_eq!(
        su::install_args(Path::new("/home/deck/.var/app/io.github.ravenor907.TurbineModManager/cache/u.flatpak")),
        vec![
            "flatpak-spawn",
            "--host",
            "flatpak",
            "install",
            "--user",
            "-y",
            "--reinstall",
            "--bundle",
            "/home/deck/.var/app/io.github.ravenor907.TurbineModManager/cache/u.flatpak"
        ]
    );
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
    let mut c = Client::new("0.3", &dir.join("cache"));
    c.program = script;
    (dir, c)
}

fn serve(dir: &Path, url: &str, body: &[u8]) {
    let key: String = url.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
    fs::write(dir.join("served").join(key), body).unwrap();
}

#[test]
fn checks_downloads_and_verifies() {
    let (dir, client) = fake("flow");
    // Only pre-releases so far: the latest endpoint answers 404, which means up to date.
    assert_eq!(su::check(&client, "0.1", false).unwrap(), Check::UpToDate);
    serve(&dir, su::LIST_URL, data("github_turbine_releases.json").as_bytes());
    assert_eq!(su::check(&client, "0.1", true).unwrap(), Check::UpToDate);

    let bundle = b"0123456789".to_vec();
    let sha = turbine_engine::sha256::digest_hex(&bundle);
    serve(&dir, su::LATEST_URL, release("v0.31", false, "", &format!("sha256 {sha}")).as_bytes());
    let Check::Available(r) = su::check(&client, "0.3", false).unwrap() else { panic!("0.31 is newer than 0.3") };
    serve(&dir, &r.asset_url, &bundle);
    let d = su::download(&client, &r, None).unwrap();
    assert_eq!(fs::read(&d.path).unwrap(), bundle);
    assert!(d.path.starts_with(dir.join("cache")));

    // A bundle that doesn't match is deleted.
    serve(&dir, &r.asset_url, b"9876543210");
    assert_eq!(su::download(&client, &r, None), Err(NetError::Mismatch("its SHA-256 is different".into())));
    // No published hash at all: refused before downloading.
    let mut bare = (*r).clone();
    bare.sha256 = None;
    assert!(su::download(&client, &bare, None).unwrap_err().to_string().contains("doesn't publish a SHA-256"));
    // A .sha256 asset is used when the notes have none.
    let url = "https://github.com/Ravenor907/Turbine-Mod-Manager/releases/download/v0.31/TurbineModManager.flatpak.sha256";
    bare.sha256_asset_url = Some(url.into());
    serve(&dir, url, format!("{sha}  TurbineModManager.flatpak\n").as_bytes());
    serve(&dir, &r.asset_url, &bundle);
    assert!(su::download(&client, &bare, None).is_ok());
}

#[test]
fn the_update_goes_where_turbine_is_installed() {
    let system = "[Application]\nname=io.github.ravenor907.TurbineModManager\n\n[Instance]\ninstance-id=1\napp-path=/var/lib/flatpak/app/io.github.ravenor907.TurbineModManager/x86_64/master/abc/files\n";
    let user = "[Instance]\napp-path=/home/deck/.local/share/flatpak/app/io.github.ravenor907.TurbineModManager/x86_64/master/abc/files\n";
    assert_eq!(su::install_scope(system), "--system");
    assert_eq!(su::install_scope(user), "--user");
    assert_eq!(su::install_scope(""), "--user");
    // app-path outside [Instance] doesn't count.
    assert_eq!(su::install_scope("[Application]\napp-path=/var/lib/flatpak/app/x\n"), "--user");
    let a = su::install_args_to(Path::new("/tmp/u.flatpak"), "--system");
    assert_eq!(a[4], "--system");
    assert_eq!(su::install_args_to(Path::new("/tmp/u.flatpak"), "anything")[4], "--user");
}
