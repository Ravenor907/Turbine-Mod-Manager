import QtQuick

// The one base for everything that covers the window: sheets, menus, pop-ups, the text prompt, the quick wheel and
// the tutorial. It is the only place that decides "a tap outside closes", and the only place that decides how they
// come and go.
//
// Why it exists: in Qt 6 a MouseArea that takes a press stops the items under it, but never the pointer handlers
// (TapHandler, DragHandler, PointHandler) of the items under it: they still see the press and fire on release. So
// a backdrop alone can't stop a tap reaching a card or a button beneath. Main.qml turns everything under the top
// layer off while an overlay is up (app.topLayer); this backdrop then takes every press, click, scroll and hover that
// lands outside the card, and never lets a drag beneath steal the press.
//
// Use: the overlay's file has Overlay as its root, sets onOutsideTapped (or closeOnOutside: false), and puts a
// CardGuard as the first child of its card, so taps on the card don't count as outside.
//
// Coming in (Motion.qml): the backdrop (a Scrim, or `backdrop`) fades in; a Card (a dialog, a menu, a pop-up) fades
// in and grows from Motion's popScale; a SideSheet fades in and slides in from the right; anything else with
// showFade and showMove (the wheel's pie) is moved the same way as a Card. An overlay with no card (`fadesWhole`)
// fades in whole. Going: the card is gone at once (its words already show what the press changed) and the backdrop
// fades away behind it. One closing as another opens hands its backdrop over, so the window never brightens between
// them. It is on screen and takes presses from the first frame; nothing here ever holds a press back.
Item {
    id: ov
    anchors.fill: parent
    property bool closeOnOutside: true
    signal outsideTapped()

    // Each overlay's file hands the motion system in (motion: ui.motion).
    property Motion motion: Motion { level: "off" }
    // The backdrop that fades in and, on closing, away: the first Scrim among the children unless set.
    property Item backdrop: null
    // No card: the whole overlay fades in (the tutorial, the installer's page).
    property bool fadesWhole: false
    // Where the fading backdrop is drawn after closing, among the overlay's siblings.
    property real ghostZ: z

    // 0 to 1 as it comes in: the backdrop, the card's fade, the card's travel.
    property real backPart: 1
    property real fadePart: 1
    property real movePart: 1
    opacity: fadesWhole ? fadePart : 1
    NumberAnimation { id: backIn; target: ov; properties: "backPart"; from: 0; to: 1; duration: ov.motion.standard; easing.type: ov.motion.ease }
    NumberAnimation { id: fadeIn; target: ov; properties: "fadePart"; from: 0; to: 1; duration: ov.motion.standard; easing.type: ov.motion.ease }
    NumberAnimation { id: moveIn; target: ov; properties: "movePart"; from: 0; to: 1; duration: ov.motion.moveStandard; easing.type: ov.motion.glide }

    // The backdrop and the cards follow the parts (wired once, when the overlay is made).
    Component.onCompleted: {
        for (let i = 0; i < children.length; i++) {
            const k = children[i]
            if (!backdrop && k.isScrim === true) backdrop = k
            if (typeof k.showMove === "number") {
                k.showFade = Qt.binding(() => ov.fadePart)
                k.showMove = Qt.binding(() => ov.movePart)
            }
        }
        if (backdrop && !fadesWhole) backdrop.opacity = Qt.binding(() => ov.backPart)
    }
    onVisibleChanged: {
        const was = fadesWhole ? fadePart : backPart
        ghostFade.stop(); if (ghost) ghost.visible = false
        backIn.stop(); fadeIn.stop(); moveIn.stop()
        backPart = 1; fadePart = 1; movePart = 1
        if (!motion.on) return
        if (visible) {
            // Another overlay closed just now: its backdrop is this one's, already there.
            if (!motion.turnClosed) { backPart = 0; backIn.start() }
            fadePart = 0; fadeIn.start()
            movePart = 0; moveIn.start()
            motion.overlayOpened(ov)
        } else {
            const next = motion.turnOpened
            motion.overlayClosed()
            if (next && next !== ov) next.takeBackdrop()
            else leaveBackdrop(was)
        }
    }
    // The overlay opening as this one closes keeps the backdrop as it is (no fade in from nothing).
    function takeBackdrop() { backIn.stop(); backPart = 1 }

    // The backdrop fading away after the overlay closed: a plain rectangle beside it, in its color, taking no input.
    property Rectangle ghost: null
    Component { id: ghostMaker; Rectangle { visible: false; enabled: false } }
    NumberAnimation { id: ghostFade; properties: "opacity"; to: 0; duration: ov.motion.quick; easing.type: ov.motion.ease
                      onFinished: if (ov.ghost) ov.ghost.visible = false }
    Connections { target: ov.motion; function onGhostsOff() { if (ov.visible) return; ghostFade.stop(); if (ov.ghost) ov.ghost.visible = false } }
    function leaveBackdrop(from) {
        const b = backdrop
        if (!b || !parent || b.width <= 0 || b.height <= 0) return
        if (!ghost) ghost = ghostMaker.createObject(parent)
        if (!ghost) return
        const p = b.mapToItem(parent, 0, 0)
        ghost.x = p.x; ghost.y = p.y; ghost.width = b.width; ghost.height = b.height
        ghost.color = b.color
        ghost.z = ghostZ
        ghost.opacity = from
        ghost.visible = true
        ghostFade.target = ghost
        ghostFade.restart()
    }

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.AllButtons
        hoverEnabled: true
        preventStealing: true
        onWheel: (w) => { w.accepted = true }
        onClicked: (m) => { if (m.button === Qt.LeftButton && ov.closeOnOutside) ov.outsideTapped() }
    }
}
