//! nxm links, offline: strict parsing, the game they belong to, and expiry.

use turbine_engine::nxm::{self, NxmLink};

#[test]
fn reads_a_mod_manager_download_link() {
    let l = nxm::parse("nxm://skyrimspecialedition/mods/12604/files/200?key=iL0D8DbD1OBn9-iW0n3nJw&expires=1762444800&user_id=1234567").unwrap();
    assert_eq!(
        l,
        NxmLink {
            domain: "skyrimspecialedition".into(),
            mod_id: 12604,
            file_id: 200,
            key: Some("iL0D8DbD1OBn9-iW0n3nJw".into()),
            expires: Some(1762444800),
            user_id: Some(1234567),
        }
    );
    assert_eq!(l.store_ids(), vec!["489830"]);
    assert_eq!(l.game_name(), Some("Skyrim Special Edition"));
    assert!(l.supported());
    assert_eq!(
        l.download_link_path(),
        "/v1/games/skyrimspecialedition/mods/12604/files/200/download_link.json?key=iL0D8DbD1OBn9-iW0n3nJw&expires=1762444800"
    );
    assert_eq!(l.redacted(), "nxm://skyrimspecialedition/mods/12604/files/200");
    assert!(!l.redacted().contains("key"));
    assert_eq!(l.mod_page_url(), "https://www.nexusmods.com/skyrimspecialedition/mods/12604");
    assert!(!l.expired_at(1762444799));
    assert!(l.expired_at(1762444800));
    assert!(l.expired_at(1800000000));
}

#[test]
fn maps_domains_to_turbine_games() {
    // Stardew Valley is on Steam and GOG: both IDs are offered so the app can pick the installed one.
    let l = nxm::parse("nxm://stardewvalley/mods/1915/files/99?key=abc&expires=2000000000").unwrap();
    assert_eq!(l.store_ids(), vec!["413150", "1453375253"]);
    // A real Nexus game Turbine doesn't support: the link is read, but no game is named.
    let other = nxm::parse("nxm://fallout4/mods/1/files/2").unwrap();
    assert!(other.store_ids().is_empty() && !other.supported() && other.game_name().is_none());
    assert_eq!(other.key, None);
    assert!(!other.expired_at(u64::MAX), "a link without a key has no expiry");
    assert_eq!(other.download_link_path(), "/v1/games/fallout4/mods/1/files/2/download_link.json");
}

#[test]
fn tolerates_harmless_variations() {
    // Scheme and host aren't case sensitive; a trailing slash and unknown plain parameters are fine.
    let l = nxm::parse("  NXM://SkyrimSpecialEdition/mods/12604/files/200/?key=k&expires=5&campaign=x_y.1  ").unwrap();
    assert_eq!((l.domain.as_str(), l.mod_id, l.file_id), ("skyrimspecialedition", 12604, 200));
    assert_eq!(l.expires, Some(5));
    assert!(nxm::parse("nxm://valheim/mods/1/files/2?").is_ok());
}

#[test]
fn refuses_anything_odd() {
    for bad in [
        "",
        "nxm://",
        "https://skyrimspecialedition/mods/1/files/2",
        "nxm:/skyrimspecialedition/mods/1/files/2",
        "nxm://skyrim-se/mods/1/files/2",
        "nxm://skyrim.se/mods/1/files/2",
        "nxm://user@skyrim/mods/1/files/2",
        "nxm://skyrim:80/mods/1/files/2",
        "nxm://skyrim/mods/1/files",
        "nxm://skyrim/mods/1/files/2/3",
        "nxm://skyrim/mod/1/files/2",
        "nxm://skyrim/mods/1/file/2",
        "nxm://skyrim/Mods/1/files/2",
        "nxm://skyrim/mods/0/files/2",
        "nxm://skyrim/mods/01/files/2",
        "nxm://skyrim/mods/-1/files/2",
        "nxm://skyrim/mods/+1/files/2",
        "nxm://skyrim/mods/1x/files/2",
        "nxm://skyrim/mods/12345678901/files/2",
        "nxm://skyrim/mods/1/files/2%20",
        "nxm://skyrim/mods/1/files/2#frag",
        "nxm://skyrim/mods/../files/2",
        "nxm://skyrim/mods/1/files/2?key=abc",
        "nxm://skyrim/mods/1/files/2?expires=5",
        "nxm://skyrim/mods/1/files/2?key=a b&expires=5",
        "nxm://skyrim/mods/1/files/2?key=a/b&expires=5",
        "nxm://skyrim/mods/1/files/2?key=a%2Fb&expires=5",
        "nxm://skyrim/mods/1/files/2?key=&expires=5",
        "nxm://skyrim/mods/1/files/2?key=a&expires=soon",
        "nxm://skyrim/mods/1/files/2?key=a&expires=-5",
        "nxm://skyrim/mods/1/files/2?key=a&key=b&expires=5",
        "nxm://skyrim/mods/1/files/2?key=a&expires=5&expires=6",
        "nxm://skyrim/mods/1/files/2?key=a&expires=5&user_id=x",
        "nxm://skyrim/mods/1/files/2?key=a&expires=5&user_id=1&user_id=2",
        "nxm://skyrim/mods/1/files/2?key",
        "nxm://skyrim/mods/1/files/2?Other=1",
        "nxm://skyrim/mods/1/files/2?other=<script>",
        "nxm://skyrim/mods/1/\nfiles/2",
        "nxm://sky\u{e9}rim/mods/1/files/2",
    ] {
        let e = nxm::parse(bad);
        assert!(e.is_err(), "{bad:?} was accepted: {e:?}");
        assert!(e.unwrap_err().starts_with("That Nexus download link can't be used ("));
    }
    let long = format!("nxm://skyrim/mods/1/files/2?key=a&expires=5&x={}", "a".repeat(3000));
    assert!(nxm::parse(&long).is_err());
    let long_key = format!("nxm://skyrim/mods/1/files/2?key={}&expires=5", "a".repeat(129));
    assert!(nxm::parse(&long_key).is_err());
}

#[test]
fn the_expiry_message_is_plain() {
    assert_eq!(nxm::EXPIRED_MESSAGE, "This download link has expired. Press Mod Manager Download on the Nexus page again.");
    assert!(nxm::now() > 1_700_000_000);
}
