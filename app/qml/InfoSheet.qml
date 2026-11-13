import QtQuick

// A plain sheet: a title, a few sentences, numbered steps or a list of notes, and one or two buttons. Used for the
// no-browser warning, the browser helper's setup steps, a mod's install notes and "Add this file?" for a download
// the watcher could only guess. A presses the highlighted button, B the cancel one (SheetButtons); Main.qml handles
// the keys. When the words don't fit (a large interface size), they scroll (Up and Down, or the wheel) and the
// buttons stay in view at the bottom.
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
    // A button's item (the build's checks click it through the pointer): buttonItem(i), below.
    readonly property bool scrolls: card.scrolls
    // Scrolls the words by a step (d = 1 down, -1 up). False when they are already at that end.
    function scrollBy(d) { return card.scrollBy(d) }
    onVisibleChanged: if (visible) card.toTop()

    Scrim { ui: sh.ui }
    DialogCard {
        id: card
        ui: sh.ui
        maxWidth: 660
        title: sh.title
        buttons: (sh.buttons || []).map(b => b.label)
        focusIndex: sh.focusIndex
        mainIndex: 0
        cancelIndex: (sh.buttons || []).findIndex(b => b.key === "b")
        onActivated: (i) => sh.activated(i)
        onHeld: (i, down) => sh.held(i, down)
        Text {
            visible: sh.text !== ""
            width: parent.width
            wrapMode: Text.WordWrap
            text: sh.text
            color: sh.ui.c.soft
            font.family: sh.ui.bodyFont; font.pixelSize: sh.ui.fpx(15)
            lineHeight: 1.08
        }
        Repeater {
            model: sh.steps
            Row {
                required property int index
                required property var modelData
                width: card.bodyColumn.width
                spacing: sh.ui.px(12)
                Rectangle {
                    width: sh.ui.px(26); height: width; radius: width / 2
                    color: sh.ui.c.raised; border.color: sh.ui.c.accent; border.width: Math.max(1, sh.ui.px(1))
                    Text { anchors.centerIn: parent; text: index + 1; color: sh.ui.c.accentText; font.family: sh.ui.bodyFont; font.pixelSize: sh.ui.fpx(14); font.bold: true }
                }
                Text {
                    width: parent.width - sh.ui.px(38)
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
                width: card.bodyColumn.width
                spacing: sh.ui.px(10)
                Rectangle { y: sh.ui.px(8); width: sh.ui.px(6); height: width; radius: width / 2; color: sh.ui.c.accent }
                Text {
                    width: parent.width - sh.ui.px(16)
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
    // The cancel button left the row (no room): B still does it, and the D-pad passes it.
    readonly property bool cancelHidden: card.buttonRow.cancelHidden
    function buttonItem(i) { return card.buttonRow.itemAt(i) }
}
