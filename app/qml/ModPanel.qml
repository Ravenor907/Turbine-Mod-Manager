import QtQuick

// The details panel: the highlighted mod on the right of My mods when there is room. Its picture, name, version
// and maker, what it does, what it wins or loses, Edit settings when it has a settings file, and what X offers. Main.qml
// places it (the right edge of the page, under the line over the list).
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
    visible: app.showModPanel
    z: app.turnTo === 0 ? 3 : 0
    transform: Translate { x: app.turnTo === 0 ? app.turnX : 0 }
    width: visible ? Math.round(parent.width * 0.36) : 0
    radius: app.px(8)
    color: c.panel
    border.color: c.line; border.width: Math.max(1, app.px(1))
    Accessible.role: Accessible.Pane
    Accessible.name: m ? "Details: " + m.name : "No mod highlighted"
    Column {
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
        Rectangle {
            id: panelEdit
            objectName: "panelEdit"
            readonly property bool foc: app.zone === "panel" && app.sheet === ""
            visible: app.panelHasSettings
            width: Math.min(parent.width, panelEditText.implicitWidth + app.px(28)); height: app.tpx(44); radius: app.px(6)
            color: foc ? c.focusFill : c.raised
            border.color: foc ? c.accentHi : c.accent; border.width: foc ? app.px(2) : Math.max(1, app.px(1.5))
            Text { id: panelEditText; anchors.centerIn: parent; text: app.panelSettingsLabel; color: c.text; font.family: app.bodyFont; font.pixelSize: app.fpx(15); font.bold: true }
            HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
            TapHandler { onTapped: { if (!app.gate(0, "panel Edit settings")) return; app.zone = "panel"; app.panelEditOpen() } }
        }
    }
    Text {
        anchors.left: parent.left; anchors.right: parent.right; anchors.bottom: parent.bottom
        anchors.margins: app.px(14)
        wrapMode: Text.WordWrap; maximumLineCount: 2; elide: Text.ElideRight
        text: modPanel.m ? app.glyph("x") + " Options: " + app.optionsLine() : ""
        color: c.muted; font.family: app.bodyFont; font.pixelSize: app.fpx(13)
        // A tap opens the options too (touch has every action).
        TapHandler { onTapped: { if (!app.gate(0, "panel options")) return; if (app.menuMod) { app.menuIndex = 0; app.sheet = "modmenu" } } }
    }
}
