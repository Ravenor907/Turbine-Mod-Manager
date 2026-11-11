//! Kingdom Come: Deliverance (2018) and Kingdom Come: Deliverance II (2025): the mod order file and the checks before
//! Play. Where archives go is in rules.rs (map_kcd).
//!
//! What the games expect, with where each fact comes from (all read Nov 7 2026):
//! - Warhorse's own guide, "Modding in Kingdom Come: Deliverance 2" (deepsilver.com/games/kingdom-come-deliverance-ii/
//!   news/modding-in-kingdom-come-deliverance-2): mods installed by hand go in the "mods" folder in the game's main
//!   folder, one unpacked folder per mod, and each mod folder must hold a mod.manifest. The released game loads no
//!   loose files except mod.manifest and mod.cfg: everything else is packed into uncompressed zip .pak files. Order:
//!   Steam Workshop mods first, then the mods folder alphabetically, unless "mod_order.txt" in the mods folder sets
//!   the order; mods not listed in mod_order.txt do not load.
//! - The paks sit in the mod folder's Data folder (Mods/<mod>/Data/<name>.pak), text in Localization/*.pak, the way
//!   the first game's mods are laid out (forum.kingdomcomerpg.com/t/please-help-cant-get-mods-to-work-besides-
//!   ultimate-saving/62676: "Mod Name\Data\Mod.pak", and a manifest of <kcd_mod><info>name, description, author,
//!   version, created_on</info><supports><kcd_version>1.*</kcd_version></supports></kcd_mod>).
//! - The first game: some mods and console commands need developer mode, the "-devmode" launch option
//!   (nexusmods.com/kingdomcomedeliverance/articles/170: Steam, the game's Properties, Launch Options; GOG, a shortcut
//!   to Bin/Win64/KingdomCome.exe with -devmode). Most mods don't.
//! - The second game: a user.cfg of settings is read only with "-devmode +exec user.cfg" in the launch options, and
//!   the file sits in the game's main folder on Steam (nexusmods.com/kingdomcomedeliverance2/articles/20).
//!
//! mod_order.txt: Turbine writes one mod folder name per line, its turned-on mods in its own order (later in the list
//! wins, the same as for files; the forum thread above writes folder names one per line). Lines for folders Turbine
//! doesn't manage stay as they were, before Turbine's. When there was no mod_order.txt, the game loaded every folder
//! alphabetically, so the player's own folders go in first, alphabetically, so they keep loading. A copy of the file
//! as it was before Turbine's first change is kept in Turbine's own folder (never in the game's), and a file Turbine
//! made itself is taken away again when it has nothing left to list.

use crate::rules::{lc, mod_folders, Check};
use crate::write_atomic;
use std::fs;
use std::io;
use std::path::Path;

pub const ORDER_FILE: &str = "mod_order.txt";
/// The copy of mod_order.txt as it was before Turbine first changed it (in Turbine's folder for the game).
pub const BACKUP_NAME: &str = "kcd-mod_order.txt.before";
/// Marks that Turbine made mod_order.txt itself (in Turbine's folder for the game).
pub const MADE_MARK: &str = "kcd-mod_order.txt.made";
/// The Mods folders Turbine saw at its last write (in Turbine's folder for the game), so a folder the player adds by
/// hand later is listed too (a folder missing from mod_order.txt doesn't load).
pub const SEEN_NAME: &str = "kcd-mods-seen.txt";
pub const DEVMODE_TITLE: &str = "One-time setting for some mods";

/// A folder name the games and mod_order.txt take: letters, digits and '_' only ("Better Lockpicking 1.2" becomes
/// "BetterLockpicking12").
pub fn folder_name(name: &str) -> String {
    let n: String = name.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
    if n.is_empty() {
        "mod".into()
    } else {
        n
    }
}

/// The new text of mod_order.txt. `existing`: the file as it is (None when there is none); `on_disk`: the folders in
/// the Mods folder now; `order`: Turbine's turned-on mod folders in its order; `managed`: every folder Turbine places
/// or may place (on or off); `add`: the player's own folders that are new since Turbine last looked, listed after
/// the player's lines, alphabetically, before Turbine's.
pub fn render_mod_order_adding(existing: Option<&str>, on_disk: &[String], order: &[String], managed: &[String], add: &[String]) -> String {
    let eol = if existing.is_some_and(|t| t.contains("\r\n")) { "\r\n" } else { "\n" };
    let is_managed = |n: &str| managed.iter().chain(order.iter()).any(|m| m.eq_ignore_ascii_case(n.trim()));
    let mut lines: Vec<String> = Vec::new();
    match existing {
        Some(text) => {
            for l in text.lines() {
                let l = l.trim_end_matches('\r');
                if !l.trim().is_empty() && !is_managed(l) {
                    lines.push(l.to_string());
                }
            }
        }
        None => {
            let mut own: Vec<&String> = on_disk.iter().filter(|f| !is_managed(f)).collect();
            own.sort_by_key(|f| f.to_lowercase());
            lines.extend(own.into_iter().cloned());
        }
    }
    let mut new_own: Vec<&String> = add.iter().filter(|f| !is_managed(f)).collect();
    new_own.sort_by_key(|f| f.to_lowercase());
    for name in new_own.into_iter().chain(order.iter()) {
        if !lines.iter().any(|l| l.trim().eq_ignore_ascii_case(name)) {
            lines.push(name.clone());
        }
    }
    if lines.is_empty() {
        return String::new();
    }
    let mut out = lines.join(eol);
    out.push_str(eol);
    out
}

/// Writes <game>/Mods/mod_order.txt from Turbine's mods. `mods` are (game-relative files, turned on) for every mod
/// Turbine manages for the game, in Turbine's order; `keep` is Turbine's own folder for the game (the copy of the
/// player's file and the mark that Turbine made it go there). Returns true when the file changed.
pub fn sync_mod_order(game_dir: &Path, keep: &Path, mods: &[(&[String], bool)]) -> Result<bool, String> {
    let mut order = Vec::new();
    let mut managed = Vec::new();
    for (files, on) in mods {
        let folders = mod_folders(files);
        if *on {
            order.extend(folders.iter().cloned());
        }
        managed.extend(folders);
    }
    let Some(dir) = crate::rules::find_ci(game_dir, "Mods").filter(|d| d.is_dir()) else {
        return Ok(false); // no Mods folder: no mod of Turbine's is in, and the player has none either
    };
    let fail = |e: io::Error| format!("Couldn't save the mods' order (Mods/mod_order.txt): {e}");
    let path = crate::rules::find_ci(&dir, ORDER_FILE).unwrap_or_else(|| dir.join(ORDER_FILE));
    let old = match fs::read(&path) {
        Ok(b) => Some(String::from_utf8_lossy(&b).into_owned()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(fail(e)),
    };
    let on_disk: Vec<String> = fs::read_dir(&dir)
        .map(|rd| rd.flatten().filter(|e| e.path().is_dir()).map(|e| e.file_name().to_string_lossy().into_owned()).collect())
        .unwrap_or_default();
    let made = keep.join(MADE_MARK);
    // The player's own folders that are new: every one when Turbine made the file (without a file the game loaded
    // them all), otherwise the ones that weren't there at Turbine's last write (a folder the player left out of their
    // own file on purpose stays out).
    let own_now: Vec<String> = on_disk
        .iter()
        .filter(|f| !managed.iter().chain(order.iter()).any(|m| m.eq_ignore_ascii_case(f)))
        .cloned()
        .collect();
    let seen_path = keep.join(SEEN_NAME);
    let seen: Option<Vec<String>> = fs::read_to_string(&seen_path).ok().map(|t| t.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect());
    let add: Vec<String> = match (&old, &seen) {
        (Some(_), _) if made.exists() => own_now.clone(),
        (Some(_), Some(seen)) => own_now.iter().filter(|f| !seen.iter().any(|s| s.eq_ignore_ascii_case(f))).cloned().collect(),
        _ => Vec::new(),
    };
    let mut seen_text: Vec<String> = own_now.clone();
    seen_text.sort_by_key(|f| f.to_lowercase());
    let seen_text = if seen_text.is_empty() { String::new() } else { seen_text.join("\n") + "\n" };
    let remember = |changed: bool| -> Result<bool, String> {
        if fs::read_to_string(&seen_path).ok().as_deref() != Some(seen_text.as_str()) && fs::create_dir_all(keep).is_ok() {
            let _ = write_atomic(&seen_path, seen_text.as_bytes());
        }
        Ok(changed)
    };
    let text = render_mod_order_adding(old.as_deref(), &on_disk, &order, &managed, &add);
    if text.is_empty() {
        // Nothing to list: a file Turbine made goes again; the player's own stays as it is.
        if old.is_some() && made.exists() {
            fs::remove_file(&path).map_err(fail)?;
            let _ = fs::remove_file(&made);
            return remember(true);
        }
        if old.is_none() {
            return remember(false);
        }
    }
    if old.as_deref() == Some(text.as_str()) {
        return remember(false);
    }
    fs::create_dir_all(keep).map_err(fail)?;
    match &old {
        Some(t) if !made.exists() && !keep.join(BACKUP_NAME).exists() => write_atomic(&keep.join(BACKUP_NAME), t.as_bytes()).map_err(fail)?,
        None => write_atomic(&made, b"").map_err(fail)?,
        _ => {}
    }
    write_atomic(&path, text.as_bytes()).map_err(fail)?;
    remember(true)
}

/// Is there a mod_order.txt of the player's to put back? (Turbine kept a copy, or Turbine made the file itself.)
pub fn can_restore(keep: &Path) -> bool {
    keep.join(BACKUP_NAME).is_file() || keep.join(MADE_MARK).exists()
}

/// Puts the player's mod_order.txt back as it was before Turbine first changed it: the kept copy goes back in place
/// (the copy stays, so this can be done again); a file Turbine made itself is taken away. Turbine's turned-on mods
/// are written into it again at the next change or Play. Returns true when something was put back.
pub fn restore_mod_order(game_dir: &Path, keep: &Path) -> Result<bool, String> {
    let fail = |e: io::Error| format!("Couldn't put back mod_order.txt: {e}");
    let Some(dir) = crate::rules::find_ci(game_dir, "Mods").filter(|d| d.is_dir()) else {
        return Ok(false);
    };
    let path = crate::rules::find_ci(&dir, ORDER_FILE).unwrap_or_else(|| dir.join(ORDER_FILE));
    let backup = keep.join(BACKUP_NAME);
    let made = keep.join(MADE_MARK);
    if backup.is_file() {
        let bytes = fs::read(&backup).map_err(fail)?;
        write_atomic(&path, &bytes).map_err(fail)?;
        let _ = fs::remove_file(&made);
        return Ok(true);
    }
    if made.exists() {
        if path.exists() {
            fs::remove_file(&path).map_err(fail)?;
        }
        let _ = fs::remove_file(&made);
        return Ok(true);
    }
    Ok(false)
}

/// What the files of one mod folder hold.
fn folder_facts(files: &[String], folder: &str) -> (bool, bool) {
    let prefix = format!("mods/{}/", folder.to_lowercase());
    let inside: Vec<String> = files.iter().map(|f| lc(f)).filter(|f| f.starts_with(&prefix)).map(|f| f[prefix.len()..].to_string()).collect();
    let manifest = inside.iter().any(|f| f == "mod.manifest");
    let paks = inside.iter().any(|f| (f.starts_with("data/") || f.starts_with("localization/")) && f.ends_with(".pak"));
    (manifest, paks)
}

/// The checks before Play that look at each turned-on mod. `on`: (name, game-relative files); `second`: the game is
/// Kingdom Come: Deliverance II; `other_game`: names of turned-on mods that came from the other game's Nexus pages.
pub fn checks(on: &[(&str, &[String])], second: bool, other_game: &[String]) -> Vec<Check> {
    let mut out = Vec::new();
    let this = if second { "Kingdom Come: Deliverance II" } else { "Kingdom Come: Deliverance" };
    let other = if second { "Kingdom Come: Deliverance" } else { "Kingdom Come: Deliverance II" };
    for (name, files) in on {
        let folders = mod_folders(files);
        let mut empty = folders.is_empty() && !files.is_empty() && files.iter().all(|f| lc(f).starts_with("mods/"));
        let mut no_manifest = false;
        for f in &folders {
            let (manifest, paks) = folder_facts(files, f);
            if !manifest && !paks {
                empty = true;
            } else if !manifest && second {
                no_manifest = true;
            }
        }
        if empty {
            out.push(Check {
                level: "warn".into(),
                title: format!("{name} may not load"),
                detail: format!(
                    "Its folder has no mod.manifest and no .pak files in Data, and {this} only loads mods that \
                     have them. Check the mod's page: it may need a different file."
                ),
            });
        } else if no_manifest {
            out.push(Check {
                level: "warn".into(),
                title: format!("{name} has no mod.manifest"),
                detail: format!(
                    "{this} only loads mods whose folder has a mod.manifest file. This one may be made for \
                     {other}, or need a newer version from its page."
                ),
            });
        }
    }
    for name in other_game {
        out.push(Check {
            level: "warn".into(),
            title: format!("{name} is made for {other}"),
            detail: format!(
                "It came from {other}'s mod pages. Mods for one game usually don't work in the other and can stop it \
                 from starting. Turn it off, or Play anyway to try it."
            ),
        });
    }
    out
}

/// A short word for a mod's row in My mods (shown on it the way the check before Play says it), or None:
/// "Made for KCD" or "Made for KCD II" (named for, or downloaded from, the other game), "May not load" (no
/// mod.manifest and no .pak files, or no mod.manifest in the second game). `from_other_game`: it came from the other
/// game's Nexus pages.
pub fn mod_badge(name: &str, files: &[String], second: bool, from_other_game: bool) -> Option<String> {
    if from_other_game || name_says_other(name, second) {
        return Some(if second { "Made for KCD".into() } else { "Made for KCD II".into() });
    }
    let folders = mod_folders(files);
    let mut flagged = folders.is_empty() && !files.is_empty() && files.iter().all(|f| lc(f).starts_with("mods/"));
    for f in &folders {
        let (manifest, paks) = folder_facts(files, f);
        if (!manifest && !paks) || (!manifest && second) {
            flagged = true;
        }
    }
    flagged.then(|| "May not load".to_string())
}

/// The first game's one-time note about developer mode, for players whose mods ask for it. `steam`: the copy runs
/// through Steam (otherwise Heroic).
pub fn devmode_check(steam: bool) -> Check {
    let how = if steam {
        "in Steam, open the game's Properties and type exactly the line below under Launch Options"
    } else {
        "in Heroic, open the game's settings and add the line below under Game Arguments"
    };
    Check {
        level: "warn".into(),
        title: DEVMODE_TITLE.into(),
        detail: format!(
            "Most Kingdom Come: Deliverance mods work as they are. A few (cheat menus, console tweaks) say on their \
             page that they need developer mode. Only for those: {how}. You only do this once.\n-devmode"
        ),
    }
}

/// Is this name a hint the mod is for the other game? ("KCD2" or "KCD II" in a first-game mod's name, "KCD1" in a
/// second-game one.)
pub fn name_says_other(name: &str, second: bool) -> bool {
    let n = name.to_lowercase().replace(['_', '-', '.'], " ");
    let words: Vec<&str> = n.split_whitespace().collect();
    let has = |w: &str| words.contains(&w);
    if second {
        has("kcd1") || n.contains("kcd 1 ")
    } else {
        has("kcd2") || n.contains("kcd ii") || n.contains("kcd 2") || n.contains("deliverance ii") || n.contains("deliverance 2")
    }
}
