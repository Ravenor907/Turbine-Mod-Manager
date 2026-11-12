import QtQuick

// Y's pop-up: one place to find. Get mods: a Search field, Sort and Category buttons (a long category list ends in
// More...). My mods: a Find field, Show and Sort buttons. Every choice takes effect the moment it is made (the
// result line on the right says what it found), and the pop-up only closes: Done (A on it, B, Start, a tap outside)
// keeps everything; Clear all (X) puts everything back. The field is typed right here (A or a tap opens Steam's
// keyboard); Enter shows the results. Main.qml moves the focus (focusRow 0 is the field, then one row per section,
// then the buttons row; focusCol the button) and handles the keys; taps come back through the signals.
Overlay {
    id: fs
    onOutsideTapped: fs.closed()
    required property var ui
    motion: ui.motion
    anchors.fill: parent

    property string title: ""
    property string subtitle: ""
    property string fieldLabel: "Search"
    property string text: ""
    property string placeholder: ""
    property var sections: []             // [{key, title, options: [label], chosen}]
    property int focusRow: 0
    property int focusCol: 0
    property string result: ""            // "24 mods", "Looking...", "Showing 3 of 12"
    readonly property int buttonsRow: sections.length + 1
    readonly property bool typing: input.activeFocus

    signal fieldClicked()
    signal optionClicked(int row, int col)
    signal doneClicked()
    signal clearClicked()
    signal closed()
    signal typed(string text)             // every change while typing
    signal entered(string text)           // Enter or Done on Steam's keyboard
    function focusField() { input.text = fs.text; input.forceActiveFocus(); input.cursorPosition = input.length }
    function stopTyping() { if (input.activeFocus) input.focus = false }
    readonly property string fieldText: input.activeFocus ? input.text : fs.text

    Scrim { ui: fs.ui }
    Card {
        ui: fs.ui
        id: box
        anchors.horizontalCenter: parent.horizontalCenter
        y: Math.max(fs.ui.px(16), (parent.height - height) / 2)
        width: Math.min(parent.width - fs.ui.px(40), fs.ui.px(560))
        height: Math.min(parent.height - fs.ui.px(32), col.implicitHeight + fs.ui.px(44))
        clip: true
        CardGuard {}
        Flickable {
            id: flick
            // Scrolls on purpose on a small window, with ScrollFade's fades and chevron as the cue.
            property string layoutOk: "scroll"
            anchors.fill: parent
            anchors.margins: fs.ui.px(22)
            contentHeight: col.implicitHeight
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            Column {
                id: col
                width: flick.width
                spacing: fs.ui.px(10)
                Item {
                    width: parent.width
                    height: titleText.implicitHeight
                Text {
                    id: titleText
                    width: parent.width - resultText.implicitWidth - fs.ui.px(12)
                    elide: Text.ElideRight
                    text: fs.title
                    color: fs.ui.c.text
                    font.family: fs.ui.titleFont; font.pixelSize: fs.ui.fpx(19); font.bold: true
                }
                // What the choices found, live.
                Text {
                    id: resultText
                    objectName: "findResult"
                    anchors.right: parent.right
                    anchors.verticalCenter: titleText.verticalCenter
                    text: fs.result
                    color: fs.ui.c.accentText
                    font.family: fs.ui.bodyFont; font.pixelSize: fs.ui.fpx(14); font.bold: true
                }
                }
                Text {
                    visible: fs.subtitle !== ""
                    width: parent.width
                    elide: Text.ElideRight
                    text: fs.subtitle
                    color: fs.ui.c.soft
                    font.family: fs.ui.bodyFont; font.pixelSize: fs.ui.fpx(13)
                }
                Text { text: fs.fieldLabel; color: fs.ui.c.soft; font.family: fs.ui.bodyFont; font.pixelSize: fs.ui.fpx(13) }
                // The field, typed right here: A, Y or a tap opens Steam's keyboard.
                Rectangle {
                    id: field
                    readonly property bool foc: fs.focusRow === 0
                    width: parent.width
                    height: fs.ui.tpx(44)
                    radius: fs.ui.px(6)
                    color: fs.ui.c.card
                    border.color: foc ? fs.ui.c.accentHi : fs.ui.c.line
                    border.width: foc ? fs.ui.px(2) : Math.max(1, fs.ui.px(1))
                    TextInput {
                        id: input
                        x: fs.ui.px(12)
                        width: parent.width - fs.ui.px(24)
                        anchors.verticalCenter: parent.verticalCenter
                        clip: true
                        maximumLength: 100
                        color: fs.ui.c.text
                        font.family: fs.ui.bodyFont; font.pixelSize: fs.ui.fpx(16)
                        // Shows the words in effect when not typing.
                        Binding on text { when: !input.activeFocus; value: fs.text }
                        onTextEdited: fs.typed(text)
                        onAccepted: { const t = text.trim(); input.focus = false; fs.entered(t) }
                        Text {
                            visible: input.text === "" && !input.activeFocus
                            anchors.verticalCenter: parent.verticalCenter
                            width: parent.width; elide: Text.ElideRight
                            text: fs.placeholder
                            color: fs.ui.c.muted
                            font: input.font
                        }
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
                        Text { text: secCol.modelData.title; color: fs.ui.c.soft; font.family: fs.ui.bodyFont; font.pixelSize: fs.ui.fpx(13) }
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
                                    // Chosen: an accent outline, accent words and a check mark. The highlight: a ring
                                    // around the chip (never a fill).
                                    width: optText.implicitWidth + fs.ui.px(28)
                                    height: fs.ui.tpx(40)
                                    radius: fs.ui.radiusSmall
                                    color: chosen ? Qt.tint(fs.ui.c.card, Qt.rgba(fs.ui.c.accent.r, fs.ui.c.accent.g, fs.ui.c.accent.b, 0.12)) : fs.ui.c.card
                                    border.color: chosen ? fs.ui.c.accent : fs.ui.c.line
                                    border.width: chosen ? Math.max(1, fs.ui.px(1.5)) : Math.max(1, fs.ui.px(1))
                                    Rectangle {
                                        visible: opt.foc
                                        anchors.fill: parent
                                        anchors.margins: -fs.ui.px(4)
                                        radius: parent.radius + fs.ui.px(4)
                                        color: "transparent"
                                        border.width: fs.ui.ringWidth
                                        border.color: fs.ui.ring
                                    }
                                    Text {
                                        id: optText
                                        anchors.centerIn: parent
                                        text: (opt.chosen ? "\u2713 " : "") + opt.modelData
                                        color: opt.chosen ? fs.ui.c.accentText : fs.ui.c.text
                                        font.family: fs.ui.bodyFont; font.pixelSize: fs.ui.fpx(15); font.bold: opt.chosen
                                    }
                                    Accessible.role: Accessible.RadioButton
                                    Accessible.name: secCol.modelData.title + ": " + opt.modelData + (opt.chosen ? ", chosen" : "")
                                    TapHandler { onTapped: fs.optionClicked(secCol.index + 1, opt.index) }
                                }
                            }
                        }
                    }
                }
                // Done and Clear all (A on them, or B and X anywhere).
                Row {
                    spacing: fs.ui.px(10)
                    topPadding: fs.ui.px(4)
                    Repeater {
                        model: ["Done", "Clear all"]
                        // Done is B anywhere, Clear all X; A presses the highlighted one.
                        ActionButton {
                            required property int index
                            required property string modelData
                            ui: fs.ui
                            label: modelData
                            focused: fs.focusRow === fs.buttonsRow && fs.focusCol === index
                            primary: index === 0
                            cap: focused ? "a" : index === 0 ? "b" : "x"
                            Accessible.role: Accessible.Button
                            Accessible.name: modelData
                            onClicked: index === 0 ? fs.doneClicked() : fs.clearClicked()
                        }
                    }
                }
            }
        }
        ScrollFade { ui: fs.ui; view: flick; color: fs.ui.c.panel }
    }
    // The focused row stays in view on a small window.
    onFocusRowChanged: {
        const y = focusRow === 0 ? 0 : flick.contentHeight * focusRow / Math.max(1, sections.length + 1)
        if (y < flick.contentY || y > flick.contentY + flick.height - fs.ui.px(60)) flick.contentY = Math.max(0, Math.min(flick.contentHeight - flick.height, y - fs.ui.px(40)))
    }
}
