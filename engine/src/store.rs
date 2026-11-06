//! A game's mods: staged files, on/off and order, Recently removed (7 days, with undo), setups that worked, and
//! applying it all to the game folder through `deploy`.
//!
//! CONTRACT (the app relies on these signatures; keep them stable).

use crate::deploy::Report;
use crate::games::Game;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct Mod {
    /// Stable ID, unique within the game.
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub version: Option<String>,
    /// Where it came from, in words: "Zip file", later "Thunderstore", "Nexus Mods".
    pub source: String,
    pub files: usize,
    pub needs_loader: bool,
    /// Seconds since 1970.
    pub added: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Removed {
    pub item: Mod,
    pub removed_at: u64,
}

/// A setup (which mods on, in which order) saved after a deploy, to go back to.
#[derive(Debug, Clone, PartialEq)]
pub struct Setup {
    pub id: String,
    pub saved_at: u64,
    pub mods_on: usize,
}

pub struct Store {
    _private: (),
}

impl Store {
    /// Opens (or creates) the store for a game. Mods are staged on the same drive as the game so they can be hard
    /// linked: under `data_home` for internal storage, in a hidden folder on the SD card or other drive otherwise.
    pub fn open(game: &Game, data_home: &Path) -> io::Result<Store> {
        let _ = (game, data_home);
        todo!("store: open")
    }

    pub fn mods(&self) -> &[Mod] {
        todo!("store: mods")
    }

    pub fn removed(&self) -> &[Removed] {
        todo!("store: removed")
    }

    pub fn setups(&self) -> &[Setup] {
        todo!("store: setups")
    }

    /// Adds a mod from a zip file. Returns the new mod (enabled, at the end of the order).
    pub fn import_zip(&mut self, archive: &Path) -> Result<Mod, String> {
        let _ = archive;
        todo!("store: import_zip")
    }

    pub fn set_enabled(&mut self, id: &str, on: bool) -> io::Result<()> {
        let _ = (id, on);
        todo!("store: set_enabled")
    }

    /// Moves a mod to a position in the order (0 = first, lowest priority).
    pub fn move_to(&mut self, id: &str, index: usize) -> io::Result<()> {
        let _ = (id, index);
        todo!("store: move_to")
    }

    /// Takes a mod out of the game and keeps it in Recently removed for 7 days.
    pub fn remove(&mut self, id: &str) -> io::Result<()> {
        let _ = id;
        todo!("store: remove")
    }

    pub fn undo_remove(&mut self, id: &str) -> io::Result<()> {
        let _ = id;
        todo!("store: undo_remove")
    }

    /// Makes the game folder match the enabled mods in order. Saves a setup when it changed something.
    pub fn apply(&mut self) -> io::Result<Report> {
        todo!("store: apply")
    }

    /// Takes every mod file out of the game (nothing is deleted; mods stay on/off as they were).
    pub fn play_original(&mut self) -> io::Result<Report> {
        todo!("store: play_original")
    }

    /// Goes back to a saved setup and applies it.
    pub fn restore(&mut self, setup_id: &str) -> io::Result<Report> {
        let _ = setup_id;
        todo!("store: restore")
    }

    /// The game build recorded at the last apply (for the update guard).
    pub fn last_build(&self) -> Option<String> {
        todo!("store: last_build")
    }

    /// Everything the interface shows, as one JSON object: {"mods":[...],"removed":[...],"setups":[...]}.
    pub fn to_json(&self) -> String {
        todo!("store: to_json")
    }
}

/// Zip files in the Downloads folder, newest first.
pub fn downloads(home: &Path) -> Vec<PathBuf> {
    let _ = home;
    todo!("store: downloads")
}
