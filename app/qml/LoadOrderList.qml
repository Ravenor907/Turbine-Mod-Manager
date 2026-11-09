import QtQuick
import QtQuick.Shapes

// The Load order tab (Skyrim): the game's plugins in the order they load, each ON or OFF, with Master and Light
// marked, the game's own files fixed at the top, and a warning on plugins whose masters are missing. Sort
// automatically sits above the list (focus index -1). Main.qml handles the keys: A turns a plugin on or off, X Move moves
// it, X shows a plugin's details, A on Sort automatically sorts (with LOOT's masterlist when it can be had). Plugins
// LOOT has notes on carry a badge: "Problem" (red), "Needs cleaning" or "Warning" (amber), "Note".
Item {
    id: lo
    required property var ui
    property var plugins: []
    property string error: ""
    property var messages: ({})           // LOOT's notes: plugin name lowercased -> [{level, text}]
    property string note: ""              // what the last sort did
    property var owners: ({})             // plugin name lowercased -> the name of the mod that placed it
    function notesFor(name) { return lo.messages[(name || "").toLowerCase()] || [] }
    function lootBadge(name) {
        const n = notesFor(name)
        if (n.length === 0) return null
        if (n.some(x => x.level === "error")) return ["Problem", "stop"]
        if (n.some(x => x.level === "warn" && x.text.indexOf("Dirty") === 0)) return ["Needs cleaning", "warn"]
        if (n.some(x => x.level === "warn")) return ["Warning", "warn"]
        return ["Note", "info"]
    }
    property int focusIndex: 0
    property bool moving: false
    property bool active: true

    signal sortClicked()
    signal rowClicked(int index)
    signal rowMenu(int index)             // right-click or a long press: the plugin's options (like X)
    signal scrolled(int first, int last)  // after a wheel scroll or a swipe: the rows in view
    function rowItem(i) { return list.itemAtIndex(i) }

    onFocusIndexChanged: if (focusIndex >= 0) list.positionViewAtIndex(focusIndex, ListView.Contain)

    Column {
        anchors.fill: parent
        spacing: lo.ui.px(6)
        Row {
            width: parent.width
            height: lo.ui.px(44)
            spacing: lo.ui.px(12)
            Rectangle {
                readonly property bool foc: lo.active && lo.focusIndex === -1
                width: sortRow.implicitWidth + lo.ui.px(26); height: lo.ui.px(40); radius: lo.ui.px(8)
                anchors.verticalCenter: parent.verticalCenter
                color: foc ? lo.ui.c.focusFill : lo.ui.c.card
                border.color: foc ? lo.ui.c.accent : lo.ui.c.line
                border.width: foc ? lo.ui.px(2) : Math.max(1, lo.ui.px(1))
                Row {
                    id: sortRow
                    anchors.centerIn: parent
                    spacing: lo.ui.px(8)
                    Rectangle {
                        visible: parent.parent.foc
                        width: lo.ui.px(24); height: width; radius: width / 2; color: lo.ui.c.raised; border.color: lo.ui.c.accentLo
                        anchors.verticalCenter: parent.verticalCenter
                        Text { anchors.centerIn: parent; text: "A"; color: lo.ui.c.text; font.family: lo.ui.bodyFont; font.pixelSize: lo.ui.fpx(13); font.bold: true }
                    }
                    Text { text: "Sort automatically"; color: lo.ui.c.text; font.family: lo.ui.bodyFont; font.pixelSize: lo.ui.fpx(15); font.bold: true; anchors.verticalCenter: parent.verticalCenter }
                }
                HoverHandler { onHoveredChanged: lo.ui.hover(parent, hovered) }
                TapHandler { onTapped: lo.sortClicked() }
            }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                width: parent.width - lo.ui.px(240)
                wrapMode: Text.WordWrap
                maximumLineCount: 2
                elide: Text.ElideRight
                text: lo.error !== "" ? lo.error : lo.note !== "" ? lo.note
                      : "Sort automatically puts these in a safe order. Lower in the list wins."
                color: lo.error !== "" ? lo.ui.c.warn : lo.note.indexOf("Sorted with LOOT") === 0 ? lo.ui.c.ok : lo.ui.c.soft
                font.family: lo.ui.bodyFont
                font.pixelSize: lo.ui.fpx(14)
            }
        }
        ListView {
            id: list
            width: parent.width
            height: parent.height - lo.ui.px(50)
            clip: true
            spacing: lo.ui.px(4)
            model: lo.plugins
            onMovementEnded: lo.scrolled(indexAt(10, contentY + lo.ui.px(10)), indexAt(10, contentY + height - lo.ui.px(10)))
            delegate: Rectangle {
                id: row
                required property int index
                required property var modelData
                readonly property var p: modelData
                readonly property bool foc: lo.active && lo.focusIndex === index
                width: list.width
                height: lo.ui.px(46)
                radius: lo.ui.px(6)
                color: foc ? lo.ui.c.focusFill : lo.ui.c.card
                border.color: lo.ui.c.accent
                border.width: foc ? lo.ui.px(2) : 0
                Row {
                    x: lo.ui.px(10)
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: lo.ui.px(10)
                    Text { width: lo.ui.px(30); horizontalAlignment: Text.AlignRight; text: row.index + 1; color: lo.ui.c.muted; font.family: lo.ui.bodyFont; font.pixelSize: lo.ui.fpx(14); anchors.verticalCenter: parent.verticalCenter }
                    // The game's own files can't be turned off or moved: a lock, and a dimmed pill.
                    Shape {
                        opacity: row.p.fixed === true ? 1 : 0
                        width: lo.ui.px(16); height: lo.ui.px(16)
                        anchors.verticalCenter: parent.verticalCenter
                        preferredRendererType: Shape.CurveRenderer
                        ShapePath {
                            strokeColor: lo.ui.c.muted; strokeWidth: 2; fillColor: "transparent"
                            capStyle: ShapePath.RoundCap; joinStyle: ShapePath.RoundJoin
                            scale: Qt.size(lo.ui.px(16) / 24, lo.ui.px(16) / 24)
                            PathSvg { path: "M6 11h12v9H6z M8.5 11V8a3.5 3.5 0 0 1 7 0v3" }
                        }
                    }
                    Rectangle {
                        width: lo.ui.px(44); height: lo.ui.px(24); radius: lo.ui.px(5)
                        anchors.verticalCenter: parent.verticalCenter
                        opacity: row.p.fixed === true ? 0.45 : 1
                        color: row.p.active ? lo.ui.c.raised : lo.ui.c.ground
                        border.color: row.p.active ? "transparent" : lo.ui.c.line
                        Text { anchors.centerIn: parent; text: row.p.active ? "ON" : "OFF"; color: row.p.active ? lo.ui.c.accentText : lo.ui.c.muted; font.family: lo.ui.bodyFont; font.pixelSize: lo.ui.fpx(13); font.bold: true }
                    }
                    // The mod's name first (the plugin file is the small line after it).
                    Text {
                        text: lo.owners[(row.p.name || "").toLowerCase()] || row.p.name
                        color: row.p.active ? lo.ui.c.text : lo.ui.c.soft
                        font.family: lo.ui.bodyFont; font.pixelSize: lo.ui.fpx(16); font.bold: true
                        anchors.verticalCenter: parent.verticalCenter
                        width: Math.min(implicitWidth, row.width * 0.4)
                        elide: Text.ElideRight
                    }
                    // Which of your mods put it there.
                    Text {
                        readonly property string from: lo.owners[(row.p.name || "").toLowerCase()] || ""
                        visible: from !== ""
                        text: row.p.name
                        color: lo.ui.c.muted
                        font.family: lo.ui.bodyFont; font.pixelSize: lo.ui.fpx(14)
                        anchors.verticalCenter: parent.verticalCenter
                        width: Math.min(implicitWidth, row.width * 0.22)
                        elide: Text.ElideRight
                    }
                    Repeater {
                        model: row.p.fixed ? [["Base game", ""]] : row.p.master ? [["Master", ""]] : []
                            .concat(row.p.unreadable ? [["Can't be read", "warn"]] : [])
                            .concat((row.p.missing || []).length > 0 ? [["Needs " + row.p.missing.join(", "), "warn"]] : [])
                            .concat(lo.lootBadge(row.p.name) ? [lo.lootBadge(row.p.name)] : [])
                            .concat(lo.moving && row.foc ? [["Moving", "info"]] : [])
                        Rectangle {
                            required property var modelData
                            width: bt.implicitWidth + lo.ui.px(12); height: lo.ui.px(22); radius: lo.ui.px(5)
                            anchors.verticalCenter: parent.verticalCenter
                            color: lo.ui.c.raised
                            Text { id: bt; anchors.centerIn: parent; text: parent.modelData[0]; color: parent.modelData[1] === "stop" ? lo.ui.c.stop : parent.modelData[1] === "warn" ? lo.ui.c.warn : parent.modelData[1] === "info" ? lo.ui.c.accentText : lo.ui.c.soft; font.family: lo.ui.bodyFont; font.pixelSize: lo.ui.fpx(13); font.bold: true }
                        }
                    }
                }
                HoverHandler { onHoveredChanged: lo.ui.hover(parent, hovered) }
                // A tap or click anywhere on the row is A on it (the pill included); right-click or a long press: X.
                TapHandler { longPressThreshold: 0.5; onTapped: lo.rowClicked(row.index); onLongPressed: lo.rowMenu(row.index) }
                TapHandler { acceptedButtons: Qt.RightButton; onTapped: lo.rowMenu(row.index) }
            }
        }
    }
}
