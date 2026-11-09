// Tests for the sign-in for Turbine (0.59): the protocol's checks, the Personal API Key finder on mocked API Keys
// pages, the service worker's sign-in flow with a fake browser and clock, and what Turbine's sign-in page shows.
"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");

const P = require("../lib/protocol.js");
const K = require("../lib/keyfind.js");
const S = require("../lib/signin.js");
const Page = require("../signin/signin.js");
const { h, page } = require("./fake-dom.js");

const SID = "0123456789abcdef0123456789abcdef";
const SID2 = "fedcba9876543210fedcba9876543210";
const KEY = "QWxhZGRpbjpvcGVuIHNlc2FtZQ==--c2Vzc2lvbktleTEyMw==--a1b2c3d4e5f6";
const OTHER = "T3RoZXJBcHBLZXkxMjM0NTY3OA==--b3RoZXI5ODc2NTQ=--ffee00112233";
const PAGE_URL = "chrome-extension://ourid/signin/signin.html";

// ---------- Protocol ----------

test("Signed in means one of Nexus's sign-in cookies, on Nexus, with a value", () => {
  assert.equal(P.signedInJar([{ name: "nexusmods_session", value: "x", domain: ".nexusmods.com" }]), true);
  assert.equal(P.signedInJar([{ name: "member_id", value: "7", domain: "users.nexusmods.com" }]), true);
  assert.equal(P.signedInJar([{ name: "nexusmods_session", value: "", domain: ".nexusmods.com" }]), false, "an emptied cookie is signed out");
  assert.equal(P.signedInJar([{ name: "nexusmods_session", value: "x", domain: ".nexusmods.com.evil.example" }]), false);
  assert.equal(P.signedInJar([{ name: "cf_clearance", value: "x", domain: ".nexusmods.com" }]), false, "other Nexus cookies aren't a sign-in");
  assert.equal(P.signedInJar(null), false);
});

test("The sign-in page and the API Keys page are recognised exactly", () => {
  assert.ok(P.isSigninPage("https://users.nexusmods.com/auth/sign_in"));
  assert.ok(P.isSigninPage("https://users.nexusmods.com/auth/sign_in?redirect=x"));
  assert.ok(!P.isSigninPage("https://users.nexusmods.com/account/profile"));
  assert.ok(!P.isSigninPage("http://users.nexusmods.com/auth/sign_in"));
  assert.ok(P.isKeyPage("https://www.nexusmods.com/settings/api-keys"));
  assert.ok(P.isKeyPage("https://next.nexusmods.com/settings/api-keys/"));
  assert.ok(P.isKeyPage("https://www.nexusmods.com/users/myaccount?tab=api%20access"));
  assert.ok(!P.isKeyPage("https://www.nexusmods.com/users/myaccount?tab=profile"));
  assert.ok(!P.isKeyPage("https://evil.example/settings/api-keys"));
  assert.ok(!P.isKeyPage("https://www.nexusmods.com.evil.example/settings/api-keys"));
  assert.ok(!P.isKeyPage("https://users.nexusmods.com/settings/api-keys"));
});

test("Only key-shaped text counts as an API key", () => {
  assert.ok(P.plausibleKey(KEY));
  assert.ok(P.plausibleKey("  " + KEY + "\n"));
  for (const bad of ["", "short1", "Personal API Key", "a".repeat(40), "1".repeat(40), "has space 0123456789abcdef", "<b>0123456789abcdefgh</b>", "a1".repeat(600), null, 5]) {
    assert.equal(P.plausibleKey(bad), false, String(bad));
  }
});

test("Sign-in key and status messages are checked both ways", () => {
  assert.deepEqual(P.validateIncoming({ type: "signin.request", id: SID, steam: true }), { type: "signin.request", id: SID, url: P.SIGNIN_URL, steam: true });
  assert.equal(P.validateIncoming({ type: "signin.request", id: SID, steam: "yes" }).steam, false);
  assert.deepEqual(P.validateIncoming({ type: "signin.keydone", id: SID, ok: true, text: "Turbine has your API key." }), { type: "signin.keydone", id: SID, ok: true, text: "Turbine has your API key." });
  assert.equal(P.validateIncoming({ type: "signin.keydone", id: "x", ok: true }), null);
  assert.deepEqual(P.build.signinStatus(SID, "page"), { type: "signin.status", id: SID, step: "page" });
  assert.equal(P.build.signinStatus(SID, "anything"), null, "only the fixed words");
  assert.equal(P.build.signinStatus("x", "page"), null);
  assert.deepEqual(P.build.signinKey(SID, " " + KEY + " "), { type: "signin.key", id: SID, key: KEY });
  assert.equal(P.build.signinKey(SID, "not a key"), null);
  assert.equal(P.build.signinKey("x", KEY), null);
});

// ---------- The Personal API Key on mocked API Keys pages ----------

// Like Nexus's API Keys page: the apps' keys in their own boxes, the Personal API Key at the bottom.
function keysPage(personal, opts) {
  const o = opts || {};
  const app = (name, key) => h("div", h("h3", name), h("p", "Use this key with " + name + "."), key ? h("input", { value: key }) : h("button", "Request an API key"));
  return page(
    h("header", h("nav", h("a", "Mods"), h("a", "Games"))),
    h(
      "main",
      h("h1", "API Keys"),
      h("section", app("Some app", OTHER), app("Another app", null)),
      h(
        "section",
        h("h2", "Personal API Key"),
        h("p", "Do not share this key."),
        o.asText ? h("div", h("code", personal)) : personal ? h("div", h("textarea", { value: personal })) : h("button", "Request an API key"),
        o.twice ? h("input", { value: OTHER }) : h("span", "")
      )
    ),
    h("script", "window.__STATE__ = {\"token\":\"" + KEY.replace(/=/g, "x") + "0\"}")
  );
}

test("The Personal API Key is found next to its label, never another app's", () => {
  assert.deepEqual(K.findPersonalKey(keysPage(KEY)), { key: KEY });
  assert.deepEqual(K.findPersonalKey(keysPage(KEY, { asText: true })), { key: KEY }, "shown as text");
});

test("No key, no label, or two keys in the box: nothing is taken", () => {
  assert.deepEqual(K.findPersonalKey(keysPage(null)), { why: "nokey" }, "not requested yet; the other app's key isn't taken");
  assert.deepEqual(K.findPersonalKey(page(h("h1", "Sign in"), h("input", { value: KEY }))), { why: "nolabel" }, "not the API Keys page");
  assert.deepEqual(K.findPersonalKey(keysPage(KEY, { twice: true })), { why: "unclear" });
  // A label inside a sentence counts; the label's own words only (not a parent's).
  assert.deepEqual(K.findPersonalKey(page(h("div", h("p", "Your personal API key:"), h("pre", KEY)))), { key: KEY });
});

// ---------- The service worker's sign-in flow ----------

function fakeClock() {
  let t = 1000000;
  let seq = 0;
  const jobs = new Map();
  const timers = {
    setTimeout: (fn, ms) => (jobs.set(++seq, { fn, at: t + ms, every: 0 }), seq),
    clearTimeout: (id) => jobs.delete(id),
    setInterval: (fn, ms) => (jobs.set(++seq, { fn, at: t + ms, every: ms }), seq),
    clearInterval: (id) => jobs.delete(id),
  };
  async function tick(ms) {
    const end = t + ms;
    for (;;) {
      let next = null;
      for (const [id, j] of jobs) if (j.at <= end && (!next || j.at < next[1].at)) next = [id, j];
      if (!next) break;
      const [id, j] = next;
      t = j.at;
      if (j.every) j.at += j.every;
      else jobs.delete(id);
      j.fn();
      await new Promise((r) => setImmediate(r));
    }
    t = end;
    await new Promise((r) => setImmediate(r));
  }
  return { timers, now: () => t, tick };
}

function fakeBrowser() {
  const listeners = {};
  const on = (name) => ({ addListener: (fn) => ((listeners[name] = listeners[name] || []).push(fn)) });
  const b = {
    listeners,
    fire: (name, ...a) => (listeners[name] || []).forEach((fn) => fn(...a)),
    jar: [],
    tabs: new Map(),
    created: [],
    updates: [],
    moves: [],
    focused: [],
    session: {},
    nextTab: 10,
  };
  b.chrome = {
    tabs: {
      create: async (o) => {
        const tab = { id: b.nextTab++, windowId: 1, url: o.url };
        b.tabs.set(tab.id, tab);
        b.created.push(o);
        return tab;
      },
      get: async (id) => {
        if (!b.tabs.has(id)) throw new Error("No tab with id " + id);
        return b.tabs.get(id);
      },
      update: async (id, o) => {
        if (!b.tabs.has(id)) throw new Error("No tab with id " + id);
        b.updates.push([id, o]);
        if (o.url) b.tabs.get(id).url = o.url;
        return b.tabs.get(id);
      },
      move: async (id, o) => {
        b.moves.push([id, o]);
        b.tabs.get(id).windowId = o.windowId;
      },
      onRemoved: on("tabRemoved"),
    },
    windows: {
      update: async (id, o) => b.focused.push([id, o]),
      create: async () => null,
      onCreated: on("windowCreated"),
    },
    cookies: {
      getAll: async (q) => b.jar.filter((c) => c.domain.replace(/^\./, "").endsWith(q.domain)),
      onChanged: on("cookieChanged"),
    },
    storage: { session: { set: async (o) => Object.assign(b.session, JSON.parse(JSON.stringify(o))) } },
  };
  return b;
}

function flow(opts) {
  const clock = fakeClock();
  const br = fakeBrowser();
  const sent = [];
  let changes = 0;
  const f = S.createSigninFlow({
    P,
    chrome: br.chrome,
    timers: clock.timers,
    now: clock.now,
    pageUrl: PAGE_URL,
    send: (m) => (opts && opts.offline ? false : (sent.push(JSON.parse(JSON.stringify(m))), true)),
    onChange: () => changes++,
  });
  return { f, clock, br, sent, changes: () => changes, kinds: () => sent.map((m) => m.type + (m.step ? ":" + m.step : "")) };
}

const SESSION_COOKIE = { name: "nexusmods_session", value: "s3cr3tvalue", domain: ".nexusmods.com", path: "/", secure: true, httpOnly: true, sameSite: "lax", expirationDate: 1893456000 };

test("A request opens Turbine's own page in front and waits; nothing goes before the sign-in", async () => {
  const x = flow();
  x.f.fromTurbine({ type: "signin.request", id: SID, steam: true });
  await x.clock.tick(10);
  assert.deepEqual(x.br.created, [{ url: PAGE_URL, active: true }]);
  assert.deepEqual(x.br.focused[0], [1, { focused: true }]);
  assert.equal(x.f.view().phase, "waiting");
  assert.match(x.f.view().text, /Turbine is waiting for you to sign in to Nexus Mods/);
  x.f.pageShown({ tab: { id: 10 } });
  await x.clock.tick(10000);
  assert.deepEqual(x.kinds(), ["signin.status:page"], "only the page-shown word; no cookies while signed out");
  // Turbine's own open of Nexus's sign-in page is taken over by this page.
  assert.equal(x.f.wantsTab(), true);
});

test("Already signed in: the sign-in goes at once; Turbine's answer brings the page to Signed in", async () => {
  const x = flow();
  x.br.jar.push(SESSION_COOKIE, { name: "SID", value: "g", domain: ".google.com" });
  x.f.fromTurbine({ type: "signin.request", id: SID, steam: true });
  await x.clock.tick(10);
  const c = x.sent.find((m) => m.type === "signin.cookies");
  assert.ok(c, "cookies sent");
  assert.equal(c.id, SID);
  assert.deepEqual(c.cookies.map((k) => k.name), ["nexusmods_session"]);
  assert.equal(x.f.view().phase, "sent");
  // The same cookies don't go again by themselves.
  await x.clock.tick(10000);
  assert.equal(x.sent.filter((m) => m.type === "signin.cookies").length, 1);
  x.f.fromTurbine({ type: "signin.done", id: SID, ok: true, text: "Turbine has your Nexus sign-in. Go back to Turbine." });
  await x.clock.tick(10);
  const v = x.f.view();
  assert.equal(v.phase, "ok");
  assert.equal(v.ok, true);
  assert.match(v.text, /^Signed in\. Go back to Turbine\.$/);
  assert.match(v.back, /Steam button/, "Game Mode says how to get back");
  assert.deepEqual(x.br.updates[x.br.updates.length - 1], [10, { url: PAGE_URL, active: true }]);
  // Nothing in session storage is a cookie value.
  assert.ok(!JSON.stringify(x.br.session).includes("s3cr3tvalue"));
});

test("Signing in on Nexus: the cookie change is noticed (and the poll as a backup)", async () => {
  const x = flow();
  x.f.fromTurbine({ type: "signin.request", id: SID, steam: false });
  await x.clock.tick(10);
  x.f.fromPage({ kind: "signin.go" }, { tab: { id: 10 } });
  await x.clock.tick(10);
  assert.deepEqual(x.br.updates[x.br.updates.length - 1], [10, { url: P.SIGNIN_URL, active: true }]);
  // Google sign-in finishes: Nexus sets its cookies one after another.
  x.br.jar.push(SESSION_COOKIE);
  x.br.fire("cookieChanged", { removed: false, cookie: SESSION_COOKIE });
  await x.clock.tick(500);
  assert.equal(x.sent.filter((m) => m.type === "signin.cookies").length, 0, "waits for the cookies to settle");
  await x.clock.tick(1500);
  assert.equal(x.sent.filter((m) => m.type === "signin.cookies").length, 1);
  assert.deepEqual(x.kinds().filter((k) => k.startsWith("signin.status")), ["signin.status:waiting", "signin.status:signedin", "signin.status:sending"]);
  assert.match(x.f.view().back || "", /^$/, "no way back said before Turbine took it");

  // The poll alone also finds it (no cookie event, a restarted worker).
  const y = flow();
  y.f.fromTurbine({ type: "signin.request", id: SID });
  await y.clock.tick(10);
  y.br.jar.push(SESSION_COOKIE);
  await y.clock.tick(2100);
  assert.equal(y.sent.filter((m) => m.type === "signin.cookies").length, 1);
});

test("I'm already signed in with no sign-in says so; Try again after a refusal sends again", async () => {
  const x = flow();
  x.f.fromTurbine({ type: "signin.request", id: SID });
  await x.clock.tick(10);
  x.f.fromPage({ kind: "signin.send" }, {});
  await x.clock.tick(10);
  assert.match(x.f.view().text, /isn't signed in to Nexus Mods yet/);
  x.br.jar.push(SESSION_COOKIE);
  x.f.fromPage({ kind: "signin.send" }, {});
  await x.clock.tick(10);
  x.f.fromTurbine({ type: "signin.done", id: SID, ok: false, text: "Turbine couldn't use this sign-in." });
  await x.clock.tick(10);
  assert.equal(x.f.view().phase, "failed");
  assert.equal(x.f.view().pending, true);
  x.f.fromPage({ kind: "signin.send" }, {});
  await x.clock.tick(10);
  assert.equal(x.sent.filter((m) => m.type === "signin.cookies").length, 2);
});

test("Game Mode: a window that opens right after the request takes the page", async () => {
  const x = flow();
  x.f.fromTurbine({ type: "signin.request", id: SID, steam: true });
  await x.clock.tick(10);
  // Steam starts the browser: its new empty window comes up over the page.
  x.br.fire("windowCreated", { id: 7, type: "normal" });
  await x.clock.tick(10);
  assert.deepEqual(x.br.moves, [[10, { windowId: 7, index: -1 }]]);
  assert.deepEqual(x.br.focused[x.br.focused.length - 1], [7, { focused: true }]);
  // Popups, and windows long after, are left alone.
  x.br.fire("windowCreated", { id: 8, type: "popup" });
  await x.clock.tick(20000);
  x.br.fire("windowCreated", { id: 9, type: "normal" });
  await x.clock.tick(10);
  assert.equal(x.br.moves.length, 1);
});

test("Pressed again: the same tab starts over with the new request; the same request again changes nothing", async () => {
  const x = flow();
  x.f.fromTurbine({ type: "signin.request", id: SID });
  await x.clock.tick(10);
  x.f.fromTurbine({ type: "signin.request", id: SID });
  await x.clock.tick(10);
  assert.equal(x.br.created.length, 1, "a reconnecting helper hears the same request: no second tab");
  x.f.fromTurbine({ type: "signin.request", id: SID2 });
  await x.clock.tick(10);
  assert.equal(x.br.created.length, 1, "a new press reuses the tab");
  assert.deepEqual(x.br.updates[x.br.updates.length - 1], [10, { active: true }]);
  assert.ok(x.br.updates.some(([id, o]) => id === 10 && o.url === PAGE_URL));
  // The old request's answer is ignored; the new one counts.
  x.f.fromTurbine({ type: "signin.done", id: SID, ok: true, text: "old" });
  assert.equal(x.f.view().phase, "waiting");
  // The tab was closed: the next press opens a new one.
  x.br.tabs.delete(10);
  x.br.fire("tabRemoved", 10);
  x.f.fromTurbine({ type: "signin.request", id: SID });
  await x.clock.tick(10);
  assert.equal(x.br.created.length, 2);
});

test("After 15 minutes the request is over and says how to start again", async () => {
  const x = flow();
  x.f.fromTurbine({ type: "signin.request", id: SID });
  await x.clock.tick(S.KEEP_MS + 5000);
  assert.equal(x.f.view().phase, "expired");
  assert.match(x.f.view().text, /Press Sign in with my browser in Turbine again/);
  x.br.jar.push(SESSION_COOKIE);
  x.f.fromPage({ kind: "signin.send" }, {});
  await x.clock.tick(10);
  assert.equal(x.sent.filter((m) => m.type === "signin.cookies").length, 0);
});

test("The API key: only on the player's press, only from the API Keys page, never stored", async () => {
  const x = flow();
  x.br.jar.push(SESSION_COOKIE);
  x.f.fromTurbine({ type: "signin.request", id: SID, steam: true });
  await x.clock.tick(10);
  // Not before Turbine took the sign-in.
  x.f.fromPage({ kind: "key.start" }, { tab: { id: 10 } });
  assert.equal(x.f.view().phase, "sent");
  x.f.fromTurbine({ type: "signin.done", id: SID, ok: true, text: "ok" });
  await x.clock.tick(10);
  x.f.fromPage({ kind: "key.start" }, { tab: { id: 10 } });
  await x.clock.tick(10);
  assert.equal(x.f.view().phase, "key-reading");
  assert.deepEqual(x.br.updates[x.br.updates.length - 1], [10, { url: P.KEY_PAGE_URL, active: true }]);
  // From anywhere but the API Keys page: ignored.
  x.f.fromPage({ kind: "key.found", key: KEY }, { url: "https://www.nexusmods.com/skyrimspecialedition/mods/1", tab: { id: 10 } });
  assert.equal(x.sent.filter((m) => m.type === "signin.key").length, 0);
  // From the page: to Turbine, and back to Turbine's page.
  const findings = K.findPersonalKey(keysPage(KEY));
  x.f.fromPage({ kind: "key.found", key: findings.key }, { url: P.KEY_PAGE_URL, tab: { id: 10 } });
  await x.clock.tick(10);
  assert.deepEqual(x.sent.filter((m) => m.type === "signin.key"), [{ type: "signin.key", id: SID, key: KEY }]);
  assert.equal(x.f.view().phase, "key-checking");
  assert.deepEqual(x.br.updates[x.br.updates.length - 1], [10, { url: PAGE_URL, active: true }]);
  assert.ok(!JSON.stringify(x.br.session).includes(KEY), "the key is never kept");
  assert.ok(!JSON.stringify(x.f.view()).includes(KEY), "nor shown");
  x.f.fromTurbine({ type: "signin.keydone", id: SID, ok: true, text: "Turbine has your API key." });
  assert.equal(x.f.view().phase, "key-ok");
  // A second key.found after that is ignored.
  x.f.fromPage({ kind: "key.found", key: KEY }, { url: P.KEY_PAGE_URL, tab: { id: 10 } });
  assert.equal(x.sent.filter((m) => m.type === "signin.key").length, 1);
});

test("The API key isn't on the page: the player is told what to do, and can try again", async () => {
  const x = flow();
  x.br.jar.push(SESSION_COOKIE);
  x.f.fromTurbine({ type: "signin.request", id: SID });
  await x.clock.tick(10);
  x.f.fromTurbine({ type: "signin.done", id: SID, ok: true, text: "ok" });
  x.f.fromPage({ kind: "key.start" }, { tab: { id: 10 } });
  x.f.fromPage({ kind: "key.missing", why: K.findPersonalKey(keysPage(null)).why }, { url: P.KEY_PAGE_URL, tab: { id: 10 } });
  assert.equal(x.f.view().phase, "key-missing");
  assert.match(x.f.view().text, /request one, then press Try again/);
  x.f.fromPage({ kind: "key.start" }, { tab: { id: 10 } });
  assert.equal(x.f.view().phase, "key-reading");
  x.f.fromTurbine({ type: "signin.keydone", id: SID, ok: false, text: "x" });
  assert.equal(x.f.view().phase, "key-reading", "a key answer only counts while Turbine checks one");
});

test("A restarted worker carries on from session storage", async () => {
  const x = flow();
  x.f.fromTurbine({ type: "signin.request", id: SID, steam: true });
  await x.clock.tick(10);
  const saved = JSON.parse(JSON.stringify(x.br.session.signin));
  const y = flow();
  y.f.restore(saved);
  assert.equal(y.f.view().phase, "waiting");
  assert.equal(y.f.view().steam, true);
  y.br.jar.push(SESSION_COOKIE);
  await y.clock.tick(2100);
  assert.equal(y.sent.filter((m) => m.type === "signin.cookies").length, 1, "polling started again");
  y.f.restore({ id: "bad", at: 1 });
  assert.equal(y.f.view().phase, "sent", "a broken saved state is ignored");
  y.f.restore(Object.assign({}, saved, { id: SID2 }));
  assert.equal(y.f.view().phase, "sent", "a request already under way isn't replaced by an older saved one");
});

// ---------- Turbine's sign-in page ----------

test("Turbine's sign-in page says what is happening at every step", () => {
  const st = (signin, conn) => ({ conn: conn || "connected", signin });
  let m = Page.model(st({ phase: "waiting", text: S.TEXT.waiting, how: S.TEXT.waitingHow }));
  assert.equal(m.title, "Turbine is waiting for you to sign in to Nexus Mods");
  assert.deepEqual(m.buttons.map((b) => b.kind), ["signin.go", "signin.send"]);
  assert.equal(m.buttons[0].label, "Sign in to Nexus Mods");
  m = Page.model(st({ phase: "ok", ok: true, text: S.TEXT.ok, back: S.TEXT.backSteam }));
  assert.equal(m.title, "Signed in. Go back to Turbine");
  assert.match(m.text, /Steam button/);
  assert.equal(m.key, true, "the API key offer shows");
  m = Page.model(st({ phase: "failed", text: "Turbine didn't take it." }));
  assert.equal(m.cls, "warn");
  assert.equal(m.buttons[0].label, "Try again");
  m = Page.model(st({ phase: "key-missing", ok: true, text: S.TEXT.keyNoKey, back: "" }));
  assert.equal(m.buttons[0].kind, "key.start");
  m = Page.model(st(null, "unpaired"));
  assert.match(m.text, /Pair again/);
  m = Page.model(null);
  assert.match(m.text, /Connecting to Turbine/);
  m = Page.model(st(null));
  assert.match(m.text, /isn't waiting for a sign-in/);
  for (const phase of ["waiting", "sent", "failed", "ok", "key-reading", "key-checking", "key-ok", "key-missing", "key-failed", "expired"]) {
    const mm = Page.model(st({ phase, text: "t", back: "b" }));
    assert.ok(mm.title && typeof mm.text === "string", phase);
  }
});
