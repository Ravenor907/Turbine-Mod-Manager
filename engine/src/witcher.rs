//! The Witcher 3 (next-gen, 4.0x): the mod priority file, script conflicts, and the settings files Turbine only
//! writes notes about. Where archives go is in rules.rs (map_witcher).
//!
//! - Mods are folders in <game>/mods whose names start with "mod" (others are ignored); DLC-style mods are folders in
//!   <game>/dlc; menu files for a mod's in-game settings go in bin/config/r4game/user_config_matrix/pc
//!   (cdprojektred.atlassian.net, REDkit wiki "Managing mods"; the game's own folder layout).
//! - mods.settings, in Documents/The Witcher 3 (inside the Proton prefix here), sets which mods load and their
//!   priority: one "[modFolder]" section each with "Enabled=1" and "Priority=N"; "lower number means higher
//!   priority" (nexusmods.com/witcher3/mods/1460, Docs tab; REDkit wiki "Managing mods" gives the range 1 to 9999).
//!   Turbine writes it from its own mod order (later in Turbine's list wins, as it does for files), keeps every
//!   section for folders it didn't place exactly as it was, keeps a copy of the file as it was before its first
//!   change, and replaces the file in one step.
//! - input.settings and dx12user.settings (key bindings and graphics settings) are the player's own: some mods ask
//!   for lines to be added to them, and Turbine only says so in the activity log. The same goes for the menu file
//!   lists (dx11filelist.txt, dx12filelist.txt) next to the menu files, which belong to the game.
//! - Two mods that change the same script (content/scripts/...) don't both work: the game uses one. Script Merger, a
//!   Windows tool, combines them into mod0000_MergedFiles; Turbine doesn't run it, it warns before Play instead.

use crate::rules::{lc, mod_folders, Check};
use crate::write_atomic;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const MODS_SETTINGS: &str = "mods.settings";
/// The copy of mods.settings as it was before Turbine first changed it.
pub const BACKUP_NAME: &str = "mods.settings.turbine-backup";
/// The folder Script Merger writes merged scripts to.
pub const MERGED_FOLDER: &str = "mod0000_MergedFiles";
/// Where the game keeps its settings, inside a Proton prefix (pfx folder), for Proton's usual user name.
pub const DOCS_IN_PREFIX: &str = "drive_c/users/steamuser/Documents/The Witcher 3";
pub const NOT_STARTED: &str = "The Witcher 3 hasn't made its settings folder yet. Start the game once, then try again.";

/// The game's settings folder inside a Proton prefix: steamuser's, or the first other user's that has one.
pub fn documents_dir(prefix: &Path) -> PathBuf {
    let usual = prefix.join(DOCS_IN_PREFIX);
    if usual.is_dir() {
        return usual;
    }
    if let Ok(rd) = fs::read_dir(prefix.join("drive_c/users")) {
        let mut users: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
        users.sort();
        for u in users {
            let d = u.join("Documents/The Witcher 3");
            if d.is_dir() {
                return d;
            }
        }
    }
    usual
}

/// One "[name]" section of mods.settings with its lines as they were.
struct Section {
    name: String,
    text: String,
}

fn split_sections(text: &str) -> (String, Vec<Section>) {
    let mut pre = String::new();
    let mut sections: Vec<Section> = Vec::new();
    for line in text.split_inclusive('\n') {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') && t.len() > 2 {
            sections.push(Section { name: t[1..t.len() - 1].trim().to_string(), text: line.to_string() });
        } else if let Some(s) = sections.last_mut() {
            s.text.push_str(line);
        } else {
            pre.push_str(line);
        }
    }
    (pre, sections)
}

/// The new text of mods.settings. `order` is the mod folders that are turned on, in Turbine's order (later wins);
/// `managed` is every mod folder Turbine has placed or may place (on or off). Sections for folders Turbine doesn't
/// manage stay exactly as they were; Turbine's own come after them.
pub fn render_mods_settings(existing: &str, order: &[String], managed: &[String]) -> String {
    let eol = if existing.is_empty() || existing.contains("\r\n") { "\r\n" } else { "\n" };
    let (pre, sections) = split_sections(existing);
    let is_managed = |n: &str| managed.iter().chain(order.iter()).any(|m| m.eq_ignore_ascii_case(n));
    let mut out = pre;
    // Sections are written with one blank line between them; the player's own keep their lines as they were.
    for s in sections.iter().filter(|s| !is_managed(&s.name)) {
        if !out.is_empty() && !out.ends_with(&format!("{eol}{eol}")) {
            out.push_str(eol);
        }
        out.push_str(s.text.trim_end_matches(['\r', '\n']));
        out.push_str(eol);
    }
    let mut seen: Vec<String> = Vec::new();
    let mut wanted: Vec<&String> = Vec::new();
    // A folder listed twice counts where it last appears (the later copy wins).
    for name in order.iter().rev() {
        if !seen.iter().any(|x| x.eq_ignore_ascii_case(name)) {
            seen.push(name.clone());
            wanted.push(name);
        }
    }
    wanted.reverse();
    let n = wanted.len();
    for (i, name) in wanted.iter().enumerate() {
        if !out.is_empty() && !out.ends_with(&format!("{eol}{eol}")) {
            out.push_str(eol);
        }
        out.push_str(&format!("[{name}]{eol}Enabled=1{eol}Priority={}{eol}", n - i));
    }
    out
}

/// Writes mods.settings in the game's settings folder `docs`. Returns true when it changed the file. The first time
/// it changes an existing file, a copy of that file is kept as BACKUP_NAME next to it.
pub fn write_mods_settings(docs: &Path, order: &[String], managed: &[String]) -> Result<bool, String> {
    if !docs.is_dir() {
        return Err(NOT_STARTED.into());
    }
    let path = docs.join(MODS_SETTINGS);
    let fail = |e: io::Error| format!("Couldn't save The Witcher 3's mod list (mods.settings): {e}");
    let old = match fs::read(&path) {
        Ok(b) => Some(b),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(fail(e)),
    };
    let old_text = old.as_deref().map(|b| String::from_utf8_lossy(b).into_owned()).unwrap_or_default();
    let new_text = render_mods_settings(&old_text, order, managed);
    if old.is_some() && new_text == old_text {
        return Ok(false);
    }
    if old.is_none() && order.is_empty() {
        return Ok(false); // nothing to say, and no file to change
    }
    if let Some(bytes) = &old {
        let backup = docs.join(BACKUP_NAME);
        if !backup.exists() {
            write_atomic(&backup, bytes).map_err(fail)?;
        }
    }
    write_atomic(&path, new_text.as_bytes()).map_err(fail)?;
    Ok(true)
}

/// Writes mods.settings for a game in its Proton prefix. `mods` are (game-relative files, turned on) for every mod
/// Turbine manages for the game, in Turbine's order.
pub fn sync_mods_settings(prefix: Option<&Path>, mods: &[(&[String], bool)]) -> Result<bool, String> {
    let Some(prefix) = prefix else { return Err(NOT_STARTED.into()) };
    let mut order = Vec::new();
    let mut managed = Vec::new();
    for (files, on) in mods {
        let folders = mod_folders(files);
        if *on {
            order.extend(folders.iter().cloned());
        }
        managed.extend(folders);
    }
    write_mods_settings(&documents_dir(prefix), &order, &managed)
}

/// Scripts (content/scripts/...) that more than one turned-on mod changes. `mods` are (name, game-relative files).
/// Each conflict is (the script's path under content/scripts, the names of the mods that change it).
pub fn script_conflicts(mods: &[(&str, &[String])]) -> Vec<(String, Vec<String>)> {
    let mut by_script: Vec<(String, String, Vec<String>, bool)> = Vec::new(); // (key, shown path, mods, merged)
    for (name, files) in mods {
        for f in files.iter() {
            let low = lc(f);
            let parts: Vec<&str> = low.split('/').collect();
            if parts.len() < 5 || parts[0] != "mods" || parts[2] != "content" || parts[3] != "scripts" || !low.ends_with(".ws") {
                continue;
            }
            let key = parts[4..].join("/");
            let shown = f.replace('\\', "/").trim_start_matches('/').splitn(5, '/').nth(4).unwrap_or(&key).to_string();
            let merged = parts[1] == MERGED_FOLDER.to_lowercase();
            match by_script.iter_mut().find(|(k, ..)| *k == key) {
                Some(entry) => {
                    if merged {
                        entry.3 = true;
                    } else if !entry.2.iter().any(|m| m == name) {
                        entry.2.push(name.to_string());
                    }
                }
                None => by_script.push((key, shown, if merged { Vec::new() } else { vec![name.to_string()] }, merged)),
            }
        }
    }
    by_script.into_iter().filter(|(_, _, m, merged)| m.len() > 1 && !merged).map(|(_, shown, m, _)| (shown, m)).collect()
}

/// The groups of turned-on mods that change the same scripts: (mod names in load order, the scripts' paths), one per
/// group, in the order `conflict_checks` gives its warnings.
pub fn conflict_groups(mods: &[(&str, &[String])]) -> Vec<(Vec<String>, Vec<String>)> {
    let mut groups: Vec<(Vec<String>, Vec<String>)> = Vec::new();
    for (script, names) in script_conflicts(mods) {
        match groups.iter_mut().find(|(n, _)| *n == names) {
            Some(g) => g.1.push(script),
            None => groups.push((names, vec![script])),
        }
    }
    groups
}

pub const CONFLICT_TITLE: &str = "Only one of these works";

/// The check before Play for script conflicts: one warning per group of mods that change the same scripts, in plain
/// words (the scripts' paths are in Game details).
pub fn conflict_checks(mods: &[(&str, &[String])]) -> Vec<Check> {
    conflict_groups(mods)
        .into_iter()
        .map(|(names, _)| {
            let who = match names.len() {
                2 => format!("{} and {}", names[0], names[1]),
                _ => format!("{} and {}", names[..names.len() - 1].join(", "), names[names.len() - 1]),
            };
            let these = if names.len() == 2 { "These two mods" } else { "These mods" };
            Check {
                level: "warn".into(),
                title: CONFLICT_TITLE.into(),
                detail: format!("{who}: {these} change the same part of the game. Only one can work."),
            }
        })
        .collect()
}

/// Notes for the activity log about a mod's archive (paths inside it): settings lines the player adds by hand, and
/// menu files the game's menu list must name.
pub fn install_notes(archive_files: &[String]) -> Vec<String> {
    let mut notes = Vec::new();
    let names: Vec<String> = archive_files.iter().map(|f| f.replace('\\', "/")).collect();
    let file = |f: &String| f.rsplit('/').next().unwrap_or(f).to_lowercase();
    // Plain words for someone in Game Mode: the files themselves (input.settings, dx12user.settings in the game's
    // Proton folder, under Documents/The Witcher 3) can't be edited there, and the mods work without the lines.
    if names.iter().any(|f| {
        let n = file(f);
        n == "input.settings" || (n.contains("input") && (n.ends_with(".settings") || n.ends_with(".txt")))
    }) {
        notes.push("This mod has extra keys you can set up later on a PC. It works without them.".to_string());
    }
    if names.iter().any(|f| matches!(file(f).as_str(), "user.settings" | "dx11user.settings" | "dx12user.settings")) {
        notes.push("This mod has extra settings you can add later on a PC, if its page asks for them. It works without them.".to_string());
    }
    let menus: Vec<String> = names
        .iter()
        .filter(|f| f.to_lowercase().contains("bin/config/r4game/user_config_matrix/pc/") && file(f).ends_with(".xml"))
        .map(|f| f.rsplit('/').next().unwrap_or(f).to_string())
        .collect();
    if !menus.is_empty() {
        // The menu's file needs a line ("name.xml;") in the game's dx12filelist.txt and dx11filelist.txt, which
        // belong to the game: Turbine doesn't change them.
        notes.push(format!(
            "This mod has its own settings menu ({}). If it doesn't show in the game's Mods menu, it can be added later \
             on a PC (the mod's page says how). The mod works without it.",
            menus.join(", ")
        ));
    }
    notes
}
