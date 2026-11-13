import QtQuick

// Get mods: the mods as picture cards, four to a row (no bar over them; Y opens the pop-up with Search, Sort
// and Category, and Add from Downloads is in the game menu): thumbnail, name, what it does in one line, "12K likes · 3.1M downloads",
// an "In your mods · On" (or Off) or "Update ready" ribbon and a "Needs the mod loader" chip where known. (Recently viewed is on the
// quick wheel.) When browsing failed (no key and Nexus refused), one big card
// opens the game's mods on Nexus instead. Main.qml moves the focus (focusArea "recent", "grid" or "fallback"
// with focusIndex) and passes taps through the signals.
Item {
    id: gm
    required property var ui

    property var result: null               // {source, items, categories, total, error, fallback}
    property bool loading: false
    property bool loadingMore: false
    // What is in effect ("Most liked · Armour · \"cloak\""), shown over the cards whenever it isn't the
    // default, and "Looking..." over the old cards while a new answer comes.
    property string summary: ""
    property var recent: []                 // the strip over the cards: [{id, name, icon, what}]
    property string recentTitle: "Good first mods"
    property bool fallback: false
    property string fallbackTitle: ""
    property string focusArea: "grid"
    property int focusIndex: 0
    property bool active: true
    property int columns: 3
    readonly property var items: result && result.items ? result.items : []
    readonly property bool showRecent: recent.length > 0 && summary === "" && !fallback

    signal recentClicked(int index)
    signal itemClicked(int index)
    signal itemMenu(int index)            // right-click on a card (its X menu)
    signal fallbackClicked(string key)
    // After a scroll by the wheel or a swipe: the first and last cards in view (the focus follows into view).
    signal scrolled(int first, int last)
    // A scroll by touch or the wheel: where the cards are now and before, and whether there is more than a screen.
    signal contentScrolled(real y, real before, bool longer)

    onFocusIndexChanged: if (focusArea === "grid") gm.ui.glide(grid, () => { grid.positionViewAtIndex(gm.focusIndex, GridView.Contain); grid.snapRow() })
    onFocusAreaChanged: if (focusArea === "grid") grid.positionViewAtIndex(focusIndex, GridView.Contain); else if (focusArea === "recent") grid.positionViewAtBeginning()
    function gridItem(i) { return grid.itemAtIndex(i) }
    readonly property bool lookingOver: loading && items.length > 0 && !fallback

    Item {
        anchors.fill: parent
        // Results (Good first mods is the grid's header, so it scrolls away with the cards)
        Rectangle {
            id: summaryBar
            objectName: "getSummary"
            visible: gm.summary !== "" && !gm.fallback
            width: parent.width
            height: visible ? summaryText.implicitHeight + gm.ui.px(12) : 0
            color: "transparent"
            Text {
                id: summaryText
                anchors.verticalCenter: parent.verticalCenter
                width: parent.width
                elide: Text.ElideRight
                text: gm.summary + "  ·  Y changes it"
                color: gm.ui.c.accentText
                font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.fpx(14); font.bold: true
            }
        }
        Item {
            id: resultsArea
            width: parent.width
            y: summaryBar.height
            height: gm.height - summaryBar.height
            // Browsing failed: open the game's mods on Nexus instead.
            Rectangle {
                id: fallbackCard
                visible: gm.fallback
                readonly property bool foc: gm.active && gm.focusArea === "fallback"
                // In the middle of the page, like the other cards that stand in for a list.
                anchors.centerIn: parent
                anchors.verticalCenterOffset: -gm.ui.px(20)
                width: Math.min(parent.width, gm.ui.px(620))
                height: fbCol.implicitHeight + gm.ui.px(44)
                radius: gm.ui.radius
                color: gm.ui.c.card
                border.color: gm.ui.c.line
                border.width: Math.max(1, gm.ui.px(1))
                Column {
                    id: fbCol
                    x: gm.ui.px(24); y: gm.ui.px(22)
                    width: parent.width - gm.ui.px(48)
                    spacing: gm.ui.px(10)
                    Text { width: parent.width; wrapMode: Text.WordWrap; text: gm.fallbackTitle; color: gm.ui.c.text; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.fpx(20); font.weight: Font.DemiBold }
                    Text { width: parent.width; wrapMode: Text.WordWrap; text: "Press Slow download on Nexus. Turbine adds the file when it lands in Downloads."; color: gm.ui.c.soft; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.fpx(15) }
                    Flow {
                        width: parent.width
                        spacing: gm.ui.px(14)
                        // A opens Nexus and X tries again; the card has the focus as a whole, so A's button has the ring.
                        ActionButton { ui: gm.ui; label: "Open on Nexus"; cap: "a"; primary: true; focused: fallbackCard.foc; onClicked: gm.fallbackClicked("a") }
                        ActionButton { ui: gm.ui; label: "Try again here"; cap: "x"; onClicked: gm.fallbackClicked("x") }
                        // A file that landed but wasn't caught can still be added from here.
                        ActionButton { ui: gm.ui; label: "Add from Downloads"; onClicked: gm.fallbackClicked("add") }
                    }
                }
                HoverHandler { onHoveredChanged: gm.ui.hover(parent, hovered) }
            }
            Text {
                visible: !gm.fallback && gm.items.length === 0
                anchors.centerIn: parent
                width: Math.min(parent.width, gm.ui.px(520))
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
                text: gm.loading ? "Looking..." : gm.result && gm.result.error ? gm.result.error : gm.result ? "Nothing matches. Try other words." : ""
                color: gm.ui.c.soft
                font.family: gm.ui.bodyFont
                font.pixelSize: gm.ui.fpx(16)
            }
            GridView {
                id: grid
                ScrollFade { ui: gm.ui; view: grid; color: gm.ui.c.ground }
                visible: !gm.fallback
                anchors.fill: parent
                clip: true
                cellWidth: width / gm.columns
                // The picture, then three lines of words (taller than the design when text keeps its size at 50%).
                cellHeight: (cellWidth - gm.ui.px(8)) * 9 / 16 + Math.max(gm.ui.px(76), gm.ui.fpx(15) * 1.3 + gm.ui.fpx(13) * 2.6 + gm.ui.px(22))
                model: gm.items
                onMovementEnded: gm.scrolled(indexAt(gm.ui.px(10), contentY + gm.ui.px(10)), indexAt(width - gm.ui.px(10), contentY + height - gm.ui.px(10)))
                // A touch or wheel scroll (not the focus moving): Main.qml hides or shows the bars.
                property real lastY: 0
                onContentYChanged: { if (moving) gm.contentScrolled(contentY - originY, lastY, contentHeight > height + gm.ui.px(40)); lastY = contentY - originY }
                // Flicks stop on a row; the bars going (the view grows) puts a whole row at the top too.
                snapMode: GridView.SnapToRow
                onHeightChanged: if (gm.focusArea === "grid") { positionViewAtIndex(gm.focusIndex, GridView.Contain); snapRow() }
                function snapRow() {
                    const hh = headerItem ? headerItem.height : 0
                    const top = contentY - originY - hh
                    if (top <= 0 || cellHeight <= 0) return
                    const r = Math.floor(top / cellHeight)
                    if (top - r * cellHeight < 1) return
                    const focusRow = Math.floor(gm.focusIndex / Math.max(1, gm.columns))
                    const next = originY + hh + (r + 1) * cellHeight
                    if (originY + hh + (focusRow + 1) * cellHeight <= next + height) contentY = Math.min(next, originY + contentHeight - height)
                    else contentY = originY + hh + r * cellHeight
                }
                header: Column {
            visible: gm.showRecent
            width: grid.width
            height: visible ? implicitHeight + gm.ui.px(8) : 0
            spacing: gm.ui.px(4)
            Text { width: parent.width; elide: Text.ElideRight; text: gm.recentTitle; color: gm.ui.c.soft; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.fpx(14); font.weight: Font.DemiBold }
            ListView {
                id: recentList
                width: parent.width
                height: gm.ui.px(52)
                orientation: ListView.Horizontal
                spacing: gm.ui.px(6)
                clip: true
                model: gm.recent
                currentIndex: gm.focusArea === "recent" ? gm.focusIndex : -1
                onCurrentIndexChanged: if (currentIndex >= 0) positionViewAtIndex(currentIndex, ListView.Contain)
                delegate: Rectangle {
                    id: rc
                    required property int index
                    required property var modelData
                    readonly property bool foc: gm.active && gm.focusArea === "recent" && gm.focusIndex === index
                    width: gm.ui.px(250); height: recentList.height; radius: gm.ui.px(6)
                    color: foc ? gm.ui.c.focusFill : gm.ui.c.card
                    Behavior on color { enabled: gm.ui.motion.on; ColorAnimation { duration: gm.ui.motion.quick; easing.type: gm.ui.motion.ease } }
                    border.color: gm.ui.c.accent
                    border.width: foc ? gm.ui.px(2) : 0
                    Thumb { id: rthumb; x: gm.ui.px(4); y: gm.ui.px(4); width: gm.ui.px(78); height: parent.height - gm.ui.px(8); src: rc.modelData.icon || ""; name: rc.modelData.name || "" }
                    Column {
                        x: rthumb.x + rthumb.width + gm.ui.px(8)
                        width: parent.width - x - gm.ui.px(6)
                        anchors.verticalCenter: parent.verticalCenter
                        Text {
                            width: parent.width; elide: Text.ElideRight
                            text: rc.modelData.name || ""
                            color: gm.ui.c.text; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.fpx(13); font.weight: Font.DemiBold
                        }
                        Text {
                            visible: text !== ""
                            width: parent.width; elide: Text.ElideRight
                            text: rc.modelData.what || ""
                            color: gm.ui.c.soft; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.fpx(12)
                        }
                    }
                    HoverHandler { onHoveredChanged: gm.ui.hover(parent, hovered) }
                    TapHandler { onTapped: gm.recentClicked(rc.index) }
                }
            }
        }
                footer: Item {
                    width: grid.width
                    height: gm.loadingMore ? gm.ui.px(36) : 0
                    Text { visible: gm.loadingMore; anchors.centerIn: parent; text: "Loading more..."; color: gm.ui.c.soft; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.fpx(15) }
                }
                delegate: Item {
                    id: cell
                    required property int index
                    required property var modelData
                    readonly property bool foc: gm.active && gm.focusArea === "grid" && gm.focusIndex === index
                    width: grid.cellWidth; height: grid.cellHeight
                    Accessible.role: Accessible.ListItem
                    Accessible.name: (modelData.name || "") + (modelData.installed ? ", installed" + (modelData.on === true ? ", on" : modelData.on === false ? ", off" : "") : "")
                                     + (modelData.summary ? ", " + modelData.summary : "") + ", " + (index + 1) + " of " + gm.items.length
                    Accessible.focused: foc
                    Rectangle {
                        anchors.fill: parent
                        anchors.margins: gm.ui.px(4)
                        radius: gm.ui.px(6)
                        color: cell.foc ? gm.ui.c.focusFill : gm.ui.c.card
                        Behavior on color { enabled: gm.ui.motion.on; ColorAnimation { duration: gm.ui.motion.quick; easing.type: gm.ui.motion.ease } }
                        border.color: gm.ui.c.accent
                        border.width: cell.foc ? gm.ui.px(2) : 0
                        clip: true
                        Thumb {
                            id: thumb
                            x: gm.ui.px(2); y: gm.ui.px(2)
                            width: parent.width - gm.ui.px(4)
                            height: width * 9 / 16
                            src: cell.modelData.icon || ""
                            name: cell.modelData.name || ""
                        }
                        // Ribbon: Installed
                        Rectangle {
                            visible: cell.modelData.installed === true
                            anchors.right: thumb.right; anchors.top: thumb.top
                            anchors.margins: gm.ui.px(6)
                            width: rib.implicitWidth + gm.ui.px(14); height: gm.ui.px(22); radius: gm.ui.px(4)
                            color: gm.ui.c.panel
                            border.color: cell.modelData.update === true ? gm.ui.c.accent : gm.ui.c.ok
                            Text { id: rib; anchors.centerIn: parent; text: cell.modelData.update === true ? "In your mods · Update ready" : "In your mods" + (cell.modelData.on === true ? " · On" : cell.modelData.on === false ? " · Off" : ""); color: cell.modelData.update === true ? gm.ui.c.accentText : gm.ui.c.ok; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.fpx(13); font.bold: true }
                        }
                        Rectangle {
                            visible: cell.modelData.code === true
                            anchors.left: thumb.left; anchors.top: thumb.top
                            anchors.margins: gm.ui.px(6)
                            width: Math.min(codeText.implicitWidth + gm.ui.px(12), thumb.width - gm.ui.px(12)); height: gm.ui.px(22); radius: gm.ui.px(4)
                            color: gm.ui.c.panel
                            Text { id: codeText; anchors.centerIn: parent; width: Math.min(implicitWidth, parent.width - gm.ui.px(8)); elide: Text.ElideRight; text: "Needs the mod loader"; color: gm.ui.c.text; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.fpx(13); font.bold: true }
                        }
                        Column {
                            x: gm.ui.px(10)
                            y: thumb.y + thumb.height + gm.ui.px(6)
                            width: parent.width - gm.ui.px(20)
                            spacing: gm.ui.px(1)
                            Text {
                                width: parent.width
                                elide: Text.ElideRight
                                text: cell.modelData.name || ""
                                color: gm.ui.c.text; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.fpx(15); font.weight: Font.DemiBold
                            }
                            // What it does, in one line.
                            Text { width: parent.width; elide: Text.ElideRight; text: cell.modelData.summary || cell.modelData.owner || ""; color: gm.ui.c.soft; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.fpx(13) }
                            Text {
                                width: parent.width; elide: Text.ElideRight
                                // Nexus endorsements and Thunderstore ratings are both likes (COMPONENTS.md, Words).
                                text: gm.ui.shortCount(cell.modelData.rating) + (cell.modelData.rating === 1 ? " like" : " likes")
                                      + " · " + gm.ui.shortCount(cell.modelData.downloads) + (cell.modelData.downloads === 1 ? " download" : " downloads")
                                color: gm.ui.c.muted; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.fpx(13)
                            }
                        }
                        HoverHandler { onHoveredChanged: gm.ui.hover(parent, hovered) }
                        // Touch: hold a moment for the card's options (like X).
                        TapHandler { longPressThreshold: 0.5; onTapped: gm.itemClicked(cell.index); onLongPressed: gm.itemMenu(cell.index) }
                        TapHandler { acceptedButtons: Qt.RightButton; onTapped: gm.itemMenu(cell.index) }
                    }
                }
            }
        }
    }

    // A new search, sort or category is on its way: the old cards dim under "Looking..." until it lands.
    Rectangle {
        objectName: "getLooking"
        visible: gm.lookingOver
        x: 0; y: summaryBar.height
        width: parent.width; height: parent.height - y
        color: Qt.rgba(gm.ui.c.ground.r, gm.ui.c.ground.g, gm.ui.c.ground.b, 0.6)
        Rectangle {
            anchors.centerIn: parent
            width: lookText.implicitWidth + gm.ui.px(32); height: gm.ui.px(44); radius: gm.ui.px(6)
            color: gm.ui.c.raised; border.color: gm.ui.c.accent; border.width: Math.max(1, gm.ui.px(1.5))
            Text { id: lookText; anchors.centerIn: parent; text: "Looking..."; color: gm.ui.c.text; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.fpx(16); font.bold: true }
        }
    }
    // A thumbnail: the picture, or a raised tile with the mod's first letter.
    component Thumb: Rectangle {
        id: th
        property string src: ""
        property string name: ""
        radius: gm.ui.px(4)
        color: gm.ui.c.raised
        clip: true
        Image {
            id: img
            anchors.fill: parent
            source: th.src
            sourceSize.width: Math.max(64, Math.round(th.width))
            fillMode: Image.PreserveAspectCrop
            asynchronous: true
            visible: status === Image.Ready
            // A picture fades in when it has loaded.
            opacity: status === Image.Ready ? 1 : 0
            Behavior on opacity { enabled: gm.ui.motion.on; NumberAnimation { duration: gm.ui.motion.standard; easing.type: gm.ui.motion.ease } }
        }
        Text {
            visible: !img.visible
            anchors.centerIn: parent
            text: (th.name || "?").substring(0, 1).toUpperCase()
            color: gm.ui.c.muted
            font.family: gm.ui.bodyFont; font.pixelSize: Math.max(gm.ui.px(16), th.height * 0.4); font.bold: true
        }
    }
}
