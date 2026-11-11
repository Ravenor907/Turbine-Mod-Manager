import QtQuick
import QtQuick.Shapes

// One row of GameMenu and ModMenu: an icon tile, the label and a detail line, and › when it leads further.
Rectangle {
    id: entry
    required property var ui
    property string label: ""
    property string detail: ""
    property string icon: ""              // a name from icons below, or SVG path data on a 24 x 24 grid
    readonly property string iconData: icon ? (icons[icon] || icon) : ""
    property bool available: true
    property bool danger: false
    property bool more: false
    property bool focused: false
    property color dangerColor: ui.c.warn
    property bool iconShown: true
    signal clicked()
    signal held(bool down)                // a finger or the pointer pressed down on it (true) and let go (false)

    readonly property var icons: ({
        update: "M12 4v11M7 10l5 5 5-5M5 20h14",
        power: "M12 3v8M6.5 6.5a8 8 0 1 0 11 0",
        overlap: "M5 3h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z M11 9h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2h-8a2 2 0 0 1-2-2v-8a2 2 0 0 1 2-2z",
        files: "M14 3H6a1 1 0 0 0-1 1v16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V8z M14 3v5h5",
        move: "M8 4v16M4 8l4-4 4 4M16 20V4M12 16l4 4 4-4",
        remove: "M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13",
        play: "M7 4l13 8-13 8z",
        back: "M15 5l-7 7 7 7",
        restore: "M4 4v6h6 M5 15a8 8 0 1 0 2-8.5L4 10",
        profile: "M12 4a4 4 0 1 1 0 8 4 4 0 0 1 0-8z M4 21c1-4 4-6 8-6s7 2 8 6",
        info: "M12 3a9 9 0 1 1 0 18 9 9 0 0 1 0-18z M12 11v6 M12 7.5v0.5"
    })

    implicitHeight: Math.max(ui.px(48), entryText.implicitHeight + ui.px(14))
    radius: ui.px(6)
    color: "transparent"

    // Focus: warm gradient with a brass border.
    Rectangle {
        visible: entry.focused
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
    TapHandler { onTapped: entry.clicked(); onPressedChanged: entry.held(pressed) }

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
