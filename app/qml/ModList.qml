import QtQuick

// My mods: the list of the game's mods (ModRow), with the rows that aren't mods: one just removed (dimmed for 8
// seconds, A puts it back), mods held back until their updates, Dependencies, requirements still to get and Removed
// mods. A click or tap on a row is A on it; a right-click opens its menu. Main.qml places it beside the details
// panel.
//
// app (required): Main.qml's window. kit (required): Main.qml's kit (see COMPONENTS.md). look (required): the Look
// settings (Shorter rows).
ListView {
    id: modList
    required property var app
    required property var kit
    required property var look
    readonly property var c: kit.c
    readonly property Motion mo: kit.motion
    visible: app.page === "game" && app.game !== null && app.tab === 0 && app.modsLoaded && !app.modPage
    z: app.turnTo === 0 ? 3 : 0
    transform: Translate { x: app.turnTo === 0 ? app.turnX : 0 }
    // The bars sliding away or back change the list's height: the focused row stays fully in view.
    onHeightChanged: if (app.modIndex >= 0) app.reveal(modList, app.modIndex)
    // A touch or wheel scroll hides the bars going down and brings them back going up or at the top.
    property real lastY: 0
    onContentYChanged: { if (moving) app.scrollFollow(contentY + topMargin, lastY + topMargin, contentHeight > height + app.px(40)); lastY = contentY }
    topMargin: app.px(4)
    bottomMargin: app.px(4)
    ScrollFade { ui: modList.kit; view: modList; color: modList.c.ground }
    clip: true
    spacing: app.px(app.desktopLayout ? 3 : 6)
    model: app.listItems
    // The load order numbers of the mods in the closed Dependencies group, said on its row ("load order 1, 4") so the
    // numbers missing from the list above read clearly. Empty when numbers don't show or the group is open.
    readonly property string depsOrderWords: {
        if (!app.ordered || app.listSort !== "order" || app.depsOpen) return ""
        const n = app.depMods.map(m => app.mods.findIndex(x => x.id === m.id) + 1).filter(x => x > 0).sort((a, b) => a - b)
        return n.length > 0 ? "load order " + n.join(", ") : ""
    }
    currentIndex: Math.max(0, app.modIndex)
    highlightFollowsCurrentItem: false
    onCurrentIndexChanged: app.glide(modList, () => app.reveal(modList, modList.currentIndex))
    // After the left stick scrolls, focus the nearest row still in view.
    onMovementEnded: {
        const top = indexAt(10, contentY + app.px(10)), bottom = indexAt(10, contentY + height - app.px(10))
        if (top >= 0 && app.modIndex < top) app.modIndex = top
        else if (bottom >= 0 && app.modIndex > bottom) app.modIndex = bottom
    }
    delegate: Item {
        id: entry
        required property int index
        required property var modelData
        readonly property string kind: modelData.kind
        readonly property var m: modelData.m
        readonly property bool foc: app.zone === "main" && app.sheet === "" && app.modIndex === index
        readonly property Item pill: row.pillItem
        width: modList.width
        // Just moved (it slides in from where it was) or put back (it fades in).
        transform: Translate { id: entryShift; y: 0 }
        NumberAnimation { id: entrySlide; target: entryShift; properties: "y"; to: 0; duration: mo.moveFast; easing.type: mo.enter }
        NumberAnimation { id: entryFade; target: entry; properties: "opacity"; from: 0; to: 1; duration: mo.base; easing.type: mo.enter }
        Component.onCompleted: {
            const n = app.rowNudge
            if (!n || !entry.m || n.id !== entry.m.id || Date.now() - n.at > 400) return
            if (n.dir === 0) entryFade.start()
            else if (mo.moves) { entryShift.y = n.dir * (entry.height + modList.spacing); entrySlide.start() }
        }
        height: kind === "mod" ? row.implicitHeight : kind === "held" ? Math.max(app.touchFloor + app.px(8), app.px(52)) : app.px(app.desktopLayout ? 38 : 46)
        ModRow {
            id: row
            visible: entry.kind === "mod"
            ui: kit
            // Rows inside the open Dependencies group sit a little in.
            x: app.px(entry.modelData.dep ? 24 : 4)
            width: parent.width - app.px(entry.modelData.dep ? 28 : 8)
            name: entry.m.name || ""
            detail: entry.kind === "mod" ? app.rowLine(entry.m).text : ""
            detailLevel: entry.kind === "mod" ? app.rowLine(entry.m).level : ""
            isOn: entry.m.enabled === true
            ordered: app.ordered && app.listSort === "order"
            compact: app.desktopLayout || look.compactList
            order: entry.kind === "mod" ? app.mods.findIndex(x => x.id === entry.m.id) + 1 : 0
            position: entry.index + 1
            count: app.shownMods.length
            ticked: app.selecting && entry.kind === "mod" ? (app.ticked[entry.m.id] ? 1 : 0) : -1
            focused: entry.foc
            // Problems are the row's second line (rowLine); the badge is for Moving, an update and New.
            badge: app.moving && entry.foc ? "Moving" : app.updateMap[entry.m.id] && !app.updateMap[entry.m.id].deprecated ? "Update ready"
                   : entry.kind === "mod" && app.statusOf(entry.m) !== "" ? app.statusOf(entry.m)
                   : entry.kind === "mod" && app.isNew(entry.m) ? "New" : ""
            badgeLevel: entry.kind === "mod" && !app.moving && app.statusOf(entry.m) !== "" && !(app.updateMap[entry.m.id] && !app.updateMap[entry.m.id].deprecated) ? "warn" : "info"
            // A click or tap anywhere on the row is A on it (turns it on or off; ticks it in Select several).
            onClicked: { if (!app.gate(0, "onClicked")) return; if (app.moving && entry.kind === "mod") { app.moveByTap(entry.index); return } if (entry.kind === "mod" && !app.selecting && app.rowClickSeen(entry.index, entry.m)) return; app.rowA(() => { if (!app.moving) app.modIndex = entry.index }) }
            // A right-click (LT with the pointer) opens the mod's menu, like X.
            onMenuClicked: { if (!app.gate(0, "onMenuClicked")) return; if (app.sheet === "" && !app.moving) { app.modIndex = entry.index; app.zone = "main"; app.sheet = "modmenu" } }
        }
        Rectangle {
            visible: entry.kind !== "mod"
            x: app.px(4)
            width: parent.width - app.px(8)
            height: parent.height
            radius: app.px(6)
            // Removed (n) at the bottom; a mod just uninstalled stays dimmed in its place for 8 seconds.
            color: entry.foc ? c.focusFill : "transparent"
            border.color: entry.foc ? c.accent : c.line
            border.width: entry.foc ? Math.max(1, app.px(2)) : Math.max(1, app.px(1))
            opacity: entry.kind === "ghost" && !entry.foc ? 0.6 : 1
            // The groups (Dependencies, Removed mods) are folders: a folder icon and a chevron that turns open.
            readonly property bool group: entry.kind === "deps" || entry.kind === "removed"
            Icon {
                id: groupIcon
                win: app
                visible: parent.group
                x: app.px(12)
                anchors.verticalCenter: parent.verticalCenter
                d: kit.icons.folder
                tint: entry.foc ? c.accentText : c.muted
            }
            Label {
                win: app
                x: parent.group ? groupIcon.x + groupIcon.width + app.px(10) : app.px(14)
                anchors.verticalCenter: parent.verticalCenter
                width: parent.width - x - app.px(14) - (heldButton.visible ? heldButton.width + app.px(12) : 0) - (groupChevron.visible ? groupChevron.width + app.px(8) : 0)
                       - (depsOrder.visible ? depsOrder.implicitWidth + app.px(12) : 0)
                elide: Text.ElideRight
                text: entry.kind === "ghost" ? "Removed: " + entry.m.name + ". A puts it back."
                      : entry.kind === "held" ? app.heldWords
                      : entry.kind === "deps" ? "Dependencies (" + app.depMods.length + ") · what your mods need"
                      : entry.kind === "needsq" && app.reqQ ? app.reqQ.forName + " still needs " + app.plural(app.reqQ.items.length - app.reqQ.index, "mod", "mods") + " · A goes on where you left off"
                      : "Removed mods (" + app.removedMods.length + ") · kept 7 days"
                color: entry.kind === "ghost" ? c.soft : entry.kind === "held" || entry.kind === "needsq" ? c.warn : parent.group ? c.text : c.muted
                font.pixelSize: app.fpx(15)
                font.weight: parent.group ? Font.DemiBold : Font.Normal
            }
            Label {
                id: depsOrder
                win: app
                visible: entry.kind === "deps" && modList.depsOrderWords !== ""
                anchors.right: groupChevron.left
                anchors.rightMargin: app.px(10)
                anchors.verticalCenter: parent.verticalCenter
                text: modList.depsOrderWords
                color: c.muted
                font.pixelSize: app.fpx(14)
            }
            Icon {
                id: groupChevron
                win: app
                visible: parent.group
                anchors.right: parent.right
                anchors.rightMargin: app.px(12)
                anchors.verticalCenter: parent.verticalCenter
                d: kit.icons.forward
                rotation: entry.kind === "deps" && app.depsOpen ? 90 : 0
                tint: entry.foc ? c.accentText : c.muted
            }
            // The held note's way back, on the row itself (a tap anywhere on the row does it too).
            Rectangle {
                id: heldButton
                visible: entry.kind === "held"
                anchors.right: parent.right
                anchors.rightMargin: app.px(6)
                anchors.verticalCenter: parent.verticalCenter
                height: parent.height - app.px(8)
                width: heldButtonText.implicitWidth + app.px(24)
                radius: app.px(6)
                color: c.accent
                Label {
                    id: heldButtonText
                    win: app
                    anchors.centerIn: parent
                    text: "Turn them back on"
                    color: c.onAccent
                    font.pixelSize: app.fpx(15)
                    font.bold: true
                }
            }
            HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
            TapHandler { onTapped: { if (!app.gate(0, "rowA")) return; app.rowA(() => { app.modIndex = entry.index }) } }
        }
    }
}
