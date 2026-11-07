// Service worker: keeps the link to Turbine, reports Nexus downloads and nxm links, and feeds the panel and popup.
/* global importScripts, chrome, TurbineProtocol, TurbineBridge, TurbineDownloads */
"use strict";

importScripts("lib/protocol.js", "lib/bridge.js", "lib/downloads.js", "lib/save.js");

const P = TurbineProtocol;
const EXT_VERSION = chrome.runtime.getManifest().version;
const ADDED_KEEP_MS = 10 * 60 * 1000;

// What the panel and popup show. Kept in session storage so a restarted worker still knows it.
let view = { status: null, added: null, choices: null, lastError: null };
const ports = new Set();

const bridge = TurbineBridge.createBridge({
  WebSocket,
  extVersion: EXT_VERSION,
  loadPairing: async () => {
    const s = await chrome.storage.local.get(["turbinePort", "turbineToken"]);
    return s.turbinePort ? { port: s.turbinePort, token: s.turbineToken } : null;
  },
  onState: (st) => {
    if (st.conn !== "connected") view.choices = null;
    updateBadge();
    broadcast();
  },
  onMessage: handleTurbine,
});

const tracker = TurbineDownloads.createDownloadTracker({
  search: (q) => chrome.downloads.search(q),
  send: (msg) => bridge.send(msg),
  save: (list) => chrome.storage.session.set({ trackedDownloads: list }),
});

function handleTurbine(msg) {
  switch (msg.type) {
    case "status":
      view.status = msg;
      break;
    case "added":
      view.added = Object.assign({ at: Date.now() }, msg);
      break;
    case "choices":
      view.choices = msg;
      break;
    case "choices.done":
      if (view.choices && view.choices.sessionId === msg.sessionId) view.choices = null;
      break;
    case "error":
      view.lastError = msg.text;
      break;
    default:
      break;
  }
  chrome.storage.session.set({ view });
  updateBadge();
  broadcast();
}

function snapshot() {
  const st = bridge.getState();
  if (view.added && Date.now() - view.added.at > ADDED_KEEP_MS) view.added = null;
  return {
    kind: "state",
    conn: st.conn,
    error: st.error,
    turbineVersion: st.welcome ? st.welcome.turbineVersion : null,
    status: view.status,
    added: view.added,
    choices: st.conn === "connected" ? view.choices : null,
    lastError: view.lastError,
  };
}

function broadcast() {
  const s = snapshot();
  for (const port of ports) {
    try {
      port.postMessage(s);
    } catch (e) {
      ports.delete(port);
    }
  }
}

const BADGE = {
  ok: { text: "OK", color: "#4F7A3A" },
  busy: { text: "...", color: "#C99E52" },
  warn: { text: "!", color: "#B9822E" },
  stop: { text: "X", color: "#A23B2A" },
};

function updateBadge() {
  const st = bridge.getState();
  let text = "";
  let color = "#6B5A3E";
  if (st.conn === "refused") {
    text = "!";
    color = BADGE.warn.color;
  } else if (st.conn === "connected" && view.choices) {
    text = "?";
    color = BADGE.busy.color;
  } else if (st.conn === "connected" && view.status) {
    const b = BADGE[view.status.level] || BADGE.ok;
    text = b.text;
    color = b.color;
    if (view.status.level === "busy" && typeof view.status.progress === "number") text = Math.round(view.status.progress * 100) + "%";
  }
  chrome.action.setBadgeText({ text });
  chrome.action.setBadgeBackgroundColor({ color });
  if (chrome.action.setBadgeTextColor) chrome.action.setBadgeTextColor({ color: "#0B0A08" });
}

function senderIsNexus(sender) {
  try {
    return P.isNexusHost(new URL(sender.url || (sender.tab && sender.tab.url) || "").hostname);
  } catch (e) {
    return false;
  }
}

// Messages from the panel on Nexus pages.
function handlePanel(msg, port) {
  if (!msg || typeof msg !== "object") return;
  switch (msg.kind) {
    case "hello":
      bridge.nudge();
      port.postMessage(snapshot());
      break;
    case "page": {
      const m = P.build.page(msg.url);
      if (m) bridge.send(m);
      break;
    }
    case "nxm":
      if (P.isNxmUrl(msg.url)) bridge.send(P.build.nxm(msg.url));
      break;
    case "choice":
      if (view.choices && msg.sessionId === view.choices.sessionId && Array.isArray(msg.selected)) {
        bridge.send(P.build.choice(msg.sessionId, msg.step, msg.selected.map(String)));
      }
      break;
    case "choice.back":
      if (view.choices && msg.sessionId === view.choices.sessionId) bridge.send(P.build.choiceBack(msg.sessionId));
      break;
    case "choice.cancel":
      if (view.choices && msg.sessionId === view.choices.sessionId) {
        bridge.send(P.build.choiceCancel(msg.sessionId));
        view.choices = null;
        broadcast();
      }
      break;
    case "dismiss":
      view.added = null;
      view.lastError = null;
      chrome.storage.session.set({ view });
      broadcast();
      break;
    default:
      break;
  }
}

chrome.runtime.onConnect.addListener((port) => {
  if (port.name === "turbine-panel" && !senderIsNexus(port.sender)) {
    port.disconnect();
    return;
  }
  if (port.name !== "turbine-panel" && port.name !== "turbine-popup") return;
  ports.add(port);
  port.onDisconnect.addListener(() => ports.delete(port));
  port.onMessage.addListener((msg) => {
    if (port.name === "turbine-panel") handlePanel(msg, port);
    else if (msg && msg.kind === "reconnect") bridge.reconnect();
    else if (msg && msg.kind === "hello") port.postMessage(snapshot());
  });
});

// The pairing page content script hands over the port and token Turbine put in its URL.
chrome.runtime.onMessage.addListener((msg, sender, reply) => {
  if (!msg || msg.kind !== "pair") return false;
  const fromUrl = P.parsePairingUrl(String(msg.pageUrl || ""));
  let senderOk = false;
  try {
    const u = new URL(sender.url || "");
    senderOk = u.protocol === "http:" && u.hostname === "127.0.0.1" && fromUrl && Number(u.port) === fromUrl.port && u.pathname.startsWith("/pair");
  } catch (e) {
    senderOk = false;
  }
  if (!senderOk) {
    reply({ ok: false });
    return false;
  }
  chrome.storage.local.set({ turbinePort: fromUrl.port, turbineToken: fromUrl.token }).then(() => {
    bridge.reconnect();
    reply({ ok: true });
  });
  return true;
});

// Nexus downloads go into Downloads/Game Mods/<game>/ with no Save box (lib/save.js).
chrome.downloads.onDeterminingFilename.addListener((item, suggest) => {
  const st = bridge.getState();
  const games = st.welcome && Array.isArray(st.welcome.games) ? st.welcome.games : [];
  const p = TurbineSave.plan(item, games, chrome.runtime.id);
  if (!p) return false;
  chrome.downloads
    .download({ url: p.url, filename: p.filename, saveAs: false, conflictAction: "uniquify" })
    .then(() => {
      tracker.forget(item.id);
      // The new copy is on its way: the browser's own one (and its Save box) goes.
      chrome.downloads.cancel(item.id).catch(() => {});
      chrome.downloads.erase({ id: item.id }).catch(() => {});
    })
    .catch((e) => {
      // Couldn't start it again (the link may have expired): the browser's own download goes on as usual.
      bridge.send(P.build.restartFailed(item.filename, e && e.message));
      suggest();
    });
  return true;
});

chrome.downloads.onCreated.addListener((item) => {
  if (tracker.onCreated(item)) bridge.nudge();
});
chrome.downloads.onChanged.addListener((delta) => tracker.onChanged(delta));
chrome.downloads.onErased.addListener((id) => tracker.onErased(id));

chrome.runtime.onStartup.addListener(() => bridge.nudge());
chrome.runtime.onInstalled.addListener(() => bridge.nudge());

chrome.storage.session.get(["view", "trackedDownloads"]).then((s) => {
  if (s.view) view = Object.assign(view, s.view, { choices: null });
  tracker.restore(s.trackedDownloads);
  updateBadge();
  broadcast();
});

bridge.start();
