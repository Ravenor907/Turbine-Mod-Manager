import QtQuick
import QtQuick.Shapes

// Get Mods: search (A opens Steam's keyboard), where the mods come from, the sort, Add from Downloads, the categories,
// then the mods as cards in three columns: name, one line about it, how many downloads, and "Code mod" for mods that
// run their own code (they need the mod loader). Main.qml moves the focus (focusArea "bar", "cats" or "grid" with
// focusIndex) and passes taps through the signals.
Item {
    id: gm
    required property var ui

    property var result: null               // the browse answer from the engine: {source, items, categories, total, error}
    property bool loading: false
    property string search: ""
    property string sourceLabel: ""       // "Thunderstore", "Nexus Mods"
    property bool canSwitchSource: false
    property string sortLabel: ""
    property string category: ""
    property var bar: []                  // the top row's buttons: [{key, label}]
    property string focusArea: "grid"
    property int focusIndex: 0
    property bool active: true            // focus is here (not on the rail)
    readonly property int columns: 3
    readonly property var items: result && result.items ? result.items : []
    readonly property var cats: result && result.categories ? [""].concat(result.categories) : []

    signal barClicked(int index)
    signal categoryClicked(int index)
    signal itemClicked(int index)

    function count(n) {
        if (n >= 1000000) return (n / 1000000).toFixed(1).replace(".0", "") + "M"
        if (n >= 1000) return Math.round(n / 1000) + "K"
        return String(n)
    }
    onFocusIndexChanged: if (focusArea === "grid") grid.positionViewAtIndex(focusIndex, GridView.Contain)
    onFocusAreaChanged: if (focusArea === "cats") catList.positionViewAtIndex(focusIndex, ListView.Contain)

    Column {
        anchors.fill: parent
        spacing: gm.ui.px(8)
        // Search, source, sort, Add from Downloads
        Row {
            id: barRow
            width: parent.width
            height: gm.ui.px(44)
            spacing: gm.ui.px(8)
            Repeater {
                model: gm.bar
                Rectangle {
                    id: b
                    required property int index
                    required property var modelData
                    readonly property bool foc: gm.active && gm.focusArea === "bar" && gm.focusIndex === index
                    readonly property bool isSearch: modelData.key === "search"
                    width: isSearch ? barRow.width - gm.ui.px(8) * (gm.bar.length - 1) - otherWidth : label.implicitWidth + gm.ui.px(30)
                    readonly property real otherWidth: {
                        let w = 0
                        for (let i = 0; i < barRow.children.length; i++) {
                            const c = barRow.children[i]
                            if (c !== b && c.isSearch === false) w += c.width
                        }
                        return w
                    }
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
                        x: b.isSearch ? gm.ui.px(40) : gm.ui.px(15)
                        width: b.isSearch ? parent.width - gm.ui.px(110) : implicitWidth
                        anchors.verticalCenter: parent.verticalCenter
                        elide: Text.ElideRight
                        text: b.modelData.label
                        color: b.isSearch && gm.search === "" ? gm.ui.c.muted : gm.ui.c.text
                        font.family: gm.ui.bodyFont
                        font.pixelSize: gm.ui.px(15)
                        font.bold: !b.isSearch
                    }
                    Row {
                        visible: b.isSearch && b.foc
                        anchors.right: parent.right; anchors.rightMargin: gm.ui.px(10)
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: gm.ui.px(6)
                        Rectangle {
                            width: gm.ui.px(24); height: width; radius: width / 2
                            color: gm.ui.c.raised; border.color: gm.ui.c.accentLo
                            Text { anchors.centerIn: parent; text: "A"; color: gm.ui.c.text; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.px(13); font.bold: true }
                        }
                        Text { text: "Type"; color: gm.ui.c.soft; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.px(14); anchors.verticalCenter: parent.verticalCenter }
                    }
                    HoverHandler { onHoveredChanged: gm.ui.hover(parent, hovered) }
                    TapHandler { onTapped: gm.barClicked(b.index) }
                }
            }
        }
        // Categories
        ListView {
            id: catList
            visible: gm.cats.length > 1
            width: parent.width
            height: visible ? gm.ui.px(36) : 0
            orientation: ListView.Horizontal
            spacing: gm.ui.px(6)
            clip: true
            model: gm.cats
            delegate: Rectangle {
                required property int index
                required property string modelData
                readonly property bool on: gm.category === modelData
                readonly property bool foc: gm.active && gm.focusArea === "cats" && gm.focusIndex === index
                width: ct.implicitWidth + gm.ui.px(26); height: catList.height; radius: height / 2
                color: on ? gm.ui.c.raised : gm.ui.c.card
                border.color: foc ? gm.ui.c.text : on ? gm.ui.c.accent : gm.ui.c.line
                border.width: foc ? gm.ui.px(2) : Math.max(1, gm.ui.px(1))
                Text {
                    id: ct
                    anchors.centerIn: parent
                    text: parent.modelData === "" ? "All" : parent.modelData
                    color: parent.on ? gm.ui.c.text : gm.ui.c.soft
                    font.family: gm.ui.bodyFont
                    font.pixelSize: gm.ui.px(14)
                    font.bold: parent.on
                }
                HoverHandler { onHoveredChanged: gm.ui.hover(parent, hovered) }
                TapHandler { onTapped: gm.categoryClicked(parent.index) }
            }
        }
        // Results
        Item {
            width: parent.width
            height: parent.height - barRow.height - catList.height - gm.ui.px(16)
            Text {
                visible: gm.items.length === 0
                anchors.centerIn: parent
                width: Math.min(parent.width, gm.ui.px(520))
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
                text: gm.loading ? "Looking..." : gm.result && gm.result.error ? gm.result.error : gm.result ? "Nothing matches. Try other words or All." : ""
                color: gm.ui.c.soft
                font.family: gm.ui.bodyFont
                font.pixelSize: gm.ui.px(16)
            }
            GridView {
                id: grid
                anchors.fill: parent
                clip: true
                cellWidth: width / gm.columns
                cellHeight: gm.ui.px(96)
                model: gm.items
                interactive: true
                delegate: Item {
                    id: cell
                    required property int index
                    required property var modelData
                    readonly property bool foc: gm.active && gm.focusArea === "grid" && gm.focusIndex === index
                    width: grid.cellWidth; height: grid.cellHeight
                    Rectangle {
                        anchors.fill: parent
                        anchors.margins: gm.ui.px(3)
                        radius: gm.ui.px(8)
                        color: cell.foc ? gm.ui.c.focusFill : gm.ui.c.card
                        border.color: gm.ui.c.accent
                        border.width: cell.foc ? gm.ui.px(2) : 0
                        Rectangle {
                            id: pic
                            x: gm.ui.px(10); anchors.verticalCenter: parent.verticalCenter
                            width: gm.ui.px(58); height: width; radius: gm.ui.px(6)
                            color: gm.ui.c.raised
                            clip: true
                            Image {
                                id: icon
                                anchors.fill: parent
                                source: cell.modelData.icon || ""
                                sourceSize.width: gm.ui.px(116)
                                fillMode: Image.PreserveAspectCrop
                                asynchronous: true
                                visible: status === Image.Ready
                            }
                            Text {
                                visible: !icon.visible
                                anchors.centerIn: parent
                                text: (cell.modelData.name || "?").substring(0, 2).toUpperCase()
                                color: gm.ui.c.soft
                                font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.px(16); font.bold: true
                            }
                        }
                        Column {
                            x: pic.x + pic.width + gm.ui.px(10)
                            width: parent.width - x - gm.ui.px(10)
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: gm.ui.px(2)
                            Text { width: parent.width; elide: Text.ElideRight; text: cell.modelData.name; color: gm.ui.c.text; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.px(16); font.bold: true }
                            Text { width: parent.width; elide: Text.ElideRight; text: cell.modelData.summary; color: gm.ui.c.soft; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.px(14) }
                            Row {
                                spacing: gm.ui.px(8)
                                Text { text: "↓ " + gm.count(cell.modelData.downloads || 0); color: gm.ui.c.soft; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.px(14); anchors.verticalCenter: parent.verticalCenter }
                                Rectangle {
                                    visible: cell.modelData.code === true
                                    width: codeText.implicitWidth + gm.ui.px(12); height: gm.ui.px(22); radius: gm.ui.px(5)
                                    color: gm.ui.c.raised
                                    Text { id: codeText; anchors.centerIn: parent; text: "Code mod"; color: gm.ui.c.text; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.px(13); font.bold: true }
                                }
                                Rectangle {
                                    visible: cell.modelData.installed === true
                                    width: haveText.implicitWidth + gm.ui.px(12); height: gm.ui.px(22); radius: gm.ui.px(5)
                                    color: gm.ui.c.raised
                                    Text { id: haveText; anchors.centerIn: parent; text: "Have it"; color: gm.ui.c.ok; font.family: gm.ui.bodyFont; font.pixelSize: gm.ui.px(13); font.bold: true }
                                }
                            }
                        }
                        HoverHandler { onHoveredChanged: gm.ui.hover(parent, hovered) }
                        TapHandler { onTapped: gm.itemClicked(cell.index) }
                    }
                }
            }
        }
    }
}
