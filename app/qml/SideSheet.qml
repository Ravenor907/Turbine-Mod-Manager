import QtQuick

// The one side sheet look (the mod menu, Add from Downloads, Other files, Loadouts, a plugin's details): flush with
// the window's right edge, from the top down to the bottom bar, with a line and a soft shadow on its left. Nothing
// of it is cut by the window's edges. The sheet's own items go inside.
//
// ui (required): Main.qml's kit. showFade, showMove: 0 to 1 as its Overlay brings it in (a fade, and a slide in from
// the right, Motion's sheetShift).
Rectangle {
    id: side
    required property var ui
    property real showFade: 1
    property real showMove: 1
    opacity: showFade
    transform: Translate { x: (1 - side.showMove) * side.ui.px(side.ui.motion.sheetShift) }
    anchors.right: parent ? parent.right : undefined
    anchors.top: parent ? parent.top : undefined
    color: ui.c.panel
    Rectangle {
        anchors.right: parent.left
        width: side.ui.px(18); height: parent.height
        gradient: Gradient {
            orientation: Gradient.Horizontal
            GradientStop { position: 0; color: "transparent" }
            GradientStop { position: 1; color: Qt.rgba(0, 0, 0, 0.35) }
        }
    }
    Rectangle { width: Math.max(1, side.ui.px(1)); height: parent.height; color: side.ui.c.line }
}
