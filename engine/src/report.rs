//! 0.58: Settings > Report a problem. A plain text file the player can send: the recent activity log, Turbine's
//! version, its settings, the screen size and each game with how Turbine plays it. Nothing secret goes in: any value
//! after a word like key, token, cookie, password or authorization is replaced, and so is anything that looks like a
//! long secret (a Nexus key, a session id).

/// Is this word (a setting's or header's name) one whose value is secret? Any name with pass, pwd, token, secret,
/// key, auth, rcon, webhook, cookie or session in it.
pub fn secret_word(w: &str) -> bool {
    let l = w.to_ascii_lowercase();
    let l = l.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-');
    ["pass", "pwd", "token", "secret", "key", "auth", "rcon", "webhook", "cookie", "session"].iter().any(|p| l.contains(p)) || l == "sid"
}

/// A run of 24 or more letters and digits (with + = _ - .): a key or token as written. Paths stay readable.
fn long_secret(tok: &str) -> bool {
    let core = tok.trim_matches(|c: char| "\"',;()[]{}<>".contains(c));
    core.len() >= 24
        && core.chars().all(|c| c.is_ascii_alphanumeric() || "+=_-.".contains(c))
        && core.chars().any(|c| c.is_ascii_digit())
        && core.chars().any(|c| c.is_ascii_alphabetic())
}

/// One line with its secrets replaced by "(hidden)".
pub fn redact(line: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut hide_next = false;
    for part in line.split(' ') {
        if part.is_empty() {
            out.push(String::new());
            continue;
        }
        if hide_next && (part == "=" || part == ":") {
            out.push(part.to_string());
            continue;
        }
        if hide_next {
            out.push("(hidden)".into());
            // "Authorization: Bearer abc": the scheme and the value both.
            hide_next = matches!(part.to_ascii_lowercase().as_str(), "bearer" | "basic");
            continue;
        }
        if let Some(pos) = part.find(['=', ':']) {
            let (k, v) = part.split_at(pos);
            if secret_word(k) {
                if v.len() > 1 {
                    out.push(format!("{k}{}(hidden)", &v[..1]));
                } else {
                    out.push(part.to_string());
                    hide_next = true;
                }
                continue;
            }
        } else if secret_word(part) && !part.eq_ignore_ascii_case("key") {
            // "AdminPass hunter2 to hunter3": the rest of the line could all be the secret.
            out.push(part.to_string());
            out.push("(hidden)".into());
            return out.join(" ");
        }
        if long_secret(part) {
            out.push("(hidden)".into());
            continue;
        }
        out.push(part.to_string());
    }
    out.join(" ")
}

/// What goes into the report.
pub struct ProblemReport {
    pub version: String,
    pub when: String,
    pub device: Vec<String>,
    pub screen: String,
    /// "name: value" lines (the interface's settings).
    pub settings: Vec<String>,
    /// One line per game: "Name (store:id): how Turbine plays it".
    pub games: Vec<String>,
    pub log: Vec<String>,
}

/// The report's text, every line redacted.
pub fn build(r: &ProblemReport) -> String {
    let mut t = String::new();
    t.push_str("Turbine Mod Manager problem report\n");
    t.push_str(&format!("Turbine: {}\nMade: {}\nScreen: {}\n", r.version, r.when, r.screen));
    for d in &r.device {
        t.push_str(&format!("{d}\n"));
    }
    t.push_str("\nSettings (no keys, tokens or cookies):\n");
    for s in &r.settings {
        t.push_str(&format!("  {s}\n"));
    }
    t.push_str(&format!("\nGames ({}):\n", r.games.len()));
    for g in &r.games {
        t.push_str(&format!("  {g}\n"));
    }
    t.push_str(&format!("\nRecent activity ({} lines, newest last):\n", r.log.len()));
    for l in &r.log {
        t.push_str(l);
        t.push('\n');
    }
    t.lines().map(redact).collect::<Vec<_>>().join("\n") + "\n"
}

/// The file name: Turbine-report-<stamp>.txt.
pub fn file_name(stamp: &str) -> String {
    format!("Turbine-report-{stamp}.txt")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_are_hidden() {
        assert_eq!(redact("net apikey=abcdef123"), "net apikey=(hidden)");
        assert_eq!(redact("\"nexusKey\": \"abc\""), "\"nexusKey\": (hidden)");
        assert_eq!(redact("Cookie: sid=1"), "Cookie: (hidden)");
        assert_eq!(redact("Authorization: Bearer abc"), "Authorization: (hidden) (hidden)");
        assert_eq!(redact("token aGVsbG8gd29ybGQgaGVsbG8gd29ybGQ1"), "token (hidden)");
        assert_eq!(redact("cooppassword = hunter2"), "cooppassword (hidden)");
        assert!(redact("sha 9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08").contains("(hidden)"));
        // Ordinary lines stay as they are, paths too.
        let l = "2026-11-09 03:58:28.334 launch  ELDEN RING through me3 0.13.0: /home/deck/.var/app/io.github.ravenor907.TurbineModManager/data/me3";
        assert_eq!(redact(l), l);
        assert_eq!(redact("Interface size: 100%"), "Interface size: 100%");
        assert_eq!(redact("Monkey: 3"), "Monkey: (hidden)", "on any doubt, hidden");
        assert_eq!(redact("AdminPass hunter2 to hunter3"), "AdminPass (hidden)");
        assert_eq!(redact("rcon_pass=abc"), "rcon_pass=(hidden)");
        assert_eq!(redact("Press key a"), "Press key a");
    }

    #[test]
    fn report_has_every_part() {
        let r = ProblemReport {
            version: "0.58".into(),
            when: "2026-11-09 10:00".into(),
            device: vec!["Device: ROG Xbox Ally X".into()],
            screen: "1920 x 1080, scale 2.10".into(),
            settings: vec!["Interface size: 100%".into(), "nexusKey: secretsecret".into()],
            games: vec!["ELDEN RING (steam:1245620): me3, offline with mods".into()],
            log: vec!["a line".into()],
        };
        let t = build(&r);
        for want in ["Turbine: 0.58", "Screen: 1920 x 1080", "Interface size: 100%", "ELDEN RING (steam:1245620)", "a line", "nexusKey: (hidden)"] {
            assert!(t.contains(want), "{want} in {t}");
        }
        assert!(!t.contains("secretsecret"));
        assert_eq!(file_name("2026-11-09_10-00-00"), "Turbine-report-2026-11-09_10-00-00.txt");
    }
}
