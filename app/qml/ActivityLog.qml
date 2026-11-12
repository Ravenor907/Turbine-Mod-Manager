import QtQuick

// Labs > Activity log: the last 500 lines of what Turbine did, newest at the bottom. Up and Down (the D-pad) scroll
// a few lines at a time, the left stick turned like a wheel and the mouse wheel scroll it too. A runs the focused
// button (Copy to clipboard or Save to Downloads); Left and Right choose it; B closes. Main.qml sends the keys.
Overlay {
    id: al
    onOutsideTapped: al.closeClicked()
    required property var ui
    motion: ui.motion
    anchors.fill: parent

    property var lines: []
    property int focusAt: 0
    // "What happened" (plain sentences with the time) unless the full log is asked for (X, for reports).
    property bool full: false
    readonly property var buttons: [{ label: "Copy to clipboard" }, { label: "Save to Downloads" }, { label: full ? "Show what happened" : "Show the full log", key: "x" }, { label: "Close", key: "b" }]
    readonly property var shown: full ? lines : plain(lines)
    readonly property bool empty: shown.length === 0
    function plain(raw) {
        let out = []
        for (const l of raw) {
            const m = String(l).match(/^\d{4}-\d{2}-\d{2} (\d{2}):(\d{2}):\d{2}(\.\d+)? +(\S+) +(.*)$/)
            if (!m) continue
            const kind = m[4], text = m[5]
            if (!/^(action|engine|net|catch|play|scan)$/.test(kind)) continue
            if (!/^(Added|Removed|Uninstall|Play|Placed|Skipped|Refused|Not added|Renamed|Turn|Download|Got|Couldn't|Failed|Updated|Update|Nexus game for|Name reset|Deploy failed|Error)/i.test(text)) continue
            const h = parseInt(m[1]), mm = m[2]
            out.push((h % 12 === 0 ? 12 : h % 12) + ":" + mm + (h < 12 ? " AM" : " PM") + "  " + text)
        }
        return out
    }

    signal copyClicked()
    signal saveClicked()
    signal closeClicked()

    // Newest at the bottom: the list starts at its end.
    function toEnd() { list.positionViewAtEnd() }
    onLinesChanged: Qt.callLater(toEnd)
    onVisibleChanged: if (visible) { focusAt = 0; full = false; Qt.callLater(toEnd) }

    function scrollBy(d) {
        const max = Math.max(0, list.contentHeight - list.height) + list.originY
        list.contentY = Math.max(list.originY, Math.min(max, list.contentY + d * al.ui.px(90)))
    }
    function handleButton(b) {
        if (b === "up" || b === "down") scrollBy(b === "down" ? 1 : -1)
        else if (b === "left") focusAt = Math.max(0, focusAt - 1)
        else if (b === "right") focusAt = Math.min(buttons.length - 1, focusAt + 1)
        else if (b === "a") activate(focusAt)
        else if (b === "x") activate(2)
        else if (b === "b") closeClicked()
    }
    function activate(i) {
        focusAt = i
        if (i === 0) copyClicked()
        else if (i === 1) saveClicked()
        else if (i === 2) { full = !full; Qt.callLater(toEnd) }
        else closeClicked()
    }

    Scrim { ui: al.ui }
    Card {
        ui: al.ui
        id: box
        anchors.centerIn: parent
        width: Math.min(parent.width - al.ui.px(40), al.ui.px(1100))
        // The sheet layer already ends above the bottom bar.
        height: parent.height - al.ui.px(32)
        clip: true
        CardGuard {}
        Text {
            id: head
            x: al.ui.px(22); y: al.ui.px(18)
            width: parent.width - al.ui.px(44)
            text: "Activity log"
            color: al.ui.c.text
            font.family: al.ui.titleFont; font.pixelSize: al.ui.fpx(18); font.bold: true
        }
        Text {
            id: sub
            anchors.top: head.bottom; anchors.topMargin: al.ui.px(4)
            x: al.ui.px(22)
            width: parent.width - al.ui.px(44)
            wrapMode: Text.WordWrap
            text: (al.full ? "The last " + al.lines.length + " lines" : "What Turbine did") + ", newest at the bottom." + (al.empty ? "" : " Scroll with Up and Down or the left stick.")
            color: al.ui.c.soft
            font.family: al.ui.bodyFont; font.pixelSize: al.ui.fpx(14)
        }
        Rectangle {
            id: well
            anchors.top: sub.bottom; anchors.topMargin: al.ui.px(10)
            anchors.bottom: btnRow.top; anchors.bottomMargin: al.ui.px(12)
            x: al.ui.px(22)
            width: parent.width - al.ui.px(44)
            radius: al.ui.px(6)
            color: al.ui.c.card
            border.color: al.ui.c.line
            ListView {
                id: list
                anchors.fill: parent
                anchors.margins: al.ui.px(8)
                clip: true
                model: al.shown
                boundsBehavior: Flickable.StopAtBounds
                // Plenty built ahead, so jumping to the end lands on the real end.
                cacheBuffer: 4000
                delegate: Text {
                    required property var modelData
                    width: list.width
                    wrapMode: Text.WrapAnywhere
                    text: modelData
                    color: al.ui.c.text
                    font.family: "monospace"; font.pixelSize: al.ui.fpx(12)
                    lineHeight: 1.1
                }
            }
            // Nothing yet: said once, in the middle.
            Text {
                visible: al.empty
                anchors.centerIn: parent
                width: parent.width - al.ui.px(40)
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
                text: al.full ? "The log is empty." : "Nothing to tell yet. " + al.ui.glyph("x") + " shows the full log."
                color: al.ui.c.muted
                font.family: al.ui.bodyFont; font.pixelSize: al.ui.fpx(15)
            }
            // More above: a fade under the top edge.
            Rectangle {
                visible: !list.atYBeginning
                anchors.left: parent.left; anchors.right: parent.right; anchors.top: parent.top
                anchors.margins: al.ui.px(1)
                height: al.ui.px(24)
                radius: al.ui.px(6)
                gradient: Gradient {
                    GradientStop { position: 0; color: al.ui.c.card }
                    GradientStop { position: 1; color: "transparent" }
                }
            }
        }
        Row {
            id: btnRow
            x: al.ui.px(22)
            anchors.bottom: parent.bottom
            anchors.bottomMargin: al.ui.px(18)
            spacing: al.ui.px(10)
            Repeater {
                model: al.buttons
                // The highlighted one shows A's cap; Show the full log X's, Close B's.
                ActionButton {
                    required property int index
                    required property var modelData
                    ui: al.ui
                    label: modelData.label
                    focused: al.focusAt === index
                    cap: focused ? "a" : (modelData.key || "")
                    capRoom: true
                    onClicked: al.activate(index)
                }
            }
        }
    }
}
