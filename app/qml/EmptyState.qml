import QtQuick

// A calm message where a list would be: a title, a line or two of plain words and, optionally, one button with its
// controller key (e.g. "A Add mods"). Content is centered in whatever size it is given, e.g. anchors.fill: parent.
//
// ui (required): Main.qml's kit (colors, sizes, fonts, motion and helpers; see COMPONENTS.md).
Item {
    id: root

    required property var ui

    property string title: ""             // e.g. "No mods yet"
    property string body: ""              // e.g. "Press A to add mods from zip files in Downloads."
    property string buttonKey: ""         // e.g. "a"; empty with no label hides the button
    property string buttonLabel: ""       // e.g. "Add mods"
    property bool buttonFocused: true
    property string note: ""              // optional small line under the button
    property real maxWidth: ui.px(560)

    signal activated()

    implicitWidth: maxWidth
    implicitHeight: content.implicitHeight

    // Showing up on its own (not with a page or tab coming in), it fades in as it rises a little into place.
    onVisibleChanged: if (visible) arrive.appear(() => root.ui.arriving, 0, ui.px(ui.motion.nudge))
    Column {
        id: content
        opacity: arrive.part
        transform: Arrive { id: arrive; motion: root.ui.motion }
        anchors.centerIn: parent
        width: Math.min(root.width > 0 ? root.width : root.maxWidth, root.maxWidth)
        spacing: root.ui.px(10)

        Text {
            visible: text !== ""
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
            text: root.title || ""
            wrapMode: Text.WordWrap
            color: root.ui.c.text
            font.family: root.ui.titleFont
            font.pixelSize: root.ui.fpx(19)
            font.bold: true
        }
        Text {
            visible: text !== ""
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
            text: root.body || ""
            wrapMode: Text.WordWrap
            color: root.ui.c.soft
            font.family: root.ui.bodyFont
            font.pixelSize: root.ui.fpx(15)
            lineHeight: 1.15
        }
        Item { visible: button.visible; width: 1; height: root.ui.px(2) }
        ActionButton {
            id: button
            ui: root.ui
            visible: root.buttonKey !== "" || root.buttonLabel !== ""
            anchors.horizontalCenter: parent.horizontalCenter
            label: root.buttonLabel || ""
            cap: root.buttonKey
            primary: true
            focused: root.buttonFocused
            onClicked: root.activated()
        }
        Text {
            visible: text !== ""
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
            text: root.note || ""
            wrapMode: Text.WordWrap
            color: root.ui.c.muted
            font.family: root.ui.bodyFont
            font.pixelSize: root.ui.fpx(14)
        }
    }
}
