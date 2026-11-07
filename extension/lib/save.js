// Saving Nexus downloads into Downloads/Game Mods/<game>/ with no Save box. When the browser starts a download from
// Nexus's file servers (the player pressed Slow download on a page), the helper starts the same file again itself
// with saveAs false and a relative path (allowed only inside Downloads), then cancels the browser's own copy. If
// starting again fails (the link expired), the browser's own download goes on as usual and Turbine is told.
// Nexus's buttons and wait are never touched: only where the finished file is saved changes.
(function (root) {
  "use strict";

  const P = typeof module !== "undefined" && module.exports ? require("./protocol.js") : root.TurbineProtocol;

  const GAME_MODS = "Game Mods";
  const OTHER = "Other";

  // The same cleaning Turbine does (watch::game_folder_name): no characters browsers or file systems refuse.
  function folderName(name) {
    const cleaned = String(name || "")
      .replace(/[<>:"/\\|?*\u0000-\u001f\u007f]/g, "")
      .split(/\s+/)
      .filter(Boolean)
      .join(" ")
      .replace(/^\.+|\.+$/g, "")
      .slice(0, 80)
      .trim();
    return cleaned || OTHER;
  }

  // The file's own name from a path or URL, without folders or characters a file name can't have.
  function baseName(text) {
    let s = String(text || "");
    try {
      if (/^https?:/.test(s)) s = decodeURIComponent(new URL(s).pathname);
    } catch (e) {
      /* keep the text */
    }
    s = s.split(/[\\/]/).pop() || "";
    s = s.replace(/[<>:"|?*\u0000-\u001f\u007f]/g, "_").replace(/^\.+/, "").trim();
    return s.slice(0, 200);
  }

  // What to do with a download the browser just started. Returns null to leave it alone, or
  // {filename: "Game Mods/<game>/<file>", game} to start it again there. games: Turbine's welcome list
  // [{domain, name, folder}]. ownId: this helper's extension id (its own downloads are left alone).
  function plan(item, games, ownId) {
    if (!item || (ownId && item.byExtensionId === ownId)) return null;
    // Only files from Nexus's file servers, started from a Nexus page.
    const u = (() => {
      try {
        return new URL(item.finalUrl || item.url || "");
      } catch (e) {
        return null;
      }
    })();
    if (!u || u.protocol !== "https:" || !/(^|\.)nexus-cdn\.com$/i.test(u.hostname)) return null;
    const ref = P.parseNexusUrl(item.referrer || "");
    if (!ref && !/(^|\.)nexusmods\.com$/i.test((() => { try { return new URL(item.referrer || "").hostname; } catch (e) { return ""; } })())) return null;
    const name = baseName(item.filename) || baseName(item.finalUrl || item.url);
    if (!name) return null;
    const game = ref ? (games || []).find((g) => g && g.domain === ref.domain) : null;
    const folder = game ? folderName(game.folder || game.name) : OTHER;
    return { filename: GAME_MODS + "/" + folder + "/" + name, game: game ? game.name : null, url: item.finalUrl || item.url };
  }

  const api = { plan, folderName, baseName, GAME_MODS, OTHER };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else root.TurbineSave = api;
})(typeof globalThis !== "undefined" ? globalThis : this);
