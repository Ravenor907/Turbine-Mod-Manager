import QtQuick
import QtQuick.Shapes

// A line icon from SVG path data drawn on a 24 x 24 grid, at 22 design pixels.
//
// win (required): Main.qml's window. d: the path data. tint: its color.
Shape {
    id: icon
    required property var win
    property string d: ""
    property color tint: win.colors.muted
    width: win.px(22); height: win.px(22)
    preferredRendererType: Shape.CurveRenderer
    ShapePath {
        strokeColor: icon.tint
        strokeWidth: 2
        fillColor: "transparent"
        capStyle: ShapePath.RoundCap
        joinStyle: ShapePath.RoundJoin
        scale: Qt.size(icon.win.px(22) / 24, icon.win.px(22) / 24)
        PathSvg { path: icon.d }
    }
}
