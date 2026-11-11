// Tests for the extension's plain logic and the hello/token flow against a fake Turbine.
// Run: cd extension && node test/test.js   (Node 22 or newer, nothing to install)
"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("fs");
const path = require("path");
const vm = require("vm");

const P = require("../lib/protocol.js");
const C = require("../lib/choices.js");
const { createBridge } = require("../lib/bridge.js");
const { createDownloadTracker } = require("../lib/downloads.js");
const S = require("../lib/save.js");
const { startFakeTurbine } = require("./fake-turbine.js");
const { sleep, until, runWorker } = require("./helpers.js");

const TOKEN = "a".repeat(32) + "0123456789abcdef0123456789abcdef";

async function fakeOnFreePort(opts) {
  for (let port = P.PORT_MIN; port <= P.PORT_MAX; port++) {
    try {
      return await new Promise((resolve, reject) => {
        startFakeTurbine(Object.assign({ port }, opts)).then(resolve, reject);
        setTimeout(() => reject(new Error("slow")), 1000);
      });
    } catch (e) {
      /* port busy, try the next */
    }
  }
  throw new Error("no free port in the bridge range");
}

// ---------- URLs ----------

test("Nexus mod page URLs give the game domain, mod id and file id", () => {
  assert.deepEqual(P.parseNexusUrl("https://www.nexusmods.com/skyrimspecialedition/mods/266"), {
    domain: "skyrimspecialedition",
    modId: 266,
    fileId: null,
  });
  assert.deepEqual(P.parseNexusUrl("https://www.nexusmods.com/stardewvalley/mods/2400?tab=files&file_id=119630&nmm=1"), {
    domain: "stardewvalley",
    modId: 2400,
    fileId: 119630,
  });
  assert.deepEqual(P.parseNexusUrl("https://nexusmods.com/games/valheim/mods/387?tab=description"), {
    domain: "valheim",
    modId: 387,
    fileId: null,
  });
  assert.deepEqual(P.parseNexusUrl("https://www.nexusmods.com/games/riskofrain2"), { domain: "riskofrain2", modId: null, fileId: null });
  assert.deepEqual(P.parseNexusUrl("https://www.nexusmods.com/skyrimspecialedition/mods/266/"), {
    domain: "skyrimspecialedition",
    modId: 266,
    fileId: null,
  });
});

test("Pages that are not a game or mod page give nothing", () => {
  for (const u of [
    "https://www.nexusmods.com/",
    "https://www.nexusmods.com/games",
    "https://www.nexusmods.com/users/123",
    "https://www.nexusmods.com/about",
    "https://evil.example/skyrimspecialedition/mods/266",
    "https://nexusmods.com.evil.example/skyrimspecialedition/mods/266",
    "http://www.nexusmods.com/skyrimspecialedition/mods/266",
    "not a url",
    "",
    null,
  ]) {
    assert.equal(P.parseNexusUrl(u), null, String(u));
  }
  assert.equal(P.parseNexusUrl("https://www.nexusmods.com/skyrimspecialedition/mods/abc").modId, null);
  assert.equal(P.parseNexusUrl("https://www.nexusmods.com/skyrimspecialedition/mods/0").modId, null);
});

test("Nexus downloads are recognised by URL, final URL or referrer", () => {
  assert.ok(P.isNexusDownload({ url: "https://cf-files.nexus-cdn.com/x.zip" }));
  assert.ok(P.isNexusDownload({ url: "https://supporter-files.nexus-cdn.com/1/2/x.7z" }));
  assert.ok(P.isNexusDownload({ url: "https://files.example/x.zip", referrer: "https://www.nexusmods.com/valheim/mods/1?tab=files" }));
  assert.ok(P.isNexusDownload({ url: "https://a.example/redirect", finalUrl: "https://cf-files.nexusmods.com/x.zip" }));
  assert.ok(!P.isNexusDownload({ url: "https://example.com/x.zip", referrer: "https://example.com/" }));
  assert.ok(!P.isNexusDownload({ url: "https://nexus-cdn.com.evil.example/x.zip" }));
  assert.ok(!P.isNexusDownload({ url: "https://fakenexusmods.com/x.zip" }));
  assert.ok(!P.isNexusDownload({ url: "blob:https://www.nexusmods.com/123" }));
  assert.ok(!P.isNexusDownload(null));
});

test("nxm links and the pairing page URL are checked", () => {
  assert.ok(P.isNxmUrl("nxm://skyrimspecialedition/mods/266/files/1000?key=abc&expires=123&user_id=9"));
  assert.ok(P.isNxmUrl("NXM://stardewvalley/mods/2400/files/119630"));
  assert.ok(!P.isNxmUrl("https://www.nexusmods.com/"));
  assert.ok(!P.isNxmUrl("nxm://"));
  assert.ok(!P.isNxmUrl("nxm://game/mods 1"));
  assert.ok(!P.isNxmUrl("nxm://" + "a".repeat(3000)));

  assert.deepEqual(P.parsePairingUrl("http://127.0.0.1:47701/pair#" + TOKEN), { port: 47701, token: TOKEN });
  assert.deepEqual(P.parsePairingUrl("http://127.0.0.1:47710/pair?x=1#" + TOKEN.toUpperCase()), { port: 47710, token: TOKEN });
  assert.equal(P.parsePairingUrl("http://127.0.0.1:8080/pair#" + TOKEN), null, "port outside the range");
  assert.equal(P.parsePairingUrl("http://localhost:47701/pair#" + TOKEN), null, "only 127.0.0.1");
  assert.equal(P.parsePairingUrl("https://127.0.0.1:47701/pair#" + TOKEN), null);
  assert.equal(P.parsePairingUrl("http://127.0.0.1:47701/other#" + TOKEN), null);
  assert.equal(P.parsePairingUrl("http://127.0.0.1:47701/pair#short"), null);
  assert.equal(P.parsePairingUrl("http://127.0.0.1:47701/pair"), null);
});

// ---------- Messages ----------

test("Messages from Turbine are checked and cleaned", () => {
  assert.equal(P.validateIncoming("not json"), null);
  assert.equal(P.validateIncoming("[]"), null);
  assert.equal(P.validateIncoming({ type: "something.new" }), null, "unknown types are ignored");
  assert.equal(P.validateIncoming({ type: "status" }), null);

  const w = P.validateIncoming('{"type":"welcome","turbineVersion":"0.3","games":[{"id":489830,"name":"Skyrim Special Edition","domain":"skyrimspecialedition"}]}');
  assert.equal(w.turbineVersion, "0.3");
  assert.deepEqual(w.games[0], { id: "489830", name: "Skyrim Special Edition", domain: "skyrimspecialedition", folder: "" });

  const s = P.validateIncoming({ type: "status", text: "Downloading", level: "nonsense", progress: 7 });
  assert.equal(s.level, "ok");
  assert.equal(s.progress, 1);
  assert.equal(s.game, null);
  assert.equal(P.validateIncoming({ type: "status", text: "x", level: "busy", progress: null }).progress, null);

  const a = P.validateIncoming({
    type: "added",
    game: "Valheim",
    mod: "Better UI",
    notes: ["Needs BepInEx", 5],
    needs: [{ name: "BepInEx", url: "https://www.nexusmods.com/valheim/mods/4" }, { name: "Bad", url: "javascript:alert(1)" }, { url: "x" }],
  });
  assert.deepEqual(a.notes, ["Needs BepInEx"]);
  assert.deepEqual(a.needs, [
    { name: "BepInEx", url: "https://www.nexusmods.com/valheim/mods/4" },
    { name: "Bad", url: null },
  ]);

  const c = P.validateIncoming({
    type: "choices",
    sessionId: "s1",
    modName: "M",
    stepTitle: "Look",
    stepIndex: 9,
    stepCount: 2,
    groups: [
      {
        title: "Body",
        kind: "one",
        options: [
          { id: 1, name: "A", description: "d", image: "data:image/png;base64,iVBORw0KGgo=", state: "selected" },
          { id: "b", name: "B", image: "https://tracker.example/x.png", state: "weird" },
        ],
      },
    ],
  });
  assert.equal(c.stepIndex, 1, "step index is clamped");
  assert.equal(c.groups[0].options[0].id, "1");
  assert.equal(c.groups[0].options[1].image, null, "only data URLs are shown as images");
  assert.equal(c.groups[0].options[1].state, "available");
  assert.equal(P.validateIncoming({ type: "choices", sessionId: "s", groups: [{ options: [{}] }] }), null);
  assert.deepEqual(P.validateIncoming({ type: "choices.done", sessionId: "s1" }), { type: "choices.done", sessionId: "s1" });
  assert.deepEqual(P.validateIncoming({ type: "error", text: "Nope" }), { type: "error", text: "Nope" });
  // Turbine asks a helper older than its own to reload from its folder; extra fields are dropped.
  assert.deepEqual(P.validateIncoming('{"type":"reload"}'), { type: "reload" });
  assert.deepEqual(P.validateIncoming({ type: "reload", url: "https://evil.example/" }), { type: "reload" });
  // "wait": Turbine waits for the player on the page (press Slow download).
  assert.equal(P.validateIncoming({ type: "status", text: "Press Slow download on this page.", level: "wait" }).level, "wait");
});

test("Turbine may only open Nexus pages in a new tab", () => {
  for (const ok of [
    "https://www.nexusmods.com/skyrimspecialedition/mods/12604?tab=files&file_id=1",
    "https://next.nexusmods.com/settings/api-keys",
    "https://www.nexusmods.com/settings/api-keys",
  ]) {
    assert.equal(P.isNexusTabUrl(ok), new URL(ok).href, ok);
    assert.deepEqual(P.validateIncoming({ type: "open", url: ok }), { type: "open", url: new URL(ok).href });
  }
  for (const bad of [
    "http://www.nexusmods.com/",
    "https://nexusmods.com/",
    "https://nexusmods.com.evil.example/",
    "https://evilnexusmods.com/",
    "https://user@www.nexusmods.com/",
    "https://www.nexusmods.com:8443/",
    "https://www.nexusmods.com/ x",
    "https://www.nexusmods.com\\@evil.example/",
    "javascript:alert(1)",
    "nxm://skyrim/mods/1/files/2",
    "chrome://settings",
    "https://www.nexusmods.com/" + "a".repeat(2100),
    "",
    42,
    null,
  ]) {
    assert.equal(P.isNexusTabUrl(bad), null, String(bad));
    assert.equal(P.validateIncoming({ type: "open", url: bad }), null, String(bad));
  }
  assert.equal(P.validateIncoming({ type: "open" }), null);
});

test("Messages to Turbine have the protocol's fields", () => {
  assert.deepEqual(P.build.hello(TOKEN, "0.3"), { type: "hello", token: TOKEN, version: 1, ext: "0.3" });
  assert.deepEqual(P.build.page("https://www.nexusmods.com/valheim/mods/387?tab=files"), {
    type: "page",
    url: "https://www.nexusmods.com/valheim/mods/387?tab=files",
    domain: "valheim",
    modId: 387,
  });
  assert.equal(P.build.page("https://example.com/"), null);
  assert.deepEqual(P.build.choice("s", 0, ["a", "b"]), { type: "choice", sessionId: "s", step: 0, selected: ["a", "b"] });
  assert.deepEqual(P.build.choiceBack("s"), { type: "choice.back", sessionId: "s" });
  assert.deepEqual(P.build.choiceCancel("s"), { type: "choice.cancel", sessionId: "s" });
  assert.deepEqual(P.build.downloadFinished(3, "/home/deck/Downloads/x.zip", 10), {
    type: "download.finished",
    id: 3,
    path: "/home/deck/Downloads/x.zip",
    size: 10,
  });
  assert.deepEqual(Object.keys(P.build.downloadStarted({ id: 1 })), ["type", "id", "url", "finalUrl", "filename", "referrer", "mime", "totalBytes"]);
});

// ---------- Installer choices ----------

function choicesMsg(groups, stepIndex, stepCount) {
  return P.validateIncoming({ type: "choices", sessionId: "s", modName: "M", stepTitle: "T", stepIndex: stepIndex || 0, stepCount: stepCount || 1, groups });
}
const opt = (id, state) => ({ id, name: id, state: state || "available" });

test("Pick one: starts on the first usable option and behaves like radio buttons", () => {
  const st = C.create(choicesMsg([{ title: "G", kind: "one", options: [opt("a", "disabled"), opt("b"), opt("c")] }]));
  assert.deepEqual(C.selectedIds(st), ["b"]);
  assert.ok(C.isRadio(st, 0));
  assert.ok(!C.toggle(st, 0, "a"), "disabled cannot be picked");
  assert.ok(!C.toggle(st, 0, "b"), "tapping the picked one keeps it");
  assert.ok(C.toggle(st, 0, "c"));
  assert.deepEqual(C.selectedIds(st), ["c"]);
  assert.ok(C.isValid(st));
});

test("Pick one keeps a preselected option and a required one is locked", () => {
  let st = C.create(choicesMsg([{ kind: "one", options: [opt("a"), opt("b", "selected")] }]));
  assert.deepEqual(C.selectedIds(st), ["b"]);
  st = C.create(choicesMsg([{ kind: "one", options: [opt("a", "selected"), opt("b", "required")] }]));
  assert.deepEqual(C.selectedIds(st), ["b"]);
  assert.ok(!C.toggle(st, 0, "a"));
});

test("Pick one or none can be cleared", () => {
  const st = C.create(choicesMsg([{ kind: "atmostone", options: [opt("a"), opt("b")] }]));
  assert.deepEqual(C.selectedIds(st), []);
  assert.ok(C.isValid(st));
  C.toggle(st, 0, "a");
  C.toggle(st, 0, "b");
  assert.deepEqual(C.selectedIds(st), ["b"]);
  C.toggle(st, 0, "b");
  assert.deepEqual(C.selectedIds(st), []);
});

test("Pick one or more needs at least one before Next", () => {
  const st = C.create(choicesMsg([{ title: "Extras", kind: "atleastone", options: [opt("a"), opt("b")] }]));
  assert.ok(!C.isValid(st));
  assert.equal(C.problems(st)[0].text, "Pick at least one option in Extras");
  C.toggle(st, 0, "a");
  C.toggle(st, 0, "b");
  assert.ok(C.isValid(st));
  assert.deepEqual(C.selectedIds(st), ["a", "b"]);
  C.toggle(st, 0, "a");
  C.toggle(st, 0, "b");
  assert.ok(!C.isValid(st));
});

test("Pick any, all, required and disabled", () => {
  const st = C.create(
    choicesMsg([
      { kind: "any", options: [opt("a", "required"), opt("b", "selected"), opt("c"), opt("d", "disabled")] },
      { kind: "all", options: [opt("e"), opt("f")] },
    ])
  );
  assert.deepEqual(C.selectedIds(st), ["a", "b", "e", "f"]);
  assert.ok(!C.toggle(st, 0, "a"), "required stays on");
  assert.ok(!C.toggle(st, 0, "d"), "disabled stays off");
  assert.ok(!C.toggle(st, 1, "e"), "all stays on");
  assert.ok(C.toggle(st, 0, "b"));
  assert.ok(C.toggle(st, 0, "c"));
  assert.deepEqual(C.selectedIds(st), ["a", "c", "e", "f"]);
  assert.ok(!C.isRadio(st, 0));
});

test("Steps: Back only after the first step, Install on the last", () => {
  const g = [{ kind: "any", options: [opt("a")] }];
  let st = C.create(choicesMsg(g, 0, 3));
  assert.ok(!C.canGoBack(st));
  assert.ok(!C.isLastStep(st));
  st = C.create(choicesMsg(g, 2, 3));
  assert.ok(C.canGoBack(st));
  assert.ok(C.isLastStep(st));
});

// ---------- Downloads ----------

function fakeTimers() {
  const list = [];
  let n = 0;
  return {
    list,
    setTimeout: (fn, ms) => (list.push({ id: ++n, fn, ms, every: false }), n),
    clearTimeout: (id) => list.splice(0, list.length, ...list.filter((x) => x.id !== id)),
    setInterval: (fn, ms) => (list.push({ id: ++n, fn, ms, every: true }), n),
    clearInterval: (id) => list.splice(0, list.length, ...list.filter((x) => x.id !== id)),
    async runOnce(every) {
      for (const x of list.filter((t) => t.every === every)) {
        if (!every) list.splice(list.indexOf(x), 1);
        await x.fn();
      }
    },
  };
}

test("A Nexus download is reported as started, progress and finished with the absolute path", async () => {
  const items = new Map();
  const sent = [];
  const timers = fakeTimers();
  const tr = createDownloadTracker({ search: async (q) => (items.has(q.id) ? [items.get(q.id)] : []), send: (m) => sent.push(m), timers });

  const item = {
    id: 7,
    url: "https://cf-files.nexus-cdn.com/files/x.zip",
    finalUrl: "https://cf-files.nexus-cdn.com/files/x.zip",
    referrer: "https://www.nexusmods.com/valheim/mods/387?tab=files&file_id=5",
    filename: "",
    mime: "application/zip",
    totalBytes: 100,
    bytesReceived: 0,
    state: "in_progress",
  };
  items.set(7, item);
  assert.ok(tr.onCreated(item));
  assert.equal(sent.length, 0, "waits for the real file name");
  item.filename = "/home/deck/Downloads/x.zip";
  await tr.onChanged({ id: 7, filename: { current: item.filename } });
  assert.equal(sent[0].type, "download.started");
  assert.equal(sent[0].filename, "/home/deck/Downloads/x.zip");
  assert.equal(sent[0].totalBytes, 100);

  item.bytesReceived = 40;
  await tr.poll();
  await tr.poll();
  assert.deepEqual(sent[1], { type: "download.progress", id: 7, receivedBytes: 40, totalBytes: 100 });
  assert.equal(sent.length, 2, "no repeat when nothing moved");

  item.bytesReceived = 100;
  item.fileSize = 100;
  item.state = "complete";
  await tr.onChanged({ id: 7, state: { current: "complete" } });
  assert.deepEqual(sent[2], { type: "download.finished", id: 7, path: "/home/deck/Downloads/x.zip", size: 100 });
  assert.deepEqual(tr.activeIds(), []);
  assert.equal(timers.list.length, 0, "polling stops when nothing is left");
});

test("A download whose name never arrives is still announced; failures and other sites", async () => {
  const items = new Map();
  const sent = [];
  const timers = fakeTimers();
  const tr = createDownloadTracker({ search: async (q) => (items.has(q.id) ? [items.get(q.id)] : []), send: (m) => sent.push(m), timers });

  assert.ok(!tr.onCreated({ id: 1, url: "https://example.com/a.zip", referrer: "https://example.com/" }));
  await tr.onChanged({ id: 1, state: { current: "complete" } });
  assert.equal(sent.length, 0, "other sites are ignored");

  const item = { id: 2, url: "https://supporter-files.nexus-cdn.com/a.7z", filename: "", state: "in_progress" };
  items.set(2, item);
  tr.onCreated(item);
  await timers.runOnce(false);
  assert.equal(sent[0].type, "download.started");
  await tr.onChanged({ id: 2, state: { current: "interrupted" }, error: { current: "NETWORK_FAILED" } });
  assert.deepEqual(sent[1], { type: "download.failed", id: 2, error: "NETWORK_FAILED" });

  const saved = [];
  const tr2 = createDownloadTracker({ search: async () => [], send: (m) => sent.push(m), timers: fakeTimers(), save: (l) => saved.push(l) });
  tr2.restore([{ id: 9, announced: true }]);
  assert.deepEqual(tr2.activeIds(), [9], "survives a service worker restart");
  tr2.onErased(9);
  assert.deepEqual(sent[2], { type: "download.failed", id: 9, error: "USER_ERASED" });
});

// ---------- Bridge, end to end against a fake Turbine ----------

function bridgeFor(port, token, extra) {
  const states = [];
  const msgs = [];
  const b = createBridge(
    Object.assign(
      {
        WebSocket: globalThis.WebSocket,
        extVersion: "0.3",
        loadPairing: async () => (port ? { port, token } : null),
        onState: (s) => states.push(s.conn),
        onMessage: (m) => msgs.push(m),
        backoffMs: [50, 100],
      },
      extra || {}
    )
  );
  return { b, states, msgs };
}

test("Hello with the right token gets welcome, then messages flow both ways", async (t) => {
  const fake = await fakeOnFreePort({ token: TOKEN, games: [{ id: 892970, name: "Valheim", domain: "valheim" }] });
  t.after(() => fake.close());
  const { b, msgs } = bridgeFor(fake.port, TOKEN, { pingMs: 100 });
  t.after(() => b.stop());
  assert.equal(b.send({ type: "ping" }), false, "nothing is sent before connecting");
  b.start();
  await until(() => b.getState().conn === "connected", 3000, "connected");
  const hello = fake.received[0];
  assert.deepEqual(hello, { type: "hello", token: TOKEN, version: 1, ext: "0.3" });
  assert.equal(b.getState().welcome.games[0].domain, "valheim");

  assert.ok(b.send(P.build.nxm("nxm://valheim/mods/387/files/5")));
  await fake.waitFor((m) => m.type === "nxm");
  await fake.waitFor((m) => m.type === "ping", 2000);

  fake.send({ type: "status", text: "Downloading", level: "busy", progress: 0.5, game: "Valheim" });
  fake.send({ type: "mystery" });
  await until(() => msgs.some((m) => m.type === "status"), 2000, "status");
  assert.equal(msgs.filter((m) => m.type === "mystery").length, 0);
});

test("A wrong token is refused and the bridge says so", async (t) => {
  const fake = await fakeOnFreePort({ token: TOKEN });
  t.after(() => fake.close());
  const { b } = bridgeFor(fake.port, "b".repeat(64), { backoffMs: [5000] });
  t.after(() => b.stop());
  b.start();
  await until(() => b.getState().conn === "refused", 3000, "refused");
  assert.equal(fake.rejected, 1);
  assert.equal(b.getState().error, "This browser is not paired.");
  assert.equal(fake.received.length, 0);
});

test("No pairing yet means no connection attempt", async () => {
  const { b } = bridgeFor(null, null);
  b.start();
  await until(() => b.getState().conn === "unpaired", 1000, "unpaired");
  b.stop();
});

test("Reconnects after Turbine goes away and comes back", async (t) => {
  const fake = await fakeOnFreePort({ token: TOKEN });
  const port = fake.port;
  const { b, states } = bridgeFor(port, TOKEN);
  t.after(() => b.stop());
  b.start();
  await until(() => b.getState().conn === "connected", 3000, "first connect");
  await fake.close();
  await until(() => b.getState().conn === "offline", 3000, "offline");
  const again = await startFakeTurbine({ port, token: TOKEN });
  t.after(() => again.close());
  await until(() => b.getState().conn === "connected", 3000, "reconnected");
  assert.equal(again.received[0].type, "hello");
  assert.ok(states.includes("offline"));
});

// ---------- Sign-in pass-through ----------

const SID = "0123456789abcdef0123456789abcdef";

test("Only Nexus's own cookies pass the domain filter", () => {
  for (const ok of [".nexusmods.com", "nexusmods.com", "users.nexusmods.com", ".WWW.NexusMods.com"]) assert.ok(P.isNexusCookieDomain(ok), ok);
  for (const bad of ["evilnexusmods.com", "nexusmods.com.evil.example", ".google.com", "accounts.google.com", "nexusmods.co", "", ".", "a..nexusmods.com", "nexus mods.com", null, 5])
    assert.ok(!P.isNexusCookieDomain(bad), String(bad));
  const list = [
    { name: "nexusmods_session", value: "v1", domain: ".nexusmods.com", path: "/", secure: true, httpOnly: true, hostOnly: false, sameSite: "lax", expirationDate: 1893456000.5, storeId: "0", session: false },
    { name: "SID", value: "google", domain: ".google.com", path: "/", secure: true },
    { name: "x", value: "y", domain: "nexusmods.com.evil.example" },
    { name: "u", value: "w", domain: "users.nexusmods.com", hostOnly: true, sameSite: "weird" },
    { name: 7, value: "bad", domain: ".nexusmods.com" },
  ];
  const got = P.nexusCookies(list);
  assert.deepEqual(got.map((c) => c.name), ["nexusmods_session", "u"]);
  assert.deepEqual(Object.keys(got[0]).sort(), ["domain", "expirationDate", "hostOnly", "httpOnly", "name", "path", "sameSite", "secure", "value"]);
  assert.equal(got[1].sameSite, "unspecified");
  assert.equal(got[1].expirationDate, null);
  assert.equal(P.nexusCookies(Array(200).fill(list[0])).length, 80);
  assert.deepEqual(P.nexusCookies("nope"), []);
});

test("Sign-in messages are checked both ways", () => {
  assert.deepEqual(P.validateIncoming({ type: "signin.request", id: SID, url: "https://users.nexusmods.com/auth/sign_in" }), { type: "signin.request", id: SID, url: "https://users.nexusmods.com/auth/sign_in", steam: false });
  // A page that isn't Nexus's: the sign-in page instead.
  assert.equal(P.validateIncoming({ type: "signin.request", id: SID, url: "https://evil.example/" }).url, P.SIGNIN_URL);
  assert.equal(P.validateIncoming({ type: "signin.request", id: "short" }), null);
  assert.equal(P.validateIncoming({ type: "signin.request", id: SID.toUpperCase() }), null);
  assert.deepEqual(P.validateIncoming({ type: "signin.done", id: SID, ok: true, text: "Done" }), { type: "signin.done", id: SID, ok: true, text: "Done" });
  assert.equal(P.validateIncoming({ type: "signin.done", id: "x", ok: true }), null);
  assert.equal(P.build.signinCookies("x", []), null, "no request, nothing to send");
  const m = P.build.signinCookies(SID, [{ name: "a", value: "b", domain: ".nexusmods.com" }, { name: "c", value: "d", domain: ".google.com" }]);
  assert.equal(m.type, "signin.cookies");
  assert.deepEqual(m.cookies.map((c) => c.name), ["a"]);
  assert.equal(P.validateIncoming({ type: "status", text: "x", level: "hint" }).level, "hint");
});

test("background.js wires downloads, the panel and pairing to the bridge", async (t) => {
  const fake = await fakeOnFreePort({ token: TOKEN });
  t.after(() => fake.close());
  const dir = path.join(__dirname, "..");
  const local = { turbinePort: fake.port, turbineToken: TOKEN };
  const session = {};
  const listeners = {};
  const on = (name) => ({ addListener: (fn) => ((listeners[name] = listeners[name] || []).push(fn)) });
  const fire = (name, ...args) => (listeners[name] || []).map((fn) => fn(...args));
  const items = new Map();
  const badge = [];
  const restarted = [];
  const cancelled = [];
  const tabsMade = [];
  const windowsFocused = [];
  let reloads = 0;
  const cookieQueries = [];
  // What chrome.cookies.getAll would find for each domain (the fake filters like Chrome does).
  const jar = [
    { name: "nexusmods_session", value: "s3cr3t", domain: ".nexusmods.com", path: "/", secure: true, httpOnly: true, hostOnly: false, sameSite: "lax", expirationDate: 1893456000 },
    { name: "SID", value: "google", domain: ".google.com", path: "/", secure: true },
  ];
  const pick = (store, keys) => Object.fromEntries((Array.isArray(keys) ? keys : [keys]).filter((k) => k in store).map((k) => [k, store[k]]));
  const tabUrls = new Map();
  const tabUpdates = [];
  const moved = [];
  const chrome = {
    runtime: {
      id: "ourid",
      getURL: (p) => "chrome-extension://ourid/" + p,
      getManifest: () => JSON.parse(fs.readFileSync(path.join(dir, "manifest.json"), "utf8")),
      reload: () => reloads++,
      onConnect: on("connect"),
      onMessage: on("message"),
      onStartup: on("startup"),
      onInstalled: on("installed"),
    },
    storage: {
      local: { get: async (k) => pick(local, k), set: async (o) => Object.assign(local, o) },
      session: { get: async (k) => pick(session, k), set: async (o) => Object.assign(session, o) },
    },
    downloads: {
      onCreated: on("dlCreated"),
      onChanged: on("dlChanged"),
      onErased: on("dlErased"),
      onDeterminingFilename: on("dlName"),
      search: async (q) => (items.has(q.id) ? [items.get(q.id)] : []),
      download: async (o) => {
        restarted.push(o);
        return 99;
      },
      cancel: async (id) => cancelled.push(id),
      erase: async (q) => cancelled.push(q.id),
    },
    tabs: {
      create: async (o) => {
        tabsMade.push(o);
        const id = 5 + tabsMade.length - 1;
        tabUrls.set(id, o.url);
        return { id, windowId: 3 };
      },
      get: async (id) => {
        if (!tabUrls.has(id)) throw new Error("no tab");
        return { id, windowId: 3, url: tabUrls.get(id) };
      },
      update: async (id, o) => {
        tabUpdates.push([id, o]);
        if (o.url) tabUrls.set(id, o.url);
        return { id, windowId: 3 };
      },
      move: async (id, o) => moved.push([id, o]),
      onRemoved: on("tabRemoved"),
    },
    windows: {
      update: async (id, o) => windowsFocused.push([id, o]),
      create: async () => {},
      onCreated: on("windowCreated"),
    },
    cookies: {
      onChanged: on("cookieChanged"),
      getAll: async (q) => {
        cookieQueries.push(q);
        return jar.filter((c) => c.domain === q.domain || c.domain.endsWith("." + q.domain) || c.domain === "." + q.domain);
      },
    },
    action: {
      setBadgeText: (o) => badge.push(o.text),
      setBadgeBackgroundColor: () => {},
      setBadgeTextColor: () => {},
    },
  };
  const ctx = runWorker(chrome, globalThis.WebSocket);
  t.after(() => vm.runInContext("bridge.stop(); tracker.stop();", ctx));

  await fake.waitFor((m) => m.type === "hello");
  await until(() => vm.runInContext("bridge.getState().conn", ctx) === "connected", 3000, "worker connected");
  assert.equal(fake.received[0].ext, JSON.parse(fs.readFileSync(path.join(dir, "manifest.json"), "utf8")).version);

  // A panel on a Nexus page reports the page and an nxm link.
  const panelMsgs = [];
  const panel = {
    name: "turbine-panel",
    sender: { url: "https://www.nexusmods.com/valheim/mods/387" },
    postMessage: (m) => panelMsgs.push(m),
    disconnect: () => {},
    onDisconnect: on("panelGone"),
    onMessage: on("panelMsg"),
  };
  fire("connect", panel);
  fire("panelMsg", { kind: "page", url: "https://www.nexusmods.com/valheim/mods/387?tab=files" });
  fire("panelMsg", { kind: "nxm", url: "nxm://valheim/mods/387/files/5?key=k" });
  fire("panelMsg", { kind: "nxm", url: "javascript:alert(1)" });
  const page = await fake.waitFor((m) => m.type === "page");
  assert.deepEqual(page, { type: "page", url: "https://www.nexusmods.com/valheim/mods/387?tab=files", domain: "valheim", modId: 387 });
  await fake.waitFor((m) => m.type === "nxm");
  assert.equal(fake.received.filter((m) => m.type === "nxm").length, 1);

  // A Slow download from Nexus is started again into Game Mods with no Save box; the browser's copy goes.
  const kept = fire("dlName", { id: 41, url: "https://cf-files.nexus-cdn.com/1/387/Mod-387-1-0-1699999999.zip", referrer: "https://www.nexusmods.com/valheim/mods/387", filename: "" }, () => {});
  assert.deepEqual(kept, [true]);
  await until(() => cancelled.includes(41), 2000, "browser copy cancelled");
  assert.equal(restarted[0].saveAs, false);
  assert.ok(restarted[0].filename.startsWith("Game Mods/"), restarted[0].filename);
  assert.deepEqual(fire("dlName", { id: 42, url: "https://example.com/x.zip", referrer: "https://example.com/", filename: "x.zip" }, () => {}), [false]);

  // A panel from another site is turned away.
  let kicked = false;
  fire("connect", { name: "turbine-panel", sender: { url: "https://evil.example/" }, disconnect: () => (kicked = true), onDisconnect: on("x"), onMessage: on("y"), postMessage: () => {} });
  assert.ok(kicked);

  // Status reaches the panel and the badge.
  fake.send({ type: "status", text: "Downloading", level: "busy", progress: 0.42, game: "Valheim" });
  await until(() => panelMsgs.some((m) => m.status && m.status.text === "Downloading"), 2000, "status to panel");
  assert.ok(badge.includes("42%"));

  // Choices go out and back.
  fake.send({ type: "choices", sessionId: "s9", modName: "M", stepTitle: "T", stepIndex: 0, stepCount: 1, groups: [{ title: "G", kind: "one", options: [{ id: "x", name: "X", state: "selected" }] }] });
  await until(() => panelMsgs.some((m) => m.choices && m.choices.sessionId === "s9"), 2000, "choices to panel");
  fire("panelMsg", { kind: "choice", sessionId: "s9", step: 0, selected: ["x"] });
  assert.deepEqual(await fake.waitFor((m) => m.type === "choice"), { type: "choice", sessionId: "s9", step: 0, selected: ["x"] });
  fake.send({ type: "choices.done", sessionId: "s9" });
  await until(() => panelMsgs.length && panelMsgs[panelMsgs.length - 1].choices === null, 2000, "choices cleared");

  // Turbine asks for a Nexus page (Game Mode): it opens in a new tab in front. Anything else is ignored.
  fake.send({ type: "open", url: "https://example.com/" });
  fake.send({ type: "open", url: "https://www.nexusmods.com/valheim/mods/387?tab=files&file_id=5" });
  await until(() => tabsMade.length === 1, 2000, "tab opened");
  assert.equal(JSON.stringify(tabsMade[0]), JSON.stringify({ url: "https://www.nexusmods.com/valheim/mods/387?tab=files&file_id=5", active: true }));
  await until(() => windowsFocused.length === 1, 2000, "window focused");
  assert.equal(JSON.stringify(windowsFocused[0]), JSON.stringify([3, { focused: true }]));
  await sleep(50);
  assert.equal(tabsMade.length, 1, "the other site was not opened");

  // Turbine waits for the player to press Slow download: the panel hears it and the badge says so.
  fake.send({ type: "status", text: "Press Slow download on this page. Turbine adds the mod when it finishes.", level: "wait", progress: null, game: "Valheim" });
  await until(() => panelMsgs.some((m) => m.status && m.status.level === "wait"), 2000, "wait status to panel");
  assert.equal(badge[badge.length - 1], "!");

  // Turbine put a newer helper in this one's folder: it reloads itself from there.
  assert.equal(reloads, 0);
  fake.send({ type: "reload" });
  await until(() => reloads === 1, 2000, "reloaded");

  // A finished Nexus download goes out with its absolute path.
  const item = { id: 11, url: "https://cf-files.nexus-cdn.com/a.zip", referrer: "https://www.nexusmods.com/valheim/mods/387", filename: "/home/deck/Downloads/a.zip", totalBytes: 5, bytesReceived: 5, fileSize: 5, state: "complete" };
  items.set(11, item);
  fire("dlCreated", item);
  await Promise.all(fire("dlChanged", { id: 11, state: { current: "complete" } }));
  assert.deepEqual(await fake.waitFor((m) => m.type === "download.finished"), { type: "download.finished", id: 11, path: "/home/deck/Downloads/a.zip", size: 5 });

  // Sign-in pass-through: nothing is sent before Turbine asks.
  fire("panelMsg", { kind: "where", url: "https://www.nexusmods.com/" });
  fire("windowCreated", { id: 4, type: "normal" });
  fire("panelMsg", { kind: "signin.send" });
  await sleep(100);
  assert.equal(fake.received.filter((m) => m.type === "signin.cookies").length, 0, "no cookies without a request");
  assert.equal(cookieQueries.length, 0, "cookies aren't even read without a request");
  // Turbine asks (Game Mode: it also asks for Nexus's sign-in page, which Turbine's own sign-in page takes over).
  const before = tabsMade.length;
  fake.send({ type: "open", url: "https://users.nexusmods.com/auth/sign_in" });
  fake.send({ type: "signin.request", id: SID, url: "https://users.nexusmods.com/auth/sign_in", steam: true });
  await until(() => tabsMade.length === before + 1, 2000, "Turbine's sign-in page opened");
  assert.equal(JSON.stringify(tabsMade[before]), JSON.stringify({ url: "chrome-extension://ourid/signin/signin.html", active: true }));
  const pageTab = 5 + before;
  // This browser is already signed in to Nexus: the cookies go at once, Nexus's only.
  const sent = await fake.waitFor((m) => m.type === "signin.cookies");
  assert.equal(sent.id, SID);
  assert.deepEqual(sent.cookies.map((c) => c.domain), [".nexusmods.com"]);
  assert.equal(cookieQueries[0].domain, "nexusmods.com");
  await until(() => panelMsgs.some((m) => m.signin && m.signin.pending), 2000, "sign-in shown on the panel");
  await sleep(1700);
  assert.equal(tabsMade.length, before + 1, "Nexus's sign-in page didn't open in a second tab");
  // Turbine's sign-in page connects (only from this helper's own page) and tells Turbine it is up.
  const pageMsgs = [];
  let refused = false;
  fire("connect", { name: "turbine-signin", sender: { id: "otherid", url: "chrome-extension://otherid/signin/signin.html" }, disconnect: () => (refused = true), onDisconnect: on("x"), onMessage: on("y"), postMessage: () => {} });
  assert.ok(refused, "another extension's page is turned away");
  fire("connect", { name: "turbine-signin", sender: { id: "ourid", url: "chrome-extension://ourid/signin/signin.html", tab: { id: pageTab } }, postMessage: (m) => pageMsgs.push(m), disconnect: () => {}, onDisconnect: on("pageGone"), onMessage: on("pageMsg") });
  fire("pageMsg", { kind: "hello" });
  await until(() => pageMsgs.some((m) => m.signin && m.signin.phase === "sent"), 2000, "the page shows the sign-in going");
  // The same cookies aren't sent again by themselves; the panel's button sends them again on purpose.
  fire("panelMsg", { kind: "where", url: "https://www.nexusmods.com/skyrimspecialedition" });
  await sleep(100);
  assert.equal(fake.received.filter((m) => m.type === "signin.cookies").length, 1);
  fire("panelMsg", { kind: "signin.send" });
  await until(() => fake.received.filter((m) => m.type === "signin.cookies").length === 2, 2000, "button sends again");
  // Turbine took it: the request is over, nothing more goes, and the page says Signed in. Go back to Turbine.
  fake.send({ type: "signin.done", id: SID, ok: true, text: "Turbine has your Nexus sign-in. Go back to Turbine." });
  await until(() => panelMsgs.some((m) => m.signin && m.signin.ok), 2000, "signed in shown");
  await until(() => pageMsgs.some((m) => m.signin && m.signin.phase === "ok" && /Steam button/.test(m.signin.back)), 2000, "page says how to go back");
  assert.ok(tabUpdates.some(([id, o]) => id === pageTab && o.url === "chrome-extension://ourid/signin/signin.html"));
  fire("panelMsg", { kind: "signin.send" });
  await sleep(100);
  assert.equal(fake.received.filter((m) => m.type === "signin.cookies").length, 2, "nothing after the request ended");
  // The panel can't hand a key in from a page that isn't the API Keys page.
  fire("pageMsg", { kind: "key.start" });
  await sleep(50);
  fire("panelMsg", { kind: "key.found", key: "QWxhZGRpbjpvcGVuIHNlc2FtZQ==--c2Vzc2lvbktleTEyMw==" });
  await sleep(100);
  assert.equal(fake.received.filter((m) => m.type === "signin.key").length, 0);
  assert.ok(!JSON.stringify(session).includes("QWxhZGRpbjpvcGVuIHNlc2FtZQ"), "no key in session storage");

  // Pairing: only accepted from the pairing page itself, then stored.
  const NEW = "c".repeat(64);
  const replies = [];
  fire("message", { kind: "pair", pageUrl: "http://127.0.0.1:" + fake.port + "/pair#" + NEW }, { url: "https://evil.example/" }, (r) => replies.push(r));
  assert.equal(JSON.stringify(replies[0]), JSON.stringify({ ok: false }));
  assert.equal(local.turbineToken, TOKEN);
  fire("message", { kind: "pair", pageUrl: "http://127.0.0.1:" + fake.port + "/pair#" + NEW }, { url: "http://127.0.0.1:" + fake.port + "/pair" }, (r) => replies.push(r));
  await until(() => replies.length === 2, 2000, "pair reply");
  assert.equal(JSON.stringify(replies[1]), JSON.stringify({ ok: true }));
  assert.equal(local.turbineToken, NEW);
  await until(() => fake.rejected === 1, 3000, "reconnects with the new token (which this fake does not know)");
});

test("Every file loads, the manifest is minimal and the house rules hold", () => {
  const dir = path.join(__dirname, "..");
  const m = JSON.parse(fs.readFileSync(path.join(dir, "manifest.json"), "utf8"));
  assert.equal(m.manifest_version, 3);
  assert.equal(m.name, "Turbine browser helper");
  // cookies: only for the sign-in Turbine asks for, and only nexusmods.com's (host permissions limit it too).
  assert.deepEqual(m.permissions, ["cookies", "downloads", "storage"]);
  assert.deepEqual(m.host_permissions, ["https://*.nexusmods.com/*", "http://127.0.0.1/*"]);
  assert.match(m.version, /^\d+\.\d+$/, "one dot in the version");
  const files = [m.background.service_worker, m.action.default_popup, ...Object.values(m.icons), ...m.content_scripts.flatMap((c) => c.js)];
  for (const f of files) assert.ok(fs.existsSync(path.join(dir, f)), f);

  // The extension ID comes from the public key.
  const crypto = require("crypto");
  const hex = crypto.createHash("sha256").update(Buffer.from(m.key, "base64")).digest("hex").slice(0, 32);
  const id = hex.replace(/[0-9a-f]/g, (c) => String.fromCharCode(97 + parseInt(c, 16)));
  assert.equal(id, "iniglfblobfbekihldmgekchfehncbeb");

  const walk = (d) => fs.readdirSync(d, { withFileTypes: true }).flatMap((e) => (e.isDirectory() ? walk(path.join(d, e.name)) : [path.join(d, e.name)]));
  for (const f of walk(dir).filter((f) => /\.(js|json|html|css|md)$/.test(f))) {
    const text = fs.readFileSync(f, "utf8");
    assert.ok(!text.includes(String.fromCharCode(0x2014)) && !text.includes(String.fromCharCode(0x2013)), "no em or en dash in " + f);
    if (f.endsWith(".js")) new vm.Script(text, { filename: f });
  }
});

// ---------- Saving into Game Mods/<game> ----------

test("Nexus downloads are saved into Game Mods/<game> with the game's folder name", () => {
  const games = [{ id: "steam:292030", name: "The Witcher 3", domain: "witcher3", folder: "The Witcher 3" }];
  const item = {
    id: 7,
    url: "https://cf-files.nexus-cdn.com/952/5/All Quest Objectives On Map-5-1-0-5-1699999999.zip?md5=x&expires=1",
    finalUrl: "https://cf-files.nexus-cdn.com/952/5/All%20Quest%20Objectives%20On%20Map-5-1-0-5-1699999999.zip?md5=x&expires=1",
    referrer: "https://www.nexusmods.com/witcher3/mods/5?tab=files&file_id=12",
    filename: "",
  };
  const p = S.plan(item, games, "ourid");
  assert.equal(p.filename, "Game Mods/The Witcher 3/All Quest Objectives On Map-5-1-0-5-1699999999.zip");
  assert.equal(p.game, "The Witcher 3");
  assert.equal(p.url, item.finalUrl);
  // An unknown game goes to Other.
  const other = S.plan(Object.assign({}, item, { referrer: "https://www.nexusmods.com/baldursgate3/mods/1" }), games, "ourid");
  assert.equal(other.filename, "Game Mods/Other/All Quest Objectives On Map-5-1-0-5-1699999999.zip");
  // Its own downloads, files not from Nexus's servers, and downloads not started from Nexus are left alone.
  assert.equal(S.plan(Object.assign({}, item, { byExtensionId: "ourid" }), games, "ourid"), null);
  assert.equal(S.plan(Object.assign({}, item, { url: "https://example.com/a.zip", finalUrl: "https://example.com/a.zip" }), games, "ourid"), null);
  assert.equal(S.plan(Object.assign({}, item, { referrer: "https://example.com/" }), games, "ourid"), null);
  assert.equal(S.plan(Object.assign({}, item, { finalUrl: "http://cf-files.nexus-cdn.com/a.zip", url: "http://cf-files.nexus-cdn.com/a.zip" }), games, "ourid"), null);
});

test("Game folder names match Turbine's and file names stay inside the folder", () => {
  assert.equal(S.folderName("The Witcher 3: Wild Hunt"), "The Witcher 3 Wild Hunt");
  assert.equal(S.folderName("a/b\\c?"), "abc");
  assert.equal(S.folderName(".."), "Other");
  assert.equal(S.baseName("/home/deck/Downloads/../x.zip"), "x.zip");
  assert.equal(S.baseName("..hidden.zip"), "hidden.zip");
  assert.equal(S.baseName("https://cf-files.nexus-cdn.com/1/2/My%20Mod.7z?x=1"), "My Mod.7z");
});

test("A replaced download is dropped without a word to Turbine", async () => {
  const sent = [];
  const t = createDownloadTracker({ search: async () => [], send: (m) => sent.push(m), pollMs: 5, nameWaitMs: 5 });
  t.onCreated({ id: 3, url: "https://cf-files.nexus-cdn.com/1/2/a.zip", referrer: "https://www.nexusmods.com/witcher3/mods/5" });
  t.forget(3);
  t.onErased(3);
  await sleep(30);
  assert.deepEqual(sent, []);
  t.stop();
});

test("The welcome game list keeps each game's folder", () => {
  const w = P.validateIncoming({ type: "welcome", turbineVersion: "0.5", games: [{ id: "steam:292030", name: "The Witcher 3", domain: "witcher3", folder: "The Witcher 3" }] });
  assert.equal(w.games[0].folder, "The Witcher 3");
});
require("./signin-tests.js");
