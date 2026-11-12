import QtQuick

// Something arriving in place: new content, another page, the next card. It fades up and, at Full, travels a short
// way into place. Put it in the item's transform and bind the item's opacity to its part:
//     opacity: arrive.part
//     transform: Arrive { id: arrive; motion: kit.motion }
// then call arrive.play(dx, dy) when the content changes (dx, dy: where it comes from, in screen pixels; Motion's
// distances times the kit's px). A new play takes over from the last at once; the state it shows has already
// changed, so nothing waits for it. Only opacity and a translation move: no layout.
Translate {
    id: arr
    required property Motion motion
    // 0 to 1: the item's opacity (or a factor of it).
    property real part: 1
    // The time of the fade and of the travel (Motion's tokens).
    property int fadeTime: motion.standard
    property int moveTime: motion.moveStandard
    property NumberAnimation fade: NumberAnimation { target: arr; properties: "part"; to: 1; duration: arr.fadeTime; easing.type: arr.motion.ease }
    property NumberAnimation slideX: NumberAnimation { target: arr; properties: "x"; to: 0; duration: arr.moveTime; easing.type: arr.motion.glide }
    property NumberAnimation slideY: NumberAnimation { target: arr; properties: "y"; to: 0; duration: arr.moveTime; easing.type: arr.motion.glide }
    // from: the opacity it fades up from (0, or Motion's swapFrom for words changing in place). A play over one still
    // running goes on from where that one is (presses in quick succession never flash or jump back); settle() first
    // for an exact start.
    function play(dx, dy, from) {
        const busy = running
        fade.stop(); slideX.stop(); slideY.stop()
        if (!motion.on) { part = 1; x = 0; y = 0; return }
        const start = from === undefined ? 0 : from
        part = busy ? Math.max(start, part) : start
        fade.start()
        if (motion.moves && (dx || dy)) { if (!busy) { x = dx || 0; y = dy || 0 } slideX.start(); slideY.start() }
        else { x = 0; y = 0 }
    }
    // Straight to the end (another level, or the item went away).
    function settle() { fade.stop(); slideX.stop(); slideY.stop(); part = 1; x = 0; y = 0 }
    // Something that just showed up: hidden at once, then, once the moment's changes are all in, it plays unless
    // busy() says it is coming in with its page or tab anyway (then it simply shows).
    function appear(busy, dx, dy) {
        if (!motion.on) return
        fade.stop(); slideX.stop(); slideY.stop(); part = 0
        Qt.callLater(() => { if (busy()) arr.settle(); else arr.play(dx, dy) })
    }
    readonly property bool running: fade.running || slideX.running || slideY.running
}
