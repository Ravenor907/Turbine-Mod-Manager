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
    property int cancelIndex: -1          // the button B presses (Not now)
    readonly property bool scrolls: card.scrolls
    readonly property bool cancelHidden: card.buttonRow.cancelHidden

    signal activated(int index, string key)
    signal closed()

    // Up and Down scroll the lines when they don't all fit (the buttons always stay in view).
    function scroll(d) { card.scrollBy(d) }
    onChecksChanged: card.toTop()

    function levelColor(level) {
        if (level === "ok") return ui.c.ok
        if (level === "stop") return ui.c.stop
        return ui.c.warn
    }

    anchors.fill: parent

    Scrim { ui: root.ui }

    DialogCard {
        id: card
        ui: root.ui
        maxWidth: 600
        title: root.title || ""
        buttons: (root.buttons || []).slice(0, 4).map(b => b.label || "")
        focusIndex: root.focusIndex
        // The main button: the one marked, else the first (what the check recommends).
        mainIndex: Math.max(0, (root.buttons || []).findIndex(b => b.main === true))
        cancelIndex: root.cancelIndex
        note: root.note || ""
        onActivated: (i) => root.activated(i, "")
        Repeater {
            model: root.checks || []
            Rectangle {
                id: checkRow
                required property var modelData
                readonly property string level: modelData.level || "ok"
                readonly property bool flagged: level !== "ok"
                readonly property color hue: root.levelColor(level)
                readonly property color tone: root.ui.readable(hue, color)
                width: card.bodyColumn.width
                height: Math.max(root.ui.px(44), checkText.implicitHeight + root.ui.px(18))
                radius: root.ui.radiusSmall
                color: flagged ? Qt.tint(root.ui.c.card, Qt.rgba(hue.r, hue.g, hue.b, 0.1)) : root.ui.c.card
                border.width: flagged ? Math.max(1, root.ui.px(1)) : 0
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
                    spacing: root.ui.px(2)
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
