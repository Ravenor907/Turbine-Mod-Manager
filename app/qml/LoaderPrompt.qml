import QtQuick

// "These mods need SKSE (Skyrim Script Extender)": said once, in plain words, when mods need a loader the game
// doesn't have. canDownload: true asks to install it from its official source; false says Turbine can't get it
// itself (an info line). The buttons come from Main.qml. A centered card over a dimmed backdrop. It fills its parent; show and hide it with visible.
//
// ui (required): Main.qml's kit (colors, sizes, fonts, motion and helpers; see COMPONENTS.md).
Overlay {
    id: root
    onOutsideTapped: root.closed()

    required property var ui
    motion: ui.motion

    property string title: "Mod loader needed"
    property string loaderName: ""        // short name, e.g. "SKSE"
    property string loaderFullName: ""    // e.g. "Skyrim Script Extender"
    property string explain: ""           // what the loader is, in plain words
    property var neededBy: []             // mod names
    property string sourceUrl: ""         // official home, e.g. "skse.silverlock.org"
    property bool canDownload: false
    property bool sourceShown: false
    property string infoText: "Turbine can't get " + (loaderName || "it") + " by itself. Open its page, download it there, and Turbine adds it when it lands in Downloads."
    // Buttons: { label }; the first is the main one.
    property var buttons: []
    property int focusIndex: 0
    property int cancelIndex: 1           // the button B presses
    readonly property bool cancelHidden: card.buttonRow.cancelHidden
    onVisibleChanged: if (visible) card.toTop()

    signal activated(int index, string key)
    signal closed()

    readonly property string message: "These mods need " + (loaderName || "a mod loader")
        + (loaderFullName !== "" ? " (" + loaderFullName + ")" : "") + "."
        + (canDownload ? " Install it from its official source?" : "")

    anchors.fill: parent

    Scrim { ui: root.ui }

    DialogCard {
        id: card
        ui: root.ui
        maxWidth: 560
        title: root.title || ""
        buttons: (root.buttons || []).slice(0, 3).map(b => b.label || "")
        focusIndex: root.focusIndex
        mainIndex: 0
        cancelIndex: root.cancelIndex
        onActivated: (i) => root.activated(i, "")
        Text {
            width: parent.width
            text: root.message
            wrapMode: Text.WordWrap
            color: root.ui.c.text
            font.family: root.ui.bodyFont
            font.pixelSize: root.ui.fpx(16)
        }
        Text {
            visible: text !== ""
            width: parent.width
            text: root.explain || ""
            wrapMode: Text.WordWrap
            color: root.ui.c.soft
            font.family: root.ui.bodyFont
            font.pixelSize: root.ui.fpx(14)
        }
        Row {
            visible: (root.neededBy || []).length > 0
            width: parent.width
            spacing: root.ui.px(5)
            Text {
                id: neededLabel
                text: "Needed by:"
                color: root.ui.c.soft
                font.family: root.ui.bodyFont
                font.pixelSize: root.ui.fpx(14)
            }
            Text {
                width: parent.width - neededLabel.width - parent.spacing
                text: (root.neededBy || []).join(", ")
                wrapMode: Text.WordWrap
                color: root.ui.c.text
                font.family: root.ui.bodyFont
                font.pixelSize: root.ui.fpx(14)
                font.weight: Font.DemiBold
            }
        }

        // Turbine can't get it itself: say so plainly.
        Rectangle {
            visible: !root.canDownload && root.infoText !== ""
            width: parent.width
            height: infoRow.implicitHeight + root.ui.px(16)
            radius: root.ui.px(6)
            color: root.ui.c.card
            Row {
                id: infoRow
                x: root.ui.px(12)
                y: root.ui.px(8)
                width: parent.width - root.ui.px(24)
                spacing: root.ui.px(10)
                Rectangle {
                    id: infoMark
                    width: root.ui.px(22)
                    height: width
                    radius: width / 2
                    color: root.ui.c.raised
                    border.width: Math.max(1, root.ui.px(1.5))
                    border.color: root.ui.c.accent
                    Text {
                        anchors.centerIn: parent
                        text: "i"
                        color: root.ui.readable(root.ui.c.accentHi, root.ui.c.raised)
                        font.family: root.ui.bodyFont
                        font.pixelSize: root.ui.fpx(14)
                        font.bold: true
                    }
                }
                Text {
                    width: parent.width - infoMark.width - parent.spacing
                    text: root.infoText || ""
                    wrapMode: Text.WordWrap
                    color: root.ui.c.soft
                    font.family: root.ui.bodyFont
                    font.pixelSize: root.ui.fpx(14)
                }
            }
        }

        // Where to get it, once asked for.
        Rectangle {
            visible: root.sourceShown && root.sourceUrl !== ""
            width: parent.width
            height: sourceCol.implicitHeight + root.ui.px(18)
            radius: root.ui.px(6)
            color: root.ui.c.card
            border.width: Math.max(1, root.ui.px(1.5))
            border.color: root.ui.c.accentLo
            Column {
                id: sourceCol
                x: root.ui.px(12)
                y: root.ui.px(9)
                width: parent.width - root.ui.px(24)
                spacing: root.ui.px(2)
                Text {
                    text: "Official source"
                    color: root.ui.c.muted
                    font.family: root.ui.bodyFont
                    font.pixelSize: root.ui.fpx(14)
                }
                Text {
                    width: parent.width
                    text: root.sourceUrl || ""
                    wrapMode: Text.Wrap
                    color: root.ui.readable(root.ui.c.accentHi, root.ui.c.card)
                    font.family: root.ui.bodyFont
                    font.pixelSize: root.ui.fpx(17)
                    font.weight: Font.DemiBold
                }
            }
        }
    }
}
