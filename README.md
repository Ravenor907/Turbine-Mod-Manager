# Turbine Mod Manager

A controller-first mod manager for SteamOS handhelds, for games from Steam and Heroic (Epic, GOG).

Made for SteamOS on handhelds such as the Steam Deck, Legion Go and ROG Ally, in Game Mode and Desktop Mode. Other Linux systems are not supported.

## What it does

Get mods from Nexus Mods and Thunderstore inside Turbine, or add files you already downloaded (zip, 7z or rar). Turn each mod on or off with A; where the order matters, Turbine keeps it safe and you can move a mod. Changes go into the game by themselves a few seconds after your last change and when you leave Turbine, so there is no Save to remember. Play checks the game first (a game update, a missing mod loader, low space) and offers the fix as a button, then starts the game. Play without mods starts the game clean without deleting anything, and the game menu can go back to a setup that worked.

Turbine only ever removes files it placed itself, keeps a backup of any game file a mod replaces, and never runs anything from a mod archive. Games that need their anti-cheat online never get mods. Elden Ring and the other me3 games run their mods through me3 without touching the game folder; Seamless Co-op is chosen at Play.

The optional Decky plugin puts Mods on or off and Play in Steam's Quick Access menu. Decky's Install Plugin From URL takes https://github.com/Ravenor907/Turbine-Mod-Manager/releases/latest/download/Turbine-Decky.zip

## Controls

| Button | Does |
| --- | --- |
| A | Select (turn a mod on or off, pick, open) |
| B | Back one step, everywhere |
| X | Options for the selected thing |
| Y | Find: search, sort and categories for the list |
| LB / RB | Tabs (My mods, Get mods, and the game's other tabs) |
| LT / RT | Page up and down |
| View | The quick wheel |
| Start | Play (hold for Play options) |
| Right stick | Pointer |

Touch works for everything the buttons do: tap to do, long-press for options.

## Folders it uses

The Flatpak can write to Steam's library folders (internal and SD cards under /run/media), ~/Games and Heroic's folders, because that is where mods go. It reads your Downloads folder to find mod files.

## Games started through Steam (Elden Ring and other me3 games)

For a game Turbine starts through me3 (Elden Ring, Nightreign, Armored Core VI, Dark Souls III, Sekiro), Turbine sets one line in the game's Steam Launch Options the first time you play it with mods, with one Steam restart that it asks about first, so Steam starts it and Game Mode shows it. That line starts the game normally when Turbine's helper isn't there, and the helper starts it normally whenever no mods are on. Before you uninstall Turbine, use Settings, Help, Undo the launch options change.

## License

GNU General Public License, version 3 or later. See [LICENSE](LICENSE).
