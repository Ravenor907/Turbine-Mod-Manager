#!/usr/bin/env python3
"""Builds a fake SteamOS home folder for the build's screenshot checks: an internal Steam library, an SD card library,
Heroic (Flatpak) with Epic, GOG and a game added by hand, and a few small test mods as zip files in Downloads. Cover art
is plain generated pictures, never real art, and the mods are made up (their "code" is a few placeholder bytes).
Usage: fake-home.py <folder>"""
import json
import os
import struct
import sys
import time
import zipfile
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
        # The first Nexus test games: Cyberpunk 2077 and The Witcher 3 (Windows builds run through Proton).
        (steam, "1091500", "Cyberpunk 2077", "Cyberpunk 2077", "bin/x64/Cyberpunk2077.exe", True, (200, 190, 40), (40, 20, 50)),
        (steam, "292030", "The Witcher 3: Wild Hunt", "The Witcher 3", "bin/x64/witcher3.exe", True, (150, 30, 30), (20, 20, 25)),
        # Kingdom Come: Deliverance I and II (Windows builds through Proton; the second game ships a lower-case mods folder).
        (steam, "379430", "Kingdom Come: Deliverance", "KingdomComeDeliverance", "Bin/Win64/KingdomCome.exe", True, (120, 100, 60), (30, 25, 15)),
        (steam, "1771300", "Kingdom Come: Deliverance II", "KingdomComeDeliverance2", "Bin/Win64MasterMasterSteamPGO/KingdomCome.exe", True, (90, 110, 70), (20, 28, 15)),
        # 0.56, Mods for any game: two made-up games Turbine has no rules for (Basic support).
        (steam, "3999901", "Lantern Hollow", "Lantern Hollow", "Lantern/Binaries/Win64/Lantern-Win64-Shipping.exe", True, (180, 120, 50), (30, 20, 10)),
        (steam, "3999902", "Brass Pier", "Brass Pier", "BrassPier.exe", True, (150, 110, 60), (25, 20, 15)),
    ]
    for lib, app_id, name, folder, exe, proton, top, bottom in games:
        apps = os.path.join(lib, "steamapps")
        write(os.path.join(apps, f"appmanifest_{app_id}.acf"), manifest(app_id, name, folder))
        write(os.path.join(apps, "common", folder, exe))
        if proton:
            os.makedirs(os.path.join(apps, "compatdata", app_id, "pfx"), exist_ok=True)
        png(os.path.join(steam, "appcache/librarycache", app_id, "library_600x900.png"), 60, 90, top, bottom)
    # 0.64: Cyberpunk's Proton setup has only Steam's 2019 Visual C++ runtime (14.29), the case RED4ext and Cyber Engine
    # Tweaks fail with error 998: a vcruntime140.dll with that version in its version resource.
    vc = bytearray(b"MZ" + b"\0" * 510) + bytes([0xBD, 0x04, 0xEF, 0xFE]) + (0x00010000).to_bytes(4, "little") \
        + ((14 << 16) | 29).to_bytes(4, "little") + (30153 << 16).to_bytes(4, "little") + b"\0" * 64
    os.makedirs(os.path.join(steam, "steamapps/compatdata/1091500/pfx/drive_c/windows/system32"), exist_ok=True)
    with open(os.path.join(steam, "steamapps/compatdata/1091500/pfx/drive_c/windows/system32/vcruntime140.dll"), "wb") as f:
        f.write(bytes(vc))
    heroic = os.path.join(home, ".var/app/com.heroicgameslauncher.hgl/config/heroic")
    cp = os.path.join(home, "Games/Heroic/Cyberpunk 2077")
    sv = os.path.join(home, "Games/Heroic/Stardew Valley")
    write(os.path.join(cp, "bin/x64/Cyberpunk2077.exe"))
    write(os.path.join(sv, "StardewValley"))
    write(os.path.join(heroic, "legendaryConfig/legendary/installed.json"),
          '{"Ginger": {"title": "Cyberpunk 2077", "install_path": "%s", "platform": "Windows", "version": "2.3"}}' % cp)
    k1 = os.path.join(home, "Games/Heroic/Kingdom Come Deliverance")
    k2 = os.path.join(home, "Games/Heroic/Kingdom Come Deliverance II")
    write(os.path.join(k1, "Bin/Win64/KingdomCome.exe"))
    write(os.path.join(k2, "Bin/Win64MasterMasterSteamPGO/KingdomCome.exe"))
    os.makedirs(os.path.join(steam, "steamapps/common/KingdomComeDeliverance2/mods"), exist_ok=True)
    write(os.path.join(heroic, "gog_store/installed.json"), json.dumps({"installed": [
        {"appName": "1453375253", "install_path": sv, "platform": "linux", "version": "1.6.15"},
        {"appName": "1719198803", "install_path": k1, "platform": "windows", "version": "1.9.6"},
        {"appName": "1248083010", "install_path": k2, "platform": "windows", "version": "1.5.58"}]}))
    write(os.path.join(heroic, "store_cache/gog_library.json"), json.dumps({"games": [
        {"app_name": "1453375253", "title": "Stardew Valley"},
        {"app_name": "1719198803", "title": "Kingdom Come: Deliverance"},
        {"app_name": "1248083010", "title": "Kingdom Come: Deliverance II"}]}))

    # Lantern Hollow: an Unreal game (one Content/Paks folder) with anti-cheat. Brass Pier: a Data folder, a Mods
    # folder with a mod already in it, and a file of its own a mod will try to replace.
    lh = os.path.join(steam, "steamapps/common/Lantern Hollow")
    write(os.path.join(lh, "Lantern/Content/Paks/Lantern-Windows.pak"), "pak")
    write(os.path.join(lh, "EasyAntiCheat/EasyAntiCheat_EOS_Setup.exe"))
    bp = os.path.join(steam, "steamapps/common/Brass Pier")
    write(os.path.join(bp, "Data/pier.txt"), "the game's own pier\n")
    write(os.path.join(bp, "Data/textures/water.dds"), "dds")
    write(os.path.join(bp, "Mods/OldMod/mod.json"), "{}")

    skyrim_plugins(os.path.join(steam, "steamapps/common/Skyrim Special Edition/Data"),
                   os.path.join(steam, "steamapps/compatdata/489830/pfx/drive_c/users/steamuser/AppData/Local/Skyrim Special Edition"))
    downloads(os.path.join(home, "Downloads"))


def plugin(path, flags, masters):
    """A minimal plugin: its TES4 header record with HEDR, an author and its masters (nothing else is read)."""
    def sub(kind, data):
        return kind + struct.pack("<H", len(data)) + data
    body = sub(b"HEDR", struct.pack("<fII", 1.7, 0, 0x800)) + sub(b"CNAM", b"TurbineTest\0")
    for m in masters:
        body += sub(b"MAST", m.encode() + b"\0") + sub(b"DATA", struct.pack("<Q", 0))
    data = b"TES4" + struct.pack("<IIIIHH", len(body), flags, 0, 0, 44, 0) + body
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        f.write(data)


def skyrim_plugins(data, settings):
    """Skyrim's own masters and three made-up plugins, one of them missing a master, for the Load order tab."""
    for name in ["Skyrim.esm", "Update.esm", "Dawnguard.esm", "HearthFires.esm", "Dragonborn.esm"]:
        plugin(os.path.join(data, name), 1, [] if name == "Skyrim.esm" else ["Skyrim.esm"])
    plugin(os.path.join(data, "Unofficial Skyrim Patch.esp"), 1, ["Skyrim.esm", "Update.esm", "Dawnguard.esm"])
    plugin(os.path.join(data, "Quiet Roads.esp"), 0, ["Skyrim.esm"])
    plugin(os.path.join(data, "Lantern Paths.esp"), 0x200, ["Skyrim.esm", "Lanterns Of Skyrim.esm"])
    write(os.path.join(settings, "plugins.txt"), "*Unofficial Skyrim Patch.esp\n*Quiet Roads.esp\n*Lantern Paths.esp\n")


def tiny_png(w, h, top, bottom):
    """A small picture as PNG bytes (for an installer's option pictures)."""
    import tempfile
    with tempfile.TemporaryDirectory() as d:
        p = os.path.join(d, "x.png")
        png(p, w, h, top, bottom)
        with open(p, "rb") as f:
            return f.read()


FOMOD_CONFIG = """<?xml version="1.0" encoding="UTF-8"?>
<config xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <moduleName>Lush Meadows</moduleName>
  <requiredInstallFiles><folder source="00 Core" destination=""/></requiredInstallFiles>
  <installSteps order="Explicit">
    <installStep name="Grass">
      <optionalFileGroups order="Explicit">
        <group name="Grass style" type="SelectExactlyOne">
          <plugins order="Explicit">
            <plugin name="Full and lush">
              <description>Thick grass everywhere, as in the pictures. Uses the most graphics memory and can cost a few frames per second in forests.</description>
              <image path="fomod/images/full.png"/>
              <files><folder source="10 Full" destination=""/></files>
              <typeDescriptor><type name="Recommended"/></typeDescriptor>
            </plugin>
            <plugin name="Lighter grass">
              <description>About half as dense. Smoother on handhelds.</description>
              <image path="fomod/images/light.png"/>
              <files><folder source="11 Light" destination=""/></files>
              <typeDescriptor><type name="Optional"/></typeDescriptor>
            </plugin>
            <plugin name="Keep the game's grass">
              <description>Only changes the ground textures.</description>
              <typeDescriptor><type name="Optional"/></typeDescriptor>
            </plugin>
          </plugins>
        </group>
      </optionalFileGroups>
    </installStep>
    <installStep name="Extras">
      <optionalFileGroups order="Explicit">
        <group name="Extras" type="SelectAny">
          <plugins order="Explicit">
            <plugin name="Wildflowers">
              <description>Small flowers along the roads.</description>
              <files><folder source="20 Flowers" destination=""/></files>
              <typeDescriptor><type name="Optional"/></typeDescriptor>
            </plugin>
            <plugin name="Fallen leaves">
              <description>Leaves on the forest floor in autumn regions.</description>
              <files><folder source="21 Leaves" destination=""/></files>
              <typeDescriptor><type name="Optional"/></typeDescriptor>
            </plugin>
          </plugins>
        </group>
      </optionalFileGroups>
    </installStep>
  </installSteps>
</config>
"""


def zip_file(path, files, when):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as z:
        for name, data in files.items():
            z.writestr(name, data)
    os.utime(path, (when, when))


def downloads(folder):
    """Test mods, one per kind of game rules. Each is listed in the screenshot step by file name."""
    now = int(time.time())
    fake_dll = b"MZ" + b"\0" * 62 + b"placeholder, never run"
    # Skyrim: loose files, the way most Skyrim mods are packed (meshes and textures at the top of the zip).
    zip_file(os.path.join(folder, "Brighter Lanterns 1.2.zip"), {
        "meshes/lanterns/lantern01.nif": b"nif" * 40,
        "textures/lanterns/lantern01.dds": b"DDS " + b"\0" * 124,
        "textures/lanterns/lantern01_n.dds": b"DDS " + b"\0" * 124,
        "readme.txt": "Brighter lanterns for the roads of Skyrim.\n",
    }, now - 3600)
    # Skyrim: a second lantern mod that changes one of the same files (who wins: the later one in the list).
    zip_file(os.path.join(folder, "Lantern Colors 1.1.zip"), {
        "textures/lanterns/lantern01.dds": b"DDS " + b"\1" * 124,
        "textures/lanterns/lantern02.dds": b"DDS " + b"\1" * 124,
    }, now - 3000)
    # Skyrim: a mod with an SKSE plugin, so it needs the script loader.
    zip_file(os.path.join(folder, "Lantern Glow SKSE 2.0.zip"), {
        "Data/SKSE/Plugins/LanternGlow.dll": fake_dll,
        "Data/SKSE/Plugins/LanternGlow.ini": "[General]\nGlow=1.5\n",
        "Data/LanternGlow.esp": b"TES4" + b"\0" * 60,
    }, now - 7200)
    # Thunderstore package for a BepInEx game: manifest, icon and readme at the top, the plugin in plugins/.
    zip_file(os.path.join(folder, "TurbineTest-Bigger_Pockets-1.0.3.zip"), {
        "manifest.json": json.dumps({"name": "Bigger_Pockets", "version_number": "1.0.3",
                                     "website_url": "", "description": "Carry more.",
                                     "dependencies": ["BepInEx-BepInExPack-5.4.2100"]}),
        "icon.png": b"\x89PNG\r\n\x1a\n",
        "README.md": "# Bigger Pockets\nCarry more.\n",
        "plugins/BiggerPockets.dll": fake_dll,
    }, now - 86400)
    # Stardew Valley: a SMAPI mod is one folder with a manifest.json and its code.
    zip_file(os.path.join(folder, "Quiet Rooster 1.4.zip"), {
        "QuietRooster/manifest.json": json.dumps({"Name": "Quiet Rooster", "Author": "TurbineTest", "Version": "1.4.0",
                                                  "UniqueID": "TurbineTest.QuietRooster", "EntryDll": "QuietRooster.dll",
                                                  "MinimumApiVersion": "4.0.0"}),
        "QuietRooster/QuietRooster.dll": fake_dll,
    }, now - 2 * 86400)
    # Skyrim: a mod with an installer (FOMOD): one question per step, pictures for the options.
    zip_file(os.path.join(folder, "Lush Meadows 1.0.zip"), {
        "fomod/info.xml": "<fomod><Name>Lush Meadows</Name><Author>TurbineTest</Author><Version>1.0</Version></fomod>",
        "fomod/ModuleConfig.xml": FOMOD_CONFIG,
        "fomod/images/full.png": tiny_png(96, 54, (120, 140, 160), (60, 110, 50)),
        "fomod/images/light.png": tiny_png(96, 54, (130, 150, 170), (110, 130, 80)),
        "00 Core/textures/terrain/meadow01.dds": b"DDS " + b"\0" * 124,
        "10 Full/meshes/grass/meadowgrass01.nif": b"nif" * 40,
        "11 Light/meshes/grass/meadowgrass01.nif": b"nif" * 20,
        "20 Flowers/meshes/flowers/flower01.nif": b"nif" * 10,
        "21 Leaves/textures/leaves/leaf01.dds": b"DDS " + b"\0" * 124,
    }, now - 3 * 86400)
    # Cyberpunk 2077: a Cyber Engine Tweaks mod (a folder with init.lua), which needs the framework that isn't installed.
    zip_file(os.path.join(folder, "Quiet Streets CET 1.0.zip"), {
        "QuietStreets/init.lua": "-- placeholder, never run\nreturn {}\n",
        "QuietStreets/settings.json": "{}",
    }, now - 4 * 86400)
    # 0.64: Cyberpunk 2077: Cyber Engine Tweaks itself (its plugin and the ASI loader), for the Visual C++ runtime check.
    zip_file(os.path.join(folder, "Cyber Engine Tweaks 1.35.zip"), {
        "bin/x64/plugins/cyber_engine_tweaks.asi": b"MZ" + b"\0" * 62,
        "bin/x64/version.dll": b"MZ" + b"\0" * 62,
        "bin/x64/global.ini": "[GlobalSets]\nLoadPlugins=1\n",
    }, now - 4 * 86400)
    # Cyberpunk 2077: a REDmod mod (a folder with info.json), which loads through the game's own launcher.
    zip_file(os.path.join(folder, "Neon Signs REDmod 1.1.zip"), {
        "NeonSigns/info.json": json.dumps({"name": "NeonSigns", "version": "1.1.0"}),
        "NeonSigns/archives/neonsigns.archive": b"RDAR" + b"\0" * 60,
    }, now - 4 * 86400)
    # The Witcher 3: two script mods that change the same script (they need merging), one with key bindings to add.
    script = "// placeholder script\n"
    zip_file(os.path.join(folder, "Better Torches 1.0.zip"), {
        "modBetterTorches/content/scripts/game/player/playerWitcher.ws": script,
    }, now - 5 * 86400)
    zip_file(os.path.join(folder, "Faster Roach 2.0.zip"), {
        "modFasterRoach/content/scripts/game/player/playerWitcher.ws": script,
        "input.settings": "[Exploration]\nIK_F8=(Action=FasterRoach)\n",
    }, now - 5 * 86400)
    # A Nexus download, named the way Nexus names files (name, mod ID, version, upload time).
    zip_file(os.path.join(folder, "All Quest Objectives On Map-5-1-0-5-1699999999.zip"), {
        "modAllQuestObjectives/content/scripts/game/gui/hud/modules/hudModuleMinimap2.ws": script,
    }, now - 2 * 86400)
    # Kingdom Come: Deliverance: a mod folder with its manifest and pak, and a bare pak (Turbine makes its folder).
    manifest = ('<?xml version="1.0" encoding="utf-8"?>\n<kcd_mod><info><name>%s</name><author>TurbineTest</author>'
                '<version>1.0</version></info><supports><kcd_version>1.*</kcd_version></supports></kcd_mod>\n')
    pak = b"PK\x05\x06" + b"\0" * 18
    zip_file(os.path.join(folder, "Better Lockpicking 1.0.zip"), {
        "BetterLockpicking/mod.manifest": manifest % "Better Lockpicking",
        "BetterLockpicking/Data/lockpicking.pak": pak,
        "readme.txt": "Easier lockpicking.\n",
    }, now - 6 * 86400)
    zip_file(os.path.join(folder, "Unlimited Saving 3.0.zip"), {"Data/zzz_saving.pak": pak}, now - 6 * 86400)
    # Kingdom Come: Deliverance II: a mod folder inside Mods, and one named for the first game (a warning before Play).
    zip_file(os.path.join(folder, "Herb Picking II 1.5.zip"), {
        "Mods/HerbPicking/mod.manifest": manifest % "Instant Herb Picking II",
        "Mods/HerbPicking/Data/herbs.pak": pak,
    }, now - 6 * 86400)
    zip_file(os.path.join(folder, "Fast Travel KCD1 2.0.zip"), {
        "FastTravel/mod.manifest": manifest % "Fast Travel",
        "FastTravel/Data/travel.pak": pak,
    }, now - 6 * 86400)
    # 0.56, Basic support: an Unreal pak mod (Lantern Hollow); a mod with its own Data folder, one with loose files
    # Turbine can't place, and one that would replace the game's own files (Brass Pier).
    zip_file(os.path.join(folder, "Wand Glow 1.0.zip"), {
        "WandGlow_P.pak": b"pak" * 20, "WandGlow_P.utoc": b"utoc", "WandGlow_P.ucas": b"ucas", "readme.txt": "Glowing wands.\n",
    }, now - 5 * 86400)
    zip_file(os.path.join(folder, "Brass Lamps 1.0.zip"), {
        "Data/textures/lamp.dds": b"DDS " + b"\0" * 60, "readme.txt": "Brass lamps on the pier.\n",
    }, now - 5 * 86400)
    zip_file(os.path.join(folder, "Odd Layout 1.0.zip"), {"odd.cfg": b"x=1\n", "odd2.cfg": b"y=2\n"}, now - 5 * 86400)
    zip_file(os.path.join(folder, "Pier Fix 1.0.zip"), {
        "Data/pier.txt": b"a mod's pier\n", "Data/pier2.txt": b"new\n", "BrassPier.exe": fake_dll,
    }, now - 5 * 86400)
    # A profile share file someone sent, for Lethal Company.
    share = {"turbineProfile": 1, "app": "Turbine Mod Manager",
             "game": {"name": "Lethal Company", "store": "steam", "storeId": "1966720"}, "profile": "Friday night",
             "mods": [{"name": "LateCompany", "enabled": True, "source": "thunderstore", "id": "anormaltwig-LateCompany", "version": "1.0.10"},
                      {"name": "More Suits", "enabled": True, "source": "thunderstore", "id": "x753-More_Suits", "version": "1.4.3"},
                      {"name": "Brighter Flashlights", "enabled": True, "source": "zip", "id": "", "version": None}]}
    write(os.path.join(folder, "Turbine profile - Lethal Company - Friday night.json"), json.dumps(share, indent=1))


def anticheat_games(home):
    """0.57: anti-cheat games for their own pictures (a separate home, so the shared one's rail stays as it was): Elden
    Ring (me3), Halo MCC (its own switch) and Destiny 2 (online only), with made-up files."""
    steam = os.path.join(home, ".local/share/Steam")
    for app_id, name, folder, exe, top, bottom in [
        ("1245620", "ELDEN RING", "ELDEN RING", "Game/eldenring.exe", (160, 140, 80), (30, 25, 15)),
        ("976730", "Halo: The Master Chief Collection", "Halo The Master Chief Collection", "mcclauncher.exe", (60, 110, 60), (15, 25, 15)),
        ("1085660", "Destiny 2", "Destiny 2", "destiny2.exe", (90, 90, 120), (20, 20, 30)),
    ]:
        apps = os.path.join(steam, "steamapps")
        write(os.path.join(apps, f"appmanifest_{app_id}.acf"), manifest(app_id, name, folder))
        write(os.path.join(apps, "common", folder, exe), "the game's own program\n")
        os.makedirs(os.path.join(apps, "compatdata", app_id, "pfx/drive_c/users/steamuser/AppData/Roaming"), exist_ok=True)
        png(os.path.join(steam, "appcache/librarycache", app_id, "library_600x900.png"), 60, 90, top, bottom)
    er = os.path.join(steam, "steamapps/common/ELDEN RING/Game")
    write(os.path.join(er, "start_protected_game.exe"), "the anti-cheat starter\n")
    write(os.path.join(er, "regulation.bin"), "the game's own regulation\n")
    write(os.path.join(er, "parts/am_f_0000.partsbnd.dcx"), "parts\n")
    write(os.path.join(steam, "steamapps/compatdata/1245620/pfx/drive_c/users/steamuser/AppData/Roaming/EldenRing/76561190000000000/ER0000.sl2"), "a save\n")
    write(os.path.join(steam, "steamapps/common/Destiny 2/BattlEye/BEService.exe"))
    now = time.time()
    zip_file(os.path.join(home, "Downloads", "Golden Armor 1.0.zip"), {
        "parts/am_f_9000.partsbnd.dcx": b"made-up armor",
    }, now - 3 * 86400)
    zip_file(os.path.join(home, "Downloads", "Guardian Colors 1.0.zip"), {"colors.txt": b"x"}, now - 3 * 86400)
    # 0.58: the other two kinds of Elden Ring save (modded through me3, Seamless Co-op), and a made-up Seamless Co-op
    # with a settings file laid out the way its own is (sections, numbered choices in the comments, a password).
    saves = os.path.join(steam, "steamapps/compatdata/1245620/pfx/drive_c/users/steamuser/AppData/Roaming/EldenRing/76561190000000000")
    write(os.path.join(saves, "TurbineModded.sl2"), "a modded save\n")
    write(os.path.join(saves, "ER0000.co2"), "a co-op save\n")
    zip_file(os.path.join(home, "Downloads", "Seamless Co-op 2.0.1.zip"), {
        "SeamlessCoop/ersc.dll": b"MZ made-up",
        "SeamlessCoop/ersc_settings.ini": (
            "[GAMEPLAY]\r\n\r\n; Let other players invade your world. 0=FALSE 1=TRUE\r\nallow_invaders = 1\r\n\r\n"
            "; What shows over players' heads. 0 = Normal | 1 = None | 2 = Ping\r\noverhead_player_display = 0\r\n\r\n"
            "; Volume before a save loads. 0 = MUTE 10 = MAX\r\ndefault_boot_master_volume = 5\r\n\r\n"
            "[SCALING]\r\n\r\n; Extra enemy health (%) per player. (Default: 35)\r\nenemy_health_scaling = 35\r\n\r\n"
            "[PASSWORD]\r\n\r\n; Session password\r\ncooppassword = \r\n"),
        "ersc_launcher.exe": b"MZ made-up launcher",
    }, now - 2 * 86400)


if __name__ == "__main__":
    main(os.path.abspath(sys.argv[1]))
    if "--anticheat" in sys.argv[2:]:
        anticheat_games(os.path.abspath(sys.argv[1]))
