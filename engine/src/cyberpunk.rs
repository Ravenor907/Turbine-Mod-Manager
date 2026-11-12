//! Cyberpunk 2077: its modding frameworks (which files make each one, what each needs) and the checks before Play
//! that come from them. Where archives go is in rules.rs (map_cyberpunk).
//!
//! Where things live, from each project's own packaging (shallow clones read Nov 6 2026):
//! - RED4ext (github.com/wopss/RED4ext, commit 9f135ac): CMakeLists.txt lines 131 to 162 install the loader DLL to
//!   bin/x64 and RED4ext.dll to red4ext/; src/loader/CMakeLists.txt line 6 names the loader "winmm" (so Wine must load
//!   bin/x64/winmm.dll from the game folder); src/dll/Paths.cpp line 48 loads plugins from red4ext/plugins.
//! - Cyber Engine Tweaks (github.com/maximegmd/CyberEngineTweaks, commit 20b42c6): xmake.lua lines 132 to 161 package
//!   bin/x64/plugins/cyber_engine_tweaks.asi, its data under bin/x64/plugins/cyber_engine_tweaks/, and an ASI loader
//!   (vendor/asiloader: version.dll and global.ini) in bin/x64 (so Wine must load the game folder's version.dll);
//!   src/Paths.cpp line 81 loads Lua mods from bin/x64/plugins/cyber_engine_tweaks/mods; README.md line 45: "You
//!   first need to install RED4ext".
//! - redscript (github.com/jac3km4/redscript, commit e83eeba): scripts/package.ps1 packages engine/tools/scc.exe,
//!   engine/tools/scc_lib.dll, engine/config/base/scripts.ini and r6/config/cybercmd/scc.toml; mods are .reds files
//!   under r6/scripts (wiki.redmodding.org, "Core Mods explained").
//! - ArchiveXL (github.com/psiberx/cp2077-archive-xl, commit 8210dae), TweakXL (psiberx/cp2077-tweak-xl, commit
//!   3875f1d) and Codeware (psiberx/cp2077-codeware, commit 6418bc4): tools/dist/steps/compose-red4ext.ps1 puts each
//!   in red4ext/plugins/<Name>/<Name>.dll, so each needs RED4ext; TweakXL's make-tweaks-dir.ps1 makes r6/tweaks, where
//!   its .yaml/.tweak mods go; ArchiveXL's ExtensionService.cpp line 31 loads ".xl" files that sit next to .archive
//!   files in archive/pc/mod.
//! - REDmod (cyberpunk.net/en/modding-support, wiki.redmodding.org "REDmod: Usage"): mods go in mods/<name>/ with an
//!   info.json; the tool is tools/redmod/bin/redMod.exe; mods only load when the game starts with -modded, after
//!   `redmod deploy` has compiled their scripts, tweaks and sounds. The game's own launcher does both when "Enable
//!   mods" is on in its settings. Turbine doesn't run redMod.exe (a Windows program), so it explains this once.
//! - Nexus mod IDs: wiki.redmodding.org "Core Mods explained" links each framework's Nexus page.

use crate::rules::{lc, Check};

/// One framework other mods build on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Framework {
    pub key: &'static str,
    pub name: &'static str,
    /// One sentence on what it is.
    pub explain: &'static str,
    /// The game-relative file (lowercase) whose presence means it is installed.
    pub detect: &'static str,
    /// Its Nexus mod ID (cyberpunk2077).
    pub nexus_mod: u64,
    /// Keys of the frameworks it needs.
    pub requires: &'static [&'static str],
    /// The DLL Wine must load from the game folder for it to start, if any ("winmm", "version").
    pub dll: Option<&'static str>,
}

pub const FRAMEWORKS: &[Framework] = &[
    Framework {
        key: "red4ext",
        name: "RED4ext",
        explain: "RED4ext loads plugins that add new code to the game; many popular mods are built on it.",
        detect: "red4ext/red4ext.dll",
        nexus_mod: 2380,
        requires: &[],
        dll: Some("winmm"),
    },
    Framework {
        key: "cet",
        name: "Cyber Engine Tweaks",
        explain: "Cyber Engine Tweaks runs Lua mods and adds an in-game overlay for their settings.",
        detect: "bin/x64/plugins/cyber_engine_tweaks.asi",
        nexus_mod: 107,
        requires: &["red4ext"],
        dll: Some("version"),
    },
    Framework {
        key: "redscript",
        name: "redscript",
        explain: "redscript compiles script mods (.reds files) into the game when it starts.",
        detect: "engine/tools/scc.exe",
        nexus_mod: 1511,
        requires: &[],
        dll: None,
    },
    Framework {
        key: "archivexl",
        name: "ArchiveXL",
        explain: "ArchiveXL lets mods add new items, clothes and other content without replacing the game's own.",
        detect: "red4ext/plugins/archivexl/archivexl.dll",
        nexus_mod: 4198,
        requires: &["red4ext"],
        dll: None,
    },
    Framework {
        key: "tweakxl",
        name: "TweakXL",
        explain: "TweakXL lets mods change the game's built-in values (prices, stats, item records).",
        detect: "red4ext/plugins/tweakxl/tweakxl.dll",
        nexus_mod: 4197,
        requires: &["red4ext"],
        dll: None,
    },
    Framework {
        key: "codeware",
        name: "Codeware",
        explain: "Codeware is a shared library many script and Lua mods use.",
        detect: "red4ext/plugins/codeware/codeware.dll",
        nexus_mod: 7780,
        requires: &["red4ext"],
        dll: None,
    },
];

pub fn framework(key: &str) -> Option<&'static Framework> {
    FRAMEWORKS.iter().find(|f| f.key == key)
}

/// Where the REDmod tool is, relative to the game folder (it comes with the free REDmod DLC).
pub const REDMOD_TOOL: &str = "tools/redmod/bin/redMod.exe";
/// The title of the one-time REDmod explanation, so the app can tell it was seen.
pub const REDMOD_TITLE: &str = "REDmod mods need the game's launcher";
pub const REDMOD_HELP: &str = "Some of your mods are REDmod mods. They load only after Cyberpunk 2077's own launcher \
sets them up, a step Turbine can't do.\
\n• When the launcher opens after Play, press Settings there and turn on Enable mods.\
\n• Then press Play in the launcher: it sets the mods up (a progress window shows) and starts the game.\
\n• You do this once: the launcher redoes it automatically when your mods change.\
\n• Remove --launcher-skip from the game's Launch Options in Steam if you added it there.";
pub const REDMOD_MISSING: &str = "REDmod isn't installed. It's a free add-on: in Steam, open Cyberpunk 2077's page and \
install REDmod from its downloadable content. On GOG, install it where you installed the game.";

/// The frameworks these files (game-relative) are, by their detect file.
pub fn provides(files: &[String]) -> Vec<&'static str> {
    let files: Vec<String> = files.iter().map(|f| lc(f)).collect();
    FRAMEWORKS.iter().filter(|fw| files.iter().any(|f| f == fw.detect)).map(|fw| fw.key).collect()
}

/// The frameworks these files (game-relative) need, the frameworks among them included (ArchiveXL needs RED4ext).
/// Frameworks the files provide themselves are left out.
pub fn needs(files: &[String]) -> Vec<&'static str> {
    let own = provides(files);
    let mut out: Vec<&'static str> = Vec::new();
    fn add(out: &mut Vec<&'static str>, k: &'static str) {
        if !out.contains(&k) {
            out.push(k);
        }
    }
    for f in files.iter().map(|f| lc(f)) {
        let ext = f.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
        if f.starts_with("bin/x64/plugins/cyber_engine_tweaks/mods/") {
            add(&mut out, "cet");
        }
        if f.starts_with("red4ext/plugins/") && ext == "dll" {
            add(&mut out, "red4ext");
        }
        if f.starts_with("r6/scripts/") && ext == "reds" {
            add(&mut out, "redscript");
        }
        if f.starts_with("r6/tweaks/") && matches!(ext, "yaml" | "yml" | "tweak") {
            add(&mut out, "tweakxl");
        }
        if ext == "xl" && (f.starts_with("archive/pc/mod/") || (f.starts_with("mods/") && f.contains("/archives/"))) {
            add(&mut out, "archivexl");
        }
    }
    for k in &own {
        for r in framework(k).map(|f| f.requires).unwrap_or(&[]) {
            add(&mut out, r);
        }
    }
    // What a needed framework needs in turn (CET needs RED4ext).
    let mut i = 0;
    while i < out.len() {
        for r in framework(out[i]).map(|f| f.requires).unwrap_or(&[]) {
            add(&mut out, r);
        }
        i += 1;
    }
    out.retain(|k| !own.contains(k));
    out
}

/// Does this mod (game-relative files) carry REDmod mods (mods/<name>/info.json)?
pub fn has_redmod(files: &[String]) -> bool {
    files.iter().map(|f| lc(f)).any(|f| {
        let parts: Vec<&str> = f.split('/').collect();
        parts.len() == 3 && parts[0] == "mods" && parts[2] == "info.json"
    })
}

/// The DLLs Wine must load from the game folder for these frameworks (game-relative files of what is turned on and
/// what is already in the game folder).
pub fn wanted_dlls(files: &[String]) -> Vec<&'static str> {
    provides(files).iter().filter_map(|k| framework(k).and_then(|f| f.dll)).collect()
}

/// The checks before Play for Cyberpunk: missing frameworks and the REDmod explanation. `mods` are the turned-on
/// mods' (name, game-relative files); `game_files` are files already in the game folder (only the frameworks'
/// detect files and REDMOD_TOOL are looked at).
pub fn checks(mods: &[(&str, &[String])], game_files: &[String]) -> Vec<Check> {
    let mut present: Vec<&'static str> = provides(game_files);
    for (_, files) in mods {
        for k in provides(files) {
            if !present.contains(&k) {
                present.push(k);
            }
        }
    }
    let mut out = Vec::new();
    for fw in FRAMEWORKS {
        if present.contains(&fw.key) {
            continue;
        }
        let users: Vec<&str> = mods.iter().filter(|(_, files)| needs(files).contains(&fw.key)).map(|(n, _)| *n).collect();
        if users.is_empty() {
            continue;
        }
        let who = match users.len() {
            1 => format!("{} needs it.", users[0]),
            2 => format!("{} and {} need it.", users[0], users[1]),
            n => format!("{}, {} and {} more need it.", users[0], users[1], n - 2),
        };
        out.push(Check {
            level: "warn".into(),
            title: format!("{} is missing", fw.name),
            detail: format!(
                "{who} {} Without it those mods do nothing. Get it from {}",
                fw.explain,
                crate::nexus::mod_page_url("cyberpunk2077", fw.nexus_mod)
            ),
        });
    }
    if mods.iter().any(|(_, files)| has_redmod(files)) {
        let tool = REDMOD_TOOL.to_lowercase();
        let mut detail = REDMOD_HELP.to_string();
        if !game_files.iter().any(|f| lc(f) == tool) {
            detail = format!("{REDMOD_MISSING} {detail}");
        }
        out.push(Check { level: "warn".into(), title: REDMOD_TITLE.into(), detail });
    }
    out
}

/// The game-relative files `checks` and `wanted_dlls` look for in the game folder.
pub fn files_to_look_for() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = FRAMEWORKS.iter().map(|f| f.detect).collect();
    v.push(REDMOD_TOOL);
    v
}
