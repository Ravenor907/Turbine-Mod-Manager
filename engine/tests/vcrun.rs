//! 0.64: the Visual C++ 2022 runtime in a Proton prefix (Cyberpunk's RED4ext and Cyber Engine Tweaks, error 998).

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use turbine_engine::net::{self, Client, NetError};
use turbine_engine::vcrun::{self, State};

const REG: &str = "WINE REGISTRY Version 2\n;; All keys relative to \\\\User\\\\S-1-5-21-0-0-0-1000\n\n#arch=win64\n\n[Software\\\\Wine\\\\DllOverrides] 1730000001\n#time=1db2f8e7a6c1a01\n\"atl100\"=\"native,builtin\"\n";

fn scratch(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("turbine-vcrun-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

/// A DLL with a version resource: "MZ", padding, then VS_FIXEDFILEINFO.
fn dll(major: u16, minor: u16, build: u16, wine: bool) -> Vec<u8> {
    let mut b = b"MZ".to_vec();
    b.resize(64, 0);
    if wine {
        b.extend_from_slice(b"Wine builtin DLL");
    }
    b.resize(512, 0);
    b.extend_from_slice(&[0xBD, 0x04, 0xEF, 0xFE]);
    b.extend_from_slice(&0x0001_0000u32.to_le_bytes());
    b.extend_from_slice(&(((major as u32) << 16) | minor as u32).to_le_bytes());
    b.extend_from_slice(&((build as u32) << 16).to_le_bytes());
    b.resize(1024, 0);
    b
}

fn prefix(tag: &str, file: Option<Vec<u8>>) -> PathBuf {
    let p = scratch(tag).join("pfx");
    fs::create_dir_all(p.join("drive_c/windows/system32")).unwrap();
    fs::write(p.join("user.reg"), REG).unwrap();
    if let Some(f) = file {
        fs::write(p.join("drive_c/windows/system32/vcruntime140.dll"), f).unwrap();
    }
    p
}

#[test]
fn tells_what_the_prefix_has() {
    assert_eq!(vcrun::state(None), State::NoPrefix);
    assert_eq!(vcrun::state(Some(Path::new("/nonexistent/pfx"))), State::NoPrefix);
    assert_eq!(vcrun::state(Some(&prefix("none", None))), State::Missing);
    assert_eq!(vcrun::state(Some(&prefix("wine", Some(dll(14, 42, 0, true))))), State::WineOnly);
    // Steam's first-start runtime is the 2019 one (14.29): that is the case that fails with error 998.
    let old = prefix("old", Some(dll(14, 29, 30153, false)));
    assert_eq!(vcrun::state(Some(&old)), State::Old("14.29.30153".into()));
    assert!(vcrun::state(Some(&old)).needs_install());
    // The 2022 runtime, but Wine still loads its own until the overrides say native first.
    let new = prefix("new", Some(dll(14, 42, 34433, false)));
    assert_eq!(vcrun::state(Some(&new)), State::NotUsed("14.42.34433".into()));
    let changed = vcrun::set_overrides(&new).unwrap();
    assert!(changed.contains(&"vcruntime140".to_string()) && changed.contains(&"msvcp140".to_string()) && changed.len() == vcrun::DLLS.len());
    assert_eq!(vcrun::state(Some(&new)), State::Ready("14.42.34433".into()));
    assert!(!vcrun::state(Some(&new)).needs_install());
    assert!(vcrun::set_overrides(&new).unwrap().is_empty(), "set once");
    for s in [State::Missing, State::WineOnly, State::Old("14.29.1".into()), State::NotUsed("14.4.1".into())] {
        assert!(!s.words().contains('\u{2014}'));
    }
}

#[test]
fn the_checksum_comes_from_microsofts_address() {
    let sha = "a".repeat(64);
    let url = format!("https://download.visualstudio.microsoft.com/download/pr/1754ea58-11a6-44ab-a262-696e194ce543/{}/VC_redist.x64.exe", sha.to_uppercase());
    assert_eq!(vcrun::sha_from_url(&url).as_deref(), Some(sha.as_str()));
    assert_eq!(vcrun::sha_from_url("https://download.visualstudio.microsoft.com/download/pr/x/VC_redist.x64.exe"), None);
    assert!(net::allowed_url(vcrun::REDIST_URL) && net::allowed_url(&url));
}

const FAKE_CURL: &str = r#"#!/bin/sh
out=; hdr=
while [ $# -gt 0 ]; do
  case "$1" in
    --output) out=$2; shift;;
    --dump-header) hdr=$2; shift;;
  esac
  shift
done
printf 'HTTP/2 200\r\n\r\n' > "$hdr"
cp '@BODY@' "$out"
printf '200 %s' '@EFF@'
"#;

fn client(tag: &str, body: &[u8], effective: &str) -> Client {
    let dir = scratch(tag);
    fs::write(dir.join("body"), body).unwrap();
    let script = dir.join("curl");
    fs::write(&script, FAKE_CURL.replace("@BODY@", &dir.join("body").display().to_string()).replace("@EFF@", effective)).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    let mut c = Client::new("0.64", &dir.join("tmp"));
    c.program = script;
    c
}

#[test]
fn downloads_only_what_microsoft_named() {
    let body = dll(14, 42, 0, false);
    let sha = turbine_engine::sha256::digest_hex(&body);
    let good = format!("https://download.visualstudio.microsoft.com/download/pr/abc/{}/VC_redist.x64.exe", sha.to_uppercase());
    let d = vcrun::download(&client("ok", &body, &good), None).unwrap();
    assert_eq!(fs::read(&d.path).unwrap(), body);
    // Another file at that address: deleted.
    let wrong = format!("https://download.visualstudio.microsoft.com/download/pr/abc/{}/VC_redist.x64.exe", "b".repeat(64));
    assert!(matches!(vcrun::download(&client("bad", &body, &wrong), None), Err(NetError::Mismatch(_))));
    // No checksum in the address: refused.
    let bare = "https://download.visualstudio.microsoft.com/download/pr/abc/VC_redist.x64.exe";
    assert!(vcrun::download(&client("bare", &body, bare), None).unwrap_err().to_string().contains("checksum"));
    // Not a Windows program: refused.
    let text = b"<html>nope</html>".to_vec();
    let sha2 = turbine_engine::sha256::digest_hex(&text);
    let u = format!("https://download.visualstudio.microsoft.com/download/pr/abc/{sha2}/VC_redist.x64.exe");
    assert!(matches!(vcrun::download(&client("html", &text, &u), None), Err(NetError::Mismatch(_))));
}

#[test]
fn runs_through_the_games_proton_and_its_runtime() {
    let lib = scratch("lib");
    let common = lib.join("steamapps/common");
    for (name, rt) in [("Proton 9.0", "1628350"), ("Proton 10.0", "1628350"), ("Proton - Experimental", "1628350")] {
        fs::create_dir_all(common.join(name)).unwrap();
        fs::write(common.join(name).join("proton"), "#!/usr/bin/env python3\n").unwrap();
        fs::write(common.join(name).join("toolmanifest.vdf"), format!("\"manifest\"\n{{\n  \"commandline\" \"/proton %verb%\"\n  \"require_tool_appid\" \"{rt}\"\n}}\n")).unwrap();
    }
    let libs = vec![lib.clone()];
    assert_eq!(vcrun::find_proton(&libs, None), Some(common.join("Proton 10.0")), "the newest numbered Proton");
    assert_eq!(vcrun::find_proton(&libs, Some(&common.join("Proton 9.0"))), Some(common.join("Proton 9.0")), "the one on record");
    assert_eq!(vcrun::find_proton(&libs, Some(Path::new("/gone"))), Some(common.join("Proton 10.0")));
    // The runtime it names isn't installed: nothing can run it.
    assert_eq!(vcrun::runtime_command(&libs, &common.join("Proton 10.0")), None);
    let sniper = common.join("SteamLinuxRuntime_sniper");
    fs::create_dir_all(&sniper).unwrap();
    fs::write(lib.join("steamapps/appmanifest_1628350.acf"), "\"AppState\"\n{\n\t\"appid\"\t\t\"1628350\"\n\t\"installdir\"\t\t\"SteamLinuxRuntime_sniper\"\n}\n").unwrap();
    fs::write(sniper.join("toolmanifest.vdf"), "\"manifest\"\n{\n  \"commandline\" \"/_v2-entry-point --verb=%verb% --\"\n}\n").unwrap();
    fs::write(sniper.join("_v2-entry-point"), "").unwrap();
    let rt = vcrun::runtime_command(&libs, &common.join("Proton 10.0")).unwrap();
    assert_eq!(rt, vec![format!("{}/_v2-entry-point", sniper.display()), "--verb=runinprefix".into(), "--".into()]);
    let args = vcrun::install_args(&lib.join("steamapps/compatdata/1091500"), Path::new("/home/deck/.local/share/Steam"), &rt, &common.join("Proton 10.0"), Path::new("/c/vc_redist.x64.exe"));
    assert_eq!(&args[..2], &["flatpak-spawn".to_string(), "--host".to_string()]);
    assert!(args.contains(&format!("--env=STEAM_COMPAT_DATA_PATH={}", lib.join("steamapps/compatdata/1091500").display())));
    let tail: Vec<&str> = args[args.len() - 5..].iter().map(String::as_str).collect();
    assert_eq!(tail, ["runinprefix", "/c/vc_redist.x64.exe", "/install", "/quiet", "/norestart"]);
    assert!(args.iter().all(|a| a != "sh" && a != "bash" && a != "-c"), "never a shell");
    assert!(vcrun::installer_ok(0) && vcrun::installer_ok(3010) && vcrun::installer_ok(1638) && !vcrun::installer_ok(1618));
    assert!(vcrun::installer_words(1618).contains("Close the game"));
}
