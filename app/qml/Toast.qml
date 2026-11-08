import QtQuick

// A short message at the bottom, e.g. "Epic Loot deleted for good." It sizes and places
// itself at the bottom center of its parent and hides itself after timeout milliseconds (0 keeps it up).
// Call show(...) to put up a message; it stays hidden until then.
//
// ui (required): an object that gives
//   ui.c          colors: ground, panel, card, raised, line, text, soft, muted, accent, accentHi, accentLo,
//                 onAccent, focusFill, ok, warn, light (bool); stop is optional (a fallback is used without it)
//   ui.px(n)      design pixels to screen pixels
//   ui.titleFont  font family for titles
//   ui.bodyFont   font family for everything else
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

    // Contrast: the same hue, darkened on light grounds or lightened on dark ones until it reads at 4.5:1 or better.
    function lum(col) {
        const f = v => v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4)
        return 0.2126 * f(col.r) + 0.7152 * f(col.g) + 0.0722 * f(col.b)
    }
    function readable(col, bg) {
        let out = col
        const lb = lum(bg)
        for (let i = 0; i < 14; i++) {
            const lc = lum(out)
            if ((Math.max(lc, lb) + 0.05) / (Math.min(lc, lb) + 0.05) >= 4.5) break
            out = lb < 0.18 ? Qt.lighter(out, 1.12) : Qt.darker(out, 1.12)
        }
        return out
    }

    Timer { id: hideTimer; interval: Math.max(1, root.timeout); onTriggered: root.hide() }

    readonly property real maxWidth: parent ? parent.width - ui.px(40) : ui.px(600)
    x: parent ? (parent.width - width) / 2 : 0
    y: parent ? parent.height - height - bottomMargin : 0
    z: 50
    width: Math.min(maxWidth, toastRow.implicitWidth + ui.px(24))
    height: ui.px(46)
    radius: ui.px(6)
    color: Qt.tint(ui.c.raised, Qt.rgba(ui.c.accent.r, ui.c.accent.g, ui.c.accent.b, 0.06))
    border.width: Math.max(1, ui.px(1.5))
    border.color: ui.c.accentLo
    opacity: shown ? 1 : 0
    visible: opacity > 0
    Behavior on opacity { NumberAnimation { duration: 160 } }

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
                font.pixelSize: root.ui.px(14)
                font.bold: true
            }
        }

        Text {
            id: mainText
            anchors.verticalCenter: parent.verticalCenter
            width: Math.min(implicitWidth, root.maxWidth - root.ui.px(76) - (detailText.visible ? detailText.width + parent.spacing : 0))
            text: root.escaped(root.text || "") + (root.strong !== "" ? (root.text !== "" ? " " : "") + "<b>" + root.escaped(root.strong) + "</b>" : "")
            textFormat: Text.StyledText
            elide: Text.ElideRight
            color: root.ui.c.text
            font.family: root.ui.bodyFont
            font.pixelSize: root.ui.px(15)
        }
        Text {
            id: detailText
            visible: text !== ""
            anchors.verticalCenter: parent.verticalCenter
            text: root.detail || ""
            color: root.ui.c.muted
            font.family: root.ui.bodyFont
            font.pixelSize: root.ui.px(14)
        }
    }

    // Mod names come from zip files, so anything that looks like markup is shown as plain text.
    function escaped(s) { return String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;") }
}
