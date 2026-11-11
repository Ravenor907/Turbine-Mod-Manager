import QtQuick

// The game menu that View opens (0.59): every row from Main.qml's gameMenuItems, in sections (Play, Mods, Fix a
// problem, This game) with a small head over the first row of each. Rows that can't be used now are dimmed (A on
// one says why). A centered card over a dimmed backdrop. It fills its parent; show and hide it with visible.
//
// ui (required): Main.qml's kit (colors, sizes, fonts, motion and helpers; see COMPONENTS.md).
Overlay {
    id: root
    onOutsideTapped: root.closed()

    required property var ui
    motion: ui.motion

    property string gameName: ""
    property string subtitle: "Game menu"
    // The rows: { key, label, detail, enabled, icon, danger, more, group }
    property var items: []
    property int focusIndex: 0

    // index is the row's place in items; key is its key.
    signal activated(int index, string key)
    // A finger or the pointer pressed down on a row (true) and let go or slid off (false): the holds (Reset).
    signal held(int index, bool down)
    signal closed()

    // A row's item (the build's checks click it through the pointer).
    function entryItem(i) { const it = itemRepeater.itemAt(i); return it ? it.entry : null }
    readonly property var allItems: items || []

    function showFocused() {
        const it = itemRepeater.itemAt(focusIndex)
        if (!it) return
        // The section head comes into view with its first row.
        if (it.y < flick.contentY) flick.contentY = it.y
        else if (it.y + it.height > flick.contentY + flick.height) flick.contentY = it.y + it.height - flick.height
        // The last row scrolls all the way, past the fade.
        if (focusIndex === itemRepeater.count - 1) flick.contentY = Math.max(0, flick.contentHeight - flick.height)
    }
    onFocusIndexChanged: Qt.callLater(showFocused)
    onVisibleChanged: if (visible) { flick.contentY = 0; Qt.callLater(showFocused) }

    anchors.fill: parent

    Rectangle {
        anchors.fill: parent
        color: Qt.rgba(0.02, 0.016, 0.012, 0.74)
    }

    Rectangle {
        id: card
        readonly property real fullHeight: head.implicitHeight + body.height + root.ui.px(16 + 8 + 16)
        width: Math.min(parent.width - root.ui.px(40), root.ui.px(500))
        height: Math.min(fullHeight, parent.height - root.ui.px(40))
        x: (parent.width - width) / 2
        y: (parent.height - height) / 2
        color: root.ui.c.panel
        radius: root.ui.px(6)
        border.width: Math.max(1, root.ui.px(1.5))
        border.color: root.ui.c.accentLo

        CardGuard {}   // taps on the card don't close it

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
                font.pixelSize: root.ui.fpx(18)
                font.bold: true
            }
            Text {
                visible: text !== ""
                width: parent.width
                text: root.subtitle || ""
                color: root.ui.c.soft
                font.family: root.ui.bodyFont
                font.pixelSize: root.ui.fpx(14)
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
                    // By count, so a row whose words change (Keep holding A: 4...) updates in place: a rebuilt row
                    // would lose the finger or the pointer pressing it, and with it the release that stops a hold.
                    model: root.allItems.length
                    Column {
                        id: slot
                        required property int index
                        readonly property var modelData: root.allItems[index] || ({})
                        readonly property alias entry: slotEntry
                        readonly property string head: modelData.group && (index === 0 || (root.allItems[index - 1] || {}).group !== modelData.group) ? modelData.group : ""
                        width: body.width
                        spacing: root.ui.px(2)
                        // The section's head (never focused: Up and Down go row to row).
                        Text {
                            visible: slot.head !== ""
                            topPadding: slot.index === 0 ? 0 : root.ui.px(8)
                            leftPadding: root.ui.px(4)
                            text: slot.head
                            color: root.ui.c.muted
                            font.family: root.ui.bodyFont; font.pixelSize: root.ui.fpx(13); font.bold: true
                        }
                    MenuEntry {
                        id: slotEntry
                        readonly property int index: slot.index
                        readonly property var modelData: slot.modelData
                        ui: root.ui
                        width: body.width
                        label: modelData.label || ""
                        detail: modelData.detail || ""
                        icon: modelData.icon || ""
                        available: modelData.enabled !== false
                        danger: modelData.danger === true
                        more: modelData.more === true
                        focused: index === root.focusIndex
                        Accessible.role: Accessible.MenuItem
                        Accessible.name: (modelData.label || "") + (modelData.enabled === false ? ", not available" : "") + (modelData.detail ? ", " + modelData.detail : "") + ", " + (index + 1) + " of " + root.allItems.length
                        Accessible.focused: index === root.focusIndex
                        // Curved icons aren't clipped by the list, so a row cut off at an edge hides its icon.
                        iconShown: slot.y + y >= flick.contentY - 1 && slot.y + y + height <= flick.contentY + flick.height + 1
                        dangerColor: root.ui.readable(root.ui.c.warn, root.ui.c.panel)
                        onClicked: root.activated(index, modelData.key || "")
                        onHeld: (down) => root.held(index, down)
                    }
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
}
