//! 0.56: save backups before Play, "Why won't it start?", the launch option doctor and Check game folder. The
//! engine does the work (saves.rs, deploy::verify, steam::launch_options); this hands it to QML. All read only,
//! except a backup (a copy into Turbine's own folder) and putting one back (which keeps the saves first).

use crate::backend::{data_home, qobject, Worker};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use std::path::PathBuf;
use turbine_engine::json::quote;
use turbine_engine::rules::GameRules;
use turbine_engine::store::Store;
use turbine_engine::{deploy, saves, steam, Game};

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Where a game's save backups are kept: <data>/turbine-mod-manager/saves/<store>-<id>.
pub(crate) fn backups_root(game: &Game) -> PathBuf {
    let id: String = game.id().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    crate::anygame::turbine_data().join("saves").join(id)
}

fn save_folders(game: &Game) -> Vec<PathBuf> {
    saves::folders(game, &GameRules::for_game(game).saves, &turbine_engine::home_dir())
}

/// Every place the game keeps saves, there now or not (a backup can go back into a folder that was deleted).
fn save_locations(game: &Game) -> Vec<PathBuf> {
    saves::locations(game, &GameRules::for_game(game).saves, &turbine_engine::home_dir())
}

/// Before Play: a copy of the game's saves (nothing when Turbine doesn't know where they are). When it can't be
/// made (too big, too many files, no room), the player is told why and Play goes on.
pub(crate) fn backup_before_play(w: &Worker, game: &Game) {
    let folders = save_folders(game);
    if folders.is_empty() {
        return;
    }
    let root = backups_root(game);
    let newest = saves::list(&root).first().map(|s| s.id.clone());
    match saves::backup(&folders, &root, now()) {
        Ok(Some(s)) if Some(&s.id) == newest.as_ref() => crate::log::line("engine", &format!("Saves unchanged since the last backup: {}", game.name)),
        Ok(Some(s)) => crate::log::line("engine", &format!("Saves backed up before Play: {} ({} files)", game.name, s.files)),
        Ok(None) => {}
        Err(e) => w.say(format!("{}'s saves weren't backed up before Play: {e}.", game.name), "warn"),
    }
}

impl qobject::Turbine {
    /// The current game's save backups, newest first: {"folders": n, "backups": [{"id","when","files","bytes"}]}.
    pub fn saves_json(&self) -> QString {
        let Some(game) = self.current_game() else { return QString::from("{\"folders\":0,\"backups\":[]}") };
        let list: Vec<String> = saves::list(&backups_root(&game))
            .iter()
            .map(|s| format!("{{\"id\":{},\"when\":{},\"files\":{},\"bytes\":{}}}", quote(&s.id), s.when, s.files, s.bytes))
            .collect();
        QString::from(format!("{{\"folders\":{},\"backups\":[{}]}}", save_locations(&game).len(), list.join(",")).as_str())
    }

    /// Puts a save backup back (the saves as they are now are kept first).
    pub fn restore_saves(self: Pin<&mut Self>, id: &QString) {
        let Some(game) = self.current_game() else { return };
        let id = id.to_string();
        self.send(
            Some(game.id()),
            "Put back saves",
            true,
            Box::new(move |w: &mut Worker| {
                let folders = save_locations(&game);
                match saves::restore(&id, &folders, &backups_root(&game), now()) {
                    Ok(_) => {
                        crate::log::line("engine", &format!("Saves put back for {}: the backup {id} (the saves before it were kept)", game.name));
                        w.say(format!("{}'s saves are back as they were. The saves from just before are kept as a backup too.", game.name), "ok");
                    }
                    Err(e) if saves::couldnt_keep(&e) => {
                        crate::log::line("engine", &format!("Saves not put back for {}: {e}", game.name));
                        let (i, why) = (id.clone(), e.to_string());
                        w.post(move |t| t.saves_keep_failed(QString::from(i.as_str()), QString::from(why.as_str())));
                    }
                    Err(e) => w.say(format!("Couldn't put the saves back: {}.", crate::log::plain(&e.to_string())), "error"),
                }
            }),
        );
    }

    /// Puts a save backup back without keeping the saves as they are now (the player confirmed it).
    pub fn restore_saves_anyway(self: Pin<&mut Self>, id: &QString) {
        let Some(game) = self.current_game() else { return };
        let id = id.to_string();
        self.send(
            Some(game.id()),
            "Put back saves",
            true,
            Box::new(move |w: &mut Worker| match saves::restore_without_keeping(&id, &save_locations(&game), &backups_root(&game)) {
                Ok(()) => {
                    crate::log::line("engine", &format!("Saves put back for {}: the backup {id} (the saves before it were not kept, as asked)", game.name));
                    w.say(format!("{}'s saves are back as they were.", game.name), "ok");
                }
                Err(e) => w.say(format!("Couldn't put the saves back: {}.", crate::log::plain(&e.to_string())), "error"),
            }),
        );
    }

    /// "Why won't it start?" and Check game folder, read only: {"verify": {"ok","missing":[...],"changed":[...],
    /// "unreadable"} or null, "launch": {"listed", "set", "needed"}, "copies": ["<launcher> · <folder>"],
    /// "saves": n}.
    pub fn game_health(&self) -> QString {
        let Some(game) = self.current_game() else { return QString::from("{}") };
        let rules = GameRules::for_game(&game);
        let root = Store::root_for(&game, &data_home()).join("deploy");
        let paths = deploy::Paths { game: game.install_dir.clone(), backups: root.join("backups"), record: root.join("record.json"), plan: root.join("plan.json") };
        let verify = match deploy::verify(&paths) {
            Ok(v) => format!(
                "{{\"ok\":{},\"missing\":[{}],\"changed\":[{}],\"unreadable\":{}}}",
                v.ok,
                v.missing.iter().map(|x| quote(x)).collect::<Vec<_>>().join(","),
                v.changed.iter().map(|x| quote(x)).collect::<Vec<_>>().join(","),
                v.unreadable
            ),
            Err(_) => "null".into(),
        };
        // 0.58: a game me3 starts with mods needs Turbine's Steam launch helper in its Launch Options.
        use turbine_engine::anticheat as ac;
        let me3 = game.store == turbine_engine::games::Store::Steam && rules.anti_cheat.as_ref().is_some_and(|a| ac::mode(a) == ac::Mode::Me3);
        let needed = if me3 {
            crate::anticheat::ensure_helper(&data_home());
            ac::launch_option(&data_home())
        } else {
            rules.launch_option.clone().unwrap_or_default()
        };
        let set = if game.store == turbine_engine::games::Store::Steam { steam::launch_options(&turbine_engine::home_dir(), &game.store_id) } else { None };
        let ok = if me3 { set.as_deref().is_some_and(|x| ac::launch_option_ok(x, &data_home())) } else { set.as_deref().is_some_and(|x| !needed.is_empty() && x.contains(needed.replace("%command%", "").trim())) };
        let launch = format!(
            "{{\"listed\":{},\"set\":{},\"needed\":{},\"ok\":{ok}}}",
            set.is_some(),
            quote(set.as_deref().unwrap_or("")),
            quote(&needed)
        );
        let copies: Vec<String> = self
            .rust()
            .found
            .iter()
            .filter(|g| g.name.eq_ignore_ascii_case(&game.name) && g.install_dir != game.install_dir)
            .map(|g| quote(&format!("{} · {}", g.store.key(), g.install_dir.display())))
            .collect();
        QString::from(
            format!("{{\"verify\":{verify},\"launch\":{launch},\"copies\":[{}],\"saves\":{}}}", copies.join(","), save_folders(&game).len()).as_str(),
        )
    }
}
