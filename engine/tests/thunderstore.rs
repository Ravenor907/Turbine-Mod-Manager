//! Thunderstore, offline. The package list sample (tests/data/thunderstore_v1_lethal.json) follows the v1 list
//! format field for field as Thunderstore's API code defines it; it is not a recorded live answer. The share sample
//! was zipped by Python's zipfile, an independent implementation.

mod support;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use support::{data, scratch};
use turbine_engine::json::Json;
use turbine_engine::net::{Client, NetError};
use turbine_engine::thunderstore::{self as ts, Query, Sort};

fn sample() -> Vec<ts::Package> {
    let (p, skipped) = ts::parse_index(fs::File::open(data("thunderstore_v1_lethal.json")).unwrap()).unwrap();
    assert_eq!(skipped, 1, "the entry without a name is skipped");
    p
}

#[test]
fn reads_the_v1_package_list() {
    let p = sample();
    assert_eq!(p.len(), 10);
    let bep = ts::find(&p, "bepinex-bepinexpack").unwrap();
    assert_eq!(bep.owner, "BepInEx");
    assert_eq!(bep.latest().unwrap().number, "5.4.2100");
    assert_eq!(bep.latest().unwrap().file_size, 637381);
    assert_eq!(bep.downloads, 5_100_000);
    assert_eq!(bep.rating, 2500);
    assert_eq!(bep.categories, vec!["BepInEx", "Libraries"]);
    assert!(bep.description.starts_with("BepInEx pack"));
    assert_eq!(bep.latest().unwrap().download_url, "https://thunderstore.io/package/download/BepInEx/BepInExPack/5.4.2100/");
    // Versions are ordered newest first by number, not by text (1.10.0 is after 1.4.3).
    let suits = ts::find(&p, "x753-More_Suits").unwrap();
    assert_eq!(suits.latest().unwrap().number, "1.10.0");
    assert_eq!(suits.display_name(), "More Suits");
    let lib = ts::find(&p, "Evaisa-LethalLib").unwrap();
    assert_eq!(lib.latest().unwrap().dependencies, vec!["BepInEx-BepInExPack-5.4.2100", "Evaisa-HookGenPatcher-0.0.5"]);
    assert!(ts::find(&p, "Someone-SpicyMod").unwrap().has_nsfw_content);
    assert!(ts::find(&p, "Oldie-OldThing").unwrap().is_deprecated);
}

#[test]
fn list_reader_handles_edges() {
    let mut seen = Vec::new();
    let n = ts::for_each_element(" [ {\"a\":\"x,]}\\\"\"} , [1,[2]] ,3 ]\n".as_bytes(), |t| {
        seen.push(t.to_string());
        Ok(())
    })
    .unwrap();
    assert_eq!(n, 3);
    assert_eq!(seen, vec!["{\"a\":\"x,]}\\\"\"}", "[1,[2]]", "3"]);
    assert_eq!(ts::for_each_element("[]".as_bytes(), |_| Ok(())), Ok(0));
    assert_eq!(ts::for_each_element("\u{feff}[1]".as_bytes(), |_| Ok(())), Ok(1));
    for bad in ["", "{}", "[1,]", "[1", "[1]x", "[,1]", "[1}]"] {
        assert!(ts::for_each_element(bad.as_bytes(), |_| Ok(())).is_err(), "{bad}");
    }
    // A reader that hands over one byte at a time gives the same result.
    struct Slow<'a>(&'a [u8]);
    impl std::io::Read for Slow<'_> {
        fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
            if self.0.is_empty() || b.is_empty() {
                return Ok(0);
            }
            b[0] = self.0[0];
            self.0 = &self.0[1..];
            Ok(1)
        }
    }
    let text = fs::read(data("thunderstore_v1_lethal.json")).unwrap();
    let (slow, _) = ts::parse_index(Slow(&text)).unwrap();
    assert_eq!(slow, sample());
}

#[test]
fn a_large_list_is_read_entry_by_entry() {
    let mut text = String::from("[");
    for i in 0..20_000 {
        if i > 0 {
            text.push(',');
        }
        text.push_str(&format!(
            "{{\"name\":\"Mod{i}\",\"full_name\":\"Owner{o}-Mod{i}\",\"owner\":\"Owner{o}\",\"rating_score\":{r},\"is_deprecated\":false,\"has_nsfw_content\":false,\"categories\":[\"Mods\"],\"date_updated\":\"2024\",\"versions\":[{{\"version_number\":\"1.0.{i}\",\"download_url\":\"https://thunderstore.io/package/download/Owner{o}/Mod{i}/1.0.{i}/\",\"dependencies\":[],\"downloads\":{i},\"file_size\":100,\"description\":\"d\",\"icon\":\"\",\"date_created\":\"2024\",\"is_active\":true}}]}}",
            o = i % 50,
            r = i % 7
        ));
    }
    text.push(']');
    let (p, skipped) = ts::parse_index(text.as_bytes()).unwrap();
    assert_eq!((p.len(), skipped), (20_000, 0));
    let found = ts::search(&p, &Query { text: "mod19999".into(), ..Query::default() });
    assert_eq!(found[0].name, "Mod19999");
}

#[test]
fn cache_round_trips_and_ages() {
    let dir = scratch("cache");
    let p = sample();
    let path = ts::cache_path(&dir, "lethal-company");
    assert!(path.ends_with("thunderstore/lethal-company.json"));
    ts::save_cache(&path, &p).unwrap();
    assert_eq!(ts::load_cache(&path).unwrap(), p);
    assert!(ts::cache_is_fresh(&path, SystemTime::now()));
    assert!(!ts::cache_is_fresh(&path, SystemTime::now() + Duration::from_secs(3601)));
    assert!(!ts::cache_is_fresh(&path, SystemTime::now() - Duration::from_secs(600)), "a cache from the future is refreshed");
    assert!(!ts::cache_is_fresh(&dir.join("none.json"), SystemTime::now()));
    // The compact copy is much smaller than the API's answer.
    assert!(fs::metadata(&path).unwrap().len() < fs::metadata(data("thunderstore_v1_lethal.json")).unwrap().len() / 2);
    assert_eq!(ts::cache_path(&dir, "../evil"), dir.join("thunderstore/evil.json"));
}

#[test]
fn search_filters_and_sorts() {
    let p = sample();
    let names = |q: &Query| ts::search(&p, q).iter().map(|x| x.full_name.clone()).collect::<Vec<_>>();
    let all = names(&Query::default());
    assert!(!all.contains(&"Someone-SpicyMod".to_string()), "NSFW is hidden by default");
    assert!(!all.contains(&"Oldie-OldThing".to_string()), "deprecated is hidden by default");
    assert_eq!(all[0], "BepInEx-BepInExPack", "most downloaded first");
    let shown = names(&Query { show_nsfw: true, show_deprecated: true, ..Query::default() });
    assert_eq!(shown[0], "Someone-SpicyMod");
    assert!(shown.contains(&"Oldie-OldThing".to_string()));
    assert_eq!(names(&Query { text: "more suits".into(), ..Query::default() }), vec!["x753-More_Suits"]);
    assert_eq!(names(&Query { text: "rack".into(), ..Query::default() }), vec!["x753-More_Suits"], "description matches");
    assert_eq!(names(&Query { text: "evaisa".into(), sort: Sort::Name, ..Query::default() }), vec!["Evaisa-HookGenPatcher", "Evaisa-LethalLib"]);
    assert_eq!(names(&Query { category: Some("suits".into()), ..Query::default() }), vec!["x753-More_Suits"]);
    assert!(!names(&Query { hide_modpacks: true, ..Query::default() }).contains(&"Pack-BigPack".to_string()));
    assert_eq!(names(&Query { sort: Sort::Rating, ..Query::default() })[0], "x753-More_Suits");
    assert_eq!(names(&Query { sort: Sort::Updated, ..Query::default() })[0], "x753-More_Suits");
    // A name match ranks above a description-only match.
    let lib = names(&Query { text: "lethal".into(), ..Query::default() });
    assert_eq!(lib[0], "Evaisa-LethalLib");
    let cats = ts::categories(&p);
    assert_eq!(cats.first().map(String::as_str), Some("BepInEx"));
    assert!(cats.contains(&"Modpacks".to_string()));
}

#[test]
fn dependency_strings() {
    let d = ts::parse_dep("Evaisa-LethalLib-0.16.1").unwrap();
    assert_eq!((d.owner.as_str(), d.name.as_str(), d.version.as_deref()), ("Evaisa", "LethalLib", Some("0.16.1")));
    assert_eq!(ts::parse_dep("x753-More_Suits").unwrap().version, None);
    for bad in ["", "OnlyOne", "a--1.0", "a-b-c-d", "a-b-x1", "a b-c-1.0"] {
        assert_eq!(ts::parse_dep(bad), None, "{bad}");
    }
    assert_eq!(ts::community_for_steam_app("1966720"), Some("lethal-company"));
    assert_eq!(ts::community_for_steam_app("413150"), None, "Stardew isn't on Thunderstore");
    assert_eq!(ts::index_url("valheim"), "https://thunderstore.io/c/valheim/api/v1/package/");
}

#[test]
fn resolves_dependencies_newest_first_in_install_order() {
    let p = sample();
    let plan = ts::resolve(&p, &["Evaisa-LethalLib"], &[], false);
    let order: Vec<String> = plan.install.iter().map(|x| format!("{}-{}", x.full_name, x.version)).collect();
    // BepInEx is pinned at 5.4.2000 by HookGenPatcher and 5.4.2100 by LethalLib: the newest is used once.
    assert_eq!(order, vec!["BepInEx-BepInExPack-5.4.2100", "Evaisa-HookGenPatcher-0.0.5", "Evaisa-LethalLib-0.16.1"]);
    assert_eq!(plan.install[2].needs, vec!["BepInEx-BepInExPack", "Evaisa-HookGenPatcher"]);
    assert_eq!(plan.install[0].file_size, 637381);
    assert!(plan.missing.is_empty() && plan.cycles.is_empty());

    // With BepInEx installed by Turbine, the pack is skipped.
    let plan = ts::resolve(&p, &["Evaisa-LethalLib"], &[], true);
    assert_eq!(plan.skipped_loaders, vec!["BepInEx-BepInExPack"]);
    assert_eq!(plan.install.len(), 2);

    // Installed dependencies at the pinned version or newer stay; older ones are upgraded.
    let installed = vec![("Evaisa-HookGenPatcher".to_string(), "0.0.5".to_string()), ("BepInEx-BepInExPack".into(), "5.4.1".into())];
    let plan = ts::resolve(&p, &["Evaisa-LethalLib"], &installed, false);
    assert_eq!(plan.already, vec!["Evaisa-HookGenPatcher"]);
    let bep = plan.install.iter().find(|x| x.full_name == "BepInEx-BepInExPack").unwrap();
    assert!(bep.upgrade);

    // Asking for something already at its newest version installs nothing.
    let plan = ts::resolve(&p, &["Evaisa-HookGenPatcher"], &[("evaisa-hookgenpatcher".into(), "0.0.5".into()), ("BepInEx-BepInExPack".into(), "5.4.2100".into())], false);
    assert!(plan.install.is_empty());
    assert_eq!(plan.already, vec!["Evaisa-HookGenPatcher"]);
}

#[test]
fn exact_versions_missing_packages_and_cycles() {
    let p = sample();
    let plan = ts::resolve(&p, &["Evaisa-LethalLib-0.15.0"], &[], false);
    assert_eq!(plan.install.last().unwrap().version, "0.15.0", "an exact request keeps its version");
    let plan = ts::resolve(&p, &["Pack-BigPack"], &[], false);
    assert_eq!(plan.missing, vec!["Ghost-Missing-1.0.0"]);
    assert_eq!(plan.install.last().unwrap().full_name, "Pack-BigPack");
    // The pack pins LethalLib 0.15.0, a minimum: the newest is installed.
    assert_eq!(plan.install.iter().find(|x| x.full_name == "Evaisa-LethalLib").unwrap().version, "0.16.1");
    let plan = ts::resolve(&p, &["Loop-Alpha"], &[], false);
    assert_eq!(plan.cycles, vec!["Loop-Alpha > Loop-Beta > Loop-Alpha"]);
    assert_eq!(plan.install.iter().map(|x| x.full_name.as_str()).collect::<Vec<_>>(), vec!["Loop-Beta", "Loop-Alpha"]);
    let plan = ts::resolve(&p, &["Nobody-Nothing", "garbage"], &[], false);
    assert_eq!(plan.missing, vec!["Nobody-Nothing", "garbage"]);
    let plan = ts::resolve(&p, &["Evaisa-LethalLib-9.9.9"], &[], false);
    assert_eq!(plan.missing, vec!["Evaisa-LethalLib-9.9.9"]);
    let info = ts::source_info(&ts::resolve(&p, &["Evaisa-LethalLib"], &[], true).install[1], Some("ab".into()));
    assert_eq!(info.kind, "thunderstore");
    assert_eq!(info.id, "Evaisa-LethalLib");
    assert_eq!(info.version.as_deref(), Some("0.16.1"));
    assert_eq!(info.needs, vec!["BepInEx-BepInExPack", "Evaisa-HookGenPatcher"]);
}

#[test]
fn base64_both_ways() {
    for n in 0..70 {
        let data: Vec<u8> = (0..n).map(|i| (i * 37 + 11) as u8).collect();
        let enc = ts::base64_encode(&data);
        assert_eq!(enc.len() % 4, 0);
        assert_eq!(ts::base64_decode(&enc).unwrap(), data);
        let wrapped: String = enc.chars().collect::<Vec<_>>().chunks(10).map(|c| c.iter().collect::<String>() + "\n").collect();
        assert_eq!(ts::base64_decode(&wrapped).unwrap(), data);
    }
    assert_eq!(ts::base64_encode(b"Man"), "TWFu");
    assert_eq!(ts::base64_decode("TWE").unwrap(), b"Ma");
    assert!(ts::base64_decode("TW!u").is_err());
    assert!(ts::base64_decode("TWE=x").is_err());
    assert!(ts::base64_decode("T").is_err());
}

#[test]
fn reads_export_r2x_in_both_indent_styles() {
    let a = ts::parse_export_r2x(&fs::read_to_string(data("export_indented.r2x")).unwrap()).unwrap();
    assert_eq!(a.name, "Friday Night");
    assert_eq!(a.mods.len(), 2);
    assert_eq!(a.mods[0], ts::ProfileMod { full_name: "BepInEx-BepInExPack".into(), version: "5.4.2100".into(), enabled: true });
    assert!(!a.mods[1].enabled);
    assert_eq!(a.wanted(), vec!["BepInEx-BepInExPack-5.4.2100"]);

    let b = "profileName: 'Bob''s # crew'   # a comment\nmods:\n- name: \"Evaisa-LethalLib\"\n  version:\n    major: 0\n    minor: 16\n    patch: 1\n  enabled: true\n- name: BepInEx-BepInExPack\n  version:\n    major: 5\n  enabled: true\n";
    let b = ts::parse_export_r2x(b).unwrap();
    assert_eq!(b.name, "Bob's # crew");
    assert_eq!(b.mods[0].version, "0.16.1");
    assert_eq!(b.mods[1].version, "5.0.0");
    let empty = ts::parse_export_r2x("profileName: Empty\nmods: []\n").unwrap();
    assert!(empty.mods.is_empty());
    for bad in [
        "mods: []\n",
        "profileName: x\nmods:\n  - name: NoDash\n    version: {major: 1}\n",
        "profileName: x\nmods:\n  - name: A-B\n",
        "profileName: x\nmods:\n\t- name: A-B\n",
        "profileName: 'open\n",
        "profileName: x\n    stray: 1\n",
    ] {
        assert!(ts::parse_export_r2x(bad).is_err(), "{bad:?}");
    }
    let y = ts::parse_yaml_subset("a:\n  b: 1\n  c:\n    - x\n    - y: z\n").unwrap();
    assert_eq!(y.get("a").and_then(|a| a.get("c")).map(|c| c.as_array().len()), Some(2));
    assert_eq!(y.get("a").and_then(|a| a.get("b")), Some(&Json::Num(1.0)));
}

#[test]
fn reads_a_share_made_elsewhere() {
    let payload = format!("{}\n{}", ts::PROFILE_PREFIX, fs::read_to_string(data("profile_share.b64")).unwrap());
    let zip = ts::decode_profile_payload(&payload).unwrap();
    let dir = scratch("share");
    let path = dir.join("share.r2z");
    fs::write(&path, &zip).unwrap();
    let p = ts::read_profile_zip(&path).unwrap();
    assert_eq!(p.name, "Friday Night");
    assert_eq!(p.mods.len(), 2);
    assert_eq!(p.files, vec!["BepInEx/config/x753.More_Suits.cfg"]);
    assert!(ts::decode_profile_payload("hello").is_err());
    assert!(ts::decode_profile_payload(&format!("{}\naGVsbG8=", ts::PROFILE_PREFIX)).is_err(), "not a zip");
    assert!(ts::valid_code("0190a2b4-7c1d-4e8f-9a0b-1c2d3e4f5a6b"));
    assert!(ts::valid_code(" 0190A2B4-7C1D-4E8F-9A0B-1C2D3E4F5A6B "));
    assert!(!ts::valid_code("0190a2b47c1d4e8f9a0b1c2d3e4f5a6b"));
    assert!(!ts::valid_code("../../etc/passwd-0000-0000-000000000"));
    assert_eq!(
        ts::profile_get_url("0190A2B4-7C1D-4E8F-9A0B-1C2D3E4F5A6B"),
        "https://thunderstore.io/api/experimental/legacyprofile/get/0190a2b4-7c1d-4e8f-9a0b-1c2d3e4f5a6b/"
    );
}

#[test]
fn export_round_trips_through_the_reader() {
    let profile = ts::SharedProfile {
        name: "Bob's run: #2".into(),
        mods: vec![
            ts::ProfileMod { full_name: "BepInEx-BepInExPack".into(), version: "5.4.2100".into(), enabled: true },
            ts::ProfileMod { full_name: "x753-More_Suits".into(), version: "1.4.3".into(), enabled: false },
        ],
        files: vec![],
    };
    let r2x = ts::build_export_r2x(&profile);
    assert!(r2x.starts_with("profileName: 'Bob''s run: #2'\nmods:\n  - name: BepInEx-BepInExPack\n    version:\n      major: 5\n"));
    assert_eq!(ts::parse_export_r2x(&r2x).unwrap(), profile);
    assert_eq!(ts::build_export_r2x(&ts::SharedProfile { name: "true".into(), ..Default::default() }), "profileName: 'true'\nmods: []\n");

    let zip = ts::build_zip(&[("export.r2x".into(), r2x.into_bytes()), ("BepInEx/config/a.cfg".into(), b"k = v\n".to_vec())]).unwrap();
    let dir = scratch("export");
    let path = dir.join("out.r2z");
    fs::write(&path, &zip).unwrap();
    let back = ts::read_profile_zip(&path).unwrap();
    assert_eq!(back.mods, profile.mods);
    assert_eq!(back.files, vec!["BepInEx/config/a.cfg"]);
    assert!(ts::build_zip(&[("../evil".into(), vec![])]).is_err());
    let payload = ts::encode_profile_payload(&zip);
    assert!(payload.starts_with(ts::PROFILE_PREFIX));
    assert_eq!(ts::decode_profile_payload(&payload).unwrap(), zip);
    assert_eq!(ts::parse_create_answer("{\"key\":\"0190a2b4-7c1d-4e8f-9a0b-1c2d3e4f5a6b\"}").unwrap(), "0190a2b4-7c1d-4e8f-9a0b-1c2d3e4f5a6b");
    assert!(ts::parse_create_answer("{\"detail\":\"Request was throttled.\"}").is_err());
}

// ---- Through a stand-in curl ----

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
if [ -f "$SERVED/$key.exit" ]; then printf '000 %s' "$url"; exit "$(cat "$SERVED/$key.exit")"; fi
if [ ! -f "$SERVED/$key" ]; then printf 'HTTP/2 404\r\n\r\n' > "$hdr"; printf '404 %s' "$url"; exit 22; fi
status=200
if [ -f "$SERVED/$key.status" ]; then status=$(cat "$SERVED/$key.status"); fi
printf 'HTTP/2 %s\r\n\r\n' "$status" > "$hdr"
if [ "$status" -ge 400 ]; then printf '%s %s' "$status" "$url"; exit 22; fi
cp "$SERVED/$key" "$out"
printf '%s %s' "$status" "$url"
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

fn offline(dir: &Path, url: &str) {
    let key: String = url.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
    fs::remove_file(dir.join("served").join(&key)).ok();
    fs::write(dir.join("served").join(key + ".exit"), "6").unwrap();
}

fn requests(dir: &Path) -> usize {
    fs::read_to_string(dir.join("served/requests.log")).unwrap_or_default().lines().count()
}

#[test]
fn fetch_index_uses_the_cache_for_an_hour_and_falls_back_offline() {
    let (dir, client) = fake("fetch");
    let url = ts::index_url("lethal-company");
    serve(&dir, &url, &fs::read(data("thunderstore_v1_lethal.json")).unwrap());
    let cache = dir.join("cache");
    let first = ts::fetch_index(&client, "lethal-company", &cache, false).unwrap();
    assert!(!first.from_cache);
    assert_eq!(first.packages.len(), 10);
    assert_eq!(first.note.as_deref(), Some("Skipped 1 Thunderstore entries that couldn't be read."));
    assert_eq!(requests(&dir), 1);
    let second = ts::fetch_index(&client, "lethal-company", &cache, false).unwrap();
    assert!(second.from_cache);
    assert_eq!(second.packages, first.packages);
    assert_eq!(requests(&dir), 1, "fresh cache: no request");
    offline(&dir, &url);
    let third = ts::fetch_index(&client, "lethal-company", &cache, true).unwrap();
    assert!(third.from_cache);
    assert!(third.note.unwrap().starts_with("No internet connection."));
    let none = ts::fetch_index(&client, "valheim", &cache, false);
    assert_eq!(none.unwrap_err(), NetError::NotFound);
}

#[test]
fn downloads_a_version_and_checks_its_size() {
    let (dir, client) = fake("dl");
    let p = sample();
    let plan = ts::resolve(&p, &["Evaisa-HookGenPatcher"], &[], true);
    let pick = &plan.install[0];
    serve(&dir, &pick.download_url, &vec![b'z'; pick.file_size as usize]);
    let (d, info) = ts::download(&client, pick, None).unwrap();
    assert_eq!(d.size, 50000);
    assert_eq!(info.sha256.as_deref(), Some(d.sha256.as_str()));
    assert_eq!(info.url.as_deref(), Some("https://thunderstore.io/package/download/Evaisa/HookGenPatcher/0.0.5/"));
    serve(&dir, &pick.download_url, b"short");
    assert!(matches!(ts::download(&client, pick, None), Err(NetError::Mismatch(_))));
}

#[test]
fn imports_and_exports_share_codes() {
    let (dir, client) = fake("codes");
    let code = "0190a2b4-7c1d-4e8f-9a0b-1c2d3e4f5a6b";
    let payload = format!("{}\n{}", ts::PROFILE_PREFIX, fs::read_to_string(data("profile_share.b64")).unwrap());
    serve(&dir, &ts::profile_get_url(code), payload.as_bytes());
    let (p, path) = ts::import_code(&client, code).unwrap();
    assert_eq!(p.name, "Friday Night");
    assert!(path.exists());
    let e = ts::import_code(&client, "0190a2b4-7c1d-4e8f-9a0b-000000000000").unwrap_err();
    assert_eq!(e.to_string(), "Thunderstore doesn't know that code. It may have expired or have a typo.");
    assert!(ts::import_code(&client, "nope").unwrap_err().to_string().starts_with("That isn't a profile code."));

    serve(&dir, ts::PROFILE_CREATE_URL, b"{\"key\":\"11111111-2222-4333-8444-555555555555\"}");
    let got = ts::export_code(&client, &p, &[("BepInEx/config/a.cfg".into(), b"x=1".to_vec())]).unwrap();
    assert_eq!(got, "11111111-2222-4333-8444-555555555555");
    // What was uploaded reads back as the same profile.
    let sent = fs::read_to_string(dir.join("served/last_post")).unwrap();
    let zip = ts::decode_profile_payload(&sent).unwrap();
    let back = dir.join("back.r2z");
    fs::write(&back, zip).unwrap();
    let again = ts::read_profile_zip(&back).unwrap();
    assert_eq!(again.mods, p.mods);
    assert_eq!(again.files, vec!["BepInEx/config/a.cfg"]);
    // The upload is a raw POST body sent as application/octet-stream, as Thunderstore's own tests send it, and
    // starts with the format's marker line.
    let headers = fs::read_to_string(dir.join("served/sent_headers")).unwrap();
    assert!(headers.lines().any(|h| h == "Content-Type: application/octet-stream"), "{headers}");
    assert!(sent.starts_with(&format!("{}\n", ts::PROFILE_PREFIX)));
    assert!(ts::PROFILE_CREATE_URL.ends_with("/api/experimental/legacyprofile/create/"));

    // Thunderstore's refusals get plain words: 6 uploads a minute (429), and 400 for an empty body or a full server.
    let status = |s: u16| {
        let key: String = ts::PROFILE_CREATE_URL.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
        fs::write(dir.join("served").join(key + ".status"), s.to_string()).unwrap();
    };
    status(429);
    let e = ts::export_code(&client, &p, &[]).unwrap_err().to_string();
    assert_eq!(e, "Thunderstore makes only a few codes a minute. Try again in a minute.");
    status(400);
    let e = ts::export_code(&client, &p, &[]).unwrap_err().to_string();
    assert_eq!(e, "Thunderstore didn't take the upload. Try again later.");
    status(200);
    serve(&dir, ts::PROFILE_CREATE_URL, b"{\"detail\":\"nope\"}");
    assert_eq!(ts::export_code(&client, &p, &[]).unwrap_err().to_string(), "Thunderstore didn't send a code back.");
}
