// The panel's words: node --experimental-strip-types --test tests/words.test.ts (from decky/).
import { test } from "node:test";
import assert from "node:assert/strict";
import { describeLaunch, gameFromPath, newestLaunch, statusLine } from "../src/words.ts";

test("the status line says what Turbine has for the game", () => {
  assert.equal(statusLine({ ok: true, found: true, modded: true, mods: 5, on: 3 }), "Modded with Turbine: 5 mods, 3 on");
  assert.equal(statusLine({ ok: true, found: true, modded: true, mods: 1, on: 1 }), "Modded with Turbine: 1 mod, all on");
  assert.equal(statusLine({ ok: true, found: true, modded: true, mods: 2, on: 0 }), "Modded with Turbine: 2 mods, all off");
  assert.equal(statusLine({ ok: true, found: true, modded: false, mods: 0, on: 0 }), "Not modded with Turbine");
  assert.equal(statusLine({ ok: true, found: true, refused: true, mods: 2, on: 0 }), "No mods (online anti-cheat)");
  assert.equal(statusLine({ ok: true, found: false }), "Not a game Turbine sees");
  assert.equal(statusLine({ ok: false, words: "Turbine Mod Manager isn't installed." }), "Turbine Mod Manager isn't installed.");
});

test("a library page gives its app id", () => {
  assert.equal(gameFromPath("/routes/library/app/489830"), "489830");
  assert.equal(gameFromPath("/library/app/1245620/achievements"), "1245620");
  assert.equal(gameFromPath("/library/home"), null);
  assert.equal(gameFromPath(""), null);
});

test("the newer launch wins, and the panel's own on a tie", () => {
  const t = { at: 1000, how: "mods", result: "started", seconds: null };
  const p = { at: 1010, how: "mods", result: "closed", seconds: 8 };
  assert.equal(newestLaunch(t, null), t);
  assert.equal(newestLaunch(null, p), p);
  assert.equal(newestLaunch(t, p), p);
  assert.equal(newestLaunch({ ...t, at: 5000 }, p).at, 5000);
  assert.equal(newestLaunch(null, null), null);
});

test("the last launch in words", () => {
  const now = 2_000_000 * 1000;
  const quick = describeLaunch({ at: 2_000_000 - 30, how: "mods", result: "closed", seconds: 12 }, now);
  assert.equal(quick, "With mods, just now: it closed after 12 seconds. A mod may have stopped it: try Play without mods, or Report a problem.");
  assert.equal(describeLaunch({ at: 2_000_000 - 600, how: "coop", result: "closed", seconds: 1800 }, now), "Seamless Co-op, 10 minutes ago: it closed after 30 minutes.");
  assert.equal(describeLaunch({ at: 2_000_000 - 7200, how: "plain", result: "closed", seconds: 5 }, now), "Without mods, 2 hours ago: it closed after 5 seconds.");
  assert.match(describeLaunch({ at: 2_000_000, how: "mods", result: "not-started", seconds: null }, now), /didn't start\. A mod may/);
  assert.equal(describeLaunch({ at: 2_000_000, how: "mods", result: "started", seconds: null }, now), "With mods, just now: asked Steam to start it.");
  for (const r of ["started", "running", "closed", "not-started", "failed"]) {
    assert.doesNotMatch(describeLaunch({ at: 1, how: "mods", result: r, seconds: 3 }, now), /\u2014|\u2013/);
  }
});
