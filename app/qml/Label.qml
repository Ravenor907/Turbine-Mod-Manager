import QtQuick

// Main.qml's plain words: the body font at 15, the text color, one line cut at the end when it doesn't fit.
//
// win (required): Main.qml's window (colors, fonts and sizes). A name of its own, so `app` in the bindings written
// on a Label still means the window of the file it is in.
Text {
    required property var win
    color: win.colors.text
    font.family: win.bodyFont
    font.pixelSize: win.fpx(15)
    elide: Text.ElideRight
    verticalAlignment: Text.AlignVCenter
}
