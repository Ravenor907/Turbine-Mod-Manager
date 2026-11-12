import QtQuick
import Turbine

// Every sheet, menu and prompt in one layer, over the page and under the bottom bar: game details, the mod and game
// menus, Add from Downloads, Before you play, the loader prompt, the installer, versions, pages, profiles, the info
// and confirm sheets, Y's pop-up, Edit settings, Labs' sheets, and the focused game's name beside the rail. Each sheet
// is a component of its own; this file connects each to Main.qml's state and actions. Every tap asks app.gate(2)
// first. The layer is off under the quick wheel and the tutorial (Main.qml's topLayer).
//
// app (required): Main.qml's window. kit (required): Main.qml's kit (see COMPONENTS.md). toast, seen, shot (required):
// Main.qml's toast, seen settings and screenshot settings. rail (required): the game rail (the installer and the
// game's name sit beside it). resetHold, confirmHold, nxmRingMove (required): Main.qml's timers for the holds and the
// Mod Manager Download question.
Item {
    id: sheetLayer
    required property var app
    required property var kit
    required property var toast
    required property var seen
    required property var shot
    required property Item rail
    required property Timer resetHold
    required property Timer confirmHold
    required property Timer nxmRingMove
    readonly property var c: kit.c
    // The sheets Main.qml drives by id.
    property alias askView: askView
    property alias activityView: activityView
    property alias checkView: checkView
    property alias confirmSheet: confirmSheet
    property alias findView: findView
    property alias fomodView: fomodView
    property alias gameMenuView: gameMenuView
    property alias infoView: infoView
    property alias perfView: perfView
    property alias pickMenu: pickMenu
    property alias profilesView: profilesView
    property alias reportView: reportView
    property alias settingsView: settingsView
    enabled: app.topLayer <= 2
    Overlay {
        visible: app.sheet === "details" || app.sheet === "notice"
        motion: kit.motion
        onOutsideTapped: { if (!app.gate(2, "sheet onOutsideTapped")) return; app.act("b") }
        Scrim { ui: kit }

        // Game details: what Turbine found, so the owner can check detection on the device.
        DialogCard {
            visible: app.sheet === "details" && app.game !== null
            ui: kit
            maxWidth: 620
            title: app.game ? app.game.name : ""
            buttons: ["Close"]
            focusIndex: 0
            mainIndex: -1
            onActivated: { if (!app.gate(2, "details Close")) return; app.act("a") }
            Repeater {
                model: app.game ? [
                    ["Store", app.game.storeLabel + (app.game.launcher !== "Steam" ? " (through Heroic)" : "")],
                    ["Runs as", app.runtimeWords(app.game)],
                    ["Stored on", (app.game.launcher === "Steam" ? "Steam library, " : "") + app.game.drive],
                    ["Folder", String(app.game.installDir || "").split("/").filter(x => x !== "").slice(-1)[0] || ""],
                    // A Steam build number means nothing to people; the number stays for reports, in Labs.
                    ["Game version", app.game.launcher === "Steam" ? "" : (app.game.build || "Not reported")],
                ].concat(app.labsOn ? [["Store ID", app.game.storeId], ["Build", app.game.build || "none"], ["Full folder", app.game.installDir]] : []).concat([

                    ["Cover art", app.game.cover || app.game.header ? "Found on the device" : "None cached"],
                    ["Support", !app.rules ? "" : app.rules.supportLevel === "basic" ? "Basic support. Turbine has no rules of its own for this game yet. It puts each mod where its files show they belong, or asks you once."
                                : app.rules.supportLevel === "custom" ? "Added by you (from your games folder)" : "Full support"]
                ]).concat(app.rules && app.rules.supportLevel !== "full" ? [
                    ["Nexus game", app.rules.domain ? (app.rules.nexusName || app.rules.domain) + (app.rules.nexusHow ? " (" + app.rules.nexusHow + ")" : "") : app.rules.domainState === "none" ? "Not on Nexus (your pick)" : "Not found yet"]
                ] : []).concat(app.rules && app.rules.antiCheat ? [
                    ["Anti-cheat", app.rules.antiCheat.name + ". Mods only in offline or single player modes."]
                ] : []).concat(app.checkPaths.length > 0 ? [
                    ["Also checks", app.checkPaths.join("; ")]
                ] : []).concat(app.rules && app.rules.launchOption ? [
                    ["Loader setup", app.rules.launchHelp || ""],
                    ["Launch Option", app.rules.launchOption]
                ] : []) : []
                Row {
                    required property var modelData
                    visible: modelData[1] !== ""
                    width: parent.width
                    spacing: app.px(12)
                    Label { win: app; width: app.px(110); text: parent.modelData[0]; color: c.muted; font.pixelSize: app.fpx(14) }
                    // Words wrap between words; a folder path with no spaces still breaks.
                    Label { win: app; width: parent.width - app.px(122); text: parent.modelData[1]; wrapMode: Text.Wrap; elide: Text.ElideNone; font.pixelSize: app.fpx(15) }
                }
            }
        }

        // Not SteamOS: said once, then Turbine runs as it is.
        DialogCard {
            visible: app.sheet === "notice"
            ui: kit
            maxWidth: 540
            title: "Made for SteamOS"
            buttons: ["OK"]
            focusIndex: 0
            mainIndex: 0
            onActivated: { if (!app.gate(2, "notice OK")) return; app.act("a") }
            Label { win: app; width: parent.width; wrapMode: Text.WordWrap; elide: Text.ElideNone; color: c.soft; text: "This system isn't supported. Turbine Mod Manager is built and tested for SteamOS handhelds only, so some things may not work here." }
        }
    }

    // ---------------------------------------------------------------- menus and prompts (they cover the window)
    ModMenu {
        ui: kit
        visible: app.sheet === "modmenu"
        title: app.menuMod ? app.menuMod.name : ""
        subtitle: !app.menuMod ? ""
                  : [app.modWords(app.menuMod), (app.menuMod.enabled ? "On" : "Off") + (app.ordered ? ", " + app.ordinal(app.mods.indexOf(app.menuMod) + 1) + " in load order" : "")].filter(s => s !== "").join(" · ")
        items: app.modMenuItems.map(m => ({ label: m.title, detail: m.detail, icon: m.icon, danger: m.danger === true }))
        focusIndex: app.menuIndex
        onActivated: (index) => { if (!app.gate(2, "sheet onActivated")) return; app.menuIndex = index; app.modMenuRun(index) }
        onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
    }
    GameMenu {
        id: gameMenuView
        ui: kit
        visible: app.sheet === "gamemenu"
        gameName: app.gameTitle
        items: app.gameMenuItems.map(m => ({ key: m.act || "", label: m.title, detail: m.detail, icon: m.icon, danger: m.danger === true, enabled: m.ok !== false, group: m.group, more: m.act === "restore" && app.setups.length > 1 }))
        focusIndex: app.menuIndex
        // Reset runs only while a finger or the pointer stays down on it (a tap says to hold).
        onActivated: (index, key) => { if (!app.gate(2, "sheet onActivated")) return; app.menuIndex = index; if (key !== "reset" || !resetHold.running) app.gameMenuRun(index) }
        onHeld: (index, down) => {
            const it = app.gameMenuItems[index]
            if (!it || it.act !== "reset") return
            app.menuIndex = index
            app.clickHeld = down
            if (down && !resetHold.running && !app.pressTwice) { app.resetHeld = 0; resetHold.start() }
            else if (!down) app.stopReset()
        }
        onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
    }
    AddMods {
        ui: kit
        visible: app.sheet === "add"
        subtitle: app.addAll ? "Every mod file in Downloads and its Game Mods folders, newest first"
                             : "Mod files in Downloads and Game Mods/" + app.gameTitle + ", newest first"
        files: app.addList.map(d => ({ name: d.name, size: app.sizeWords(d.size), age: (d.modified > 0 ? app.age(d.modified) : "") + (app.addAll && d.folder ? " · " + d.folder : "") }))
        focusIndex: app.addIndex
        emptyText: app.addAll || app.downloads.length === 0 ? "No mod files in Downloads yet. Get one with Get mods, or download a mod's zip file in Desktop Mode." : "None for " + app.gameTitle + ". Find, All files lists the rest."
        otherLabel: app.downloads.length === 0 ? "" : "Find"
        onOtherClicked: { if (!app.gate(2, "sheet onOtherClicked")) return; app.act("y") }
        onActivated: (index) => { if (!app.gate(2, "sheet onActivated")) return; app.addIndex = index; app.addChosen() }
        onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
    }
    PrePlayCheck {
        id: checkView
        ui: kit
        visible: app.sheet === "check"
        checks: app.checks
        buttons: app.checkChoices.map(label => ({ key: "", label: label, main: !app.checkStops && label === "Play anyway" }))
        focusIndex: app.checkIndex
        cancelIndex: app.checkChoices.indexOf("Not now")
        note: app.checkStops ? "Play without mods starts the game with no mods. Nothing is deleted." : ""
        onActivated: (index, key) => { if (!app.gate(2, "sheet onActivated")) return; app.checkIndex = index; app.checkChoose(index) }
        onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
    }
    LoaderPrompt {
        ui: kit
        visible: app.sheet === "loader"
        title: (app.loaderShort || "Mod loader") + " needed"
        loaderName: app.loaderShort
        loaderFullName: app.rules && app.rules.loader && app.rules.loader.name.indexOf(" (") > 0 ? app.rules.loader.name.split(" (")[1].replace(")", "") : ""
        explain: app.rules && app.rules.loader ? app.rules.loader.explain : ""
        neededBy: app.loaderFor
        sourceUrl: app.rules && app.rules.loader ? app.rules.loader.source : ""
        sourceShown: app.loaderSourceShown
        canDownload: app.loaderCanGet
        buttons: app.loaderLabels.map(l => ({ key: "", label: l }))
        focusIndex: app.loaderIndex
        onActivated: (index, key) => { if (!app.gate(2, "sheet onActivated")) return; app.loaderIndex = index; app.act(key || "a") }
        onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
    }

    // Installer choices: over the main area, the rail stays in view.
    FomodSheet {
        id: fomodView
        ui: kit
        visible: app.sheet === "fomod" && app.fomod !== null
        x: rail.width
        width: parent.width - rail.width
        height: parent.height - app.px(8)
        info: app.fomod
        onChoose: (g, o) => { if (!app.gate(2, "sheet onChoose")) return; app.note("action", "Installer: question " + (g + 1) + ", option " + (o + 1)); Turbine.fomodChoose(g, o) }
        onNext: { if (!app.gate(2, "sheet onNext")) return; app.note("action", "Installer: next step"); Turbine.fomodNext() }
        onBack: { if (!app.gate(2, "sheet onBack")) return; app.note("action", "Installer: back a step"); Turbine.fomodBack() }
        onStop: { if (!app.gate(2, "sheet onStop")) return; app.note("action", "Installer: stopped"); Turbine.fomodCancel() }
    }
    VersionsSheet {
        ui: kit
        visible: app.sheet === "versions"
        modName: app.detailsData && app.detailsData.name ? app.detailsData.name : ""
        versions: app.otherVersions
        focusIndex: app.versionIndex
        note: app.detailsData && app.detailsData.versionsNote ? app.detailsData.versionsNote : ""
        onActivated: (i) => { if (!app.gate(2, "sheet onActivated")) return; app.versionIndex = i; app.installVersion(i) }
        onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
    }
    PagesSheet {
        ui: kit
        visible: app.sheet === "pages"
        title: app.pagesTitle
        pages: app.pagesList
        focusIndex: app.pagesIndex
        onActivated: (i) => { if (!app.gate(2, "sheet onActivated")) return; app.pagesIndex = i; app.openPage(i) }
        onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
    }
    CodeSheet {
        ui: kit
        visible: app.sheet === "code"
        info: app.codeData
        onCopyClicked: { if (!app.gate(2, "sheet onCopyClicked")) return; app.act("a") }
        onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
    }
    PluginSheet {
        ui: kit
        visible: app.sheet === "plugin" && app.orderPlugin !== null
        plugin: app.orderPlugin
        position: app.orderIndex + 1
        notes: app.orderPlugin ? (app.orderMessages[app.orderPlugin.name.toLowerCase()] || []) : []
        onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
    }
    ProfilesSheet {
        id: profilesView
        ui: kit
        visible: app.sheet === "profiles"
        gameName: app.gameTitle
        profiles: app.profileList
        active: app.profileData ? app.profileData.active : ""
        actions: app.profileActions
        plan: app.importData
        premium: app.account.premium === true
        focusIndex: app.profIndex
        onActivated: (i) => { if (!app.gate(2, "sheet onActivated")) return; app.profIndex = i; app.profileRun(i) }
        onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
    }
    AddMods {
        ui: kit
        visible: app.sheet === "shares"
        title: "Open a loadout file"
        subtitle: "Shared loadout files in your Downloads folder, newest first"
        files: app.shares.map(x => ({ name: x.name, size: "", age: "" }))
        focusIndex: app.shareIndex
        actionLabel: "Read"
        emptyText: "No loadout files in Downloads. Someone makes one with Loadouts, Save to a loadout file."
        onActivated: (index) => { if (!app.gate(2, "sheet onActivated")) return; app.shareIndex = index; app.act("a") }
        onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
    }
    // The no-browser warning, the browser helper's steps, install notes, "Add this file?" and Add Chrome to Steam.
    InfoSheet {
        id: infoView
        ui: kit
        visible: ["nobrowser", "helper", "notes", "found", "quick", "chain", "addbrowser", "msg"].indexOf(app.sheet) >= 0
        title: app.sheet === "quick" && app.quick ? app.quick.name + " closed after " + app.plural(app.quick.seconds, "second", "seconds")
               : app.sheet === "chain" ? "Next: " + (app.chainQueue[0] ? app.chainQueue[0].name : "")
               : app.sheet === "nobrowser" ? (app.browser.title || "No web browser found")
               : app.sheet === "helper" ? "Set up the browser helper"
               : app.sheet === "addbrowser" && app.addMode === "helper" ? "Set up the browser helper too"
               : app.sheet === "addbrowser" ? "Add Chrome to Steam so it opens in Game Mode"
               : app.sheet === "found" ? "Found a download"
               : app.sheet === "msg" && app.msgData ? app.msgData.title
               : "Added " + app.notesName
        text: app.sheet === "quick" ? app.quickText
              : app.sheet === "chain" ? (app.chainAfter ? app.chainAfter + " is in. " : "") + "It also needs " + (app.chainQueue[0] ? app.chainQueue[0].name : "") + ". " + (app.chainQueue[0] && app.chainQueue[0].modId > 0 ? "A opens its file in Turbine. Press Slow download; it's added too."
                                    : "A opens its files on Nexus. Pick the top file under Main files, press Manual download, then Slow download. It's added too.")
              : app.sheet === "nobrowser" ? (app.browser.text || "")
              : app.sheet === "addbrowser" && app.addMode === "helper" ? "Turbine started " + app.openName + " through Steam. Without the browser helper it opens on its start page, not on the mod. The helper opens the mod's page for you and saves the download in the right game's folder."
              : app.sheet === "addbrowser" ? "Game Mode only shows apps Steam starts. Add your browser to Steam once, and Turbine opens Nexus in it right over Turbine. Once:"
              : app.sheet === "helper" ? "Optional. Skips the Save box and puts each download in the right game's folder. Turbine put it in Downloads, in the folder Turbine browser helper. Then, once:"
              : app.sheet === "found" && app.found ? "Found " + app.found.name + " (" + app.sizeWords(app.found.size) + ")."
                    + (app.found.game ? " Add it to " + app.found.gameName + "?" : " Which game is it for?")
              : app.sheet === "notes" ? "Something to know about this mod:"
              : app.sheet === "msg" && app.msgData ? (app.msgData.text || "")
              : ""
        steps: app.sheet === "nobrowser" ? (app.browser.steps || [])
               : app.sheet === "helper" ? ["In Desktop Mode, open chrome://extensions in the browser.",
                                           "Turn on Developer mode (top right).",
                                           "Press Load unpacked.",
                                           "Choose Downloads, then the folder Turbine browser helper."]
               : app.sheet === "addbrowser" && app.addMode !== "helper" ? ["Switch to Desktop Mode: press the Steam button, open Power, and pick Switch to Desktop.",
                                           "In Desktop Mode, open Steam, then Games, then Add a Non-Steam Game to My Library.",
                                           "Tick Google Chrome and press Add Selected Programs.",
                                           "Switch back to Game Mode, open Turbine and press Recheck."]
               : app.sheet === "msg" && app.msgData ? (app.msgData.steps || [])
               : []
        notes: app.sheet === "notes" ? app.notesList : app.sheet === "msg" && app.msgData ? (app.msgData.notes || []) : []
        footnote: app.sheet === "msg" && app.msgData ? (app.msgData.footnote || "")
                  : app.sheet === "helper" ? "Then press Pair: the pairing page opens in the browser and links the helper to Turbine."
                  : app.sheet === "addbrowser" && app.addMode !== "helper" ? (app.bridge.paired ? "Until then, Steam's browser opens Nexus instead." : "Also set up the browser helper (Settings, Getting mods): without it the browser opens, but not on the mod's page. Until then, Steam's browser opens Nexus instead.")
                  : app.sheet === "addbrowser" ? "Steam's browser opens the mod's page without the helper."
                  : app.sheet === "found" && app.found ? (app.found.game ? "Why " + app.found.gameName + ": " + app.found.why + ". " : "")
                    + "The file stays in Downloads either way." + (app.found.more > 0 ? " " + app.plural(app.found.more, "more file waits", "more files wait") + " after this one." : "")
                  : ""
        buttons: app.infoButtons
        focusIndex: app.infoIndex
        onActivated: (i) => { if (!app.gate(2, "sheet onActivated")) return; app.infoIndex = i; app.infoRun(i) }
        onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
    }
    // Removed mods (A Put back, X Delete now) and the hold-to-confirm sheet for deleting for good.
    RemovedSheet {
        ui: kit
        visible: app.sheet === "removed"
        mods: app.removedMods
        focusIndex: app.removedIndex
        onActivated: (i) => { if (!app.gate(2, "sheet onActivated")) return; app.removedIndex = i; app.removedRun(i) }
        onDeleteClicked: (i) => { if (!app.gate(2, "sheet onDeleteClicked")) return; app.removedIndex = i; app.removedDelete(i) }
        onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
    }
    InfoSheet {
        id: confirmSheet
        ui: kit
        visible: app.sheet === "confirm"
        title: app.confirmVerb === "delete" ? "Delete for good?" : app.confirmVerb === "remove it" ? "Remove the leftover file?" : "Replace your saves?"
        text: app.confirmText
        buttons: [{ key: "", label: app.pressTwice ? (app.armedKey === app.deleteKey() ? "Press A again to " + app.confirmVerb : "Press A twice to " + app.confirmVerb)
                                 : confirmHold.running ? "Keep holding A: " + Math.max(1, Math.ceil(app.holdNeeded - app.confirmHeld)) + "..." : "Hold A to " + app.confirmVerb },
                  { key: "", label: "Keep" }]
        focusIndex: app.confirmIndex
        // Hold mode: a tap on Delete only moves the highlight; holding a finger or the pointer on it counts like
        // holding A. Press twice mode: a tap counts like a press of A.
        onActivated: (i) => { if (!app.gate(2, "sheet onActivated")) return
            if (i !== 0) { app.confirmIndex = i; app.act("b"); return }
            if (app.pressTwice) { app.confirmIndex = 0; app.startConfirm(); return }
            app.confirmIndex = i
            if (!confirmHold.running) app.holdHint(app.holdNeeded + " seconds to " + app.confirmWords())
        }
        onHeld: (i, down) => {
            if (i !== 0 || app.pressTwice) return
            app.confirmIndex = 0
            app.clickHeld = down
            if (down) app.startConfirm(); else app.stopConfirm()
        }
        onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
    }
    // X on a Get mods card.
    ModMenu {
        ui: kit
        visible: app.sheet === "cardmenu"
        title: app.cardItem ? app.cardItem.name : ""
        subtitle: app.cardItem ? (app.cardItem.owner || "") : ""
        items: app.cardMenuItems.map(m => ({ label: m.title, detail: m.detail, icon: m.icon, danger: false }))
        focusIndex: app.menuIndex
        onActivated: (index) => { if (!app.gate(2, "sheet onActivated")) return; app.menuIndex = index; app.cardMenuRun(index) }
        onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
    }
    // Y's pop-up: Search, Sort and Category on Get mods; Find, Show, Sort and Select several on My mods.
    FindSheet {
        id: findView
        ui: kit
        visible: app.sheet === "findpop"
        title: app.findPop.kind === "get" ? "Find mods for " + app.gameTitle : "Find in my mods"
        subtitle: app.findPop.kind === "get" ? app.nexusLine : ""
        fieldLabel: app.findPop.kind === "get" ? "Search" : "Find"
        text: app.findPop.text
        placeholder: app.findPop.kind === "get" ? "Search " + app.gameTitle + " mods" : "Part of a mod's name"
        result: app.findResult
        sections: app.findSections
        focusRow: app.findRow
        focusCol: app.findCol
        onFieldClicked: { if (!app.gate(2, "sheet onFieldClicked")) return; app.findPopField() }
        onTyped: (text) => app.findPopTyping(text)
        onEntered: (text) => app.findPopEntered(text)
        onDoneClicked: { if (!app.gate(2, "sheet onDoneClicked")) return; app.findPopDone() }
        onClearClicked: { if (!app.gate(2, "sheet onClearClicked")) return; app.findPopClear() }
        onOptionClicked: (r, c) => { if (!app.gate(2, "sheet onOptionClicked")) return; app.findRow = r; app.findCol = c; app.findPopChoose(r, c) }
        onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.findPopDone() }
    }
    // A small menu made on the spot: Play options (hold Start), a profile's Rename and Delete, Select several's actions.
    ModMenu {
        id: pickMenu
        ui: kit
        visible: app.sheet === "pick"
        title: app.pickTitle
        subtitle: app.pickSub
        items: app.pickItems.map(m => ({ label: m.title, detail: m.detail, icon: m.icon, danger: m.danger === true, enabled: m.dim !== true }))
        focusIndex: app.menuIndex
        onActivated: (index) => { if (!app.gate(2, "sheet onActivated")) return
            if (app.nxmAsking) nxmRingMove.stop()   // a tap is the player's choice: the ring stays
            // The Mod Manager Download question: nothing for its first second, and a tap (or two, or a click) on Get
            // it only picks it: only A gets it, so a page can't get a mod by asking for taps where the row appears.
            if (app.nxmAsking && (app.nxmAskQuiet() || index === 0)) {
                if (!app.nxmAskQuiet()) app.menuIndex = 0
                app.note("nxm", "Tap on the question: " + (app.nxmAskQuiet() ? "ignored (it just opened)" : "Get it picked; only A gets it"))
                // Said on screen too (newbie M3): touch alone can't get it, on purpose.
                if (!app.nxmAskQuiet()) toast.info("Get it is picked. Press A to get it: a tap only picks it, so a page can never get a mod by itself.")
                return
            }
            app.menuIndex = index; app.pickRun(index)
        }
        onClosed: { if (!app.gate(2, "sheet onClosed")) return; if (app.nxmAskQuiet()) return; app.act("b") }
    }
    // The focused game's name beside the rail, up in the header (never over the tabs), and until a game has been
    // opened from the rail once, how to pick one.
    Rectangle {
        id: railLabel
        readonly property bool on: app.zone === "rail" && app.railIndex < app.games.length
        readonly property bool hint: !seen.railHintDone || shot.page === "rail"
        visible: on && app.sheet === ""
        z: 20
        // Beside the focused tile, like a tooltip (never over the top of the page).
        x: rail.width + app.px(8)
        y: {
            const moved = rail.railList.contentY + app.railIndex + rail.width
            const it = rail.itemAt(app.railIndex)
            if (!it || !on) return app.px(6)
            const p = it.mapToItem(sheetLayer, 0, it.height / 2)
            return Math.max(app.px(6), Math.min(sheetLayer.height - height - app.px(6), p.y - height / 2 + moved * 0))
        }
        width: Math.max(railLabelText.implicitWidth, railHintText.visible ? railHintText.implicitWidth : 0) + app.px(24)
        height: railLabelText.implicitHeight + (railHintText.visible ? railHintText.implicitHeight + app.px(2) : 0) + app.px(12)
        radius: kit.radiusSmall
        color: c.raised
        border.color: c.accent
        border.width: Math.max(1, app.px(1))
        Column {
            anchors.centerIn: parent
            spacing: app.px(2)
            Text { id: railLabelText; anchors.horizontalCenter: parent.horizontalCenter; text: app.railName(app.railIndex); color: c.text; font.family: app.bodyFont; font.pixelSize: app.fpx(15); font.weight: Font.DemiBold }
            Text { id: railHintText; visible: railLabel.hint; anchors.horizontalCenter: parent.horizontalCenter; text: "Up and Down pick a game, A opens it"; color: c.soft; font.family: app.bodyFont; font.pixelSize: app.fpx(13) }
        }
    }
    // X on "Found <file>": which game it goes into.
    GamePicker {
        ui: kit
        visible: app.sheet === "pickgame"
        fileName: app.found ? app.found.name : ""
        games: app.games
        focusIndex: app.pickIndex
        onActivated: (i) => { if (!app.gate(2, "sheet onActivated")) return; app.pickIndex = i; app.pickGame(i) }
        onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
    }
    AskText {
        id: askView
        ui: kit
        // Drawn only while it is the sheet, so another sheet can never open under it.
        visible: shown && app.sheet === "ask"
        focusIndex: app.askIndex
        onAccepted: (text) => app.askDone(text)
        onEdited: (text) => { if (app.askPurpose === "find") { app.findText = text; app.modIndex = 0 } }
        onCancelled: app.askCancelled()
        onFieldTapped: { app.askIndex = -1; app.askKeyboard() }
    }

    // Edit settings (a mod's config files, the game's own, and Seamless Co-op's).
    SettingsEditor {
        id: settingsView
        ui: kit
        visible: app.sheet === "settingsedit"
        onSaveClicked: { if (!app.gate(2, "sheet onSaveClicked")) return; app.settingsSaveNow() }
        onOptionsClicked: { if (!app.gate(2, "sheet onOptionsClicked")) return; app.openSettingsOptions() }
        onCloseClicked: {
            if (!app.gate(2, "sheet onCloseClicked")) return
            // Anything still waiting to save goes in first: B never loses a change.
            app.settingsLeaving = true
            if (changes > 0) app.settingsSaveNow(true)
            app.settingsLeaving = false
            if (app.coopAfterSettings) { app.coopAfterSettings = false; toast.info("Co-op waits for a session password.") }
            app.note("action", "Edit settings closed"); app.sheet = app.settingsReturn
        }
        // The Co-op tile's row, on Seamless Co-op's own settings page.
        extraRows: app.sheet === "settingsedit" && app.seamlessMod && app.settingsMod === app.seamlessMod.id && app.coopTileOffered ? [{
            key: "cooptile", title: "Co-op tile in Steam",
            detail: app.coopTileThere ? "In your Steam library: it starts " + app.gameTitle + " with Seamless Co-op in one press."
                                      : "A tile in your Steam library that starts " + app.gameTitle + " with Seamless Co-op in one press" + (app.coopTileHow !== "" ? " (" + app.coopTileHow + ")." : "."),
            label: app.coopTileThere ? "Remove" : "Add" }] : []
        onActionClicked: (key) => { if (!app.gate(2, "sheet onActionClicked")) return
            if (key !== "cooptile") return
            // Anything still waiting to save goes in first, then the tile's own question.
            app.settingsLeaving = true
            if (changes > 0) app.settingsSaveNow(true)
            app.settingsLeaving = false
            app.coopAfterSettings = false
            app.note("action", "Co-op settings: the Co-op tile row")
            app.openCoopTile()
        }
        onTypeAsked: (row, value, label) => { if (!app.gate(2, "sheet onTypeAsked")) return
            app.settingsTypeRow = row
            const help = row.kind === "line" ? "The whole line, as it is in the file." : (row.f.help || "") + (row.f.kind === "number" ? " A number." : "")
            app.openAsk("settingvalue", label, help.trim(), value, label, 2000, false, "Done")
        }
    }
    ActivityLog {
        id: activityView
        ui: kit
        visible: app.sheet === "activity"
        lines: app.activityLines
        onCopyClicked: { if (!app.gate(2, "sheet onCopyClicked")) return
            Turbine.copyText(app.activityLines.slice(-500).join("\n") + "\n")
            toast.info("Copied the last " + Math.min(500, app.activityLines.length) + " lines of the activity log.")
        }
        onSaveClicked: { if (!app.gate(2, "sheet onSaveClicked")) return
            const r = app.parse(Turbine.saveActivityLog(), { error: "Couldn't save it." })
            toast.info(r.error ? r.error : "Saved to Downloads as " + r.file + ".")
        }
        onCloseClicked: { if (!app.gate(2, "sheet onCloseClicked")) return; app.sheet = "" }
    }
    PerfCheck {
        id: perfView
        ui: kit
        win: app
        onCopyClicked: (text) => { if (!app.gate(2, "sheet onCopyClicked")) return; Turbine.copyText(text); toast.info("Performance check copied.") }
        onReportClicked: (text) => { if (!app.gate(2, "sheet onReportClicked")) return; app.openReport(null, "Performance check", text) }
        onCloseClicked: { if (!app.gate(2, "sheet onCloseClicked")) return; perfView.close(); app.sheet = "" }
        Component.onCompleted: if (shot.page === "perfcheck") reportText = "Turbine performance check\nTurbine: 0.2 (build sample)\nVerdict: Smooth\n"
            + "Still: 60 fps, median 16.7 ms, worst 18.9 ms, spikes 0.0%, CPU 9%\nScrolling: 59 fps, median 16.7 ms, worst 24.0 ms, spikes 0.4%, CPU 31%\n"
            + "Idle: 0 frames in 4.0 s, CPU 0.2%\n\nStartup: first frame 640 ms after start\nDrawing: Vulkan, software drawing no"
    }
    LabsReport {
        id: reportView
        ui: kit
        onSendClicked: { if (!app.gate(2, "sheet onSendClicked")) return; app.sendReport() }
        onCloseClicked: { if (!app.gate(2, "sheet onCloseClicked")) return; app.dismissReport() }
        onEditStarted: app.reportEditing(true)
        onEditEnded: app.reportEditing(false)
    }
}
