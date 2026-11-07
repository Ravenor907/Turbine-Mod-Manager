import QtQuick

// Pages to open on Nexus Mods, for what a free account can't download inside Turbine (a mod, what it needs, SKSE,
// a shared profile's Nexus mods). A opens the focused page in the browser; pressing Mod Manager Download there
// sends the file back to Turbine. B closes. Main.qml handles the keys.
Item {
    id: pg
    required property var ui
    anchors.fill: parent

    property string title: ""
    property var pages: []                // [{name, url, note}]
    property int focusIndex: 0
    property bool gameMode: false
    property bool handlerSet: true        // Turbine is the program for Nexus links

    signal activated(int index)
    signal closed()

    Rectangle { anchors.fill: parent; color: "#C0000000" }
    MouseArea { anchors.fill: parent; onClicked: pg.closed() }
    Rectangle {
        anchors.centerIn: parent
        width: Math.min(parent.width - pg.ui.px(40), pg.ui.px(680))
        height: Math.min(parent.height - pg.ui.px(80), col.implicitHeight + pg.ui.px(40))
        radius: pg.ui.px(10)
        color: pg.ui.c.panel
        border.color: pg.ui.c.line
        clip: true
        MouseArea { anchors.fill: parent }
        Column {
            id: col
            x: pg.ui.px(22); y: pg.ui.px(20)
            width: parent.width - pg.ui.px(44)
            spacing: pg.ui.px(10)
            Text {
                width: parent.width
                wrapMode: Text.WordWrap
                text: pg.title
                color: pg.ui.c.text
                font.family: pg.ui.titleFont; font.pixelSize: pg.ui.px(18); font.bold: true
            }
            Text {
                width: parent.width
                wrapMode: Text.WordWrap
                text: "Download it on the page with Slow download (or Mod Manager Download): Turbine adds the file when it lands in Downloads."
                color: pg.ui.c.text
                font.family: pg.ui.bodyFont; font.pixelSize: pg.ui.px(15)
            }
            Text {
                visible: pg.gameMode || !pg.handlerSet
                width: parent.width
                wrapMode: Text.WordWrap
                text: [pg.gameMode ? "This works best in Desktop Mode, with the browser and Turbine side by side." : "",
                       !pg.handlerSet ? "Mod Manager Download also needs Turbine set for Nexus downloads in Settings, Accounts. Slow download needs nothing." : ""].filter(s => s !== "").join(" ")
                color: pg.ui.c.warn
                font.family: pg.ui.bodyFont; font.pixelSize: pg.ui.px(14)
            }
            Repeater {
                model: pg.pages
                Rectangle {
                    id: row
                    required property int index
                    required property var modelData
                    readonly property bool foc: pg.focusIndex === index
                    width: col.width
                    height: pg.ui.px(row.modelData.note ? 62 : 50)
                    radius: pg.ui.px(6)
                    color: foc ? pg.ui.c.focusFill : pg.ui.c.card
                    border.color: pg.ui.c.accent
                    border.width: foc ? pg.ui.px(2) : 0
                    Column {
                        x: pg.ui.px(14)
                        width: parent.width - pg.ui.px(180)
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: pg.ui.px(2)
                        Text { width: parent.width; elide: Text.ElideRight; text: row.modelData.name; color: pg.ui.c.text; font.family: pg.ui.bodyFont; font.pixelSize: pg.ui.px(16); font.bold: true }
                        Text { visible: !!row.modelData.note; width: parent.width; elide: Text.ElideRight; text: row.modelData.note || ""; color: pg.ui.c.soft; font.family: pg.ui.bodyFont; font.pixelSize: pg.ui.px(13) }
                    }
                    Row {
                        anchors.right: parent.right; anchors.rightMargin: pg.ui.px(12); anchors.verticalCenter: parent.verticalCenter
                        spacing: pg.ui.px(6)
                        Rectangle {
                            visible: row.foc
                            width: pg.ui.px(24); height: width; radius: width / 2; color: pg.ui.c.raised; border.color: pg.ui.c.accentLo
                            anchors.verticalCenter: parent.verticalCenter
                            Text { anchors.centerIn: parent; text: "A"; color: pg.ui.c.text; font.family: pg.ui.bodyFont; font.pixelSize: pg.ui.px(13); font.bold: true }
                        }
                        Text { text: "Open on Nexus"; color: row.foc ? pg.ui.c.accentText : pg.ui.c.soft; font.family: pg.ui.bodyFont; font.pixelSize: pg.ui.px(15); font.bold: row.foc; anchors.verticalCenter: parent.verticalCenter }
                    }
                    HoverHandler { onHoveredChanged: pg.ui.hover(parent, hovered) }
                    TapHandler { onTapped: pg.activated(row.index) }
                }
            }
        }
    }
}
