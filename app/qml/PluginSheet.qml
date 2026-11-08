import QtQuick

// One plugin's details from the Load order tab (X): what kind it is, its place, the plugins it needs, and LOOT's
// notes on it (warnings like a dirty plugin or one that doesn't work with another). B closes it.
Item {
    id: sheet
    required property var ui
    anchors.fill: parent

    property var plugin: null             // the engine's plugin entry
    property int position: 0              // 1-based
    property var notes: []                // [{level: "say" | "warn" | "error", text}]

    signal closed()

    function levelWord(l) { return l === "error" ? "Problem" : l === "warn" ? "Warning" : "Note" }

    Rectangle { anchors.fill: parent; color: "#B0000000" }
    MouseArea { anchors.fill: parent; onClicked: sheet.closed() }
    Rectangle {
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.bottomMargin: sheet.ui.px(56)
        width: Math.min(parent.width * 0.6, sheet.ui.px(560))
        color: sheet.ui.c.panel
        border.color: sheet.ui.c.line
        MouseArea { anchors.fill: parent }
        Flickable {
            id: view
            anchors.fill: parent
            anchors.margins: sheet.ui.px(18)
            contentHeight: col.implicitHeight
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            Column {
                id: col
                width: view.width
                spacing: sheet.ui.px(8)
                Text {
                    width: parent.width
                    wrapMode: Text.WrapAnywhere
                    text: sheet.plugin ? sheet.plugin.name : ""
                    color: sheet.ui.c.text
                    font.family: sheet.ui.titleFont; font.pixelSize: sheet.ui.px(18); font.bold: true
                }
                Text {
                    width: parent.width
                    wrapMode: Text.WordWrap
                    text: !sheet.plugin ? "" : [
                        sheet.plugin.active ? "On" : "Off",
                        "loads " + sheet.position + (sheet.position === 1 ? "st" : sheet.position === 2 ? "nd" : sheet.position === 3 ? "rd" : "th"),
                        sheet.plugin.fixed ? "part of the game" : "",
                        sheet.plugin.master ? "master" : "",
                        sheet.plugin.light ? "light plugin" : ""
                    ].filter(s => s !== "").join(" · ")
                    color: sheet.ui.c.soft
                    font.family: sheet.ui.bodyFont; font.pixelSize: sheet.ui.px(15)
                }
                Text {
                    visible: !!sheet.plugin && (sheet.plugin.masters || []).length > 0
                    width: parent.width
                    wrapMode: Text.WordWrap
                    text: sheet.plugin ? "Needs: " + (sheet.plugin.masters || []).join(", ") : ""
                    color: sheet.ui.c.soft
                    font.family: sheet.ui.bodyFont; font.pixelSize: sheet.ui.px(15)
                }
                Text {
                    visible: !!sheet.plugin && (sheet.plugin.missing || []).length > 0
                    width: parent.width
                    wrapMode: Text.WordWrap
                    text: sheet.plugin ? "Missing or off: " + (sheet.plugin.missing || []).join(", ") : ""
                    color: sheet.ui.c.warn
                    font.family: sheet.ui.bodyFont; font.pixelSize: sheet.ui.px(15); font.bold: true
                }
                Text {
                    topPadding: sheet.ui.px(6)
                    text: "NOTES"
                    color: sheet.ui.c.muted
                    font.family: sheet.ui.bodyFont; font.pixelSize: sheet.ui.px(14); font.bold: true; font.letterSpacing: sheet.ui.px(1)
                }
                Text {
                    visible: sheet.notes.length === 0
                    width: parent.width
                    wrapMode: Text.WordWrap
                    text: "No notes about this plugin."
                    color: sheet.ui.c.soft
                    font.family: sheet.ui.bodyFont; font.pixelSize: sheet.ui.px(15)
                }
                Repeater {
                    model: sheet.notes
                    Rectangle {
                        required property var modelData
                        width: col.width
                        height: noteCol.implicitHeight + sheet.ui.px(16)
                        radius: sheet.ui.px(6)
                        color: sheet.ui.c.card
                        Rectangle {
                            width: sheet.ui.px(4); height: parent.height; radius: sheet.ui.px(2)
                            color: parent.modelData.level === "error" ? sheet.ui.c.stop : parent.modelData.level === "warn" ? sheet.ui.c.warn : sheet.ui.c.line
                        }
                        Column {
                            id: noteCol
                            x: sheet.ui.px(14); y: sheet.ui.px(8)
                            width: parent.width - sheet.ui.px(24)
                            spacing: sheet.ui.px(2)
                            Text {
                                text: sheet.levelWord(parent.parent.modelData.level)
                                color: parent.parent.modelData.level === "error" ? sheet.ui.c.stop : parent.parent.modelData.level === "warn" ? sheet.ui.c.warn : sheet.ui.c.muted
                                font.family: sheet.ui.bodyFont; font.pixelSize: sheet.ui.px(14); font.bold: true
                            }
                            Text {
                                width: parent.width
                                wrapMode: Text.WordWrap
                                textFormat: Text.PlainText
                                text: parent.parent.modelData.text
                                color: sheet.ui.c.text
                                font.family: sheet.ui.bodyFont; font.pixelSize: sheet.ui.px(15)
                            }
                        }
                    }
                }
            }
        }
    }
}
