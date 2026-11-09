"""The backend's pure parts: the arguments Turbine gets, the environment, reading its answer, the launch record.
Run with: python3 -m unittest discover -s tests (from decky/). Decky's own module is stood in for here."""

import asyncio
import json
import os
import sys
import tempfile
import types
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.dirname(HERE))

fake = types.ModuleType("decky")
fake.DECKY_USER_HOME = tempfile.mkdtemp()
fake.DECKY_PLUGIN_SETTINGS_DIR = tempfile.mkdtemp()


class _Log:
    def info(self, *a):
        pass

    def warning(self, *a):
        pass


fake.logger = _Log()
sys.modules["decky"] = fake

import main  # noqa: E402


class Args(unittest.TestCase):
    def test_each_request(self):
        self.assertEqual(main.request_args("status", "489830"), ["--status", "489830", "--json"])
        self.assertEqual(main.request_args("mods-off", "steam:489830"), ["--mods", "off", "steam:489830", "--json"])
        self.assertEqual(main.request_args("mods-on", 489830), ["--mods", "on", "489830", "--json"])
        self.assertEqual(main.request_args("play", "1245620", ["--anyway"]), ["--play", "1245620", "--json", "--anyway"])
        self.assertEqual(main.request_args("play-coop", "1245620")[0], "--play-coop")
        self.assertEqual(main.request_args("report", "Elden Ring"), ["--report", "Elden Ring", "--json"])

    def test_refuses_what_isnt_a_game(self):
        for bad in ["", "   ", "--open", "-x", "a\nb", "x" * 201, None, True, ["1"], {"a": 1}]:
            with self.assertRaises(ValueError, msg=repr(bad)):
                main.request_args("status", bad)
        with self.assertRaises(ValueError):
            main.request_args("rm", "1")


class Env(unittest.TestCase):
    def test_decky_libraries_dont_reach_flatpak(self):
        env = main.host_env({"LD_LIBRARY_PATH": "/tmp/_MEI123", "PATH": "/usr/bin"}, "/home/deck", 1000)
        self.assertNotIn("LD_LIBRARY_PATH", env)
        self.assertEqual(env["HOME"], "/home/deck")
        self.assertEqual(env["XDG_RUNTIME_DIR"], "/run/user/1000")
        self.assertEqual(env["DBUS_SESSION_BUS_ADDRESS"], "unix:path=/run/user/1000/bus")
        self.assertEqual(env["PATH"], "/usr/bin")

    def test_the_original_library_path_comes_back(self):
        env = main.host_env({"LD_LIBRARY_PATH": "/tmp/_MEI", "LD_LIBRARY_PATH_ORIG": "/opt/lib", "XDG_RUNTIME_DIR": "/run/user/1001"}, "/home/a", 1001)
        self.assertEqual(env["LD_LIBRARY_PATH"], "/opt/lib")
        self.assertNotIn("LD_LIBRARY_PATH_ORIG", env)
        self.assertEqual(env["DBUS_SESSION_BUS_ADDRESS"], "unix:path=/run/user/1001/bus")


class Answer(unittest.TestCase):
    def test_reads_the_last_json_line(self):
        out = "some log\n{\"ok\":true,\"found\":true}\n"
        self.assertEqual(main.parse_answer(0, out, ""), {"ok": True, "found": True})
        self.assertEqual(main.parse_answer(1, "{\"ok\":false,\"words\":\"Close it first.\"}", "")["words"], "Close it first.")

    def test_no_answer_is_said_plainly(self):
        a = main.parse_answer(1, "", "error: app/io.github.ravenor907.TurbineModManager/x86_64/master not installed")
        self.assertFalse(a["ok"])
        self.assertTrue(a.get("missing"))
        a = main.parse_answer(127, "", "flatpak not found")
        self.assertTrue(a.get("missing"))
        a = main.parse_answer(124, "{broken", "it took longer than 45 seconds")
        self.assertFalse(a["ok"])
        self.assertIn("longer", a["words"])
        self.assertNotIn("\u2014", a["words"])


class Launches(unittest.TestCase):
    def test_record_follows_one_launch(self):
        s = main.launch_record({}, "steam:1", "mods", "started", None, 1000)
        self.assertEqual(s["steam:1"], {"at": 1000, "how": "mods", "result": "started", "seconds": None})
        s = main.launch_record(s, "steam:1", "mods", "closed", 12, 1040)
        self.assertEqual(s["steam:1"], {"at": 1000, "how": "mods", "result": "closed", "seconds": 12})
        # A new Play starts a new record.
        s = main.launch_record(s, "steam:1", "plain", "started", None, 2000)
        self.assertEqual(s["steam:1"]["at"], 2000)

    def test_bad_input_changes_nothing(self):
        s = {"steam:1": {"at": 1, "how": "mods", "result": "started", "seconds": None}}
        for args in [("", "mods", "closed"), ("steam:1", "both", "closed"), ("steam:1", "mods", "exploded")]:
            self.assertEqual(main.launch_record(s, *args, seconds=1, now=5), s)
        self.assertEqual(main.launch_record("junk", "steam:2", "mods", "started", None, 5)["steam:2"]["at"], 5)

    def test_kept_small(self):
        s = {}
        for i in range(60):
            s = main.launch_record(s, "steam:%d" % i, "mods", "started", None, i)
        self.assertEqual(len(s), 50)
        self.assertNotIn("steam:0", s)
        self.assertIn("steam:59", s)


class Plugin(unittest.TestCase):
    def test_runs_flatpak_with_fixed_arguments(self):
        p = main.Plugin()
        seen = []

        async def fake_run(argv, timeout):
            seen.append(argv)
            return 0, json.dumps({"ok": True, "found": True, "game": {"id": "steam:7"}}), ""

        p._run = fake_run
        a = asyncio.run(p.status("7"))
        self.assertTrue(a["ok"])
        self.assertEqual(seen[0], ["flatpak", "run", main.APP_ID, "--status", "7", "--json"])
        self.assertIn("pluginLaunch", a)

    def test_a_shortcut_is_tried_by_name(self):
        p = main.Plugin()
        seen = []

        async def fake_run(argv, timeout):
            seen.append(argv[4])
            found = argv[4] == "Hades"
            return 0, json.dumps({"ok": True, "found": found, "game": {"id": "epic:x"} if found else None}), ""

        p._run = fake_run
        a = asyncio.run(p.status("3000000001", "Hades"))
        self.assertEqual(seen, ["3000000001", "Hades"])
        self.assertTrue(a["found"])

    def test_refuses_unknown_play_and_links(self):
        p = main.Plugin()
        self.assertFalse(asyncio.run(p.play("1", "everything"))["ok"])
        self.assertFalse(asyncio.run(p.open_url("https://example.com")))
        self.assertFalse(asyncio.run(p.open_url("heroic://launch?appName=a b")))

    def test_note_launch_is_kept(self):
        p = main.Plugin()
        self.assertTrue(asyncio.run(p.note_launch("steam:9", "coop", "started")))
        self.assertTrue(asyncio.run(p.note_launch("steam:9", "coop", "closed", 30)))
        self.assertEqual(p._read_launches()["steam:9"]["seconds"], 30)


if __name__ == "__main__":
    unittest.main()
