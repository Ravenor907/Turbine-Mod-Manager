import QtQuick

// Labs > Report a bug: the screenshot of Turbine taken just before this opened, Include screenshot (on unless switched
// off), what went wrong (up to 2000 characters, typed with Steam's on-screen keyboard), Include performance check
// (when Labs > Performance check opened it; on unless switched off), Include crash log (off unless ticked; only when
// there is one), then Not now and Save report side by side. The D-pad moves between them, A uses one. B closes
// and keeps the words as a draft (Report a bug brings them back); nothing typed is ever lost by B.
// Save report saves the report (Turbine writes the details itself: version, device, the game, its mods, the activity log)
// and the screenshot to Downloads and copies the report to the clipboard, ready to paste.
//
// Main.qml takes the screenshot, calls open(), passes controller buttons to handleButton() and does the saving when
// sendClicked comes.
Overlay {
    id: report
    onOutsideTapped: report.closeClicked()
    required property var ui
    motion: ui.motion
    anchors.fill: parent
    visible: false

    property string shotUrl: ""           // the screenshot, as a file URL; "" when there is none
    property string item: ""              // a Labs checklist item's title when opened from it
    property string text: ""
    property bool include: true
    property string perf: ""
    property bool includePerf: true
    property bool hasLog: false
    property bool includeLog: false
    property string aboutLine: ""         // "Also saves Turbine 0.2 ..." under the switches
    property bool editing: false
    // The focus with a controller: 0 Include screenshot, 1 the text, 5 Include performance check, 4 Include crash
    // log, 2 Save report, 3 Not now. Done typing moves the focus to Save report, so A saves.
    property int focusAt: 1
    readonly property var order: (shotUrl !== "" ? [0] : []).concat([1]).concat(perf !== "" ? [5] : []).concat(hasLog ? [4] : []).concat([2])

    signal sendClicked()
    signal closeClicked()
    signal editStarted()
    signal editEnded()

    function open(itemTitle, startText, perfText, crashLog) {
        item = itemTitle || ""
        text = startText || ""
        perf = perfText || ""
        includePerf = true
        include = true
        hasLog = !!crashLog
        includeLog = false
        focusAt = 1
        editing = false
        view.contentY = 0
        visible = true
    }
    function close() {
        if (editing) endEdit()
        visible = false
        text = ""; perf = ""; item = ""
    }
    function beginEdit() {
        focusAt = 1
        editing = true
        editor.text = report.text
        editor.cursorPosition = editor.length
        editor.forceActiveFocus()
        editStarted()
    }
    function endEdit() {
        if (!editing) return
        report.text = editor.text.slice(0, 2000)
        editing = false
        if (report.text.trim() !== "") focusAt = 2
        editEnded()
    }
    function activate() {
        if (focusAt === 0) include = !include
        else if (focusAt === 1) beginEdit()
        else if (focusAt === 4) includeLog = !includeLog
        else if (focusAt === 5) includePerf = !includePerf
        else if (focusAt === 2) sendClicked()
        else closeClicked()
    }
    function handleButton(b) {
        if (editing) {
            if (b === "b" || b === "start") endEdit()
            return
        }
        if (b === "up") {
            const i = order.indexOf(focusAt === 3 ? 2 : focusAt)
            focusAt = order[Math.max(0, i - 1)]
        } else if (b === "down") {
            if (focusAt === 2 || focusAt === 3) return
            const i = order.indexOf(focusAt)
            focusAt = order[Math.min(order.length - 1, i + 1)]
        } else if (b === "left" || b === "right") {
            if (focusAt === 2) focusAt = 3
            else if (focusAt === 3) focusAt = 2
        } else if (b === "a") activate()
        else if (b === "start") sendClicked()
        else if (b === "b") closeClicked()
    }
    onFocusAtChanged: Qt.callLater(reveal)
    function reveal() {
        const it = focusAt === 0 ? shotRow : focusAt === 1 ? textBox : focusAt === 4 ? logRow : focusAt === 5 ? perfRow : buttons
        if (!it || !it.visible) return
        const y = it.mapToItem(col, 0, 0).y
        if (y < view.contentY) view.contentY = Math.max(0, y)
        else if (y + it.height > view.contentY + view.height) view.contentY = Math.min(Math.max(0, view.contentHeight - view.height), y + it.height - view.height)
    }

    component Switch: Rectangle {
        property bool on: false
        width: report.ui.px(46); height: report.ui.px(26); radius: height / 2
        color: on ? report.ui.c.accent : report.ui.c.ground
        border.color: on ? report.ui.c.accent : report.ui.c.line
        border.width: Math.max(1, report.ui.px(1.5))
        Rectangle {
            width: parent.height - report.ui.px(8); height: width; radius: width / 2
            anchors.verticalCenter: parent.verticalCenter
            x: parent.on ? parent.width - width - report.ui.px(4) : report.ui.px(4)
            color: parent.on ? report.ui.c.onAccent : report.ui.c.muted
        }
    }
    component SwitchRow: Rectangle {
        id: srow
        property string label: ""
        property bool on: false
        property bool focused: false
        signal toggled()
        width: parent ? parent.width : 0
        height: report.ui.px(46)
        radius: report.ui.px(6)
        color: focused ? report.ui.c.focusFill : report.ui.c.card
        border.color: report.ui.c.accent
        border.width: focused ? report.ui.px(2) : 0
        Text {
            x: report.ui.px(14)
            anchors.verticalCenter: parent.verticalCenter
            text: srow.label
            color: report.ui.c.text
            font.family: report.ui.bodyFont
            font.pixelSize: report.ui.fpx(15)
            font.bold: true
        }
        Switch { on: srow.on; anchors.right: parent.right; anchors.rightMargin: report.ui.px(12); anchors.verticalCenter: parent.verticalCenter }
        HoverHandler { onHoveredChanged: report.ui.hover(parent, hovered) }
        TapHandler { onTapped: srow.toggled() }
    }
    component Button: Rectangle {
        id: btn
        property string label: ""
        property string key: ""
        property bool primary: false
        property bool focused: false
        property bool dim: false
        signal clicked()
        width: btnRow.implicitWidth + report.ui.px(28)
        height: report.ui.tpx(44)
        radius: report.ui.px(8)
        opacity: dim ? 0.55 : 1
        color: primary ? report.ui.c.accent : (focused ? report.ui.c.focusFill : report.ui.c.raised)
        border.color: focused ? report.ui.c.text : report.ui.c.line
        border.width: focused ? report.ui.px(2) : Math.max(1, report.ui.px(1))
        Row {
            id: btnRow
            anchors.centerIn: parent
            spacing: report.ui.px(8)
            Rectangle {
                visible: btn.key !== ""
                width: report.ui.px(24); height: width; radius: width / 2
                anchors.verticalCenter: parent.verticalCenter
                color: report.ui.c.raised
                border.color: report.ui.c.accentLo
                Text { anchors.centerIn: parent; text: btn.key.toUpperCase(); color: report.ui.c.text; font.family: report.ui.bodyFont; font.pixelSize: report.ui.fpx(13); font.bold: true }
            }
            Text { text: btn.label; color: btn.primary ? report.ui.c.onAccent : report.ui.c.text; font.family: report.ui.bodyFont; font.pixelSize: report.ui.fpx(15); font.bold: true; anchors.verticalCenter: parent.verticalCenter }
        }
        HoverHandler { onHoveredChanged: report.ui.hover(parent, hovered) }
        TapHandler { onTapped: btn.clicked() }
    }

    Rectangle { anchors.fill: parent; color: "#C8000000" }
    // A tap beside it closes it (Main.qml asks first when something was typed).

    Rectangle {
        id: box
        anchors.centerIn: parent
        width: Math.min(parent.width - report.ui.px(32), report.ui.px(620))
        height: Math.min(parent.height - report.ui.px(24), col.implicitHeight + report.ui.px(36))
        radius: report.ui.px(10)
        color: report.ui.c.panel
        border.color: report.ui.c.line
        clip: true
        CardGuard {}   // taps inside don't close it
        Flickable {
            id: view
            anchors.fill: parent
            anchors.margins: report.ui.px(18)
            contentHeight: col.implicitHeight
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            Column {
                id: col
                width: view.width
                spacing: report.ui.px(10)
                Text {
                    text: "Report a bug"
                    color: report.ui.c.text
                    font.family: report.ui.titleFont
                    font.pixelSize: report.ui.fpx(18)
                    font.bold: true
                }
                Text {
                    visible: report.item !== ""
                    width: parent.width
                    wrapMode: Text.WordWrap
                    text: "About: " + report.item
                    color: report.ui.c.soft
                    font.family: report.ui.bodyFont
                    font.pixelSize: report.ui.fpx(15)
                }
                // The screenshot, as it will be saved (smaller here).
                Rectangle {
                    visible: report.shotUrl !== ""
                    width: parent.width
                    height: Math.min(Math.round(width * 10 / 16), report.ui.px(170))
                    radius: report.ui.px(6)
                    color: report.ui.c.ground
                    border.color: report.ui.c.line
                    opacity: report.include ? 1 : 0.35
                    Image {
                        anchors.fill: parent
                        anchors.margins: 1
                        fillMode: Image.PreserveAspectFit
                        source: report.shotUrl
                        cache: false
                        asynchronous: true
                    }
                }
                SwitchRow {
                    id: shotRow
                    visible: report.shotUrl !== ""
                    label: "Include screenshot"
                    on: report.include
                    focused: report.focusAt === 0 && !report.editing
                    onToggled: { report.focusAt = 0; report.include = !report.include }
                }
                // What went wrong: A or a tap starts typing (Steam's keyboard comes up in Game Mode).
                Rectangle {
                    id: textBox
                    width: parent.width
                    height: Math.max(report.ui.px(90), (report.editing ? editor.contentHeight : body.implicitHeight) + report.ui.px(24))
                    radius: report.ui.px(6)
                    color: report.ui.c.ground
                    border.width: report.focusAt === 1 ? report.ui.px(2) : Math.max(1, report.ui.px(1))
                    border.color: report.focusAt === 1 ? report.ui.c.accent : report.ui.c.line
                    Text {
                        id: body
                        visible: !report.editing
                        x: report.ui.px(12); y: report.ui.px(12)
                        width: parent.width - report.ui.px(24)
                        wrapMode: Text.Wrap
                        textFormat: Text.PlainText
                        text: report.text !== "" ? report.text : "What went wrong? (A to type)"
                        color: report.text !== "" ? report.ui.c.text : report.ui.c.muted
                        font.family: report.ui.bodyFont
                        font.pixelSize: report.ui.fpx(15)
                    }
                    TextEdit {
                        id: editor
                        visible: report.editing
                        x: report.ui.px(12); y: report.ui.px(12)
                        width: parent.width - report.ui.px(24)
                        wrapMode: TextEdit.Wrap
                        textFormat: TextEdit.PlainText
                        color: report.ui.c.text
                        selectionColor: report.ui.c.accent
                        selectedTextColor: report.ui.c.onAccent
                        font.family: report.ui.bodyFont
                        font.pixelSize: report.ui.fpx(15)
                        onTextChanged: if (length > 2000) remove(2000, length)
                        Keys.onEscapePressed: report.endEdit()
                    }
                    Text {
                        visible: report.editing
                        anchors.right: parent.right; anchors.bottom: parent.bottom
                        anchors.margins: report.ui.px(6)
                        text: editor.length + " / 2000"
                        color: report.ui.c.muted
                        font.family: report.ui.bodyFont
                        font.pixelSize: report.ui.fpx(13)
                    }
                    HoverHandler { onHoveredChanged: report.ui.hover(parent, hovered) }
                    TapHandler { onTapped: if (!report.editing) report.beginEdit() }
                }
                Row {
                    visible: report.editing
                    spacing: report.ui.px(10)
                    Button { label: "Done typing"; key: "b"; focused: true; onClicked: report.endEdit() }
                }
                SwitchRow {
                    id: perfRow
                    visible: report.perf !== ""
                    label: "Include performance check"
                    on: report.includePerf
                    focused: report.focusAt === 5 && !report.editing
                    onToggled: { report.focusAt = 5; report.includePerf = !report.includePerf }
                }
                SwitchRow {
                    id: logRow
                    visible: report.hasLog
                    label: "Include crash log"
                    on: report.includeLog
                    focused: report.focusAt === 4 && !report.editing
                    onToggled: { report.focusAt = 4; report.includeLog = !report.includeLog }
                }
                Text {
                    width: parent.width
                    wrapMode: Text.WordWrap
                    text: report.aboutLine
                    color: report.ui.c.muted
                    font.family: report.ui.bodyFont
                    font.pixelSize: report.ui.fpx(14)
                }
                Row {
                    id: buttons
                    anchors.right: parent.right
                    spacing: report.ui.px(10)
                    Button { label: "Not now"; focused: report.focusAt === 3 && !report.editing; onClicked: report.closeClicked() }
                    Button {
                        label: "Save report"; key: "a"; primary: true
                        dim: report.text.trim() === ""
                        focused: report.focusAt === 2 && !report.editing
                        onClicked: report.sendClicked()
                    }
                }
            }
        }
    }
}
