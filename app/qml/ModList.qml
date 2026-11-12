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
    visible: app.page === "game" && app.game !== null && app.onTab(0) && app.modsLoaded && !app.modPage
    z: app.turnTo === 0 ? 3 : 0
    // Showing up on its own (the mods read after a moment), the list fades in.
    onVisibleChanged: if (visible) listIn.appear(() => modList.kit.arriving, 0, 0)
    opacity: app.tabFade(0) * listIn.part
    // Fading out as another tab comes in, it takes no taps (they reach the page coming in).
    enabled: app.turnFrom !== 0
    transform: [Translate { x: app.tabShift(0) }, Arrive { id: listIn; motion: modList.mo }]
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
    // Rows that change: the list is built again on every change (Main.qml's listItems), so each row is told where it
    // was. At Full a row whose place changed (moved, or pushed by a row that came or went) glides from where it was;
    // a row that is new fades in where it is, and a mod uninstalled or put back fades over in place. A pill that
    // turned on or off fades from its last look. Not when the list is another (a game switch, a new find): only
    // when at least half of it was there before, and never while its page is coming in.
    readonly property bool flips: mo.on && visible && !kit.arriving
    property var placed: ({})             // the rows on screen now: key -> {y, on}
    property var before: ({})             // the same for the list before this one
    property real builtAt: 0
    property bool flipping: false
    function keyOf(it) { return it.kind + ":" + (it.m && it.m.id !== undefined ? it.m.id : "") }
    // The list makes the new rows before it says its model changed, so the first row of a new list starts it.
    property var shownModel: null
    function newList() {
        const list = app.listItems
        if (list === shownModel) return
        shownModel = list
        before = placed; placed = ({})
        builtAt = Date.now()
        const now = (list || []).map(keyOf)
        let kept = 0
        for (const k of now) if (before[k] !== undefined || before[twin(k)] !== undefined) kept++
        flipping = flips && now.length > 0 && kept * 2 >= now.length
    }
    onModelChanged: newList()
    // A mod's row and its uninstalled row are one place in the list.
    function twin(k) { return k.indexOf("mod:") === 0 ? "ghost:" + k.slice(4) : k.indexOf("ghost:") === 0 ? "mod:" + k.slice(6) : "" }
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
        // Where it was in the list before this one (see flips): it glides or fades from there. Placed as soon as it
        // is made and again when the list puts it in its place, both before anything is drawn.
        readonly property string key: modList.keyOf(modelData)
        opacity: rowIn.part
        transform: Arrive { id: rowIn; motion: mo; fadeTime: mo.standard; moveTime: mo.moveStandard }
        property bool placedOnce: false
        function place() {
            modList.newList()
            const on = entry.kind === "mod" && entry.m.enabled === true
            modList.placed[key] = { y: y, on: on }
            if (!modList.flipping || Date.now() - modList.builtAt > 300) return
            const was = modList.before[key], pair = modList.before[modList.twin(key)]
            rowIn.settle()
            if (was !== undefined) {
                if (Math.abs(was.y - y) > 1) rowIn.play(0, was.y - y, 1)
                if (!placedOnce && entry.kind === "mod" && was.on !== on) row.flipFrom(was.on)
            }
            else if (pair !== undefined) rowIn.play(0, pair.y - y, mo.swapFrom)
            else rowIn.play(0, 0, 0)
            placedOnce = true
        }
        onYChanged: place()
        Component.onCompleted: place()
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
            Behavior on color { enabled: mo.on; ColorAnimation { duration: mo.quick; easing.type: mo.ease } }
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
