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
        property int textSize: 100          // percent
        property bool highContrast: false
        property string vision: "typical"   // typical, red, green, blue, none
        property string layout: "handheld"  // handheld, tv
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
        readonly property bool slow: ["mods", "menu", "gamemenu", "check", "add", "loader", "report", "browse", "moddetails", "fomod",
                                      "loadorder", "profiles", "accounts", "updates", "lootsort", "lootnotes", "versions", "code",
                                      "nexusdetails", "nexusversions", "nexusfree", "nxm", "browser", "helper"].indexOf(page) >= 0
        // Pages that show Labs mode (it isn't saved from a picture).
        readonly property bool labs: ["about", "labs", "report", "perfcheck"].indexOf(page) >= 0
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
            else if (page === "accounts") app.openSection("Accounts")
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
                app.openSection("Accounts")
                if (page === "browser") app.setRow = app.accountRows.findIndex(r => r.key === "browser")
                else if (page === "helper") { app.helperFolder = "/home/deck/Downloads/Turbine browser helper"; app.infoIndex = 0; app.sheet = "helper" }
                else app.noBrowser()
            }
            else if (page === "overlay" || page === "overlaygm") {
                app.status = { active: true, text: "Added to Cyberpunk 2077", level: "ok", progress: -1, game: "Cyberpunk 2077",
                               added: "Cyber Engine Tweaks 1.32.0", done: true, ask: "" }
                app.showOverlay(true, page === "overlaygm")
            }
            else if (page === "updates") { app.openSection("Updates"); Turbine.selfCheck(false, false) }
            else if (page === "perfcheck") app.openPerf()
            else if (page === "settings" && arg("season") !== "") app.setRow = 6
        }
    }
    Timer {
        running: shot.on && shot.slow
        interval: 2500
        onTriggered: {
            if (shot.page === "menu" && app.menuMod) app.sheet = "modmenu"
            else if (shot.page === "check") app.startPlay()
            else if (shot.page === "moddetails" && app.browseData && app.browseData.items && app.browseData.items.length > 0) app.openDetails(app.browseData.items[0])
            else if (shot.page === "versions" && app.browseData && app.browseData.items && app.browseData.items.length > 0)
                app.openDetails(app.browseData.items.find(x => x.id === "x753-More_Suits") || app.browseData.items[0])
            else if ((shot.page === "nexusdetails" || shot.page === "nexusversions" || shot.page === "nexusfree") && app.getItems.length > 0) app.openDetails(app.getItems[0])
            else if (shot.page === "code") { app.openProfiles(); Turbine.profileCode("Default"); app.sheet = "code" }
            else if (shot.page === "profiles") app.openProfiles()
            else if (shot.page === "lootsort" || shot.page === "lootnotes") { app.orderIndex = -1; Turbine.orderSort() }
            // After the first frames, so the screenshot inside the report has something in it.
            else if (shot.page === "report") app.openReport(app.labsItems[6], "Undo put the mod back at the bottom of the list instead of where it was.", "")
        }
    }
    Timer {
        running: shot.on
        interval: shot.page.indexOf("loot") === 0 || shot.page.indexOf("nxm") === 0 ? 6500 : shot.slow ? 4500 : 1500
        // The overlay pictures are of the overlay window itself.
        onTriggered: (shot.page === "overlay" ? panelWin : shot.page === "overlaygm" ? hudWin : app).contentItem.grabToImage(function (r) { r.saveToFile(shot.path); Qt.quit() })
    }

    // ---------------------------------------------------------------- size
    readonly property bool tv: look.layout === "tv"
    readonly property real fit: Math.min(3, Math.max(1, Math.min(width / 883, height / 514))) * (tv ? 1.5 : 1)
    readonly property real s: fit * look.textSize / 100
    function px(v) { return Math.round(v * s) }

    // ---------------------------------------------------------------- Labs mode and the activity log
    readonly property bool labsOn: labsSet.on || (shot.on && shot.labs)
    function note(kind, text) { Turbine.log(kind, text) }
    onPageChanged: note("page", "Page " + page)
    onTabChanged: {
        note("page", "Tab " + (tabs[tab] || tab))
        modPage = false
        if (tab === 1 && !browseData) browseNow()
        else if (tab === 2) { orderIndex = 0; Turbine.loadOrder() }
    }
    onZoneChanged: note("page", "Focus zone " + zone)
    onSetSectionChanged: {
        note("page", "Settings section " + (settingsSections[setSection] || setSection))
        if (settingsSections[setSection] === "Accounts") { Turbine.handlerStatus(); if (!shot.on) Turbine.bridgeRefresh() }
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
        readonly property string titleFont: app.titleFont
        readonly property string bodyFont: app.bodyFont
        function px(v) { return app.px(v) }
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
    readonly property var tabs: rules && rules.plugins ? ["Mods", "Get Mods", "Load order"] : ["Mods", "Get Mods"]
    // The Mods tab lists the mods, then Recently removed.
    readonly property var listItems: mods.map(m => ({ kind: "mod", m: m })).concat(removedMods.map(m => ({ kind: "removed", m: m })))
    readonly property var focusedItem: listItems.length > 0 ? listItems[Math.min(modIndex, listItems.length - 1)] : null

    function parse(text, fallback) {
        if (!text) return fallback
        try { return JSON.parse(text) } catch (e) { return fallback }
    }
    Connections {
        target: Turbine
        function onGamesChanged() {
            let list = app.parse(Turbine.games, [])
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
        function onAsked(json) { app.found = app.parse(json, null); if (app.found && app.sheet === "") { app.infoIndex = 0; app.sheet = "found" } }
        function onPad(button, pressed) { pressed ? app.press(button) : app.release(button) }
        // 0.3
        function onBrowseJsonChanged() {
            const v = app.parse(Turbine.browseJson, null)
            if (!v || !app.game || v.game !== app.game.id) return
            app.browseData = v
            app.browseLoading = false
            if (app.getArea === "grid") app.getIndex = Math.max(0, Math.min(app.getIndex, (v.items || []).length - 1))
        }
        function onDetailsJsonChanged() {
            app.detailsData = app.parse(Turbine.detailsJson, null)
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
        function onSelfJsonChanged() {
            app.selfUpd = app.parse(Turbine.selfJson, { state: "idle" })
            if (app.selfUpd.state === "available" && app.selfAuto) {
                app.selfAuto = false
                toast.showPlain("Turbine " + app.selfUpd.version + " is ready to install: Settings, Updates.")
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
        if (page === "perf") return { level: "ok", words: "2,000 rows", detail: "Hold the D-pad or stick to scroll fast." }
        if (!game) return { level: Turbine.problems !== "" ? "warn" : "ok", words: "No games found yet", detail: "" }
        if (dl.active) return { level: "busy", words: "Downloading " + dl.label, detail: "", progress: dlPart >= 0 ? dlPart : 0,
                                 progressText: dl.total > 0 ? Math.round(dlPart * 100) + "% · " + sizeWords(dl.got) + " of " + sizeWords(dl.total) : sizeWords(dl.got) }
        if (msgShown) return { level: ({ work: "busy", ok: "ok", warn: "warn", error: "stop" })[msgLevel] || "ok", words: msgText, detail: "" }
        if (Turbine.pending > 0)
            return { level: "busy", words: plural(Turbine.pending, "change", "changes") + " waiting", detail: "They go into the game in a few seconds" }
        if (needLoader.length > 0)
            return { level: "warn", words: plural(needLoader.length, "mod needs ", "mods need ") + loaderShort, detail: "The game still plays, without " + (needLoader.length === 1 ? "that one" : "those") }
        const on = mods.filter(m => m.enabled).length
        const found = game.launcher === "Steam" ? "Found in Steam" : "Found in Heroic (" + game.storeLabel + ")"
        return { level: "ok", words: "Ready to play", detail: !modsLoaded ? found : mods.length === 0 ? "No mods yet · " + found : on + " of " + mods.length + " mods on",
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
        modIndex = to
        Turbine.moveTo(m.id, to)
        edited()
    }
    function uninstall(m) {
        note("action", "Uninstall " + m.name)
        mods = mods.filter(x => x.id !== m.id)
        Turbine.removeMod(m.id)
        edited()
        modIndex = Math.max(0, Math.min(modIndex, mods.length - 1))
        toast.showUndo(m.id, m.name)
    }
    function putBack(id) {
        note("action", "Put back " + id)
        Turbine.undoRemove(id)
        toast.hide()
        edited()
    }
    // Reorder mode (Y): the D-pad moves the mod, A, B or Y puts it down.
    property bool moving: false
    property int moveFrom: -1
    function startMove() {
        if (!ordered || !focusedItem || focusedItem.kind !== "mod") return
        moving = true
        moveFrom = modIndex
    }
    function stepMove(d) {
        const to = modIndex + d
        if (to < 0 || to >= mods.length) return
        let list = mods.slice()
        const m = list[modIndex]
        list[modIndex] = list[to]
        list[to] = m
        mods = list
        modIndex = to
    }
    function endMove() {
        moving = false
        if (modIndex !== moveFrom && mods[modIndex]) {
            note("action", "Reorder " + mods[modIndex].name + " from " + (moveFrom + 1) + " to " + (modIndex + 1))
            Turbine.moveTo(mods[modIndex].id, modIndex)
            edited()
        }
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
        if (trouble || shot.page === "check") { checkIndex = 0; sheet = "check" }
        else Turbine.play(game.id)
    }
    readonly property bool checkStops: checks.some(x => x.level === "stop")
    readonly property var checkChoices: checkStops ? ["Play original", "Wait"] : ["Play anyway", "Play original", "Wait"]
    property int checkIndex: 0
    function checkChoose(i) {
        const what = checkChoices[i]
        note("action", "Check before Play: " + what)
        sheet = ""
        if (checks.some(x => x.title === launchTitle) && what !== "Wait") {
            let done = parse(seen.launchDone, []); done.push(game.id); seen.launchDone = JSON.stringify(done)
        }
        // Other read-once lines (REDmod): seen once the player moves on from the check.
        const once = checks.filter(x => x.once && x.title !== launchTitle)
        if (once.length > 0 && what !== "Wait" && !shot.on) {
            let done = parse(seen.onceDone, []); for (const x of once) done.push(game.id + "|" + x.title); seen.onceDone = JSON.stringify(done)
        }
        if (what === "Play anyway") Turbine.play(game.id)
        else if (what === "Play original") Turbine.playOriginal()
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
        { icon: "play", title: "Play original", detail: "Starts the game with no mods. Nothing is deleted.", ok: true },
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
    readonly property var modMenuItems: !menuMod ? [] : [
        { icon: "power", title: menuMod.enabled ? "Turn off" : "Turn on", detail: menuMod.enabled ? "Stays installed" : "Goes into the game", act: "toggle" }
    ].concat(updateMap[menuMod.id] ? [
        { icon: "update", title: "Update to " + updateMap[menuMod.id].to, detail: (updateMap[menuMod.id].from ? "You have " + updateMap[menuMod.id].from + ". " : "") + "Keeps its place and on or off", act: "update" }
    ] : []).concat(ordered ? [
        { icon: "move", title: "Move to top", detail: "First in the load order. Mods below it win when files overlap.", act: "top" },
        { icon: "move", title: "Move to bottom", detail: "Last in the load order. It wins when files overlap.", act: "bottom" }
    ] : []).concat([
        { icon: "remove", title: "Uninstall", detail: "Kept in Recently removed for 7 days", act: "remove", danger: true }
    ])
    property int menuIndex: 0
    function modMenuRun(i) {
        const item = modMenuItems[i], m = menuMod
        if (!item || !m) return
        note("action", "Mod menu: " + item.title + " for " + m.name)
        sheet = ""
        if (item.act === "toggle") toggle(m)
        else if (item.act === "top") moveMod(m, 0)
        else if (item.act === "bottom") moveMod(m, mods.length - 1)
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
    readonly property int railCount: games.length + 3   // games, add, Downloads, Settings
    onSheetChanged: {
        note("page", sheet === "" ? "Sheet closed" : "Sheet " + sheet); menuIndex = 0; stopReset()
        if (sheet === "" && fomod) sheet = "fomod"
        else if (sheet === "" && found) { infoIndex = 0; sheet = "found" }
        else if (sheet === "" && pendingNotes) { pendingNotes = false; infoIndex = 0; sheet = "notes" }
        else if (sheet === "") offerLoader()
    }

    Timer { interval: 16; repeat: true; running: app.active || shot.on; onTriggered: Turbine.poll() }

    property string repeatButton: ""
    Timer {
        id: repeater
        interval: 380
        repeat: true
        onTriggered: { interval = 85; app.act(app.repeatButton) }
    }
    function press(b) {
        if (["up", "down", "left", "right"].indexOf(b) >= 0) {
            repeatButton = b
            repeater.interval = 380
            repeater.restart()
        }
        if (b === "view") holdStart("View")
        act(b)
    }
    function release(b) {
        if (b === repeatButton) { repeater.stop(); repeatButton = "" }
        if (b === "a" && resetHold.running) stopReset()
        if (b === "view") holdStop()
    }

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

    function switchGame(step) {
        if (games.length === 0) return
        current = (current + step + games.length) % games.length
        note("action", "Switch game to " + games[current].name)
        if (page !== "game") page = "game"
        railIndex = current
    }
    function openSettings() { page = "settings"; zone = "settings"; setRow = 0 }

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
            else if (b === "b" || b === "view") sheet = ""
        } else if (sheet === "add") {
            if (b === "up") addIndex = Math.max(0, addIndex - 1)
            else if (b === "down") addIndex = Math.min(downloads.length - 1, addIndex + 1)
            else if (b === "a") addChosen()
            else if (b === "b") sheet = ""
        } else if (sheet === "check") {
            if (b === "left") checkIndex = Math.max(0, checkIndex - 1)
            else if (b === "right") checkIndex = Math.min(checkChoices.length - 1, checkIndex + 1)
            else if (b === "a") checkChoose(checkIndex)
            else if (b === "b") sheet = ""
        } else if (sheet === "loader") {
            if (b === "left" || b === "up") loaderIndex = 0
            else if (b === "right" || b === "down") loaderIndex = 1
            else if (b === "a" && loaderIndex === 0 && loaderCanGet) { sheet = ""; note("action", "Install the loader"); Turbine.installLoader() }
            else if (b === "a" && loaderIndex === 0 && !loaderSourceShown) loaderSourceShown = true
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
            else if (b === "b" || b === "x") sheet = ""
        } else if (sheet === "code") {
            if (b === "a" && codeData && codeData.state === "ready") { Turbine.copyText(codeData.code); toast.showPlain("Code copied.") }
            else if (b === "b") sheet = "profiles"
        } else if (sheet === "plugin") {
            if (b === "b" || b === "x" || b === "a") sheet = ""
        } else if (["nobrowser", "helper", "notes", "found"].indexOf(sheet) >= 0) infoAct(b)
        else if (sheet === "fomod") fomodView.handleButton(b)
        else if (sheet === "ask") askView.handleButton(b)
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
            else if (b === "a" || b === "b" || b === "y") endMove()
            return
        }
        if (orderMoving) { orderAct(b); return }
        if (b === "y" && undoShown) { putBack(undoId); return }
        if (b === "start") { startPlay(); return }
        if (b === "lb") { switchGame(-1); return }
        if (b === "rb") { switchGame(1); return }
        if (zone === "rail") {
            if (b === "up") railIndex = Math.max(0, railIndex - 1)
            else if (b === "down") railIndex = Math.min(railCount - 1, railIndex + 1)
            else if (b === "right" || b === "b") zone = page === "settings" ? "settings" : page === "perf" ? "perf" : "main"
            else if (b === "a") railSelect(railIndex)
            return
        }
        if (page === "game" && modPage) {
            if (b === "b") modPage = false
            else if (b === "a") installFromDetails()
            else if (b === "x") openVersions()
            else if (b === "left") { zone = "rail"; railIndex = current }
            else if (b === "lt") tab = Math.max(0, tab - 1)
            else if (b === "rt") tab = Math.min(tabs.length - 1, tab + 1)
            else if (b === "view" && game) { menuIndex = 0; sheet = "gamemenu" }
            return
        }
        if (page === "game" && tab === 1 && game && sources.length > 0 && ["up", "down", "left", "right", "a"].indexOf(b) >= 0) { getAct(b); return }
        if (page === "game" && tab === 2 && game && ["up", "down", "left", "a", "x", "y"].indexOf(b) >= 0) { orderAct(b); return }
        if (page === "game") {
            if (b === "left") { zone = "rail"; railIndex = current }
            else if (b === "lt") tab = Math.max(0, tab - 1)
            else if (b === "rt") tab = Math.min(tabs.length - 1, tab + 1)
            else if (b === "view" && game) { menuIndex = 0; sheet = "gamemenu" }
            else if (b === "a" && games.length === 0) load()
            else if (tab === 0 && game) {
                if (b === "up") modIndex = Math.max(0, modIndex - 1)
                else if (b === "down") modIndex = Math.min(listItems.length - 1, modIndex + 1)
                else if (b === "a" && modsLoaded && listItems.length === 0) openAdd()
                else if (b === "a" && focusedItem) focusedItem.kind === "mod" ? toggle(focusedItem.m) : putBack(focusedItem.m.id)
                else if (b === "x" && menuMod) sheet = "modmenu"
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
    function railSelect(i) {
        note("action", "Rail " + (i < games.length ? games[i].name : i === games.length ? "Add game" : i === games.length + 1 ? "Downloads" : "Settings"))
        if (i === games.length + 1) {
            if (games.length > 0) { page = "game"; zone = "main"; tab = 1 }
            if (dl.active) toast.showPlain("Downloading " + dl.label + (Turbine.netBusy > 1 ? ", then " + (Turbine.netBusy - 1) + " more" : "") + ". It shows on the status line.")
            return
        }
        if (i < games.length) { current = i; page = "game"; zone = "main" }
        else if (i === games.length) toast.showPlain("Turbine finds Steam and Heroic games by itself. Install a game and it shows up here.")
        else openSettings()
    }



    // ---------------------------------------------------------------- Get Mods (0.3)
    // Where this game's mods come from: Thunderstore for its games, Nexus Mods for the rest (with a key).
    readonly property var sources: !rules ? [] : [].concat(rules.community ? ["thunderstore"] : []).concat(rules.domain ? ["nexus"] : [])
    property string getSource: ""
    readonly property string srcNow: sources.indexOf(getSource) >= 0 ? getSource : (sources[0] || "")
    function srcName(s) { return s === "nexus" ? "Nexus Mods" : s === "thunderstore" ? "Thunderstore" : "" }
    property string getText: ""
    property string getCategory: ""
    property int getSortIndex: 0
    readonly property var sortKeys: srcNow === "nexus" ? ["endorsements", "downloads", "updated", "name"] : ["downloads", "rating", "updated", "name"]
    readonly property var sortNames: srcNow === "nexus" ? ["Most endorsed", "Most downloaded", "Recently updated", "A to Z"]
                                                        : ["Most downloaded", "Top rated", "Recently updated", "A to Z"]
    property var browseData: null
    property bool browseLoading: false
    property string getArea: "bar"        // bar, cats, grid
    property int getIndex: 0
    readonly property var getBar: [{ key: "search", label: getText !== "" ? getText : "Search " + gameTitle + " mods" },
                                   { key: "source", label: "From " + srcName(srcNow) },
                                   { key: "sort", label: sortNames[getSortIndex] || "" },
                                   { key: "add", label: "Add from Downloads" }]
    readonly property var getCats: browseData && browseData.categories && browseData.categories.length > 0 ? [""].concat(browseData.categories) : []
    readonly property var getItems: browseData && browseData.items ? browseData.items : []
    function browseNow() {
        if (!game || !rules || srcNow === "") return
        browseLoading = true
        Turbine.browse(getText, getCategory, sortKeys[getSortIndex] || "", srcNow)
    }
    function getAct(b) {
        const cols = 3
        if (getArea === "bar") {
            if (b === "left") { if (getIndex > 0) getIndex--; else { zone = "rail"; railIndex = current } }
            else if (b === "right") getIndex = Math.min(getBar.length - 1, getIndex + 1)
            else if (b === "down") { if (getCats.length > 0) { getArea = "cats"; getIndex = Math.max(0, getCats.indexOf(getCategory)) } else if (getItems.length > 0) { getArea = "grid"; getIndex = 0 } }
            else if (b === "a") getBarRun(getIndex)
        } else if (getArea === "cats") {
            if (b === "left") { if (getIndex > 0) getIndex--; else { zone = "rail"; railIndex = current } }
            else if (b === "right") getIndex = Math.min(getCats.length - 1, getIndex + 1)
            else if (b === "up") { getArea = "bar"; getIndex = 0 }
            else if (b === "down" && getItems.length > 0) { getArea = "grid"; getIndex = 0 }
            else if (b === "a") { getCategory = getCats[getIndex]; note("action", "Get Mods category " + (getCategory || "All")); browseNow() }
        } else {
            if (b === "left") { if (getIndex % cols > 0) getIndex--; else { zone = "rail"; railIndex = current } }
            else if (b === "right") getIndex = Math.min(getItems.length - 1, getIndex + 1)
            else if (b === "up") { if (getIndex >= cols) getIndex -= cols; else if (getCats.length > 0) { getArea = "cats"; getIndex = Math.max(0, getCats.indexOf(getCategory)) } else { getArea = "bar"; getIndex = 0 } }
            else if (b === "down") getIndex = Math.min(getItems.length - 1, getIndex + cols)
            else if (b === "a" && getItems[getIndex]) openDetails(getItems[getIndex])
        }
    }
    function getBarRun(i) {
        const k = getBar[i] ? getBar[i].key : ""
        getArea = "bar"; getIndex = i
        if (k === "search") openAsk("search", "Search " + gameTitle + " mods", "Words from the mod's name or what it does.", getText, "Search", 100, false, "Search")
        else if (k === "source" && sources.length > 1) {
            getSource = sources[(sources.indexOf(srcNow) + 1) % sources.length]; getCategory = ""; getSortIndex = 0
            note("action", "Get Mods source " + getSource); browseData = null; browseNow()
        }
        else if (k === "sort") { getSortIndex = (getSortIndex + 1) % sortNames.length; note("action", "Get Mods sort " + sortNames[getSortIndex]); browseNow() }
        else if (k === "add") openAdd()
    }

    // A mod's details, with Install.
    property bool modPage: false
    property var detailsData: null
    property var detailsItem: null
    function openDetails(item) {
        note("action", "Mod details: " + item.name)
        detailsItem = item
        detailsData = { loading: true }
        modPage = true
        Turbine.details(item.id, srcNow)
    }
    // Pages a free account opens instead of downloading: the mod itself, then what it needs.
    function needPages() {
        const n = detailsData && detailsData.needs ? detailsData.needs : null
        return !n ? [] : (n.steps || []).filter(x => x.action === "open")
            .map(x => ({ name: x.name, url: x.url, note: "Needed by " + detailsData.name }))
    }
    function installFromDetails() {
        if (!detailsData || detailsData.loading || detailsData.error) return
        if (detailsData.premium === false && detailsData.page) {
            note("action", "Open on Nexus: " + detailsData.name)
            openPages(detailsData.name + " is on Nexus Mods", [{ name: detailsData.name, url: detailsData.page, note: "The mod's files" }].concat(needPages()))
            return
        }
        if (detailsData.premium === false) { toast.showPlain(detailsData.message); return }
        note("action", "Install " + detailsData.name + " from " + srcNow)
        const later = needPages()
        Turbine.install(detailsData.id, srcNow)
        edited()
        modPage = false
        if (later.length > 0) openPages("It also needs these", later)
    }

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
        if (!openNexus(p.url)) return
        toast.showPlain("Opened " + p.name + " in the browser. Press Mod Manager Download there, or Slow download: Turbine picks it up from Downloads.")
    }

    // ---------------------------------------------------------------- the browser (catch.rs)
    // What the browser check found, the browser helper's link, and the status shown over the browser.
    property var browser: ({ checked: false })
    property var bridge: ({ running: false })
    property var status: ({ active: false })
    property bool movedPaired: false
    readonly property bool browserFound: !browser.checked || (!!browser.result && browser.result.found === true)
    // Opens a Nexus page in the player's browser and watches Downloads for its file. False when there is no browser.
    function openNexus(url) {
        if (!browserFound) { noBrowser(); return false }
        const w = parse(Turbine.watchPage(url), {})
        if (w.watching) note("catch", "Watching Downloads for this mod (" + w.game + ")")
        if (!shot.on) Qt.openUrlExternally(url)
        return true
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
        : "After setting it up, Pair opens a page that links the helper to this Turbine."
    // Install notes after adding a mod, and "Add this file?" for a download the watcher could only guess.
    property string notesName: ""
    property var notesList: []
    property var found: null
    property int infoIndex: 0
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
        : sheet === "found" ? [{ key: "a", label: "Add it" }, { key: "b", label: "Not this one" }]
        : [{ key: "a", label: "OK" }]
    function infoRun(i) {
        const label = infoButtons[i] ? infoButtons[i].label : ""
        note("action", "Sheet " + sheet + ": " + label)
        if (sheet === "nobrowser") { if (label === "Check again") { Turbine.browserCheck(); toast.showPlain("Looking for a browser...") } else sheet = "" }
        else if (sheet === "helper") { if (label === "Pair") { sheet = ""; pair(false) } else sheet = "" }
        else if (sheet === "found") { const f = found; found = null; sheet = ""; if (f) Turbine.watchConfirm(f.path, label === "Add it") }
        else sheet = ""
    }
    function infoAct(b) {
        if (b === "left" || b === "up") infoIndex = Math.max(0, infoIndex - 1)
        else if (b === "right" || b === "down") infoIndex = Math.min(infoButtons.length - 1, infoIndex + 1)
        else if (b === "a") infoRun(infoIndex)
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
        note("action", "Other versions of " + detailsData.name)
        versionIndex = 0
        sheet = "versions"
    }
    function installVersion(i) {
        const v = otherVersions[i]
        if (!v || v.installed) return
        if (detailsData.premium === false && detailsData.page) {
            openPages(detailsData.name + " " + (v.version || v.name), [{ name: v.name + (v.version ? " " + v.version : ""), url: detailsData.page + "&file_id=" + v.id, note: v.group }])
            return
        }
        note("action", "Install version " + (v.version || v.name) + " of " + detailsData.name)
        sheet = ""
        Turbine.installVersion(detailsData.id, detailsData.source || srcNow, v.id)
        edited()
        modPage = false
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
    property int orderFrom: -1
    function orderAct(b) {
        const p = order[orderIndex]
        if (orderMoving) {
            if (b === "up" || b === "down") {
                const to = orderIndex + (b === "up" ? -1 : 1)
                if (to < 0 || to >= order.length || order[to].fixed) return
                let list = order.slice(); const x = list[orderIndex]; list[orderIndex] = list[to]; list[to] = x
                order = list; orderIndex = to
            } else if (b === "a" || b === "b" || b === "y") {
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
        else if (b === "y" && p && !p.fixed) { orderMoving = true; orderFrom = orderIndex }
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
    readonly property var settingsSections: ["Look", "Accounts", "Updates", "About"].concat(labsOn ? ["Labs"] : [])
    readonly property string sec: settingsSections[setSection] || "Look"
    function openSection(name) { openSettings(); setSection = Math.max(0, settingsSections.indexOf(name)); setRow = 0 }
    onLabsOnChanged: if (!labsOn && setSection >= settingsSections.length) { setSection = settingsSections.indexOf("About"); setRow = 0 }
    // Accounts, Updates and Labs: rows of one kind each. action (A does it), toggle and choice (Left, Right or A
    // change it), item (a Labs checklist item), info (words only).
    readonly property var rowsNow: sec === "Accounts" ? accountRows : sec === "Updates" ? updateRows : sec === "Labs" ? labsRows : []
    readonly property string accountWords: account.state === "checking" ? "Checking the key with Nexus Mods..."
        : account.state === "ok" ? "Signed in as " + account.name + (account.premium ? " · Premium: downloads work right here" : " · Free account: Open on Nexus, then Mod Manager Download")
        : account.state === "error" ? account.message
        : account.message ? account.message : "Not signed in. Add your personal API key to browse Nexus Mods."
    readonly property var accountRows: [
        { kind: "action", key: "nexusKey", title: "Nexus Mods", how: accountWords, action: account.hasKey ? "Change key" : "Add key" }
    ].concat([
        { kind: "action", key: "nxmHandler", title: "Use Turbine for Nexus downloads",
          how: handler.state === "yes" ? "Turbine gets Mod Manager Download links from the browser. Works with free accounts."
             : handler.state === "no" ? "Mod Manager Download links now go to " + (handler.current || "another program") + ". A makes Turbine get them."
             : "Makes the browser's Mod Manager Download button send files to Turbine." + (handler.message ? " Turbine couldn't check which program gets them now." : ""),
          action: handler.state === "yes" ? "Set again" : "Use Turbine" }
    ]).concat(account.hasKey ? [{ kind: "action", key: "signOut", title: "Sign out of Nexus Mods", how: "Deletes the key from this device", action: "Sign out" }] : []).concat([
        { kind: "info", key: "help", title: "Where the key comes from", how: "In Desktop Mode, sign in at nexusmods.com, open Site preferences, API Keys, and copy your Personal API Key. Then paste it here." },
        { kind: "info", key: "ts", title: "Thunderstore", how: "No account needed. Valheim, Lethal Company and Risk of Rain 2 mods come from there." },
        // Browser
        { kind: "action", key: "browser", head: "Browser", title: "Web browser", how: browserWords, action: browserFound ? "Check again" : "How to get one" },
        { kind: "action", key: "helper", title: "Set up the browser helper",
          how: "Shows Turbine's progress and installer choices on Nexus pages, and sends Slow downloads to Turbine. For Chrome, Brave, Edge and other Chromium browsers.",
          action: "Set up" },
        { kind: "action", key: "pair", title: bridgeTitle, how: bridgeWords, action: bridge.paired ? "Pair again" : "Pair" },
        { kind: "toggle", key: "overlay", title: "Show status over other apps",
          how: "A small Turbine card shows download progress and Added to the game while you're in the browser.",
          value: browserSet.overlay, options: [false, true], names: ["Off", "On"] }
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
        else if (r.key === "steamBrowser") labsRun(r)
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
        { id: "text-size", title: "Text size in Game Mode", how: "In Game Mode, look at every page at Text size 75% to 175%.",
          expect: "Everything is readable; nothing is cut off or overlapping." },
        { id: "add-downloads", title: "Add mods from Downloads", how: "Put a mod's zip in Downloads, then Get Mods > Add mods from Downloads.",
          expect: "The mod shows in the list, turned on, with its name and version." },
        { id: "auto-apply", title: "Auto-apply, then start from Steam", how: "Turn a mod on or off, wait 5 seconds, close Turbine, start the game from the Steam library.",
          expect: "The game has the change without pressing Play in Turbine." },
        { id: "uninstall-undo", title: "Uninstall and Undo", how: "Uninstall a mod from its menu (X), then press Y for Undo.",
          expect: "It moves to Recently removed and comes back with Undo, in the same place." },
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
        { id: "ts-install", title: "Thunderstore install", how: "Valheim, Lethal Company or Risk of Rain 2: Get Mods, pick a mod, A Install.",
          expect: "It downloads with progress on the status line and the rail ring, and shows in Mods with what it needs." },
        { id: "loader-install", title: "Loader install", how: "Turn on a mod that needs the loader, then A Install in the prompt (SKSE, BepInEx or SMAPI).",
          expect: "The loader downloads from its official home, shows as a mod, and the Needs badges go away." },
        { id: "fomod", title: "Installer choices (FOMOD)", how: "Add a Skyrim mod with an installer, from Get Mods or Downloads.",
          expect: "One question per page with pictures; B goes back; the files of the chosen options go in." },
        { id: "load-order", title: "Load order", how: "Skyrim: Load order tab. Turn a plugin off, move one with Y, then Sort automatically.",
          expect: "The game loads plugins in that order; masters stay first; missing masters are named." },
        { id: "ts-code", title: "Thunderstore code", how: "Lethal Company or another Thunderstore game, online: game menu (View), Profiles, Create a Thunderstore code.",
          expect: "A code shows big and is copied; importing it on another device (or Profiles, Import a Thunderstore profile code) gets the same mods." },
        { id: "other-versions", title: "Other versions", how: "Get Mods, a mod's details, X Other versions, then A on an older version.",
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
        { id: "slow-download", title: "Slow download from Downloads", how: "Open a Nexus mod page from Turbine, press Slow download there and wait for it to finish.",
          expect: "Turbine adds the file to the right game by itself (or asks first when it couldn't read Nexus's file list); the file stays in Downloads." },
        { id: "md5-identify", title: "Files named from Nexus", how: "With a Nexus key, add a mod you downloaded from Nexus by hand (Add from Downloads).",
          expect: "It shows with its Nexus name and version, and gets Update ready badges like mods installed from Get Mods." },
        { id: "overlay-desktop", title: "Status over the browser (Desktop Mode)", how: "Desktop Mode: start a Nexus download from Turbine, then stay in the browser.",
          expect: "A small Turbine card stays above the browser with progress, then Added to the game, and goes 6 seconds later. Its buttons work with a tap or click." },
        { id: "overlay-gamemode", title: "Status over the browser (Game Mode)", how: "Game Mode: open a Nexus page from Turbine (it opens over Turbine), download a mod, watch the bottom left corner.",
          expect: "The brass card shows over the browser and never takes the controller; it goes when idle, and the Performance Overlay still works after." },
        { id: "gamemode-window", title: "Turbine's window in Game Mode", how: "Open Turbine in Game Mode and use it for a minute (this build draws through X11 on SteamOS).",
          expect: "Turbine fills the screen, looks sharp, and the controller works as before." },
        { id: "browser-helper", title: "Browser helper", how: "Settings, Accounts: Set up the browser helper, follow the steps, then Pair. Download a mod on Nexus.",
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
    function labsChoose(item, st) {
        let a = Object.assign({}, labsAnswers)
        if (st === "") delete a[item.id]
        else a[item.id] = st
        labsAnswers = a
        if (!shot.on) labsSet.states = JSON.stringify(a)
        note("labs", "Checklist: " + item.title + " = " + (st || "not tested"))
        if (st === "problem") openReport(item, "", "")
    }
    function labsRun(row) {
        note("action", "Labs: " + row.title)
        if (row.key === "report") openReport(null, "", "")
        else if (row.key === "perf") openPerf()
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
            "Scale: fit " + fit.toFixed(2) + "x, text size " + look.textSize + "%, everything " + s.toFixed(2) + "x, layout " + look.layout,
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
    function reportEditing(on) {
        note("labs", on ? "Typing the report" : "Done typing the report")
        if (Turbine.gamemode && !shot.on) Qt.openUrlExternally(on ? "steam://open/keyboard" : "steam://close/keyboard")
        if (!on) keyRoot.forceActiveFocus()
    }
    function openPerf() { perfView.version = Turbine.version; perfView.extra = perfExtra(); perfView.open(); sheet = "perf" }
    function perfExtra() {
        return "Window: " + width + " x " + height + ", " + (Turbine.gamemode ? "Game Mode" : "Desktop") + "\n"
            + "Turbine scale: fit " + fit.toFixed(2) + "x, text size " + look.textSize + "%, everything " + s.toFixed(2) + "x, layout " + look.layout + "\n"
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
    readonly property var lookRows: [
        { label: "Theme", options: ["brass", "black", "light"], names: ["Brass", "Black", "Light"], key: "theme",
          step: (d) => cycle("theme", ["brass", "black", "light"], d) },
        { label: "Accent", options: Object.keys(c.swatches), key: "accent", swatch: true,
          step: (d) => cycle("accent", ["brass", "iron", "copper", "jade", "fjord", "heather", "ember"], d) },
        { label: "Text size", key: "textSize", slider: true, min: 75, max: 175, unit: "%",
          step: (d) => slide("textSize", 75, 175, 25, d) },
        { label: "Contrast", options: [false, true], names: ["Standard", "High"], key: "highContrast",
          step: (d) => { look.highContrast = !look.highContrast } },
        { label: "Color vision", options: ["typical", "red", "green", "blue", "none"], names: ["Typical", "Red", "Green", "Blue", "None"], key: "vision",
          step: (d) => cycle("vision", ["typical", "red", "green", "blue", "none"], d) },
        { label: "Layout", options: ["handheld", "tv"], names: ["Handheld", "TV"], key: "layout",
          step: (d) => cycle("layout", ["handheld", "tv"], d) },
        { label: "Seasonal", options: ["off", "date", "autumn", "snow"], names: ["Off", "By date", "Autumn", "Snow"], key: "season",
          now: seasonPick, step: (d) => { look.season = seasonPick; cycle("season", ["off", "date", "autumn", "snow"], d) } },
        { label: "Amount", key: "fallAmount", slider: true, min: 10, max: 100, unit: "", dim: seasonKind === "",
          step: (d) => slide("fallAmount", 10, 100, 10, d) },
        { label: "Size", key: "fallSize", slider: true, min: 50, max: 200, unit: "%", dim: seasonKind === "",
          step: (d) => slide("fallSize", 50, 200, 25, d) }
    ]
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
        spacing: app.px(6)
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
        anchors.fill: parent

        // ===== Rail
        Rectangle {
            id: rail
            width: app.px(72)
            height: parent.height
            color: c.rail
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
                anchors.bottom: railFoot.top
                anchors.bottomMargin: app.px(10)
                width: parent.width
                clip: true
                spacing: app.px(10)
                model: app.games.length + 1
                currentIndex: app.zone === "rail" ? Math.min(app.railIndex, app.games.length) : app.current
                highlightFollowsCurrentItem: false
                onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                delegate: Item {
                    id: tileItem
                    required property int index
                    readonly property bool isAdd: index === app.games.length
                    readonly property var g: isAdd ? null : app.games[index]
                    readonly property bool here: !isAdd && index === app.current && app.page === "game"
                    readonly property bool focused: app.zone === "rail" && app.railIndex === index
                    width: railList.width
                    height: app.px(44)
                    Rectangle {
                        visible: tileItem.here
                        x: app.px(1); width: app.px(4); height: app.px(28); radius: app.px(2)
                        anchors.verticalCenter: parent.verticalCenter
                        color: c.accent
                    }
                    Rectangle {
                        id: tile
                        width: app.px(44); height: app.px(44)
                        anchors.horizontalCenter: parent.horizontalCenter
                        radius: app.px(8)
                        color: tileItem.isAdd ? "transparent" : c.raised
                        border.width: tileItem.isAdd ? app.px(2) : (tileItem.here || tileItem.focused ? app.px(2) : 0)
                        border.color: tileItem.focused ? c.text : tileItem.isAdd ? c.line : c.accent
                        clip: true
                        Image {
                            id: art
                            anchors.fill: parent
                            anchors.margins: tile.border.width
                            visible: status === Image.Ready
                            source: tileItem.g ? app.fileUrl(tileItem.g.cover || tileItem.g.header) : ""
                            fillMode: Image.PreserveAspectCrop
                            sourceSize.height: app.px(88)
                            asynchronous: true
                        }
                        Text {
                            anchors.centerIn: parent
                            visible: tileItem.isAdd || !art.visible
                            text: tileItem.isAdd ? "+" : app.initials(tileItem.g.name)
                            color: tileItem.isAdd ? c.muted : c.soft
                            font.family: app.bodyFont
                            font.pixelSize: app.px(tileItem.isAdd ? 22 : 14)
                            font.bold: true
                        }
                    }
                    TapHandler { onTapped: { app.railIndex = tileItem.index; app.railSelect(tileItem.index) } }
                }
            }
            Column {
                id: railFoot
                anchors.bottom: parent.bottom
                anchors.bottomMargin: app.px(10)
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: app.px(6)
                // Downloads: a brass ring fills while something downloads; A goes to Get Mods.
                Rectangle {
                    id: dlButton
                    width: app.px(44); height: app.px(44); radius: app.px(8)
                    readonly property bool focused: app.zone === "rail" && app.railIndex === app.games.length + 1
                    color: "transparent"
                    border.width: focused ? app.px(2) : 0
                    border.color: c.text
                    Icon {
                        anchors.centerIn: parent
                        tint: app.dl.active ? c.accentHi : c.muted
                        d: "M12 4v11M7 10l5 5 5-5M5 20h14"
                    }
                    Shape {
                        visible: app.dl.active
                        anchors.fill: parent
                        preferredRendererType: Shape.CurveRenderer
                        ShapePath {
                            strokeColor: c.line; strokeWidth: app.px(3); fillColor: "transparent"
                            PathAngleArc { centerX: dlButton.width / 2; centerY: dlButton.height / 2; radiusX: dlButton.width / 2 - app.px(3); radiusY: radiusX; startAngle: 0; sweepAngle: 360 }
                        }
                        ShapePath {
                            strokeColor: c.accentHi; strokeWidth: app.px(3); fillColor: "transparent"; capStyle: ShapePath.RoundCap
                            PathAngleArc {
                                centerX: dlButton.width / 2; centerY: dlButton.height / 2; radiusX: dlButton.width / 2 - app.px(3); radiusY: radiusX
                                startAngle: -90; sweepAngle: app.dlPart >= 0 ? 360 * app.dlPart : 90
                            }
                        }
                    }
                    Rectangle {
                        visible: Turbine.netBusy > 1
                        anchors.right: parent.right; anchors.top: parent.top
                        width: app.px(18); height: width; radius: width / 2
                        color: c.accent
                        Text { anchors.centerIn: parent; text: Turbine.netBusy; color: c.onAccent; font.family: app.bodyFont; font.pixelSize: app.px(11); font.bold: true }
                    }
                    TapHandler { onTapped: app.railSelect(app.games.length + 1) }
                }
                Rectangle {
                    width: app.px(44); height: app.px(44); radius: app.px(8)
                    readonly property bool focused: app.zone === "rail" && app.railIndex === app.games.length + 2
                    color: app.page === "settings" || app.page === "perf" ? c.raised : "transparent"
                    border.width: focused ? app.px(2) : 0
                    border.color: c.text
                    Icon {
                        anchors.centerIn: parent
                        tint: app.page === "settings" ? c.accent : c.muted
                        d: "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.6 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.6a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"
                    }
                    TapHandler { onTapped: app.openSettings() }
                }
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

                // Header
                Item {
                    width: parent.width
                    height: app.px(52)
                    // The game's short name when it has one ("Skyrim Special Edition"); one size smaller if it still
                    // doesn't fit, and only then cut short.
                    Text {
                        id: headerTitle
                        readonly property bool fits: headerFull.advanceWidth <= width
                        anchors.centerIn: parent
                        width: parent.width - app.px(320)
                        horizontalAlignment: Text.AlignHCenter
                        text: app.page === "settings" ? "SETTINGS" : app.page === "perf" ? "SCROLLING TEST"
                              : app.game ? app.gameTitle.toUpperCase() : "TURBINE MOD MANAGER"
                        color: c.text
                        font.family: app.titleFont
                        font.pixelSize: app.px(fits ? 17 : 15)
                        font.bold: true
                        font.letterSpacing: app.px(fits ? 1.5 : 1)
                        elide: Text.ElideRight
                        TextMetrics {
                            id: headerFull
                            text: headerTitle.text
                            font.family: app.titleFont
                            font.pixelSize: app.px(17)
                            font.bold: true
                            font.letterSpacing: app.px(1.5)
                        }
                    }
                    Rectangle {
                        visible: app.page !== "game" || app.modPage
                        anchors.left: parent.left
                        anchors.verticalCenter: parent.verticalCenter
                        width: backRow.implicitWidth + app.px(20); height: app.px(30); radius: app.px(6)
                        color: c.panel
                        Hint { id: backRow; anchors.centerIn: parent; k: "b"; label: "Back" }
                        TapHandler { onTapped: app.act("b") }
                    }
                    Rectangle {
                        visible: app.page === "game" && app.game !== null
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        width: playRow.implicitWidth + app.px(28); height: app.px(38); radius: app.px(8)
                        opacity: Turbine.busy ? 0.7 : 1
                        // accentHi to accent: the Play words stay at 4.5:1 over the whole button.
                        gradient: Gradient {
                            GradientStop { position: 0; color: c.accentHi }
                            GradientStop { position: 1; color: c.accent }
                        }
                        Row {
                            id: playRow
                            anchors.centerIn: parent
                            spacing: app.px(8)
                            Key { k: "start"; anchors.verticalCenter: parent.verticalCenter }
                            Text { text: "▶ Play"; color: c.onAccent; font.family: app.bodyFont; font.pixelSize: app.px(15); font.bold: true; anchors.verticalCenter: parent.verticalCenter }
                        }
                        TapHandler { onTapped: app.startPlay() }
                    }
                }

                // Status line: always says what's going on, in words.
                StatusLine {
                    ui: kit
                    width: parent.width
                    height: app.px(44)
                    level: app.statusNow.level
                    words: app.statusNow.words
                    detail: app.statusNow.detail
                    progress: app.statusNow.progress !== undefined ? app.statusNow.progress : -1
                    progressText: app.statusNow.progressText || ""
                    pillText: app.statusNow.pill || ""
                    onPillClicked: if (app.updateCount > 0) Turbine.updateAll()
                }

                // Tabs (game page only)
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
                                Label { anchors.centerIn: parent; text: parent.modelData; font.bold: true; color: app.tab === parent.index ? c.text : c.muted }
                                TapHandler { onTapped: app.tab = parent.index }
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

                    // When there is no list to show: no games, mods being read, no mods yet, Load order.
                    EmptyState {
                        id: middle
                        readonly property bool none: !app.game
                        readonly property bool loading: app.game !== null && app.tab === 0 && !app.modsLoaded
                        readonly property bool empty: app.game !== null && app.tab === 0 && app.modsLoaded && app.listItems.length === 0
                        ui: kit
                        anchors.fill: parent
                        visible: app.page === "game" && (none || loading || empty)
                        title: none ? "No games yet" : loading ? "Reading mods..." : "No mods yet"
                        body: none ? "Turbine looks in Steam's libraries, including SD cards, and in Heroic. Install a game and it shows up here."
                              : loading ? ""
                              : app.sources.length > 0 ? "Find mods on the Get Mods tab (RT), or add them from archives in your Downloads folder."
                              : "Add mods from archives (zip, 7z, rar) in your Downloads folder. Download a mod in Desktop Mode first."
                        buttonKey: none || empty ? "a" : ""
                        buttonLabel: none ? "Look again" : empty ? "Add mods from Downloads" : ""
                        onActivated: none ? app.load() : app.openAdd()
                    }

                    // Mods tab: the mods, then Recently removed.
                    ListView {
                        id: modList
                        visible: app.page === "game" && app.game !== null && app.tab === 0 && app.listItems.length > 0
                        anchors.fill: parent
                        topMargin: app.px(4)
                        bottomMargin: app.px(4)
                        clip: true
                        spacing: app.px(6)
                        model: app.listItems
                        currentIndex: app.modIndex
                        highlightFollowsCurrentItem: false
                        onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                        delegate: Item {
                            id: entry
                            required property int index
                            required property var modelData
                            readonly property bool isMod: modelData.kind === "mod"
                            readonly property var m: modelData.m
                            readonly property bool foc: app.zone === "main" && app.sheet === "" && app.modIndex === index
                            readonly property bool firstRemoved: !isMod && index === app.mods.length
                            width: modList.width
                            height: isMod ? row.implicitHeight : removedRow.height + (firstRemoved ? removedHead.height : 0)
                            ModRow {
                                id: row
                                visible: entry.isMod
                                ui: kit
                                x: app.px(4)
                                width: parent.width - app.px(8)
                                name: entry.m.name
                                detail: app.modWords(entry.m)
                                isOn: entry.m.enabled
                                ordered: app.ordered
                                order: entry.index + 1
                                focused: entry.foc
                                badge: app.moving && entry.foc ? "Moving" : (entry.m.needsLoader && app.loaderMissing ? "Needs " + app.loaderShort : app.updateMap[entry.m.id] ? "Update ready" : "")
                                badgeLevel: app.moving && entry.foc ? "info" : (entry.m.needsLoader && app.loaderMissing) ? "warn" : "info"
                                showHints: entry.foc && !app.moving
                                onClicked: app.modIndex = entry.index
                                onToggleClicked: { app.modIndex = entry.index; app.toggle(entry.m) }
                            }
                            Label {
                                id: removedHead
                                visible: entry.firstRemoved
                                x: app.px(8)
                                height: app.px(30)
                                text: "RECENTLY REMOVED · KEPT 7 DAYS"
                                color: c.muted
                                font.pixelSize: app.px(14)
                                font.bold: true
                                font.letterSpacing: app.px(1)
                            }
                            Rectangle {
                                id: removedRow
                                visible: !entry.isMod
                                x: app.px(4)
                                y: entry.firstRemoved ? removedHead.height : 0
                                width: parent.width - app.px(8)
                                height: app.px(46)
                                radius: app.px(6)
                                color: entry.foc ? c.focusFill : c.card
                                border.color: c.accent
                                border.width: entry.foc ? Math.max(1, app.px(1.5)) : 0
                                Label {
                                    x: app.px(14)
                                    anchors.verticalCenter: parent.verticalCenter
                                    width: parent.width - app.px(200)
                                    text: entry.m.name || ""
                                    color: c.muted
                                    font.pixelSize: app.px(15)
                                }
                                Hint {
                                    visible: entry.foc
                                    anchors.right: parent.right
                                    anchors.rightMargin: app.px(12)
                                    anchors.verticalCenter: parent.verticalCenter
                                    k: "a"; label: "Put back"
                                }
                                TapHandler { onTapped: { app.modIndex = entry.index; app.putBack(entry.m.id) } }
                            }
                        }
                    }

                    // Get Mods tab: online for games with a source, otherwise archives from Downloads.
                    EmptyState {
                        ui: kit
                        anchors.fill: parent
                        visible: app.page === "game" && app.game !== null && app.tab === 1 && app.sources.length === 0
                        title: "Get Mods"
                        body: "Add mods from archives (zip, 7z, rar) in your Downloads folder, for example ones you downloaded in Desktop Mode."
                        buttonKey: "a"
                        buttonLabel: "Add mods from Downloads"
                        buttonFocused: app.zone === "main"
                        note: app.rules ? "This game has no online mod source in Turbine yet." : ""
                        onActivated: app.openAdd()
                    }
                    GetMods {
                        ui: kit
                        anchors.fill: parent
                        anchors.topMargin: app.px(4)
                        visible: app.page === "game" && app.game !== null && app.tab === 1 && app.sources.length > 0 && !app.modPage
                        result: app.browseData
                        loading: app.browseLoading
                        search: app.getText
                        category: app.getCategory
                        bar: app.getBar
                        focusArea: app.getArea
                        focusIndex: app.getIndex
                        active: app.zone === "main" && app.sheet === ""
                        onBarClicked: (i) => app.getBarRun(i)
                        onCategoryClicked: (i) => { app.getArea = "cats"; app.getIndex = i; app.getAct("a") }
                        onItemClicked: (i) => { app.getArea = "grid"; app.getIndex = i; app.openDetails(app.getItems[i]) }
                    }
                    ModDetails {
                        ui: kit
                        anchors.fill: parent
                        anchors.topMargin: app.px(4)
                        visible: app.page === "game" && app.modPage
                        info: app.detailsData
                        focused: app.zone === "main"
                        onInstallClicked: app.installFromDetails()
                        onVersionsClicked: app.openVersions()
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
                                                TapHandler { onTapped: { app.setRow = srow.index; look[srow.modelData.key] = srow.modelData.options[parent.index]; app.note("action", "Look: " + srow.modelData.key + " " + look[srow.modelData.key]) } }
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
                                    TapHandler { onTapped: { app.setRow = 0; app.labsTap() } }
                                }
                                Column {
                                    anchors.verticalCenter: parent.verticalCenter
                                    spacing: app.px(4)
                                    Label { text: "Turbine Mod Manager " + Turbine.version; font.family: app.titleFont; font.pixelSize: app.px(19); font.bold: true }
                                    Label { visible: app.labsOn; text: "Labs mode on"; color: c.accentText; font.pixelSize: app.px(15); font.bold: true }
                                }
                            }
                            Label { width: parent.width; wrapMode: Text.WordWrap; elide: Text.ElideNone; color: c.soft; text: "A mod manager for SteamOS handhelds. Free software under the GNU General Public License, version 3 or later. Licenses of the parts it uses are in the licenses folder of the project." }
                            Rectangle {
                                readonly property bool foc: app.zone === "settings" && app.setRow === 1
                                width: perfRow.implicitWidth + app.px(28); height: app.px(40); radius: app.px(8)
                                color: foc ? c.focusFill : c.card
                                border.color: c.accent
                                border.width: foc ? app.px(2) : 0
                                Hint { id: perfRow; anchors.centerIn: parent; k: "a"; label: "Scrolling test (2,000 rows)" }
                                TapHandler { onTapped: { app.page = "perf"; app.zone = "perf" } }
                            }
                        }
                        // Labs: Report a bug, Performance check, and the checklist of things to confirm on the device.
                        ListView {
                            id: labsList
                            width: parent.width - app.px(184)
                            height: parent.height
                            visible: app.sec === "Accounts" || app.sec === "Updates" || app.sec === "Labs"
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
                                    text: (lrow.modelData.head || "").toUpperCase() + (lrow.modelData.head && lrow.isItem ? " · " + app.labsDone : "")
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
                Label { text: app.game ? app.game.name : ""; font.family: app.titleFont; font.pixelSize: app.px(19); font.bold: true; width: parent.width }
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
                Label { text: "Made for SteamOS"; font.family: app.titleFont; font.pixelSize: app.px(19); font.bold: true }
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
        ui: kit
        visible: app.sheet === "check"
        checks: app.checks
        buttons: app.checkChoices.map((label, i) => ({ key: i === 0 ? "a" : label === "Wait" ? "b" : "", label: label }))
        focusIndex: app.checkIndex
        note: app.checkStops ? "Play original starts the game with no mods. Nothing is deleted." : ""
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
               : app.sheet === "found" ? "Is this the mod's file?"
               : "Added " + app.notesName
        text: app.sheet === "nobrowser" ? (app.browser.text || "")
              : app.sheet === "helper" ? "Turbine put the helper in Downloads, in the folder Turbine browser helper. Then, once:"
              : app.sheet === "found" && app.found ? app.found.name + " (" + app.sizeWords(app.found.size) + ") arrived in Downloads after the Nexus page opened. Turbine couldn't read the mod's file list, so it isn't sure this is the file. Add it to " + app.found.gameName + "?"
              : app.sheet === "notes" ? "Something to know about this mod:" : ""
        steps: app.sheet === "nobrowser" ? (app.browser.steps || [])
               : app.sheet === "helper" ? ["In Desktop Mode, open chrome://extensions in the browser.",
                                           "Turn on Developer mode (top right).",
                                           "Press Load unpacked.",
                                           "Choose Downloads, then the folder Turbine browser helper."]
               : []
        notes: app.sheet === "notes" ? app.notesList : []
        footnote: app.sheet === "helper" ? "Then press Pair: the pairing page opens in the browser and links the helper to Turbine." : ""
        buttons: app.infoButtons
        focusIndex: app.infoIndex
        onActivated: (i) => { app.infoIndex = i; app.infoRun(i) }
        onClosed: app.act("b")
    }
    // The status card over other apps (see showOverlay).
    StatusOverlay {
        id: panelWin
        ui: kit
        hud: false
        status: shot.on ? app.status : app.transfer
        fomod: app.fomod
        onConfirm: (yes) => { if (app.status.ask) Turbine.watchConfirm(app.status.ask, yes); app.found = null; if (app.sheet === "found") app.sheet = "" }
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

    // Legend: only the controls that matter here. Declared after the menus so their backdrop never dims it.
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
        readonly property bool gamePage: app.page === "game" && app.game !== null && app.sheet === ""
        Row {
            anchors.verticalCenter: parent.verticalCenter
            x: app.px(12)
            spacing: app.px(16)
            // Moving a mod
            Hint { visible: app.moving; k: "▲▼"; label: "Move" }
            Hint { visible: app.moving; k: "a"; label: "Done" }
            // Sheets
            Hint { visible: ["modmenu", "gamemenu", "add", "fomod", "profiles", "shares", "versions"].indexOf(app.sheet) >= 0; k: "▲▼"; label: "Choose" }
            Hint { visible: app.sheet === "versions"; k: "a"; label: app.detailsData && app.detailsData.premium === false ? "Open on Nexus" : "Install" }
            Hint { visible: app.sheet === "pages"; k: "▲▼"; label: "Choose" }
            Hint { visible: app.sheet === "pages"; k: "a"; label: "Open on Nexus" }
            Hint { visible: ["nobrowser", "helper", "found"].indexOf(app.sheet) >= 0; k: "◀▶"; label: "Choose" }
            Hint { visible: app.sheet === "code" && !!app.codeData && app.codeData.state === "ready"; k: "a"; label: "Copy" }
            Hint { visible: app.sheet === "check" || app.sheet === "loader" || app.sheet === "perf"; k: "◀▶"; label: "Choose" }
            Hint { visible: app.sheet === "report" && !reportView.editing; k: "▲▼"; label: "Move" }
            Hint { visible: app.sheet === "report" && reportView.editing; k: "b"; label: "Done typing" }
            Hint { visible: app.sheet !== "" && app.sheet !== "ask" && !(app.sheet === "report" && reportView.editing); k: "b"; label: app.sheet === "check" ? "Wait" : app.sheet === "fomod" ? "Back a step" : app.sheet === "found" ? "Not this one" : "Close" }
            Hint { visible: app.sheet === "ask"; k: "a"; label: "Done" }
            Hint { visible: app.sheet === "ask"; k: "b"; label: "Cancel" }
            // Get Mods, details, load order
            Hint { visible: legend.gamePage && app.modPage; k: "a"; label: app.detailsData && app.detailsData.premium === false && app.detailsData.page ? "Open on Nexus" : "Install" }
            Hint { visible: legend.gamePage && app.modPage && app.otherVersions.length > 0; k: "x"; label: "Other versions" }
            Hint { visible: legend.gamePage && app.modPage; k: "b"; label: "Back to the list" }
            Hint { visible: legend.gamePage && !app.modPage && app.tab === 1 && app.sources.length > 0; k: "a"; label: app.getArea === "grid" ? "Details" : "Choose" }
            Hint { visible: legend.gamePage && app.tab === 2 && !app.orderMoving; k: "a"; label: app.orderIndex < 0 ? "Sort" : "Turn on or off" }
            Hint { visible: legend.gamePage && app.tab === 2 && !app.orderMoving && app.orderIndex >= 0; k: "y"; label: "Move" }
            Hint { visible: legend.gamePage && app.tab === 2 && !app.orderMoving && app.orderIndex >= 0; k: "x"; label: "Details" }
            Hint { visible: app.orderMoving; k: "▲▼"; label: "Move" }
            Hint { visible: app.orderMoving; k: "a"; label: "Done" }
            // Game page
            Hint { visible: legend.gamePage && !app.moving && app.tab === 0 && app.ordered && app.mods.length > 1; k: "y"; label: "Reorder" }
            Hint { visible: legend.gamePage && !app.moving && !app.modPage; k: "lb+rb"; label: "Switch game" }
            Hint { visible: legend.gamePage && !app.moving; k: "view"; label: "Game menu" }
            Hint { visible: app.sheet === "" && app.page === "settings"; k: "lt+rt"; label: "Sections" }
            Hint { visible: app.sheet === "" && app.page === "settings" && (app.sec === "Look" || (!!app.rowsNow[app.setRow] && ["choice", "toggle"].indexOf(app.rowsNow[app.setRow].kind) >= 0)); k: "◀▶"; label: "Change" }
            Hint { visible: app.sheet === "" && app.page === "settings" && !!app.rowsNow[app.setRow] && app.rowsNow[app.setRow].kind === "item"; k: "◀▶"; label: "Choose" }
            Hint { visible: app.sheet === "" && app.page !== "game"; k: "b"; label: "Back" }
            Hint { visible: app.sheet === "" && !app.moving && app.zone !== "rail" && !(app.page === "game" && (app.modPage || app.tab === 2)); k: "◀"; label: "Games" }
        }
        Label {
            anchors.right: parent.right
            anchors.rightMargin: app.px(12)
            anchors.verticalCenter: parent.verticalCenter
            visible: !Turbine.pads
            text: "No controller found"
            color: c.muted
            font.pixelSize: app.px(14)
        }
    }

    // ---------------------------------------------------------------- toast
    // Plain messages, and after Uninstall "Recently removed: name · Kept 7 days · Y Undo".
    property string undoId: ""
    readonly property bool undoShown: toast.shown && undoId !== "" && sheet === ""
    Toast {
        id: toast
        ui: kit
        function showUndo(id, name) { app.undoId = id; show("Recently removed:", name, "Kept 7 days", "y", "Undo") }
        function showPlain(m) { app.undoId = ""; show(m) }
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
}
