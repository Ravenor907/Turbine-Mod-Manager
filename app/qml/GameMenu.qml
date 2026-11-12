import QtQuick

// The game menu that View opens: every row from Main.qml's gameMenuItems, in sections (Play, Mods, Fix a
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
        // (Not laid out yet: flick.height changing calls this again.)
        if (!it || flick.height <= it.height) return
        // The section head comes into view with its first row; a little room around the row clears the fades.
        const m = Math.min(root.ui.px(20), (flick.height - it.height) / 2)
        if (it.y - m < flick.contentY) flick.contentY = Math.max(0, it.y - m)
        else if (it.y + it.height + m > flick.contentY + flick.height) flick.contentY = Math.min(Math.max(0, flick.contentHeight - flick.height), it.y + it.height + m - flick.height)
        // The last row scrolls all the way, past the fade.
        if (focusIndex === itemRepeater.count - 1) flick.contentY = Math.max(0, flick.contentHeight - flick.height)
    }
    onFocusIndexChanged: Qt.callLater(showFocused)
    onVisibleChanged: if (visible) { flick.contentY = 0; Qt.callLater(showFocused) }

    anchors.fill: parent

    Scrim { ui: root.ui }

    Card {
        id: card
        ui: root.ui
        readonly property real fullHeight: head.implicitHeight + body.height + root.ui.px(20 + 10 + 16 + 6)
        width: Math.min(parent.width - root.ui.px(40), root.ui.px(520))
        height: Math.min(fullHeight, parent.height - root.ui.px(24))
        x: (parent.width - width) / 2
        y: (parent.height - height) / 2

        CardGuard {}   // taps on the card don't close it

        Column {
            id: head
            x: root.ui.px(20)
            y: root.ui.px(20)
            width: parent.width - root.ui.px(40)
            spacing: root.ui.px(3)
            Text {
                width: parent.width
                text: root.gameName || ""
                elide: Text.ElideRight
                color: root.ui.c.text
                font.family: root.ui.titleFont
                font.pixelSize: root.ui.fpx(19)
                font.bold: true
            }
            Text {
                visible: text !== ""
                width: parent.width
                text: root.subtitle || ""
                color: root.ui.c.muted
                font.family: root.ui.bodyFont
                font.pixelSize: root.ui.fpx(14)
            }
        }

        Flickable {
            id: flick
            anchors.top: head.bottom
            anchors.topMargin: root.ui.px(10)
            anchors.bottom: parent.bottom
            anchors.bottomMargin: root.ui.px(16)
            x: root.ui.px(14)
            width: parent.width - root.ui.px(28)
            clip: true
            contentWidth: width
            // A little room under the last row, so it never sits against the sheet's edge.
            contentHeight: body.height + root.ui.px(6)
            boundsBehavior: Flickable.StopAtBounds
            onHeightChanged: if (root.visible) Qt.callLater(root.showFocused)

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
                            color: root.ui.c.accentText
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
        ScrollFade { ui: root.ui; view: flick; color: root.ui.c.panel; chevronBelow: true }
    }
}
