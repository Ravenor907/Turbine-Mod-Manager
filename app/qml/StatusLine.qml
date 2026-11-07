import QtQuick
import QtQuick.Shapes

// The line under the header that always says in words what is going on: a state icon, bold state words, a muted
// detail and an optional pill on the right ("2 updates") or a progress bar with text.
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

    property string level: "ok"           // ok, busy, warn, stop (named level because Item already has state)
    property string words: ""             // bold, e.g. "Ready to play"
    property string detail: ""            // muted, e.g. "15 of 17 mods on"
    property string pillText: ""          // right side pill, e.g. "2 updates"; empty hides it
    property string pillKey: ""           // optional button shown in the pill
    property real progress: -1            // 0..1 shows a bar on the right; below 0 hides it
    property string progressText: ""      // shown after the bar, e.g. "64% · 1.1 of 1.7 MB"

    signal pillClicked()

    function levelColor(level) {
        if (level === "ok") return ui.c.ok
        if (level === "stop") return ui.c.stop !== undefined ? ui.c.stop : (ui.c.light ? Qt.rgba(0.557, 0.165, 0.118, 1) : Qt.rgba(0.941, 0.565, 0.478, 1))
        if (level === "busy") return ui.c.accentHi
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

    implicitWidth: ui.px(600)
    implicitHeight: ui.px(44)
    radius: ui.px(6)
    color: ui.c.panel
    clip: true

    Row {
        id: leftPart
        anchors.left: parent.left
        anchors.leftMargin: root.ui.px(12)
        anchors.right: rightPart.left
        anchors.rightMargin: root.ui.px(10)
        anchors.verticalCenter: parent.verticalCenter
        spacing: root.ui.px(10)

        // State icon: check, exclamation mark, cross, or a turning arc while busy.
        Item {
            id: iconBox
            width: root.ui.px(22)
            height: width
            anchors.verticalCenter: parent.verticalCenter

            Rectangle {
                visible: root.level !== "busy"
                anchors.fill: parent
                radius: width / 2
                color: root.readable(root.levelColor(root.level), root.ui.c.panel)
            }
            Shape {
                visible: root.level === "ok"
                anchors.fill: parent
                preferredRendererType: Shape.CurveRenderer
                ShapePath {
                    strokeColor: root.ui.c.onAccent
                    strokeWidth: Math.max(2, root.ui.px(2.2))
                    fillColor: "transparent"
                    capStyle: ShapePath.RoundCap
                    joinStyle: ShapePath.RoundJoin
                    startX: iconBox.width * 0.28; startY: iconBox.height * 0.52
                    PathLine { x: iconBox.width * 0.44; y: iconBox.height * 0.68 }
                    PathLine { x: iconBox.width * 0.73; y: iconBox.height * 0.35 }
                }
            }
            Shape {
                visible: root.level === "stop"
                anchors.fill: parent
                preferredRendererType: Shape.CurveRenderer
                ShapePath {
                    strokeColor: root.ui.c.onAccent
                    strokeWidth: Math.max(2, root.ui.px(2.2))
                    fillColor: "transparent"
                    capStyle: ShapePath.RoundCap
                    startX: iconBox.width * 0.33; startY: iconBox.height * 0.33
                    PathLine { x: iconBox.width * 0.67; y: iconBox.height * 0.67 }
                    PathMove { x: iconBox.width * 0.67; y: iconBox.height * 0.33 }
                    PathLine { x: iconBox.width * 0.33; y: iconBox.height * 0.67 }
                }
            }
            Text {
                visible: root.level === "warn"
                anchors.centerIn: parent
                text: "!"
                color: root.ui.c.onAccent
                font.family: root.ui.bodyFont
                font.pixelSize: root.ui.px(15)
                font.bold: true
            }
            Shape {
                id: spinner
                visible: root.level === "busy"
                anchors.fill: parent
                preferredRendererType: Shape.CurveRenderer
                ShapePath {
                    strokeColor: root.ui.c.line
                    strokeWidth: Math.max(2, root.ui.px(3))
                    fillColor: "transparent"
                    PathAngleArc {
                        centerX: iconBox.width / 2; centerY: iconBox.height / 2
                        radiusX: iconBox.width / 2 - root.ui.px(3); radiusY: radiusX
                        startAngle: 0; sweepAngle: 360
                    }
                }
                ShapePath {
                    strokeColor: root.ui.c.accentHi
                    strokeWidth: Math.max(2, root.ui.px(3))
                    fillColor: "transparent"
                    capStyle: ShapePath.RoundCap
                    PathAngleArc {
                        centerX: iconBox.width / 2; centerY: iconBox.height / 2
                        radiusX: iconBox.width / 2 - root.ui.px(3); radiusY: radiusX
                        startAngle: -90; sweepAngle: 90
                    }
                }
                RotationAnimator on rotation {
                    from: 0
                    to: 360
                    duration: 900
                    loops: Animation.Infinite
                    running: spinner.visible
                }
            }
        }

        Text {
            id: wordsText
            anchors.verticalCenter: parent.verticalCenter
            width: Math.min(implicitWidth, leftPart.width - iconBox.width - leftPart.spacing)
            text: root.words || ""
            elide: Text.ElideRight
            color: root.level === "warn" || root.level === "stop" ? root.readable(root.levelColor(root.level), root.ui.c.panel) : root.ui.c.text
            font.family: root.ui.bodyFont
            font.pixelSize: root.ui.px(15)
            font.weight: Font.DemiBold
        }
        Text {
            visible: text !== ""
            anchors.verticalCenter: parent.verticalCenter
            width: Math.max(0, leftPart.width - iconBox.width - wordsText.width - leftPart.spacing * 2)
            text: root.detail || ""
            elide: Text.ElideRight
            color: root.ui.c.muted
            font.family: root.ui.bodyFont
            font.pixelSize: root.ui.px(14)
        }
    }

    Row {
        id: rightPart
        anchors.right: parent.right
        anchors.rightMargin: root.ui.px(8)
        anchors.verticalCenter: parent.verticalCenter
        spacing: root.ui.px(8)

        // Progress: a brass bar and its words.
        Rectangle {
            visible: root.progress >= 0
            width: root.ui.px(150)
            height: root.ui.px(6)
            radius: height / 2
            anchors.verticalCenter: parent.verticalCenter
            color: root.ui.c.line
            Rectangle {
                width: parent.width * Math.max(0, Math.min(1, root.progress))
                height: parent.height
                radius: parent.radius
                gradient: Gradient {
                    orientation: Gradient.Horizontal
                    GradientStop { position: 0; color: root.ui.c.accentLo }
                    GradientStop { position: 1; color: root.ui.c.accentHi }
                }
            }
        }
        Text {
            visible: root.progress >= 0 && text !== ""
            anchors.verticalCenter: parent.verticalCenter
            text: root.progressText || ""
            color: root.ui.c.soft
            font.family: root.ui.bodyFont
            font.pixelSize: root.ui.px(14)
        }

        Rectangle {
            visible: root.progress < 0 && root.pillText !== ""
            width: pillRow.implicitWidth + root.ui.px(20)
            height: root.ui.px(30)
            radius: root.ui.px(6)
            anchors.verticalCenter: parent.verticalCenter
            color: Qt.tint(root.ui.c.panel, Qt.rgba(root.ui.c.accent.r, root.ui.c.accent.g, root.ui.c.accent.b, 0.16))
            Row {
                id: pillRow
                readonly property color bg: parent.color
                anchors.centerIn: parent
                spacing: root.ui.px(6)
                KeyCap { visible: root.pillKey !== ""; ui: root.ui; k: root.pillKey; anchors.verticalCenter: parent.verticalCenter }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: root.pillText || ""
                    color: root.readable(root.ui.c.accentHi, pillRow.bg)
                    font.family: root.ui.bodyFont
                    font.pixelSize: root.ui.px(14)
                    font.weight: Font.DemiBold
                }
            }
            HoverHandler { onHoveredChanged: root.ui.hover(parent, hovered) }
            TapHandler { onTapped: root.pillClicked() }
        }
    }

    // A controller button drawn as a small cap.
    component KeyCap: Rectangle {
        required property var ui
        property string k: ""
        implicitWidth: Math.max(ui.px(24), capLabel.implicitWidth + ui.px(12))
        implicitHeight: ui.px(24)
        radius: k.length > 1 ? ui.px(6) : height / 2
        color: ui.c.raised
        border.color: ui.c.accentLo
        border.width: Math.max(1, ui.px(1))
        Text {
            id: capLabel
            anchors.centerIn: parent
            text: parent.k.toUpperCase()
            color: parent.ui.c.text
            font.family: parent.ui.bodyFont
            font.pixelSize: parent.ui.px(14)
            font.bold: true
        }
    }
}
