import QtQuick

// Edit settings: a mod's (or the game's) config file as a page of plain controls. Up and Down pick a setting;
// Left and Right change it (a toggle flips, a number steps, a list moves to the next choice); A flips a toggle,
// moves a list on, or opens Steam's keyboard for a number or text; every change saves itself; X opens the
// options (the file as plain text, where A edits a line, or as a list; Reset to default); B goes back. Taps and
// clicks work on every control. The file's comments are the help under each setting. Main.qml sends the keys.
Overlay {
    id: se
    closeOnOutside: false
    required property var ui
    motion: ui.motion
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
    // Rows after the settings that do something (Seamless Co-op's page: the Co-op tile): [{key, title, detail, label}].
    property var extraRows: []
    signal actionClicked(string key)

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
        for (const x of extraRows) out.push({ kind: "action", key: x.key, title: x.title, detail: x.detail, label: x.label })
        return out
    }
    readonly property var lineRows: lines.map((t, i) => ({ kind: "line", n: i + 1, text: lineEdits[i + 1] !== undefined ? lineEdits[i + 1] : t }))
    readonly property var shown: textMode || fields.length === 0 ? lineRows : rows
    readonly property bool plain: textMode || fields.length === 0

    signal saveClicked()
    // X opens the options (Show as text or as a list, Reset to default); Y is never used here.
    signal optionsClicked()
    function toggleView() { textMode = !textMode; focusIndex = firstField() }
    signal closeClicked()
    // A number or text to type: the field (or line), its value now, the words for the keyboard.
    signal typeAsked(var row, string value, string label)

    // Plain names ("allow_invaders" is "Allow invaders"; Seamless Co-op's own get their meaning).
    readonly property var plainNames: ({ cooppassword: "Session password", allow_invaders: "Allow invaders", overhead_player_display: "Over other players' heads",
        always_spectate_on_death: "Watch others after you die", default_boot_master_volume: "Volume at start", enemy_health_scaling: "Enemy health per player (%)",
        enemy_damage_scaling: "Enemy damage per player (%)", enemy_posture_scaling: "Enemy posture per player (%)", boss_health_scaling: "Boss health per player (%)",
        boss_damage_scaling: "Boss damage per player (%)", boss_posture_scaling: "Boss posture per player (%)", save_file_extension: "Save file extension",
        death_debuff: "Death debuff", allow_summons: "Spirit summons", skip_splash_screens: "Skip the start-up screens", mod_language_override: "Language" })
    function nameOf(k) {
        const key = String(k || "")
        if (plainNames[key.toLowerCase()]) return plainNames[key.toLowerCase()]
        if (!/[_]/.test(key) && !/^[a-z]+$/.test(key)) return key
        const w = key.replace(/_/g, " ").trim()
        return w.charAt(0).toUpperCase() + w.slice(1)
    }
    readonly property bool coop: title.indexOf("Seamless Co-op") >= 0
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
        if (r.kind === "action") { actionClicked(r.key); return }
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
        if (r.kind === "action") return r.label
        return r.f.kind === "toggle" ? "Switch" : r.f.kind === "list" ? "Next" : "Type"
    }
    function move(d) {
        let i = focusIndex
        do { i += d } while (i >= 0 && i < shown.length && shown[i].kind === "head")
        if (i >= 0 && i < shown.length) focusIndex = i
        list.positionViewAtIndex(focusIndex, ListView.Contain)
    }
    // The first Left or Right after the page opens changes nothing (it is often a press carried over).
    property bool fresh: true
    onVisibleChanged: if (visible) fresh = true
    function handleButton(b) {
        if (b === "left" || b === "right") { if (fresh) { fresh = false; return } }
        else fresh = false
        if (b === "up" || b === "down") move(b === "down" ? 1 : -1)
        else if (b === "lt" || b === "rt") { for (let k = 0; k < 6; k++) move(b === "rt" ? 1 : -1) }
        else if (b === "left" || b === "right") change(shown[focusIndex], b === "right" ? 1 : -1)
        else if (b === "a") activate(shown[focusIndex])
        else if (b === "start") saveClicked()
        else if (b === "x") optionsClicked()
        // Changes save themselves; B only goes back (Main.qml saves anything still waiting first).
        else if (b === "b") closeClicked()
    }
    onShownChanged: if (focusIndex >= shown.length) focusIndex = Math.max(0, shown.length - 1)

    Scrim { ui: se.ui }
    Card {
        ui: se.ui
        id: box
        anchors.centerIn: parent
        width: Math.min(parent.width - se.ui.px(32), se.ui.px(980))
        // The sheet layer already ends above the bottom bar.
        height: parent.height - se.ui.px(32)
        clip: true
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
            text: (se.coop ? "Every co-op player must use the same password and the same settings. " : "")
                  + (se.changes > 0 ? "Saving..." : se.sub + " · changes save by themselves")
                  + (se.plain && se.fields.length === 0 ? "  Turbine can't read this file as settings: here it is line by line." : "")
            color: se.leaveArmed ? se.ui.c.warn : se.ui.c.soft
            font.family: se.ui.bodyFont; font.pixelSize: se.ui.fpx(14)
        }
        ListView {
            id: list
            ScrollFade { ui: se.ui; view: list; color: se.ui.c.panel }
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
                    font.family: se.ui.bodyFont; font.pixelSize: se.ui.fpx(14); font.bold: true
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
                        text: row.f ? se.nameOf(row.f.key) : row.modelData.kind === "line" ? "Line " + row.modelData.n : row.modelData.kind === "action" ? row.modelData.title : ""
                        elide: Text.ElideRight
                        color: se.ui.c.text
                        font.family: se.ui.bodyFont; font.pixelSize: se.ui.fpx(15); font.bold: true
                    }
                    Text {
                        width: parent.width
                        visible: text !== ""
                        text: row.f ? row.f.help : row.modelData.kind === "line" ? row.modelData.text : row.modelData.kind === "action" ? row.modelData.detail : ""
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
                    // A row that does something: its button.
                    Rectangle {
                        visible: row.modelData.kind === "action"
                        width: Math.max(se.ui.tpx(64), actionText.implicitWidth + se.ui.px(24)); height: se.ui.tpx(34); radius: se.ui.px(6)
                        color: se.ui.c.raised; border.color: se.ui.c.accent
                        Text { id: actionText; anchors.centerIn: parent; text: row.modelData.kind === "action" ? row.modelData.label : ""; color: se.ui.c.text; font.family: se.ui.bodyFont; font.pixelSize: se.ui.fpx(14); font.bold: true }
                        TapHandler { onTapped: { se.focusIndex = row.index; se.activate(row.modelData) } }
                    }
                    // Toggle: one pill, On or Off.
                    Rectangle {
                        visible: row.f !== null && row.f.kind === "toggle"
                        readonly property bool on: row.f !== null && row.val === row.f.on
                        width: se.ui.tpx(64); height: se.ui.tpx(34); radius: height / 2
                        color: on ? se.ui.c.accent : se.ui.c.raised
                        Behavior on color { enabled: se.ui.motion.on; ColorAnimation { duration: se.ui.motion.fast; easing.type: se.ui.motion.enter } }
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
                                Behavior on width { enabled: se.ui.motion.moves; NumberAnimation { duration: se.ui.motion.moveFast; easing.type: se.ui.motion.enter } }
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
                model: [{ k: "start", label: "Save" }, { k: "x", label: "Options" }, { k: "b", label: "Back" }]
                // Each with its key (Start saves, X options, B back); Save is the main one once there is a change.
                ActionButton {
                    required property var modelData
                    ui: se.ui
                    label: modelData.label
                    cap: modelData.k
                    primary: modelData.k === "start" && se.changes > 0
                    onClicked: se.handleButton(modelData.k)
                }
            }
        }
    }
}
