import QtQuick
import QtQuick.Shapes
import Turbine

// The bottom bar (the only bar on the game screen). One structure on every screen, left to right: the game's name
// (or the page's) with a short status under it (a coloured dot when something needs a look; tappable then), LB RB and
// the tabs ("My mods / Get mods / Load order", the one on screen bold), the legend (Main.qml's legendHints), then
// View's Wheel, Start's Settings and Play on the right. Elsewhere it is the legend and View alone.
// What shows when room is short goes by one priority order (fitted below): A and B, the LB RB caps, View's and
// Start's caps with their short words, and Play always show; then the game's name and status, X, Y, the tab names,
// the fifth hint, and last the words for holding Start. Scrolling down slides it away with the rail.
// Main.qml sets its left margin (the rail's width).
//
// app (required): Main.qml's window. kit (required): Main.qml's kit (colors and motion, see COMPONENTS.md).
Rectangle {
    id: legend
    required property var app
    required property var kit
    readonly property var c: kit.c
    readonly property Motion mo: kit.motion
    // The bar's own scale: it grows with the Interface size up to 115%, so that at 150% on a Steam Deck A, B, View,
    // Start and Play still fit beside each other. Its hints, caps and words take sizes from bw instead of the window.
    readonly property real bs: Math.min(app.s, app.fit * 1.15)
    QtObject {
        id: bw
        readonly property var colors: app.colors
        readonly property string bodyFont: app.bodyFont
        readonly property bool mouseLegend: app.mouseLegend
        function px(v) { return Math.round(v * legend.bs) }
        function fpx(v) { return app.fpxAt(v, legend.bs) }
        function tpx(v) { return Math.max(app.touchFloor, Math.round(v * legend.bs)) }
        function glyph(k) { return app.glyph(k) }
        function gate(layer, why) { return app.gate(layer, why) }
        function note(kind, words) { app.note(kind, words) }
        function act(b) { app.act(b) }
        function r3Click() { app.r3Click() }
        function hover(item, on) { app.hover(item, on) }
    }
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
        // Caps of every shape on one line with the round ones.
        anchors.verticalCenter: parent ? parent.verticalCenter : undefined
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
    anchors.bottomMargin: shown ? 0 : -height - bw.px(2)
    Behavior on anchors.bottomMargin { enabled: mo.moves; NumberAnimation { duration: mo.moveBase; easing.type: mo.enter } }
    anchors.left: parent.left
    anchors.right: parent.right
    // A taller bar with room between its groups.
    height: Math.max(bw.px(60), bw.tpx(30) + bw.px(10), barGameCol.implicitHeight + bw.px(12))
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
    Rectangle { width: parent.width; height: Math.max(1, bw.px(1)); color: c.line }
    readonly property real gap: bw.px(16)
    readonly property real edge: bw.px(16)
    readonly property real wordSize: bw.fpx(15)
    // Touch: a sideways swipe on the bar is LB or RB.
    DragHandler {
        enabled: app.barGame && app.sheet === "" && !app.wheelOpen
        target: null
        xAxis.enabled: true; yAxis.enabled: false
        acceptedDevices: PointerDevice.TouchScreen
        onActiveChanged: if (!active && Math.abs(activeTranslation.x) > bw.px(60)) { app.note("touch", "Swipe on the bottom bar"); app.switchTab(activeTranslation.x < 0 ? 1 : -1) }
    }
    // ---- Left: the game (its whole name, wrapping to two lines before it is ever cut) and the tabs.
    Row {
        id: barRow
        x: legend.edge
        height: parent.height
        spacing: legend.gap
        Column {
            id: barGameCol
            visible: legend.nameAsked && legend.nameFits
            // Measured from the words themselves (never from the Texts' own sizes, which follow this width).
            TextMetrics { id: barNameMetrics; font: barName.font; text: barName.text }
            // The name gets up to 30% of the bar; past that it wraps onto a second line.
            readonly property real most: Math.max(bw.px(130), legend.width * 0.3)
            readonly property real want: Math.min(most, Math.ceil(barNameMetrics.advanceWidth) + bw.px(2))
            // The status line: a few words, as wide as the name or a little wider.
            readonly property real subWant: legend.nameColW(legend.statusWide)
            width: visible ? Math.max(want, barSub.visible ? subWant : 0) : 0
            anchors.verticalCenter: parent.verticalCenter
            spacing: bw.px(1)
            Text {
                id: barName
                width: barGameCol.want
                wrapMode: Text.Wrap
                maximumLineCount: 2
                elide: Text.ElideRight
                text: app.barPlace !== "" ? app.barPlace : app.game ? app.shortGame(app.game.name) : ""
                color: c.accentText; font.family: app.bodyFont; font.pixelSize: bw.fpx(15); font.bold: true
            }
            // The status: plain words, and a dot in its color when it needs a look (a tap says why).
            Row {
                id: barSubRow
                visible: legend.statusFits && legend.subFull !== ""
                spacing: bw.px(6)
                readonly property bool flagged: app.page === "game" && !app.modPage && app.barTappable
                Rectangle {
                    visible: barSubRow.flagged
                    width: bw.px(7); height: width; radius: width / 2
                    anchors.verticalCenter: parent.verticalCenter
                    color: app.statusNow.level === "stop" ? c.stop : app.statusNow.level === "warn" ? c.warn : c.accentHi
                }
                Text {
                    id: barSub
                    readonly property real most: barGameCol.subWant - (barSubRow.flagged ? bw.px(13) : 0)
                    width: Math.min(implicitWidth, most)
                    elide: Text.ElideRight
                    // As many of the status's parts ("1 mod on · Mods offline") as fit whole.
                    text: legend.statusFit(legend.subFull, most)
                    color: c.soft
                    font.family: app.bodyFont; font.pixelSize: bw.fpx(13)
                }
            }
            Accessible.role: app.barTappable ? Accessible.Button : Accessible.StaticText
            Accessible.name: barName.text + ", " + app.barStatus
            HoverHandler { enabled: app.barTappable && app.sheet === "" && !app.wheelOpen; onHoveredChanged: app.hover(parent, hovered) }
            TapHandler { enabled: app.barTappable && app.sheet === "" && !app.wheelOpen; onTapped: app.statusTap() }
        }
        Rectangle { visible: barGameCol.visible && (barTabs.visible || legend.hintsFit > 0); width: Math.max(1, bw.px(1)); height: bw.px(30); color: c.line; anchors.verticalCenter: parent.verticalCenter }
        // LB RB and the tabs (the one on screen bold); a tap on a name opens that tab.
        Row {
            id: barTabs
            // On a mod page LB and RB step through the results (the line on the left says which).
            visible: app.barGame && !app.modPage
            anchors.verticalCenter: parent.verticalCenter
            // After the mouse the pages are buttons by name (My mods, Get mods, Load order), no caps.
            spacing: app.mouseLegend ? bw.px(22) : bw.px(6)
            leftPadding: app.mouseLegend ? bw.px(10) : 0
            rightPadding: leftPadding
            // Dimmed where they do nothing (one tab, or a sheet open).
            Key { id: barLb; win: bw; k: "lb"; opacity: app.tabs.length < 2 || app.sheet !== "" || app.tab === 0 ? 0.35 : 1; anchors.verticalCenter: parent.verticalCenter }
            Key { id: barRb; win: bw; k: "rb"; opacity: app.tabs.length < 2 || app.sheet !== "" || app.tab >= app.tabs.length - 1 ? 0.35 : 1; anchors.verticalCenter: parent.verticalCenter }
            Item { visible: legend.tabWordsFit && !app.mouseLegend; width: bw.px(4); height: 1 }
            Repeater {
                model: legend.tabWordsFit || app.mouseLegend ? app.tabs : []
                Row {
                    required property int index
                    required property string modelData
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: bw.px(6)
                    Text { visible: parent.index > 0 && !app.mouseLegend; text: "/"; color: c.muted; font.family: app.bodyFont; font.pixelSize: bw.fpx(14); anchors.verticalCenter: parent.verticalCenter }
                    Text {
                        Rectangle {
                            visible: app.mouseLegend
                            z: -1
                            anchors.centerIn: parent
                            width: parent.width + bw.px(20); height: Math.max(bw.px(32), bw.tpx(26)); radius: bw.px(6)
                            color: app.tab === parent.parent.index ? c.focusFill : c.raised
                            border.color: app.tab === parent.parent.index ? c.accent : c.accentLo; border.width: Math.max(1, bw.px(1))
                        }
                        text: parent.modelData.replace(/ \(\d+\)$/, "")
                        color: app.tab === parent.index ? c.text : c.muted
                        font.family: app.bodyFont; font.pixelSize: bw.fpx(14); font.bold: app.tab === parent.index
                        anchors.verticalCenter: parent.verticalCenter
                        Accessible.role: Accessible.PageTab
                        Accessible.name: text + (app.tab === parent.index ? ", selected" : "") + ", " + (parent.index + 1) + " of " + app.tabs.length
                        HoverHandler { enabled: app.sheet === "" && !app.wheelOpen; onHoveredChanged: app.hover(parent, hovered) }
                        TapHandler { enabled: app.sheet === "" && !app.wheelOpen; onTapped: { const i = parent.parent.index; app.note("touch", "Tab tapped: " + app.tabs[i]); app.modPage = false; app.tab = i } }
                    }
                }
            }
        }
        Rectangle { visible: barTabs.visible && legend.hintsFit > 0; width: Math.max(1, bw.px(1)); height: bw.px(30); color: c.line; anchors.verticalCenter: parent.verticalCenter }
    }
    // ---- Right: View's Wheel, Start's Settings and Play (and Seamless Co-op where it is on), always whole.
    Row {
        id: playGroup
        anchors.right: parent.right
        anchors.rightMargin: legend.edge
        anchors.verticalCenter: parent.verticalCenter
        spacing: legend.gap
        // The wheel's button: VIEW's cap and Wheel (a tap opens it too). While the wheel is open B's Close is the one
        // close hint, so this steps aside.
        Item {
            id: wheelBtn
            visible: app.sheet === "" && !app.wheelOpen
            anchors.verticalCenter: parent.verticalCenter
            width: wheelRow.implicitWidth; height: Math.max(bw.tpx(44), wheelRow.implicitHeight)
            Row {
                id: wheelRow
                anchors.verticalCenter: parent.verticalCenter
                spacing: bw.px(6)
                Key { id: wheelKey; win: bw; k: "view"; anchors.verticalCenter: parent.verticalCenter }
                Label { win: bw; text: "Wheel"; font.pixelSize: legend.wordSize; color: c.soft; anchors.verticalCenter: parent.verticalCenter }
            }
            HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
            TapHandler { onTapped: app.wheelPress() }
        }
        // What Start does: a tap is Settings, a 3 second hold plays (or shows Play options). A tap on
        // it is a tap of Start. The hold's words show only when the bar has room for them.
        Item {
            id: startHint
            visible: app.barGame && app.sheet === "" && !app.wheelOpen
            anchors.verticalCenter: parent.verticalCenter
            width: startHintRow.implicitWidth; height: Math.max(bw.tpx(44), startHintRow.implicitHeight)
            Row {
                id: startHintRow
                anchors.verticalCenter: parent.verticalCenter
                spacing: bw.px(6)
                // With the keyboard Settings is Esc, on a game's first page (elsewhere Esc goes back, so no cap).
                Key { id: startHintKey; win: bw; k: app.keysLegend ? "b" : "start"; off: app.keysLegend && !app.escTop; anchors.verticalCenter: parent.verticalCenter }
                Label { text: "Settings"; win: bw; color: c.soft; font.pixelSize: legend.wordSize; anchors.verticalCenter: parent.verticalCenter }
                Label { visible: legend.holdFits && !app.mouseLegend; win: bw; text: "· " + app.startHoldWords; color: c.muted; font.pixelSize: bw.fpx(13); anchors.verticalCenter: parent.verticalCenter }
            }
            Accessible.role: Accessible.Button
            Accessible.name: "Settings. Hold START for 3 seconds: " + (app.startHoldPlays ? "Play" : "Play options")
            HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
            TapHandler { onTapped: { if (!app.gate(3, "Start hint")) return; app.note("touch", "Start hint tapped"); app.startTap() } }
        }
        Rectangle {
            id: playBtn
            // A sheet whose own Start button is in the legend ("START Save"): Play steps aside.
            visible: app.barGame && !(app.sheet !== "" && app.legendHints.some(h => h[0] === "start"))
            anchors.verticalCenter: parent.verticalCenter
            width: playRow.implicitWidth + bw.px(28); height: Math.max(bw.px(40), bw.tpx(30)); radius: kit.radiusSmall
            // Dimmed while a sheet or the wheel is open.
            opacity: Turbine.busy || app.sheet !== "" || app.wheelOpen ? 0.55 : 1
            // The focus ring when Play has the focus (a mod just arrived).
            Rectangle {
                visible: app.zone === "play" && app.sheet === "" && !app.wheelOpen
                anchors.fill: parent
                anchors.margins: -bw.px(4)
                radius: parent.radius + bw.px(4)
                color: "transparent"
                border.width: kit.ringWidth
                border.color: kit.ring
            }
            gradient: Gradient {
                GradientStop { position: 0; color: c.accentHi }
                GradientStop { position: 1; color: c.accent }
            }
            Row {
                id: playRow
                anchors.centerIn: parent
                spacing: bw.px(8)
                // Play waits for changes being saved: it says so.
                Text { text: (app.savingNow ? "Play (saving...)" : app.modHold && app.mods.some(m => m.enabled) ? "Play without mods" : "Play") + (app.seamlessIn && !app.savingNow ? (app.lastPlayChoice === "coop" ? " · Co-op" : " · Solo") : ""); color: app.onFill(c.accent); font.family: app.bodyFont; font.pixelSize: bw.fpx(16); font.bold: true; anchors.verticalCenter: parent.verticalCenter }
            }
            // Start held: a brass ring fills here over 3 seconds (shown once the press is clearly a hold).
            Rectangle {
                id: startRing
                readonly property bool shown: app.startHeld > 0.1
                visible: shown
                anchors.centerIn: parent
                width: parent.height - bw.px(6); height: width; radius: width / 2
                color: c.ground
                Shape {
                    anchors.fill: parent
                    preferredRendererType: Shape.CurveRenderer
                    ShapePath {
                        strokeColor: c.accentHi
                        strokeWidth: Math.max(2, bw.px(4))
                        fillColor: "transparent"
                        capStyle: ShapePath.RoundCap
                        PathAngleArc {
                            centerX: startRing.width / 2; centerY: startRing.height / 2
                            radiusX: startRing.width / 2 - bw.px(5); radiusY: radiusX
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
    // ---- Middle: the hints in priority order (A, B, X, Y, then one more); the ones that don't fit are left out,
    // never squeezed under Play or past the edge.
    Item {
        id: hintsBox
        readonly property real rightEdge: playGroup.width > 0 ? playGroup.x - legend.gap : legend.width - legend.edge
        // A fixed gap after the name and the tabs.
        x: barRow.x + barRow.width + (barRow.width > 0 ? legend.gap : 0)
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
            spacing: bw.px(16)
            Repeater {
                model: app.legendHints.slice(0, legend.hintsFit)
                LegendHint { win: bw }
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
            LegendHint { win: bw }
        }
    }
    property int measureTick: 0
    function hintsWidth(n) {
        let w = measureTick * 0
        for (let i = 0; i < Math.min(n, measureRep.count); i++) { const it = measureRep.itemAt(i); if (it) w += it.implicitWidth + (i > 0 ? legendRow.spacing : 0) }
        return w
    }
    // ---- What fits, in priority order. Everything is measured from the words and caps, never from the items' own
    // sizes (which follow these answers).
    TextMetrics { id: tabWordsMetrics; font.family: app.bodyFont; font.pixelSize: bw.fpx(14); font.bold: true; text: app.tabs.map(t => t.replace(/ \(\d+\)$/, "")).join(" / ") }
    TextMetrics { id: wheelMetrics; font.family: app.bodyFont; font.pixelSize: legend.wordSize; text: "Wheel" }
    TextMetrics { id: settingsMetrics; font.family: app.bodyFont; font.pixelSize: legend.wordSize; text: "Settings" }
    TextMetrics { id: holdMetrics; font.family: app.bodyFont; font.pixelSize: bw.fpx(13); text: "· " + app.startHoldWords }
    TextMetrics { id: nameMetrics; font.family: app.bodyFont; font.pixelSize: bw.fpx(15); font.bold: true; text: barName.text }
    readonly property bool tabsShown: app.barGame && !app.modPage
    readonly property bool nameAsked: app.barGame || app.barPlace !== ""
    readonly property bool wheelShown: app.sheet === "" && !app.wheelOpen
    readonly property bool startShown: app.barGame && app.sheet === "" && !app.wheelOpen
    // The status in full. On a mod page: where it is ("Get mods > SkyUI", "Mod 3 of 30"); Settings: the section.
    readonly property string subFull: app.page === "settings" ? app.sec : app.page === "place" ? (app.placeData && app.placeData.pending ? app.placeData.pending.name : "")
                                      : app.modPage ? "Get mods > " + (app.detailsData && app.detailsData.name ? app.detailsData.name : app.detailsItem ? app.detailsItem.name : "") + app.pageResultWords : app.barStatus
    TextMetrics { id: statusMetrics; font.family: app.bodyFont; font.pixelSize: bw.fpx(13); text: legend.subFull }
    FontMetrics { id: statusFm; font.family: app.bodyFont; font.pixelSize: bw.fpx(13) }
    // The status cut to whole parts that fit in w (the first part is kept, cut short if it has to be).
    function statusFit(full, w) {
        if (statusFm.advanceWidth(full) <= w) return full
        const parts = full.split(" · ")
        let out = parts[0]
        for (let i = 1; i < parts.length; i++) { const t = out + " · " + parts[i]; if (statusFm.advanceWidth(t) > w) break; out = t }
        return out
    }
    // The name's column: the name, and the status a few words wide (wide: all of the status, up to 30% of the bar).
    function nameColW(wide) {
        const most = Math.max(bw.px(130), width * 0.3)
        const want = Math.min(most, Math.ceil(nameMetrics.advanceWidth) + bw.px(2))
        const sub = wide ? Math.ceil(statusMetrics.advanceWidth) + bw.px(15) : bw.px(120)
        return Math.max(want, Math.min(Math.max(want, sub), most))
    }
    // (The keyboard's names for LB and RB are wider than the controller's.)
    readonly property real capsW: barLb.implicitWidth + barRb.implicitWidth + barTabs.spacing + barTabs.leftPadding * 2
    readonly property real tabWordsW: bw.px(4) + barTabs.spacing + tabWordsMetrics.advanceWidth + bw.px(6) * Math.max(0, app.tabs.length - 1) * 2 + barTabs.spacing * Math.max(0, app.tabs.length - 1)
    readonly property real playW: playBtn.visible ? playRow.implicitWidth + bw.px(28) : 0
    function rightW(hold) {
        let parts = []
        if (wheelShown) parts.push(wheelKey.implicitWidth + bw.px(6) + wheelMetrics.advanceWidth)
        if (startShown) parts.push(startHintKey.implicitWidth + bw.px(6) + settingsMetrics.advanceWidth + (hold ? bw.px(6) + holdMetrics.advanceWidth : 0))
        if (playBtn.visible) parts.push(playW)
        return parts.length ? parts.reduce((x, y) => x + y, 0) + gap * (parts.length - 1) : 0
    }
    // The bar's width a choice needs: h hints, the name (and status), the tab names, the hold words.
    // After the mouse the tabs are buttons by name, always shown.
    readonly property real tabButtonsW: tabWordsMetrics.advanceWidth + bw.px(20) * app.tabs.length + bw.px(22) * Math.max(0, app.tabs.length - 1) + bw.px(20)
    function need(h, name, tabs, hold, wide) {
        let left = []
        if (name && nameAsked) left.push(nameColW(wide === true))
        if (name && nameAsked && (tabsShown || h > 0)) left.push(Math.max(1, bw.px(1)))
        if (tabsShown) left.push(app.mouseLegend ? tabButtonsW : capsW + (tabs ? tabWordsW : 0))
        if (tabsShown && h > 0) left.push(Math.max(1, bw.px(1)))
        const leftW = left.length ? left.reduce((x, y) => x + y, 0) + gap * left.length : 0
        const r = rightW(hold)
        return edge * 2 + leftW + hintsWidth(h) + (r > 0 ? gap + r : 0)
    }
    readonly property int hintCount: Math.min(app.legendHints.length, measureRep.count)
    // The plan: [the hints shown, the name and status, the tab names, the hold words]. A and B always; then the
    // name, X, Y, the tab names, the fifth hint, the words for holding Start.
    readonly property var plan: {
        const w = width + measureTick * 0
        let h = Math.min(2, hintCount), name = false, tabs = false, hold = false
        const fits = (hh, n, t, o) => need(hh, n, t, o) <= w
        if (fits(h, true, tabs, hold)) name = true
        if (h === 2 && h < hintCount && fits(h + 1, name, tabs, hold)) h++
        if (h === 3 && h < hintCount && fits(h + 1, name, tabs, hold)) h++
        if (tabsShown && !app.mouseLegend && fits(h, name, true, hold)) tabs = true
        if (h === 4 && h < hintCount && fits(h + 1, name, tabs, hold)) h++
        if (startShown && fits(h, name, tabs, true)) hold = true
        const wide = name && need(h, name, tabs, hold, true) <= w
        return [h, name, tabs, hold, wide]
    }
    readonly property int hintsFit: plan[0]
    readonly property bool nameFits: plan[1]
    readonly property bool statusFits: plan[1]
    readonly property bool tabWordsFit: plan[2]
    readonly property bool holdFits: plan[3]
    readonly property bool statusWide: plan[4]
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
