import QtQuick

// The game rail on the left: the games (pinned first, then the last played, portrait art), Show hidden games when
// every game is hidden, and the gear for Settings at the bottom. It folds away while scrolling down and on Get mods.
// A tap opens a game, a right-click its options.
//
// app (required): Main.qml's window. kit (required): Main.qml's kit (colors and motion, see COMPONENTS.md).
Rectangle {
    id: rail
    required property var app
    required property var kit
    readonly property var c: kit.c
    readonly property Motion mo: kit.motion
    // What a rail index shows: a game's tile, then Show hidden games, then the gear.
    function itemAt(i) { return i < app.games.length ? railList.itemAtIndex(i) : i < app.games.length + app.railExtra ? hiddenTile : gear }
    // For the build's checks.
    property alias railList: railList
    // Full screen while scrolling down (the bottom bar goes too).
    // And on Get mods (a page of the book: no game list).
    width: app.railAway ? 0 : app.px(72)
    Behavior on width { enabled: mo.moves; NumberAnimation { duration: mo.moveBase; easing.type: mo.enter } }
    visible: width > 0
    clip: true
    height: parent.height
    color: c.rail
    z: 5
    Column {
        id: railTop
        anchors.top: parent.top
        anchors.topMargin: app.px(10)
        anchors.horizontalCenter: parent.horizontalCenter
        spacing: app.px(10)
        // No app icon here (the owner's room for the games); Turbine shows itself once, at start (splash),
        // and in Settings, About.
        Rectangle { width: app.px(28); height: app.px(2); radius: 1; color: c.line; anchors.horizontalCenter: parent.horizontalCenter }
    }
    // A tile scrolled half under the divider fades out instead of showing a cut edge.
    Rectangle {
        z: 2
        anchors.top: railList.top
        width: parent.width; height: app.px(12)
        visible: !railList.atYBeginning
        gradient: Gradient {
            GradientStop { position: 0; color: c.rail }
            GradientStop { position: 1; color: Qt.rgba(c.rail.r, c.rail.g, c.rail.b, 0) }
        }
    }
    ListView {
        id: railList
        anchors.top: railTop.bottom
        anchors.topMargin: app.px(10)
        // The hidden-games tile sits after the last game, just above the gear, where its focus is.
        anchors.bottom: hiddenTile.visible ? hiddenTile.top : gear.top
        anchors.bottomMargin: app.px(10)
        width: parent.width
        clip: true
        spacing: app.px(8)
        model: app.games.length
        currentIndex: app.zone === "rail" ? Math.min(app.railIndex, app.games.length - 1) : app.current
        highlightFollowsCurrentItem: false
        onCurrentIndexChanged: app.glide(railList, () => railList.positionViewAtIndex(railList.currentIndex, ListView.Contain))
        delegate: Item {
            id: tileItem
            required property int index
            readonly property var g: app.games[index]
            readonly property bool here: index === app.current && app.page === "game"
            readonly property bool focused: app.zone === "rail" && app.railIndex === index
            width: railList.width
            height: app.px(66)
            Rectangle {
                visible: tileItem.here
                x: app.px(1); width: app.px(4); height: app.px(36); radius: app.px(2)
                anchors.verticalCenter: parent.verticalCenter
                color: c.accent
            }
            Rectangle {
                id: tile
                width: app.px(44); height: app.px(66)
                anchors.horizontalCenter: parent.horizontalCenter
                radius: app.px(6)
                color: c.raised
                // Only the focus ring outlines a tile; the game on screen has the brass bar on its left.
                border.width: tileItem.focused ? app.px(2) : 0
                border.color: c.text
                clip: true
                Image {
                    id: art
                    anchors.fill: parent
                    anchors.margins: tile.border.width
                    visible: status === Image.Ready
                    opacity: status === Image.Ready ? 1 : 0
                    Behavior on opacity { enabled: mo.on; NumberAnimation { duration: mo.base; easing.type: mo.enter } }
                    source: tileItem.g ? app.fileUrl(tileItem.g.cover || tileItem.g.header) : ""
                    fillMode: Image.PreserveAspectCrop
                    sourceSize.height: app.px(132)
                    asynchronous: true
                }
                // The game's name on its art, so the rail reads without the label beside it.
                Rectangle {
                    visible: art.visible
                    anchors.left: parent.left; anchors.right: parent.right; anchors.bottom: parent.bottom
                    anchors.margins: tile.border.width
                    height: artName.implicitHeight + app.px(4)
                    color: "#C8000000"
                    Text {
                        id: artName
                        anchors.centerIn: parent
                        width: parent.width - app.px(4)
                        horizontalAlignment: Text.AlignHCenter
                        wrapMode: Text.WordWrap
                        maximumLineCount: 2
                        elide: Text.ElideRight
                        text: tileItem.g ? app.shortGame(tileItem.g.name) : ""
                        color: "#FFFFFF"
                        font.family: app.bodyFont
                        font.pixelSize: Math.max(7, app.px(8))
                        font.bold: true
                    }
                }
                // No art: the whole name (shrunk to fit, never cut, so "Deliverance II" keeps its II) and
                // the store under it, so two copies or two games with long names still look different.
                Text {
                    visible: !art.visible
                    x: app.px(3); y: app.px(4)
                    width: parent.width - app.px(6)
                    height: parent.height - app.px(8) - tileStore.height
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                    wrapMode: Text.WordWrap
                    maximumLineCount: 6
                    elide: Text.ElideNone
                    // Long words ("Cyberpunk", "Deliverance") shrink to fit the tile instead of being cut.
                    fontSizeMode: Text.Fit
                    minimumPixelSize: Math.max(5, app.px(5))
                    text: tileItem.g ? tileItem.g.name : ""
                    color: c.soft
                    font.family: app.bodyFont
                    font.pixelSize: app.fpx(10)
                    font.bold: true
                }
                Text {
                    id: tileStore
                    visible: !art.visible
                    anchors.bottom: parent.bottom
                    anchors.bottomMargin: app.px(3)
                    width: parent.width
                    horizontalAlignment: Text.AlignHCenter
                    text: tileItem.g ? (tileItem.g.launcher === "Steam" ? "Steam" : (tileItem.g.storeLabel || tileItem.g.launcher || "")) : ""
                    color: c.muted
                    font.family: app.bodyFont
                    font.pixelSize: app.fpx(8)
                    fontSizeMode: Text.HorizontalFit
                    minimumPixelSize: Math.max(5, app.px(5))
                }
            }
            // A mod waiting or an update ready.
            Rectangle {
                visible: tileItem.here && app.updateCount > 0
                anchors.right: tile.right; anchors.top: tile.top
                anchors.margins: -app.px(3)
                width: app.px(10); height: width; radius: width / 2
                color: c.accentHi
                border.color: c.rail; border.width: app.px(2)
            }
            HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
            TapHandler { onTapped: { if (!app.gate(0, "railIndex")) return; { app.railIndex = tileItem.index; app.railSelect(tileItem.index) } } }
            // Right-click (LT with the pointer): the game's options, like X.
            TapHandler { acceptedButtons: Qt.RightButton; onTapped: { if (!app.gate(0, "sheet (right)")) return; if (app.sheet === "") { app.zone = "rail"; app.railIndex = tileItem.index; app.openRailMenu() } } }
        }
    }
    // Every game hidden from the rail: one way back, right here.
    Rectangle {
        id: hiddenTile
        visible: app.railExtra > 0
        anchors.bottom: gear.top
        anchors.bottomMargin: app.px(10)
        anchors.horizontalCenter: parent.horizontalCenter
        width: app.px(60); height: app.px(66); radius: app.px(6)
        readonly property bool focused: app.zone === "rail" && app.railIndex === app.games.length
        color: c.raised
        border.width: focused ? app.px(2) : Math.max(1, app.px(1))
        border.color: focused ? c.text : c.line
        Text {
            anchors.fill: parent
            anchors.margins: app.px(4)
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
            wrapMode: Text.WordWrap
            text: "Show hidden games"
            color: c.text
            font.family: app.bodyFont
            font.pixelSize: app.fpx(11)
            font.bold: true
        }
        HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
        TapHandler { onTapped: { if (!app.gate(0, "showHiddenGames")) return; app.showHiddenGames() } }
    }
    Rectangle {
        id: gear
        anchors.bottom: parent.bottom
        anchors.bottomMargin: app.px(10)
        anchors.horizontalCenter: parent.horizontalCenter
        width: app.px(44); height: app.px(44); radius: app.px(8)
        readonly property bool focused: app.zone === "rail" && app.railIndex === app.games.length + app.railExtra
        color: app.page === "settings" || app.page === "perf" ? c.raised : "transparent"
        border.width: focused ? app.px(2) : 0
        border.color: c.text
        Icon {
            win: app
            anchors.centerIn: parent
            tint: app.page === "settings" ? c.accent : c.muted
            d: "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.6 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.6a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"
        }
        // A newer Turbine is ready to install.
        Rectangle {
            visible: app.selfUpd.state === "available"
            anchors.right: parent.right; anchors.top: parent.top
            anchors.margins: -app.px(2)
            width: app.px(10); height: width; radius: width / 2
            color: c.accentHi
            border.color: c.rail; border.width: app.px(2)
        }
        HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
        TapHandler { onTapped: { if (!app.gate(0, "openSettings")) return; app.openSettings() } }
    }
}
