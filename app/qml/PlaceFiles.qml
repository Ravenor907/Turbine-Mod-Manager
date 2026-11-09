import QtQuick

// "Where do these files go?": a full page for a mod of a game Turbine has no rules for (Basic support). Two panes:
// the game's folders (the folder on screen is where the files go; its first row is "Put them here") and the mod's
// own folders (the folder on screen is what goes in). Side by side on wide screens, one at a time on narrow ones
// (LB/RB switch). The strip at the bottom says how many files go where. Main.qml drives the keys; taps work too.
Item {
    id: pf
    required property var ui

    property string topLine: ""
    property string gamePath: ""          // "Hogwarts Legacy > Phoenix > Content"
    property string modPath: ""           // "Putting in: everything inside Textures"
    property var gameRows: []             // [{kind: "here" | "suggest" | "folder" | "new", label, detail}]
    property var modRows: []              // [{kind: "folder", label, detail}]
    property int pane: 0                  // 0 the game, 1 the mod's files
    property bool twoPanes: true
    property int gameIndex: 0
    property int modIndex: 0
    property string preview: ""
    property bool active: true
    property bool moveOnly: false         // Change where it goes: only the game's folders

    signal rowClicked(int pane, int index)
    signal rowHeld(int pane, int index)
    signal swiped(int dir)

    Accessible.role: Accessible.Pane
    Accessible.name: "Where do these files go?"
    Accessible.description: topLine

    Column {
        id: head
        width: parent.width
        spacing: pf.ui.px(6)
        Text {
            width: parent.width
            wrapMode: Text.WordWrap
            text: pf.topLine
            color: pf.ui.c.text
            font.family: pf.ui.bodyFont; font.pixelSize: pf.ui.fpx(16)
        }
        // One pane at a time: which one is on screen, LB and RB switch.
        Row {
            visible: !pf.twoPanes && !pf.moveOnly
            spacing: pf.ui.px(8)
            Repeater {
                model: ["Game folder", "Mod files"]
                Rectangle {
                    required property int index
                    required property string modelData
                    width: tabText.implicitWidth + pf.ui.px(24); height: pf.ui.px(30); radius: pf.ui.px(5)
                    color: pf.pane === index ? pf.ui.c.raised : "transparent"
                    border.color: pf.pane === index ? pf.ui.c.accent : pf.ui.c.line
                    Text { id: tabText; anchors.centerIn: parent; text: parent.modelData; color: pf.pane === parent.index ? pf.ui.c.text : pf.ui.c.muted; font.family: pf.ui.bodyFont; font.pixelSize: pf.ui.fpx(14); font.bold: true }
                    Accessible.role: Accessible.PageTab
                    Accessible.name: modelData
                    TapHandler { onTapped: pf.swiped(parent.index === 0 ? -1 : 1) }
                }
            }
        }
    }

    Row {
        id: panes
        anchors.top: head.bottom
        anchors.topMargin: pf.ui.px(10)
        anchors.bottom: strip.top
        anchors.bottomMargin: pf.ui.px(10)
        width: parent.width
        spacing: pf.ui.px(12)
        Repeater {
            model: pf.moveOnly ? 1 : 2
            Rectangle {
                id: paneBox
                required property int index
                readonly property bool shown: pf.twoPanes || pf.moveOnly || pf.pane === index
                readonly property var rows: index === 0 ? pf.gameRows : pf.modRows
                readonly property int foc: index === 0 ? pf.gameIndex : pf.modIndex
                visible: shown
                width: pf.twoPanes && !pf.moveOnly ? (panes.width - panes.spacing) / 2 : panes.width
                height: panes.height
                radius: pf.ui.px(8)
                color: pf.ui.c.card
                border.color: pf.pane === index && pf.active ? pf.ui.c.accentLo : pf.ui.c.line
                Accessible.role: Accessible.List
                Accessible.name: index === 0 ? "Game folder: " + pf.gamePath : "Mod files: " + pf.modPath
                Column {
                    id: paneHead
                    x: pf.ui.px(12); y: pf.ui.px(10)
                    width: parent.width - pf.ui.px(24)
                    spacing: pf.ui.px(2)
                    Text { text: paneBox.index === 0 ? "GAME FOLDER" : "MOD FILES"; color: pf.ui.c.muted; font.family: pf.ui.bodyFont; font.pixelSize: pf.ui.fpx(12); font.bold: true }
                    Text {
                        width: parent.width
                        elide: Text.ElideLeft
                        text: paneBox.index === 0 ? pf.gamePath : pf.modPath
                        color: pf.ui.c.accentText
                        font.family: pf.ui.bodyFont; font.pixelSize: pf.ui.fpx(14); font.bold: true
                    }
                }
                ListView {
                    id: list
                    x: pf.ui.px(8)
                    anchors.top: paneHead.bottom
                    anchors.topMargin: pf.ui.px(8)
                    anchors.bottom: parent.bottom
                    anchors.bottomMargin: pf.ui.px(8)
                    width: parent.width - pf.ui.px(16)
                    clip: true
                    spacing: pf.ui.px(4)
                    boundsBehavior: Flickable.StopAtBounds
                    model: paneBox.rows
                    currentIndex: paneBox.foc
                    onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                    delegate: Rectangle {
                        id: row
                        required property int index
                        required property var modelData
                        readonly property bool foc: pf.active && pf.pane === paneBox.index && paneBox.foc === index
                        width: list.width
                        height: pf.ui.px(46)
                        radius: pf.ui.px(6)
                        color: foc ? pf.ui.c.focusFill : modelData.kind === "here" ? pf.ui.c.raised : "transparent"
                        border.color: pf.ui.c.accent
                        border.width: foc ? pf.ui.px(2) : 0
                        Accessible.role: Accessible.ListItem
                        Accessible.name: modelData.label + (modelData.detail ? ", " + modelData.detail : "")
                        Accessible.description: (index + 1) + " of " + paneBox.rows.length
                        Column {
                            x: pf.ui.px(12)
                            width: parent.width - pf.ui.px(24) - (badge.visible ? badge.width + pf.ui.px(8) : 0)
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: pf.ui.px(1)
                            Text {
                                width: parent.width; elide: Text.ElideMiddle
                                text: (row.modelData.kind === "folder" || row.modelData.kind === "new" ? "▸ " : "") + row.modelData.label
                                color: row.modelData.kind === "here" ? pf.ui.c.accentText : pf.ui.c.text
                                font.family: pf.ui.bodyFont; font.pixelSize: pf.ui.fpx(15); font.bold: row.modelData.kind === "here" || row.foc
                            }
                            Text {
                                visible: !!row.modelData.detail
                                width: parent.width; elide: Text.ElideRight
                                text: row.modelData.detail || ""
                                color: pf.ui.c.soft
                                font.family: pf.ui.bodyFont; font.pixelSize: pf.ui.fpx(12)
                            }
                        }
                        Rectangle {
                            id: badge
                            visible: !!row.modelData.badge
                            anchors.right: parent.right; anchors.rightMargin: pf.ui.px(10); anchors.verticalCenter: parent.verticalCenter
                            width: badgeText.implicitWidth + pf.ui.px(14); height: pf.ui.px(20); radius: height / 2
                            color: "transparent"
                            border.color: pf.ui.c.accentLo
                            Text { id: badgeText; anchors.centerIn: parent; text: row.modelData.badge || ""; color: pf.ui.c.accentText; font.family: pf.ui.bodyFont; font.pixelSize: pf.ui.fpx(11); font.bold: true }
                        }
                        HoverHandler { onHoveredChanged: pf.ui.hover(parent, hovered) }
                        TapHandler {
                            onTapped: pf.rowClicked(paneBox.index, row.index)
                            onLongPressed: pf.rowHeld(paneBox.index, row.index)
                        }
                    }
                }
            }
        }
    }
    // A sideways swipe switches pane (touch).
    DragHandler {
        target: null
        xAxis.enabled: true; yAxis.enabled: false
        acceptedDevices: PointerDevice.TouchScreen
        onActiveChanged: if (!active && Math.abs(activeTranslation.x) > pf.ui.px(60)) pf.swiped(activeTranslation.x < 0 ? 1 : -1)
    }

    Rectangle {
        id: strip
        anchors.bottom: parent.bottom
        width: parent.width
        height: stripText.implicitHeight + pf.ui.px(16)
        radius: pf.ui.px(6)
        color: pf.ui.c.raised
        Accessible.role: Accessible.StaticText
        Accessible.name: pf.preview
        Text {
            id: stripText
            x: pf.ui.px(12)
            anchors.verticalCenter: parent.verticalCenter
            width: parent.width - pf.ui.px(24)
            wrapMode: Text.WordWrap
            text: pf.preview
            color: pf.ui.c.text
            font.family: pf.ui.bodyFont; font.pixelSize: pf.ui.fpx(14)
        }
    }
}
