// A tiny stand-in for Turbine's bridge server, built on Node's own http module (no packages).
// It speaks just enough WebSocket (RFC 6455, text frames, close, ping) and protocol v1 for the tests.
"use strict";

const http = require("http");
const crypto = require("crypto");

function frame(opcode, payload) {
  const data = Buffer.from(payload || "");
  let header;
  if (data.length < 126) header = Buffer.from([0x80 | opcode, data.length]);
  else if (data.length < 65536) {
    header = Buffer.alloc(4);
    header[0] = 0x80 | opcode;
    header[1] = 126;
    header.writeUInt16BE(data.length, 2);
  } else {
    header = Buffer.alloc(10);
    header[0] = 0x80 | opcode;
    header[1] = 127;
    header.writeBigUInt64BE(BigInt(data.length), 2);
  }
  return Buffer.concat([header, data]);
}

// Reads complete client frames from buf. Returns {frames, rest}.
function parseFrames(buf) {
  const frames = [];
  for (;;) {
    if (buf.length < 2) break;
    const opcode = buf[0] & 0x0f;
    const masked = (buf[1] & 0x80) !== 0;
    let len = buf[1] & 0x7f;
    let off = 2;
    if (len === 126) {
      if (buf.length < 4) break;
      len = buf.readUInt16BE(2);
      off = 4;
    } else if (len === 127) {
      if (buf.length < 10) break;
      len = Number(buf.readBigUInt64BE(2));
      off = 10;
    }
    const maskLen = masked ? 4 : 0;
    if (buf.length < off + maskLen + len) break;
    const mask = masked ? buf.subarray(off, off + 4) : null;
    const payload = Buffer.from(buf.subarray(off + maskLen, off + maskLen + len));
    if (mask) for (let i = 0; i < payload.length; i++) payload[i] ^= mask[i % 4];
    frames.push({ opcode, payload });
    buf = buf.subarray(off + maskLen + len);
  }
  return { frames, rest: buf };
}

// opts: {token, port, turbineVersion, games}
function startFakeTurbine(opts) {
  // Serves a bare pairing page at /pair, like Turbine does; the extension reads the token from the hash.
  const server = http.createServer((req, res) => {
    if (req.method === "GET" && req.url.startsWith("/pair")) {
      res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
      res.end("<!DOCTYPE html><title>Turbine pairing</title><h1>Turbine pairing</h1>");
      return;
    }
    res.writeHead(404);
    res.end();
  });
  const fake = {
    server,
    received: [], // every JSON message from any client, in order
    clients: new Set(),
    origins: [],
    rejected: 0,
    waiters: [],
    send(msg) {
      for (const c of fake.clients) if (c.authed) c.socket.write(frame(1, JSON.stringify(msg)));
    },
    // Resolves with the first received message matching pred (already received ones count).
    waitFor(pred, ms) {
      const hit = fake.received.find(pred);
      if (hit) return Promise.resolve(hit);
      return new Promise((resolve, reject) => {
        const w = { pred, resolve: (m) => (clearTimeout(timer), resolve(m)) };
        fake.waiters.push(w);
        const timer = setTimeout(() => {
          fake.waiters = fake.waiters.filter((x) => x !== w);
          reject(new Error("timed out waiting for a message: " + pred.toString() + " got " + JSON.stringify(fake.received.map((m) => m.type))));
        }, ms || 3000);
      });
    },
    dropAll() {
      for (const c of fake.clients) c.socket.destroy();
      fake.clients.clear();
    },
    close() {
      fake.dropAll();
      return new Promise((r) => server.close(() => r()));
    },
  };

  server.on("upgrade", (req, socket) => {
    if (req.url !== "/bridge" || (req.headers.upgrade || "").toLowerCase() !== "websocket") {
      socket.destroy();
      return;
    }
    fake.origins.push(req.headers.origin || null);
    const accept = crypto
      .createHash("sha1")
      .update(req.headers["sec-websocket-key"] + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11")
      .digest("base64");
    socket.write(
      "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: " + accept + "\r\n\r\n"
    );
    const client = { socket, authed: false };
    fake.clients.add(client);
    let buf = Buffer.alloc(0);
    socket.on("data", (chunk) => {
      const r = parseFrames(Buffer.concat([buf, chunk]));
      buf = r.rest;
      for (const f of r.frames) {
        if (f.opcode === 8) {
          socket.end(frame(8, ""));
          return;
        }
        if (f.opcode === 9) {
          socket.write(frame(10, f.payload));
          continue;
        }
        if (f.opcode !== 1) continue;
        let msg;
        try {
          msg = JSON.parse(f.payload.toString("utf8"));
        } catch (e) {
          continue;
        }
        if (!client.authed) {
          // Protocol: the first message must be hello with the right token, or the socket is closed.
          if (msg.type !== "hello" || msg.token !== opts.token || msg.version !== 1) {
            fake.rejected += 1;
            socket.write(frame(1, JSON.stringify({ type: "error", text: "This browser is not paired." })));
            socket.end(frame(8, Buffer.from([0x0f, 0xa0]))); // close code 4000
            return;
          }
          client.authed = true;
          socket.write(
            frame(1, JSON.stringify({ type: "welcome", turbineVersion: opts.turbineVersion || "0.3", games: opts.games || [] }))
          );
        }
        fake.received.push(msg);
        fake.waiters = fake.waiters.filter((w) => {
          if (w.pred(msg)) {
            w.resolve(msg);
            return false;
          }
          return true;
        });
      }
    });
    socket.on("close", () => fake.clients.delete(client));
    socket.on("error", () => fake.clients.delete(client));
  });

  return new Promise((resolve) => server.listen(opts.port || 0, "127.0.0.1", () => {
    fake.port = server.address().port;
    resolve(fake);
  }));
}

module.exports = { startFakeTurbine };
