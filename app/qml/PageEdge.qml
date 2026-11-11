import QtQuick
import QtQuick.Shapes

// One page edge on Get mods: the page to the left or the page to the right, a button for a click or a finger (at
// least 44 px wide). It slides with the page while Get mods turns.
//
// win (required): Main.qml's window. label: the page's name. dir: -1 the page to the left, 1 the page to the right.
Rectangle {
    id: edge
    required property var win
    property string label: ""
    property int dir: -1
    width: win.tpx(34); height: parent.height
    z: win.turnTo === 1 ? 3 : 0
    transform: Translate { x: edge.win.turnTo === 1 ? edge.win.turnX : 0 }
    color: edgeHover.hovered ? win.colors.focusFill : win.colors.panel
    Rectangle { x: edge.dir < 0 ? parent.width - width : 0; width: Math.max(1, edge.win.px(1)); height: parent.height; color: edge.win.colors.line }
    Text {
        anchors.centerIn: parent
        rotation: edge.dir < 0 ? -90 : 90
        text: edge.label
        color: edge.dir < 0 ? edge.win.colors.accentText : edge.win.colors.soft
        font.family: edge.win.bodyFont; font.pixelSize: edge.win.fpx(14); font.weight: Font.DemiBold
    }
    // The arrow, pointing off the page.
    Shape {
        width: edge.win.px(14); height: edge.win.px(18)
        anchors.horizontalCenter: parent.horizontalCenter
        y: parent.height * 0.76
        preferredRendererType: Shape.CurveRenderer
        ShapePath {
            strokeWidth: 0; strokeColor: "transparent"
            fillColor: edge.dir < 0 ? edge.win.colors.accentText : edge.win.colors.soft
            startX: edge.dir < 0 ? edge.win.px(14) : 0; startY: 0
            PathLine { x: edge.dir < 0 ? 0 : edge.win.px(14); y: edge.win.px(9) }
            PathLine { x: edge.dir < 0 ? edge.win.px(14) : 0; y: edge.win.px(18) }
            PathLine { x: edge.dir < 0 ? edge.win.px(14) : 0; y: 0 }
        }
    }
    Accessible.role: Accessible.Button
    Accessible.name: edge.label + (edge.dir < 0 ? ", the page to the left" : ", the page to the right")
    HoverHandler { id: edgeHover }
    TapHandler { onTapped: { if (!edge.win.gate(0, "page edge")) return; edge.win.note("touch", "Page edge: " + edge.label); edge.win.turnPage(edge.dir) } }
}
