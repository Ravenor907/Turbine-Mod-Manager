import QtQuick
import QtQuick.Shapes

// The game menu that View opens: the safety tools for the whole game. Play without mods (no mods, nothing deleted),
// Restore last working setup (with when it was saved), then any extra items. A centered card over a dimmed
// backdrop like the approved mockup, or a side sheet with placement: "side". It fills its parent; show and hide it
// with visible.
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

    property string gameName: ""
    property string subtitle: "Game menu"
    property string originalDetail: "Starts the game with no mods. Nothing is deleted."
    property string restoreDate: ""       // e.g. "From Nov 4, 9:12 PM · 16 mods on"
    property bool restoreAvailable: restoreDate !== ""
    property string restoreEmptyText: "Nothing saved yet. Turbine saves your setup each time the game runs fine with mods."
    // Extra items after the two fixed ones: { key, label, detail, enabled, icon, danger, more } (see ModMenu)
    property var extraItems: []
    property int focusIndex: 0
    property string placement: "center"   // center or side
    property real bottomInset: ui.px(56)  // side placement: room left for the legend

    // index counts the fixed items too; key is "original", "restore" or the extra item's key.
    signal activated(int index, string key)
    signal closed()

    readonly property var allItems: [
        { key: "original", label: "Play without mods", detail: originalDetail, icon: "play", enabled: true },
        { key: "restore", label: "Restore last working setup", detail: restoreAvailable ? restoreDate : restoreEmptyText,
          icon: "restore", enabled: restoreAvailable, more: true }
    ].concat(extraItems || [])

    readonly property var icons: ({
        update: "M12 4v11M7 10l5 5 5-5M5 20h14",
        power: "M12 3v8M6.5 6.5a8 8 0 1 0 11 0",
        overlap: "M5 3h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z M11 9h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2h-8a2 2 0 0 1-2-2v-8a2 2 0 0 1 2-2z",
        files: "M14 3H6a1 1 0 0 0-1 1v16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V8z M14 3v5h5",
        move: "M8 4v16M4 8l4-4 4 4M16 20V4M12 16l4 4 4-4",
        remove: "M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13",
        play: "M7 4l13 8-13 8z",
        restore: "M4 4v6h6 M5 15a8 8 0 1 0 2-8.5L4 10",
        profile: "M12 4a4 4 0 1 1 0 8 4 4 0 0 1 0-8z M4 21c1-4 4-6 8-6s7 2 8 6",
        info: "M12 3a9 9 0 1 1 0 18 9 9 0 0 1 0-18z M12 11v6 M12 7.5v0.5"
    })
    function iconPath(v) { return v ? (icons[v] || v) : "" }

    function showFocused() {
        const it = itemRepeater.itemAt(focusIndex)
        if (!it) return
        if (it.y < flick.contentY) flick.contentY = it.y
        else if (it.y + it.height > flick.contentY + flick.height) flick.contentY = it.y + it.height - flick.height
        // The last row scrolls all the way, past the fade.
        if (focusIndex === itemRepeater.count - 1) flick.contentY = Math.max(0, flick.contentHeight - flick.height)
    }
    onFocusIndexChanged: Qt.callLater(showFocused)
    onVisibleChanged: if (visible) { flick.contentY = 0; Qt.callLater(showFocused) }

    readonly property bool side: placement === "side"

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
        readonly property real fullHeight: head.implicitHeight + body.height + root.ui.px(16 + 8 + 16)
        width: root.side ? Math.min(parent.width - root.ui.px(40), root.ui.px(392)) : Math.min(parent.width - root.ui.px(40), root.ui.px(500))
        height: root.side ? Math.max(root.ui.px(120), parent.height - root.bottomInset + border.width) : Math.min(fullHeight, parent.height - root.ui.px(40))
        // Side placement: the top and right edges sit just outside the parent, so only the left and bottom lines show.
        x: root.side ? parent.width - width + border.width : (parent.width - width) / 2
        y: root.side ? -border.width : (parent.height - height) / 2
        color: root.ui.c.panel
        radius: root.side ? 0 : root.ui.px(6)
        bottomLeftRadius: root.ui.px(6)
        border.width: Math.max(1, root.ui.px(1.5))
        border.color: root.side ? root.ui.c.line : root.ui.c.accentLo

        MouseArea { anchors.fill: parent }   // taps on the card don't close it

        Column {
            id: head
            x: root.ui.px(18)
            y: root.ui.px(16)
            width: parent.width - root.ui.px(36)
            spacing: root.ui.px(3)
            Text {
                width: parent.width
                text: root.gameName || ""
                elide: Text.ElideRight
                color: root.ui.c.text
                font.family: root.ui.titleFont
                font.pixelSize: root.ui.px(18)
                font.bold: true
            }
            Text {
                visible: text !== ""
                width: parent.width
                text: root.subtitle || ""
                color: root.ui.c.soft
                font.family: root.ui.bodyFont
                font.pixelSize: root.ui.px(14)
            }
        }

        Flickable {
            id: flick
            anchors.top: head.bottom
            anchors.topMargin: root.ui.px(8)
            anchors.bottom: parent.bottom
            anchors.bottomMargin: root.ui.px(16)
            x: root.ui.px(14)
            width: parent.width - root.ui.px(28)
            clip: true
            contentWidth: width
            // A little room under the last row, so it never sits against the sheet's edge.
            contentHeight: body.height + root.ui.px(6)
            boundsBehavior: Flickable.StopAtBounds

            Column {
                id: body
                width: flick.width
                spacing: root.ui.px(5)
                Repeater {
                    id: itemRepeater
                    model: root.allItems
                    MenuEntry {
                        required property int index
                        required property var modelData
                        ui: root.ui
                        width: body.width
                        label: modelData.label || ""
                        detail: modelData.detail || ""
                        iconData: root.iconPath(modelData.icon)
                        available: modelData.enabled !== false
                        danger: modelData.danger === true
                        more: modelData.more === true
                        focused: index === root.focusIndex
                        // Curved icons aren't clipped by the list, so a row cut off at an edge hides its icon.
                        iconShown: y >= flick.contentY - 1 && y + height <= flick.contentY + flick.height + 1
                        dangerColor: root.readable(root.ui.c.warn, root.ui.c.panel)
                        onClicked: root.activated(index, modelData.key || "")
                    }
                }
            }
        }
        // More rows below: a fade over the bottom edge of the list.
        Rectangle {
            visible: flick.contentY < flick.contentHeight - flick.height - 1
            anchors.left: flick.left
            anchors.right: flick.right
            anchors.bottom: flick.bottom
            height: root.ui.px(26)
            gradient: Gradient {
                GradientStop { position: 0; color: "transparent" }
                GradientStop { position: 1; color: root.ui.c.panel }
            }
        }
    }

    // One action in the sheet: icon tile, label, detail, and A on the focused one (or › when it leads further).
    component MenuEntry: Rectangle {
        id: entry
        required property var ui
        property string label: ""
        property string detail: ""
        property string iconData: ""
        property bool available: true
        property bool danger: false
        property bool more: false
        property bool focused: false
        property color dangerColor: ui.c.warn
        property bool iconShown: true
        signal clicked()

        implicitHeight: Math.max(ui.px(48), entryText.implicitHeight + ui.px(14))
        radius: ui.px(6)
        color: "transparent"

        // Focus: warm gradient with a brass border.
        Rectangle {
            visible: entry.focused
            anchors.fill: parent
            radius: entry.radius
            border.width: Math.max(1, entry.ui.px(1.5))
            border.color: entry.ui.c.accent
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0; color: entry.ui.c.focusFill }
                GradientStop { position: 1; color: Qt.tint(entry.ui.c.panel, Qt.rgba(entry.ui.c.focusFill.r, entry.ui.c.focusFill.g, entry.ui.c.focusFill.b, 0.5)) }
            }
        }

        HoverHandler { onHoveredChanged: entry.ui.hover(parent, hovered) }
        TapHandler { onTapped: entry.clicked() }

        Rectangle {
            id: entryIcon
            x: entry.ui.px(12)
            anchors.verticalCenter: parent.verticalCenter
            width: entry.ui.px(28)
            height: width
            radius: entry.ui.px(6)
            color: entry.focused ? Qt.tint(entry.ui.c.card, Qt.rgba(entry.ui.c.accent.r, entry.ui.c.accent.g, entry.ui.c.accent.b, 0.28)) : entry.ui.c.raised
            Shape {
                visible: entry.iconData !== "" && entry.iconShown
                anchors.centerIn: parent
                width: entry.ui.px(18)
                height: width
                preferredRendererType: Shape.CurveRenderer
                ShapePath {
                    strokeColor: entry.focused ? entry.ui.c.accentHi : entry.ui.c.soft
                    strokeWidth: 2
                    fillColor: "transparent"
                    capStyle: ShapePath.RoundCap
                    joinStyle: ShapePath.RoundJoin
                    scale: Qt.size(entry.ui.px(18) / 24, entry.ui.px(18) / 24)
                    PathSvg { path: entry.iconData }
                }
            }
        }

        Item {
            id: entryEnd
            anchors.right: parent.right
            anchors.rightMargin: entry.ui.px(12)
            anchors.verticalCenter: parent.verticalCenter
            width: entry.more && !entry.focused ? entryArrow.implicitWidth : 0
            height: entry.ui.px(24)
            // No A cap on the highlighted row: the legend says A Choose (never printed twice).
            KeyCap { id: entryKey; visible: false; ui: entry.ui; k: "a"; anchors.verticalCenter: parent.verticalCenter }
            Text {
                id: entryArrow
                visible: !entry.focused && entry.more
                anchors.verticalCenter: parent.verticalCenter
                text: "›"
                color: entry.ui.c.muted
                font.family: entry.ui.bodyFont
                font.pixelSize: entry.ui.px(18)
            }
        }

        Column {
            id: entryText
            anchors.left: entryIcon.right
            anchors.leftMargin: entry.ui.px(12)
            anchors.right: entryEnd.left
            anchors.rightMargin: entry.ui.px(8)
            anchors.verticalCenter: parent.verticalCenter
            spacing: entry.ui.px(1)
            Text {
                width: parent.width
                text: entry.label
                elide: Text.ElideRight
                color: !entry.available ? (entry.focused ? entry.ui.c.soft : entry.ui.c.muted) : entry.danger ? entry.dangerColor : entry.ui.c.text
                font.family: entry.ui.bodyFont
                font.pixelSize: entry.ui.px(16)
                font.weight: Font.DemiBold
            }
            Text {
                visible: text !== ""
                width: parent.width
                text: entry.detail
                wrapMode: Text.WordWrap
                lineHeight: 1.1
                color: entry.focused ? entry.ui.c.soft : entry.ui.c.muted
                font.family: entry.ui.bodyFont
                font.pixelSize: entry.ui.px(14)
            }
        }
    }

    // A controller button drawn as a small cap.
    component KeyCap: Rectangle {
        required property var ui
        property string k: ""
        implicitWidth: Math.max(ui.px(24), capLabel.implicitWidth + ui.px(12))
        implicitHeight: ui.px(24)
        width: implicitWidth
        height: implicitHeight
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
