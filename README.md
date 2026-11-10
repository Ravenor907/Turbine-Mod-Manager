<p align="center"><img src="site/img/banner.png" alt="Turbine Mod Manager" width="100%"></p>

<p align="center"><b>Mods for your SteamOS handheld, with the controller, in Game Mode. Pick a game, add a mod, press Play.</b></p>

<p align="center">
  <a href="https://github.com/Ravenor907/Turbine-Mod-Manager/releases/latest/download/TurbineModManager.flatpak"><b>Download Turbine</b></a>
  &nbsp;·&nbsp; <a href="https://github.com/Ravenor907/Turbine-Mod-Manager/releases">Releases</a>
  &nbsp;·&nbsp; <a href="https://github.com/Ravenor907/Turbine-Mod-Manager/issues">Report a problem</a>
</p>

## Features

**Built for the handheld in your hands**
- Made for SteamOS on the Steam Deck, Legion Go, ROG Ally and other handhelds. Runs in Game Mode and Desktop Mode.
- Controller first: A selects, B goes back, everywhere. Touch works for every button.
- Keyboard and mouse the PC way: W A S D, Q and E for pages, F to find, R for options, hold Tab for the wheel, hold Space to play.
- Interface size from 50% to 150%. Animations Full, Reduced or Off.

**Elden Ring and the other FromSoftware games**
- Elden Ring, Nightreign, Armored Core VI, Dark Souls III and Sekiro run their mods through me3.
- Start them from Steam's own Play: pick "Turbine (mods)" once in the game's Compatibility settings and Game Mode shows the game as usual.
- The game folder is never touched, and with no mods on the game starts clean.
- Online play stays clean: games that need their anti-cheat online never get mods.

**Seamless Co-op made easy**
- Choose Play with mods (solo) or Play Seamless Co-op right at Play.
- Co-op settings are one press away, session password first. Co-op never starts without a password.
- Turbine keeps the newest version and only one copy per game.
- Optional Co-op tile in your Steam library.

**Get mods without leaving the couch**
- Nexus Mods built in: browse by game with pictures, likes and downloads, then get a mod with one press.
- Sign in to Nexus through Chrome with the small browser helper.
- Mod Manager Download links always ask first. Nothing downloads without your say.
- Thunderstore for the games that use it, plus zip, 7z and rar files you already have.
- Games from Steam and Heroic (Epic and GOG), on internal storage and SD cards.

**Safe by design**
- Nothing installs without you, and everything can be undone: Play without mods, go back to a setup that worked, or turn every mod off.
- Mods apply themselves a few seconds after your last change. No Save button to forget.
- Saves are backed up before a modded Play, and any game file a mod replaces is backed up too.
- Mod settings files get real controls (switches, sliders, lists), and your changes are kept when the mod updates.

**Fits right into Steam**
- Optional Steam menu plugin (Decky): Mods on or off and Play from the Quick Access menu.
- Steam library art for Turbine, set up for you.

<p align="center">
  <img src="site/img/shot-getmods.png" width="49%" alt="Get mods: a grid of mod cards with pictures, likes and downloads">
  <img src="site/img/shot-mymods.png" width="49%" alt="My mods: mods turned on, with the selected mod's details and Play bottom right">
  <img src="site/img/shot-coop.png" width="49%" alt="Elden Ring with Seamless Co-op on: Co-op settings one press away and Play Co-op bottom right">
  <img src="site/img/shot-wheel.png" width="49%" alt="The quick wheel: Find, Updates, Add from Downloads, Play options, Game menu, Settings, Games and Loadouts">
</p>
<p align="center"><sub>Real Turbine screens with made-up sample mods.</sub></p>

**What's coming:** 0.64 lines up a mod's required mods for you, one press each.

## Install

1. In Desktop Mode, download [TurbineModManager.flatpak](https://github.com/Ravenor907/Turbine-Mod-Manager/releases/latest/download/TurbineModManager.flatpak) from the latest release.
2. Double-click it in Downloads and press **Install** in Discover. The first install takes a few minutes.
3. Right-click **Turbine Mod Manager** in the app launcher and choose **Add to Steam**.
4. Go back to Game Mode. Turbine is in your Library under **Non-Steam**. It keeps itself up to date.

Prefer a terminal? `flatpak install --user ~/Downloads/TurbineModManager.flatpak`

**Steam menu plugin (optional):** in Decky, open the gear, then Developer, then **Install Plugin From URL**, and paste:
`https://github.com/Ravenor907/Turbine-Mod-Manager/releases/latest/download/Turbine-Decky.zip`

## Controls

| Do | Controller | Keyboard | Mouse |
| --- | --- | --- | --- |
| Select | A | Enter or Space | Click |
| Back | B | Esc | Back side button |
| Options | X | R | Right-click |
| Find (search, sort, categories) | Y | F | |
| Pages (My mods, Get mods, Load order) | LB / RB | Q / E | Page names, Forward side button |
| Page up and down | LT / RT | | Wheel |
| Quick wheel | View | Hold Tab | |
| Settings | Start | Esc on a game's first page | |
| Play | Hold Start 3 seconds, or A on Play | Hold Space 3 seconds | Click Play |
| Pointer | Right stick | | Mouse |
| Interface size | | Ctrl and + or - | Ctrl and wheel |

The hints at the bottom follow whatever you used last. A first-run tour shows the buttons; Settings, Help brings it back.

## FAQ

**Can I get banned?**
Turbine never adds mods to a game that needs its anti-cheat to play online. Those games show why and Play with mods is refused. The FromSoftware games run mods offline through me3, and Turbine tells you plainly before the first modded Play. Don't take a save you modded offline into online play.

**How do I play Seamless Co-op?**
Get Seamless Co-op for Elden Ring in Turbine, set a session password in Co-op settings (everyone in your group uses the same one), then choose Play Seamless Co-op. It loads only that way, so Play with mods stays solo.

**Do I need Nexus Premium?**
No. A free Nexus account works: Nexus's own download page opens inside Turbine, you press Slow download, and the mod is added and turned on. Premium skips that page so a mod installs with one press. Thunderstore mods and files you already have need no account.

**Does it work on Windows or other Linux systems?**
No. Turbine is made for SteamOS handhelds.

**How do I uninstall it?**
Turn your mods off, set each FromSoftware game's Compatibility back to your usual Proton in Steam, then remove Turbine in Discover.

## Help

Something not working? In Turbine open Settings, Help, **Report a problem** and press **Save report**. Then [open an issue](https://github.com/Ravenor907/Turbine-Mod-Manager/issues/new) and attach the file. It holds no keys or passwords.

[Releases](https://github.com/Ravenor907/Turbine-Mod-Manager/releases) · [Issues](https://github.com/Ravenor907/Turbine-Mod-Manager/issues) · Free software under the GNU GPL, version 3 or later. See [LICENSE](LICENSE).
