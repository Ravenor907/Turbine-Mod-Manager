import QtQuick

// One button's cap in Main.qml's hints and bar ("A", "LB", "Enter"): the glyph follows the pad and the last input
// (Main.qml's glyph()).
//
// win (required): Main.qml's window. k: the button ("a", "lb", "view"). off: this cap has nothing to say here.
Rectangle {
    required property var win
    property string k: ""
    property bool square: k.length > 1
    // No caps after the mouse (and `off`: this cap has nothing to say here). Without a size a row
    // leaves it out, so the words beside it close up.
    property bool off: false
    readonly property bool gone: win.mouseLegend || off
    // Whole pixels, so the outline never falls between two and loses its right edge.
    implicitWidth: gone ? 0 : Math.ceil(Math.max(win.px(24), keyText.implicitWidth + win.px(12)))
    implicitHeight: gone ? 0 : win.px(24)
    radius: square ? win.px(5) : height / 2
    color: gone ? "transparent" : win.colors.raised
    border.color: win.colors.accentLo
    border.width: gone ? 0 : Math.max(1, win.px(1))
    Text {
        id: keyText
        anchors.centerIn: parent
        text: parent.win.glyph(parent.k)
        visible: !parent.gone
        color: parent.win.colors.text
        font.family: parent.win.bodyFont
        font.pixelSize: parent.win.fpx(14)
        font.bold: true
    }
}
