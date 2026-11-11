import QtQuick

// "Getting 2 of 3: <name>" at the top of the window while requirements are got one after another, with Skip and Stop
// (also in X options on the Nexus page). Shown while Main.qml's reqStripWords says something.
//
// app (required): Main.qml's window. kit (required): Main.qml's kit (colors, see COMPONENTS.md).
Rectangle {
    required property var app
    required property var kit
    readonly property var c: kit.c
    visible: app.reqStripWords !== ""
    z: 95
    anchors.horizontalCenter: parent.horizontalCenter
    y: app.px(8)
    width: reqStripRow.implicitWidth + app.px(20)
    height: Math.max(app.px(40), app.tpx(34))
    radius: app.px(8)
    color: c.panel
    border.color: c.accent
    border.width: Math.max(1, app.px(1.5))
    Row {
        id: reqStripRow
        anchors.centerIn: parent
        spacing: app.px(12)
        Label { win: app; text: app.reqStripWords; font.pixelSize: app.fpx(15); color: c.text; anchors.verticalCenter: parent.verticalCenter }
        Repeater {
            model: [["Skip", () => app.reqSkip()], ["Stop", () => app.reqStop()]]
            Rectangle {
                required property var modelData
                width: reqBtnText.implicitWidth + app.px(20); height: Math.max(app.px(30), app.tpx(30)); radius: app.px(6)
                anchors.verticalCenter: parent.verticalCenter
                color: c.raised; border.color: c.accentLo; border.width: Math.max(1, app.px(1))
                Label { id: reqBtnText; win: app; anchors.centerIn: parent; text: parent.modelData[0]; font.pixelSize: app.fpx(14); color: c.text }
                Accessible.role: Accessible.Button
                Accessible.name: modelData[0]
                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                TapHandler { onTapped: { app.note("touch", "Requirements strip: " + parent.modelData[0]); parent.modelData[1]() } }
            }
        }
    }
}
