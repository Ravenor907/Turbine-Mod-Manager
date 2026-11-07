//! Browsing Nexus Mods without a key (v2 GraphQL with Turbine's application headers only), the file a mod page
//! offers, download names, and the Game Mods folders the browser helper sorts downloads into. Canned answers only.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use turbine_engine::json;
use turbine_engine::net::{Client, NetError};
use turbine_engine::nexus::{self, Nexus, SearchSort};
use turbine_engine::store::{clean_name, nexus_ids_from_name};
use turbine_engine::watch::{self, Basis, Catcher, Event, Seen, Skip};

fn data(name: &str) -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data").join(name)).unwrap()
}

fn num(j: &json::Json) -> f64 {
    match j {
        json::Json::Num(n) => *n,
        _ => -1.0,
    }
}

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("turbine-keyless-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

// A stand-in curl: serves files named after the URL and records the headers and the posted body.
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
n=$(wc -l < "$SERVED/requests.log" | tr -d ' ')
if [ -n "$data" ]; then cp "${data#@}" "$SERVED/post_$n"; fi
key=$(printf '%s' "$url" | tr -c 'A-Za-z0-9' '_')
status=200
if [ -f "$SERVED/$key.status" ]; then status=$(cat "$SERVED/$key.status"); fi
body="$SERVED/$key"
if [ -f "$SERVED/$key.$n" ]; then body="$SERVED/$key.$n"; fi
if [ ! -f "$body" ]; then status=404; fi
{ printf 'HTTP/2 %s\r\n\r\n' "$status"; } > "$hdr"
if [ "$status" -ge 400 ]; then printf '%s %s' "$status" "$url"; exit 22; fi
cp "$body" "$out"
printf '%s %s' "$status" "$url"
"#;

struct Fake {
    dir: PathBuf,
    nx: Nexus,
}

fn key_of(url: &str) -> String {
    url.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect()
}

impl Fake {
    fn new(tag: &str) -> Fake {
        let dir = scratch(tag);
        fs::create_dir_all(dir.join("served")).unwrap();
        let script = dir.join("curl");
        fs::write(&script, FAKE_CURL.replace("@SERVED@", &dir.join("served").display().to_string())).unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        let mut c = Client::new("0.5", &dir.join("tmp"));
        c.program = script;
        Fake { dir, nx: Nexus::public(c, "0.5") }
    }
    /// The answer to the nth request (1 is the first).
    fn serve_nth(&self, url: &str, n: usize, body: &str) {
        fs::write(self.dir.join("served").join(format!("{}.{n}", key_of(url))), body).unwrap();
    }
    fn serve(&self, url: &str, body: &str) {
        fs::write(self.dir.join("served").join(key_of(url)), body).unwrap();
    }
    fn status(&self, url: &str, s: u16) {
        fs::write(self.dir.join("served").join(key_of(url) + ".status"), s.to_string()).unwrap();
    }
    fn sent(&self) -> String {
        fs::read_to_string(self.dir.join("served/sent_headers")).unwrap_or_default()
    }
    fn post(&self, n: usize) -> json::Json {
        json::parse(&fs::read_to_string(self.dir.join("served").join(format!("post_{n}"))).unwrap()).unwrap()
    }
    fn requests(&self) -> usize {
        fs::read_to_string(self.dir.join("served/requests.log")).unwrap_or_default().lines().count()
    }
}

#[test]
fn public_headers_have_no_key() {
    let h = nexus::headers_public("0.5");
    assert!(!h.iter().any(|(k, _)| k.eq_ignore_ascii_case("apikey")));
    assert!(h.contains(&("Application-Name".into(), "TurbineModManager".into())));
    assert!(h.contains(&("Application-Version".into(), "0.5".into())));
    // With a key, the same headers plus the key.
    let k = nexus::headers("abc", "0.5");
    assert_eq!(k.len(), h.len() + 1);
    assert!(k.contains(&("apikey".into(), "abc".into())));
}

#[test]
fn keyless_search_sends_no_key_and_reads_the_page() {
    let f = Fake::new("search");
    assert!(!f.nx.has_key());
    f.serve(nexus::GRAPHQL, &data("nexus_graphql_search.json"));
    let page = f.nx.search("stardewvalley", "", SearchSort::Downloads, 30, 30, false).unwrap();
    assert_eq!(page.mods.len(), 2);
    assert!(!page.relaxed);
    let sent = f.sent();
    assert!(!sent.to_ascii_lowercase().contains("apikey"), "no key header: {sent}");
    assert!(sent.contains("Application-Name: TurbineModManager"));
    assert!(sent.contains("Application-Version: 0.5"));
    let v = f.post(1);
    let vars = v.get("variables").unwrap();
    assert_eq!(num(vars.get("offset").unwrap()), 30.0, "the next page asks by offset");
    assert_eq!(num(vars.get("count").unwrap()), 30.0);
}

#[test]
fn v1_calls_need_the_key_and_make_no_request() {
    let f = Fake::new("v1");
    assert_eq!(f.nx.mod_info("witcher3", 5).unwrap_err(), NetError::Failed(nexus::NEEDS_KEY.into()));
    assert_eq!(f.nx.files("witcher3", 5).unwrap_err(), NetError::Failed(nexus::NEEDS_KEY.into()));
    assert!(f.nx.validate().is_err());
    assert_eq!(f.requests(), 0);
}

#[test]
fn refused_browsing_is_an_error_the_app_can_fall_back_on() {
    let f = Fake::new("refused");
    f.serve(nexus::GRAPHQL, "{}");
    f.status(nexus::GRAPHQL, 401);
    assert!(matches!(f.nx.search("witcher3", "", SearchSort::Endorsements, 0, 30, false), Err(NetError::Denied(401))));
    // A GraphQL error for a plain sort is an error too.
    let g = Fake::new("gqlerror");
    g.serve(nexus::GRAPHQL, &data("nexus_graphql_error.json"));
    let e = g.nx.search("witcher3", "", SearchSort::Downloads, 0, 30, false).unwrap_err();
    assert!(e.to_string().contains("Nexus couldn't search"), "{e}");
}

#[test]
fn too_many_requests_backs_off() {
    let f = Fake::new("429");
    f.serve(nexus::GRAPHQL, "{}");
    f.status(nexus::GRAPHQL, 429);
    assert_eq!(f.nx.search("witcher3", "", SearchSort::Endorsements, 0, 30, false).unwrap_err(), NetError::RateLimited);
    let before = f.requests();
    assert_eq!(f.nx.search("witcher3", "", SearchSort::Endorsements, 0, 30, false).unwrap_err(), NetError::RateLimited);
    assert_eq!(f.requests(), before, "no request while backing off");
}

#[test]
fn trending_filters_by_the_last_30_days_and_falls_back_to_most_endorsed() {
    let now = 1_762_473_600; // 2025-11-07
    let body = json::parse(&nexus::search_body_at("witcher3", "", SearchSort::Trending, 0, 30, false, now)).unwrap();
    let vars = body.get("variables").unwrap();
    let up = &vars.get("filter").unwrap().get("updatedAt").unwrap().as_array()[0];
    assert_eq!(up.str_at("value"), Some("2025-10-08T00:00:00Z"));
    assert_eq!(vars.get("sort").unwrap().as_array()[0].get("endorsements").unwrap().str_at("direction"), Some("DESC"));
    let newest = json::parse(&nexus::search_body("witcher3", "", SearchSort::Newest, 0, 30, false)).unwrap();
    assert!(newest.get("variables").unwrap().get("sort").unwrap().as_array()[0].get("createdAt").is_some());

    // Nexus refuses the filter: asked again as Most endorsed, and the page says so.
    let f = Fake::new("trending");
    f.serve_nth(nexus::GRAPHQL, 1, &data("nexus_graphql_error.json"));
    f.serve_nth(nexus::GRAPHQL, 2, &data("nexus_graphql_search.json"));
    let page = f.nx.search("witcher3", "", SearchSort::Trending, 0, 30, false).unwrap();
    assert!(page.relaxed);
    assert_eq!(page.mods.len(), 2);
    assert!(f.post(2).get("variables").unwrap().get("filter").unwrap().get("updatedAt").is_none());
    assert_eq!(SearchSort::from_key("trending"), SearchSort::Trending);
    assert_eq!(SearchSort::from_key("newest"), SearchSort::Newest);
    assert_eq!(SearchSort::from_key("anything"), SearchSort::Endorsements);
}

#[test]
fn dates_round_trip() {
    assert_eq!(nexus::iso_day(0), "1970-01-01T00:00:00Z");
    assert_eq!(nexus::iso_day(1_709_164_800), "2024-02-29T00:00:00Z");
    assert_eq!(nexus::iso_secs("2024-02-29T00:00:00Z"), 1_709_164_800);
    assert_eq!(nexus::iso_secs("2025-03-04T10:20:30Z"), 1_741_083_630);
    assert_eq!(nexus::iso_secs("nonsense"), 0);
}

#[test]
fn keyless_details_files_and_requirements() {
    let f = Fake::new("details");
    f.serve_nth(nexus::GRAPHQL, 1, &data("nexus_graphql_details.json"));
    f.serve_nth(nexus::GRAPHQL, 2, &data("nexus_graphql_files.json"));
    let (d, reqs) = f.nx.details("witcher3", 5).unwrap();
    assert_eq!(d.info.name, "All Quest Objectives On Map");
    assert_eq!(d.info.author, "Lantern Folk");
    assert_eq!(d.game_id, 952);
    assert_eq!(d.info.endorsements, 12045);
    assert!(d.info.available);
    assert_eq!(d.info.updated_timestamp, 1_741_083_630);
    assert_eq!(reqs.iter().map(|r| r.mod_id).collect::<Vec<_>>(), vec![2109, 3001]);
    let list = f.nx.files_any("witcher3", 5, Some(d.game_id)).unwrap();
    let v = f.post(2);
    assert_eq!(v.get("variables").unwrap().str_at("gameId"), Some("952"));
    assert_eq!(v.get("variables").unwrap().str_at("modId"), Some("5"));
    assert_eq!(list.files.len(), 5);
    assert_eq!(list.file(12).unwrap().size_bytes, 25_165_824);
    assert_eq!(list.file(11).unwrap().size_bytes, 24_000 * 1024, "no byte size: KB times 1024");
    assert!(list.file(14).unwrap().is_removed());
    assert!(!f.sent().to_ascii_lowercase().contains("apikey"));
}

#[test]
fn file_pick_is_the_newest_main_file() {
    let list = nexus::parse_mod_files(&data("nexus_graphql_files.json")).unwrap();
    assert_eq!(list.newest_main().unwrap().file_id, 12, "newest MAIN, not the newer optional file");
    let none = nexus::parse_mod_files(&data("nexus_graphql_files_nomain.json")).unwrap();
    assert_eq!(none.newest_main().unwrap().file_id, 21, "no MAIN: the newest file that can still be downloaded");
    assert!(nexus::parse_mod_files(&data("nexus_graphql_error.json")).is_err());
}

#[test]
fn nexus_download_names_are_cleaned() {
    assert_eq!(clean_name("Fast Travel from Anywhere-324-1-0-5-1699999999.zip"), ("Fast Travel from Anywhere".into(), Some("1.0.5".into())));
    assert_eq!(clean_name("All_Quest_Objectives_On_Map-5-1-0-5-1699999999.7z"), ("All Quest Objectives On Map".into(), Some("1.0.5".into())));
    assert_eq!(clean_name("SkyUI_5_2_SE-12604-5-2SE-1573517013.zip").0, "SkyUI 5 2 SE");
    assert_eq!(clean_name("Better Torches 1.0.zip").0, "Better Torches 1.0");
    assert_eq!(nexus_ids_from_name("Fast Travel from Anywhere-324-1-0-5-1699999999.zip"), Some((324, "1.0.5".into())));
    assert_eq!(nexus_ids_from_name("Better Torches 1.0.zip"), None);
    assert_eq!(nexus_ids_from_name("Mod-1-2.zip"), None);
}

#[test]
fn game_mods_folders_are_sure_matches() {
    let base = scratch("gamemods");
    let dl = base.join("Downloads");
    let w3 = dl.join(watch::GAME_MODS).join(watch::game_folder_name("The Witcher 3: Wild Hunt"));
    fs::create_dir_all(&w3).unwrap();
    fs::create_dir_all(dl.join(watch::GAME_MODS).join(watch::OTHER_FOLDER)).unwrap();
    assert_eq!(watch::game_folder_name("The Witcher 3: Wild Hunt"), "The Witcher 3 Wild Hunt");
    assert_eq!(watch::game_folder_name("a/b\\c?"), "abc");
    assert_eq!(watch::game_folder_name(".."), "Other");
    let folders = watch::with_game_mods(std::slice::from_ref(&dl));
    assert!(folders.contains(&dl));
    assert!(folders.contains(&w3));
    let file = w3.join("All Quest Objectives On Map-5-1-0-5-1699999999.zip");
    assert_eq!(watch::game_mods_folder_of(&file).as_deref(), Some("The Witcher 3 Wild Hunt"));
    assert_eq!(watch::game_mods_folder_of(&dl.join("x.zip")), None);
    assert_eq!(watch::game_mods_folder_of(&dl.join("Game Mods/Other/x.zip")), None);
    // Sure, even on the first run's catch-up, and ahead of everything else.
    let d = watch::decide_all(Some("steam:292030"), Some("steam:489830"), None, None, Some("steam:1"), true);
    assert_eq!(d.game.as_deref(), Some("steam:292030"));
    assert_eq!(d.basis, Basis::Folder);
    assert!(d.sure);
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn unfinished_downloads_are_left_alone() {
    let dir = PathBuf::from("/home/deck/Downloads/Game Mods/The Witcher 3");
    let s = |n: &str, size: u64| Seen { path: dir.join(n), size, modified: 1_800_000_100 };
    let mut c = Catcher::new(vec![dir.clone()], 1_800_000_000, 1_800_000_050);
    let ev = c.step(&[s("Map-5-1-0.zip", 10), s("Map-5-1-0.zip.crdownload", 10), s("Other.zip.tmp", 5)]);
    assert!(ev.iter().any(|e| matches!(e, Event::Skipped { why: Skip::Unfinished, .. })));
    assert!(!ev.iter().any(|e| matches!(e, Event::Ready(_))));
    // Gone: it counts once its size holds.
    let mut ev = c.step(&[s("Map-5-1-0.zip", 10)]);
    ev.extend(c.step(&[s("Map-5-1-0.zip", 10)]));
    assert!(ev.iter().any(|e| matches!(e, Event::Ready(a) if a.path.ends_with("Map-5-1-0.zip"))));
}
