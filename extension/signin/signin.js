// Turbine's sign-in page in the player's browser: says that Turbine is waiting for the Nexus sign-in, offers the
// way there, then says "Signed in. Go back to Turbine" (and how). The service worker does the work (lib/signin.js);
// this page only shows its state and passes on the player's presses.
/* global chrome */
(function (root) {
  "use strict";

  // What the page shows for a state snapshot from the service worker: {cls, title, text, how, buttons, key}.
  function model(st) {
    const conn = st ? st.conn : "connecting";
    const g = st ? st.signin : null;
    if (conn === "unpaired" || conn === "refused") {
      return { cls: "warn", title: "This browser isn't paired with Turbine", text: "In Turbine, open Settings, then Accounts, and choose Pair again. Then press Sign in with my browser in Turbine.", how: "", buttons: [], key: false };
    }
    if (!g) {
      if (conn !== "connected") {
        return { cls: "wait", title: "Turbine is waiting for you to sign in to Nexus Mods", text: "Connecting to Turbine...", how: "Turbine has to be running. If this stays here, open Turbine again.", buttons: [{ label: "Connect now", kind: "reconnect", cls: "ghost" }], key: false };
      }
      return { cls: "warn", title: "No sign-in to do", text: "Turbine isn't waiting for a sign-in. To sign in, press Sign in with my browser in Turbine.", how: "", buttons: [], key: false };
    }
    const offline = conn !== "connected" ? " Turbine isn't connected right now; it carries on when it is." : "";
    switch (g.phase) {
      case "waiting":
        return {
          cls: "wait",
          title: "Turbine is waiting for you to sign in to Nexus Mods",
          text: g.text + offline,
          how: g.how || "",
          buttons: [
            { label: "Sign in to Nexus Mods", kind: "signin.go", cls: "" },
            { label: "I'm already signed in", kind: "signin.send", cls: "ghost" },
          ],
          key: false,
        };
      case "sending":
      case "sent":
        return { cls: "wait", title: "Signing in to Turbine", text: g.text + offline, how: "", buttons: [], key: false };
      case "failed":
        return { cls: "warn", title: "Turbine didn't take the sign-in", text: g.text + offline, how: "", buttons: [{ label: "Try again", kind: "signin.send", cls: "" }, { label: "Sign in to Nexus Mods", kind: "signin.go", cls: "ghost" }], key: false };
      case "ok":
        return { cls: "ok", title: "Signed in. Go back to Turbine", text: g.back || "", how: "", buttons: [], key: true };
      case "key-reading":
      case "key-checking":
        return { cls: "ok", title: "Signed in. Go back to Turbine", text: g.text, how: g.back || "", buttons: [], key: false };
      case "key-ok":
        return { cls: "ok", title: "Signed in. Go back to Turbine", text: g.text + " You can close this tab.", how: g.back || "", buttons: [], key: false };
      case "key-missing":
      case "key-failed":
        return { cls: "warn", title: "Signed in. Go back to Turbine", text: g.text, how: g.back || "", buttons: [{ label: "Try again", kind: "key.start", cls: "ghost" }], key: false };
      default:
        return { cls: "warn", title: "This sign-in is over", text: g.text, how: "", buttons: [], key: false };
    }
  }

  if (typeof module !== "undefined" && module.exports) {
    module.exports = { model };
    return;
  }

  const $ = (id) => document.getElementById(id);
  let port = null;
  let sig = "";

  function post(msg) {
    if (!port) return;
    try {
      port.postMessage(msg);
    } catch (e) {
      /* the worker restarted; the port reconnects */
    }
  }

  function render(st) {
    const m = model(st);
    const s = JSON.stringify(m);
    if (s === sig) return;
    sig = s;
    $("state").className = "state " + m.cls;
    $("title").textContent = m.title;
    $("text").textContent = m.text;
    $("how").textContent = m.how;
    document.title = "Turbine: " + m.title;
    const box = $("buttons");
    const had = document.activeElement && box.contains(document.activeElement) ? document.activeElement.dataset.kind : "";
    box.replaceChildren();
    for (const b of m.buttons) {
      const el = document.createElement("button");
      el.type = "button";
      el.textContent = b.label;
      el.className = b.cls;
      el.dataset.kind = b.kind;
      el.addEventListener("click", () => post({ kind: b.kind }));
      box.appendChild(el);
    }
    $("keyBox").hidden = !m.key;
    // The first button has the focus, so a controller's A (or Enter) presses it.
    const again = had && box.querySelector('[data-kind="' + had + '"]');
    if (again) again.focus();
    else if (box.firstChild) box.firstChild.focus();
    else if (m.key) $("keyGo").focus();
  }

  function connect() {
    try {
      port = chrome.runtime.connect({ name: "turbine-signin" });
    } catch (e) {
      return;
    }
    port.onMessage.addListener((msg) => {
      if (msg && msg.kind === "state") render(msg);
    });
    port.onDisconnect.addListener(() => {
      port = null;
      setTimeout(connect, 1000);
    });
    post({ kind: "hello" });
  }

  $("keyGo").addEventListener("click", () => post({ kind: "key.start" }));
  render(null);
  connect();
})(typeof globalThis !== "undefined" ? globalThis : this);
