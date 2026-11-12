import QtQuick
import QtQuick.Shapes

// Scroll cues for a list, grid or sheet that scrolls: where there is more to see, its edge fades into the color it
// sits on, and a small chevron at the bottom says there is more below. Declare it anywhere; it puts itself on the view
// (over the rows, without scrolling with them), covers it and takes no input, so nothing under it is ever hidden behind
// a hint.
//
// ui (required): Main.qml's kit. view (required): the Flickable or ListView. color: what the view sits on.
// size: how tall each fade is. chevron: the chevron at the bottom. The cues fade in and out (Motion's quick).
Item {
    id: sf
    required property var ui
    required property Flickable view
    property color color: ui.c.panel
    property real size: ui.px(26)
    property bool chevron: true
    // The chevron just under the view (in the gap before a dialog's buttons) rather than over its last line.
    property bool chevronBelow: false
    readonly property bool longer: view.visible && view.contentHeight > view.height + 1
    readonly property bool above: longer && !view.atYBeginning
    readonly property bool below: longer && !view.atYEnd
    // With the chevron under the view it stays beside the view (its sibling), so the view's clip doesn't cut it.
    Component.onCompleted: if (!chevronBelow && view && parent !== view) parent = view
    anchors.fill: chevronBelow ? view : parent
    z: 10
    enabled: false
    component Cue: NumberAnimation { duration: sf.ui.motion.quick; easing.type: sf.ui.motion.ease }
    Rectangle {
        width: parent.width; height: sf.size
        opacity: sf.above ? 1 : 0
        visible: opacity > 0
        Behavior on opacity { enabled: sf.ui.motion.on; Cue {} }
        gradient: Gradient {
            GradientStop { position: 0; color: sf.color }
            GradientStop { position: 1; color: Qt.rgba(sf.color.r, sf.color.g, sf.color.b, 0) }
        }
    }
    Rectangle {
        anchors.bottom: parent.bottom
        width: parent.width; height: sf.size
        opacity: sf.below ? 1 : 0
        visible: opacity > 0
        Behavior on opacity { enabled: sf.ui.motion.on; Cue {} }
        gradient: Gradient {
            GradientStop { position: 0; color: Qt.rgba(sf.color.r, sf.color.g, sf.color.b, 0) }
            GradientStop { position: 0.7; color: sf.color }
            GradientStop { position: 1; color: sf.color }
        }
    }
    // More below.
    Shape {
        opacity: sf.below && sf.chevron ? 1 : 0
        visible: opacity > 0
        Behavior on opacity { enabled: sf.ui.motion.on; Cue {} }
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: sf.chevronBelow ? -height - sf.ui.px(1) : sf.ui.px(1)
        width: sf.ui.px(16); height: sf.ui.px(8)
        preferredRendererType: Shape.CurveRenderer
        ShapePath {
            strokeColor: sf.ui.c.soft
            strokeWidth: Math.max(1.5, sf.ui.px(2))
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            startX: sf.ui.px(2); startY: sf.ui.px(1)
            PathLine { x: sf.ui.px(8); y: sf.ui.px(7) }
            PathLine { x: sf.ui.px(14); y: sf.ui.px(1) }
        }
    }
}
