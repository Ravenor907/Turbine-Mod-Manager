//! nxm links: what the "Mod Manager Download" button on a Nexus Mods page hands to the program registered for the
//! nxm:// scheme. A link names one file of one mod and, for the signed-in player, carries a short-lived key that lets
//! even free accounts get that one download link through the API:
//!
//!   nxm://<game domain>/mods/<mod id>/files/<file id>?key=<key>&expires=<unix time>&user_id=<account id>
//!
//! Everything in a link is checked strictly before any of it is used: the game domain is lowercase letters and
//! digits only, the IDs are plain numbers, the key uses only the characters Nexus keys are made of, and each known
//! parameter appears at most once. Unknown parameters are ignored (so a new one from Nexus doesn't break downloads),
//! but they must still be plain names and values.

use crate::nexus;

/// The words shown when a link's time is up.
pub const EXPIRED_MESSAGE: &str = "This download link has expired. Press Mod Manager Download on the Nexus page again.";

/// Most characters a link may have (real ones are well under 300).
pub const MAX_LINK_LEN: usize = 2048;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NxmLink {
    /// Nexus's game domain, such as "skyrimspecialedition".
    pub domain: String,
    pub mod_id: u64,
    pub file_id: u64,
    /// The per-download key. Always together with `expires`.
    pub key: Option<String>,
    /// When the key stops working, in seconds since 1970 (UTC).
    pub expires: Option<u64>,
    /// The Nexus account the key was made for.
    pub user_id: Option<u64>,
}

fn domain_ok(d: &str) -> bool {
    (1..=64).contains(&d.len()) && d.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

/// A positive whole number of at most `max_digits` digits, nothing else (no sign, spaces or leading zeros).
fn number(s: &str, max_digits: usize) -> Option<u64> {
    if s.is_empty() || s.len() > max_digits || !s.bytes().all(|b| b.is_ascii_digit()) || s.starts_with('0') {
        return None;
    }
    s.parse().ok()
}

fn key_ok(k: &str) -> bool {
    (1..=128).contains(&k.len()) && k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Reads an nxm link. The error is one plain sentence for the status line.
pub fn parse(link: &str) -> Result<NxmLink, String> {
    let bad = |why: &str| format!("That Nexus download link can't be used ({why}).");
    let link = link.trim();
    if link.len() > MAX_LINK_LEN {
        return Err(bad("it is far too long"));
    }
    if !link.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(bad("it has spaces or unusual characters"));
    }
    let rest = link
        .get(..6)
        .filter(|p| p.eq_ignore_ascii_case("nxm://"))
        .map(|_| &link[6..])
        .ok_or_else(|| bad("it isn't an nxm:// link"))?;
    if rest.contains('#') {
        return Err(bad("it has a part after #"));
    }
    let (path, query) = match rest.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (rest, None),
    };
    let path = path.strip_suffix('/').unwrap_or(path);
    let parts: Vec<&str> = path.split('/').collect();
    let [domain, m, mod_id, f, file_id] = parts.as_slice() else {
        return Err(bad("it doesn't point at one file of one mod"));
    };
    // The host part of a URL is not case sensitive; everything else is.
    let domain = domain.to_ascii_lowercase();
    if !domain_ok(&domain) {
        return Err(bad("the game name in it isn't valid"));
    }
    if *m != "mods" || *f != "files" {
        return Err(bad("it doesn't point at one file of one mod"));
    }
    let mod_id = number(mod_id, 10).ok_or_else(|| bad("the mod number isn't valid"))?;
    let file_id = number(file_id, 12).ok_or_else(|| bad("the file number isn't valid"))?;

    let (mut key, mut expires, mut user_id) = (None, None, None);
    for pair in query.unwrap_or("").split('&').filter(|p| !p.is_empty()) {
        let (k, v) = pair.split_once('=').ok_or_else(|| bad("a part of it has no value"))?;
        let dup = || bad("a part of it appears twice");
        match k {
            "key" => {
                if key.is_some() {
                    return Err(dup());
                }
                if !key_ok(v) {
                    return Err(bad("its key isn't valid"));
                }
                key = Some(v.to_string());
            }
            "expires" => {
                if expires.is_some() {
                    return Err(dup());
                }
                expires = Some(number(v, 12).ok_or_else(|| bad("its expiry time isn't valid"))?);
            }
            "user_id" => {
                if user_id.is_some() {
                    return Err(dup());
                }
                user_id = Some(number(v, 12).ok_or_else(|| bad("its account number isn't valid"))?);
            }
            _ => {
                let name_ok = (1..=32).contains(&k.len()) && k.bytes().all(|b| b.is_ascii_lowercase() || b == b'_');
                let value_ok = v.len() <= 256 && v.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b));
                if !name_ok || !value_ok {
                    return Err(bad("it has a part Turbine doesn't understand"));
                }
            }
        }
    }
    if key.is_some() != expires.is_some() {
        return Err(bad("its key and expiry time must come together"));
    }
    Ok(NxmLink { domain, mod_id, file_id, key, expires, user_id })
}

impl NxmLink {
    /// Store IDs of the Turbine games this link is for (Stardew Valley has a Steam and a GOG ID). Empty when Turbine
    /// doesn't support the game.
    pub fn store_ids(&self) -> Vec<&'static str> {
        nexus::store_ids_for_domain(&self.domain)
    }

    /// The game's name when Turbine supports it.
    pub fn game_name(&self) -> Option<&'static str> {
        nexus::game_name_for_domain(&self.domain)
    }

    pub fn supported(&self) -> bool {
        !self.store_ids().is_empty()
    }

    /// Has the key run out at `now` (seconds since 1970)? A link without a key never expires.
    pub fn expired_at(&self, now: u64) -> bool {
        self.expires.is_some_and(|e| e <= now)
    }

    /// The API path for this file's download link, with the key when the link has one.
    pub fn download_link_path(&self) -> String {
        let mut p = format!("/v1/games/{}/mods/{}/files/{}/download_link.json", self.domain, self.mod_id, self.file_id);
        if let (Some(k), Some(e)) = (&self.key, self.expires) {
            p.push_str(&format!("?key={k}&expires={e}"));
        }
        p
    }

    /// The link without its key, for the activity log.
    pub fn redacted(&self) -> String {
        format!("nxm://{}/mods/{}/files/{}", self.domain, self.mod_id, self.file_id)
    }

    /// The mod's page on the site.
    pub fn mod_page_url(&self) -> String {
        nexus::mod_page_url(&self.domain, self.mod_id)
    }
}

/// Seconds since 1970 now.
pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}
