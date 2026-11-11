import QtQuick
import QtQuick.Shapes

// "Before you play": the check Play runs before a modded launch. One line per check (a passed check, a warning or
// a stop), up to 4 buttons (SheetButtons) and an optional note under them. A centered card over a
// dimmed backdrop. It fills its parent; show and hide it with visible.
//
// ui (required): Main.qml's kit (colors, sizes, fonts, motion and helpers; see COMPONENTS.md).
Overlay {
    id: root
    onOutsideTapped: root.closed()
    // Plain words as StyledText. Options that start with two hyphens ("--launcher-skip") get a thin space between
    // the hyphens and show in bold: in Barlow the two hyphens otherwise touch and read as one long dash.
    function styled(t) {
        const esc = t.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/\n/g, "<br>")
        return esc.replace(/(^|\s)--([A-Za-z0-9][A-Za-z0-9-]*)/g, "$1<b>-\u2009-$2</b>")
    }

    required property var ui
    motion: ui.motion

    property string title: "Before you play"
    // Each check: { level: "ok" | "warn" | "stop", title, detail }
    property var checks: []
    // Up to 4 buttons: { label, main (true for the one drawn in brass) }. A runs the focused one.
    property var buttons: []
    property int focusIndex: 0
    property string note: ""              // e.g. "Wait checks again every few hours."
    readonly property bool scrolls: checkList.contentHeight > checkList.height + 1

    signal activated(int index, string key)
    signal closed()

    // Up and Down scroll the lines when they don't all fit (the buttons always stay in view).
    function scroll(d) {
        checkList.contentY = Math.max(0, Math.min(checkList.contentHeight - checkList.height, checkList.contentY + d * ui.px(80)))
    }
    onChecksChanged: checkList.contentY = 0

    function levelColor(level) {
        if (level === "ok") return ui.c.ok
        if (level === "stop") return ui.c.stop
        return ui.c.warn
    }

    anchors.fill: parent

    Rectangle {
        anchors.fill: parent
        color: Qt.rgba(0.02, 0.016, 0.012, 0.74)
    }

    Rectangle {
        id: card
        anchors.centerIn: parent
        width: Math.min(parent.width - root.ui.px(40), root.ui.px(560))
        // Clear of the controls legend at the bottom of the window.
        height: Math.min(content.implicitHeight + root.ui.px(34), parent.height - root.ui.px(112))
        radius: root.ui.px(6)
        color: root.ui.c.panel
        border.width: Math.max(1, root.ui.px(1.5))
        border.color: root.ui.c.accentLo
        clip: true

        CardGuard {}   // taps on the card don't close it

        Column {
            id: content
            x: root.ui.px(20)
            y: root.ui.px(18)
            width: parent.width - root.ui.px(40)
            spacing: root.ui.px(8)

            Text {
                id: titleText
                width: parent.width
                text: root.title || ""
                elide: Text.ElideRight
                color: root.ui.c.text
                font.family: root.ui.titleFont
                font.pixelSize: root.ui.fpx(18)
                font.bold: true
            }

            Flickable {
                id: checkList
                width: parent.width
                // What's left of the screen after the title, the buttons and the note.
                height: Math.min(checkCol.implicitHeight, root.height - root.ui.px(112) - root.ui.px(34) - titleText.height
                                 - buttonFlow.height - (noteText.visible ? noteText.height + content.spacing : 0) - content.spacing * 2)
                contentHeight: checkCol.implicitHeight
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                Column {
                    id: checkCol
                    width: checkList.width
                    spacing: content.spacing
            Repeater {
                model: root.checks || []
                Rectangle {
                    id: checkRow
                    required property var modelData
                    readonly property string level: modelData.level || "ok"
                    readonly property bool flagged: level !== "ok"
                    readonly property color hue: root.levelColor(level)
                    readonly property color tone: root.ui.readable(hue, color)
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
                            font.pixelSize: root.ui.fpx(16)
                            font.weight: Font.DemiBold
                        }
                        Text {
                            visible: (checkRow.modelData.detail || "") !== ""
                            width: parent.width
                            // Options like --launcher-skip with their two hyphens shown apart.
                            textFormat: Text.StyledText
                            text: root.styled(checkRow.modelData.detail || "")
                            wrapMode: Text.WordWrap
                            color: checkRow.flagged ? root.ui.c.soft : root.ui.c.muted
                            font.family: root.ui.bodyFont
                            font.pixelSize: root.ui.fpx(14)
                        }
                    }
                }
            }
                }
            }

            SheetButtons {
                id: buttonFlow
                ui: root.ui
                width: parent.width
                topPadding: root.ui.px(6)
                labels: (root.buttons || []).slice(0, 4).map(b => b.label || "")
                focusIndex: root.focusIndex
                mainIndex: (root.buttons || []).findIndex(b => b.main === true)
                onActivated: (i) => root.activated(i, "")
            }

            Text {
                id: noteText
                visible: text !== ""
                width: parent.width
                text: root.note || ""
                wrapMode: Text.WordWrap
                color: root.ui.c.soft
                font.family: root.ui.bodyFont
                font.pixelSize: root.ui.fpx(14)
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
            font.pixelSize: dot.ui.fpx(15)
            font.bold: true
        }
    }
}
