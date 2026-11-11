//! Nexus Mods, offline. The samples in tests/data/nexus_*.json follow Nexus's documented v1 response types and the
//! v2 GraphQL schema (the requirements sample uses the ModRequirements, ModRequirement and ModRequirementsDlc types
//! of the published schema); they are not recorded live answers (api.nexusmods.com can't be reached from the build
//! machine).

mod support;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use support::{data_text as data, scratch};
use turbine_engine::json;
use turbine_engine::net::{Client, NetError};
use turbine_engine::net::SourceInfo;
use turbine_engine::nexus::{self, Account, Nexus, RateLimit, Requirement, RequirementStep, SearchSort};
use turbine_engine::nxm;

#[test]
fn reads_the_account() {
    let a = nexus::parse_validate(&data("nexus_validate.json")).unwrap();
    assert_eq!(a, Account { user_id: 1234567, name: "SamplePlayer".into(), is_premium: true, is_supporter: true });
    assert!(nexus::parse_validate("{\"message\":\"Please provide a valid API Key\"}").is_err());
    assert!(nexus::parse_validate("not json").is_err());
}

#[test]
fn reads_mod_details_and_files() {
    let m = nexus::parse_mod(&data("nexus_mod.json")).unwrap();
    assert_eq!(m.mod_id, 12604);
    assert_eq!(m.domain, "skyrimspecialedition");
    assert_eq!(m.version, "5.2SE");
    assert_eq!(m.endorsements, 300000);
    assert!(m.available && !m.adult);
    let files = nexus::parse_files(&data("nexus_files.json")).unwrap();
    assert_eq!(files.files.len(), 3);
    assert_eq!(files.files[0].size_bytes, 2700 * 1024, "no size_in_bytes: KB are used");
    assert_eq!(files.files[1].size_bytes, 2816572);
    assert_eq!(files.main_file().unwrap().file_id, 200);
    assert_eq!(files.newest_replacement(100).unwrap().file_id, 200);
    assert_eq!(files.newest_replacement(200), None);
    assert!(!files.files[0].is_current());
    assert_eq!(files.files[2].category, "OPTIONAL");
}

#[test]
fn reads_update_lists_and_links() {
    let u = nexus::parse_updated(&data("nexus_updated.json")).unwrap();
    assert_eq!(u.len(), 2);
    assert_eq!(u[0].mod_id, 12604);
    assert_eq!(u[0].latest_file_update, 1600000000);
    assert!(nexus::parse_updated("{}").is_err());
    // Mirrors on hosts Turbine doesn't download from are dropped.
    let l = nexus::parse_download_links(&data("nexus_download_link.json")).unwrap();
    assert_eq!(l.len(), 1);
    assert_eq!(l[0].short_name, "Nexus CDN");
    assert!(nexus::parse_download_links("[]").is_err());
}

#[test]
fn rate_limits_are_respected() {
    let h = |a: &str, b: &str| vec![("x-rl-hourly-remaining".to_string(), a.to_string()), ("X-RL-Daily-Remaining".to_string(), b.to_string()), ("x-rl-daily-reset".to_string(), "2026-11-07 00:00:00 +0000".to_string())];
    let rl = nexus::rate_limit(&h("99", "2400"));
    assert_eq!(rl.hourly_remaining, Some(99));
    assert_eq!(rl.daily_remaining, Some(2400));
    assert_eq!(rl.blocked(), None);
    // The hourly allowance used up still leaves the daily one.
    assert_eq!(nexus::rate_limit(&h("0", "100")).blocked(), None);
    let stop = nexus::rate_limit(&h("0", "0")).blocked().unwrap();
    assert_eq!(stop, "Nexus's daily request limit is used up. It resets at 2026-11-07 00:00:00 +0000. Turbine tries again after that.");
    assert_eq!(RateLimit::default().blocked(), None);
}

#[test]
fn graphql_search_body_and_answer() {
    let body = nexus::search_body_in("stardewvalley", "content \"patcher\"", "", SearchSort::Endorsements, 20, 500, false, nxm::now());
    let j = json::parse(&body).unwrap();
    assert!(j.str_at("query").unwrap().contains("mods(filter: $filter, sort: $sort, offset: $offset, count: $count, facets: {categoryName: []})"));
    let v = j.get("variables").unwrap();
    let f = v.get("filter").unwrap();
    assert_eq!(f.get("gameDomainName").unwrap().as_array()[0].str_at("value"), Some("stardewvalley"));
    assert_eq!(f.get("name").unwrap().as_array()[0].str_at("value"), Some("content \"patcher\""));
    assert_eq!(f.get("name").unwrap().as_array()[0].str_at("op"), Some("WILDCARD"));
    assert_eq!(f.get("adultContent").unwrap().as_array()[0].get("value"), Some(&json::Json::Bool(false)));
    assert_eq!(v.get("sort").unwrap().as_array()[0].get("endorsements").unwrap().str_at("direction"), Some("DESC"));
    assert_eq!(v.get("count"), Some(&json::Json::Num(100.0)), "count is capped");
    let plain = json::parse(&nexus::search_body_in("valheim", " ", "", SearchSort::Name, 0, 10, true, nxm::now())).unwrap();
    let f = plain.get("variables").unwrap().get("filter").unwrap();
    assert!(f.get("name").is_none() && f.get("adultContent").is_none());

    let page = nexus::parse_search(&data("nexus_graphql_search.json")).unwrap();
    assert_eq!(page.total, 2);
    assert_eq!(page.mods[0].name, "Content Patcher");
    assert_eq!(page.mods[0].endorsements, 50000);
    assert_eq!(page.mods[1].picture_url, "");
    let err = nexus::parse_search(&data("nexus_graphql_error.json")).unwrap_err();
    assert_eq!(err, "Nexus couldn't search (Argument 'filter' has an invalid value)");
}

#[test]
fn ids_domains_and_keys() {
    assert_eq!(nexus::domain_for_store_id("489830"), Some("skyrimspecialedition"));
    assert_eq!(nexus::domain_for_store_id("1453375253"), Some("stardewvalley"));
    assert_eq!(nexus::domain_for_store_id("1"), None);
    let id = nexus::source_id("skyrimspecialedition", 12604, 200);
    assert_eq!(id, "skyrimspecialedition:12604:200");
    assert_eq!(nexus::parse_source_id(&id), Some(("skyrimspecialedition".into(), 12604, 200)));
    assert_eq!(nexus::parse_source_id("a:1"), None);
    assert_eq!(nexus::parse_source_id("a:1:2:3"), None);
    let files = nexus::parse_files(&data("nexus_files.json")).unwrap();
    let info = nexus::source_info("skyrimspecialedition", 12604, &files.files[1], None);
    assert_eq!(info.kind, "nexus");
    assert_eq!(info.version.as_deref(), Some("5.2SE"));
    assert_eq!(info.url.as_deref(), Some("https://www.nexusmods.com/skyrimspecialedition/mods/12604"));
    assert!(nexus::plausible_key("  abcdefghijklmnopqrstuvwxyz0123456789--abc==  "));
    assert!(!nexus::plausible_key("short"));
    assert!(!nexus::plausible_key("has a space in the middle of it ok"));
    let h = nexus::headers("key ", "0.3");
    assert!(h.contains(&("apikey".into(), "key".into())));
    assert!(h.contains(&("Application-Name".into(), "TurbineModManager".into())));
    assert!(h.contains(&("Application-Version".into(), "0.3".into())));
}

#[test]
fn key_file_is_private() {
    let dir = scratch("key");
    let path = dir.join("secrets/nexus-key");
    let key = "abcdefghijklmnopqrstuvwxyz0123456789";
    nexus::save_key(&path, &format!("{key}\n")).unwrap();
    assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
    assert_eq!(nexus::load_key(&path).as_deref(), Some(key));
    nexus::save_key(&path, "abcdefghijklmnopqrstuvwxyz0123456789XYZ").unwrap();
    assert_eq!(nexus::load_key(&path).as_deref(), Some("abcdefghijklmnopqrstuvwxyz0123456789XYZ"));
    assert_eq!(nexus::load_key(&dir.join("none")), None);
}

// ---- Through a stand-in curl that also records the headers it was given ----

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
fn session_calls_send_app_headers_and_stop_at_the_limit() {
    let f = Fake::new("session");
    f.serve("https://api.nexusmods.com/v1/users/validate.json", data("nexus_validate.json").as_bytes());
    f.rl(50, 900);
    let acct = f.nx.validate().unwrap();
    assert!(acct.is_premium);
    let sent = fs::read_to_string(f.dir.join("served/sent_headers")).unwrap();
    assert!(sent.contains("apikey: abcdefghijklmnopqrstuvwxyz0123456789"));
    assert!(sent.contains("Application-Name: TurbineModManager"));
    assert!(sent.contains("Application-Version: 0.3"));
    assert_eq!(f.nx.limits.take().hourly_remaining, Some(50));

    f.serve("https://api.nexusmods.com/v1/games/skyrimspecialedition/mods/updated.json?period=1w", data("nexus_updated.json").as_bytes());
    assert_eq!(f.nx.updated("skyrimspecialedition", "1w").unwrap().len(), 2);
    assert!(f.nx.updated("skyrimspecialedition", "2y").is_err());
    assert!(f.nx.updated("somegame", "1d").unwrap_err().to_string().contains("doesn't support"));

    // Once Nexus says both allowances are used up, Turbine stops asking.
    f.rl(0, 0);
    f.serve("https://api.nexusmods.com/v1/games/skyrimspecialedition/mods/12604.json", data("nexus_mod.json").as_bytes());
    assert_eq!(f.nx.mod_info("skyrimspecialedition", 12604).unwrap().name, "SkyUI");
    let before = f.requests();
    let e = f.nx.mod_info("skyrimspecialedition", 12604).unwrap_err();
    assert!(e.to_string().starts_with("Nexus's daily request limit is used up."));
    assert_eq!(f.requests(), before);
}

#[test]
fn bad_keys_and_free_accounts_get_plain_messages() {
    let f = Fake::new("free");
    let u = "https://api.nexusmods.com/v1/users/validate.json";
    f.serve(u, b"{\"message\":\"Please provide a valid API Key\"}");
    f.status(u, 401);
    assert_eq!(f.nx.validate().unwrap_err().to_string(), "Nexus didn't accept that API key. Copy it again from your Nexus account page.");
    let free = Account { user_id: 1, name: "x".into(), is_premium: false, is_supporter: false };
    let before = f.requests();
    let e = f.nx.download_links(&free, "skyrimspecialedition", 12604, 200).unwrap_err();
    assert_eq!(e, NetError::Failed(nexus::FREE_ACCOUNT_MESSAGE.into()));
    assert_eq!(f.requests(), before, "no request for a free account");
    // A Premium account that Nexus still refuses gets the same plain message.
    let premium = Account { is_premium: true, ..free };
    let link = "https://api.nexusmods.com/v1/games/skyrimspecialedition/mods/12604/files/200/download_link.json";
    f.serve(link, b"{}");
    f.status(link, 403);
    assert_eq!(f.nx.download_links(&premium, "skyrimspecialedition", 12604, 200).unwrap_err().to_string(), nexus::FREE_ACCOUNT_MESSAGE);
}

#[test]
fn premium_download_checks_size() {
    let f = Fake::new("premium");
    let acct = Account { user_id: 1, name: "x".into(), is_premium: true, is_supporter: false };
    let link = "https://api.nexusmods.com/v1/games/skyrimspecialedition/mods/12604/files/200/download_link.json";
    f.serve(link, data("nexus_download_link.json").as_bytes());
    let files = nexus::parse_files(&data("nexus_files.json")).unwrap();
    let file = files.files[1].clone();
    let cdn = "https://supporter-files.nexus-cdn.com/1704/12604/SkyUI_5_2_SE-12604-5-2SE.7z?md5=abc&expires=1700000000&user_id=1";
    f.serve(cdn, &vec![7u8; file.size_bytes as usize]);
    let (d, info) = f.nx.download(&acct, "skyrimspecialedition", 12604, &file, None).unwrap();
    assert_eq!(d.size, 2816572);
    assert_eq!(info.id, "skyrimspecialedition:12604:200");
    assert_eq!(info.sha256.as_deref(), Some(d.sha256.as_str()));
    f.serve(cdn, b"tiny");
    assert!(matches!(f.nx.download(&acct, "skyrimspecialedition", 12604, &file, None), Err(NetError::Mismatch(_))));
}

#[test]
fn search_posts_the_query() {
    let f = Fake::new("search");
    f.serve(nexus::GRAPHQL, data("nexus_graphql_search.json").as_bytes());
    let page = f.nx.search("stardewvalley", "content", SearchSort::Downloads, 0, 20, false).unwrap();
    assert_eq!(page.mods.len(), 2);
    let posted = fs::read_to_string(f.dir.join("served/last_post")).unwrap();
    let j = json::parse(&posted).unwrap();
    assert_eq!(j.get("variables").unwrap().get("sort").unwrap().as_array()[0].get("downloads").unwrap().str_at("direction"), Some("DESC"));
    let sent = fs::read_to_string(f.dir.join("served/sent_headers")).unwrap();
    assert!(sent.contains("Content-Type: application/json"));
}

// ---- Other versions ----

#[test]
fn files_are_grouped_for_other_versions() {
    let list = nexus::parse_files(&data("nexus_files_all.json")).unwrap();
    assert_eq!(list.files.len(), 9);
    assert_eq!(list.files[8].category, "DELETED", "a null category name falls back to the category number");
    let groups = list.groups();
    let shape: Vec<(&str, &str, Vec<u64>)> =
        groups.iter().map(|g| (g.category.as_str(), g.label, g.files.iter().map(|f| f.file_id).collect())).collect();
    assert_eq!(
        shape,
        vec![
            ("MAIN", "Main files", vec![200]),
            ("UPDATE", "Updates", vec![210]),
            ("OPTIONAL", "Optional files", vec![310, 300]),
            ("MISCELLANEOUS", "Other files", vec![400]),
            ("OLD_VERSION", "Old versions", vec![100, 10]),
            ("ARCHIVED", "Archived", vec![500]),
        ],
        "removed files are left out, newest first in each group"
    );
    let other = list.other_versions(200);
    assert_eq!(other[0].category, "UPDATE", "the installed file's group is gone when it was the only one");
    assert_eq!(other.iter().map(|g| g.files.len()).sum::<usize>(), 7);

    // Picking a file: the wanted one while it can be downloaded, else what replaced it, else the main file.
    assert_eq!(list.pick(Some(100)).unwrap().file_id, 100, "old versions can still be downloaded");
    assert_eq!(list.pick(Some(600)).unwrap().file_id, 200, "a removed file falls back to the main file");
    assert_eq!(list.pick(Some(999)).unwrap().file_id, 200);
    assert_eq!(list.pick(None).unwrap().file_id, 200);
    let gone = nexus::parse_files("{\"files\":[{\"file_id\":1,\"category_id\":1,\"is_primary\":false,\"uploaded_timestamp\":1}],\"file_updates\":[{\"old_file_id\":5,\"new_file_id\":1,\"uploaded_timestamp\":2}]}").unwrap();
    assert_eq!(gone.pick(Some(5)).unwrap().file_id, 1, "a file missing from the list follows its replacement");

    let one = nexus::parse_file(&data("nexus_file.json")).unwrap();
    assert_eq!((one.file_id, one.size_bytes, nexus::category_label(&one.category)), (200, 2816572, "Main files"));
    assert!(nexus::parse_file("{}").is_err());
    assert_eq!(nexus::files_page_url("valheim", 7), "https://www.nexusmods.com/valheim/mods/7?tab=files");
    assert_eq!(nexus::file_page_url("valheim", 7, 9), "https://www.nexusmods.com/valheim/mods/7?tab=files&file_id=9");
}

// ---- Requirements ----

fn req(name: &str, mod_id: u64, domain: &str) -> Requirement {
    Requirement {
        name: name.into(),
        mod_id,
        domain: domain.into(),
        url: if domain.is_empty() { String::new() } else { nexus::mod_page_url(domain, mod_id) },
        notes: String::new(),
        is_dlc: false,
        external: false,
    }
}

#[test]
fn reads_requirements() {
    let body = json::parse(&nexus::requirements_body("skyrimspecialedition", 12604)).unwrap();
    let q = body.str_at("query").unwrap();
    for field in ["legacyModsByDomain(ids: $ids)", "modRequirements", "nexusRequirements(count: 20)", "dlcRequirements", "gameExpansion", "externalRequirement"] {
        assert!(q.contains(field), "{field}");
    }
    let id = &body.get("variables").unwrap().get("ids").unwrap().as_array()[0];
    assert_eq!(id.str_at("gameDomain"), Some("skyrimspecialedition"));
    assert_eq!(id.get("modId"), Some(&json::Json::Num(12604.0)));

    let r = nexus::parse_requirements(&data("nexus_graphql_requirements.json"), "skyrimspecialedition").unwrap();
    assert_eq!(r.len(), 6);
    assert!(r[0].external && r[0].mod_id == 0 && r[0].url == "https://skse.silverlock.org/");
    assert_eq!(r[0].notes, "Use the build that matches your game version.");
    assert_eq!(r[1], Requirement { notes: String::new(), ..req("Address Library for SKSE Plugins", 32444, "skyrimspecialedition") });
    // No address: the same game as the mod. Control characters are dropped from notes.
    assert_eq!((r[2].domain.as_str(), r[2].mod_id), ("skyrimspecialedition", 17230));
    assert_eq!(r[2].url, "https://www.nexusmods.com/skyrimspecialedition/mods/17230");
    assert_eq!(r[2].notes, "Recommended load order: before SkyUI");
    // Another Nexus section, read from its address.
    assert_eq!((r[3].domain.as_str(), r[3].mod_id, r[3].url.as_str()), ("site", 3863, "https://www.nexusmods.com/site/mods/3863"));
    // An address that isn't http(s) is never handed to the browser.
    assert!(r[4].external && r[4].url.is_empty());
    assert!(r[5].is_dlc && r[5].name == "Dawnguard" && r[5].url.is_empty());

    assert_eq!(
        nexus::parse_requirements(&data("nexus_graphql_requirements_none.json"), "valheim").unwrap_err(),
        "Nexus doesn't know that mod"
    );
    assert_eq!(
        nexus::parse_requirements(&data("nexus_graphql_error.json"), "valheim").unwrap_err(),
        "Nexus couldn't list the requirements (Argument 'filter' has an invalid value)"
    );
    let empty = "{\"data\":{\"legacyModsByDomain\":{\"nodes\":[{\"modId\":1,\"modRequirements\":{\"nexusRequirements\":{\"nodes\":[]},\"dlcRequirements\":[]}}]}}}";
    assert!(nexus::parse_requirements(empty, "valheim").unwrap().is_empty());
}

#[test]
fn resolves_missing_requirements() {
    let r = nexus::parse_requirements(&data("nexus_graphql_requirements.json"), "skyrimspecialedition").unwrap();
    let src = |id: &str| SourceInfo { kind: "nexus".into(), id: id.into(), ..SourceInfo::default() };
    let installed = vec![
        ("address-library".to_string(), src("skyrimspecialedition:32444:9001")),
        ("zip-mod".to_string(), SourceInfo { kind: "zip".into(), ..SourceInfo::default() }),
        // An old record with only the mod ID still counts.
        ("ussep".to_string(), src("17230")),
        // Same mod ID in another game doesn't.
        ("tool".to_string(), src("valheim:3863:1")),
    ];
    let c = nexus::check_requirements(&r, &installed);
    assert_eq!(c.installed.iter().map(|x| x.mod_id).collect::<Vec<_>>(), vec![32444, 17230]);
    assert_eq!(c.missing.iter().map(|x| x.mod_id).collect::<Vec<_>>(), vec![3863]);
    assert_eq!(c.unchecked.len(), 3, "outside downloads and DLC can't be checked");

    let mut missing = c.missing.clone();
    missing.push(req("Address Library for SKSE Plugins", 32444, "skyrimspecialedition"));
    // Premium: Turbine downloads mods of supported games; others stay pages.
    assert_eq!(
        nexus::requirement_steps(&missing, true),
        vec![
            RequirementStep::OpenPage { name: "Some Tool".into(), url: "https://www.nexusmods.com/site/mods/3863?tab=files".into() },
            RequirementStep::Download { name: "Address Library for SKSE Plugins".into(), domain: "skyrimspecialedition".into(), mod_id: 32444 },
        ]
    );
    // Free accounts: pages to open, on the Files tab.
    assert_eq!(
        nexus::requirement_steps(&missing, false)[1],
        RequirementStep::OpenPage {
            name: "Address Library for SKSE Plugins".into(),
            url: "https://www.nexusmods.com/skyrimspecialedition/mods/32444?tab=files".into()
        }
    );
    let j = json::parse(&nexus::requirements_json(&c, false)).unwrap();
    assert_eq!(j.get("missing").unwrap().as_array().len(), 1);
    assert_eq!(j.get("unchecked").unwrap().as_array()[2].get("isDlc"), Some(&json::Json::Bool(true)));
    assert_eq!(j.get("steps").unwrap().as_array()[0].str_at("action"), Some("open"));
}

#[test]
fn mod_page_addresses() {
    assert_eq!(nexus::parse_mod_page_url("https://www.nexusmods.com/valheim/mods/387"), Some(("valheim".into(), 387)));
    assert_eq!(nexus::parse_mod_page_url("https://nexusmods.com/valheim/mods/387?tab=files"), Some(("valheim".into(), 387)));
    assert_eq!(nexus::parse_mod_page_url("https://www.nexusmods.com/valheim/mods/387/"), Some(("valheim".into(), 387)));
    for bad in [
        "https://evil.com/valheim/mods/387",
        "https://www.nexusmods.com.evil.com/valheim/mods/387",
        "https://www.nexusmods.com/valheim/collections/387",
        "https://www.nexusmods.com/Val-heim/mods/387",
        "https://www.nexusmods.com/valheim/mods/0",
        "https://www.nexusmods.com/valheim/mods/+3",
        "https://www.nexusmods.com/valheim/mods",
        "ftp://www.nexusmods.com/valheim/mods/3",
    ] {
        assert_eq!(nexus::parse_mod_page_url(bad), None, "{bad}");
    }
    assert_eq!(nexus::store_ids_for_domain("stardewvalley"), vec!["413150", "1453375253"]);
    assert_eq!(nexus::game_name_for_domain("valheim"), Some("Valheim"));
    assert!(nexus::supported_domain("riskofrain2") && !nexus::supported_domain("site"));
}

#[test]
fn requirements_through_graphql() {
    let f = Fake::new("reqs");
    f.serve(nexus::GRAPHQL, data("nexus_graphql_requirements.json").as_bytes());
    let r = f.nx.requirements("skyrimspecialedition", 12604).unwrap();
    assert_eq!(r.len(), 6);
    let posted = json::parse(&fs::read_to_string(f.dir.join("served/last_post")).unwrap()).unwrap();
    assert!(posted.str_at("query").unwrap().contains("nexusRequirements"));
    assert!(f.nx.requirements("fallout4", 1).unwrap_err().to_string().contains("doesn't support"));
}

// ---- nxm downloads ----

const LINK: &str = "nxm://skyrimspecialedition/mods/12604/files/200?key=iL0D8DbD1OBn9-iW0n3nJw&expires=1762444800&user_id=1";

#[test]
fn nxm_download_works_for_free_accounts() {
    let f = Fake::new("nxm");
    let link = nxm::parse(LINK).unwrap();
    f.serve("https://api.nexusmods.com/v1/games/skyrimspecialedition/mods/12604/files/200.json", data("nexus_file.json").as_bytes());
    let dl = "https://api.nexusmods.com/v1/games/skyrimspecialedition/mods/12604/files/200/download_link.json?key=iL0D8DbD1OBn9-iW0n3nJw&expires=1762444800";
    f.serve(dl, data("nexus_download_link.json").as_bytes());
    let cdn = "https://supporter-files.nexus-cdn.com/1704/12604/SkyUI_5_2_SE-12604-5-2SE.7z?md5=abc&expires=1700000000&user_id=1";
    f.serve(cdn, &vec![3u8; 2816572]);
    let (d, info) = f.nx.download_from_nxm_at(&link, 1762444000, None).unwrap();
    assert_eq!(d.size, 2816572);
    assert_eq!(info.id, "skyrimspecialedition:12604:200");
    assert_eq!(info.version.as_deref(), Some("5.2SE"));
    assert_eq!(info.sha256.as_deref(), Some(d.sha256.as_str()));
    assert!(fs::read_to_string(f.dir.join("served/requests.log")).unwrap().contains("download_link.json?key="));

    // Expired by the clock: no request at all.
    let before = f.requests();
    let e = f.nx.download_from_nxm_at(&link, 1762444800, None).unwrap_err();
    assert_eq!(e.to_string(), nxm::EXPIRED_MESSAGE);
    assert_eq!(f.requests(), before);
    // Nexus says the key is gone (410, or 404 for the link after the file was found).
    f.status(dl, 410);
    assert_eq!(f.nx.download_from_nxm_at(&link, 1762444000, None).unwrap_err().to_string(), nxm::EXPIRED_MESSAGE);
    // A key for another account is refused with words that say what to do.
    f.status(dl, 403);
    let e = f.nx.download_from_nxm_at(&link, 1762444000, None).unwrap_err().to_string();
    assert!(e.starts_with("Nexus didn't accept this download link.") && e.ends_with("Press Mod Manager Download on the Nexus page again."));
    // A short download is caught.
    f.status(dl, 200);
    f.serve(cdn, b"tiny");
    assert!(matches!(f.nx.download_from_nxm_at(&link, 1762444000, None), Err(NetError::Mismatch(_))));
    // Games Turbine doesn't support are refused before any request.
    let other = nxm::parse("nxm://fallout4/mods/1/files/2?key=a&expires=1762444800").unwrap();
    let before = f.requests();
    assert!(f.nx.download_from_nxm_at(&other, 1, None).unwrap_err().to_string().contains("doesn't support Nexus mods for fallout4"));
    assert_eq!(f.requests(), before);
}

#[test]
fn premium_downloads_by_id() {
    let f = Fake::new("byid");
    let premium = Account { user_id: 1, name: "x".into(), is_premium: true, is_supporter: false };
    let free = Account { is_premium: false, ..premium.clone() };
    f.serve("https://api.nexusmods.com/v1/games/skyrimspecialedition/mods/12604/files.json", data("nexus_files_all.json").as_bytes());
    let link = "https://api.nexusmods.com/v1/games/skyrimspecialedition/mods/12604/files/200/download_link.json";
    f.serve(link, data("nexus_download_link.json").as_bytes());
    let cdn = "https://supporter-files.nexus-cdn.com/1704/12604/SkyUI_5_2_SE-12604-5-2SE.7z?md5=abc&expires=1700000000&user_id=1";
    f.serve(cdn, &vec![1u8; 2816572]);
    // The removed file 600 falls back to the main file.
    let (_, info) = f.nx.download_by_id(&premium, "skyrimspecialedition", 12604, Some(600), None).unwrap();
    assert_eq!(info.id, "skyrimspecialedition:12604:200");
    let (_, info) = f.nx.download_by_id(&premium, "skyrimspecialedition", 12604, None, None).unwrap();
    assert_eq!(info.id, "skyrimspecialedition:12604:200");
    let before = f.requests();
    assert_eq!(f.nx.download_by_id(&free, "skyrimspecialedition", 12604, None, None).unwrap_err().to_string(), nexus::FREE_ACCOUNT_MESSAGE);
    assert_eq!(f.requests(), before);
    let groups = f.nx.files("skyrimspecialedition", 12604).unwrap().groups();
    assert_eq!(groups.len(), 6);
    f.serve("https://api.nexusmods.com/v1/games/skyrimspecialedition/mods/12604/files/200.json", data("nexus_file.json").as_bytes());
    assert_eq!(f.nx.file_info("skyrimspecialedition", 12604, 200).unwrap().version, "5.2SE");
}
