import QtQuick

// Installer choices (a mod's FOMOD installer): one question per page, big options, and a preview of the focused one
// (its picture from the archive and its description). The D-pad moves, A picks; in "pick one" questions A also goes
// on. The last row is Next (or Install on the last page). B goes back a page, and on the first page asks to stop.
// LB and RB do nothing here, so a game can't be switched mid-install. Main.qml drives it with handleButton().
Item {
    id: fs
    required property var ui
    property var info: null               // the engine's installer JSON: {game, waiting, data, images}
    property int page: 0                  // which group of the current step
    property int focusIndex: 0
    property bool askStop: false

    signal choose(int group, int option)
    signal next()
    signal back()
    signal stop()

    readonly property var step: info && info.data ? info.data.step : null
    readonly property var groups: step ? step.groups : []
    readonly property var group: groups.length > 0 ? groups[Math.min(page, groups.length - 1)] : null
    readonly property var options: group ? group.options : []
    readonly property bool lastPage: !step || (page >= groups.length - 1 && step.isLast)
    readonly property int rows: options.length + 1
    readonly property var focused: focusIndex < options.length ? options[focusIndex] : null
    readonly property var preview: focused || (options.find(o => o.selected) || null)
    function imageFor(p) { return p && info && info.images && info.images[p] ? "file://" + info.images[p] : "" }

    onInfoChanged: { if (page >= groups.length) page = Math.max(0, groups.length - 1); if (focusIndex >= rows) focusIndex = rows - 1 }
    readonly property int stepIndex: step ? step.index : -1
    onStepIndexChanged: if (!pageAtEnd) { page = 0; focusIndex = 0 }

    function handleButton(b) {
        if (askStop) {
            if (b === "a") { askStop = false; stop() }
            else if (b === "b") askStop = false
            return
        }
        if (b === "up") focusIndex = Math.max(0, focusIndex - 1)
        else if (b === "down") focusIndex = Math.min(rows - 1, focusIndex + 1)
        else if (b === "a") activate(focusIndex)
        else if (b === "b") goBack()
        else if (b === "start" && focusIndex >= options.length) goOn()
    }
    function activate(i) {
        focusIndex = i
        if (i >= options.length) { goOn(); return }
        const o = options[i]
        if (!o.canChange) return
        const pickOne = group.kind === "exactlyOne"
        if (!(pickOne && o.selected)) choose(page, i)
        if (pickOne) goOn()
    }
    function goOn() {
        if (page < groups.length - 1) { page++; focusIndex = 0 }
        else next()
    }
    function goBack() {
        if (page > 0) { page--; focusIndex = 0 }
        else if (step && step.canGoBack) { back(); pageAtEnd = true }
        else askStop = true
    }
    // After going back a step, start at its last question.
    property bool pageAtEnd: false
    onGroupsChanged: if (pageAtEnd && stepIndex !== lastStep) { pageAtEnd = false; page = Math.max(0, groups.length - 1); focusIndex = 0 }
    property int lastStep: -1
    onBack: lastStep = stepIndex

    Rectangle { anchors.fill: parent; color: fs.ui.c.ground }
    MouseArea { anchors.fill: parent }
    Column {
        anchors.fill: parent
        anchors.margins: fs.ui.px(14)
        spacing: fs.ui.px(8)
        Item {
            width: parent.width; height: fs.ui.px(44)
            Rectangle {
                anchors.left: parent.left; anchors.verticalCenter: parent.verticalCenter
                width: backRow.implicitWidth + fs.ui.px(20); height: fs.ui.px(32); radius: fs.ui.px(6)
                color: fs.ui.c.panel
                Row {
                    id: backRow
                    anchors.centerIn: parent
                    spacing: fs.ui.px(6)
                    Rectangle {
                        width: fs.ui.px(24); height: width; radius: width / 2; color: fs.ui.c.raised; border.color: fs.ui.c.accentLo
                        anchors.verticalCenter: parent.verticalCenter
                        Text { anchors.centerIn: parent; text: "B"; color: fs.ui.c.text; font.family: fs.ui.bodyFont; font.pixelSize: fs.ui.fpx(13); font.bold: true }
                    }
                    Text { text: fs.page > 0 || (fs.step && fs.step.canGoBack) ? "Back a step" : "Stop"; color: fs.ui.c.soft; font.family: fs.ui.bodyFont; font.pixelSize: fs.ui.fpx(15); anchors.verticalCenter: parent.verticalCenter }
                }
                HoverHandler { onHoveredChanged: fs.ui.hover(parent, hovered) }
                TapHandler { onTapped: fs.goBack() }
            }
            Text {
                anchors.centerIn: parent
                width: parent.width - fs.ui.px(360)
                horizontalAlignment: Text.AlignHCenter
                elide: Text.ElideRight
                text: fs.info && fs.info.data ? (fs.info.data.name || fs.info.data.module) : ""
                color: fs.ui.c.text
                font.family: fs.ui.titleFont; font.pixelSize: fs.ui.fpx(17); font.bold: true
            }
            Text {
                anchors.right: parent.right; anchors.verticalCenter: parent.verticalCenter
                text: fs.step ? "Step " + fs.step.number + (fs.groups.length > 1 ? ", question " + (fs.page + 1) + " of " + fs.groups.length : "") : ""
                color: fs.ui.c.soft
                font.family: fs.ui.bodyFont; font.pixelSize: fs.ui.fpx(15)
            }
        }
        StatusLine {
            ui: fs.ui
            width: parent.width
            height: fs.ui.px(44)
            level: fs.info && fs.info.data && fs.info.data.warning ? "warn" : "busy"
            words: fs.info && fs.info.data && fs.info.data.warning ? fs.info.data.warning : "Installing " + (fs.info && fs.info.data ? (fs.info.data.name || fs.info.data.module) : "")
            detail: fs.info && fs.info.data && fs.info.data.warning ? "" : "Waiting for your choices"
        }
        Row {
            width: parent.width
            height: parent.height - fs.ui.px(44 + 44 + 16)
            spacing: fs.ui.px(14)
            Column {
                width: parent.width * 0.58
                height: parent.height
                spacing: fs.ui.px(6)
                Row {
                    spacing: fs.ui.px(10)
                    Text { text: fs.group ? fs.group.name : (fs.step ? fs.step.name : ""); color: fs.ui.c.text; font.family: fs.ui.bodyFont; font.pixelSize: fs.ui.fpx(19); font.bold: true }
                    Text { text: fs.group ? fs.group.hint : ""; color: fs.ui.c.soft; font.family: fs.ui.bodyFont; font.pixelSize: fs.ui.fpx(15); topPadding: fs.ui.px(4) }
                }
                ListView {
                    id: optList
                    width: parent.width
                    height: parent.height - fs.ui.px(34)
                    clip: true
                    spacing: fs.ui.px(6)
                    model: fs.rows
                    currentIndex: fs.focusIndex
                    highlightFollowsCurrentItem: false
                    onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                    delegate: Rectangle {
                        id: opt
                        required property int index
                        readonly property bool isNext: index >= fs.options.length
                        readonly property var o: isNext ? null : fs.options[index]
                        readonly property bool foc: fs.focusIndex === index && !fs.askStop
                        readonly property bool many: fs.group && fs.group.kind !== "exactlyOne" && fs.group.kind !== "atMostOne"
                        width: optList.width
                        height: isNext ? fs.ui.px(48) : fs.ui.px(64)
                        radius: fs.ui.px(8)
                        color: isNext ? (foc ? fs.ui.c.accentHi : fs.ui.c.accent) : foc ? fs.ui.c.focusFill : fs.ui.c.card
                        border.color: foc ? (isNext ? fs.ui.c.text : fs.ui.c.accent) : "transparent"
                        border.width: foc ? fs.ui.px(2) : 0
                        opacity: !isNext && o && !o.canChange && !o.selected ? 0.55 : 1
                        // Radio (pick one) or tick box (pick any)
                        Rectangle {
                            visible: !opt.isNext
                            x: fs.ui.px(16); anchors.verticalCenter: parent.verticalCenter
                            width: fs.ui.px(26); height: width
                            radius: opt.many ? fs.ui.px(5) : width / 2
                            color: "transparent"
                            border.color: opt.o && opt.o.selected ? fs.ui.c.accentHi : fs.ui.c.muted
                            border.width: fs.ui.px(2)
                            Rectangle {
                                visible: !!opt.o && opt.o.selected
                                anchors.centerIn: parent
                                width: parent.width * 0.5; height: width
                                radius: opt.many ? fs.ui.px(2) : width / 2
                                color: fs.ui.c.accentHi
                            }
                        }
                        Column {
                            visible: !opt.isNext
                            x: fs.ui.px(56)
                            width: parent.width - fs.ui.px(110)
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: fs.ui.px(2)
                            Text { width: parent.width; elide: Text.ElideRight; text: opt.o ? opt.o.name : ""; color: fs.ui.c.text; font.family: fs.ui.bodyFont; font.pixelSize: fs.ui.fpx(17); font.bold: true }
                            Text {
                                width: parent.width; elide: Text.ElideRight
                                text: !opt.o ? "" : opt.o.type === "required" ? "Always installed" : opt.o.type === "recommended" ? "Recommended by the mod" : opt.o.type === "notUsable" ? "Can't be used with your setup" : (opt.o.description || "").split("\n")[0]
                                color: fs.ui.c.soft; font.family: fs.ui.bodyFont; font.pixelSize: fs.ui.fpx(14)
                            }
                        }
                        Text {
                            visible: opt.isNext
                            anchors.centerIn: parent
                            text: fs.lastPage ? "Install with these choices" : "Next question"
                            color: fs.ui.c.onAccent
                            font.family: fs.ui.bodyFont; font.pixelSize: fs.ui.fpx(16); font.bold: true
                        }
                        Rectangle {
                            visible: opt.foc && !opt.isNext
                            anchors.right: parent.right; anchors.rightMargin: fs.ui.px(14); anchors.verticalCenter: parent.verticalCenter
                            width: fs.ui.px(26); height: width; radius: width / 2; color: fs.ui.c.raised; border.color: fs.ui.c.accentLo
                            Text { anchors.centerIn: parent; text: "A"; color: fs.ui.c.text; font.family: fs.ui.bodyFont; font.pixelSize: fs.ui.fpx(13); font.bold: true }
                        }
                        HoverHandler { onHoveredChanged: fs.ui.hover(parent, hovered) }
                        TapHandler { onTapped: fs.activate(opt.index) }
                    }
                }
            }
            // Preview of the focused option
            Rectangle {
                width: parent.width * 0.42 - fs.ui.px(14)
                height: parent.height
                radius: fs.ui.px(8)
                color: fs.ui.c.panel
                Column {
                    x: fs.ui.px(14); y: fs.ui.px(14)
                    width: parent.width - fs.ui.px(28)
                    spacing: fs.ui.px(8)
                    Rectangle {
                        width: parent.width; height: width * 0.56; radius: fs.ui.px(6)
                        color: fs.ui.c.raised
                        clip: true
                        Image {
                            id: prev
                            anchors.fill: parent
                            source: fs.imageFor(fs.preview ? fs.preview.image : "") || fs.imageFor(fs.info && fs.info.data ? fs.info.data.image : "")
                            fillMode: Image.PreserveAspectCrop
                            asynchronous: true
                            visible: status === Image.Ready
                        }
                        Text {
                            visible: !prev.visible
                            anchors.centerIn: parent
                            text: "No picture"
                            color: fs.ui.c.muted
                            font.family: fs.ui.bodyFont; font.pixelSize: fs.ui.fpx(14)
                        }
                    }
                    Text { width: parent.width; elide: Text.ElideRight; text: fs.preview ? fs.preview.name : ""; color: fs.ui.c.text; font.family: fs.ui.bodyFont; font.pixelSize: fs.ui.fpx(17); font.bold: true }
                    Text {
                        width: parent.width
                        wrapMode: Text.WordWrap
                        maximumLineCount: 8
                        elide: Text.ElideRight
                        textFormat: Text.PlainText
                        text: fs.preview ? fs.preview.description : ""
                        color: fs.ui.c.soft
                        font.family: fs.ui.bodyFont; font.pixelSize: fs.ui.fpx(15)
                    }
                }
            }
        }
    }
    // Stop installing?
    Rectangle {
        visible: fs.askStop
        anchors.centerIn: parent
        width: Math.min(parent.width - fs.ui.px(40), fs.ui.px(480))
        height: stopCol.implicitHeight + fs.ui.px(32)
        radius: fs.ui.px(10)
        color: fs.ui.c.panel
        border.color: fs.ui.c.line
        Column {
            id: stopCol
            x: fs.ui.px(18); y: fs.ui.px(16)
            width: parent.width - fs.ui.px(36)
            spacing: fs.ui.px(10)
            Text { text: "Stop installing?"; color: fs.ui.c.text; font.family: fs.ui.titleFont; font.pixelSize: fs.ui.fpx(18); font.bold: true }
            Text { width: parent.width; wrapMode: Text.WordWrap; text: "The mod isn't added. Nothing in the game changes."; color: fs.ui.c.soft; font.family: fs.ui.bodyFont; font.pixelSize: fs.ui.fpx(15) }
            Text { text: "A  Stop      B  Keep choosing"; color: fs.ui.c.text; font.family: fs.ui.bodyFont; font.pixelSize: fs.ui.fpx(15); font.bold: true }
        }
    }
}
