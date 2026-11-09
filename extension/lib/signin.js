// Sign-in for Turbine: the player pressed "Sign in with my browser" in Turbine, which sent "signin.request".
//
// 1. The helper opens its own page (signin/signin.html): "Turbine is waiting for you to sign in to Nexus Mods". In
//    Game Mode Steam starts the browser at the same moment, which can put a new empty window over the one the page
//    went into: a window that opens within a few seconds takes the page.
// 2. The helper looks at Nexus's sign-in cookie itself (now, every 2 seconds, and when a Nexus cookie changes). Already
//    signed in counts. Then Nexus's cookies (nexusmods.com only) go to Turbine, which puts them into its own browser.
// 3. Turbine answers "signin.done": the page says "Signed in. Go back to Turbine" (and how, in Game Mode).
// 4. On the player's press only ("Give Turbine my API key too"), the tab goes to Nexus's API Keys page, the content
//    script reads the Personal API Key there (lib/keyfind.js), and it goes to Turbine, which checks it with Nexus and
//    keeps it ("signin.keydone"). The key is never stored by the helper, never shown, never logged.
//
// A pressed-again request (a new id) starts over in the same tab. Everything here runs in the service worker; the
// state (never a cookie or key value) is kept in session storage so a restarted worker carries on, and Turbine
// sends the request again when the helper reconnects.
(function (root) {
  "use strict";

  const KEEP_MS = 15 * 60 * 1000; // as long as Turbine waits (bridge.rs SIGNIN_WAIT)
  const POLL_MS = 2000;
  const SETTLE_MS = 1200; // after a Nexus cookie changes, wait for the rest of the sign-in's cookies
  const NEW_WINDOW_MS = 15000; // a window opening this soon after the request takes the page
  const REFOCUS_MS = [1500, 4000];

  // What each step says. The page and the panel on Nexus pages show these.
  const TEXT = {
    waiting: "Turbine is waiting for you to sign in to Nexus Mods.",
    waitingHow: "Press Sign in to Nexus Mods and sign in there (Google works too). Turbine takes the sign-in by itself when you're done.",
    notYet: "This browser isn't signed in to Nexus Mods yet. Press Sign in to Nexus Mods first.",
    sending: "Signed in to Nexus. Sending it to Turbine...",
    ok: "Signed in. Go back to Turbine.",
    failed: "Turbine didn't take the sign-in.",
    expired: "This sign-in request is over (Turbine waits 15 minutes). Press Sign in with my browser in Turbine again.",
    keyReading: "Reading your Personal API Key on this page for Turbine...",
    keyChecking: "Turbine is checking your API key with Nexus...",
    keyNoLabel: "This page doesn't show a Personal API Key. Sign in to Nexus on it, then press Try again.",
    keyNoKey: "There's no Personal API Key on this page yet. Scroll to the bottom of the page and request one, then press Try again.",
    backSteam: "Turbine comes back by itself. If it doesn't, press the Steam button, then pick Turbine at the top of the menu (it's still running).",
    backDesktop: "Turbine comes to the front by itself. If it doesn't, switch to it in the taskbar.",
    keyOk: "You can close this tab.",
    keyUnclear: "Turbine couldn't tell which key on this page is your Personal API Key. Copy it yourself, then in Turbine open Settings, Accounts and press Paste.",
  };

  function createSigninFlow(o) {
    const P = o.P;
    const chrome = o.chrome;
    const t = o.timers || globalThis;
    const now = o.now || (() => Date.now());
    const pageUrl = o.pageUrl;

    // {id, at, steam, tabId, phase, text, keyAt}. phase: waiting, sending, sent, ok, failed, key-reading,
    // key-checking, key-ok, key-failed, key-missing.
    let s = null;
    let sentSig = "";
    let lastStep = "";
    let pollTimer = null;
    let settleTimer = null;
    let windowUntil = 0;

    function live() {
      return !!s && now() - s.at < KEEP_MS;
    }
    function waiting() {
      return live() && (s.phase === "waiting" || s.phase === "sending" || s.phase === "sent" || s.phase === "failed");
    }

    function save() {
      if (chrome.storage && chrome.storage.session) chrome.storage.session.set({ signin: s }).catch(() => {});
    }
    function changed() {
      save();
      if (o.onChange) o.onChange();
    }
    function set(phase, text) {
      s.phase = phase;
      s.text = text;
      changed();
    }
    // Tells Turbine where the sign-in is (for its activity log; a fixed word only).
    function step(word) {
      if (!s || word === lastStep) return;
      const m = P.build.signinStatus(s.id, word);
      if (m && o.send(m)) lastStep = word;
    }

    // ---------- The page's tab ----------

    function focusTab(tabId) {
      if (!chrome.tabs || typeof tabId !== "number") return;
      chrome.tabs
        .get(tabId)
        .then((tab) => {
          if (!tab) return;
          chrome.tabs.update(tabId, { active: true }).catch(() => {});
          if (chrome.windows && typeof tab.windowId === "number") chrome.windows.update(tab.windowId, { focused: true }).catch(() => {});
        })
        .catch(() => {});
    }

    // Opens the page, or brings the one already open back with the new request.
    async function showPage() {
      if (!chrome.tabs) return;
      windowUntil = now() + NEW_WINDOW_MS;
      let tabId = s.tabId;
      if (typeof tabId === "number") {
        try {
          await chrome.tabs.update(tabId, { url: pageUrl, active: true });
        } catch (e) {
          tabId = null;
        }
      }
      if (typeof tabId !== "number") {
        try {
          const tab = await chrome.tabs.create({ url: pageUrl, active: true });
          tabId = tab && typeof tab.id === "number" ? tab.id : null;
        } catch (e) {
          if (chrome.windows) {
            const w = await chrome.windows.create({ url: pageUrl, focused: true }).catch(() => null);
            tabId = w && w.tabs && w.tabs[0] ? w.tabs[0].id : null;
          }
        }
      }
      if (!s) return;
      s.tabId = typeof tabId === "number" ? tabId : null;
      save();
      if (s.tabId === null) return;
      focusTab(s.tabId);
      // Steam's start of the browser can bring another window to the front a moment later: look again.
      for (const ms of REFOCUS_MS) t.setTimeout(() => s && windowUntil > now() && focusTab(s.tabId), ms);
    }

    // A window that opens right after the request (Game Mode: the browser started through Steam opens a new empty
    // window over the others) takes the page, so the player sees it there.
    function onWindowCreated(w) {
      if (!s || typeof s.tabId !== "number" || !w || typeof w.id !== "number" || now() > windowUntil) return;
      if (w.type && w.type !== "normal") return;
      const tabId = s.tabId;
      chrome.tabs
        .move(tabId, { windowId: w.id, index: -1 })
        .then(() => {
          chrome.tabs.update(tabId, { active: true }).catch(() => {});
          chrome.windows.update(w.id, { focused: true }).catch(() => {});
        })
        .catch(() => {});
    }

    function onTabRemoved(tabId) {
      if (s && s.tabId === tabId) {
        s.tabId = null;
        save();
      }
    }

    function goTo(url) {
      if (!s || typeof s.tabId !== "number" || !chrome.tabs) return;
      chrome.tabs.update(s.tabId, { url, active: true }).catch(() => {});
    }

    // ---------- Signed in? ----------

    async function jar() {
      if (!chrome.cookies) return [];
      try {
        return await chrome.cookies.getAll({ domain: "nexusmods.com" });
      } catch (e) {
        return [];
      }
    }

    // Sends Nexus's cookies when the browser is signed in. Sent again only when they changed, or on the player's
    // press ("button").
    async function check(why) {
      if (!waiting()) return false;
      const all = await jar();
      if (!waiting()) return false;
      if (!P.signedInJar(all)) {
        if (why === "button") {
          set("waiting", TEXT.notYet);
          step("waiting");
        }
        return false;
      }
      const m = P.build.signinCookies(s.id, all);
      if (!m || m.cookies.length === 0) return false;
      const sig = m.cookies.map((c) => c.domain + "|" + c.name + "|" + c.value.length + "|" + c.expirationDate).join(";");
      if (sig === sentSig && why !== "button" && s.phase !== "waiting") return false;
      step("signedin");
      if (!o.send(m)) return false;
      sentSig = sig;
      step("sending");
      set("sent", TEXT.sending);
      return true;
    }

    function startPolling() {
      if (pollTimer) return;
      pollTimer = t.setInterval(() => {
        if (!live() || !waiting()) {
          if (s && !live() && s.phase !== "expired") set("expired", TEXT.expired);
          t.clearInterval(pollTimer);
          pollTimer = null;
          return;
        }
        check("poll");
      }, POLL_MS);
    }

    function onCookie(info) {
      if (!waiting() || !info || !info.cookie || !P.isNexusCookieDomain(info.cookie.domain)) return;
      if (settleTimer) t.clearTimeout(settleTimer);
      settleTimer = t.setTimeout(() => {
        settleTimer = null;
        check("cookie");
      }, SETTLE_MS);
    }

    // ---------- From Turbine ----------

    function fromTurbine(msg) {
      if (msg.type === "signin.request") {
        if (s && s.id === msg.id && live()) {
          // The same request again (the helper reconnected): carry on where it was.
          startPolling();
          if (waiting()) check("again");
          return;
        }
        const tabId = s ? s.tabId : null;
        s = { id: msg.id, at: now(), steam: msg.steam === true, tabId, phase: "waiting", text: TEXT.waiting, keyAt: 0 };
        sentSig = "";
        lastStep = "";
        changed();
        showPage();
        startPolling();
        check("request");
        return;
      }
      if (!s || msg.id !== s.id) return;
      if (msg.type === "signin.done") {
        if (msg.ok) {
          set("ok", TEXT.ok);
          // Back to Turbine's page, which says so (the player may be on Nexus's own page now).
          goTo(pageUrl);
        } else {
          set("failed", (msg.text || TEXT.failed).trim());
        }
      } else if (msg.type === "signin.keydone") {
        if (s.phase !== "key-checking") return;
        set(msg.ok ? "key-ok" : "key-failed", msg.text || (msg.ok ? "Turbine has your API key." : "Turbine didn't take the key."));
      }
    }

    // ---------- From the page and the panel ----------

    function fromPage(msg, sender) {
      if (!s) return;
      const senderUrl = sender ? sender.url || (sender.tab && sender.tab.url) || "" : "";
      const senderTab = sender && sender.tab && typeof sender.tab.id === "number" ? sender.tab.id : null;
      switch (msg.kind) {
        case "signin.go":
          if (!waiting()) return;
          if (senderTab !== null) s.tabId = senderTab;
          step("waiting");
          goTo(P.SIGNIN_URL);
          break;
        case "signin.send":
          check("button");
          break;
        case "key.start":
          // Only after Turbine took the sign-in, and only within the request's 15 minutes.
          if (!live() || !["ok", "key-failed", "key-missing"].includes(s.phase)) return;
          if (senderTab !== null) s.tabId = senderTab;
          s.keyAt = now();
          set("key-reading", TEXT.keyReading);
          step("keypage");
          goTo(P.KEY_PAGE_URL);
          break;
        case "key.found": {
          if (!live() || s.phase !== "key-reading" || !P.isKeyPage(senderUrl)) return;
          const m = P.build.signinKey(s.id, msg.key);
          if (!m) {
            set("key-missing", TEXT.keyNoKey);
            step("keymissing");
            return;
          }
          if (!o.send(m)) {
            set("key-failed", "Turbine isn't connected. Open Turbine, then press Try again.");
            return;
          }
          step("keysent");
          set("key-checking", TEXT.keyChecking);
          goTo(pageUrl);
          break;
        }
        case "key.missing":
          if (!live() || s.phase !== "key-reading" || !P.isKeyPage(senderUrl)) return;
          set("key-missing", msg.why === "unclear" ? TEXT.keyUnclear : msg.why === "nolabel" ? TEXT.keyNoLabel : TEXT.keyNoKey);
          step("keymissing");
          break;
        default:
          break;
      }
    }

    // Turbine's page is on screen: say so to Turbine (its activity log shows the player saw it).
    function pageShown(sender) {
      if (!live()) return;
      if (sender && sender.tab && typeof sender.tab.id === "number" && s.tabId === null) {
        s.tabId = sender.tab.id;
        save();
      }
      if (s.phase === "waiting") step("page");
    }

    // Where a Nexus tab is now (the panel's report, URL only): a sign-in may just have finished.
    function onWhere(url) {
      if (!waiting()) return;
      let u = null;
      try {
        u = new URL(url);
      } catch (e) {
        return;
      }
      if (!P.isNexusHost(u.hostname)) return;
      check("page");
    }

    // Is Turbine's sign-in page taking care of the sign-in (so Nexus's sign-in page needn't open on its own)?
    function wantsTab() {
      return live() && (typeof s.tabId === "number" || now() - s.at < 5000);
    }

    // What the page and the panel show.
    function view() {
      if (!s) return null;
      if (!live() && s.phase !== "ok" && s.phase !== "key-ok") return { phase: "expired", pending: false, ok: false, text: TEXT.expired, steam: s.steam };
      const ok = ["ok", "key-reading", "key-checking", "key-ok", "key-failed", "key-missing"].includes(s.phase);
      return { phase: s.phase, pending: !ok, ok, text: s.text, how: s.phase === "waiting" ? TEXT.waitingHow : "", back: ok ? (s.steam ? TEXT.backSteam : TEXT.backDesktop) : "", steam: s.steam };
    }

    function restore(saved) {
      // A request that came in before the saved state was read is newer: it stays.
      if (s || !saved || !P.isSigninId(saved.id) || typeof saved.at !== "number") return;
      s = {
        id: saved.id,
        at: saved.at,
        steam: saved.steam === true,
        tabId: typeof saved.tabId === "number" ? saved.tabId : null,
        phase: typeof saved.phase === "string" ? saved.phase : "waiting",
        text: typeof saved.text === "string" ? saved.text : TEXT.waiting,
        keyAt: 0,
      };
      // A key read that was under way when the worker stopped starts over on the player's press.
      if (s.phase === "key-reading" || s.phase === "key-checking") s.phase = "key-failed";
      if (waiting()) startPolling();
      if (o.onChange) o.onChange();
    }

    function stop() {
      if (pollTimer) t.clearInterval(pollTimer);
      if (settleTimer) t.clearTimeout(settleTimer);
      pollTimer = settleTimer = null;
    }

    if (chrome.cookies && chrome.cookies.onChanged) chrome.cookies.onChanged.addListener(onCookie);
    if (chrome.windows && chrome.windows.onCreated) chrome.windows.onCreated.addListener(onWindowCreated);
    if (chrome.tabs && chrome.tabs.onRemoved) chrome.tabs.onRemoved.addListener(onTabRemoved);

    return { fromTurbine, fromPage, pageShown, onWhere, wantsTab, view, restore, stop, live };
  }

  const api = { createSigninFlow, TEXT, KEEP_MS, POLL_MS };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else root.TurbineSignin = api;
})(typeof globalThis !== "undefined" ? globalThis : this);
