import QtQuick

// The one dialog every centred sheet with buttons uses (COMPONENTS.md): a Card with its title (a short word on the
// right, a line under it), the body, and the buttons in a footer under the body. It sizes to its content, up to the
// space it has (the sheet layer ends above the bottom bar); then the body scrolls (ScrollFade shows there is more)
// and the footer stays where it is. The items written inside go into the body, one under the other, as wide as the
// card's text.
//
// ui (required): Main.qml's kit. buttons: the buttons' words; focusIndex, mainIndex, cancelIndex as SheetButtons.
// note: a muted line under the buttons. maxWidth: the widest it gets, in design pixels.
Card {
    id: dc
    property string title: ""
    property string subtitle: ""
    property string side: ""               // a short word on the title's right ("12 mods")
    property real maxWidth: 620
    property var buttons: []
    property int focusIndex: 0
    property int mainIndex: 0
    property int cancelIndex: -1
    property string note: ""
    default property alias body: bodyCol.data
    readonly property alias flick: flick
    readonly property alias bodyColumn: bodyCol
    readonly property alias buttonRow: buttonRow
    readonly property bool scrolls: flick.contentHeight > flick.height + 1
    signal activated(int index)
    signal held(int index, bool down)

    // Scrolls the body by a step (d = 1 down, -1 up). False when it is already at that end.
    function scrollBy(d) {
        const before = flick.contentY
        flick.contentY = Math.max(0, Math.min(flick.contentHeight - flick.height, flick.contentY + d * ui.px(80)))
        return flick.contentY !== before
    }
    // Brings an item of the body fully into view.
    function follow(it) { ui.follow(flick, bodyCol, it) }
    function toTop() { flick.contentY = 0 }

    readonly property real pad: ui.px(22)
    readonly property real most: parent ? parent.height - ui.px(24) : 0
    anchors.centerIn: parent
    width: parent ? Math.min(parent.width - ui.px(32), ui.px(maxWidth)) : 0
    height: Math.min(most, head.height + bodyCol.implicitHeight + foot.height + pad * 2 + (bodyCol.implicitHeight > 0 ? ui.px(14) + ui.px(6) : 0))

    CardGuard {}

    Column {
        id: head
        x: dc.pad; y: dc.pad
        width: parent.width - dc.pad * 2
        spacing: dc.ui.px(4)
        Item {
            width: parent.width
            height: titleText.implicitHeight
            Text {
                id: titleText
                width: parent.width - (sideText.visible ? sideText.implicitWidth + dc.ui.px(12) : 0)
                text: dc.title
                wrapMode: Text.WordWrap
                maximumLineCount: 2
                elide: Text.ElideRight
                color: dc.ui.c.text
                font.family: dc.ui.titleFont; font.pixelSize: dc.ui.fpx(19); font.bold: true
            }
            Text {
                id: sideText
                visible: dc.side !== ""
                anchors.right: parent.right
                anchors.baseline: titleText.baseline
                text: dc.side
                color: dc.ui.c.accentText
                font.family: dc.ui.bodyFont; font.pixelSize: dc.ui.fpx(15); font.bold: true
            }
        }
        Text {
            visible: dc.subtitle !== ""
            width: parent.width
            text: dc.subtitle
            wrapMode: Text.WordWrap
            color: dc.ui.c.muted
            font.family: dc.ui.bodyFont; font.pixelSize: dc.ui.fpx(14)
        }
    }
    Flickable {
        id: flick
        // The body scrolls on purpose when it doesn't fit, with ScrollFade's fades and chevron as the cue.
        property string layoutOk: "scroll"
        x: dc.pad
        width: parent.width - dc.pad * 2
        anchors.top: head.bottom
        anchors.topMargin: bodyCol.implicitHeight > 0 ? dc.ui.px(14) : 0
        anchors.bottom: foot.top
        anchors.bottomMargin: bodyCol.implicitHeight > 0 ? dc.ui.px(6) : 0
        clip: true
        contentWidth: width
        contentHeight: bodyCol.implicitHeight
        boundsBehavior: Flickable.StopAtBounds
        Column {
            id: bodyCol
            width: flick.width
            spacing: dc.ui.px(10)
        }
    }
    ScrollFade { ui: dc.ui; view: flick; color: dc.fill; chevronBelow: true }
    Column {
        id: foot
        x: dc.pad
        anchors.bottom: parent.bottom
        anchors.bottomMargin: dc.pad
        width: parent.width - dc.pad * 2
        spacing: dc.ui.px(10)
        topPadding: dc.ui.px(10)
        SheetButtons {
            id: buttonRow
            visible: dc.buttons.length > 0
            ui: dc.ui
            width: parent.width
            labels: dc.buttons
            focusIndex: dc.focusIndex
            mainIndex: dc.mainIndex
            cancelIndex: dc.cancelIndex
            onActivated: (i) => dc.activated(i)
            onHeld: (i, down) => dc.held(i, down)
        }
        Text {
            visible: dc.note !== ""
            width: parent.width
            text: dc.note
            wrapMode: Text.WordWrap
            color: dc.ui.c.muted
            font.family: dc.ui.bodyFont; font.pixelSize: dc.ui.fpx(14)
        }
    }
}
