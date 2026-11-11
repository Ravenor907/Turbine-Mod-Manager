import QtQuick
import QtQuick.Shapes
import Turbine

// The controller pointer, drawn over everything (sheets included): the arrow where Turbine's pointer is, and the
// outline on the button or row under it (Main.qml's hoverItem and hoverBox). It takes no input: the pointer's clicks
// and wheel steps reach the window as real mouse events.
//
// app (required): Main.qml's window. kit (required): Main.qml's kit (colors, see COMPONENTS.md).
Item {
    required property var app
    required property var kit
    readonly property var c: kit.c
    anchors.fill: parent
    z: 10000
    visible: Turbine.pointerShown
    // On the Nexus page the pointer never hides: still for 3 seconds, it dims.
    opacity: app.page === "browser" && app.pointerStill ? 0.45 : 1
    enabled: false
    Rectangle {
        visible: app.hoverItem !== null && app.hoverBox.width > 0
        x: app.hoverBox.x - app.px(2); y: app.hoverBox.y - app.px(2)
        width: app.hoverBox.width + app.px(4); height: app.hoverBox.height + app.px(4)
        radius: app.px(7)
        color: Qt.rgba(c.accent.r, c.accent.g, c.accent.b, 0.10)
        border.width: Math.max(1, app.px(1.5))
        border.color: c.accentHi
    }
    // The arrow: brass with a dark edge, its tip on the spot that gets clicked, about 24 design pixels tall.
    Shape {
        x: Turbine.pointerX
        y: Turbine.pointerY
        width: app.px(24); height: app.px(24)
        preferredRendererType: Shape.CurveRenderer
        ShapePath {
            strokeColor: "#0B0A08"
            strokeWidth: Math.max(1.5, app.px(1.6))
            joinStyle: ShapePath.RoundJoin
            fillColor: "#D8AE5E"
            scale: Qt.size(app.px(24) / 24, app.px(24) / 24)
            PathSvg { path: "M1 1 L1 20 L6 15.5 L9.6 23 L13 21.4 L9.5 14 L16 14 Z" }
        }
    }
}
