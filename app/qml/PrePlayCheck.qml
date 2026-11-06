import QtQuick
import QtQuick.Shapes

// "Before you play": the check Start runs before a modded launch. One line per check (a passed check, a warning or
// a stop), up to 3 buttons with the focused one in brass, and an optional note under them. A centered card over a
// dimmed backdrop. It fills its parent; show and hide it with visible.
//
// ui (required): an object that gives
//   ui.c          colors: ground, panel, card, raised, line, text, soft, muted, accent, accentHi, accentLo,
//                 onAccent, focusFill, ok, warn, light (bool); stop is optional (a fallback is used without it)
//   ui.px(n)      design pixels to screen pixels
//   ui.titleFont  font family for titles
//   ui.bodyFont   font family for everything else
Item {
    id: root

    required property var ui

    property string title: "Before you play"
    // Each check: { level: "ok" | "warn" | "stop", title, detail }
    property var checks: []
    // Up to 3 buttons: { key (optional, e.g. "a" or "b"), label }
    property var buttons: []
    property int focusIndex: 0
    property string note: ""              // e.g. "Wait checks again every few hours."

    signal activated(int index, string key)
    signal closed()

    function levelColor(level) {
        if (level === "ok") return ui.c.ok
        if (level === "stop") return ui.c.stop !== undefined ? ui.c.stop : (ui.c.light ? Qt.rgba(0.557, 0.165, 0.118, 1) : Qt.rgba(0.941, 0.565, 0.478, 1))
        return ui.c.warn
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

    anchors.fill: parent

    Rectangle {
        anchors.fill: parent
        color: Qt.rgba(0.02, 0.016, 0.012, 0.74)
        MouseArea { anchors.fill: parent; onClicked: root.closed() }
    }

    Rectangle {
        id: card
        anchors.centerIn: parent
        width: Math.min(parent.width - root.ui.px(40), root.ui.px(560))
        height: Math.min(content.implicitHeight + root.ui.px(34), parent.height - root.ui.px(24))
        radius: root.ui.px(6)
        color: root.ui.c.panel
        border.width: Math.max(1, root.ui.px(1.5))
        border.color: root.ui.c.accentLo
        clip: true

        MouseArea { anchors.fill: parent }   // taps on the card don't close it

        Column {
            id: content
            x: root.ui.px(20)
            y: root.ui.px(18)
            width: parent.width - root.ui.px(40)
            spacing: root.ui.px(8)

            Text {
                width: parent.width
                text: (root.title || "").toUpperCase()
                elide: Text.ElideRight
                color: root.ui.c.text
                font.family: root.ui.titleFont
                font.pixelSize: root.ui.px(18)
                font.bold: true
                font.letterSpacing: root.ui.px(1)
            }

            Repeater {
                model: root.checks || []
                Rectangle {
                    id: checkRow
                    required property var modelData
                    readonly property string level: modelData.level || "ok"
                    readonly property bool flagged: level !== "ok"
                    readonly property color hue: root.levelColor(level)
                    readonly property color tone: root.readable(hue, color)
                    width: content.width
                    height: Math.max(root.ui.px(44), checkText.implicitHeight + root.ui.px(16))
                    radius: root.ui.px(6)
                    color: flagged ? Qt.tint(root.ui.c.card, Qt.rgba(hue.r, hue.g, hue.b, 0.1)) : root.ui.c.card
                    border.width: flagged ? Math.max(1, root.ui.px(1.5)) : 0
                    border.color: Qt.tint(root.ui.c.card, Qt.rgba(hue.r, hue.g, hue.b, 0.45))

                    StateDot {
                        id: checkDot
                        ui: root.ui
                        level: checkRow.level
                        tone: checkRow.tone
                        x: root.ui.px(12)
                        anchors.verticalCenter: parent.verticalCenter
                    }
                    Column {
                        id: checkText
                        anchors.left: checkDot.right
                        anchors.leftMargin: root.ui.px(12)
                        anchors.right: parent.right
                        anchors.rightMargin: root.ui.px(12)
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: root.ui.px(1)
                        Text {
                            width: parent.width
                            text: checkRow.modelData.title || ""
                            wrapMode: Text.WordWrap
                            color: checkRow.flagged ? checkRow.tone : root.ui.c.text
                            font.family: root.ui.bodyFont
                            font.pixelSize: root.ui.px(16)
                            font.weight: Font.DemiBold
                        }
                        Text {
                            visible: text !== ""
                            width: parent.width
                            text: checkRow.modelData.detail || ""
                            wrapMode: Text.WordWrap
                            color: checkRow.flagged ? root.ui.c.soft : root.ui.c.muted
                            font.family: root.ui.bodyFont
                            font.pixelSize: root.ui.px(14)
                        }
                    }
                }
            }

            Flow {
                width: parent.width
                spacing: root.ui.px(10)
                topPadding: root.ui.px(6)
                Repeater {
                    model: (root.buttons || []).slice(0, 3)
                    ActionButton {
                        required property int index
                        required property var modelData
                        ui: root.ui
                        k: modelData.key || ""
                        label: modelData.label || ""
                        focused: index === root.focusIndex
                        onClicked: root.activated(index, modelData.key || "")
                    }
                }
            }

            Text {
                visible: text !== ""
                width: parent.width
                text: root.note || ""
                wrapMode: Text.WordWrap
                color: root.ui.c.soft
                font.family: root.ui.bodyFont
                font.pixelSize: root.ui.px(14)
            }
        }
    }

    // A round state mark: check for ok, ! for a warning, a cross for a stop.
    component StateDot: Rectangle {
        id: dot
        required property var ui
        property string level: "ok"
        property color tone: "white"
        width: ui.px(22)
        height: width
        radius: width / 2
        color: tone
        Shape {
            visible: dot.level === "ok"
            anchors.fill: parent
            preferredRendererType: Shape.CurveRenderer
            ShapePath {
                strokeColor: dot.ui.c.onAccent
                strokeWidth: Math.max(2, dot.ui.px(2.2))
                fillColor: "transparent"
                capStyle: ShapePath.RoundCap
                joinStyle: ShapePath.RoundJoin
                startX: dot.width * 0.28; startY: dot.height * 0.52
                PathLine { x: dot.width * 0.44; y: dot.height * 0.68 }
                PathLine { x: dot.width * 0.73; y: dot.height * 0.35 }
            }
        }
        Shape {
            visible: dot.level === "stop"
            anchors.fill: parent
            preferredRendererType: Shape.CurveRenderer
            ShapePath {
                strokeColor: dot.ui.c.onAccent
                strokeWidth: Math.max(2, dot.ui.px(2.2))
                fillColor: "transparent"
                capStyle: ShapePath.RoundCap
                startX: dot.width * 0.33; startY: dot.height * 0.33
                PathLine { x: dot.width * 0.67; y: dot.height * 0.67 }
                PathMove { x: dot.width * 0.67; y: dot.height * 0.33 }
                PathLine { x: dot.width * 0.33; y: dot.height * 0.67 }
            }
        }
        Text {
            visible: dot.level !== "ok" && dot.level !== "stop"
            anchors.centerIn: parent
            text: "!"
            color: dot.ui.c.onAccent
            font.family: dot.ui.bodyFont
            font.pixelSize: dot.ui.px(15)
            font.bold: true
        }
    }

    // A button: brass when focused, plain otherwise, with its controller key in front.
    component ActionButton: Rectangle {
        id: btn
        required property var ui
        property string k: ""
        property string label: ""
        property bool focused: false
        signal clicked()
        width: btnRow.implicitWidth + ui.px(36)
        height: ui.px(44)
        radius: ui.px(6)
        color: ui.c.raised
        border.width: Math.max(1, ui.px(1.5))
        border.color: focused ? ui.c.accentHi : ui.c.line

        Rectangle {
            visible: btn.focused
            anchors.fill: parent
            anchors.margins: -btn.ui.px(3)
            radius: btn.radius + btn.ui.px(3)
            color: "transparent"
            border.width: btn.ui.px(3)
            border.color: Qt.rgba(btn.ui.c.accent.r, btn.ui.c.accent.g, btn.ui.c.accent.b, 0.22)
        }
        Rectangle {
            visible: btn.focused
            anchors.fill: parent
            radius: btn.radius
            border.width: Math.max(1, btn.ui.px(1.5))
            border.color: btn.ui.c.accentHi
            gradient: Gradient {
                GradientStop { position: 0; color: btn.ui.c.accentHi }
                GradientStop { position: 0.5; color: btn.ui.c.accent }
                GradientStop { position: 1; color: btn.ui.c.accentLo }
            }
        }
        TapHandler { onTapped: btn.clicked() }

        Row {
            id: btnRow
            anchors.centerIn: parent
            spacing: btn.ui.px(6)
            Rectangle {
                visible: btn.k !== ""
                width: Math.max(btn.ui.px(24), btnKeyText.implicitWidth + btn.ui.px(12))
                height: btn.ui.px(24)
                radius: btn.k.length > 1 ? btn.ui.px(6) : height / 2
                anchors.verticalCenter: parent.verticalCenter
                color: btn.focused ? Qt.rgba(0, 0, 0, 0.2) : btn.ui.c.ground
                border.width: btn.focused ? 0 : Math.max(1, btn.ui.px(1))
                border.color: btn.ui.c.accentLo
                Text {
                    id: btnKeyText
                    anchors.centerIn: parent
                    text: btn.k.toUpperCase()
                    color: btn.focused ? btn.ui.c.onAccent : btn.ui.c.text
                    font.family: btn.ui.bodyFont
                    font.pixelSize: btn.ui.px(14)
                    font.bold: true
                }
            }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: btn.label
                color: btn.focused ? btn.ui.c.onAccent : btn.ui.c.text
                font.family: btn.ui.bodyFont
                font.pixelSize: btn.ui.px(16)
                font.weight: Font.DemiBold
            }
        }
    }
}
