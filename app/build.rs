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
    CxxQtBuilder::new_qml_module(QmlModule::new("Turbine").qml_files(files))
        .qt_module("Quick")
        .qt_module("Gui")
        .files(["src/backend.rs"])
        .cpp_file("src/qtglue.cpp")
        .build();
}
