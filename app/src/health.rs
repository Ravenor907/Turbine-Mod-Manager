//! 0.56: save backups before Play, "Why won't it start?", the launch option doctor and Check game folder. The
//! engine does the work (saves.rs, deploy::verify, steam::launch_options); this hands it to QML. All read only,
//! except a backup (a copy into Turbine's own folder) and putting one back (which keeps the saves first).

use crate::backend::{data_home, qobject, Worker};
use core::pin::Pin;
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
fn backups_root(game: &Game) -> PathBuf {
    let id: String = game.id().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    crate::anygame::turbine_data().join("saves").join(id)
}

fn save_folders(game: &Game) -> Vec<PathBuf> {
    saves::folders(game, &GameRules::for_game(game).saves, &turbine_engine::home_dir())
}

/// Before Play: a copy of the game's saves (nothing when Turbine doesn't know where they are).
pub(crate) fn backup_before_play(game: &Game) {
    let folders = save_folders(game);
    if folders.is_empty() {
        return;
    }
    match saves::backup(&folders, &backups_root(game), now()) {
        Ok(Some(s)) => crate::log::line("engine", &format!("Saves backed up before Play: {} ({} files)", game.name, s.files)),
        Ok(None) => {}
        Err(e) => crate::log::line("engine", &format!("Couldn't back up {}'s saves before Play: {e}", game.name)),
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
        QString::from(format!("{{\"folders\":{},\"backups\":[{}]}}", save_folders(&game).len(), list.join(",")).as_str())
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
                let folders = save_folders(&game);
                match saves::restore(&id, &folders, &backups_root(&game), now()) {
                    Ok(_) => {
                        crate::log::line("engine", &format!("Saves put back for {}: the backup {id} (the saves before it were kept)", game.name));
                        w.say(format!("{}'s saves are back as they were. The saves from just before are kept as a backup too.", game.name), "ok");
                    }
                    Err(e) => w.say(format!("Couldn't put the saves back: {e}"), "error"),
                }
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
        let set = if game.store == turbine_engine::games::Store::Steam { steam::launch_options(&turbine_engine::home_dir(), &game.store_id) } else { None };
        let launch = format!(
            "{{\"listed\":{},\"set\":{},\"needed\":{}}}",
            set.is_some(),
            quote(set.as_deref().unwrap_or("")),
            quote(rules.launch_option.as_deref().unwrap_or(""))
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
