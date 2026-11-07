import QtQuick

// Profiles (game menu > Profiles): the game's setups by name, the one in use marked; A switches to one (it goes into
// the game right away). Below them: New profile, Export to a share file, Import a share file, Import a Thunderstore
// code. When a share is being imported, the sheet shows what it needs instead: mods Turbine gets by itself, from Nexus
// Mods, ones to add by hand, ones already here, and Get them.
Item {
    id: ps
    required property var ui
    anchors.fill: parent
    visible: false

    property string gameName: ""
    property var profiles: []             // [{name, mods, on}]
    property string active: ""
    property var actions: []              // [{key, label, detail}]
    property var plan: null               // a share being imported: {profile, plan:{automatic,nexus,byHand,installed}, error}
    property int focusIndex: 0
    property bool premium: false          // the Nexus account can download inside Turbine

    signal activated(int index)
    signal closed()

    readonly property int rows: plan ? (plan.error ? 1 : 2) : profiles.length + actions.length

    Rectangle { anchors.fill: parent; color: "#B0000000" }
    MouseArea { anchors.fill: parent; onClicked: ps.closed() }
    Rectangle {
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.bottomMargin: ps.ui.px(56)
        width: Math.min(parent.width * 0.55, ps.ui.px(520))
        color: ps.ui.c.panel
        border.color: ps.ui.c.line
        MouseArea { anchors.fill: parent }
        Column {
            x: ps.ui.px(18); y: ps.ui.px(18)
            width: parent.width - ps.ui.px(36)
            spacing: ps.ui.px(8)
            Text { text: ps.plan ? "Import a profile" : "Profiles"; color: ps.ui.c.text; font.family: ps.ui.titleFont; font.pixelSize: ps.ui.px(18); font.bold: true }
            Text {
                width: parent.width; wrapMode: Text.WordWrap
                text: ps.plan ? (ps.plan.error ? ps.plan.error : "\"" + ps.plan.profile + "\" for " + ps.gameName)
                              : ps.gameName + ": which mods are on and their order, saved by name. Switching puts that setup in the game."
                color: ps.plan && ps.plan.error ? ps.ui.c.warn : ps.ui.c.soft
                font.family: ps.ui.bodyFont; font.pixelSize: ps.ui.px(15)
            }
            // The plan of a share being imported
            Repeater {
                model: !ps.plan || !ps.plan.plan ? [] : [
                    ["Turbine gets these for you", ps.plan.plan.automatic, "ok"],
                    [ps.premium ? "From Nexus Mods: Turbine gets these too (Premium)" : "From Nexus Mods: Turbine lists their pages to open after this", ps.plan.plan.nexus, ps.premium ? "ok" : "warn"],
                    ["Added from files: add them by hand", ps.plan.plan.byHand, "warn"],
                    ["You have these already", ps.plan.plan.installed, ""]
                ].filter(x => x[1].length > 0)
                Column {
                    required property var modelData
                    width: parent.width
                    spacing: ps.ui.px(2)
                    Text { width: parent.width; wrapMode: Text.WordWrap; text: parent.modelData[0] + " (" + parent.modelData[1].length + ")"; color: parent.modelData[2] === "ok" ? ps.ui.c.ok : parent.modelData[2] === "warn" ? ps.ui.c.warn : ps.ui.c.text; font.family: ps.ui.bodyFont; font.pixelSize: ps.ui.px(15); font.bold: true }
                    Text { width: parent.width; elide: Text.ElideRight; text: parent.modelData[1].map(m => m.name).join(", "); color: ps.ui.c.soft; font.family: ps.ui.bodyFont; font.pixelSize: ps.ui.px(14) }
                }
            }
            Repeater {
                model: ps.plan ? [{ label: ps.plan.error ? "Close" : "Get them and make the profile", detail: ps.plan.error ? "" : "Downloads what Turbine can get, then switches to it" },
                                  { label: "Cancel", detail: "" }].slice(ps.plan.error ? 1 : 0)
                         : ps.profiles.map(p => ({ label: p.name, detail: p.on + " of " + p.mods + " mods on" + (p.name === ps.active ? " · In use" : ""), current: p.name === ps.active }))
                           .concat(ps.actions)
                Rectangle {
                    id: row
                    required property int index
                    required property var modelData
                    readonly property bool foc: ps.focusIndex === index
                    readonly property bool firstAction: !ps.plan && index === ps.profiles.length
                    width: parent.width
                    height: ps.ui.px(modelData.detail ? 58 : 46) + (firstAction ? ps.ui.px(8) : 0)
                    color: "transparent"
                    Rectangle {
                        y: row.firstAction ? ps.ui.px(8) : 0
                        width: parent.width; height: parent.height - y
                        radius: ps.ui.px(6)
                        color: row.foc ? ps.ui.c.focusFill : ps.ui.c.card
                        border.color: ps.ui.c.accent
                        border.width: row.foc ? ps.ui.px(2) : 0
                        Rectangle {
                            visible: row.modelData.current === true
                            x: 0; width: ps.ui.px(4); height: parent.height; radius: ps.ui.px(2); color: ps.ui.c.accent
                        }
                        Column {
                            x: ps.ui.px(14)
                            width: parent.width - ps.ui.px(60)
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: ps.ui.px(2)
                            Text { width: parent.width; elide: Text.ElideRight; text: row.modelData.label; color: ps.ui.c.text; font.family: ps.ui.bodyFont; font.pixelSize: ps.ui.px(16); font.bold: true }
                            Text { visible: !!row.modelData.detail; width: parent.width; elide: Text.ElideRight; text: row.modelData.detail || ""; color: ps.ui.c.soft; font.family: ps.ui.bodyFont; font.pixelSize: ps.ui.px(14) }
                        }
                        Rectangle {
                            visible: row.foc
                            anchors.right: parent.right; anchors.rightMargin: ps.ui.px(12); anchors.verticalCenter: parent.verticalCenter
                            width: ps.ui.px(24); height: width; radius: width / 2; color: ps.ui.c.raised; border.color: ps.ui.c.accentLo
                            Text { anchors.centerIn: parent; text: "A"; color: ps.ui.c.text; font.family: ps.ui.bodyFont; font.pixelSize: ps.ui.px(13); font.bold: true }
                        }
                    }
                    HoverHandler { onHoveredChanged: ps.ui.hover(parent, hovered) }
                    TapHandler { onTapped: ps.activated(row.index) }
                }
            }
        }
    }
}
