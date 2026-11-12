import QtQuick

// A controller button drawn as a small cap: round for one letter, rounded for a longer name ("LB").
Rectangle {
    id: cap
    required property var ui
    property string k: ""
    implicitWidth: Math.max(ui.px(24), capLabel.implicitWidth + ui.px(12))
    implicitHeight: ui.px(24)
    radius: k.length > 1 ? ui.px(6) : height / 2
    color: ui.c.raised
    border.color: ui.c.accentLo
    border.width: Math.max(1, ui.px(1))
    Text {
        id: capLabel
        anchors.centerIn: parent
        text: cap.ui.glyph(cap.k)
        color: cap.ui.c.text
        font.family: cap.ui.bodyFont
        font.pixelSize: cap.ui.fpx(14)
        font.bold: true
    }
}
