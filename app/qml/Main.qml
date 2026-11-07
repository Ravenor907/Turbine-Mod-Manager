import QtQuick
import QtQuick.Window
import QtQuick.Shapes
import QtCore
import Turbine

// Turbine Mod Manager's interface: the game rail, a game's page (its mods, Get Mods, the mod and game menus, the check
// before Play), Settings, game details, a notice for systems that aren't SteamOS and a 2,000 row list to test scrolling. Everything is sized from a design size the way
// Huginn's handheld mode is: 883 x 552 on a Steam Deck drawn at 1.45x, 914 x 514 on a 7 inch 1080p handheld at 2.1x.
Window {
    id: app
    // qtglue.cpp finds this window by name to deliver the controller pointer's mouse and wheel events.
    objectName: "turbineMain"
    visible: true
    title: "Turbine Mod Manager"
    width: shot.on ? shot.w : 1280
    height: shot.on ? shot.h : 800
    visibility: (Turbine.gamemode && !shot.on) ? Window.FullScreen : Window.Windowed
    color: c.ground

    // ---------------------------------------------------------------- settings that last between launches
    Settings {
        id: look
        category: "look"
        property string theme: "brass"      // brass, black, light
        property string accent: "brass"     // brass, iron, copper, jade, fjord, heather, ember
        property int textSize: 100          // percent (before 0.5; moved into size once)
        property int size: 0                // Interface size, percent (80 to 150); 0 until set or moved over
        property bool highContrast: false
        property string vision: "typical"   // typical, red, green, blue, none
        property string layout: "handheld"  // handheld, desktop, tv (tv only with an external display)
        property string season: ""          // off, date, autumn, snow; "" until chosen (off on handhelds, by date on TV)
        property int fallAmount: 40         // 10 to 100
        property int fallSize: 100          // 50 to 200 percent
    }
    // Labs mode (ten presses on the icon in Settings > About) and the owner's checklist answers.
    Settings {
        id: labsSet
        category: "labs"
        property bool on: false
        property string states: "{}"        // item id: "works" or "problem"; missing means not tested
    }
    Settings {
        id: seen
        category: "seen"
        property bool notSteamOS: false
        property string lastGame: ""
        property string launchDone: "[]"    // games whose one-time Steam Launch Option note was seen
        property string onceDone: "[]"      // "<game id>|<title>": check lines read once (REDmod)
        property string played: "[]"        // game ids, last played first (the rail's order)
        property string bannerDone: "[]"    // games whose "No mods yet" line on Get mods was seen
    }
    // Get mods: the last 8 mod pages opened, per game ({"<game id>": [{id, name, icon, source}]}).
    Settings {
        id: getSet
        category: "getmods"
        property string recent: "{}"
    }
    // Settings > Accounts > Browser.
    Settings {
        id: browserSet
        category: "browser"
        property bool overlay: true         // Show status over other apps
    }
    // Settings > Updates (and Labs > Update channel).
    Settings {
        id: updates
        category: "updates"
        property bool auto: true
        property bool prerelease: false
    }

    // ---------------------------------------------------------------- screenshot mode (used by the build checks)
    QtObject {
        id: shot
        readonly property var args: Qt.application.arguments
        function arg(name) {
            for (let i = 0; i < args.length; i++)
                if (args[i].startsWith("--" + name + "=")) return args[i].substring(name.length + 3)
            return ""
        }
        readonly property string path: arg("screenshot")
        readonly property bool on: path !== ""
        readonly property int w: parseInt(arg("size").split("x")[0]) || 1280
        readonly property int h: parseInt(arg("size").split("x")[1]) || 800
        readonly property string page: arg("page")
        // Pages that wait for the engine (imports, menus, the check) get more time before the picture.
        readonly property bool slow: ["pointer", "pointerscroll", "checkend", "caught", "nexusnokey", "mods",
                                      "getmods", "getmodsfallback", "modpagefree", "modpagepremium", "getmodsback", "emptygame", "emptygamemods",
                                      "removed", "newmod", "opening", "opening8s", "desktop", "desktopgrid", "tv", "rail", "menu", "gamemenu", "check", "add", "loader", "report", "browse", "moddetails", "fomod",
                                      "loadorder", "profiles", "accounts", "updates", "lootsort", "lootnotes", "versions", "code",
                                      "nexusdetails", "nexusversions", "nexusfree", "nxm", "browser", "helper"].indexOf(page) >= 0
        // Pages that show Labs mode (it isn't saved from a picture).
        readonly property bool labs: ["about", "labs", "report", "perfcheck"].indexOf(page) >= 0
        // --layout=desktop or tv (not saved from a picture); --external-display pretends an HDMI display is there.
        readonly property string layout: arg("layout")
        readonly property bool external: args.indexOf("--external-display") >= 0
        function start() {
            const files = arg("selftest-import")
            // "file|thunderstore|Owner-Name|1.0.3" adds a zip as if it came from Thunderstore.
            if (files !== "") for (const f of files.split(",")) {
                const p = f.split("|")
                if (p.length === 4) Turbine.selftestImport(p[0], p[1], p[2], p[3]); else Turbine.importZip(f)
            }
            if (page === "gamemenu") app.sheet = "gamemenu"
            else if (page === "add") { app.tab = 1; app.openAdd() }
            else if (page === "about") app.openSection("About")
            else if (page === "labs") {
                app.labsAnswers = { "pad-quiet": "works", "proton-bepinex": "problem" }
                app.openSection("Labs"); app.setRow = 5
            }
            else if (page === "browse" || page === "moddetails" || page === "versions") { app.tab = 1; app.zone = "main" }
            else if (page === "nexusdetails" || page === "nexusversions" || page === "nexusfree") {
                if (page === "nexusfree") Turbine.setSampleFree()
                app.tab = 1; app.zone = "main"
            }
            else if (page === "loadorder" || page === "lootsort" || page === "lootnotes") { app.tab = 2; app.zone = "main" }
            else if (page === "accounts") app.openSection("Getting mods")
            // The browser: Settings > Accounts with a browser found and the helper connected; the no-browser
            // warning; the helper's setup steps; the status card in both its windows.
            else if (page === "browser" || page === "helper" || page === "nobrowser") {
                app.browser = page === "nobrowser" ? { checked: true, result: { found: false, name: "", how: "" }, title: "No web browser found",
                    text: "Turbine opens Nexus Mods in your web browser, and this system doesn't have one yet. Installing one takes a minute.",
                    steps: ["Switch to Desktop Mode: press the Steam button, open Power, and pick Switch to Desktop.",
                            "In Desktop Mode, open Discover, search for Google Chrome or Firefox, and press Install.",
                            "Optional, so it also opens in Game Mode: open the app menu, right-click the browser, and pick Add to Steam.",
                            "Switch back to Game Mode and open Turbine again."] }
                    : { checked: true, result: { found: true, name: "Google Chrome", how: "flatpak" } }
                app.bridge = { running: true, port: 47701, paired: true, connected: page === "browser" ? 1 : 0, ext: "iniglfblobfbekihldmgekchfehncbeb", error: "" }
                app.openSection("Getting mods")
                if (page === "helper") { app.helperFolder = "/home/deck/Downloads/Turbine browser helper"; app.infoIndex = 0; app.sheet = "helper" }
                else if (page === "nobrowser") app.noBrowser()
            }
            else if (page === "overlay" || page === "overlaygm") {
                app.status = { active: true, text: "Added to Cyberpunk 2077", level: "ok", progress: -1, game: "Cyberpunk 2077",
                               added: "Cyber Engine Tweaks 1.32.0", done: true, ask: "" }
                app.showOverlay(true, page === "overlaygm")
            }
            else if (page === "updates") { app.openSection("Updates"); Turbine.selfCheck(false, false) }
            // "Found <file>. Add it to <game>?" for a download Turbine wasn't sure about.
            else if (page === "caught") {
                app.found = { path: "/home/deck/Downloads/Appearance Menu Mod-790-2-6-1.zip", name: "Appearance Menu Mod-790-2-6-1.zip", size: 18874368,
                              game: app.game ? app.game.id : "", gameName: app.game ? app.game.name : "", why: "it is the game last open in Turbine", more: 1 }
                app.status = { active: true, text: "Found " + app.found.name + ". Add it to " + app.found.gameName + "?", level: "warn", progress: -1,
                               game: app.found.gameName, added: "", done: false, ask: app.found.path }
                app.infoIndex = 0; app.sheet = "found"
            }
            else if (page === "caughtgm") {
                app.status = { active: true, text: "Found Appearance Menu Mod-790-2-6-1.zip. Add it to Cyberpunk 2077?", level: "warn", progress: -1,
                               game: "Cyberpunk 2077", added: "", done: false, ask: "/home/deck/Downloads/Appearance Menu Mod-790-2-6-1.zip" }
                app.showOverlay(true, true)
            }
            else if (page === "nexusnokey" || page === "getmodsfallback" || page === "opening" || page === "opening8s") {
                Turbine.setSampleNexusFail(); app.tab = 1; app.zone = "main"
            }
            else if (page === "getmods" || page === "getmodsback" || page === "desktopgrid" || page === "tv") {
                // Two seeded Recently viewed mods.
                app.recentSeed = [{ id: "2", name: "Lantern Glow SKSE", icon: app.fileUrl(arg("selftest-data") + "/thumb2.png"), source: "nexus" }, { id: "3", name: "Quiet Snow", icon: app.fileUrl(arg("selftest-data") + "/thumb3.png"), source: "nexus" }]
                app.tab = 1; app.zone = "main"
            }
            else if (page === "modpagefree" || page === "modpagepremium") {
                if (page === "modpagefree") Turbine.setSampleFree()
                app.tab = 1; app.zone = "main"
            }
            else if (page === "removed") app.modIndex = app.listItems.length - 1
            else if (page === "rail") { app.zone = "rail"; app.railIndex = 0 }
            else if (page === "size150" || page === "tvhidden") {
                app.openSection("Look")
                if (page === "size150") { app.sizePreview(150); app.setRow = 0 }
                else app.setRow = app.lookRows.findIndex(r => r.key === "layout")
            }
            else if (page === "overlaywait") {
                app.status = { active: true, text: "Press Save in the browser", level: "wait", progress: -1, game: "", added: "", done: false, ask: "" }
                app.showOverlay(true, false)
            }
            else if (page === "pointerscroll") { app.page = "perf"; app.zone = "perf" }
            else if (page === "perfcheck") app.openPerf()
            else if (page === "settings" && arg("season") !== "") app.setRow = 6
        }
    }
    Timer {
        running: shot.on && shot.slow
        interval: 2500
        onTriggered: {
            if (shot.page === "menu" && app.menuMod) app.sheet = "modmenu"
            else if (shot.page === "check" || shot.page === "checkend") { app.startPlay(); if (shot.page === "checkend") checkEndShot.start() }
            else if (shot.page === "moddetails" && app.browseData && app.browseData.items && app.browseData.items.length > 0) app.openDetails(app.browseData.items[0])
            else if (shot.page === "versions" && app.browseData && app.browseData.items && app.browseData.items.length > 0)
                app.openDetails(app.browseData.items.find(x => x.id === "x753-More_Suits") || app.browseData.items[0])
            else if ((shot.page === "nexusdetails" || shot.page === "nexusversions" || shot.page === "nexusfree") && app.getItems.length > 0) app.openDetails(app.getItems[0])
            else if (shot.page === "code") { app.openProfiles(); Turbine.profileCode("Default"); app.sheet = "code" }
            else if (shot.page === "profiles") app.openProfiles()
            else if (shot.page === "browser") app.setRow = app.accountRows.findIndex(r => r.key === "overlay")
            else if (shot.page === "lootsort" || shot.page === "lootnotes") { app.orderIndex = -1; Turbine.orderSort() }
            // The controller pointer: over the second mod's ON pill, clicked through the same mouse events the right
            // stick and RT make (the mod turns off), then the 2,000 row list scrolled by wheel steps.
            else if (shot.page === "pointer") app.pointerCheck()
            else if (shot.page === "pointerscroll") app.pointerScrollCheck()
            // After the first frames, so the screenshot inside the report has something in it.
            else if (shot.page === "report") app.openReport(app.labsItems.find(x => x.id === "uninstall-undo"), "Undo put the mod back at the bottom of the list instead of where it was.", "")
            else if ((shot.page === "modpagefree" || shot.page === "modpagepremium") && app.getItems.length > 0) app.openDetails(app.getItems[0])
            else if (shot.page === "getmodsback") { app.getText = "lantern"; app.browseNow(); getBackShot.start() }
            else if (shot.page === "emptygame" || shot.page === "emptygamemods" || shot.page === "removed") {
                // One removed mod (two for the Removed sheet): imported, then removed.
                for (const m of app.mods) Turbine.removeMod(m.id)
                app.landedOn = ""
                if (shot.page === "emptygamemods") { app.tab = 0; app.modIndex = 0 }
                if (shot.page === "removed") removedShot.start()
            }
            else if (shot.page === "opening") { app.getArea = "fallback"; app.getAct("a") }
            else if (shot.page === "opening8s") { app.getArea = "fallback"; app.getAct("a"); app.openHelpNow() }
        }
    }
    Timer {
        running: shot.on
        interval: shot.page.indexOf("loot") === 0 || shot.page.indexOf("nxm") === 0 || shot.page === "getmodsback" ? 6500 : shot.slow ? 4500 : 1500
        // The overlay pictures are of the overlay window itself.
        onTriggered: (shot.page === "overlay" || shot.page === "overlaywait" ? panelWin : shot.page === "overlaygm" || shot.page === "caughtgm" ? hudWin : app).contentItem.grabToImage(function (r) { r.saveToFile(shot.path); Qt.quit() })
    }
    Timer { id: removedShot; interval: 900; onTriggered: app.openRemoved() }
    // getmodsback: the 5th card of the "lantern" search, its page, then B.
    Timer {
        id: getBackShot
        interval: 1500
        property int stepNo: 0
        onTriggered: {
            if (stepNo === 0) { app.getArea = "grid"; app.getIndex = Math.min(4, app.getItems.length - 1); app.getAct("a"); stepNo = 1; start() }
            else { app.act("b"); app.note("shot", "Back from the mod page: search \"" + app.getText + "\", card " + (app.getIndex + 1)) }
        }
    }

    // ---------------------------------------------------------------- size
    // Layout: handheld, desktop (mouse in Desktop Mode: smaller, denser rows, 4 columns) or tv (only with an
    // external display). The whole interface is drawn from a design size and never grows past what fits the window:
    // 760 x 460 design units is the smallest it reflows into.
    readonly property string layoutNow: shot.layout !== "" ? shot.layout : (look.layout === "tv" && !externalShown) ? "handheld" : look.layout
    readonly property bool tv: layoutNow === "tv"
    readonly property bool desktopLayout: layoutNow === "desktop"
    readonly property int sizeDefault: tv ? 120 : desktopLayout ? 80 : 100
    readonly property int sizeNow: sizeTrial > 0 ? sizeTrial : look.size > 0 ? look.size : sizeDefault
    readonly property real fit: Math.max(0.5, Math.min(width / 883, height / 514))
    readonly property real s: Math.max(0.4, Math.min(fit * sizeNow / 100, width / 760, height / 460))
    function px(v) { return Math.round(v * s) }
    // Design width: 3 grid columns on a handheld, 4 from 1100 (TV and Desktop layouts), 2 below 860.
    readonly property real designWidth: width / s
    readonly property int gridColumns: desktopLayout || designWidth >= 1100 ? 4 : designWidth < 860 ? 2 : 3
    readonly property int rowHeight: desktopLayout ? 38 : 0
    // An external display: HDMI or DisplayPort in /sys, or a Qt screen that isn't the built-in panel.
    property bool externalShown: shot.external
    function checkDisplays() {
        if (shot.on) return
        let ext = Turbine.externalDisplay()
        for (const sc of Qt.application.screens) if (sc.name && !sc.name.startsWith("eDP")) ext = true
        if (ext !== externalShown) note("window", ext ? "External display connected" : "No external display")
        const lost = externalShown && !ext && look.layout === "tv"
        externalShown = ext
        if (lost) { look.layout = "handheld"; toast.info("TV unplugged. Back to the handheld layout.") }
    }
    Connections { target: Qt.application; function onScreensChanged() { app.checkDisplays() } }
    // Interface size and Layout preview live; "Keep this size?" puts them back after 10 seconds unless kept.
    property int sizeTrial: 0
    property string layoutBefore: ""
    property int sizeBefore: 0
    property int keepLeft: 10
    function sizePreview(v) {
        if (sheet !== "keepsize") { sizeBefore = look.size; layoutBefore = look.layout }
        sizeTrial = v
        keepLeft = 10; keepTimer.restart(); sheet = "keepsize"
    }
    function layoutPreview(v) {
        if (sheet !== "keepsize") { sizeBefore = look.size; layoutBefore = look.layout }
        look.layout = v
        keepLeft = 10; keepTimer.restart(); sheet = "keepsize"
    }
    function keepSize(yes) {
        keepTimer.stop()
        if (yes) { if (sizeTrial > 0) look.size = sizeTrial; note("action", "Kept size " + sizeNow + "%, layout " + look.layout) }
        else { look.layout = layoutBefore || look.layout; note("action", "Size put back") }
        sizeTrial = 0
        if (sheet === "keepsize") sheet = ""
    }
    Timer { id: keepTimer; interval: 1000; repeat: true; onTriggered: { if (shot.on) return; app.keepLeft--; if (app.keepLeft <= 0) app.keepSize(false) } }

    // ---------------------------------------------------------------- Labs mode and the activity log
    readonly property bool labsOn: labsSet.on || (shot.on && shot.labs)
    function note(kind, text) { Turbine.log(kind, text) }
    onPageChanged: note("page", "Page " + page)
    onTabChanged: {
        note("page", "Tab " + (tabs[tab] || tab))
        modPage = false
        history = []
        if (tab === 1 && !browseData) browseNow()
        else if (tab === 2) { orderIndex = 0; Turbine.loadOrder() }
    }
    onZoneChanged: note("page", "Focus zone " + zone)
    onSetSectionChanged: {
        note("page", "Settings section " + (settingsSections[setSection] || setSection))
        if (settingsSections[setSection] === "Getting mods") { Turbine.handlerStatus(); if (!shot.on) Turbine.bridgeRefresh() }
    }
    onWidthChanged: sizeNote.restart()
    onHeightChanged: sizeNote.restart()
    Timer { id: sizeNote; interval: 500; onTriggered: app.note("window", "Window " + app.width + " x " + app.height + ", scale " + app.s.toFixed(2)) }
    // The first frame drawn: how long starting took (Labs > Performance check).
    property bool shown: false
    onFrameSwapped: if (!shown) { shown = true; Turbine.markShown() }

    // ---------------------------------------------------------------- fonts
    FontLoader { id: cinzel; source: "file:///app/share/turbine-mod-manager/fonts/Cinzel.ttf" }
    FontLoader { id: barlow; source: "file:///app/share/turbine-mod-manager/fonts/Barlow-Regular.ttf" }
    FontLoader { source: "file:///app/share/turbine-mod-manager/fonts/Barlow-Medium.ttf" }
    FontLoader { source: "file:///app/share/turbine-mod-manager/fonts/Barlow-SemiBold.ttf" }
    FontLoader { source: "file:///app/share/turbine-mod-manager/fonts/Barlow-Bold.ttf" }
    readonly property string titleFont: cinzel.status === FontLoader.Ready ? cinzel.font.family : "serif"
    readonly property string bodyFont: barlow.status === FontLoader.Ready ? barlow.font.family : "sans-serif"

    // ---------------------------------------------------------------- colors
    QtObject {
        id: c
        readonly property bool hc: look.highContrast
        readonly property bool light: look.theme === "light" && !hc
        readonly property bool black: look.theme === "black" || hc
        readonly property color ground: hc ? "#000000" : light ? "#F1EADB" : black ? "#000000" : "#0B0A08"
        readonly property color rail: hc ? "#000000" : light ? "#E6DCC6" : black ? "#000000" : "#080706"
        readonly property color panel: hc ? "#000000" : light ? "#E7DDC8" : black ? "#0C0B0A" : "#141109"
        readonly property color card: hc ? "#000000" : light ? "#F7F2E7" : black ? "#080707" : "#100E0A"
        readonly property color raised: hc ? "#1A1A1A" : light ? "#DDD1B8" : black ? "#161412" : "#211B12"
        readonly property color line: hc ? "#FFFFFF" : light ? "#CDBF9F" : black ? "#26221C" : "#33291B"
        readonly property color text: hc ? "#FFFFFF" : light ? "#2A2216" : "#EADFC6"
        readonly property color soft: hc ? "#FFFFFF" : light ? "#4A3E2A" : "#CDBE9F"
        readonly property color muted: hc ? "#E0E0E0" : light ? "#5E5038" : "#AC9873"
        readonly property var swatches: ({ brass: "#C99E52", iron: "#A9B4BE", copper: "#B07A5A", jade: "#6FAE8C",
                                           fjord: "#6E9CC8", heather: "#A58BC6", ember: "#CF6F54" })
        readonly property color accentBase: look.vision === "none" ? "#BDBDBD" : (swatches[look.accent] || swatches.brass)
        readonly property color accent: hc ? "#FFD54A" : light ? Qt.darker(accentBase, 1.8) : accentBase
        readonly property color accentHi: hc ? "#FFE680" : light ? Qt.darker(accentBase, 1.6) : Qt.lighter(accentBase, 1.22)
        readonly property color accentLo: light ? Qt.darker(accentBase, 2.1) : Qt.darker(accentBase, 1.35)
        readonly property color onAccent: light ? "#FFFFFF" : "#0B0A08"
        // Accent-colored words: at least 4.5:1 on every surface here, in every theme, accent and color vision mode.
        readonly property color accentText: hc ? "#FFD54A" : light ? Qt.darker(accentBase, 2.2) : Qt.lighter(accentBase, 1.12)
        readonly property color focusFill: hc ? "#333300" : light ? Qt.lighter(accentBase, 1.55) : Qt.darker(accentBase, 4.2)
        // Status colors stay apart for each kind of color vision; words always carry the meaning too. Each reads at
        // 4.5:1 or better as text on ground, panel, card, raised and focusFill.
        readonly property color ok: look.vision === "none" ? soft : (look.vision === "red" || look.vision === "green") ? (light ? "#25507E" : "#7FAAD3") : light ? "#3F5824" : "#9DBF7A"
        readonly property color warn: look.vision === "none" ? text : (look.vision === "red" || look.vision === "green") ? (light ? "#704700" : "#E0A040") : light ? "#843C20" : "#E6A98A"
        readonly property color stop: look.vision === "none" ? text : hc ? "#FF8A80" : light ? "#8E2A1E" : "#F0907A"
    }

    // ---------------------------------------------------------------- what the designer's components need
    readonly property var colors: c
    QtObject {
        id: kit
        readonly property var c: app.colors
        // Cinzel is only for the wordmark and the game's name in the header: every other title is Barlow.
        readonly property string titleFont: app.bodyFont
        readonly property string bodyFont: app.bodyFont
        function px(v) { return app.px(v) }
        function hover(item, on) { app.hover(item, on) }
    }

    // ---------------------------------------------------------------- data
    property var games: []
    property int current: 0
    readonly property var game: games.length > 0 ? games[Math.min(current, games.length - 1)] : null
    function load() {
        Turbine.scan()
    }

    // The current game's mods, rules and Downloads, from the engine as JSON.
    property var mods: []
    property var removedMods: []
    property var setups: []
    property bool modsLoaded: false
    property var rules: null
    property var downloads: []
    property var checks: []
    readonly property bool ordered: rules !== null && rules.ordered === true
    // The name for tight places (the header): the game rules' short name when it has one.
    readonly property string gameTitle: game ? ((rules && rules.shortName) || game.name) : ""
    readonly property bool loaderMissing: rules !== null && rules.loader !== null && rules.loader !== undefined && !rules.loaderInstalled
    readonly property string loaderShort: rules && rules.loader ? rules.loader.name.split(" (")[0] : ""
    readonly property var needLoader: loaderMissing ? mods.filter(m => m.enabled && m.needsLoader) : []
    readonly property var tabs: ["My mods (" + mods.length + ")", "Get mods"].concat(rules && rules.plugins ? ["Load order"] : [])
    // My mods: the pinned "+ Get mods" row, the mods, then one collapsed row for Recently removed.
    readonly property var listItems: [{ kind: "get", m: { id: "", name: "" } }].concat(mods.map(m => ({ kind: "mod", m: m })))
        .concat(removedMods.length > 0 ? [{ kind: "removed", m: { id: "", name: "" } }] : [])
    // Mods added since the game was last played get a "New" badge.
    readonly property real lastPlayedAt: { const p = setups.filter(x => x.played === true).map(x => x.savedAt); return p.length > 0 ? Math.max.apply(null, p) : 0 }
    function isNew(m) { return m.added > lastPlayedAt }
    readonly property var focusedItem: listItems.length > 0 ? listItems[Math.min(modIndex, listItems.length - 1)] : null

    function parse(text, fallback) {
        if (!text) return fallback
        try { return JSON.parse(text) } catch (e) { return fallback }
    }
    Connections {
        target: Turbine
        function onGamesChanged() {
            // Last played first.
            const order = app.parse(seen.played, [])
            let list = app.parse(Turbine.games, []).map((g, i) => ({ g: g, i: i }))
            list.sort((a, b) => {
                const x = order.indexOf(a.g.id), y = order.indexOf(b.g.id)
                return (x < 0 ? 1e6 + a.i : x) - (y < 0 ? 1e6 + b.i : y)
            })
            list = list.map(x => x.g)
            app.games = list
            const want = shot.arg("game") || seen.lastGame
            let i = list.findIndex(g => g.id === want)
            app.current = i >= 0 ? i : 0
        }
        function onModsJsonChanged() {
            if (Turbine.modsJson === "") { app.modsLoaded = false; app.mods = []; app.removedMods = []; app.setups = []; return }
            if (app.moving) return
            const v = app.parse(Turbine.modsJson, {})
            app.mods = v.mods || []
            app.removedMods = v.removed || []
            app.setups = v.setups || []
            app.modsLoaded = true
            app.modIndex = Math.max(0, Math.min(app.modIndex, app.listItems.length - 1))
            if (app.removedIndex > app.removedMods.length) app.removedIndex = app.removedMods.length
            if (app.sheet === "removed" && app.removedMods.length === 0) app.sheet = ""
            app.focusArrived()
            app.maybeLandOnGet()
            app.offerLoader()
            app.maybeCheckUpdates()
        }
        function onRulesJsonChanged() {
            app.rules = app.parse(Turbine.rulesJson, null)
            app.offerLoader()
            if (app.rules && app.tab === 1 && !app.browseData) app.browseNow()
        }
        function onDownloadsJsonChanged() {
            app.downloads = app.parse(Turbine.downloadsJson, [])
            app.addIndex = Math.max(0, Math.min(app.addIndex, app.downloads.length - 1))
        }
        function onMessage(text, level) { app.say(text, level) }
        function onChecked(json) { app.checked(app.parse(json, [])) }
        function onLaunch(url) { if (!shot.on) Qt.openUrlExternally(url) }
        function onPages(title, json) { app.openPages(title, app.parse(json, [])) }
        function onHandlerJsonChanged() { app.handler = app.parse(Turbine.handlerJson, { state: "unknown" }) }
        // Catching browser downloads (catch.rs).
        function onStatusJsonChanged() { if (!shot.on) app.status = app.parse(Turbine.statusJson, { active: false }) }
        function onBrowserJsonChanged() {
            if (shot.on) return
            app.browser = app.parse(Turbine.browserJson, { checked: false })
            if (app.sheet === "nobrowser" && app.browserFound) { app.sheet = ""; toast.showPlain("Found " + app.browser.result.name + ". Nexus pages open there.") }
        }
        function onBridgeJsonChanged() {
            if (shot.on) return
            app.bridge = app.parse(Turbine.bridgeJson, { running: false })
            // The helper was paired on another port: the pairing page again, once.
            if (app.bridge.moved && !app.movedPaired) { app.movedPaired = true; app.note("bridge", "Port changed: opening the pairing page again"); app.pair(false) }
        }
        function onAdded(name, notes) { app.openNotes(name, app.parse(notes, [])) }
        function onAsked(json) {
            const f = app.parse(json, null)
            app.found = f && f.path ? f : null
            if (app.found && app.sheet === "") { app.infoIndex = 0; app.sheet = "found" }
            else if (!app.found && (app.sheet === "found" || app.sheet === "pickgame")) app.sheet = ""
        }
        function onPad(button, pressed) { pressed ? app.press(button) : app.release(button) }
        // 0.3
        function onBrowseJsonChanged() {
            const v = app.parse(Turbine.browseJson, null)
            if (!v || !app.game || v.game !== app.game.id) return
            app.loadingMore = false
            if (v.fallback === true) {
                // Browsing without a key failed: Open on Nexus instead, for the rest of this session.
                if (!app.browseData || !v.append) { app.nexusPublic = "failed"; app.browseData = v }
                app.browseLoading = false
                app.getArea = "fallback"; app.getIndex = 0
                return
            }
            if (v.append && app.browseData && app.browseData.items) {
                const more = Object.assign({}, v, { items: app.browseData.items.concat(v.items || []) })
                app.browseData = more
                app.noMore = (v.items || []).length === 0
                return
            }
            app.nexusPublic = v.source === "nexus" ? "ok" : app.nexusPublic
            app.browseData = v
            app.noMore = (v.items || []).length < 30
            app.browseLoading = false
            if (app.restoreFocus) {
                const r = app.restoreFocus
                app.restoreFocus = null
                app.getArea = r.area; app.getIndex = Math.max(0, Math.min(r.index, (v.items || []).length - 1))
            } else if (app.getArea === "grid") app.getIndex = Math.max(0, Math.min(app.getIndex, (v.items || []).length - 1))
            else if (app.getArea === "fallback") { app.getArea = "bar"; app.getIndex = 0 }
        }
        function onDetailsJsonChanged() {
            app.detailsData = app.parse(Turbine.detailsJson, null)
            if (app.detailsData && app.detailsData.name && app.detailsItem) app.rememberViewed(app.detailsItem, app.detailsData)
            if ((shot.page === "versions" || shot.page === "nexusversions") && app.otherVersions.length > 0) app.openVersions()
        }
        function onDownloadJsonChanged() { app.dl = app.parse(Turbine.downloadJson, { active: false }) }
        function onFomodJsonChanged() {
            app.fomod = app.parse(Turbine.fomodJson, null)
            if (app.fomod && app.sheet === "") app.sheet = "fomod"
            else if (!app.fomod && app.sheet === "fomod") app.sheet = ""
        }
        function onOrderJsonChanged() {
            const v = app.parse(Turbine.orderJson, null)
            if (!v || !app.game || v.game !== app.game.id) return
            if (app.orderMoving) return
            app.order = v.plugins || []
            app.orderError = v.error || ""
            app.orderMessages = v.messages || {}
            if (v.note) app.orderNote = v.note
            if (shot.page === "lootsort" && v.sortedBy === "loot") app.orderIndex = app.order.length - 1
            if (shot.page === "lootnotes" && v.sortedBy === "loot") {
                const i = app.order.findIndex(x => (app.orderMessages[x.name.toLowerCase()] || []).length > 1)
                if (i >= 0) { app.orderIndex = i; app.sheet = "plugin" }
            }
            app.orderIndex = Math.max(-1, Math.min(app.orderIndex, app.order.length - 1))
        }
        function onProfilesJsonChanged() {
            const v = app.parse(Turbine.profilesJson, null)
            if (!v || !app.game || v.game !== app.game.id) return
            app.profileData = v.data
        }
        function onSharesJsonChanged() { app.shares = app.parse(Turbine.sharesJson, []) }
        function onImportJsonChanged() {
            const v = app.parse(Turbine.importJson, null)
            app.importData = v && (v.plan || v.error) ? v : null
            if (app.importData && app.sheet === "") { app.profIndex = 0; app.sheet = "profiles" }
            if (app.importData) app.profIndex = 0
        }
        function onUpdatesJsonChanged() { app.updatesData = app.parse(Turbine.updatesJson, null) }
        function onAccountJsonChanged() { app.account = app.parse(Turbine.accountJson, { state: "none" }) }
        function onCodeJsonChanged() {
            app.codeData = app.parse(Turbine.codeJson, null)
            if (app.codeData && app.codeData.state === "ready") Turbine.copyText(app.codeData.code)
        }
        function onArrived(gameId, id, name, on) { app.modArrived(gameId, id, name, on) }
        function onLabsJsonChanged() {
            const v = app.parse(Turbine.labsJson, null)
            if (!v || !v.id) return
            app.labsResult = v
            app.labsChoose(app.labsItems.find(x => x.id === v.id), v.ok ? "works" : "problem", true)
            if (v.ok) toast.done(v.text); else toast.problem(v.text)
        }
        function onSelfJsonChanged() {
            app.selfUpd = app.parse(Turbine.selfJson, { state: "idle" })
            if (app.selfUpd.state === "available" && app.selfAuto) {
                app.selfAuto = false
                toast.info("Turbine " + app.selfUpd.version + " is ready to install: Settings, Updates.")
            }
            if (app.selfUpd.state === "ready" && app.backToStable) { app.backToStable = false; updates.prerelease = false }
        }
    }
    onGameChanged: {
        if (!game) return
        seen.lastGame = game.id
        moving = false
        modIndex = 0
        if (tab >= 2) tab = 0
        modPage = false
        history = []
        landedOn = ""
        browseData = null
        getArea = "bar"; getIndex = 0; getText = ""; getCategory = ""
        order = []; orderIndex = 0; orderMessages = {}; orderNote = ""
        profileData = null
        Turbine.select(game.id)
        if (tab === 1) Qt.callLater(browseNow)
    }
    Component.onCompleted: {
        // Recorded samples instead of the internet (the build's screenshot checks); first, before anything asks.
        if (shot.arg("selftest-data") !== "") Turbine.setSampleData(shot.arg("selftest-data"))
        else if (!shot.on) {
            Turbine.accountStatus()
            Turbine.startCatching()
            if (updates.auto) { selfAuto = true; Turbine.selfCheck(labsOn && updates.prerelease, false) }
        }
        load()
        note("app", "Qt platform " + Qt.platform.pluginName + (Turbine.gamemode ? ", Game Mode" : ""))
        if (shot.page === "settings") { page = "settings"; zone = "settings" }
        else if (shot.page === "details") sheet = "details"
        else if (shot.page === "perf") { page = "perf"; zone = "perf" }
        else if (shot.page === "notice") sheet = "notice"
        else if (!shot.on && !Turbine.steamos && !seen.notSteamOS) sheet = "notice"
        if (look.size === 0 && look.textSize !== 100) look.size = Math.max(80, Math.min(150, Math.round(look.textSize / 10) * 10))
        checkDisplays()
        if (shot.on) shot.start()
    }

    // ---------------------------------------------------------------- status line
    // Messages from the engine show until they're done ("work"), for a while ("ok", errors), then the line goes back
    // to saying how the game stands.
    property string msgText: ""
    property string msgLevel: ""
    Timer { id: msgTimer; interval: 4000 }
    function say(text, level) {
        msgText = text
        msgLevel = level
        msgTimer.interval = level === "error" ? 10000 : 4000
        if (level !== "work") msgTimer.restart()
        else msgTimer.stop()
    }
    readonly property bool msgShown: msgText !== "" && (msgLevel === "work" || msgTimer.running)
    function plural(n, one, many) { return n + " " + (n === 1 ? one : many) }
    readonly property var statusNow: {
        if (page === "settings") return { level: "ok", words: "Changes show right away", detail: "" }
        if (page === "perf") return { level: "ok", words: "2,000 rows", detail: "Hold the D-pad to scroll fast, or turn the left stick in a circle." }
        if (!game) return { level: Turbine.problems !== "" ? "warn" : "ok", words: "No games found yet", detail: "" }
        if (dl.active) return { level: "busy", words: "Downloading " + dl.label, detail: "", progress: dlPart >= 0 ? dlPart : 0,
                                 progressText: dl.total > 0 ? Math.round(dlPart * 100) + "% · " + sizeWords(dl.got) + " of " + sizeWords(dl.total) : sizeWords(dl.got) }
        if (openState === "opening") return { level: "busy", words: "Opening Nexus in " + openName + "...", detail: "" }
        if (openState === "open") return { level: "ok", words: "Nexus is open in " + openName + ". Turbine adds your download by itself.", detail: "" }
        if (msgShown) return { level: ({ work: "busy", ok: "ok", warn: "warn", error: "stop" })[msgLevel] || "ok", words: msgText, detail: "" }
        if (Turbine.pending > 0)
            return { level: "busy", words: plural(Turbine.pending, "change", "changes") + " waiting", detail: "They go into the game in a few seconds" }
        if (needLoader.length > 0)
            return { level: "warn", words: plural(needLoader.length, "mod needs ", "mods need ") + loaderShort, detail: "The game still plays, without " + (needLoader.length === 1 ? "that one" : "those") }
        const on = mods.filter(m => m.enabled).length
        return { level: "ok", words: "Ready to play", detail: !modsLoaded ? "" : mods.length === 0 ? "No mods yet" : plural(on, "mod", "mods") + " on",
                 pill: updateCount > 0 ? plural(updateCount, "update", "updates") + " ready" : "" }
    }

    // ---------------------------------------------------------------- applying changes
    // Changes go into the game 3 seconds after the last edit, when Turbine loses focus and when it closes.
    Timer { id: autoApply; interval: 3000; onTriggered: if (!shot.on) Turbine.applyNow() }
    function edited() { autoApply.restart() }
    onActiveChanged: {
        note("window", active ? "Turbine in front" : "Turbine in the background")
        if (active) {
            // Back from a game: presses made there (still queued) must not act here.
            repeater.stop()
            repeatButton = ""
            Turbine.wake()
        } else if (!shot.on && Turbine.pending > 0) {
            autoApply.stop()
            Turbine.applyNow()
        }
    }
    onClosing: Turbine.finish()

    // ---------------------------------------------------------------- mod actions
    function toggle(m) {
        const on = !m.enabled
        note("action", "Turn " + (on ? "on " : "off ") + m.name)
        mods = mods.map(x => x.id === m.id ? Object.assign({}, x, { enabled: on }) : x)
        Turbine.setEnabled(m.id, on)
        edited()
        if (on && m.needsLoader && loaderMissing) { loaderFor = [m.name]; openLoader() }
    }
    function moveMod(m, to) {
        const from = mods.findIndex(x => x.id === m.id)
        if (from < 0 || to === from) return
        note("action", "Move " + m.name + " from " + (from + 1) + " to " + (to + 1))
        let list = mods.slice()
        list.splice(from, 1)
        list.splice(to, 0, m)
        mods = list
        modIndex = to + 1
        Turbine.moveTo(m.id, to)
        edited()
    }
    function uninstall(m) {
        note("action", "Uninstall " + m.name)
        mods = mods.filter(x => x.id !== m.id)
        Turbine.removeMod(m.id)
        edited()
        modIndex = Math.max(0, Math.min(modIndex, listItems.length - 1))
        toast.showUndo(m.id, m.name)
    }
    // ---------------------------------------------------------------- Recently removed (one collapsed row; a sheet)
    property int removedIndex: 0
    function openRemoved() { removedIndex = removedMods.length > 0 ? 1 : 0; sheet = "removed" }
    function removedRun(i) {
        if (i === 0) askDelete("all", "Delete all " + plural(removedMods.length, "removed mod", "removed mods") + " for good? They can't be put back.")
        else if (removedMods[i - 1]) putBack(removedMods[i - 1].id)
    }
    function removedDelete(i) {
        const m = removedMods[i - 1]
        if (m) askDelete(m.id, "Delete " + m.name + " for good? It can't be put back.")
        else if (i === 0) removedRun(0)
    }
    // Hold A to confirm (data that can't come back): the same hold the game menu's Reset uses.
    property string confirmWhat: ""
    property string confirmText: ""
    readonly property int holdNeeded: 5
    function askDelete(what, text) { confirmWhat = what; confirmText = text; infoIndex = 0; stopReset(); sheet = "confirm" }
    function confirmDone() {
        const w = confirmWhat
        sheet = removedMods.length > (w === "all" ? removedMods.length : 1) ? "removed" : ""
        if (w === "all") { Turbine.purgeAllRemoved(); toast.done("Removed mods deleted for good.") }
        else { const m = removedMods.find(x => x.id === w); Turbine.purgeRemoved(w); toast.done((m ? m.name : "The mod") + " deleted for good.") }
        removedIndex = Math.min(removedIndex, Math.max(0, removedMods.length - 1))
    }
    function putBack(id) {
        note("action", "Put back " + id)
        Turbine.undoRemove(id)
        toast.hide()
        edited()
        if (sheet === "removed" && removedMods.length <= 1) sheet = ""
    }
    // Reorder mode (Y): the D-pad moves the mod, A, B or Y puts it down.
    property bool moving: false
    property int moveFrom: -1
    // In move mode modIndex still counts the pinned row: the mod at modIndex is mods[modIndex - 1].
    property var moveSaved: []
    function startMove() {
        if (!ordered || !focusedItem || focusedItem.kind !== "mod") return
        moving = true
        moveFrom = modIndex
        moveSaved = mods.slice()
    }
    // d rows (Up/Down 1, Left/Right 10); LT/RT pass a large number for top or bottom.
    function stepMove(d) {
        const from = modIndex - 1
        const to = Math.max(0, Math.min(mods.length - 1, from + d))
        if (to === from) return
        let list = mods.slice()
        const m = list.splice(from, 1)[0]
        list.splice(to, 0, m)
        mods = list
        modIndex = to + 1
    }
    function endMove() {
        moving = false
        const m = mods[modIndex - 1]
        if (modIndex !== moveFrom && m) {
            note("action", "Reorder " + m.name + " from " + moveFrom + " to " + modIndex)
            Turbine.moveTo(m.id, modIndex - 1)
            edited()
        }
    }
    // B: the mod goes back where it was.
    function cancelMove() {
        moving = false
        mods = moveSaved
        modIndex = moveFrom
        note("action", "Move cancelled")
    }
    // The loader prompt shows once per game while a mod that needs the missing loader is on.
    property string loaderOffered: ""
    property var loaderFor: []
    property bool loaderSourceShown: false
    property int loaderIndex: 0
    function openLoader() { loaderSourceShown = false; loaderIndex = 0; sheet = "loader" }
    function offerLoader() {
        if (!game || !modsLoaded || loaderOffered === game.id || needLoader.length === 0) return
        if (shot.on && shot.page !== "loader") return
        if (sheet !== "" || page !== "game") return
        loaderOffered = game.id
        loaderFor = needLoader.map(m => m.name)
        openLoader()
    }

    // ---------------------------------------------------------------- Play
    function startPlay() {
        if (!game) return
        note("action", "Play " + game.name)
        if (!shot.on) { let p = parse(seen.played, []).filter(x => x !== game.id); p.unshift(game.id); seen.played = JSON.stringify(p.slice(0, 50)) }
        if (moving) endMove()
        // Play applies the waiting changes itself; its job runs on the worker after any apply already running, so
        // the timer must not start another one meanwhile.
        autoApply.stop()
        say("Checking before Play...", "work")
        Turbine.check()
    }
    readonly property string launchTitle: "One-time Steam setting for the mod loader"
    function launchSeen() { return game && parse(seen.launchDone, []).indexOf(game.id) >= 0 }
    function onceSeen(title) { return game && parse(seen.onceDone, []).indexOf(game.id + "|" + title) >= 0 }
    function checked(list) {
        if (launchSeen()) list = list.filter(x => x.title !== launchTitle)
        list = list.filter(x => !(x.once && x.title !== launchTitle && onceSeen(x.title)))
        if (list.length === 0) list = [{ level: "ok", title: "Ready to play", detail: "Everything checks out." }]
        checks = list
        note("engine", "Check before Play: " + list.map(x => x.level + " " + x.title).join("; "))
        const trouble = list.some(x => x.level === "stop" || x.level === "warn")
        if (msgLevel === "work" && msgText === "Checking before Play...") msgText = ""
        if (trouble || shot.page === "check" || shot.page === "checkend") { checkIndex = 0; sheet = "check" }
        else Turbine.play(game.id)
    }
    readonly property bool checkStops: checks.some(x => x.level === "stop")
    readonly property var checkChoices: checkStops ? ["Play without mods", "Not now"] : ["Play anyway", "Play without mods", "Not now"]
    property int checkIndex: 0
    function checkChoose(i) {
        const what = checkChoices[i]
        note("action", "Check before Play: " + what)
        sheet = ""
        if (checks.some(x => x.title === launchTitle) && what !== "Not now") {
            let done = parse(seen.launchDone, []); done.push(game.id); seen.launchDone = JSON.stringify(done)
        }
        // Other read-once lines (REDmod): seen once the player moves on from the check.
        const once = checks.filter(x => x.once && x.title !== launchTitle)
        if (once.length > 0 && what !== "Not now" && !shot.on) {
            let done = parse(seen.onceDone, []); for (const x of once) done.push(game.id + "|" + x.title); seen.onceDone = JSON.stringify(done)
        }
        if (what === "Play anyway") Turbine.play(game.id)
        else if (what === "Play without mods") Turbine.playOriginal()
    }

    // ---------------------------------------------------------------- game menu
    // The setup to go back to: the newest one the game was played with (the engine's restore target).
    readonly property var lastGood: {
        const played = setups.filter(x => x.played === true).sort((a, b) => b.savedAt - a.savedAt)
        return played.length > 0 ? played[0] : null
    }
    function when(sec) { return Qt.formatDateTime(new Date(sec * 1000), "MMM d, h:mm AP") }
    // Reset shows only when the engine found records it couldn't read for this game. It runs after A is held 5 s.
    readonly property bool canReset: game !== null && Turbine.resetFor === game.id
    property real resetHeld: 0
    Timer {
        id: resetHold
        interval: 100
        repeat: true
        onTriggered: {
            app.resetHeld += 0.1
            if (app.resetHeld >= 5) { stop(); app.resetHeld = 0; app.sheet = ""; Turbine.resetGame() }
        }
    }
    function stopReset() { resetHold.stop(); resetHeld = 0 }
    readonly property var gameMenuItems: [
        { icon: "play", title: "Play without mods", detail: "Starts the game with no mods. Nothing is deleted.", ok: true },
        { icon: "back", title: "Restore last working setup",
          detail: lastGood ? "From " + when(lastGood.savedAt) + " · " + plural(lastGood.modsOn, "mod", "mods") + " on" : "Nothing saved yet. A setup is saved each time you play with mods.",
          ok: lastGood !== null }
    ].concat(canReset ? [
        { icon: "restore", title: "Reset game files", act: "reset", danger: true, ok: true,
          detail: resetHold.running ? "Keep holding A: " + Math.max(1, Math.ceil(5 - resetHeld)) + "..."
                  : "Hold A for 5 seconds. Takes every mod file out and puts the game's own files back. Mods stay listed, turned off." }
    ] : []).concat([
        { icon: "profile", title: "Profiles", detail: profileData && profileList.length > 1 ? "In use: " + profileData.active : "Save setups by name, switch, share", ok: true, act: "profiles" }
    ]).concat(updateCount > 0 ? [
        { icon: "update", title: "Update all", detail: plural(updateCount, "mod has", "mods have") + " a newer version", ok: true, act: "updateall" }
    ] : []).concat([
        { icon: "info", title: "Game details", detail: "What Turbine found for this game", ok: true, act: "details" }
    ])
    function gameMenuRun(i) {
        const item = gameMenuItems[i]
        if (!item || !item.ok) return
        note("action", "Game menu: " + item.title)
        if (i === 0) { sheet = ""; Turbine.playOriginal() }
        else if (i === 1) { sheet = ""; Turbine.restoreSetup("") }
        else if (item.act === "reset") { if (resetHold.running) stopReset(); else { resetHeld = 0; resetHold.start() } }
        else if (item.act === "profiles") openProfiles()
        else if (item.act === "updateall") { sheet = ""; Turbine.updateAll() }
        else sheet = "details"
    }

    // ---------------------------------------------------------------- mod menu
    function ordinal(n) {
        const s = ["th", "st", "nd", "rd"], v = n % 100
        return n + (s[(v - 20) % 10] || s[v] || s[0])
    }
    readonly property var menuMod: focusedItem && focusedItem.kind === "mod" ? focusedItem.m : null
    // A new mod arrived: switch to its game, My mods, focus it, and say so once.
    property var arrivedMod: null
    function modArrived(gameId, id, name, on) {
        note("action", "New mod " + name + " in " + gameId)
        const i = games.findIndex(g => g.id === gameId)
        if (i >= 0 && i !== current) current = i
        if (page !== "game") { page = "game" }
        if (sheet === "add" || sheet === "found") sheet = ""
        zone = "main"; tab = 0; modPage = false
        arrivedMod = { id: id, name: name }
        focusArrived()
        if (!shot.on || shot.page === "newmod") toast.done("New: " + name + (on ? " (on)" : " (off)") + ". Start: Play")
    }
    function focusArrived() {
        if (!arrivedMod) return
        const i = listItems.findIndex(x => x.kind === "mod" && x.m.id === arrivedMod.id)
        if (i >= 0) { modIndex = i; arrivedMod = null }
    }
    readonly property var modMenuItems: !menuMod ? [] : [
        { icon: "power", title: menuMod.enabled ? "Turn off" : "Turn on", detail: menuMod.enabled ? "Stays installed" : "Goes into the game", act: "toggle" }
    ].concat(updateMap[menuMod.id] ? [
        { icon: "update", title: "Update to " + updateMap[menuMod.id].to, detail: (updateMap[menuMod.id].from ? "You have " + updateMap[menuMod.id].from + ". " : "") + "Keeps its place and on or off", act: "update" }
    ] : []).concat(ordered ? [
        { icon: "move", title: "Move to top", detail: "First in the load order. Mods below it win when files overlap.", act: "top" },
        { icon: "move", title: "Move to bottom", detail: "Last in the load order. It wins when files overlap.", act: "bottom" }
    ] : []).concat([
        { icon: "remove", title: "Uninstall", detail: "Kept in Removed mods for 7 days", act: "remove", danger: true }
    ])
    property int menuIndex: 0
    function modMenuRun(i) {
        const item = modMenuItems[i], m = menuMod
        if (!item || !m) return
        note("action", "Mod menu: " + item.title + " for " + m.name)
        sheet = ""
        if (item.act === "toggle") toggle(m)
        else if (item.act === "top") { moveMod(m, 0); modIndex = 1 }
        else if (item.act === "bottom") { moveMod(m, mods.length - 1); modIndex = mods.length }
        else if (item.act === "remove") uninstall(m)
        else if (item.act === "update") { Turbine.updateMod(m.id); edited() }
    }
    function modWords(m) {
        let parts = []
        if (m.version) parts.push(m.version)
        if (m.source) parts.push(m.source)
        return parts.join(" · ")
    }

    // ---------------------------------------------------------------- Add mods from Downloads
    property int addIndex: 0
    function openAdd() { Turbine.listDownloads(); addIndex = 0; sheet = "add" }
    function addChosen() {
        const d = downloads[addIndex]
        sheet = ""
        if (!d) return
        note("action", "Add from Downloads: " + d.name)
        tab = 0
        Turbine.importZip(d.path)
        edited()
    }
    function age(sec) {
        const d = Math.floor((Date.now() / 1000 - sec) / 86400)
        return d <= 0 ? "Today" : d === 1 ? "Yesterday" : d + " days ago"
    }
    function sizeWords(b) {
        if (b >= 1048576) return (b / 1048576).toFixed(1) + " MB"
        if (b >= 1024) return Math.round(b / 1024) + " KB"
        return b + " bytes"
    }

    // ---------------------------------------------------------------- controller and keyboard
    // zone: where the controller focus is. "rail" (games and buttons on the left), "main" (the game page),
    // "settings", "perf". sheet: an overlay on top (details, notice, modmenu, gamemenu, add, check, loader).
    property string page: "game"          // game, settings, perf
    property string zone: "main"
    property string sheet: ""
    property int railIndex: 0             // 0..games.length-1, then "add", then "settings"
    property int tab: 0                   // 0 Mods, 1 Get Mods, 2 Load order
    property int modIndex: 0
    property int setSection: 0            // 0 Look, 1 About
    property int setRow: 0
    property int perfIndex: 0
    readonly property int railCount: games.length + 1   // the games, then Settings
    onSheetChanged: {
        note("page", sheet === "" ? "Sheet closed" : "Sheet " + sheet); menuIndex = 0; stopReset(); stopConfirm()
        pickHover()
        if (sheet === "" && fomod) sheet = "fomod"
        else if (sheet === "" && found) { infoIndex = 0; sheet = "found" }
        else if (sheet === "" && pendingNotes) { pendingNotes = false; infoIndex = 0; sheet = "notes" }
        else if (sheet === "") offerLoader()
    }

    Timer { interval: 16; repeat: true; running: app.active || shot.on; onTriggered: Turbine.poll() }

    // Holding the D-pad repeats; after 1.5 s each repeat moves 5 rows.
    property string repeatButton: ""
    property real repeatSince: 0
    Timer {
        id: repeater
        interval: 380
        repeat: true
        onTriggered: {
            interval = 85
            const n = Date.now() - app.repeatSince > 1500 && (app.repeatButton === "up" || app.repeatButton === "down") ? 5 : 1
            for (let i = 0; i < n; i++) app.act(app.repeatButton)
        }
    }
    // Hold B for 1 second: every sheet and page closes, back to the game's list.
    Timer {
        id: homeHold
        interval: 1000
        onTriggered: app.goHome()
    }
    function goHome() {
        note("action", "Hold B: home")
        if (sheet === "ask") askView.cancel ? askView.cancel() : null
        sheet = ""; moving = false; orderMoving = false; modPage = false; history = []
        page = "game"; zone = "main"; tab = 0; modIndex = 0
    }
    function press(b) {
        if (["up", "down", "left", "right"].indexOf(b) >= 0) {
            repeatButton = b
            repeatSince = Date.now()
            repeater.interval = 380
            repeater.restart()
        }
        if (b === "view") holdStart("View")
        if (b === "b") homeHold.restart()
        if (b === "a" && sheet === "confirm") { startConfirm(); return }
        act(b)
    }
    function release(b) {
        if (b === repeatButton) { repeater.stop(); repeatButton = "" }
        if (b === "a" && resetHold.running) stopReset()
        if (b === "a" && confirmHold.running) stopConfirm()
        if (b === "b") homeHold.stop()
        if (b === "view") holdStop()
    }
    property real confirmHeld: 0
    Timer {
        id: confirmHold
        interval: 100
        repeat: true
        onTriggered: {
            app.confirmHeld += 0.1
            if (app.confirmHeld >= app.holdNeeded) { stop(); app.confirmHeld = 0; app.confirmDone() }
        }
    }
    function startConfirm() { if (!confirmHold.running) { confirmHeld = 0; confirmHold.start() } }
    function stopConfirm() { confirmHold.stop(); confirmHeld = 0 }

    Item {
        id: keyRoot
        anchors.fill: parent
        focus: true
        Keys.onPressed: (e) => {
            const map = {}
            map[Qt.Key_Up] = "up"; map[Qt.Key_Down] = "down"; map[Qt.Key_Left] = "left"; map[Qt.Key_Right] = "right"
            map[Qt.Key_Return] = "a"; map[Qt.Key_Enter] = "a"; map[Qt.Key_Space] = "a"
            map[Qt.Key_Escape] = "b"; map[Qt.Key_Backspace] = "b"
            map[Qt.Key_X] = "x"; map[Qt.Key_Y] = "y"
            map[Qt.Key_PageUp] = "lb"; map[Qt.Key_PageDown] = "rb"
            map[Qt.Key_Q] = "lt"; map[Qt.Key_E] = "rt"
            map[Qt.Key_P] = "start"; map[Qt.Key_Tab] = "view"
            const b = map[e.key]
            if (!b) return
            e.accepted = true
            if (!e.isAutoRepeat) app.note("key", "Key " + b)
            if (b === "view" && !e.isAutoRepeat) app.holdStart("Tab")
            // Holding a key repeats it, except A (Reset waits for A to be held down) and View (held for Labs reports).
            if (!((b === "a" || b === "view") && e.isAutoRepeat)) app.act(b)
        }
        Keys.onReleased: (e) => {
            if (!e.isAutoRepeat && (e.key === Qt.Key_Return || e.key === Qt.Key_Enter || e.key === Qt.Key_Space)) app.release("a")
            if (!e.isAutoRepeat && e.key === Qt.Key_Tab) app.holdStop()
        }
    }

    function sheetAct(b) {
        if (sheet === "report") reportView.handleButton(b)
        else if (sheet === "perf") perfView.handleButton(b)
        else if (sheet === "details" || sheet === "notice") {
            if (b === "a" || b === "b" || b === "view") {
                if (sheet === "notice") seen.notSteamOS = true
                sheet = ""
            }
        } else if (sheet === "modmenu") {
            if (b === "up") menuIndex = Math.max(0, menuIndex - 1)
            else if (b === "down") menuIndex = Math.min(modMenuItems.length - 1, menuIndex + 1)
            else if (b === "a") modMenuRun(menuIndex)
            else if (b === "b" || b === "x") sheet = ""
        } else if (sheet === "gamemenu") {
            if (b === "up") { stopReset(); menuIndex = Math.max(0, menuIndex - 1) }
            else if (b === "down") { stopReset(); menuIndex = Math.min(gameMenuItems.length - 1, menuIndex + 1) }
            else if (b === "a") gameMenuRun(menuIndex)
            else if (b === "b" || b === "view" || b === "x") sheet = ""
        } else if (sheet === "add") {
            if (b === "up") addIndex = Math.max(0, addIndex - 1)
            else if (b === "down") addIndex = Math.min(downloads.length - 1, addIndex + 1)
            else if (b === "a") addChosen()
            else if (b === "b") sheet = ""
        } else if (sheet === "check") {
            // Direct keys: A Play anyway, X Play without mods, B Not now; Start plays when nothing stops it.
            if (b === "left") checkIndex = Math.max(0, checkIndex - 1)
            else if (b === "right") checkIndex = Math.min(checkChoices.length - 1, checkIndex + 1)
            else if (b === "a" && !checkStops) checkChoose(checkChoices.indexOf("Play anyway"))
            else if (b === "a") checkChoose(checkChoices.indexOf("Play without mods"))
            else if (b === "start" && !checkStops) checkChoose(checkChoices.indexOf("Play anyway"))
            else if (b === "x") checkChoose(checkChoices.indexOf("Play without mods"))
            else if (b === "up" || b === "down") checkView.scroll(b === "up" ? -1 : 1)
            else if (b === "b") checkChoose(checkChoices.indexOf("Not now"))
        } else if (sheet === "loader") {
            // Direct keys: A Get it, B Not now.
            if (b === "a" && loaderCanGet) { sheet = ""; note("action", "Install the loader"); Turbine.installLoader() }
            else if (b === "a" && !loaderSourceShown) loaderSourceShown = true
            else if (b === "a" || b === "b") sheet = ""
        } else if (sheet === "pages") {
            if (b === "up") pagesIndex = Math.max(0, pagesIndex - 1)
            else if (b === "down") pagesIndex = Math.min(pagesList.length - 1, pagesIndex + 1)
            else if (b === "a") openPage(pagesIndex)
            else if (b === "b") sheet = ""
        } else if (sheet === "versions") {
            if (b === "up") versionIndex = Math.max(0, versionIndex - 1)
            else if (b === "down") versionIndex = Math.min(otherVersions.length - 1, versionIndex + 1)
            else if (b === "a") installVersion(versionIndex)
            else if (b === "b" || b === "x") { sheet = ""; if (history.length > 0 && history[history.length - 1].kind === "files") history = history.slice(0, -1) }
        } else if (sheet === "code") {
            if (b === "a" && codeData && codeData.state === "ready") { Turbine.copyText(codeData.code); toast.done("Code copied.") }
            else if (b === "b") sheet = "profiles"
        } else if (sheet === "plugin") {
            if (b === "b" || b === "x" || b === "a") sheet = ""
        } else if (sheet === "pickgame") {
            if (b === "up") pickIndex = Math.max(0, pickIndex - 1)
            else if (b === "down") pickIndex = Math.min(games.length - 1, pickIndex + 1)
            else if (b === "a") pickGame(pickIndex)
            else if (b === "b") { infoIndex = 0; sheet = "found" }
        } else if (sheet === "removed") {
            if (b === "up") removedIndex = Math.max(0, removedIndex - 1)
            else if (b === "down") removedIndex = Math.min(removedMods.length, removedIndex + 1)
            else if (b === "a") removedRun(removedIndex)
            else if (b === "x") removedDelete(removedIndex)
            else if (b === "b") sheet = ""
        } else if (sheet === "confirm") {
            if (b === "b") { stopConfirm(); sheet = removedMods.length > 0 ? "removed" : "" }
        } else if (sheet === "keepsize") {
            if (b === "a") keepSize(true)
            else if (b === "b") keepSize(false)
        } else if (sheet === "openhelp") {
            if (b === "a") { sheet = ""; steamOpen() }
            else if (b === "b") { sheet = ""; note("action", "Didn't see it open: Close") }
        } else if (sheet === "cardmenu") {
            if (b === "up") menuIndex = Math.max(0, menuIndex - 1)
            else if (b === "down") menuIndex = Math.min(cardMenuItems.length - 1, menuIndex + 1)
            else if (b === "a") cardMenuRun(menuIndex)
            else if (b === "b" || b === "x") sheet = ""
        } else if (["nobrowser", "helper", "notes", "found"].indexOf(sheet) >= 0) infoAct(b)
        else if (sheet === "fomod") fomodView.handleButton(b)
        else if (sheet === "ask") {
            // X shows Steam's keyboard again (closed by mistake); Y clears the field.
            if (b === "x" && Turbine.gamemode && !shot.on) Qt.openUrlExternally("steam://open/keyboard")
            else if (b === "y" && askView.clear) askView.clear()
            else askView.handleButton(b)
        }
        else if (sheet === "profiles") {
            if (b === "up") profIndex = Math.max(0, profIndex - 1)
            else if (b === "down") profIndex = Math.min(profilesView.rows - 1, profIndex + 1)
            else if (b === "a") profileRun(profIndex)
            else if (b === "b" || b === "view") { if (importData) { importData = null; profIndex = 0 } else sheet = "" }
        } else if (sheet === "shares") {
            if (b === "up") shareIndex = Math.max(0, shareIndex - 1)
            else if (b === "down") shareIndex = Math.min(shares.length - 1, shareIndex + 1)
            else if (b === "a" && shares[shareIndex]) { Turbine.profileImportFile(shares[shareIndex].path); sheet = "profiles" }
            else if (b === "b") sheet = "profiles"
        }
    }
    // Turbine can fetch the loader itself (SKSE, BepInEx, SMAPI).
    readonly property bool loaderCanGet: !!rules && !!rules.loader && ["bethesda", "bepinex", "smapi"].indexOf(rules.kind) >= 0

    function act(b) {
        if (sheet !== "") { sheetAct(b); return }
        if (moving) {
            if (b === "up") stepMove(-1)
            else if (b === "down") stepMove(1)
            else if (b === "left") stepMove(-10)
            else if (b === "right") stepMove(10)
            else if (b === "lt") stepMove(-100000)
            else if (b === "rt") stepMove(100000)
            else if (b === "a" || b === "y") endMove()
            else if (b === "b") cancelMove()
            return
        }
        if (orderMoving) { orderAct(b); return }
        if (b === "y" && undoShown) { putBack(undoId); return }
        if (b === "start") { startPlay(); return }
        if (zone === "rail") {
            if (b === "up") railIndex = Math.max(0, railIndex - 1)
            else if (b === "down") railIndex = Math.min(railCount - 1, railIndex + 1)
            else if (b === "right" || b === "b") zone = page === "settings" ? "settings" : page === "perf" ? "perf" : "main"
            else if (b === "a") railSelect(railIndex)
            else if (b === "lb") switchGame(-1)
            else if (b === "rb") switchGame(1)
            // X or View: the game menu for the focused game.
            else if ((b === "x" || b === "view") && railIndex < games.length) { current = railIndex; page = "game"; menuIndex = 0; sheet = "gamemenu" }
            return
        }
        if (page === "game" && modPage) { modPageAct(b); return }
        if (b === "lb") { switchGame(-1); return }
        if (b === "rb") { switchGame(1); return }
        if (page === "game" && tab === 1 && game && sources.length > 0 && ["up", "down", "left", "right", "a", "b", "x", "y", "r3"].indexOf(b) >= 0) { getAct(b); return }
        if (page === "game" && tab === 2 && game && ["up", "down", "left", "a", "x", "y"].indexOf(b) >= 0) { orderAct(b); return }
        if (page === "game") {
            if (b === "left") { zone = "rail"; railIndex = current }
            else if (b === "lt") tab = Math.max(0, tab - 1)
            else if (b === "rt") tab = Math.min(tabs.length - 1, tab + 1)
            else if (b === "view" && game) { menuIndex = 0; sheet = "gamemenu" }
            else if (b === "a" && games.length === 0) load()
            else if (b === "b" && tab === 2) tab = 0
            else if (b === "b" && tab === 1) tab = 0
            else if (b === "b") { zone = "rail"; railIndex = current }
            else if (tab === 0 && game) {
                const it = focusedItem
                if (b === "up") modIndex = Math.max(0, modIndex - 1)
                else if (b === "down") modIndex = Math.min(listItems.length - 1, modIndex + 1)
                else if (b === "a" && it && it.kind === "get") tab = 1
                else if (b === "a" && it && it.kind === "removed") openRemoved()
                else if (b === "a" && it && it.kind === "mod") toggle(it.m)
                else if (b === "x" && menuMod) sheet = "modmenu"
                else if (b === "x" && it && it.kind === "removed") openRemoved()
                else if (b === "y") startMove()
            } else if (tab === 1 && game) {
                if (b === "a") openAdd()
            }
            return
        }
        if (page === "settings") {
            const rows = sec === "Look" ? lookRows.length : sec === "About" ? 2 : rowsNow.length
            const r = sec === "Look" || sec === "About" ? null : rowsNow[setRow]
            const kind = r ? r.kind : ""
            if (b === "b") { page = "game"; zone = "main" }
            else if (b === "lt") { if (setSection > 0) { setSection--; setRow = 0 } }
            else if (b === "rt") { if (setSection < settingsSections.length - 1) { setSection++; setRow = 0 } }
            else if (b === "up") setRow = Math.max(0, setRow - 1)
            else if (b === "down") setRow = Math.min(rows - 1, setRow + 1)
            else if (sec === "Look" && (b === "left" || b === "right")) lookRows[setRow].step(b === "left" ? -1 : 1)
            else if (sec === "Look" && b === "a") lookRows[setRow].step(1)
            else if (sec === "About" && b === "a") { if (setRow === 0) labsTap(); else { page = "perf"; zone = "perf"; perfIndex = 0 } }
            else if (kind === "item" && b === "left") labsChip = Math.max(0, labsChip - 1)
            else if (kind === "item" && b === "right") labsChip = Math.min(2, labsChip + 1)
            else if (kind === "item" && b === "a") labsChoose(r, ["works", "problem", ""][labsChip])
            else if ((kind === "choice" || kind === "toggle") && (b === "left" || b === "right" || b === "a")) rowStep(r, b === "left" ? -1 : 1)
            else if (kind === "action" && b === "a") rowRun(r)
            else if (b === "left") zone = "rail"
            return
        }
        if (page === "perf") {
            if (b === "b") { page = "settings"; zone = "settings" }
            else if (b === "up") perfIndex = Math.max(0, perfIndex - 1)
            else if (b === "down") perfIndex = Math.min(1999, perfIndex + 1)
            else if (b === "left") zone = "rail"
        }
    }
    function switchGame(step) {
        if (games.length === 0) return
        current = (current + step + games.length) % games.length
        note("action", "Switch game to " + games[current].name)
        if (page !== "game") page = "game"
        modPage = false; history = []
        railIndex = current
    }
    function railSelect(i) {
        note("action", "Rail " + (i < games.length ? games[i].name : "Settings"))
        if (i < games.length) { current = i; page = "game"; zone = "main" }
        else openSettings()
    }

    // ---------------------------------------------------------------- Get mods
    // Where this game's mods come from: Thunderstore for its games, Nexus Mods for the rest (no key needed).
    readonly property var sources: !rules ? [] : [].concat(rules.community ? ["thunderstore"] : []).concat(rules.domain ? ["nexus"] : [])
    property string getSource: ""
    readonly property string srcNow: sources.indexOf(getSource) >= 0 ? getSource : (sources[0] || "")
    function srcName(s) { return s === "nexus" ? "Nexus Mods" : s === "thunderstore" ? "Thunderstore" : "" }
    property string getText: ""
    property string getCategory: ""
    property int getSortIndex: 0
    readonly property var sortKeys: srcNow === "nexus" ? ["trending", "endorsements", "downloads", "updated", "newest"] : ["downloads", "rating", "updated", "newest"]
    readonly property var sortNames: srcNow === "nexus" ? ["Trending", "Most endorsed", "Most downloaded", "Recently updated", "Newest"]
                                                        : ["Most downloaded", "Top rated", "Recently updated", "Newest"]
    property var browseData: null
    property bool browseLoading: false
    property bool loadingMore: false
    property bool noMore: false
    property string getArea: "bar"        // bar, recent, grid, fallback
    property int getIndex: 0
    // Nexus browsing without a key: "" until tried this session, "ok", or "failed" (Open on Nexus instead).
    property string nexusPublic: ""
    readonly property bool nexusFallback: srcNow === "nexus" && (nexusPublic === "failed" || (browseData && browseData.fallback === true))
    readonly property var getCats: browseData && browseData.categories && browseData.categories.length > 0 ? [""].concat(browseData.categories) : []
    readonly property var getItems: browseData && browseData.items ? browseData.items : []
    readonly property var getBar: [{ key: "search", label: getText !== "" ? getText : "Search " + gameTitle + " mods" },
                                   { key: "sort", label: "Sort: " + (sortNames[getSortIndex] || "") }]
        .concat(getCats.length > 0 ? [{ key: "category", label: "Category: " + (getCategory || "All") }] : [])
        .concat(sources.length > 1 ? [{ key: "source", label: "From: " + srcName(srcNow) }] : [])
        .concat([{ key: "add", label: "Add a file from Downloads" }])
    // Recently viewed: the last 8 mod pages opened in this game.
    property var recentSeed: null
    readonly property var recentViewed: recentSeed ? recentSeed : (game ? (parse(getSet.recent, {})[game.id] || []).filter(x => x.source === srcNow) : [])
    function rememberViewed(item, d) {
        if (!game || shot.on) return
        let all = parse(getSet.recent, {})
        let list = (all[game.id] || []).filter(x => !(x.id === String(item.id) && x.source === srcNow))
        list.unshift({ id: String(item.id), name: d.name || item.name, icon: item.icon || "", source: srcNow })
        all[game.id] = list.slice(0, 8)
        getSet.recent = JSON.stringify(all)
    }
    // The one-time line over the grid for a game with no mods yet.
    readonly property bool bannerOn: !!game && modsLoaded && mods.length === 0 && parse(seen.bannerDone, []).indexOf(game.id) < 0
    readonly property string getBanner: bannerOn ? "No mods for " + gameTitle + " yet. Pick one below, or search." : ""
    // Entering a game with no mods lands on Get mods (once per visit).
    property string landedOn: ""
    function maybeLandOnGet() {
        if (!game || !modsLoaded || landedOn === game.id || page !== "game") return
        landedOn = game.id
        if (mods.length === 0 && sources.length > 0 && tab === 0 && shot.page !== "emptygamemods" && shot.page.indexOf("mods") !== 0 && shot.page !== "removed") tab = 1
    }
    onModsChanged: if (game && mods.length > 0 && parse(seen.bannerDone, []).indexOf(game.id) < 0 && !shot.on) {
        let d = parse(seen.bannerDone, []); d.push(game.id); seen.bannerDone = JSON.stringify(d)
    }
    // Get mods history: B goes back exactly one step and puts everything back as it was.
    property var history: []
    property var restoreFocus: null
    function pushResults() {
        history = history.concat([{ kind: "results", text: getText, sort: getSortIndex, category: getCategory, source: srcNow, area: getArea, index: getIndex }])
    }
    function getBack() {
        if (sheet !== "") return
        if (modPage) {
            const h = history.slice()
            const top = h.pop()
            history = h
            if (top && top.kind === "mod" && h.length > 0 && h[h.length - 1].kind === "mod") {
                // Back to the mod page this requirement page came from.
                const prev = h[h.length - 1]
                history = h.slice(0, -1)
                openDetails(prev.item, true)
                return
            }
            modPage = false
            const res = h.length > 0 && h[h.length - 1].kind === "results" ? h[h.length - 1] : null
            if (res) {
                history = h.slice(0, -1)
                const same = res.text === getText && res.sort === getSortIndex && res.category === getCategory && res.source === srcNow && browseData
                getText = res.text; getSortIndex = res.sort; getCategory = res.category; getSource = res.source
                if (same) { getArea = res.area; getIndex = res.index }
                else { restoreFocus = { area: res.area, index: res.index }; browseNow() }
            }
            note("action", "Back to the results")
            return
        }
        if (getArea === "grid" || getArea === "recent") { getArea = "bar"; getIndex = 0; return }
        if (getText !== "") { getText = ""; getArea = "bar"; getIndex = 0; note("action", "Search cleared"); browseNow(); return }
        tab = 0
    }
    function browseNow() {
        if (!game || !rules || srcNow === "") return
        if (srcNow === "nexus" && nexusPublic === "failed" && !account.hasKey) {
            browseLoading = false
            browseData = { game: game.id, source: "nexus", items: [], categories: [], fallback: true }
            getArea = "fallback"; getIndex = 0
            return
        }
        browseLoading = true
        noMore = false
        Turbine.browse(getText, getCategory, sortKeys[getSortIndex] || "", srcNow)
    }
    function browseMore() {
        if (loadingMore || noMore || !browseData || srcNow !== "nexus" || getItems.length < 30) return
        loadingMore = true
        note("action", "Get mods: the next 30 from " + getItems.length)
        Turbine.browsePage(getText, getCategory, sortKeys[getSortIndex] || "", srcNow, getItems.length)
    }
    // Try again here: browsing without a key once more.
    function retryBrowse() {
        note("action", "Try again here")
        nexusPublic = ""
        browseData = null
        getArea = "bar"; getIndex = 0
        browseNow()
    }
    function getAct(b) {
        const cols = gridColumns
        if (b === "b") { getBack(); return }
        if (b === "r3") { getBarRun(0); return }
        if (getArea === "fallback") {
            if (b === "a") openNexusMods()
            else if (b === "x") retryBrowse()
            else if (b === "left") { zone = "rail"; railIndex = current }
            return
        }
        if (getArea === "bar") {
            const k = getBar[getIndex] ? getBar[getIndex].key : ""
            if (b === "left") { if (getIndex > 0) getIndex--; else { zone = "rail"; railIndex = current } }
            else if (b === "right") getIndex = Math.min(getBar.length - 1, getIndex + 1)
            else if (b === "down") { if (recentViewed.length > 0 && getText === "") { getArea = "recent"; getIndex = 0 } else if (getItems.length > 0) { getArea = "grid"; getIndex = 0 } }
            else if (b === "a") getBarRun(getIndex)
            else if (b === "x" && (k === "sort" || k === "category" || k === "source")) cycleBar(k, -1)
        } else if (getArea === "recent") {
            if (b === "left") { if (getIndex > 0) getIndex--; else { zone = "rail"; railIndex = current } }
            else if (b === "right") getIndex = Math.min(recentViewed.length - 1, getIndex + 1)
            else if (b === "up") { getArea = "bar"; getIndex = 0 }
            else if (b === "down" && getItems.length > 0) { getArea = "grid"; getIndex = 0 }
            else if (b === "a" && recentViewed[getIndex]) openDetails(recentViewed[getIndex])
        } else {
            if (b === "left") { if (getIndex % cols > 0) getIndex--; else { zone = "rail"; railIndex = current } }
            else if (b === "right") { if (getIndex % cols < cols - 1) getIndex = Math.min(getItems.length - 1, getIndex + 1) }
            else if (b === "up") { if (getIndex >= cols) getIndex -= cols; else if (recentViewed.length > 0 && getText === "") { getArea = "recent"; getIndex = 0 } else { getArea = "bar"; getIndex = 0 } }
            else if (b === "down") getIndex = Math.min(getItems.length - 1, getIndex + cols)
            else if (b === "a" && getItems[getIndex]) openDetails(getItems[getIndex])
            else if (b === "x" && getItems[getIndex]) { menuIndex = 0; sheet = "cardmenu" }
            else if (b === "y" && getItems[getIndex]) quickGet(getItems[getIndex])
            // Infinite scroll: the next 30 when the focus reaches the last row.
            if (getIndex >= getItems.length - cols) browseMore()
        }
    }
    // Left and Right (or A) cycle Sort, Category and From.
    function cycleBar(k, d) {
        if (k === "sort") { getSortIndex = (getSortIndex + d + sortNames.length) % sortNames.length; note("action", "Get mods sort " + sortNames[getSortIndex]) }
        else if (k === "category") { getCategory = getCats[(getCats.indexOf(getCategory) + d + getCats.length) % getCats.length]; note("action", "Get mods category " + (getCategory || "All")) }
        else if (k === "source") { getSource = sources[(sources.indexOf(srcNow) + d + sources.length) % sources.length]; getCategory = ""; getSortIndex = 0; note("action", "Get mods source " + getSource); browseData = null }
        browseNow()
    }
    function getBarRun(i) {
        const k = getBar[i] ? getBar[i].key : ""
        getArea = "bar"; getIndex = i
        if (k === "search") openAsk("search", "Search " + gameTitle + " mods", "Words from the mod's name or what it does.", getText, "Search", 100, false, "Search")
        else if (k === "sort" || k === "category" || k === "source") cycleBar(k, 1)
        else if (k === "add") openAdd()
    }
    // X on a card: Other files and Open on Nexus.
    readonly property var cardItem: getArea === "grid" ? getItems[getIndex] : null
    readonly property var cardMenuItems: !cardItem ? [] : [
        { icon: "info", title: "Open its page", detail: "Pictures, files and what it needs", act: "page" },
        { icon: "update", title: "Get it", detail: getWords(), act: "get" }
    ].concat(srcNow === "nexus" ? [{ icon: "play", title: "Open on Nexus", detail: "In your browser", act: "nexus" }] : [])
    function cardMenuRun(i) {
        const it = cardMenuItems[i], c = cardItem
        if (!it || !c) return
        sheet = ""
        if (it.act === "page") openDetails(c)
        else if (it.act === "get") quickGet(c)
        else if (it.act === "nexus") openNexus("https://www.nexusmods.com/" + rules.domain + "/mods/" + c.id)
    }
    function getWords() {
        if (srcNow === "thunderstore") return "Installs it with what it needs"
        return account.premium === true ? "Installs it in one press" : "Opens its file page on Nexus"
    }
    // Y on a card (and the card menu's Get it): the same as the mod page's main button.
    function quickGet(c) {
        if (c.installed) { toast.info("Already in your mods.") ; return }
        if (srcNow === "nexus" && account.premium !== true) {
            // Free: the page knows the exact file, so open the mod page first.
            openDetails(c)
            pendingGet = true
            return
        }
        note("action", "Get " + c.name + " from " + srcNow)
        Turbine.install(c.id, srcNow)
        edited()
    }
    property bool pendingGet: false

    // ---------------------------------------------------------------- the mod page (pushed over Get mods)
    property bool modPage: false
    property var detailsData: null
    property var detailsItem: null
    property int pageIndex: 0
    function openDetails(item, fromHistory) {
        note("action", "Mod page: " + item.name)
        if (!fromHistory) {
            if (!modPage) pushResults()
        }
        history = history.concat([{ kind: "mod", id: String(item.id), item: item }])
        detailsItem = item
        detailsData = { loading: true }
        modPage = true
        pageIndex = 0
        Turbine.details(String(item.id), item.source || srcNow)
    }
    onDetailsDataChanged: if (pendingGet && detailsData && !detailsData.loading) { pendingGet = false; if (!detailsData.error) installFromDetails() }
    readonly property bool pageInstalled: !!detailsData && (detailsData.installed === true || (detailsItem && detailsItem.installed === true))
    readonly property string mainLabel: !detailsData || detailsData.loading ? "" : detailsData.error ? "" : pageInstalled ? "Installed · On"
        : busyHere ? (dl.active ? "Installing " + Math.round(Math.max(0, dlPart) * 100) + "%" : "Installing...")
        : detailsData.source === "nexus" && detailsData.premium === false ? (nexusFallback ? "Open on Nexus" : "Get it on Nexus")
        : "Install"
    readonly property string mainLine: !detailsData || detailsData.loading || detailsData.error ? ""
        : pageInstalled ? "Already in your mods. X: Other files"
        : detailsData.source === "nexus" && detailsData.premium === false ? "Opens this file's page. Press Slow download there and wait the few seconds Nexus asks. Turbine adds it by itself."
        : ""
    property bool busyHere: false
    readonly property var pageNeeds: {
        const n = detailsData && detailsData.needs ? detailsData.needs : null
        if (!n) return []
        return (n.installed || []).concat(n.missing || []).concat(n.unchecked || [])
    }
    readonly property int pageRows: 2 + pageNeeds.length
    function modPageAct(b) {
        if (b === "b") { getBack(); return }
        if (b === "a") {
            if (pageIndex === 0) installFromDetails()
            else if (pageIndex === 1) openVersions()
            else openNeed(pageIndex - 2)
        }
        else if (b === "x") openVersions()
        else if (b === "up") { if (pageIndex > 0) pageIndex = pageIndex === 2 && otherVersions.length === 0 ? 0 : pageIndex - 1; else modView.scroll(-1) }
        else if (b === "down") { if (pageIndex < pageRows - 1) pageIndex = pageIndex === 0 && otherVersions.length === 0 ? Math.min(2, pageRows - 1) : pageIndex + 1; else modView.scroll(1) }
        else if (b === "left") { zone = "rail"; railIndex = current }
        else if (b === "lt") { modPage = false; history = []; tab = Math.max(0, tab - 1) }
        else if (b === "rt") { modPage = false; history = []; tab = Math.min(tabs.length - 1, tab + 1) }
        // LB and RB: the previous or next mod in the results.
        else if (b === "lb" || b === "rb") {
            const i = getItems.findIndex(x => detailsItem && String(x.id) === String(detailsItem.id))
            const j = i + (b === "lb" ? -1 : 1)
            if (i >= 0 && j >= 0 && j < getItems.length) {
                history = history.slice(0, -1)
                getIndex = j
                const h = history.slice()
                if (h.length > 0 && h[h.length - 1].kind === "results") { h[h.length - 1] = Object.assign({}, h[h.length - 1], { area: "grid", index: j }); history = h }
                openDetails(getItems[j], true)
            }
        }
        else if (b === "view" && game) { menuIndex = 0; sheet = "gamemenu" }
    }
    // A on a requirement: its page, pushed on the history.
    function openNeed(i) {
        const r = pageNeeds[i]
        if (!r) return
        if (r.modId > 0 && (!r.domain || r.domain === rules.domain)) openDetails({ id: String(r.modId), name: r.name, source: "nexus", icon: "" }, true)
        else if (r.url) openNexus(r.url)
    }
    function installFromDetails() {
        if (!detailsData || detailsData.loading || detailsData.error) return
        if (pageInstalled) { toast.info("Already in your mods. X: Other files"); return }
        if (detailsData.source === "nexus" && detailsData.premium === false) {
            const url = detailsData.filePage || detailsData.page
            note("action", "Get it on Nexus: " + detailsData.name + " (" + url + ")")
            openNexus(url)
            return
        }
        note("action", "Install " + detailsData.name + " from " + srcNow)
        Turbine.install(detailsData.id, detailsData.source || srcNow)
        edited()
        busyHere = true
    }
    Connections { target: app; function onDlChanged() { if (!app.dl.active && app.busyHere && !Turbine.busy) app.busyHere = false } }

    // Pages to open on Nexus Mods (free accounts, SKSE, a profile's Nexus mods).
    property string pagesTitle: ""
    property var pagesList: []
    property int pagesIndex: 0
    function openPages(title, list) {
        if (!list || list.length === 0) return
        pagesTitle = title; pagesList = list; pagesIndex = 0
        Turbine.handlerStatus()
        sheet = "pages"
    }
    function openPage(i) {
        const p = pagesList[i]
        if (!p) return
        note("action", "Open page: " + p.name)
        openNexus(p.url)
    }

    // ---------------------------------------------------------------- the browser (catch.rs)
    // What the browser check found, the browser helper's link, and the status shown over the browser.
    property var browser: ({ checked: false })
    property var bridge: ({ running: false })
    property var status: ({ active: false })
    property bool movedPaired: false
    readonly property bool browserFound: !browser.checked || (!!browser.result && browser.result.found === true)
    // Opens a Nexus page in the player's browser and watches Downloads for its file. False when there is no browser.
    // In Game Mode Turbine starts the browser itself (launch.rs); the status pill says what is happening (info look,
    // never a toast), and if Turbine is still in front 8 seconds later, "Didn't see Nexus open?" offers Steam's browser.
    property string openState: ""         // "", "opening", "open"
    property string openName: "the browser"
    property string openUrlNow: ""
    function openNexus(url) {
        if (!browserFound) { noBrowser(); return false }
        const w = parse(Turbine.watchPage(url), {})
        if (w.watching) note("catch", "Watching Downloads for this mod (" + w.game + ")")
        openUrlNow = url
        openName = browser.result && browser.result.name ? browser.result.name : shot.on ? "Chrome" : "the browser"
        openState = "opening"
        openTimer.restart()
        if (!shot.on) {
            const r = parse(Turbine.openUrl(url), { how: "desktop" })
            if (r.how !== "gamemode") Qt.openUrlExternally(url)
        }
        return true
    }
    function openNexusMods() {
        if (!rules || !rules.domain) return
        note("action", "Open " + gameTitle + " mods on Nexus Mods")
        openNexus("https://www.nexusmods.com/" + rules.domain + "/mods")
    }
    function steamOpen() {
        note("action", "Open in Steam's browser")
        openState = ""
        if (shot.on) return
        const e = Turbine.openInSteam(openUrlNow)
        if (e !== "") Qt.openUrlExternally("steam://openurl/" + openUrlNow)
    }
    function openHelpNow() { openTimer.stop(); if (sheet === "") { note("browser", "Still in front 8 seconds after Open: offering Steam's browser"); sheet = "openhelp" } }
    // 8 seconds after Open, Turbine still in front and no browser window: the safety net (Game Mode only).
    Timer {
        id: openTimer
        interval: 8000
        onTriggered: {
            if (app.openState === "opening" && app.active && Turbine.gamemode) app.openHelpNow()
            if (app.openState === "opening") app.openState = ""
        }
    }
    Timer { id: openGone; interval: 8000; onTriggered: app.openState = "" }
    Connections {
        target: app
        function onActiveChanged() {
            if (!app.active && app.openState === "opening") { app.openState = "open"; openTimer.stop() }
            else if (app.active && app.openState === "open") openGone.restart()
        }
    }
    function noBrowser() {
        note("browser", "No browser found: showing how to get one")
        infoIndex = 0
        sheet = "nobrowser"
    }
    // The browser helper: copied into Downloads, then the four steps; Pair opens the pairing page.
    property string helperFolder: ""
    function setupHelper() {
        const r = parse(Turbine.helperSetup(), { error: "Turbine couldn't put the helper in Downloads." })
        if (r.error) { toast.showPlain(r.error); return }
        helperFolder = r.folder
        infoIndex = 0
        sheet = "helper"
    }
    function pair(again) {
        if (!browserFound) { noBrowser(); return }
        const url = again ? Turbine.bridgePairAgain() : Turbine.bridgePairUrl()
        if (url === "") { toast.showPlain("Turbine's link for the browser helper isn't running. Close Turbine and open it again."); return }
        if (!shot.on) Qt.openUrlExternally(url)
        toast.showPlain("Opened the pairing page in the browser. It says Paired with Turbine when the helper is set up.")
    }
    readonly property string browserWords: !browser.checked ? "Looking for a web browser..."
        : browserFound ? browser.result.name + (browser.result.how === "flatpak" ? " (installed from Discover)" : browser.result.how === "default" ? " (your default browser)" : "")
          + ". Nexus pages open there."
        : "None found. Turbine opens Nexus Mods in your browser; A shows how to get one."
    readonly property string bridgeTitle: bridge.connected > 0 ? "Browser helper: Connected" : bridge.paired ? "Browser helper: Paired" : "Browser helper: Not paired"
    readonly property string bridgeWords: !bridge.running ? (bridge.error ? "Turbine's link isn't running: " + bridge.error + "." : "Turbine's link isn't running.")
        : bridge.connected > 0 ? "The helper is talking to Turbine now: downloads, progress and installer choices show on Nexus pages."
        : bridge.paired ? "Paired. It connects when the browser is open."
        : "Optional. After setting it up, Pair opens a page that links the helper to this Turbine."
    // Install notes after adding a mod, and "Add this file?" for a download the watcher could only guess.
    property string notesName: ""
    property var notesList: []
    property var found: null
    property int infoIndex: 0
    // X on "Found <file>": which installed game it goes into.
    property int pickIndex: 0
    function openPickGame() {
        if (!found || games.length === 0) return
        pickIndex = Math.max(0, games.findIndex(g => g.id === found.game))
        note("action", "Pick the game for " + found.name)
        sheet = "pickgame"
    }
    function pickGame(i) {
        const g = games[i], f = found
        if (!g || !f) return
        found = null; sheet = ""
        Turbine.catchPick(f.path, g.id)
    }
    function openNotes(name, list) {
        if (!list || list.length === 0) return
        notesName = name; notesList = list
        note("engine", "Install notes for " + name + ": " + list.length)
        if (sheet === "" || sheet === "notes") { infoIndex = 0; sheet = "notes" }
        else pendingNotes = true
    }
    property bool pendingNotes: false
    readonly property var infoButtons: sheet === "nobrowser" ? [{ key: "a", label: "Check again" }, { key: "b", label: "Close" }]
        : sheet === "helper" ? [{ key: "a", label: "Pair" }, { key: "b", label: "Close" }]
        : sheet === "found" ? (found && found.game ? [{ key: "a", label: "Add it" }] : []).concat([{ key: "x", label: found && found.game ? "Other game" : "Pick the game" }, { key: "b", label: "Not a mod" }])
        : [{ key: "a", label: "OK" }]
    function infoRun(i) {
        const label = infoButtons[i] ? infoButtons[i].label : ""
        note("action", "Sheet " + sheet + ": " + label)
        if (sheet === "nobrowser") { if (label === "Check again") { Turbine.browserCheck(); toast.showPlain("Looking for a browser...") } else sheet = "" }
        else if (sheet === "helper") { if (label === "Pair") { sheet = ""; pair(false) } else sheet = "" }
        else if (sheet === "found") {
            const f = found
            if (!f) { sheet = ""; return }
            if (label === "Other game" || label === "Pick the game") { openPickGame(); return }
            found = null; sheet = ""
            Turbine.watchConfirm(f.path, label === "Add it")
        }
        else sheet = ""
    }
    function infoAct(b) {
        if (b === "left" || b === "up") infoIndex = Math.max(0, infoIndex - 1)
        else if (b === "right" || b === "down") infoIndex = Math.min(infoButtons.length - 1, infoIndex + 1)
        else if (b === "a") infoRun(infoIndex)
        else if (b === "x" && infoButtons.findIndex(x => x.key === "x") >= 0) infoRun(infoButtons.findIndex(x => x.key === "x"))
        else if (b === "b") {
            const i = infoButtons.findIndex(x => x.key === "b")
            if (i >= 0) infoRun(i); else sheet = ""
        }
    }

    // The status card over other apps: only while Turbine isn't the window in front (the player is in the
    // browser), only with Settings > Accounts > Show status over other apps on. Game Mode uses the input-less
    // overlay window gamescope paints over the browser; Desktop Mode a small panel kept above the browser.
    readonly property var transfer: status.active ? status
        : dl.active ? { active: true, text: "Downloading " + dl.label, level: "busy", progress: dlPart, game: gameTitle, added: "", done: false, ask: "" }
        : null
    readonly property bool overlayWanted: !shot.on && browserSet.overlay && !app.active && transfer !== null
    onOverlayWantedChanged: showOverlay(overlayWanted, Turbine.gamemode)
    property bool hudMarked: false
    function showOverlay(on, gameMode) {
        if (gameMode) {
            if (on) { Turbine.overlayMark(true); hudMarked = true; hudWin.visible = true }
            else if (hudWin.visible || hudMarked) { hudWin.visible = false; Turbine.overlayMark(false); hudMarked = false }
        } else panelWin.visible = on
        note("overlay", (on ? "Status shown over other apps" : "Status overlay hidden") + (gameMode ? " (Game Mode)" : " (Desktop)"))
    }
    // A result stays 6 seconds, then the card goes.
    Timer { id: statusHide; interval: 6000; onTriggered: Turbine.statusDone() }
    onStatusChanged: if (status.active && status.done && !shot.on) statusHide.restart(); else statusHide.stop()

    // Nexus links (Mod Manager Download): from the command line, or from a later launch through the running Turbine.
    property var handler: ({ state: "unknown" })
    Timer { interval: 300; running: true; repeat: true; onTriggered: { const l = Turbine.takeLink(); if (l !== "") app.linkArrived(l) } }
    function linkArrived(link) {
        const r = parse(Turbine.openLink(link), { error: "That Nexus link can't be read." })
        if (!shot.on) { app.raise(); app.requestActivate() }
        if (r.needsKey) {
            // No Nexus key: Slow download works instead, and that file goes into this mod's game by itself.
            note("nxm", "Mod Manager Download without a Nexus key")
            if (r.page) Turbine.watchPage(r.page)
            say(r.error, "warn")
            msgTimer.interval = 12000; msgTimer.restart()
            toast.showPlain(r.error)
            return
        }
        if (r.error) { note("nxm", "Link refused: " + r.error); say(r.error, "error"); return }
        const i = games.findIndex(g => g.id === r.game)
        if (i >= 0 && i !== current) current = i
        page = "game"; zone = "main"; tab = 0; modPage = false
        if (sheet !== "fomod") sheet = ""
        note("nxm", "Link for " + r.name)
        Turbine.nxmDownload(link, r.game)
        edited()
    }

    // Other versions of the mod on the details page (X).
    property int versionIndex: 0
    readonly property var otherVersions: detailsData && detailsData.versions ? detailsData.versions : []
    function openVersions() {
        if (otherVersions.length === 0) return
        note("action", "Other files of " + detailsData.name)
        versionIndex = 0
        history = history.concat([{ kind: "files" }])
        sheet = "versions"
    }
    function installVersion(i) {
        const v = otherVersions[i]
        if (!v || v.installed) return
        sheet = ""
        if (detailsData.premium === false && detailsData.page) {
            note("action", "Get it on Nexus: " + v.name + " " + (v.version || ""))
            openNexus(detailsData.page + "&file_id=" + v.id)
            return
        }
        note("action", "Install version " + (v.version || v.name) + " of " + detailsData.name)
        Turbine.installVersion(detailsData.id, detailsData.source || srcNow, v.id)
        edited()
        busyHere = true
    }

    // Profiles > Create a Thunderstore code.
    property var codeData: null

    // Downloads: the status line and the rail's Downloads ring.
    property var dl: ({ active: false })
    readonly property real dlPart: dl.active && dl.total > 0 ? Math.min(1, dl.got / dl.total) : -1

    // Installer choices (FOMOD).
    property var fomod: null

    // Load order (Skyrim).
    property var order: []
    property string orderError: ""
    property var orderMessages: ({})      // LOOT's notes: plugin name lowercased -> [{level, text}]
    property string orderNote: ""         // what the last Sort automatically did
    readonly property var orderPlugin: orderIndex >= 0 && orderIndex < order.length ? order[orderIndex] : null
    property int orderIndex: 0
    property bool orderMoving: false
    property var orderSaved: []
    property int orderFrom: -1
    function orderAct(b) {
        const p = order[orderIndex]
        if (orderMoving) {
            if (b === "b") { order = orderSaved; orderIndex = orderFrom; orderMoving = false; note("action", "Move cancelled"); return }
            if (b === "up" || b === "down") {
                const to = orderIndex + (b === "up" ? -1 : 1)
                if (to < 0 || to >= order.length || order[to].fixed) return
                let list = order.slice(); const x = list[orderIndex]; list[orderIndex] = list[to]; list[to] = x
                order = list; orderIndex = to
            } else if (b === "a" || b === "y") {
                orderMoving = false
                if (orderIndex !== orderFrom && order[orderIndex]) Turbine.orderMove(order[orderIndex].name, orderIndex)
            }
            return
        }
        if (b === "up") orderIndex = Math.max(-1, orderIndex - 1)
        else if (b === "down") orderIndex = Math.min(order.length - 1, orderIndex + 1)
        else if (b === "a" && orderIndex === -1) Turbine.orderSort()
        else if (b === "a" && p && !p.fixed) {
            order = order.map((x, i) => i === orderIndex ? Object.assign({}, x, { active: !x.active }) : x)
            Turbine.orderToggle(p.name, !p.active)
        }
        else if (b === "y" && p && !p.fixed) { orderMoving = true; orderFrom = orderIndex; orderSaved = order.slice() }
        else if (b === "x" && p) { note("action", "Plugin details: " + p.name); sheet = "plugin" }
        else if (b === "left") { zone = "rail"; railIndex = current }
    }

    // Profiles.
    property var profileData: null
    property var shares: []
    property int shareIndex: 0
    property var importData: null
    property int profIndex: 0
    readonly property var profileList: profileData ? profileData.profiles : []
    readonly property var profileActions: [
        { key: "new", label: "New profile", detail: "From the mods as they are now" },
        { key: "export", label: "Export to a share file", detail: "Saves \"" + (profileData ? profileData.active : "") + "\" to Downloads, to share" },
        { key: "importFile", label: "Import a share file", detail: "A Turbine profile file in Downloads" }
    ].concat(rules && rules.community ? [
        { key: "code", label: "Create a Thunderstore code", detail: "Uploads \"" + (profileData ? profileData.active : "") + "\" to Thunderstore and shows a code to share" },
        { key: "importCode", label: "Import a Thunderstore profile code", detail: "A code someone shared with you" }
    ] : [])
    function openProfiles() { profIndex = 0; importData = null; Turbine.profiles(); sheet = "profiles" }
    function profileRun(i) {
        if (importData) {
            if (i === 0 && !importData.error) {
                note("action", "Import profile: get them")
                Turbine.profileImportGo(); sheet = ""
                const steps = account.premium ? (importData.nexusPremium || []) : (importData.nexusFree || [])
                const open = steps.filter(x => x.action === "open").map(x => ({ name: x.mod.name, url: x.url, note: "From the shared profile" }))
                if (open.length > 0) { importData = null; openPages("Open these pages for the profile", open); return }
            }
            importData = null
            if (sheet === "profiles") profIndex = 0
            return
        }
        if (i < profileList.length) {
            const n = profileList[i].name
            if (profileData && n === profileData.active) return
            note("action", "Switch profile to " + n)
            Turbine.profileSwitch(n)
            sheet = ""
            return
        }
        const a = profileActions[i - profileList.length]
        if (!a) return
        note("action", "Profiles: " + a.label)
        if (a.key === "new") openAsk("newProfile", "New profile", "A name for the mods as they are now.", "", "Profile name", 60, false, "Make it")
        else if (a.key === "export" && profileData) Turbine.profileExport(profileData.active)
        else if (a.key === "code" && profileData) { Turbine.profileCode(profileData.active); sheet = "code" }
        else if (a.key === "importFile") { Turbine.listShares(); shareIndex = 0; sheet = "shares" }
        else if (a.key === "importCode") openAsk("code", "Thunderstore profile code", "The code looks like 0190a2b4-7c1d-4e8f-9a0b-1c2d3e4f5a6b.", "", "Profile code", 40, false, "Read it")
    }

    // Mod updates: badges, the mod menu and Update all.
    property var updatesData: null
    readonly property var updateMap: {
        let m = {}
        if (updatesData && game && updatesData.game === game.id) for (const u of updatesData.items || []) m[u.mod] = u
        return m
    }
    readonly property int updateCount: Object.keys(updateMap).length
    property var updatesChecked: ({})
    function maybeCheckUpdates() {
        if (!game || updatesChecked[game.id] || shot.on) return
        const any = mods.some(m => m.source === "Thunderstore" || m.source === "Nexus Mods")
        if (!any) return
        let c = Object.assign({}, updatesChecked); c[game.id] = true; updatesChecked = c
        Turbine.checkUpdates()
    }

    // Accounts and Turbine's own updates.
    property var account: ({ state: "none" })
    property var selfUpd: ({ state: "idle" })
    property bool selfAuto: false
    property bool backToStable: false

    // A line of text (search, profile name, code, Nexus key), typed with Steam's keyboard in Game Mode.
    property string askPurpose: ""
    property string askReturn: ""
    function openAsk(purpose, title, noteText, start, placeholder, max, secret, done) {
        askPurpose = purpose
        askReturn = sheet
        askView.open(title, noteText, start, placeholder, max, secret, done)
        sheet = "ask"
        reportEditing(true)
    }
    function askDone(text) {
        reportEditing(false)
        sheet = askReturn === "ask" ? "" : askReturn
        if (askPurpose === "search") { getText = text; getArea = "bar"; getIndex = 0; note("action", "Get Mods search"); browseNow() }
        else if (askPurpose === "nexusKey" && text !== "") Turbine.setNexusKey(text)
        else if (askPurpose === "newProfile" && text !== "") Turbine.profileCreate(text)
        else if (askPurpose === "code" && text !== "") Turbine.profileImportCode(text)
    }
    function askCancelled() {
        reportEditing(false)
        sheet = askReturn === "ask" ? "" : askReturn
    }

    // ---------------------------------------------------------------- Labs
    readonly property var settingsSections: ["Getting mods", "Look", "Updates", "About"].concat(labsOn ? ["Labs"] : [])
    readonly property string sec: settingsSections[setSection] || "Getting mods"
    function openSettings() { page = "settings"; zone = "settings"; setRow = 0; if (sec === "Look") checkDisplays() }
    function openSection(name) { openSettings(); setSection = Math.max(0, settingsSections.indexOf(name)); setRow = 0 }
    onLabsOnChanged: if (!labsOn && setSection >= settingsSections.length) { setSection = settingsSections.indexOf("About"); setRow = 0 }
    // Accounts, Updates and Labs: rows of one kind each. action (A does it), toggle and choice (Left, Right or A
    // change it), item (a Labs checklist item), info (words only).
    readonly property var rowsNow: sec === "Getting mods" ? accountRows : sec === "Updates" ? updateRows : sec === "Labs" ? labsRows : []
    readonly property string accountWords: account.state === "checking" ? "Checking the key with Nexus Mods..."
        : account.state === "ok" ? "Signed in as " + account.name + (account.premium ? " · Premium: installs in one press with no browser" : " · Free account: Get it on Nexus opens the file's page")
        : account.state === "error" ? account.message
        : account.message ? account.message : "Premium: installs in one press with no browser. Where the key comes from: in Desktop Mode, sign in at nexusmods.com, open Site preferences, API Keys, and copy your Personal API Key."
    // Getting mods: first a verdict, not a setting; then what is optional.
    readonly property var accountRows: [
        { kind: "info", key: "verdict", title: nexusPublic === "failed" ? "You're ready. Get mods opens Nexus in your browser, nothing to set up." : "You're ready. Browse and get mods now, nothing to set up.",
          how: "Get mods on each game's page. Thunderstore mods install in one press; Nexus mods open their file page, and Turbine adds the download by itself." },
        { kind: "action", key: "nexusKey", head: "Optional, makes it faster", title: "Nexus Mods key", how: accountWords, action: account.hasKey ? "Change key" : "Add key" }
    ].concat(account.hasKey ? [{ kind: "action", key: "signOut", title: "Sign out of Nexus Mods", how: "Deletes the key from this device", action: "Sign out" }] : []).concat([
        { kind: "action", key: "helper", title: "Browser helper", action: "Set up",
          how: "Saves Nexus downloads with no Save box, sorted into Game Mods folders. Tip: in Chrome's settings, turn off Ask where to save each file." },
        { kind: "toggle", key: "overlay", title: "Show status over other apps",
          how: "A small Turbine card shows download progress and Added while you're in the browser.",
          value: browserSet.overlay, options: [false, true], names: ["Off", "On"] }
    ]).concat(bridge.paired || bridge.connected > 0 || shot.page === "browser" ? [
        { kind: "action", key: "pair", title: bridgeTitle, how: bridgeWords, action: bridge.paired ? "Pair again" : "Pair" }
    ] : []).concat([
        { kind: "action", key: "browser", title: "Web browser", how: browserWords, action: browserFound ? "Check again" : "How to get one" },
        { kind: "action", key: "nxmHandler", title: "Use Turbine for Mod Manager Download",
          how: handler.state === "yes" ? "Turbine gets Mod Manager Download links from the browser (they need a Nexus key)."
             : handler.state === "no" ? "Mod Manager Download links now go to " + (handler.current || "another program") + ". A makes Turbine get them."
             : "Makes the browser's Mod Manager Download button send files to Turbine (it needs your Nexus key; Slow download works without one).",
          action: handler.state === "yes" ? "Set again" : "Use Turbine" }
    ])
    readonly property string selfWords: selfUpd.state === "checking" ? "Looking for a newer Turbine..."
        : selfUpd.state === "available" ? "Turbine " + selfUpd.version + (selfUpd.prerelease ? " (pre-release)" : "") + " is ready to install"
        : selfUpd.state === "stable" ? "The latest stable Turbine is " + selfUpd.version
        : selfUpd.state === "current" ? "Turbine " + Turbine.version + " is the newest" + (labsOn && updates.prerelease ? ", pre-releases included" : "")
        : selfUpd.state === "installing" ? "Downloading and installing Turbine " + selfUpd.version + "..."
        : selfUpd.state === "ready" ? "Turbine " + selfUpd.version + " is installed. Restart to use it."
        : selfUpd.state === "error" ? selfUpd.message
        : "This is Turbine " + Turbine.version
    readonly property var updateRows: [
        { kind: "action", key: "checkNow", title: "Check for updates", how: selfWords, action: selfUpd.state === "checking" ? "" : "Check now" }
    ].concat(selfUpd.state === "available" || selfUpd.state === "stable" ? [
        { kind: "action", key: "install", title: "Install Turbine " + selfUpd.version, how: (selfUpd.notes || "").split("\n")[0] || "Downloads it, checks it, then installs it", action: "Install" }
    ] : []).concat(selfUpd.state === "ready" ? [
        { kind: "action", key: "restart", title: "Restart Turbine", how: "Closes Turbine and opens the new version", action: "Restart" }
    ] : []).concat([
        { kind: "toggle", key: "auto", title: "Check when Turbine starts", how: "Looks once at each start. Nothing installs without you.", value: updates.auto, options: [false, true], names: ["Off", "On"] },
        { kind: "info", key: "channel", title: "Update channel: " + (labsOn && updates.prerelease ? "Pre-release" : "Stable"), how: labsOn ? "Change it in Labs" : "Stable releases only" }
    ])
    function rowStep(r, d) {
        if (r.key === "auto") updates.auto = !updates.auto
        else if (r.key === "overlay") browserSet.overlay = !browserSet.overlay
        else if (r.key === "channel") { updates.prerelease = !updates.prerelease; Turbine.selfCheck(updates.prerelease, false) }
        note("action", "Setting " + r.key + " changed")
    }
    function rowRun(r) {
        note("action", "Settings: " + r.title)
        if (r.key === "report") openReport(null, "", "")
        else if (r.key === "perf") openPerf()
        else if (r.key === "steamBrowser" || r.key === "keyless") labsRun(r)
        else if (r.key === "nexusKey") openAsk("nexusKey", "Nexus Mods API key", "Paste or type your Personal API Key from nexusmods.com (Site preferences, API Keys). It's kept in a file only you can read.", "", "API key", 1024, true, "Check it")
        else if (r.key === "signOut") Turbine.nexusSignOut()
        else if (r.key === "nxmHandler") Turbine.handlerSet()
        else if (r.key === "browser") { if (browserFound) Turbine.browserCheck(); else noBrowser() }
        else if (r.key === "helper") setupHelper()
        else if (r.key === "pair") pair(bridge.paired)
        else if (r.key === "checkNow") Turbine.selfCheck(labsOn && updates.prerelease, false)
        else if (r.key === "install") Turbine.selfInstall()
        else if (r.key === "restart") { Turbine.finish(); Turbine.restart(); Qt.quit() }
        else if (r.key === "stable") { backToStable = true; Turbine.selfCheck(false, true); openSection("Updates") }
    }
    // Ten presses on the icon in Settings > About, a few seconds apart at most, turn Labs mode on or off.
    property int labsTaps: 0
    Timer { id: labsTapReset; interval: 3000; onTriggered: app.labsTaps = 0 }
    function labsTap() {
        labsTaps++
        labsTapReset.restart()
        if (labsTaps < 10) return
        labsTaps = 0
        labsSet.on = !labsSet.on
        note("labs", labsSet.on ? "Labs mode on" : "Labs mode off")
        toast.showPlain(labsSet.on ? "Labs mode on. Labs is at the end of Settings." : "Labs mode off")
    }
    // Things to confirm on the device: what the build server can't check. title, how to try it, what should happen.
    readonly property var labsItems: [
        { id: "pad-quiet", title: "Controller quiet after a game", how: "Play a game, press buttons as you quit it, then come back to Turbine.",
          expect: "Nothing happens in Turbine from buttons pressed in the game." },
        { id: "proton-bepinex", title: "Proton setting on a BepInEx game", how: "Add a BepInEx mod to Valheim, Lethal Company or Risk of Rain 2, then Play.",
          expect: "The check before Play says BepInEx gets turned on in Proton, and the game starts with the mod working." },
        { id: "running-check", title: "Running game check", how: "With a BepInEx game running, switch back to Turbine and press Play.",
          expect: "Turbine says the game is still running and changes nothing until it's closed." },
        { id: "nexus-keyless", title: "Get mods without a key", how: "Labs, Nexus without a key: Check. Then open Get mods in Skyrim, The Witcher 3 or Cyberpunk 2077.",
          expect: "The check says Nexus answered, and Get mods shows the game's mods as pictures with no key." },
        { id: "nexus-gamemode", title: "Nexus opens in Game Mode", how: "Game Mode: a mod page, A Get it on Nexus.",
          expect: "The browser opens over Turbine on that file's page. If not, after 8 seconds Turbine offers Steam's browser." },
        { id: "game-mods-folder", title: "Slow download with no Save box", how: "Browser helper set up in Chrome. Press Slow download on a mod's file page.",
          expect: "No Save box; the file lands in Downloads, Game Mods, the game's folder, and Turbine adds it to that game." },
        { id: "desktop-pad", title: "Controller in Desktop Mode", how: "Desktop Mode with Steam running: use Turbine with the controller.",
          expect: "Buttons work (as Steam's mouse and keyboard if Turbine sees no pad itself), and no line says no controller was found." },
        { id: "text-size", title: "Interface size in Game Mode", how: "In Game Mode, look at every page at Interface size 80% to 150%.",
          expect: "Everything is readable; nothing is cut off or overlapping." },
        { id: "add-downloads", title: "Add mods from Downloads", how: "Put a mod's zip in Downloads, then Get mods > Add a file from Downloads.",
          expect: "The mod shows in the list, turned on, with its name and version." },
        { id: "auto-apply", title: "Auto-apply, then start from Steam", how: "Turn a mod on or off, wait 5 seconds, close Turbine, start the game from the Steam library.",
          expect: "The game has the change without pressing Play in Turbine." },
        { id: "uninstall-undo", title: "Uninstall and Undo", how: "Uninstall a mod from its menu (X), then press Y for Undo.",
          expect: "It moves to Removed mods and comes back with Undo, in the same place." },
        { id: "play-original", title: "Play original", how: "Game menu (View) > Play original.",
          expect: "The game starts with no mods. The mods stay in Turbine and go back in at the next Play." },
        { id: "restore", title: "Restore last setup played", how: "Play with mods, change some, then Game menu (View) > Restore last working setup.",
          expect: "The mods go back to how they were the last time the game was played." },
        { id: "sd-card", title: "Game on the SD card", how: "Add a mod to a game on the SD card and play it; then try with the card out.",
          expect: "The mod works. With the card out, Turbine says the game is offline, never that mods are gone." },
        { id: "heroic-play", title: "Heroic game Play", how: "Pick an Epic or GOG game from Heroic and press Play.",
          expect: "Heroic starts the game with the mods in place." },
        { id: "seasonal", title: "Seasonal leaves and snow", how: "Settings > Look > Seasonal: Autumn, then Snow; try Amount and Size.",
          expect: "They fall smoothly behind the panels and stop while a game runs." },
        { id: "quick-report", title: "Screenshot and report", how: "Hold View for 7 seconds (or hold the icon at the top of the rail).",
          expect: "A brass ring fills, then a screenshot and report land in Downloads and the report pastes from the clipboard." },
        { id: "ts-install", title: "Thunderstore install", how: "Valheim, Lethal Company or Risk of Rain 2: Get mods, pick a mod, A Install.",
          expect: "It downloads with progress on the status line and the rail ring, and shows in Mods with what it needs." },
        { id: "loader-install", title: "Loader install", how: "Turn on a mod that needs the loader, then A Install in the prompt (SKSE, BepInEx or SMAPI).",
          expect: "The loader downloads from its official home, shows as a mod, and the Needs badges go away." },
        { id: "fomod", title: "Installer choices (FOMOD)", how: "Add a Skyrim mod with an installer, from Get mods or Downloads.",
          expect: "One question per page with pictures; B goes back; the files of the chosen options go in." },
        { id: "load-order", title: "Load order", how: "Skyrim: Load order tab. Turn a plugin off, move one with Y, then Sort automatically.",
          expect: "The game loads plugins in that order; masters stay first; missing masters are named." },
        { id: "ts-code", title: "Thunderstore code", how: "Lethal Company or another Thunderstore game, online: game menu (View), Profiles, Create a Thunderstore code.",
          expect: "A code shows big and is copied; importing it on another device (or Profiles, Import a Thunderstore profile code) gets the same mods." },
        { id: "other-versions", title: "Other versions", how: "Get mods, a mod page, X Other files, then A on an older version.",
          expect: "That version downloads and takes the installed one's place, same spot and on or off." },
        { id: "nxm-handler", title: "Mod Manager Download", how: "Settings, Accounts: Use Turbine for Nexus downloads. Then in Desktop Mode press Mod Manager Download on a Nexus mod's Files tab, with Turbine open and with it closed.",
          expect: "Turbine comes to the front on that game, the download shows on the status line, and the mod is added (with installer choices when it has them)." },
        { id: "nexus-free", title: "Free Nexus account", how: "With a free account: a Nexus mod's details, A Open on Nexus; also SKSE's Install in Skyrim.",
          expect: "The page opens in the browser; Mod Manager Download there sends the file to Turbine. In Game Mode Turbine says it works best in Desktop Mode." },
        { id: "nexus-requirements", title: "Nexus requirements", how: "Skyrim: details of a Nexus mod that needs others (like SkyUI needs SKSE).",
          expect: "Also needs lists them as Have it, Missing, Game DLC or Outside Nexus; Premium installs the missing ones with the mod, free accounts get their pages." },
        { id: "profile-nexus", title: "Profile import with Nexus mods", how: "Import a share file that has Nexus mods.",
          expect: "Premium: Turbine downloads them too. Free: it lists the pages to open, and the profile is made with what's there." },
        { id: "loot-sort", title: "LOOT sorting", how: "Skyrim with a few plugin mods, online: Load order tab, A on Sort automatically, then X on a plugin with a badge.",
          expect: "The line at the top says Sorted with LOOT's masterlist; the game starts with that order; badges like Dirty show LOOT's notes, and X lists them." },
        { id: "nexus-key", title: "Nexus Mods key", how: "Settings, Accounts: add your personal API key with Steam's keyboard.",
          expect: "It says your name and Premium or Free; Nexus Mods browsing works; Sign out removes the key." },
        { id: "mod-updates", title: "Mod updates", how: "Have an older Thunderstore or Nexus mod, open its game.",
          expect: "Update ready shows on it; X Update or the game menu's Update all brings the new version, same place and on or off." },
        { id: "profiles", title: "Profiles", how: "Game menu (View), Profiles: make one, switch, export a share file, import it or a Thunderstore code.",
          expect: "Switching changes which mods are on right away; an import gets the mods and makes the profile." },
        { id: "self-update", title: "Turbine updates itself", how: "Settings, Updates: Check now, Install, then Restart.",
          expect: "The new version installs and opens. With Pre-release in Labs, test builds show; Go back to stable returns to stable." },
        { id: "archives", title: "7z and rar mods", how: "Add a .7z and a .rar mod from Downloads.",
          expect: "Both unpack and install like zip files." },
        { id: "heroic-art", title: "Heroic covers", how: "Look at Heroic (Epic and GOG) games in the rail.",
          expect: "They show their cover art; games without art show their initials." },
        { id: "launch-option", title: "Valheim Launch Option", how: "Valheim with a BepInEx mod: press Play and read the check, then set the Launch Option in Steam.",
          expect: "The check explains the one-time setting once; after setting it, mods load when the game starts from Steam." },
        { id: "no-browser", title: "No browser warning", how: "On a system without a browser, open a Nexus page from Turbine (a mod's details, A Open on Nexus).",
          expect: "A warning says no browser was found, with the Discover and Add to Steam steps; Check again finds one once it's installed." },
        { id: "slow-download", title: "Slow download from Downloads", how: "With no browser helper and no setup: open Nexus any way you like (Get mods, a bookmark, typing the address), find a mod and press Slow download.",
          expect: "When the file finishes, Turbine adds it to the right game, or shows Found <file>. Add it to <game>? with A Add, X another game, B Not a mod. The file stays in Downloads." },
        { id: "catch-closed", title: "Downloads while Turbine is closed", how: "Close Turbine, download a mod with Slow download, then open Turbine.",
          expect: "Turbine catches up on it at start (adds it, or asks). A file you answered Not a mod is never asked about again." },
        { id: "nxm-no-key", title: "Mod Manager Download without a key", how: "With no Nexus key in Settings, Accounts, press Mod Manager Download on a Nexus mod.",
          expect: "Turbine says Mod Manager Download needs your Nexus API key and to use Slow download; a Slow download of that mod then goes into its game by itself." },
        { id: "nexus-no-key", title: "Open on Nexus when browsing fails", how: "If Get mods shows Browse ... mods on Nexus Mods: A Open on Nexus, then X Try again here.",
          expect: "The game's mods open in the browser and Turbine says: Download with Slow download. Turbine adds it when it lands in Downloads." },
        { id: "pointer", title: "Controller pointer", how: "Push the right stick, then press RT on a button, a mod's ON pill, a tab and a Settings row; press LT on a mod. Hold RT and move to drag. Try Game Mode and Desktop Mode, built-in controls and an Xbox pad.",
          expect: "A brass pointer moves smoothly (slow for small moves, fast across the screen), outlines what it's over, RT clicks it, LT on a mod opens its menu. It hides 3 seconds after the stick stops; A and the D-pad work as before; with the pointer hidden LT and RT still switch tabs." },
        { id: "scroll-stick", title: "Left stick scroll wheel", how: "Over a long list (Labs, Scrolling test, or Get mods), turn the left stick in circles both ways, then push it straight up and down.",
          expect: "Clockwise scrolls down, counterclockwise up, smoothly; pushing straight scrolls too. While Steam's keyboard is up, the sticks and triggers do nothing in Turbine." },
        { id: "md5-identify", title: "Files named from Nexus", how: "With a Nexus key, add a mod you downloaded from Nexus by hand (Add from Downloads).",
          expect: "It shows with its Nexus name and version, and gets Update ready badges like mods installed from Get mods." },
        { id: "overlay-desktop", title: "Status over the browser (Desktop Mode)", how: "Desktop Mode: start a Nexus download from Turbine, then stay in the browser.",
          expect: "A small Turbine card stays above the browser with progress, then Added to the game, and goes 6 seconds later. Its buttons work with a tap or click." },
        { id: "overlay-gamemode", title: "Status over the browser (Game Mode)", how: "Game Mode: open a Nexus page from Turbine (it opens over Turbine), download a mod, watch the bottom left corner.",
          expect: "The brass card shows over the browser and never takes the controller; it goes when idle, and the Performance Overlay still works after." },
        { id: "gamemode-window", title: "Turbine's window in Game Mode", how: "Open Turbine in Game Mode and use it for a minute (this build draws through X11 on SteamOS).",
          expect: "Turbine fills the screen, looks sharp, and the controller works as before." },
        { id: "browser-helper", title: "Browser helper (optional)", how: "Settings, Accounts: Browser helper (optional), Set up, follow the steps, then Pair. Download a mod on Nexus.",
          expect: "The pairing page says Paired; Settings says Connected; the Nexus page shows Turbine's progress, Added, and installer choices that work from either side." },
        { id: "steam-browser", title: "Steam's built-in browser", how: "Game Mode: Labs, Open Nexus in Steam's browser, sign in, then Slow download a mod for the game on screen.",
          expect: "The file arrives in Downloads and Turbine asks to add it. If it doesn't, say in the report where the file went (or that Steam didn't download it)." },
        { id: "cyberpunk", title: "Cyberpunk 2077 mods", how: "Cyberpunk 2077: add Cyber Engine Tweaks, RED4ext and a mod that needs them, plus a REDmod mod; press Play.",
          expect: "The check names missing frameworks and explains REDmod once; Proton settings get winmm and version; the mods load in the game." },
        { id: "witcher3", title: "The Witcher 3 mods", how: "The Witcher 3: add two mods, reorder them, press Play (start the game once first if it never ran).",
          expect: "Mods go in mods/modName; their order is written to mods.settings; scripts that clash show in the check; settings notes show after adding." },
        { id: "skyrim-nexus", title: "Skyrim from Nexus", how: "Skyrim Special Edition: get SKSE, Address Library and an SKSE plugin mod from Nexus (Mod Manager Download or Slow download).",
          expect: "SKSE goes in as the loader; without Address Library the check says it's missing; with it, the plugin loads." }
    ]
    readonly property var labsRows: [
        { kind: "action", key: "report", title: "Report a bug", how: "Screenshot, what went wrong, details Turbine adds by itself", action: "Report" },
        { kind: "action", key: "perf", title: "Performance check", how: "Frame times, processor, startup, controller", action: "Open" },
        { kind: "choice", key: "channel", title: "Update channel", how: "Pre-release gets test builds first", value: updates.prerelease, options: [false, true], names: ["Stable", "Pre-release"] },
        { kind: "action", key: "keyless", title: "Nexus without a key", how: labsResult ? labsResult.text : "Asks Nexus for this game's mods with no key and says what came back", action: "Check" },
        { kind: "action", key: "steamBrowser", title: "Open Nexus in Steam's browser", how: "For the Steam's built-in browser check below: opens nexusmods.com in Steam and watches Downloads for the game on screen", action: "Open" }
    ].concat(updates.prerelease ? [
        { kind: "action", key: "stable", title: "Go back to stable", how: "Installs the latest stable Turbine, even if it's older, and turns Pre-release off", action: "Go back" }
    ] : []).concat(labsItems.map((x, i) => Object.assign({ kind: "item", head: i === 0 ? "To confirm on your device" : "" }, x)))
    property var labsAnswers: parse(labsSet.states, {})
    readonly property string labsDone: {
        const n = labsItems.filter(x => labsAnswers[x.id] === "works").length
        const p = labsItems.filter(x => labsAnswers[x.id] === "problem").length
        return n + " of " + labsItems.length + " work" + (p > 0 ? ", " + p + " with a problem" : "")
    }
    function labsState(id) { return labsAnswers[id] || "" }
    // Where Left and Right are on a checklist row: 0 Works, 1 Problem, 2 Not tested.
    property int labsChip: 0
    function labsFocusRow() {
        const r = labsRows[setRow]
        if (r && r.kind === "item") { const st = labsState(r.id); labsChip = st === "works" ? 0 : st === "problem" ? 1 : 2 }
    }
    onSetRowChanged: if (sec === "Labs") labsFocusRow()
    property var labsResult: null
    function labsChoose(item, st, quiet) {
        if (!item) return
        let a = Object.assign({}, labsAnswers)
        if (st === "") delete a[item.id]
        else a[item.id] = st
        labsAnswers = a
        if (!shot.on) labsSet.states = JSON.stringify(a)
        note("labs", "Checklist: " + item.title + " = " + (st || "not tested"))
        if (st === "problem" && !quiet) openReport(item, "", "")
    }
    function labsRun(row) {
        note("action", "Labs: " + row.title)
        if (row.key === "report") openReport(null, "", "")
        else if (row.key === "perf") openPerf()
        else if (row.key === "keyless") { Turbine.nexusKeylessCheck(); toast.info("Asking Nexus...") }
        else if (row.key === "steamBrowser") {
            const url = Turbine.watchSteamBrowser()
            if (!shot.on) Qt.openUrlExternally(url)
            toast.showPlain("Opened Nexus Mods in Steam's browser. Try a Slow download; Turbine asks before adding what it finds.")
        }
    }
    function labsItemText(item) {
        if (!item) return ""
        return "Item: " + item.title + "\nHow to try it: " + item.how + "\nWhat should happen: " + item.expect
               + "\nChecklist answer: " + (labsState(item.id) || "not tested")
    }

    // What the interface looks like right now, for reports.
    function focusWords() {
        if (sheet === "modmenu") return "Mod menu item " + (modMenuItems[menuIndex] ? modMenuItems[menuIndex].title : menuIndex)
        if (sheet === "gamemenu") return "Game menu item " + (gameMenuItems[menuIndex] ? gameMenuItems[menuIndex].title : menuIndex)
        if (sheet === "add") return "Add mods row " + (downloads[addIndex] ? downloads[addIndex].name : addIndex)
        if (sheet === "check") return "Check before Play button " + (checkChoices[checkIndex] || checkIndex)
        if (sheet !== "") return "Sheet " + sheet
        if (zone === "rail") return "Rail item " + railIndex + (railIndex < games.length ? " (" + games[railIndex].name + ")" : "")
        if (page === "settings") return "Settings " + (settingsSections[setSection] || setSection) + " row " + setRow
               + (sec === "Look" && lookRows[setRow] ? " (" + lookRows[setRow].label + ")" : rowsNow[setRow] ? " (" + rowsNow[setRow].title + ")" : "")
        if (page === "perf") return "Scrolling test row " + perfIndex
        if (tab === 0 && focusedItem) return (focusedItem.kind === "mod" ? "Mod row " : "Removed row ") + (modIndex + 1) + " (" + focusedItem.m.name + ")"
        return "Tab " + (tabs[tab] || tab)
    }
    function interfaceText() {
        return [
            "Page: " + page + (sheet !== "" ? ", sheet " + sheet : "") + ", tab " + (tabs[tab] || tab) + (moving ? ", moving a mod" : ""),
            "Focus: " + focusWords() + " (zone " + zone + ")",
            "Window: " + width + " x " + height + (visibility === Window.FullScreen ? ", full screen" : ", windowed") + (active ? ", in front" : ", in the background"),
            "Screen: " + Screen.width + " x " + Screen.height + " px, pixel ratio " + Screen.devicePixelRatio.toFixed(2),
            "Scale: fit " + fit.toFixed(2) + "x, interface size " + sizeNow + "%, everything " + s.toFixed(2) + "x, layout " + layoutNow,
            "Look: theme " + look.theme + ", accent " + look.accent + ", contrast " + (look.highContrast ? "high" : "standard")
                + ", color vision " + look.vision + ", seasonal " + seasonPick + (seasonKind !== "" ? " (" + seasonKind + ", amount " + look.fallAmount + ", size " + look.fallSize + "%)" : ""),
            "Status line: " + statusNow.words + (statusNow.detail ? " · " + statusNow.detail : ""),
            "Waiting changes: " + Turbine.pending + ", busy " + (Turbine.busy ? "yes" : "no"),
            "Labs mode: " + (labsOn ? "on" : "off")
        ].join("\n")
    }

    // Report a bug: the screenshot is taken first, so the report screen isn't in it.
    property string reportContext: ""
    property var reportItem: null
    property bool discardArmed: false
    Timer { id: discardReset; interval: 3000; onTriggered: app.discardArmed = false }
    function openReport(item, startText, perfText) {
        reportContext = interfaceText()
        reportItem = item
        const tmp = Turbine.shotTemp()
        const open = function (ok) {
            reportView.shotUrl = ok ? fileUrl(tmp) + "?" + Date.now() : ""
            reportView.aboutLine = "Send saves the report to Downloads and copies it, with Turbine " + Turbine.version
                + ", this device, the game and its mods, and the last 300 lines of the activity log."
            reportView.open(item ? item.title : "", startText !== "" ? startText : item ? "Problem: " + item.title + ". " : "",
                            perfText, Turbine.hasCrashLog())
            perfView.visible = false
            sheet = "report"
        }
        if (!app.contentItem.grabToImage(function (r) { open(r.saveToFile(tmp)) })) open(false)
    }
    function sendReport() {
        if (reportView.text.trim() === "") { toast.showPlain("Say what went wrong first."); return }
        const stamp = Turbine.stamp()
        const withShot = reportView.include && reportView.shotUrl !== ""
        const shotName = withShot ? "Turbine-screenshot-" + stamp + ".png" : ""
        const text = Turbine.report(labsItemText(reportItem), reportView.text, reportContext,
                                    reportView.includePerf ? reportView.perf : "", reportView.includeLog && reportView.hasLog, shotName)
        const res = parse(Turbine.saveReport(text, stamp, withShot ? Turbine.shotTemp() : ""), { error: "Couldn't save the report." })
        if (res.error !== "") { toast.showPlain(res.error); return }
        Turbine.copyText(text)
        closeReport()
        toast.showPlain("Report saved to Downloads as " + res.report + ". The report is copied.")
    }
    function closeReport() {
        reportView.close()
        discardArmed = false
        if (sheet === "report") sheet = ""
    }
    function dismissReport() {
        if (reportView.text.trim() === "" || discardArmed) { note("labs", "Report closed without saving"); closeReport(); return }
        discardArmed = true
        discardReset.restart()
        toast.showPlain("Press B again to throw this report away.")
    }
    // Steam's on-screen keyboard is up: the controller pointer leaves the sticks and triggers to it.
    property bool keyboardUp: false
    onKeyboardUpChanged: if (!shot.on) Turbine.pointerPaused = keyboardUp
    function reportEditing(on) {
        keyboardUp = on
        note("labs", on ? "Typing the report" : "Done typing the report")
        if (Turbine.gamemode && !shot.on) Qt.openUrlExternally(on ? "steam://open/keyboard" : "steam://close/keyboard")
        if (!on) keyRoot.forceActiveFocus()
    }
    function openPerf() { perfView.version = Turbine.version; perfView.extra = perfExtra(); perfView.open(); sheet = "perf" }
    function perfExtra() {
        return "Window: " + width + " x " + height + ", " + (Turbine.gamemode ? "Game Mode" : "Desktop") + "\n"
            + "Turbine scale: fit " + fit.toFixed(2) + "x, interface size " + sizeNow + "%, everything " + s.toFixed(2) + "x, layout " + layoutNow + "\n"
            + "Look: theme " + look.theme + ", high contrast " + (look.highContrast ? "on" : "off") + ", seasonal " + seasonPick
            + (seasonKind !== "" ? " (" + seasonKind + (fallen.moving ? ", moving" : ", still") + ")" : "")
    }

    // Labs: hold View (or the icon at the top of the rail) for 7 seconds: a screenshot and a report go to Downloads
    // and the report is copied. Letting go early stops it.
    property real holdMs: 0
    Timer {
        id: holdTimer
        interval: 50
        repeat: true
        onTriggered: {
            app.holdMs += interval
            if (app.holdMs >= 7000) { stop(); app.holdMs = 0; app.quickReport() }
        }
    }
    readonly property bool holdShown: holdTimer.running && holdMs >= 300
    function holdStart(how) {
        if (!labsOn || holdTimer.running) return
        holdMs = 0
        holdTimer.start()
        note("labs", "Holding " + how + " for a screenshot and report")
    }
    function holdStop() {
        if (!holdTimer.running) return
        holdTimer.stop()
        holdMs = 0
        note("labs", "Let go early: no screenshot")
    }
    function quickReport() {
        const stamp = Turbine.stamp()
        const tmp = Turbine.shotTemp()
        const done = function (ok) {
            const shotName = ok ? "Turbine-screenshot-" + stamp + ".png" : ""
            const text = Turbine.report("", "", interfaceText(), "", false, shotName)
            const res = parse(Turbine.saveReport(text, stamp, ok ? tmp : ""), { error: "Couldn't save the report." })
            if (res.error !== "") { toast.showPlain(res.error); return }
            Turbine.copyText(text)
            toast.showPlain(ok ? "Screenshot and report saved to Downloads. The report is copied." : "Report saved to Downloads (the screenshot didn't work). The report is copied.")
        }
        // A moment for the ring to go before the picture.
        Qt.callLater(function () { if (!app.contentItem.grabToImage(function (r) { done(r.saveToFile(tmp)) })) done(false) })
    }

    // The app icon: the small, simpler one where the full one would be under 64 physical pixels.
    readonly property string iconFull: "file:///app/share/icons/hicolor/128x128/apps/io.github.ravenor907.TurbineModManager.png"
    readonly property string iconSmall: "file:///app/share/turbine-mod-manager/icon-small.png"

    // ---------------------------------------------------------------- Look settings rows
    readonly property var layoutOptions: externalShown ? ["handheld", "desktop", "tv"] : ["handheld", "desktop"]
    readonly property var lookRows: [
        { label: "Interface size", key: "size", slider: true, min: 80, max: 150, unit: "%", now: sizeNow,
          step: (d) => sizePreview(Math.max(80, Math.min(150, sizeNow + d * 10))) },
        { label: "Layout", options: layoutOptions, names: layoutOptions.map(o => o === "tv" ? "TV" : o.charAt(0).toUpperCase() + o.substring(1)), key: "layout", now: layoutNow,
          step: (d) => layoutPreview(layoutOptions[(layoutOptions.indexOf(layoutNow) + d + layoutOptions.length) % layoutOptions.length]) },
        { label: "Theme", options: ["brass", "black", "light"], names: ["Brass", "Black", "Light"], key: "theme",
          step: (d) => cycle("theme", ["brass", "black", "light"], d) },
        { label: "Accent", options: Object.keys(c.swatches), key: "accent", swatch: true,
          step: (d) => cycle("accent", ["brass", "iron", "copper", "jade", "fjord", "heather", "ember"], d) },

        { label: "Contrast", options: [false, true], names: ["Standard", "High"], key: "highContrast",
          step: (d) => { look.highContrast = !look.highContrast } },
        { label: "Color vision", options: ["typical", "red", "green", "blue", "none"], names: ["Typical", "Red", "Green", "Blue", "None"], key: "vision",
          step: (d) => cycle("vision", ["typical", "red", "green", "blue", "none"], d) },

        { label: "Seasonal", options: ["off", "date", "autumn", "snow"], names: ["Off", "By date", "Autumn", "Snow"], key: "season",
          now: seasonPick, step: (d) => { look.season = seasonPick; cycle("season", ["off", "date", "autumn", "snow"], d) } }
    ].concat(seasonKind !== "" ? [
        { label: "Amount", key: "fallAmount", slider: true, min: 10, max: 100, unit: "",
          step: (d) => slide("fallAmount", 10, 100, 10, d) },
        { label: "Size", key: "fallSize", slider: true, min: 50, max: 200, unit: "%",
          step: (d) => slide("fallSize", 50, 200, 25, d) }
    ] : [])
    function slide(key, lo, hi, by, d) {
        look[key] = Math.max(lo, Math.min(hi, look[key] + d * by))
        note("action", "Look: " + key + " " + look[key])
    }
    // Seasonal: off on handhelds and by date on TV until chosen. By date: snow from December 1 to the end of
    // February, leaves the rest of the year. Checked every hour, so a new season starts without a restart.
    readonly property string seasonPick: shot.arg("season") !== "" ? shot.arg("season") : look.season !== "" ? look.season : (tv ? "date" : "off")
    property date today: new Date()
    Timer { interval: 3600000; running: true; repeat: true; onTriggered: app.today = new Date() }
    readonly property string seasonKind: {
        if (seasonPick === "autumn" || seasonPick === "snow") return seasonPick
        if (seasonPick !== "date") return ""
        const md = (today.getMonth() + 1) * 100 + today.getDate()
        return md >= 1201 || md <= 229 ? "snow" : "autumn"
    }
    function cycle(key, list, d) {
        const i = list.indexOf(look[key])
        look[key] = list[(i + d + list.length) % list.length]
        note("action", "Look: " + key + " " + look[key])
    }

    // ---------------------------------------------------------------- shared pieces
    component Label: Text {
        color: c.text
        font.family: app.bodyFont
        font.pixelSize: app.px(15)
        elide: Text.ElideRight
        verticalAlignment: Text.AlignVCenter
    }
    component Key: Rectangle {
        property string k: ""
        property bool square: k.length > 1
        implicitWidth: Math.max(app.px(24), keyText.implicitWidth + app.px(12))
        implicitHeight: app.px(24)
        radius: square ? app.px(5) : height / 2
        color: c.raised
        border.color: c.accentLo
        border.width: Math.max(1, app.px(1))
        Text {
            id: keyText
            anchors.centerIn: parent
            text: parent.k.toUpperCase()
            color: c.text
            font.family: app.bodyFont
            font.pixelSize: app.px(14)
            font.bold: true
        }
    }
    // One or more keys ("lb+rb") and what they do.
    component Hint: Row {
        id: hint
        property string k: ""
        property string label: ""
        // In the legend, a hint for one button is also a button: a tap or click does what the button does.
        readonly property bool clickable: parent === legendRow && ["a", "b", "x", "y", "view"].indexOf(k) >= 0
        spacing: app.px(6)
        TapHandler { enabled: hint.clickable; onTapped: { app.note("action", "Legend " + hint.k + " (" + hint.label + ")"); app.act(hint.k) } }
        HoverHandler { enabled: hint.clickable; onHoveredChanged: app.hover(hint, hovered) }
        Repeater {
            model: hint.k.split("+")
            Key { required property string modelData; k: modelData; visible: modelData !== ""; anchors.verticalCenter: parent.verticalCenter }
        }
        Label { visible: hint.label !== ""; text: hint.label; font.pixelSize: app.px(15); color: c.soft; anchors.verticalCenter: parent.verticalCenter }
    }
    component Icon: Shape {
        property string d: ""
        property color tint: c.muted
        width: app.px(22); height: app.px(22)
        preferredRendererType: Shape.CurveRenderer
        ShapePath {
            strokeColor: tint
            strokeWidth: 2
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            scale: Qt.size(app.px(22) / 24, app.px(22) / 24)
            PathSvg { path: d }
        }
    }
    function fileUrl(p) { return p ? "file://" + p.split("/").map(encodeURIComponent).join("/") : "" }
    function initials(name) {
        const words = (name || "?").replace(/[^A-Za-z0-9 ]/g, " ").split(" ").filter(w => w.length > 0 && w !== "The" && w !== "of")
        return (words.length > 1 ? words[0][0] + words[1][0] : (words[0] || "?").substring(0, 2)).toUpperCase()
    }
    function runtimeWords(g) {
        if (g.runtime === "windows") return g.store === "steam" ? "Windows game, runs through Proton" : "Windows game, runs through Wine or Proton"
        if (g.runtime === "native") return "Native Linux game"
        return "Not started yet"
    }
    // ---------------------------------------------------------------- layout
    // Seasonal leaves or snow, behind everything. Still while Turbine isn't in front (a game is running).
    Seasonal {
        id: fallen
        anchors.fill: parent
        kind: app.seasonKind
        amount: look.fallAmount
        size: look.fallSize
        scale: app.s
        light: c.light
        moving: app.active || shot.on
    }
    Row {
        id: mainRow
        anchors.fill: parent

        // ===== Rail: the logo, the games (last played first, portrait art), Settings pinned at the bottom.
        Rectangle {
            id: rail
            width: app.px(72)
            height: parent.height
            color: c.rail
            z: 5
            Column {
                id: railTop
                anchors.top: parent.top
                anchors.topMargin: app.px(10)
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: app.px(10)
                // The app icon (the simpler small one under 64 physical pixels); a plain "T" if it can't be loaded.
                // In Labs mode, holding it for 7 seconds saves a screenshot and a report.
                Image {
                    id: logo
                    readonly property bool small: app.px(44) * Screen.devicePixelRatio < 64
                    width: app.px(44); height: app.px(44)
                    anchors.horizontalCenter: parent.horizontalCenter
                    source: small ? app.iconSmall : app.iconFull
                    sourceSize.width: app.px(88)
                    smooth: true
                    mipmap: true
                    visible: status === Image.Ready
                    HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                    TapHandler {
                        onPressedChanged: pressed ? app.holdStart("the rail icon") : app.holdStop()
                    }
                }
                Rectangle {
                    visible: logo.status !== Image.Ready
                    width: app.px(44); height: app.px(44); radius: width / 2
                    anchors.horizontalCenter: parent.horizontalCenter
                    gradient: Gradient {
                        GradientStop { position: 0; color: c.accentHi }
                        GradientStop { position: 1; color: c.accent }
                    }
                    Text {
                        anchors.centerIn: parent
                        text: "T"
                        color: c.onAccent
                        font.family: app.titleFont
                        font.pixelSize: app.px(24)
                        font.bold: true
                    }
                }
                Rectangle { width: app.px(28); height: app.px(2); radius: 1; color: c.line; anchors.horizontalCenter: parent.horizontalCenter }
            }
            ListView {
                id: railList
                anchors.top: railTop.bottom
                anchors.topMargin: app.px(10)
                anchors.bottom: gear.top
                anchors.bottomMargin: app.px(10)
                width: parent.width
                clip: true
                spacing: app.px(8)
                model: app.games.length
                currentIndex: app.zone === "rail" ? Math.min(app.railIndex, app.games.length - 1) : app.current
                highlightFollowsCurrentItem: false
                onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                delegate: Item {
                    id: tileItem
                    required property int index
                    readonly property var g: app.games[index]
                    readonly property bool here: index === app.current && app.page === "game"
                    readonly property bool focused: app.zone === "rail" && app.railIndex === index
                    width: railList.width
                    height: app.px(66)
                    Rectangle {
                        visible: tileItem.here
                        x: app.px(1); width: app.px(4); height: app.px(36); radius: app.px(2)
                        anchors.verticalCenter: parent.verticalCenter
                        color: c.accent
                    }
                    Rectangle {
                        id: tile
                        width: app.px(44); height: app.px(66)
                        anchors.horizontalCenter: parent.horizontalCenter
                        radius: app.px(6)
                        color: c.raised
                        border.width: tileItem.here || tileItem.focused ? app.px(2) : 0
                        border.color: tileItem.focused ? c.text : c.accent
                        clip: true
                        Image {
                            id: art
                            anchors.fill: parent
                            anchors.margins: tile.border.width
                            visible: status === Image.Ready
                            source: tileItem.g ? app.fileUrl(tileItem.g.cover || tileItem.g.header) : ""
                            fillMode: Image.PreserveAspectCrop
                            sourceSize.height: app.px(132)
                            asynchronous: true
                        }
                        Text {
                            anchors.centerIn: parent
                            width: parent.width - app.px(6)
                            visible: !art.visible
                            horizontalAlignment: Text.AlignHCenter
                            wrapMode: Text.WordWrap
                            maximumLineCount: 4
                            elide: Text.ElideRight
                            text: tileItem.g ? tileItem.g.name : ""
                            color: c.soft
                            font.family: app.bodyFont
                            font.pixelSize: app.px(10)
                            font.bold: true
                        }
                    }
                    // A mod waiting or an update ready.
                    Rectangle {
                        visible: tileItem.here && app.updateCount > 0
                        anchors.right: tile.right; anchors.top: tile.top
                        anchors.margins: -app.px(3)
                        width: app.px(10); height: width; radius: width / 2
                        color: c.accentHi
                        border.color: c.rail; border.width: app.px(2)
                    }
                    HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                    TapHandler { onTapped: { app.railIndex = tileItem.index; app.railSelect(tileItem.index) } }
                }
            }
            Rectangle {
                id: gear
                anchors.bottom: parent.bottom
                anchors.bottomMargin: app.px(10)
                anchors.horizontalCenter: parent.horizontalCenter
                width: app.px(44); height: app.px(44); radius: app.px(8)
                readonly property bool focused: app.zone === "rail" && app.railIndex === app.games.length
                color: app.page === "settings" || app.page === "perf" ? c.raised : "transparent"
                border.width: focused ? app.px(2) : 0
                border.color: c.text
                Icon {
                    anchors.centerIn: parent
                    tint: app.page === "settings" ? c.accent : c.muted
                    d: "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.6 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.6a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"
                }
                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                TapHandler { onTapped: app.openSettings() }
            }
        }

        // ===== Main
        Item {
            id: main
            width: parent.width - rail.width
            height: parent.height

            Column {
                id: mainCol
                anchors.fill: parent
                anchors.leftMargin: app.px(14)
                anchors.rightMargin: app.px(14)
                anchors.bottomMargin: legend.height + app.px(16)
                spacing: app.px(8)

                // Header: the game's wide art darkened under the words, its name, the status under it, Start Play.
                Rectangle {
                    id: header
                    width: parent.width + app.px(28)
                    x: -app.px(14)
                    height: app.px(72)
                    color: c.panel
                    clip: true
                    Image {
                        id: hero
                        anchors.fill: parent
                        visible: app.page === "game" && status === Image.Ready
                        source: app.page === "game" && app.game && app.game.header ? app.fileUrl(app.game.header) : ""
                        fillMode: Image.PreserveAspectCrop
                        asynchronous: true
                    }
                    Rectangle {
                        visible: hero.visible
                        anchors.fill: parent
                        gradient: Gradient {
                            orientation: Gradient.Horizontal
                            GradientStop { position: 0; color: Qt.rgba(c.ground.r, c.ground.g, c.ground.b, 0.85) }
                            GradientStop { position: 1; color: Qt.rgba(c.ground.r, c.ground.g, c.ground.b, 0.40) }
                        }
                    }
                    Rectangle {
                        id: backBtn
                        visible: app.page !== "game" || app.modPage
                        x: app.px(14)
                        anchors.verticalCenter: parent.verticalCenter
                        width: backRow.implicitWidth + app.px(20); height: app.px(32); radius: app.px(6)
                        color: c.raised
                        Hint { id: backRow; anchors.centerIn: parent; k: "b"; label: "Back" }
                        HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                        TapHandler { onTapped: app.act("b") }
                    }
                    Column {
                        x: backBtn.visible ? backBtn.x + backBtn.width + app.px(14) : app.px(16)
                        width: parent.width - x - playBtn.width - app.px(40)
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: app.px(2)
                        Text {
                            id: headerTitle
                            width: parent.width
                            elide: Text.ElideRight
                            text: app.page === "settings" ? "Settings" : app.page === "perf" ? "Scrolling test" : app.game ? app.gameTitle : "Turbine Mod Manager"
                            color: c.text
                            font.family: app.page === "game" && app.game ? app.titleFont : app.bodyFont
                            font.pixelSize: app.px(20)
                            font.weight: Font.DemiBold
                        }
                        Row {
                            width: parent.width
                            spacing: app.px(8)
                            Rectangle {
                                width: app.px(8); height: width; radius: width / 2
                                anchors.verticalCenter: parent.verticalCenter
                                color: app.statusNow.level === "warn" ? c.warn : app.statusNow.level === "stop" ? c.stop : app.statusNow.level === "busy" ? c.accentHi : c.ok
                            }
                            Text {
                                id: statusWords
                                width: Math.min(implicitWidth, parent.width - app.px(16) - (pill.visible ? pill.width + app.px(8) : 0))
                                elide: Text.ElideRight
                                anchors.verticalCenter: parent.verticalCenter
                                text: app.statusNow.words + (app.statusNow.detail ? " · " + app.statusNow.detail : "")
                                color: app.statusNow.level === "stop" ? c.stop : app.statusNow.level === "warn" ? c.warn : c.soft
                                font.family: app.bodyFont
                                font.pixelSize: app.px(14)
                            }
                            // While anything downloads: "Getting 2 mods · 64%"; otherwise updates ready.
                            Rectangle {
                                id: pill
                                readonly property string words: app.dl.active ? "Getting " + app.plural(Math.max(1, Turbine.netBusy), "mod", "mods") + (app.dlPart >= 0 ? " · " + Math.round(app.dlPart * 100) + "%" : "")
                                                                 : app.statusNow.pill || ""
                                visible: words !== ""
                                anchors.verticalCenter: parent.verticalCenter
                                width: pillText.implicitWidth + app.px(16); height: app.px(22); radius: height / 2
                                color: c.raised
                                border.color: c.accentLo
                                Text { id: pillText; anchors.centerIn: parent; text: pill.words; color: c.accentText; font.family: app.bodyFont; font.pixelSize: app.px(13); font.bold: true }
                                TapHandler { onTapped: if (app.updateCount > 0 && !app.dl.active) Turbine.updateAll() }
                            }
                        }
                    }
                    Rectangle {
                        id: playBtn
                        visible: app.page === "game" && app.game !== null
                        anchors.right: parent.right
                        anchors.rightMargin: app.px(14)
                        anchors.verticalCenter: parent.verticalCenter
                        width: playRow.implicitWidth + app.px(24); height: app.px(38); radius: app.px(8)
                        opacity: Turbine.busy ? 0.7 : 1
                        gradient: Gradient {
                            GradientStop { position: 0; color: c.accentHi }
                            GradientStop { position: 1; color: c.accent }
                        }
                        Row {
                            id: playRow
                            anchors.centerIn: parent
                            spacing: app.px(8)
                            Key { k: "start"; anchors.verticalCenter: parent.verticalCenter }
                            Text { text: "Play"; color: c.onAccent; font.family: app.bodyFont; font.pixelSize: app.px(16); font.bold: true; anchors.verticalCenter: parent.verticalCenter }
                        }
                        HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                        TapHandler { onTapped: app.startPlay() }
                    }
                }

                // Tabs (game page only): My mods, Get mods, and Load order for plugin games. Always shown.
                Rectangle {
                    visible: app.page === "game" && app.game !== null
                    width: parent.width
                    height: app.px(38)
                    radius: app.px(6)
                    color: c.panel
                    Row {
                        id: tabRow
                        anchors.fill: parent
                        anchors.margins: app.px(3)
                        spacing: app.px(4)
                        Key { id: ltKey; k: "lt"; anchors.verticalCenter: parent.verticalCenter }
                        Repeater {
                            model: app.tabs
                            Rectangle {
                                required property int index
                                required property string modelData
                                width: (tabRow.width - ltKey.width * 2 - tabRow.spacing * (app.tabs.length + 1)) / app.tabs.length
                                height: tabRow.height
                                radius: app.px(5)
                                color: app.tab === index ? c.raised : "transparent"
                                Rectangle { visible: app.tab === parent.index; anchors.bottom: parent.bottom; width: parent.width; height: app.px(2); color: c.accent }
                                Label { anchors.centerIn: parent; text: parent.modelData; font.weight: Font.DemiBold; color: app.tab === parent.index ? c.text : c.muted }
                                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                TapHandler { onTapped: { app.modPage = false; app.tab = parent.index } }
                            }
                        }
                        Key { k: "rt"; anchors.verticalCenter: parent.verticalCenter }
                    }
                }
                // Page content
                Item {
                    id: content
                    width: parent.width
                    height: mainCol.height - y
                    clip: true

                    // No games, mods being read.
                    EmptyState {
                        id: middle
                        readonly property bool none: !app.game
                        readonly property bool loading: app.game !== null && app.tab === 0 && !app.modsLoaded
                        ui: kit
                        anchors.fill: parent
                        visible: app.page === "game" && (none || loading)
                        title: none ? "No games yet" : "Reading mods..."
                        body: none ? "Turbine looks in Steam's libraries, including SD cards, and in Heroic. Install a game and it shows up here." : ""
                        buttonKey: none ? "a" : ""
                        buttonLabel: none ? "Look again" : ""
                        onActivated: app.load()
                    }

                    // My mods: "+ Get mods for <game>", the mods, then the collapsed Removed row.
                    Label {
                        id: moveLine
                        visible: app.page === "game" && app.tab === 0 && app.moving
                        height: visible ? app.px(26) : 0
                        x: app.px(6)
                        text: "Lower in the list wins when two mods change the same file."
                        color: c.soft
                        font.pixelSize: app.px(14)
                    }
                    ListView {
                        id: modList
                        visible: app.page === "game" && app.game !== null && app.tab === 0 && app.modsLoaded
                        anchors.fill: parent
                        anchors.topMargin: moveLine.height
                        topMargin: app.px(4)
                        bottomMargin: app.px(4)
                        clip: true
                        spacing: app.px(app.desktopLayout ? 3 : 6)
                        model: app.listItems
                        currentIndex: app.modIndex
                        highlightFollowsCurrentItem: false
                        onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                        // After the left stick scrolls, focus the nearest row still in view.
                        onMovementEnded: {
                            const top = indexAt(10, contentY + app.px(10)), bottom = indexAt(10, contentY + height - app.px(10))
                            if (top >= 0 && app.modIndex < top) app.modIndex = top
                            else if (bottom >= 0 && app.modIndex > bottom) app.modIndex = bottom
                        }
                        delegate: Item {
                            id: entry
                            required property int index
                            required property var modelData
                            readonly property string kind: modelData.kind
                            readonly property var m: modelData.m
                            readonly property bool foc: app.zone === "main" && app.sheet === "" && app.modIndex === index
                            readonly property Item pill: row.pillItem
                            width: modList.width
                            height: kind === "mod" ? row.implicitHeight : app.px(app.desktopLayout ? 38 : 46)
                            ModRow {
                                id: row
                                visible: entry.kind === "mod"
                                ui: kit
                                x: app.px(4)
                                width: parent.width - app.px(8)
                                name: entry.m.name || ""
                                detail: app.modWords(entry.m)
                                isOn: entry.m.enabled === true
                                ordered: app.ordered
                                compact: app.desktopLayout
                                order: entry.index
                                focused: entry.foc
                                badge: app.moving && entry.foc ? "Moving" : (entry.m.needsLoader && app.loaderMissing ? "Needs " + app.loaderShort : app.updateMap[entry.m.id] ? "Update ready" : entry.kind === "mod" && app.isNew(entry.m) ? "New" : "")
                                badgeLevel: app.moving && entry.foc ? "info" : (entry.m.needsLoader && app.loaderMissing) ? "warn" : "info"
                                showHints: entry.foc && !app.moving
                                onClicked: app.modIndex = entry.index
                                onToggleClicked: { app.modIndex = entry.index; app.toggle(entry.m) }
                                // A right-click (LT with the pointer) opens the mod's menu, like X.
                                onMenuClicked: if (app.sheet === "" && !app.moving) { app.modIndex = entry.index; app.zone = "main"; app.sheet = "modmenu" }
                            }
                            Rectangle {
                                visible: entry.kind !== "mod"
                                x: app.px(4)
                                width: parent.width - app.px(8)
                                height: parent.height
                                radius: app.px(6)
                                color: entry.foc ? c.focusFill : entry.kind === "get" ? c.card : "transparent"
                                border.color: entry.foc ? c.accent : c.line
                                border.width: entry.foc ? Math.max(1, app.px(2)) : (entry.kind === "removed" ? Math.max(1, app.px(1)) : 0)
                                Label {
                                    x: app.px(14)
                                    anchors.verticalCenter: parent.verticalCenter
                                    width: parent.width - app.px(180)
                                    text: entry.kind === "get" ? "+ Get mods for " + app.gameTitle
                                          : app.plural(app.removedMods.length, "removed mod", "removed mods") + " (kept 7 days)"
                                    color: entry.kind === "get" ? c.accentText : c.muted
                                    font.pixelSize: app.px(15)
                                    font.weight: entry.kind === "get" ? Font.DemiBold : Font.Normal
                                }
                                Hint {
                                    visible: entry.foc
                                    anchors.right: parent.right
                                    anchors.rightMargin: app.px(12)
                                    anchors.verticalCenter: parent.verticalCenter
                                    k: "a"; label: entry.kind === "get" ? "Get mods" : "Open"
                                }
                                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                TapHandler { onTapped: { app.modIndex = entry.index; app.act("a") } }
                            }
                        }
                    }

                    // Get mods: a game with no source can still add a file.
                    EmptyState {
                        ui: kit
                        anchors.fill: parent
                        visible: app.page === "game" && app.game !== null && app.tab === 1 && app.sources.length === 0
                        title: "Get mods"
                        body: "This game isn't on Nexus Mods or Thunderstore. You can still add a mod file."
                        buttonKey: "a"
                        buttonLabel: "Add a file from Downloads"
                        buttonFocused: app.zone === "main"
                        onActivated: app.openAdd()
                    }
                    GetMods {
                        id: getView
                        ui: kit
                        anchors.fill: parent
                        anchors.topMargin: app.px(4)
                        visible: app.page === "game" && app.game !== null && app.tab === 1 && app.sources.length > 0 && !app.modPage
                        result: app.browseData
                        loading: app.browseLoading
                        loadingMore: app.loadingMore
                        search: app.getText
                        category: app.getCategory
                        bar: app.getBar
                        recent: app.recentViewed
                        banner: app.getBanner
                        fallback: app.nexusFallback
                        fallbackTitle: "Browse " + app.gameTitle + " mods on Nexus Mods"
                        columns: app.gridColumns
                        focusArea: app.getArea
                        focusIndex: app.getIndex
                        active: app.zone === "main" && app.sheet === ""
                        onBarClicked: (i) => app.getBarRun(i)
                        onRecentClicked: (i) => { app.getArea = "recent"; app.getIndex = i; app.getAct("a") }
                        onItemClicked: (i) => { app.getArea = "grid"; app.getIndex = i; app.openDetails(app.getItems[i]) }
                        onFallbackClicked: (k) => { app.getArea = "fallback"; app.getAct(k) }
                    }
                    ModDetails {
                        id: modView
                        ui: kit
                        anchors.fill: parent
                        anchors.topMargin: app.px(4)
                        visible: app.page === "game" && app.modPage
                        info: app.detailsData
                        focused: app.zone === "main" && app.sheet === ""
                        mainLabel: app.mainLabel
                        mainLine: app.mainLine
                        mainProgress: app.busyHere && app.dl.active ? Math.max(0, app.dlPart) : -1
                        focusIndex: app.pageIndex
                        onInstallClicked: { app.pageIndex = 0; app.installFromDetails() }
                        onVersionsClicked: { app.pageIndex = 1; app.openVersions() }
                        onNeedClicked: (i) => { app.pageIndex = i + 2; app.openNeed(i) }
                    }
                    // Load order tab (Skyrim)
                    LoadOrderList {
                        ui: kit
                        anchors.fill: parent
                        anchors.topMargin: app.px(4)
                        visible: app.page === "game" && app.game !== null && app.tab === 2
                        plugins: app.order
                        error: app.orderError
                        messages: app.orderMessages
                        note: app.orderNote
                        focusIndex: app.orderIndex
                        moving: app.orderMoving
                        active: app.zone === "main" && app.sheet === ""
                        onSortClicked: { app.orderIndex = -1; Turbine.orderSort() }
                        onRowClicked: (i) => app.orderIndex = i
                        onToggleClicked: (i) => { app.orderIndex = i; app.orderAct("a") }
                    }

                    // Settings
                    Row {
                        visible: app.page === "settings"
                        anchors.fill: parent
                        spacing: app.px(14)
                        Column {
                            width: app.px(170)
                            spacing: app.px(4)
                            Repeater {
                                model: app.settingsSections
                                Rectangle {
                                    required property int index
                                    required property string modelData
                                    width: parent.width; height: app.px(40); radius: app.px(6)
                                    color: app.setSection === index ? c.raised : "transparent"
                                    Rectangle { visible: app.setSection === parent.index; width: app.px(3); height: parent.height; color: c.accent; radius: app.px(2) }
                                    Label { x: app.px(14); anchors.verticalCenter: parent.verticalCenter; text: parent.modelData; font.pixelSize: app.px(16); font.bold: app.setSection === parent.index; color: app.setSection === parent.index ? c.text : c.muted }
                                    HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                    TapHandler { onTapped: { app.setSection = parent.index; app.setRow = 0 } }
                                }
                            }
                        }
                        // Look
                        ListView {
                            id: lookList
                            width: parent.width - app.px(184)
                            height: parent.height
                            visible: app.sec === "Look"
                            clip: true
                            spacing: app.px(4)
                            model: app.lookRows
                            currentIndex: app.sec === "Look" ? app.setRow : 0
                            highlightFollowsCurrentItem: false
                            onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                            delegate: Rectangle {
                                id: srow
                                required property int index
                                required property var modelData
                                readonly property bool foc: app.zone === "settings" && app.setRow === index
                                readonly property var now: srow.modelData.now !== undefined ? srow.modelData.now : look[srow.modelData.key]
                                width: lookList.width; height: app.px(50); radius: app.px(6)
                                color: foc ? c.focusFill : c.card
                                border.color: c.accent
                                border.width: foc ? app.px(2) : 0
                                opacity: srow.modelData.dim ? 0.6 : 1
                                Label { x: app.px(14); width: app.px(140); anchors.verticalCenter: parent.verticalCenter; text: srow.modelData.label; font.pixelSize: app.px(16); font.bold: true }
                                Item {
                                    x: app.px(160); width: parent.width - app.px(174); height: parent.height
                                    // Choices
                                    Row {
                                        visible: !srow.modelData.swatch && !srow.modelData.slider
                                        anchors.verticalCenter: parent.verticalCenter
                                        width: parent.width
                                        spacing: app.px(4)
                                        Repeater {
                                            model: srow.modelData.names || []
                                            Rectangle {
                                                required property int index
                                                required property string modelData
                                                readonly property bool on: srow.now === srow.modelData.options[index]
                                                width: (parent.width - app.px(4) * ((srow.modelData.names || []).length - 1)) / (srow.modelData.names || [1]).length
                                                height: app.px(32); radius: app.px(5)
                                                color: on ? c.raised : c.ground
                                                Rectangle { visible: parent.on; anchors.bottom: parent.bottom; width: parent.width; height: app.px(2); color: c.accent }
                                                Label { anchors.centerIn: parent; text: parent.modelData; font.pixelSize: app.px(14); font.bold: parent.on; color: parent.on ? c.text : c.muted }
                                                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                                TapHandler {
                                                    onTapped: {
                                                        app.setRow = srow.index
                                                        const v = srow.modelData.options[parent.index]
                                                        if (srow.modelData.key === "layout") app.layoutPreview(v)
                                                        else if (srow.modelData.key === "season") { look.season = v; app.note("action", "Look: season " + v) }
                                                        else { look[srow.modelData.key] = v; app.note("action", "Look: " + srow.modelData.key + " " + v) }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    // Accent swatches
                                    Row {
                                        visible: srow.modelData.swatch === true
                                        anchors.verticalCenter: parent.verticalCenter
                                        spacing: app.px(10)
                                        Repeater {
                                            model: ["brass", "iron", "copper", "jade", "fjord", "heather", "ember"]
                                            Rectangle {
                                                required property string modelData
                                                width: app.px(24); height: width; radius: width / 2
                                                anchors.verticalCenter: parent.verticalCenter
                                                color: c.swatches[modelData]
                                                border.color: c.text
                                                border.width: look.accent === modelData ? app.px(2) : 0
                                                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                                TapHandler { onTapped: { look.accent = parent.modelData; app.note("action", "Look: accent " + look.accent) } }
                                            }
                                        }
                                        Label { text: look.accent.charAt(0).toUpperCase() + look.accent.substring(1); color: c.soft; font.pixelSize: app.px(14); anchors.verticalCenter: parent.verticalCenter }
                                    }
                                    // Sliders: Text size, Seasonal amount and size
                                    Row {
                                        visible: srow.modelData.slider === true
                                        anchors.verticalCenter: parent.verticalCenter
                                        width: parent.width
                                        spacing: app.px(12)
                                        Rectangle {
                                            id: track
                                            readonly property real part: srow.modelData.slider ? (srow.now - srow.modelData.min) / (srow.modelData.max - srow.modelData.min) : 0
                                            width: parent.width - app.px(70); height: app.px(6); radius: height / 2
                                            anchors.verticalCenter: parent.verticalCenter
                                            color: c.line
                                            Rectangle { width: parent.width * track.part; height: parent.height; radius: parent.radius; color: c.accent }
                                            Rectangle {
                                                x: parent.width * track.part - width / 2
                                                anchors.verticalCenter: parent.verticalCenter
                                                width: app.px(18); height: width; radius: width / 2; color: c.accentHi
                                            }
                                        }
                                        Label { text: srow.now + (srow.modelData.unit || ""); color: c.accentText; font.bold: true; anchors.verticalCenter: parent.verticalCenter }
                                    }
                                }
                                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                TapHandler { onTapped: app.setRow = srow.index }
                            }
                        }
                        // About: the app icon (ten presses turn Labs mode on or off), version, the scrolling test.
                        Column {
                            width: parent.width - app.px(184)
                            spacing: app.px(8)
                            visible: app.sec === "About"
                            Row {
                                spacing: app.px(14)
                                Rectangle {
                                    id: aboutIcon
                                    readonly property bool foc: app.zone === "settings" && app.setRow === 0
                                    width: app.px(76); height: width; radius: app.px(10)
                                    color: foc ? c.focusFill : "transparent"
                                    border.color: c.accent
                                    border.width: foc ? app.px(2) : 0
                                    Image {
                                        anchors.centerIn: parent
                                        width: app.px(64); height: width
                                        source: app.iconFull
                                        sourceSize.width: app.px(128)
                                        smooth: true
                                        mipmap: true
                                    }
                                    HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                    TapHandler { onTapped: { app.setRow = 0; app.labsTap() } }
                                }
                                Column {
                                    anchors.verticalCenter: parent.verticalCenter
                                    spacing: app.px(4)
                                    Label { text: "Turbine Mod Manager " + Turbine.version; font.pixelSize: app.px(19); font.weight: Font.DemiBold }
                                    Label { visible: app.labsOn; text: "Labs mode on"; color: c.accentText; font.pixelSize: app.px(15); font.bold: true }
                                }
                            }
                            Label { visible: !Turbine.pads; width: parent.width; wrapMode: Text.WordWrap; elide: Text.ElideNone; color: c.muted; font.pixelSize: app.px(14); text: "Using Steam's controller settings (no controller seen directly)." }
                            Label { width: parent.width; wrapMode: Text.WordWrap; elide: Text.ElideNone; color: c.soft; text: "A mod manager for SteamOS handhelds. Free software under the GNU General Public License, version 3 or later. Licenses of the parts it uses are in the licenses folder of the project." }
                            Rectangle {
                                readonly property bool foc: app.zone === "settings" && app.setRow === 1
                                width: perfRow.implicitWidth + app.px(28); height: app.px(40); radius: app.px(8)
                                color: foc ? c.focusFill : c.card
                                border.color: c.accent
                                border.width: foc ? app.px(2) : 0
                                Hint { id: perfRow; anchors.centerIn: parent; k: "a"; label: "Scrolling test (2,000 rows)" }
                                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                TapHandler { onTapped: { app.page = "perf"; app.zone = "perf" } }
                            }
                        }
                        // Labs: Report a bug, Performance check, and the checklist of things to confirm on the device.
                        ListView {
                            id: labsList
                            width: parent.width - app.px(184)
                            height: parent.height
                            visible: app.sec === "Getting mods" || app.sec === "Updates" || app.sec === "Labs"
                            clip: true
                            spacing: app.px(4)
                            model: app.rowsNow
                            currentIndex: visible ? app.setRow : 0
                            highlightFollowsCurrentItem: false
                            onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                            onVisibleChanged: if (visible) Qt.callLater(() => { positionViewAtBeginning(); positionViewAtIndex(currentIndex, ListView.Contain) })
                            delegate: Item {
                                id: lrow
                                required property int index
                                required property var modelData
                                readonly property bool foc: app.zone === "settings" && app.setRow === index
                                readonly property bool isItem: modelData.kind === "item"
                                readonly property bool hasChips: isItem || modelData.kind === "choice" || modelData.kind === "toggle"
                                readonly property string answer: isItem ? app.labsState(modelData.id) : ""
                                width: labsList.width
                                height: (modelData.head ? labsHead.height : 0) + box.height
                                Label {
                                    id: labsHead
                                    visible: !!lrow.modelData.head
                                    height: visible ? app.px(34) : 0
                                    x: app.px(6)
                                    verticalAlignment: Text.AlignBottom
                                    bottomPadding: app.px(6)
                                    text: (lrow.modelData.head || "") + (lrow.modelData.head && lrow.isItem ? " · " + app.labsDone : "")
                                    color: c.muted
                                    font.pixelSize: app.px(14)
                                    font.bold: true
                                    font.letterSpacing: app.px(1)
                                }
                                Rectangle {
                                    id: box
                                    y: labsHead.height
                                    width: parent.width
                                    height: Math.max(lrow.isItem ? app.px(66) : app.px(50), labsWords.implicitHeight + app.px(18))
                                    radius: app.px(6)
                                    color: lrow.foc ? c.focusFill : c.card
                                    border.color: c.accent
                                    border.width: lrow.foc ? app.px(2) : 0
                                    Column {
                                        id: labsWords
                                        x: app.px(14)
                                        anchors.verticalCenter: parent.verticalCenter
                                        width: parent.width - app.px(28) - (lrow.hasChips ? chips.width + app.px(10) : actionHint.width + app.px(10))
                                        spacing: app.px(2)
                                        Label { width: parent.width; text: lrow.modelData.title; font.pixelSize: app.px(16); font.bold: true }
                                        Label { width: parent.width; text: lrow.modelData.how; color: c.soft; font.pixelSize: app.px(14); wrapMode: Text.WordWrap; maximumLineCount: 2 }
                                    }
                                    Hint {
                                        id: actionHint
                                        visible: lrow.modelData.kind === "action"
                                        anchors.right: parent.right; anchors.rightMargin: app.px(12)
                                        anchors.verticalCenter: parent.verticalCenter
                                        k: lrow.foc ? "a" : ""
                                        label: lrow.modelData.action || ""
                                    }
                                    Row {
                                        id: chips
                                        visible: lrow.hasChips
                                        anchors.right: parent.right; anchors.rightMargin: app.px(10)
                                        anchors.verticalCenter: parent.verticalCenter
                                        spacing: app.px(4)
                                        Repeater {
                                            model: lrow.isItem ? [["works", "Works"], ["problem", "Problem"], ["", "Not tested"]]
                                                   : lrow.hasChips ? lrow.modelData.options.map((o, i) => [o, lrow.modelData.names[i]]) : []
                                            Rectangle {
                                                required property int index
                                                required property var modelData
                                                readonly property bool on: lrow.isItem ? lrow.answer === modelData[0] : lrow.modelData.value === modelData[0]
                                                readonly property bool cursor: lrow.isItem && lrow.foc && app.labsChip === index
                                                width: chipText.implicitWidth + app.px(18); height: app.px(32); radius: app.px(5)
                                                color: on ? c.raised : c.ground
                                                border.color: cursor ? c.text : on ? c.accent : c.line
                                                border.width: cursor ? app.px(2) : Math.max(1, app.px(1))
                                                Label {
                                                    id: chipText
                                                    anchors.centerIn: parent
                                                    text: parent.modelData[1]
                                                    font.pixelSize: app.px(14)
                                                    font.bold: parent.on
                                                    color: !parent.on ? c.muted : !lrow.isItem ? c.text : parent.index === 0 ? c.ok : parent.index === 1 ? c.stop : c.text
                                                }
                                                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                                TapHandler {
                                                    onTapped: {
                                                        app.setRow = lrow.index
                                                        if (lrow.isItem) { app.labsChip = parent.index; app.labsChoose(lrow.modelData, parent.modelData[0]) }
                                                        else if (!parent.on) app.rowStep(lrow.modelData, 1)
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                    TapHandler { onTapped: { app.setRow = lrow.index; if (lrow.modelData.kind === "action") app.rowRun(lrow.modelData) } }
                                }
                            }
                        }
                    }

                    // Scrolling test: 2,000 mod rows.
                    ListView {
                        id: perfList
                        visible: app.page === "perf"
                        anchors.fill: parent
                        clip: true
                        spacing: app.px(4)
                        model: 2000
                        currentIndex: app.perfIndex
                        highlightMoveDuration: 80
                        onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                        delegate: Rectangle {
                            id: prow
                            required property int index
                            readonly property bool foc: app.perfIndex === index
                            readonly property bool off: index % 7 === 3
                            width: perfList.width
                            height: app.px(52)
                            radius: app.px(6)
                            color: foc ? c.focusFill : c.card
                            border.color: c.accent
                            border.width: foc ? app.px(2) : 0
                            Row {
                                anchors.verticalCenter: parent.verticalCenter
                                x: app.px(12)
                                spacing: app.px(12)
                                Rectangle {
                                    width: app.px(38); height: app.px(22); radius: app.px(5)
                                    anchors.verticalCenter: parent.verticalCenter
                                    color: prow.off ? c.ground : c.raised
                                    Label { anchors.centerIn: parent; text: prow.off ? "OFF" : "ON"; font.pixelSize: app.px(14); font.bold: true; color: prow.off ? c.muted : c.accentText }
                                }
                                Column {
                                    anchors.verticalCenter: parent.verticalCenter
                                    Label { text: "Test mod " + (prow.index + 1); font.pixelSize: app.px(16); font.bold: true; color: prow.off ? c.muted : c.text }
                                    Label { text: "Row " + (prow.index + 1) + " of 2,000"; font.pixelSize: app.px(14); color: c.muted }
                                }
                            }
                        }
                    }
                }
            }

        }
    }

    // ---------------------------------------------------------------- sheets
    Rectangle {
        anchors.fill: parent
        visible: app.sheet === "details" || app.sheet === "notice"
        color: "#B0000000"
        TapHandler { onTapped: app.act("b") }

        // Game details: what Turbine found, so the owner can check detection on the device.
        Rectangle {
            visible: app.sheet === "details" && app.game !== null
            anchors.centerIn: parent
            width: Math.min(parent.width - app.px(40), app.px(600))
            height: detailCol.implicitHeight + app.px(32)
            radius: app.px(10)
            color: c.panel
            border.color: c.line
            Column {
                id: detailCol
                x: app.px(18); y: app.px(16)
                width: parent.width - app.px(36)
                spacing: app.px(8)
                Label { text: app.game ? app.game.name : ""; font.pixelSize: app.px(19); font.weight: Font.DemiBold; width: parent.width }
                Repeater {
                    model: app.game ? [
                        ["Store", app.game.storeLabel + (app.game.launcher !== "Steam" ? " (through Heroic)" : "")],
                        ["Store ID", app.game.storeId],
                        ["Runs as", app.runtimeWords(app.game)],
                        ["Stored on", app.game.drive],
                        ["Folder", app.game.installDir],
                        ["Version", app.game.build || "Not reported"],
                        ["Cover art", app.game.cover || app.game.header ? "Found on the device" : "None cached"]
                    ].concat(app.rules && app.rules.launchOption ? [
                        ["Loader setup", app.rules.launchHelp || ""],
                        ["Launch Option", app.rules.launchOption]
                    ] : []) : []
                    Row {
                        required property var modelData
                        width: detailCol.width
                        spacing: app.px(12)
                        Label { width: app.px(110); text: parent.modelData[0]; color: c.muted; font.pixelSize: app.px(14) }
                        Label { width: parent.width - app.px(122); text: parent.modelData[1]; wrapMode: Text.WrapAnywhere; elide: Text.ElideNone; font.pixelSize: app.px(15) }
                    }
                }
                Hint { k: "b"; label: "Close" }
            }
        }

        // Not SteamOS: said once, then Turbine runs as it is.
        Rectangle {
            visible: app.sheet === "notice"
            anchors.centerIn: parent
            width: Math.min(parent.width - app.px(40), app.px(520))
            height: noticeCol.implicitHeight + app.px(36)
            radius: app.px(10)
            color: c.panel
            border.color: c.line
            Column {
                id: noticeCol
                x: app.px(20); y: app.px(18)
                width: parent.width - app.px(40)
                spacing: app.px(10)
                Label { text: "Made for SteamOS"; font.pixelSize: app.px(19); font.weight: Font.DemiBold }
                Label { width: parent.width; wrapMode: Text.WordWrap; elide: Text.ElideNone; color: c.soft; text: "This system isn't supported. Turbine Mod Manager is built and tested for SteamOS handhelds only, so some things may not work here." }
                Hint { k: "a"; label: "OK" }
            }
        }
    }

    // ---------------------------------------------------------------- menus and prompts (they cover the window)
    ModMenu {
        ui: kit
        visible: app.sheet === "modmenu"
        title: app.menuMod ? app.menuMod.name : ""
        subtitle: !app.menuMod ? ""
                  : [app.modWords(app.menuMod), (app.menuMod.enabled ? "On" : "Off") + (app.ordered ? ", " + app.ordinal(app.mods.indexOf(app.menuMod) + 1) + " in load order" : "")].filter(s => s !== "").join(" · ")
        items: app.modMenuItems.map(m => ({ label: m.title, detail: m.detail, icon: m.icon, danger: m.danger === true }))
        focusIndex: app.menuIndex
        onActivated: (index) => { app.menuIndex = index; app.modMenuRun(index) }
        onClosed: app.act("b")
    }
    GameMenu {
        ui: kit
        visible: app.sheet === "gamemenu"
        gameName: app.gameTitle
        restoreDate: app.lastGood ? "From " + app.when(app.lastGood.savedAt) + " · " + app.plural(app.lastGood.modsOn, "mod", "mods") + " on" : ""
        restoreEmptyText: "Nothing saved yet. A setup is saved each time you play with mods."
        extraItems: app.gameMenuItems.slice(2).map(m => ({ key: m.act, label: m.title, detail: m.detail, icon: m.icon, danger: m.danger === true }))
        focusIndex: app.menuIndex
        onActivated: (index, key) => { app.menuIndex = index; app.gameMenuRun(index) }
        onClosed: app.act("b")
    }
    AddMods {
        ui: kit
        visible: app.sheet === "add"
        subtitle: "Zip files in your Downloads folder, newest first"
        files: app.downloads.map(d => ({ name: d.name, size: app.sizeWords(d.size), age: d.modified > 0 ? app.age(d.modified) : "" }))
        focusIndex: app.addIndex
        emptyText: "No zip files in Downloads yet. Download a mod's zip file in Desktop Mode, then come back."
        laterText: "Browsing and searching mods online comes in a later build."
        onActivated: (index) => { app.addIndex = index; app.addChosen() }
        onClosed: app.act("b")
    }
    PrePlayCheck {
        id: checkView
        ui: kit
        visible: app.sheet === "check"
        checks: app.checks
        buttons: app.checkChoices.map((label, i) => ({ key: label === "Play anyway" ? "a" : label === "Play without mods" ? (app.checkStops ? "a" : "x") : "b", label: label }))
        focusIndex: app.checkIndex
        note: app.checkStops ? "Play without mods starts the game with no mods. Nothing is deleted." : ""
        onActivated: (index, key) => { app.checkIndex = index; app.checkChoose(index) }
        onClosed: app.act("b")
    }
    LoaderPrompt {
        ui: kit
        visible: app.sheet === "loader"
        title: (app.loaderShort || "Mod loader") + " needed"
        loaderName: app.loaderShort
        loaderFullName: app.rules && app.rules.loader && app.rules.loader.name.indexOf(" (") > 0 ? app.rules.loader.name.split(" (")[1].replace(")", "") : ""
        explain: app.rules && app.rules.loader ? app.rules.loader.explain : ""
        neededBy: app.loaderFor
        sourceUrl: app.rules && app.rules.loader ? app.rules.loader.source : ""
        sourceShown: app.loaderSourceShown
        canDownload: app.loaderCanGet
        focusIndex: app.loaderIndex
        onActivated: (index, key) => { app.loaderIndex = index; app.act(key || "a") }
        onClosed: app.act("b")
    }

    // Installer choices: over the main area, the rail stays in view.
    FomodSheet {
        id: fomodView
        ui: kit
        visible: app.sheet === "fomod" && app.fomod !== null
        x: rail.width
        width: parent.width - rail.width
        height: parent.height - legend.height - app.px(16)
        info: app.fomod
        onChoose: (g, o) => { app.note("action", "Installer: question " + (g + 1) + ", option " + (o + 1)); Turbine.fomodChoose(g, o) }
        onNext: { app.note("action", "Installer: next step"); Turbine.fomodNext() }
        onBack: { app.note("action", "Installer: back a step"); Turbine.fomodBack() }
        onStop: { app.note("action", "Installer: stopped"); Turbine.fomodCancel() }
    }
    VersionsSheet {
        ui: kit
        visible: app.sheet === "versions"
        modName: app.detailsData && app.detailsData.name ? app.detailsData.name : ""
        versions: app.otherVersions
        focusIndex: app.versionIndex
        note: app.detailsData && app.detailsData.versionsNote ? app.detailsData.versionsNote : ""
        onActivated: (i) => { app.versionIndex = i; app.installVersion(i) }
        onClosed: app.act("b")
    }
    PagesSheet {
        ui: kit
        visible: app.sheet === "pages"
        title: app.pagesTitle
        pages: app.pagesList
        focusIndex: app.pagesIndex
        gameMode: Turbine.gamemode
        handlerSet: app.handler.state !== "no"
        onActivated: (i) => { app.pagesIndex = i; app.openPage(i) }
        onClosed: app.act("b")
    }
    CodeSheet {
        ui: kit
        visible: app.sheet === "code"
        info: app.codeData
        onCopyClicked: app.act("a")
        onClosed: app.act("b")
    }
    PluginSheet {
        ui: kit
        visible: app.sheet === "plugin" && app.orderPlugin !== null
        plugin: app.orderPlugin
        position: app.orderIndex + 1
        notes: app.orderPlugin ? (app.orderMessages[app.orderPlugin.name.toLowerCase()] || []) : []
        onClosed: app.act("b")
    }
    ProfilesSheet {
        id: profilesView
        ui: kit
        visible: app.sheet === "profiles"
        gameName: app.gameTitle
        profiles: app.profileList
        active: app.profileData ? app.profileData.active : ""
        actions: app.profileActions
        plan: app.importData
        premium: app.account.premium === true
        focusIndex: app.profIndex
        onActivated: (i) => { app.profIndex = i; app.profileRun(i) }
        onClosed: app.act("b")
    }
    AddMods {
        ui: kit
        visible: app.sheet === "shares"
        title: "Import a share file"
        subtitle: "Turbine profile files in your Downloads folder, newest first"
        files: app.shares.map(x => ({ name: x.name, size: "", age: "" }))
        focusIndex: app.shareIndex
        actionLabel: "Read"
        emptyText: "No Turbine profile files in Downloads. Someone makes one with Profiles, Export to a share file."
        laterText: ""
        onActivated: (index) => { app.shareIndex = index; app.act("a") }
        onClosed: app.act("b")
    }
    // The no-browser warning, the browser helper's steps, install notes, and "Add this file?".
    InfoSheet {
        ui: kit
        visible: ["nobrowser", "helper", "notes", "found"].indexOf(app.sheet) >= 0
        title: app.sheet === "nobrowser" ? (app.browser.title || "No web browser found")
               : app.sheet === "helper" ? "Set up the browser helper"
               : app.sheet === "found" ? "Found a download"
               : "Added " + app.notesName
        text: app.sheet === "nobrowser" ? (app.browser.text || "")
              : app.sheet === "helper" ? "Optional: mods download without it. It only adds installer choices on the Nexus page. Turbine put it in Downloads, in the folder Turbine browser helper. Then, once:"
              : app.sheet === "found" && app.found ? "Found " + app.found.name + " (" + app.sizeWords(app.found.size) + ")."
                    + (app.found.game ? " Add it to " + app.found.gameName + "?" : " Which game is it for?")
              : app.sheet === "notes" ? "Something to know about this mod:" : ""
        steps: app.sheet === "nobrowser" ? (app.browser.steps || [])
               : app.sheet === "helper" ? ["In Desktop Mode, open chrome://extensions in the browser.",
                                           "Turn on Developer mode (top right).",
                                           "Press Load unpacked.",
                                           "Choose Downloads, then the folder Turbine browser helper."]
               : []
        notes: app.sheet === "notes" ? app.notesList : []
        footnote: app.sheet === "helper" ? "Then press Pair: the pairing page opens in the browser and links the helper to Turbine."
                  : app.sheet === "found" && app.found ? (app.found.game ? "Why " + app.found.gameName + ": " + app.found.why + ". " : "")
                    + "The file stays in Downloads either way." + (app.found.more > 0 ? " " + app.plural(app.found.more, "more file waits", "more files wait") + " after this one." : "")
                  : ""
        buttons: app.infoButtons
        focusIndex: app.infoIndex
        onActivated: (i) => { app.infoIndex = i; app.infoRun(i) }
        onClosed: app.act("b")
    }
    // Removed mods (A Put back, X Delete now) and the hold-to-confirm sheet for deleting for good.
    RemovedSheet {
        ui: kit
        visible: app.sheet === "removed"
        mods: app.removedMods
        focusIndex: app.removedIndex
        now: app.today
        onActivated: (i) => { app.removedIndex = i; app.removedRun(i) }
        onDeleteClicked: (i) => { app.removedIndex = i; app.removedDelete(i) }
        onClosed: app.act("b")
    }
    InfoSheet {
        ui: kit
        visible: app.sheet === "confirm"
        title: "Delete for good?"
        text: app.confirmText
        buttons: [{ key: "a", label: confirmHold.running ? "Keep holding A: " + Math.max(1, Math.ceil(app.holdNeeded - app.confirmHeld)) + "..." : "Hold to delete" },
                  { key: "b", label: "Keep" }]
        focusIndex: 0
        onActivated: (i) => { if (i === 0) app.startConfirm(); else app.act("b") }
        onClosed: app.act("b")
    }
    // "Didn't see Nexus open?": a choice, not a failure (info look).
    InfoSheet {
        ui: kit
        visible: app.sheet === "openhelp"
        title: "Didn't see Nexus open?"
        text: "Turbine started " + app.openName + " for you. If it isn't showing, Steam's own browser opens the same page. Turbine still adds your download by itself."
        buttons: [{ key: "a", label: "Open in Steam's browser" }, { key: "b", label: "Close" }]
        focusIndex: 0
        onActivated: (i) => app.act(i === 0 ? "a" : "b")
        onClosed: app.act("b")
    }
    // X on a Get mods card.
    ModMenu {
        ui: kit
        visible: app.sheet === "cardmenu"
        title: app.cardItem ? app.cardItem.name : ""
        subtitle: app.cardItem ? (app.cardItem.owner || "") : ""
        items: app.cardMenuItems.map(m => ({ label: m.title, detail: m.detail, icon: m.icon, danger: false }))
        focusIndex: app.menuIndex
        onActivated: (index) => { app.menuIndex = index; app.cardMenuRun(index) }
        onClosed: app.act("b")
    }
    // Keep this size? Drawn at a fixed safe scale, so it is always readable whatever size is being tried.
    Item {
        id: keepSheet
        anchors.fill: parent
        visible: app.sheet === "keepsize"
        z: 60
        readonly property real k: Math.max(0.5, Math.min(app.width / 883, app.height / 514))
        function kp(v) { return Math.round(v * k) }
        Rectangle {
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.bottom: parent.bottom
            anchors.bottomMargin: keepSheet.kp(70)
            width: keepCol.implicitWidth + keepSheet.kp(40)
            height: keepCol.implicitHeight + keepSheet.kp(28)
            radius: keepSheet.kp(10)
            color: c.panel
            border.color: c.accent
            border.width: Math.max(1, keepSheet.kp(2))
            Column {
                id: keepCol
                anchors.centerIn: parent
                spacing: keepSheet.kp(10)
                Text { text: "Keep this size?"; color: c.text; font.family: app.bodyFont; font.pixelSize: keepSheet.kp(20); font.weight: Font.DemiBold }
                Text { text: "Going back in " + app.keepLeft + " · Interface size " + app.sizeNow + "% · " + (app.layoutNow === "tv" ? "TV" : app.layoutNow.charAt(0).toUpperCase() + app.layoutNow.substring(1)); color: c.soft; font.family: app.bodyFont; font.pixelSize: keepSheet.kp(15) }
                Row {
                    spacing: keepSheet.kp(18)
                    Repeater {
                        model: [["a", "Keep"], ["b", "Go back"]]
                        Row {
                            required property var modelData
                            spacing: keepSheet.kp(8)
                            Rectangle {
                                width: keepSheet.kp(26); height: width; radius: width / 2
                                color: c.raised; border.color: c.accentLo
                                Text { anchors.centerIn: parent; text: parent.parent.modelData[0].toUpperCase(); color: c.text; font.family: app.bodyFont; font.pixelSize: keepSheet.kp(14); font.bold: true }
                            }
                            Text { text: parent.modelData[1]; color: c.text; font.family: app.bodyFont; font.pixelSize: keepSheet.kp(16); font.bold: true; anchors.verticalCenter: parent.verticalCenter }
                            TapHandler { onTapped: app.keepSize(parent.modelData[0] === "a") }
                        }
                    }
                }
            }
        }
    }
    // The focused game's name beside the rail.
    Rectangle {
        id: railLabel
        readonly property Item tileNow: app.zone === "rail" && app.railIndex < app.games.length ? railList.itemAtIndex(app.railIndex) : null
        visible: tileNow !== null && app.sheet === ""
        z: 20
        x: rail.width + app.px(4)
        y: tileNow ? tileNow.mapToItem(app.contentItem, 0, 0).y + (tileNow.height - height) / 2 + railList.contentY * 0 : 0
        width: railLabelText.implicitWidth + app.px(24)
        height: app.px(32)
        radius: app.px(6)
        color: c.raised
        border.color: c.accent
        border.width: Math.max(1, app.px(1))
        Text { id: railLabelText; anchors.centerIn: parent; text: app.games[app.railIndex] ? app.games[app.railIndex].name : ""; color: c.text; font.family: app.bodyFont; font.pixelSize: app.px(15); font.weight: Font.DemiBold }
    }
    // The status card over other apps (see showOverlay).
    StatusOverlay {
        id: panelWin
        ui: kit
        hud: false
        status: shot.on ? app.status : app.transfer
        fomod: app.fomod
        onConfirm: (yes) => { if (app.status.ask) Turbine.watchConfirm(app.status.ask, yes); app.found = null; if (app.sheet === "found" || app.sheet === "pickgame") app.sheet = "" }
        // Picking a game needs the list: Turbine comes to the front with it.
        onPick: { app.raise(); app.requestActivate(); if (app.found) app.openPickGame() }
        onChoose: (g, o) => Turbine.fomodChoose(g, o)
        onNext: Turbine.fomodNext()
        onBack: Turbine.fomodBack()
        onStop: Turbine.fomodCancel()
        onOpenTurbine: { app.raise(); app.requestActivate() }
    }
    StatusOverlay {
        id: hudWin
        objectName: "turbineHud"
        ui: kit
        hud: true
        status: shot.on ? app.status : app.transfer
        fomod: app.fomod
    }
    // X on "Found <file>": which game it goes into.
    GamePicker {
        ui: kit
        visible: app.sheet === "pickgame"
        fileName: app.found ? app.found.name : ""
        games: app.games
        focusIndex: app.pickIndex
        onActivated: (i) => { app.pickIndex = i; app.pickGame(i) }
        onClosed: app.act("b")
    }
    AskText {
        id: askView
        ui: kit
        onAccepted: (text) => app.askDone(text)
        onCancelled: app.askCancelled()
    }

    PerfCheck {
        id: perfView
        ui: kit
        win: app
        onCopyClicked: (text) => { Turbine.copyText(text); toast.showPlain("Performance check copied.") }
        onReportClicked: (text) => app.openReport(null, "Performance check", text)
        onCloseClicked: { perfView.close(); app.sheet = "" }
        Component.onCompleted: if (shot.page === "perfcheck") reportText = "Turbine performance check\nTurbine: 0.2 (build sample)\nVerdict: Smooth\n"
            + "Still: 60 fps, median 16.7 ms, worst 18.9 ms, spikes 0.0%, CPU 9%\nScrolling: 59 fps, median 16.7 ms, worst 24.0 ms, spikes 0.4%, CPU 31%\n"
            + "Idle: 0 frames in 4.0 s, CPU 0.2%\n\nStartup: first frame 640 ms after start\nDrawing: Vulkan, software drawing no"
    }
    LabsReport {
        id: reportView
        ui: kit
        onSendClicked: app.sendReport()
        onCloseClicked: app.dismissReport()
        onEditStarted: app.reportEditing(true)
        onEditEnded: app.reportEditing(false)
    }

    // Legend: built from the same rules act() follows, so what it says is what the buttons do. The focused item
    // shows its own A and X; the legend lists the rest. Declared after the menus so their backdrop never dims it.
    readonly property var legendHints: {
        const s = sheet
        if (moving) return [["▲▼", "Move"], ["◀▶", "10 rows"], ["lt+rt", "Top or bottom"], ["a", "Done"], ["b", "Cancel"]]
        if (orderMoving) return [["▲▼", "Move"], ["a", "Done"], ["b", "Cancel"]]
        if (s === "modmenu" || s === "gamemenu" || s === "cardmenu") return [["▲▼", "Choose"], ["a", "Choose"], ["b", "Close"]]
        if (s === "add") return [["▲▼", "Choose"], ["a", "Add"], ["b", "Close"]]
        if (s === "shares") return [["▲▼", "Choose"], ["a", "Read"], ["b", "Back"]]
        if (s === "profiles") return [["▲▼", "Choose"], ["a", "Choose"], ["b", "Close"]]
        if (s === "check") return (checkStops ? [["a", "Play without mods"]] : [["a", "Play anyway"], ["x", "Play without mods"], ["start", "Play"]])
            .concat(checkView.scrolls ? [["▲▼", "Scroll"]] : []).concat([["b", "Not now"]])
        if (s === "loader") return [["a", loaderCanGet ? "Get it" : "Where it comes from"], ["b", "Not now"]]
        if (s === "pages") return [["▲▼", "Choose"], ["a", "Open"], ["b", "Close"]]
        if (s === "versions") return [["▲▼", "Choose"], ["a", detailsData && detailsData.premium === false ? "Get it on Nexus" : "Install"], ["b", "Close"]]
        if (s === "code") return (codeData && codeData.state === "ready" ? [["a", "Copy"]] : []).concat([["b", "Back"]])
        if (s === "plugin" || s === "details" || s === "notice") return [["b", "Close"]]
        if (s === "pickgame") return [["▲▼", "Choose"], ["a", "Add here"], ["b", "Back"]]
        if (s === "removed") return [["▲▼", "Choose"], ["a", removedIndex === 0 ? "Empty" : "Put back"]].concat(removedIndex > 0 ? [["x", "Delete now"]] : []).concat([["b", "Close"]])
        if (s === "confirm") return [["a", "Hold to delete"], ["b", "Keep"]]
        if (s === "keepsize") return [["a", "Keep"], ["b", "Go back"]]
        if (s === "openhelp") return [["a", "Open in Steam's browser"], ["b", "Close"]]
        if (s === "nobrowser" || s === "helper") return [["◀▶", "Choose"], ["a", "Choose"], ["b", "Close"]]
        if (s === "found") return [["◀▶", "Choose"], ["x", found && found.game ? "Other game" : "Pick the game"], ["b", "Not a mod"]]
        if (s === "notes") return [["a", "OK"]]
        if (s === "fomod") return [["a", "Choose"], ["start", "Next"], ["b", "Back a step"]]
        if (s === "ask") return [["a", "Done"], ["b", "Cancel"]].concat(Turbine.gamemode ? [["x", "Keyboard"]] : []).concat([["y", "Clear"]])
        if (s === "perf") return [["◀▶", "Choose"], ["b", "Close"]]
        if (s === "report") return reportView.editing ? [["b", "Done typing"]] : [["▲▼", "Move"], ["b", "Close"]]
        if (s !== "") return [["b", "Close"]]
        if (zone === "rail") return [["a", "Open"], ["b", "Back"]].concat(railIndex < games.length ? [["x", "Game menu"]] : []).concat([["lb+rb", "Switch game"]])
        if (page === "settings") return [["lt+rt", "Sections"]]
            .concat(sec === "Look" || (!!rowsNow[setRow] && ["choice", "toggle"].indexOf(rowsNow[setRow].kind) >= 0) ? [["◀▶", "Change"]] : [])
            .concat(!!rowsNow[setRow] && rowsNow[setRow].kind === "item" ? [["◀▶", "Choose"]] : [])
            .concat([["b", "Back to the game"], ["◀", "Game list"]])
        if (page === "perf") return [["▲▼", "Scroll"], ["b", "Back"]]
        if (!game) return [["a", "Look again"]]
        if (modPage) return [["b", "Back to the results"], ["lb+rb", "Prev / next mod"], ["▲▼", "Scroll"], ["view", "Game menu"]]
        let h = []
        if (tab === 0) {
            if (undoShown) h.push(["y", "Undo"])
            else if (ordered && focusedItem && focusedItem.kind === "mod" && mods.length > 1) h.push(["y", "Reorder"])
            h.push(["b", "Game list"])
        } else if (tab === 1 && sources.length > 0) {
            if (getArea === "fallback") h = [["b", "My mods"]]
            else if (getArea === "grid") h = [["y", "Get it"], ["b", "Back"]]
            else if (getArea === "bar" && getBar[getIndex] && ["sort", "category", "source"].indexOf(getBar[getIndex].key) >= 0) h = [["a", "Change"], ["b", getText !== "" ? "Clear search" : "My mods"]]
            else h = [["a", "Choose"], ["b", getText !== "" ? "Clear search" : "My mods"]]
            if (getArea !== "fallback") h.push(["r3", "Search"])
        } else if (tab === 1) h = [["b", "My mods"]]
        else if (tab === 2) h = (orderIndex >= 0 ? [["y", "Move"], ["x", "Details"]] : []).concat([["b", "My mods"]])
        return h.concat([["lb+rb", "Switch game"], ["lt+rt", "Tabs"], ["view", "Game menu"]])
    }
    Rectangle {
        id: legend
        anchors.bottom: parent.bottom
        anchors.bottomMargin: app.px(8)
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: rail.width + app.px(14)
        anchors.rightMargin: app.px(14)
        height: app.px(36)
        radius: app.px(6)
        color: c.panel
        z: 40
        Row {
            id: legendRow
            anchors.verticalCenter: parent.verticalCenter
            x: app.px(12)
            spacing: app.px(16)
            Repeater {
                model: app.legendHints
                Hint {
                    required property var modelData
                    k: modelData[0]
                    label: modelData[1]
                    visible: x + width < legend.width - pointerHints.width - app.px(30)
                }
            }
        }
        // The controller pointer (Steam's desktop layout), only while it shows: RT clicks, the left stick scrolls.
        Row {
            id: pointerHints
            anchors.right: parent.right
            anchors.rightMargin: app.px(12)
            anchors.verticalCenter: parent.verticalCenter
            spacing: app.px(16)
            visible: Turbine.pointerShown && !app.keyboardUp
            width: visible ? implicitWidth : 0
            Hint { k: "rt"; label: "Click" }
            Hint { k: "ls"; label: "Scroll" }
        }
    }

    // ---------------------------------------------------------------- toast
    // Something that just happened, in one of three looks: info, done, problem (real failures only). Never words
    // already on screen; the same words aren't shown twice in a row. After Uninstall: "Removed: name · Kept 7 days ·
    // Y Undo".
    property string undoId: ""
    readonly property bool undoShown: toast.shown && undoId !== "" && sheet === ""
    property string lastToast: ""
    property real lastToastAt: 0
    Toast {
        id: toast
        ui: kit
        function put(kind, m) {
            if (m === app.lastToast && Date.now() - app.lastToastAt < 6000 && shown) return
            app.lastToast = m; app.lastToastAt = Date.now()
            app.undoId = ""
            toast.kind = kind
            show(m)
        }
        function info(m) { put("info", m) }
        function done(m) { put("done", m) }
        function problem(m) { put("problem", m) }
        function showUndo(id, name) { app.undoId = id; toast.kind = "info"; show("Removed:", name, "Kept 7 days", "y", "Undo") }
        function showPlain(m) { put("info", m) }
        timeout: app.undoId !== "" ? 7000 : 4000
        onActionClicked: if (app.undoId !== "") app.putBack(app.undoId)
        onHidden: app.undoId = ""
    }
    // Labs: the brass ring that fills around the rail icon while View (or the icon) is held for a report.
    Shape {
        id: holdRing
        visible: app.holdShown
        readonly property real r: app.px(28)
        x: rail.width / 2 - r - app.px(3)
        y: app.px(10) + app.px(22) - r - app.px(3)
        width: (r + app.px(3)) * 2; height: width
        preferredRendererType: Shape.CurveRenderer
        ShapePath {
            strokeColor: c.line
            strokeWidth: app.px(4)
            fillColor: "transparent"
            PathAngleArc { centerX: holdRing.width / 2; centerY: holdRing.height / 2; radiusX: holdRing.r; radiusY: holdRing.r; startAngle: 0; sweepAngle: 360 }
        }
        ShapePath {
            strokeColor: c.accentHi
            strokeWidth: app.px(4)
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            PathAngleArc { centerX: holdRing.width / 2; centerY: holdRing.height / 2; radiusX: holdRing.r; radiusY: holdRing.r; startAngle: -90; sweepAngle: 360 * Math.min(1, app.holdMs / 7000) }
        }
    }

    // ---------------------------------------------------------------- the controller pointer
    // Steam's desktop layout: the right stick moves this pointer, RT clicks (held, it drags), LT right-clicks, the
    // left stick turned in a circle is a scroll wheel. Turbine sends real mouse and wheel events into this window
    // (backend.rs, qtglue.cpp), so everything that takes a tap or a mouse also takes the pointer; this draws it, over
    // everything (sheets included), and outlines the button or row under it. It hides 3 seconds after the right
    // stick stops; A and the D-pad work as always.
    property Item hoverItem: null
    property rect hoverBox: Qt.rect(0, 0, 0, 0)
    // Everything under the pointer that takes a tap; the outline goes on the innermost one (a row's ON pill
    // rather than the row). With a sheet open, the page under it doesn't count.
    property var hoverList: []
    function hover(item, on) {
        let list = hoverList.filter(x => !!x && x !== item)
        if (on) list.push(item)
        hoverList = list
        pickHover()
    }
    function inside(item, top) {
        for (let p = item; p; p = p.parent) if (p === top) return true
        return false
    }
    function pickHover() {
        const live = hoverList.filter(x => !!x && x.visible && !(sheet !== "" && inside(x, mainRow)))
        let best = null
        for (const x of live) if (!live.some(y => y !== x && inside(y, x))) { best = x; break }
        hoverItem = best
        placeHover()
    }
    function placeHover() {
        try {
            if (!hoverItem || !hoverItem.visible) { hoverBox = Qt.rect(0, 0, 0, 0); return }
            const p = hoverItem.mapToItem(null, 0, 0)
            hoverBox = Qt.rect(p.x, p.y, hoverItem.width, hoverItem.height)
        } catch (e) {
            hoverBox = Qt.rect(0, 0, 0, 0)
        }
    }
    Connections {
        target: Turbine
        function onPointerXChanged() { app.placeHover() }
        function onPointerYChanged() { app.placeHover() }
        function onPointerShownChanged() { app.note("pad", Turbine.pointerShown ? "Pointer showing" : "Pointer hidden") }
    }
    property int pointerTries: 0
    function pointerCheck() {
        const second = modList.itemAtIndex(2), first = modList.itemAtIndex(1)
        const it = second && second.kind === "mod" ? second : first && first.kind === "mod" ? first : null
        if (!it || !it.pill) {
            // The test mods may still be going in: look again shortly.
            if (++pointerTries < 6) pointerRetry.start(); else note("pad", "Pointer check: no mod row to click")
            return
        }
        const p = it.pill.mapToItem(null, it.pill.width / 2, it.pill.height / 2)
        Turbine.selftestPointer(p.x, p.y, true, 0)
    }
    Timer { id: pointerRetry; interval: 300; onTriggered: app.pointerCheck() }
    function pointerScrollCheck() {
        const p = perfList.mapToItem(null, perfList.width / 2, perfList.height / 2)
        Turbine.selftestPointer(p.x, p.y, false, 12)
        scrollCheckNote.start()
    }
    // The build's picture of the end of the check before Play: wheel steps over it.
    Timer { id: checkEndShot; interval: 900; onTriggered: Turbine.selftestPointer(app.width / 2, app.height * 0.45, false, 40) }
    Timer { id: scrollCheckNote; interval: 1500; onTriggered: app.note("pad", "Scroll check: the list moved " + Math.round(perfList.contentY) + " px") }
    Item {
        id: pointerLayer
        anchors.fill: parent
        z: 10000
        visible: Turbine.pointerShown
        enabled: false
        Rectangle {
            visible: app.hoverItem !== null && app.hoverBox.width > 0
            x: app.hoverBox.x - app.px(2); y: app.hoverBox.y - app.px(2)
            width: app.hoverBox.width + app.px(4); height: app.hoverBox.height + app.px(4)
            radius: app.px(7)
            color: Qt.rgba(c.accent.r, c.accent.g, c.accent.b, 0.10)
            border.width: Math.max(1, app.px(1.5))
            border.color: c.accentHi
        }
        // The arrow: brass with a dark edge, its tip on the spot that gets clicked, about 24 design pixels tall.
        Shape {
            x: Turbine.pointerX
            y: Turbine.pointerY
            width: app.px(24); height: app.px(24)
            preferredRendererType: Shape.CurveRenderer
            ShapePath {
                strokeColor: "#0B0A08"
                strokeWidth: Math.max(1.5, app.px(1.6))
                joinStyle: ShapePath.RoundJoin
                fillColor: "#D8AE5E"
                scale: Qt.size(app.px(24) / 24, app.px(24) / 24)
                PathSvg { path: "M1 1 L1 20 L6 15.5 L9.6 23 L13 21.4 L9.5 14 L16 14 Z" }
            }
        }
    }
}
