//! Turbine Mod Manager's engine. It has no interface of its own: the app calls into it.

// Newer clippy suggests as_chunks for fixed-size chunks; the explicit form reads better here and works on every
// toolchain the build uses.
#![allow(unknown_lints, clippy::chunks_exact_to_as_chunks)]

pub mod anticheat;
pub mod archives;
pub mod basic;
pub mod browser;
pub mod compat_tool;
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
pub mod quick;
pub mod rules;
pub mod report;
pub mod saves;
pub mod selfupdate;
pub mod settings;
pub mod shortcuts;
pub mod sha256;
pub mod steam;
pub mod steam_art;
pub mod steam_restart;
pub mod steam_setup;
pub mod store;
pub mod thunderstore;
pub mod updates;
pub mod vcrun;
pub mod vdf;
pub mod watch;
pub mod witcher;
pub mod wsproto;
pub mod xml;
pub mod zip;

pub use games::{Game, Runtime, Scan, Store};

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

/// Writes a file whole or not at all: a hidden file beside it first (synced), then renamed over it, then the folder
/// synced so the rename itself lasts through a power cut (matters on SD cards). Makes the folder when it is missing.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp = path.with_file_name(format!(".{name}.turbine-new"));
    {
        let mut f = File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    if let Some(p) = path.parent() {
        if let Ok(d) = File::open(p) {
            let _ = d.sync_all();
        }
    }
    Ok(())
}

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
