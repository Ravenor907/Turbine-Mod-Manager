import QtQuick

// The backdrop under everything that covers the window (sheets, menus, the wheel, the tutorial): dark enough that
// the words behind it can't be read, so the card on top is the only thing to read. The first child of an Overlay,
// which fades it in and away (Overlay.qml).
//
// ui (required): Main.qml's kit (its scrim color).
Rectangle {
    required property var ui
    readonly property bool isScrim: true
    anchors.fill: parent
    color: ui.scrim
}
