mod actions;
mod backend;
mod bridge;
mod catch;
mod labs;
mod log;
mod loot;
mod online;
mod pad;
mod single;
mod system;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};

/// The version from the repository's VERSION file, set by the build (0.1, 0.2 ...).
pub const VERSION: &str = match option_env!("TURBINE_VERSION") {
    Some(v) => v,
    None => "dev",
};

extern "C" {
    // qtglue.cpp
    fn turbine_install_qt_handler(sink: extern "C" fn(i32, *const std::os::raw::c_char, *const std::os::raw::c_char));
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
    // One Turbine at a time: a Nexus link given to a second launch goes to the running one, and this one ends before
    // it touches the logs (screenshot runs skip this).
    let link = single::link_from_args().filter(|l| single::plausible_link(l));
    let screenshot = std::env::args().any(|a| a.starts_with("--screenshot="));
    let start = if screenshot { None } else { Some(single::start(link.as_deref())) };
    if matches!(start, Some(single::Start::Forwarded)) {
        return;
    }
    log::start(log::folder(&backend::data_home()));
    log::line("app", &format!("Turbine Mod Manager {VERSION} (build {}) starting", labs::COMMIT));
    log::line("app", &format!("Device {}; {}", labs::device(), labs::system()));
    if let Some(p) = platform {
        log::line("app", &format!("Drawing through {p} (SteamOS: the status overlay needs X11)"));
    }
    // SAFETY: the sink is a plain function that lives for the whole run.
    unsafe { turbine_install_qt_handler(log::qt_message) };
    let redacted = |l: &str| turbine_engine::nxm::parse(l).map(|x| x.redacted()).unwrap_or_default();
    let mut listening = false;
    match start {
        Some(single::Start::First(listener)) => {
            listening = true;
            single::listen(listener, move |l| {
                log::line("nxm", &format!("Link from another launch: {}", redacted(&l)));
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
}
