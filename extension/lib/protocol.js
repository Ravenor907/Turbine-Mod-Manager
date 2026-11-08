// Turbine bridge protocol v1: URL parsing, message checks and message builders.
// Pure code with no browser calls, shared by the service worker, the content scripts and the tests.
(function (root) {
  "use strict";

  const PROTOCOL_VERSION = 1;
  const PORT_MIN = 47701;
  const PORT_MAX = 47710;
  const LEVELS = ["ok", "busy", "wait", "hint", "warn", "stop"];
  // Sign-in pass-through: Nexus's own cookies only, at most this many, at most this long.
  const SIGNIN_URL = "https://users.nexusmods.com/auth/sign_in";
  const MAX_COOKIES = 80;
  const KINDS = ["one", "any", "atleastone", "atmostone", "all"];
  const STATES = ["selected", "available", "required", "disabled"];

  function parseUrl(text) {
    if (typeof text !== "string" || text === "") return null;
    try {
      return new URL(text);
    } catch (e) {
      return null;
    }
  }

  function hostIs(host, base) {
    host = String(host || "").toLowerCase();
    return host === base || host.endsWith("." + base);
  }

  function isNexusHost(host) {
    return hostIs(host, "nexusmods.com") || hostIs(host, "nexus-cdn.com");
  }

  // True when a download came from Nexus: its URL, final URL or referrer is on nexusmods.com or nexus-cdn.com.
  function isNexusDownload(item) {
    if (!item) return false;
    for (const field of [item.url, item.finalUrl, item.referrer]) {
      const u = parseUrl(field);
      if (u && (u.protocol === "https:" || u.protocol === "http:") && isNexusHost(u.hostname)) return true;
    }
    return false;
  }

  function positiveInt(text) {
    if (typeof text !== "string" || !/^[0-9]{1,12}$/.test(text)) return null;
    const n = Number(text);
    return n > 0 ? n : null;
  }

  // Reads a Nexus page URL. Returns {domain, modId, fileId} (ids may be null) or null when it is not a game page.
  // Handles /<game>/mods/<id>?tab=files&file_id=<n> and the /games/<game>/... form.
  function parseNexusUrl(text) {
    const u = parseUrl(text);
    if (!u || u.protocol !== "https:" || !hostIs(u.hostname, "nexusmods.com")) return null;
    let parts = u.pathname.split("/").filter(Boolean);
    if (parts[0] === "games") parts = parts.slice(1);
    if (parts.length === 0) return null;
    const domain = parts[0].toLowerCase();
    if (!/^[a-z0-9][a-z0-9_-]{0,63}$/.test(domain)) return null;
    const reserved = ["games", "users", "about", "news", "search", "mods", "collections", "media", "settings", "account", "premium", "contact"];
    if (reserved.includes(domain)) return null;
    let modId = null;
    if (parts[1] === "mods" && parts.length >= 3) modId = positiveInt(parts[2]);
    let fileId = positiveInt(u.searchParams.get("file_id") || u.searchParams.get("id") || "");
    if (modId === null) fileId = null;
    return { domain, modId, fileId };
  }

  // May Turbine have this page opened in a new tab? Only https pages on www.nexusmods.com or another
  // nexusmods.com host, with no user name or port, at most 2048 characters. Opening a tab is all the helper does
  // with it: it never clicks anything on the page.
  function isNexusTabUrl(text) {
    if (typeof text !== "string" || text.length > 2048 || /[\s\\]/.test(text)) return null;
    const u = parseUrl(text);
    if (!u || u.protocol !== "https:" || u.username || u.password || u.port) return null;
    const host = u.hostname.toLowerCase();
    if (!host.endsWith(".nexusmods.com")) return null;
    if (!/^([a-z0-9-]+\.)+nexusmods\.com$/.test(host)) return null;
    return u.href;
  }

  function isNxmUrl(text) {
    if (typeof text !== "string" || text.length > 2048) return false;
    return /^nxm:\/\/[a-z0-9_-]+\/[^\s"'<>\\]+$/i.test(text);
  }

  // A sign-in request id from Turbine: 32 hex digits.
  function isSigninId(text) {
    return typeof text === "string" && /^[0-9a-f]{32}$/.test(text);
  }

  // A cookie's domain is Nexus's own: nexusmods.com or a host under it (never a look-alike).
  function isNexusCookieDomain(domain) {
    if (typeof domain !== "string") return false;
    const d = domain.replace(/^\./, "").toLowerCase();
    if (!/^[a-z0-9.-]{1,253}$/.test(d) || d.split(".").some((p) => p === "")) return false;
    return d === "nexusmods.com" || d.endsWith(".nexusmods.com");
  }

  // The cookies chrome.cookies.getAll gave, reduced to Nexus's own, with only the fields Turbine needs.
  function nexusCookies(list) {
    if (!Array.isArray(list)) return [];
    return list
      .filter((c) => c && typeof c.name === "string" && typeof c.value === "string" && isNexusCookieDomain(c.domain))
      .slice(0, MAX_COOKIES)
      .map((c) => ({
        name: c.name,
        value: c.value,
        domain: c.domain,
        hostOnly: c.hostOnly === true,
        path: typeof c.path === "string" && c.path ? c.path : "/",
        secure: c.secure === true,
        httpOnly: c.httpOnly === true,
        sameSite: ["no_restriction", "lax", "strict", "unspecified"].includes(c.sameSite) ? c.sameSite : "unspecified",
        expirationDate: typeof c.expirationDate === "number" && isFinite(c.expirationDate) ? c.expirationDate : null,
      }));
  }

  function isToken(text) {
    return typeof text === "string" && /^[0-9a-f]{64}$/.test(text);
  }

  function isPort(n) {
    return Number.isInteger(n) && n >= PORT_MIN && n <= PORT_MAX;
  }

  // Reads the pairing page location: http://127.0.0.1:<port>/pair#<token>. Returns {port, token} or null.
  function parsePairingUrl(text) {
    const u = parseUrl(text);
    if (!u || u.protocol !== "http:" || u.hostname !== "127.0.0.1") return null;
    if (!u.pathname.startsWith("/pair")) return null;
    const port = Number(u.port);
    const token = u.hash.replace(/^#/, "").toLowerCase();
    if (!isPort(port) || !isToken(token)) return null;
    return { port, token };
  }

  function str(v, max) {
    return typeof v === "string" ? v.slice(0, max || 2000) : null;
  }

  function httpsLink(v) {
    const u = parseUrl(v);
    return u && u.protocol === "https:" ? u.href : null;
  }

  function imageDataUrl(v) {
    if (typeof v !== "string") return null;
    return /^data:image\/(png|jpeg|gif|webp|bmp);base64,[A-Za-z0-9+/=\s]+$/.test(v) ? v : null;
  }

  // Checks a message from Turbine and returns a clean copy, or null if it is unknown or malformed.
  function validateIncoming(msg) {
    if (typeof msg === "string") {
      try {
        msg = JSON.parse(msg);
      } catch (e) {
        return null;
      }
    }
    if (!msg || typeof msg !== "object" || Array.isArray(msg)) return null;
    switch (msg.type) {
      case "welcome": {
        const games = Array.isArray(msg.games)
          ? msg.games
              .filter((g) => g && typeof g === "object")
              .map((g) => ({ id: str(String(g.id ?? ""), 100), name: str(g.name, 200) || "", domain: str(g.domain, 100), folder: str(g.folder, 100) || "" }))
          : [];
        return { type: "welcome", turbineVersion: str(msg.turbineVersion, 40) || "", games };
      }
      case "status": {
        if (typeof msg.text !== "string") return null;
        const level = LEVELS.includes(msg.level) ? msg.level : "ok";
        let progress = null;
        if (typeof msg.progress === "number" && isFinite(msg.progress)) progress = Math.min(1, Math.max(0, msg.progress));
        return { type: "status", text: str(msg.text, 500), level, progress, game: str(msg.game, 200) };
      }
      case "added": {
        if (typeof msg.game !== "string" || typeof msg.mod !== "string") return null;
        const notes = Array.isArray(msg.notes) ? msg.notes.filter((n) => typeof n === "string").map((n) => n.slice(0, 1000)).slice(0, 20) : [];
        const needs = Array.isArray(msg.needs)
          ? msg.needs
              .filter((n) => n && typeof n.name === "string")
              .map((n) => ({ name: n.name.slice(0, 200), url: httpsLink(n.url) }))
              .slice(0, 30)
          : [];
        return { type: "added", game: str(msg.game, 200), mod: str(msg.mod, 300), notes, needs };
      }
      case "choices": {
        if (typeof msg.sessionId !== "string" || !Array.isArray(msg.groups)) return null;
        const stepCount = Number.isInteger(msg.stepCount) && msg.stepCount > 0 ? msg.stepCount : 1;
        const stepIndex = Number.isInteger(msg.stepIndex) && msg.stepIndex >= 0 ? Math.min(msg.stepIndex, stepCount - 1) : 0;
        const groups = [];
        for (const g of msg.groups) {
          if (!g || !Array.isArray(g.options)) return null;
          const kind = KINDS.includes(g.kind) ? g.kind : "any";
          const options = [];
          for (const o of g.options) {
            if (!o || (typeof o.id !== "string" && typeof o.id !== "number")) return null;
            options.push({
              id: String(o.id),
              name: str(o.name, 300) || String(o.id),
              description: str(o.description, 5000) || "",
              image: imageDataUrl(o.image),
              state: STATES.includes(o.state) ? o.state : "available",
            });
          }
          groups.push({ title: str(g.title, 300) || "", kind, options });
        }
        return {
          type: "choices",
          sessionId: msg.sessionId.slice(0, 200),
          modName: str(msg.modName, 300) || "",
          stepTitle: str(msg.stepTitle, 300) || "",
          stepIndex,
          stepCount,
          groups,
        };
      }
      case "choices.done":
        if (typeof msg.sessionId !== "string") return null;
        return { type: "choices.done", sessionId: msg.sessionId };
      case "error":
        if (typeof msg.text !== "string") return null;
        return { type: "error", text: str(msg.text, 1000) };
      case "open": {
        const url = isNexusTabUrl(msg.url);
        return url ? { type: "open", url } : null;
      }
      // Turbine put a newer helper in the folder this one was loaded from: reload from it.
      case "reload":
        return { type: "reload" };
      // The player pressed "Sign in with my browser" in Turbine: send Nexus's sign-in once he has signed in here.
      case "signin.request":
        if (!isSigninId(msg.id)) return null;
        return { type: "signin.request", id: msg.id, url: isNexusTabUrl(msg.url) || SIGNIN_URL };
      case "signin.done":
        if (!isSigninId(msg.id)) return null;
        return { type: "signin.done", id: msg.id, ok: msg.ok === true, text: str(msg.text, 300) || "" };
      default:
        return null;
    }
  }

  // Builders for messages sent to Turbine.
  const build = {
    hello: (token, ext) => ({ type: "hello", token, version: PROTOCOL_VERSION, ext: String(ext) }),
    ping: () => ({ type: "ping" }),
    downloadStarted: (d) => ({
      type: "download.started",
      id: d.id,
      url: d.url || "",
      finalUrl: d.finalUrl || d.url || "",
      filename: d.filename || "",
      referrer: d.referrer || "",
      mime: d.mime || "",
      totalBytes: typeof d.totalBytes === "number" && d.totalBytes >= 0 ? d.totalBytes : null,
    }),
    downloadProgress: (id, receivedBytes, totalBytes) => ({
      type: "download.progress",
      id,
      receivedBytes,
      totalBytes: typeof totalBytes === "number" && totalBytes >= 0 ? totalBytes : null,
    }),
    downloadFinished: (id, path, size) => ({ type: "download.finished", id, path, size }),
    downloadFailed: (id, error) => ({ type: "download.failed", id, error: String(error || "unknown") }),
    restartFailed: (filename, error) => ({ type: "download.restartFailed", filename: String(filename || ""), error: String(error || "unknown") }),
    nxm: (url) => ({ type: "nxm", url }),
    page: (url) => {
      const p = parseNexusUrl(url);
      if (!p) return null;
      return { type: "page", url, domain: p.domain, modId: p.modId };
    },
    choice: (sessionId, step, selected) => ({ type: "choice", sessionId, step, selected: selected.slice() }),
    choiceBack: (sessionId) => ({ type: "choice.back", sessionId }),
    choiceCancel: (sessionId) => ({ type: "choice.cancel", sessionId }),
    // Only for a request Turbine made, and only Nexus's own cookies.
    signinCookies: (id, cookies) => (isSigninId(id) ? { type: "signin.cookies", id, cookies: nexusCookies(cookies) } : null),
  };

  const api = {
    PROTOCOL_VERSION,
    PORT_MIN,
    PORT_MAX,
    isNexusHost,
    isNexusDownload,
    parseNexusUrl,
    isNxmUrl,
    isNexusTabUrl,
    isSigninId,
    isNexusCookieDomain,
    nexusCookies,
    SIGNIN_URL,
    isToken,
    isPort,
    parsePairingUrl,
    validateIncoming,
    build,
  };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else root.TurbineProtocol = api;
})(typeof globalThis !== "undefined" ? globalThis : this);
