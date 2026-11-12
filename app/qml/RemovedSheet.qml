import QtQuick

// Recently removed: one row per mod ("<name> · removed Nov 6 · 6 days left"), then "Delete all (2) permanently".
// A (or Put back) puts the focused mod back, X (or Delete now) deletes it permanently; both deletions ask first
// (Main.qml's hold-to-confirm sheet).
// Main.qml handles the keys.
Overlay {
    id: rs
    onOutsideTapped: rs.closed()
    required property var ui
    motion: ui.motion
    anchors.fill: parent

    property var mods: []                 // [{id, name, removedAt}]
    property int focusIndex: 0            // the mods, then Empty

    signal activated(int index)
    signal deleteClicked(int index)
    signal closed()

    function rowWords(m) {
        const at = new Date(m.removedAt * 1000)
        const left = Math.max(0, Math.min(7, Math.ceil((m.removedAt + 7 * 86400 - Date.now() / 1000) / 86400)))
        return Qt.formatDate(at, "MMM d") + " · " + (left === 1 ? "1 day left" : left + " days left")
    }

    Scrim { ui: rs.ui }
    onFocusIndexChanged: Qt.callLater(showFocused)
    onVisibleChanged: if (visible) { flick.contentY = 0; Qt.callLater(showFocused) }
    function showFocused() {
        const it = rows.itemAt(focusIndex)
        if (!it) return
        const y = it.y + list.y
        if (y < flick.contentY) flick.contentY = y
        else if (y + it.height > flick.contentY + flick.height) flick.contentY = y + it.height - flick.height
    }
    Card {
        ui: rs.ui
        anchors.centerIn: parent
        width: Math.min(parent.width - rs.ui.px(40), rs.ui.px(620))
        height: Math.min(parent.height - rs.ui.px(60), col.implicitHeight + rs.ui.px(40))
        clip: true
        CardGuard {}
        // Scrolls by touch and follows the focus (a long list at 150% runs past the card).
        Flickable {
            id: flick
            ScrollFade { ui: rs.ui; view: flick; color: rs.ui.c.panel }
            anchors.fill: parent
            anchors.margins: rs.ui.px(20)
            contentHeight: col.implicitHeight
            clip: true
            boundsBehavior: Flickable.StopAtBounds
        Column {
            id: col
            width: flick.width
            spacing: rs.ui.px(8)
            Text { text: "Removed mods"; color: rs.ui.c.text; font.family: rs.ui.bodyFont; font.pixelSize: rs.ui.fpx(19); font.weight: Font.DemiBold }
            Text { width: parent.width; wrapMode: Text.WordWrap; text: "Deleted automatically after 7 days."; color: rs.ui.c.soft; font.family: rs.ui.bodyFont; font.pixelSize: rs.ui.fpx(14) }
            Column {
                id: list
                width: parent.width
                spacing: rs.ui.px(8)
            Repeater {
                id: rows
                // The mods first, Delete all permanently last.
                model: rs.mods.concat([{ empty: true }])
                Rectangle {
                    id: row
                    required property int index
                    required property var modelData
                    readonly property bool foc: rs.focusIndex === index
                    width: list.width
                    height: Math.max(rs.ui.tpx(48), rowText.implicitHeight + rs.ui.px(12))
                    radius: rs.ui.px(6)
                    color: foc ? rs.ui.c.focusFill : rs.ui.c.card
                    border.color: rs.ui.c.accent
                    border.width: foc ? rs.ui.px(2) : 0
                    HoverHandler { onHoveredChanged: rs.ui.hover(parent, hovered) }
                    // Mouse areas, not tap handlers: a tap on a row's button must not also reach the row.
                    MouseArea { anchors.fill: parent; onClicked: rs.activated(row.index) }
                    Column {
                        id: rowText
                        x: rs.ui.px(14)
                        anchors.verticalCenter: parent.verticalCenter
                        width: parent.width - rs.ui.px(28) - buttons.width
                        Text {
                            width: parent.width
                            wrapMode: Text.WordWrap
                            text: row.modelData.empty ? "Delete all (" + rs.mods.length + ") permanently" : row.modelData.name
                            color: row.modelData.empty ? rs.ui.c.warn : rs.ui.c.text
                            font.family: rs.ui.bodyFont; font.pixelSize: rs.ui.fpx(16); font.weight: Font.DemiBold
                        }
                        Text {
                            visible: !row.modelData.empty
                            text: row.modelData.empty ? "" : "removed " + rs.rowWords(row.modelData)
                            color: rs.ui.c.muted
                            font.family: rs.ui.bodyFont; font.pixelSize: rs.ui.fpx(13)
                        }
                    }
                    // Every action is a button too (touch has no X): Put back, and Delete now.
                    Row {
                        id: buttons
                        anchors.right: parent.right; anchors.rightMargin: rs.ui.px(6)
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: rs.ui.px(6)
                        RowButton { objectName: "putBack"; visible: !row.modelData.empty; label: "Put back"; onClicked: rs.activated(row.index) }
                        RowButton { objectName: "deleteNow"; visible: !row.modelData.empty; label: "Delete now"; danger: true; onClicked: rs.deleteClicked(row.index) }
                    }
                }
            }
            }
        }
        }
    }

    component RowButton: Rectangle {
        id: rb
        property string label: ""
        property bool danger: false
        signal clicked()
        width: rbText.implicitWidth + rs.ui.px(20); height: rs.ui.tpx(40); radius: rs.ui.px(6)
        color: rs.ui.c.raised
        border.color: danger ? rs.ui.c.warn : rs.ui.c.line; border.width: Math.max(1, rs.ui.px(1))
        Text { id: rbText; anchors.centerIn: parent; text: rb.label; color: rb.danger ? rs.ui.c.warn : rs.ui.c.text; font.family: rs.ui.bodyFont; font.pixelSize: rs.ui.fpx(14); font.bold: true }
        HoverHandler { onHoveredChanged: rs.ui.hover(parent, hovered) }
        MouseArea { anchors.fill: parent; onClicked: rb.clicked() }
    }
}
