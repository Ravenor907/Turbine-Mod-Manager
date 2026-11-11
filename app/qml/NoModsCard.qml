import QtQuick

// An empty My mods (any game): a centered card with a big Get mods (A) and Add from Downloads (X), or only Add from
// Downloads where the game has nowhere to get mods from.
//
// app (required): Main.qml's window. kit (required): Main.qml's kit (colors, see COMPONENTS.md).
Rectangle {
    required property var app
    required property var kit
    readonly property var c: kit.c
    visible: app.emptyMods
    z: app.turnTo === 0 ? 3 : 0
    transform: Translate { x: app.turnTo === 0 ? app.turnX : 0 }
    anchors.centerIn: parent
    width: Math.min(parent.width - app.px(32), app.px(520))
    height: emptyCol.implicitHeight + app.px(48)
    radius: app.px(12)
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
            Rectangle {
                id: emptyGet
                visible: app.canGetMods
                width: emptyGetRow.implicitWidth + app.px(36); height: Math.max(app.px(52), app.tpx(44)); radius: app.px(8)
                color: app.zone === "main" ? c.focusFill : c.raised
                border.color: c.accent; border.width: app.px(app.zone === "main" ? 2 : 1)
                Row { id: emptyGetRow; anchors.centerIn: parent; spacing: app.px(10)
                    Key { win: app; k: "a"; anchors.verticalCenter: parent.verticalCenter }
                    Text { text: "Get mods"; color: c.text; font.family: app.bodyFont; font.pixelSize: app.fpx(18); font.bold: true; anchors.verticalCenter: parent.verticalCenter } }
                Accessible.role: Accessible.Button; Accessible.name: "Get mods"
                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                TapHandler { onTapped: { if (!app.gate(0, "note")) return; { app.note("action", "No mods yet: Get mods (tap)"); app.tab = 1 } } }
            }
            Rectangle {
                id: emptyAdd
                width: emptyAddRow.implicitWidth + app.px(36); height: Math.max(app.px(52), app.tpx(44)); radius: app.px(8)
                color: !app.canGetMods && app.zone === "main" ? c.focusFill : c.raised
                border.color: c.line; border.width: Math.max(1, app.px(1))
                Row { id: emptyAddRow; anchors.centerIn: parent; spacing: app.px(10)
                    Key { win: app; k: app.canGetMods ? "x" : "a"; anchors.verticalCenter: parent.verticalCenter }
                    Text { text: "Add from Downloads"; color: c.text; font.family: app.bodyFont; font.pixelSize: app.fpx(18); font.bold: true; anchors.verticalCenter: parent.verticalCenter } }
                Accessible.role: Accessible.Button; Accessible.name: "Add from Downloads"
                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                TapHandler { onTapped: { if (!app.gate(0, "note")) return; { app.note("action", "No mods yet: Add a file (tap)"); app.openAdd() } } }
            }
        }
    }
}
