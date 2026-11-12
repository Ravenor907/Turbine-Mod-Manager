import QtQuick

// The quick wheel (View): a pie of eight wedges in fixed places, the same on every screen. The left stick (or a
// pointer over it) picks a wedge by its angle; the D-pad still works (Up then Right is Up-right). The picked wedge
// fills with the accent and the middle says what it does. A wedge with nothing to do here keeps its name in grey,
// and picked it fills grey too (never the accent).
// The middle is neutral: with nothing picked (the stick at rest) it says Close and how to choose.
// While it is open everything behind it is turned off (Main.qml's topLayer), and this backdrop takes every press,
// scroll and hover; a tap outside the pie or in the middle closes it.
// Main.qml owns the slots ({label, ok, desc}) and the keys; a tap on a wedge does it.
Overlay {
    id: qw
    closeOnOutside: false
    required property var ui
    motion: ui.motion
    property var slots: []
    property int focusIndex: -1
    property string hint: "Push the stick to choose"
    property int mousePick: -1            // the wedge a mouse over it picked
    onVisibleChanged: mousePick = -1
    signal activated(int index)
    signal pointed(int index)
    signal closed()
    signal middleTapped()

    anchors.fill: parent
    z: 70
    // Its backdrop fades away under the bottom bar (z 40), as the bar was over it.
    ghostZ: 39

    // The backdrop takes every press, scroll and hover, so the page behind never sees one.
    Scrim { ui: qw.ui }
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
        // It fades in and grows from the middle as it opens (Overlay.qml).
        property real showFade: 1
        property real showMove: 1
        opacity: showFade
        scale: 1 - (1 - showMove) * (1 - qw.ui.motion.popScale)

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
            // The picked wedge's fill fades in: a cover in the plain wedge's look is drawn once over it and fades
            // away (an opacity, so nothing is painted again while it fades).
            onSelChanged: { requestPaint(); selCover.cover() }
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
                    g.fillStyle = String(qw.ui.c.raised)
                    g.fill()
                    if (i === sel) { g.fillStyle = String(okList[i] ? qw.ui.c.accent : qw.ui.c.line); g.fill() }
                    g.lineWidth = Math.max(2, qw.ui.px(4))
                    g.strokeStyle = String(qw.ui.c.ground)
                    g.stroke()
                }
            }
        }
        Canvas {
            id: selCover
            anchors.fill: parent
            property int wedge: -1
            opacity: 0
            visible: opacity > 0
            NumberAnimation { id: selFade; target: selCover; properties: "opacity"; from: 1; to: 0; duration: qw.ui.motion.quick; easing.type: qw.ui.motion.ease }
            function cover() {
                selFade.stop(); opacity = 0
                if (!qw.ui.motion.on || canvas.sel < 0) return
                wedge = canvas.sel
                requestPaint()
                selFade.start()
            }
            onPaint: {
                const g = getContext("2d")
                g.reset()
                if (wedge < 0) return
                const cx = width / 2, cy = height / 2, R = pie.outer, r = pie.inner
                const a0 = (wedge * 45 - 22.5 - 90) * Math.PI / 180, a1 = (wedge * 45 + 22.5 - 90) * Math.PI / 180
                g.beginPath()
                g.arc(cx, cy, R, a0, a1, false)
                g.arc(cx, cy, r, a1, a0, true)
                g.closePath()
                g.fillStyle = String(qw.ui.c.raised)
                g.fill()
                g.lineWidth = Math.max(2, qw.ui.px(4))
                g.strokeStyle = String(qw.ui.c.ground)
                g.stroke()
            }
        }
        // The wedges' names (never empty: a wedge with nothing to do here keeps its name, in grey).
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
                color: !modelData.ok ? qw.ui.c.muted : foc ? qw.ui.c.onAccent : qw.ui.c.text
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
            readonly property var s: qw.focusIndex >= 0 ? qw.slots[qw.focusIndex] : null
            border.color: s && !s.ok ? qw.ui.c.line : qw.ui.c.accent; border.width: Math.max(1, qw.ui.px(2))
            Column {
                anchors.centerIn: parent
                width: parent.width - qw.ui.px(16)
                spacing: qw.ui.px(2)
                Text {
                    width: parent.width; horizontalAlignment: Text.AlignHCenter; elide: Text.ElideRight
                    text: middle.s ? middle.s.label : "Close"
                    color: middle.s && !middle.s.ok ? qw.ui.c.soft : qw.ui.c.accentText; font.family: qw.ui.bodyFont; font.pixelSize: qw.ui.fpx(middle.s ? 16 : 19); font.bold: true
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
