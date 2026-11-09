//! Turbine Mod Manager's engine. It has no interface of its own: the app calls into it.

// Newer clippy suggests as_chunks for fixed-size chunks; the explicit form reads better here and works on every
// toolchain the build uses.
#![allow(unknown_lints, clippy::chunks_exact_to_as_chunks)]

pub mod anticheat;
pub mod archives;
pub mod basic;
pub mod browser;
pub mod cyberpunk;
pub mod deploy;
pub mod fomod;
pub mod gamedata;
pub mod games;
pub mod heroic;
pub mod inflate;
pub mod json;
pub mod kcd;
pub mod loaders;
pub mod md5;
pub mod needs;
pub mod net;
pub mod nexus;
pub mod nexus_games;
pub mod nxm;
pub mod overlaps;
pub mod plugins;
pub mod profiles;
pub mod proton;
pub mod rules;
pub mod saves;
pub mod selfupdate;
pub mod shortcuts;
pub mod sha256;
pub mod steam;
pub mod store;
pub mod thunderstore;
pub mod updates;
pub mod vdf;
pub mod watch;
pub mod witcher;
pub mod xml;
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
