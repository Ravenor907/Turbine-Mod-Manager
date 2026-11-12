import QtQuick

// How Turbine moves, in one place. Every animation in app/qml takes its time, its curve, its distance and its scale
// from here: no other file writes a duration in milliseconds or names an easing curve (tools/qml-check.js fails the
// build on one).
//
// The level is the player's (Settings, Look, Animations):
//   full     fades, glides, slides and small scales;
//   reduced  fades only: nothing travels or grows;
//   off      instant: nothing animates at all (the build's checks run this way, so they never race an animation).
//
// The language: quick (120 ms) for the focus, a press, words that change in place and everything that leaves;
// standard (200 ms) for sheets, dialogs, menus, the wheel, toasts, tabs and rows that come, go or move; gentle
// (280 ms) for a whole page (another game, Settings, Nexus). Things fade and settle on OutCubic and travel into
// place on OutQuint (fast, then a soft landing); going is quicker than coming and starts at once (a backdrop
// lifting, a toast going), so the next thing can be read sooner; nothing bounces or overshoots.
// Distances are short (16, 24 or 32 design pixels) and scales small (0.96 to 1, 0.97 for a press).
// Never in the way: an animation only ever follows a state that has already changed, so a press during one acts at
// once, a new one takes over from the last instead of queueing behind it, and only opacity, position and scale
// change while anything moves (no layout, no layers, no shaders).
QtObject {
    id: motion
    property string level: "full"

    readonly property bool on: level !== "off"
    // Position and scale change over time only at Full; Reduced keeps the fades.
    readonly property bool moves: level === "full"

    // Fades (Full and Reduced).
    readonly property int quick: on ? 120 : 0
    readonly property int standard: on ? 200 : 0
    readonly property int gentle: on ? 280 : 0
    // Travel: glides, slides and scales (0 unless Full).
    readonly property int moveQuick: moves ? 120 : 0
    readonly property int moveStandard: moves ? 200 : 0
    readonly property int moveGentle: moves ? 280 : 0

    // Fades and small changes, in or out; travel into place; the even pace of a ring that fills.
    readonly property int ease: Easing.OutCubic
    readonly property int glide: Easing.OutQuint
    readonly property int even: Easing.Linear

    // How far things travel (design pixels; 0 unless Full): a small shift (a page or the panel settling, a toast
    // rising), a tab or a card coming in, a side sheet sliding in from the right.
    readonly property real nudge: moves ? 16 : 0
    readonly property real shift: moves ? 24 : 0
    readonly property real sheetShift: moves ? 32 : 0
    // What a dialog, menu or the wheel grows from as it fades in, and what a button or row shrinks to while pressed.
    readonly property real popScale: moves ? 0.96 : 1
    readonly property real pressScale: moves ? 0.97 : 1
    // Words or a panel changing in place fade up from this, never from nothing (scrolling a list never flickers).
    readonly property real swapFrom: 0.35

    // The start-up splash: the icon comes in over the ground, holds, then the whole splash fades into the app.
    // Reduced: a short plain fade. Off: none.
    readonly property int splashIn: moves ? 400 : on ? 150 : 0
    readonly property int splashHold: moves ? 350 : on ? 300 : 0
    readonly property int splashOut: moves ? gentle : on ? 150 : 0
    readonly property real splashFrom: moves ? 0.92 : 1
    readonly property real splashTo: moves ? 1.04 : 1

    // Not transitions, so they run at every level: a busy spinner's turn, and the scrolling test's own loops.
    readonly property int spin: 900
    readonly property int testSpin: 1200
    readonly property int testScroll: 4600

    // Overlays (Overlay.qml) hand the backdrop over when one closes as another opens in the same moment, so the
    // window never brightens between them. Set and cleared within one pass of the event loop.
    property var turnOpened: null
    property bool turnClosed: false
    signal ghostsOff()
    function overlayOpened(ov) { turnOpened = ov; ghostsOff(); Qt.callLater(motion.endTurn) }
    function overlayClosed() { turnClosed = true; Qt.callLater(motion.endTurn) }
    function endTurn() { turnOpened = null; turnClosed = false }
}
