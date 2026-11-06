# Turbine Mod Manager

A controller-first mod manager for SteamOS handhelds, for games from Steam and Heroic (Epic, GOG).

Made for SteamOS on handhelds such as the Steam Deck, Legion Go and ROG Ally, in Game Mode and Desktop Mode. Other Linux systems are not supported.

## Status

Early test builds.

- 0.1 finds your Steam and Heroic games (internal storage and SD cards) and sets up the interface, controls and screen scaling.
- 0.2 adds mods: add them from zip files in your Downloads folder, turn them on and off, set their order where order matters, and uninstall with Undo (removed mods are kept for 7 days). Changes go into the game by themselves a few seconds after your last edit and when you leave Turbine. Play checks the game first (game updated, script loader missing, low space) and then starts it, for Steam and Heroic games. The game menu has Play original (no mods, nothing deleted) and Restore last working setup.

Turbine only ever removes files it placed itself, keeps a backup of any game file a mod replaces, and never runs anything from a mod archive. Downloading mods and loaders from the internet comes in a later build.

Game rules for: Skyrim Special Edition, Stardew Valley, Valheim, Lethal Company and Risk of Rain 2. Other games work generically: files go where the archive says, from the game folder.

## Controls

| Button | Does |
| --- | --- |
| Start | Play (checks the game, then starts it) |
| A | Main action (turn a mod on or off, pick) |
| B | Back |
| X | Menu for the selected mod |
| Y | Reorder (only where order matters), or Undo right after an uninstall |
| LB / RB | Switch game |
| LT / RT | Switch tabs |
| View | Game menu |

## Folders it uses

The Flatpak can write to Steam's library folders (internal and SD cards under /run/media), ~/Games and Heroic's folders, because that is where mods go. It reads your Downloads folder to find mod zip files.

## License

GNU General Public License, version 3 or later. See [LICENSE](LICENSE).
