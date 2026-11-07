import QtQuick

// A plain sheet: a title, a few sentences, numbered steps or a list of notes, and one or two buttons. Used for the
// no-browser warning, the browser helper's setup steps, a mod's install notes and "Add this file?" for a download
// the watcher could only guess. A runs the focused button, B closes. Main.qml handles the keys.
Item {
    id: sh
    required property var ui
    anchors.fill: parent

    property string title: ""
    property string text: ""
    property var steps: []                // numbered, in order
    property var notes: []                // bulleted, no order
    property string footnote: ""          // a muted line under the steps (a folder name, a warning)
    property var buttons: []              // [{key: "a" or "b", label}]
    property int focusIndex: 0

    signal activated(int index)
    signal closed()

    Rectangle { anchors.fill: parent; color: "#C0000000" }
    MouseArea { anchors.fill: parent; onClicked: sh.closed() }
    Rectangle {
        anchors.centerIn: parent
        width: Math.min(parent.width - sh.ui.px(40), sh.ui.px(660))
        height: Math.min(parent.height - sh.ui.px(60), col.implicitHeight + sh.ui.px(40))
        radius: sh.ui.px(10)
        color: sh.ui.c.panel
        border.color: sh.ui.c.line
        clip: true
        MouseArea { anchors.fill: parent }
        Rectangle { width: parent.width; height: sh.ui.px(3); color: sh.ui.c.accent; radius: sh.ui.px(2) }
        Column {
            id: col
            x: sh.ui.px(22); y: sh.ui.px(20)
            width: parent.width - sh.ui.px(44)
            spacing: sh.ui.px(10)
            Text {
                width: parent.width
                wrapMode: Text.WordWrap
                text: sh.title
                color: sh.ui.c.text
                font.family: sh.ui.titleFont; font.pixelSize: sh.ui.px(18); font.bold: true
            }
            Text {
                visible: sh.text !== ""
                width: parent.width
                wrapMode: Text.WordWrap
                text: sh.text
                color: sh.ui.c.soft
                font.family: sh.ui.bodyFont; font.pixelSize: sh.ui.px(15)
            }
            Repeater {
                model: sh.steps
                Row {
                    required property int index
                    required property var modelData
                    width: col.width
                    spacing: sh.ui.px(12)
                    Rectangle {
                        width: sh.ui.px(28); height: width; radius: width / 2
                        color: sh.ui.c.raised; border.color: sh.ui.c.accent; border.width: Math.max(1, sh.ui.px(1))
                        Text { anchors.centerIn: parent; text: index + 1; color: sh.ui.c.accentText; font.family: sh.ui.bodyFont; font.pixelSize: sh.ui.px(15); font.bold: true }
                    }
                    Text {
                        width: parent.width - sh.ui.px(40)
                        anchors.verticalCenter: parent.verticalCenter
                        wrapMode: Text.WordWrap
                        text: modelData
                        color: sh.ui.c.text
                        font.family: sh.ui.bodyFont; font.pixelSize: sh.ui.px(15)
                    }
                }
            }
            Repeater {
                model: sh.notes
                Row {
                    required property var modelData
                    width: col.width
                    spacing: sh.ui.px(10)
                    Rectangle { y: sh.ui.px(8); width: sh.ui.px(7); height: width; radius: width / 2; color: sh.ui.c.accent }
                    Text {
                        width: parent.width - sh.ui.px(17)
                        wrapMode: Text.WordWrap
                        text: modelData
                        color: sh.ui.c.text
                        font.family: sh.ui.bodyFont; font.pixelSize: sh.ui.px(15)
                    }
                }
            }
            Text {
                visible: sh.footnote !== ""
                width: parent.width
                // Long folder names still break; words don't.
                wrapMode: Text.Wrap
                text: sh.footnote
                color: sh.ui.c.muted
                font.family: sh.ui.bodyFont; font.pixelSize: sh.ui.px(14)
            }
            Row {
                spacing: sh.ui.px(10)
                topPadding: sh.ui.px(4)
                Repeater {
                    model: sh.buttons
                    Rectangle {
                        required property int index
                        required property var modelData
                        readonly property bool foc: sh.focusIndex === index
                        width: btnRow.implicitWidth + sh.ui.px(24); height: sh.ui.px(40)
                        radius: sh.ui.px(6)
                        color: foc ? sh.ui.c.focusFill : sh.ui.c.card
                        border.color: foc ? sh.ui.c.accent : sh.ui.c.line
                        border.width: foc ? sh.ui.px(2) : Math.max(1, sh.ui.px(1))
                        Row {
                            id: btnRow
                            anchors.centerIn: parent
                            spacing: sh.ui.px(8)
                            Rectangle {
                                visible: !!modelData.key
                                width: sh.ui.px(24); height: width; radius: width / 2; color: sh.ui.c.raised; border.color: sh.ui.c.accentLo
                                anchors.verticalCenter: parent.verticalCenter
                                Text { anchors.centerIn: parent; text: (modelData.key || "").toUpperCase(); color: sh.ui.c.text; font.family: sh.ui.bodyFont; font.pixelSize: sh.ui.px(13); font.bold: true }
                            }
                            Text { text: modelData.label; color: parent.parent.foc ? sh.ui.c.accentText : sh.ui.c.text; font.family: sh.ui.bodyFont; font.pixelSize: sh.ui.px(15); font.bold: true; anchors.verticalCenter: parent.verticalCenter }
                        }
                        HoverHandler { onHoveredChanged: sh.ui.hover(parent, hovered) }
                        TapHandler { onTapped: sh.activated(index) }
                    }
                }
            }
        }
    }
}
