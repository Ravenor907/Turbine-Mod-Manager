import QtQuick

// The X menu for one mod: a side sheet on the right over a dimmed backdrop, so the list stays in view behind it.
// A title and a line under it, then the actions. It fills its parent; show and hide it with visible. Main.qml also
// uses it for the card menu, the Y pop-up and the pick lists.
//
// ui (required): Main.qml's kit (colors, sizes, fonts, motion and helpers; see COMPONENTS.md).
Overlay {
    id: root
    onOutsideTapped: root.closed()

    required property var ui
    motion: ui.motion

    property string title: ""             // e.g. the mod's name
    property string subtitle: ""          // e.g. "0.4.19.16 · Nexus · On, 3rd in load order"
    // Each item: { label, detail, enabled (default true), icon (a name from MenuEntry's icons or SVG path data on a
    // 24 x 24 grid), danger (bool, label in the warning color), more (bool, shows › for a next step) }
    property var items: []
    property int focusIndex: 0
    property real bottomInset: 0          // the sheet layer already ends above the bottom bar

    signal activated(int index)
    signal closed()

    // Keep the focused item in view when the list is taller than the sheet.
    function showFocused() {
        const it = itemRepeater.itemAt(focusIndex)
        // (Not laid out yet: flick.height changing calls this again.)
        if (!it || flick.height <= it.height) return
        // A little room around the row clears the fades.
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

    // A side sheet (SideSheet.qml): full height down to the bottom bar, flush with the right edge.
    SideSheet {
        id: sheet
        ui: root.ui
        width: Math.min(parent.width - root.ui.px(40), root.ui.px(392))
        height: Math.max(root.ui.px(120), parent.height - root.bottomInset)

        CardGuard {}   // taps on the sheet don't close it

        Column {
            id: head
            x: root.ui.px(18)
            y: root.ui.px(16)
            width: parent.width - root.ui.px(32)
            spacing: root.ui.px(3)
            Text {
                width: parent.width
                text: root.title || ""
                wrapMode: Text.WordWrap
                maximumLineCount: 3
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
                wrapMode: Text.WordWrap
                color: root.ui.c.muted
                font.family: root.ui.bodyFont
                font.pixelSize: root.ui.fpx(14)
            }
            Item { width: 1; height: root.ui.px(7) }
            Rectangle { width: parent.width; height: Math.max(1, root.ui.px(1)); color: root.ui.c.line }
        }

        Flickable {
            id: flick
            anchors.top: head.bottom
            anchors.topMargin: root.ui.px(8)
            anchors.bottom: parent.bottom
            anchors.bottomMargin: root.ui.px(10)
            x: root.ui.px(14)
            width: parent.width - root.ui.px(26)
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
                    model: root.items || []
                    MenuEntry {
                        required property int index
                        required property var modelData
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
                        Accessible.name: (modelData.label || "") + (modelData.detail ? ", " + modelData.detail : "") + ", " + (index + 1) + " of " + (root.items || []).length
                        Accessible.focused: index === root.focusIndex
                        // Curved icons aren't clipped by the list, so a row cut off at an edge hides its icon.
                        iconShown: y >= flick.contentY - 1 && y + height <= flick.contentY + flick.height + 1
                        dangerColor: root.ui.readable(root.ui.c.warn, root.ui.c.panel)
                        onClicked: root.activated(index)
                    }
                }
            }
        }
        ScrollFade { ui: root.ui; view: flick; color: root.ui.c.panel }
    }
}
