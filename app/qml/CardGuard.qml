import QtQuick

// The card of an overlay (0.59): taps, scrolls and hover on the card stay on it, so they never count as a tap
// outside (Overlay.qml) and never reach anything beneath. The card's own Flickable, above it, still scrolls.
MouseArea {
    anchors.fill: parent
    acceptedButtons: Qt.AllButtons
    preventStealing: true
    onWheel: (w) => { w.accepted = true }
}
