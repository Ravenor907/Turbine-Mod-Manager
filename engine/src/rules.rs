//! What Turbine knows about each game: where mods go, how archives are laid out, which script loader it uses, the
//! checks before Play, and how to launch it.
//!
//! CONTRACT (other modules and the app rely on these signatures; keep them stable).

use crate::games::Game;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Bethesda games: mods go in Data, plugins have a load order.
    Bethesda,
    /// Unity games using BepInEx (Thunderstore games).
    BepInEx,
    /// Stardew Valley with SMAPI: each mod is a folder in Mods.
    Smapi,
    /// Anything else: files go where the archive says, from the game folder.
    Generic,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Loader {
    /// Plain name, e.g. "SKSE (Skyrim Script Extender)".
    pub name: String,
    /// One sentence on what it is, for the "needs a loader" prompt.
    pub explain: String,
    /// Files (relative to the game folder) whose presence means the loader is installed.
    pub detect: Vec<String>,
    /// Where it comes from (official page), shown to the player.
    pub source: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameRules {
    pub kind: Kind,
    /// Folder (relative to the game folder) most mods go into; "" for the game folder itself.
    pub mod_root: String,
    /// The game has a plugin load order (shows the Load order tab).
    pub plugins: bool,
    /// Mod order matters (shows order numbers and Y to reorder).
    pub ordered: bool,
    pub loader: Option<Loader>,
    /// Shown in Game details: "Verified" or "Works generically".
    pub support: String,
}

/// One line of the check before Play.
#[derive(Debug, Clone, PartialEq)]
pub struct Check {
    /// "ok", "warn" (Play anyway allowed) or "stop" (Play original or Wait only).
    pub level: String,
    pub title: String,
    pub detail: String,
}

impl GameRules {
    /// Rules for a detected game (known games by store ID; everything else Generic).
    pub fn for_game(game: &Game) -> GameRules {
        let _ = game;
        todo!("rules: for_game")
    }

    /// Maps the files of a mod archive (paths inside the archive, '/' separated) to paths inside the game folder.
    /// Files that don't belong (readme, images at the top of a Thunderstore package) may be left out.
    /// `mod_name` is a folder-safe name for games that keep each mod in its own folder.
    pub fn map_archive(&self, files: &[String], mod_name: &str) -> Result<Vec<(String, String)>, String> {
        let _ = (files, mod_name);
        todo!("rules: map_archive")
    }

    /// Does this mod (its game-relative files) need the game's script loader?
    pub fn needs_loader(&self, game_files: &[String]) -> bool {
        let _ = game_files;
        todo!("rules: needs_loader")
    }

    /// Is the loader installed in this game folder?
    pub fn loader_installed(&self, game: &Game) -> bool {
        let _ = game;
        todo!("rules: loader_installed")
    }
}

/// The check before Play. `last_build` is the game build recorded at the last deploy; `needs_loader` is whether any
/// enabled mod needs the loader; `free_bytes` is free space on the game's drive.
pub fn pre_play(game: &Game, rules: &GameRules, last_build: Option<&str>, needs_loader: bool, free_bytes: Option<u64>) -> Vec<Check> {
    let _ = (game, rules, last_build, needs_loader, free_bytes);
    todo!("rules: pre_play")
}

/// The URL that starts the game through its launcher (steam://rungameid/..., Heroic's own link).
pub fn launch_url(game: &Game) -> Option<String> {
    let _ = game;
    todo!("rules: launch_url")
}
