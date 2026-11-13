import QtQuick
import QtQuick.Shapes

// One row of GameMenu and ModMenu: an icon tile, the label and a detail line, and › when it leads further.
Rectangle {
    id: entry
    required property var ui
    property string label: ""
    property string detail: ""
    property string icon: ""              // a name from the kit's icons, or SVG path data on a 24 x 24 grid
    // The row's icon: the one its words call for (kit.iconFor), else the one it was given.
    readonly property string iconData: ui.iconPath(ui.iconFor(icon, label))
    property bool available: true
    property bool danger: false
    property bool more: false
    property bool focused: false
    property color dangerColor: ui.c.warn
    property bool iconShown: true
    signal clicked()
    signal held(bool down)                // a finger or the pointer pressed down on it (true) and let go (false)

    Accessible.role: Accessible.MenuItem
    Accessible.name: label + (available ? "" : ", not available") + (detail !== "" ? ", " + detail : "")
    Accessible.focused: focused
    implicitHeight: Math.max(ui.px(48), entryText.implicitHeight + ui.px(14))
    radius: ui.px(6)
    color: "transparent"
    // Pressed by a finger or the pointer: it gives a little (Full).
    scale: entryTap.pressed ? ui.motion.pressScale : 1
    Behavior on scale { enabled: entry.ui.motion.moves; NumberAnimation { duration: entry.ui.motion.moveQuick; easing.type: entry.ui.motion.ease } }

    // Focus: warm gradient with a brass border, fading in as the focus arrives and out as it leaves.
    Rectangle {
        opacity: entry.focused ? 1 : 0
        visible: opacity > 0
        Behavior on opacity { enabled: entry.ui.motion.on; NumberAnimation { duration: entry.ui.motion.quick; easing.type: entry.ui.motion.ease } }
        anchors.fill: parent
        radius: entry.radius
        border.width: Math.max(1, entry.ui.px(1.5))
        border.color: entry.ui.c.accent
        gradient: Gradient {
            orientation: Gradient.Horizontal
            GradientStop { position: 0; color: entry.ui.c.focusFill }
            GradientStop { position: 1; color: Qt.tint(entry.ui.c.panel, Qt.rgba(entry.ui.c.focusFill.r, entry.ui.c.focusFill.g, entry.ui.c.focusFill.b, 0.5)) }
        }
    }

    HoverHandler { onHoveredChanged: entry.ui.hover(parent, hovered) }
    TapHandler { id: entryTap; onTapped: entry.clicked(); onPressedChanged: entry.held(pressed) }

    Rectangle {
        id: entryIcon
        x: entry.ui.px(12)
        anchors.verticalCenter: parent.verticalCenter
        width: entry.ui.px(28)
        height: width
        radius: entry.ui.px(6)
        color: entry.focused ? Qt.tint(entry.ui.c.card, Qt.rgba(entry.ui.c.accent.r, entry.ui.c.accent.g, entry.ui.c.accent.b, 0.28)) : entry.ui.c.raised
        Shape {
            visible: entry.iconData !== "" && entry.iconShown
            anchors.centerIn: parent
            width: entry.ui.px(18)
            height: width
            preferredRendererType: Shape.CurveRenderer
            ShapePath {
                strokeColor: entry.focused ? entry.ui.c.accentHi : entry.ui.c.soft
                strokeWidth: 2
                fillColor: "transparent"
                capStyle: ShapePath.RoundCap
                joinStyle: ShapePath.RoundJoin
                scale: Qt.size(entry.ui.px(18) / 24, entry.ui.px(18) / 24)
                PathSvg { path: entry.iconData }
            }
        }
    }

    Item {
        id: entryEnd
        anchors.right: parent.right
        anchors.rightMargin: entry.ui.px(12)
        anchors.verticalCenter: parent.verticalCenter
        width: entry.more && !entry.focused ? entryArrow.implicitWidth : 0
        height: entry.ui.px(24)
        // No A cap on the highlighted row: the legend says A Choose (never printed twice).
        Text {
            id: entryArrow
            visible: !entry.focused && entry.more
            anchors.verticalCenter: parent.verticalCenter
            text: "›"
            color: entry.ui.c.muted
            font.family: entry.ui.bodyFont
            font.pixelSize: entry.ui.fpx(18)
        }
    }

    Column {
        id: entryText
        anchors.left: entryIcon.right
        anchors.leftMargin: entry.ui.px(12)
        anchors.right: entryEnd.left
        anchors.rightMargin: entry.ui.px(8)
        anchors.verticalCenter: parent.verticalCenter
        spacing: entry.ui.px(1)
        Text {
            width: parent.width
            text: entry.label
            elide: Text.ElideRight
            color: !entry.available ? (entry.focused ? entry.ui.c.soft : entry.ui.c.muted) : entry.danger ? entry.dangerColor : entry.ui.c.text
            font.family: entry.ui.bodyFont
            font.pixelSize: entry.ui.fpx(16)
            font.weight: Font.DemiBold
        }
        Text {
            visible: text !== ""
            width: parent.width
            text: entry.detail
            wrapMode: Text.WordWrap
            lineHeight: 1.1
            color: entry.focused ? entry.ui.c.soft : entry.ui.c.muted
            font.family: entry.ui.bodyFont
            font.pixelSize: entry.ui.fpx(14)
        }
    }
}
