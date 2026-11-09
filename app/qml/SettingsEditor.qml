import QtQuick

// 0.58, Edit settings: a mod's (or the game's) config file as a page of plain controls. Up and Down pick a setting;
// Left and Right change it (a toggle flips, a number steps, a list moves to the next choice); A flips a toggle,
// moves a list on, or opens Steam's keyboard for a number or text; Start saves; X is Reset to default; Y switches to
// the plain text of the file (A there edits the line); B goes back (twice when there are changes not saved). Taps
// and clicks work on every control. The file's comments are the help under each setting. Main.qml sends the keys.
Overlay {
    id: se
    closeOnOutside: false
    required property var ui
    anchors.fill: parent

    property string title: ""
    property string sub: ""
    // Settings that come first (Seamless Co-op's session password), by name.
    property var pinFirst: []
    // {"fields":[...], "lines":[...], "edited": bool, "rel", "format"}
    property var doc: ({ fields: [], lines: [] })
    property var edits: ({})              // field id: new value
    property var lineEdits: ({})          // line number: new text
    property bool textMode: false
    property int focusIndex: 0
    property bool leaveArmed: false

    readonly property int changes: Object.keys(edits).length + Object.keys(lineEdits).length
    readonly property var fields: doc && doc.fields ? doc.fields : []
    readonly property var lines: doc && doc.lines ? doc.lines : []
    // The rows: a heading where the section changes, then its settings; pinned ones first, without a heading.
    readonly property var rows: {
        let out = []
        const pinned = fields.filter(f => pinFirst.some(p => f.key.toLowerCase().indexOf(p) >= 0))
        for (const f of pinned) out.push({ kind: "field", f: f })
        let sec = null
        for (const f of fields) {
            if (pinned.indexOf(f) >= 0) continue
            if (f.section !== sec) { sec = f.section; if (sec !== "") out.push({ kind: "head", text: sec }) }
            out.push({ kind: "field", f: f })
        }
        return out
    }
    readonly property var lineRows: lines.map((t, i) => ({ kind: "line", n: i + 1, text: lineEdits[i + 1] !== undefined ? lineEdits[i + 1] : t }))
    readonly property var shown: textMode || fields.length === 0 ? lineRows : rows
    readonly property bool plain: textMode || fields.length === 0

    signal saveClicked()
    signal resetClicked()
    signal closeClicked()
    // A number or text to type: the field (or line), its value now, the words for the keyboard.
    signal typeAsked(var row, string value, string label)

    function valueOf(f) { return edits[f.id] !== undefined ? edits[f.id] : f.value }
    function setValue(f, v) {
        let e = Object.assign({}, edits)
        if (String(v) === String(f.value)) delete e[f.id]; else e[f.id] = String(v)
        edits = e
        leaveArmed = false
    }
    function setLine(n, t) {
        let e = Object.assign({}, lineEdits)
        if (t === lines[n - 1]) delete e[n]; else e[n] = t
        lineEdits = e
        leaveArmed = false
    }
    function reset() { edits = ({}); lineEdits = ({}); focusIndex = firstField(); textMode = false; leaveArmed = false }
    function firstField() { for (let i = 0; i < shown.length; i++) if (shown[i].kind !== "head") return i; return 0 }
    function decimalsOf(step) { const s = String(step); return s.indexOf(".") >= 0 ? s.split(".")[1].length : 0 }
    function stepNumber(f, d) {
        const now = parseFloat(valueOf(f))
        let v = (isNaN(now) ? 0 : now) + d * f.step
        if (f.min !== null && f.min !== undefined) v = Math.max(f.min, v)
        if (f.max !== null && f.max !== undefined) v = Math.min(f.max, v)
        setValue(f, v.toFixed(decimalsOf(f.step)))
    }
    function cycle(f, d) {
        const o = f.options || []
        if (o.length === 0) return
        let i = o.findIndex(x => x.value === valueOf(f))
        i = (i + d + o.length) % o.length
        setValue(f, o[i].value)
    }
    function change(r, d) {
        if (!r || r.kind !== "field") return
        const f = r.f
        if (f.kind === "toggle") setValue(f, valueOf(f) === f.on ? f.off : f.on)
        else if (f.kind === "number") stepNumber(f, d)
        else if (f.kind === "list") cycle(f, d)
    }
    function activate(r) {
        if (!r) return
        if (r.kind === "line") { typeAsked(r, r.text, "Line " + r.n); return }
        if (r.kind !== "field") return
        const f = r.f
        if (f.kind === "toggle" || f.kind === "list") change(r, 1)
        else typeAsked(r, valueOf(f), f.key)
    }
    // What A does on the focused row, for the legend.
    readonly property string aWord: {
        const r = shown[focusIndex]
        if (!r || r.kind === "head") return ""
        if (r.kind === "line") return "Edit line"
        return r.f.kind === "toggle" ? "Switch" : r.f.kind === "list" ? "Next" : "Type"
    }
    function move(d) {
        let i = focusIndex
        do { i += d } while (i >= 0 && i < shown.length && shown[i].kind === "head")
        if (i >= 0 && i < shown.length) focusIndex = i
        list.positionViewAtIndex(focusIndex, ListView.Contain)
    }
    function handleButton(b) {
        if (b === "up" || b === "down") move(b === "down" ? 1 : -1)
        else if (b === "lt" || b === "rt") { for (let k = 0; k < 6; k++) move(b === "rt" ? 1 : -1) }
        else if (b === "left" || b === "right") change(shown[focusIndex], b === "right" ? 1 : -1)
        else if (b === "a") activate(shown[focusIndex])
        else if (b === "start") saveClicked()
        else if (b === "x") resetClicked()
        else if (b === "y") { textMode = !textMode; focusIndex = firstField() }
        else if (b === "b") {
            if (changes > 0 && !leaveArmed) leaveArmed = true
            else closeClicked()
        }
    }
    onShownChanged: if (focusIndex >= shown.length) focusIndex = Math.max(0, shown.length - 1)

    Rectangle { anchors.fill: parent; color: "#C0000000" }
    Rectangle {
        id: box
        anchors.centerIn: parent
        anchors.verticalCenterOffset: -se.ui.px(24)
        width: Math.min(parent.width - se.ui.px(32), se.ui.px(980))
        height: parent.height - se.ui.px(120)
        radius: se.ui.px(10)
        color: se.ui.c.panel
        border.color: se.ui.c.line
        clip: true
        Rectangle { width: parent.width; height: se.ui.px(3); color: se.ui.c.accent; radius: se.ui.px(2) }
        Text {
            id: head
            x: se.ui.px(20); y: se.ui.px(16)
            width: parent.width - se.ui.px(40)
            text: se.title
            wrapMode: Text.Wrap
            color: se.ui.c.text
            font.family: se.ui.titleFont; font.pixelSize: se.ui.fpx(19); font.bold: true
        }
        Text {
            id: subText
            anchors.top: head.bottom; anchors.topMargin: se.ui.px(4)
            x: se.ui.px(20)
            width: parent.width - se.ui.px(40)
            wrapMode: Text.Wrap
            text: (se.leaveArmed ? "Changes not saved. B again leaves without them; Start saves." : se.changes > 0 ? se.changes + (se.changes === 1 ? " change" : " changes") + " not saved yet. Start saves." : se.sub)
                  + (se.plain && se.fields.length === 0 ? "  Turbine can't read this file as settings: here it is line by line." : "")
            color: se.leaveArmed ? se.ui.c.warn : se.ui.c.soft
            font.family: se.ui.bodyFont; font.pixelSize: se.ui.fpx(14)
        }
        ListView {
            id: list
            anchors.top: subText.bottom; anchors.topMargin: se.ui.px(10)
            anchors.bottom: btnRow.top; anchors.bottomMargin: se.ui.px(10)
            x: se.ui.px(16)
            width: parent.width - se.ui.px(32)
            clip: true
            spacing: se.ui.px(6)
            boundsBehavior: Flickable.StopAtBounds
            model: se.shown
            delegate: Rectangle {
                id: row
                required property int index
                required property var modelData
                readonly property bool foc: se.focusIndex === index
                readonly property bool isHead: modelData.kind === "head"
                readonly property var f: modelData.kind === "field" ? modelData.f : null
                readonly property string val: f ? se.valueOf(f) : modelData.kind === "line" ? modelData.text : ""
                width: list.width
                height: isHead ? headText.implicitHeight + se.ui.px(12) : Math.max(se.ui.tpx(48), rowCol.implicitHeight + se.ui.px(16))
                radius: se.ui.px(6)
                color: isHead ? "transparent" : foc ? se.ui.c.focusFill : se.ui.c.card
                border.color: se.ui.c.accent
                border.width: foc && !isHead ? se.ui.px(2) : 0
                Text {
                    id: headText
                    visible: row.isHead
                    anchors.bottom: parent.bottom
                    x: se.ui.px(4)
                    text: row.modelData.text || ""
                    color: se.ui.c.accentText
                    font.family: se.ui.bodyFont; font.pixelSize: se.ui.fpx(13); font.bold: true; font.capitalization: Font.AllUppercase
                }
                Column {
                    id: rowCol
                    visible: !row.isHead
                    x: se.ui.px(12)
                    anchors.verticalCenter: parent.verticalCenter
                    width: parent.width - control.width - se.ui.px(36)
                    spacing: se.ui.px(2)
                    Text {
                        width: parent.width
                        text: row.f ? row.f.key : row.modelData.kind === "line" ? "Line " + row.modelData.n : ""
                        elide: Text.ElideRight
                        color: se.ui.c.text
                        font.family: se.ui.bodyFont; font.pixelSize: se.ui.fpx(15); font.bold: true
                    }
                    Text {
                        width: parent.width
                        visible: text !== ""
                        text: row.f ? row.f.help : row.modelData.kind === "line" ? row.modelData.text : ""
                        wrapMode: Text.Wrap
                        maximumLineCount: row.foc ? 6 : 2
                        elide: Text.ElideRight
                        color: se.ui.c.soft
                        font.family: row.modelData.kind === "line" ? "monospace" : se.ui.bodyFont
                        font.pixelSize: se.ui.fpx(13)
                    }
                }
                // The control on the right.
                Row {
                    id: control
                    visible: !row.isHead && row.modelData.kind !== "line"
                    anchors.right: parent.right; anchors.rightMargin: se.ui.px(12)
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: se.ui.px(8)
                    // Toggle: one pill, On or Off.
                    Rectangle {
                        visible: row.f !== null && row.f.kind === "toggle"
                        readonly property bool on: row.f !== null && row.val === row.f.on
                        width: se.ui.tpx(64); height: se.ui.tpx(34); radius: height / 2
                        color: on ? se.ui.c.accent : se.ui.c.raised
                        border.color: se.ui.c.line
                        Text { anchors.centerIn: parent; text: parent.on ? "On" : "Off"; color: parent.on ? se.ui.c.onAccent : se.ui.c.soft; font.family: se.ui.bodyFont; font.pixelSize: se.ui.fpx(14); font.bold: true }
                        TapHandler { onTapped: { se.focusIndex = row.index; se.change(row.modelData, 1) } }
                    }
                    // Number and list: - value + (a slider under the value when the range is known).
                    Rectangle {
                        visible: row.f !== null && (row.f.kind === "number" || row.f.kind === "list")
                        width: se.ui.tpx(36); height: width; radius: se.ui.px(6)
                        color: se.ui.c.raised; border.color: se.ui.c.line
                        anchors.verticalCenter: parent.verticalCenter
                        Text { anchors.centerIn: parent; text: row.f && row.f.kind === "list" ? "<" : "−"; color: se.ui.c.text; font.pixelSize: se.ui.fpx(20); font.bold: true }
                        TapHandler { onTapped: { se.focusIndex = row.index; se.change(row.modelData, -1) } }
                    }
                    Column {
                        visible: row.f !== null && row.f.kind !== "toggle"
                        anchors.verticalCenter: parent.verticalCenter
                        width: Math.min(se.ui.px(240), Math.max(se.ui.px(90), valueText.implicitWidth + se.ui.px(16)))
                        spacing: se.ui.px(4)
                        Rectangle {
                            width: parent.width; height: se.ui.tpx(34); radius: se.ui.px(6)
                            color: se.ui.c.ground; border.color: se.ui.c.line
                            Text {
                                id: valueText
                                anchors.centerIn: parent
                                width: Math.min(implicitWidth, parent.width - se.ui.px(12))
                                elide: Text.ElideRight
                                text: {
                                    if (!row.f) return ""
                                    if (row.f.kind === "list") { const o = (row.f.options || []).find(x => x.value === row.val); return o ? o.label : row.val }
                                    return row.val === "" ? "(empty)" : row.val
                                }
                                color: row.val === "" ? se.ui.c.muted : se.ui.c.text
                                font.family: se.ui.bodyFont; font.pixelSize: se.ui.fpx(15)
                            }
                            TapHandler { onTapped: { se.focusIndex = row.index; se.activate(row.modelData) } }
                        }
                        Rectangle {
                            visible: row.f !== null && row.f.kind === "number" && row.f.min !== null && row.f.max !== null
                            width: parent.width; height: se.ui.px(4); radius: 2
                            color: se.ui.c.line
                            Rectangle {
                                readonly property real part: row.f && row.f.max > row.f.min ? Math.max(0, Math.min(1, (parseFloat(row.val) - row.f.min) / (row.f.max - row.f.min))) : 0
                                width: parent.width * part; height: parent.height; radius: 2; color: se.ui.c.accent
                            }
                        }
                    }
                    Rectangle {
                        visible: row.f !== null && (row.f.kind === "number" || row.f.kind === "list")
                        width: se.ui.tpx(36); height: width; radius: se.ui.px(6)
                        color: se.ui.c.raised; border.color: se.ui.c.line
                        anchors.verticalCenter: parent.verticalCenter
                        Text { anchors.centerIn: parent; text: row.f && row.f.kind === "list" ? ">" : "+"; color: se.ui.c.text; font.pixelSize: se.ui.fpx(20); font.bold: true }
                        TapHandler { onTapped: { se.focusIndex = row.index; se.change(row.modelData, 1) } }
                    }
                }
                TapHandler { enabled: !row.isHead; onTapped: se.focusIndex = row.index }
            }
        }
        Flow {
            id: btnRow
            x: se.ui.px(20)
            width: parent.width - se.ui.px(40)
            anchors.bottom: parent.bottom
            anchors.bottomMargin: se.ui.px(14)
            spacing: se.ui.px(10)
            Repeater {
                model: [{ k: "start", label: "Save" }, { k: "x", label: "Reset to default" }, { k: "y", label: se.plain && se.fields.length > 0 ? "Settings" : "Plain text" }, { k: "b", label: "Back" }]
                Rectangle {
                    required property var modelData
                    visible: !(modelData.k === "y" && se.fields.length === 0)
                    width: bRow.implicitWidth + se.ui.px(24); height: se.ui.tpx(38); radius: se.ui.px(6)
                    color: modelData.k === "start" && se.changes > 0 ? se.ui.c.accent : se.ui.c.card
                    border.color: se.ui.c.line
                    Row {
                        id: bRow
                        anchors.centerIn: parent
                        spacing: se.ui.px(6)
                        Text { text: parent.parent.modelData.label; color: parent.parent.modelData.k === "start" && se.changes > 0 ? se.ui.c.onAccent : se.ui.c.text; font.family: se.ui.bodyFont; font.pixelSize: se.ui.fpx(14); font.bold: true }
                    }
                    TapHandler { onTapped: se.handleButton(parent.modelData.k) }
                }
            }
        }
    }
}
