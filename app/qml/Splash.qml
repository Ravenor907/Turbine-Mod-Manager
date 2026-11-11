import QtQuick

// The start-up splash: Turbine's icon in the middle, zooming in as it fades in, then out as it fades out, about 1.2 s
// in all (Motion.qml's splash times: a plain short fade at Reduced, nothing at Off). Once per launch; any press, tap
// or click skips it (Main.qml calls finish()). Everything loads underneath while it plays.
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
    visible: shown
    readonly property int inMs: mo.splashIn
    readonly property int holdMs: mo.splashHold
    readonly property int outMs: mo.splashOut
    property real started: 0
    function finish(why) {
        if (!shown) return
        app.splashDone = true
        app.note("app", "Splash done (" + why + ") after " + Math.round(Date.now() - started) + " ms")
    }
    // Animations off: no splash at all.
    Component.onCompleted: { started = Date.now(); if (shown && !mo.on) finish("animations off"); else if (shown) { splashAnim.start(); app.note("app", "Splash: start") } }
    Rectangle { anchors.fill: parent; color: c.ground; opacity: splashIcon.opacity }
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
            NumberAnimation { target: splashIcon; properties: "opacity"; from: 0; to: 1; duration: splash.inMs; easing.type: mo.enter }
            NumberAnimation { target: splashIcon; properties: "scale"; from: mo.splashFrom; to: 1; duration: splash.inMs; easing.type: mo.enter }
        }
        PauseAnimation { duration: splash.holdMs }
        ParallelAnimation {
            NumberAnimation { target: splashIcon; properties: "opacity"; to: 0; duration: splash.outMs; easing.type: mo.leave }
            NumberAnimation { target: splashIcon; properties: "scale"; to: mo.splashTo; duration: splash.outMs; easing.type: mo.leave }
        }
        ScriptAction { script: splash.finish("played") }
    }
    // A tap or click skips it and goes nowhere else.
    MouseArea {  // overlay-check: the splash takes every press while it plays
        anchors.fill: parent
        acceptedButtons: Qt.AllButtons
        preventStealing: true
        onWheel: (w) => { w.accepted = true; splash.finish("wheel") }
        onPressed: { app.coverPress(100000); splash.finish("tap") }
        onReleased: app.coverEnd()
        onCanceled: app.coverEnd()
    }
}
