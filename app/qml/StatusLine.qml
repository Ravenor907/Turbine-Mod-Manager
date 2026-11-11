import QtQuick
import QtQuick.Shapes

// A line that says in words what is going on: a state icon, bold state words and a muted detail (the FOMOD
// installer's top line).
//
// ui (required): Main.qml's kit (colors, sizes, fonts, motion and helpers; see COMPONENTS.md).
Rectangle {
    id: root

    required property var ui

    property string level: "ok"           // ok, busy, warn, stop (named level because Item already has state)
    property string words: ""             // bold, e.g. "Ready to play"
    property string detail: ""            // muted, e.g. "15 of 17 mods on"

    function levelColor(level) {
        if (level === "ok") return ui.c.ok
        if (level === "stop") return ui.c.stop
        if (level === "busy") return ui.c.accentHi
        return ui.c.warn
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
        anchors.right: parent.right
        anchors.rightMargin: root.ui.px(18)
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
                color: root.ui.readable(root.levelColor(root.level), root.ui.c.panel)
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
                font.pixelSize: root.ui.fpx(15)
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
                    duration: root.ui.motion.spin
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
            color: root.level === "warn" || root.level === "stop" ? root.ui.readable(root.levelColor(root.level), root.ui.c.panel) : root.ui.c.text
            font.family: root.ui.bodyFont
            font.pixelSize: root.ui.fpx(15)
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
            font.pixelSize: root.ui.fpx(14)
        }
    }
}
