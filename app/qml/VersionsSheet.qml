import QtQuick

// "Other versions" from a mod's details (X): older Thunderstore versions, or a Nexus mod's other files (old
// versions, optional files, updates) in their groups. A installs the focused one in place of the installed copy;
// B closes. Main.qml handles the keys.
Overlay {
    id: vs
    onOutsideTapped: vs.closed()
    required property var ui
    motion: ui.motion
    anchors.fill: parent

    property string modName: ""
    property var versions: []             // [{id, version, name, size, date, downloads, installed, group, note}]
    property int focusIndex: 0
    property string note: ""              // a line under the title (e.g. Premium is needed)

    signal activated(int index)
    signal closed()

    onFocusIndexChanged: list.positionViewAtIndex(focusIndex, ListView.Contain)

    Scrim { ui: vs.ui }
    SideSheet {
        ui: vs.ui
        anchors.bottom: parent.bottom
        width: Math.min(parent.width * 0.6, vs.ui.px(560))
        CardGuard {}
        Column {
            id: head
            x: vs.ui.px(18); y: vs.ui.px(18)
            width: parent.width - vs.ui.px(36)
            spacing: vs.ui.px(6)
            Text { text: "Other files"; color: vs.ui.c.text; font.family: vs.ui.titleFont; font.pixelSize: vs.ui.fpx(18); font.bold: true }
            Text {
                width: parent.width; wrapMode: Text.WordWrap
                text: vs.note !== "" ? vs.note : vs.modName + ": A installs the chosen one in place of the one you have, in the same spot and on or off."
                color: vs.note !== "" ? vs.ui.c.warn : vs.ui.c.soft
                font.family: vs.ui.bodyFont; font.pixelSize: vs.ui.fpx(15)
            }
        }
        ListView {
            id: list
            ScrollFade { ui: vs.ui; view: list; color: vs.ui.c.panel }
            anchors.top: head.bottom
            anchors.topMargin: vs.ui.px(10)
            anchors.bottom: parent.bottom
            anchors.bottomMargin: vs.ui.px(12)
            x: vs.ui.px(18)
            width: parent.width - vs.ui.px(36)
            clip: true
            spacing: vs.ui.px(4)
            model: vs.versions
            delegate: Item {
                id: row
                required property int index
                required property var modelData
                readonly property bool foc: vs.focusIndex === index
                readonly property bool newGroup: index === 0 || vs.versions[index - 1].group !== modelData.group
                width: list.width
                height: box.height + (newGroup ? groupHead.height : 0)
                Text {
                    id: groupHead
                    visible: row.newGroup
                    height: visible ? vs.ui.px(28) : 0
                    verticalAlignment: Text.AlignBottom
                    bottomPadding: vs.ui.px(4)
                    text: row.modelData.group || ""
                    color: vs.ui.c.muted
                    font.family: vs.ui.bodyFont; font.pixelSize: vs.ui.fpx(13); font.bold: true
                }
                Rectangle {
                    id: box
                    y: groupHead.height
                    width: parent.width
                    height: vs.ui.px(row.modelData.note ? 66 : 52)
                    radius: vs.ui.px(6)
                    color: row.foc ? vs.ui.c.focusFill : vs.ui.c.card
                    border.color: vs.ui.c.accent
                    border.width: row.foc ? vs.ui.px(2) : 0
                    Column {
                        x: vs.ui.px(14)
                        width: parent.width - vs.ui.px(120)
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: vs.ui.px(2)
                        Text { width: parent.width; elide: Text.ElideRight; text: row.modelData.name; color: vs.ui.c.text; font.family: vs.ui.bodyFont; font.pixelSize: vs.ui.fpx(16); font.bold: true }
                        Text {
                            width: parent.width; elide: Text.ElideRight
                            text: [row.modelData.version ? "Version " + row.modelData.version : "", vs.ui.niceDate(row.modelData.date), vs.ui.fileSize(row.modelData.size)].filter(s => s !== "").join(" · ")
                            color: vs.ui.c.soft; font.family: vs.ui.bodyFont; font.pixelSize: vs.ui.fpx(14)
                        }
                        Text { visible: !!row.modelData.note; width: parent.width; elide: Text.ElideRight; text: row.modelData.note || ""; color: vs.ui.c.muted; font.family: vs.ui.bodyFont; font.pixelSize: vs.ui.fpx(13) }
                    }
                    Rectangle {
                        visible: row.modelData.installed === true
                        anchors.right: parent.right; anchors.rightMargin: vs.ui.px(12); anchors.verticalCenter: parent.verticalCenter
                        width: have.implicitWidth + vs.ui.px(14); height: vs.ui.px(24); radius: vs.ui.px(5)
                        color: vs.ui.c.raised
                        Text { id: have; anchors.centerIn: parent; text: "You have this"; color: vs.ui.c.ok; font.family: vs.ui.bodyFont; font.pixelSize: vs.ui.fpx(13); font.bold: true }
                    }
                    Row {
                        visible: row.foc && row.modelData.installed !== true
                        anchors.right: parent.right; anchors.rightMargin: vs.ui.px(12); anchors.verticalCenter: parent.verticalCenter
                        spacing: vs.ui.px(6)
                        Rectangle {
                            width: vs.ui.px(24); height: width; radius: width / 2; color: vs.ui.c.raised; border.color: vs.ui.c.accentLo
                            anchors.verticalCenter: parent.verticalCenter
                            Text { anchors.centerIn: parent; text: "A"; color: vs.ui.c.text; font.family: vs.ui.bodyFont; font.pixelSize: vs.ui.fpx(13); font.bold: true }
                        }
                        Text { text: "Install"; color: vs.ui.c.soft; font.family: vs.ui.bodyFont; font.pixelSize: vs.ui.fpx(15); anchors.verticalCenter: parent.verticalCenter }
                    }
                    HoverHandler { onHoveredChanged: vs.ui.hover(parent, hovered) }
                    TapHandler { onTapped: vs.activated(row.index) }
                }
            }
        }
    }
}
