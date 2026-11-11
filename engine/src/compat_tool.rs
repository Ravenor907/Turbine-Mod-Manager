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
//! last used for it or the one it last ran with, or the player's pick), else `default.proton` (Steam's choice for
//! all games, or the newest finished Proton), else the newest finished numbered Proton installed, then a Beta,
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
    # The second line says where the record came from.
    how=$(sed -n '2p' "$f")
    how="${how%$'\r'}"
    case "$f" in
      */default.proton)
        case "$how" in
          steam) try "$d" "Steam's own choice for all games, kept by Turbine" ;;
          *) try "$d" "Turbine's choice for every game" ;;
        esac ;;
      *)
        case "$how" in
          you) try "$d" "your pick in Turbine for this game" ;;
          tool) try "$d" "the Proton this game last ran with, kept by Turbine" ;;
          *) try "$d" "the Proton Steam last used for this game, kept by Turbine" ;;
        esac ;;
    esac
  fi
done
if [ -z "$proton" ]; then
  # The newest finished Proton ("Proton 11.0"), then numbered ones that are not finished ("Proton 12.0 (Beta)"),
  # then Experimental, Hotfix and other tools.
  found=()
  beta=()
  tab=$'\t'
  for l in "${libs[@]}"; do
    for d in "$l"/steamapps/common/Proton\ [0-9]*; do
      [ -f "$d/proton" ] || continue
      name=$(basename "$d")
      num=$(printf '%s' "$name" | sed 's/^Proton \([0-9.]*\).*/\1/')
      if [ "$name" = "Proton $num" ]; then found+=("$num$tab$d"); else beta+=("$num$tab$d"); fi
    done
  done
  count=$(( ${#found[@]} + ${#beta[@]} ))
  if [ "${#found[@]}" -gt 0 ]; then
    while IFS= read -r line; do
      try "${line#*"$tab"}" "the newest Proton installed (Turbine has no record of the one Steam used for this game; $count numbered Proton found)"
    done < <(printf '%s\n' "${found[@]}" | sort -t "$tab" -k1,1 -V -r)
  fi
  if [ "${#beta[@]}" -gt 0 ]; then
    while IFS= read -r line; do
      try "${line#*"$tab"}" "the newest Proton installed is a Beta (no finished Proton could be used; $count numbered Proton found)"
    done < <(printf '%s\n' "${beta[@]}" | sort -t "$tab" -k1,1 -V -r)
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
# Read by bash, not started as a program of its own: a helper Turbine is rewriting at this very moment can't be
# refused as "busy" that way.
exec /bin/bash "$helper" "${chain[@]}"
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

/// Where a game's Proton record came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtonHow {
    /// Steam itself used it for the game (seen in the helper's log, or the game's entry in Steam's settings).
    Steam,
    /// The tool chose it by itself at a launch (no record then), and it is kept so the game keeps what works.
    Tool,
    /// The player picked it in Turbine.
    You,
}

impl ProtonHow {
    pub fn word(self) -> &'static str {
        match self {
            ProtonHow::Steam => "steam",
            ProtonHow::Tool => "tool",
            ProtonHow::You => "you",
        }
    }

    fn from_word(w: &str) -> ProtonHow {
        match w.trim() {
            "you" => ProtonHow::You,
            "tool" => ProtonHow::Tool,
            _ => ProtonHow::Steam,
        }
    }

    /// In plain words, for the Settings row.
    pub fn words(self) -> &'static str {
        match self {
            ProtonHow::Steam => "the one Steam last used for this game",
            ProtonHow::Tool => "the one this game last ran with",
            ProtonHow::You => "your pick",
        }
    }
}

fn write_record(file: &Path, proton_dir: &Path, how: &str) -> io::Result<()> {
    let text = proton_dir.to_string_lossy();
    if text.contains('\n') || !proton_dir.is_absolute() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a folder path"));
    }
    if let Some(d) = file.parent() {
        fs::create_dir_all(d)?;
    }
    fs::write(file, format!("{text}\n{how}\n"))
}

/// Writes down the Proton folder for a game, with where the record came from (the file's second line).
pub fn write_proton_record(data: &Path, app_id: &str, proton_dir: &Path, how: ProtonHow) -> io::Result<()> {
    if !app_id_ok(app_id) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a Steam app id"));
    }
    write_record(&proton_file(data, app_id), proton_dir, how.word())
}

/// The player's pick for a game: a Proton folder, or None for "Automatic" (the record goes and stays gone: the tool
/// then takes default.proton at each launch, or the Proton of a later launch by Steam itself).
pub fn set_proton_pick(data: &Path, app_id: &str, proton_dir: Option<&Path>) -> io::Result<()> {
    match proton_dir {
        Some(d) if is_proton(d) => write_proton_record(data, app_id, d, ProtonHow::You),
        Some(_) => Err(io::Error::new(io::ErrorKind::NotFound, "that Proton isn't installed any more")),
        None if !app_id_ok(app_id) => Err(io::Error::new(io::ErrorKind::InvalidInput, "not a Steam app id")),
        None => {
            // Automatic: the record goes, and launches up to now stop deciding (the newest line's time is kept).
            let log = fs::read_to_string(crate::anticheat::helper_log(data)).unwrap_or_default();
            let newest = log.lines().rev().find_map(line_time).unwrap_or("");
            fs::create_dir_all(crate::anticheat::launch_dir(data))?;
            fs::write(proton_auto_file(data, app_id), format!("{newest}\n"))?;
            match fs::remove_file(proton_file(data, app_id)) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
                _ => Ok(()),
            }
        }
    }
}

/// The Proton folder written down for a game, if any.
pub fn proton_choice(data: &Path, app_id: &str) -> Option<PathBuf> {
    proton_record(data, app_id).map(|(d, _)| d)
}

/// The record for a game: the folder and where it came from.
pub fn proton_record(data: &Path, app_id: &str) -> Option<(PathBuf, ProtonHow)> {
    let t = fs::read_to_string(proton_file(data, app_id)).ok()?;
    let mut lines = t.lines();
    let line = lines.next()?.trim_end_matches('\r');
    let how = ProtonHow::from_word(lines.next().unwrap_or(""));
    (!line.is_empty()).then(|| (PathBuf::from(line), how))
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

/// The file that says the player chose Automatic for a game, with the time of the newest line in the helper's log
/// at that moment: launches before it no longer decide the game's Proton.
pub fn proton_auto_file(data: &Path, app_id: &str) -> PathBuf {
    crate::anticheat::launch_dir(data).join(format!("{app_id}.proton.auto"))
}

/// The time at the start of a log line ("2026-11-10 10:00:00"), when it has one.
fn line_time(line: &str) -> Option<&str> {
    let t = line.get(..19)?;
    (t.as_bytes()[4] == b'-' && t.as_bytes()[10] == b' ' && t.as_bytes()[13] == b':').then_some(t)
}

/// Keeps the Proton Steam itself last used for the game (0.62 break test M5: the record never moved once it was
/// written). Called at every Play and when the Proton row is read. Rules, after the re-check's N3:
/// - The player's own pick is never replaced.
/// - Only Steam's own launches count (the game started with a Proton of Steam's choosing, through the Launch
///   Options line). What the tool chose by itself is never written down: a record of it froze the game on the
///   tool's first pick and undid "Automatic" the moment the row was read. With no record the tool takes
///   default.proton at each launch (Steam's choice for all games, or the newest finished Proton), so the game
///   moves to a newer Proton when Steam does.
/// - After the player chose Automatic, launches from before that moment don't count any more.
///
/// Only a Proton that is still installed is written down. Answers the folder now on record.
pub fn remember_proton(data: &Path, app_id: &str) -> Option<PathBuf> {
    let now = proton_record(data, app_id);
    match &now {
        Some((d, ProtonHow::You)) if is_proton(d) => return Some(d.clone()),
        // A record of the tool's own pick (written by a build that never shipped): gone.
        Some((_, ProtonHow::Tool)) => {
            let _ = fs::remove_file(proton_file(data, app_id));
        }
        _ => {}
    }
    let now = now.filter(|(_, h)| *h != ProtonHow::Tool);
    let log = fs::read_to_string(crate::anticheat::helper_log(data)).unwrap_or_default();
    let since = fs::read_to_string(proton_auto_file(data, app_id)).ok().map(|t| t.trim().to_string());
    let counted: String = match &since {
        Some(mark) => log.lines().filter(|l| line_time(l).is_some_and(|t| t > mark.as_str())).map(|l| format!("{l}\n")).collect(),
        None => log,
    };
    if let Some(dir) = last_steam_proton_in(&counted, app_id) {
        if is_proton(&dir) && now.as_ref().map(|(d, _)| d.as_path()) != Some(dir.as_path()) {
            let _ = write_proton_record(data, app_id, &dir, ProtonHow::Steam);
        }
    }
    proton_choice(data, app_id)
}

/// `remember_proton`, and with no record after it: the game's own entry in Steam's settings (the tool Steam was
/// told to use for it before the player picks Turbine's), when that is a Proton installed here. Also keeps
/// `default.proton` up to date (`write_default_proton`).
pub fn remember_proton_with(steam_root: &Path, data: &Path, app_id: &str) -> Option<PathBuf> {
    let _ = write_default_proton(steam_root, data);
    if let Some(d) = remember_proton(data, app_id) {
        if is_proton(&d) {
            return Some(d);
        }
    }
    let config = fs::read_to_string(config_file(steam_root)).unwrap_or_default();
    let name = chosen_in(&config, app_id).filter(|n| n != TOOL_NAME)?;
    let dir = installed_protons(steam_root).into_iter().find(|p| p.steam_name.as_deref() == Some(name.as_str()) || p.matches_steam_name(&name))?.dir;
    write_proton_record(data, app_id, &dir, ProtonHow::Steam).ok()?;
    Some(dir)
}

// ---- The Protons installed (0.62) ----

/// What kind of Proton a folder is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProtonKind {
    /// A finished, numbered Proton from Steam ("Proton 11.0").
    Stable,
    /// A numbered one that isn't finished ("Proton 12.0 (Beta)").
    Beta,
    Experimental,
    Hotfix,
    /// Another tool's Proton in compatibilitytools.d (GE-Proton and the like).
    Other,
}

impl ProtonKind {
    pub fn word(self) -> &'static str {
        match self {
            ProtonKind::Stable => "stable",
            ProtonKind::Beta => "beta",
            ProtonKind::Experimental => "experimental",
            ProtonKind::Hotfix => "hotfix",
            ProtonKind::Other => "other",
        }
    }
}

/// One Proton installed on this device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proton {
    pub dir: PathBuf,
    /// Its folder's name ("Proton 11.0", "GE-Proton10-4").
    pub name: String,
    pub kind: ProtonKind,
    /// Its number ("11.0"), for numbered ones.
    pub number: String,
    /// The Steam Linux Runtime it asks for is installed (or it asks for none): the tool can start it.
    pub usable: bool,
    /// Its name in Steam's settings when its folder says so (another tool's compatibilitytool.vdf).
    pub steam_name: Option<String>,
}

impl Proton {
    /// Is this the Proton Steam's settings call `name` ("proton_11", "proton_experimental", "GE-Proton10-4")?
    pub fn matches_steam_name(&self, name: &str) -> bool {
        match self.kind {
            ProtonKind::Experimental => name == "proton_experimental",
            ProtonKind::Hotfix => name == "proton_hotfix",
            ProtonKind::Stable | ProtonKind::Beta => name.strip_prefix("proton_").is_some_and(|n| !n.is_empty() && n == self.number.split('.').next().unwrap_or("")),
            ProtonKind::Other => self.name == name,
        }
    }
}

/// Steam's library folders (its own folder first), from steamapps/libraryfolders.vdf.
pub fn library_folders(steam_root: &Path) -> Vec<PathBuf> {
    let mut out = vec![steam_root.to_path_buf()];
    if let Some(v) = fs::read_to_string(steam_root.join("steamapps/libraryfolders.vdf")).ok().and_then(|t| vdf::parse(&t).ok()) {
        for (_, lib) in v.get("libraryfolders").map(|l| l.entries()).unwrap_or(&[]) {
            if let Some(p) = lib.str_at(&["path"]).map(PathBuf::from) {
                if p.is_absolute() && p.join("steamapps").is_dir() && !out.contains(&p) {
                    out.push(p);
                }
            }
        }
    }
    out
}

/// Every Proton installed: Steam's own in each library, then other tools' in compatibilitytools.d. Best first:
/// finished ones by number (newest first), then Betas, Experimental, Hotfix, others; usable ones before the rest
/// within each kind.
pub fn installed_protons(steam_root: &Path) -> Vec<Proton> {
    let libs = library_folders(steam_root);
    let value = |file: &Path, key: &str| -> Option<String> {
        let v = vdf::parse(&fs::read_to_string(file).ok()?).ok()?;
        v.entries().first().and_then(|(_, body)| body.str_at(&[key])).map(str::to_string)
    };
    let usable = |dir: &Path| -> bool {
        let Some(need) = value(&dir.join("toolmanifest.vdf"), "require_tool_appid") else { return true };
        libs.iter().any(|l| value(&l.join(format!("steamapps/appmanifest_{need}.acf")), "installdir").is_some_and(|d| l.join("steamapps/common").join(d).join("toolmanifest.vdf").is_file()))
    };
    let mut out: Vec<Proton> = Vec::new();
    for l in &libs {
        let Ok(rd) = fs::read_dir(l.join("steamapps/common")) else { continue };
        for e in rd.flatten() {
            let dir = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if !name.starts_with("Proton") || !is_proton(&dir) || out.iter().any(|p| p.name == name) {
                continue;
            }
            let number: String = name.strip_prefix("Proton ").unwrap_or("").chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
            let kind = if name == "Proton - Experimental" {
                ProtonKind::Experimental
            } else if name == "Proton Hotfix" {
                ProtonKind::Hotfix
            } else if !number.is_empty() && name == format!("Proton {number}") {
                ProtonKind::Stable
            } else if !number.is_empty() {
                ProtonKind::Beta
            } else {
                ProtonKind::Other
            };
            out.push(Proton { usable: usable(&dir), dir, name, kind, number, steam_name: None });
        }
    }
    if let Ok(rd) = fs::read_dir(tools_dir(steam_root)) {
        for e in rd.flatten() {
            let dir = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if name == FOLDER || !is_proton(&dir) {
                continue;
            }
            let steam_name = fs::read_to_string(dir.join("compatibilitytool.vdf"))
                .ok()
                .and_then(|t| vdf::parse(&t).ok())
                .and_then(|v| v.path(&["compatibilitytools", "compat_tools"]).and_then(|c| c.entries().first().map(|(k, _)| k.clone())));
            out.push(Proton { usable: usable(&dir), dir, name, kind: ProtonKind::Other, number: String::new(), steam_name });
        }
    }
    out.sort_by(|a, b| {
        a.kind.cmp(&b.kind).then(b.usable.cmp(&a.usable)).then_with(|| crate::updates::compare_versions(&b.number, &a.number)).then_with(|| crate::updates::compare_versions(&b.name, &a.name))
    });
    out
}

/// The file that names the Proton for games without a record of their own (read by the script).
pub fn default_proton_file(data: &Path) -> PathBuf {
    crate::anticheat::launch_dir(data).join("default.proton")
}

/// Keeps default.proton: Steam's own choice for all games (the "0" entry of its settings) when that Proton is
/// installed and usable, else the newest finished Proton. Written only when it changes. Answers the folder.
pub fn write_default_proton(steam_root: &Path, data: &Path) -> Option<PathBuf> {
    let list = installed_protons(steam_root);
    let config = fs::read_to_string(config_file(steam_root)).unwrap_or_default();
    let steams = chosen_in(&config, "0")
        .filter(|n| n != TOOL_NAME)
        .and_then(|name| list.iter().find(|p| p.usable && (p.steam_name.as_deref() == Some(name.as_str()) || p.matches_steam_name(&name))));
    let (pick, how) = match steams {
        Some(p) => (p, "steam"),
        None => (list.iter().find(|p| p.usable)?, "turbine"),
    };
    let file = default_proton_file(data);
    let want = format!("{}\n{how}\n", pick.dir.to_string_lossy());
    if fs::read_to_string(&file).is_ok_and(|t| t == want) || write_record(&file, &pick.dir, how).is_ok() {
        Some(pick.dir.clone())
    } else {
        None
    }
}

/// For the Settings row "Proton for modded play": {"name","dir","how","words","auto","list":[{"name","dir",
/// "kind","usable","current"}]}. `how` is "you", "steam", "tool" or "auto" (no record: the tool takes `auto`, the
/// default, at the next launch). The list is every Proton installed, best first.
pub fn proton_json(steam_root: &Path, data: &Path, app_id: &str) -> String {
    use crate::json::quote;
    let list = installed_protons(steam_root);
    let record = proton_record(data, app_id).filter(|(d, _)| is_proton(d));
    let auto = write_default_proton(steam_root, data).or_else(|| list.iter().find(|p| p.usable).map(|p| p.dir.clone()));
    let name_of = |d: &Path| d.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let current = record.as_ref().map(|(d, _)| d.clone()).or_else(|| auto.clone());
    let (how, words) = match &record {
        Some((_, h)) => (h.word(), h.words().to_string()),
        None => ("auto", "chosen by Turbine: Steam's own choice for all games, or the newest finished Proton".to_string()),
    };
    let items: Vec<String> = list
        .iter()
        .map(|p| format!("{{\"name\":{},\"dir\":{},\"kind\":{},\"usable\":{},\"current\":{}}}", quote(&p.name), quote(&p.dir.to_string_lossy()), quote(p.kind.word()), p.usable, current.as_deref() == Some(p.dir.as_path())))
        .collect();
    format!(
        "{{\"name\":{},\"dir\":{},\"how\":{},\"words\":{},\"auto\":{},\"list\":[{}]}}",
        quote(&current.as_deref().map(name_of).unwrap_or_default()),
        quote(&current.as_deref().map(|d| d.to_string_lossy().into_owned()).unwrap_or_default()),
        quote(how),
        quote(&words),
        quote(&auto.as_deref().map(name_of).unwrap_or_default()),
        items.join(",")
    )
}

// ---- What the tool said (for the launch check and reports) ----

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

// ---- The Co-op tile (0.62 decision) ----
//
// Steam keeps its non-Steam games in a file it writes back from memory when it closes, and Game Mode starts Steam
// again at once. So the tile is added only where the file can be written safely: Desktop Mode. With Steam closed
// Turbine writes it at once; with Steam running it is one Steam restart (which works on the desktop: Steam stays
// closed until the script starts it again). In Game Mode it isn't offered as a button; the row says where it can be
// added. Play Seamless Co-op is always in Turbine's Play choice, and in the Steam menu with Turbine's plugin.

/// Where the optional Seamless Co-op tile in Steam's library can come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileWay {
    /// Desktop Mode with Steam closed: Turbine writes Steam's list of non-Steam games now (with a backup).
    DesktopSteamClosed,
    /// Desktop Mode with Steam running: one Steam restart, asked first.
    DesktopRestart,
    /// Game Mode with Turbine's Steam menu plugin: not from here; co-op is one press in the Steam menu already.
    GameModePlugin,
    /// Game Mode without the plugin: not from here.
    GameMode,
}

/// The decision for this moment.
pub fn tile_way(game_mode: bool, decky_plugin: bool, steam_running: bool) -> TileWay {
    match (game_mode, steam_running, decky_plugin) {
        (false, false, _) => TileWay::DesktopSteamClosed,
        (false, true, _) => TileWay::DesktopRestart,
        // Game Mode with Steam closed doesn't happen for longer than a moment (the session starts it again).
        (true, _, true) => TileWay::GameModePlugin,
        (true, _, false) => TileWay::GameMode,
    }
}

impl TileWay {
    /// For the app: "direct" (Turbine does it now), "restart" (one Steam restart, asked first), "later" (not from
    /// here: the words say where).
    pub fn how(self) -> &'static str {
        match self {
            TileWay::DesktopSteamClosed => "direct",
            TileWay::DesktopRestart => "restart",
            TileWay::GameModePlugin | TileWay::GameMode => "later",
        }
    }

    /// The words for the row and the question. `add`: adding the tile (false: taking it out).
    pub fn words(self, add: bool) -> &'static str {
        match (self, add) {
            (TileWay::DesktopSteamClosed, true) => "Steam is closed, so Turbine can add it now. It shows when Steam starts.",
            (TileWay::DesktopSteamClosed, false) => "Steam is closed, so Turbine can take it out now.",
            (TileWay::DesktopRestart, _) => "Steam has to restart once for this.",
            (TileWay::GameModePlugin, true) => "The tile can be added in Desktop Mode (open Turbine there). Here in Game Mode, Play Seamless Co-op is already one press in the Steam menu (Turbine's plugin) and in Turbine's Play choice.",
            (TileWay::GameMode, true) => "The tile can be added in Desktop Mode (open Turbine there). Here in Game Mode, Play Seamless Co-op is in Turbine's Play choice.",
            (TileWay::GameModePlugin | TileWay::GameMode, false) => "The tile can be taken out in Desktop Mode (open Turbine there), or in Steam: the tile's options, Manage, Remove non-Steam game.",
        }
    }

    /// The button's label ("" when there is nothing to press here).
    pub fn label(self, add: bool) -> &'static str {
        match (self, add) {
            (TileWay::DesktopSteamClosed, true) => "Add it",
            (TileWay::DesktopSteamClosed, false) => "Remove it",
            (TileWay::DesktopRestart, true) => "Restart Steam and add it",
            (TileWay::DesktopRestart, false) => "Restart Steam and remove it",
            _ => "",
        }
    }

    /// {"how","words","label"} for the app.
    pub fn offer_json(self, add: bool) -> String {
        use crate::json::quote;
        format!("{{\"how\":{},\"words\":{},\"label\":{}}}", quote(self.how()), quote(self.words(add)), quote(self.label(add)))
    }
}

/// The answer for a game that has no Co-op tile at all.
pub const NO_TILE_JSON: &str = "{\"how\":\"none\",\"words\":\"\",\"label\":\"\"}";

/// Why the tile can't be written right now.
pub const TILE_NEEDS_STEAM_CLOSED: &str = "Steam is running, so Turbine can't change its list of non-Steam games now. Close Steam (Desktop Mode), then try again.";

// ---- For the app ----

/// Why Turbine doesn't restart Steam to change Steam's own settings files in Game Mode.
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
        quote(tile.how()),
        quote(tile.words(true)),
    )
}
