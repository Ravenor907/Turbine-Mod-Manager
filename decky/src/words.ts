// The panel's words and small decisions, kept apart from Steam and Decky so they can be tested on their own
// (tests/words.test.ts).

/** Turbine's brass, from its icon. */
export const BRASS = { light: "#E6C67E", mid: "#C99E52", dark: "#8F6A2C", ground: "#141109" };

export type Launch = { at: number; how: string; result: string; seconds: number | null; words?: string };

export type Status = {
  ok: boolean;
  found?: boolean;
  words?: string;
  missing?: boolean;
  game?: { id: string; name: string; store: string; storeId: string };
  modded?: boolean;
  mods?: number;
  on?: number;
  held?: number;
  modsOn?: boolean;
  coop?: boolean;
  coopOn?: boolean;
  refused?: boolean;
  antiCheat?: string;
  running?: boolean | null;
  turbineOpen?: boolean;
  turbineUrl?: string;
  lastLaunch?: Launch | null;
  pluginLaunch?: Launch | null;
};

function count(n: number, one: string, many: string): string {
  return n + " " + (n === 1 ? one : many);
}

/** The line under the game's name. */
export function statusLine(s: Status): string {
  if (!s.ok) return s.words || "Turbine didn't answer.";
  if (s.found === false) return "Not a game Turbine sees (Turbine itself, or a game it can't find)";
  if (s.refused) return "No mods (online anti-cheat)";
  const mods = s.mods ?? 0;
  const on = s.on ?? 0;
  if (!s.modded || mods === 0) return "Not modded with Turbine";
  if (on === 0) return "Modded with Turbine: " + count(mods, "mod", "mods") + ", all off";
  return "Modded with Turbine: " + count(mods, "mod", "mods") + (on === mods ? ", all on" : ", " + on + " on");
}

/** A Steam library page's app id, from the main window's address ("/library/app/489830"). */
export function gameFromPath(path: string): string | null {
  const m = /\/library\/app\/(\d{1,20})(?:\/|$)/.exec(path || "");
  return m ? m[1] : null;
}

/** Turbine's record of the last launch, or the panel's own (it watches games it started): the newer one. The panel's
 * wins a tie (within a minute of each other), since it knows how the game ended. */
export function newestLaunch(turbine: Launch | null, plugin: Launch | null): Launch | null {
  if (!turbine) return plugin;
  if (!plugin) return turbine;
  return plugin.at + 60 >= turbine.at ? plugin : turbine;
}

function ago(at: number, nowMs: number): string {
  const secs = Math.max(0, Math.round(nowMs / 1000 - at));
  if (secs < 90) return "just now";
  const mins = Math.round(secs / 60);
  if (mins < 90) return mins + " minutes ago";
  const hours = Math.round(mins / 60);
  if (hours < 36) return count(hours, "hour", "hours") + " ago";
  return count(Math.round(hours / 24), "day", "days") + " ago";
}

/** "With mods, 5 minutes ago: closed after 12 seconds. A mod may have stopped it." */
export function describeLaunch(l: Launch, nowMs: number): string {
  const how = l.how === "coop" ? "Seamless Co-op" : l.how === "plain" ? "Without mods" : "With mods";
  let what: string;
  switch (l.result) {
    case "started":
      what = "asked Steam to start it";
      break;
    case "running":
      what = "it started";
      break;
    case "closed":
      what = l.seconds === null || l.seconds === undefined ? "it ran and closed" : "it closed after " + (l.seconds < 120 ? count(l.seconds, "second", "seconds") : count(Math.round(l.seconds / 60), "minute", "minutes"));
      break;
    case "not-started":
      what = "it didn't start";
      break;
    default:
      what = "it didn't start";
  }
  let s = how + ", " + ago(l.at, nowMs) + ": " + what + ".";
  const quick = l.result === "closed" && l.seconds !== null && l.seconds !== undefined && l.seconds < 60;
  if (l.how !== "plain" && (quick || l.result === "not-started")) s += " A mod may have stopped it: try Play without mods, or Report a problem.";
  return s;
}
