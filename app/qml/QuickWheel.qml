import QtQuick

// The quick wheel (View): a pie of eight wedges in fixed places, the same on every screen. The left stick (or a
// pointer over it) picks a wedge by its angle; the D-pad still works (Up then Right is Up-right). The picked wedge
// fills with the accent and the middle says what it does. A wedge with nothing to do here keeps its name, dimmed.
// 0.62: the middle is neutral: with nothing picked (the stick at rest) it says Close and how to choose.
// While it is open everything behind it is turned off (Main.qml's topLayer), and this backdrop takes every press,
// scroll and hover; a tap outside the pie or in the middle closes it.
// Main.qml owns the slots ({label, ok, desc}) and the keys; a tap on a wedge does it.
Overlay {
    id: qw
    closeOnOutside: false
    required property var ui
    property var slots: []
    property int focusIndex: -1
    property string hint: "Push the stick to choose"
    property int mousePick: -1            // the wedge a mouse over it picked
    onVisibleChanged: mousePick = -1
    property bool reduceMotion: false
    signal activated(int index)
    signal pointed(int index)
    signal closed()
    signal middleTapped()

    anchors.fill: parent
    z: 70

    // The backdrop takes every press, scroll and hover, so the page behind never sees one.
    Rectangle { anchors.fill: parent; color: "#D0050403" }
    MouseArea {
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.AllButtons
        preventStealing: true
        onWheel: (w) => { w.accepted = true }
        onPressed: (m) => { m.accepted = true }
        onClicked: (m) => {
            if (m.button !== Qt.LeftButton) return
            const i = pie.wedgeAt(m.x - pie.x, m.y - pie.y)
            if (i === -2) qw.closed()
            else if (i === -1) qw.middleTapped()
            else qw.activated(i)
        }
        onPositionChanged: (m) => {
            const i = pie.wedgeAt(m.x - pie.x, m.y - pie.y)
            // Over a wedge: it is picked. Off it again (the middle, outside): a wedge the mouse picked lets go.
            if (i >= 0) { if (i !== qw.focusIndex) { qw.mousePick = i; qw.pointed(i) } }
            else if (qw.focusIndex >= 0 && qw.focusIndex === qw.mousePick) { qw.mousePick = -1; qw.pointed(-1) }
        }
    }

    Item {
        id: pie
        readonly property real outer: Math.min(qw.width, qw.height) * 0.34
        readonly property real inner: outer * 0.30
        width: outer * 2; height: outer * 2
        anchors.centerIn: parent
        scale: qw.visible ? 1 : 0.92
        Behavior on scale { enabled: !qw.reduceMotion; NumberAnimation { duration: 120 } }

        // -1 the middle, -2 outside the pie, else the wedge (0 is Up, then clockwise).
        function wedgeAt(px, py) {
            const dx = px - outer, dy = py - outer
            const d = Math.sqrt(dx * dx + dy * dy)
            if (d > outer) return -2
            if (d < inner) return -1
            let deg = Math.atan2(dx, -dy) * 180 / Math.PI
            if (deg < 0) deg += 360
            return Math.round(deg / 45) % 8
        }

        Canvas {
            id: canvas
            anchors.fill: parent
            property int sel: qw.focusIndex
            property var okList: qw.slots.map(s => !!s.ok)
            onSelChanged: requestPaint()
            onOkListChanged: requestPaint()
            onWidthChanged: requestPaint()
            property color acc: qw.ui.c.accent
            onAccChanged: requestPaint()
            onPaint: {
                const g = getContext("2d")
                g.reset()
                const cx = width / 2, cy = height / 2, R = pie.outer, r = pie.inner
                for (let i = 0; i < 8; i++) {
                    // Wedge i is centred on i * 45 degrees from Up, clockwise.
                    const a0 = (i * 45 - 22.5 - 90) * Math.PI / 180, a1 = (i * 45 + 22.5 - 90) * Math.PI / 180
                    g.beginPath()
                    g.arc(cx, cy, R, a0, a1, false)
                    g.arc(cx, cy, r, a1, a0, true)
                    g.closePath()
                    g.fillStyle = i === sel ? String(qw.ui.c.accent) : String(qw.ui.c.raised)
                    g.fill()
                    g.lineWidth = Math.max(2, qw.ui.px(4))
                    g.strokeStyle = String(qw.ui.c.ground)
                    g.stroke()
                }
            }
        }
        // The wedges' names (never empty: a wedge with nothing to do here keeps its name, dimmed).
        Repeater {
            model: qw.slots
            Text {
                required property int index
                required property var modelData
                readonly property bool foc: index === qw.focusIndex
                readonly property real a: (index * 45 - 90) * Math.PI / 180
                readonly property real mid: (pie.outer + pie.inner) / 2 + pie.outer * 0.04
                // Diagonal wedges are narrower where the label sits: a smaller width keeps it off their borders.
                width: pie.outer * (index % 2 === 1 ? 0.42 : 0.55)
                x: pie.outer + Math.cos(a) * mid - width / 2
                y: pie.outer + Math.sin(a) * mid - height / 2
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
                maximumLineCount: 2
                elide: Text.ElideRight
                text: modelData.label || ""
                opacity: modelData.ok ? 1 : 0.45
                color: foc ? qw.ui.c.onAccent : qw.ui.c.text
                font.family: qw.ui.bodyFont; font.pixelSize: qw.ui.fpx(17); font.weight: Font.DemiBold
                Accessible.role: Accessible.MenuItem
                Accessible.name: (modelData.label || "") + (modelData.ok ? "" : ", not available here")
            }
        }
        // The middle: the picked wedge's name and what it does; nothing picked: Close, and how to choose.
        Rectangle {
            id: middle
            anchors.centerIn: parent
            width: pie.inner * 2 - qw.ui.px(8); height: width; radius: width / 2
            color: qw.ui.c.panel
            border.color: qw.ui.c.accent; border.width: Math.max(1, qw.ui.px(2))
            readonly property var s: qw.focusIndex >= 0 ? qw.slots[qw.focusIndex] : null
            Column {
                anchors.centerIn: parent
                width: parent.width - qw.ui.px(16)
                spacing: qw.ui.px(2)
                Text {
                    width: parent.width; horizontalAlignment: Text.AlignHCenter; elide: Text.ElideRight
                    text: middle.s ? middle.s.label : "Close"
                    color: qw.ui.c.accentText; font.family: qw.ui.bodyFont; font.pixelSize: qw.ui.fpx(middle.s ? 16 : 19); font.bold: true
                }
                Text {
                    width: parent.width; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap; maximumLineCount: 3; elide: Text.ElideRight
                    text: middle.s ? (middle.s.ok ? (middle.s.desc || "") : "Not available here") : qw.hint
                    color: qw.ui.c.soft; font.family: qw.ui.bodyFont; font.pixelSize: qw.ui.fpx(12)
                }
            }
        }
    }
}
