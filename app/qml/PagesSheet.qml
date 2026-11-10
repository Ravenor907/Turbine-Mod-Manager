import QtQuick

// Pages to open on Nexus Mods, for what a free account can't download inside Turbine (a mod, what it needs, SKSE,
// a shared profile's Nexus mods). A opens the focused page in the browser; pressing Mod Manager Download there
// sends the file back to Turbine. B closes. Main.qml handles the keys.
Overlay {
    id: pg
    onOutsideTapped: pg.closed()
    required property var ui
    motion: ui.motion
    anchors.fill: parent

    property string title: ""
    property var pages: []                // [{name, url, note}]
    property int focusIndex: 0
    // The ring's row is kept in view from here, not from the rows: a row can be made again (the list changed) before
    // a delayed call runs, and its call would then find nothing.
    onFocusIndexChanged: Qt.callLater(pg.followFocus)
    onVisibleChanged: if (visible) Qt.callLater(pg.followFocus)
    function followFocus() {
        if (typeof rowRepeater === "undefined" || !rowRepeater || typeof scroller === "undefined" || !scroller) return
        const it = rowRepeater.itemAt(focusIndex)
        if (it) scroller.follow(it)
    }
    property bool gameMode: false
    property bool handlerSet: true        // Turbine is the program for Nexus links

    signal activated(int index)
    signal closed()

    Rectangle { anchors.fill: parent; color: "#C0000000" }
    Rectangle {
        anchors.centerIn: parent
        width: Math.min(parent.width - pg.ui.px(40), pg.ui.px(680))
        height: Math.min(parent.height - pg.ui.px(80), col.implicitHeight + pg.ui.px(40))
        radius: pg.ui.px(10)
        color: pg.ui.c.panel
        border.color: pg.ui.c.line
        clip: true
        CardGuard {}
        // 0.59: scrolls by touch and follows the focus when the rows don't fit (150%, a long list).
        Flickable {
            id: scroller
            anchors.fill: parent
            anchors.margins: pg.ui.px(20)
            contentHeight: col.implicitHeight
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            function follow(it) {
                if (!it) return
                const y = it.mapToItem(col, 0, 0).y
                if (y < contentY) contentY = Math.max(0, y)
                else if (y + it.height > contentY + height) contentY = Math.min(Math.max(0, contentHeight - height), y + it.height - height)
            }
        Column {
            id: col
            width: scroller.width
            spacing: pg.ui.px(10)
            Text {
                width: parent.width
                wrapMode: Text.WordWrap
                text: pg.title
                color: pg.ui.c.text
                font.family: pg.ui.titleFont; font.pixelSize: pg.ui.fpx(18); font.bold: true
            }
            Text {
                width: parent.width
                wrapMode: Text.WordWrap
                text: "Press Slow download on each page. Turbine adds each file by itself."
                color: pg.ui.c.text
                font.family: pg.ui.bodyFont; font.pixelSize: pg.ui.fpx(15)
            }
            Text {
                visible: false   // 0.59 (T15): the page opens in Turbine, so no setting is needed
                width: parent.width
                wrapMode: Text.WordWrap
                text: "Mod Manager Download also needs Turbine set for Nexus downloads in Settings, Getting mods. Slow download needs nothing."
                color: pg.ui.c.soft
                font.family: pg.ui.bodyFont; font.pixelSize: pg.ui.fpx(14)
            }
            Repeater {
                id: rowRepeater
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
                        Text { width: parent.width; elide: Text.ElideRight; text: row.modelData.name; color: pg.ui.c.text; font.family: pg.ui.bodyFont; font.pixelSize: pg.ui.fpx(16); font.bold: true }
                        Text { visible: !!row.modelData.note; width: parent.width; elide: Text.ElideRight; text: row.modelData.note || ""; color: pg.ui.c.soft; font.family: pg.ui.bodyFont; font.pixelSize: pg.ui.fpx(13) }
                    }
                    Row {
                        anchors.right: parent.right; anchors.rightMargin: pg.ui.px(12); anchors.verticalCenter: parent.verticalCenter
                        spacing: pg.ui.px(6)
                        Rectangle {
                            visible: row.foc
                            width: pg.ui.px(24); height: width; radius: width / 2; color: pg.ui.c.raised; border.color: pg.ui.c.accentLo
                            anchors.verticalCenter: parent.verticalCenter
                            Text { anchors.centerIn: parent; text: "A"; color: pg.ui.c.text; font.family: pg.ui.bodyFont; font.pixelSize: pg.ui.fpx(13); font.bold: true }
                        }
                        Text { text: "Open on Nexus"; color: row.foc ? pg.ui.c.accentText : pg.ui.c.soft; font.family: pg.ui.bodyFont; font.pixelSize: pg.ui.fpx(15); font.bold: row.foc; anchors.verticalCenter: parent.verticalCenter }
                    }
                    HoverHandler { onHoveredChanged: pg.ui.hover(parent, hovered) }
                    TapHandler { onTapped: pg.activated(row.index) }
                }
            }
        }
        }
    }
}
