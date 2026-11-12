import QtQuick

// "Which game is it for?": the installed games, for a download Turbine found in Downloads. A (or a click) adds the
// file to the focused game, B goes back to the question. Main.qml handles the keys.
Overlay {
    id: gp
    onOutsideTapped: gp.closed()
    required property var ui
    motion: ui.motion
    anchors.fill: parent

    property string fileName: ""
    property var games: []                // Main.qml's games: [{id, name, launcher, storeLabel, ...}]
    property int focusIndex: 0

    signal activated(int index)
    signal closed()

    onFocusIndexChanged: list.positionViewAtIndex(focusIndex, ListView.Contain)

    Scrim { ui: gp.ui }
    Card {
        ui: gp.ui
        anchors.centerIn: parent
        width: Math.min(parent.width - gp.ui.px(40), gp.ui.px(620))
        height: Math.min(parent.height - gp.ui.px(80), head.implicitHeight + gp.games.length * gp.ui.px(56) + gp.ui.px(56))
        clip: true
        CardGuard {}
        Column {
            id: head
            x: gp.ui.px(22); y: gp.ui.px(20)
            width: parent.width - gp.ui.px(44)
            spacing: gp.ui.px(6)
            Text {
                width: parent.width
                text: "Which game is it for?"
                color: gp.ui.c.text
                font.family: gp.ui.titleFont; font.pixelSize: gp.ui.fpx(18); font.bold: true
            }
            Text {
                width: parent.width
                wrapMode: Text.WordWrap
                text: gp.fileName + " goes into the game you pick."
                color: gp.ui.c.soft
                font.family: gp.ui.bodyFont; font.pixelSize: gp.ui.fpx(15)
            }
        }
        ListView {
            id: list
            ScrollFade { ui: gp.ui; view: list; color: gp.ui.c.panel }
            x: gp.ui.px(22)
            anchors.top: head.bottom
            anchors.topMargin: gp.ui.px(12)
            anchors.bottom: parent.bottom
            anchors.bottomMargin: gp.ui.px(18)
            width: parent.width - gp.ui.px(44)
            clip: true
            spacing: gp.ui.px(6)
            boundsBehavior: Flickable.StopAtBounds
            model: gp.games
            currentIndex: gp.focusIndex
            delegate: Rectangle {
                id: row
                required property int index
                required property var modelData
                readonly property bool foc: gp.focusIndex === index
                width: list.width
                height: gp.ui.px(50)
                radius: gp.ui.px(6)
                color: foc ? gp.ui.c.focusFill : gp.ui.c.card
                border.color: gp.ui.c.accent
                border.width: foc ? gp.ui.px(2) : 0
                Column {
                    x: gp.ui.px(14)
                    width: parent.width - gp.ui.px(140)
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: gp.ui.px(1)
                    Text { width: parent.width; elide: Text.ElideRight; text: row.modelData.name; color: gp.ui.c.text; font.family: gp.ui.bodyFont; font.pixelSize: gp.ui.fpx(16); font.bold: true }
                    Text {
                        width: parent.width; elide: Text.ElideRight
                        text: (row.modelData.launcher === "Steam" ? "Steam" : "Heroic" + (row.modelData.storeLabel ? " (" + row.modelData.storeLabel + ")" : ""))
                              + (row.modelData.support === "basic" ? " · Basic support" : row.modelData.support === "custom" ? " · Added by you" : "")
                        color: gp.ui.c.soft; font.family: gp.ui.bodyFont; font.pixelSize: gp.ui.fpx(13)
                    }
                }
                Row {
                    visible: row.foc
                    anchors.right: parent.right; anchors.rightMargin: gp.ui.px(12); anchors.verticalCenter: parent.verticalCenter
                    spacing: gp.ui.px(6)
                    Rectangle {
                        width: gp.ui.px(24); height: width; radius: width / 2; color: gp.ui.c.raised; border.color: gp.ui.c.accentLo
                        anchors.verticalCenter: parent.verticalCenter
                        Text { anchors.centerIn: parent; text: "A"; color: gp.ui.c.text; font.family: gp.ui.bodyFont; font.pixelSize: gp.ui.fpx(13); font.bold: true }
                    }
                    Text { text: "Add here"; color: gp.ui.c.accentText; font.family: gp.ui.bodyFont; font.pixelSize: gp.ui.fpx(15); font.bold: true; anchors.verticalCenter: parent.verticalCenter }
                }
                HoverHandler { onHoveredChanged: gp.ui.hover(parent, hovered) }
                TapHandler { onTapped: gp.activated(row.index) }
            }
        }
    }
}
