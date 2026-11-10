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

    // 0.62: everything that covers the window comes in the same way: a short fade, and at Full a small growth
    // (Motion.qml). It is on screen and takes presses from the first frame: the animation never holds anything
    // back, and closing is instant. Each overlay's file hands the motion system in (motion: ui.motion).
    property Motion motion: Motion { level: "off" }
    onVisibleChanged: {
        if (!visible) { enterFade.stop(); enterGrow.stop(); opacity = 1; scale = 1; return }
        if (!motion.on) return
        enterFade.restart()
        if (motion.moves) enterGrow.restart()
    }
    NumberAnimation { id: enterFade; target: ov; properties: "opacity"; from: 0; to: 1; duration: ov.motion.fast; easing.type: ov.motion.enter }
    NumberAnimation { id: enterGrow; target: ov; properties: "scale"; from: ov.motion.fromScale; to: 1; duration: ov.motion.moveBase; easing.type: ov.motion.enter }

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.AllButtons
        hoverEnabled: true
        preventStealing: true
        onWheel: (w) => { w.accepted = true }
        onClicked: (m) => { if (m.button === Qt.LeftButton && ov.closeOnOutside) ov.outsideTapped() }
    }
}
