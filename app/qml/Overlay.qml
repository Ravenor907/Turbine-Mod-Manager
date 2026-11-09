import QtQuick

// The one base for everything that covers the window: sheets, menus, pop-ups, the text prompt, the quick wheel and
// the tutorial (0.59). It is the only place that decides "a tap outside closes".
//
// Why it exists: in Qt 6 a MouseArea that takes a press stops the items under it, but never the pointer handlers
// (TapHandler, DragHandler, PointHandler) of the items under it: they still see the press and fire on release. So
// a backdrop alone can't stop a tap reaching a card or a button beneath. Main.qml turns everything under the top
// layer off while an overlay is up (app.topLayer); this backdrop then takes every press, click, scroll and hover that
// lands outside the card, and never lets a drag beneath steal the press.
//
// Use: the overlay's file has Overlay as its root, sets onOutsideTapped (or closeOnOutside: false), and puts a
// CardGuard as the first child of its card, so taps on the card don't count as outside.
Item {
    id: ov
    anchors.fill: parent
    property bool closeOnOutside: true
    signal outsideTapped()

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.AllButtons
        hoverEnabled: true
        preventStealing: true
        onWheel: (w) => { w.accepted = true }
        onClicked: (m) => { if (m.button === Qt.LeftButton && ov.closeOnOutside) ov.outsideTapped() }
    }
}
