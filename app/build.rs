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
    CxxQtBuilder::new_qml_module(QmlModule::new("Turbine").qml_files(files))
        .qt_module("Quick")
        .files(["src/backend.rs"])
        .build();
}
