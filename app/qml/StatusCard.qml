import QtQuick

// Turbine's status card, shown over the browser while a download is caught: what is happening, progress, the
// "Added to <game>" result, and, when Turbine needs an answer, either buttons (the Desktop Mode panel, which takes
// clicks and taps) or a line saying where to answer (the Game Mode overlay, which gamescope never gives input).
Rectangle {
    id: card
    required property var ui
    property var status: null             // {text, level, progress, game, added, done, ask}
    property var fomod: null              // the installer waiting for choices (Main.qml's fomod), or null
    property bool interactive: true

    signal confirm(bool yes)
    signal pick()
    signal choose(int group, int option)
    signal next()
    signal back()
    signal stop()
    signal openTurbine()

    readonly property string level: status ? (status.level || "ok") : "ok"
    readonly property real progress: status && status.progress !== undefined ? status.progress : -1
    readonly property bool asking: !!status && !!status.ask
    readonly property var step: fomod && fomod.data ? fomod.data.step : null
    readonly property bool choosing: step !== null && !asking && !(status && status.done)

    function levelColor() {
        return level === "ok" ? ui.c.ok : level === "stop" ? ui.c.stop : level === "warn" ? ui.c.warn : ui.c.accentHi
    }

    width: ui.px(340)
    height: col.implicitHeight + ui.px(28)
    radius: ui.px(10)
    color: ui.c.panel
    border.color: ui.c.accentLo
    border.width: Math.max(1, ui.px(1))
    Rectangle { x: parent.radius; width: parent.width - parent.radius * 2; height: ui.px(3); radius: ui.px(2); color: ui.c.accent }

    Column {
        id: col
        x: ui.px(16); y: ui.px(14)
        width: parent.width - ui.px(32)
        spacing: ui.px(8)
        Row {
            width: parent.width
            spacing: ui.px(8)
            Rectangle {
                width: ui.px(10); height: width; radius: width / 2
                anchors.verticalCenter: parent.verticalCenter
                color: card.levelColor()
            }
            Text {
                text: "Turbine"
                color: ui.c.accentText
                font.family: ui.bodyFont; font.pixelSize: ui.px(14); font.bold: true
                anchors.verticalCenter: parent.verticalCenter
            }
            Text {
                visible: !!card.status && !!card.status.game
                width: parent.width - ui.px(110)
                elide: Text.ElideRight
                text: card.status && card.status.game ? "· " + card.status.game : ""
                color: ui.c.muted
                font.family: ui.bodyFont; font.pixelSize: ui.px(14)
                anchors.verticalCenter: parent.verticalCenter
            }
        }
        Text {
            width: parent.width
            wrapMode: Text.WordWrap
            maximumLineCount: 3
            elide: Text.ElideRight
            text: card.status && card.status.text ? card.status.text : ""
            color: ui.c.text
            font.family: ui.bodyFont; font.pixelSize: ui.px(17); font.bold: true
        }
        Text {
            visible: !!card.status && !!card.status.added
            width: parent.width
            wrapMode: Text.WordWrap
            maximumLineCount: 2
            elide: Text.ElideRight
            text: card.status && card.status.added ? card.status.added + " is in your mods, turned on" : ""
            color: ui.c.soft
            font.family: ui.bodyFont; font.pixelSize: ui.px(15)
        }
        // Progress
        Item {
            visible: card.progress >= 0 && !(card.status && card.status.done)
            width: parent.width; height: ui.px(22)
            Rectangle {
                id: track
                anchors.verticalCenter: parent.verticalCenter
                width: parent.width - pct.width - ui.px(10); height: ui.px(8); radius: height / 2
                color: ui.c.raised
                Rectangle { width: parent.width * Math.max(0, Math.min(1, card.progress)); height: parent.height; radius: parent.radius; color: ui.c.accent }
            }
            Text {
                id: pct
                anchors.right: parent.right; anchors.verticalCenter: parent.verticalCenter
                text: Math.round(Math.max(0, card.progress) * 100) + "%"
                color: ui.c.soft
                font.family: ui.bodyFont; font.pixelSize: ui.px(14)
            }
        }
        // Where to answer, when this card can't take input.
        Text {
            visible: !card.interactive && (card.asking || card.choosing)
            width: parent.width
            wrapMode: Text.WordWrap
            text: card.asking ? "Switch to Turbine to answer (Steam button, then Turbine): A add it, X another game, B not a mod."
                  : "Pick the installer's choices on the Nexus page, or in Turbine."
            color: ui.c.warn
            font.family: ui.bodyFont; font.pixelSize: ui.px(14)
        }
        // "Add it?" (Desktop Mode panel)
        Row {
            visible: card.interactive && card.asking
            spacing: ui.px(8)
            CardButton { visible: !!card.status && !!card.status.game; label: "Add it"; strong: true; onClicked: card.confirm(true) }
            CardButton { label: card.status && card.status.game ? "Other game" : "Pick the game"; strong: !(card.status && card.status.game); onClicked: card.pick() }
            CardButton { label: "Not a mod"; onClicked: card.confirm(false) }
        }
        // Installer choices (Desktop Mode panel)
        Column {
            visible: card.interactive && card.choosing
            width: parent.width
            spacing: ui.px(6)
            Text {
                width: parent.width
                elide: Text.ElideRight
                text: card.step ? (card.fomod.data.name || "") + (card.step.name ? ": " + card.step.name : "") : ""
                color: ui.c.soft
                font.family: ui.bodyFont; font.pixelSize: ui.px(14); font.bold: true
            }
            Repeater {
                model: card.step ? card.step.groups : []
                Column {
                    id: grp
                    required property int index
                    required property var modelData
                    width: parent.width
                    spacing: ui.px(4)
                    Text {
                        width: parent.width
                        elide: Text.ElideRight
                        text: (grp.modelData.name || "Options") + " · " + (grp.modelData.hint || "")
                        color: ui.c.muted
                        font.family: ui.bodyFont; font.pixelSize: ui.px(13)
                    }
                    Repeater {
                        model: grp.modelData.options
                        Rectangle {
                            required property int index
                            required property var modelData
                            width: grp.width; height: ui.px(32); radius: ui.px(5)
                            color: modelData.selected ? ui.c.focusFill : ui.c.card
                            border.color: modelData.selected ? ui.c.accent : ui.c.line
                            border.width: Math.max(1, ui.px(1))
                            opacity: modelData.canChange ? 1 : 0.6
                            Text {
                                x: ui.px(10); width: parent.width - ui.px(20)
                                anchors.verticalCenter: parent.verticalCenter
                                elide: Text.ElideRight
                                text: (modelData.selected ? "✓ " : "") + modelData.name
                                color: ui.c.text
                                font.family: ui.bodyFont; font.pixelSize: ui.px(14); font.bold: modelData.selected
                            }
                            TapHandler { enabled: parent.modelData.canChange; onTapped: card.choose(grp.index, parent.index) }
                        }
                    }
                }
            }
            Row {
                spacing: ui.px(8)
                CardButton { label: card.step && card.step.isLast ? "Install" : "Next"; strong: true; onClicked: card.next() }
                CardButton { visible: !!card.step && card.step.canGoBack; label: "Back"; onClicked: card.back() }
                CardButton { label: "Cancel"; onClicked: card.stop() }
            }
        }
        CardButton {
            visible: card.interactive && !card.asking && !card.choosing
            label: "Open Turbine"
            onClicked: card.openTurbine()
        }
    }

    component CardButton: Rectangle {
        id: b
        property string label: ""
        property bool strong: false
        signal clicked()
        width: t.implicitWidth + card.ui.px(24); height: card.ui.px(34); radius: card.ui.px(6)
        color: strong ? card.ui.c.accent : card.ui.c.card
        border.color: strong ? card.ui.c.accentHi : card.ui.c.line
        border.width: Math.max(1, card.ui.px(1))
        Text {
            id: t
            anchors.centerIn: parent
            text: b.label
            color: b.strong ? card.ui.c.onAccent : card.ui.c.text
            font.family: card.ui.bodyFont; font.pixelSize: card.ui.px(14); font.bold: true
        }
        TapHandler { onTapped: b.clicked() }
    }
}
