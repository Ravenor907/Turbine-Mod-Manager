// Turbine in Steam's Quick Access menu (optional; needs Decky Loader). For the game you're in (or the game page you
// last opened): is it modded with Turbine, Mods on or off, Play with mods, Play Seamless Co-op, Play without mods,
// Open Turbine on it, and how the last launch went. Every change is Turbine's own: the backend (main.py) asks
// Turbine's Flatpak with one request and shows its answer.

import { ButtonItem, Field, PanelSection, PanelSectionRow, Router, staticClasses, ToggleField } from "@decky/ui";
import { callable, definePlugin, toaster } from "@decky/api";
import { useEffect, useRef, useState } from "react";
import { ICON } from "./icon";
import { BRASS, describeLaunch, gameFromPath, newestLaunch, statusLine, type Launch, type Status } from "./words";

type Answer = { ok?: boolean; words?: string; launch?: string; how?: string; checks?: { level: string; title: string }[]; ask?: boolean; needsTurbine?: boolean; sent?: boolean; turbineUrl?: string; missing?: boolean; password?: boolean };

const getStatus = callable<[game: string, name: string], Status>("status");
const setMods = callable<[game: string, on: boolean], Answer>("mods");
const playGame = callable<[game: string, how: string, anyway: boolean], Answer>("play");
const openTurbine = callable<[game: string, report: boolean], Answer>("open_turbine");
const noteLaunch = callable<[gameId: string, how: string, result: string, seconds: number | null], boolean>("note_launch");
const openUrl = callable<[url: string], boolean>("open_url");

// ---------------------------------------------------------------------------------------------- Steam's side
// Steam's own objects, reached loosely: they are Steam's internals and can change with a Steam update, so every use
// is guarded and the panel says so plainly when one is missing.

type SteamApps = { RunGame?: (gameId: string, launchOptions: string, a: number, b: number) => void };
type SteamUrl = { ExecuteSteamURL?: (url: string) => void };
type SteamSessions = { RegisterForAppLifetimeNotifications?: (cb: (n: { unAppID: number; bRunning: boolean }) => void) => { unregister: () => void } };
type SteamClientish = { Apps?: SteamApps; URL?: SteamUrl; GameSessions?: SteamSessions };

function steam(): SteamClientish | undefined {
  return (window as unknown as { SteamClient?: SteamClientish }).SteamClient;
}

/** The game in front: the running one, else the game page open in the library, else the last one seen. */
let lastSeen: { id: string; name: string } | null = null;
function currentGame(): { id: string; name: string } | null {
  try {
    const r = Router as unknown as { MainRunningApp?: { appid?: number | string; display_name?: string } };
    const app = r.MainRunningApp;
    if (app && app.appid) {
      lastSeen = { id: String(app.appid), name: app.display_name || "" };
      return lastSeen;
    }
  } catch (e) {
    console.warn("Turbine: no running app", e);
  }
  try {
    const w = Router as unknown as { WindowStore?: { GamepadUIMainWindowInstance?: { m_history?: { location?: { pathname?: string } } } } };
    const id = gameFromPath(w.WindowStore?.GamepadUIMainWindowInstance?.m_history?.location?.pathname || "");
    if (id) {
      const store = (window as unknown as { appStore?: { GetAppOverviewByAppID?: (id: number) => { display_name?: string } | null } }).appStore;
      lastSeen = { id, name: store?.GetAppOverviewByAppID?.(Number(id))?.display_name || "" };
      return lastSeen;
    }
  } catch (e) {
    console.warn("Turbine: no library page", e);
  }
  return lastSeen;
}

/** Opens a steam:// link (a game, or Turbine's own entry in Steam) the way Steam's own buttons do. */
function runSteamUrl(url: string): boolean {
  const sc = steam();
  const id = /^steam:\/\/rungameid\/(\d+)$/.exec(url);
  try {
    if (id && sc?.Apps?.RunGame) {
      sc.Apps.RunGame(id[1], "", -1, 100);
      return true;
    }
    if (sc?.URL?.ExecuteSteamURL) {
      sc.URL.ExecuteSteamURL(url);
      return true;
    }
  } catch (e) {
    console.warn("Turbine: Steam didn't take " + url, e);
  }
  return false;
}

async function startLink(url: string): Promise<boolean> {
  if (url.startsWith("steam://")) return runSteamUrl(url);
  if (url.startsWith("heroic://")) return openUrl(url);
  return false;
}

// ---------------------------------------------------------------------------------------------- the last launch
// A game started from this panel is watched here (Turbine has no window then to watch it): running, then closed
// after so many seconds. Steam tells the plugin when an app starts or stops.

let watching: { appid: number; gameId: string; how: string; at: number; running: boolean } | null = null;
let lifetime: { unregister: () => void } | null = null;

/** Keeps the last launch; a failure to keep it only goes to the log (the panel still works without it). */
function keepLaunch(gameId: string, how: string, result: string, seconds: number | null) {
  noteLaunch(gameId, how, result, seconds).catch((e) => console.warn("Turbine: the last launch wasn't kept", e));
}

const NO_ANSWER = "Turbine's panel couldn't reach Turbine. Close this menu and open it again; if it stays, restart Steam.";

function watchLaunch(appid: string, gameId: string, how: string) {
  const n = Number(appid);
  if (!Number.isFinite(n) || n <= 0) return;
  watching = { appid: n, gameId, how, at: Date.now(), running: false };
  keepLaunch(gameId, how, "started", null);
}

function onLifetime(n: { unAppID: number; bRunning: boolean }) {
  const w = watching;
  if (!w || n.unAppID !== w.appid) return;
  if (n.bRunning && !w.running) {
    w.running = true;
    w.at = Date.now();
    keepLaunch(w.gameId, w.how, "running", null);
  } else if (!n.bRunning && w.running) {
    const secs = Math.round((Date.now() - w.at) / 1000);
    watching = null;
    keepLaunch(w.gameId, w.how, "closed", secs);
    if (w.how !== "plain" && secs < 60) {
      toaster.toast({ title: "Turbine", body: "The game closed after " + secs + " seconds. A mod may have stopped it: open Turbine's panel for Play without mods or Report a problem." });
    }
  }
}

// ---------------------------------------------------------------------------------------------- the panel

function Content() {
  const [game, setGame] = useState<{ id: string; name: string } | null>(currentGame());
  const [status, setStatus] = useState<Status | null>(null);
  const [busy, setBusy] = useState<string>("");
  const [words, setWords] = useState<string>("");
  const [checks, setChecks] = useState<{ how: string; list: NonNullable<Answer["checks"]> } | null>(null);

  // The panel closes while a request is still out: nothing is set on it after that, and its timer is dropped.
  const live = useRef(true);
  const later = useRef<ReturnType<typeof setTimeout> | null>(null);

  const refresh = async (g = game) => {
    if (!g) return;
    try {
      const s = await getStatus(g.id, g.name);
      if (!live.current) return;
      setStatus(s && typeof s === "object" ? s : { ok: false, words: NO_ANSWER });
    } catch (e) {
      console.warn("Turbine: status", e);
      if (live.current) setStatus({ ok: false, words: NO_ANSWER });
    }
  };

  useEffect(() => {
    live.current = true;
    const g = currentGame();
    setGame(g);
    void refresh(g);
    return () => {
      live.current = false;
      if (later.current) clearTimeout(later.current);
      later.current = null;
    };
  }, []);

  const gid = status?.found && status.game ? status.game.id : "";
  const target = gid || game?.id || "";
  const running = status?.running === true;
  const launch: Launch | null = newestLaunch(status?.lastLaunch ?? null, status?.pluginLaunch ?? null);

  // One request at a time; the answer's words show under the buttons, and the status reads again.
  const act = async (what: string, run: () => Promise<Answer>, after?: (a: Answer) => Promise<void> | void) => {
    if (busy || !target) return;
    setBusy(what);
    setWords("");
    setChecks(null);
    try {
      const a = await run();
      if (!live.current) return;
      if (!a || typeof a !== "object") {
        setWords(NO_ANSWER);
      } else if (a.missing) {
        setWords(a.words || "Turbine Mod Manager isn't installed.");
      } else {
        setWords(a.words || "");
        if (after) await after(a);
        // Turbine is open and does it there: read again once it has had time to.
        if (a.sent && live.current) {
          if (later.current) clearTimeout(later.current);
          later.current = setTimeout(() => {
            later.current = null;
            void refresh();
          }, 4000);
        }
      }
    } catch (e) {
      console.warn("Turbine: " + what, e);
      if (live.current) setWords(NO_ANSWER);
    } finally {
      if (live.current) {
        setBusy("");
        void refresh();
      }
    }
  };

  const play = (how: string, anyway = false) =>
    act(how === "plain" ? "Starting without mods..." : how === "coop" ? "Starting Seamless Co-op..." : "Getting the mods ready...", () => playGame(target, how, anyway), async (a) => {
      if (a.ask && a.checks && a.checks.length > 0) {
        setChecks({ how, list: a.checks });
        return;
      }
      if (a.launch) {
        if (await startLink(a.launch)) {
          if (game && gid) watchLaunch(game.id, gid, a.how || how);
          setWords(a.words || "Starting the game.");
        } else {
          setWords("Steam didn't start the game. Start it from its page; Turbine has the mods ready.");
        }
      } else if (a.sent) {
        // Turbine is open and does it there: bring it to the front so its questions show.
        if (a.turbineUrl) runSteamUrl(a.turbineUrl);
      } else if (a.password === false && a.turbineUrl) {
        // Seamless Co-op has no session password: Turbine opens on its settings, and goes on once it is set.
        if (!runSteamUrl(a.turbineUrl)) setWords("Set the Seamless Co-op session password in Turbine first. Start Turbine from your library.");
      }
    });

  const open = (report: boolean) =>
    act(report ? "Opening Report a problem..." : "Opening Turbine...", () => openTurbine(target, report), (a) => {
      if (a.turbineUrl) {
        if (!runSteamUrl(a.turbineUrl)) setWords("Steam didn't open Turbine. Start Turbine from your library.");
      }
    });

  if (!game) {
    return (
      <PanelSection title="Turbine">
        <PanelSectionRow>
          <Field label="No game yet" description="Start a game, or open a game's page in your library, then open this panel again." />
        </PanelSectionRow>
      </PanelSection>
    );
  }

  const name = status?.game?.name || game.name || "This game";
  const refused = status?.refused === true;
  const known = status?.ok === true && status.found === true;

  return (
    <>
      <PanelSection>
        <PanelSectionRow>
          <div style={{ display: "flex", alignItems: "center", gap: "10px", padding: "6px 0" }}>
            <img src={ICON} width={36} height={36} style={{ borderRadius: "8px", boxShadow: "0 0 0 1px " + BRASS.dark }} />
            <div style={{ minWidth: 0 }}>
              <div style={{ color: BRASS.light, fontWeight: 700, fontSize: "15px", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{name}</div>
              <div style={{ color: BRASS.mid, fontSize: "12px" }}>{status ? statusLine(status) : "Asking Turbine..."}</div>
            </div>
          </div>
        </PanelSectionRow>
        {status?.words ? (
          <PanelSectionRow>
            <div style={{ fontSize: "12px", color: "#dcd3c0", padding: "2px 0 6px" }}>{status.words}</div>
          </PanelSectionRow>
        ) : null}
      </PanelSection>

      {known ? (
        <PanelSection title="Mods">
          <PanelSectionRow>
            <ToggleField
              label="Mods on"
              description={
                refused
                  ? "This game's anti-cheat is online only: no mods."
                  : running
                    ? "Close the game first to change its mods."
                    : (status?.mods ?? 0) === 0
                      ? "No mods yet: Open Turbine to add some."
                      : (status?.on ?? 0) > 0
                        ? "Off takes every mod out of the game. On brings back exactly these."
                        : (status?.held ?? 0) > 0
                          ? "On brings back the mods that were on."
                          : "Every mod is off. Choose them in Turbine."
              }
              checked={(status?.on ?? 0) > 0}
              disabled={!!busy || refused || running || ((status?.on ?? 0) === 0 && (status?.held ?? 0) === 0)}
              onChange={(on: boolean) => void act(on ? "Turning mods on..." : "Turning mods off...", () => setMods(target, on))}
            />
          </PanelSectionRow>
        </PanelSection>
      ) : null}

      {known ? (
        <PanelSection title="Play">
          {checks ? (
            <>
              <PanelSectionRow>
                <Field label="Before you play" description={checks.list.map((c) => c.title).join(". ") + ". Open Turbine to see each one, or play anyway."} />
              </PanelSectionRow>
              <PanelSectionRow>
                <ButtonItem layout="below" disabled={!!busy} onClick={() => void play(checks.how, true)}>
                  Play anyway
                </ButtonItem>
              </PanelSectionRow>
            </>
          ) : null}
          {!refused ? (
            <PanelSectionRow>
              <ButtonItem layout="below" disabled={!!busy || running} onClick={() => void play("mods")}>
                Play with mods
              </ButtonItem>
            </PanelSectionRow>
          ) : null}
          {status?.coop ? (
            <PanelSectionRow>
              <ButtonItem layout="below" disabled={!!busy || running} onClick={() => void play("coop")}>
                Play Seamless Co-op
              </ButtonItem>
            </PanelSectionRow>
          ) : null}
          <PanelSectionRow>
            <ButtonItem layout="below" disabled={!!busy || running} onClick={() => void play("plain")}>
              Play without mods
            </ButtonItem>
          </PanelSectionRow>
          {running ? (
            <PanelSectionRow>
              <div style={{ fontSize: "12px", color: BRASS.mid }}>{name} is running. Close it to play it another way.</div>
            </PanelSectionRow>
          ) : null}
        </PanelSection>
      ) : null}

      <PanelSection title="Turbine">
        {busy || words ? (
          <PanelSectionRow>
            <div style={{ fontSize: "12px", color: busy ? BRASS.light : "#dcd3c0", padding: "2px 0 6px" }}>{busy || words}</div>
          </PanelSectionRow>
        ) : null}
        {known && launch ? (
          <PanelSectionRow>
            <Field label="Last launch" description={describeLaunch(launch, Date.now())} />
          </PanelSectionRow>
        ) : null}
        <PanelSectionRow>
          <ButtonItem layout="below" disabled={!!busy} onClick={() => void open(false)}>
            Open Turbine on this game
          </ButtonItem>
        </PanelSectionRow>
        <PanelSectionRow>
          <ButtonItem layout="below" disabled={!!busy} onClick={() => void open(true)}>
            Report a problem
          </ButtonItem>
        </PanelSectionRow>
      </PanelSection>
    </>
  );
}

export default definePlugin(() => {
  try {
    lifetime = steam()?.GameSessions?.RegisterForAppLifetimeNotifications?.(onLifetime) ?? null;
  } catch (e) {
    console.warn("Turbine: Steam's app notifications aren't there", e);
  }
  return {
    name: "Turbine",
    titleView: (
      <div className={staticClasses.Title} style={{ color: BRASS.light }}>
        Turbine
      </div>
    ),
    content: <Content />,
    icon: <img src={ICON} style={{ width: "1em", height: "1em", borderRadius: "3px" }} />,
    onDismount() {
      try {
        lifetime?.unregister();
      } catch (e) {
        console.warn("Turbine: unregister", e);
      }
      lifetime = null;
    },
  };
});
