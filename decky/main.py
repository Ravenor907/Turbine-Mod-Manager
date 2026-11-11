"""Turbine in Steam's Quick Access menu: the plugin's backend.

Decky runs this on the host, as the player (the plugin asks for no root). It runs Turbine's Flatpak with one request
and reads Turbine's one JSON line back:

    flatpak run io.github.ravenor907.TurbineModManager --status <game> --json
    ... --mods on|off <game> --json, --play / --play-coop / --play-plain <game> --json, --open / --report <game> --json

Nothing here changes a game's files itself: Turbine does every change with the same steps its own buttons use.
"""

import asyncio
import json
import os
import time

import decky

APP_ID = "io.github.ravenor907.TurbineModManager"
# How long one request may take: a status reads the games; a Play may copy mod files into the game first.
STATUS_TIMEOUT = 45
CHANGE_TIMEOUT = 300
MAX_GAME = 200
HOWS = {"mods": "--play", "coop": "--play-coop", "plain": "--play-plain"}
LAUNCH_RESULTS = {"started", "running", "closed"}


def clean_game(game):
    """The game as Turbine's command line gets it, or None when it can't be one."""
    if not isinstance(game, (str, int)) or isinstance(game, bool):
        return None
    g = str(game).strip()
    if not g or len(g) > MAX_GAME or g.startswith("-"):
        return None
    if any(ord(c) < 32 or ord(c) == 127 for c in g):
        return None
    return g


def request_args(action, game, extra=None):
    """Turbine's arguments for one request (fixed flags, the game as one argument, never a shell)."""
    flags = {
        "status": ["--status"],
        "mods-on": ["--mods", "on"],
        "mods-off": ["--mods", "off"],
        "play": ["--play"],
        "play-coop": ["--play-coop"],
        "play-plain": ["--play-plain"],
        "open": ["--open"],
        "report": ["--report"],
    }
    if action not in flags:
        raise ValueError("unknown request: %s" % action)
    g = clean_game(game)
    if g is None:
        raise ValueError("that game can't be read")
    return flags[action] + [g, "--json"] + list(extra or [])


def host_env(base, user_home, uid):
    """The environment for programs on the host. Decky's own (a bundled Python) points LD_LIBRARY_PATH at its
    libraries, which would break flatpak; the player's runtime folder and session bus are where Turbine's socket
    and Flatpak's portal are."""
    env = dict(base)
    orig = env.pop("LD_LIBRARY_PATH_ORIG", None)
    if orig:
        env["LD_LIBRARY_PATH"] = orig
    else:
        env.pop("LD_LIBRARY_PATH", None)
    env["HOME"] = user_home
    runtime = "/run/user/%d" % uid
    if not env.get("XDG_RUNTIME_DIR"):
        env["XDG_RUNTIME_DIR"] = runtime
    if not env.get("DBUS_SESSION_BUS_ADDRESS"):
        env["DBUS_SESSION_BUS_ADDRESS"] = "unix:path=%s/bus" % env["XDG_RUNTIME_DIR"]
    return env


def parse_answer(code, out, err):
    """Turbine's answer: its last JSON line, or plain words when there isn't one."""
    for line in reversed((out or "").splitlines()):
        line = line.strip()
        if line.startswith("{"):
            try:
                v = json.loads(line)
            except ValueError:
                continue
            if isinstance(v, dict):
                return v
    text = (err or "").strip().splitlines()
    if code == 127 or any("not installed" in t for t in text):
        return {"ok": False, "missing": True, "words": "Turbine Mod Manager isn't installed. Install it from its release page (Flatpak), then try again."}
    detail = text[-1][:200] if text else "no answer"
    return {"ok": False, "words": "Turbine didn't answer (%s)." % detail}


def launch_record(store, game_id, how, result, seconds, now):
    """The plugin's own record of the last launch it started for a game (Turbine can't watch a game it started
    without its window). Returns the new store."""
    out = dict(store) if isinstance(store, dict) else {}
    if not clean_game(game_id) or how not in HOWS or result not in LAUNCH_RESULTS:
        return out
    prev = out.get(game_id) if isinstance(out.get(game_id), dict) else {}
    at = prev.get("at") if result != "started" and isinstance(prev.get("at"), (int, float)) else int(now)
    rec = {"at": int(at), "how": how, "result": result, "seconds": None}
    if isinstance(seconds, (int, float)) and not isinstance(seconds, bool) and seconds >= 0:
        rec["seconds"] = int(seconds)
    out[game_id] = rec
    # Kept small: the 50 most recent games.
    if len(out) > 50:
        keep = sorted(out.items(), key=lambda kv: kv[1].get("at", 0) if isinstance(kv[1], dict) else 0, reverse=True)[:50]
        out = dict(keep)
    return out


class Plugin:
    lock = None

    def _settings_file(self):
        return os.path.join(decky.DECKY_PLUGIN_SETTINGS_DIR, "launches.json")

    def _read_launches(self):
        try:
            with open(self._settings_file(), "r", encoding="utf-8") as f:
                v = json.load(f)
            return v if isinstance(v, dict) else {}
        except (OSError, ValueError):
            return {}

    def _write_launches(self, store):
        path = self._settings_file()
        os.makedirs(os.path.dirname(path), exist_ok=True)
        tmp = path + ".tmp"
        with open(tmp, "w", encoding="utf-8") as f:
            json.dump(store, f)
        os.replace(tmp, path)

    def _env(self):
        return host_env(os.environ, decky.DECKY_USER_HOME, os.getuid())

    async def _run(self, argv, timeout):
        """Runs one program on the host with fixed arguments and gives (exit code, stdout, stderr)."""
        try:
            proc = await asyncio.create_subprocess_exec(
                *argv,
                stdin=asyncio.subprocess.DEVNULL,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
                env=self._env(),
                cwd=decky.DECKY_USER_HOME,
            )
        except FileNotFoundError:
            return 127, "", "%s not found" % argv[0]
        try:
            out, err = await asyncio.wait_for(proc.communicate(), timeout)
        except asyncio.TimeoutError:
            proc.kill()
            await proc.wait()
            return 124, "", "it took longer than %d seconds" % timeout
        return proc.returncode, out.decode("utf-8", "replace"), err.decode("utf-8", "replace")

    async def _turbine(self, action, game, extra=None, timeout=CHANGE_TIMEOUT):
        try:
            args = request_args(action, game, extra)
        except ValueError as e:
            return {"ok": False, "words": "Can't ask Turbine that: %s." % e}
        # One request at a time: Turbine changes one game's mods at a time too.
        if self.lock is None:
            self.lock = asyncio.Lock()
        async with self.lock:
            started = time.monotonic()
            code, out, err = await self._run(["flatpak", "run", APP_ID] + args, timeout)
            answer = parse_answer(code, out, err)
            decky.logger.info("Turbine %s %s: exit %s in %.1f s: %s", action, game, code, time.monotonic() - started, json.dumps(answer)[:400])
            return answer

    # ---- what the panel calls ----

    async def status(self, game, name=""):
        """What the panel shows for a game: its Steam app id first, then its name (a non-Steam shortcut)."""
        answer = await self._turbine("status", game, timeout=STATUS_TIMEOUT)
        if answer.get("ok") and answer.get("found") is False and clean_game(name):
            answer = await self._turbine("status", name, timeout=STATUS_TIMEOUT)
        if answer.get("found"):
            gid = (answer.get("game") or {}).get("id", "")
            answer["pluginLaunch"] = self._read_launches().get(gid)
        return answer

    async def mods(self, game, on):
        return await self._turbine("mods-on" if on is True else "mods-off", game)

    async def play(self, game, how, anyway=False):
        if how not in HOWS:
            return {"ok": False, "words": "Can't ask Turbine that."}
        action = {"mods": "play", "coop": "play-coop", "plain": "play-plain"}[how]
        return await self._turbine(action, game, ["--anyway"] if anyway is True else None)

    async def open_turbine(self, game, report=False):
        return await self._turbine("report" if report is True else "open", game, timeout=STATUS_TIMEOUT)

    async def note_launch(self, game_id, how, result, seconds=None):
        """The panel saw a game it started run or close: kept for "last launch"."""
        store = launch_record(self._read_launches(), game_id, how, result, seconds, time.time())
        try:
            self._write_launches(store)
        except OSError as e:
            decky.logger.warning("Couldn't keep the last launch: %s", e)
            return False
        return True

    async def open_url(self, url):
        """A Heroic game starts through Heroic's own link (heroic://launch...), opened on the host."""
        if not isinstance(url, str) or not url.startswith("heroic://launch?") or len(url) > 1000 or any(ord(c) < 33 for c in url):
            return False
        code, _, err = await self._run(["xdg-open", url], 20)
        if code != 0:
            decky.logger.warning("xdg-open %s: %s %s", url, code, err.strip()[:200])
        return code == 0

    async def _main(self):
        decky.logger.info("Turbine's Steam menu plugin is ready")

    async def _unload(self):
        decky.logger.info("Turbine's Steam menu plugin stopped")

    async def _uninstall(self):
        pass
