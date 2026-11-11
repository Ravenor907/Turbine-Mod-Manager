import QtQuick

// Settings, About (Labs mode): the scrolling test, 2,000 made-up mod rows to scroll through.
//
// app (required): Main.qml's window. kit (required): Main.qml's kit (colors and motion, see COMPONENTS.md).
ListView {
    id: perfList
    required property var app
    required property var kit
    readonly property var c: kit.c
    readonly property Motion mo: kit.motion
    visible: app.page === "perf"
    anchors.fill: parent
    clip: true
    spacing: app.px(4)
    model: 2000
    currentIndex: app.perfIndex
    highlightMoveDuration: mo.moveFast
    onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
    delegate: Rectangle {
        id: prow
        required property int index
        readonly property bool foc: app.perfIndex === index
        readonly property bool off: index % 7 === 3
        width: perfList.width
        height: app.px(52)
        radius: app.px(6)
        color: foc ? c.focusFill : c.card
        border.color: c.accent
        border.width: foc ? app.px(2) : 0
        Row {
            anchors.verticalCenter: parent.verticalCenter
            x: app.px(12)
            spacing: app.px(12)
            Rectangle {
                width: app.px(38); height: app.px(22); radius: app.px(5)
                anchors.verticalCenter: parent.verticalCenter
                color: prow.off ? c.ground : c.raised
                Label { win: app; anchors.centerIn: parent; text: prow.off ? "OFF" : "ON"; font.pixelSize: app.fpx(14); font.bold: true; color: prow.off ? c.muted : c.accentText }
            }
            Column {
                anchors.verticalCenter: parent.verticalCenter
                Label { win: app; text: "Test mod " + (prow.index + 1); font.pixelSize: app.fpx(16); font.bold: true; color: prow.off ? c.muted : c.text }
                Label { win: app; text: "Row " + (prow.index + 1) + " of 2,000"; font.pixelSize: app.fpx(14); color: c.muted }
            }
        }
    }
}
