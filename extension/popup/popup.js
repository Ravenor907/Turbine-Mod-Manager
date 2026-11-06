// Toolbar popup: connection state, the current status and how to pair again.
/* global chrome */
"use strict";

const CONN = {
  connected: ["Connected to Turbine", "ok"],
  connecting: ["Connecting to Turbine...", "busy"],
  offline: ["Turbine is not running", ""],
  refused: ["Turbine did not accept this browser. Pair again.", "warn"],
  unpaired: ["Not paired with Turbine yet", "warn"],
};

const $ = (id) => document.getElementById(id);
const port = chrome.runtime.connect({ name: "turbine-popup" });

port.onMessage.addListener((s) => {
  if (!s || s.kind !== "state") return;
  const [text, level] = CONN[s.conn] || CONN.offline;
  $("conn").textContent = text;
  $("dot").className = "dot " + level;
  $("version").textContent = s.conn === "connected" && s.turbineVersion ? "Turbine " + s.turbineVersion : s.error || "";
  const st = s.conn === "connected" ? s.status : null;
  $("statusBox").hidden = !st && !s.choices;
  if (s.choices) $("status").textContent = "Turbine needs your install choices. Open the Turbine panel on the Nexus page.";
  else if (st) $("status").textContent = st.text;
  $("game").textContent = st && st.game ? "Game: " + st.game : "";
  const hasBar = !!st && typeof st.progress === "number";
  $("bar").hidden = !hasBar;
  if (hasBar) $("fill").style.width = Math.round(st.progress * 100) + "%";
  $("retry").hidden = s.conn === "connected";
});

port.postMessage({ kind: "hello" });
$("retry").addEventListener("click", () => port.postMessage({ kind: "reconnect" }));
