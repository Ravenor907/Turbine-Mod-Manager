mod actions;
mod anygame;
mod backend;
mod bridge;
mod browse;
mod catch;
mod labs;
mod launch;
mod log;
mod loot;
mod online;
mod pad;
mod pointer;
mod single;
mod system;
mod web;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};

/// The version from the repository's VERSION file, set by the build (0.1, 0.2 ...).
pub const VERSION: &str = match option_env!("TURBINE_VERSION") {
    Some(v) => v,
    None => "dev",
};

extern "C" {
    // qtglue.cpp
    fn turbine_install_qt_handler(sink: extern "C" fn(i32, *const std::os::raw::c_char, *const std::os::raw::c_char));
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
    // Before any thread starts: nothing else reads the environment yet.
    std::env::set_var("QTWEBENGINE_CHROMIUM_FLAGS", all);
}

/// QtWebEngine's QML module comes from the Flatpak's base under /app; it is added to where QML looks when it isn't
/// there already.
fn qml_import_path() {
    for dir in ["/app/lib/qml", "/app/lib/x86_64-linux-gnu/qml"] {
        if std::path::Path::new(dir).join("QtWebEngine").is_dir() {
            let now = std::env::var("QML_IMPORT_PATH").unwrap_or_default();
            if !now.split(':').any(|p| p == dir) {
                let v = if now.is_empty() { dir.to_string() } else { format!("{now}:{dir}") };
                std::env::set_var("QML_IMPORT_PATH", v);
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
    // Before any thread starts: nothing else reads the environment yet.
    std::env::set_var("QT_QPA_PLATFORM", "xcb");
    Some("xcb")
}

fn main() {
    let _ = labs::started();
    let platform = choose_platform();
    // One Turbine at a time: a Nexus link given to a second launch goes to the running one, a second launch with no
    // link brings the running one to the front, and either way this one ends before it touches the logs, the
    // partial downloads or the browser's profile (screenshot runs skip this).
    let link = single::link_from_args().filter(|l| single::plausible_link(l));
    let screenshot = std::env::args().any(|a| a.starts_with("--screenshot="));
    let lock = if screenshot { browse::Lock::Unknown("a screenshot run".into()) } else { browse::lock_instance(&log::folder(&backend::data_home()).join(browse::LOCK_FILE)) };
    let start = if screenshot { None } else { Some(single::start(link.as_deref())) };
    if matches!(start, Some(single::Start::Forwarded)) {
        return;
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
    match &lock {
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
        for p in browse::remove_partials(&lock, &turbine_engine::home_dir().join("Downloads")) {
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
    let mut app = QGuiApplication::new();
    if let Some(mut a) = app.as_mut() {
        a.as_mut().set_organization_name(&QString::from("Turbine Mod Manager"));
        a.as_mut().set_application_name(&QString::from("Turbine Mod Manager"));
    }
    // The browser's profile: kept in Turbine's data folder (signed in stays signed in), its cache in the cache folder.
    let web_data = log::folder(&backend::data_home()).join("browser");
    let web_cache = online::temp_dir().parent().map(|p| p.join("browser")).unwrap_or_else(|| web_data.join("cache"));
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
