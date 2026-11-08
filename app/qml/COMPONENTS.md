# QML components

One component per file in this folder. Every .qml file here is part of the Turbine module (see app/build.rs), so
Main.qml can use them by name with no extra import. They only draw and report taps: the controller still goes
through `app.act()`, which moves `focusIndex` and calls the actions. Taps on rows, buttons and pills emit the
signals below, so touch works too.

## Controls (0.54): one focus ring, one meaning per button

| Button | Meaning everywhere |
| --- | --- |
| A | Do the focused thing |
| B | Back one level, focus put back where it was. Hold 1 s: home |
| X | Quick menu for the focused item (max 6 rows) |
| Y | Find (My mods, Load order) or Search (Get mods) |
| LB / RB | Previous / next tab of this view: game tabs, Settings sections, installer steps; on a mod page the previous / next result. Never another game |
| LT / RT | Pointer hidden: a page up / down (move mode: top / bottom). Pointer showing: LT right click, RT left click |
| R3 | Left mouse button. Pointer showing: press and release at the pointer, held it drags. Pointer hidden: the pointer shows on the focused item and A runs |
| Start | Play; on a sheet its main button. Hold 0.6 s: Play options |
| View | The quick wheel |
| D-pad, left stick pushed | Move the focus (repeat 380 ms, then 85 ms, 5 rows at a time after 1.5 s) |
| Left stick swept round | Scroll wheel, clockwise down; every 30 degrees one notch (3 rows) |
| Right stick | The pointer: hover moves the focus ring, it hides on any other press or a touch, or after 3 s still |
| Grips | Only when a layout sends them: L4/R4 are LT/RT, L5 is Y, R5 is View. Never in the legend |

The left stick (pointer.rs `left_stick`): pushed past 0.5 and kept within 25 degrees for 80 ms it is one D-pad
press (released under 0.35); swept 40 degrees first it is the wheel until it comes back under 0.35.

### One table: `binds()`

`binds()` in Main.qml answers what each button does in the context on screen right now:
`{a, b, x, y, start, lb, rb, lt, rt, view}`, each `{label, run, pre}`. `act()` runs it; `legendFor(binds())` is the
legend, so the legend can't promise something a button doesn't do. The D-pad and the stick go to `navigate()` and
are never in the legend. Sheets answer through `sheetBinds()`.

Legend rules: at most 5; A, B, X, Y in that order, then one more (the pointer's "RT Click, LT Menu" while it shows,
or Start when a sheet covers Play). LB/RB are caps at the ends of the tab bar (and the Settings sections and the mod
page header), View is the cap on the quick wheel's button, Start is the cap on Play: never printed twice. Holds read
"hold A Delete". After a touch the legend hides until the next press.

### Focus

Zones: the rail, Play (in the header), the content. Left at the first column reaches the rail; Right or A comes
back. Up from the top row reaches the filter bar, then Play. Tab bars aren't focusable (LB/RB, a swipe). Each game
remembers its tab and row (`focusMemo`, `lastFocusId`), each tab its own row; Get mods remembers its search and card.

| After | Focus lands on |
| --- | --- |
| Toggle | the same row |
| Uninstall | the same row, dimmed "Removed: name. A puts it back." for 8 s, then the next row |
| Move put down or cancelled | the moved row |
| Get it on Nexus | the same button, "Opening Nexus..." (A does nothing for 8 s) |
| A sheet or menu closes | the item that opened it |
| B on a mod page | its card |
| Switching games | the game's tab and row; first visit: the first mod (a game with no mods: Get mods, first card) |
| B in Settings | the screen and row Settings was opened from |
| A mod arrives | Play |

No sheet opens without a press: the ones that come from outside (a download, an installer's questions, install notes)
say why in the activity log; anything else is logged as "opened with no press" and fails the build's checks.

### Touch

Tap does it. Hold 0.5 s: the quick menu (mod rows, cards, plugins). A sideways swipe on the tab bar is LB/RB. Tap
outside a sheet closes it. A touch moves the ring, hides the pointer and the legend.

## QuickWheel

View's wheel: eight places around a circle, the same on every screen; a place with nothing to do here is dimmed,
never moved. Tap View: it stays open, the D-pad picks (Up then Right is Up-right), A does it, B closes. Hold View,
push, let go of View: that place. In Labs the middle is Report.

| Place | My mods | Get mods | Load order |
| --- | --- | --- | --- |
| Up | Find | Search | Find plugin |
| Up-right | Sort | Sort | Sort |
| Right | Show | Category | Show (dimmed) |
| Down-right | Play options | Play options | Play options |
| Down | Full menu | Full menu | Full menu |
| Down-left | Settings | Settings | Settings |
| Left | Games | Games | Games |
| Up-left | Updates (n) | Add a file | Loadouts |

Properties: `slots` ([{label, ok}]), `focusIndex` (-1 the middle), `labs`, `viewGlyph`. Signals: `activated(index)`,
`closed()`. Full menu is GameMenu with Activity log and Help added.

## SheetButtons

The row of buttons on every sheet with buttons (Before you play, the loader prompt, InfoSheet, AskText, Delete for
good). `labels`, `focusIndex` (the highlight), `mainIndex` (Start's button, drawn a little stronger). Signal
`activated(index)` (a tap). The rule lives in Main.qml's `buttonSheet()`: the D-pad moves the highlight, A presses
it, B is the cancel button, Start the main one. No key caps on the buttons and no other keys.

## Tutorial

The beginner tutorial, four cards on first run with the real buttons drawn large: A picks and B goes back with X
options and Y find; View and the wheel; getting a mod (Get mods, A on a mod, Slow download on Nexus, Turbine adds
it); Start plays. `card`, `count`, `viewGlyph`, `startGlyph`; signals `next()` and `skip()`. A or Right: next, Left:
back, B or Start: skip. The last card's A says "Pick a game to start" and lands on the rail. Remembered once seen
(`seen.tutorialDone`); Settings, Help shows it again.

## The `ui` object

Every component has `required property var ui`. It must give:

| Member | What |
| --- | --- |
| `ui.c` | colors: `ground`, `panel`, `card`, `raised`, `line`, `text`, `soft`, `muted`, `accent`, `accentHi`, `accentLo`, `onAccent`, `focusFill`, `ok`, `warn`, and `light` (bool). `stop` is optional (a red-orange is used without it). |
| `ui.px(n)` | design pixels to screen pixels |
| `ui.titleFont` | font family for titles (Cinzel) |
| `ui.bodyFont` | font family for everything else (Barlow) |

Main.qml's `c` already has every color, so the wiring is one object next to it:

```qml
readonly property var colors: c          // on the Window (app)
QtObject {
    id: kit
    readonly property var c: app.colors
    readonly property string titleFont: app.titleFont
    readonly property string bodyFont: app.bodyFont
    function px(v) { return app.px(v) }
}
```

Then pass `ui: kit` to each component. (Keep the id different from `ui`: inside a component instance, `ui: ui`
would read the instance's own property.) Theme, accent, text size and contrast changes show right away, because the
components read through `ui` in their bindings. Text that carries a color (badges, warnings, brass words) is
darkened or lightened by the component itself until it reads at 4.5:1 on its background, on every theme, accent
and color vision mode. Suggested addition to `c`: `readonly property color stop: light ? "#8E2A1E" : "#F0907A"`.

Controller keys are passed as plain names (`"a"`, `"b"`, `"x"`, `"y"`, `"lb"`, `"start"`, `"view"`) and drawn in
capitals; one letter draws a round cap, longer names a square one.

## ModRow

One mod in the list. Height 54 (design px); set `width`.

| Property | Type | Default | What |
| --- | --- | --- | --- |
| `name` | string | "" | mod name |
| `detail` | string | "" | the one line under it, e.g. "5.2 · Nexus" |
| `isOn` | bool | true | ON pill (filled) or OFF pill (outlined); off rows are dimmer but stay at 4.5:1 |
| `ordered` | bool | false | show the order number (only for games where order matters) |
| `order` | int | 0 | the number shown |
| `badge` | string | "" | one action badge, e.g. "Needs SKSE"; empty hides it |
| `badgeLevel` | string | "warn" | `ok`, `info` (brass), `warn`, `stop` |
| `focused` | bool | false | focus look |
| `showHints` | bool | `focused` | show "A Turn off · X More" (Main.qml sets false: the legend says it) |
| `aLabel` | string | "Turn off" / "Turn on" | follows `isOn`; "" hides the A hint |
| `xLabel` | string | "More" | "" hides the X hint |

Signals: `clicked()` (row tapped), `toggleClicked()` (ON/OFF pill tapped), `menuClicked()` (right click, or a 0.5 s
hold on touch: the quick menu).

```qml
ListView {
    id: modList
    model: app.mods
    spacing: app.px(4)
    delegate: ModRow {
        required property int index
        required property var modelData
        ui: kit
        width: modList.width
        name: modelData.name || ""
        detail: modelData.detail || ""
        isOn: modelData.on === true
        ordered: !!(app.game && app.game.ordered)
        order: index + 1
        badge: modelData.badge || ""
        badgeLevel: modelData.badgeLevel || "warn"
        focused: app.zone === "main" && app.modIndex === index
        onClicked: app.modIndex = index
        onToggleClicked: { app.modIndex = index; app.act("a") }
    }
}
```

## StatusLine

The line under the header. Height 44; set `width`.

| Property | Type | Default | What |
| --- | --- | --- | --- |
| `level` | string | "ok" | `ok` (check), `busy` (turning arc), `warn` (!), `stop` (cross). Named `level` because Item already has `state`. |
| `words` | string | "" | bold state words; warn and stop color them |
| `detail` | string | "" | muted words after them |
| `pillText` | string | "" | right side pill, e.g. "2 updates"; empty hides it |
| `pillKey` | string | "" | optional key inside the pill |
| `progress` | real | -1 | 0..1 shows a brass bar on the right instead of the pill; below 0 hides it |
| `progressText` | string | "" | words after the bar, e.g. "64% · 1.1 of 1.7 MB" |

Signal: `pillClicked()`.

```qml
StatusLine {
    ui: kit
    width: parent.width
    level: Turbine.applying ? "busy" : app.problems > 0 ? "warn" : "ok"
    words: Turbine.applying ? "Applying 1 change to the game..." : "Ready to play"
    detail: Turbine.applying ? "Removing Epic Loot" : "15 of 17 mods on"
}
```

## ModMenu

The X menu for one mod: a side sheet on the right over a dimmed backdrop. Fills its parent; use `visible`.

| Property | Type | Default | What |
| --- | --- | --- | --- |
| `title` | string | "" | mod name (shown in capitals) |
| `subtitle` | string | "" | e.g. "0.4.19.16 · Nexus · On, 3rd in load order" |
| `items` | array | [] | `{ label, detail, enabled, icon, danger, more }`; `enabled` defaults to true, `danger` colors the label, `more` shows › |
| `focusIndex` | int | 0 | focused item; scrolled into view by itself |
| `overlapWins` | array of strings | [] | e.g. "4 files from SkyUI" |
| `overlapLoses` | array of strings | [] | e.g. "1 file to Unofficial Skyrim Special Edition Patch" |
| `showOverlaps` | bool | either list has lines | set true to show "No other mod changes the same files." |
| `overlapTitle`, `noOverlapText` | string | "Overlaps", ... | words for the block |
| `bottomInset` | real | `ui.px(56)` | room left for the legend under the sheet |

`icon` is one of `update`, `power`, `overlap`, `files`, `move`, `remove`, `play`, `restore`, `profile`, `info`, or
SVG path data on a 24 x 24 grid.

Signals: `activated(int index)` (item tapped), `closed()` (backdrop tapped).

```qml
ModMenu {
    ui: kit
    visible: app.sheet === "mod"
    title: app.mod ? app.mod.name : ""
    subtitle: "0.4.19.16 · Nexus · On, 3rd in load order"
    items: [
        { label: "Turn off", detail: "Stays installed", icon: "power" },
        { label: "Move to...", detail: "Pick its place in the load order", icon: "move", more: true },
        { label: "Uninstall", detail: "Kept in Recently removed for 7 days", icon: "remove", danger: true }
    ]
    focusIndex: app.menuIndex
    overlapWins: ["4 files from SkyUI"]
    overlapLoses: ["1 file to Unofficial Skyrim Special Edition Patch"]
    onActivated: (index) => { app.menuIndex = index; app.act("a") }
    onClosed: app.act("b")
}
```

## GameMenu

The View menu. Item 0 is Play original, item 1 is Restore last working setup, then `extraItems`. A centered card
like the approved mockup; `placement: "side"` makes it a side sheet like ModMenu. Fills its parent; use `visible`.

| Property | Type | Default | What |
| --- | --- | --- | --- |
| `gameName` | string | "" | title (shown in capitals) |
| `subtitle` | string | "Game menu" | |
| `originalDetail` | string | "Starts the game with no mods. Nothing is deleted." | |
| `restoreDate` | string | "" | e.g. "From Nov 4, 9:12 PM · 16 mods on" |
| `restoreAvailable` | bool | `restoreDate !== ""` | false greys the item and shows `restoreEmptyText` |
| `restoreEmptyText` | string | "Nothing saved yet. ..." | |
| `extraItems` | array | [] | same shape as ModMenu items, plus `key` |
| `focusIndex` | int | 0 | counts the two fixed items |
| `placement` | string | "center" | `center` or `side` |
| `bottomInset` | real | `ui.px(56)` | side placement only |

Signals: `activated(int index, string key)` (key is `original`, `restore` or the extra item's key), `closed()`.

```qml
GameMenu {
    ui: kit
    visible: app.sheet === "game"
    gameName: app.game ? app.game.name : ""
    restoreDate: app.lastGood ? "From " + app.lastGood.when + " · " + app.lastGood.count + " mods on" : ""
    extraItems: [{ key: "removed", label: "Recently removed", detail: "Mods kept for 7 days", icon: "remove", more: true }]
    focusIndex: app.menuIndex
    onActivated: (index, key) => { app.menuIndex = index; app.act("a") }
    onClosed: app.act("b")
}
```

## PrePlayCheck

"Before you play", a centered card. Fills its parent; use `visible`.

| Property | Type | Default | What |
| --- | --- | --- | --- |
| `title` | string | "Before you play" | shown in capitals |
| `checks` | array | [] | `{ level: "ok" | "warn" | "stop", title, detail }`; warn and stop lines are tinted |
| `buttons` | array | [] | up to 4 `{ label, main }` (SheetButtons): Fix it, Play anyway, Play without mods, Not now |
| `focusIndex` | int | 0 | the highlighted button (A presses it; B is Not now, Start Play anyway when nothing stops play) |
| `note` | string | "" | small line under the buttons |

Signals: `activated(int index, string key)`, `closed()`.

```qml
PrePlayCheck {
    ui: kit
    visible: app.sheet === "check"
    checks: [
        { level: "ok", title: "Mods applied", detail: "15 mods on, all in place" },
        { level: "warn", title: "SKSE needs an update", detail: "Steam updated Skyrim today. 6 of your mods won't load until SKSE catches up." },
        { level: "ok", title: "Space OK", detail: "41 GB free" }
    ]
    buttons: app.checkChoices.map(label => ({ label: label, main: label === "Play anyway" }))
    focusIndex: app.checkIndex
    onActivated: (index, key) => { app.checkIndex = index; app.checkChoose(index) }
    onClosed: app.act("b")
}
```

## Toast

A short message at the bottom center of its parent, above the legend. Hidden until `show()`.

| Property | Type | Default | What |
| --- | --- | --- | --- |
| `text` | string | "" | e.g. "Recently removed:" |
| `strong` | string | "" | bold part after it, e.g. "Epic Loot" (shown as plain text, never as markup) |
| `detail` | string | "" | muted, e.g. "Kept 7 days" |
| `actionKey`, `actionLabel` | string | "" | e.g. "y", "Undo"; both empty hides the action |
| `timeout` | int | 5000 | ms before it hides itself; 0 keeps it up |
| `bottomMargin` | real | `ui.px(58)` | distance from the parent's bottom |
| `shown` | bool | false | read it to know whether Y should undo |

Functions: `show(text, strong, detail, actionKey, actionLabel)` (the last four optional), `hide()`.
Signals: `actionClicked()`, `hidden()`.

```qml
Toast {
    id: toast
    ui: kit
    onActionClicked: app.act("y")
}
// later: toast.show("Recently removed:", "Epic Loot", "Kept 7 days", "y", "Undo")
```

Note: Main.qml's current toast has id `toast` and a `show(m)` function; this one takes the same first argument, so
`toast.show("Some words")` keeps working after the swap.

## AddMods

"Add mods": a side sheet listing zip files in Downloads. Fills its parent; use `visible`.

| Property | Type | Default | What |
| --- | --- | --- | --- |
| `files` | array | [] | `{ name, size, age, note }`, e.g. `{ name: "SkyUI_5_2_SE.zip", size: "2.6 MB", age: "2 days ago" }`; `note` is optional ("Already added") |
| `focusIndex` | int | 0 | focused file (shows "A Add"); kept in view |
| `title`, `subtitle` | string | "Add mods", "Zip files in your Downloads folder" | |
| `actionLabel` | string | "Add" | |
| `emptyText` | string | "No zip files in Downloads. Download a mod's zip in Desktop Mode, then come back." | |
| `laterText` | string | "Browsing and downloading mods online comes in a later version." | pinned to the bottom; "" hides it |
| `bottomInset` | real | `ui.px(56)` | room left for the legend |

Signals: `activated(int index)`, `closed()`.

```qml
AddMods {
    ui: kit
    visible: app.sheet === "add"
    files: app.zips
    focusIndex: app.addIndex
    onActivated: (index) => { app.addIndex = index; app.act("a") }
    onClosed: app.act("b")
}
```

## LoaderPrompt

"These mods need SKSE (Skyrim Script Extender)", a centered card. Fills its parent; use `visible`.

| Property | Type | Default | What |
| --- | --- | --- | --- |
| `title` | string | "Mod loader needed" | e.g. "Script Extender needed" |
| `loaderName`, `loaderFullName` | string | "" | e.g. "SKSE", "Skyrim Script Extender" |
| `explain` | string | "" | what the loader is, in plain words |
| `neededBy` | array of strings | [] | mod names, shown as "Needed by: ..." |
| `sourceUrl` | string | "" | official home, e.g. "skse.silverlock.org" |
| `canDownload` | bool | false | false (0.2): an info line, buttons A "Show where to get it" and B "Close". true: A "Install", B "Not now" |
| `sourceShown` | bool | false | shows `sourceUrl` as text; set it when A is pressed in the 0.2 variant |
| `infoText` | string | "This test version doesn't download anything. Get SKSE from its official site in Desktop Mode, then add its zip with Add mods." | |
| `buttons` | array | [] | `{ label }` (SheetButtons); empty uses the defaults above |
| `focusIndex` | int | 0 | |

Signals: `activated(int index, string key)`, `closed()`.

```qml
LoaderPrompt {
    ui: kit
    visible: app.sheet === "loader"
    title: "Script Extender needed"
    loaderName: "SKSE"
    loaderFullName: "Skyrim Script Extender"
    explain: "SKSE is a free add-on that lets mods run their own code inside Skyrim."
    neededBy: ["SkyUI", "RaceMenu"]
    sourceUrl: "skse.silverlock.org"
    sourceShown: app.loaderSourceShown
    buttons: app.loaderLabels.map(l => ({ label: l }))
    focusIndex: app.loaderIndex
    onActivated: (index, key) => { app.loaderIndex = index; app.act("a") }
    onClosed: app.act("b")
}
```

## EmptyState

Title, body and an optional button, centered in whatever area it is given.

| Property | Type | Default | What |
| --- | --- | --- | --- |
| `title` | string | "" | e.g. "No mods yet" |
| `body` | string | "" | |
| `buttonKey`, `buttonLabel` | string | "" | both empty hides the button |
| `buttonFocused` | bool | true | |
| `note` | string | "" | small line under the button |
| `maxWidth` | real | `ui.px(560)` | |

Signal: `activated()` (button tapped).

```qml
EmptyState {
    ui: kit
    anchors.fill: parent
    visible: app.mods.length === 0
    title: "No mods yet"
    body: "Add mods from zip files in your Downloads folder."
    buttonKey: "a"
    buttonLabel: "Add mods"
    onActivated: app.act("a")
}
```

## Seasonal

Falling leaves (`kind: "autumn"`) or snow (`kind: "snow"`) in one Canvas, drawn behind everything; `""` hides it.
`amount` (10 to 100, for a 1280 x 800 window), `size` (50 to 200 percent), `scale` (design to screen pixels),
`light` (colors for the Light theme) and `moving` (false holds it still; Main.qml passes the window being active).

## LabsReport

Labs > Report a bug, in the same shape as the owner's other app: the screenshot, Include screenshot, what went wrong
(typed with Steam's keyboard, up to 2000 characters), Include performance check, Include crash log, Cancel and Send.
Main.qml calls `open(itemTitle, startText, perfText, hasCrashLog)`, passes buttons to `handleButton(b)` and saves on
`sendClicked`. `editStarted` and `editEnded` say when typing starts and stops (Steam's keyboard opens and closes).

## PerfCheck

Labs > Performance check: Run check (about 13 seconds: still, scrolling, idle), Copy report, Report a bug. Needs `win`
(the Window, for its frame times). Signals `copyClicked(text)`, `reportClicked(text)`, `closeClicked()`.

## 0.3 components

- `GetMods`: the Get Mods tab (search, source, sort, Add from Downloads, categories, mod cards in three columns).
  Main.qml gives `result` (the engine's browse answer), `bar`, `focusArea` ("bar", "cats", "grid") and `focusIndex`.
- `ModDetails`: a mod's page, its own full screen (header "Get mods > Name", no Back button, LB/RB the previous or
  next result). `info` is the engine's details answer; the main button has the focus. `helpLabels` and `helpFocus`:
  the two buttons that show under the main one 8 seconds after Open ("Open in Steam's browser", "Always use Steam's
  browser"), `helpClicked(index)`.
- `FomodSheet`: installer choices, one question per page with a picture preview. Main.qml passes buttons to
  `handleButton(b)`; it emits `choose(group, option)`, `next()`, `back()` and `stop()`.
- `LoadOrderList`: the Load order tab (Skyrim). `plugins` is the engine's load order, `focusIndex` -1 is Sort
  automatically.
- `ProfilesSheet`: Profiles from the game menu, and a share being imported (`plan`).
- `AskText`: one line of text typed with Steam's keyboard: `open(title, note, start, placeholder, max, secret,
  done)`, then `accepted(text)` or `cancelled()`. `focusIndex` -1 is the field (A opens Steam's keyboard again), then
  Clear, Cancel and Done (SheetButtons); Start is Done.
