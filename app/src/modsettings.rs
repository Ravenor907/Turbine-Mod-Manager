//! 0.58: Edit settings (a mod's config files, or the game's own from the game menu), Seamless Co-op settings (its
//! ersc_settings.ini through the same page, password first), and Elden Ring's saves view (normal, modded and co-op
//! saves apart). The engine reads and writes (turbine_engine::settings, saves); this hands it to QML. Every save
//! is logged, never a password.

use crate::backend::{data_home, qobject, Worker};
use core::pin::Pin;
use cxx_qt::Threading;
use cxx_qt_lib::QString;
use std::path::PathBuf;
use turbine_engine::json::{self, quote, Json};
use turbine_engine::rules::GameRules;
use turbine_engine::settings::{self, Format};
use turbine_engine::store::{self, Store};
use turbine_engine::{deploy, saves, Game};

/// Config files bigger than this aren't offered.
const LIMIT: usize = 40;

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// A setting whose value never goes into the log (pass, pwd, token, secret, key, auth, rcon, webhook in its name).
fn secret(key: &str) -> bool {
    turbine_engine::report::secret_word(key)
}

/// Where the game's own config files may be edited: not for a game me3 starts (its folder must stay as Steam
/// installed it) and not for an online-only game.
fn game_files_allowed(game: &Game) -> bool {
    let r = GameRules::for_game(game);
    r.anti_cheat.as_ref().is_none_or(|a| a.me3_game().is_none() && !a.refuses())
}

struct Target {
    /// The file as it is now (the edited copy, or the mod's or game's own).
    now: PathBuf,
    /// The mod's own file ("" mod: the game's file itself).
    default: PathBuf,
    root: PathBuf,
    game_dir: PathBuf,
    mod_id: String,
}

fn target(game: &Game, mod_id: &str, rel: &str) -> Result<Target, String> {
    if !settings::safe_rel(rel) || settings::format_of(rel).is_none() {
        return Err("That isn't a settings file Turbine can open.".into());
    }
    let root = Store::root_for(game, &data_home());
    if mod_id.is_empty() {
        if !game_files_allowed(game) {
            return Err("This game's own files stay as Steam installed them.".into());
        }
        let f = game.install_dir.join(rel);
        return Ok(Target { now: f.clone(), default: f, root, game_dir: game.install_dir.clone(), mod_id: String::new() });
    }
    if !settings::safe_rel(mod_id) {
        return Err("Not a mod.".into());
    }
    let default = store::staged_dir(&root, mod_id).join(rel);
    if !default.is_file() {
        return Err("The mod doesn't have that file any more.".into());
    }
    let now = settings::effective(&root, mod_id, rel, &default);
    Ok(Target { now, default, root, game_dir: game.install_dir.clone(), mod_id: mod_id.to_string() })
}

/// The game's own config files: in its folder (three levels down at most), not ones Turbine placed.
fn game_files(game: &Game) -> Vec<String> {
    if !game_files_allowed(game) {
        return Vec::new();
    }
    let root = Store::root_for(game, &data_home()).join("deploy");
    let ours: Vec<String> = deploy::Record::load(&root.join("record.json")).map(|r| r.files.iter().map(|p| p.rel.to_lowercase()).collect()).unwrap_or_default();
    settings::config_files(&game.install_dir, 3, LIMIT * 2).into_iter().filter(|r| !ours.contains(&r.to_lowercase())).take(LIMIT).collect()
}

/// The page's JSON for one file (see `settings_load`).
fn settings_json(game: &Game, id: &str, rel: &str) -> String {
    let head = format!("\"mod\":{},\"rel\":{}", quote(id), quote(rel));
    let t = match target(game, id, rel) {
        Ok(t) => t,
        Err(e) => return format!("{{{head},\"error\":{}}}", quote(&e)),
    };
    let text = match std::fs::read(&t.now) {
        Ok(b) if b.len() as u64 <= settings::MAX_BYTES => String::from_utf8_lossy(&b).to_string(),
        Ok(_) => return format!("{{{head},\"error\":\"That file is too big to edit here (over 512 KB). Open it as text in Desktop Mode.\"}}"),
        Err(e) => return format!("{{{head},\"error\":{}}}", quote(&format!("Couldn't read it: {}", crate::log::plain(&e.to_string())))),
    };
    let fmt = settings::format_of(rel).unwrap_or(Format::Ini);
    let big = text.len() > settings::PARSE_MAX;
    let fields = settings::parse(&text, fmt);
    let lines: Vec<String> = text.split('\n').take(600).map(|l| quote(l.trim_end_matches('\r'))).collect();
    let edited = if id.is_empty() { settings::game_original(&t.root, rel).exists() } else { t.now != t.default };
    // 0.6: settings to look at again after the mod's last update (the player's file before it is kept).
    let (look, gone) = if id.is_empty() { (Vec::new(), Vec::new()) } else { settings::review_notes(&t.root, id, rel) };
    let review = settings::review_words("The mod", &look, &gone).unwrap_or_default();
    let review_names: Vec<String> = look.iter().chain(gone.iter()).map(|n| quote(n)).collect();
    crate::log::line("settings", &format!("Edit settings open: {} {} ({}, {} settings{})", if id.is_empty() { game.name.as_str() } else { id }, rel, fmt.key(), fields.len(), if big { ", too big to read as settings: plain text" } else { "" }));
    format!(
        "{{{head},\"format\":\"{}\",\"fields\":{},\"lines\":[{}],\"edited\":{edited},\"big\":{big},\"review\":{},\"reviewNames\":[{}],\"error\":\"\"}}",
        fmt.key(),
        settings::fields_json(&fields),
        lines.join(","),
        quote(&review),
        review_names.join(",")
    )
}

impl qobject::Turbine {
    /// The settings files of a mod of the current game ("" for the game's own): [{"rel","edited"}].
    pub fn settings_files(&self, mod_id: &QString) -> QString {
        let Some(game) = self.current_game() else { return QString::from("[]") };
        let id = mod_id.to_string();
        let root = Store::root_for(&game, &data_home());
        let list: Vec<(String, bool)> = if id.is_empty() {
            game_files(&game).into_iter().map(|r| (r.clone(), settings::game_original(&root, &r).exists())).collect()
        } else if settings::safe_rel(&id) {
            settings::config_files(&store::staged_dir(&root, &id), 8, LIMIT).into_iter().map(|r| (r.clone(), settings::edited_path(&root, &id, &r).is_file())).collect()
        } else {
            Vec::new()
        };
        let items: Vec<String> = list
            .iter()
            .map(|(r, e)| {
                let review = !id.is_empty() && settings::review_path(&root, &id, r).is_file();
                format!("{{\"rel\":{},\"edited\":{e},\"review\":{review}}}", quote(r))
            })
            .collect();
        QString::from(format!("[{}]", items.join(",")).as_str())
    }

    /// 0.62: is Seamless Co-op's session password set, for the game on screen? 1 set, 0 empty (a co-op start
    /// must open Co-op settings first), -1 when its settings file isn't found.
    pub fn coop_password_state(&self, mod_id: &QString) -> i32 {
        let Some(game) = self.current_game() else { return -1 };
        let root = Store::root_for(&game, &data_home());
        match coop_password(&game, &root, &mod_id.to_string()) {
            Some(p) if p.set() => 1,
            Some(_) => 0,
            None => -1,
        }
    }

    /// One settings file for the page, read and parsed on its own thread (a big file never stops the interface):
    /// the answer comes as `settingsLoaded` {"mod","rel","format","fields":[...],"lines":[...],"edited","big","error"}.
    pub fn settings_load(self: Pin<&mut Self>, mod_id: &QString, rel: &QString) {
        let (id, rel) = (mod_id.to_string(), rel.to_string());
        let Some(game) = self.current_game() else { return };
        let qt = self.qt_thread();
        let _ = std::thread::Builder::new().name("turbine-settings-read".into()).spawn(move || {
            let json = settings_json(&game, &id, &rel);
            let _ = qt.queue(move |t| t.settings_loaded(QString::from(json.as_str())));
        });
    }

    /// Saves changes: {"fields":[{"id","value"}]} or {"lines":[{"line","text"}]}. Answers {"ok","error"}. A mod's
    /// file goes into Turbine's own copy (the mod's file is never changed); the game's file in place, its original
    /// and the copy before kept.
    pub fn settings_save(&self, mod_id: &QString, rel: &QString, edits: &QString) -> QString {
        let answer = |ok: bool, e: &str| QString::from(format!("{{\"ok\":{ok},\"error\":{}}}", quote(e)).as_str());
        let Some(game) = self.current_game() else { return answer(false, "No game") };
        let (id, rel) = (mod_id.to_string(), rel.to_string());
        let t = match target(&game, &id, &rel) {
            Ok(t) => t,
            Err(e) => return answer(false, &e),
        };
        let Ok(text) = std::fs::read_to_string(&t.now) else { return answer(false, "Couldn't read the file (or it isn't text Turbine can keep as it is).") };
        let fmt = settings::format_of(&rel).unwrap_or(Format::Ini);
        let Ok(v) = json::parse(&edits.to_string()) else { return answer(false, "Nothing to save.") };
        // By each setting's identity in the file as it is now, never its position: a file that changed since the page
        // opened is refused rather than changed in the wrong place.
        let mut edits: Vec<settings::Edit> = Vec::new();
        let mut said: Vec<String> = Vec::new();
        for f in v.get("fields").map(Json::as_array).unwrap_or(&[]) {
            let (Some(key), Some(was), Some(val)) = (f.str_at("key"), f.str_at("was"), f.str_at("value")) else { continue };
            if was == val {
                continue;
            }
            said.push(if secret(key) { format!("{key} changed") } else { format!("{key} {was} to {val}") });
            edits.push(settings::Edit {
                section: f.str_at("section").unwrap_or("").to_string(),
                key: key.to_string(),
                occ: f.get("occ").and_then(num).unwrap_or(0),
                was: was.to_string(),
                value: val.to_string(),
            });
        }
        let mut new = match settings::set_by_key(&text, fmt, &edits) {
            Ok(t) => t,
            Err(e) => return answer(false, &e),
        };
        for l in v.get("lines").map(Json::as_array).unwrap_or(&[]) {
            let (Some(n), Some(was), Some(tx)) = (l.get("line").and_then(num), l.str_at("was"), l.str_at("text")) else { continue };
            let now = new.split('\n').nth(n.saturating_sub(1)).map(|x| x.trim_end_matches('\r'));
            if now != Some(was) {
                return answer(false, &format!("Line {n} changed in the file since this page opened. Open it again."));
            }
            match settings::set_line(&new, n, tx) {
                Ok(t) => {
                    new = t;
                    said.push(format!("line {n}"));
                }
                Err(e) => return answer(false, &e),
            }
        }
        if new == text {
            return answer(true, "");
        }
        let r = if t.mod_id.is_empty() { settings::save_game_file_at(&t.root, &t.game_dir, &rel, &new, game.build.as_deref()) } else { settings::save_edit(&t.root, &t.mod_id, &rel, &t.default, &new) };
        match r {
            Ok(()) => {
                if !t.mod_id.is_empty() {
                    settings::clear_review(&t.root, &t.mod_id, &rel);
                }
                crate::log::line("settings", &format!("Edit settings saved: {} {}: {}", if id.is_empty() { game.name.as_str() } else { id.as_str() }, rel, said.join("; ")));
                answer(true, "")
            }
            Err(e) => {
                crate::log::line("settings", &format!("Edit settings: couldn't save {rel}: {e}"));
                answer(false, &format!("Couldn't save it: {}", crate::log::plain(&e.to_string())))
            }
        }
    }

    /// Reset to default: the mod's own file again (or the game's original). The version before is kept as a backup.
    pub fn settings_reset(&self, mod_id: &QString, rel: &QString) -> QString {
        let Some(game) = self.current_game() else { return QString::from("{\"ok\":false,\"error\":\"No game\"}") };
        let (id, rel) = (mod_id.to_string(), rel.to_string());
        let t = match target(&game, &id, &rel) {
            Ok(t) => t,
            Err(e) => return QString::from(format!("{{\"ok\":false,\"error\":{}}}", quote(&e)).as_str()),
        };
        let r = if id.is_empty() { settings::reset_game_file_at(&t.root, &t.game_dir, &rel, game.build.as_deref()) } else { settings::reset(&t.root, &id, &rel) };
        let json = match r {
            Ok(done) => {
                crate::log::line("settings", &format!("Edit settings reset to default: {} {} ({})", if id.is_empty() { game.name.as_str() } else { id.as_str() }, rel, if done { "done" } else { "already the default" }));
                "{\"ok\":true,\"error\":\"\"}".to_string()
            }
            Err(e) => format!("{{\"ok\":false,\"error\":{}}}", quote(&format!("Couldn't reset it: {}", crate::log::plain(&e.to_string())))),
        };
        QString::from(json.as_str())
    }

    /// Elden Ring's saves, each kind apart: {"kinds":[{"kind","label","files":[{"rel","bytes","modified"}],
    /// "backups":[{"id","when","count"}]}]}.
    pub fn er_saves_json(&self) -> QString {
        let Some(game) = self.current_game() else { return QString::from("{\"kinds\":[]}") };
        let rules = GameRules::for_game(&game);
        let folders = saves::folders(&game, &rules.saves, &turbine_engine::home_dir());
        let root = crate::health::backups_root(&game);
        let snaps = saves::list(&root);
        let ext = coop_ext(&game);
        let kinds: Vec<String> = [saves::Kind::Solo, saves::Kind::Modded, saves::Kind::Coop]
            .iter()
            .map(|k| {
                let files: Vec<String> = saves::files_of_with(&folders, *k, &ext)
                    .iter()
                    .map(|f| format!("{{\"rel\":{},\"bytes\":{},\"modified\":{}}}", quote(&f.rel), f.bytes, f.modified))
                    .collect();
                let backups: Vec<String> = snaps
                    .iter()
                    .filter_map(|s| {
                        let n = saves::count_in_with(&root, &s.id, *k, &ext);
                        (n > 0).then(|| format!("{{\"id\":{},\"when\":{},\"count\":{n}}}", quote(&s.id), s.when))
                    })
                    .collect();
                format!("{{\"kind\":\"{}\",\"label\":{},\"files\":[{}],\"backups\":[{}]}}", k.key(), quote(k.label()), files.join(","), backups.join(","))
            })
            .collect();
        QString::from(format!("{{\"folders\":{},\"kinds\":[{}]}}", folders.len(), kinds.join(",")).as_str())
    }

    /// Puts back one kind of Elden Ring save from a backup (the saves as they are now are kept first).
    pub fn restore_save_kind(self: Pin<&mut Self>, kind: &QString, id: &QString) {
        let Some(game) = self.current_game() else { return };
        let (kind, id) = (kind.to_string(), id.to_string());
        self.send(
            Some(game.id()),
            "Put back one kind of save",
            true,
            Box::new(move |w: &mut Worker| {
                let Some(k) = saves::Kind::from_key(&kind) else { return };
                let rules = GameRules::for_game(&game);
                let folders = saves::locations(&game, &rules.saves, &turbine_engine::home_dir());
                match saves::restore_kind_with(&id, k, &folders, &crate::health::backups_root(&game), now(), &coop_ext(&game)) {
                    Ok(0) => w.say(format!("That backup has no {} saves.", k.label()), "warn"),
                    Ok(n) => {
                        crate::log::line("engine", &format!("Saves put back for {}: {} from {id} ({n} files; the saves before were kept)", game.name, k.label()));
                        w.say(format!("{}: {} save back as it was. The others are untouched, and the saves from just before are kept too.", game.name, k.label()), "ok");
                    }
                    Err(e) => w.say(format!("Couldn't put that save back: {e}."), "error"),
                }
            }),
        );
    }
}

/// 0.62: Seamless Co-op's session password in both places it lives: Turbine's copy of ersc_settings.ini (the one
/// Co-op settings shows) and the copy the mod itself loads (in the me3 profile's mods folder, placed by the last
/// apply). The mod refuses to start with an empty one, so every co-op start checks the copy it will load.
pub(crate) struct CoopPassword {
    /// Turbine's copy: without the spaces or quotes round it ("" when not set).
    pub mine: String,
    /// The copy the mod loads, when it is in place.
    pub loaded: Option<String>,
    pub loaded_file: PathBuf,
}

impl CoopPassword {
    /// Is there a password the mod will get (after `coop_password_ready` brought its copy up to date)?
    pub(crate) fn set(&self) -> bool {
        !self.mine.is_empty() || self.loaded.as_deref().is_some_and(|l| !l.is_empty())
    }
}

/// A password value as the mod reads it: no spaces round it and no pair of quotes ("" and '' are empty).
pub(crate) fn plain_password(v: &str) -> String {
    let t = v.trim();
    let quoted = t.len() >= 2 && ((t.starts_with('"') && t.ends_with('"')) || (t.starts_with('\'') && t.ends_with('\'')));
    let t = if quoted { &t[1..t.len() - 1] } else { t };
    t.trim().to_string()
}

/// The password setting in a settings file's text (cooppassword under [PASSWORD]).
fn password_field(text: &str) -> Option<settings::Field> {
    settings::parse(text, Format::Ini).into_iter().find(|f| f.key.to_ascii_lowercase().contains("password"))
}

/// None when Seamless Co-op's settings file (or its password setting) isn't found.
pub(crate) fn coop_password(game: &Game, root: &std::path::Path, mod_id: &str) -> Option<CoopPassword> {
    if !settings::safe_rel(mod_id) {
        return None;
    }
    let staged = store::staged_dir(root, mod_id);
    let rel = settings::config_files(&staged, 8, LIMIT).into_iter().find(|r| r.to_ascii_lowercase().ends_with("ersc_settings.ini"))?;
    let mine_file = settings::effective(root, mod_id, &rel, &staged.join(&rel));
    let mine = plain_password(&password_field(&std::fs::read_to_string(mine_file).ok()?)?.value);
    // Where the mod's files go: a game me3 starts has them in its profile's mods folder, never the game folder.
    let loaded_file = if GameRules::for_game(game).anti_cheat.as_ref().and_then(|a| a.me3_game()).is_some() {
        turbine_engine::anticheat::profile_mods(root).join(turbine_engine::anticheat::package_rel(&rel))
    } else {
        game.install_dir.join(&rel)
    };
    let loaded = std::fs::read_to_string(&loaded_file).ok().and_then(|t| password_field(&t)).map(|f| plain_password(&f.value));
    Some(CoopPassword { mine, loaded, loaded_file })
}

/// The id of the installed Seamless Co-op, if it is installed.
pub(crate) fn seamless_id(store: &Store) -> Option<String> {
    // The copy the game loads, known by its files: one from Nexus counts like Turbine's own download.
    store.seamless().map(|m| m.id)
}

pub(crate) const NEEDS_PASSWORD: &str = "Set the Seamless Co-op session password first. Turbine opens its settings.";

/// Just before a Seamless Co-op start, after the mods went in. The copy of the settings the mod loads can be older
/// than Turbine's (a settings file changed in place is kept by an apply): when its password is empty or differs,
/// Turbine's goes into it. Err with the words to say when there is no password at all: the start must not go on.
pub(crate) fn coop_password_ready(game: &Game, root: &std::path::Path, mod_id: &str) -> Result<(), String> {
    let Some(p) = coop_password(game, root, mod_id) else { return Ok(()) };
    if p.mine.is_empty() {
        return if p.set() { Ok(()) } else { Err(NEEDS_PASSWORD.to_string()) };
    }
    let Some(loaded) = &p.loaded else { return Ok(()) };
    if *loaded == p.mine {
        return Ok(());
    }
    let put = || -> Result<(), String> {
        let text = std::fs::read_to_string(&p.loaded_file).map_err(|e| e.to_string())?;
        let f = password_field(&text).ok_or("its password setting isn't there")?;
        let edit = settings::Edit { section: f.section.clone(), key: f.key.clone(), occ: f.occ, was: f.value.clone(), value: p.mine.clone() };
        let new = settings::set_by_key(&text, Format::Ini, &[edit])?;
        let tmp = p.loaded_file.with_extension("ini.turbine-tmp");
        std::fs::write(&tmp, new).and_then(|_| std::fs::rename(&tmp, &p.loaded_file)).map_err(|e| e.to_string())
    };
    match put() {
        Ok(()) => {
            crate::log::line("settings", &format!("Seamless Co-op: the settings the game loads had {} session password; Turbine's went into {}", if loaded.is_empty() { "no" } else { "another" }, p.loaded_file.display()));
            Ok(())
        }
        Err(e) => {
            crate::log::line("settings", &format!("Seamless Co-op: couldn't put the session password into {}: {e}", p.loaded_file.display()));
            if loaded.is_empty() {
                Err(format!("Couldn't put the session password into the co-op settings the game loads: {}", crate::log::plain(&e)))
            } else {
                Ok(())
            }
        }
    }
}

/// Seamless Co-op's save extension, as its settings say (it can be changed there): "co2" otherwise.
fn coop_ext(game: &Game) -> String {
    let root = Store::root_for(game, &data_home());
    let Ok(rd) = std::fs::read_dir(root.join("mods")) else { return "co2".into() };
    for e in rd.flatten() {
        let id = e.file_name().to_string_lossy().to_string();
        for rel in settings::config_files(&e.path(), 4, 20).into_iter().filter(|r| r.to_ascii_lowercase().ends_with("ersc_settings.ini")) {
            let src = settings::effective(&root, &id, &rel, &e.path().join(&rel));
            if let Ok(t) = std::fs::read_to_string(src) {
                if let Some(f) = settings::parse(&t, Format::Ini).into_iter().find(|f| f.key == "save_file_extension") {
                    let v = f.value.trim().trim_start_matches('.');
                    if !v.is_empty() && v.chars().all(|c| c.is_ascii_alphanumeric()) {
                        return v.to_string();
                    }
                }
            }
        }
    }
    "co2".into()
}

fn num(j: &Json) -> Option<usize> {
    match j {
        Json::Num(n) if *n >= 0.0 => Some(*n as usize),
        _ => None,
    }
}
