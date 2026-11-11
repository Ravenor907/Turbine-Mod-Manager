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
    property real bottomInset: ui.px(8)   // the sheet layer already ends above the bottom bar

    signal activated(int index)
    signal closed()

    // Keep the focused item in view when the list is taller than the sheet.
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

    anchors.fill: parent

    Rectangle {
        anchors.fill: parent
        color: Qt.rgba(0.02, 0.016, 0.012, 0.74)
    }

    Rectangle {
        id: sheet
        // The top and right edges sit just outside the parent, so only the left and bottom lines show.
        anchors.right: parent.right
        anchors.rightMargin: -border.width
        anchors.top: parent.top
        anchors.topMargin: -border.width
        border.width: Math.max(1, root.ui.px(1.5))
        border.color: root.ui.c.line
        width: Math.min(parent.width - root.ui.px(40), root.ui.px(392))
        height: Math.max(root.ui.px(120), parent.height - root.bottomInset + border.width)
        color: root.ui.c.panel
        bottomLeftRadius: root.ui.px(6)

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
                elide: Text.ElideRight
                color: root.ui.c.text
                font.family: root.ui.titleFont
                font.pixelSize: root.ui.fpx(17)
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
