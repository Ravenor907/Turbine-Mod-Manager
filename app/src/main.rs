mod actions;
mod backend;
mod labs;
mod log;
mod online;
mod pad;
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

fn main() {
    let _ = labs::started();
    log::start(log::folder(&backend::data_home()));
    log::line("app", &format!("Turbine Mod Manager {VERSION} (build {}) starting", labs::COMMIT));
    log::line("app", &format!("Device {}; {}", labs::device(), labs::system()));
    // SAFETY: the sink is a plain function that lives for the whole run.
    unsafe { turbine_install_qt_handler(log::qt_message) };
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
    log::clean_exit();
}
