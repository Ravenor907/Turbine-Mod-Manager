<p align="center"><img src="site/img/banner.png" alt="Turbine Mod Manager" width="100%"></p>

<p align="center"><b>A controller-first mod manager for your Steam and Heroic games.</b><br>
For SteamOS handhelds, in Game Mode and Desktop Mode. Installed as a Flatpak. Built and tested on SteamOS 3.9.</p>

<p align="center">
  <a href="https://github.com/Ravenor907/Turbine-Mod-Manager/releases/latest/download/TurbineModManager.flatpak"><b>Download</b></a>
  &nbsp;·&nbsp; <a href="https://github.com/Ravenor907/Turbine-Mod-Manager/releases">Releases</a>
  &nbsp;·&nbsp; <a href="https://ravenor907.github.io/Turbine-Mod-Manager/">Website</a>
  &nbsp;·&nbsp; <a href="https://github.com/Ravenor907/Turbine-Mod-Manager/issues">Report a problem</a>
</p>

Not for Windows or other Linux systems. Never mods games that need their anti-cheat online.

## What it does

- **Any Steam or Heroic game** (Epic, GOG), on internal storage or an SD card.
- **Get mods in the app** from Nexus Mods and Thunderstore, or add zip, 7z and rar files.
- **What a mod needs, lined up:** Get them all fetches the other mods it needs, kept under Dependencies in My mods.
- **Knows the big games:** Skyrim load order, Cyberpunk frameworks, SKSE, SMAPI and BepInEx.
- **FromSoftware games** through me3, game folder untouched. Seamless Co-op chosen at Play.
- **Safe to try:** replaced game files and your saves are backed up. Play without mods or go back with one press.
- **Checks before you play:** a mod that is on but missing what it needs is named before the game starts.
- **Controller first,** with touch, keyboard and mouse. Optional Steam menu plugin (Decky).

<p align="center">
  <img src="site/img/shot-getmods.png" width="49%" alt="Get mods for Skyrim: a grid of mod cards with pictures, likes and downloads">
  <img src="site/img/shot-mymods.png" width="49%" alt="My mods: a rail of games down the left and the selected mod's details on the right">
</p>

## Setup

**You need:** SteamOS (built and tested on 3.9); internet (Turbine is about 100 MB, and the first install also gets
the parts it runs on from Flathub); a free Nexus Mods account to download from Nexus (browsing, Thunderstore and your
own files need none; Premium is optional and makes Nexus downloads one press). No web browser: Nexus opens inside
Turbine.

1. **Switch to Desktop Mode:** Steam button, Power, Switch to Desktop.
2. **Download** [TurbineModManager.flatpak](https://github.com/Ravenor907/Turbine-Mod-Manager/releases/latest/download/TurbineModManager.flatpak). It goes to Downloads.
3. **Install:** double-click it in Downloads and press Install in Discover. The first install takes a few minutes.
4. **Add to Steam:** in the app launcher, right-click Turbine Mod Manager and choose Add to Steam.
5. **Start it in Game Mode:** Return to Gaming Mode, then find Turbine in your Library under Non-Steam. A five-card
   tour shows the buttons, then you pick a game.
6. **Your first Nexus mod:** Nexus opens inside Turbine. Sign in (free), press Slow download, and the mod is added and
   turned on. If it needs other mods, Turbine lists them: press Get them all.
7. **FromSoftware games only, once per game:** Turbine shows these steps when you first press Play. Restart Steam once
   so it finds Turbine's tool, then in the game's Properties, Compatibility, turn on "Force the use of a specific Steam
   Play compatibility tool" and pick "Turbine (mods)".

Prefer a terminal? `flatpak install --user ~/Downloads/TurbineModManager.flatpak`

Updates: Turbine checks for a new version when it starts and installs it only when you say yes.

**Steam menu plugin (optional):** in Decky, open the gear, then Developer, then **Install Plugin From URL**, and paste:
`https://github.com/Ravenor907/Turbine-Mod-Manager/releases/latest/download/Turbine-Decky.zip`

## Games

- **Full support:** Skyrim Special Edition, Cyberpunk 2077, The Witcher 3, Kingdom Come: Deliverance I and II,
  Stardew Valley, Valheim, Lethal Company, Risk of Rain 2.
- **Through me3:** Elden Ring, Elden Ring Nightreign, Armored Core VI, Dark Souls III, Sekiro.
- **Every other Steam and Heroic game:** basic support.
- **Never modded:** games that need their anti-cheat online, such as Apex Legends, Destiny 2 and Fortnite.

## Controls

| Do | Controller | Keyboard | Mouse |
| --- | --- | --- | --- |
| Move | Stick or D-pad | W A S D or arrows | Point |
| Select | A | Enter or Space | Click (double-click opens a mod) |
| Back | B | Esc | Back side button |
| Options | X | R | Right-click |
| Find and sort | Y | F | |
| Pages | LB / RB | Q / E | Page names, forward side button |
| Quick wheel | View | Hold Tab | |
| Settings | Start | Esc on a game's first page | |
| Play options | Hold Start 3 seconds | Hold Space 3 seconds | |
| Help | | F1 | |

Play with mods: select Play in the bottom bar. Play options choose with or without mods, or Seamless Co-op. Touch
works too: tap to pick, hold for options.

## Help

**Cyberpunk: RED4ext or Cyber Engine Tweaks shows error 998.** The game's Proton setup is missing Microsoft's Visual
C++ runtime. Turbine spots this in **Before you play**: press **Install the Visual C++ runtime** (one press, about
25 MB from Microsoft). By hand instead: install Protontricks from Discover in Desktop Mode, then run
`flatpak run com.github.Matoking.protontricks 1091500 --force vcrun2022` in Konsole.

**Can I get banned?** Turbine never adds mods to a game that needs its anti-cheat online. FromSoftware games run mods
offline through me3, and Turbine says so before the first modded Play.

**Something not working?** In Turbine open Settings, Help, **Report a problem** and press **Save report**. Then
[open an issue](https://github.com/Ravenor907/Turbine-Mod-Manager/issues/new) and attach the file. It holds no keys or
passwords.

[Releases](https://github.com/Ravenor907/Turbine-Mod-Manager/releases) · [Issues](https://github.com/Ravenor907/Turbine-Mod-Manager/issues) · Free software under the GNU GPL, version 3 or later. See [LICENSE](LICENSE).
