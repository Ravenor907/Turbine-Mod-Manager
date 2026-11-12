import QtQuick

// An empty My mods (any game): a centered card with a big Get mods (A) and Add from Downloads (X), or only Add from
// Downloads where the game has nowhere to get mods from.
//
// app (required): Main.qml's window. kit (required): Main.qml's kit (colors, see COMPONENTS.md).
Rectangle {
    id: card
    required property var app
    required property var kit
    readonly property var c: kit.c
    readonly property bool cardFocus: app.zone === "main" && app.listItems.length === 0
    visible: app.onTab(0) && app.emptyModsRoom
    z: app.turnTo === 0 ? 3 : 0
    // Showing up on its own (the last mod uninstalled, the mods read), it fades in as it rises into place.
    onVisibleChanged: if (visible) arrive.appear(() => card.kit.arriving, 0, app.px(kit.motion.nudge))
    opacity: app.tabFade(0) * arrive.part
    enabled: app.turnFrom !== 0
    transform: [Translate { x: app.tabShift(0) }, Arrive { id: arrive; motion: card.kit.motion }]
    anchors.centerIn: parent
    width: Math.min(parent.width - app.px(32), app.px(520))
    height: emptyCol.implicitHeight + app.px(48)
    radius: kit.radius
    color: c.card
    border.color: c.line
    Column {
        id: emptyCol
        anchors.centerIn: parent
        width: parent.width - app.px(48)
        spacing: app.px(14)
        Text {
            width: parent.width; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.Wrap
            text: "No mods for " + app.gameTitle + " yet"
            color: c.text; font.family: app.bodyFont; font.pixelSize: app.fpx(22); font.bold: true
        }
        Text {
            width: parent.width; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.Wrap
            text: app.canGetMods ? "Get mods finds them for you, or add a mod file you downloaded." : "Add a mod file you downloaded: zip, 7z or rar."
            color: c.soft; font.family: app.bodyFont; font.pixelSize: app.fpx(15)
        }
        Flow {
            anchors.horizontalCenter: parent.horizontalCenter
            width: Math.min(parent.width, emptyGet.width + emptyAdd.width + spacing + 1)
            spacing: app.px(14)
            // Get mods is the main button; the ring shows only while A is for the card (not for a row above it,
            // like Removed mods).
            ActionButton {
                id: emptyGet
                ui: kit
                visible: app.canGetMods
                label: "Get mods"
                cap: "a"
                primary: true
                focused: card.cardFocus
                Accessible.role: Accessible.Button; Accessible.name: "Get mods"
                onClicked: { if (!app.gate(0, "note")) return; { app.note("action", "No mods yet: Get mods (tap)"); app.tab = 1 } }
            }
            ActionButton {
                id: emptyAdd
                ui: kit
                label: "Add from Downloads"
                cap: app.canGetMods ? "x" : "a"
                primary: !app.canGetMods
                focused: !app.canGetMods && card.cardFocus
                Accessible.role: Accessible.Button; Accessible.name: "Add from Downloads"
                onClicked: { if (!app.gate(0, "note")) return; { app.note("action", "No mods yet: Add a file (tap)"); app.openAdd() } }
            }
        }
    }
}
