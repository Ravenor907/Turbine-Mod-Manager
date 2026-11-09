//! Nexus Mods single sign-on (SSO): the player approves Turbine on nexusmods.com and Nexus hands Turbine an API key
//! over a WebSocket. Only the standard library and the engine's JSON reader are used.
//!
//! It stays off until Nexus gives Turbine an application slug. Until then the browser helper's sign-in is the way in,
//! and method() says so. The slug is read, first found wins, from:
//! 1. the TURBINE_NEXUS_SSO_SLUG environment variable (for testing a slug before it ships),
//! 2. the file nexus-sso-slug in Turbine's config folder (~/.var/app/io.github.ravenor907.TurbineModManager/config/
//!    turbine-mod-manager/ inside the Flatpak),
//! 3. app/nexus-sso-slug.txt in the source, built in (one line; lines starting with # are notes).
//!
//! How SSO works (Nexus's protocol 2):
//! - Connect to wss://sso.nexusmods.com and send {"id":<new UUID>,"token":null,"protocol":2}.
//! - Nexus answers {"success":true,"data":{"connection_token":"..."},"error":null}. Keep the token: after a dropped
//!   connection, connecting again with the same id and this token picks the request up where it was.
//! - Open https://www.nexusmods.com/sso?id=<id>&application=<slug> in the player's browser. He signs in if needed and
//!   presses Authorise.
//! - Nexus sends {"success":true,"data":{"api_key":"..."},"error":null}. That key is the same kind of key the player
//!   could paste by hand: the caller hands it to the usual key check, which saves it.
//! - A ping every 30 seconds keeps the connection open while the player is on the page.
//!
//! TLS: the standard library has none, so the connection runs through the runtime's `openssl s_client` with fixed
//! arguments (certificate and host name checked, never a shell). Without openssl, method() falls back to the helper.
//! The key is never written to the log.
//!
//! Its tests run on their own, against a stand-in SSO server on 127.0.0.1:
//! rustc --test app/src/nexus_sso.rs --extern turbine_engine=<the engine's rlib>.

// The sign-in screen calls method() and sign_in(); until it does, parts of this file are used only by its tests.
#![allow(dead_code)]

use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use turbine_engine::json::{self, quote, Json};

/// Nexus's SSO server.
pub const SSO_HOST: &str = "sso.nexusmods.com";
pub const SSO_PORT: u16 = 443;
/// The page the player approves Turbine on.
pub const AUTHORISE_PAGE: &str = "https://www.nexusmods.com/sso";
/// Nexus's SSO protocol version (sends the connection token back so a dropped connection can resume).
pub const PROTOCOL: u64 = 2;
/// Nexus asks for a ping every 30 seconds.
pub const PING_EVERY: Duration = Duration::from_secs(30);
/// How long the player has to approve Turbine before the request is dropped.
pub const GIVE_UP_AFTER: Duration = Duration::from_secs(10 * 60);
/// Times a dropped connection is opened again before giving up.
pub const RECONNECTS: u32 = 5;
/// Longest message accepted from the server (its messages are a few hundred bytes).
pub const MAX_MESSAGE: usize = 64 * 1024;
/// The environment variable and file name that hold the slug.
pub const SLUG_ENV: &str = "TURBINE_NEXUS_SSO_SLUG";
pub const SLUG_FILE: &str = "nexus-sso-slug";
/// The slug built into this version (empty until Nexus gives one).
const BUILT_IN: &str = include_str!("../nexus-sso-slug.txt");

const OP_CONT: u8 = 0x0;
const OP_TEXT: u8 = 0x1;
const OP_BINARY: u8 = 0x2;
const OP_CLOSE: u8 = 0x8;
const OP_PING: u8 = 0x9;
const OP_PONG: u8 = 0xA;
const WS_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

// ---------------------------------------------------------------------------------------------------------------
// The slug and the sign-in method

/// How Turbine signs the player in to Nexus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Method {
    /// Nexus's own approval page; the key arrives by itself.
    Sso { slug: String },
    /// The browser helper's sign-in (the default until Nexus gives a slug).
    Helper,
}

/// A slug Nexus could have issued: 1 to 64 lowercase letters, digits, - or _.
pub fn slug_ok(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64 && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

/// The first line of a slug file that isn't empty or a note (#). None when there is none or it isn't a slug.
pub fn slug_in(text: &str) -> Option<String> {
    let line = text.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with('#'))?;
    slug_ok(line).then(|| line.to_string())
}

/// Picks the slug from its three places, first found wins (see the top of this file).
pub fn slug_from(env: Option<&str>, file: Option<&str>, built_in: &str) -> Option<String> {
    env.and_then(slug_in).or_else(|| file.and_then(slug_in)).or_else(|| slug_in(built_in))
}

/// Turbine's config folder (the Flatpak sets XDG_CONFIG_HOME to the app's own).
fn config_dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").filter(|v| !v.is_empty()).map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("turbine-mod-manager"))
}

/// The slug in effect, or None while Nexus hasn't given one.
pub fn slug() -> Option<String> {
    let env = std::env::var(SLUG_ENV).ok();
    let file = config_dir().and_then(|d| std::fs::read_to_string(d.join(SLUG_FILE)).ok());
    slug_from(env.as_deref(), file.as_deref(), BUILT_IN)
}

/// Whether the runtime has openssl to carry the secure connection.
pub fn tls_available() -> bool {
    Command::new("openssl")
        .arg("version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Picks the method from a slug and whether TLS is there. SSO needs both.
pub fn method_for(slug: Option<String>, tls: bool) -> Method {
    match slug {
        Some(slug) if tls => Method::Sso { slug },
        _ => Method::Helper,
    }
}

/// The sign-in method to use now: SSO once a slug is set (and openssl is there), otherwise the browser helper.
pub fn method() -> Method {
    match slug() {
        Some(s) => method_for(Some(s), tls_available()),
        None => Method::Helper,
    }
}

// ---------------------------------------------------------------------------------------------------------------
// The SSO messages

/// A random UUID (version 4), the request's id.
pub fn new_id() -> io::Result<String> {
    let mut b = [0u8; 16];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut b)?;
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    Ok(format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32]))
}

/// What Turbine sends after connecting: {"id":...,"token":null or "...","protocol":2}.
pub fn hello_message(id: &str, token: Option<&str>) -> String {
    let token = token.map(quote).unwrap_or_else(|| "null".into());
    format!("{{\"id\":{},\"token\":{token},\"protocol\":{PROTOCOL}}}", quote(id))
}

/// The page the player approves Turbine on.
pub fn authorise_url(id: &str, slug: &str) -> String {
    format!("{AUTHORISE_PAGE}?id={}&application={}", encode(id), encode(slug))
}

fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// One message from the SSO server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    /// The request is waiting; keep this token for reconnecting.
    Token(String),
    /// The player approved Turbine.
    ApiKey(String),
    /// Nexus said no (its own words).
    Failed(String),
    /// Anything else (ignored).
    Other,
}

pub fn parse_reply(text: &str) -> Reply {
    let Ok(j) = json::parse(text) else { return Reply::Other };
    if j.get("success").and_then(Json::as_bool) == Some(false) {
        let why = j.str_at("error").filter(|e| !e.trim().is_empty()).unwrap_or("Nexus didn't say why.");
        return Reply::Failed(why.chars().take(200).collect());
    }
    let data = j.get("data");
    if let Some(k) = data.and_then(|d| d.str_at("api_key")).filter(|k| key_ok(k)) {
        return Reply::ApiKey(k.to_string());
    }
    if let Some(t) = data.and_then(|d| d.str_at("connection_token")).filter(|t| !t.is_empty() && t.len() <= 256) {
        return Reply::Token(t.to_string());
    }
    Reply::Other
}

/// A key worth handing on: printable, no spaces, a sane length. Nexus's own check decides the rest.
fn key_ok(k: &str) -> bool {
    (16..=1024).contains(&k.len()) && k.bytes().all(|b| b.is_ascii_graphic())
}

// ---------------------------------------------------------------------------------------------------------------
// WebSocket client (RFC 6455)

/// SHA-1 (FIPS 180-4), only for the handshake's Sec-WebSocket-Accept check.
pub fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    for block in msg.chunks(64) {
        let mut w = [0u32; 80];
        for (i, c) in block.chunks(4).enumerate() {
            w[i] = u32::from_be_bytes([c[0], c[1], c[2], c[3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = h;
        for (i, wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | (!b & d), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };
            let t = a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(*wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = t;
        }
        for (x, y) in h.iter_mut().zip([a, b, c, d, e]) {
            *x = x.wrapping_add(y);
        }
    }
    let mut out = [0u8; 20];
    for (i, x) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&x.to_be_bytes());
    }
    out
}

pub fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= c.len() {
                s.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                s.push('=');
            }
        }
    }
    s
}

/// The Sec-WebSocket-Accept value the server must answer for `key`.
pub fn accept_for(key: &str) -> String {
    base64(&sha1(format!("{key}{WS_GUID}").as_bytes()))
}

/// The client's opening request.
pub fn handshake_request(host: &str, key: &str) -> String {
    format!(
        "GET / HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {key}\r\n\
         Sec-WebSocket-Version: 13\r\nUser-Agent: TurbineModManager\r\n\r\n"
    )
}

/// Checks the server's answer to the opening request (the head, up to the empty line).
pub fn check_handshake(head: &str, key: &str) -> Result<(), String> {
    let mut lines = head.split("\r\n");
    let status = lines.next().unwrap_or("");
    if status.split_whitespace().nth(1) != Some("101") {
        return Err(format!("the server answered \"{}\"", status.chars().take(60).collect::<String>()));
    }
    let mut upgrade = false;
    let mut accept = None;
    for l in lines {
        if let Some((name, value)) = l.split_once(':') {
            let (name, value) = (name.trim().to_ascii_lowercase(), value.trim());
            if name == "upgrade" && value.eq_ignore_ascii_case("websocket") {
                upgrade = true;
            }
            if name == "sec-websocket-accept" {
                accept = Some(value.to_string());
            }
        }
    }
    if !upgrade {
        return Err("the server didn't switch to WebSocket".into());
    }
    if accept.as_deref() != Some(accept_for(key).as_str()) {
        return Err("the server's WebSocket answer didn't match".into());
    }
    Ok(())
}

/// A client frame: always masked (RFC 6455 section 5.3).
pub fn client_frame(opcode: u8, payload: &[u8], mask: [u8; 4]) -> Vec<u8> {
    let mut f = vec![0x80 | opcode];
    match payload.len() {
        n if n < 126 => f.push(0x80 | n as u8),
        n if n <= 0xFFFF => {
            f.push(0x80 | 126);
            f.extend_from_slice(&(n as u16).to_be_bytes());
        }
        n => {
            f.push(0x80 | 127);
            f.extend_from_slice(&(n as u64).to_be_bytes());
        }
    }
    f.extend_from_slice(&mask);
    f.extend(payload.iter().enumerate().map(|(i, b)| b ^ mask[i % 4]));
    f
}

/// One frame from the server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub fin: bool,
    pub opcode: u8,
    pub payload: Vec<u8>,
}

/// Reads one server frame. Server frames are never masked; anything over `limit` is refused.
pub fn read_frame(r: &mut impl Read, limit: usize) -> io::Result<Frame> {
    let bad = |m: &str| io::Error::new(io::ErrorKind::InvalidData, m.to_string());
    let mut h = [0u8; 2];
    r.read_exact(&mut h)?;
    if h[0] & 0x70 != 0 {
        return Err(bad("reserved bits set"));
    }
    if h[1] & 0x80 != 0 {
        return Err(bad("masked frame from the server"));
    }
    let len = match h[1] & 0x7f {
        126 => {
            let mut b = [0u8; 2];
            r.read_exact(&mut b)?;
            u16::from_be_bytes(b) as u64
        }
        127 => {
            let mut b = [0u8; 8];
            r.read_exact(&mut b)?;
            u64::from_be_bytes(b)
        }
        n => n as u64,
    };
    if len > limit as u64 {
        return Err(bad("message too long"));
    }
    let mut payload = vec![0u8; len as usize];
    r.read_exact(&mut payload)?;
    Ok(Frame { fin: h[0] & 0x80 != 0, opcode: h[0] & 0x0f, payload })
}

fn random4() -> [u8; 4] {
    let mut b = [0u8; 4];
    if std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut b)).is_err() {
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(7);
        b = n.to_le_bytes();
    }
    b
}

// ---------------------------------------------------------------------------------------------------------------
// Connections

/// One open connection: a reader, a writer, and the openssl process carrying it (if any; stopped on drop).
pub struct Conn {
    pub reader: Box<dyn Read + Send>,
    pub writer: Box<dyn Write + Send>,
    pub child: Option<Child>,
    /// A TCP connection to shut on drop (so the reader thread ends too).
    pub socket: Option<TcpStream>,
}

impl Drop for Conn {
    fn drop(&mut self) {
        if let Some(s) = &self.socket {
            let _ = s.shutdown(std::net::Shutdown::Both);
        }
        if let Some(c) = &mut self.child {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

/// A plain TCP connection (the tests' stand-in server).
pub fn tcp_connect(addr: &str) -> io::Result<Conn> {
    let s = TcpStream::connect(addr)?;
    s.set_nodelay(true)?;
    Ok(Conn { reader: Box::new(s.try_clone()?), writer: Box::new(s.try_clone()?), child: None, socket: Some(s) })
}

/// The arguments for openssl's TLS client: the certificate chain and the host name are checked, and a bad one ends
/// the connection.
pub fn openssl_args(host: &str, port: u16) -> Vec<String> {
    [
        "s_client",
        "-quiet",
        "-connect",
        &format!("{host}:{port}"),
        "-servername",
        host,
        "-verify_hostname",
        host,
        "-verify_return_error",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// A TLS connection to Nexus's SSO server through the runtime's openssl.
pub fn tls_connect() -> io::Result<Conn> {
    let mut child = Command::new("openssl")
        .args(openssl_args(SSO_HOST, SSO_PORT))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let writer = child.stdin.take().ok_or_else(|| io::Error::other("no input to openssl"))?;
    let reader = child.stdout.take().ok_or_else(|| io::Error::other("no output from openssl"))?;
    Ok(Conn { reader: Box::new(reader), writer: Box::new(writer), child: Some(child), socket: None })
}

// ---------------------------------------------------------------------------------------------------------------
// The sign-in

/// Why sign-in stopped without a key. plain() is what the player reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SsoError {
    /// No slug yet: use the browser helper.
    NotSetUp,
    /// Couldn't reach Nexus.
    Connect(String),
    /// Nexus refused (its words).
    Refused(String),
    /// The connection dropped and couldn't be picked up again.
    Lost(String),
    /// The player didn't approve in time.
    TimedOut,
    /// The player stopped it.
    Cancelled,
    /// The approval page couldn't be opened.
    Browser(String),
}

impl SsoError {
    pub fn plain(&self) -> String {
        match self {
            SsoError::NotSetUp => "Signing in through Nexus isn't set up yet. Use the browser helper instead.".into(),
            SsoError::Connect(e) => format!("Turbine couldn't reach Nexus to sign in ({e}). Check the internet connection, then try again."),
            SsoError::Refused(e) => format!("Nexus didn't let Turbine sign in: {e}"),
            SsoError::Lost(e) => format!("The connection to Nexus dropped ({e}). Try again."),
            SsoError::TimedOut => "Nexus didn't hear back in 10 minutes, so the sign-in stopped. Try again when you're ready.".into(),
            SsoError::Cancelled => "Sign-in stopped.".into(),
            SsoError::Browser(e) => format!("Turbine couldn't open the Nexus page ({e})."),
        }
    }
}

/// What happens along the way, for the screen and the activity log. Never carries the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Progress {
    /// Open this page for the player (once). An error here stops the sign-in.
    OpenPage(String),
    /// Nexus is waiting for the player to press Authorise.
    Waiting,
    /// The connection dropped; trying again (attempt number).
    Reconnecting(u32),
}

/// The knobs, with Nexus's values in Options::nexus(); the tests shorten them.
pub struct Options {
    pub slug: String,
    pub host: String,
    pub ping_every: Duration,
    pub give_up_after: Duration,
    pub reconnects: u32,
    pub reconnect_wait: Duration,
}

impl Options {
    pub fn nexus(slug: &str) -> Options {
        Options {
            slug: slug.to_string(),
            host: SSO_HOST.to_string(),
            ping_every: PING_EVERY,
            give_up_after: GIVE_UP_AFTER,
            reconnects: RECONNECTS,
            reconnect_wait: Duration::from_secs(2),
        }
    }
}

/// Signs in through Nexus's SSO with the slug in effect. Blocks until the player approves Turbine, it fails, or
/// `cancel` is set; run it off the UI thread. `progress` gets OpenPage first (open it in the player's browser),
/// then Waiting. Ok is the player's API key: hand it to the usual key check (online's set_key), which saves it.
///
/// While method() is Helper this answers NotSetUp at once, so a caller can offer both and let this decide.
pub fn sign_in(progress: &mut dyn FnMut(Progress) -> Result<(), String>, cancel: &AtomicBool) -> Result<String, SsoError> {
    match method() {
        Method::Sso { slug } => run(&Options::nexus(&slug), &tls_connect, progress, cancel),
        Method::Helper => Err(SsoError::NotSetUp),
    }
}

enum Event {
    Text(String),
    Gone(String),
}

/// The sign-in itself, over any connection `connect` makes (TLS to Nexus, or the tests' stand-in).
pub fn run(
    opts: &Options,
    connect: &dyn Fn() -> io::Result<Conn>,
    progress: &mut dyn FnMut(Progress) -> Result<(), String>,
    cancel: &AtomicBool,
) -> Result<String, SsoError> {
    if !slug_ok(&opts.slug) {
        return Err(SsoError::NotSetUp);
    }
    let id = new_id().map_err(|e| SsoError::Connect(e.to_string()))?;
    let deadline = Instant::now() + opts.give_up_after;
    let mut token: Option<String> = None;
    let mut opened = false;
    let mut attempt = 0u32;
    loop {
        let why = match session(opts, connect, &id, &mut token, &mut opened, progress, cancel, deadline) {
            Ok(key) => return Ok(key),
            Err(Stop::Done(e)) => return Err(e),
            Err(Stop::Dropped(why)) => why,
        };
        // A connection that never got a token can't be picked up again.
        if token.is_none() {
            return Err(if attempt == 0 { SsoError::Connect(why) } else { SsoError::Lost(why) });
        }
        attempt += 1;
        if attempt > opts.reconnects {
            return Err(SsoError::Lost(why));
        }
        progress(Progress::Reconnecting(attempt)).map_err(SsoError::Browser)?;
        let until = Instant::now() + opts.reconnect_wait;
        while Instant::now() < until {
            if cancel.load(Ordering::SeqCst) {
                return Err(SsoError::Cancelled);
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

enum Stop {
    Done(SsoError),
    Dropped(String),
}

#[allow(clippy::too_many_arguments)]
fn session(
    opts: &Options,
    connect: &dyn Fn() -> io::Result<Conn>,
    id: &str,
    token: &mut Option<String>,
    opened: &mut bool,
    progress: &mut dyn FnMut(Progress) -> Result<(), String>,
    cancel: &AtomicBool,
    deadline: Instant,
) -> Result<String, Stop> {
    let dropped = |e: String| Stop::Dropped(e);
    let mut conn = connect().map_err(|e| dropped(e.to_string()))?;
    let key = base64(&{
        let mut k = [0u8; 16];
        for c in k.chunks_mut(4) {
            c.copy_from_slice(&random4());
        }
        k
    });
    conn.writer.write_all(handshake_request(&opts.host, &key).as_bytes()).map_err(|e| dropped(e.to_string()))?;
    conn.writer.flush().map_err(|e| dropped(e.to_string()))?;
    let mut reader = BufReader::new(std::mem::replace(&mut conn.reader, Box::new(io::empty())));
    let mut head = String::new();
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).map_err(|e| dropped(e.to_string()))?;
        if n == 0 {
            return Err(dropped("the server closed the connection".into()));
        }
        if head.len() + line.len() > 8 * 1024 {
            return Err(dropped("the server's answer was too long".into()));
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        head.push_str(&line);
    }
    // A server that answers but won't do WebSocket isn't going to change its mind: stop.
    check_handshake(&head, &key).map_err(|e| Stop::Done(SsoError::Connect(e)))?;

    let writer: Arc<Mutex<Box<dyn Write + Send>>> = Arc::new(Mutex::new(std::mem::replace(&mut conn.writer, Box::new(io::sink()))));
    let send = |w: &Arc<Mutex<Box<dyn Write + Send>>>, op: u8, payload: &[u8]| -> io::Result<()> {
        let mut w = w.lock().unwrap_or_else(|e| e.into_inner());
        w.write_all(&client_frame(op, payload, random4()))?;
        w.flush()
    };
    send(&writer, OP_TEXT, hello_message(id, token.as_deref()).as_bytes()).map_err(|e| dropped(e.to_string()))?;

    // The reader runs on its own thread (an openssl pipe can't time out a read) and answers pings itself.
    let (tx, rx) = mpsc::channel::<Event>();
    let w2 = Arc::clone(&writer);
    std::thread::spawn(move || {
        let mut text: Vec<u8> = Vec::new();
        let mut in_text = false;
        loop {
            let f = match read_frame(&mut reader, MAX_MESSAGE) {
                Ok(f) => f,
                Err(e) => {
                    let why = if e.kind() == io::ErrorKind::UnexpectedEof { "the server closed the connection".into() } else { e.to_string() };
                    let _ = tx.send(Event::Gone(why));
                    return;
                }
            };
            match f.opcode {
                OP_PING => {
                    let _ = send(&w2, OP_PONG, &f.payload);
                }
                OP_PONG => {}
                OP_CLOSE => {
                    let _ = send(&w2, OP_CLOSE, f.payload.get(..2).unwrap_or(&[]));
                    let _ = tx.send(Event::Gone("the server closed the connection".into()));
                    return;
                }
                OP_TEXT | OP_CONT if f.opcode == OP_TEXT || in_text => {
                    if f.opcode == OP_TEXT {
                        text.clear();
                    }
                    text.extend_from_slice(&f.payload);
                    in_text = !f.fin;
                    if text.len() > MAX_MESSAGE {
                        let _ = tx.send(Event::Gone("message too long".into()));
                        return;
                    }
                    if f.fin {
                        let t = String::from_utf8_lossy(&text).into_owned();
                        if tx.send(Event::Text(t)).is_err() {
                            return;
                        }
                    }
                }
                OP_BINARY | OP_CONT => {}
                _ => {
                    let _ = tx.send(Event::Gone("unknown frame from the server".into()));
                    return;
                }
            }
        }
    });

    let mut last_ping = Instant::now();
    let close = |w: &Arc<Mutex<Box<dyn Write + Send>>>| {
        let _ = send(w, OP_CLOSE, &1000u16.to_be_bytes());
    };
    loop {
        if cancel.load(Ordering::SeqCst) {
            close(&writer);
            return Err(Stop::Done(SsoError::Cancelled));
        }
        if Instant::now() >= deadline {
            close(&writer);
            return Err(Stop::Done(SsoError::TimedOut));
        }
        if last_ping.elapsed() >= opts.ping_every {
            send(&writer, OP_PING, b"").map_err(|e| dropped(e.to_string()))?;
            last_ping = Instant::now();
        }
        match rx.recv_timeout(Duration::from_millis(50)) {
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Err(dropped("the connection ended".into())),
            Ok(Event::Gone(why)) => return Err(dropped(why)),
            Ok(Event::Text(t)) => match parse_reply(&t) {
                Reply::Token(t) => {
                    *token = Some(t);
                    if !*opened {
                        *opened = true;
                        progress(Progress::OpenPage(authorise_url(id, &opts.slug))).map_err(|e| {
                            close(&writer);
                            Stop::Done(SsoError::Browser(e))
                        })?;
                    }
                    progress(Progress::Waiting).map_err(|e| Stop::Done(SsoError::Browser(e)))?;
                }
                Reply::ApiKey(k) => {
                    close(&writer);
                    return Ok(k);
                }
                Reply::Failed(why) => {
                    close(&writer);
                    return Err(Stop::Done(SsoError::Refused(why)));
                }
                Reply::Other => {}
            },
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::sync::atomic::AtomicUsize;

    const KEY: &str = "VX2PMnc4T5Q3dUFmjE45WyRyZ3VkKzIvYVJiMUdick5XQU9QWHdUbFo4";

    #[test]
    fn sha1_and_base64_match_the_standards() {
        let hex = |b: [u8; 20]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
        assert_eq!(hex(sha1(b"abc")), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(hex(sha1(b"")), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(hex(sha1(&[b'a'; 1000])), "291e9a6c66994949b57ba5e650361e98fc36b1ba");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        // RFC 6455 section 1.3's example.
        assert_eq!(accept_for("dGhlIHNhbXBsZSBub25jZQ=="), "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }

    #[test]
    fn the_slug_comes_from_the_first_place_that_has_one() {
        assert!(slug_ok("turbinemodmanager"));
        assert!(slug_ok("turbine-mod_manager2"));
        assert!(!slug_ok(""));
        assert!(!slug_ok("Turbine"));
        assert!(!slug_ok("turbine mod"));
        assert!(!slug_ok("a&application=evil"));
        assert!(!slug_ok(&"a".repeat(65)));
        let built_in = "# Nexus gives Turbine its slug. Put it on the line below.\n\n";
        assert_eq!(slug_from(None, None, built_in), None);
        assert_eq!(slug_from(None, None, "# note\nturbine\n"), Some("turbine".into()));
        assert_eq!(slug_from(None, Some("  mine \n"), "turbine"), Some("mine".into()));
        assert_eq!(slug_from(Some("test"), Some("mine"), "turbine"), Some("test".into()));
        // A broken value is skipped, not used.
        assert_eq!(slug_from(Some("Not A Slug"), None, "turbine"), Some("turbine".into()));
        assert_eq!(slug_from(Some(""), Some("# only a note"), ""), None);
        // The file in the source stays empty until Nexus gives a slug, so the helper stays the default.
        assert_eq!(slug_in(BUILT_IN), None, "app/nexus-sso-slug.txt has a slug: is that on purpose?");
    }

    #[test]
    fn the_helper_stays_the_default_without_a_slug() {
        assert_eq!(method_for(None, true), Method::Helper);
        assert_eq!(method_for(Some("turbine".into()), false), Method::Helper);
        assert_eq!(method_for(Some("turbine".into()), true), Method::Sso { slug: "turbine".into() });
        let stop = AtomicBool::new(false);
        let mut opts = Options::nexus("");
        opts.reconnects = 0;
        let r = run(&opts, &|| Err(io::Error::other("never called")), &mut |_| Ok(()), &stop);
        assert_eq!(r, Err(SsoError::NotSetUp));
    }

    #[test]
    fn messages_and_the_page() {
        let id = new_id().unwrap();
        assert_eq!(id.len(), 36);
        assert_eq!(&id[14..15], "4");
        assert!(matches!(&id[19..20], "8" | "9" | "a" | "b"));
        assert_ne!(id, new_id().unwrap());
        let hello = json::parse(&hello_message(&id, None)).unwrap();
        assert_eq!(hello.str_at("id"), Some(id.as_str()));
        assert!(matches!(hello.get("token"), Some(Json::Null)));
        assert!(matches!(hello.get("protocol"), Some(Json::Num(n)) if *n == 2.0));
        let again = json::parse(&hello_message(&id, Some("tok\"en"))).unwrap();
        assert_eq!(again.str_at("token"), Some("tok\"en"));
        assert_eq!(authorise_url("ab-12", "turbine"), "https://www.nexusmods.com/sso?id=ab-12&application=turbine");
        assert_eq!(authorise_url("a b", "t&x"), "https://www.nexusmods.com/sso?id=a%20b&application=t%26x");
    }

    #[test]
    fn replies_from_nexus() {
        let tok = r#"{"success":true,"data":{"connection_token":"X5SjO3P4i8tiBgMdYVTCh3z57ZnWVK5z"},"error":null}"#;
        assert_eq!(parse_reply(tok), Reply::Token("X5SjO3P4i8tiBgMdYVTCh3z57ZnWVK5z".into()));
        let key = format!(r#"{{"success":true,"data":{{"api_key":"{KEY}"}},"error":null}}"#);
        assert_eq!(parse_reply(&key), Reply::ApiKey(KEY.into()));
        assert_eq!(parse_reply(r#"{"success":false,"data":null,"error":"Invalid application"}"#), Reply::Failed("Invalid application".into()));
        assert_eq!(parse_reply(r#"{"success":false,"error":null}"#), Reply::Failed("Nexus didn't say why.".into()));
        assert_eq!(parse_reply(r#"{"success":true,"data":{"api_key":"short"},"error":null}"#), Reply::Other);
        assert_eq!(parse_reply(r#"{"success":true,"data":{"api_key":"has a space in it, so no thanks"}}"#), Reply::Other);
        assert_eq!(parse_reply("not json"), Reply::Other);
        assert_eq!(parse_reply("{}"), Reply::Other);
    }

    #[test]
    fn frames() {
        let f = client_frame(OP_TEXT, b"hello", [1, 2, 3, 4]);
        assert_eq!(f[0], 0x81);
        assert_eq!(f[1], 0x80 | 5);
        assert_eq!(&f[2..6], &[1, 2, 3, 4]);
        let unmasked: Vec<u8> = f[6..].iter().enumerate().map(|(i, b)| b ^ [1, 2, 3, 4][i % 4]).collect();
        assert_eq!(unmasked, b"hello");
        assert_eq!(client_frame(OP_TEXT, &[0; 300], [0; 4])[1], 0x80 | 126);
        assert_eq!(client_frame(OP_TEXT, &[0; 70000], [0; 4])[1], 0x80 | 127);
        let mut ok: &[u8] = &[0x81, 2, b'h', b'i'];
        assert_eq!(read_frame(&mut ok, 100).unwrap(), Frame { fin: true, opcode: OP_TEXT, payload: b"hi".to_vec() });
        let mut long = vec![0x81, 126, 0x01, 0x00];
        long.extend(vec![b'x'; 256]);
        assert_eq!(read_frame(&mut long.as_slice(), 1000).unwrap().payload.len(), 256);
        assert!(read_frame(&mut long.as_slice(), 100).is_err(), "too long");
        let mut masked: &[u8] = &[0x81, 0x82, 0, 0, 0, 0, b'h', b'i'];
        assert!(read_frame(&mut masked, 100).is_err(), "server frames are never masked");
        let mut rsv: &[u8] = &[0xC1, 0];
        assert!(read_frame(&mut rsv, 100).is_err());
        let mut short: &[u8] = &[0x81, 5, b'h'];
        assert!(read_frame(&mut short, 100).is_err());
    }

    #[test]
    fn the_handshake_answer_is_checked() {
        let key = "dGhlIHNhbXBsZSBub25jZQ==";
        let good = "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n";
        assert!(check_handshake(good, key).is_ok());
        assert!(check_handshake(&good.replace("s3pP", "AAAA"), key).is_err());
        assert!(check_handshake(&good.replace("101 Switching Protocols", "403 Forbidden"), key).is_err());
        assert!(check_handshake(&good.replace("Upgrade: websocket\r\n", ""), key).is_err());
        let req = handshake_request("sso.nexusmods.com", key);
        assert!(req.starts_with("GET / HTTP/1.1\r\nHost: sso.nexusmods.com\r\n"));
        assert!(req.contains("Sec-WebSocket-Version: 13\r\n") && req.ends_with("\r\n\r\n"));
    }

    #[test]
    fn openssl_checks_the_certificate_and_the_name() {
        let a = openssl_args(SSO_HOST, SSO_PORT);
        assert_eq!(a[0], "s_client");
        assert!(a.windows(2).any(|w| w == ["-connect", "sso.nexusmods.com:443"]));
        assert!(a.windows(2).any(|w| w == ["-verify_hostname", "sso.nexusmods.com"]));
        assert!(a.windows(2).any(|w| w == ["-servername", "sso.nexusmods.com"]));
        assert!(a.iter().any(|s| s == "-verify_return_error"));
    }

    // ---- A stand-in SSO server ------------------------------------------------------------------------------

    /// What the stand-in does on one connection.
    #[derive(Clone)]
    enum Script {
        /// Give a token, then the key once the page was opened.
        Approve,
        /// Give a token, then drop the connection without a close.
        TokenThenDrop,
        /// Expect the old id and token, then send the key at once.
        Resume,
        /// Answer with an error.
        Refuse,
        /// Answer the handshake with a wrong accept value.
        BadAccept,
        /// Give a token and then say nothing.
        Silent,
        /// Ping the client, and wait for its pong and its own ping before the key; the key comes in two parts.
        PingPong,
        /// Close right after the handshake, before any token.
        CloseAtOnce,
    }

    struct Seen {
        hellos: Vec<Json>,
        client_pings: usize,
        pongs: usize,
        closes: usize,
    }

    fn read_client_frame(s: &mut TcpStream) -> io::Result<(u8, Vec<u8>)> {
        let mut h = [0u8; 2];
        s.read_exact(&mut h)?;
        assert!(h[1] & 0x80 != 0, "client frames must be masked");
        let len = match h[1] & 0x7f {
            126 => {
                let mut b = [0u8; 2];
                s.read_exact(&mut b)?;
                u16::from_be_bytes(b) as usize
            }
            127 => {
                let mut b = [0u8; 8];
                s.read_exact(&mut b)?;
                u64::from_be_bytes(b) as usize
            }
            n => n as usize,
        };
        let mut mask = [0u8; 4];
        s.read_exact(&mut mask)?;
        let mut p = vec![0u8; len];
        s.read_exact(&mut p)?;
        for (i, b) in p.iter_mut().enumerate() {
            *b ^= mask[i % 4];
        }
        Ok((h[0] & 0x0f, p))
    }

    fn server_frame(fin: bool, op: u8, p: &[u8]) -> Vec<u8> {
        let mut f = vec![if fin { 0x80 } else { 0 } | op, p.len() as u8];
        f.extend_from_slice(p);
        f
    }

    fn token_msg(t: &str) -> String {
        format!(r#"{{"success":true,"data":{{"connection_token":"{t}"}},"error":null}}"#)
    }

    fn key_msg() -> String {
        format!(r#"{{"success":true,"data":{{"api_key":"{KEY}"}},"error":null}}"#)
    }

    /// Starts the stand-in on a free port. Each connection follows the next script; `opened` is set by the test when
    /// the page "opens", which is when the stand-in sends the key.
    fn stand_in(scripts: Vec<Script>, opened: Arc<AtomicBool>) -> (String, Arc<Mutex<Seen>>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap().to_string();
        let seen = Arc::new(Mutex::new(Seen { hellos: vec![], client_pings: 0, pongs: 0, closes: 0 }));
        let seen2 = Arc::clone(&seen);
        std::thread::spawn(move || {
            for script in scripts {
                let Ok((mut s, _)) = l.accept() else { return };
                s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
                // The handshake.
                let mut head = Vec::new();
                let mut b = [0u8; 1];
                while !head.ends_with(b"\r\n\r\n") {
                    if s.read_exact(&mut b).is_err() {
                        return;
                    }
                    head.push(b[0]);
                }
                let head = String::from_utf8_lossy(&head).into_owned();
                assert!(head.starts_with("GET / HTTP/1.1\r\n"));
                assert!(head.contains("Sec-WebSocket-Version: 13"));
                let key = head.lines().find_map(|l| l.strip_prefix("Sec-WebSocket-Key: ")).unwrap().trim().to_string();
                let accept = if matches!(script, Script::BadAccept) { "AAAAAAAAAAAAAAAAAAAAAAAAAAA=".into() } else { accept_for(&key) };
                write!(s, "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n").unwrap();
                if matches!(script, Script::BadAccept) {
                    continue;
                }
                if matches!(script, Script::CloseAtOnce) {
                    let _ = s.write_all(&server_frame(true, OP_CLOSE, &1001u16.to_be_bytes()));
                    let _ = read_client_frame(&mut s);
                    continue;
                }
                // The hello.
                let (op, p) = read_client_frame(&mut s).unwrap();
                assert_eq!(op, OP_TEXT);
                let hello = json::parse(&String::from_utf8(p).unwrap()).unwrap();
                seen2.lock().unwrap().hellos.push(hello);
                let send = |s: &mut TcpStream, text: &str| s.write_all(&server_frame(true, OP_TEXT, text.as_bytes()));
                match script {
                    Script::Refuse => {
                        send(&mut s, r#"{"success":false,"data":null,"error":"Invalid application slug"}"#).unwrap();
                    }
                    Script::TokenThenDrop => {
                        send(&mut s, &token_msg("tok-1")).unwrap();
                        while !opened.load(Ordering::SeqCst) {
                            std::thread::sleep(Duration::from_millis(5));
                        }
                        let _ = s.shutdown(std::net::Shutdown::Both);
                        continue;
                    }
                    Script::Resume => {
                        send(&mut s, &token_msg("tok-1")).unwrap();
                        send(&mut s, &key_msg()).unwrap();
                    }
                    Script::Approve | Script::Silent => {
                        send(&mut s, &token_msg("tok-1")).unwrap();
                        if matches!(script, Script::Approve) {
                            while !opened.load(Ordering::SeqCst) {
                                std::thread::sleep(Duration::from_millis(5));
                            }
                            send(&mut s, &key_msg()).unwrap();
                        }
                    }
                    Script::PingPong => {
                        send(&mut s, &token_msg("tok-1")).unwrap();
                        s.write_all(&server_frame(true, OP_PING, b"are you there")).unwrap();
                        let (mut got_pong, mut got_ping) = (false, false);
                        while !(got_pong && got_ping) {
                            let (op, p) = read_client_frame(&mut s).unwrap();
                            match op {
                                OP_PONG => {
                                    assert_eq!(p, b"are you there");
                                    got_pong = true;
                                    seen2.lock().unwrap().pongs += 1;
                                }
                                OP_PING => {
                                    got_ping = true;
                                    seen2.lock().unwrap().client_pings += 1;
                                    s.write_all(&server_frame(true, OP_PONG, &p)).unwrap();
                                }
                                other => panic!("unexpected frame {other}"),
                            }
                        }
                        // The key in two parts.
                        let k = key_msg();
                        let (a, b) = k.split_at(20);
                        s.write_all(&server_frame(false, OP_TEXT, a.as_bytes())).unwrap();
                        s.write_all(&server_frame(true, OP_CONT, b.as_bytes())).unwrap();
                    }
                    Script::BadAccept | Script::CloseAtOnce => unreachable!(),
                }
                // Wait for the client's close (or the end).
                while let Ok((op, _)) = read_client_frame(&mut s) {
                    if op == OP_CLOSE {
                        seen2.lock().unwrap().closes += 1;
                        break;
                    }
                }
            }
        });
        (addr, seen)
    }

    fn quick(slug: &str) -> Options {
        Options {
            slug: slug.into(),
            host: "127.0.0.1".into(),
            ping_every: Duration::from_secs(60),
            give_up_after: Duration::from_secs(10),
            reconnects: 2,
            reconnect_wait: Duration::from_millis(10),
        }
    }

    /// Runs a sign-in against the stand-in; returns the result and what the screen was told.
    fn sign_in_with(opts: &Options, scripts: Vec<Script>, cancel: &AtomicBool) -> (Result<String, SsoError>, Vec<Progress>, Arc<Mutex<Seen>>) {
        let opened = Arc::new(AtomicBool::new(false));
        let (addr, seen) = stand_in(scripts, Arc::clone(&opened));
        let mut told = vec![];
        let r = run(opts, &|| tcp_connect(&addr), &mut |p| {
            if matches!(p, Progress::OpenPage(_)) {
                opened.store(true, Ordering::SeqCst);
            }
            told.push(p);
            Ok(())
        }, cancel);
        (r, told, seen)
    }

    #[test]
    fn approve_on_nexus_gives_the_key() {
        let stop = AtomicBool::new(false);
        let (r, told, seen) = sign_in_with(&quick("turbine"), vec![Script::Approve], &stop);
        assert_eq!(r, Ok(KEY.to_string()));
        let seen = seen.lock().unwrap();
        let hello = &seen.hellos[0];
        let id = hello.str_at("id").unwrap().to_string();
        assert!(matches!(hello.get("token"), Some(Json::Null)));
        assert!(matches!(hello.get("protocol"), Some(Json::Num(n)) if *n == 2.0));
        assert_eq!(told[0], Progress::OpenPage(format!("https://www.nexusmods.com/sso?id={id}&application=turbine")));
        assert_eq!(told[1], Progress::Waiting);
        assert_eq!(told.iter().filter(|p| matches!(p, Progress::OpenPage(_))).count(), 1);
        // Nothing the screen is told carries the key.
        assert!(told.iter().all(|p| !format!("{p:?}").contains(KEY)));
        drop(seen);
        std::thread::sleep(Duration::from_millis(100));
    }

    #[test]
    fn a_dropped_connection_picks_up_with_the_same_request() {
        let stop = AtomicBool::new(false);
        let (r, told, seen) = sign_in_with(&quick("turbine"), vec![Script::TokenThenDrop, Script::Resume], &stop);
        assert_eq!(r, Ok(KEY.to_string()));
        let seen = seen.lock().unwrap();
        assert_eq!(seen.hellos.len(), 2);
        assert_eq!(seen.hellos[0].str_at("id"), seen.hellos[1].str_at("id"), "same request id");
        assert_eq!(seen.hellos[1].str_at("token"), Some("tok-1"), "the connection token goes back");
        assert!(told.contains(&Progress::Reconnecting(1)));
        // The page opens once, not again on the reconnect.
        assert_eq!(told.iter().filter(|p| matches!(p, Progress::OpenPage(_))).count(), 1);
    }

    #[test]
    fn nexus_saying_no_stops_with_its_words() {
        let stop = AtomicBool::new(false);
        let (r, told, _) = sign_in_with(&quick("turbine"), vec![Script::Refuse], &stop);
        assert_eq!(r, Err(SsoError::Refused("Invalid application slug".into())));
        assert!(told.is_empty(), "no page opens when Nexus refuses");
        assert!(r.unwrap_err().plain().contains("Invalid application slug"));
    }

    #[test]
    fn a_server_that_isnt_websocket_is_refused() {
        let stop = AtomicBool::new(false);
        let (r, _, _) = sign_in_with(&quick("turbine"), vec![Script::BadAccept], &stop);
        assert!(matches!(r, Err(SsoError::Connect(ref e)) if e.contains("didn't match")), "{r:?}");
    }

    #[test]
    fn closing_before_a_token_is_a_connect_failure() {
        let stop = AtomicBool::new(false);
        let (r, told, _) = sign_in_with(&quick("turbine"), vec![Script::CloseAtOnce], &stop);
        assert!(matches!(r, Err(SsoError::Connect(_))), "{r:?}");
        assert!(told.is_empty());
    }

    #[test]
    fn nothing_listening_is_a_connect_failure() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap().to_string();
        drop(l);
        let stop = AtomicBool::new(false);
        let r = run(&quick("turbine"), &|| tcp_connect(&addr), &mut |_| Ok(()), &stop);
        assert!(matches!(r, Err(SsoError::Connect(_))), "{r:?}");
    }

    #[test]
    fn pings_both_ways_and_a_key_in_two_parts() {
        let stop = AtomicBool::new(false);
        let mut opts = quick("turbine");
        opts.ping_every = Duration::from_millis(100);
        let (r, _, seen) = sign_in_with(&opts, vec![Script::PingPong], &stop);
        assert_eq!(r, Ok(KEY.to_string()));
        let seen = seen.lock().unwrap();
        assert!(seen.pongs >= 1 && seen.client_pings >= 1);
    }

    #[test]
    fn the_player_can_stop_it() {
        let stop = Arc::new(AtomicBool::new(false));
        let s2 = Arc::clone(&stop);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            s2.store(true, Ordering::SeqCst);
        });
        let started = Instant::now();
        let (r, told, seen) = sign_in_with(&quick("turbine"), vec![Script::Silent], &stop);
        assert_eq!(r, Err(SsoError::Cancelled));
        assert!(started.elapsed() < Duration::from_secs(3));
        assert!(matches!(told[0], Progress::OpenPage(_)));
        std::thread::sleep(Duration::from_millis(200));
        assert_eq!(seen.lock().unwrap().closes, 1, "a close goes to Nexus");
    }

    #[test]
    fn waiting_too_long_gives_up() {
        let stop = AtomicBool::new(false);
        let mut opts = quick("turbine");
        opts.give_up_after = Duration::from_millis(300);
        let (r, _, _) = sign_in_with(&opts, vec![Script::Silent], &stop);
        assert_eq!(r, Err(SsoError::TimedOut));
    }

    #[test]
    fn too_many_drops_give_up() {
        let stop = AtomicBool::new(false);
        let mut opts = quick("turbine");
        opts.reconnects = 1;
        let (r, told, _) =
            sign_in_with(&opts, vec![Script::TokenThenDrop, Script::TokenThenDrop, Script::TokenThenDrop], &stop);
        assert!(matches!(r, Err(SsoError::Lost(_))), "{r:?}");
        assert_eq!(told.iter().filter(|p| matches!(p, Progress::Reconnecting(_))).count(), 1);
    }

    #[test]
    fn a_page_that_wont_open_stops_the_sign_in() {
        let opened = Arc::new(AtomicBool::new(false));
        let (addr, _) = stand_in(vec![Script::Silent], opened);
        let stop = AtomicBool::new(false);
        let calls = AtomicUsize::new(0);
        let r = run(&quick("turbine"), &|| tcp_connect(&addr), &mut |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            Err("no browser".into())
        }, &stop);
        assert_eq!(r, Err(SsoError::Browser("no browser".into())));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn every_error_reads_plainly() {
        for e in [
            SsoError::NotSetUp,
            SsoError::Connect("x".into()),
            SsoError::Refused("x".into()),
            SsoError::Lost("x".into()),
            SsoError::TimedOut,
            SsoError::Cancelled,
            SsoError::Browser("x".into()),
        ] {
            let t = e.plain();
            assert!(t.ends_with('.') || t.ends_with('x'), "{t}");
            assert!(!t.contains('\u{2014}') && !t.contains('\u{2013}'));
        }
    }
}
