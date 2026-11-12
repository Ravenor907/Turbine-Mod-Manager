import QtQuick

// The details panel: the highlighted mod on the right of My mods when there is room. Its picture, name, version
// and maker, what it does, what it wins or loses, Edit settings when it has a settings file, and what X offers. On a
// row that isn't a mod (Dependencies, Removed mods, a mod just uninstalled) a short summary of that row instead.
// Main.qml places it (the right edge of the page, under the line over the list).
//
// app (required): Main.qml's window. kit (required): Main.qml's kit (colors and motion, see COMPONENTS.md).
Rectangle {
    id: modPanel
    required property var app
    required property var kit
    readonly property var c: kit.c
    readonly property Motion mo: kit.motion
    // For the build's checks.
    property alias panelEdit: panelEdit
    readonly property var m: app.menuMod
    readonly property var row: m ? null : app.focusedItem
    // The summary for a row that isn't a mod: [title, words].
    readonly property var rowWords: {
        const r = row
        if (!r) return ["", ""]
        if (r.kind === "deps") {
            const n = app.depMods.length
            return ["Dependencies", app.plural(n, "mod", "mods") + " that your other mods need: " + app.depMods.map(x => x.name).join(", ")
                    + ". They stay in this group so the list stays short. " + (app.depsOpen ? "A closes it." : "A opens it.")]
        }
        if (r.kind === "removed") return ["Removed mods", app.plural(app.removedMods.length, "mod", "mods") + " you uninstalled. Each waits here for 7 days, then it's deleted automatically. A shows them, to put one back."]
        if (r.kind === "ghost") return ["Uninstalled", r.m.name + " is out of the game. A puts it back where it was."]
        if (r.kind === "held") return ["Mods turned off", app.heldWords + ". A turns them back on."]
        if (r.kind === "needsq" && app.reqQ) return ["Still to get", app.reqQ.forName + " still needs " + app.plural(app.reqQ.items.length - app.reqQ.index, "mod", "mods") + ". A goes on where you left off."]
        return ["", ""]
    }
    visible: app.showModPanel
    z: app.turnTo === 0 ? 3 : 0
    transform: Translate { x: app.turnTo === 0 ? app.turnX : 0 }
    width: visible ? Math.round(parent.width * 0.36) : 0
    radius: app.px(8)
    color: c.panel
    border.color: c.line; border.width: Math.max(1, app.px(1))
    Accessible.role: Accessible.Pane
    Accessible.name: m ? "Details: " + m.name : "No mod highlighted"
    // A row that isn't a mod: its name and what it holds.
    Column {
        visible: !modPanel.m && modPanel.rowWords[0] !== ""
        x: app.px(18); y: app.px(18)
        width: parent.width - app.px(36)
        spacing: app.px(10)
        Icon { win: app; d: modPanel.row && modPanel.row.kind === "ghost" ? kit.icons.restore : kit.icons.folder; tint: c.accentText }
        Text {
            width: parent.width; wrapMode: Text.WordWrap
            text: modPanel.rowWords[0]
            color: c.text; font.family: app.bodyFont; font.pixelSize: app.fpx(19); font.bold: true
        }
        Text {
            width: parent.width; wrapMode: Text.WordWrap
            text: modPanel.rowWords[1]
            color: c.soft; font.family: app.bodyFont; font.pixelSize: app.fpx(14)
        }
    }
    Column {
        visible: modPanel.m !== null
        x: app.px(14); y: app.px(14)
        width: parent.width - app.px(28)
        spacing: app.px(8)
        Rectangle {
            width: parent.width; height: Math.min(width * 9 / 16, modPanel.height * 0.38); radius: app.px(6)
            color: c.raised
            clip: true
            Image {
                id: panelPic
                anchors.fill: parent
                source: modPanel.m && modPanel.m.picture ? modPanel.m.picture : ""
                fillMode: Image.PreserveAspectCrop
                asynchronous: true
                visible: status === Image.Ready
                opacity: status === Image.Ready ? 1 : 0
                Behavior on opacity { enabled: mo.on; NumberAnimation { duration: mo.base; easing.type: mo.enter } }
            }
            Text {
                visible: !panelPic.visible
                anchors.centerIn: parent
                text: modPanel.m ? (modPanel.m.name || "?").substring(0, 1).toUpperCase() : ""
                color: c.muted; font.family: app.bodyFont; font.pixelSize: app.fpx(40); font.bold: true
            }
        }
        Text {
            width: parent.width; wrapMode: Text.WordWrap; maximumLineCount: 2; elide: Text.ElideRight
            text: modPanel.m ? modPanel.m.name : ""
            color: c.text; font.family: app.bodyFont; font.pixelSize: app.fpx(19); font.bold: true
        }
        Text {
            width: parent.width; elide: Text.ElideRight
            text: app.panelLine(modPanel.m)
            color: c.soft; font.family: app.bodyFont; font.pixelSize: app.fpx(14)
        }
        Text {
            visible: text !== ""
            width: parent.width; wrapMode: Text.WordWrap; maximumLineCount: 4; elide: Text.ElideRight
            text: modPanel.m ? (modPanel.m.summary || "") : ""
            color: c.text; font.family: app.bodyFont; font.pixelSize: app.fpx(14)
        }
        Repeater {
            model: app.modPanelFacts
            Text {
                required property var modelData
                width: parent.width; wrapMode: Text.WordWrap; maximumLineCount: 2; elide: Text.ElideRight
                text: modelData.text
                color: modelData.level === "warn" ? c.warn : c.accentText
                font.family: app.bodyFont; font.pixelSize: app.fpx(14); font.weight: Font.DemiBold
            }
        }
        // Edit settings right on the mod, for a mod with a settings file
        // (Seamless Co-op: its settings, password first). Right from the list, or a tap; nothing when
        // the mod has none.
        // The shared button look: neutral, with the focus ring when Right has moved onto it.
        ActionButton {
            id: panelEdit
            objectName: "panelEdit"
            ui: kit
            readonly property bool foc: app.zone === "panel" && app.sheet === ""
            visible: app.panelHasSettings
            width: Math.min(parent.width, want)
            label: app.panelSettingsLabel
            focused: foc
            cap: foc ? "a" : ""
            onClicked: { if (!app.gate(0, "panel Edit settings")) return; app.zone = "panel"; app.panelEditOpen() }
        }
    }
    // What X offers, on one line: X's cap and "More options (9)". A tap opens them too (touch has every action).
    Row {
        visible: modPanel.m !== null
        anchors.left: parent.left; anchors.bottom: parent.bottom
        anchors.margins: app.px(14)
        spacing: app.px(8)
        Key { win: app; k: "x"; anchors.verticalCenter: parent.verticalCenter }
        Label { win: app; text: "More options (" + app.optionsCount() + ")"; color: c.soft; font.pixelSize: app.fpx(14); anchors.verticalCenter: parent.verticalCenter }
        TapHandler { onTapped: { if (!app.gate(0, "panel options")) return; if (app.menuMod) { app.menuIndex = 0; app.sheet = "modmenu" } } }
    }
}
