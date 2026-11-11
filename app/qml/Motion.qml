import QtQuick

// How Turbine moves, in one place (0.62). Every animation in app/qml takes its time and its curve from here: no
// other file writes a duration in milliseconds or names an easing curve (tools/qml-check.js fails the build on one).
//
// The level is the player's (Settings, Look, Animations):
//   full     fades, glides, slides and small scales;
//   reduced  fades only: nothing travels or grows;
//   off      instant: nothing animates at all (the build's checks run this way, so they never race an animation).
//
// The rules they follow: fast and never in the way (a press during an animation acts at once: animations only ever
// follow a state that has already changed, they never hold one back), things come in on OutCubic and leave on
// InCubic, nothing bounces or overshoots, and only opacity, position and scale are animated in long lists.
QtObject {
    id: motion
    property string level: "full"

    readonly property bool on: level !== "off"
    // Position and scale change over time only at Full; Reduced keeps the fades.
    readonly property bool moves: level === "full"

    // Fades (and anything else that may run at Reduced).
    readonly property int fast: on ? 120 : 0
    readonly property int base: on ? 180 : 0
    // Travel: glides, slides, scales, the page turn. 0 unless Full.
    readonly property int moveFast: moves ? 120 : 0
    readonly property int moveBase: moves ? 180 : 0
    readonly property int page: moves ? 320 : on ? 180 : 0

    // Coming in, going out, and the even pace of a spinner or a ring that fills.
    readonly property int enter: Easing.OutCubic
    readonly property int leave: Easing.InCubic
    readonly property int even: Easing.Linear

    // A sheet or pop-up grows from this to 1 as it fades in; a slide travels this far (design pixels, times `slide`).
    readonly property real fromScale: moves ? 0.97 : 1
    readonly property real slide: moves ? 1 : 0

    // The start-up splash: in, hold, out. Reduced: a short plain fade. Off: none.
    readonly property int splashIn: moves ? 450 : on ? 150 : 0
    readonly property int splashHold: moves ? 250 : on ? 300 : 0
    readonly property int splashOut: moves ? 500 : on ? 150 : 0
    readonly property real splashFrom: moves ? 0.7 : 1
    readonly property real splashTo: moves ? 1.25 : 1

    // Not transitions, so they run at every level: a busy spinner's turn, and the scrolling test's own loops.
    readonly property int spin: 900
    readonly property int testSpin: 1200
    readonly property int testScroll: 4600
}
