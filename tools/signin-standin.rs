// A stand-in for Turbine's end of "Sign in with my browser", built from the app's real bridge.rs, for the build's
// whole-handshake check (extension/test/handshake.js). It answers the helper the way web.rs and actions.rs do:
// Nexus cookies for the waiting request are taken, then the API key for the same request is "checked with Nexus"
// (here: it must equal the key the test's mocked API Keys page shows, given in TURBINE_STANDIN_KEY).
// Prints "READY <port> <token>" when listening; "signin" on stdin presses Sign in with my browser (Game Mode).
// Every line it prints is checked by the test: no cookie value or key may ever appear in them.
// Build: rustc --edition 2024 tools/signin-standin.rs --extern turbine_engine=<the engine's rlib>
#![allow(dead_code)]

#[path = "../app/src/bridge.rs"]
mod bridge;

use std::io::BufRead;
use std::sync::Arc;
use std::time::Instant;
use turbine_engine::json::Json;

fn say(line: &str) {
    println!("{line}");
}

fn main() {
    let dir = std::env::args().nth(1).expect("a folder for the pairing files");
    let want_key = std::env::var("TURBINE_STANDIN_KEY").unwrap_or_default();
    let handler: bridge::Handler = Arc::new(move |kind: &str, msg: &Json| match kind {
        "signin.cookies" => {
            let id = msg.str_at("id").unwrap_or("").to_string();
            match bridge::signin_cookies(msg, bridge::signin_waiting(Instant::now()).as_deref()) {
                Ok((cookies, _)) if !cookies.is_empty() => {
                    bridge::signin_taken();
                    let names: Vec<&str> = cookies.iter().map(|c| c.name.as_str()).collect();
                    say(&format!("TOOK {}", names.join(",")));
                    bridge::broadcast(&bridge::signin_done_message(&id, true, "Turbine has your Nexus sign-in. Go back to Turbine."));
                }
                Ok(_) => bridge::broadcast(&bridge::signin_done_message(&id, false, "No Nexus sign-in came.")),
                Err(e) => say(&format!("REFUSED {e}")),
            }
        }
        "signin.key" => {
            let id = msg.str_at("id").unwrap_or("").to_string();
            match bridge::signin_key(msg, bridge::signin_key_waiting(Instant::now()).as_deref()) {
                Ok(k) => {
                    let ok = !want_key.is_empty() && k == want_key;
                    say(if ok { "KEY accepted" } else { "KEY rejected" });
                    bridge::broadcast(&bridge::signin_keydone_message(&id, ok, if ok { "Turbine has your API key." } else { "Nexus didn't accept that key." }));
                }
                Err(e) => say(&format!("KEY refused: {e}")),
            }
        }
        "signin.status" => {
            if let Some(l) = bridge::signin_status_line(msg, bridge::signin_key_waiting(Instant::now()).as_deref()) {
                say(&format!("STATUS {l}"));
            }
        }
        "connected" | "disconnected" => say(&format!("BRIDGE {kind}")),
        _ => {}
    });
    let port = bridge::start(std::path::Path::new(&dir), |l| println!("LOG {l}"), handler).expect("the bridge starts");
    let token = bridge::pair_url().and_then(|u| u.split('#').nth(1).map(str::to_string)).expect("a token");
    say(&format!("READY {port} {token}"));
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        if line.trim() == "signin" {
            let id = bridge::signin_request_for(true).expect("a request");
            say(&format!("ASKED {}", &id[..8]));
        }
    }
}
