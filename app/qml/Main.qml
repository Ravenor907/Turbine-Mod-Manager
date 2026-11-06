import QtQuick
import QtQuick.Window
import QtQuick.Shapes
import QtCore
import Turbine

// Turbine Mod Manager's whole interface for 0.1: the game rail, a game's page, Settings, game details, a notice for
// systems that aren't SteamOS and a 2,000 row list to test scrolling. Everything is sized from a design size the way
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
    }
    Timer {
        running: shot.on
        interval: 1500
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
        readonly property color muted: hc ? "#E0E0E0" : light ? "#5E5038" : "#A08C68"
        readonly property var swatches: ({ brass: "#C99E52", iron: "#A9B4BE", copper: "#B07A5A", jade: "#6FAE8C",
                                           fjord: "#6E9CC8", heather: "#A58BC6", ember: "#CF6F54" })
        readonly property color accentBase: look.vision === "none" ? "#BDBDBD" : (swatches[look.accent] || swatches.brass)
        readonly property color accent: hc ? "#FFD54A" : light ? Qt.darker(accentBase, 1.7) : accentBase
        readonly property color accentHi: hc ? "#FFE680" : light ? Qt.darker(accentBase, 1.4) : Qt.lighter(accentBase, 1.22)
        readonly property color accentLo: light ? Qt.darker(accentBase, 2.1) : Qt.darker(accentBase, 1.35)
        readonly property color onAccent: light ? "#FFFFFF" : "#0B0A08"
        readonly property color focusFill: hc ? "#333300" : light ? Qt.lighter(accentBase, 1.55) : Qt.darker(accentBase, 4.2)
        // Status colors stay apart for each kind of color vision; words always carry the meaning too.
        readonly property color ok: look.vision === "none" ? soft : (look.vision === "red" || look.vision === "green") ? "#6E9CC8" : light ? "#4E6B2E" : "#9DBF7A"
        readonly property color warn: look.vision === "none" ? text : (look.vision === "red" || look.vision === "green") ? "#E0A040" : light ? "#9A4A2A" : "#E6A98A"
    }

    // ---------------------------------------------------------------- data
    property var games: []
    property int current: 0
    readonly property var game: games.length > 0 ? games[Math.min(current, games.length - 1)] : null
    function load() {
        Turbine.scan()
    }
    Connections {
        target: Turbine
        function onGamesChanged() {
            let list = []
            try { list = JSON.parse(Turbine.games) } catch (e) { list = [] }
            app.games = list
            let i = list.findIndex(g => g.id === seen.lastGame)
            app.current = i >= 0 ? i : 0
        }
        function onPad(button, pressed) { pressed ? app.press(button) : app.release(button) }
    }
    Component.onCompleted: {
        load()
        if (shot.page === "settings") { page = "settings"; zone = "settings" }
        else if (shot.page === "details") sheet = "details"
        else if (shot.page === "perf") { page = "perf"; zone = "perf" }
        else if (shot.page === "notice") sheet = "notice"
        else if (!shot.on && !Turbine.steamos && !seen.notSteamOS) sheet = "notice"
    }
    onCurrentChanged: if (game) seen.lastGame = game.id

    // ---------------------------------------------------------------- controller and keyboard
    // zone: where the controller focus is. "rail" (games and buttons on the left), "main" (the game page),
    // "settings", "perf". sheet: an overlay on top ("details" or "notice").
    property string page: "game"          // game, settings, perf
    property string zone: "main"
    property string sheet: ""
    property int railIndex: 0             // 0..games.length-1, then "add", then "settings"
    property int tab: 0                   // 0 Mods, 1 Get Mods
    property int setSection: 0            // 0 Look, 1 About
    property int setRow: 0
    property int perfIndex: 0
    readonly property int railCount: games.length + 2

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
    function play() {
        if (!game) return
        if (game.store === "steam") Qt.openUrlExternally("steam://rungameid/" + game.storeId)
        else toast.show("Play for Heroic games comes in the next test build. Start it from Heroic for now.")
    }
    function openSettings() { page = "settings"; zone = "settings"; setRow = 0 }

    function act(b) {
        if (sheet !== "") {
            if (b === "a" || b === "b" || (b === "view" && sheet === "details")) {
                if (sheet === "notice") seen.notSteamOS = true
                sheet = ""
            }
            return
        }
        if (b === "start") { play(); return }
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
            else if (b === "lt") tab = 0
            else if (b === "rt") tab = 1
            else if (b === "view" && game) sheet = "details"
            else if (b === "a" && games.length === 0) load()
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
        else if (i === games.length) toast.show("Turbine finds Steam and Heroic games by itself. Install a game and it shows up here.")
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
        implicitWidth: Math.max(app.px(20), keyText.implicitWidth + app.px(10))
        implicitHeight: app.px(20)
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
            font.pixelSize: app.px(11)
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
            Key { required property string modelData; k: modelData; anchors.verticalCenter: parent.verticalCenter }
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
                // Placeholder logo until the owner's icon is ready.
                Rectangle {
                    width: app.px(44); height: app.px(44); radius: width / 2
                    anchors.horizontalCenter: parent.horizontalCenter
                    gradient: Gradient {
                        GradientStop { position: 0; color: c.accentHi }
                        GradientStop { position: 1; color: c.accentLo }
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
                    Text {
                        anchors.centerIn: parent
                        width: parent.width - app.px(320)
                        horizontalAlignment: Text.AlignHCenter
                        text: app.page === "settings" ? "SETTINGS" : app.page === "perf" ? "SCROLLING TEST"
                              : app.game ? app.game.name.toUpperCase() : "TURBINE MOD MANAGER"
                        color: c.text
                        font.family: app.titleFont
                        font.pixelSize: app.px(17)
                        font.bold: true
                        font.letterSpacing: app.px(1.5)
                        elide: Text.ElideRight
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
                        gradient: Gradient {
                            GradientStop { position: 0; color: c.accentHi }
                            GradientStop { position: 1; color: c.accentLo }
                        }
                        Row {
                            id: playRow
                            anchors.centerIn: parent
                            spacing: app.px(8)
                            Key { k: "start"; anchors.verticalCenter: parent.verticalCenter }
                            Text { text: "▶ Play"; color: c.onAccent; font.family: app.bodyFont; font.pixelSize: app.px(15); font.bold: true; anchors.verticalCenter: parent.verticalCenter }
                        }
                        TapHandler { onTapped: app.play() }
                    }
                }

                // Status line: always says what's going on, in words.
                Rectangle {
                    width: parent.width
                    height: app.px(44)
                    radius: app.px(6)
                    color: c.panel
                    Row {
                        anchors.verticalCenter: parent.verticalCenter
                        x: app.px(12)
                        spacing: app.px(10)
                        Rectangle {
                            width: app.px(22); height: width; radius: width / 2
                            anchors.verticalCenter: parent.verticalCenter
                            color: Turbine.problems !== "" ? c.warn : c.ok
                            Text { anchors.centerIn: parent; text: Turbine.problems !== "" ? "!" : "✓"; color: c.onAccent; font.pixelSize: app.px(13); font.bold: true }
                        }
                        Label {
                            text: app.page === "settings" ? "Changes show right away"
                                  : app.page === "perf" ? "2,000 rows. Hold the D-pad or stick to scroll fast."
                                  : !app.game ? "No games found yet"
                                  : app.game.launcher === "Steam" ? "Found in Steam" : "Found in Heroic (" + app.game.storeLabel + ")"
                            font.bold: true
                        }
                        Label {
                            text: app.page === "game" && app.game ? app.runtimeWords(app.game) + " · " + app.game.drive : ""
                            color: c.muted
                            font.pixelSize: app.px(14)
                        }
                    }
                }

                // Tabs (game page only)
                Rectangle {
                    visible: app.page === "game" && app.game !== null
                    width: parent.width
                    height: app.px(38)
                    radius: app.px(6)
                    color: c.panel
                    Row {
                        anchors.fill: parent
                        anchors.margins: app.px(3)
                        spacing: app.px(4)
                        Key { k: "lt"; anchors.verticalCenter: parent.verticalCenter }
                        Repeater {
                            model: ["Mods", "Get Mods"]
                            Rectangle {
                                required property int index
                                required property string modelData
                                width: (parent.width - app.px(56) - app.px(8)) / 2
                                height: parent.height
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
                    width: parent.width
                    height: mainCol.height - y
                    clip: true

                    // Game page: no mods yet in 0.1, so it explains what's coming.
                    Column {
                        visible: app.page === "game"
                        anchors.centerIn: parent
                        width: Math.min(parent.width, app.px(560))
                        spacing: app.px(10)
                        Label {
                            width: parent.width
                            horizontalAlignment: Text.AlignHCenter
                            font.family: app.titleFont
                            font.pixelSize: app.px(19)
                            font.bold: true
                            text: !app.game ? "No games yet" : app.tab === 0 ? "No mods yet" : "Getting mods"
                        }
                        Label {
                            width: parent.width
                            horizontalAlignment: Text.AlignHCenter
                            wrapMode: Text.WordWrap
                            elide: Text.ElideNone
                            color: c.soft
                            text: !app.game ? "Turbine looks in Steam's libraries, including SD cards, and in Heroic. Install a game and it shows up here."
                                  : app.tab === 0 ? "This test build finds your games. Turning mods on and off, and Play with mods, come in the next build."
                                  : "Browsing Thunderstore and installing mods from files come in a later build."
                        }
                        Rectangle {
                            visible: !app.game
                            anchors.horizontalCenter: parent.horizontalCenter
                            width: againRow.implicitWidth + app.px(28); height: app.px(40); radius: app.px(8)
                            color: c.focusFill
                            border.color: c.accent
                            border.width: app.px(2)
                            Hint { id: againRow; anchors.centerIn: parent; k: "a"; label: "Look again" }
                            TapHandler { onTapped: app.load() }
                        }
                        Label {
                            visible: app.game !== null
                            width: parent.width
                            horizontalAlignment: Text.AlignHCenter
                            color: c.muted
                            font.pixelSize: app.px(14)
                            text: "Press View to see what Turbine found for this game."
                        }
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
                                            Label { text: look.textSize + "%"; color: c.accent; font.bold: true; anchors.verticalCenter: parent.verticalCenter }
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
                                    Label { anchors.centerIn: parent; text: prow.off ? "OFF" : "ON"; font.pixelSize: app.px(13); font.bold: true; color: prow.off ? c.muted : c.accentHi }
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

            // Legend: only the controls that matter here.
            Rectangle {
                id: legend
                anchors.bottom: parent.bottom
                anchors.bottomMargin: app.px(8)
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.leftMargin: app.px(14)
                anchors.rightMargin: app.px(14)
                height: app.px(36)
                radius: app.px(6)
                color: c.panel
                Row {
                    anchors.verticalCenter: parent.verticalCenter
                    x: app.px(12)
                    spacing: app.px(16)
                    Hint { visible: app.page === "game" && app.game !== null; k: "lb+rb"; label: "Switch game" }
                    Hint { visible: app.page === "game" && app.game !== null; k: "view"; label: "Game details" }
                    Hint { visible: app.page === "settings"; k: "lt+rt"; label: "Sections" }
                    Hint { visible: app.page === "settings" && app.setSection === 0; k: "◀▶"; label: "Change" }
                    Hint { visible: app.page !== "game"; k: "b"; label: "Back" }
                    Hint { visible: app.zone !== "rail"; k: "◀"; label: "Games" }
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
        }
    }

    // ---------------------------------------------------------------- sheets
    Rectangle {
        anchors.fill: parent
        visible: app.sheet !== ""
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

    // ---------------------------------------------------------------- toast
    Rectangle {
        id: toast
        property string message: ""
        function show(m) { message = m; toastTimer.restart() }
        visible: toastTimer.running
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: app.px(56)
        width: Math.min(parent.width - app.px(120), toastText.implicitWidth + app.px(32))
        height: toastText.implicitHeight + app.px(20)
        radius: app.px(8)
        color: c.raised
        border.color: c.accentLo
        Label { id: toastText; anchors.centerIn: parent; width: parent.width - app.px(32); text: toast.message; wrapMode: Text.WordWrap; elide: Text.ElideNone; horizontalAlignment: Text.AlignHCenter }
        Timer { id: toastTimer; interval: 3500 }
    }
}
