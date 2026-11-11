//! Proton settings: the DLL override in a prefix's user.reg, written the way Wine writes it.

use std::fs;
use std::path::{Path, PathBuf};
use turbine_engine::proton::{self, BACKUP_NAME, NOT_STARTED};
use turbine_engine::rules::GameRules;
use turbine_engine::{Game, Runtime, Store};

/// A user.reg the way Proton leaves it, cut down: header, a few keys, values of several kinds, a hex value that
/// goes on over two lines, and an escaped string.
const SAMPLE: &str = "WINE REGISTRY Version 2\n\
;; All keys relative to \\\\User\\\\S-1-5-21-0-0-0-1000\n\
\n\
#arch=win64\n\
\n\
[Control Panel\\\\Desktop] 1730000000\n\
#time=1db2f8e7a6c1a00\n\
\"FontSmoothing\"=\"2\"\n\
\"UserPreferencesMask\"=hex:9e,1e,07,80,12,00,00,00,9e,1e,07,80,12,00,00,00,9e,1e,07,80,12,00,00,00,9e,\\\n\
  1e,07,80\n\
\"Wallpaper\"=\"C:\\\\users\\\\steamuser\\\\Pictures\\\\a \\\"b\\\".png\"\n\
\n\
[Software\\\\Wine\\\\DllOverrides] 1730000001\n\
#time=1db2f8e7a6c1a01\n\
\"atl100\"=\"native,builtin\"\n\
\"d3dcompiler_47\"=\"native,builtin\"\n\
\"xinput1_3\"=\"native,builtin\"\n\
\n\
[Software\\\\Wine\\\\Drives] 1730000002\n\
#time=1db2f8e7a6c1a02\n\
\"x:\"=dword:00000001\n";

fn prefix(name: &str, text: Option<&str>) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("turbine-proton-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    if let Some(t) = text {
        fs::write(dir.join("user.reg"), t).unwrap();
    }
    dir
}

fn reg(p: &Path) -> String {
    fs::read_to_string(p.join("user.reg")).unwrap()
}

#[test]
fn reads_overrides() {
    let p = prefix("read", Some(SAMPLE));
    assert_eq!(proton::read_override(&p, "atl100").unwrap().as_deref(), Some("native,builtin"));
    // Names are matched ignoring case, like Wine does.
    assert_eq!(proton::read_override(&p, "XInput1_3").unwrap().as_deref(), Some("native,builtin"));
    assert_eq!(proton::read_override(&p, "winhttp").unwrap(), None);
    // A value of the same name in another key doesn't count.
    assert_eq!(proton::read_override(&p, "FontSmoothing").unwrap(), None);
}

#[test]
fn adds_the_override_in_order_and_changes_nothing_else() {
    let p = prefix("add", Some(SAMPLE));
    assert!(proton::ensure_override(&p, "winhttp", "native,builtin").unwrap());
    let want = SAMPLE.replace(
        "\"d3dcompiler_47\"=\"native,builtin\"\n",
        "\"d3dcompiler_47\"=\"native,builtin\"\n\"winhttp\"=\"native,builtin\"\n",
    );
    assert_eq!(reg(&p), want);
    assert_eq!(proton::read_override(&p, "winhttp").unwrap().as_deref(), Some("native,builtin"));
    // The first change keeps a copy of the file as it was.
    assert_eq!(fs::read_to_string(p.join(BACKUP_NAME)).unwrap(), SAMPLE);
    // No temporary files left behind.
    let names: Vec<String> = fs::read_dir(&p).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(names.len(), 2, "{names:?}");
}

#[test]
fn same_value_writes_nothing() {
    let p = prefix("same", Some(SAMPLE));
    assert!(!proton::ensure_override(&p, "ATL100", "native,builtin").unwrap());
    assert_eq!(reg(&p), SAMPLE);
    assert!(!p.join(BACKUP_NAME).exists());
}

#[test]
fn changes_an_existing_value_in_place() {
    let text = SAMPLE.replace("\"xinput1_3\"=\"native,builtin\"\n", "\"winhttp\"=\"builtin\"\n\"xinput1_3\"=\"native,builtin\"\n");
    let p = prefix("change", Some(&text));
    assert!(proton::ensure_override(&p, "winhttp", "native,builtin").unwrap());
    assert_eq!(reg(&p), SAMPLE.replace("\"xinput1_3\"=\"native,builtin\"\n", "\"winhttp\"=\"native,builtin\"\n\"xinput1_3\"=\"native,builtin\"\n"));
    // The backup is the file before Turbine's first change.
    assert_eq!(fs::read_to_string(p.join(BACKUP_NAME)).unwrap(), text);
}

#[test]
fn backup_is_made_once() {
    let p = prefix("once", Some(SAMPLE));
    proton::ensure_override(&p, "winhttp", "native,builtin").unwrap();
    proton::ensure_override(&p, "winhttp", "builtin").unwrap();
    assert_eq!(fs::read_to_string(p.join(BACKUP_NAME)).unwrap(), SAMPLE);
    assert_eq!(proton::read_override(&p, "winhttp").unwrap().as_deref(), Some("builtin"));
}

#[test]
fn makes_the_key_when_it_is_missing() {
    let text = SAMPLE.replace(
        "[Software\\\\Wine\\\\DllOverrides] 1730000001\n#time=1db2f8e7a6c1a01\n\"atl100\"=\"native,builtin\"\n\"d3dcompiler_47\"=\"native,builtin\"\n\"xinput1_3\"=\"native,builtin\"\n\n",
        "",
    );
    assert!(!text.contains("DllOverrides"));
    let p = prefix("newkey", Some(&text));
    assert!(proton::ensure_override(&p, "winhttp", "native,builtin").unwrap());
    let out = reg(&p);
    assert!(out.starts_with(&text), "the old text must stay as it was");
    let added = &out[text.len()..];
    let lines: Vec<&str> = added.split('\n').collect();
    assert_eq!(lines[0], "");
    assert!(lines[1].starts_with("[Software\\\\Wine\\\\DllOverrides] "), "{added}");
    assert!(lines[1]["[Software\\\\Wine\\\\DllOverrides] ".len()..].parse::<u64>().is_ok(), "{added}");
    assert!(lines[2].starts_with("#time=1"), "{added}");
    assert_eq!(lines[3], "\"winhttp\"=\"native,builtin\"");
    assert_eq!(lines[4], "");
    assert_eq!(proton::read_override(&p, "winhttp").unwrap().as_deref(), Some("native,builtin"));
}

#[test]
fn key_at_the_end_without_a_final_newline() {
    let text = "WINE REGISTRY Version 2\n\n[Software\\\\Wine\\\\DllOverrides] 1\n\"atl100\"=\"native,builtin\"";
    let p = prefix("noeol", Some(text));
    assert!(proton::ensure_override(&p, "winhttp", "native,builtin").unwrap());
    assert_eq!(reg(&p), format!("{text}\n\"winhttp\"=\"native,builtin\"\n"));
}

#[test]
fn keeps_windows_line_endings() {
    let text = SAMPLE.replace('\n', "\r\n");
    let p = prefix("crlf", Some(&text));
    assert!(proton::ensure_override(&p, "winhttp", "native,builtin").unwrap());
    let want = text.replace(
        "\"d3dcompiler_47\"=\"native,builtin\"\r\n",
        "\"d3dcompiler_47\"=\"native,builtin\"\r\n\"winhttp\"=\"native,builtin\"\r\n",
    );
    assert_eq!(reg(&p), want);
}

#[test]
fn escapes_like_wine() {
    let p = prefix("escape", Some(SAMPLE));
    proton::ensure_override(&p, "odd\"name\\", "a\tb\u{e9}z\u{e9}c").unwrap();
    let out = reg(&p);
    // dump_strW(): backslash and quote get a backslash, tab is \t, and a character over 127 is \x, padded to four
    // digits only when a hex digit follows it.
    assert!(out.contains("\"odd\\\"name\\\\\"=\"a\\tb\\xe9z\\x00e9c\"\n"), "{out}");
    assert_eq!(proton::read_override(&p, "odd\"name\\").unwrap().as_deref(), Some("a\tb\u{e9}z\u{e9}c"));
    // The Wallpaper string with escaped quotes and backslashes is read back right.
    assert_eq!(proton::read_override(&p, "atl100").unwrap().as_deref(), Some("native,builtin"));
}

#[test]
fn a_value_that_is_not_text_is_left_alone() {
    let text = SAMPLE.replace("\"atl100\"=\"native,builtin\"", "\"atl100\"=dword:00000001");
    let p = prefix("dword", Some(&text));
    assert!(proton::read_override(&p, "atl100").is_err());
    // Changing it replaces the line (it is the same setting), keeping everything else.
    assert!(proton::ensure_override(&p, "atl100", "native,builtin").unwrap());
    assert_eq!(reg(&p), SAMPLE);
}

#[test]
fn friendly_error_before_the_first_start() {
    let p = prefix("missing", None);
    assert_eq!(proton::read_override(&p, "winhttp").unwrap_err(), NOT_STARTED);
    assert_eq!(proton::ensure_override(&p, "winhttp", "native,builtin").unwrap_err(), NOT_STARTED);
    assert!(NOT_STARTED.contains("Start the game once from Steam, then try again"));
    assert!(!p.join("user.reg").exists());
    assert!(!p.join(BACKUP_NAME).exists());
}

#[test]
fn other_files_are_refused() {
    let p = prefix("notreg", Some("REGEDIT4\n\n[HKEY_CURRENT_USER\\Software]\n"));
    assert!(proton::ensure_override(&p, "winhttp", "native,builtin").is_err());
    assert_eq!(reg(&p), "REGEDIT4\n\n[HKEY_CURRENT_USER\\Software]\n");
}

#[test]
fn running_pattern_matches_both_path_styles_but_not_itself() {
    let pat = proton::running_pattern(Path::new("/home/deck/.local/share/Steam/steamapps/common/Risk of Rain 2/"));
    assert_eq!(pat, "[/\\\\]home[/\\\\]deck[/\\\\]\\.local[/\\\\]share[/\\\\]Steam[/\\\\]steamapps[/\\\\]common[/\\\\]Risk of Rain 2");
    // The pattern's own text holds no "/home", so pgrep (and flatpak-spawn passing it on) never match themselves.
    assert!(!pat.contains("/home"));
    let pat = proton::running_pattern(Path::new("/games/A+B (1)"));
    assert_eq!(pat, "[/\\\\]games[/\\\\]A\\+B \\(1\\)");
}

fn game(id: &str, runtime: Runtime) -> Game {
    Game {
        store: Store::Steam,
        store_id: id.into(),
        name: "Test".into(),
        install_dir: "/nowhere".into(),
        library: "/nowhere".into(),
        build: None,
        runtime,
        cover: None,
        header: None,
        prefix: None,
    }
}

#[test]
fn which_games_want_the_override() {
    for id in ["1966720", "632360"] {
        let g = game(id, Runtime::Windows);
        assert!(proton::wants_winhttp(&g, &GameRules::for_game(&g)), "{id}");
    }
    // Native Linux builds of BepInEx games start it through a script instead.
    let v = game("892970", Runtime::Native);
    assert!(!proton::wants_winhttp(&v, &GameRules::for_game(&v)));
    // Skyrim's loader works differently.
    let s = game("489830", Runtime::Windows);
    assert!(!proton::wants_winhttp(&s, &GameRules::for_game(&s)));
}
