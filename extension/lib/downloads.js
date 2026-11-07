// Watches the browser's downloads and reports the ones that come from Nexus to Turbine.
// The browser API is passed in (search, timers, send, a small store) so the tests can drive it.
(function (root) {
  "use strict";

  const P = typeof module !== "undefined" && module.exports ? require("./protocol.js") : root.TurbineProtocol;

  const POLL_MS = 1000;
  const NAME_WAIT_MS = 1500;

  function createDownloadTracker(opts) {
    const t = opts.timers || globalThis;
    const pollMs = opts.pollMs || POLL_MS;
    const nameWaitMs = opts.nameWaitMs || NAME_WAIT_MS;
    // id -> {announced, received}
    const tracked = new Map();
    const nameTimers = new Map();
    let pollTimer = null;

    function save() {
      if (opts.save) opts.save(Array.from(tracked.entries()).map(([id, v]) => ({ id, announced: v.announced })));
    }

    // Restores the list after the service worker was stopped and started again.
    function restore(list) {
      if (!Array.isArray(list)) return;
      for (const e of list) if (e && Number.isInteger(e.id) && !tracked.has(e.id)) tracked.set(e.id, { announced: !!e.announced, received: -1 });
      if (tracked.size) startPolling();
    }

    async function lookup(id) {
      try {
        const items = await opts.search({ id });
        return items && items[0] ? items[0] : null;
      } catch (e) {
        return null;
      }
    }

    function announce(item) {
      const entry = tracked.get(item.id);
      if (!entry || entry.announced) return;
      entry.announced = true;
      if (nameTimers.has(item.id)) {
        t.clearTimeout(nameTimers.get(item.id));
        nameTimers.delete(item.id);
      }
      save();
      opts.send(P.build.downloadStarted(item));
    }

    function onCreated(item) {
      if (!item || !Number.isInteger(item.id) || !P.isNexusDownload(item)) return false;
      if (tracked.has(item.id)) return true;
      tracked.set(item.id, { announced: false, received: -1 });
      save();
      if (item.filename) announce(item);
      else {
        // Chrome often fills in the file name a moment later; wait briefly so Turbine gets the real path.
        nameTimers.set(
          item.id,
          t.setTimeout(async () => {
            nameTimers.delete(item.id);
            const fresh = (await lookup(item.id)) || item;
            announce(fresh);
          }, nameWaitMs)
        );
      }
      startPolling();
      return true;
    }

    async function onChanged(delta) {
      if (!delta || !tracked.has(delta.id)) return;
      const entry = tracked.get(delta.id);
      if (delta.filename && delta.filename.current && !entry.announced) {
        const item = await lookup(delta.id);
        if (item) announce(item);
      }
      const state = delta.state && delta.state.current;
      if (state === "complete") {
        const item = await lookup(delta.id);
        if (item && !entry.announced) announce(item);
        finish(delta.id);
        if (item) opts.send(P.build.downloadFinished(item.id, item.filename, item.fileSize > 0 ? item.fileSize : item.bytesReceived));
        else opts.send(P.build.downloadFailed(delta.id, "The finished download could not be found"));
      } else if (state === "interrupted") {
        const item = await lookup(delta.id);
        const err = (delta.error && delta.error.current) || (item && item.error) || "INTERRUPTED";
        if (item && !entry.announced) announce(item);
        finish(delta.id);
        opts.send(P.build.downloadFailed(delta.id, err));
      }
    }

    function onErased(id) {
      if (tracked.has(id)) {
        finish(id);
        opts.send(P.build.downloadFailed(id, "USER_ERASED"));
      }
    }

    function finish(id) {
      tracked.delete(id);
      if (nameTimers.has(id)) {
        t.clearTimeout(nameTimers.get(id));
        nameTimers.delete(id);
      }
      save();
      if (tracked.size === 0) stopPolling();
    }

    // chrome.downloads has no progress event, so active downloads are read once a second.
    async function poll() {
      for (const [id, entry] of Array.from(tracked.entries())) {
        const item = await lookup(id);
        if (!item) continue;
        if (item.state !== "in_progress" || !entry.announced) continue;
        if (item.bytesReceived !== entry.received) {
          entry.received = item.bytesReceived;
          opts.send(P.build.downloadProgress(id, item.bytesReceived, item.totalBytes));
        }
      }
    }

    function startPolling() {
      if (!pollTimer) pollTimer = t.setInterval(poll, pollMs);
    }

    function stopPolling() {
      if (pollTimer) t.clearInterval(pollTimer);
      pollTimer = null;
    }

    function activeIds() {
      return Array.from(tracked.keys());
    }

    // A download the helper replaced with its own copy (saved into Game Mods): dropped without a word to Turbine.
    function forget(id) {
      if (tracked.has(id)) finish(id);
    }

    return { onCreated, onChanged, onErased, restore, poll, activeIds, forget, stop: stopPolling };
  }

  const api = { createDownloadTracker };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else root.TurbineDownloads = api;
})(typeof globalThis !== "undefined" ? globalThis : this);
