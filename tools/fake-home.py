#!/usr/bin/env python3
"""Builds a fake SteamOS home folder for the build's screenshot checks: an internal Steam library, an SD card library
and Heroic (Flatpak) with Epic, GOG and a game added by hand. Cover art is plain generated pictures, never real art.
Usage: fake-home.py <folder>"""
import os
import struct
import sys
import zlib


def png(path, w, h, top, bottom):
    rows = b""
    for y in range(h):
        t = y / max(1, h - 1)
        px = bytes(int(top[i] + (bottom[i] - top[i]) * t) for i in range(3))
        rows += b"\x00" + px * w
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
    data = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
    data += chunk(b"IDAT", zlib.compress(rows, 9)) + chunk(b"IEND", b"")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        f.write(data)


def write(path, text=""):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w") as f:
        f.write(text)


def manifest(app_id, name, folder):
    return f'"AppState"\n{{\n\t"appid"\t\t"{app_id}"\n\t"name"\t\t"{name}"\n\t"StateFlags"\t\t"4"\n\t"installdir"\t\t"{folder}"\n\t"buildid"\t\t"18045112"\n}}\n'


def main(home):
    steam = os.path.join(home, ".local/share/Steam")
    sd = os.path.join(home, "sdcard")
    write(os.path.join(steam, "steamapps/libraryfolders.vdf"),
          f'"libraryfolders"\n{{\n"0"\n{{\n"path" "{steam}"\n}}\n"1"\n{{\n"path" "{sd}"\n}}\n}}\n')
    games = [
        (steam, "489830", "The Elder Scrolls V: Skyrim Special Edition", "Skyrim Special Edition", "SkyrimSE.exe", True, (70, 90, 120), (20, 25, 35)),
        (steam, "1966720", "Lethal Company", "Lethal Company", "Lethal Company.exe", True, (150, 60, 40), (30, 15, 10)),
        (sd, "892970", "Valheim", "Valheim", "valheim.x86_64", False, (60, 110, 100), (15, 30, 28)),
        (sd, "632360", "Risk of Rain 2", "Risk of Rain 2", "Risk of Rain 2.exe", True, (120, 80, 150), (25, 15, 35)),
    ]
    for lib, app_id, name, folder, exe, proton, top, bottom in games:
        apps = os.path.join(lib, "steamapps")
        write(os.path.join(apps, f"appmanifest_{app_id}.acf"), manifest(app_id, name, folder))
        write(os.path.join(apps, "common", folder, exe))
        if proton:
            os.makedirs(os.path.join(apps, "compatdata", app_id, "pfx"), exist_ok=True)
        png(os.path.join(steam, "appcache/librarycache", app_id, "library_600x900.png"), 60, 90, top, bottom)
    heroic = os.path.join(home, ".var/app/com.heroicgameslauncher.hgl/config/heroic")
    cp = os.path.join(home, "Games/Heroic/Cyberpunk 2077")
    sv = os.path.join(home, "Games/Heroic/Stardew Valley")
    write(os.path.join(cp, "bin/x64/Cyberpunk2077.exe"))
    write(os.path.join(sv, "StardewValley"))
    write(os.path.join(heroic, "legendaryConfig/legendary/installed.json"),
          '{"Ginger": {"title": "Cyberpunk 2077", "install_path": "%s", "platform": "Windows", "version": "2.3"}}' % cp)
    write(os.path.join(heroic, "gog_store/installed.json"),
          '{"installed": [{"appName": "1453375253", "install_path": "%s", "platform": "linux", "version": "1.6.15"}]}' % sv)
    write(os.path.join(heroic, "store_cache/gog_library.json"), '{"games": [{"app_name": "1453375253", "title": "Stardew Valley"}]}')


if __name__ == "__main__":
    main(os.path.abspath(sys.argv[1]))
