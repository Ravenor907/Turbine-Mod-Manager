//! 0.62: Turbine's own Steam compatibility tool, "Turbine (mods)".
//!
//! Game Mode starts Steam again the moment it closes, so Turbine can't write Steam's settings files there (Steam
//! writes its own copy back). A compatibility tool needs none of that: it is a new folder under Steam's
//! compatibilitytools.d, which Steam only reads, and the player picks it once per game in the game's Properties,
//! Compatibility. Steam sees new tools when it starts.
//!
//! The folder holds three files:
//! - compatibilitytool.vdf: the tool's name in Steam's list (for Windows games, run on Linux).
//! - toolmanifest.vdf: the command Steam runs (`turbine-tool <verb>`, then the game's program).
//! - turbine-tool: a script. A tool chosen in place of Proton gets no Proton from Steam, so the script builds the
//!   line Steam builds itself: the Steam Linux Runtime the chosen Proton asks for (from Proton's own toolmanifest),
//!   then that Proton, then the game. For a game launch it hands that line to Turbine's launch helper
//!   (anticheat::helper_text), which keeps deciding the same way as before: mods on -> me3.exe in place of the game's
//!   program, otherwise the normal game. Without the helper (Turbine's data gone) the script runs the plain line, so
//!   the game always starts.
//!
//! Which Proton: the one Turbine wrote down for the game (`<app id>.proton` in the helper's folder: the Proton Steam
//! last used for it, from the helper's own log), else `default.proton`, else the newest numbered Proton installed,
//! else Proton Experimental, Hotfix, then other tools in compatibilitytools.d. The script says which one and why in
//! steam-launch.log.
//!
//! Turbine only reads Steam's config.vdf (CompatToolMapping) to see whether a game uses the tool. A name Steam
//! doesn't know starts the game with no tool at all, so the folder is never removed while a game still names it.

use crate::vdf;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// The tool's internal name (what Steam keeps in CompatToolMapping).
pub const TOOL_NAME: &str = "turbine-mods";
/// The name in Steam's Compatibility list.
pub const DISPLAY_NAME: &str = "Turbine (mods)";
/// The tool's folder under compatibilitytools.d.
pub const FOLDER: &str = "TurbineMods";
/// The script Steam runs.
pub const SCRIPT: &str = "turbine-tool";

/// Steam's folder for tools that don't come from Steam itself.
pub fn tools_dir(steam_root: &Path) -> PathBuf {
    steam_root.join("compatibilitytools.d")
}

/// Turbine's tool folder.
pub fn tool_dir(steam_root: &Path) -> PathBuf {
    tools_dir(steam_root).join(FOLDER)
}

/// compatibilitytool.vdf: the tool as Steam lists it.
pub fn compatibilitytool_vdf() -> String {
    format!(
        "\"compatibilitytools\"\n{{\n  \"compat_tools\"\n  {{\n    \"{TOOL_NAME}\"\n    {{\n      \"install_path\" \".\"\n      \"display_name\" \"{DISPLAY_NAME}\"\n      \"from_oslist\"  \"windows\"\n      \"to_oslist\"    \"linux\"\n    }}\n  }}\n}}\n"
    )
}

/// toolmanifest.vdf: the command Steam runs. No required tool: the script starts the runtime the chosen Proton
/// needs itself (one tool can name only one runtime, and different Protons need different ones).
pub fn toolmanifest_vdf() -> String {
    format!("\"manifest\"\n{{\n  \"version\" \"2\"\n  \"commandline\" \"/{SCRIPT} %verb%\"\n  \"use_sessions\" \"1\"\n}}\n")
}

/// The script. `launch_dir`: the helper's folder in Turbine's data folder (anticheat::launch_dir).
pub fn script_text(launch_dir: &Path) -> String {
    let dir = launch_dir.to_string_lossy().replace('\'', "");
    SCRIPT_TEXT.replace("@LAUNCH_DIR@", &dir)
}

const SCRIPT_TEXT: &str = r##"#!/bin/bash
# Turbine Mod Manager's Steam compatibility tool, "Turbine (mods)". Steam runs it as
#   turbine-tool <verb> <the game's program> [arguments]
# behind its own launch wrapper and reaper. It builds the line Steam builds itself for a Proton game (the Steam Linux
# Runtime the chosen Proton asks for, then that Proton, then the game) and, for a game launch, hands it to Turbine's
# launch helper, which starts the game through me3 when mods are on in Turbine and the normal game otherwise. Without
# the helper it runs the plain line, so the game always starts. It runs nothing else. Every step goes to
# steam-launch.log.
tool=$(cd "$(dirname "$0")" && pwd)
launch='@LAUNCH_DIR@'
helper="$launch/turbine-launch.sh"
if [ -d "$launch" ]; then log="$launch/steam-launch.log"; else log="$tool/steam-launch.log"; fi
say() { printf '%s %s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "$*" >> "$log" 2>/dev/null; }
verb="${1:-}"
[ "$#" -gt 0 ] && shift
app="${STEAM_COMPAT_APP_ID:-${SteamAppId:-}}"
say "Turbine (mods): Steam started app ${app:-unknown} ($verb) in $(pwd): $*"
case "$verb" in
  run|waitforexitandrun) ;;
  *) say "Turbine (mods): not a launch Steam makes ($verb): nothing started"; exit 1 ;;
esac
if [ "$#" -eq 0 ]; then
  say "Turbine (mods): no program from Steam: nothing started"
  exit 1
fi
steam="${STEAM_COMPAT_CLIENT_INSTALL_PATH:-}"
if [ -z "$steam" ] || [ ! -d "$steam" ]; then steam=$(dirname "$(dirname "$tool")"); fi
# Steam's libraries: what Steam handed over, Steam's own folder, and its list of library folders.
libs=()
add_lib() {
  local l="${1%/}"
  l="${l%/steamapps}"
  [ -n "$l" ] && [ -d "$l/steamapps" ] || return 0
  local have
  for have in "${libs[@]}"; do [ "$have" = "$l" ] && return 0; done
  libs+=("$l")
}
IFS=: read -r -a handed <<< "${STEAM_COMPAT_LIBRARY_PATHS:-}"
for l in "${handed[@]}"; do add_lib "$l"; done
add_lib "$steam"
if [ -f "$steam/steamapps/libraryfolders.vdf" ]; then
  while IFS= read -r l; do add_lib "$l"; done < <(sed -n 's/^[[:space:]]*"path"[[:space:]]*"\(.*\)"[[:space:]]*$/\1/p' "$steam/steamapps/libraryfolders.vdf")
fi
value_of() { sed -n 's/^[[:space:]]*"'"$1"'"[[:space:]]*"\(.*\)"[[:space:]]*$/\1/p' "$2" 2> /dev/null | head -n 1; }
# The runtime a Proton needs, from Proton's own toolmanifest: its folder and its command. Answers 1 when the
# runtime it names isn't installed.
runtime=
rtcmd=()
runtime_for() {
  runtime=
  rtcmd=()
  local need
  need=$(value_of require_tool_appid "$1/toolmanifest.vdf")
  [ -n "$need" ] || return 0
  local l dir line
  for l in "${libs[@]}"; do
    dir=$(value_of installdir "$l/steamapps/appmanifest_$need.acf")
    if [ -n "$dir" ] && [ -f "$l/steamapps/common/$dir/toolmanifest.vdf" ]; then
      runtime="$l/steamapps/common/$dir"
      line=$(value_of commandline "$runtime/toolmanifest.vdf")
      [ -n "$line" ] || line='/_v2-entry-point --verb=%verb% --'
      line="${line//%verb%/$verb}"
      read -r -a rtcmd <<< "$line"
      rtcmd[0]="$runtime${rtcmd[0]}"
      [ -f "${rtcmd[0]}" ] && return 0
      runtime=
      rtcmd=()
    fi
  done
  missing="the Steam Linux Runtime it needs (Steam app $need) isn't installed"
  return 1
}
proton=
why=
missing=
passed=()
try() {
  [ -z "$proton" ] || return 0
  [ -n "$1" ] || return 0
  if [ ! -f "$1/proton" ] || [ ! -f "$1/toolmanifest.vdf" ]; then
    passed+=("$1 (no Proton there)")
    return 0
  fi
  if runtime_for "$1"; then
    proton="$1"
    why="$2"
  else
    passed+=("$1 ($missing)")
  fi
}
for f in "$launch/$app.proton" "$launch/default.proton"; do
  if [ -f "$f" ]; then
    d=$(head -n 1 "$f")
    d="${d%$'\r'}"
    case "$f" in
      */default.proton) try "$d" "Turbine's choice for every game" ;;
      *) try "$d" "the Proton Steam last used for this game, kept by Turbine" ;;
    esac
  fi
done
if [ -z "$proton" ]; then
  # The newest numbered Proton ("Proton 11.0", "Proton 9.0 (Beta)"), then Experimental, Hotfix and other tools.
  found=()
  tab=$'\t'
  for l in "${libs[@]}"; do
    for d in "$l"/steamapps/common/Proton\ [0-9]*; do
      [ -f "$d/proton" ] && found+=("$(basename "$d" | sed 's/^Proton \([0-9.]*\).*/\1/')$tab$d")
    done
  done
  if [ "${#found[@]}" -gt 0 ]; then
    while IFS= read -r line; do
      try "${line#*"$tab"}" "the newest Proton installed (Turbine has no record of the one Steam used for this game; ${#found[@]} numbered Proton found)"
    done < <(printf '%s\n' "${found[@]}" | sort -t "$tab" -k1,1 -V -r)
  fi
  for l in "${libs[@]}"; do try "$l/steamapps/common/Proton - Experimental" "Proton Experimental (no numbered Proton could be used)"; done
  for l in "${libs[@]}"; do try "$l/steamapps/common/Proton Hotfix" "Proton Hotfix (no numbered Proton could be used)"; done
  while IFS= read -r d; do
    [ "$d" = "$tool" ] || try "$d" "another tool's Proton in compatibilitytools.d (none of Steam's own could be used)"
  done < <(for d in "$steam"/compatibilitytools.d/*/; do d="${d%/}"; [ -f "$d/proton" ] && printf '%s\n' "$d"; done | sort -V -r)
fi
for p in "${passed[@]}"; do say "Turbine (mods): not used: $p"; done
if [ -z "$proton" ]; then
  say "Turbine (mods): no Proton that can run was found in ${libs[*]:-no Steam library}: nothing started. In Steam, start any game with Proton once (Steam then installs Proton and its runtime), or pick another tool in this game's Properties, Compatibility"
  exit 1
fi
say "Turbine (mods): Proton for this game: $proton [why: $why]"
if [ -n "$runtime" ]; then
  say "Turbine (mods): its runtime: $runtime"
  export STEAM_COMPAT_TOOL_PATHS="$proton:$runtime"
  export STEAM_COMPAT_MOUNTS="${STEAM_COMPAT_MOUNTS:+$STEAM_COMPAT_MOUNTS:}$proton:$runtime"
else
  say "Turbine (mods): this Proton names no runtime: it runs as it is"
  export STEAM_COMPAT_TOOL_PATHS="$proton"
fi
chain=("${rtcmd[@]}" "$proton/proton" "$verb" "$@")
if [ "$verb" != waitforexitandrun ]; then
  say "Turbine (mods): a set-up step of Steam's ($verb), never with mods: ${chain[*]}"
  exec "${chain[@]}"
fi
# Steam's reaper with this app's id above this script: it is how Steam knows the game is running.
reaper=no
p=$PPID
for _ in 1 2 3 4 5 6 7 8; do
  [ -r "/proc/$p/cmdline" ] || break
  c=$(tr '\0' ' ' < "/proc/$p/cmdline" 2> /dev/null)
  case "$c" in *SteamLaunch*"AppId=$app "*) reaper=yes; break ;; esac
  p=$(sed -n 's/^PPid:[[:space:]]*//p' "/proc/$p/status" 2> /dev/null)
  case "$p" in ''|0|1|*[!0-9]*) break ;; esac
done
if [ ! -x "$helper" ]; then
  say "Turbine (mods): Turbine's launch helper isn't there ($helper): the game starts the normal way, without mods: ${chain[*]}"
  exec "${chain[@]}"
fi
say "Turbine (mods): handing over to Turbine's launch helper (Steam's reaper above: $reaper)"
export TURBINE_TOOL_REAPER="$reaper"
exec "$helper" "${chain[@]}"
"##;

/// What `install` did.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Installed {
    /// The files written now (names), empty when everything was already in place.
    pub written: Vec<String>,
    /// The folder is new: Steam lists the tool after its next start.
    pub new_folder: bool,
}

fn write_if_different(path: &Path, text: &str, executable: bool) -> io::Result<bool> {
    if fs::read_to_string(path).is_ok_and(|t| t == text) {
        return Ok(false);
    }
    let tmp = path.with_extension("turbine-tmp");
    fs::write(&tmp, text)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(if executable { 0o755 } else { 0o644 }))?;
    }
    fs::rename(&tmp, path)?;
    Ok(true)
}

/// Writes the tool into Steam's compatibilitytools.d (only what is missing or different). Steam never writes in this
/// folder, so there is nothing to race with and Steam can be running. `data`: Turbine's data folder.
pub fn install(steam_root: &Path, data: &Path) -> io::Result<Installed> {
    if !steam_root.is_dir() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Steam's folder wasn't found"));
    }
    let dir = tool_dir(steam_root);
    let new_folder = !dir.join("compatibilitytool.vdf").is_file();
    fs::create_dir_all(&dir)?;
    let mut written = Vec::new();
    // The script and the manifest first: once compatibilitytool.vdf is there the tool is complete.
    let files = [
        (SCRIPT, script_text(&crate::anticheat::launch_dir(data)), true),
        ("toolmanifest.vdf", toolmanifest_vdf(), false),
        ("compatibilitytool.vdf", compatibilitytool_vdf(), false),
    ];
    for (name, text, exec) in files {
        if write_if_different(&dir.join(name), &text, exec)? {
            written.push(name.to_string());
        }
    }
    Ok(Installed { written, new_folder })
}

/// Is the tool in place, exactly as this Turbine writes it?
pub fn is_installed(steam_root: &Path, data: &Path) -> bool {
    let dir = tool_dir(steam_root);
    let same = |name: &str, text: String| fs::read_to_string(dir.join(name)).is_ok_and(|t| t == text);
    same(SCRIPT, script_text(&crate::anticheat::launch_dir(data))) && same("toolmanifest.vdf", toolmanifest_vdf()) && same("compatibilitytool.vdf", compatibilitytool_vdf())
}

/// Has Steam started since the tool's folder was made? Steam reads tools only when it starts, so until then the
/// tool isn't in its Compatibility list. `steam_started`: when the running Steam started (None: not running or
/// unknown, which counts as not seen yet).
pub fn seen_by_steam(steam_root: &Path, steam_started: Option<SystemTime>) -> bool {
    let Some(started) = steam_started else { return false };
    let Ok(made) = fs::metadata(tool_dir(steam_root).join("compatibilitytool.vdf")).and_then(|m| m.modified()) else { return false };
    made <= started
}

/// Steam's settings file with each game's compatibility tool (read only).
pub fn config_file(steam_root: &Path) -> PathBuf {
    steam_root.join("config").join("config.vdf")
}

const MAPPING: &[&str] = &["InstallConfigStore", "Software", "Valve", "Steam", "CompatToolMapping"];

/// The compatibility tool Steam has for a game in config.vdf's text (its internal name), when one is forced.
pub fn chosen_in(config_text: &str, app_id: &str) -> Option<String> {
    let v = vdf::parse(config_text).ok()?;
    let name = v.path(MAPPING)?.get(app_id)?.str_at(&["name"])?;
    (!name.is_empty()).then(|| name.to_string())
}

/// The games (app ids) that use Turbine's tool, from config.vdf's text.
pub fn games_using_in(config_text: &str) -> Vec<String> {
    let Ok(v) = vdf::parse(config_text) else { return Vec::new() };
    let Some(map) = v.path(MAPPING) else { return Vec::new() };
    map.entries().iter().filter(|(id, e)| id != "0" && e.str_at(&["name"]) == Some(TOOL_NAME)).map(|(id, _)| id.clone()).collect()
}

/// Does Steam's config.vdf say this game uses Turbine's tool? Steam writes that file some time after the player's
/// choice (at the latest when it closes), so `launched_through_tool` is the second way to know.
pub fn is_chosen(steam_root: &Path, app_id: &str) -> bool {
    fs::read_to_string(config_file(steam_root)).is_ok_and(|t| chosen_in(&t, app_id).as_deref() == Some(TOOL_NAME))
}

/// The games that use Turbine's tool, per Steam's config.vdf.
pub fn games_using(steam_root: &Path) -> Vec<String> {
    fs::read_to_string(config_file(steam_root)).map(|t| games_using_in(&t)).unwrap_or_default()
}

/// Takes the tool's folder out. Refused while a game still names the tool (Steam would start that game with no
/// tool at all): the answer is then the app ids to switch back first. Only Turbine's own three files and its log go.
pub fn remove(steam_root: &Path) -> Result<bool, Vec<String>> {
    let using = games_using(steam_root);
    if !using.is_empty() {
        return Err(using);
    }
    let dir = tool_dir(steam_root);
    if !dir.is_dir() {
        return Ok(false);
    }
    for name in [SCRIPT, "toolmanifest.vdf", "compatibilitytool.vdf", "steam-launch.log"] {
        let _ = fs::remove_file(dir.join(name));
    }
    let _ = fs::remove_dir(&dir);
    Ok(true)
}

// ---- Which Proton ----

/// The file that names the Proton folder for one game (read by the script).
pub fn proton_file(data: &Path, app_id: &str) -> PathBuf {
    crate::anticheat::launch_dir(data).join(format!("{app_id}.proton"))
}

fn app_id_ok(app_id: &str) -> bool {
    !app_id.is_empty() && app_id.bytes().all(|b| b.is_ascii_digit())
}

/// Is this folder a Proton the script can use (its program and its manifest are there)?
pub fn is_proton(dir: &Path) -> bool {
    dir.join("proton").is_file() && dir.join("toolmanifest.vdf").is_file()
}

/// Writes down the Proton folder for a game.
pub fn write_proton_choice(data: &Path, app_id: &str, proton_dir: &Path) -> io::Result<()> {
    if !app_id_ok(app_id) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a Steam app id"));
    }
    let text = proton_dir.to_string_lossy();
    if text.contains('\n') || !proton_dir.is_absolute() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a folder path"));
    }
    fs::create_dir_all(crate::anticheat::launch_dir(data))?;
    fs::write(proton_file(data, app_id), format!("{text}\n"))
}

/// The Proton folder written down for a game, if any.
pub fn proton_choice(data: &Path, app_id: &str) -> Option<PathBuf> {
    let t = fs::read_to_string(proton_file(data, app_id)).ok()?;
    let line = t.lines().next()?.trim_end_matches('\r');
    (!line.is_empty()).then(|| PathBuf::from(line))
}

/// The Proton Steam itself last used for a game, from the helper's log (steam-launch.log): the newest launch of the
/// app whose command from Steam held `<folder>/proton <verb>` and did not come through Turbine's tool.
pub fn last_steam_proton_in(log: &str, app_id: &str) -> Option<PathBuf> {
    let mark = format!(" Steam started app {app_id} in ");
    for line in log.lines().rev() {
        // Steam's own command always holds its reaper; the line the tool hands to the helper doesn't.
        if !line.contains(&mark) || !line.contains(&format!("SteamLaunch AppId={app_id} ")) || line.contains(&format!("/{SCRIPT} ")) {
            continue;
        }
        for verb in [" waitforexitandrun ", " run "] {
            let needle = format!("/proton{verb}");
            if let Some(end) = line.find(&needle) {
                let head = &line[..end];
                // The folder starts at the last " /" after the "-- " in front of Proton (paths hold spaces).
                let start = head.rfind(" -- /").map(|i| i + 4).or_else(|| head.rfind(": /").map(|i| i + 2))?;
                return Some(PathBuf::from(&head[start..]));
            }
        }
    }
    None
}

/// Keeps the Proton Steam last used for the game, when the log has one and it is still there. Called before the
/// player switches the game to Turbine's tool, and at every Play while the Launch Options way is in use. Answers
/// the folder now on record.
pub fn remember_proton(data: &Path, app_id: &str) -> Option<PathBuf> {
    let log = fs::read_to_string(crate::anticheat::helper_log(data)).unwrap_or_default();
    if let Some(dir) = last_steam_proton_in(&log, app_id) {
        if is_proton(&dir) && proton_choice(data, app_id).as_deref() != Some(dir.as_path()) {
            let _ = write_proton_choice(data, app_id, &dir);
        }
    }
    proton_choice(data, app_id)
}

// ---- What the tool said (for the launch check and reports) ----

/// The tool's own lines among the helper log's lines (each starts with its name after the time).
pub fn tool_lines(lines: &[String]) -> Vec<String> {
    lines.iter().filter(|l| l.contains(" Turbine (mods): ")).cloned().collect()
}

/// The Proton the tool used in these log lines, with why: ("/path/Proton 11.0", "the newest Proton installed ...").
pub fn proton_used(lines: &[String]) -> Option<(String, String)> {
    let mark = " Turbine (mods): Proton for this game: ";
    let l = lines.iter().rev().find(|l| l.contains(mark))?;
    let rest = &l[l.find(mark)? + mark.len()..];
    let (dir, why) = rest.split_once(" [why: ")?;
    Some((dir.to_string(), why.strip_suffix(']').unwrap_or(why).to_string()))
}

/// One plain sentence for the launch check: which Proton ran the game and why. When Turbine had no record and the
/// tool chose by itself, the sentence says so.
pub fn proton_words(lines: &[String]) -> Option<String> {
    let (dir, why) = proton_used(lines)?;
    let name = Path::new(&dir).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or(dir);
    Some(format!("Started with {name}: {why}."))
}

/// Why the tool started nothing, from these log lines (no Proton found, a bad launch), if it said so.
pub fn nothing_started(lines: &[String]) -> Option<String> {
    let l = lines.iter().rev().find(|l| l.contains(" Turbine (mods): ") && l.contains("nothing started"))?;
    let at = l.find(" Turbine (mods): ")? + " Turbine (mods): ".len();
    Some(l[at..].to_string())
}

/// Did the newest launch of this app in the helper's log come through Turbine's tool?
pub fn launched_through_tool_in(log: &str, app_id: &str) -> bool {
    let helper = format!(" Steam started app {app_id} in ");
    let tool = format!(" Turbine (mods): Steam started app {app_id} (");
    // The tool's lines for a launch stand right above the helper's first line for it.
    let mut above_helper = false;
    for line in log.lines().rev() {
        if line.contains(&tool) {
            return true;
        }
        if above_helper {
            if !line.contains(" Turbine (mods): ") {
                return false;
            }
        } else if line.contains(&helper) {
            if line.contains(&format!("/{SCRIPT} ")) {
                return true;
            }
            above_helper = true;
        }
    }
    false
}

// ---- How a game is set up to start with mods ----

/// The way Steam starts a me3 game with Turbine in between.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    /// The game uses "Turbine (mods)" in Steam (config.vdf says so, or its last launch came through the tool).
    Tool,
    /// The game's Launch Options run Turbine's helper (the pasted line).
    LaunchOptions,
    /// Neither yet: the player picks the tool once (or pastes the line).
    NotSet,
}

impl Route {
    pub fn word(self) -> &'static str {
        match self {
            Route::Tool => "tool",
            Route::LaunchOptions => "options",
            Route::NotSet => "none",
        }
    }
}

/// Which way is set for the game. `launch_options_ok`: anticheat::launch_option_ok for the game's Launch Options.
/// A newer choice in Steam that isn't in config.vdf yet still counts when the last launch came through the tool;
/// when config.vdf names another tool for the game, the tool is not counted on the log's word alone.
pub fn route(steam_root: &Path, data: &Path, app_id: &str, launch_options_ok: bool) -> Route {
    let config = fs::read_to_string(config_file(steam_root)).unwrap_or_default();
    let chosen = chosen_in(&config, app_id);
    let through = || fs::read_to_string(crate::anticheat::helper_log(data)).is_ok_and(|l| launched_through_tool_in(&l, app_id));
    let tool_there = tool_dir(steam_root).join(SCRIPT).is_file();
    if tool_there && (chosen.as_deref() == Some(TOOL_NAME) || (chosen.is_none() && through())) {
        Route::Tool
    } else if launch_options_ok {
        Route::LaunchOptions
    } else {
        Route::NotSet
    }
}

/// The one-time steps for the player, in order, as plain sentences. `listed`: Steam has started since the tool was
/// written, so it is in the list; otherwise the first step says to restart Steam.
pub fn steps(game_name: &str, listed: bool) -> Vec<String> {
    let mut s = Vec::new();
    if !listed {
        s.push("Restart Steam once, so it finds Turbine's tool: Steam button, Power, Restart Steam.".to_string());
    }
    s.push(format!("In Steam, open {game_name}, press the gear and choose Properties, then Compatibility."));
    s.push("Turn on \"Force the use of a specific Steam Play compatibility tool\".".to_string());
    s.push(format!("Pick \"{DISPLAY_NAME}\" in the list, then come back to Turbine and press Play."));
    s
}

/// What to say when Turbine's tool should go but games still use it.
pub fn still_used_words(names: &[String]) -> String {
    format!(
        "Turbine's Steam tool stays: {} still {} it. In Steam, open the game's Properties, Compatibility and pick another tool (or turn the setting off) first, or the game would not start.",
        names.join(", "),
        if names.len() == 1 { "uses" } else { "use" }
    )
}

// ---- The Co-op tile (0.62 decision) ----

/// Where the optional Seamless Co-op tile in Steam's library can come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileWay {
    /// Turbine's Decky plugin adds it with Steam's own call: no file is written, no restart.
    Decky,
    /// Desktop Mode with Steam closed: Turbine writes Steam's list of non-Steam games (with a backup).
    DesktopSteamClosed,
    /// Desktop Mode with Steam running: offered, after the player closes Steam.
    DesktopCloseSteamFirst,
    /// Game Mode without the plugin: not offered. Play Seamless Co-op stays in Turbine and in the Steam menu.
    NotOffered,
}

/// The decision: Steam's list of non-Steam games can't be written safely in Game Mode (Steam starts again at once
/// and writes its own copy back), so there the tile only comes through the Decky plugin.
pub fn tile_way(game_mode: bool, decky_plugin: bool, steam_running: bool) -> TileWay {
    match (decky_plugin, game_mode, steam_running) {
        (true, _, true) => TileWay::Decky,
        (_, true, _) => TileWay::NotOffered,
        (_, false, false) => TileWay::DesktopSteamClosed,
        (_, false, true) => TileWay::DesktopCloseSteamFirst,
    }
}

impl TileWay {
    pub fn word(self) -> &'static str {
        match self {
            TileWay::Decky => "decky",
            TileWay::DesktopSteamClosed => "desktop",
            TileWay::DesktopCloseSteamFirst => "closesteam",
            TileWay::NotOffered => "no",
        }
    }

    /// The row's words on the Co-op settings page.
    pub fn words(self) -> &'static str {
        match self {
            TileWay::Decky => "Add a Co-op tile to your Steam library. It starts Seamless Co-op in one press.",
            TileWay::DesktopSteamClosed => "Add a Co-op tile to your Steam library. Steam is closed, so Turbine can add it now.",
            TileWay::DesktopCloseSteamFirst => "A Co-op tile can be added while Steam is closed. Close Steam, then come back here.",
            TileWay::NotOffered => "A Co-op tile can be added in Desktop Mode with Steam closed, or here with Turbine's Decky plugin. Play Seamless Co-op is always in Turbine's Play choice and in the Steam menu.",
        }
    }
}

// ---- For the app ----

/// Why Turbine doesn't restart Steam to change Steam's own settings files in Game Mode (0.62: that way is gone).
pub const GAME_MODE_NO_WRITE: &str = "Game Mode starts Steam again at once, so Turbine can't change Steam's own settings here. Pick \"Turbine (mods)\" for the game in Steam instead (Turbine shows the three steps), or do this in Desktop Mode.";

/// May Turbine close Steam, write one of Steam's settings files and start it again? Never in Game Mode while Steam
/// runs (the session starts Steam again before the file is written, and Steam writes its own copy back).
pub fn restart_and_write_allowed(game_mode: bool, steam_running: Option<bool>) -> Result<(), &'static str> {
    if game_mode && steam_running != Some(false) {
        Err(GAME_MODE_NO_WRITE)
    } else {
        Ok(())
    }
}

/// What the app needs to know about the tool for one game.
pub struct Ask<'a> {
    pub steam_root: &'a Path,
    pub data: &'a Path,
    pub app_id: &'a str,
    pub game_name: &'a str,
    /// The game's Launch Options run Turbine's helper (anticheat::launch_option_ok).
    pub launch_options_ok: bool,
    /// When the running Steam started (None: not running, or unknown).
    pub steam_started: Option<SystemTime>,
    pub game_mode: bool,
    pub decky_plugin: bool,
    pub steam_running: bool,
}

/// The tool's state for one game as JSON for the app:
/// {"installed","listed","chosen","route","steps":[..],"restartFirst","proton","protonName","lastProton",
///  "nothingStarted","using":[app ids],"toolName","tile","tileWords"}.
/// `route` is "tool", "options" or "none"; `listed` false means Steam must start again before the tool shows in its
/// list; `proton` is the folder on record for the game ("" when the tool will choose by itself), `lastProton` the
/// sentence about the Proton the tool used at the last launch.
pub fn state_json(a: &Ask) -> String {
    use crate::json::quote;
    let installed = is_installed(a.steam_root, a.data);
    let listed = installed && seen_by_steam(a.steam_root, a.steam_started);
    let r = route(a.steam_root, a.data, a.app_id, a.launch_options_ok);
    let chosen = is_chosen(a.steam_root, a.app_id);
    let proton = proton_choice(a.data, a.app_id).filter(|p| is_proton(p));
    let name = proton.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let log: Vec<String> = fs::read_to_string(crate::anticheat::helper_log(a.data)).unwrap_or_default().lines().rev().take(60).map(str::to_string).collect::<Vec<_>>().into_iter().rev().collect();
    let steps: Vec<String> = steps(a.game_name, listed).iter().map(|s| quote(s)).collect();
    let using: Vec<String> = games_using(a.steam_root).iter().map(|s| quote(s)).collect();
    let tile = tile_way(a.game_mode, a.decky_plugin, a.steam_running);
    format!(
        "{{\"installed\":{installed},\"listed\":{listed},\"chosen\":{chosen},\"route\":{},\"steps\":[{}],\"restartFirst\":{},\"proton\":{},\"protonName\":{},\"lastProton\":{},\"nothingStarted\":{},\"using\":[{}],\"toolName\":{},\"tile\":{},\"tileWords\":{}}}",
        quote(r.word()),
        steps.join(","),
        !listed,
        quote(&proton.map(|p| p.to_string_lossy().to_string()).unwrap_or_default()),
        quote(&name),
        quote(&proton_words(&log).unwrap_or_default()),
        quote(&nothing_started(&log).unwrap_or_default()),
        using.join(","),
        quote(DISPLAY_NAME),
        quote(tile.word()),
        quote(tile.words()),
    )
}
