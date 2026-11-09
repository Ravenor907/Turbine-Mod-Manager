import QtQuick
import QtQuick.Shapes

// Get mods: the bar (search with Steam's keyboard, sort, category, where from, Add from Downloads), then a
// the mods as picture cards, four to a row: thumbnail, name, what it does in one line, "12K likes · 3.1M downloads",
// an "Installed · On" (or Off) or "Update ready" ribbon and a "Code mod" chip where known. (Recently viewed is on the
// quick wheel.) When browsing failed (no key and Nexus refused), one big card
// opens the game's mods on Nexus instead. Main.qml moves the focus (focusArea "bar", "recent", "grid" or "fallback"
// with focusIndex) and passes taps through the signals.
Item {
    id: gm
    required property var ui

    property var result: null               // {source, items, categories, total, error, fallback}
    property bool loading: false
    property bool loadingMore: false
    property string search: ""
    property string category: ""
    property var bar: []                    // [{key, label}]
    property var recent: []                 // the strip over the cards: [{id, name, icon, what}]
    property string recentTitle: "Good first mods"
    property string banner: ""              // "No mods for The Witcher 3 yet. Pick one below, or search."
    property bool fallback: false
    property string fallbackTitle: ""
    property string focusArea: "grid"
    property int focusIndex: 0
    property bool active: true
    property int columns: 3
    readonly property var items: result && result.items ? result.items : []
    readonly property var cats: result && result.categories ? [""].concat(result.categories) : []
    readonly property bool showRecent: recent.length > 0 && search === "" && !fallback

    signal barClicked(int index)
    signal recentClicked(int index)
    signal itemClicked(int index)
    signal itemMenu(int index)            // right-click on a card (its X menu)
    signal fallbackClicked(string key)
    // After a scroll by the wheel or a swipe: the first and last cards in view (the focus follows into view).
    signal scrolled(int first, int last)

    function count(n) {
        n = n || 0
        if (n >= 1000000) return (n / 1000000).toFixed(1).replace(".0", "") + "M"
        if (n >= 1000) return Math.round(n / 1000) + "K"
        return String(n)
    }
    onFocusIndexChanged: if (focusArea === "grid") grid.positionViewAtIndex(focusIndex, GridView.Contain)
    onFocusAreaChanged: if (focusArea === "grid") grid.positionViewAtIndex(focusIndex, GridView.Contain); else if (focusArea === "bar") grid.positionViewAtBeginning()
    function gridItem(i) { return grid.itemAtIndex(i) }

    Column {
        anchors.fill: parent
        spacing: gm.ui.px(8)
        Row {
            id: barRow
            visible: !gm.fallback
            width: parent.width
            height: visible ? gm.ui.px(42) : 0
            spacing: gm.ui.px(8)
            Repeater {
                model: gm.bar
                Rectangle {
                    id: b
                    required property int index
                    required property var modelData
                    readonly property bool foc: gm.active && gm.focusArea === "bar" && gm.focusIndex === index
                    readonly property bool isSearch: modelData.key === "search"
                    readonly property real otherWidth: {
                        let w = 0
                        for (let i = 0; i < barRow.children.length; i++) {
                            const c = barRow.children[i]
                            if (c !== b && c.isSearch === false) w += c.width
                        }
                        return w
                    }
                    width: isSearch ? Math.max(gm.ui.px(140), barRow.width - gm.ui.px(8) * (gm.bar.length - 1) - otherWidth) : label.implicitWidth + gm.ui.px(26)
                    height: parent.height
                    radius: gm.ui.px(6)
                    color: foc ? gm.ui.c.focusFill : isSearch ? gm.ui.c.ground : gm.ui.c.card
                    border.color: foc ? gm.ui.c.accent : gm.ui.c.line
                    border.width: foc ? gm.ui.px(2) : Math.max(1, gm.ui.px(1))
                    Shape {
                        visible: b.isSearch
                        x: gm.ui.px(12); anchors.verticalCenter: parent.verticalCenter
                        width: gm.ui.px(18); height: width
                        preferredRendererType: Shape.CurveRenderer
                        ShapePath {
                            strokeColor: gm.ui.c.muted; strokeWidth: 2; fillColor: "transparent"; capStyle: ShapePath.RoundCap
                            scale: Qt.size(gm.ui.px(18) / 24, gm.ui.px(18) / 24)
                            PathSvg { path: "M10.5 3a7.5 7.5 0 1 1 0 15 7.5 7.5 0 0 1 0-15z M16 16l5 5" }
                        }
                    }
                    Text {
                        id: label
                        x: b.isSearch ? gm.ui.px(38) : gm.ui.px(13)
                        width: b.isSearch ? parent.width - gm.ui.px(50) : implicitWidth
                        anchors.verticalCenter: parent.verticalCenter
                        elide: Text.ElideRight
                        text: b.modelData.label
                        color: b.isSearch && gm.search === "" ? gm.ui.c.muted : gm.ui.c.text
                        font.family: gm.ui.bodyFont
                        font.pixelSize: gm.ui.px(15)
                        font.weight: b.isSearch ? Font.Normal : Font.DemiBold
                    }
                    HoverHandler { onHoveredChanged: gm.ui.hover(parent, hovered) }
                    TapHandler { onTapped: gm.barClicked(b.index) }
                }
            }
        }
        // A one-time line for a game with no mods yet.
        Rectangle {
            visible: gm.banner !== "" && !gm.fallback
            width: parent.width
            height: visible ? gm.ui.px(40) : 0
            radius: gm.ui.px(6)
            color: gm.ui.c.card
            border.color: gm.ui.c.accentLo
            Row {
                x: gm.ui.px(12); anchors.verticalCenter: parent.verticalCenter
                spacing: gm.ui.px(10)
                InfoMark { anchors.verticalCenter: parent.verticalCenter }
                Text { text: gm.banner; color: gm.ui.c.text; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.px(15); anchors.verticalCenter: parent.verticalCenter }
            }
        }
        // Recently viewed
        Column {
            visible: gm.showRecent
            width: parent.width
            height: visible ? implicitHeight : 0
            spacing: gm.ui.px(4)
            Text { text: gm.recentTitle; color: gm.ui.c.soft; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.px(14); font.weight: Font.DemiBold }
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
                            color: gm.ui.c.text; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.px(13); font.weight: Font.DemiBold
                        }
                        Text {
                            visible: text !== ""
                            width: parent.width; elide: Text.ElideRight
                            text: rc.modelData.what || ""
                            color: gm.ui.c.soft; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.px(12)
                        }
                    }
                    HoverHandler { onHoveredChanged: gm.ui.hover(parent, hovered) }
                    TapHandler { onTapped: gm.recentClicked(rc.index) }
                }
            }
        }
        // Results
        Item {
            id: resultsArea
            width: parent.width
            height: gm.height - y
            // Browsing failed: open the game's mods on Nexus instead.
            Rectangle {
                id: fallbackCard
                visible: gm.fallback
                readonly property bool foc: gm.active && gm.focusArea === "fallback"
                anchors.horizontalCenter: parent.horizontalCenter
                y: gm.ui.px(10)
                width: Math.min(parent.width, gm.ui.px(620))
                height: fbCol.implicitHeight + gm.ui.px(36)
                radius: gm.ui.px(10)
                color: foc ? gm.ui.c.focusFill : gm.ui.c.card
                border.color: gm.ui.c.accent
                border.width: foc ? gm.ui.px(2) : Math.max(1, gm.ui.px(1))
                Column {
                    id: fbCol
                    x: gm.ui.px(20); y: gm.ui.px(18)
                    width: parent.width - gm.ui.px(40)
                    spacing: gm.ui.px(10)
                    Text { width: parent.width; wrapMode: Text.WordWrap; text: gm.fallbackTitle; color: gm.ui.c.text; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.px(20); font.weight: Font.DemiBold }
                    Text { width: parent.width; wrapMode: Text.WordWrap; text: "Download with Slow download. Turbine adds it when it lands in Downloads."; color: gm.ui.c.soft; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.px(15) }
                    Flow {
                        width: parent.width
                        spacing: gm.ui.px(14)
                        ActionKey { k: "a"; label: "Open on Nexus"; strong: true; onClicked: gm.fallbackClicked("a") }
                        ActionKey { k: "x"; label: "Try again here"; onClicked: gm.fallbackClicked("x") }
                        // A file that landed but wasn't caught can still be added from here.
                        ActionKey { k: "y"; label: "Add from Downloads"; onClicked: gm.fallbackClicked("y") }
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
                font.pixelSize: gm.ui.px(16)
            }
            GridView {
                id: grid
                visible: !gm.fallback
                anchors.fill: parent
                clip: true
                cellWidth: width / gm.columns
                cellHeight: (cellWidth - gm.ui.px(8)) * 9 / 16 + gm.ui.px(76)
                model: gm.items
                onMovementEnded: gm.scrolled(indexAt(gm.ui.px(10), contentY + gm.ui.px(10)), indexAt(width - gm.ui.px(10), contentY + height - gm.ui.px(10)))
                footer: Item {
                    width: grid.width
                    height: gm.loadingMore ? gm.ui.px(36) : 0
                    Text { visible: gm.loadingMore; anchors.centerIn: parent; text: "Loading more..."; color: gm.ui.c.soft; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.px(15) }
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
                            Text { id: rib; anchors.centerIn: parent; text: cell.modelData.update === true ? "Update ready" : "Installed" + (cell.modelData.on === true ? " · On" : cell.modelData.on === false ? " · Off" : ""); color: cell.modelData.update === true ? gm.ui.c.accentText : gm.ui.c.ok; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.px(13); font.bold: true }
                        }
                        Rectangle {
                            visible: cell.modelData.code === true
                            anchors.left: thumb.left; anchors.top: thumb.top
                            anchors.margins: gm.ui.px(6)
                            width: codeText.implicitWidth + gm.ui.px(12); height: gm.ui.px(22); radius: gm.ui.px(4)
                            color: gm.ui.c.panel
                            Text { id: codeText; anchors.centerIn: parent; text: "Code mod"; color: gm.ui.c.text; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.px(13); font.bold: true }
                        }
                        Column {
                            x: gm.ui.px(10)
                            y: thumb.y + thumb.height + gm.ui.px(6)
                            width: parent.width - gm.ui.px(20)
                            spacing: gm.ui.px(1)
                            Text {
                                width: parent.width
                                elide: Text.ElideRight
                                text: cell.modelData.name
                                color: gm.ui.c.text; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.px(15); font.weight: Font.DemiBold
                            }
                            // What it does, in one line.
                            Text { width: parent.width; elide: Text.ElideRight; text: cell.modelData.summary || cell.modelData.owner || ""; color: gm.ui.c.soft; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.px(13) }
                            Text {
                                width: parent.width; elide: Text.ElideRight
                                text: (cell.modelData.source === "thunderstore" || gm.result && gm.result.source === "thunderstore"
                                       ? gm.count(cell.modelData.rating) + " ratings" : gm.count(cell.modelData.rating) + " likes")
                                      + " · " + gm.count(cell.modelData.downloads) + " downloads"
                                color: gm.ui.c.muted; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.px(13)
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
        }
        Text {
            visible: !img.visible
            anchors.centerIn: parent
            text: (th.name || "?").substring(0, 1).toUpperCase()
            color: gm.ui.c.muted
            font.family: gm.ui.bodyFont; font.pixelSize: Math.max(gm.ui.px(16), th.height * 0.4); font.bold: true
        }
    }
    component InfoMark: Rectangle {
        width: gm.ui.px(20); height: width; radius: width / 2
        color: "transparent"; border.color: gm.ui.c.accentHi; border.width: Math.max(1, gm.ui.px(1.5))
        Text { anchors.centerIn: parent; text: "i"; color: gm.ui.c.accentHi; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.px(13); font.bold: true }
    }
    component ActionKey: Rectangle {
        id: ak
        property string k: ""
        property string label: ""
        property bool strong: false
        signal clicked()
        width: akRow.implicitWidth + gm.ui.px(24); height: gm.ui.px(40); radius: gm.ui.px(8)
        gradient: Gradient {
            GradientStop { position: 0; color: ak.strong ? gm.ui.c.accentHi : gm.ui.c.raised }
            GradientStop { position: 1; color: ak.strong ? gm.ui.c.accent : gm.ui.c.raised }
        }
        Row {
            id: akRow
            anchors.centerIn: parent
            spacing: gm.ui.px(8)
            // No key caps on cards: the legend says what A and X do.
            Rectangle {
                visible: false
                width: gm.ui.px(24); height: width; radius: width / 2
                anchors.verticalCenter: parent.verticalCenter
                color: ak.strong ? Qt.rgba(0, 0, 0, 0.18) : gm.ui.c.panel
                Text { anchors.centerIn: parent; text: ak.k.toUpperCase(); color: ak.strong ? gm.ui.c.onAccent : gm.ui.c.text; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.px(13); font.bold: true }
            }
            Text { text: ak.label; color: ak.strong ? gm.ui.c.onAccent : gm.ui.c.text; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.px(16); font.bold: true; anchors.verticalCenter: parent.verticalCenter }
        }
        HoverHandler { onHoveredChanged: gm.ui.hover(parent, hovered) }
        TapHandler { onTapped: ak.clicked() }
    }
}
