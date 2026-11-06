//! Identifying a downloaded file on Nexus by its MD5 (md5_search), and the Nexus domains of the new games.
//! tests/data/nexus_md5_search.json follows the response type Nexus's own client library declares for md5_search
//! (a list of {"mod", "file_details"}), with the fields of the other v1 samples; it is not a recorded live answer
//! (api.nexusmods.com can't be reached from the build machine).

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use turbine_engine::md5::md5_hex;
use turbine_engine::net::Client;
use turbine_engine::nexus::{self, Nexus};
use turbine_engine::nxm;

fn data(name: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data").join(name)).unwrap()
}

fn scratch(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("turbine-nexusmd5-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

const FAKE_CURL: &str = r#"#!/bin/sh
SERVED='@SERVED@'
out=; hdr=; url=; data=
while [ $# -gt 0 ]; do
  case "$1" in
    --output) out=$2; shift;;
    --dump-header) hdr=$2; shift;;
    --data-binary) data=$2; shift;;
    --header) printf '%s\n' "$2" >> "$SERVED/sent_headers"; shift;;
    --) url=$2; shift;;
  esac
  shift
done
printf '%s\n' "$url" >> "$SERVED/requests.log"
if [ -n "$data" ]; then cp "${data#@}" "$SERVED/last_post"; fi
key=$(printf '%s' "$url" | tr -c 'A-Za-z0-9' '_')
status=200
if [ -f "$SERVED/$key.status" ]; then status=$(cat "$SERVED/$key.status"); fi
if [ ! -f "$SERVED/$key" ]; then status=404; fi
{ printf 'HTTP/2 %s\r\n' "$status"; if [ -f "$SERVED/rl" ]; then cat "$SERVED/rl"; fi; printf '\r\n'; } > "$hdr"
if [ "$status" -ge 400 ]; then printf '%s %s' "$status" "$url"; exit 22; fi
cp "$SERVED/$key" "$out"
printf '%s %s' "$status" "$url"
"#;

struct Fake {
    dir: PathBuf,
    nx: Nexus,
}

#[allow(dead_code)]
impl Fake {
    fn new(tag: &str) -> Fake {
        let dir = scratch(tag);
        fs::create_dir_all(dir.join("served")).unwrap();
        let script = dir.join("curl");
        fs::write(&script, FAKE_CURL.replace("@SERVED@", &dir.join("served").display().to_string())).unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        let mut c = Client::new("0.3", &dir.join("tmp"));
        c.program = script;
        let nx = Nexus::new(c, "abcdefghijklmnopqrstuvwxyz0123456789", "0.3");
        Fake { dir, nx }
    }
    fn serve(&self, url: &str, body: &[u8]) {
        let key: String = url.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
        fs::write(self.dir.join("served").join(key), body).unwrap();
    }
    fn status(&self, url: &str, s: u16) {
        let key: String = url.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
        fs::write(self.dir.join("served").join(key + ".status"), s.to_string()).unwrap();
    }
    fn rl(&self, hourly: u32, daily: u32) {
        fs::write(self.dir.join("served/rl"), format!("x-rl-hourly-remaining: {hourly}\r\nx-rl-daily-remaining: {daily}\r\n")).unwrap();
    }
    fn requests(&self) -> usize {
        fs::read_to_string(self.dir.join("served/requests.log")).unwrap_or_default().lines().count()
    }
}

#[test]
fn md5_search_answer_is_read() {
    let list = nexus::parse_md5_search(&data("nexus_md5_search.json"), "cyberpunk2077").unwrap();
    assert_eq!(list.len(), 2);
    let m = &list[0];
    assert_eq!((m.domain.as_str(), m.mod_info.mod_id, m.mod_info.name.as_str()), ("cyberpunk2077", 107, "Cyber Engine Tweaks"));
    assert_eq!((m.file.file_id, m.file.version.as_str(), m.file.size_bytes), (123456, "1.36.0", 23_606_784));
    assert_eq!(m.md5, "0c5b2e54a1f1e6d1a5f0e2b3c4d5e6f7");
    // The same file in two places: the current, primary one is picked; size decides first.
    assert_eq!(nexus::best_md5_match(&list, 23_606_784).unwrap().mod_info.mod_id, 107);
    assert_eq!(nexus::best_md5_match(&list, 1).unwrap().mod_info.mod_id, 107, "no size fits: the downloadable one");
    assert!(nexus::best_md5_match(&[], 5).is_none());
    // Its record is a proper Nexus source: name, version and update checks work like Mod Manager Download.
    let src = nexus::md5_source_info(m, Some("ab".into()));
    assert_eq!(src.kind, "nexus");
    assert_eq!(src.id, nexus::source_id("cyberpunk2077", 107, 123456));
    assert_eq!(src.version.as_deref(), Some("1.36.0"));
    assert_eq!(src.url.as_deref(), Some("https://www.nexusmods.com/cyberpunk2077/mods/107"));
    assert_eq!(nexus::parse_source_id(&src.id), Some(("cyberpunk2077".to_string(), 107, 123456)));
    // Wrong shapes are errors, not panics.
    assert!(nexus::parse_md5_search("{\"message\":\"No file\"}", "x").is_err());
    assert!(nexus::parse_md5_search("[{\"mod\":{}}]", "x").unwrap().is_empty());
    assert_eq!(nexus::md5_search_path("witcher3", "ABCDEF"), "/v1/games/witcher3/mods/md5_search/abcdef.json");
}

#[test]
fn identify_hashes_the_file_and_asks_nexus() {
    let f = Fake::new("identify");
    let archive = f.dir.join("Cyber Engine Tweaks.zip");
    fs::write(&archive, b"pretend archive").unwrap();
    let md5 = md5_hex(b"pretend archive");
    let url = format!("https://api.nexusmods.com/v1/games/cyberpunk2077/mods/md5_search/{md5}.json");
    f.serve(&url, data("nexus_md5_search.json").as_bytes());
    let m = f.nx.identify("cyberpunk2077", &archive).unwrap().unwrap();
    assert_eq!(m.mod_info.mod_id, 107);
    let log = fs::read_to_string(f.dir.join("served/requests.log")).unwrap();
    assert!(log.contains(&md5));
    // A file Nexus doesn't know: no match, no error.
    fs::write(&archive, b"changed").unwrap();
    assert_eq!(f.nx.identify("cyberpunk2077", &archive).unwrap(), None);
    // Games Turbine doesn't support are refused before any request.
    assert!(f.nx.identify("somegame", &archive).is_err());
}

#[test]
fn new_games_have_nexus_domains() {
    for (id, domain) in [("1091500", "cyberpunk2077"), ("1423049311", "cyberpunk2077"), ("292030", "witcher3"), ("1207664643", "witcher3"), ("1495134320", "witcher3")] {
        assert_eq!(nexus::domain_for_store_id(id), Some(domain), "{id}");
    }
    assert_eq!(nexus::store_ids_for_domain("witcher3"), vec!["292030", "1207664643", "1495134320"]);
    assert_eq!(nexus::game_name_for_domain("cyberpunk2077"), Some("Cyberpunk 2077"));
    let link = nxm::parse("nxm://cyberpunk2077/mods/107/files/123456?key=abc&expires=9999999999&user_id=1").unwrap();
    assert!(link.supported());
    assert_eq!(link.store_ids(), vec!["1091500", "1423049311"]);
}
