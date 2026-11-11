# QML components

One component per file in this folder. Every .qml file here is part of the Turbine module (app/build.rs picks them
up), so Main.qml uses them by name with no import. Components only draw and report taps: the controller and the
keyboard go through Main.qml (`act()`, `binds()`), which moves each component's `focusIndex` and runs the actions.
Taps on rows and buttons emit the signals below, so touch and the mouse work too. Each file's header comment says
the rest; this is the map.

## The `ui` kit

Most components take `required property var ui`, and Main.qml always passes `kit`:

| Member | What it is |
| --- | --- |
| `c` | the colors: ground, rail, panel, card, raised, line, text, soft, muted, accent, accentHi, accentLo, accentText, onAccent, focusFill, ok, warn, stop, light (bool) |
| `titleFont`, `bodyFont` | font families (both Barlow; Cinzel is Main.qml's own, for the wordmark) |
| `px(n)`, `fpx(n)`, `tpx(n)` | design pixels to screen pixels; text (never under 12); touch targets (never under the touch floor) |
| `hover(item, on)` | the pointer's hover ring |
| `motion` | the Motion object (durations and curves) |
| `glide(view, jump)` | a list brought to its focused row, gliding at Full |
| `follow(view, content, it)` | scrolls a Flickable so `it` is fully in view |
| `readable(col, bg)`, `lum(col)` | a color made readable on a background (4.5:1) |
| `fileSize(b)`, `niceDate(t)`, `shortCount(n)` | "24.0 MB", "Mar 4, 2025", "3.1M" |

## Main.qml's own parts

Main.qml keeps the state and the logic; these files are its screen, cut into parts. Unlike the components below they
know Main.qml: each takes `required property var app` (Main.qml's window) and `kit`, reads Main.qml's properties and
calls its functions directly (taps ask `app.gate()` first). Inside, `c` and `mo` are `kit.c` and `kit.motion`.

- **GameRail**: the game rail. `itemAt(i)` (a tile, Show hidden games or the gear); `railList`.
- **ModList**: My mods' list and its rows (ModRow, and the removed, held, Dependencies and Removed rows). `look`.
- **ModPanel**: the details panel beside My mods. `panelEdit`.
- **NoModsCard**: the empty My mods card.
- **SettingsPage**: the Settings page. `look`; `lookList`, `labsList`.
- **ScrollTest**: the 2,000 row scrolling test.
- **SheetLayer**: every sheet, menu and prompt, wired to Main.qml. `toast`, `seen`, `shot`, `rail`, `resetHold`,
  `confirmHold`, `nxmRingMove`; the sheets Main.qml drives (`settingsView`, `askView` and the rest) as aliases.
- **BottomBar**: the bottom bar and its legend. `check(tag)` (the build's bar check); `playBtn`, `startHint` and the
  other items the checks reach.
- **RequirementsStrip**: "Getting 2 of 3" with Skip and Stop.
- **Splash**: the start-up splash. `shot`; `finish(why)`, `shown`, `splashIcon`.
- **PointerLayer**: the controller pointer and its hover outline.
- **LabsChecklist**: Labs' checklist items (data only; Main.qml's `labsItems`).

Main.qml's small pieces take `required property var win` (Main.qml's window; a name of its own so `app` in bindings
written on them keeps its meaning): **Label** (words in the body font), **Key** (a button's cap; the glyph follows the
pad and the last input; `k`, `off`), **Hint** (keys and their words; `k`, `label`, `pre`, `k2`, `label2`, `inBar`),
**PageEdge** (Get mods' page edges; `label`, `dir`) and **Icon** (a line icon; `d`, `tint`).

## Building blocks

- **Overlay**: the root of everything that covers the window (sheets, menus, the wheel, the tutorial); a tap outside
  closes it. `closeOnOutside`, `motion`; signal `outsideTapped()`.
- **CardGuard**: the first child of an overlay's card, so taps on the card never count as outside.
- **SheetButtons**: the button row of every sheet with buttons. `labels`, `focusIndex`, `mainIndex`; signals
  `activated(index)`, `held(index, down)`; `itemAt(i)`.
- **MenuEntry**: one row of GameMenu and ModMenu. `label`, `detail`, `icon` (a name from its `icons`, or SVG path
  data), `available`, `danger`, `more`, `focused`, `dangerColor`, `iconShown`; signals `clicked()`, `held(down)`.
- **KeyCap**: a controller button as a small cap. `k` ("a", "lb").
- **Motion**: every duration and curve (tools/qml-check.js fails the build on one written elsewhere). `level`
  ("full", "reduced", "off"); read `on`, `moves`, `fast`, `base`, `moveFast`, `moveBase`, `page`, `enter`, `leave`,
  `even`, `fromScale`, `slide`, the splash's times and `spin`.

## The game page

- **ModRow**: one mod in My mods. `name`, `detail`, `detailLevel`, `isOn`, `ordered`, `order`, `badge`,
  `badgeLevel`, `focused`, `position`, `count`, `compact`, `ticked` (-1 no box, 0, 1); `pillItem`; signals
  `clicked()`, `menuClicked()`.
- **GetMods**: Get mods' picture cards, the Good first mods strip and the fallback card. `result`, `loading`,
  `loadingMore`, `summary`, `recent`, `recentTitle`, `fallback`, `fallbackTitle`, `focusArea` ("recent", "grid",
  "fallback"), `focusIndex`, `active`, `columns`; signals `recentClicked`, `itemClicked`, `itemMenu`,
  `fallbackClicked(key)`, `scrolled(first, last)`, `contentScrolled(y, before, longer)`; `gridItem(i)`.
- **ModDetails**: a mod's page over Get mods. `info`, `fallbackPicture`, `focused`, `mainLabel`, `mainLine`,
  `mainProgress`, `focusIndex` (0 the main button, 2 and on the requirements), `settingsLabel`, `settingsFocused`;
  signals `installClicked()`, `needClicked(i)`, `settingsClicked()`; `scroll(d)`.
- **LoadOrderList**: the Load order tab. `plugins`, `error`, `messages` (LOOT's notes), `note`, `owners`,
  `focusIndex` (-1 Sort automatically), `moving`, `active`; signals `sortClicked()`, `rowClicked(i)`, `rowMenu(i)`,
  `scrolled(first, last)`; `rowItem(i)`.
- **EmptyState**: a calm message where a list would be. `title`, `body`, `buttonKey`, `buttonLabel`,
  `buttonFocused`, `note`, `maxWidth`; signal `activated()`.
- **PlaceFiles**: "Where do these files go?" for Basic games. `topLine`, `gamePath`, `modPath`, `gameRows`,
  `modRows`, `pane`, `twoPanes`, `gameIndex`, `modIndex`, `preview`, `active`, `moveOnly`; signals
  `rowClicked(pane, i)`, `rowHeld(pane, i)`, `swiped(dir)`.
- **NexusBrowser**: Nexus inside Turbine, with its bar and Downloads tab. Main.qml owns the downloads, the legend
  and what each button does; this draws and reports (`pageEvent(what, detail)` and the other signals in the file).
- **Seasonal**: falling leaves or snow behind the panels. `kind`, `amount`, `size`, `scale`, `moving`, `light`.

## Menus and sheets

All are Overlays that fill their parent and show with `visible`; Main.qml handles the keys.

- **GameMenu**: View's game menu in sections. `gameName`, `subtitle`, `items` ({key, label, detail, enabled, icon,
  danger, more, group}), `focusIndex`; signals `activated(index, key)`, `held(index, down)`, `closed()`;
  `entryItem(i)`.
- **ModMenu**: a side sheet of actions (the mod menu, the card menu, pick lists). `title`, `subtitle`, `items`
  ({label, detail, enabled, icon, danger, more}), `focusIndex`, `bottomInset`; signals `activated(index)`,
  `closed()`.
- **QuickWheel**: the eight-wedge wheel. `slots` ({label, ok, desc}), `focusIndex`, `hint`, `mousePick`; signals
  `activated(i)`, `pointed(i)`, `closed()`, `middleTapped()`.
- **FindSheet**: Y's pop-up (Search or Find, Sort, Category or Show). `title`, `subtitle`, `fieldLabel`, `text`,
  `placeholder`, `sections`, `focusRow`, `focusCol`, `result`; signals `fieldClicked()`, `optionClicked(row, col)`,
  `doneClicked()`, `clearClicked()`, `closed()`, `typed(text)`, `entered(text)`.
- **PrePlayCheck**: "Before you play". `title`, `checks` ({level, title, detail}), `buttons` ({label, main}),
  `focusIndex`, `note`; signals `activated(index, key)`, `closed()`; `scroll(d)`.
- **LoaderPrompt**: "These mods need SKSE". `title`, `loaderName`, `loaderFullName`, `explain`, `neededBy`,
  `sourceUrl`, `canDownload`, `sourceShown`, `infoText`, `buttons` ({label}), `focusIndex`; signals
  `activated(index, key)`, `closed()`.
- **InfoSheet**: a plain sheet of words, steps or notes and its buttons. `title`, `text`, `steps`, `notes`,
  `footnote`, `buttons`, `focusIndex`; signals `activated(i)`, `held(i, down)`, `closed()`; `scrollBy(d)`.
- **AskText**: a line of text (search, names, codes, the key). `shown`, `title`, `note`, `placeholder`, `maxLength`,
  `secret`, `doneLabel`, `focusIndex`; signals `accepted(text)`, `cancelled()`, `edited(text)`, `fieldTapped()`;
  `open(...)`, `accept()`, `cancel()`, `clear()`.
- **AddMods**: Add from Downloads (and Open a loadout file). `title`, `subtitle`, `files` ({name, size, age}),
  `focusIndex`, `actionLabel`, `emptyText`, `otherLabel`, `bottomInset`; signals `activated(i)`, `otherClicked()`,
  `closed()`.
- **GamePicker**: "Which game is it for?". `fileName`, `games`, `focusIndex`; signals `activated(i)`, `closed()`.
- **VersionsSheet**: a mod's other versions or files. `modName`, `versions`, `focusIndex`, `note`; signals
  `activated(i)`, `closed()`.
- **PagesSheet**: Nexus pages to open for a free account. `title`, `pages` ({name, url, note}), `focusIndex`;
  signals `activated(i)`, `closed()`.
- **ProfilesSheet**: loadouts, and what an imported one needs. `gameName`, `profiles`, `active`, `actions`, `plan`,
  `focusIndex`, `premium`; signals `activated(i)`, `closed()`.
- **CodeSheet**: a Thunderstore profile code. `info`; signals `copyClicked()`, `closed()`.
- **RemovedSheet**: recently removed mods. `mods` ({id, name, removedAt}), `focusIndex`; signals `activated(i)`,
  `deleteClicked(i)`, `closed()`.
- **PluginSheet**: one plugin's details. `plugin`, `position`, `notes`; signal `closed()`.
- **FomodSheet**: a FOMOD installer's choices. `info`, `page`, `focusIndex`, `askStop`, `stopFocus`; signals
  `choose(group, option)`, `next()`, `back()`, `stop()`; Main.qml sends keys to `handleButton(b)`.
- **SettingsEditor**: Edit settings for a mod's or the game's config file. `title`, `sub`, `pinFirst`, `doc`,
  `textMode`, `focusIndex`, `extraRows`; signals `saveClicked()`, `optionsClicked()`, `closeClicked()`,
  `actionClicked(key)`, `typeAsked(row, value, label)`; `handleButton(b)`.
- **Tutorial**: the beginner cards. `card`, `count`, `viewGlyph`, `startGlyph`, `keys`, `thunderstore`; signals
  `next()`, `skip()`, `back()`.
- **Toast**: a short message at the bottom. `kind` ("info", "done", "problem"), `timeout`, `bottomMargin`,
  `actionLabel`, `actionGlyph`; `show(text, strong, detail)`, `hide()`; signals `hidden()`, `actionClicked()`,
  `pressedOn()`, `pressEnded()`.

## Labs

- **ActivityLog**: the last 500 lines of the activity log. `lines`, `focusAt`, `full`; signals `copyClicked()`,
  `saveClicked()`, `closeClicked()`; `handleButton(b)`.
- **LabsReport**: Report a bug. `shotUrl`, `item`, `text` and its include switches; signals `sendClicked()`,
  `closeClicked()`, `editStarted()`, `editEnded()`; `open(...)`, `handleButton(b)`.
- **PerfCheck**: the performance check. `win` (required), `version`, `extra`; signals `copyClicked(text)`,
  `reportClicked(text)`, `closeClicked()`; `open()`, `handleButton(b)`.

## Over other apps

- **StatusOverlay**: the window that carries the status card over the browser (`hud`: the Game Mode overlay that
  takes no input, else the Desktop panel). `status`, `fomod`; the same signals as StatusCard.
- **StatusCard**: the card itself. `status`, `fomod`, `interactive`; signals `confirm(yes)`, `pick()`,
  `choose(group, option)`, `next()`, `back()`, `stop()`, `openTurbine()`.
- **StatusLine**: a state icon, bold words and a detail (FomodSheet's top line). `level` (ok, busy, warn, stop),
  `words`, `detail`.

## The build's checks

- **SelfCheck**: what each `--page=` picture of tools/ci-shots.sh sets up, presses and logs. Main.qml loads it only
  in screenshot runs (`shot.on`), passes itself as `app` and lists the items the checks reach by id in `checkIds`.
