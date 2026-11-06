import QtQuick

// A line of text from the player: a search, a profile name, a Thunderstore profile code, the Nexus Mods key. In Game
// Mode Steam's on-screen keyboard opens with it (Main.qml asks Steam). A or Enter is Done, B or Esc cancels.
Item {
    id: ask
    required property var ui
    anchors.fill: parent
    visible: false

    property string title: ""
    property string note: ""
    property string placeholder: ""
    property int maxLength: 200
    property bool secret: false           // shown as dots (the Nexus key)
    property string doneLabel: "Done"

    signal accepted(string text)
    signal cancelled()

    function open(t, n, start, ph, max, hide, done) {
        title = t; note = n || ""; placeholder = ph || ""
        maxLength = max || 200; secret = !!hide; doneLabel = done || "Done"
        field.text = start || ""
        visible = true
        field.forceActiveFocus()
        field.cursorPosition = field.length
    }
    function close() { visible = false }
    function accept() { const t = field.text.trim(); visible = false; accepted(t) }
    function cancel() { visible = false; cancelled() }
    function handleButton(b) {
        if (b === "a" || b === "start") accept()
        else if (b === "b") cancel()
    }

    Rectangle { anchors.fill: parent; color: "#C8000000" }
    MouseArea { anchors.fill: parent; onClicked: ask.cancel() }
    Rectangle {
        anchors.horizontalCenter: parent.horizontalCenter
        y: ask.ui.px(40)
        width: Math.min(parent.width - ask.ui.px(32), ask.ui.px(640))
        height: col.implicitHeight + ask.ui.px(36)
        radius: ask.ui.px(10)
        color: ask.ui.c.panel
        border.color: ask.ui.c.line
        MouseArea { anchors.fill: parent }
        Column {
            id: col
            x: ask.ui.px(18); y: ask.ui.px(18)
            width: parent.width - ask.ui.px(36)
            spacing: ask.ui.px(10)
            Text { text: ask.title; color: ask.ui.c.text; font.family: ask.ui.titleFont; font.pixelSize: ask.ui.px(18); font.bold: true }
            Text {
                visible: ask.note !== ""
                width: parent.width
                wrapMode: Text.WordWrap
                text: ask.note
                color: ask.ui.c.soft
                font.family: ask.ui.bodyFont
                font.pixelSize: ask.ui.px(15)
            }
            Rectangle {
                width: parent.width
                height: ask.ui.px(48)
                radius: ask.ui.px(6)
                color: ask.ui.c.ground
                border.color: ask.ui.c.accent
                border.width: ask.ui.px(2)
                TextInput {
                    id: field
                    anchors.fill: parent
                    anchors.leftMargin: ask.ui.px(12); anchors.rightMargin: ask.ui.px(12)
                    verticalAlignment: TextInput.AlignVCenter
                    color: ask.ui.c.text
                    selectionColor: ask.ui.c.accent
                    selectedTextColor: ask.ui.c.onAccent
                    font.family: ask.ui.bodyFont
                    font.pixelSize: ask.ui.px(16)
                    maximumLength: ask.maxLength
                    echoMode: ask.secret ? TextInput.Password : TextInput.Normal
                    clip: true
                    Keys.onReturnPressed: ask.accept()
                    Keys.onEnterPressed: ask.accept()
                    Keys.onEscapePressed: ask.cancel()
                    Text {
                        visible: field.text === ""
                        anchors.verticalCenter: parent.verticalCenter
                        text: ask.placeholder
                        color: ask.ui.c.muted
                        font: field.font
                    }
                }
            }
            Row {
                anchors.right: parent.right
                spacing: ask.ui.px(10)
                Repeater {
                    model: [["b", "Cancel"], ["a", ask.doneLabel]]
                    Rectangle {
                        required property var modelData
                        required property int index
                        width: r.implicitWidth + ask.ui.px(26); height: ask.ui.px(40); radius: ask.ui.px(8)
                        color: index === 1 ? ask.ui.c.accent : ask.ui.c.raised
                        Row {
                            id: r
                            anchors.centerIn: parent
                            spacing: ask.ui.px(8)
                            Rectangle {
                                width: ask.ui.px(24); height: width; radius: width / 2
                                anchors.verticalCenter: parent.verticalCenter
                                color: ask.ui.c.raised; border.color: ask.ui.c.accentLo
                                Text { anchors.centerIn: parent; text: parent.parent.parent.modelData[0].toUpperCase(); color: ask.ui.c.text; font.family: ask.ui.bodyFont; font.pixelSize: ask.ui.px(13); font.bold: true }
                            }
                            Text { text: parent.parent.modelData[1]; color: parent.parent.index === 1 ? ask.ui.c.onAccent : ask.ui.c.text; font.family: ask.ui.bodyFont; font.pixelSize: ask.ui.px(15); font.bold: true; anchors.verticalCenter: parent.verticalCenter }
                        }
                        TapHandler { onTapped: parent.index === 1 ? ask.accept() : ask.cancel() }
                    }
                }
            }
        }
    }
}
