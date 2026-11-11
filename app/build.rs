use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    // Every .qml file in qml/ belongs to the Turbine module (Main.qml plus one file per component).
    let mut files: Vec<String> = std::fs::read_dir("qml")
        .expect("qml folder")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("qml"))
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    files.sort();
    for f in &files {
        println!("cargo:rerun-if-changed={f}");
    }
    println!("cargo:rerun-if-changed=qml");
    // The commit this build comes from: the build server writes it into build-commit.txt.
    println!("cargo:rerun-if-changed=build-commit.txt");
    if let Ok(c) = std::fs::read_to_string("build-commit.txt") {
        let c = c.trim();
        if !c.is_empty() && c.chars().all(|ch| ch.is_ascii_alphanumeric()) {
            println!("cargo:rustc-env=TURBINE_COMMIT={c}");
        }
    }
    // Nexus inside Turbine needs Qt WebEngine, which the KDE runtime doesn't have: the Flatpak takes it from the
    // io.qt.qtwebengine.BaseApp base, installed under /app. Qt's own module lookup only searches the SDK's folders,
    // so its headers and libraries are added here by hand. Turbine is only ever built as that Flatpak.
    let web = webengine();
    let mut b = CxxQtBuilder::new_qml_module(QmlModule::new("Turbine").qml_files(files))
        .qt_module("Quick")
        .qt_module("Gui")
        .qt_module("Network")
        .files(["src/backend.rs"])
        .cpp_file("src/qtglue.cpp")
        .cpp_file("src/webglue.cpp");
    // SAFETY: only adds include folders to the C++ compiler cxx-qt-build runs (the documented use of cc_builder).
    b = unsafe {
        b.cc_builder(|cc| {
            for dir in &web.includes {
                cc.include(dir);
            }
        })
    };
    b.build();
    // qtglue.cpp marks the Game Mode overlay window through Xlib.
    println!("cargo:rustc-link-lib=X11");
    println!("cargo:rustc-link-search=native={}", web.lib.display());
    // Qt WebEngine's own libraries (WebChannel, Positioning and so on) are found the same way when linking.
    println!("cargo:rustc-link-arg=-Wl,-rpath-link,{}", web.lib.display());
    println!("cargo:rustc-link-lib=Qt6WebEngineQuick");
    println!("cargo:rustc-link-lib=Qt6WebEngineCore");
}

struct WebEngine {
    lib: std::path::PathBuf,
    includes: Vec<std::path::PathBuf>,
}

/// Where the BaseApp put Qt WebEngine (TURBINE_WEBENGINE_PREFIX overrides /app). Stops the build with one clear
/// message when it isn't there.
fn webengine() -> WebEngine {
    println!("cargo:rerun-if-env-changed=TURBINE_WEBENGINE_PREFIX");
    let prefix = std::path::PathBuf::from(std::env::var("TURBINE_WEBENGINE_PREFIX").unwrap_or_else(|_| "/app".into()));
    let lib = ["lib", "lib/x86_64-linux-gnu", "lib64"]
        .iter()
        .map(|d| prefix.join(d))
        .find(|d| d.join("libQt6WebEngineQuick.so").exists());
    let inc = ["include", "include/qt6", "include/x86_64-linux-gnu/qt6"]
        .iter()
        .map(|d| prefix.join(d))
        .find(|d| d.join("QtWebEngineQuick").is_dir());
    match (lib, inc) {
        (Some(lib), Some(inc)) => {
            let includes = vec![inc.clone(), inc.join("QtWebEngineQuick"), inc.join("QtWebEngineCore")];
            WebEngine { lib, includes }
        }
        _ => panic!(
            "Qt WebEngine wasn't found under {}. Turbine is built as a Flatpak on io.qt.qtwebengine.BaseApp 6.10 \
             (packaging/io.github.ravenor907.TurbineModManager.json), which puts it there.",
            prefix.display()
        ),
    }
}
