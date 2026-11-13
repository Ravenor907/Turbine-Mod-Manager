// The WebSocket link to Turbine: hello with the token, keepalive pings, reconnect with growing waits.
// Everything outside (WebSocket, timers, stored pairing) is passed in so the tests can run it in Node.
(function (root) {
  "use strict";

  const P = typeof module !== "undefined" && module.exports ? require("./protocol.js") : root.TurbineProtocol;

  const PING_MS = 20000; // Chrome keeps the service worker alive while the socket sees traffic within 30 s
  const WELCOME_MS = 10000;
  const BACKOFF_MS = [1000, 2000, 4000, 8000, 15000, 30000];

  // Connection states: unpaired, connecting, connected, offline (Turbine not running), refused (token rejected).
  function createBridge(opts) {
    const WS = opts.WebSocket;
    const t = opts.timers || globalThis;
    const backoff = opts.backoffMs || BACKOFF_MS;
    const pingMs = opts.pingMs || PING_MS;
    const welcomeMs = opts.welcomeMs || WELCOME_MS;

    let ws = null;
    let conn = "connecting";
    let welcome = null;
    let lastError = null;
    let attempt = 0;
    let retryTimer = null;
    let pingTimer = null;
    let welcomeTimer = null;
    let stopped = true;
    let generation = 0;

    function setState(next, error) {
      if (error !== undefined) lastError = error;
      conn = next;
      if (opts.onState) opts.onState(getState());
    }

    function getState() {
      return { conn, welcome, error: lastError };
    }

    function clearTimers() {
      if (retryTimer) t.clearTimeout(retryTimer);
      if (pingTimer) t.clearInterval(pingTimer);
      if (welcomeTimer) t.clearTimeout(welcomeTimer);
      retryTimer = pingTimer = welcomeTimer = null;
    }

    function scheduleRetry() {
      if (stopped || retryTimer) return;
      const wait = backoff[Math.min(attempt, backoff.length - 1)];
      attempt += 1;
      retryTimer = t.setTimeout(() => {
        retryTimer = null;
        connect();
      }, wait);
    }

    function dropSocket() {
      const old = ws;
      ws = null;
      generation += 1;
      if (old) {
        try {
          old.close();
        } catch (e) {
          /* already closed */
        }
      }
    }

    async function connect() {
      if (stopped) return;
      dropSocket();
      clearTimers();
      const gen = generation;
      let pairing;
      try {
        pairing = await opts.loadPairing();
      } catch (e) {
        // The browser couldn't read the stored pairing: try again, not "Connecting" for ever.
        if (stopped || gen !== generation) return;
        setState("offline", "The browser could not read the pairing with Turbine");
        scheduleRetry();
        return;
      }
      if (stopped || gen !== generation) return;
      if (!pairing || !P.isPort(pairing.port) || !P.isToken(pairing.token)) {
        welcome = null;
        setState("unpaired", null);
        return; // nothing to retry until the pairing page stores a token
      }
      setState("connecting");
      let sock;
      try {
        sock = new WS("ws://127.0.0.1:" + pairing.port + "/bridge");
      } catch (e) {
        setState("offline", "The browser could not open the link to Turbine");
        scheduleRetry();
        return;
      }
      ws = sock;
      let opened = false;
      let welcomed = false;

      sock.onopen = () => {
        if (gen !== generation) return;
        opened = true;
        try {
          sock.send(JSON.stringify(P.build.hello(pairing.token, opts.extVersion || "0")));
        } catch (e) {
          /* the socket closed under us; onclose follows and retries */
        }
        welcomeTimer = t.setTimeout(() => {
          welcomeTimer = null;
          if (gen === generation && !welcomed) {
            lastError = "Turbine did not answer";
            try {
              sock.close();
            } catch (e) {
              /* ignore */
            }
          }
        }, welcomeMs);
      };

      sock.onmessage = (ev) => {
        if (gen !== generation) return;
        const msg = P.validateIncoming(typeof ev.data === "string" ? ev.data : String(ev.data));
        if (!msg) return;
        if (msg.type === "welcome") {
          welcomed = true;
          welcome = msg;
          attempt = 0;
          if (welcomeTimer) t.clearTimeout(welcomeTimer);
          welcomeTimer = null;
          if (pingTimer) t.clearInterval(pingTimer);
          pingTimer = t.setInterval(() => send(P.build.ping()), pingMs);
          setState("connected", null);
        } else if (msg.type === "error" && !welcomed) {
          lastError = msg.text;
        }
        if (opts.onMessage) opts.onMessage(msg);
      };

      sock.onerror = () => {
        /* onclose follows and handles it */
      };

      sock.onclose = () => {
        if (gen !== generation) return;
        ws = null;
        clearTimers();
        welcome = null;
        if (opened && !welcomed) setState("refused", lastError || "Turbine did not accept this browser. Pair again from Turbine.");
        else setState("offline", welcomed ? null : lastError);
        scheduleRetry();
      };
    }

    function send(msg) {
      if (!ws || conn !== "connected" || !msg) return false;
      try {
        ws.send(JSON.stringify(msg));
        return true;
      } catch (e) {
        return false;
      }
    }

    function start() {
      if (!stopped) return;
      stopped = false;
      connect();
    }

    // Connect now (new pairing, or the player asked); also wakes a waiting retry.
    function reconnect() {
      stopped = false;
      attempt = 0;
      connect();
    }

    // Start if idle, or retry right away if we are only waiting between attempts.
    function nudge() {
      if (stopped) return start();
      if (conn === "offline" || conn === "refused" || conn === "unpaired") {
        if (retryTimer || conn === "unpaired") reconnect();
      }
    }

    function stop() {
      stopped = true;
      clearTimers();
      dropSocket();
      setState("offline");
    }

    return { start, stop, reconnect, nudge, send, getState };
  }

  const api = { createBridge, PING_MS, BACKOFF_MS };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else root.TurbineBridge = api;
})(typeof globalThis !== "undefined" ? globalThis : this);
