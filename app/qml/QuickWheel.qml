import QtQuick

// The quick wheel (View): eight fixed places around a circle, the same on every screen. Slots with nothing to do
// on this screen are dimmed, never moved. The highlighted one has the brass ring; in Labs the middle says Report.
// Main.qml owns the slots ({label, ok}) and the keys; a tap on a slot does it, a tap outside closes.
Item {
    id: qw
    required property var ui
    property var slots: []
    property int focusIndex: -1
    property bool labs: false
    property string viewGlyph: "VIEW"
    signal activated(int index)
    signal closed()

    anchors.fill: parent
    z: 70

    Rectangle { anchors.fill: parent; color: "#B0000000" }
    MouseArea { anchors.fill: parent; onClicked: qw.closed() }

    Item {
        id: ring
        readonly property real r: Math.min(qw.width, qw.height) * 0.30
        width: r * 2 + qw.ui.px(150); height: r * 2 + qw.ui.px(60)
        anchors.centerIn: parent
        Rectangle {
            anchors.centerIn: parent
            width: ring.r * 2 + qw.ui.px(40); height: width; radius: width / 2
            color: qw.ui.c.panel
            border.color: qw.ui.c.line; border.width: Math.max(1, qw.ui.px(2))
            MouseArea { anchors.fill: parent }
        }
        // The middle: View's name (Labs: the report shot).
        Rectangle {
            anchors.centerIn: parent
            width: qw.ui.px(88); height: width; radius: width / 2
            color: qw.focusIndex < 0 ? qw.ui.c.raised : qw.ui.c.card
            border.color: qw.focusIndex < 0 && qw.labs ? qw.ui.c.accentHi : qw.ui.c.accentLo
            border.width: Math.max(1, qw.ui.px(2))
            Column {
                anchors.centerIn: parent
                Text { anchors.horizontalCenter: parent.horizontalCenter; text: qw.viewGlyph; color: qw.ui.c.text; font.family: qw.ui.bodyFont; font.pixelSize: qw.ui.px(14); font.bold: true }
                Text { visible: qw.labs; anchors.horizontalCenter: parent.horizontalCenter; text: "Report"; color: qw.ui.c.soft; font.family: qw.ui.bodyFont; font.pixelSize: qw.ui.px(12) }
            }
        }
        Repeater {
            model: qw.slots
            Rectangle {
                id: sl
                required property int index
                required property var modelData
                readonly property bool foc: index === qw.focusIndex
                readonly property real a: (index * 45 - 90) * Math.PI / 180
                width: Math.max(qw.ui.px(110), slText.implicitWidth + qw.ui.px(24)); height: qw.ui.px(46)
                radius: qw.ui.px(8)
                x: ring.width / 2 + Math.cos(a) * ring.r - width / 2
                y: ring.height / 2 + Math.sin(a) * ring.r - height / 2
                opacity: modelData.ok ? 1 : 0.38
                color: foc ? qw.ui.c.focusFill : qw.ui.c.card
                border.color: foc ? qw.ui.c.accentHi : qw.ui.c.line
                border.width: foc ? Math.max(2, qw.ui.px(3)) : Math.max(1, qw.ui.px(1))
                Text { id: slText; anchors.centerIn: parent; text: sl.modelData.label; color: sl.foc ? qw.ui.c.accentText : qw.ui.c.text; font.family: qw.ui.bodyFont; font.pixelSize: qw.ui.px(16); font.weight: Font.DemiBold }
                HoverHandler { onHoveredChanged: qw.ui.hover(sl, hovered) }
                TapHandler { onTapped: qw.activated(sl.index) }
            }
        }
    }
}
