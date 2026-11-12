import QtQuick

// The start-up splash: Turbine's icon in the middle of the ground, fading in as it grows a little into place,
// a moment's hold, then the whole splash fades into the app, about 1 s in all (Motion.qml's splash times: a plain
// short fade at Reduced, nothing at Off). The ground covers the app from the first frame, so nothing underneath
// shows before the icon. Once per launch; any press, tap or click skips it (Main.qml calls finish()): it stops
// taking input at once and fades away quickly. Everything loads underneath while it plays.
//
// app (required): Main.qml's window. kit (required): Main.qml's kit (colors and motion, see COMPONENTS.md). shot
// (required): Main.qml's screenshot settings (pictures show it only on the splash pages).
Item {
    id: splash
    required property var app
    required property var kit
    required property var shot
    readonly property var c: kit.c
    readonly property Motion mo: kit.motion
    // For the build's checks.
    property alias splashIcon: splashIcon
    objectName: "splash"
    anchors.fill: parent
    z: 90000
    property bool shown: (!shot.on || shot.page.indexOf("splash") === 0) && !app.splashDone
    // Still drawn while it fades into the app after it is done (it takes no input then).
    visible: shown || away.running
    readonly property int inMs: mo.splashIn
    readonly property int holdMs: mo.splashHold
    readonly property int outMs: mo.splashOut
    property real started: 0
    function finish(why) {
        if (!shown) return
        // Skipped: the rest of it goes quickly. Played through: the fade out is the end of it.
        if (why !== "played") { splashAnim.stop(); if (mo.on) away.restart() }
        app.splashDone = true
        app.note("app", "Splash done (" + why + ") after " + Math.round(Date.now() - started) + " ms")
    }
    // Animations off: no splash at all.
    Component.onCompleted: { started = Date.now(); if (shown && !mo.on) finish("animations off"); else if (shown) { splashAnim.start(); app.note("app", "Splash: start") } }
    Rectangle { anchors.fill: parent; color: c.ground }
    Image {
        id: splashIcon
        anchors.centerIn: parent
        width: Math.min(parent.width, parent.height) * 0.28; height: width
        source: app.iconBig
        sourceSize.width: 512
        smooth: true; mipmap: true
        opacity: 0
        scale: mo.splashFrom
    }
    SequentialAnimation {
        id: splashAnim
        ParallelAnimation {
            NumberAnimation { target: splashIcon; properties: "opacity"; from: 0; to: 1; duration: splash.inMs; easing.type: mo.ease }
            NumberAnimation { target: splashIcon; properties: "scale"; from: mo.splashFrom; to: 1; duration: splash.inMs; easing.type: mo.glide }
        }
        PauseAnimation { duration: splash.holdMs }
        // Into the app: the ground and the icon fade together, the icon growing on a little as it goes.
        ParallelAnimation {
            NumberAnimation { target: splash; properties: "opacity"; to: 0; duration: splash.outMs; easing.type: mo.ease }
            NumberAnimation { target: splashIcon; properties: "scale"; to: mo.splashTo; duration: splash.outMs; easing.type: mo.ease }
        }
        ScriptAction { script: splash.finish("played") }
    }
    NumberAnimation { id: away; target: splash; properties: "opacity"; to: 0; duration: mo.quick; easing.type: mo.ease }
    // A tap or click skips it and goes nowhere else.
    MouseArea {  // overlay-check: the splash takes every press while it plays
        enabled: splash.shown
        anchors.fill: parent
        acceptedButtons: Qt.AllButtons
        preventStealing: true
        onWheel: (w) => { w.accepted = true; splash.finish("wheel") }
        onPressed: { app.coverPress(100000); splash.finish("tap") }
        onReleased: app.coverEnd()
        onCanceled: app.coverEnd()
    }
}
