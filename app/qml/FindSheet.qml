import QtQuick

// Y's pop-up (0.57): one place to find. Get mods: a Search field, Sort and Category buttons (a long category list
// ends in More...). My mods: a Find field, Show and Sort buttons, and Select several. The chosen button in each row
// is filled; the focus ring is on the button A would choose. Nothing changes until Start (or Show results) shows the
// results; B (or Close, or a tap outside) closes. Main.qml moves the focus (focusRow 0 is the field, then one row
// per section, focusCol the button) and handles the keys; taps come back through the signals.
Item {
    id: fs
    required property var ui
    anchors.fill: parent

    property string title: ""
    property string subtitle: ""
    property string fieldLabel: "Search"
    property string text: ""
    property string placeholder: ""
    property var sections: []             // [{key, title, options: [label], chosen}]
    property int focusRow: 0
    property int focusCol: 0

    signal fieldClicked()
    signal optionClicked(int row, int col)
    signal showClicked()
    signal closed()

    Rectangle { anchors.fill: parent; color: "#C0000000" }
    MouseArea { anchors.fill: parent; onClicked: fs.closed() }
    Rectangle {
        id: box
        anchors.horizontalCenter: parent.horizontalCenter
        y: Math.max(fs.ui.px(16), (parent.height - fs.ui.px(56) - height) / 2)
        width: Math.min(parent.width - fs.ui.px(40), fs.ui.px(560))
        height: Math.min(parent.height - fs.ui.px(80), col.implicitHeight + fs.ui.px(40))
        radius: fs.ui.px(8)
        color: fs.ui.c.panel
        border.color: fs.ui.c.line
        border.width: Math.max(1, fs.ui.px(1))
        clip: true
        MouseArea { anchors.fill: parent }
        Flickable {
            id: flick
            anchors.fill: parent
            anchors.margins: fs.ui.px(20)
            contentHeight: col.implicitHeight
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            Column {
                id: col
                width: flick.width
                spacing: fs.ui.px(10)
                Text {
                    width: parent.width
                    elide: Text.ElideRight
                    text: fs.title
                    color: fs.ui.c.text
                    font.family: fs.ui.titleFont; font.pixelSize: fs.ui.px(18); font.bold: true
                }
                Text {
                    visible: fs.subtitle !== ""
                    width: parent.width
                    elide: Text.ElideRight
                    text: fs.subtitle
                    color: fs.ui.c.soft
                    font.family: fs.ui.bodyFont; font.pixelSize: fs.ui.px(13)
                }
                Text { text: fs.fieldLabel; color: fs.ui.c.soft; font.family: fs.ui.bodyFont; font.pixelSize: fs.ui.px(13) }
                // The field: A or Y opens the keyboard.
                Rectangle {
                    id: field
                    readonly property bool foc: fs.focusRow === 0
                    width: parent.width
                    height: fs.ui.px(40)
                    radius: fs.ui.px(6)
                    color: fs.ui.c.card
                    border.color: foc ? fs.ui.c.accentHi : fs.ui.c.line
                    border.width: foc ? fs.ui.px(2) : Math.max(1, fs.ui.px(1))
                    Text {
                        x: fs.ui.px(12)
                        width: parent.width - fs.ui.px(24)
                        anchors.verticalCenter: parent.verticalCenter
                        elide: Text.ElideRight
                        text: fs.text !== "" ? fs.text : fs.placeholder
                        color: fs.text !== "" ? fs.ui.c.text : fs.ui.c.muted
                        font.family: fs.ui.bodyFont; font.pixelSize: fs.ui.px(16)
                    }
                    Accessible.role: Accessible.EditableText
                    Accessible.name: fs.fieldLabel + ": " + (fs.text !== "" ? fs.text : "empty")
                    TapHandler { onTapped: fs.fieldClicked() }
                }
                Repeater {
                    model: fs.sections
                    Column {
                        id: secCol
                        required property int index
                        required property var modelData
                        width: col.width
                        spacing: fs.ui.px(6)
                        Text { text: secCol.modelData.title; color: fs.ui.c.soft; font.family: fs.ui.bodyFont; font.pixelSize: fs.ui.px(13) }
                        Flow {
                            width: parent.width
                            spacing: fs.ui.px(8)
                            Repeater {
                                model: secCol.modelData.options
                                Rectangle {
                                    id: opt
                                    required property int index
                                    required property string modelData
                                    readonly property bool chosen: index === secCol.modelData.chosen
                                    readonly property bool foc: fs.focusRow === secCol.index + 1 && fs.focusCol === index
                                    width: optText.implicitWidth + fs.ui.px(28)
                                    height: fs.ui.px(34)
                                    radius: fs.ui.px(6)
                                    color: chosen ? fs.ui.c.accent : fs.ui.c.card
                                    border.color: foc ? fs.ui.c.text : fs.ui.c.line
                                    border.width: foc ? fs.ui.px(3) : Math.max(1, fs.ui.px(1))
                                    Text {
                                        id: optText
                                        anchors.centerIn: parent
                                        text: opt.modelData
                                        color: opt.chosen ? fs.ui.c.onAccent : fs.ui.c.text
                                        font.family: fs.ui.bodyFont; font.pixelSize: fs.ui.px(15); font.bold: opt.chosen
                                    }
                                    Accessible.role: Accessible.RadioButton
                                    Accessible.name: secCol.modelData.title + ": " + opt.modelData + (opt.chosen ? ", chosen" : "")
                                    TapHandler { onTapped: fs.optionClicked(secCol.index + 1, opt.index) }
                                }
                            }
                        }
                    }
                }
                // For touch and the mouse (the legend says Start and B).
                Row {
                    spacing: fs.ui.px(10)
                    topPadding: fs.ui.px(4)
                    Repeater {
                        model: ["Show results", "Close"]
                        Rectangle {
                            required property int index
                            required property string modelData
                            width: bt.implicitWidth + fs.ui.px(32)
                            height: fs.ui.px(38)
                            radius: fs.ui.px(6)
                            color: index === 0 ? fs.ui.c.accent : fs.ui.c.raised
                            border.color: fs.ui.c.line
                            border.width: index === 0 ? 0 : Math.max(1, fs.ui.px(1))
                            Text {
                                id: bt
                                anchors.centerIn: parent
                                text: parent.modelData
                                color: parent.index === 0 ? fs.ui.c.onAccent : fs.ui.c.text
                                font.family: fs.ui.bodyFont; font.pixelSize: fs.ui.px(15); font.bold: true
                            }
                            Accessible.role: Accessible.Button
                            Accessible.name: modelData
                            TapHandler { onTapped: parent.index === 0 ? fs.showClicked() : fs.closed() }
                        }
                    }
                }
            }
        }
    }
    // The focused row stays in view on a small window.
    onFocusRowChanged: {
        const y = focusRow === 0 ? 0 : flick.contentHeight * focusRow / Math.max(1, sections.length + 1)
        if (y < flick.contentY || y > flick.contentY + flick.height - fs.ui.px(60)) flick.contentY = Math.max(0, Math.min(flick.contentHeight - flick.height, y - fs.ui.px(40)))
    }
}
