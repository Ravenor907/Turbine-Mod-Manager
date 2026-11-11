//! 0.6: changes to Steam's own files (a game's Launch Options in localconfig.vdf, the co-op tile in shortcuts.vdf)
//! with one Steam restart the player agreed to. Steam keeps those files in memory and writes them back when it
//! closes, so they can only be changed while Steam is closed. In Game Mode Turbine itself is a game Steam started:
//! closing Steam closes Turbine too. So the change is made by a small script on the host, started through
//! `flatpak-spawn --host`, which leaves Steam's process tree at once: it asks Steam to close, waits until it has,
//! runs Turbine's own `--apply-steam-setup` (the engine's careful writers, below), starts Steam again (Game Mode's
//! session starts it by itself; on the desktop the script does) and brings Turbine back through its Steam entry.
//! Every step goes to restart.log; the result goes to result.txt for Turbine to read when it is back.
//!
//! Before any of this Turbine checks that no game is running and no download is under way (`busy`).

use crate::json::{self, quote, Json};
use crate::shortcuts::{self, NewShortcut, TileChange};
use crate::steam_setup::{self, SteamChange};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The folder for the restart's files, in Turbine's data folder.
pub fn dir(data: &Path) -> PathBuf {
    data.join("steam-setup")
}

pub fn job_file(data: &Path) -> PathBuf {
    dir(data).join("job.json")
}

pub fn result_file(data: &Path) -> PathBuf {
    dir(data).join("result.txt")
}

pub fn log_file(data: &Path) -> PathBuf {
    dir(data).join("restart.log")
}

pub fn script_path(data: &Path) -> PathBuf {
    dir(data).join("turbine-steam-setup.sh")
}

/// One change the restart makes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// Set a game's Launch Options (`value` "" takes them away).
    Options { file: PathBuf, app_id: String, value: String },
    /// Add a tile (with its art from `art`, recorded in `record`).
    Tile { file: PathBuf, tile: NewShortcut, marker: String, art: PathBuf, record: PathBuf },
    /// Remove a tile and Turbine's art for it.
    NoTile { file: PathBuf, marker: String, record: PathBuf },
}

/// What the restart is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub changes: Vec<Change>,
    /// What Turbine does when it is back ("play steam:1245620 solo", "" for nothing): read by the app.
    pub resume: String,
    /// The player's words for what is set up ("ELDEN RING's launch setting").
    pub what: String,
    /// When it was asked (seconds since 1970): an old job is never resumed.
    pub at: u64,
}

fn p(v: &Json, k: &str) -> Option<PathBuf> {
    v.str_at(k).map(PathBuf::from)
}

impl Job {
    pub fn to_json(&self) -> String {
        let s = |p: &Path| quote(&p.to_string_lossy());
        let items: Vec<String> = self
            .changes
            .iter()
            .map(|c| match c {
                Change::Options { file, app_id, value } => format!("{{\"kind\":\"options\",\"file\":{},\"app\":{},\"value\":{}}}", s(file), quote(app_id), quote(value)),
                Change::Tile { file, tile, marker, art, record } => format!(
                    "{{\"kind\":\"tile\",\"file\":{},\"marker\":{},\"art\":{},\"record\":{},\"name\":{},\"exe\":{},\"startDir\":{},\"options\":{},\"flatpak\":{}}}",
                    s(file),
                    quote(marker),
                    s(art),
                    s(record),
                    quote(&tile.name),
                    quote(&tile.exe),
                    quote(&tile.start_dir),
                    quote(&tile.launch_options),
                    quote(&tile.flatpak_id)
                ),
                Change::NoTile { file, marker, record } => format!("{{\"kind\":\"notile\",\"file\":{},\"marker\":{},\"record\":{}}}", s(file), quote(marker), s(record)),
            })
            .collect();
        format!("{{\"changes\":[{}],\"resume\":{},\"what\":{},\"at\":{}}}\n", items.join(","), quote(&self.resume), quote(&self.what), self.at)
    }

    pub fn from_json(text: &str) -> Option<Job> {
        let v = json::parse(text).ok()?;
        let mut changes = Vec::new();
        for c in v.get("changes").map(Json::as_array).unwrap_or(&[]) {
            changes.push(match c.str_at("kind")? {
                "options" => Change::Options { file: p(c, "file")?, app_id: c.str_at("app")?.to_string(), value: c.str_at("value")?.to_string() },
                "tile" => Change::Tile {
                    file: p(c, "file")?,
                    marker: c.str_at("marker")?.to_string(),
                    art: p(c, "art")?,
                    record: p(c, "record")?,
                    tile: NewShortcut {
                        name: c.str_at("name")?.to_string(),
                        exe: c.str_at("exe")?.to_string(),
                        start_dir: c.str_at("startDir")?.to_string(),
                        launch_options: c.str_at("options")?.to_string(),
                        flatpak_id: c.str_at("flatpak")?.to_string(),
                        icon: String::new(),
                    },
                },
                "notile" => Change::NoTile { file: p(c, "file")?, marker: c.str_at("marker")?.to_string(), record: p(c, "record")? },
                _ => return None,
            });
        }
        let at = v.get("at").and_then(|a| match a {
            Json::Num(n) => Some(*n as u64),
            _ => None,
        });
        Some(Job { changes, resume: v.str_at("resume").unwrap_or("").to_string(), what: v.str_at("what").unwrap_or("").to_string(), at: at.unwrap_or(0) })
    }

    pub fn save(&self, data: &Path) -> io::Result<()> {
        fs::create_dir_all(dir(data))?;
        let f = job_file(data);
        let tmp = f.with_extension("tmp");
        fs::write(&tmp, self.to_json())?;
        fs::rename(tmp, f)
    }

    pub fn load(data: &Path) -> Option<Job> {
        Job::from_json(&fs::read_to_string(job_file(data)).ok()?)
    }
}

/// Makes the job's changes (Steam is closed: the script only runs this after Steam has gone). Each change is noted
/// for `steam_setup::check_after_restart_since`. Answers one line per change, or the first error.
pub fn apply_job(data: &Path, job: &Job) -> Result<Vec<String>, String> {
    let mut lines = Vec::new();
    for c in &job.changes {
        match c {
            Change::Options { file, app_id, value } => {
                let was = steam_setup::write_launch_options(file, app_id, value, false)?;
                lines.push(format!("Launch Options for {app_id} set (were {})", was.as_deref().map(|w| format!("\"{w}\"")).unwrap_or_else(|| "empty".into())));
                let _ = steam_setup::note_change(data, SteamChange::Options { file: file.clone(), app_id: app_id.clone(), value: value.clone() });
            }
            Change::Tile { file, tile, marker, art, record } => {
                let r = shortcuts::add_tile_with_art(file, tile, marker, art, record, false)?;
                lines.push(match r {
                    TileChange::Added(id) => format!("Tile {} added ({id})", tile.name),
                    TileChange::Already(id) => format!("Tile {} already there ({id})", tile.name),
                    _ => format!("Tile {}: {r:?}", tile.name),
                });
                let _ = steam_setup::note_change(data, SteamChange::Tile { file: file.clone(), marker: marker.clone() });
            }
            Change::NoTile { file, marker, record } => {
                let r = shortcuts::remove_tile_with_art(file, marker, record, false)?;
                lines.push(format!("Tile with {marker}: {r:?}"));
                let _ = steam_setup::note_change(data, SteamChange::NoTile { file: file.clone(), marker: marker.clone() });
            }
        }
    }
    Ok(lines)
}

/// The result the script left: ("done" | "failed" | "running", the words after it). None when there is none.
pub fn read_result(data: &Path) -> Option<(String, String)> {
    let t = fs::read_to_string(result_file(data)).ok()?;
    let l = t.lines().next()?.trim().to_string();
    let (state, words) = l.split_once(' ').map(|(a, b)| (a.to_string(), b.to_string())).unwrap_or((l, String::new()));
    Some((state, words))
}

/// Clears the job and its result (Turbine has resumed, or the player said not now).
pub fn finish(data: &Path) {
    let _ = fs::remove_file(job_file(data));
    let _ = fs::remove_file(result_file(data));
}

// ---- Is Steam busy? ----

/// Steam's registry file (~/.steam/registry.vdf, the client's own state while it runs).
fn registry(home: &Path) -> Option<crate::vdf::Vdf> {
    for f in [home.join(".steam/registry.vdf"), home.join(".steam/steam/registry.vdf")] {
        if let Ok(t) = fs::read_to_string(&f) {
            if let Ok(v) = crate::vdf::parse(&t) {
                return Some(v);
            }
        }
    }
    None
}

/// Appmanifest bits that mean a download is under way now (not only queued or paused).
const ACTIVE_FLAGS: u32 = 256 | 1024 | 1_048_576 | 2_097_152 | 4_194_304;

/// Why Steam shouldn't be restarted now, in the player's words: a game running (Steam's RunningAppID, or a game with
/// Running set), or a download under way (Updating set, or an appmanifest downloading). None when it is fine.
/// `own_appid`: Turbine's own Steam entry, which is running while the player looks at this.
pub fn busy(home: &Path, games: &[crate::Game], own_appid: Option<u32>) -> Option<String> {
    let name = |id: &str| games.iter().find(|g| g.store == crate::Store::Steam && g.store_id == id).map(|g| g.name.clone()).unwrap_or_else(|| format!("a game ({id})"));
    let ours = |id: &str| own_appid.is_some_and(|o| id == o.to_string()) || id == "0" || id.is_empty();
    if let Some(reg) = registry(home) {
        let steam = reg.path(&["Registry", "HKCU", "Software", "Valve", "Steam"]);
        if let Some(id) = steam.and_then(|s| s.str_at(&["RunningAppID"])) {
            if !ours(id) {
                return Some(format!("{} is running. Close it first (save your game), then try again.", name(id)));
            }
        }
        for (id, app) in steam.and_then(|s| s.get("apps")).map(crate::vdf::Vdf::entries).unwrap_or(&[]) {
            if app.str_at(&["Running"]) == Some("1") && !ours(id) {
                return Some(format!("{} is running. Close it first (save your game), then try again.", name(id)));
            }
            if app.str_at(&["Updating"]) == Some("1") {
                return Some(format!("Steam is downloading {}. Let it finish (or pause it), then try again.", name(id)));
            }
        }
    }
    // Every Steam library Turbine knows (the Steam folders and the libraries its games are in).
    let mut libs: Vec<PathBuf> = crate::steam::steam_roots(home);
    for g in games.iter().filter(|g| g.store == crate::Store::Steam) {
        if !libs.contains(&g.library) {
            libs.push(g.library.clone());
        }
    }
    for lib in libs {
        let Ok(rd) = fs::read_dir(lib.join("steamapps")) else { continue };
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if !(n.starts_with("appmanifest_") && n.ends_with(".acf")) {
                continue;
            }
            let Ok(t) = fs::read_to_string(e.path()) else { continue };
            let Ok(v) = crate::vdf::parse(&t) else { continue };
            let flags: u32 = v.str_at(&["AppState", "StateFlags"]).and_then(|s| s.parse().ok()).unwrap_or(4);
            if flags & ACTIVE_FLAGS != 0 {
                let id = v.str_at(&["AppState", "appid"]).unwrap_or("");
                let shown = v.str_at(&["AppState", "name"]).map(str::to_string).unwrap_or_else(|| name(id));
                return Some(format!("Steam is downloading {shown}. Let it finish (or pause it), then try again."));
            }
        }
    }
    None
}

// ---- The host script ----

/// The script's text. Arguments: <data folder> <gamemode|desktop> <Turbine's steam:// link or ""> -- <the command
/// that runs Turbine's --apply-steam-setup>. The first run starts itself again in its own session and returns at
/// once, so nothing that happens to Turbine (Steam closing it) reaches it. Names can be changed for the tests:
/// TURBINE_STEAM (the steam command), TURBINE_STEAM_PROC (Steam's process name), TURBINE_PROC (Turbine's),
/// TURBINE_POLL (seconds between checks), TURBINE_WAIT (how many checks before giving up), TURBINE_SETTLE (seconds
/// between Steam starting and the link back to Turbine).
pub fn script_text() -> String {
    r##"#!/bin/bash
# Turbine Mod Manager: one Steam restart to change Steam's own settings (the player asked for it in Turbine). Steam
# writes its settings back when it closes and reads them when it starts, so: close Steam, wait, let Turbine make the
# change, start Steam again, bring Turbine back. Every step goes to restart.log next to this file.
if [ "$1" != --detached ]; then
  setsid bash "$0" --detached "$@" < /dev/null > /dev/null 2>&1 &
  exit 0
fi
shift
trap '' HUP TERM INT
data="$1"; mode="$2"; back="$3"; shift 3
[ "$1" = -- ] && shift
here="$data/steam-setup"
log="$here/restart.log"
result="$here/result.txt"
steam_cmd="${TURBINE_STEAM:-steam}"
steam_proc="${TURBINE_STEAM_PROC:-steam}"
turbine_proc="${TURBINE_PROC:-turbine-mod-man}"
poll="${TURBINE_POLL:-1}"
tries="${TURBINE_WAIT:-90}"
say() { printf '%s %s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "$*" >> "$log"; }
finish() { printf '%s\n' "$*" > "$result.tmp" && mv -f "$result.tmp" "$result"; say "Result: $*"; }
steam_up() { pgrep -x "$steam_proc" > /dev/null 2>&1; }
wait_for() {
  local want="$1" n=0
  while [ "$n" -lt "$tries" ]; do
    if [ "$want" = up ] && steam_up; then return 0; fi
    if [ "$want" = down ] && ! steam_up; then return 0; fi
    sleep "$poll"
    n=$((n + 1))
  done
  return 1
}
start_steam() {
  say "Starting Steam"
  setsid "$steam_cmd" < /dev/null > /dev/null 2>&1 &
}
printf 'running\n' > "$result"
say "Restart asked ($mode)"
if steam_up; then
  say "Asking Steam to close"
  "$steam_cmd" -shutdown < /dev/null > /dev/null 2>&1 &
  if ! wait_for down; then
    say "Steam didn't close"
    finish "failed Steam didn't close, so nothing was changed. Try again, or restart the device."
    exit 1
  fi
fi
say "Steam is closed"
if out=$("$@" 2>&1); then
  say "Changed: $out"
  state="done"
  words=""
  # 0.6 (break M3): Steam must still be closed now, or it may have read its old settings before the change landed
  # (Game Mode's session starts it again quickly). Then it isn't counted: Turbine offers it again.
  if steam_up; then
    say "Steam started again before the change was written"
    state="failed"
    words="Steam started again too soon, so the change may not hold. Turbine offers it again."
  fi
else
  say "The change failed: $out"
  state="failed"
  words="Steam's settings couldn't be changed: $(printf '%s' "$out" | tail -n 1)"
fi
if [ "$mode" = gamemode ]; then
  say "Waiting for Game Mode to start Steam again"
  if ! wait_for up; then start_steam; wait_for up; fi
else
  start_steam
  wait_for up
fi
if steam_up; then say "Steam is running again"; else say "Steam didn't start again"; state="failed"; words="Steam didn't start again. Start it yourself; the change is made."; fi
finish "$state $words"
if [ -n "$back" ] && ! pgrep -x "$turbine_proc" > /dev/null 2>&1; then
  # Steam takes a moment after it starts before it takes links.
  sleep "${TURBINE_SETTLE:-10}"
  say "Bringing Turbine back: $back"
  "$steam_cmd" "$back" < /dev/null > /dev/null 2>&1 &
fi
exit 0
"##
    .to_string()
}

/// Writes the script (executable) into Turbine's data folder.
pub fn write_script(data: &Path) -> io::Result<PathBuf> {
    let path = script_path(data);
    fs::create_dir_all(dir(data))?;
    let text = script_text();
    if fs::read_to_string(&path).ok().as_deref() != Some(text.as_str()) {
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, &text)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&tmp, fs::Permissions::from_mode(0o755))?;
        }
        fs::rename(&tmp, &path)?;
    }
    Ok(path)
}

/// The script's arguments after `bash <script>`: data, mode, the link back to Turbine, `--`, the apply command.
pub fn script_args(data: &Path, game_mode: bool, back: &str, apply: &[String]) -> Vec<String> {
    let mut v = vec![data.to_string_lossy().to_string(), if game_mode { "gamemode".into() } else { "desktop".into() }, back.to_string(), "--".into()];
    v.extend(apply.iter().cloned());
    v
}

/// The command that applies the job from the host: Turbine's Flatpak with --apply-steam-setup (or this program when
/// Turbine isn't a Flatpak).
pub fn apply_command(flatpak_id: Option<&str>, exe: &Path) -> Vec<String> {
    match flatpak_id {
        Some(id) => vec!["flatpak".into(), "run".into(), "--command=turbine-mod-manager".into(), id.into(), "--apply-steam-setup".into()],
        None => vec![exe.to_string_lossy().to_string(), "--apply-steam-setup".into()],
    }
}
