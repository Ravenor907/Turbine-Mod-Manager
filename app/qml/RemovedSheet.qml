import QtQuick

// Recently removed: "Empty (delete all 2 for good)", then one row per mod ("<name> · removed Nov 6 · 6 days left").
// A puts the focused mod back, X deletes it for good; both deletions ask first (Main.qml's hold-to-confirm sheet).
// Main.qml handles the keys.
Item {
    id: rs
    required property var ui
    anchors.fill: parent

    property var mods: []                 // [{id, name, removedAt}]
    property int focusIndex: 0            // 0 is Empty, then the mods
    property var now: new Date()

    signal activated(int index)
    signal deleteClicked(int index)
    signal closed()

    function rowWords(m) {
        const at = new Date(m.removedAt * 1000)
        const left = Math.max(0, Math.min(7, Math.ceil((m.removedAt + 7 * 86400 - Date.now() / 1000) / 86400)))
        return Qt.formatDate(at, "MMM d") + " · " + (left === 1 ? "1 day left" : left + " days left")
    }

    Rectangle { anchors.fill: parent; color: "#C0000000" }
    MouseArea { anchors.fill: parent; onClicked: rs.closed() }
    Rectangle {
        anchors.centerIn: parent
        width: Math.min(parent.width - rs.ui.px(40), rs.ui.px(620))
        height: Math.min(parent.height - rs.ui.px(60), col.implicitHeight + rs.ui.px(40))
        radius: rs.ui.px(10)
        color: rs.ui.c.panel
        border.color: rs.ui.c.line
        clip: true
        MouseArea { anchors.fill: parent }
        Column {
            id: col
            x: rs.ui.px(22); y: rs.ui.px(20)
            width: parent.width - rs.ui.px(44)
            spacing: rs.ui.px(8)
            Text { text: "Removed mods"; color: rs.ui.c.text; font.family: rs.ui.bodyFont; font.pixelSize: rs.ui.fpx(19); font.weight: Font.DemiBold }
            Text { width: parent.width; wrapMode: Text.WordWrap; text: "Kept for 7 days, then deleted by themselves."; color: rs.ui.c.soft; font.family: rs.ui.bodyFont; font.pixelSize: rs.ui.fpx(14) }
            Repeater {
                model: [{ empty: true }].concat(rs.mods)
                Rectangle {
                    id: row
                    required property int index
                    required property var modelData
                    readonly property bool foc: rs.focusIndex === index
                    width: col.width
                    height: rs.ui.px(48)
                    radius: rs.ui.px(6)
                    color: foc ? rs.ui.c.focusFill : rs.ui.c.card
                    border.color: rs.ui.c.accent
                    border.width: foc ? rs.ui.px(2) : 0
                    Column {
                        x: rs.ui.px(14)
                        anchors.verticalCenter: parent.verticalCenter
                        width: parent.width - rs.ui.px(28) - hints.width
                        Text {
                            width: parent.width
                            elide: Text.ElideRight
                            text: row.modelData.empty ? "Empty (delete all " + rs.mods.length + " for good)" : row.modelData.name
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
                    Row {
                        id: hints
                        visible: row.foc
                        anchors.right: parent.right; anchors.rightMargin: rs.ui.px(12)
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: rs.ui.px(12)
                        KeyHint { k: "a"; label: row.modelData.empty ? "Empty" : "Put back" }
                        KeyHint { visible: !row.modelData.empty; k: "x"; label: "Delete now" }
                    }
                    HoverHandler { onHoveredChanged: rs.ui.hover(parent, hovered) }
                    TapHandler { onTapped: rs.activated(row.index) }
                }
            }
        }
    }

    component KeyHint: Row {
        property string k: ""
        property string label: ""
        spacing: rs.ui.px(6)
        Rectangle {
            width: rs.ui.px(24); height: width; radius: width / 2
            anchors.verticalCenter: parent.verticalCenter
            color: rs.ui.c.raised; border.color: rs.ui.c.accentLo; border.width: Math.max(1, rs.ui.px(1))
            Text { anchors.centerIn: parent; text: parent.parent.k.toUpperCase(); color: rs.ui.c.text; font.family: rs.ui.bodyFont; font.pixelSize: rs.ui.fpx(13); font.bold: true }
        }
        Text { text: parent.label; color: rs.ui.c.soft; font.family: rs.ui.bodyFont; font.pixelSize: rs.ui.fpx(14); anchors.verticalCenter: parent.verticalCenter }
    }
}
