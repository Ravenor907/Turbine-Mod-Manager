//! Turbine Mod Manager's engine. It has no interface of its own: the app calls into it.

pub mod deploy;
pub mod games;
pub mod heroic;
pub mod inflate;
pub mod json;
pub mod proton;
pub mod rules;
pub mod steam;
pub mod store;
pub mod vdf;
pub mod zip;

pub use games::{Game, Runtime, Scan, Store};

use std::path::Path;

/// Every installed Steam and Heroic game under this home folder, sorted by name.
pub fn scan_games(home: &Path) -> Scan {
    let mut scan = Scan::default();
    steam::scan(home, &mut scan);
    heroic::scan(home, &mut scan);
    scan.games.sort_by(|a, b| {
        a.name.to_lowercase().cmp(&b.name.to_lowercase()).then(a.store.cmp(&b.store))
    });
    scan
}

/// The real home folder. Inside the Flatpak, HOME is still the player's home (only XDG_* folders are redirected),
/// which is where Steam and Heroic keep their files.
/// TURBINE_HOME points it somewhere else for the build's screenshot checks.
pub fn home_dir() -> std::path::PathBuf {
    std::env::var_os("TURBINE_HOME")
        .or_else(|| std::env::var_os("HOME"))
        .map(Into::into)
        .unwrap_or_else(|| "/home/deck".into())
}
