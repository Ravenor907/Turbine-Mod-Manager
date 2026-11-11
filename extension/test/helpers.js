// Small helpers shared by the extension's tests.
"use strict";

const fs = require("fs");
const path = require("path");
const vm = require("vm");

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function until(fn, ms, what) {
  const end = Date.now() + (ms || 3000);
  while (Date.now() < end) {
    if (fn()) return;
    await sleep(10);
  }
  throw new Error("timed out: " + (what || "condition"));
}

// Runs the helper's real service worker (background.js) in a fake browser with this chrome API and WebSocket.
function runWorker(chrome, WebSocket) {
  const dir = path.join(__dirname, "..");
  const ctx = vm.createContext({ chrome, WebSocket, URL, setTimeout, clearTimeout, setInterval, clearInterval, console, JSON, Date, Math });
  ctx.globalThis = ctx;
  ctx.importScripts = (...files) => {
    for (const f of files) vm.runInContext(fs.readFileSync(path.join(dir, f), "utf8"), ctx, { filename: f });
  };
  vm.runInContext(fs.readFileSync(path.join(dir, "background.js"), "utf8"), ctx, { filename: "background.js" });
  return ctx;
}

module.exports = { sleep, until, runWorker };
