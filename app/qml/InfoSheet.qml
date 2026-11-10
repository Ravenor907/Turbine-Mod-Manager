import QtQuick

// A plain sheet: a title, a few sentences, numbered steps or a list of notes, and one or two buttons. Used for the
// no-browser warning, the browser helper's setup steps, a mod's install notes and "Add this file?" for a download
// the watcher could only guess. A presses the highlighted button, B the cancel one, Start the
// first (SheetButtons); Main.qml handles the keys. When the words don't fit (a large interface size), they scroll (Up and Down, or the
// wheel) and the buttons stay in view at the bottom.
Overlay {
    id: sh
    onOutsideTapped: sh.closed()
    required property var ui
    motion: ui.motion
    anchors.fill: parent

    property string title: ""
    property string text: ""
    property var steps: []                // numbered, in order
    property var notes: []                // bulleted, no order
    property string footnote: ""          // a muted line under the steps (a folder name, a warning)
    property var buttons: []              // [{key: "a" or "b", label}]
    property int focusIndex: 0

    signal activated(int index)
    signal held(int index, bool down)
    signal closed()
    // A button's item (the build's checks click it through the pointer).
    function buttonItem(i) { return btnFlow.itemAt(i) }

    readonly property bool scrolls: flick.contentHeight > flick.height + 1
    // Scrolls the words by a step (d = 1 down, -1 up). False when they are already at that end.
    function scrollBy(d) {
        const before = flick.contentY
        flick.contentY = Math.max(0, Math.min(flick.contentHeight - flick.height, flick.contentY + d * sh.ui.px(80)))
        return flick.contentY !== before
    }
    onVisibleChanged: if (visible) flick.contentY = 0

    Rectangle { anchors.fill: parent; color: "#C0000000" }
    Rectangle {
        anchors.centerIn: parent
        width: Math.min(parent.width - sh.ui.px(40), sh.ui.px(660))
        // Clear of the legend at the bottom (px 44) and never past the top.
        height: Math.min(parent.height - sh.ui.px(110), col.implicitHeight + btnFlow.implicitHeight + sh.ui.px(52))
        anchors.verticalCenterOffset: -sh.ui.px(20)
        radius: sh.ui.px(10)
        color: sh.ui.c.panel
        border.color: sh.ui.c.line
        clip: true
        CardGuard {}
        Rectangle { width: parent.width; height: sh.ui.px(3); color: sh.ui.c.accent; radius: sh.ui.px(2) }
        Flickable {
            id: flick
            x: sh.ui.px(22)
            width: parent.width - sh.ui.px(44)
            anchors.bottom: btnFlow.top
            anchors.bottomMargin: sh.ui.px(8)
            anchors.top: parent.top
            anchors.topMargin: sh.ui.px(20)
            clip: true
            contentWidth: width
            contentHeight: col.implicitHeight
            boundsBehavior: Flickable.StopAtBounds
        Column {
            id: col
            width: flick.width
            spacing: sh.ui.px(10)
            Text {
                width: parent.width
                wrapMode: Text.WordWrap
                text: sh.title
                color: sh.ui.c.text
                font.family: sh.ui.titleFont; font.pixelSize: sh.ui.fpx(18); font.bold: true
            }
            Text {
                visible: sh.text !== ""
                width: parent.width
                wrapMode: Text.WordWrap
                text: sh.text
                color: sh.ui.c.soft
                font.family: sh.ui.bodyFont; font.pixelSize: sh.ui.fpx(15)
            }
            Repeater {
                model: sh.steps
                Row {
                    required property int index
                    required property var modelData
                    width: col.width
                    spacing: sh.ui.px(12)
                    Rectangle {
                        width: sh.ui.px(28); height: width; radius: width / 2
                        color: sh.ui.c.raised; border.color: sh.ui.c.accent; border.width: Math.max(1, sh.ui.px(1))
                        Text { anchors.centerIn: parent; text: index + 1; color: sh.ui.c.accentText; font.family: sh.ui.bodyFont; font.pixelSize: sh.ui.fpx(15); font.bold: true }
                    }
                    Text {
                        width: parent.width - sh.ui.px(40)
                        anchors.verticalCenter: parent.verticalCenter
                        wrapMode: Text.WordWrap
                        text: modelData
                        color: sh.ui.c.text
                        font.family: sh.ui.bodyFont; font.pixelSize: sh.ui.fpx(15)
                    }
                }
            }
            Repeater {
                model: sh.notes
                Row {
                    required property var modelData
                    width: col.width
                    spacing: sh.ui.px(10)
                    Rectangle { y: sh.ui.px(8); width: sh.ui.px(7); height: width; radius: width / 2; color: sh.ui.c.accent }
                    Text {
                        width: parent.width - sh.ui.px(17)
                        wrapMode: Text.WordWrap
                        text: modelData
                        color: sh.ui.c.text
                        font.family: sh.ui.bodyFont; font.pixelSize: sh.ui.fpx(15)
                    }
                }
            }
            Text {
                visible: sh.footnote !== ""
                width: parent.width
                // Long folder names still break; words don't.
                wrapMode: Text.Wrap
                text: sh.footnote
                color: sh.ui.c.muted
                font.family: sh.ui.bodyFont; font.pixelSize: sh.ui.fpx(14)
            }
        }
        }
        // More words below: a fade over the last line.
        Rectangle {
            visible: sh.scrolls && flick.contentY < flick.contentHeight - flick.height - 1
            anchors.left: flick.left; anchors.right: flick.right; anchors.bottom: flick.bottom
            height: sh.ui.px(28)
            gradient: Gradient {
                GradientStop { position: 0; color: "transparent" }
                GradientStop { position: 1; color: sh.ui.c.panel }
            }
        }
            SheetButtons {
                id: btnFlow
                ui: sh.ui
                x: sh.ui.px(22)
                anchors.bottom: parent.bottom
                anchors.bottomMargin: sh.ui.px(18)
                width: parent.width - sh.ui.px(44)
                labels: sh.buttons.map(b => b.label)
                focusIndex: sh.focusIndex
                mainIndex: 0
                onActivated: (i) => sh.activated(i)
                onHeld: (i, down) => sh.held(i, down)
            }
    }
}
