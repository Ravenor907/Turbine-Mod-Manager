import QtQuick
import QtQuick.Shapes
import Turbine

// The bottom bar (the only bar on the game screen). Left to right: the game's name with the tab and its status
// under it (a warning there is tappable), LB RB and the tabs, then the legend (Main.qml's legendHints, the ones that
// fit), the quick wheel's button, and Start's hint and Play on the right. Elsewhere it is the legend and View alone.
// Scrolling down slides it away with the rail. Main.qml sets its left margin (the rail's width).
//
// app (required): Main.qml's window. kit (required): Main.qml's kit (colors and motion, see COMPONENTS.md).
Rectangle {
    id: legend
    required property var app
    required property var kit
    readonly property var c: kit.c
    readonly property Motion mo: kit.motion
    // Main.qml (the focus ring) and the build's checks reach these.
    property alias barLb: barLb
    property alias barRb: barRb
    property alias barName: barName
    property alias startHint: startHint
    property alias startHintKey: startHintKey
    property alias startRing: startRing
    property alias playBtn: playBtn
    // One hint of the legend from its entry in legendHints: [key, words, "hold", second key, its words].
    component LegendHint: Hint {
        required property var modelData
        inBar: true
        k: modelData[0]
        label: modelData[1]
        pre: modelData[2] || ""
        k2: modelData[3] || ""
        label2: modelData[4] || ""
    }
    // Shown unless the content went full screen (a sheet or the wheel always brings it back).
    readonly property bool shown: visible && !(app.barsHidden && app.sheet === "" && !app.wheelOpen)
    anchors.bottom: parent.bottom
    anchors.bottomMargin: shown ? 0 : -height - app.px(2)
    Behavior on anchors.bottomMargin { enabled: mo.moves; NumberAnimation { duration: mo.moveBase; easing.type: mo.enter } }
    anchors.left: parent.left
    anchors.right: parent.right
    // A taller bar with room between its groups.
    height: Math.max(app.px(60), app.tpx(30) + app.px(10), barGameCol.implicitHeight + app.px(12))
    color: c.panel
    // Over the quick wheel's backdrop too (the wheel is z 70), so it never dims.
    z: app.wheelOpen ? 80 : 40
    // The Nexus page has its legend in its own bar.
    // The bar stays for touch too: its hints are the touch buttons.
    visible: app.sheet !== "tutorial" && (app.barGame || app.page !== "browser" || app.wheelOpen || app.sheet !== "")
    // On up to the quick wheel (its Close is here); the tutorial covers it.
    enabled: app.topLayer <= 3
    // The bar takes every press on it, so a tap on the bar never reaches a sheet's backdrop or a button beneath
    // (one tap on a hint is one press of that button, like the controller).
    MouseArea {  // overlay-check: the bar takes every press on it, never a backdrop
        anchors.fill: parent
        z: -1
        acceptedButtons: Qt.AllButtons
        preventStealing: true
        onWheel: (w) => { w.accepted = true }
        onPressed: app.coverPress(3)
        onReleased: app.coverEnd()
        onCanceled: app.coverEnd()
    }
    Rectangle { width: parent.width; height: Math.max(1, app.px(1)); color: c.line }
    readonly property real gap: app.px(18)
    readonly property real edge: app.px(16)
    // Touch: a sideways swipe on the bar is LB or RB.
    DragHandler {
        enabled: app.barGame && app.sheet === "" && !app.wheelOpen
        target: null
        xAxis.enabled: true; yAxis.enabled: false
        acceptedDevices: PointerDevice.TouchScreen
        onActiveChanged: if (!active && Math.abs(activeTranslation.x) > app.px(60)) { app.note("touch", "Swipe on the bottom bar"); app.switchTab(activeTranslation.x < 0 ? 1 : -1) }
    }
    // ---- Left: the game (its whole name, wrapping to two lines before it is ever cut) and the tabs.
    Row {
        id: barRow
        x: legend.edge
        height: parent.height
        spacing: legend.gap
        Column {
            id: barGameCol
            visible: app.barGame || app.barPlace !== ""
            // Measured from the words themselves (never from the Texts' own sizes, which follow this width).
            TextMetrics { id: barNameMetrics; font: barName.font; text: barName.text }
            TextMetrics { id: barSubMetrics; font: barSub.font; text: barSub.text }
            // The name gets up to 34% of the bar; past that it wraps onto a second line.
            readonly property real most: Math.max(app.px(150), legend.width * 0.34)
            readonly property real want: Math.min(most, Math.ceil(Math.max(barNameMetrics.advanceWidth, Math.min(barSubMetrics.advanceWidth, most))) + app.px(2))
            width: visible ? want : 0
            anchors.verticalCenter: parent.verticalCenter
            Text {
                id: barName
                width: barGameCol.want
                wrapMode: Text.Wrap
                maximumLineCount: 2
                elide: Text.ElideRight
                text: app.barPlace !== "" ? app.barPlace : app.game ? app.shortGame(app.game.name) : ""
                color: c.accentText; font.family: app.bodyFont; font.pixelSize: app.fpx(15); font.bold: true
            }
            Text {
                id: barSub
                width: barGameCol.want; elide: Text.ElideRight
                // On a mod page: where it is ("Get mods > SkyUI", "Mod 3 of 30"); Settings: the section.
                text: app.page === "settings" ? app.sec : app.page === "place" ? (app.placeData && app.placeData.pending ? app.placeData.pending.name : "")
                      : app.modPage ? "Get mods > " + (app.detailsData && app.detailsData.name ? app.detailsData.name : app.detailsItem ? app.detailsItem.name : "") + app.pageResultWords : app.barStatus
                color: app.statusNow.level === "stop" ? c.stop : app.statusNow.level === "warn" ? c.warn : c.soft
                font.family: app.bodyFont; font.pixelSize: app.fpx(12)
                font.underline: app.barTappable && (app.statusNow.level === "warn" || app.statusNow.level === "stop")
            }
            Accessible.role: app.barTappable ? Accessible.Button : Accessible.StaticText
            Accessible.name: barName.text + ", " + app.barStatus
            HoverHandler { enabled: app.barTappable && app.sheet === "" && !app.wheelOpen; onHoveredChanged: app.hover(parent, hovered) }
            TapHandler { enabled: app.barTappable && app.sheet === "" && !app.wheelOpen; onTapped: app.statusTap() }
        }
        Rectangle { visible: app.barGame; width: Math.max(1, app.px(1)); height: app.px(34); color: c.line; anchors.verticalCenter: parent.verticalCenter }
        // LB RB and the tabs (the one on screen brighter); a tap on a name opens that tab.
        Row {
            id: barTabs
            // On a mod page LB and RB step through the results (the line on the left says which).
            visible: app.barGame && !app.modPage
            anchors.verticalCenter: parent.verticalCenter
            // After the mouse the pages are buttons by name (My mods, Get mods, Load order), no caps.
            spacing: app.mouseLegend ? app.px(22) : app.px(6)
            leftPadding: app.mouseLegend ? app.px(10) : 0
            rightPadding: leftPadding
            // Dimmed where they do nothing (one tab, or a sheet open).
            Key { id: barLb; win: app; k: "lb"; opacity: app.tabs.length < 2 || app.sheet !== "" || app.tab === 0 ? 0.35 : 1; anchors.verticalCenter: parent.verticalCenter }
            Key { id: barRb; win: app; k: "rb"; opacity: app.tabs.length < 2 || app.sheet !== "" || app.tab >= app.tabs.length - 1 ? 0.35 : 1; anchors.verticalCenter: parent.verticalCenter }
            // No room for every name: only the tab LB or RB goes to (and nothing when even that doesn't fit).
            Text {
                visible: !legend.tabWordsFit && legend.tabShortFits && app.barTarget >= 0 && !app.mouseLegend
                text: app.barTarget >= 0 ? (app.barTarget < app.tab ? "back to " : "") + app.tabs[app.barTarget].replace(/ \(\d+\)$/, "") : ""
                color: c.soft; font.family: app.bodyFont; font.pixelSize: app.fpx(14)
                anchors.verticalCenter: parent.verticalCenter
                TapHandler { enabled: app.sheet === "" && !app.wheelOpen; onTapped: { app.note("touch", "Tab tapped: " + app.tabs[app.barTarget]); app.modPage = false; app.tab = app.barTarget } }
            }
            Repeater {
                model: legend.tabWordsFit || app.mouseLegend ? app.tabs : []
                Row {
                    required property int index
                    required property string modelData
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: app.px(6)
                    Text { visible: parent.index > 0 && !app.mouseLegend; text: "/"; color: c.muted; font.family: app.bodyFont; font.pixelSize: app.fpx(14); anchors.verticalCenter: parent.verticalCenter }
                    Text {
                        Rectangle {
                            visible: app.mouseLegend
                            z: -1
                            anchors.centerIn: parent
                            width: parent.width + app.px(20); height: Math.max(app.px(32), app.tpx(26)); radius: app.px(6)
                            color: app.tab === parent.parent.index ? c.focusFill : c.raised
                            border.color: app.tab === parent.parent.index ? c.accent : c.accentLo; border.width: Math.max(1, app.px(1))
                        }
                        text: parent.modelData.replace(/ \(\d+\)$/, "")
                        color: app.tab === parent.index ? c.text : c.muted
                        font.family: app.bodyFont; font.pixelSize: app.fpx(14); font.bold: app.tab === parent.index
                        anchors.verticalCenter: parent.verticalCenter
                        Accessible.role: Accessible.PageTab
                        Accessible.name: text + (app.tab === parent.index ? ", selected" : "") + ", " + (parent.index + 1) + " of " + app.tabs.length
                        HoverHandler { enabled: app.sheet === "" && !app.wheelOpen; onHoveredChanged: app.hover(parent, hovered) }
                        TapHandler { enabled: app.sheet === "" && !app.wheelOpen; onTapped: { const i = parent.parent.index; app.note("touch", "Tab tapped: " + app.tabs[i]); app.modPage = false; app.tab = i } }
                    }
                }
            }
        }
        Rectangle { visible: app.barGame; width: Math.max(1, app.px(1)); height: app.px(34); color: c.line; anchors.verticalCenter: parent.verticalCenter }
    }
    // ---- Right: Play (and Seamless Co-op where it is on), always whole; the wheel's button just before them.
    Row {
        id: playGroup
        anchors.right: parent.right
        anchors.rightMargin: legend.edge
        anchors.verticalCenter: parent.verticalCenter
        spacing: app.px(10)
        // What Start does: a tap is Settings, a 3 second hold plays (or shows the Play choice). A tap on
        // it is a tap of Start. The words go when the bar has no room for them; the cap stays.
        Rectangle {
            id: startHint
            visible: app.barGame && app.sheet === "" && !app.wheelOpen
            anchors.verticalCenter: parent.verticalCenter
            width: startHintRow.implicitWidth + app.px(16); height: Math.max(app.px(34), app.tpx(26)); radius: app.px(6)
            color: c.raised
            Row {
                id: startHintRow
                anchors.centerIn: parent
                spacing: app.px(6)
                // With the keyboard Settings is Esc, on a game's first page (elsewhere Esc goes back, so no cap).
                Key { id: startHintKey; win: app; k: app.keysLegend ? "b" : "start"; off: app.keysLegend && !app.escTop; anchors.verticalCenter: parent.verticalCenter }
                Column {
                    visible: legend.startWords || startHintKey.gone
                    anchors.verticalCenter: parent.verticalCenter
                    Text { text: "Settings"; color: c.soft; font.family: app.bodyFont; font.pixelSize: app.fpx(13) }
                    Text { visible: !app.mouseLegend; text: app.startHoldWords; color: c.muted; font.family: app.bodyFont; font.pixelSize: app.fpx(12) }
                }
            }
            Accessible.role: Accessible.Button
            Accessible.name: "Settings. Hold Start for 3 seconds: " + (app.startHoldPlays ? "Play" : "Play choice")
            HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
            TapHandler { onTapped: { if (!app.gate(3, "Start hint")) return; app.note("touch", "Start hint tapped"); app.startTap() } }
        }
        Rectangle {
            id: playBtn
            // A sheet whose own Start button is in the legend ("START Save"): Play steps aside.
            visible: app.barGame && !(app.sheet !== "" && app.legendHints.some(h => h[0] === "start"))
            width: playRow.implicitWidth + app.px(28); height: Math.max(app.px(40), app.tpx(30)); radius: app.px(6)
            // Dimmed while a sheet or the wheel is open.
            opacity: Turbine.busy || app.sheet !== "" || app.wheelOpen ? 0.55 : 1
            // The focus ring when Play has the focus (a mod just arrived).
            border.color: c.text
            border.width: app.zone === "play" && app.sheet === "" && !app.wheelOpen ? app.px(3) : 0
            gradient: Gradient {
                GradientStop { position: 0; color: c.accentHi }
                GradientStop { position: 1; color: c.accent }
            }
            Row {
                id: playRow
                anchors.centerIn: parent
                spacing: app.px(8)
                // Play waits for changes being saved: it says so.
                Text { text: (app.savingNow ? "Play (saving...)" : app.modHold && app.mods.some(m => m.enabled) ? "Play without mods" : "Play") + (app.seamlessIn && !app.savingNow ? (app.lastPlayChoice === "coop" ? " · Co-op" : " · Solo") : ""); color: app.onFill(c.accent); font.family: app.bodyFont; font.pixelSize: app.fpx(16); font.bold: true; anchors.verticalCenter: parent.verticalCenter }
            }
            // Start held: a brass ring fills here over 3 seconds (shown once the press is clearly a hold).
            Rectangle {
                id: startRing
                readonly property bool shown: app.startHeld > 0.1
                visible: shown
                anchors.centerIn: parent
                width: parent.height - app.px(6); height: width; radius: width / 2
                color: c.ground
                Shape {
                    anchors.fill: parent
                    preferredRendererType: Shape.CurveRenderer
                    ShapePath {
                        strokeColor: c.accentHi
                        strokeWidth: Math.max(2, app.px(4))
                        fillColor: "transparent"
                        capStyle: ShapePath.RoundCap
                        PathAngleArc {
                            centerX: startRing.width / 2; centerY: startRing.height / 2
                            radiusX: startRing.width / 2 - app.px(5); radiusY: radiusX
                            startAngle: -90
                            sweepAngle: 360 * app.startRingPart
                        }
                    }
                }
            }
            Accessible.role: Accessible.Button
            Accessible.name: "Play"
            HoverHandler { enabled: app.sheet === "" && !app.wheelOpen; onHoveredChanged: app.hover(parent, hovered) }
            // A long press is Play options (the wheel's Play options slot shows it too).
            TapHandler { enabled: app.sheet === "" && !app.wheelOpen; onTapped: { if (!app.gate(3, "Play")) return; app.playPressed() }
                         onLongPressed: { if (!app.gate(3, "Play")) return; app.note("touch", "Play held: Play options"); app.openPlayOptions() } }
        }
    }
    // The quick wheel's button: View's cap (a tap opens it too).
    Rectangle {
        id: wheelBtn
        anchors.right: playGroup.left
        anchors.rightMargin: playGroup.width > 0 ? legend.gap : 0
        anchors.verticalCenter: parent.verticalCenter
        width: wheelRow.implicitWidth + app.px(16); height: Math.max(app.px(34), app.tpx(26)); radius: app.px(6)
        color: app.wheelOpen ? c.focusFill : c.raised
        visible: app.sheet === "" || app.wheelOpen
        Row {
            id: wheelRow
            anchors.centerIn: parent
            spacing: app.px(6)
            Key { win: app; k: "view"; anchors.verticalCenter: parent.verticalCenter }
            Label { win: app; visible: legend.wheelWord || app.wheelOpen || app.mouseLegend; text: app.wheelOpen ? "Close" : "Wheel"; font.pixelSize: app.fpx(14); color: c.soft; anchors.verticalCenter: parent.verticalCenter }
        }
        HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
        TapHandler { onTapped: app.wheelPress() }
    }
    // ---- Middle: the hints, at most 4 (3 beside the wheel's button), in priority order (A, B, X, Y, then the
    // rest); the ones that don't fit are left out, never squeezed under Play or past the edge.
    Item {
        id: hintsBox
        readonly property real rightEdge: wheelBtn.visible ? wheelBtn.x - legend.gap : playGroup.width > 0 ? playGroup.x - legend.gap : legend.width - legend.edge
        x: barRow.x + barRow.width
        width: Math.max(0, rightEdge - x)
        height: parent.height
        clip: true
        Row {
            id: legendRow
            visible: true
            // The hints fade over when they change (another screen, another row, the pad, keys or the mouse).
            readonly property string said: app.legendHints.slice(0, legend.hintsFit).map(h => app.glyph(h[0]) + " " + h[1] + (h[3] ? app.glyph(h[3]) + h[4] : "")).join("|")
            onSaidChanged: if (mo.on && legend.visible) hintsFade.restart()
            NumberAnimation { id: hintsFade; target: legendRow; properties: "opacity"; from: 0.25; to: 1; duration: mo.base; easing.type: mo.enter }
            anchors.verticalCenter: parent.verticalCenter
            spacing: app.px(16)
            Repeater {
                model: app.legendHints.slice(0, legend.hintsFit)
                LegendHint { win: app }
            }
        }
    }
    // Every hint measured where nobody sees it, so the bar knows how many fit before showing them.
    Row {
        id: hintMeasure
        x: -100000; y: 0
        opacity: 0
        enabled: false
        spacing: legendRow.spacing
        Repeater {
            id: measureRep
            model: app.legendHints
            onItemAdded: legend.measureTick++
            onItemRemoved: legend.measureTick++
            LegendHint { win: app }
        }
    }
    readonly property int hintsMost: wheelBtn.visible ? 3 : 4
    property int measureTick: 0
    function hintsWidth(n) {
        let w = measureTick * 0
        for (let i = 0; i < Math.min(n, measureRep.count); i++) { const it = measureRep.itemAt(i); if (it) w += it.implicitWidth + (i > 0 ? legendRow.spacing : 0) }
        return w
    }
    readonly property int hintsFit: {
        const most = Math.min(app.legendHints.length, hintsMost, measureRep.count)
        let n = 0
        for (let i = 1; i <= most; i++) if (hintsWidth(i) <= hintsBox.width) n = i
        return n
    }
    // The tab names show when they fit beside two hints; otherwise LB and RB with the tab they go to; otherwise
    // LB and RB alone (the status line names the tab).
    TextMetrics { id: tabWordsMetrics; font.family: app.bodyFont; font.pixelSize: app.fpx(14); font.bold: true; text: app.tabs.map(t => t.replace(/ \(\d+\)$/, "")).join(" / ") + "  " }
    TextMetrics { id: wheelMetrics; font.family: app.bodyFont; font.pixelSize: app.fpx(14); text: "Wheel" }
    TextMetrics { id: tabShortMetrics; font.family: app.bodyFont; font.pixelSize: app.fpx(14); text: app.barTarget >= 0 ? "back to " + app.tabs[app.barTarget].replace(/ \(\d+\)$/, "") : "" }
    // (The keyboard's names for LB and RB are wider than the controller's.)
    readonly property real capsW: Math.max(app.px(70), barLb.implicitWidth + barRb.implicitWidth + app.px(12))
    // The Start hint's words ("Settings", "hold: Play choice") show when they fit beside the game, the
    // caps, the wheel's cap, Play and two hints. Measured from the words, never from the hint's own size.
    TextMetrics { id: startWordsMetrics; font.family: app.bodyFont; font.pixelSize: app.fpx(13); text: app.startHoldWords + "  " }
    readonly property bool startWords: width - edge * 2 - barGameCol.width - capsW - playBtn.width - startHintKey.implicitWidth - app.px(16 + 50) - gap * 6
                                       - hintsWidth(Math.min(2, app.legendHints.length)) >= startWordsMetrics.advanceWidth + app.px(6)
    // Room for the tab words: the bar less the game, the caps, Play, the wheel's cap and two hints.
    readonly property real barFree: width - edge * 2 - barGameCol.width - capsW - playGroup.width - app.px(50) - gap * 5 - hintsWidth(Math.min(2, app.legendHints.length))
    readonly property real tabWordsNeed: tabWordsMetrics.advanceWidth + app.px(6) * app.tabs.length * 2
    readonly property real wheelWordNeed: wheelMetrics.advanceWidth + app.px(6)
    readonly property real tabShortNeed: tabShortMetrics.advanceWidth + app.px(6)
    readonly property bool tabWordsFit: barFree >= tabWordsNeed
    readonly property bool tabShortFits: barFree >= tabShortNeed
    // The tab LB or RB goes to comes before the word Wheel.
    readonly property bool wheelWord: !app.barGame ? barFreeNoGame >= wheelWordNeed : barFree >= tabWordsNeed + wheelWordNeed || (!tabWordsFit && barFree >= tabShortNeed + wheelWordNeed)
    readonly property real barFreeNoGame: width - edge * 2 - app.px(50) - gap * 2 - hintsWidth(Math.min(hintsMost, app.legendHints.length))
    // The build's check: nothing in the bar overlaps anything else, Play, or the edge, and the game's name is
    // whole. Logged as "Bar check (<size>): <n> items, <p> problems".
    function check(tag) {
        if (!visible || !shown) return
        let items = []
        const add = (name, it) => { if (it && it.visible && it.width > 0) { const p = it.mapToItem(legend, 0, 0); items.push({ name: name, x: p.x, y: p.y, w: it.width, h: it.height }) } }
        add("game", barGameCol); add("tabs", barTabs)
        for (let i = 0; i < legendRow.children.length; i++) { const h = legendRow.children[i]; if (h.k !== undefined && h.visible) add("hint " + h.k.toUpperCase(), h) }
        add("start", startHint); add("wheel", wheelBtn); add("Play", playBtn)
        let bad = []
        for (const a of items) {
            if (a.x < -0.5 || a.x + a.w > legend.width + 0.5) bad.push(a.name + " past the edge (" + Math.round(a.x) + " to " + Math.round(a.x + a.w) + " of " + Math.round(legend.width) + ")")
            if (a.y < -0.5 || a.y + a.h > legend.height + 0.5) bad.push(a.name + " taller than the bar")
        }
        for (let i = 0; i < items.length; i++) for (let j = i + 1; j < items.length; j++) {
            const a = items[i], b = items[j]
            if (a.x < b.x + b.w - 0.5 && b.x < a.x + a.w - 0.5) bad.push(a.name + " overlaps " + b.name)
        }
        if (barGameCol.visible && barName.truncated) bad.push("the game's name is cut")
        app.note("check", "Bar check (" + tag + ", " + app.width + "x" + app.height + ", size " + app.sizeNow + "%): " + items.length + " items, " + bad.length + " problems"
                 + (bad.length ? ": " + bad.join("; ") : "") + " | " + items.map(a => a.name).join(", ") + " | hints " + hintsFit + " of " + app.legendHints.length)
    }
}
