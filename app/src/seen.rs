//! What the player has seen and answered (turbine_engine::seen), kept in seen.json beside the activity log, and the
//! interface's questions about requirements, My mods' list, the rail and a game that closed right after Play,
//! answered by the engine. Each answer is JSON text.

use crate::backend::qobject;
use cxx_qt_lib::QString;
use std::path::PathBuf;
use turbine_engine::json::{self, Json};
use turbine_engine::needs::{self, ReqQueue};
use turbine_engine::precheck::PlayCtx;
use turbine_engine::seen::Seen;

fn path() -> PathBuf {
    crate::log::folder(&crate::backend::data_home()).join("seen.json")
}

/// Reads the saved state, changes it, and writes it back.
fn change<R>(f: impl FnOnce(&mut Seen) -> R) -> R {
    let p = path();
    let mut s = Seen::load(&p);
    let r = f(&mut s);
    if let Err(e) = s.save(&p) {
        crate::log::line("app", &format!("What was seen couldn't be saved ({e})"));
    }
    r
}

fn parsed(text: &QString) -> Json {
    json::parse(&text.to_string()).unwrap_or(Json::Null)
}

/// The check's context from the interface, with what was seen.
pub(crate) fn play_ctx(ctx: &str) -> PlayCtx {
    PlayCtx::read(&json::parse(ctx).unwrap_or(Json::Null), &Seen::load(&path()))
}

impl qobject::Turbine {
    pub fn adopt_seen(&self, old: &QString) -> bool {
        let old = parsed(old);
        change(|s| s.adopt(&old))
    }

    pub fn mark_launch_seen(&self, game_id: &QString) {
        change(|s| s.mark_launch(&game_id.to_string()));
    }

    pub fn mark_once_seen(&self, game_id: &QString, titles: &QString) {
        let titles: Vec<String> = parsed(titles).as_array().iter().filter_map(|t| t.as_str().map(str::to_string)).collect();
        change(|s| s.mark_once(&game_id.to_string(), &titles));
    }

    pub fn mark_played(&self, warn_key: &QString) {
        change(|s| s.mark_played(&warn_key.to_string()));
    }

    pub fn update_warned(&self, key: &QString) -> bool {
        Seen::load(&path()).update_warned(&key.to_string())
    }

    pub fn mark_update_warned(&self, key: &QString) {
        change(|s| s.mark_update_warned(&key.to_string()));
    }

    pub fn dep_moves_json(&self, game_id: &QString) -> QString {
        QString::from(Seen::load(&path()).moves_json(&game_id.to_string()).as_str())
    }

    pub fn move_dep(&self, game_id: &QString, mod_id: &QString, into: bool) {
        change(|s| s.set_dep_move(&game_id.to_string(), &mod_id.to_string(), into));
    }

    pub fn clear_dep_moves(&self) {
        change(|s| s.dep_moves.clear());
    }

    pub fn req_queue_json(&self) -> QString {
        QString::from(Seen::load(&path()).req_queue.as_ref().map(ReqQueue::to_json).unwrap_or_else(|| "null".into()).as_str())
    }

    pub fn req_queue_start(&self, game_id: &QString, for_name: &QString, items: &QString) {
        let items = needs::items_from(Some(&parsed(items)));
        change(|s| s.req_queue = Some(ReqQueue::new(&game_id.to_string(), &for_name.to_string(), items)));
    }

    pub fn req_queue_insert_next(&self, items: &QString) -> QString {
        let items = needs::items_from(Some(&parsed(items)));
        let fresh = change(|s| s.req_queue.as_mut().map(|q| q.insert_next(&items)).unwrap_or_default());
        QString::from(needs::items_json(&fresh).as_str())
    }

    pub fn req_queue_advance(&self) -> QString {
        let done = change(|s| s.req_queue.as_mut().and_then(ReqQueue::advance));
        QString::from(done.map(|d| d.to_json()).unwrap_or_else(|| "null".into()).as_str())
    }

    pub fn req_queue_clear(&self) {
        change(|s| s.req_queue = None);
    }

    pub fn needs_json(&self, input: &QString) -> QString {
        QString::from(needs::view_json(&parsed(input)).as_str())
    }

    pub fn mod_list_json(&self, filter: &QString) -> QString {
        QString::from(turbine_engine::modlist::list_json(&parsed(filter)).as_str())
    }

    pub fn quick_exit_json(&self, input: &QString) -> QString {
        QString::from(turbine_engine::store::quick_exit_json(&parsed(input)).as_str())
    }

    pub fn rail_order_json(&self, input: &QString) -> QString {
        QString::from(turbine_engine::games::rail_order_json(&parsed(input)).as_str())
    }
}
