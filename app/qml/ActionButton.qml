import QtQuick

// One button look everywhere (the sheets' button rows, Report a bug): the main button filled with the accent, the
// others raised with a thin line, and the highlight a ring just outside the button, never a fill. A cap in front
// of the words says which key presses it (A on the highlighted one, B on the cancel one; none after the mouse).
//
// ui (required): Main.qml's kit. label: the words. cap: the key ("a", "b"; "" for none). capRoom: keep room for a
// cap even when there is none, so a row of buttons never shifts as the highlight moves. primary, focused, dim.
Rectangle {
    id: ab
    required property var ui
    property string label: ""
    property string cap: ""
    property bool capRoom: false
    property bool primary: false
    property bool focused: false
    property bool dim: false
    signal clicked()
    // Pressed down (true) and let go or slid off (false).
    signal held(bool down)
    readonly property string capShown: ui.capsShown ? cap : ""
    readonly property real capSlot: ui.capsShown && (capRoom || cap !== "") ? ui.px(20) + ui.px(8) : 0
    // Its width when nothing squeezes it.
    readonly property real want: abText.implicitWidth + ui.px(40) + capSlot
    implicitWidth: want
    implicitHeight: ui.tpx(44)
    radius: ui.radiusSmall
    opacity: dim ? 0.55 : 1
    border.width: primary ? 0 : Math.max(1, ui.px(1))
    border.color: ui.c.line
    gradient: Gradient {
        GradientStop { position: 0; color: ab.primary ? ab.ui.c.accentHi : ab.ui.c.raised }
        GradientStop { position: 1; color: ab.primary ? ab.ui.c.accent : ab.ui.c.raised }
    }
    Rectangle {
        visible: ab.focused
        anchors.fill: parent
        anchors.margins: -ab.ui.px(4)
        radius: ab.radius + ab.ui.px(4)
        color: "transparent"
        border.width: ab.ui.ringWidth
        border.color: ab.ui.ring
    }
    Row {
        anchors.centerIn: parent
        spacing: ab.ui.px(8)
        Rectangle {
            visible: ab.capShown !== ""
            anchors.verticalCenter: parent.verticalCenter
            width: Math.max(ab.ui.px(20), capText.implicitWidth + ab.ui.px(8)); height: ab.ui.px(20)
            radius: capText.text.length > 1 ? ab.ui.px(5) : height / 2
            color: "transparent"
            border.width: Math.max(1, ab.ui.px(1.5))
            border.color: abText.color
            opacity: 0.85
            Text {
                id: capText
                anchors.centerIn: parent
                text: ab.capShown !== "" ? ab.ui.glyph(ab.capShown) : ""
                color: abText.color
                font.family: ab.ui.bodyFont
                font.pixelSize: ab.ui.fpx(12)
                font.bold: true
            }
        }
        Text {
            id: abText
            anchors.verticalCenter: parent.verticalCenter
            text: ab.label
            color: ab.primary ? ab.ui.c.onAccent : ab.ui.c.text
            font.family: ab.ui.bodyFont
            font.pixelSize: ab.ui.fpx(16)
            font.weight: Font.DemiBold
        }
    }
    HoverHandler { onHoveredChanged: ab.ui.hover(ab, hovered) }
    TapHandler { onTapped: ab.clicked(); onPressedChanged: ab.held(pressed) }
}
