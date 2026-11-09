// The whole "Sign in with my browser" handshake, end to end: the helper's real service worker (background.js, in a
// fake browser) against Turbine's real bridge server (app/src/bridge.rs, built into tools/signin-standin.rs), with a
// mocked Nexus: signing in sets Nexus's cookie, and a mocked API Keys page shows the Personal API Key.
// Run: TURBINE_STANDIN=<the stand-in binary> node test/handshake.js   (the build does this; skipped without it)
"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("fs");
const os = require("os");
const net = require("net");
const path = require("path");
const vm = require("vm");
const crypto = require("crypto");
const { spawn } = require("child_process");

const K = require("../lib/keyfind.js");
const P = require("../lib/protocol.js");
const { h, page } = require("./fake-dom.js");

const EXT_ID = "iniglfblobfbekihldmgekchfehncbeb";
const KEY = "TWFkZVVwRm9yVGhlVGVzdA==--bm90QVJlYWxLZXk5ODc2=--0a1b2c3d4e5f";
const OTHER = "T3RoZXJBcHBLZXkxMjM0NTY3OA==--b3RoZXI5ODc2NTQ=--ffee00112233";
const COOKIE_VALUE = "nexusSessionValueNeverPrinted42";
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function until(fn, ms, what) {
  const end = Date.now() + (ms || 5000);
  while (Date.now() < end) {
    if (fn()) return;
    await sleep(10);
  }
  throw new Error("timed out: " + (what || "condition"));
}

// The browser's WebSocket as the helper uses it, sending the extension's Origin like Chrome does (Node's own
// WebSocket can't), so Turbine's real server checks it exactly as on the device.
class ExtensionSocket {
  constructor(url) {
    const u = new URL(url);
    this.readyState = 0;
    this.buf = Buffer.alloc(0);
    this.open = false;
    this.sock = net.connect(Number(u.port), u.hostname, () => {
      const key = crypto.randomBytes(16).toString("base64");
      this.sock.write(
        `GET ${u.pathname} HTTP/1.1\r\nHost: ${u.host}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: ${key}\r\n` +
          `Sec-WebSocket-Version: 13\r\nOrigin: chrome-extension://${EXT_ID}\r\n\r\n`
      );
    });
    this.sock.on("data", (d) => this.data(d));
    this.sock.on("close", () => this.closed());
    this.sock.on("error", () => {});
  }
  data(d) {
    this.buf = Buffer.concat([this.buf, d]);
    if (!this.open) {
      const end = this.buf.indexOf("\r\n\r\n");
      if (end < 0) return;
      const head = this.buf.subarray(0, end).toString();
      this.buf = this.buf.subarray(end + 4);
      if (!/^HTTP\/1\.1 101/.test(head)) return this.sock.destroy();
      this.open = true;
      this.readyState = 1;
      if (this.onopen) this.onopen();
    }
    for (;;) {
      if (this.buf.length < 2) return;
      const op = this.buf[0] & 0x0f;
      let len = this.buf[1] & 0x7f;
      let off = 2;
      if (len === 126) {
        if (this.buf.length < 4) return;
        len = this.buf.readUInt16BE(2);
        off = 4;
      } else if (len === 127) {
        if (this.buf.length < 10) return;
        len = Number(this.buf.readBigUInt64BE(2));
        off = 10;
      }
      if (this.buf.length < off + len) return;
      const payload = this.buf.subarray(off, off + len);
      this.buf = this.buf.subarray(off + len);
      if (op === 1 && this.onmessage) this.onmessage({ data: payload.toString() });
      else if (op === 8) this.sock.end();
      else if (op === 9) this.frame(10, payload);
    }
  }
  frame(op, payload) {
    const data = Buffer.from(payload);
    const mask = crypto.randomBytes(4);
    const head = data.length < 126 ? Buffer.from([0x80 | op, 0x80 | data.length]) : Buffer.from([0x80 | op, 0x80 | 126, data.length >> 8, data.length & 255]);
    const masked = Buffer.from(data.map((b, i) => b ^ mask[i % 4]));
    this.sock.write(Buffer.concat([head, mask, masked]));
  }
  send(text) {
    if (this.readyState !== 1) throw new Error("not open");
    this.frame(1, text);
  }
  close() {
    this.sock.destroy();
  }
  closed() {
    if (this.readyState === 3) return;
    this.readyState = 3;
    if (this.onclose) this.onclose();
  }
}

// Like Nexus's API Keys page: other apps' keys in their own boxes, the Personal API Key at the bottom.
function mockedKeysPage() {
  return page(
    h("main", h("h1", "API Keys"), h("section", h("h3", "Some app"), h("input", { value: OTHER })), h("section", h("h2", "Personal API Key"), h("p", "Do not share this key."), h("div", h("input", { value: KEY }))))
  );
}

test("Sign in with my browser, end to end against Turbine's real bridge", { skip: !process.env.TURBINE_STANDIN && "TURBINE_STANDIN not set" }, async (t) => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "turbine-handshake-"));
  const turbine = spawn(process.env.TURBINE_STANDIN, [dir], { env: Object.assign({}, process.env, { TURBINE_STANDIN_KEY: KEY }) });
  t.after(() => {
    turbine.kill();
    fs.rmSync(dir, { recursive: true, force: true });
  });
  const lines = [];
  let out = "";
  turbine.stdout.on("data", (d) => {
    out += d;
    const parts = out.split("\n");
    out = parts.pop();
    lines.push(...parts);
  });
  turbine.stderr.on("data", (d) => lines.push("ERR " + d));
  const said = (re) => lines.some((l) => re.test(l));
  await until(() => said(/^READY /), 5000, "Turbine's bridge listening");
  const [, port, token] = lines.find((l) => l.startsWith("READY ")).split(" ");

  // The helper's real service worker in a fake browser: already paired, Chrome signed out of Nexus.
  const extDir = path.join(__dirname, "..");
  const local = { turbinePort: Number(port), turbineToken: token };
  const session = {};
  const listeners = {};
  const on = (name) => ({ addListener: (fn) => ((listeners[name] = listeners[name] || []).push(fn)) });
  const fire = (name, ...args) => (listeners[name] || []).map((fn) => fn(...args));
  const pick = (store, keys) => Object.fromEntries((Array.isArray(keys) ? keys : [keys]).filter((k) => k in store).map((k) => [k, store[k]]));
  const jar = [];
  const tabs = new Map();
  let nextTab = 20;
  const pageUrl = "chrome-extension://" + EXT_ID + "/signin/signin.html";
  const chrome = {
    runtime: {
      id: EXT_ID,
      getURL: (p) => "chrome-extension://" + EXT_ID + "/" + p,
      getManifest: () => JSON.parse(fs.readFileSync(path.join(extDir, "manifest.json"), "utf8")),
      reload: () => {},
      onConnect: on("connect"),
      onMessage: on("message"),
      onStartup: on("startup"),
      onInstalled: on("installed"),
    },
    storage: {
      local: { get: async (k) => pick(local, k), set: async (o) => Object.assign(local, o) },
      session: { get: async (k) => pick(session, k), set: async (o) => Object.assign(session, JSON.parse(JSON.stringify(o))) },
    },
    downloads: { onCreated: on("a"), onChanged: on("b"), onErased: on("c"), onDeterminingFilename: on("d"), search: async () => [] },
    tabs: {
      create: async (o) => {
        const tab = { id: nextTab++, windowId: 1, url: o.url };
        tabs.set(tab.id, tab);
        return tab;
      },
      get: async (id) => tabs.get(id),
      update: async (id, o) => {
        if (o.url) tabs.get(id).url = o.url;
        return tabs.get(id);
      },
      move: async () => {},
      onRemoved: on("tabRemoved"),
    },
    windows: { update: async () => {}, create: async () => {}, onCreated: on("windowCreated") },
    cookies: { onChanged: on("cookieChanged"), getAll: async (q) => jar.filter((c) => c.domain.replace(/^\./, "").endsWith(q.domain)) },
    action: { setBadgeText: () => {}, setBadgeBackgroundColor: () => {}, setBadgeTextColor: () => {} },
  };
  const ctx = vm.createContext({ chrome, WebSocket: ExtensionSocket, URL, setTimeout, clearTimeout, setInterval, clearInterval, console, JSON, Date, Math });
  ctx.globalThis = ctx;
  ctx.importScripts = (...files) => {
    for (const f of files) vm.runInContext(fs.readFileSync(path.join(extDir, f), "utf8"), ctx, { filename: f });
  };
  vm.runInContext(fs.readFileSync(path.join(extDir, "background.js"), "utf8"), ctx, { filename: "background.js" });
  t.after(() => vm.runInContext("bridge.stop(); tracker.stop(); signinFlow.stop();", ctx));
  await until(() => vm.runInContext("bridge.getState().conn", ctx) === "connected", 5000, "helper connected to Turbine");
  await until(() => said(/^BRIDGE connected/), 3000, "Turbine heard the helper connect");

  // The player presses Sign in with my browser in Turbine (Game Mode).
  turbine.stdin.write("signin\n");
  await until(() => [...tabs.values()].some((tb) => tb.url === pageUrl), 5000, "Turbine's sign-in page opened in the browser");
  const tabId = [...tabs.values()].find((tb) => tb.url === pageUrl).id;

  // The page connects and shows that Turbine waits.
  const shown = [];
  fire("connect", { name: "turbine-signin", sender: { id: EXT_ID, url: pageUrl, tab: { id: tabId } }, postMessage: (m) => shown.push(m), disconnect: () => {}, onDisconnect: on("pageGone"), onMessage: on("pageMsg") });
  fire("pageMsg", { kind: "hello" });
  await until(() => shown.some((m) => m.signin && m.signin.phase === "waiting" && /Turbine is waiting for you to sign in to Nexus Mods/.test(m.signin.text)), 3000, "waiting shown");
  await until(() => said(/^STATUS The browser shows Turbine's sign-in page/), 3000, "Turbine's log says the page is up");
  assert.ok(!said(/^TOOK/), "nothing taken while signed out");

  // Sign in to Nexus Mods: the tab goes to Nexus; the mocked Nexus signs in (its cookie appears).
  fire("pageMsg", { kind: "signin.go" });
  await until(() => tabs.get(tabId).url === P.SIGNIN_URL, 3000, "tab on Nexus's sign-in page");
  const cookie = { name: "nexusmods_session", value: COOKIE_VALUE, domain: ".nexusmods.com", path: "/", secure: true, httpOnly: true, hostOnly: false, sameSite: "lax", expirationDate: 1893456000 };
  jar.push(cookie, { name: "SID", value: "googleValue", domain: ".google.com", path: "/" }, { name: "cf_clearance", value: "cfValue1", domain: ".nexusmods.com", path: "/" }, { name: "__cf_bm", value: "cfValue2", domain: ".nexusmods.com", path: "/" });
  fire("cookieChanged", { removed: false, cookie });
  await until(() => said(/^TOOK nexusmods_session$/), 5000, "Turbine took Nexus's cookie (and only Nexus's)");
  await until(() => shown.some((m) => m.signin && m.signin.phase === "ok"), 3000, "Signed in shown");
  const okView = shown.filter((m) => m.signin && m.signin.phase === "ok").pop().signin;
  assert.equal(okView.text, "Signed in. Go back to Turbine.");
  assert.match(okView.back, /Steam button/);
  assert.equal(tabs.get(tabId).url, pageUrl, "the tab came back to Turbine's page");

  // Give Turbine my API key too: the tab goes to the API Keys page; the content script there finds the key.
  fire("pageMsg", { kind: "key.start" });
  await until(() => P.isKeyPage(tabs.get(tabId).url), 3000, "tab on the API Keys page");
  const found = K.findPersonalKey(mockedKeysPage());
  assert.equal(found.key, KEY, "the Personal API Key, not the other app's");
  const panelMsgs = [];
  fire("connect", { name: "turbine-panel", sender: { url: tabs.get(tabId).url, tab: { id: tabId } }, postMessage: (m) => panelMsgs.push(m), disconnect: () => {}, onDisconnect: on("panelGone"), onMessage: on("panelMsg") });
  fire("panelMsg", { kind: "hello" });
  await until(() => panelMsgs.some((m) => m.signin && m.signin.phase === "key-reading"), 3000, "the panel there knows the key was asked for");
  fire("panelMsg", { kind: "key.found", key: found.key });
  await until(() => said(/^KEY accepted$/), 5000, "Turbine checked and kept the key");
  await until(() => shown.some((m) => m.signin && m.signin.phase === "key-ok"), 3000, "page says Turbine has the key");
  assert.equal(tabs.get(tabId).url, pageUrl);

  if (process.env.HANDSHAKE_SHOW) console.log(lines.filter((l) => !l.startsWith("READY ")).join("\n"));
  // Nothing secret was ever printed by Turbine's side, nor kept by the helper.
  await sleep(200);
  const all = lines.filter((l) => !l.startsWith("READY ")).join("\n");
  for (const secret of [KEY, COOKIE_VALUE, "googleValue", token]) {
    assert.ok(!all.includes(secret), "never printed: " + secret.slice(0, 6));
  }
  assert.ok(!JSON.stringify(session).includes(KEY) && !JSON.stringify(session).includes(COOKIE_VALUE), "the helper kept no secret");
  assert.ok(said(/^STATUS The helper sent the Personal API Key \(not logged\)/));
});
