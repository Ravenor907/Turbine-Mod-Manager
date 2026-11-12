import QtQuick
import Turbine

// The Settings page: the sections on the left (LB and RB move between them, a tap picks one) and the section's rows
// on the right. Look's rows are choices, accent swatches and sliders; About has the app icon (ten presses turn Labs
// mode on or off) and the version; Getting mods, Updates, Labs and Help share one list of rows (actions, choices,
// switches and Labs' checklist). What the rows are and what A does on them is Main.qml's (lookRows, rowsNow,
// rowActions).
//
// app (required): Main.qml's window. kit (required): Main.qml's kit (colors and motion, see COMPONENTS.md). look
// (required): the Look settings the Look rows show and change.
Row {
    id: page
    required property var app
    required property var kit
    required property var look
    readonly property var c: kit.c
    readonly property Motion mo: kit.motion
    // Main.qml scrolls the rows, and the build's checks reach them.
    property alias lookList: lookList
    property alias labsList: labsList
    visible: app.page === "settings"
    anchors.fill: parent
    spacing: app.px(14)
    // The section on the right comes in when LB, RB or a tap picks another: a fade and a short rise from the side of
    // the list it was picked from (Arrive.qml).
    property Arrive sectionIn: Arrive { motion: page.mo }
    readonly property int sectionNow: app.setSection
    property int sectionWas: 0
    onSectionNowChanged: { if (visible) sectionIn.play(0, (sectionNow < sectionWas ? -1 : 1) * app.px(mo.nudge)); sectionWas = sectionNow }
    Column {
        width: app.px(170)
        spacing: app.px(4)
        // LB and RB move between the sections (caps at both ends of the list).
        Key { win: app; k: "lb"; anchors.horizontalCenter: parent.horizontalCenter }
        Repeater {
            model: app.settingsSections
            Rectangle {
                required property int index
                required property string modelData
                width: parent.width; height: app.px(40); radius: app.px(6)
                color: app.setSection === index ? c.raised : "transparent"
                Behavior on color { enabled: mo.on; ColorAnimation { duration: mo.quick; easing.type: mo.ease } }
                Rectangle { visible: app.setSection === parent.index; width: app.px(3); height: parent.height; color: c.accent; radius: app.px(2) }
                Label { win: app; x: app.px(14); anchors.verticalCenter: parent.verticalCenter; text: parent.modelData; font.pixelSize: app.fpx(16); font.bold: app.setSection === parent.index; color: app.setSection === parent.index ? c.text : c.muted }
                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                TapHandler { onTapped: { if (!app.gate(0, "setSection")) return; { app.setSection = parent.index; app.setRow = 0 } } }
            }
        }
        Key { win: app; k: "rb"; anchors.horizontalCenter: parent.horizontalCenter }
    }
    // Look
    ListView {
        id: lookList
        width: parent.width - app.px(184)
        height: parent.height
        visible: app.sec === "Look"
        opacity: page.sectionIn.part
        transform: page.sectionIn
        clip: true
        spacing: app.px(4)
        model: app.lookRows
        currentIndex: app.sec === "Look" ? app.setRow : 0
        highlightFollowsCurrentItem: false
        onCurrentIndexChanged: app.glide(lookList, () => app.reveal(lookList, lookList.currentIndex))
        ScrollFade { ui: page.kit; view: lookList; color: page.c.ground }
        delegate: Rectangle {
            id: srow
            required property int index
            required property var modelData
            readonly property bool foc: app.zone === "settings" && app.setRow === index
            readonly property var now: srow.modelData.now !== undefined ? srow.modelData.now : look[srow.modelData.key]
            // For the build's size check.
            readonly property Item minus: minusBtn
            readonly property Item plus: plusBtn
            width: lookList.width
            height: srow.modelData.slider ? Math.max(app.px(62), app.tpx(36) + app.fpx(13) + app.px(16)) : app.px(50)
            radius: app.px(6)
            color: foc ? c.focusFill : c.card
            Behavior on color { enabled: mo.on; ColorAnimation { duration: mo.quick; easing.type: mo.ease } }
            border.color: c.accent
            border.width: foc ? app.px(2) : 0
            opacity: srow.modelData.dim ? 0.6 : 1
            Label { win: app; x: app.px(14); width: app.px(146); anchors.verticalCenter: parent.verticalCenter; text: srow.modelData.label; font.pixelSize: app.fpx(16); font.bold: true; wrapMode: Text.WordWrap; elide: Text.ElideNone; maximumLineCount: 2 }
            Item {
                x: app.px(166); width: parent.width - app.px(180); height: parent.height
                // Choices
                Row {
                    visible: !srow.modelData.swatch && !srow.modelData.slider
                    anchors.verticalCenter: parent.verticalCenter
                    width: parent.width
                    spacing: app.px(4)
                    Repeater {
                        model: srow.modelData.names || []
                        Rectangle {
                            required property int index
                            required property string modelData
                            readonly property bool on: srow.now === srow.modelData.options[index]
                            // The one chosen: an accent outline and accent words (the same chips as the other sections).
                            width: (parent.width - app.px(4) * ((srow.modelData.names || []).length - 1)) / (srow.modelData.names || [1]).length
                            height: app.px(32); radius: kit.radiusSmall
                            color: on ? Qt.tint(c.card, Qt.rgba(c.accent.r, c.accent.g, c.accent.b, 0.12)) : c.ground
                            border.color: on ? c.accent : c.line
                            border.width: Math.max(1, app.px(on ? 1.5 : 1))
                            Behavior on color { enabled: mo.on; ColorAnimation { duration: mo.quick; easing.type: mo.ease } }
                            Label { win: app; anchors.centerIn: parent; text: parent.modelData; font.pixelSize: app.fpx(14); font.bold: parent.on; color: parent.on ? c.accentText : c.muted }
                            HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                            TapHandler {
                                onTapped: {
                                    if (!app.gate(0, "look choice " + srow.modelData.key) || !app.tapOnce()) return
                                    app.setRow = srow.index
                                    const v = srow.modelData.options[parent.index]
                                    if (srow.modelData.key === "layout") app.layoutPreview(v)
                                    else if (srow.modelData.key === "season") { look.season = v; app.note("action", "Look: season " + v) }
                                    else { look[srow.modelData.key] = v; app.note("action", "Look: " + srow.modelData.key + " " + v) }
                                }
                            }
                        }
                    }
                }
                // Accent swatches
                Row {
                    visible: srow.modelData.swatch === true
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: app.px(10)
                    Repeater {
                        model: ["brass", "iron", "copper", "jade", "fjord", "heather", "ember"]
                        Rectangle {
                            required property string modelData
                            width: app.px(24); height: width; radius: width / 2
                            anchors.verticalCenter: parent.verticalCenter
                            color: c.swatches[modelData]
                            border.color: c.text
                            border.width: look.accent === modelData ? app.px(2) : 0
                            HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                            TapHandler { onTapped: { if (!app.gate(0, "tapOnce")) return; { if (!app.tapOnce()) return; app.setRow = srow.index; look.accent = parent.modelData; app.note("action", "Look: accent " + look.accent) } } }
                        }
                    }
                    Label { win: app; text: look.accent.charAt(0).toUpperCase() + look.accent.substring(1); color: c.soft; font.pixelSize: app.fpx(14); anchors.verticalCenter: parent.verticalCenter }
                }
                // Sliders: Interface size, Seasonal amount and size. Minus and plus at the ends, and a line
                // under it saying Left and Right change it (A steps up, then wraps back down).
                Row {
                    id: sliderRow
                    visible: srow.modelData.slider === true
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.verticalCenterOffset: -app.px(8)
                    width: parent.width
                    spacing: app.px(8)
                    // Real buttons, at least 44 pixels, for a tap, a click or the pointer.
                    Rectangle {
                        id: minusBtn
                        objectName: "sizeMinus"
                        width: app.tpx(36); height: width; radius: app.px(6)
                        anchors.verticalCenter: parent.verticalCenter
                        color: c.raised; border.color: c.line; border.width: Math.max(1, app.px(1))
                        Label { win: app; anchors.centerIn: parent; text: "\u2212"; color: c.text; font.pixelSize: app.fpx(22); font.bold: true }
                        Accessible.role: Accessible.Button
                        Accessible.name: srow.modelData.label + ": smaller"
                        HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                        TapHandler { onTapped: { if (!app.gate(0, "tapOnce")) return; { if (!app.tapOnce()) return; app.setRow = srow.index; app.zone = "settings"; app.note("action", srow.modelData.label + ": - tapped"); srow.modelData.step(-1) } } }
                    }
                    Rectangle {
                        id: track
                        readonly property real part: srow.modelData.slider ? (srow.now - srow.modelData.min) / (srow.modelData.max - srow.modelData.min) : 0
                        // What the buttons and the value leave (the value's widest, "150%", measured).
                        width: Math.max(app.px(40), parent.width - minusBtn.width - plusBtn.width - valueMetrics.advanceWidth - parent.spacing * 3 - app.px(4))
                        height: app.px(6); radius: height / 2
                        anchors.verticalCenter: parent.verticalCenter
                        color: Qt.rgba(c.muted.r, c.muted.g, c.muted.b, 0.35)
                        TextMetrics { id: valueMetrics; font.family: app.bodyFont; font.pixelSize: app.fpx(15); font.bold: true; text: srow.modelData.max + (srow.modelData.unit || "") }
                        // The slider's fill and knob glide to the new value.
                        Rectangle {
                            width: parent.width * track.part; height: parent.height; radius: parent.radius; color: c.accent
                            Behavior on width { enabled: mo.moves; NumberAnimation { duration: mo.moveQuick; easing.type: mo.glide } }
                        }
                        Rectangle {
                            x: parent.width * track.part - width / 2
                            Behavior on x { enabled: mo.moves; NumberAnimation { duration: mo.moveQuick; easing.type: mo.glide } }
                            anchors.verticalCenter: parent.verticalCenter
                            width: app.px(18); height: width; radius: width / 2; color: c.accentHi
                        }
                    }
                    Rectangle {
                        id: plusBtn
                        objectName: "sizePlus"
                        width: app.tpx(36); height: width; radius: app.px(6)
                        anchors.verticalCenter: parent.verticalCenter
                        color: c.raised; border.color: c.line; border.width: Math.max(1, app.px(1))
                        Label { win: app; anchors.centerIn: parent; text: "+"; color: c.text; font.pixelSize: app.fpx(22); font.bold: true }
                        Accessible.role: Accessible.Button
                        Accessible.name: srow.modelData.label + ": bigger"
                        HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                        TapHandler { onTapped: { if (!app.gate(0, "tapOnce")) return; { if (!app.tapOnce()) return; app.setRow = srow.index; app.zone = "settings"; app.note("action", srow.modelData.label + ": + tapped"); srow.modelData.step(1) } } }
                    }
                    Label { win: app; text: srow.now + (srow.modelData.unit || ""); color: c.accentText; font.bold: true; anchors.verticalCenter: parent.verticalCenter }
                }
                Label {
                    win: app
                    visible: srow.modelData.slider === true
                    anchors.top: sliderRow.bottom
                    anchors.topMargin: app.px(2)
                    text: "Left smaller · Right bigger"
                    color: c.muted
                    font.pixelSize: app.fpx(13)
                }
            }
            HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
            TapHandler { onTapped: { if (!app.gate(0, "rowA")) return; app.rowA(() => { app.setRow = srow.index; app.zone = "settings" }, true) } }
        }
    }
    // About: the app icon (ten presses turn Labs mode on or off), version, the scrolling test.
    Column {
        width: parent.width - app.px(184)
        spacing: app.px(8)
        visible: app.sec === "About"
        opacity: page.sectionIn.part
        transform: page.sectionIn
        Row {
            spacing: app.px(14)
            Rectangle {
                readonly property bool foc: app.zone === "settings" && app.setRow === 0
                width: app.px(76); height: width; radius: app.px(10)
                color: foc ? c.focusFill : "transparent"
                Behavior on color { enabled: mo.on; ColorAnimation { duration: mo.quick; easing.type: mo.ease } }
                border.color: c.accent
                border.width: foc ? app.px(2) : 0
                Image {
                    anchors.centerIn: parent
                    width: app.px(64); height: width
                    source: app.iconFull
                    sourceSize.width: app.px(128)
                    smooth: true
                    mipmap: true
                }
                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                TapHandler { onTapped: { if (!app.gate(0, "setRow")) return; { app.setRow = 0; app.labsTap() } } }
            }
            Column {
                anchors.verticalCenter: parent.verticalCenter
                spacing: app.px(4)
                Label { win: app; text: "Turbine Mod Manager " + Turbine.version; font.pixelSize: app.fpx(19); font.weight: Font.DemiBold }
                Label { win: app; visible: app.labsOn; text: "Labs mode on"; color: c.accentText; font.pixelSize: app.fpx(15); font.bold: true }
            }
        }
        // Only in Game Mode; in Desktop Mode no controller is normal and nothing is said.
        Label { win: app; visible: !Turbine.pads && Turbine.gamemode; width: parent.width; wrapMode: Text.WordWrap; elide: Text.ElideNone; color: c.muted; font.pixelSize: app.fpx(14); text: "Using Steam's controller settings (no controller seen directly)." }
        Label { win: app; width: parent.width; wrapMode: Text.WordWrap; elide: Text.ElideNone; color: c.soft; text: "A mod manager for SteamOS handhelds. Free software under the GNU General Public License, version 3 or later. Licenses of the parts it uses are in the licenses folder of the project." }
    }
    // Labs: Report a bug, Performance check, and the checklist of things to confirm on the device.
    ListView {
        id: labsList
        width: parent.width - app.px(184)
        height: parent.height
        visible: app.sec === "Getting mods" || app.sec === "Updates" || app.sec === "Labs" || app.sec === "Help"
        opacity: page.sectionIn.part
        transform: page.sectionIn
        clip: true
        spacing: app.px(4)
        model: app.rowsNow
        currentIndex: visible ? app.setRow : 0
        highlightFollowsCurrentItem: false
        // The focused row comes fully into view: when it changes, and once the rows are laid out (a new model resets
        // the list's own currentIndex, so the row is read from Main.qml).
        readonly property int focusRow: visible ? app.setRow : -1
        onFocusRowChanged: { app.reveal(labsList, focusRow); Qt.callLater(() => app.reveal(labsList, labsList.focusRow)) }
        onHeightChanged: if (visible) Qt.callLater(() => app.reveal(labsList, labsList.focusRow))
        onContentHeightChanged: if (visible && !moving) Qt.callLater(() => app.reveal(labsList, labsList.focusRow))
        ScrollFade { ui: page.kit; view: labsList; color: page.c.ground }
        // After the left stick's wheel: the focus comes into view (a row with only words is passed).
        onMovementEnded: {
            const top = indexAt(10, contentY + app.px(10)), bottom = indexAt(10, contentY + height - app.px(10))
            if (top >= 0 && app.setRow < top) app.setRow = app.settingsLand(top, 1, app.rowsNow.length)
            else if (bottom >= 0 && app.setRow > bottom) app.setRow = app.settingsLand(bottom, -1, app.rowsNow.length)
        }
        onVisibleChanged: if (visible) Qt.callLater(() => { positionViewAtBeginning(); app.reveal(labsList, focusRow) })
        delegate: Item {
            id: lrow
            required property int index
            required property var modelData
            readonly property bool foc: app.zone === "settings" && app.setRow === index
            readonly property bool isItem: modelData.kind === "item"
            readonly property bool hasChips: isItem || modelData.kind === "choice" || modelData.kind === "toggle"
            readonly property string answer: isItem ? app.labsState(modelData.id) : ""
            width: labsList.width
            height: (modelData.head ? labsHead.height : 0) + box.height
            Label {
                id: labsHead
                win: app
                visible: !!lrow.modelData.head
                height: visible ? app.px(34) : 0
                x: app.px(6)
                verticalAlignment: Text.AlignBottom
                bottomPadding: app.px(6)
                text: (lrow.modelData.head || "") + (lrow.modelData.head && lrow.isItem ? " · " + app.labsDone : "")
                color: c.accentText
                font.pixelSize: app.fpx(14)
                font.bold: true
            }
            Rectangle {
                id: box
                y: labsHead.height
                width: parent.width
                // The chips or the action go under the words when they would take too much of the row (a large
                // Interface size): the words are never cut.
                readonly property real ctlW: lrow.hasChips ? chips.implicitWidth : actionHint.visible ? actionHint.width : 0
                readonly property bool stacked: ctlW > 0 && ctlW > (width - app.px(28)) * 0.4
                height: Math.max(lrow.isItem ? app.px(66) : app.px(50), labsWords.implicitHeight + app.px(18) + (stacked ? app.px(32) + app.px(8) : 0))
                radius: app.px(6)
                color: lrow.foc ? c.focusFill : c.card
                Behavior on color { enabled: mo.on; ColorAnimation { duration: mo.quick; easing.type: mo.ease } }
                border.color: c.accent
                border.width: lrow.foc ? app.px(2) : 0
                Column {
                    id: labsWords
                    x: app.px(14)
                    y: box.stacked ? app.px(9) : (parent.height - height) / 2
                    width: parent.width - app.px(28) - (box.stacked || box.ctlW === 0 ? 0 : box.ctlW + app.px(10))
                    spacing: app.px(2)
                    Label { win: app; width: parent.width; text: lrow.modelData.title; font.pixelSize: app.fpx(16); font.bold: true; wrapMode: Text.WordWrap; elide: Text.ElideNone }
                    Label { win: app; width: parent.width; text: lrow.modelData.how; color: c.soft; font.pixelSize: app.fpx(14); wrapMode: Text.WordWrap; elide: Text.ElideNone }
                }
                // The row's action as a chip (like -10% and +10%), with A's cap on it while the row is highlighted.
                Rectangle {
                    id: actionHint
                    visible: lrow.modelData.kind === "action" && (lrow.modelData.action || "") !== ""
                    x: box.stacked ? app.px(14) : parent.width - width - app.px(10)
                    y: box.stacked ? labsWords.y + labsWords.height + app.px(8) : (parent.height - height) / 2
                    width: actionRow.implicitWidth + app.px(20); height: app.px(32); radius: kit.radiusSmall
                    color: c.ground
                    border.color: lrow.foc ? c.accent : c.line
                    border.width: Math.max(1, app.px(1))
                    Row {
                        id: actionRow
                        anchors.centerIn: parent
                        spacing: app.px(6)
                        Key { win: app; k: "a"; visible: lrow.foc; anchors.verticalCenter: parent.verticalCenter }
                        Label { win: app; text: lrow.modelData.action || ""; font.pixelSize: app.fpx(14); font.bold: lrow.foc; color: lrow.foc ? c.text : c.soft; anchors.verticalCenter: parent.verticalCenter }
                    }
                }
                Row {
                    id: chips
                    visible: lrow.hasChips
                    x: box.stacked ? app.px(14) : parent.width - width - app.px(10)
                    y: box.stacked ? labsWords.y + labsWords.height + app.px(8) : (parent.height - height) / 2
                    spacing: app.px(4)
                    Repeater {
                        model: lrow.isItem ? [["works", "Works"], ["problem", "Problem"], ["", "Not tested"]]
                               : lrow.hasChips ? lrow.modelData.options.map((o, i) => [o, lrow.modelData.names[i]]) : []
                        Rectangle {
                            required property int index
                            required property var modelData
                            readonly property bool on: lrow.isItem ? lrow.answer === modelData[0] : lrow.modelData.value === modelData[0]
                            readonly property bool cursor: lrow.isItem && lrow.foc && app.labsChip === index
                            width: chipText.implicitWidth + app.px(18); height: app.px(32); radius: kit.radiusSmall
                            color: on ? Qt.tint(c.card, Qt.rgba(c.accent.r, c.accent.g, c.accent.b, 0.12)) : c.ground
                            border.color: on ? c.accent : c.line
                            border.width: Math.max(1, app.px(on ? 1.5 : 1))
                            // The chip the D-pad is on (Labs' checklist): a ring around it.
                            Rectangle {
                                visible: parent.cursor
                                anchors.fill: parent
                                anchors.margins: -app.px(3)
                                radius: parent.radius + app.px(3)
                                color: "transparent"
                                border.width: kit.ringWidth
                                border.color: kit.ring
                            }
                            Label {
                                id: chipText
                                win: app
                                anchors.centerIn: parent
                                text: parent.modelData[1]
                                font.pixelSize: app.fpx(14)
                                font.bold: parent.on
                                color: !parent.on ? c.muted : !lrow.isItem ? c.accentText : parent.index === 0 ? c.ok : parent.index === 1 ? c.stop : c.text
                            }
                            HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                            TapHandler {
                                onTapped: {
                                    if (!app.gate(0, "labs choice") || !app.tapOnce()) return
                                    app.setRow = lrow.index
                                    if (lrow.isItem) { app.labsChip = parent.index; app.labsChoose(lrow.modelData, parent.modelData[0]) }
                                    else if (!parent.on) app.rowStep(lrow.modelData, lrow.modelData.key === "zoom" && parent.index === 0 ? -1 : 1)
                                }
                            }
                        }
                    }
                }
                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                TapHandler { onTapped: { if (!app.gate(0, "settingsFocusable")) return; { if (app.settingsFocusable(lrow.index)) app.rowA(() => { app.setRow = lrow.index; app.zone = "settings" }, true) } } }
            }
        }
    }
}
