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
    property string detail: ""            // one line, e.g. "5.2 · Nexus"
    property bool isOn: true
    property bool ordered: false          // show the order number (only for games where order matters)
    property int order: 0
    property string badge: ""             // e.g. "Needs SKSE"; empty hides it
    property string badgeLevel: "warn"    // ok, info, warn, stop
    property bool focused: false
    property bool showHints: focused
    property string aLabel: isOn ? "Turn off" : "Turn on"
    property string xLabel: "More"

    signal clicked()
    signal toggleClicked()
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

    implicitWidth: ui.px(600)
    property bool compact: false          // Desktop layout: 38 px rows
    implicitHeight: ui.px(compact ? 38 : 54)
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
    TapHandler { onTapped: root.clicked() }
    TapHandler { acceptedButtons: Qt.RightButton; onTapped: root.menuClicked() }

    Row {
        id: leftPart
        anchors.left: parent.left
        anchors.leftMargin: root.ui.px(12)
        anchors.verticalCenter: parent.verticalCenter
        spacing: root.ui.px(12)

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
                font.pixelSize: root.ui.px(14)
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
                font.pixelSize: root.ui.px(14)
                font.bold: true
                font.letterSpacing: root.ui.px(0.4)
            }
            HoverHandler { onHoveredChanged: root.ui.hover(parent, hovered) }
            TapHandler { onTapped: root.toggleClicked() }
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
                font.pixelSize: root.ui.px(14)
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
                    font.pixelSize: root.ui.px(14)
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
                    font.pixelSize: root.ui.px(14)
                }
            }
        }
    }

    Column {
        anchors.left: leftPart.right
        anchors.leftMargin: root.ui.px(12)
        anchors.right: rightPart.left
        anchors.rightMargin: root.ui.px(12)
        anchors.verticalCenter: parent.verticalCenter
        spacing: root.ui.px(1)
        Text {
            width: parent.width
            text: root.name || ""
            elide: Text.ElideRight
            // Off rows are dimmer but stay readable (muted keeps at least 4.5:1 on every theme).
            color: root.isOn ? root.ui.c.text : root.focused ? root.ui.c.soft : root.ui.c.muted
            font.family: root.ui.bodyFont
            font.pixelSize: root.ui.px(16)
            font.weight: root.isOn ? Font.DemiBold : Font.Medium
        }
        Text {
            visible: text !== ""
            width: parent.width
            text: root.detail || ""
            elide: Text.ElideRight
            color: root.focused ? root.ui.c.soft : root.ui.c.muted
            font.family: root.ui.bodyFont
            font.pixelSize: root.ui.px(14)
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
