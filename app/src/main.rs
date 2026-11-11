mod actions;
mod anticheat;
mod anygame;
mod backend;
mod bridge;
mod browse;
mod catch;
mod health;
mod labs;
mod launch;
mod log;
mod modsettings;
mod loot;
mod nexus_sso;
mod online;
mod pad;
mod pointer;
mod quick;
mod seen;
mod single;
mod steamsetup;
mod system;
mod web;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};

/// The version from the repository's VERSION file, set by the build (0.1, 0.2 ...).
pub const VERSION: &str = match option_env!("TURBINE_VERSION") {
    Some(v) => v,
    None => "dev",
};

/// Turbine's Flatpak id.
pub const APP_ID: &str = "io.github.ravenor907.TurbineModManager";

/// Seconds since 1970 now.
pub(crate) use turbine_engine::nxm::now as now_secs;

/// Turbine's cache folder (in the Flatpak, ~/.var/app/<id>/cache/turbine-mod-manager: the host can read it, which
/// the self-update's installer needs).
pub(crate) fn cache_dir() -> std::path::PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .filter(|v| !v.is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| turbine_engine::home_dir().join(".cache"))
        .join("turbine-mod-manager")
}

unsafe extern "C" {
    // qtglue.cpp
    fn turbine_install_qt_handler(sink: extern "C" fn(i32, *const std::os::raw::c_char, *const std::os::raw::c_char));
    fn turbine_window_identity();
    // webglue.cpp
    fn turbine_web_prepare(log: extern "C" fn(*const std::os::raw::c_char)) -> i32;
    fn turbine_web_profile(
        data_dir: *const std::os::raw::c_char,
        cache_dir: *const std::os::raw::c_char,
        test_origin: *const std::os::raw::c_char,
        nxm: extern "C" fn(*const std::os::raw::c_char, *const std::os::raw::c_char),
        signed: extern "C" fn(i32),
    );
}

/// A line from webglue.cpp for the activity log.
extern "C" fn web_log(text: *const std::os::raw::c_char) {
    if text.is_null() {
        return;
    }
    // SAFETY: webglue.cpp passes a NUL-terminated UTF-8 string that lives for the call.
    let t = unsafe { std::ffi::CStr::from_ptr(text) }.to_string_lossy().into_owned();
    log::line("web", &t);
}

/// A Mod Manager Download link the Nexus page navigated to (webglue.cpp's nxm scheme handler, which checked the
/// request's initiator first), and that initiator.
extern "C" fn web_nxm(url: *const std::os::raw::c_char, initiator: *const std::os::raw::c_char) {
    if url.is_null() || initiator.is_null() {
        return;
    }
    // SAFETY: as above.
    let u = unsafe { std::ffi::CStr::from_ptr(url) }.to_string_lossy().into_owned();
    // SAFETY: as above.
    let i = unsafe { std::ffi::CStr::from_ptr(initiator) }.to_string_lossy().into_owned();
    web::nxm_link_from_page(&u, &i);
}

/// Turbine's browser has Nexus's sign-in cookie (1) or not (0): webglue.cpp watches the cookie store.
extern "C" fn web_signed(on: i32) {
    web::set_signed_in(on != 0);
}

/// Chromium's switches for the Nexus page, set before WebEngine starts: spatial navigation always (the D-pad moves
/// the page's own focus between links and buttons), and no GPU when Labs' "Nexus page: draw without the GPU" is on
/// (a black or frozen page on some GPUs). Anything already set (the build's checks) is kept.
fn chromium_flags() {
    let mut flags: Vec<String> = std::env::var("QTWEBENGINE_CHROMIUM_FLAGS").unwrap_or_default().split_whitespace().map(str::to_string).collect();
    let mut add = |f: &str| {
        if !flags.iter().any(|x| x == f) {
            flags.push(f.to_string());
        }
    };
    add("--enable-spatial-navigation");
    if browse::no_gpu_wanted(&log::folder(&backend::data_home())) {
        add("--disable-gpu");
    }
    let all = flags.join(" ");
    log::line("web", &format!("Chromium switches: {all}"));
    // SAFETY: main calls this before it starts any thread, so nothing else reads the environment yet.
    unsafe { std::env::set_var("QTWEBENGINE_CHROMIUM_FLAGS", all) };
}

/// QtWebEngine's QML module comes from the Flatpak's base under /app; it is added to where QML looks when it isn't
/// there already.
fn qml_import_path() {
    for dir in ["/app/lib/qml", "/app/lib/x86_64-linux-gnu/qml"] {
        if std::path::Path::new(dir).join("QtWebEngine").is_dir() {
            let now = std::env::var("QML_IMPORT_PATH").unwrap_or_default();
            if !now.split(':').any(|p| p == dir) {
                let v = if now.is_empty() { dir.to_string() } else { format!("{now}:{dir}") };
                // SAFETY: main calls this before it starts any thread.
                unsafe { std::env::set_var("QML_IMPORT_PATH", v) };
            }
            log::line("web", &format!("QtWebEngine's QML from {dir}"));
            return;
        }
    }
    log::line("web", "QtWebEngine's QML module wasn't found under /app/lib/qml");
}

/// The build's stand-in Nexus page: --browser-test=<http://127.0.0.1:port/...> counts as Nexus only together with
/// --selftest-data.
fn browser_test() -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    let samples = args.iter().any(|a| a.starts_with("--selftest-data="));
    let url = args.iter().find_map(|a| a.strip_prefix("--browser-test="))?;
    if !samples {
        log::line("web", "--browser-test needs --selftest-data: ignored");
        return None;
    }
    let origin = url.strip_prefix("http://127.0.0.1:").and_then(|r| {
        let port: String = r.chars().take_while(char::is_ascii_digit).collect();
        (!port.is_empty() && port.len() <= 5).then(|| format!("http://127.0.0.1:{port}"))
    });
    match origin {
        Some(o) => {
            log::line("web", &format!("Test run: {o} stands in for Nexus"));
            browse::set_test_origin(&o);
            Some(o)
        }
        None => {
            log::line("web", "--browser-test must be an http://127.0.0.1:<port>/ address: ignored");
            None
        }
    }
}

/// On SteamOS Turbine draws through X11 (Xwayland in Game Mode): gamescope only shows an overlay window that is
/// marked with an X11 property, and the desktop keeps a window above the browser only for X11 windows. Elsewhere,
/// and when QT_QPA_PLATFORM is already set (the build's screenshot checks set offscreen), nothing changes.
fn choose_platform() -> Option<&'static str> {
    let set = std::env::var_os("QT_QPA_PLATFORM").is_some_and(|v| !v.is_empty());
    let x11 = std::env::var_os("DISPLAY").is_some_and(|v| !v.is_empty());
    if set || !x11 || !system::is_steamos() {
        return None;
    }
    // SAFETY: main calls this before it starts any thread, so nothing else reads the environment yet.
    unsafe { std::env::set_var("QT_QPA_PLATFORM", "xcb") };
    Some("xcb")
}

/// Software drawing where there is no GPU to draw with (WSL, or no render node): Mesa is told to use llvmpipe from
/// the start instead of trying the GPU drivers and zink first (a screen of MESA/ZINK errors, then the same result).
/// Qt Quick keeps drawing through OpenGL, so the effects and the Nexus page work as usual, only slower. Chromium
/// leaves the GPU alone too. Anything the player already set (LIBGL_ALWAYS_SOFTWARE, QT_QUICK_BACKEND) is kept.
fn choose_drawing() -> Option<&'static str> {
    let set = |k: &str| std::env::var_os(k).is_some_and(|v| !v.is_empty());
    if set("LIBGL_ALWAYS_SOFTWARE") || set("QT_QUICK_BACKEND") {
        return None;
    }
    let why = if system::in_wsl() {
        "WSL has no GPU for Mesa"
    } else if !system::has_render_node() {
        "no GPU render node in /dev/dri"
    } else {
        return None;
    };
    // SAFETY: main calls this before it starts any thread, so nothing else reads the environment yet.
    unsafe { std::env::set_var("LIBGL_ALWAYS_SOFTWARE", "1") };
    let flags = std::env::var("QTWEBENGINE_CHROMIUM_FLAGS").unwrap_or_default();
    if !flags.split_whitespace().any(|f| f == "--disable-gpu") {
        // SAFETY: as above.
        unsafe { std::env::set_var("QTWEBENGINE_CHROMIUM_FLAGS", format!("{flags} --disable-gpu").trim()) };
    }
    Some(why)
}

fn main() {
    // The host script's call once Steam has closed: make the waiting change to Steam's settings, then end.
    if let Some(code) = steamsetup::apply_from_args() {
        std::process::exit(code);
    }
    // A request from Steam's menu (the optional Decky plugin): answered without the window, then this ends.
    if let Some(req) = quick::wanted() {
        std::process::exit(quick::run(req));
    }
    let _ = labs::started();
    let platform = choose_platform();
    let software = choose_drawing();
    let pad_filters = pad::clear_steam_filters();
    // One Turbine at a time: a Nexus link given to a second launch goes to the running one, a second launch with no
    // link brings the running one to the front, and either way this one ends before it touches the logs, the
    // partial downloads or the browser's profile (screenshot runs skip this).
    let link = single::link_from_args().filter(|l| single::plausible_link(l));
    let screenshot = std::env::args().any(|a| a.starts_with("--screenshot="));
    let lock_folder = log::folder(&backend::data_home());
    let mut lock = if screenshot { browse::Lock::Unknown("a screenshot run".into()) } else { browse::lock_instance(&lock_folder.join(browse::LOCK_FILE)) };
    let start = if screenshot { None } else { Some(single::start(link.as_deref())) };
    if matches!(start, Some(single::Start::Forwarded)) {
        return;
    }
    // The lock is held by a request from Steam's menu (Mods off, Play...), not by another window: wait for it to
    // finish (seconds), then open as usual.
    let mut waited_for_quick = false;
    if matches!(lock, browse::Lock::Taken) && browse::quick_running(&lock_folder) {
        eprintln!("A request from Steam's menu is running; Turbine opens when it is done.");
        // As long as the plugin waits for a change (decky/main.py CHANGE_TIMEOUT).
        lock = browse::lock_wait(&lock_folder.join(browse::LOCK_FILE), std::time::Duration::from_secs(300));
        waited_for_quick = true;
    }
    if matches!(lock, browse::Lock::Taken) {
        // Another Turbine holds the lock but didn't answer on its socket: this one still never shares its profile.
        eprintln!("Turbine Mod Manager is already running; this second copy closes.");
        if matches!(start, Some(single::Start::First(_))) {
            single::stop();
        }
        return;
    }
    log::start(log::folder(&backend::data_home()));
    log::line("app", &format!("Turbine Mod Manager {VERSION} (build {}) starting", labs::COMMIT));
    log::line("app", &format!("Device {}; {}", labs::device(), labs::system()));
    if let Some(p) = platform {
        log::line("app", &format!("Drawing through {p} (SteamOS: the status overlay needs X11)"));
    }
    if let Some(why) = software {
        log::line("app", &format!("Software drawing ({why})"));
    }
    for l in pad_filters {
        log::line("pad", &l);
    }
    if system::developer() {
        log::line("app", &format!("Developer switch on ({}=1): no \"not supported\" notice", system::DEVELOPER_ENV));
    }
    match &lock {
        browse::Lock::Mine(_) if waited_for_quick => log::line("app", "One Turbine at a time: waited for a request from Steam's menu to finish, then took the lock"),
        browse::Lock::Mine(_) => log::line("app", "One Turbine at a time: this one holds the lock"),
        browse::Lock::Unknown(why) => log::line("app", &format!("One Turbine at a time: not locked ({why}); partial downloads are left alone")),
        browse::Lock::Taken => {}
    }
    // SAFETY: the sink is a plain function that lives for the whole run.
    unsafe { turbine_install_qt_handler(log::qt_message) };
    // Qt WebEngine (the Nexus page): its switches, the nxm scheme and its setup all come before the application.
    chromium_flags();
    qml_import_path();
    let test_origin = browser_test();
    // SAFETY: called once on the main thread before QGuiApplication; the log function lives for the whole run.
    unsafe { turbine_web_prepare(web_log) };
    // A download Turbine's browser was writing when it last closed can't be resumed: its partial file goes (only in
    // the Turbine that holds the lock: another one may be writing them right now).
    if !screenshot {
        for p in browse::remove_partials(&lock, &turbine_engine::watch::downloads_folder(&turbine_engine::home_dir())) {
            log::line("web", &format!("Removed {}: a download that stopped when Turbine closed (Qt can't resume it)", p.display()));
        }
    }
    let redacted = |l: &str| turbine_engine::nxm::parse(l).map(|x| x.redacted()).unwrap_or_default();
    let mut listening = false;
    match start {
        Some(single::Start::First(listener)) => {
            listening = true;
            single::listen(listener, move |l| {
                if l == single::SHOW {
                    log::line("app", "Another launch of Turbine: this one comes to the front");
                } else {
                    log::line("nxm", &format!("Link from another launch: {}", redacted(&l)));
                }
                single::push(l)
            });
        }
        Some(single::Start::Alone(why)) => log::line("nxm", &format!("Not listening for Nexus links: {why}")),
        _ => {}
    }
    if let Some(l) = link {
        log::line("nxm", &format!("Started with a Nexus link: {}", redacted(&l)));
        single::push(l);
    }
    // Open or Report from Steam's menu, asked just before Steam started Turbine for it.
    if !screenshot && let Some(l) = turbine_engine::quick::take_request(&log::folder(&backend::data_home()), now_secs()) {
        log::line("quick", &format!("Started from Steam's menu: {l}"));
        single::push(l);
    }
    // Turbine's own Steam library art (at every start, so after every update too; never over the player's own).
    if !screenshot {
        std::thread::spawn(steamsetup::art_at_start);
    }
    let mut app = QGuiApplication::new();
    if let Some(mut a) = app.as_mut() {
        a.as_mut().set_organization_name(&QString::from("Turbine Mod Manager"));
        a.as_mut().set_application_name(&QString::from("Turbine Mod Manager"));
    }
    // The window carries the menu entry's name and Turbine's icon (otherwise Desktop Mode's taskbar shows a generic one).
    // SAFETY: after QGuiApplication, on the main thread; takes and keeps nothing from this side.
    unsafe { turbine_window_identity() };
    // The browser's profile: kept in Turbine's data folder (signed in stays signed in), its cache in the cache folder.
    let web_data = log::folder(&backend::data_home()).join("browser");
    let web_cache = cache_dir().join("browser");
    let c_data = std::ffi::CString::new(web_data.to_string_lossy().into_owned()).unwrap_or_default();
    let c_cache = std::ffi::CString::new(web_cache.to_string_lossy().into_owned()).unwrap_or_default();
    let c_origin = std::ffi::CString::new(test_origin.unwrap_or_default()).unwrap_or_default();
    // SAFETY: after QGuiApplication, on the main thread; the strings are copied on the C++ side.
    unsafe { turbine_web_profile(c_data.as_ptr(), c_cache.as_ptr(), c_origin.as_ptr(), web_nxm, web_signed) };
    let mut engine = QQmlApplicationEngine::new();
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from("qrc:/qt/qml/Turbine/qml/Main.qml"));
    }
    if let Some(app) = app.as_mut() {
        app.exec();
    }
    if listening {
        single::stop();
    }
    log::clean_exit();
    drop(lock);
}
