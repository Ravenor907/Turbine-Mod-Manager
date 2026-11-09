import QtQuick

// One mod in a game's list: order number (only when order matters), an ON or OFF pill, the name, one line under it,
// one optional badge and, on the focused row, what A and X do.
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

    property string name: ""
    property string detail: ""            // one line: "by STuRoK · added Oct 21", or a problem ("Needs SKSE")
    property string detailLevel: ""       // "" plain, or warn / stop / info for a problem in the line
    property bool isOn: true
    property bool ordered: false          // show the order number (only for games where order matters)
    property int order: 0
    property string badge: ""             // e.g. "Needs SKSE"; empty hides it
    property string badgeLevel: "warn"    // ok, info, warn, stop
    property bool focused: false
    property bool showHints: focused
    property string aLabel: isOn ? "Turn off" : "Turn on"
    property string xLabel: "Options"

    // A tap or click anywhere on the row (the pill included) is clicked(): Main.qml runs A on it.
    signal clicked()
    signal menuClicked()
    readonly property Item pillItem: pill

    function levelColor(level) {
        if (level === "ok") return ui.c.ok
        if (level === "info") return ui.c.accentHi
        if (level === "stop") return ui.c.stop !== undefined ? ui.c.stop : (ui.c.light ? Qt.rgba(0.557, 0.165, 0.118, 1) : Qt.rgba(0.941, 0.565, 0.478, 1))
        return ui.c.warn
    }
    function wash(col, amount) { return Qt.tint(ui.c.card, Qt.rgba(col.r, col.g, col.b, amount)) }

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

    // Screen readers: name, role, then what it is ("Lantern Glow, on, by STuRoK, 2 of 9").
    property int position: 0
    property int count: 0
    Accessible.role: Accessible.ListItem
    Accessible.name: name + ", " + (isOn ? "on" : "off") + (detail !== "" ? ", " + detail : "") + (badge !== "" ? ", " + badge : "")
                     + (count > 0 ? ", " + position + " of " + count : "")
    Accessible.focused: focused
    implicitWidth: ui.px(600)
    property bool compact: false          // Desktop layout: 38 px rows
    property int ticked: -1               // Select several: -1 no box, 0 empty box, 1 ticked
    implicitHeight: Math.max(ui.px(compact ? 38 : 54), rowWords.implicitHeight + ui.px(12))
    radius: ui.px(6)
    color: ui.c.card

    // Focus: brass glow ring, warm gradient and a brass border.
    Rectangle {
        visible: root.focused
        anchors.fill: parent
        anchors.margins: -root.ui.px(3)
        radius: root.radius + root.ui.px(3)
        color: "transparent"
        border.width: root.ui.px(3)
        border.color: Qt.rgba(root.ui.c.accent.r, root.ui.c.accent.g, root.ui.c.accent.b, 0.14)
    }
    Rectangle {
        visible: root.focused
        anchors.fill: parent
        radius: root.radius
        border.width: Math.max(1, root.ui.px(1.5))
        border.color: root.ui.c.accent
        gradient: Gradient {
            orientation: Gradient.Horizontal
            GradientStop { position: 0; color: root.ui.c.focusFill }
            GradientStop { position: 1; color: Qt.tint(root.ui.c.card, Qt.rgba(root.ui.c.focusFill.r, root.ui.c.focusFill.g, root.ui.c.focusFill.b, 0.5)) }
        }
    }

    HoverHandler { onHoveredChanged: root.ui.hover(parent, hovered) }
    // Touch: hold a moment for the mod's options (like X).
    TapHandler { longPressThreshold: 0.5; onTapped: root.clicked(); onLongPressed: root.menuClicked() }
    TapHandler { acceptedButtons: Qt.RightButton; onTapped: root.menuClicked() }

    Row {
        id: leftPart
        anchors.left: parent.left
        anchors.leftMargin: root.ui.px(12)
        anchors.verticalCenter: parent.verticalCenter
        spacing: root.ui.px(12)

        // Select several: a box, ticked with A.
        Rectangle {
            visible: root.ticked >= 0
            width: root.ui.px(22); height: width; radius: root.ui.px(4)
            anchors.verticalCenter: parent.verticalCenter
            color: root.ticked === 1 ? root.ui.c.accent : "transparent"
            border.width: Math.max(1, root.ui.px(2))
            border.color: root.ticked === 1 ? root.ui.c.accent : root.ui.c.soft
            Rectangle {
                anchors.centerIn: parent
                visible: root.ticked === 1
                width: root.ui.px(10); height: width; radius: root.ui.px(2)
                color: root.ui.c.onAccent
            }
        }

        Rectangle {
            visible: root.ordered
            width: Math.max(root.ui.px(24), orderText.implicitWidth + root.ui.px(8))
            height: root.ui.px(24)
            radius: root.ui.px(6)
            anchors.verticalCenter: parent.verticalCenter
            color: root.ui.c.raised
            Text {
                id: orderText
                anchors.centerIn: parent
                text: root.order > 0 ? String(root.order) : ""
                color: root.ui.c.soft
                font.family: root.ui.bodyFont
                font.pixelSize: root.ui.fpx(14)
                font.bold: true
            }
        }

        // ON is a filled brass pill; OFF is an outlined ring, so the shape says it too.
        Rectangle {
            id: pill
            width: root.ui.px(42)
            height: root.ui.px(24)
            radius: root.ui.px(6)
            anchors.verticalCenter: parent.verticalCenter
            color: root.isOn ? root.wash(root.ui.c.accent, 0.28) : root.ui.c.ground
            border.width: root.isOn ? 0 : Math.max(1, root.ui.px(1.5))
            border.color: root.ui.c.line
            Text {
                anchors.centerIn: parent
                text: root.isOn ? "ON" : "OFF"
                color: root.isOn ? root.readable(root.ui.c.accentHi, pill.color) : root.ui.c.muted
                font.family: root.ui.bodyFont
                font.pixelSize: root.ui.fpx(14)
                font.bold: true
                font.letterSpacing: root.ui.px(0.4)
            }
        }
    }

    Row {
        id: rightPart
        anchors.right: parent.right
        anchors.rightMargin: root.ui.px(12)
        anchors.verticalCenter: parent.verticalCenter
        spacing: root.ui.px(12)

        Rectangle {
            visible: root.badge !== ""
            width: badgeText.implicitWidth + root.ui.px(18)
            height: root.ui.px(26)
            radius: root.ui.px(6)
            anchors.verticalCenter: parent.verticalCenter
            color: root.wash(root.levelColor(root.badgeLevel), 0.16)
            Text {
                id: badgeText
                anchors.centerIn: parent
                text: root.badge || ""
                color: root.readable(root.levelColor(root.badgeLevel), parent.color)
                font.family: root.ui.bodyFont
                font.pixelSize: root.ui.fpx(14)
                font.weight: Font.DemiBold
            }
        }

        Row {
            visible: root.showHints
            spacing: root.ui.px(12)
            anchors.verticalCenter: parent.verticalCenter
            Row {
                spacing: root.ui.px(6)
                visible: root.aLabel !== ""
                KeyCap { ui: root.ui; k: "a"; anchors.verticalCenter: parent.verticalCenter }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: root.aLabel || ""
                    color: root.ui.c.soft
                    font.family: root.ui.bodyFont
                    font.pixelSize: root.ui.fpx(14)
                }
            }
            Row {
                spacing: root.ui.px(6)
                visible: root.xLabel !== ""
                KeyCap { ui: root.ui; k: "x"; anchors.verticalCenter: parent.verticalCenter }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: root.xLabel || ""
                    color: root.ui.c.soft
                    font.family: root.ui.bodyFont
                    font.pixelSize: root.ui.fpx(14)
                }
            }
        }
    }

    Column {
        id: rowWords
        anchors.left: leftPart.right
        anchors.leftMargin: root.ui.px(12)
        anchors.right: rightPart.left
        anchors.rightMargin: root.ui.px(12)
        anchors.verticalCenter: parent.verticalCenter
        spacing: root.ui.px(1)
        Text {
            width: parent.width
            text: root.name || ""
            // 0.6: a name is never cut mid-word: it wraps (the line under it gives way first).
            wrapMode: Text.WordWrap
            // Off rows are dimmer but stay readable (muted keeps at least 4.5:1 on every theme).
            color: root.isOn ? root.ui.c.text : root.focused ? root.ui.c.soft : root.ui.c.muted
            font.family: root.ui.bodyFont
            font.pixelSize: root.ui.fpx(16)
            font.weight: root.isOn ? Font.DemiBold : Font.Medium
        }
        Text {
            visible: text !== ""
            width: parent.width
            text: root.detail || ""
            elide: Text.ElideRight
            color: root.detailLevel !== "" ? root.readable(root.levelColor(root.detailLevel), root.focused ? root.ui.c.focusFill : root.ui.c.card)
                   : root.focused ? root.ui.c.soft : root.ui.c.muted
            font.weight: root.detailLevel !== "" ? Font.DemiBold : Font.Normal
            font.family: root.ui.bodyFont
            font.pixelSize: root.ui.fpx(14)
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
            font.pixelSize: parent.ui.fpx(14)
            font.bold: true
        }
    }
}
