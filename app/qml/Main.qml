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
    }
    Settings {
        id: seen
        category: "seen"
        property bool notSteamOS: false
        property string lastGame: ""
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
        readonly property bool slow: ["mods", "menu", "gamemenu", "check", "add", "loader"].indexOf(page) >= 0
        function start() {
            const files = arg("selftest-import")
            if (files !== "") for (const f of files.split(",")) Turbine.importZip(f)
            if (page === "gamemenu") app.sheet = "gamemenu"
            else if (page === "add") { app.tab = 1; app.openAdd() }
        }
    }
    Timer {
        running: shot.on && shot.slow
        interval: 2500
        onTriggered: {
            if (shot.page === "menu" && app.menuMod) app.sheet = "modmenu"
            else if (shot.page === "check") app.startPlay()
        }
    }
    Timer {
        running: shot.on
        interval: shot.slow ? 4500 : 1500
        onTriggered: app.contentItem.grabToImage(function (r) { r.saveToFile(shot.path); Qt.quit() })
    }

    // ---------------------------------------------------------------- size
    readonly property bool tv: look.layout === "tv"
    readonly property real fit: Math.min(3, Math.max(1, Math.min(width / 883, height / 514))) * (tv ? 1.5 : 1)
    readonly property real s: fit * look.textSize / 100
    function px(v) { return Math.round(v * s) }

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
        }
        function onRulesJsonChanged() { app.rules = app.parse(Turbine.rulesJson, null); app.offerLoader() }
        function onDownloadsJsonChanged() {
            app.downloads = app.parse(Turbine.downloadsJson, [])
            app.addIndex = Math.max(0, Math.min(app.addIndex, app.downloads.length - 1))
        }
        function onMessage(text, level) { app.say(text, level) }
        function onChecked(json) { app.checked(app.parse(json, [])) }
        function onLaunch(url) { if (!shot.on) Qt.openUrlExternally(url) }
        function onPad(button, pressed) { pressed ? app.press(button) : app.release(button) }
    }
    onGameChanged: {
        if (!game) return
        seen.lastGame = game.id
        moving = false
        modIndex = 0
        if (tab >= 2) tab = 0
        Turbine.select(game.id)
    }
    Component.onCompleted: {
        load()
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
        if (msgShown) return { level: ({ work: "busy", ok: "ok", warn: "warn", error: "stop" })[msgLevel] || "ok", words: msgText, detail: "" }
        if (Turbine.pending > 0)
            return { level: "busy", words: plural(Turbine.pending, "change", "changes") + " waiting", detail: "They go into the game in a few seconds" }
        if (needLoader.length > 0)
            return { level: "warn", words: plural(needLoader.length, "mod needs ", "mods need ") + loaderShort, detail: "The game still plays, without " + (needLoader.length === 1 ? "that one" : "those") }
        const on = mods.filter(m => m.enabled).length
        const found = game.launcher === "Steam" ? "Found in Steam" : "Found in Heroic (" + game.storeLabel + ")"
        return { level: "ok", words: "Ready to play", detail: !modsLoaded ? found : mods.length === 0 ? "No mods yet · " + found : on + " of " + mods.length + " mods on" }
    }

    // ---------------------------------------------------------------- applying changes
    // Changes go into the game 3 seconds after the last edit, when Turbine loses focus and when it closes.
    Timer { id: autoApply; interval: 3000; onTriggered: if (!shot.on) Turbine.applyNow() }
    function edited() { autoApply.restart() }
    onActiveChanged: {
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
        mods = mods.map(x => x.id === m.id ? Object.assign({}, x, { enabled: on }) : x)
        Turbine.setEnabled(m.id, on)
        edited()
        if (on && m.needsLoader && loaderMissing) { loaderFor = [m.name]; openLoader() }
    }
    function moveMod(m, to) {
        const from = mods.findIndex(x => x.id === m.id)
        if (from < 0 || to === from) return
        let list = mods.slice()
        list.splice(from, 1)
        list.splice(to, 0, m)
        mods = list
        modIndex = to
        Turbine.moveTo(m.id, to)
        edited()
    }
    function uninstall(m) {
        mods = mods.filter(x => x.id !== m.id)
        Turbine.removeMod(m.id)
        edited()
        modIndex = Math.max(0, Math.min(modIndex, mods.length - 1))
        toast.showUndo(m.id, m.name)
    }
    function putBack(id) {
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
        if (moving) endMove()
        say("Checking before Play...", "work")
        Turbine.check()
    }
    function checked(list) {
        checks = list
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
        sheet = ""
        if (what === "Play anyway") Turbine.play(game.id)
        else if (what === "Play original") Turbine.playOriginal()
    }

    // ---------------------------------------------------------------- game menu
    // The setup to go back to: the one saved before the newest (the newest is what's in the game now).
    readonly property var lastGood: {
        if (setups.length === 0) return null
        const s = setups.slice().sort((a, b) => b.savedAt - a.savedAt)
        return s.length > 1 && Turbine.pending === 0 ? s[1] : s[0]
    }
    function when(sec) { return Qt.formatDateTime(new Date(sec * 1000), "MMM d, h:mm AP") }
    readonly property var gameMenuItems: [
        { icon: "play", title: "Play original", detail: "Starts the game with no mods. Nothing is deleted.", ok: true },
        { icon: "back", title: "Restore last working setup",
          detail: lastGood ? "From " + when(lastGood.savedAt) + " · " + plural(lastGood.modsOn, "mod", "mods") + " on" : "No saved setup yet. One is saved each time changes go into the game.",
          ok: lastGood !== null },
        { icon: "info", title: "Game details", detail: "What Turbine found for this game", ok: true }
    ]
    function gameMenuRun(i) {
        const item = gameMenuItems[i]
        if (!item.ok) return
        if (i === 0) { sheet = ""; Turbine.playOriginal() }
        else if (i === 1) { sheet = ""; Turbine.restoreSetup(lastGood.id) }
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
    ].concat(ordered ? [
        { icon: "move", title: "Move to top", detail: "First in the load order. Mods below it win when files overlap.", act: "top" },
        { icon: "move", title: "Move to bottom", detail: "Last in the load order. It wins when files overlap.", act: "bottom" }
    ] : []).concat([
        { icon: "remove", title: "Uninstall", detail: "Kept in Recently removed for 7 days", act: "remove", danger: true }
    ])
    property int menuIndex: 0
    function modMenuRun(i) {
        const item = modMenuItems[i], m = menuMod
        if (!item || !m) return
        sheet = ""
        if (item.act === "toggle") toggle(m)
        else if (item.act === "top") moveMod(m, 0)
        else if (item.act === "bottom") moveMod(m, mods.length - 1)
        else if (item.act === "remove") uninstall(m)
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
    readonly property int railCount: games.length + 2
    onSheetChanged: { menuIndex = 0; if (sheet === "" ) offerLoader() }

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
        act(b)
    }
    function release(b) {
        if (b === repeatButton) { repeater.stop(); repeatButton = "" }
    }

    Item {
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
            if (b) { app.act(b); e.accepted = true }
        }
    }

    function switchGame(step) {
        if (games.length === 0) return
        current = (current + step + games.length) % games.length
        if (page !== "game") page = "game"
        railIndex = current
    }
    function openSettings() { page = "settings"; zone = "settings"; setRow = 0 }

    function sheetAct(b) {
        if (sheet === "details" || sheet === "notice") {
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
            if (b === "up") menuIndex = Math.max(0, menuIndex - 1)
            else if (b === "down") menuIndex = Math.min(gameMenuItems.length - 1, menuIndex + 1)
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
            else if (b === "a" && loaderIndex === 0 && !loaderSourceShown) loaderSourceShown = true
            else if (b === "a" || b === "b") sheet = ""
        }
    }

    function act(b) {
        if (sheet !== "") { sheetAct(b); return }
        if (moving) {
            if (b === "up") stepMove(-1)
            else if (b === "down") stepMove(1)
            else if (b === "a" || b === "b" || b === "y") endMove()
            return
        }
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
            const rows = setSection === 0 ? lookRows.length : 1
            if (b === "b") { page = "game"; zone = "main" }
            else if (b === "lt") { setSection = 0; setRow = 0 }
            else if (b === "rt") { setSection = 1; setRow = 0 }
            else if (b === "up") setRow = Math.max(0, setRow - 1)
            else if (b === "down") setRow = Math.min(rows - 1, setRow + 1)
            else if (setSection === 0 && (b === "left" || b === "right")) lookRows[setRow].step(b === "left" ? -1 : 1)
            else if (setSection === 0 && b === "a") lookRows[setRow].step(1)
            else if (setSection === 1 && b === "a") { page = "perf"; zone = "perf"; perfIndex = 0 }
            else if (b === "left" && setSection === 0) zone = "rail"
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
        if (i < games.length) { current = i; page = "game"; zone = "main" }
        else if (i === games.length) toast.showPlain("Turbine finds Steam and Heroic games by itself. Install a game and it shows up here.")
        else openSettings()
    }


    // ---------------------------------------------------------------- Look settings rows
    readonly property var lookRows: [
        { label: "Theme", options: ["brass", "black", "light"], names: ["Brass", "Black", "Light"], key: "theme",
          step: (d) => cycle("theme", ["brass", "black", "light"], d) },
        { label: "Accent", options: Object.keys(c.swatches), key: "accent", swatch: true,
          step: (d) => cycle("accent", ["brass", "iron", "copper", "jade", "fjord", "heather", "ember"], d) },
        { label: "Text size", key: "textSize", slider: true,
          step: (d) => { look.textSize = Math.max(75, Math.min(175, look.textSize + d * 25)) } },
        { label: "Contrast", options: [false, true], names: ["Standard", "High"], key: "highContrast",
          step: (d) => { look.highContrast = !look.highContrast } },
        { label: "Color vision", options: ["typical", "red", "green", "blue", "none"], names: ["Typical", "Red", "Green", "Blue", "None"], key: "vision",
          step: (d) => cycle("vision", ["typical", "red", "green", "blue", "none"], d) },
        { label: "Layout", options: ["handheld", "tv"], names: ["Handheld", "TV"], key: "layout",
          step: (d) => cycle("layout", ["handheld", "tv"], d) }
    ]
    function cycle(key, list, d) {
        const i = list.indexOf(look[key])
        look[key] = list[(i + d + list.length) % list.length]
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
                // The app icon; a plain "T" if it can't be loaded.
                Image {
                    id: logo
                    width: app.px(44); height: app.px(44)
                    anchors.horizontalCenter: parent.horizontalCenter
                    source: "file:///app/share/icons/hicolor/128x128/apps/io.github.ravenor907.TurbineModManager.png"
                    sourceSize.width: app.px(88)
                    smooth: true
                    mipmap: true
                    visible: status === Image.Ready
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
                Rectangle {
                    width: app.px(44); height: app.px(44); radius: app.px(8)
                    readonly property bool focused: app.zone === "rail" && app.railIndex === app.games.length + 1
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
                        visible: app.page !== "game"
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
                        visible: app.page === "game" && (none || loading || empty || app.tab === 2)
                        title: none ? "No games yet" : loading ? "Reading mods..." : empty ? "No mods yet" : "Load order"
                        body: none ? "Turbine looks in Steam's libraries, including SD cards, and in Heroic. Install a game and it shows up here."
                              : loading ? ""
                              : empty ? "Add mods from zip files in your Downloads folder. Download a mod's zip file in Desktop Mode first."
                              : "Sorting comes in a later build. For now the order on the Mods tab is the load order: a mod lower in the list wins when two mods change the same file."
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
                                badge: app.moving && entry.foc ? "Moving" : (entry.m.needsLoader && app.loaderMissing ? "Needs " + app.loaderShort : "")
                                badgeLevel: app.moving && entry.foc ? "info" : "warn"
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

                    // Get Mods tab
                    EmptyState {
                        ui: kit
                        anchors.fill: parent
                        visible: app.page === "game" && app.game !== null && app.tab === 1
                        title: "Get Mods"
                        body: "Add mods from zip files in your Downloads folder, for example ones you downloaded in Desktop Mode."
                        buttonKey: "a"
                        buttonLabel: "Add mods from Downloads"
                        buttonFocused: app.zone === "main"
                        note: "Browsing and searching mods online comes in a later build."
                        onActivated: app.openAdd()
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
                                model: ["Look", "About"]
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
                        Column {
                            width: parent.width - app.px(184)
                            spacing: app.px(4)
                            visible: app.setSection === 0
                            Repeater {
                                model: app.lookRows
                                Rectangle {
                                    id: srow
                                    required property int index
                                    required property var modelData
                                    readonly property bool foc: app.zone === "settings" && app.setRow === index
                                    width: parent.width; height: app.px(50); radius: app.px(6)
                                    color: foc ? c.focusFill : c.card
                                    border.color: c.accent
                                    border.width: foc ? app.px(2) : 0
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
                                                    readonly property bool on: look[srow.modelData.key] === srow.modelData.options[index]
                                                    width: (parent.width - app.px(4) * ((srow.modelData.names || []).length - 1)) / (srow.modelData.names || [1]).length
                                                    height: app.px(32); radius: app.px(5)
                                                    color: on ? c.raised : c.ground
                                                    Rectangle { visible: parent.on; anchors.bottom: parent.bottom; width: parent.width; height: app.px(2); color: c.accent }
                                                    Label { anchors.centerIn: parent; text: parent.modelData; font.pixelSize: app.px(14); font.bold: parent.on; color: parent.on ? c.text : c.muted }
                                                    TapHandler { onTapped: { app.setRow = srow.index; look[srow.modelData.key] = srow.modelData.options[parent.index] } }
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
                                                    TapHandler { onTapped: look.accent = parent.modelData }
                                                }
                                            }
                                            Label { text: look.accent.charAt(0).toUpperCase() + look.accent.substring(1); color: c.soft; font.pixelSize: app.px(14); anchors.verticalCenter: parent.verticalCenter }
                                        }
                                        // Text size
                                        Row {
                                            visible: srow.modelData.slider === true
                                            anchors.verticalCenter: parent.verticalCenter
                                            width: parent.width
                                            spacing: app.px(12)
                                            Rectangle {
                                                width: parent.width - app.px(70); height: app.px(6); radius: height / 2
                                                anchors.verticalCenter: parent.verticalCenter
                                                color: c.line
                                                Rectangle { width: parent.width * (look.textSize - 75) / 100; height: parent.height; radius: parent.radius; color: c.accent }
                                                Rectangle {
                                                    x: parent.width * (look.textSize - 75) / 100 - width / 2
                                                    anchors.verticalCenter: parent.verticalCenter
                                                    width: app.px(18); height: width; radius: width / 2; color: c.accentHi
                                                }
                                            }
                                            Label { text: look.textSize + "%"; color: c.accentText; font.bold: true; anchors.verticalCenter: parent.verticalCenter }
                                        }
                                    }
                                    TapHandler { onTapped: app.setRow = srow.index }
                                }
                            }
                        }
                        Column {
                            width: parent.width - app.px(184)
                            spacing: app.px(8)
                            visible: app.setSection === 1
                            Label { text: "Turbine Mod Manager " + Turbine.version; font.family: app.titleFont; font.pixelSize: app.px(19); font.bold: true }
                            Label { width: parent.width; wrapMode: Text.WordWrap; elide: Text.ElideNone; color: c.soft; text: "A mod manager for SteamOS handhelds. Free software under the GNU General Public License, version 3 or later. Licenses of the parts it uses are in the licenses folder of the project." }
                            Rectangle {
                                width: perfRow.implicitWidth + app.px(28); height: app.px(40); radius: app.px(8)
                                color: app.zone === "settings" ? c.focusFill : c.card
                                border.color: c.accent
                                border.width: app.zone === "settings" ? app.px(2) : 0
                                Hint { id: perfRow; anchors.centerIn: parent; k: "a"; label: "Scrolling test (2,000 rows)" }
                                TapHandler { onTapped: { app.page = "perf"; app.zone = "perf" } }
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
                    ] : []
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
        gameName: app.game ? app.game.name : ""
        restoreDate: app.lastGood ? "From " + app.when(app.lastGood.savedAt) + " · " + app.plural(app.lastGood.modsOn, "mod", "mods") + " on" : ""
        restoreEmptyText: "Nothing saved yet. Turbine saves your setup each time changes go into the game."
        extraItems: [{ key: "details", label: "Game details", detail: "What Turbine found for this game", icon: "info" }]
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
        focusIndex: app.loaderIndex
        onActivated: (index, key) => { app.loaderIndex = index; app.act(key || "a") }
        onClosed: app.act("b")
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
            Hint { visible: app.sheet === "modmenu" || app.sheet === "gamemenu" || app.sheet === "add"; k: "▲▼"; label: "Choose" }
            Hint { visible: app.sheet === "check" || app.sheet === "loader"; k: "◀▶"; label: "Choose" }
            Hint { visible: app.sheet !== ""; k: "b"; label: app.sheet === "check" ? "Wait" : "Close" }
            // Game page
            Hint { visible: legend.gamePage && !app.moving && app.tab === 0 && app.ordered && app.mods.length > 1; k: "y"; label: "Reorder" }
            Hint { visible: legend.gamePage && !app.moving; k: "lb+rb"; label: "Switch game" }
            Hint { visible: legend.gamePage && !app.moving; k: "view"; label: "Game menu" }
            Hint { visible: app.sheet === "" && app.page === "settings"; k: "lt+rt"; label: "Sections" }
            Hint { visible: app.sheet === "" && app.page === "settings" && app.setSection === 0; k: "◀▶"; label: "Change" }
            Hint { visible: app.sheet === "" && app.page !== "game"; k: "b"; label: "Back" }
            Hint { visible: app.sheet === "" && !app.moving && app.zone !== "rail"; k: "◀"; label: "Games" }
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
}
