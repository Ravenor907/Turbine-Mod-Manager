import QtQuick

// The Load order tab (Skyrim): the game's plugins in the order they load, each ON or OFF, with Master and Light
// marked, the game's own files fixed at the top, and a warning on plugins whose masters are missing. Sort
// automatically sits above the list (focus index -1). Main.qml handles the keys: A turns a plugin on or off, Y moves
// it, X shows a plugin's details, A on Sort automatically sorts (with LOOT's masterlist when it can be had). Plugins
// LOOT has notes on carry a badge: "LOOT: problem" (red), "Dirty" or "LOOT: warning" (amber), "LOOT: note".
Item {
    id: lo
    required property var ui
    property var plugins: []
    property string error: ""
    property var messages: ({})           // LOOT's notes: plugin name lowercased -> [{level, text}]
    property string note: ""              // what the last sort did
    function notesFor(name) { return lo.messages[(name || "").toLowerCase()] || [] }
    function lootBadge(name) {
        const n = notesFor(name)
        if (n.length === 0) return null
        if (n.some(x => x.level === "error")) return ["LOOT: problem", "stop"]
        if (n.some(x => x.level === "warn" && x.text.indexOf("Dirty") === 0)) return ["Dirty", "warn"]
        if (n.some(x => x.level === "warn")) return ["LOOT: warning", "warn"]
        return ["LOOT: note", "info"]
    }
    property int focusIndex: 0
    property bool moving: false
    property bool active: true

    signal sortClicked()
    signal rowClicked(int index)
    signal toggleClicked(int index)

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
                        Text { anchors.centerIn: parent; text: "A"; color: lo.ui.c.text; font.family: lo.ui.bodyFont; font.pixelSize: lo.ui.px(13); font.bold: true }
                    }
                    Text { text: "Sort automatically"; color: lo.ui.c.text; font.family: lo.ui.bodyFont; font.pixelSize: lo.ui.px(15); font.bold: true; anchors.verticalCenter: parent.verticalCenter }
                }
                TapHandler { onTapped: lo.sortClicked() }
            }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                width: parent.width - lo.ui.px(240)
                wrapMode: Text.WordWrap
                maximumLineCount: 2
                elide: Text.ElideRight
                text: lo.error !== "" ? lo.error : lo.note !== "" ? lo.note
                      : "Sorts with LOOT's masterlist when it can be downloaded, otherwise masters first. Lower in the list wins."
                color: lo.error !== "" ? lo.ui.c.warn : lo.note.indexOf("Sorted with LOOT") === 0 ? lo.ui.c.ok : lo.ui.c.soft
                font.family: lo.ui.bodyFont
                font.pixelSize: lo.ui.px(14)
            }
        }
        ListView {
            id: list
            width: parent.width
            height: parent.height - lo.ui.px(50)
            clip: true
            spacing: lo.ui.px(4)
            model: lo.plugins
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
                    Text { width: lo.ui.px(30); horizontalAlignment: Text.AlignRight; text: row.index + 1; color: lo.ui.c.muted; font.family: lo.ui.bodyFont; font.pixelSize: lo.ui.px(14); anchors.verticalCenter: parent.verticalCenter }
                    Rectangle {
                        width: lo.ui.px(44); height: lo.ui.px(24); radius: lo.ui.px(5)
                        anchors.verticalCenter: parent.verticalCenter
                        color: row.p.active ? lo.ui.c.raised : lo.ui.c.ground
                        border.color: row.p.active ? "transparent" : lo.ui.c.line
                        Text { anchors.centerIn: parent; text: row.p.active ? "ON" : "OFF"; color: row.p.active ? lo.ui.c.accentText : lo.ui.c.muted; font.family: lo.ui.bodyFont; font.pixelSize: lo.ui.px(13); font.bold: true }
                        TapHandler { onTapped: lo.toggleClicked(row.index) }
                    }
                    Text {
                        text: row.p.name
                        color: row.p.active ? lo.ui.c.text : lo.ui.c.soft
                        font.family: lo.ui.bodyFont; font.pixelSize: lo.ui.px(16); font.bold: true
                        anchors.verticalCenter: parent.verticalCenter
                        width: Math.min(implicitWidth, row.width * 0.5)
                        elide: Text.ElideRight
                    }
                    Repeater {
                        model: (row.p.fixed ? [["Game file", ""]] : []).concat(row.p.master ? [["Master", ""]] : []).concat(row.p.light ? [["Light", ""]] : [])
                            .concat(row.p.unreadable ? [["Can't be read", "warn"]] : [])
                            .concat((row.p.missing || []).length > 0 ? [["Needs " + row.p.missing.join(", "), "warn"]] : [])
                            .concat(lo.lootBadge(row.p.name) ? [lo.lootBadge(row.p.name)] : [])
                            .concat(lo.moving && row.foc ? [["Moving", "info"]] : [])
                        Rectangle {
                            required property var modelData
                            width: bt.implicitWidth + lo.ui.px(12); height: lo.ui.px(22); radius: lo.ui.px(5)
                            anchors.verticalCenter: parent.verticalCenter
                            color: lo.ui.c.raised
                            Text { id: bt; anchors.centerIn: parent; text: parent.modelData[0]; color: parent.modelData[1] === "stop" ? lo.ui.c.stop : parent.modelData[1] === "warn" ? lo.ui.c.warn : parent.modelData[1] === "info" ? lo.ui.c.accentText : lo.ui.c.soft; font.family: lo.ui.bodyFont; font.pixelSize: lo.ui.px(13); font.bold: true }
                        }
                    }
                }
                TapHandler { onTapped: lo.rowClicked(row.index) }
            }
        }
    }
}
