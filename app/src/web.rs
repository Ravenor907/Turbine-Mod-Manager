//! The app's side of Nexus inside Turbine: what NexusBrowser.qml asks of Rust (declared in backend.rs), the Mod
//! Manager Download links the page navigates to, the controller in the browser, and the sign-in the browser helper
//! brings back from the player's own browser. The decisions themselves are in browse.rs (tested on their own).
//!
//! Downloads: browserPlan decides where a download goes before QML accepts it (it can't move afterwards) and marks
//! the file as handled at once, so the Downloads watcher never adds it a second time; browserDone adds it as a sure
//! match for its game when it lands (Nexus's own file host after a press), or asks which game it is for. A finished
//! file is never removed by Turbine; only the partial file of a download stopped for good (a full disk, too big).

use crate::backend::qobject;
use crate::browse;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use turbine_engine::json::{quote, Json};
use turbine_engine::watch::Arrival;
use turbine_engine::Game;

extern "C" {
    // webglue.cpp
    fn turbine_web_sign_out() -> i32;
    fn turbine_web_set_cookie(
        name: *const std::os::raw::c_char,
        value: *const std::os::raw::c_char,
        host: *const std::os::raw::c_char,
        host_only: i32,
        path: *const std::os::raw::c_char,
        expires: f64,
        secure: i32,
        http_only: i32,
        same_site: i32,
    ) -> i32;
    // qtglue.cpp
    fn turbine_key_event(key: i32, pressed: i32, text: *const std::os::raw::c_char) -> i32;
}

/// Qt's key codes for the keys the controller sends into the page.
const KEY_LEFT: i32 = 0x0100_0012;
const KEY_UP: i32 = 0x0100_0013;
const KEY_RIGHT: i32 = 0x0100_0014;
const KEY_DOWN: i32 = 0x0100_0015;
const KEY_RETURN: i32 = 0x0100_0004;
const KEY_PAGE_UP: i32 = 0x0100_0016;
const KEY_PAGE_DOWN: i32 = 0x0100_0017;

/// One download Turbine's browser accepted.
#[derive(Clone, Debug)]
pub struct WebDownload {
    pub path: PathBuf,
    pub game: Option<Game>,
    /// The Nexus page it was started from (domain, mod, file), when it was one.
    pub page: Option<(String, u64, Option<u64>)>,
}

/// A Mod Manager Download link webglue.cpp's scheme handler took, after it checked the request's initiator is
/// nexusmods.com: checked again here (browse::nxm_from_page), then queued like a link from anywhere else.
pub fn nxm_link_from_page(url: &str, initiator: &str) {
    match browse::nxm_from_page_now(url, initiator, Instant::now(), &mut |l| crate::single::push(l)) {
        browse::NxmFromPage::Ask(r) => crate::log::line(
            "web",
            &format!("Mod Manager Download link from the browser (scheme handler, from {}): Turbine asks before it downloads: {r}", browse::log_url(initiator)),
        ),
        browse::NxmFromPage::Refused(e) => crate::log::line("web", &format!("Mod Manager Download link refused: {e}")),
    }
}

/// Signed in to Nexus in Turbine's browser: its sign-in cookie is in the profile (webglue.cpp watches the cookie
/// store), or the browser helper brought the sign-in in.
static SIGNED_IN: AtomicBool = AtomicBool::new(false);

pub fn set_signed_in(on: bool) {
    if SIGNED_IN.swap(on, Ordering::SeqCst) != on {
        crate::log::line("web", if on { "Turbine's browser is signed in to Nexus" } else { "Turbine's browser is signed out of Nexus" });
    }
}

/// The Downloads folder Turbine's browser saves into.
fn downloads_dir() -> PathBuf {
    turbine_engine::home_dir().join("Downloads")
}

/// Game Mods/<Game> for a game.
pub fn game_mods_dir(game: &Game) -> PathBuf {
    downloads_dir().join(browse::GAME_MODS).join(turbine_engine::watch::game_folder_name(&game.name))
}

fn c(s: &str) -> std::ffi::CString {
    std::ffi::CString::new(s.replace('\0', "")).unwrap_or_default()
}

/// A key pressed then let go into the page.
fn send_key(key: i32) {
    let none = c("");
    // SAFETY: plain calls into qtglue.cpp on the GUI thread; the text lives for the calls.
    unsafe {
        turbine_key_event(key, 1, none.as_ptr());
        turbine_key_event(key, 0, none.as_ptr());
    }
}

impl qobject::Turbine {
    // ------------------------------------------------------------------------------------------------ controller

    /// The browser is on screen (true) or closed: the pointer stays on screen the whole time it is open, and hides
    /// as soon as it closes (so RT pages the list again at once instead of clicking Turbine).
    pub fn browser_mode(mut self: Pin<&mut Self>, on: bool) {
        if self.rust().pointer.sticky == on {
            return;
        }
        self.as_mut().rust_mut().pointer.sticky = on;
        self.as_mut().rust_mut().key_nav.pointer();
        self.as_mut().set_browser_key_nav(false);
        crate::log::line("pad", if on { "Browser open: the pointer stays on screen" } else { "Browser closed: the pointer hides" });
        if !on {
            self.as_mut().pointer_hide();
            return;
        }
        if !self.rust().pointer.shown {
            if let Some((w, h)) = crate::pad::window_size() {
                let (x, y) = if self.rust().pointer.x >= 0.0 { (self.rust().pointer.x, self.rust().pointer.y) } else { (w / 2.0, h / 2.0) };
                self.as_mut().rust_mut().pointer.place(x, y);
                self.as_mut().pointer_publish();
                crate::pad::send(crate::pad::Kind::Move, x, y, 0, 0, 0);
            }
        }
    }

    /// The D-pad (or the left stick pushed) in the page: an arrow key, pressed then let go. With spatial navigation
    /// the page's own focus moves between links and buttons, and it scrolls where there are none. From now on A
    /// presses Enter on that focus, until the pointer moves or clicks.
    pub fn browser_key(mut self: Pin<&mut Self>, dir: &QString) {
        let key = match dir.to_string().as_str() {
            "up" => KEY_UP,
            "down" => KEY_DOWN,
            "left" => KEY_LEFT,
            "right" => KEY_RIGHT,
            _ => return,
        };
        self.as_mut().rust_mut().key_nav.key();
        if !*self.browser_key_nav() {
            self.as_mut().set_browser_key_nav(true);
        }
        send_key(key);
    }

    /// The D-pad held for a while in the page: a page up or down per repeat instead of five arrows.
    pub fn browser_page_key(self: Pin<&mut Self>, dir: &QString) {
        match dir.to_string().as_str() {
            "pageup" => send_key(KEY_PAGE_UP),
            "pagedown" => send_key(KEY_PAGE_DOWN),
            _ => {}
        }
    }

    /// A on the page: the same left click RT makes at the pointer (or Enter on the page's focused element, only
    /// after the D-pad moved that focus). For the build's scripted presses; a controller's A comes through handle_pad.
    pub fn browser_click(mut self: Pin<&mut Self>) {
        self.as_mut().browser_a(true);
        self.as_mut().browser_a(false);
    }

    /// A press of the player's in the page (A, RT, R3, a click or a tap): a download or a Mod Manager Download link
    /// in the next moments is his; QML's timers start again.
    pub(crate) fn browser_press_seen(mut self: Pin<&mut Self>) {
        browse::note_press(Instant::now());
        let n = self.browser_press_count().wrapping_add(1);
        self.as_mut().set_browser_press_count(n);
    }

    /// A mouse button or a finger went down on the page (QML's handler over the page sees it on its way in).
    pub fn browser_pointer_down(&self) {
        browse::note_pointer_down();
    }

    /// ... and was let go as a tap or a click: a press, made where it went down, unless the main frame changed in
    /// between (then it counts as nothing).
    pub fn browser_pressed(mut self: Pin<&mut Self>) {
        if browse::note_pointer_up(Instant::now()) {
            let n = self.browser_press_count().wrapping_add(1);
            self.as_mut().set_browser_press_count(n);
        } else {
            crate::log::line("web", "A tap or click didn't count: the page changed while it was held down");
        }
    }

    /// A pressed or let go in the browser. One press is one click (held, it holds the button down).
    pub(crate) fn browser_a(mut self: Pin<&mut Self>, pressed: bool) {
        if pressed {
            let n = self.rust().browser_presses + 1;
            self.as_mut().rust_mut().browser_presses = n;
            crate::log::line("pad", &format!("Browser press {n} (A)"));
            self.as_mut().browser_press_seen();
            let enter = self.rust().key_nav.enter(*self.browser_enter());
            if enter {
                self.as_mut().rust_mut().a_enter = true;
                let n = self.rust().browser_clicks + 1;
                self.as_mut().rust_mut().browser_clicks = n;
                crate::log::line("pad", &format!("Browser click {n} (A): Enter on the link or button the D-pad picked"));
                let t = c("\r");
                // SAFETY: as in send_key.
                unsafe { turbine_key_event(KEY_RETURN, 1, t.as_ptr()) };
                return;
            }
            let src = self.rust().mouse_src[0];
            self.as_mut().rust_mut().mouse_src[0] = src | 4;
            if src == 0 {
                self.as_mut().mouse_button(0, true, "a");
            }
        } else {
            if self.rust().a_enter {
                self.as_mut().rust_mut().a_enter = false;
                let t = c("\r");
                // SAFETY: as in send_key.
                unsafe { turbine_key_event(KEY_RETURN, 0, t.as_ptr()) };
                return;
            }
            let src = self.rust().mouse_src[0];
            if src & 4 != 0 {
                let left = src & !4;
                self.as_mut().rust_mut().mouse_src[0] = left;
                if left == 0 {
                    self.as_mut().mouse_button(0, false, "a");
                }
            }
        }
    }

    // ------------------------------------------------------------------------------------------------ pages

    /// The page Get it opens for a mod's file.
    pub fn browser_start_url(&self, domain: &QString, mod_id: f64, file_id: f64) -> QString {
        let d = domain.to_string();
        if d.is_empty() || !d.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-') || mod_id < 1.0 {
            return QString::default();
        }
        QString::from(browse::start_url(&d, mod_id as u64, file_id.max(0.0) as u64).as_str())
    }

    /// Does this address count as Nexus (its own pages, or the build's stand-in)? The bar shows other hosts in a
    /// warning chip.
    pub fn browser_is_nexus(&self, url: &QString) -> bool {
        browse::is_nexus_page(&url.to_string())
    }

    pub fn browser_zoom_default(&self, height: f64, size: f64) -> f64 {
        browse::zoom_default(height, size)
    }

    /// A Mod Manager Download link QML's navigation guard caught: a permit (the navigation goes on to webglue.cpp's
    /// scheme handler, which checks the initiator) only for the main frame while the page is on Nexus. False: QML
    /// stops the navigation there.
    pub fn browser_nxm(self: Pin<&mut Self>, url: &QString, main_frame: bool, page_url: &QString) -> bool {
        let u = url.to_string();
        let r = turbine_engine::nxm::parse(&u).map(|l| l.redacted()).unwrap_or_else(|_| "a link Turbine can't read".into());
        match browse::nxm_permit(&u, main_frame, &page_url.to_string(), Instant::now()) {
            Ok(()) => true,
            Err(e) => {
                crate::log::line("web", &format!("Mod Manager Download link refused ({r}): {e}"));
                false
            }
        }
    }

    /// The main frame shows an address now: off Nexus, the player's press is gone, and a press made there never counts
    /// for a download nor lets a Mod Manager Download link through without a question.
    pub fn browser_main_frame(&self, url: &QString) {
        let u = url.to_string();
        if browse::note_main_frame(&u, Instant::now()) {
            crate::log::line("web", &format!("The page left Nexus ({}): your last press no longer counts for a download", browse::log_url(&u)));
        }
    }

    /// Must Turbine ask before this Mod Manager Download link downloads ("Get <mod> for <game>?")? True when it
    /// wasn't for the pressed Nexus page's own mod.
    pub fn nxm_must_ask(&self, link: &QString) -> bool {
        browse::must_ask(&link.to_string())
    }

    pub fn browser_log_url(&self, url: &QString) -> QString {
        QString::from(browse::log_url(&url.to_string()).as_str())
    }

    pub fn browser_same_page(&self, a: &QString, b: &QString) -> bool {
        browse::same_page(&a.to_string(), &b.to_string())
    }

    pub fn browser_page_mod(&self, url: &QString) -> f64 {
        browse::page_ids(&url.to_string()).map(|(_, m, _)| m as f64).unwrap_or(0.0)
    }

    pub fn browser_signed_in(&self) -> bool {
        SIGNED_IN.load(Ordering::SeqCst)
    }

    /// A running download, twice a second: "" to go on, or why it stops now (QML cancels it, then browserFailed
    /// removes what it left).
    pub fn browser_dl_check(&self, path: &QString, got: f64) -> QString {
        let p = PathBuf::from(path.to_string());
        let free = p.parent().and_then(crate::system::free_bytes);
        match browse::keep_going(if got.is_finite() { got as i64 } else { 0 }, free) {
            Ok(()) => QString::default(),
            Err(e) => QString::from(e.as_str()),
        }
    }

    // ------------------------------------------------------------------------------------------------ downloads

    /// Where a download the page started goes, or why it is refused. Answers {"ok", "dir", "name", "path", "game",
    /// "folder", "sure", "mod", "error"}. On ok the file is marked as handled now, so the watcher never adds it too.
    pub fn browser_plan(mut self: Pin<&mut Self>, url: &QString, suggested: &QString, page_url: &QString, total: f64, pending: f64) -> QString {
        let (url, suggested, page) = (url.to_string(), suggested.to_string(), page_url.to_string());
        let ids = browse::page_ids(&page);
        let game = ids.as_ref().and_then(|(d, _, _)| self.game_for_domain(d));
        let req = browse::Request { url: url.clone(), suggested, total: if total.is_finite() { total as i64 } else { -1 } };
        let dl = downloads_dir();
        let taken: Vec<PathBuf> = self.rust().web_downloads.iter().map(|d| d.path.clone()).collect();
        let free = crate::system::free_bytes(&dl);
        let host = browse::host_of(&url).map(|(h, _)| h).unwrap_or_else(|| "an unknown address".into());
        let pending = if pending.is_finite() && pending > 0.0 { pending as u64 } else { 0 };
        let plan = browse::plan(&req, &dl, game.as_ref().map(|g| g.name.as_str()), free, pending, &taken, browse::last_press(), Instant::now());
        let p = match plan {
            Ok(p) => p,
            Err(e) => {
                crate::log::line("web", &format!("Download refused ({host}): {e}"));
                return QString::from(format!("{{\"ok\":false,\"error\":{}}}", quote(&e)).as_str());
            }
        };
        if let Err(e) = std::fs::create_dir_all(&p.dir) {
            let why = format!("Turbine couldn't make the folder {} ({e})", p.dir.display());
            crate::log::line("web", &why);
            return QString::from(format!("{{\"ok\":false,\"error\":{}}}", quote(&why)).as_str());
        }
        let dir = p.dir.canonicalize().unwrap_or_else(|_| p.dir.clone());
        let path = dir.join(&p.name);
        self.as_mut().mark_handled(&path);
        if p.sure {
            crate::log::line("web", &format!("Download: {} into Game Mods/{} (from {host}, after your press: a sure match)", p.name, p.folder));
        } else {
            crate::log::line("web", &format!("Download: {} into Game Mods/{} (from {host}, not Nexus's own file host: Turbine asks which game)", p.name, p.folder));
        }
        let (game, ids) = if p.sure { (game, ids) } else { (None, None) };
        let gname = game.as_ref().map(|g| g.name.clone()).unwrap_or_default();
        let mod_id = ids.as_ref().map(|(_, m, _)| *m).unwrap_or(0);
        {
            let mut rm = self.as_mut().rust_mut();
            // Interrupted downloads stay in the list until they are cancelled or finish; the oldest rows go past 50.
            rm.web_downloads.push(WebDownload { path: path.clone(), game, page: ids });
            if rm.web_downloads.len() > 50 {
                rm.web_downloads.remove(0);
            }
        }
        QString::from(
            format!(
                "{{\"ok\":true,\"dir\":{},\"name\":{},\"path\":{},\"game\":{},\"folder\":{},\"sure\":{},\"mod\":{},\"error\":\"\"}}",
                quote(&dir.to_string_lossy()),
                quote(&p.name),
                quote(&path.to_string_lossy()),
                quote(&gname),
                quote(&p.folder),
                p.sure,
                mod_id
            )
            .as_str(),
        )
    }

    /// What became of a downloaded file, for its row in the Downloads tab (matched by its path).
    pub(crate) fn add_result(mut self: Pin<&mut Self>, path: &Path, ok: bool, ask: bool, name: &str, game: &str, text: &str) {
        let n = self.rust().add_results + 1;
        self.as_mut().rust_mut().add_results = n;
        let j = format!(
            "{{\"n\":{n},\"path\":{},\"ok\":{ok},\"ask\":{ask},\"name\":{},\"game\":{},\"text\":{}}}",
            quote(&path.to_string_lossy()),
            quote(name),
            quote(game),
            quote(text)
        );
        self.as_mut().set_add_result_json(QString::from(j.as_str()));
    }

    /// A download finished: added to its game as a sure match (Nexus's own file host after a press), or Turbine asks
    /// which game it is for. A file Turbine didn't plan (dropped from the list) is never left marked as handled: it
    /// is decided the usual way.
    pub fn browser_done(mut self: Pin<&mut Self>, path: &QString) {
        let p = PathBuf::from(path.to_string());
        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|x| x.as_secs()).unwrap_or(0);
        self.as_mut().unmark_handled(&p);
        let Some(d) = self.rust().web_downloads.iter().find(|d| d.path == p).cloned() else {
            crate::log::line("web", &format!("A finished download Turbine didn't plan: {name}; deciding it the usual way"));
            return self.catch_arrival(Arrival { path: p, size, arrived: now, catch_up: false });
        };
        self.as_mut().rust_mut().web_downloads.retain(|x| x.path != p);
        crate::log::line("web", &format!("Downloaded {name} ({})", browse::size_words(size)));
        match d.game {
            Some(g) => {
                let hint = d.page.map(|(domain, m, f)| (crate::bridge::NexusRef { domain, mod_id: m, file_id: f }, None));
                self.catch_add(g, p, hint, None);
            }
            None => {
                self.as_mut().add_result(&p, false, true, &name, "", "Which game is it for? Turbine asks before adding it.");
                self.catch_ask(Arrival { path: p, size, arrived: now, catch_up: false }, "it didn't come from Nexus's own file host");
            }
        }
    }

    /// A download stopped (interrupted or cancelled): said in the log; the page's Try again resumes it. Stopped for
    /// good (cancelled, a full disk, too big): its partial file goes at once.
    pub fn browser_failed(mut self: Pin<&mut Self>, path: &QString, why: &QString) {
        let p = PathBuf::from(path.to_string());
        let why = why.to_string();
        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        crate::log::line("web", &format!("Download of {name} stopped: {why}"));
        let gone_for_good = why == "cancelled" || why.starts_with("stopped:") || why.starts_with("nospace");
        if gone_for_good {
            self.as_mut().rust_mut().web_downloads.retain(|x| x.path != p);
            self.as_mut().unmark_handled(&p);
        }
        if why.starts_with("stopped:") || why.starts_with("nospace") {
            for f in browse::remove_partial(&p) {
                crate::log::line("web", &format!("Removed {} (the download stopped for good)", f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()));
            }
        }
    }

    /// A Mod Manager Download from the browser landed in the temporary folder: it goes into Game Mods/<Game>
    /// (marked as handled first, so the watcher leaves it to this), then is added like a Slow download.
    pub(crate) fn web_place_fetched(mut self: Pin<&mut Self>, game: Game, f: crate::online::Fetched, dir: PathBuf) {
        let name = f.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| format!("{}.zip", f.label));
        if let Err(e) = std::fs::create_dir_all(&dir) {
            crate::log::line("nxm", &format!("Couldn't make {} ({e}); adding from the temporary folder", dir.display()));
            return self.add_fetched(game, f);
        }
        let dir = dir.canonicalize().unwrap_or(dir);
        let dest = dir.join(browse::unique_name(&dir, &name, &[]));
        self.as_mut().mark_handled(&dest);
        let moved = std::fs::rename(&f.path, &dest).is_ok() || {
            // Another drive: copied in as .part, renamed once it is all there.
            let part = dest.with_file_name(format!("{}.part", dest.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()));
            let ok = std::fs::copy(&f.path, &part).is_ok() && std::fs::rename(&part, &dest).is_ok();
            if ok {
                let _ = std::fs::remove_file(&f.path);
            } else {
                let _ = std::fs::remove_file(&part);
            }
            ok
        };
        if !moved {
            crate::log::line("nxm", &format!("Couldn't put {name} into {}; adding it from the temporary folder", dir.display()));
            self.as_mut().unmark_handled(&dest);
            return self.add_fetched(game, f);
        }
        crate::log::line("nxm", &format!("Saved {} into Game Mods/{}", dest.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()));
        self.as_mut().unmark_handled(&dest);
        self.catch_add(game, dest, None, Some(f.source));
    }

    // ------------------------------------------------------------------------------------------------ accounts

    /// Settings, Accounts: Sign out of Nexus in Turbine (the browser's cookies and cache).
    pub fn browser_sign_out(mut self: Pin<&mut Self>) {
        crate::log::line("action", "Sign out of Nexus in Turbine");
        crate::bridge::signin_end();
        // SAFETY: a plain call into webglue.cpp on the GUI thread.
        let r = unsafe { turbine_web_sign_out() };
        if r == 1 {
            set_signed_in(false);
            self.as_mut().say("Signed out of Nexus in Turbine.", "ok");
        } else {
            self.as_mut().say("Turbine's browser couldn't sign out.", "warn");
        }
    }

    /// Labs: Nexus page: draw without the GPU (from the next start).
    pub fn set_web_no_gpu(self: Pin<&mut Self>, on: bool) {
        let f = crate::log::folder(&crate::backend::data_home()).join(browse::NO_GPU_FILE);
        let r = if on {
            std::fs::create_dir_all(f.parent().unwrap_or(Path::new("."))).and_then(|_| std::fs::write(&f, "on\n"))
        } else {
            std::fs::remove_file(&f).or_else(|e| if e.kind() == std::io::ErrorKind::NotFound { Ok(()) } else { Err(e) })
        };
        crate::log::line("labs", &format!("Nexus page: draw without the GPU {} (from the next start){}", if on { "on" } else { "off" }, r.err().map(|e| format!(": {e}")).unwrap_or_default()));
    }

    pub fn web_no_gpu(&self) -> bool {
        browse::no_gpu_wanted(&crate::log::folder(&crate::backend::data_home()))
    }

    // ------------------------------------------------------------------------------------------------ sign-in

    /// "Sign in with my browser (Google and others)": asks the browser helper for Nexus's sign-in once the player
    /// has signed in there, and opens Nexus's sign-in page in his own browser. Answers {"how", "url", "steam",
    /// "text"}: "nohelper" (the helper isn't set up: text says how, and that email and password work right here),
    /// "desktop" (QML opens url in the default browser), "shortcut" (Game Mode: QML opens steam, the player's
    /// browser through Steam; the helper opens the page), "steam" (Game Mode with no browser in Steam: Steam's own
    /// browser, which can't hand the sign-in back; text says so).
    pub fn signin_start(self: Pin<&mut Self>) -> QString {
        crate::log::line("action", "Sign in with my browser");
        let out = |how: &str, url: &str, steam: &str, text: &str| {
            QString::from(format!("{{\"how\":{},\"url\":{},\"steam\":{},\"text\":{}}}", quote(how), quote(url), quote(steam), quote(text)).as_str())
        };
        let url = crate::bridge::SIGNIN_URL;
        if !crate::bridge::paired() {
            crate::log::line("web", "Sign in with my browser: the browser helper isn't set up");
            return out("nohelper", url, "", NO_HELPER);
        }
        match crate::bridge::signin_request_for(*self.gamemode()) {
            Ok(_) => crate::log::line(
                "web",
                &format!("Sign in with my browser: asked the helper for Nexus's sign-in{}", if crate::bridge::connected() { "" } else { " (sent when it connects, within 15 minutes)" }),
            ),
            Err(e) => {
                crate::log::line("web", &format!("Sign in with my browser couldn't start: {e}"));
                return out("error", url, "", "Turbine couldn't start the sign-in. Try again.");
            }
        }
        if !*self.gamemode() {
            return out("desktop", url, "", "Sign in in your browser. Turbine signs in by itself when you're done.");
        }
        let r = turbine_engine::json::parse(&crate::launch::open_through_steam(url)).unwrap_or(Json::Null);
        match r.str_at("how") {
            Some("shortcut") => {
                let name = r.str_at("browser").unwrap_or("your browser").to_string();
                out("shortcut", url, r.str_at("steam").unwrap_or(""), &format!("Sign in in {name}. Turbine signs in by itself and comes back when you're done."))
            }
            _ => {
                let link = turbine_engine::browser::steam_browser_link(url).unwrap_or_default();
                crate::log::line("web", "Sign in with my browser: no Chrome-family browser in Steam; Steam's browser opens, and it can't hand the sign-in back");
                out("steam", url, &link, STEAM_ONLY)
            }
        }
    }

    /// The helper's answer to a sign-in request (bridge.rs checked the channel; this checks the message).
    pub(crate) fn signin_cookies(mut self: Pin<&mut Self>, msg: &Json) {
        let want = crate::bridge::signin_waiting(Instant::now());
        let id = msg.str_at("id").unwrap_or("").to_string();
        let (cookies, dropped) = match crate::bridge::signin_cookies(msg, want.as_deref()) {
            Ok(x) => x,
            Err(e) => {
                crate::log::line("web", &format!("Sign-in from the browser refused: {e}"));
                if crate::bridge::signin_id_ok(&id) {
                    crate::bridge::broadcast(&crate::bridge::signin_done_message(&id, false, "Turbine didn't take this sign-in. Press Sign in with my browser in Turbine again."));
                }
                // Only for the request that is waiting (an old tab answering late says nothing in Turbine).
                if want.as_deref().is_some_and(|w| crate::bridge::same_token(&id, w)) {
                    self.as_mut().web_signin_note(QString::from(SIGNIN_REFUSED), QString::from("warn"));
                }
                return;
            }
        };
        let mut taken = 0;
        let mut names: Vec<String> = Vec::new();
        for ck in &cookies {
            let (n, v, h, p) = (c(&ck.name), c(&ck.value), c(&ck.host), c(&ck.path));
            // SAFETY: plain call into webglue.cpp on the GUI thread; the strings live for the call and are copied there.
            let r = unsafe {
                turbine_web_set_cookie(n.as_ptr(), v.as_ptr(), h.as_ptr(), i32::from(ck.host_only), p.as_ptr(), ck.expires, i32::from(ck.secure), i32::from(ck.http_only), ck.same_site)
            };
            if r == 1 {
                taken += 1;
                if names.len() < 12 {
                    names.push(ck.name.clone());
                }
            }
        }
        // Names and counts only: never a value.
        crate::log::line(
            "web",
            &format!("Sign-in from the browser: {taken} Nexus cookie(s) put into Turbine's browser ({}){}", names.join(", "), if dropped > 0 { format!("; {dropped} for other sites or unreadable, dropped") } else { String::new() }),
        );
        if taken == 0 {
            crate::bridge::broadcast(&crate::bridge::signin_done_message(&id, false, "Turbine couldn't use this sign-in. Sign in on Nexus here again, then press Try again."));
            self.as_mut().web_signin_note(QString::from(SIGNIN_UNUSABLE), QString::from("warn"));
            return;
        }
        // No more cookies for this request; the API key may still come for it (the player's choice on the page).
        crate::bridge::signin_taken();
        set_signed_in(true);
        let who = crate::bridge::signin_name(&cookies);
        let text = match &who {
            Some(n) => format!("Signed in to Nexus as {n}"),
            None => "Signed in to Nexus".to_string(),
        };
        crate::log::line("web", &format!("{text} (from the player's browser)"));
        crate::bridge::broadcast(&crate::bridge::signin_done_message(&id, true, "Turbine has your Nexus sign-in. Go back to Turbine."));
        let back = self.come_back_link();
        self.as_mut().web_signed_in(QString::from(text.as_str()), QString::from(back.as_str()));
    }

    /// Where the sign-in is in the player's browser (signin.status): one line in the activity log, and the pill says
    /// the page is up.
    pub(crate) fn signin_status(mut self: Pin<&mut Self>, msg: &Json) {
        let want = crate::bridge::signin_key_waiting(Instant::now());
        let Some(line) = crate::bridge::signin_status_line(msg, want.as_deref()) else { return };
        crate::log::line("web", &format!("Sign in with my browser: {line}"));
        if msg.str_at("step") == Some("page") && crate::bridge::signin_waiting(Instant::now()).is_some() {
            self.as_mut().web_signin_note(QString::from(SIGNIN_PAGE_UP), QString::from("wait"));
        }
    }

    /// The Personal API Key the helper read on the player's press (signin.key): checked against the request here,
    /// then with Nexus on the download lane (actions.rs), and kept like a pasted key. Never logged.
    pub(crate) fn signin_key(self: Pin<&mut Self>, msg: &Json) {
        let want = crate::bridge::signin_key_waiting(Instant::now());
        let id = msg.str_at("id").unwrap_or("").to_string();
        if let Some(k) = msg.str_at("key") {
            crate::log::secret(k);
        }
        match crate::bridge::signin_key(msg, want.as_deref()) {
            Ok(key) => {
                crate::log::line("web", "API key from the browser helper: checking it with Nexus");
                self.signin_key_check(id, key);
            }
            Err(e) => {
                crate::log::line("web", &format!("API key from the browser refused: {e}"));
                if crate::bridge::signin_id_ok(&id) {
                    crate::bridge::broadcast(&crate::bridge::signin_keydone_message(&id, false, "Turbine didn't take this key. Press Sign in with my browser in Turbine again, or paste the key in Turbine (Settings, Accounts)."));
                }
            }
        }
    }

    /// How Turbine comes back to the front in Game Mode after the player's browser: steam://rungameid of Turbine
    /// itself (the id Steam started it with, else its non-Steam shortcut), or "" (QML then tells the player: press
    /// the Steam button, then Turbine).
    pub(crate) fn come_back_link(&self) -> String {
        if !*self.gamemode() {
            return String::new();
        }
        if let Some(l) = std::env::var("SteamGameId").ok().as_deref().and_then(turbine_engine::shortcuts::rungameid_from_env) {
            crate::log::line("web", &format!("Coming back to Turbine through Steam ({l}, the id Steam started it with)"));
            return l;
        }
        match turbine_engine::shortcuts::find_turbine(&turbine_engine::home_dir()) {
            Some((appid, name)) => {
                let l = turbine_engine::shortcuts::rungameid_url(appid);
                crate::log::line("web", &format!("Coming back to Turbine through Steam ({l}, its shortcut \"{name}\")"));
                l
            }
            None => {
                crate::log::line("web", "No way to bring Turbine to the front found: telling the player to press the Steam button, then Turbine");
                String::new()
            }
        }
    }

    // ------------------------------------------------------------------------------------------------ build checks

    /// The build's checks of the 0.54 regression fix, in a test run only: "watch" starts the Downloads watcher,
    /// "bridge:<json>" hands a helper message to Turbine as if the helper sent it, "drop:<from>|<name>" copies a file
    /// into Downloads, "partial:<name>" grows an unfinished download there, "page:<url>" opens a Nexus page the
    /// Desktop way (the Slow download hint), "signin" passes a made-up sign-in from the helper through (0.55).
    pub fn selftest_catch(mut self: Pin<&mut Self>, step: &QString) -> QString {
        if !crate::online::using_samples() {
            return QString::from("not a test run");
        }
        let step = step.to_string();
        let dl = downloads_dir();
        let said = if step == "watch" {
            // No catch-up on the test home's old downloads: only what the check drops in counts.
            let path = crate::log::folder(&crate::backend::data_home()).join("catch.json");
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|x| x.as_secs()).unwrap_or(0);
            let _ = turbine_engine::watch::CatchState { last_seen: now, ..Default::default() }.save(&path);
            self.as_mut().start_downloads_watch();
            "watching".to_string()
        } else if let Some(j) = step.strip_prefix("bridge:") {
            match turbine_engine::json::parse(j) {
                Ok(msg) => {
                    let kind = msg.str_at("type").unwrap_or("").to_string();
                    self.as_mut().bridge_message(&kind, &msg);
                    format!("sent {kind}")
                }
                Err(e) => format!("bad json: {e}"),
            }
        } else if let Some(x) = step.strip_prefix("drop:") {
            let (from, name) = x.split_once('|').unwrap_or((x, ""));
            if name.is_empty() || name.contains('/') {
                "bad name".to_string()
            } else {
                match std::fs::copy(from, dl.join(name)) {
                    Ok(n) => format!("dropped {name} ({n} bytes)"),
                    Err(e) => format!("couldn't drop {name}: {e}"),
                }
            }
        } else if let Some(name) = step.strip_prefix("partial:") {
            if name.contains('/') {
                "bad name".to_string()
            } else {
                use std::io::Write;
                let r = std::fs::OpenOptions::new().create(true).append(true).open(dl.join(name)).and_then(|mut f| f.write_all(&[0u8; 4096]));
                format!("partial {name}: {}", if r.is_ok() { "grew" } else { "failed" })
            }
        } else if let Some(url) = step.strip_prefix("page:") {
            self.as_mut().watch_page(&QString::from(url)).to_string()
        } else if step == "signin" {
            // The sign-in pass-through end to end, as if the player pressed Sign in with my browser and the helper
            // answered: first an answer for another request (refused whole), then the real one with a Nexus cookie,
            // another site's and a look-alike's (only Nexus's goes into the profile). The value is never logged.
            match crate::bridge::signin_request() {
                Ok(id) => {
                    let msg = |id: &str| {
                        format!(
                            "{{\"type\":\"signin.cookies\",\"id\":{},\"cookies\":[{},{},{}]}}",
                            quote(id),
                            "{\"name\":\"turbine_signin_check\",\"value\":\"valueNeverLogged7f3\",\"domain\":\".nexusmods.com\",\"hostOnly\":false,\"path\":\"/\",\"secure\":true,\"httpOnly\":true,\"sameSite\":\"lax\",\"expirationDate\":4102444800}",
                            "{\"name\":\"other_site_check\",\"value\":\"x\",\"domain\":\".google.com\",\"path\":\"/\"}",
                            "{\"name\":\"look_alike_check\",\"value\":\"x\",\"domain\":\"evilnexusmods.com\",\"path\":\"/\"}"
                        )
                    };
                    let wrong = "0".repeat(32);
                    for m in [msg(&wrong), msg(&id)] {
                        if let Ok(j) = turbine_engine::json::parse(&m) {
                            self.as_mut().signin_cookies(&j);
                        }
                    }
                    "sent".to_string()
                }
                Err(e) => format!("couldn't start: {e}"),
            }
        } else {
            "unknown step".to_string()
        };
        crate::log::line("check", &format!("Catch check {step}: {said}"));
        QString::from(said.as_str())
    }
}

/// The browser's sign-in page is up (the helper said so): what the pill says while Turbine waits.
pub const SIGNIN_PAGE_UP: &str = "Waiting for your Nexus sign-in in the browser. Turbine signs in by itself when you're done.";
/// The helper's sign-in was refused (an old or broken answer).
pub const SIGNIN_REFUSED: &str = "Turbine didn't take the sign-in from the browser. Press Google sign-in to try again.";
/// The helper's sign-in had nothing Turbine's browser could use.
pub const SIGNIN_UNUSABLE: &str = "The browser's Nexus sign-in couldn't be used here. Press Try again on the browser's Turbine page, or Google sign-in to start over.";

/// What the sign-in button says when the browser helper isn't set up (one line).
pub const NO_HELPER: &str = "Google sign-in needs Chrome and the browser helper, set up once in Desktop Mode (Settings, Labs, Use Chrome in Game Mode; then Settings, Getting mods, Using your own browser). Email and password sign-in works right here without it.";
/// Game Mode with no Chrome-family browser in Steam: Steam's browser opens, but only Chrome with the helper can hand
/// the sign-in back.
pub const STEAM_ONLY: &str = "Steam's browser opened, but it can't hand the sign-in back to Turbine. Add Chrome to Steam (Settings, Getting mods), or sign in with email and password right here.";
