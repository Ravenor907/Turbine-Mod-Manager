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
    // Up to 4 buttons: { key (the button's own key: "x", "y", "b", or "" for none), label }. A runs the focused one.
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
        HoverHandler { onHoveredChanged: btn.ui.hover(parent, hovered) }
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
                    font.pixelSize: btn.ui.fpx(14)
                    font.bold: true
                }
            }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: btn.label
                color: btn.focused ? btn.ui.c.onAccent : btn.ui.c.text
                font.family: btn.ui.bodyFont
                font.pixelSize: btn.ui.fpx(16)
                font.weight: Font.DemiBold
            }
        }
    }
}
