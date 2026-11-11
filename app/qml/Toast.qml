import QtQuick

// A short message at the bottom, e.g. "Epic Loot deleted for good." It sizes and places
// itself at the bottom center of its parent and hides itself after timeout milliseconds (0 keeps it up).
// Call show(...) to put up a message; it stays hidden until then.
//
// ui (required): Main.qml's kit (colors, sizes, fonts, motion and helpers; see COMPONENTS.md).
Rectangle {
    id: root

    required property var ui

    property string text: ""              // e.g. "Recently removed:"
    property string strong: ""            // bold part after text, e.g. "Epic Loot"
    property string detail: ""            // muted, e.g. "Kept 7 days"
    property int timeout: 5000            // milliseconds; 0 keeps it up until hide()
    property real bottomMargin: ui.px(58) // distance from the parent's bottom (clears the legend)
    property bool shown: false
    // Three looks, never mixed: "info" (accent i mark), "done" (ok check), "problem" (warn mark, real failures only).
    property string kind: "info"

    signal hidden()
    // A press on the toast (0.59): Main.qml stops that press reaching anything beneath it.
    signal pressedOn()
    signal pressEnded()
    // 0.6: a toast can carry its next step as a button ("R3 Undo"): a tap on it, or the key, does it.
    property string actionLabel: ""
    property string actionGlyph: ""
    signal actionClicked()

    // show(text, strong, detail); the last two are optional.
    function show(t, s, d) {
        text = t || ""
        strong = s || ""
        detail = d || ""
        shown = true
        if (timeout > 0) hideTimer.restart()
        else hideTimer.stop()
    }
    function hide() {
        hideTimer.stop()
        if (shown) { shown = false; hidden() }
    }

    Timer { id: hideTimer; interval: Math.max(1, root.timeout); onTriggered: root.hide() }

    readonly property real maxWidth: parent ? parent.width - ui.px(40) : ui.px(600)
    x: parent ? (parent.width - width) / 2 : 0
    y: parent ? parent.height - height - bottomMargin : 0
    z: 50
    width: Math.min(maxWidth, toastRow.implicitWidth + ui.px(24))
    // 0.59: wraps to three lines rather than cutting a message off.
    height: Math.max(ui.px(46), toastRow.implicitHeight + ui.px(20))
    radius: ui.px(6)
    color: Qt.tint(ui.c.raised, Qt.rgba(ui.c.accent.r, ui.c.accent.g, ui.c.accent.b, 0.06))
    border.width: Math.max(1, ui.px(1.5))
    border.color: ui.c.accentLo
    opacity: shown ? 1 : 0
    visible: opacity > 0
    // 0.62: it fades in and out, and at Full slides up a little as it comes and back down as it goes.
    Behavior on opacity { NumberAnimation { duration: root.ui.motion.base; easing.type: root.shown ? root.ui.motion.enter : root.ui.motion.leave } }
    transform: Translate { y: (1 - root.opacity) * root.ui.px(14) * root.ui.motion.slide }

    // A tap on the toast dismisses it and goes nowhere else (0.59).
    MouseArea {  // overlay-check: the toast's own box, not the window
        anchors.fill: parent
        enabled: root.shown
        acceptedButtons: Qt.AllButtons
        preventStealing: true
        onWheel: (w) => { w.accepted = true }
        onPressed: root.pressedOn()
        onReleased: root.pressEnded()
        onCanceled: root.pressEnded()
        onClicked: (m) => {
            // A tap on the action button does the action; anywhere else dismisses.
            const p = actionBox.mapFromItem(root, m.x, m.y)
            if (root.actionLabel !== "" && p.x >= 0 && p.y >= -root.ui.px(8) && p.x <= actionBox.width && p.y <= actionBox.height + root.ui.px(8)) root.actionClicked()
            else root.hide()
        }
    }

    Row {
        id: toastRow
        anchors.verticalCenter: parent.verticalCenter
        x: root.ui.px(16)
        spacing: root.ui.px(14)

        Rectangle {
            id: mark
            anchors.verticalCenter: parent.verticalCenter
            width: root.ui.px(22); height: width; radius: width / 2
            readonly property color tone: root.kind === "done" ? root.ui.c.ok : root.kind === "problem" ? root.ui.c.warn : root.ui.c.accentHi
            color: "transparent"
            border.color: tone
            border.width: Math.max(1, root.ui.px(1.5))
            Text {
                anchors.centerIn: parent
                text: root.kind === "done" ? "\u2713" : root.kind === "problem" ? "!" : "i"
                color: mark.tone
                font.family: root.ui.bodyFont
                font.pixelSize: root.ui.fpx(14)
                font.bold: true
            }
        }

        Text {
            id: mainText
            anchors.verticalCenter: parent.verticalCenter
            width: Math.min(implicitWidth, root.maxWidth - root.ui.px(76) - (detailText.visible ? detailText.width + parent.spacing : 0) - (actionBox.visible ? actionBox.width + parent.spacing : 0))
            text: root.escaped(root.text || "") + (root.strong !== "" ? (root.text !== "" ? " " : "") + "<b>" + root.escaped(root.strong) + "</b>" : "")
            textFormat: Text.StyledText
            wrapMode: Text.WordWrap
            maximumLineCount: 3
            elide: Text.ElideRight
            color: root.ui.c.text
            font.family: root.ui.bodyFont
            font.pixelSize: root.ui.fpx(15)
        }
        Rectangle {
            id: actionBox
            objectName: "toastAction"
            visible: root.actionLabel !== ""
            anchors.verticalCenter: parent.verticalCenter
            width: visible ? actionText.implicitWidth + root.ui.px(20) : 0; height: root.ui.tpx(36); radius: root.ui.px(6)
            color: root.ui.c.raised; border.color: root.ui.c.accent; border.width: Math.max(1, root.ui.px(1.5))
            Text { id: actionText; anchors.centerIn: parent; text: (root.actionGlyph !== "" ? root.actionGlyph + " " : "") + root.actionLabel; color: root.ui.c.text; font.family: root.ui.bodyFont; font.pixelSize: root.ui.fpx(14); font.bold: true }
        }
        Text {
            id: detailText
            visible: text !== ""
            anchors.verticalCenter: parent.verticalCenter
            text: root.detail || ""
            color: root.ui.c.muted
            font.family: root.ui.bodyFont
            font.pixelSize: root.ui.fpx(14)
        }
    }

    // Mod names come from zip files, so anything that looks like markup is shown as plain text.
    function escaped(s) { return String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;") }
}
