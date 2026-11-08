import QtQuick

// The row of buttons every sheet with buttons uses (Before you play, the loader prompt, the plain sheets, text
// entry, Delete for good). One rule everywhere, kept in Main.qml (buttonSheet): the D-pad moves the highlight, A
// presses the highlighted button, B is the cancel button and Start the main one. No key caps on the buttons: the
// legend says what A, B and Start do. A tap presses a button too.
Flow {
    id: sb
    required property var ui
    property var labels: []               // the buttons' words, in order
    property int focusIndex: 0            // the highlighted one (-1: none, the focus is elsewhere on the sheet)
    property int mainIndex: -1            // Start's button: drawn a little stronger
    signal activated(int index)

    spacing: ui.px(10)
    Repeater {
        model: sb.labels
        Rectangle {
            id: btn
            required property int index
            required property string modelData
            readonly property bool foc: index === sb.focusIndex
            width: btnText.implicitWidth + sb.ui.px(36)
            height: sb.ui.px(44)
            radius: sb.ui.px(6)
            color: sb.ui.c.raised
            border.width: Math.max(1, sb.ui.px(1.5))
            border.color: foc ? sb.ui.c.accentHi : index === sb.mainIndex ? sb.ui.c.accent : sb.ui.c.line
            // The highlight: brass, with a soft ring around it.
            Rectangle {
                visible: btn.foc
                anchors.fill: parent
                anchors.margins: -sb.ui.px(3)
                radius: btn.radius + sb.ui.px(3)
                color: "transparent"
                border.width: sb.ui.px(3)
                border.color: Qt.rgba(sb.ui.c.accent.r, sb.ui.c.accent.g, sb.ui.c.accent.b, 0.22)
            }
            Rectangle {
                visible: btn.foc
                anchors.fill: parent
                radius: btn.radius
                gradient: Gradient {
                    GradientStop { position: 0; color: sb.ui.c.accentHi }
                    GradientStop { position: 0.5; color: sb.ui.c.accent }
                    GradientStop { position: 1; color: sb.ui.c.accentLo }
                }
            }
            Text {
                id: btnText
                anchors.centerIn: parent
                text: btn.modelData
                color: btn.foc ? sb.ui.c.onAccent : sb.ui.c.text
                font.family: sb.ui.bodyFont
                font.pixelSize: sb.ui.px(16)
                font.weight: Font.DemiBold
            }
            HoverHandler { onHoveredChanged: sb.ui.hover(btn, hovered) }
            TapHandler { onTapped: sb.activated(btn.index) }
        }
    }
}
