//! Turbine's end of the browser helper: a small WebSocket server on 127.0.0.1 (RFC 6455) that the helper extension
//! connects to, and the pairing page it reads its token from. Only the standard library is used.
//!
//! How it fits together (the protocol is in the team's bridge notes, v1):
//! - Turbine listens on the first free port of 47701 to 47710, on 127.0.0.1 only.
//! - GET /pair serves a tiny page. Turbine opens http://127.0.0.1:<port>/pair#<token> in the player's browser; the
//!   helper's content script reads the token from the part after # (which never reaches this server) and keeps it.
//! - GET /bridge with an Upgrade: websocket header becomes a WebSocket. The Origin header must be a browser
//!   extension's (chrome-extension://<id> or moz-extension://<id>) and the Host header Turbine's own address, so a web
//!   page can never talk to Turbine. The first message must be {"type":"hello","token":...} with the right token
//!   within 10 seconds, or the connection is closed.
//! - Every message is a JSON text frame. Client frames must be masked; binary frames, frames over 1 MB and broken
//!   frames close the connection. Pings get pongs; Turbine pings a quiet connection every 20 seconds.
//!
//! The SHA-1 and Base64 here exist only for the handshake's Sec-WebSocket-Accept value (RFC 6455 section 4.2.2);
//! SHA-1 follows FIPS 180-4. Neither is used for anything secret: the token is what keeps the bridge private.
//!
//! This file needs nothing from the rest of the app (only the engine's JSON reader), so its tests also run on their
//! own: rustc --test app/src/bridge.rs --extern turbine_engine=<the engine's rlib>.

use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};
use turbine_engine::json::{self, quote, Json};

/// The ports Turbine tries, in order.
pub const FIRST_PORT: u16 = 47701;
pub const LAST_PORT: u16 = 47710;
/// The protocol version in hello.
pub const PROTOCOL: u64 = 1;
/// Longest HTTP request head accepted.
pub const MAX_REQUEST: usize = 8 * 1024;
/// Longest message accepted from the extension (they are all small; this leaves plenty of room).
pub const MAX_MESSAGE: usize = 1 << 20;
/// Most connections at once (one per browser profile is the usual case).
pub const MAX_CLIENTS: usize = 8;
/// The fixed string RFC 6455 appends to the client's key.
const WS_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
const HELLO_WAIT: Duration = Duration::from_secs(10);
const PING_EVERY: Duration = Duration::from_secs(20);

// ---------------------------------------------------------------------------------------------------- SHA-1

/// SHA-1 of `data` (FIPS 180-4, section 6.1).
pub fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [0x6745_2301, 0xEFCD_AB89, 0x98BA_DCFE, 0x1032_5476, 0xC3D2_E1F0];
    let mut msg = data.to_vec();
    let bits = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bits.to_be_bytes());
    for block in msg.chunks(64) {
        let mut w = [0u32; 80];
        for (i, word) in block.chunks(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
        for (i, wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | (!b & d), 0x5A82_7999),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
                _ => (b ^ c ^ d, 0xCA62_C1D6),
            };
            let t = a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(*wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = t;
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }
    let mut out = [0u8; 20];
    for (i, v) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_be_bytes());
    }
    out
}

// ---------------------------------------------------------------------------------------------------- Base64

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard Base64 with padding (RFC 4648 section 4).
pub fn base64(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { B64[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { B64[n as usize & 63] as char } else { '=' });
    }
    out
}

/// Is this the Base64 of exactly 16 bytes (what Sec-WebSocket-Key must be)?
fn is_key(key: &str) -> bool {
    key.len() == 24 && key.ends_with("==") && key[..22].bytes().all(|b| B64.contains(&b))
}

/// The Sec-WebSocket-Accept value for a client's Sec-WebSocket-Key.
pub fn accept_key(key: &str) -> String {
    base64(&sha1(format!("{}{WS_GUID}", key.trim()).as_bytes()))
}

// ---------------------------------------------------------------------------------------------------- HTTP

/// The head of an HTTP request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
}

impl Request {
    /// A header's value (names ignore case); the first one when it's there more than once.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }

    fn header_has(&self, name: &str, token: &str) -> bool {
        self.header(name).is_some_and(|v| v.split(',').any(|t| t.trim().eq_ignore_ascii_case(token)))
    }
}

/// Reads a request head (everything before the blank line).
pub fn parse_request(head: &[u8]) -> Result<Request, String> {
    if head.len() > MAX_REQUEST {
        return Err("request too long".into());
    }
    let text = std::str::from_utf8(head).map_err(|_| "request isn't text".to_string())?;
    let mut lines = text.split("\r\n");
    let first = lines.next().unwrap_or("");
    let mut parts = first.split(' ');
    let (method, path, version) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    if method.is_empty() || !path.starts_with('/') || !version.starts_with("HTTP/1.") || parts.next().is_some() {
        return Err("bad request line".into());
    }
    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let (k, v) = line.split_once(':').ok_or("bad header")?;
        if k.is_empty() || k.contains(' ') {
            return Err("bad header".into());
        }
        headers.push((k.to_string(), v.trim().to_string()));
        if headers.len() > 64 {
            return Err("too many headers".into());
        }
    }
    Ok(Request { method: method.into(), path: path.into(), headers })
}

/// The extension ID in an Origin header (chrome-extension://<id> or moz-extension://<id>); None for anything else,
/// web pages included.
pub fn extension_id(origin: &str) -> Option<String> {
    let id = origin.strip_prefix("chrome-extension://").or_else(|| origin.strip_prefix("moz-extension://"))?;
    let ok = !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    ok.then(|| id.to_string())
}

/// Is the Host header Turbine's own address (so a page on some other name that points at 127.0.0.1 is refused)?
pub fn host_ok(host: Option<&str>, port: u16) -> bool {
    let Some(h) = host else { return false };
    let h = h.trim().to_ascii_lowercase();
    h == format!("127.0.0.1:{port}") || h == format!("localhost:{port}")
}

/// Compares two tokens in the same time whatever they hold.
pub fn same_token(given: &str, want: &str) -> bool {
    let (a, b) = (given.as_bytes(), want.as_bytes());
    if a.len() != b.len() || b.is_empty() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// What to do with a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// Become a WebSocket: the Sec-WebSocket-Accept value and the extension's ID.
    Upgrade { accept: String, ext: String },
    /// Serve the pairing page.
    Pair,
    /// Refuse with this HTTP status and a short reason.
    Refuse(u16, &'static str),
}

/// Decides what to do with a request to Turbine's port.
pub fn answer(req: &Request, port: u16) -> Answer {
    if req.method != "GET" {
        return Answer::Refuse(405, "Method Not Allowed");
    }
    if !host_ok(req.header("Host"), port) {
        return Answer::Refuse(403, "Forbidden");
    }
    let path = req.path.split(['?', '#']).next().unwrap_or("");
    match path {
        "/pair" => Answer::Pair,
        "/bridge" => {
            let upgrade = req.header_has("Upgrade", "websocket") && req.header_has("Connection", "upgrade");
            if !upgrade || req.header("Sec-WebSocket-Version") != Some("13") {
                return Answer::Refuse(426, "Upgrade Required");
            }
            let Some(key) = req.header("Sec-WebSocket-Key").filter(|k| is_key(k)) else {
                return Answer::Refuse(400, "Bad Request");
            };
            match req.header("Origin").and_then(extension_id) {
                Some(ext) => Answer::Upgrade { accept: accept_key(key), ext },
                None => Answer::Refuse(403, "Forbidden"),
            }
        }
        _ => Answer::Refuse(404, "Not Found"),
    }
}

/// The 101 answer that turns the connection into a WebSocket.
pub fn upgrade_response(accept: &str) -> String {
    format!("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n")
}

fn plain_response(status: u16, reason: &str) -> String {
    format!("HTTP/1.1 {status} {reason}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reason}", reason.len())
}

fn page_response(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\n\
         Referrer-Policy: no-referrer\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'none'; style-src 'unsafe-inline'\r\n\
         Connection: close\r\n\r\n{body}",
        body.len()
    )
}

/// The pairing page. It has no script of its own: the helper's content script reads the token after # in the
/// address, keeps it, and writes "Paired with Turbine" into #turbine-pair-status.
pub fn pair_page() -> String {
    r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Pair with Turbine</title>
<style>
body{margin:0;min-height:100vh;display:flex;align-items:center;justify-content:center;background:#0B0A08;color:#EADFC6;font:17px/1.5 sans-serif}
main{max-width:34em;padding:28px 32px;background:#141109;border:1px solid #33291B;border-top:3px solid #C99E52;border-radius:10px}
h1{margin:0 0 12px;font:600 22px/1.3 serif;letter-spacing:.08em;text-transform:uppercase;color:#D9B672}
p{margin:8px 0;color:#CDBE9F}
#turbine-pair-status{font-weight:600;color:#EADFC6}
#turbine-pair-status[data-state="paired"]{color:#9DBF7A}
#turbine-pair-status[data-state="error"]{color:#F0907A}
html[data-turbine-paired] #turbine-pair-status,html[data-turbine-paired] #turbine-pair-help{display:none}
#turbine-pair-yes,#turbine-pair-no{display:none;font-weight:600}
html[data-turbine-paired="yes"] #turbine-pair-yes{display:block;color:#9DBF7A}
html[data-turbine-paired="no"] #turbine-pair-no{display:block;color:#F0907A}
</style></head>
<body><main id="turbine-pair" data-turbine-pair="1" data-protocol="1">
<h1>Turbine Mod Manager</h1>
<p id="turbine-pair-status" data-state="waiting">Waiting for the browser helper...</p>
<p id="turbine-pair-yes">Paired with Turbine. You can close this tab.</p>
<p id="turbine-pair-no">Pairing didn't work. In Turbine, press Pair again.</p>
<p id="turbine-pair-help">If this still says Waiting after a few seconds, the helper isn't loaded in this browser yet. In Turbine, open Settings, Accounts, Set up the browser helper, follow the steps, then press Pair again.</p>
</main></body></html>
"#
    .to_string()
}

// ---------------------------------------------------------------------------------------------------- frames

pub const OP_CONTINUATION: u8 = 0x0;
pub const OP_TEXT: u8 = 0x1;
pub const OP_BINARY: u8 = 0x2;
pub const OP_CLOSE: u8 = 0x8;
pub const OP_PING: u8 = 0x9;
pub const OP_PONG: u8 = 0xA;

/// One frame as read from the wire, unmasked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub fin: bool,
    pub opcode: u8,
    pub payload: Vec<u8>,
}

/// Why a frame couldn't be read. Each one but Eof and Io means the client broke the protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameError {
    /// The connection ended.
    Eof,
    /// Reading stopped for another reason (a timeout shows as Io with "timed out").
    Io(io::ErrorKind),
    /// Clients must mask every frame (RFC 6455 section 5.1).
    Unmasked,
    /// Longer than the limit.
    TooBig,
    /// Reserved bits or an unknown opcode.
    Reserved,
    /// A control frame that is fragmented or over 125 bytes.
    BadControl,
}

impl FrameError {
    /// The close code to send for it (RFC 6455 section 7.4.1).
    pub fn close_code(&self) -> u16 {
        match self {
            FrameError::TooBig => 1009,
            _ => 1002,
        }
    }
}

fn read_exact(r: &mut impl Read, buf: &mut [u8]) -> Result<(), FrameError> {
    r.read_exact(buf).map_err(|e| if e.kind() == io::ErrorKind::UnexpectedEof { FrameError::Eof } else { FrameError::Io(e.kind()) })
}

/// Reads one client frame. `limit` is the longest payload accepted.
pub fn read_frame(r: &mut impl Read, limit: usize) -> Result<Frame, FrameError> {
    let mut head = [0u8; 2];
    read_exact(r, &mut head)?;
    let fin = head[0] & 0x80 != 0;
    if head[0] & 0x70 != 0 {
        return Err(FrameError::Reserved);
    }
    let opcode = head[0] & 0x0F;
    if !matches!(opcode, OP_CONTINUATION | OP_TEXT | OP_BINARY | OP_CLOSE | OP_PING | OP_PONG) {
        return Err(FrameError::Reserved);
    }
    if head[1] & 0x80 == 0 {
        return Err(FrameError::Unmasked);
    }
    let len = match head[1] & 0x7F {
        126 => {
            let mut b = [0u8; 2];
            read_exact(r, &mut b)?;
            u64::from(u16::from_be_bytes(b))
        }
        127 => {
            let mut b = [0u8; 8];
            read_exact(r, &mut b)?;
            u64::from_be_bytes(b)
        }
        n => u64::from(n),
    };
    let control = opcode >= 0x8;
    if control && (!fin || len > 125) {
        return Err(FrameError::BadControl);
    }
    if len > limit as u64 {
        return Err(FrameError::TooBig);
    }
    let mut mask = [0u8; 4];
    read_exact(r, &mut mask)?;
    let mut payload = vec![0u8; len as usize];
    read_exact(r, &mut payload)?;
    for (i, b) in payload.iter_mut().enumerate() {
        *b ^= mask[i % 4];
    }
    Ok(Frame { fin, opcode, payload })
}

/// A server frame (servers never mask, RFC 6455 section 5.1).
pub fn encode_frame(opcode: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() + 10);
    out.push(0x80 | (opcode & 0x0F));
    match payload.len() {
        n if n < 126 => out.push(n as u8),
        n if n <= 0xFFFF => {
            out.push(126);
            out.extend_from_slice(&(n as u16).to_be_bytes());
        }
        n => {
            out.push(127);
            out.extend_from_slice(&(n as u64).to_be_bytes());
        }
    }
    out.extend_from_slice(payload);
    out
}

/// A close frame's payload: the code, then a short reason.
pub fn close_payload(code: u16, reason: &str) -> Vec<u8> {
    let mut p = code.to_be_bytes().to_vec();
    p.extend(reason.bytes().take(120));
    p
}

/// What a frame (or a finished run of fragments) means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Part of a fragmented message; nothing to do yet.
    More,
    Text(String),
    Ping(Vec<u8>),
    Pong,
    /// The client is closing, with its code if it gave one.
    Close(Option<u16>),
}

/// Puts fragmented text messages back together and checks the frame order (RFC 6455 section 5.4).
#[derive(Debug, Default)]
pub struct Assembler {
    buf: Vec<u8>,
    open: bool,
}

impl Assembler {
    /// Takes one frame. Err(close code, reason) means the connection must be closed with that code.
    pub fn push(&mut self, f: Frame, limit: usize) -> Result<Event, (u16, &'static str)> {
        match f.opcode {
            OP_PING => Ok(Event::Ping(f.payload)),
            OP_PONG => Ok(Event::Pong),
            OP_CLOSE => {
                if f.payload.len() == 1 {
                    return Err((1002, "bad close frame"));
                }
                let code = (f.payload.len() >= 2).then(|| u16::from_be_bytes([f.payload[0], f.payload[1]]));
                Ok(Event::Close(code))
            }
            OP_BINARY => Err((1003, "text messages only")),
            OP_TEXT | OP_CONTINUATION => {
                if (f.opcode == OP_TEXT) == self.open {
                    return Err((1002, "fragments out of order"));
                }
                if self.buf.len() + f.payload.len() > limit {
                    return Err((1009, "message too big"));
                }
                self.buf.extend_from_slice(&f.payload);
                self.open = !f.fin;
                if self.open {
                    return Ok(Event::More);
                }
                let bytes = std::mem::take(&mut self.buf);
                String::from_utf8(bytes).map(Event::Text).map_err(|_| (1007, "not UTF-8"))
            }
            _ => Err((1002, "unknown frame")),
        }
    }
}

// ---------------------------------------------------------------------------------------------------- Nexus addresses

/// A Nexus mod (and file, when known) a download came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NexusRef {
    pub domain: String,
    pub mod_id: u64,
    pub file_id: Option<u64>,
}

fn query_value<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    query.split(['&', '#']).find_map(|kv| kv.split_once('=').filter(|(k, _)| *k == key).map(|(_, v)| v))
}

fn digits(s: &str) -> Option<u64> {
    (!s.is_empty() && s.len() <= 12 && s.bytes().all(|b| b.is_ascii_digit())).then(|| s.parse().ok()).flatten().filter(|n| *n > 0)
}

/// Reads a Nexus mod page address: https://www.nexusmods.com/<domain>/mods/<id>[?tab=files&file_id=<file>] (also
/// with /games/ before the domain, as newer links have).
pub fn nexus_page(url: &str) -> Option<NexusRef> {
    let rest = url.strip_prefix("https://")?;
    let (host, path) = rest.split_once('/')?;
    let host = host.to_ascii_lowercase();
    if host != "www.nexusmods.com" && host != "nexusmods.com" {
        return None;
    }
    let (path, query) = path.split_once('?').unwrap_or((path, ""));
    let path = path.split('#').next().unwrap_or("");
    let mut seg: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if seg.first() == Some(&"games") {
        seg.remove(0);
    }
    let (domain, mods, id) = (*seg.first()?, *seg.get(1)?, *seg.get(2)?);
    let domain_ok = !domain.is_empty() && domain.len() <= 64 && domain.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit());
    if !domain_ok || mods != "mods" {
        return None;
    }
    let mod_id = digits(id)?;
    let file_id = query_value(query, "file_id").and_then(digits);
    Some(NexusRef { domain: domain.into(), mod_id, file_id })
}

/// Where a download came from on Nexus: the page it was started from (the referrer) first, then the download's own
/// address when that is a Nexus page. Nexus's file servers (*.nexus-cdn.com) name the game by number, so their
/// addresses alone don't say which game a file is for.
pub fn nexus_ref(url: &str, referrer: &str) -> Option<NexusRef> {
    nexus_page(referrer).or_else(|| nexus_page(url))
}

/// Is this address on Nexus's own site or its file servers (what the extension reports downloads for)?
pub fn is_nexus_host(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else { return false };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("").to_ascii_lowercase();
    let host = host.rsplit_once('@').map(|(_, h)| h.to_string()).unwrap_or(host);
    let host = host.split(':').next().unwrap_or("");
    host == "nexusmods.com" || host.ends_with(".nexusmods.com") || host == "nexus-cdn.com" || host.ends_with(".nexus-cdn.com")
}

// ---------------------------------------------------------------------------------------------------- the server

/// Called with each message from a paired extension: its "type" and the whole message. Also called with "connected"
/// and "disconnected" (the message then is {"ext": id}).
pub type Handler = Arc<dyn Fn(&str, &Json) + Send + Sync>;

struct Client {
    id: u64,
    out: Arc<Mutex<TcpStream>>,
}

struct State {
    port: u16,
    token: String,
    dir: PathBuf,
    /// The extension that last paired (its ID), if any.
    paired: Option<String>,
    clients: Vec<Client>,
    /// The welcome message's games list, as JSON.
    games: String,
    error: String,
    log: fn(&str),
    /// The port isn't the one the helper was paired on: it needs the pairing page again.
    moved: bool,
    /// A Nexus page waiting for the helper to connect (Game Mode starts the browser through Steam first), and when
    /// it was asked for.
    pending_open: Option<(String, Instant)>,
    /// The helper version the last connected helper said in hello ("" when none has connected yet). Kept on disk,
    /// so Turbine knows an old helper before it connects.
    helper_version: String,
    /// The helper version Turbine already asked to reload this run (asked once, so an old copy elsewhere can't loop).
    reload_asked: Option<String>,
}

fn state() -> MutexGuard<'static, Option<State>> {
    static S: OnceLock<Mutex<Option<State>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(None)).lock().unwrap_or_else(|e| e.into_inner())
}

fn log(text: &str) {
    let f = state().as_ref().map(|s| s.log);
    if let Some(f) = f {
        f(text);
    }
}

const TOKEN_FILE: &str = "bridge-token";
const PAIRED_FILE: &str = "bridge-paired";
/// The port used last time: tried first, so a paired helper finds Turbine where it was.
const PORT_FILE: &str = "bridge-port";
/// The version the last connected helper reported.
const HELPER_VERSION_FILE: &str = "bridge-helper-version";

/// The first helper version that knows "open" (opens a Nexus page in a new tab).
pub const OPEN_SINCE: &str = "0.53";
/// The first helper version that knows "reload" (reloads itself from its folder with chrome.runtime.reload()).
pub const RELOAD_SINCE: &str = "0.54";
/// What the player does once when the helper is too old to reload itself.
pub const OLD_HELPER_TEXT: &str =
    "Your browser helper is out of date. In Chrome, open chrome://extensions and press the reload arrow on Turbine browser helper.";

/// The helper version this Turbine carries (from its manifest), set once at start; "" when it has none.
pub static BUNDLED: OnceLock<String> = OnceLock::new();
/// Run before Turbine asks a helper to reload: puts Turbine's newer helper files in the folder it loads from. False
/// when that didn't fully work (the reload is then not asked for: it would load a half-updated helper, or the old one).
pub static BEFORE_RELOAD: OnceLock<fn() -> bool> = OnceLock::new();

/// Compares two dotted versions ("0.53", "0.41", "1.2"): numbers part by part, missing parts are 0. Anything that
/// isn't a number counts as 0, so a broken version is always "older".
pub fn version_older(a: &str, b: &str) -> bool {
    let parts = |v: &str| -> Vec<u64> { v.trim().split('.').map(|p| p.trim().parse().unwrap_or(0)).collect() };
    let (x, y) = (parts(a), parts(b));
    for i in 0..x.len().max(y.len()) {
        let (p, q) = (x.get(i).copied().unwrap_or(0), y.get(i).copied().unwrap_or(0));
        if p != q {
            return p < q;
        }
    }
    false
}

/// What Turbine does about a helper of version `helper` when it carries `bundled`.
#[derive(Debug, PartialEq, Eq)]
pub enum HelperAge {
    /// As new as Turbine's (or Turbine carries none, or the version isn't known).
    Current,
    /// Older, and it knows "reload": Turbine asks it to reload itself from its folder.
    Reload,
    /// Older, and too old to reload itself: the player presses reload in chrome://extensions once.
    Manual,
}

pub fn helper_age(helper: &str, bundled: &str) -> HelperAge {
    if helper.is_empty() || bundled.is_empty() || !version_older(helper, bundled) {
        HelperAge::Current
    } else if version_older(helper, RELOAD_SINCE) {
        HelperAge::Manual
    } else {
        HelperAge::Reload
    }
}

/// {"type":"reload"}: the helper reloads itself from its folder (an unpacked extension picks up new files).
pub fn reload_message() -> String {
    "{\"type\":\"reload\"}".to_string()
}

/// 32 random bytes as hex, from the system's random source.
pub fn new_token() -> io::Result<String> {
    let mut b = [0u8; 32];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut b)?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

fn plausible_token(t: &str) -> bool {
    t.len() == 64 && t.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Writes a small file only the player can read.
fn write_private(path: &Path, text: &str) -> io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&tmp)?;
    f.write_all(text.as_bytes())?;
    f.sync_all()?;
    std::fs::rename(&tmp, path)
}

fn load_or_make_token(dir: &Path) -> Result<String, String> {
    let file = dir.join(TOKEN_FILE);
    if let Ok(t) = std::fs::read_to_string(&file) {
        let t = t.trim();
        if plausible_token(t) {
            return Ok(t.to_string());
        }
    }
    let t = new_token().map_err(|e| format!("no random numbers ({e})"))?;
    write_private(&file, &t).map_err(|e| format!("couldn't save the pairing token ({e})"))?;
    Ok(t)
}

/// Starts the server on the first free port. Returns the port. `log` writes a line to the activity log.
pub fn start(dir: &Path, log_line: fn(&str), handler: Handler) -> Result<u16, String> {
    let token = load_or_make_token(dir)?;
    let paired = std::fs::read_to_string(dir.join(PAIRED_FILE)).ok().and_then(|s| extension_id(&format!("chrome-extension://{}", s.trim())));
    let helper_version = std::fs::read_to_string(dir.join(HELPER_VERSION_FILE)).map(|v| v.trim().chars().take(20).collect::<String>()).unwrap_or_default();
    let last: Option<u16> = std::fs::read_to_string(dir.join(PORT_FILE)).ok().and_then(|s| s.trim().parse().ok()).filter(|p| (FIRST_PORT..=LAST_PORT).contains(p));
    let mut listener = None;
    for port in last.into_iter().chain(FIRST_PORT..=LAST_PORT) {
        if let Ok(l) = TcpListener::bind(("127.0.0.1", port)) {
            listener = Some((l, port));
            break;
        }
    }
    let Some((listener, port)) = listener else {
        let e = format!("ports {FIRST_PORT} to {LAST_PORT} are all in use");
        *state() = Some(State { port: 0, token, dir: dir.to_path_buf(), paired, clients: Vec::new(), games: "[]".into(), error: e.clone(), log: log_line, moved: false, pending_open: None, helper_version, reload_asked: None });
        return Err(e);
    };
    // A paired helper looks for Turbine on the old port; when it had to change, the pairing page opens again.
    let moved = paired.is_some() && last.is_some_and(|l| l != port);
    if last != Some(port) {
        let _ = write_private(&dir.join(PORT_FILE), &port.to_string());
    }
    *state() = Some(State { port, token, dir: dir.to_path_buf(), paired, clients: Vec::new(), games: "[]".into(), error: String::new(), log: log_line, moved, pending_open: None, helper_version, reload_asked: None });
    std::thread::Builder::new()
        .name("turbine-bridge".into())
        .spawn(move || {
            for conn in listener.incoming() {
                let Ok(stream) = conn else { continue };
                let h = handler.clone();
                let _ = std::thread::Builder::new().name("turbine-bridge-conn".into()).spawn(move || serve(stream, port, h));
            }
        })
        .map_err(|e| format!("couldn't start ({e})"))?;
    Ok(port)
}

/// The pairing page's address with the token after #, or None when the server isn't running.
pub fn pair_url() -> Option<String> {
    let s = state();
    let s = s.as_ref()?;
    (s.port != 0).then(|| format!("http://127.0.0.1:{}/pair#{}", s.port, s.token))
}

/// "Pair again": a new token; every connection is closed and the old pairing is forgotten.
pub fn pair_again() -> Result<(), String> {
    let mut guard = state();
    let s = guard.as_mut().ok_or("The browser helper link isn't running.")?;
    let t = new_token().map_err(|e| format!("No random numbers ({e})."))?;
    write_private(&s.dir.join(TOKEN_FILE), &t).map_err(|e| format!("The new pairing couldn't be saved ({e})."))?;
    s.token = t;
    s.paired = None;
    s.moved = false;
    let _ = std::fs::remove_file(s.dir.join(PAIRED_FILE));
    for c in s.clients.drain(..) {
        if let Ok(o) = c.out.lock() {
            let _ = o.shutdown(Shutdown::Both);
        }
    }
    Ok(())
}

/// The games list sent in welcome: [{"id","name","domain"}], as JSON.
pub fn set_games(json: String) {
    if let Some(s) = state().as_mut() {
        s.games = json;
    }
}

/// {"running","port","paired","connected","ext","error","moved","helper","bundled","outdated","manual"} for Settings.
/// "helper": the version the last connected helper reported; "outdated": it is older than the one Turbine carries;
/// "manual": too old to reload itself (the player presses reload in chrome://extensions once).
pub fn status_json() -> String {
    let bundled = BUNDLED.get().map(String::as_str).unwrap_or("");
    let g = state();
    match g.as_ref() {
        None => format!(
            "{{\"running\":false,\"port\":0,\"paired\":false,\"connected\":0,\"ext\":\"\",\"error\":\"\",\"moved\":false,\"helper\":\"\",\"bundled\":{},\"outdated\":false,\"manual\":false}}",
            quote(bundled)
        ),
        Some(s) => {
            let age = helper_age(&s.helper_version, bundled);
            format!(
                "{{\"running\":{},\"port\":{},\"paired\":{},\"connected\":{},\"ext\":{},\"error\":{},\"moved\":{},\"helper\":{},\"bundled\":{},\"outdated\":{},\"manual\":{}}}",
                s.port != 0,
                s.port,
                s.paired.is_some(),
                s.clients.len(),
                quote(s.paired.as_deref().unwrap_or("")),
                quote(&s.error),
                s.moved && s.clients.is_empty(),
                quote(&s.helper_version),
                quote(bundled),
                age != HelperAge::Current,
                age == HelperAge::Manual
            )
        }
    }
}

/// The version the last connected helper reported ("" when none has connected yet).
pub fn helper_version() -> String {
    state().as_ref().map(|s| s.helper_version.clone()).unwrap_or_default()
}

/// Does the helper know "open"? False only when the last one that connected was older than OPEN_SINCE: then a page
/// shouldn't wait for it.
pub fn helper_knows_open() -> bool {
    let v = helper_version();
    v.is_empty() || !version_older(&v, OPEN_SINCE)
}

/// A helper said its version in hello: kept (and saved). Answers whether Turbine should ask it to reload now (it is
/// older than Turbine's, knows "reload", and wasn't asked for this version yet this run).
pub fn note_helper_version(v: &str) -> bool {
    let v: String = v.trim().chars().filter(|c| c.is_ascii_digit() || *c == '.').take(20).collect();
    let bundled = BUNDLED.get().cloned().unwrap_or_default();
    let mut g = state();
    let Some(s) = g.as_mut() else { return false };
    if v.is_empty() {
        return false;
    }
    if s.helper_version != v {
        s.helper_version = v.clone();
        let _ = write_private(&s.dir.join(HELPER_VERSION_FILE), &v);
    }
    if helper_age(&v, &bundled) != HelperAge::Reload || s.reload_asked.as_deref() == Some(v.as_str()) {
        return false;
    }
    s.reload_asked = Some(v);
    true
}

/// Is an extension connected right now?
pub fn connected() -> bool {
    state().as_ref().is_some_and(|s| !s.clients.is_empty())
}

/// Has a helper ever paired with this Turbine?
pub fn paired() -> bool {
    state().as_ref().is_some_and(|s| s.paired.is_some())
}

/// How long a page waits for the helper to connect before it is dropped (the browser was closed again, or it has
/// no helper).
pub const OPEN_WAIT: Duration = Duration::from_secs(180);

/// {"type":"open","url":...}: the helper opens the page in a new tab.
pub fn open_message(url: &str) -> String {
    format!("{{\"type\":\"open\",\"url\":{}}}", quote(url))
}

/// Asks the helper to open a Nexus page in a new tab. Sent now when a helper is connected (answers true); otherwise
/// kept until one connects within OPEN_WAIT (answers false). Only nexusmods.com https pages; anything else is
/// refused.
pub fn open_tab(url: &str) -> Result<bool, String> {
    if !turbine_engine::browser::nexus_tab_url(url) {
        return Err("only Nexus Mods pages open this way".into());
    }
    if connected() {
        broadcast(&open_message(url));
        if connected() {
            if let Some(s) = state().as_mut() {
                s.pending_open = None;
            }
            return Ok(true);
        }
    }
    if let Some(s) = state().as_mut() {
        s.pending_open = Some((url.to_string(), Instant::now()));
    }
    Ok(false)
}

/// The waiting page, if it is still fresh; it is taken (sent once).
fn take_pending_open() -> Option<String> {
    let mut g = state();
    let s = g.as_mut()?;
    let (url, at) = s.pending_open.take()?;
    (at.elapsed() <= OPEN_WAIT).then_some(url)
}

/// Sends a message to every connected extension. Connections that can't take it are dropped.
pub fn broadcast(text: &str) {
    let frame = encode_frame(OP_TEXT, text.as_bytes());
    let outs: Vec<(u64, Arc<Mutex<TcpStream>>)> = state().as_ref().map(|s| s.clients.iter().map(|c| (c.id, c.out.clone())).collect()).unwrap_or_default();
    for (id, out) in outs {
        let ok = out.lock().map(|mut o| o.write_all(&frame).is_ok()).unwrap_or(false);
        if !ok {
            drop_client(id);
        }
    }
}

fn drop_client(id: u64) {
    if let Some(s) = state().as_mut() {
        s.clients.retain(|c| c.id != id);
    }
}

fn send(out: &Arc<Mutex<TcpStream>>, opcode: u8, payload: &[u8]) -> bool {
    out.lock().map(|mut o| o.write_all(&encode_frame(opcode, payload)).is_ok()).unwrap_or(false)
}

/// Reads the request head, up to the blank line.
fn read_head(stream: &mut TcpStream) -> Option<Vec<u8>> {
    let mut buf = Vec::with_capacity(1024);
    let mut byte = [0u8; 1];
    while buf.len() <= MAX_REQUEST {
        match stream.read(&mut byte) {
            Ok(1) => {
                buf.push(byte[0]);
                if buf.ends_with(b"\r\n\r\n") {
                    buf.truncate(buf.len() - 4);
                    return Some(buf);
                }
            }
            _ => return None,
        }
    }
    None
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// One connection: the HTTP request, then (for /bridge) the WebSocket until it closes.
fn serve(mut stream: TcpStream, port: u16, handler: Handler) {
    let _ = stream.set_read_timeout(Some(HELLO_WAIT));
    let _ = stream.set_nodelay(true);
    let Some(head) = read_head(&mut stream) else { return };
    let req = match parse_request(&head) {
        Ok(r) => r,
        Err(_) => {
            let _ = stream.write_all(plain_response(400, "Bad Request").as_bytes());
            return;
        }
    };
    let (accept, ext) = match answer(&req, port) {
        Answer::Pair => {
            let _ = stream.write_all(page_response(&pair_page()).as_bytes());
            return;
        }
        Answer::Refuse(code, why) => {
            log(&format!("Refused {} {} ({code})", req.method, req.path.split(['?', '#']).next().unwrap_or("")));
            let _ = stream.write_all(plain_response(code, why).as_bytes());
            return;
        }
        Answer::Upgrade { accept, ext } => (accept, ext),
    };
    if state().as_ref().is_some_and(|s| s.clients.len() >= MAX_CLIENTS) {
        let _ = stream.write_all(plain_response(503, "Service Unavailable").as_bytes());
        return;
    }
    if stream.write_all(upgrade_response(&accept).as_bytes()).is_err() {
        return;
    }
    let Ok(writer) = stream.try_clone() else { return };
    let out = Arc::new(Mutex::new(writer));
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    let mut asm = Assembler::default();
    let mut hello = false;
    let started = Instant::now();
    let mut quiet_since = Instant::now();
    let mut pinged = false;
    let close = |code: u16, why: &str| {
        let _ = send(&out, OP_CLOSE, &close_payload(code, why));
        if let Ok(o) = out.lock() {
            let _ = o.shutdown(Shutdown::Both);
        }
    };
    loop {
        let frame = match read_frame(&mut stream, MAX_MESSAGE) {
            Ok(f) => f,
            Err(FrameError::Io(k)) if matches!(k, io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) => {
                if !hello {
                    log("Browser helper didn't say hello in time");
                    close(1008, "hello first");
                    break;
                }
                if pinged && quiet_since.elapsed() >= PING_EVERY * 2 {
                    log("Browser helper stopped answering");
                    close(1001, "no answer");
                    break;
                }
                pinged = send(&out, OP_PING, b"turbine");
                continue;
            }
            Err(FrameError::Eof) | Err(FrameError::Io(_)) => break,
            Err(e) => {
                log(&format!("Browser helper sent a broken frame ({e:?})"));
                close(e.close_code(), "protocol error");
                break;
            }
        };
        quiet_since = Instant::now();
        pinged = false;
        let text = match asm.push(frame, MAX_MESSAGE) {
            Ok(Event::More) | Ok(Event::Pong) => continue,
            Ok(Event::Ping(p)) => {
                send(&out, OP_PONG, &p);
                continue;
            }
            Ok(Event::Close(_)) => {
                close(1000, "bye");
                break;
            }
            Err((code, why)) => {
                log(&format!("Browser helper connection closed: {why}"));
                close(code, why);
                break;
            }
            Ok(Event::Text(t)) => t,
        };
        let Ok(msg) = json::parse(&text) else {
            if !hello {
                close(1008, "hello first");
                break;
            }
            continue;
        };
        let kind = msg.str_at("type").unwrap_or("").to_string();
        if !hello {
            let given = msg.str_at("token").unwrap_or("");
            let want = state().as_ref().map(|s| s.token.clone()).unwrap_or_default();
            if kind != "hello" || !same_token(given, &want) || started.elapsed() > HELLO_WAIT {
                let why = "Turbine didn't accept this pairing. In Turbine, open Settings, Accounts and press Pair again.";
                send(&out, OP_TEXT, format!("{{\"type\":\"error\",\"text\":{}}}", quote(why)).as_bytes());
                log(&format!("Browser helper {ext} refused: {}", if kind != "hello" { "no hello" } else { "wrong pairing token" }));
                close(1008, "not paired");
                break;
            }
            hello = true;
            let _ = stream.set_read_timeout(Some(PING_EVERY));
            let welcome = {
                let mut g = state();
                let Some(s) = g.as_mut() else { break };
                if s.paired.as_deref() != Some(ext.as_str()) {
                    s.paired = Some(ext.clone());
                    let _ = write_private(&s.dir.join(PAIRED_FILE), &ext);
                }
                s.clients.push(Client { id, out: out.clone() });
                s.moved = false;
                format!("{{\"type\":\"welcome\",\"turbineVersion\":{},\"protocol\":{PROTOCOL},\"games\":{}}}", quote(VERSION.get().map(String::as_str).unwrap_or("")), s.games)
            };
            send(&out, OP_TEXT, welcome.as_bytes());
            let hv = msg.str_at("ext").unwrap_or("?").to_string();
            let reload = note_helper_version(&hv);
            let bundled = BUNDLED.get().cloned().unwrap_or_default();
            log(&format!(
                "Browser helper {ext} connected (helper version {hv}{})",
                match helper_age(&hv, &bundled) {
                    HelperAge::Current => String::new(),
                    HelperAge::Reload => format!(", older than Turbine's {bundled}"),
                    HelperAge::Manual => format!(", older than Turbine's {bundled} and too old to reload itself"),
                }
            ));
            if let Some(url) = take_pending_open() {
                if version_older(&hv, OPEN_SINCE) {
                    log(&format!("Didn't ask helper {hv} to open {url}: it is too old to know how"));
                } else {
                    let sent = send(&out, OP_TEXT, open_message(&url).as_bytes());
                    log(&format!("Asked the helper to open {url} in a new tab{}", if sent { "" } else { " (it didn't take it)" }));
                }
            }
            if reload {
                // Turbine's newer files go into its folder first (catch.rs), so the reload picks them up.
                let copied = BEFORE_RELOAD.get().is_none_or(|f| f());
                if copied {
                    let sent = send(&out, OP_TEXT, reload_message().as_bytes());
                    log(&format!("Asked helper {hv} to reload itself from its folder (Turbine has {bundled}){}", if sent { "" } else { " (it didn't take it)" }));
                } else {
                    log(&format!("Didn't ask helper {hv} to reload: Turbine's {bundled} files didn't all go into its folder"));
                }
            }
            handler("connected", &msg);
            continue;
        }
        if kind == "hello" || kind == "ping" {
            continue;
        }
        handler(&kind, &msg);
    }
    drop_client(id);
    if hello {
        log(&format!("Browser helper {ext} disconnected"));
        handler("disconnected", &Json::Null);
    }
}

/// Turbine's version for the welcome message (set once at start).
pub static VERSION: OnceLock<String> = OnceLock::new();

// ---------------------------------------------------------------------------------------------------- tests

// ------------------------------------------------------------------------------------------------ the open guard

/// The guard against opening the same page twice while the browser is still coming up (launch.rs): `opened` holds
/// each page opened in the last `window`. Some(milliseconds since) when `url` is one of them; otherwise it is
/// remembered as opened `now` and the answer is None.
pub fn page_guard(opened: &mut Vec<(String, Instant)>, url: &str, now: Instant, window: Duration) -> Option<u128> {
    opened.retain(|(_, at)| now.saturating_duration_since(*at) < window);
    if let Some((_, at)) = opened.iter().find(|(u, _)| u == url) {
        return Some(now.saturating_duration_since(*at).as_millis());
    }
    opened.push((url.to_string(), now));
    None
}

// ------------------------------------------------------------------------------------------------ the helper's copy

/// A symbolic link (never followed).
pub fn is_link(p: &Path) -> bool {
    std::fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink())
}

/// Copies the helper's files (plain files only, tests left out) into `to`, Turbine's own helper folder. Every file
/// is first written new into a staging folder inside `to`, then renamed into place, so a failure part way leaves
/// the old files as they were, and a link at a file's place is replaced, never written through. Nothing is copied
/// when any folder on the way is a link. Returns how many files were copied.
pub fn copy_helper(from: &Path, to: &Path) -> Result<usize, String> {
    if !from.join("manifest.json").is_file() {
        return Err("this build has no browser helper in it".into());
    }
    // The first setup makes the folder; after that it must be the plain folder Turbine made, never a link.
    if !is_link(to) && !to.exists() {
        std::fs::create_dir_all(to).map_err(|e| format!("helper folder: {e}"))?;
    }
    if is_link(to) || !to.is_dir() {
        return Err("the helper folder isn't a plain folder".into());
    }
    // The files to copy, as paths inside the helper (folders first need checking: none may be a link).
    fn walk(from: &Path, rel: &Path, depth: usize, out: &mut Vec<PathBuf>) -> Result<(), String> {
        if depth > 6 {
            return Ok(());
        }
        let rd = std::fs::read_dir(from.join(rel)).map_err(|e| e.to_string())?;
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || (depth == 0 && name == "test") {
                continue;
            }
            let Ok(m) = std::fs::symlink_metadata(e.path()) else { continue };
            if m.is_dir() {
                walk(from, &rel.join(&name), depth + 1, out)?;
            } else if m.is_file() && m.len() <= 8 << 20 {
                out.push(rel.join(&name));
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    walk(from, Path::new(""), 0, &mut files)?;
    for f in &files {
        let mut p = to.to_path_buf();
        for part in f.parent().into_iter().flat_map(|d| d.components()) {
            p.push(part);
            if is_link(&p) {
                return Err(format!("{} is a link", p.display()));
            }
        }
    }
    let stage = to.join(".turbine-update");
    if is_link(&stage) {
        return Err("the staging folder is a link".into());
    }
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir(&stage).map_err(|e| format!("staging folder: {e}"))?;
    let staged = (|| -> Result<(), String> {
        for (i, f) in files.iter().enumerate() {
            let bytes = std::fs::read(from.join(f)).map_err(|e| format!("{}: {e}", f.display()))?;
            // create_new: a fresh file in the fresh staging folder, never an existing one or a link.
            let mut out = std::fs::OpenOptions::new().write(true).create_new(true).open(stage.join(i.to_string())).map_err(|e| format!("{}: {e}", f.display()))?;
            std::io::Write::write_all(&mut out, &bytes).map_err(|e| format!("{}: {e}", f.display()))?;
            out.sync_all().map_err(|e| format!("{}: {e}", f.display()))?;
        }
        Ok(())
    })();
    if let Err(e) = staged {
        let _ = std::fs::remove_dir_all(&stage);
        return Err(e);
    }
    let mut result = Ok(files.len());
    for (i, f) in files.iter().enumerate() {
        let dest = to.join(f);
        if let Some(dir) = dest.parent() {
            if let Err(e) = std::fs::create_dir_all(dir) {
                result = Err(format!("{}: {e}", f.display()));
                break;
            }
        }
        // A rename replaces whatever is at `dest` (a link included) instead of writing through it.
        if let Err(e) = std::fs::rename(stage.join(i.to_string()), &dest) {
            result = Err(format!("{}: {e}", f.display()));
            break;
        }
    }
    let _ = std::fs::remove_dir_all(&stage);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    #[test]
    fn sha1_matches_the_standard_vectors() {
        assert_eq!(hex(&sha1(b"")), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(hex(&sha1(b"abc")), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(
            hex(&sha1(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq")),
            "84983e441c3bd26ebaae4aa1f95129e5e54670f1"
        );
        assert_eq!(hex(&sha1(&vec![b'a'; 1_000_000])), "34aa973cd4c4daa4f61eeb2bdbad27316534016f");
        // Lengths around the 55/56/64 byte padding edges.
        assert_eq!(hex(&sha1(&[b'x'; 55])), hex(&sha1(&[b'x'; 55])));
        assert_ne!(hex(&sha1(&[b'x'; 56])), hex(&sha1(&[b'x'; 57])));
        assert_eq!(hex(&sha1(b"The quick brown fox jumps over the lazy dog")), "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12");
    }

    #[test]
    fn base64_matches_rfc_4648() {
        for (i, o) in [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foob", "Zm9vYg=="), ("fooba", "Zm9vYmE="), ("foobar", "Zm9vYmFy")] {
            assert_eq!(base64(i.as_bytes()), o);
        }
        assert_eq!(base64(&[0xFB, 0xFF]), "+/8=");
    }

    #[test]
    fn handshake_accept_matches_rfc_6455() {
        // RFC 6455 section 1.3.
        assert_eq!(accept_key("dGhlIHNhbXBsZSBub25jZQ=="), "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }

    fn request(lines: &[&str]) -> Request {
        parse_request(lines.join("\r\n").as_bytes()).unwrap()
    }

    fn ws_request(origin: &str, host: &str) -> Request {
        request(&[
            "GET /bridge HTTP/1.1",
            &format!("Host: {host}"),
            "Upgrade: websocket",
            "Connection: keep-alive, Upgrade",
            "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==",
            "Sec-WebSocket-Version: 13",
            &format!("Origin: {origin}"),
        ])
    }

    #[test]
    fn upgrade_needs_an_extension_origin_and_turbines_host() {
        let ok = answer(&ws_request("chrome-extension://abcdefghijklmnopabcdefghijklmnop", "127.0.0.1:47701"), 47701);
        assert_eq!(ok, Answer::Upgrade { accept: "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=".into(), ext: "abcdefghijklmnopabcdefghijklmnop".into() });
        let moz = answer(&ws_request("moz-extension://0a1b2c3d-1111-2222-3333-444455556666", "localhost:47701"), 47701);
        assert!(matches!(moz, Answer::Upgrade { .. }));
        // Web pages, missing origins, odd IDs and other hosts are refused.
        for origin in ["https://www.nexusmods.com", "null", "chrome-extension://", "chrome-extension://ABC", "chrome-extension://a/b", "file://"] {
            assert_eq!(answer(&ws_request(origin, "127.0.0.1:47701"), 47701), Answer::Refuse(403, "Forbidden"), "{origin}");
        }
        assert_eq!(answer(&ws_request("chrome-extension://abc", "evil.example:47701"), 47701), Answer::Refuse(403, "Forbidden"));
        assert_eq!(answer(&ws_request("chrome-extension://abc", "127.0.0.1:47702"), 47701), Answer::Refuse(403, "Forbidden"));
        let no_origin = request(&["GET /bridge HTTP/1.1", "Host: 127.0.0.1:47701", "Upgrade: websocket", "Connection: Upgrade",
            "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==", "Sec-WebSocket-Version: 13"]);
        assert_eq!(answer(&no_origin, 47701), Answer::Refuse(403, "Forbidden"));
    }

    #[test]
    fn upgrade_needs_the_right_headers() {
        let base = ["GET /bridge HTTP/1.1", "Host: 127.0.0.1:47701", "Origin: chrome-extension://abc"];
        let mut no_upgrade = base.to_vec();
        no_upgrade.extend(["Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==", "Sec-WebSocket-Version: 13"]);
        assert_eq!(answer(&request(&no_upgrade), 47701), Answer::Refuse(426, "Upgrade Required"));
        let mut old = base.to_vec();
        old.extend(["Upgrade: websocket", "Connection: Upgrade", "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==", "Sec-WebSocket-Version: 8"]);
        assert_eq!(answer(&request(&old), 47701), Answer::Refuse(426, "Upgrade Required"));
        let mut bad_key = base.to_vec();
        bad_key.extend(["Upgrade: websocket", "Connection: Upgrade", "Sec-WebSocket-Key: short", "Sec-WebSocket-Version: 13"]);
        assert_eq!(answer(&request(&bad_key), 47701), Answer::Refuse(400, "Bad Request"));
        let post = request(&["POST /bridge HTTP/1.1", "Host: 127.0.0.1:47701"]);
        assert_eq!(answer(&post, 47701), Answer::Refuse(405, "Method Not Allowed"));
        assert_eq!(answer(&request(&["GET /pair HTTP/1.1", "Host: 127.0.0.1:47701"]), 47701), Answer::Pair);
        assert_eq!(answer(&request(&["GET /pair?x=1 HTTP/1.1", "Host: localhost:47701"]), 47701), Answer::Pair);
        assert_eq!(answer(&request(&["GET /other HTTP/1.1", "Host: 127.0.0.1:47701"]), 47701), Answer::Refuse(404, "Not Found"));
        assert!(upgrade_response("abc").starts_with("HTTP/1.1 101 "));
    }

    #[test]
    fn bad_requests_are_refused() {
        assert!(parse_request(b"GET / HTTP/1.1\r\nHost 127.0.0.1").is_err());
        assert!(parse_request(b"GET nothing HTTP/1.1").is_err());
        assert!(parse_request(b"GET / FTP/1.0").is_err());
        assert!(parse_request(&vec![b'a'; MAX_REQUEST + 1]).is_err());
        let r = parse_request(b"GET /pair HTTP/1.1\r\nhOsT:  127.0.0.1:47701 ").unwrap();
        assert_eq!(r.header("Host"), Some("127.0.0.1:47701"));
    }

    #[test]
    fn tokens_compare_whole() {
        let t = "a".repeat(64);
        assert!(same_token(&t, &t));
        assert!(!same_token(&"a".repeat(63), &t));
        assert!(!same_token(&format!("{}b", "a".repeat(63)), &t));
        assert!(!same_token("", ""));
        assert!(plausible_token(&new_token().unwrap()));
        assert_ne!(new_token().unwrap(), new_token().unwrap());
    }

    /// A client frame: masked, as browsers send them.
    fn client_frame(fin: bool, opcode: u8, payload: &[u8]) -> Vec<u8> {
        let mask = [0x37, 0xfa, 0x21, 0x3d];
        let mut out = vec![if fin { 0x80 } else { 0 } | opcode];
        match payload.len() {
            n if n < 126 => out.push(0x80 | n as u8),
            n if n <= 0xFFFF => {
                out.push(0x80 | 126);
                out.extend_from_slice(&(n as u16).to_be_bytes());
            }
            n => {
                out.push(0x80 | 127);
                out.extend_from_slice(&(n as u64).to_be_bytes());
            }
        }
        out.extend_from_slice(&mask);
        out.extend(payload.iter().enumerate().map(|(i, b)| b ^ mask[i % 4]));
        out
    }

    #[test]
    fn reads_the_rfc_example_frame() {
        // RFC 6455 section 5.7: a masked "Hello".
        let bytes = [0x81, 0x85, 0x37, 0xfa, 0x21, 0x3d, 0x7f, 0x9f, 0x4d, 0x51, 0x58];
        let f = read_frame(&mut &bytes[..], MAX_MESSAGE).unwrap();
        assert_eq!(f, Frame { fin: true, opcode: OP_TEXT, payload: b"Hello".to_vec() });
        // And Turbine's own (unmasked) frames are what the RFC shows for a server.
        assert_eq!(encode_frame(OP_TEXT, b"Hello"), vec![0x81, 0x05, 0x48, 0x65, 0x6c, 0x6c, 0x6f]);
    }

    #[test]
    fn reads_long_lengths_and_writes_them() {
        for n in [125usize, 126, 300, 65535, 65536, 70000] {
            let payload = vec![b'x'; n];
            let f = read_frame(&mut &client_frame(true, OP_TEXT, &payload)[..], MAX_MESSAGE).unwrap();
            assert_eq!(f.payload.len(), n);
            let out = encode_frame(OP_TEXT, &payload);
            let head = if n < 126 { 2 } else if n <= 0xFFFF { 4 } else { 10 };
            assert_eq!(out.len(), n + head);
        }
    }

    #[test]
    fn refuses_broken_frames() {
        // Unmasked client frame.
        assert_eq!(read_frame(&mut &[0x81u8, 0x02, b'h', b'i'][..], MAX_MESSAGE), Err(FrameError::Unmasked));
        // Reserved bits and opcodes.
        let mut rsv = client_frame(true, OP_TEXT, b"hi");
        rsv[0] |= 0x40;
        assert_eq!(read_frame(&mut &rsv[..], MAX_MESSAGE), Err(FrameError::Reserved));
        assert_eq!(read_frame(&mut &client_frame(true, 0x3, b"hi")[..], MAX_MESSAGE), Err(FrameError::Reserved));
        // Over the limit: refused before the payload is read.
        assert_eq!(read_frame(&mut &client_frame(true, OP_TEXT, &[b'x'; 200])[..], 100), Err(FrameError::TooBig));
        assert_eq!(FrameError::TooBig.close_code(), 1009);
        // Control frames: no fragments, at most 125 bytes.
        assert_eq!(read_frame(&mut &client_frame(false, OP_PING, b"x")[..], MAX_MESSAGE), Err(FrameError::BadControl));
        assert_eq!(read_frame(&mut &client_frame(true, OP_PING, &[0; 126])[..], MAX_MESSAGE), Err(FrameError::BadControl));
        // Cut short.
        let full = client_frame(true, OP_TEXT, b"hello");
        assert_eq!(read_frame(&mut &full[..full.len() - 2], MAX_MESSAGE), Err(FrameError::Eof));
    }

    fn feed(asm: &mut Assembler, bytes: &[u8], limit: usize) -> Result<Event, (u16, &'static str)> {
        asm.push(read_frame(&mut &bytes[..], MAX_MESSAGE).unwrap(), limit)
    }

    #[test]
    fn assembles_fragments_and_answers_control_frames() {
        let mut a = Assembler::default();
        assert_eq!(feed(&mut a, &client_frame(false, OP_TEXT, b"{\"type\":"), 100), Ok(Event::More));
        // A ping may come between fragments.
        assert_eq!(feed(&mut a, &client_frame(true, OP_PING, b"p"), 100), Ok(Event::Ping(b"p".to_vec())));
        assert_eq!(feed(&mut a, &client_frame(true, OP_CONTINUATION, b"\"hello\"}"), 100), Ok(Event::Text("{\"type\":\"hello\"}".into())));
        assert_eq!(feed(&mut a, &client_frame(true, OP_PONG, b""), 100), Ok(Event::Pong));
        assert_eq!(feed(&mut a, &client_frame(true, OP_CLOSE, &close_payload(1000, "bye")), 100), Ok(Event::Close(Some(1000))));
        assert_eq!(feed(&mut a, &client_frame(true, OP_CLOSE, b""), 100), Ok(Event::Close(None)));
    }

    #[test]
    fn closes_on_bad_messages() {
        let mut a = Assembler::default();
        assert_eq!(feed(&mut a, &client_frame(true, OP_BINARY, b"x"), 100), Err((1003, "text messages only")));
        let mut a = Assembler::default();
        assert_eq!(feed(&mut a, &client_frame(true, OP_CONTINUATION, b"x"), 100).unwrap_err().0, 1002);
        let mut a = Assembler::default();
        feed(&mut a, &client_frame(false, OP_TEXT, b"x"), 100).unwrap();
        assert_eq!(feed(&mut a, &client_frame(true, OP_TEXT, b"y"), 100).unwrap_err().0, 1002);
        let mut a = Assembler::default();
        feed(&mut a, &client_frame(false, OP_TEXT, &[b'x'; 60]), 100).unwrap();
        assert_eq!(feed(&mut a, &client_frame(true, OP_CONTINUATION, &[b'x'; 60]), 100).unwrap_err().0, 1009);
        let mut a = Assembler::default();
        assert_eq!(feed(&mut a, &client_frame(true, OP_TEXT, &[0xff, 0xfe]), 100).unwrap_err().0, 1007);
        let mut a = Assembler::default();
        assert_eq!(feed(&mut a, &client_frame(true, OP_CLOSE, &[3]), 100).unwrap_err().0, 1002);
    }

    #[test]
    fn open_asks_only_for_nexus_pages() {
        assert_eq!(open_message("https://www.nexusmods.com/a?b=\"c\""), "{\"type\":\"open\",\"url\":\"https://www.nexusmods.com/a?b=\\\"c\\\"\"}");
        assert!(open_tab("https://evil.example/").is_err());
        assert!(open_tab("http://www.nexusmods.com/").is_err());
        // Nothing is connected here, so a Nexus page waits for the helper.
        assert_eq!(open_tab("https://www.nexusmods.com/skyrimspecialedition/mods/1"), Ok(false));
    }

    #[test]
    fn reads_nexus_addresses() {
        let r = nexus_ref("https://cf-files.nexus-cdn.com/3333/107/Cyber%20Engine%20Tweaks-107-1-32-0.zip?md5=x",
            "https://www.nexusmods.com/cyberpunk2077/mods/107?tab=files&file_id=95641&nmm=1").unwrap();
        assert_eq!(r, NexusRef { domain: "cyberpunk2077".into(), mod_id: 107, file_id: Some(95641) });
        let r = nexus_ref("https://supporter-files.nexus-cdn.com/x.7z", "https://nexusmods.com/games/witcher3/mods/3017").unwrap();
        assert_eq!(r, NexusRef { domain: "witcher3".into(), mod_id: 3017, file_id: None });
        assert_eq!(nexus_ref("https://www.nexusmods.com/skyrimspecialedition/mods/12604?tab=files&file_id=abc", "").unwrap().file_id, None);
        // The file servers alone don't name the game; other sites aren't Nexus.
        assert_eq!(nexus_ref("https://cf-files.nexus-cdn.com/1704/12604/x.7z", ""), None);
        assert_eq!(nexus_ref("https://example.com/cyberpunk2077/mods/107", "http://www.nexusmods.com/cyberpunk2077/mods/107"), None);
        assert_eq!(nexus_page("https://www.nexusmods.com.evil.example/cyberpunk2077/mods/107"), None);
        assert_eq!(nexus_page("https://www.nexusmods.com/Cyberpunk2077/mods/107"), None);
        assert_eq!(nexus_page("https://www.nexusmods.com/cyberpunk2077/collections/107"), None);
        assert!(is_nexus_host("https://cf-files.nexus-cdn.com/a"));
        assert!(is_nexus_host("https://www.nexusmods.com/a"));
        assert!(!is_nexus_host("https://nexus-cdn.com.evil.example/a"));
        assert!(!is_nexus_host("https://evil.example/@www.nexusmods.com"));
        assert!(!is_nexus_host("http://www.nexusmods.com/a"));
    }

    #[test]
    fn pair_page_is_small_and_scriptless() {
        let p = pair_page();
        assert!(p.contains("id=\"turbine-pair-status\"") && !p.contains("<script"));
        assert!(page_response(&p).contains("default-src 'none'"));
    }

    /// A real connection: the handshake, hello with a wrong and a right token, welcome, a message and a ping.
    #[test]
    fn talks_websocket_over_a_socket() {
        let dir = std::env::temp_dir().join(format!("turbine-bridge-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let got: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let g = got.clone();
        let handler: Handler = Arc::new(move |kind: &str, _msg: &Json| g.lock().unwrap().push(kind.to_string()));
        let port = start(&dir, |_| {}, handler).unwrap();
        set_games("[{\"id\":\"steam:1091500\",\"name\":\"Cyberpunk 2077\",\"domain\":\"cyberpunk2077\"}]".into());
        let token = pair_url().unwrap().split('#').nth(1).unwrap().to_string();
        let connect = || {
            let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
            s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            write!(s, "GET /bridge HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
                       Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\nOrigin: chrome-extension://testext\r\n\r\n").unwrap();
            let head = read_head(&mut s).unwrap();
            assert!(String::from_utf8_lossy(&head).contains("s3pPLMBiTxaQ9kYGzzhZRbK+xOo="));
            s
        };
        let server_frame = |s: &mut TcpStream| {
            let mut h = [0u8; 2];
            s.read_exact(&mut h).unwrap();
            let mut len = usize::from(h[1] & 0x7F);
            if len == 126 {
                let mut b = [0u8; 2];
                s.read_exact(&mut b).unwrap();
                len = usize::from(u16::from_be_bytes(b));
            }
            let mut p = vec![0u8; len];
            s.read_exact(&mut p).unwrap();
            (h[0] & 0x0F, p)
        };
        // Wrong token: closed with 1008.
        let mut s = connect();
        s.write_all(&client_frame(true, OP_TEXT, b"{\"type\":\"hello\",\"token\":\"nope\",\"version\":1}")).unwrap();
        let (op, p) = server_frame(&mut s);
        assert_eq!(op, OP_TEXT);
        assert!(String::from_utf8(p).unwrap().contains("\"error\""));
        let (op, p) = server_frame(&mut s);
        assert_eq!((op, u16::from_be_bytes([p[0], p[1]])), (OP_CLOSE, 1008));
        assert!(!connected());
        // Right token: welcome, then messages reach the handler and pings get pongs.
        let mut s = connect();
        let hello = format!("{{\"type\":\"hello\",\"token\":\"{token}\",\"version\":1,\"ext\":\"0.1\"}}");
        s.write_all(&client_frame(true, OP_TEXT, hello.as_bytes())).unwrap();
        let (op, p) = server_frame(&mut s);
        assert_eq!(op, OP_TEXT);
        let w = String::from_utf8(p).unwrap();
        assert!(w.contains("\"welcome\"") && w.contains("cyberpunk2077"));
        assert!(connected());
        assert!(status_json().contains("\"paired\":true"));
        s.write_all(&client_frame(true, OP_PING, b"x")).unwrap();
        assert_eq!(server_frame(&mut s), (OP_PONG, b"x".to_vec()));
        s.write_all(&client_frame(true, OP_TEXT, b"{\"type\":\"ping\"}")).unwrap();
        s.write_all(&client_frame(true, OP_TEXT, b"{\"type\":\"page\",\"url\":\"https://www.nexusmods.com/cyberpunk2077/mods/107\"}")).unwrap();
        broadcast("{\"type\":\"status\",\"text\":\"Downloading\"}");
        let (op, p) = server_frame(&mut s);
        assert_eq!(op, OP_TEXT);
        assert!(String::from_utf8(p).unwrap().contains("Downloading"));
        // Pair again: the old connection ends and the token changes.
        pair_again().unwrap();
        assert!(!pair_url().unwrap().ends_with(&token));
        assert!(!connected());
        std::thread::sleep(Duration::from_millis(100));
        let kinds = got.lock().unwrap().clone();
        assert!(kinds.contains(&"connected".to_string()) && kinds.contains(&"page".to_string()), "{kinds:?}");
        assert_eq!(std::fs::read_to_string(dir.join(PORT_FILE)).unwrap(), port.to_string());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn helper_versions_compare_by_number() {
        assert!(version_older("0.41", "0.53"));
        assert!(version_older("0.9", "0.10"));
        assert!(version_older("0.53", "1.0"));
        assert!(version_older("0.5", "0.5.1"));
        assert!(!version_older("0.53", "0.53"));
        assert!(!version_older("0.54", "0.53"));
        assert!(!version_older("1.0", "0.99"));
        assert!(version_older("?", "0.1"));
    }

    #[test]
    fn old_helpers_reload_or_ask_the_player() {
        assert_eq!(helper_age("0.53", "0.53"), HelperAge::Current);
        assert_eq!(helper_age("0.60", "0.54"), HelperAge::Current);
        assert_eq!(helper_age("", "0.54"), HelperAge::Current);
        assert_eq!(helper_age("0.52", ""), HelperAge::Current);
        // 0.53 and earlier don't know "reload": the player presses the reload arrow once.
        assert_eq!(helper_age("0.41", "0.54"), HelperAge::Manual);
        assert_eq!(helper_age("0.53", "0.54"), HelperAge::Manual);
        assert_eq!(helper_age("0.54", "0.55"), HelperAge::Reload);
        assert_eq!(reload_message(), "{\"type\":\"reload\"}");
        assert!(OLD_HELPER_TEXT.contains("chrome://extensions"));
    }

    // ---------------------------------------------------------------- the helper's copy (catch.rs runs it)

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("turbine-copy-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn helper_at(dir: &Path, version: &str) {
        std::fs::create_dir_all(dir.join("lib")).unwrap();
        std::fs::create_dir_all(dir.join("test")).unwrap();
        std::fs::write(dir.join("manifest.json"), format!("{{\"name\":\"Turbine browser helper\",\"version\":\"{version}\"}}")).unwrap();
        std::fs::write(dir.join("background.js"), format!("// {version}")).unwrap();
        std::fs::write(dir.join("lib/bridge.js"), format!("// lib {version}")).unwrap();
        std::fs::write(dir.join("test/test.js"), "// tests stay behind").unwrap();
    }

    #[test]
    fn helper_copy_replaces_files_and_leaves_nothing_behind() {
        let root = temp("plain");
        let (from, to) = (root.join("new"), root.join("Downloads/helper"));
        helper_at(&from, "0.54");
        helper_at(&to, "0.53");
        std::fs::remove_dir_all(to.join("test")).unwrap();
        assert_eq!(copy_helper(&from, &to), Ok(3));
        assert!(std::fs::read_to_string(to.join("manifest.json")).unwrap().contains("0.54"));
        assert_eq!(std::fs::read_to_string(to.join("lib/bridge.js")).unwrap(), "// lib 0.54");
        assert!(!to.join("test").exists(), "tests are left out");
        assert!(!to.join(".turbine-update").exists(), "the staging folder is gone");
        // The first setup: the folder is made.
        let fresh = root.join("Downloads/fresh");
        assert_eq!(copy_helper(&from, &fresh), Ok(3));
        assert!(fresh.join("lib/bridge.js").is_file());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn helper_copy_never_writes_through_a_link() {
        use std::os::unix::fs::symlink;
        // A file in the helper folder that is a link to a file outside it: the link is replaced, the outside file
        // keeps its words.
        let root = temp("link");
        let (from, to) = (root.join("new"), root.join("Downloads/helper"));
        helper_at(&from, "0.54");
        helper_at(&to, "0.53");
        let outside = root.join("precious.txt");
        std::fs::write(&outside, "mine").unwrap();
        std::fs::remove_file(to.join("background.js")).unwrap();
        symlink(&outside, to.join("background.js")).unwrap();
        assert!(copy_helper(&from, &to).is_ok());
        assert_eq!(std::fs::read_to_string(&outside).unwrap(), "mine");
        assert!(!is_link(&to.join("background.js")));
        // A folder in it that is a link: nothing is copied at all.
        let elsewhere = root.join("elsewhere");
        std::fs::create_dir_all(&elsewhere).unwrap();
        std::fs::remove_dir_all(to.join("lib")).unwrap();
        symlink(&elsewhere, to.join("lib")).unwrap();
        std::fs::write(to.join("manifest.json"), "{\"name\":\"Turbine browser helper\",\"version\":\"0.53\"}").unwrap();
        assert!(copy_helper(&from, &to).is_err());
        assert!(std::fs::read_dir(&elsewhere).unwrap().next().is_none(), "nothing written through the folder link");
        assert!(std::fs::read_to_string(to.join("manifest.json")).unwrap().contains("0.53"), "nothing replaced");
        // The helper folder itself a link: refused.
        let real = root.join("real");
        helper_at(&real, "0.53");
        let linked = root.join("Downloads/linked");
        symlink(&real, &linked).unwrap();
        assert!(copy_helper(&from, &linked).is_err());
        assert!(std::fs::read_to_string(real.join("manifest.json")).unwrap().contains("0.53"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_open_guard_remembers_each_page() {
        let w = Duration::from_secs(8);
        let t0 = Instant::now();
        let mut l = Vec::new();
        assert_eq!(page_guard(&mut l, "a", t0, w), None);
        assert_eq!(page_guard(&mut l, "b", t0 + Duration::from_secs(1), w), None);
        // A, B, then A again within 8 seconds: A is held back (the old guard kept only the last page).
        assert_eq!(page_guard(&mut l, "a", t0 + Duration::from_secs(2), w), Some(2000));
        assert_eq!(page_guard(&mut l, "b", t0 + Duration::from_secs(3), w), Some(2000));
        // After 8 seconds a page opens again.
        assert_eq!(page_guard(&mut l, "a", t0 + Duration::from_secs(9), w), None);
        assert_eq!(l.len(), 1, "old pages are forgotten");
    }
}
