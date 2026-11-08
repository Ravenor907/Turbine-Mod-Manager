//! Lists the built-in game and loader data files (engine/data) so they are part of the engine: dropping a new file
//! into data/games and building again is all a new game needs. Standard library only, like the rest of the engine.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn list(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut out: Vec<(String, PathBuf)> = fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "json") && p.is_file())
                .map(|p| (p.file_name().unwrap_or_default().to_string_lossy().into_owned(), p))
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

fn table(name: &str, files: &[(String, PathBuf)]) -> String {
    let mut s = format!("pub(crate) const {name}: &[(&str, &str)] = &[\n");
    for (file, path) in files {
        let abs = fs::canonicalize(path).unwrap_or_else(|_| path.clone());
        s.push_str(&format!("    ({file:?}, include_str!({:?})),\n", abs.to_string_lossy()));
    }
    s.push_str("];\n");
    s
}

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into()));
    let data = root.join("data");
    let games = list(&data.join("games"));
    let loaders = list(&data.join("loaders"));
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set by cargo (or by hand for a plain rustc build)"));
    let text = format!("{}{}", table("BUILTIN_GAMES", &games), table("BUILTIN_LOADERS", &loaders));
    fs::write(out.join("builtin.rs"), text).expect("write builtin.rs");
    println!("cargo:rerun-if-changed=data");
    println!("cargo:rerun-if-changed=data/games");
    println!("cargo:rerun-if-changed=data/loaders");
}
