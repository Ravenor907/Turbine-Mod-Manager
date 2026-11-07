import QtQuick

// Profiles > Create a Thunderstore code: uploads the profile, then shows its code big enough to read across a room
// and copies it. A copies it again, B closes.
Item {
    id: cs
    required property var ui
    anchors.fill: parent

    property var info: null               // {state: "working" | "ready" | "error", profile, code, left, files, message}

    signal copyClicked()
    signal closed()

    readonly property string state_: info ? info.state : ""

    Rectangle { anchors.fill: parent; color: "#C8000000" }
    MouseArea { anchors.fill: parent; onClicked: cs.closed() }
    Rectangle {
        anchors.centerIn: parent
        width: Math.min(parent.width - cs.ui.px(40), cs.ui.px(720))
        height: col.implicitHeight + cs.ui.px(40)
        radius: cs.ui.px(10)
        color: cs.ui.c.panel
        border.color: cs.ui.c.line
        MouseArea { anchors.fill: parent }
        Column {
            id: col
            x: cs.ui.px(22); y: cs.ui.px(20)
            width: parent.width - cs.ui.px(44)
            spacing: cs.ui.px(12)
            Text {
                text: "Thunderstore profile code"
                color: cs.ui.c.text
                font.family: cs.ui.titleFont; font.pixelSize: cs.ui.px(18); font.bold: true
            }
            Text {
                width: parent.width
                wrapMode: Text.WordWrap
                text: cs.info ? "Profile \"" + cs.info.profile + "\"" : ""
                color: cs.ui.c.soft
                font.family: cs.ui.bodyFont; font.pixelSize: cs.ui.px(15)
            }
            Rectangle {
                width: parent.width
                height: codeText.implicitHeight + cs.ui.px(28)
                radius: cs.ui.px(8)
                color: cs.ui.c.card
                border.color: cs.state_ === "ready" ? cs.ui.c.accent : cs.ui.c.line
                border.width: Math.max(1, cs.ui.px(cs.state_ === "ready" ? 2 : 1))
                Text {
                    id: codeText
                    anchors.centerIn: parent
                    width: parent.width - cs.ui.px(24)
                    horizontalAlignment: Text.AlignHCenter
                    // One line, as big as fits (up to 30 design px): codes are read off the screen.
                    wrapMode: Text.NoWrap
                    fontSizeMode: Text.HorizontalFit
                    minimumPixelSize: cs.ui.px(14)
                    textFormat: Text.PlainText
                    text: cs.state_ === "ready" ? cs.info.code : cs.state_ === "working" ? "Uploading..." : "No code"
                    color: cs.state_ === "ready" ? cs.ui.c.accentText : cs.ui.c.muted
                    font.family: "monospace"
                    font.pixelSize: cs.ui.px(cs.state_ === "ready" ? 30 : 22)
                    font.bold: true
                }
            }
            Text {
                width: parent.width
                wrapMode: Text.WordWrap
                text: cs.state_ === "working" ? "Uploading the profile's Thunderstore mods and its BepInEx settings to Thunderstore..."
                      : cs.state_ === "error" ? (cs.info.message || "The code couldn't be made.")
                      : "Copied. Anyone can paste this code into Turbine (Profiles, Import a Thunderstore profile code) or another Thunderstore mod tool to get the same mods."
                color: cs.state_ === "error" ? cs.ui.c.warn : cs.ui.c.text
                font.family: cs.ui.bodyFont; font.pixelSize: cs.ui.px(15)
            }
            Text {
                visible: !!cs.info && cs.state_ === "ready" && (cs.info.left > 0 || cs.info.files > 0)
                width: parent.width
                wrapMode: Text.WordWrap
                text: !cs.info ? "" : [
                    cs.info.files > 0 ? cs.info.files + (cs.info.files === 1 ? " settings file is" : " settings files are") + " included." : "",
                    cs.info.left > 0 ? cs.info.left + (cs.info.left === 1 ? " mod isn't" : " mods aren't") + " in the code: only Thunderstore mods can be." : ""
                ].filter(s => s !== "").join(" ")
                color: cs.ui.c.soft
                font.family: cs.ui.bodyFont; font.pixelSize: cs.ui.px(14)
            }
            Row {
                spacing: cs.ui.px(18)
                Row {
                    visible: cs.state_ === "ready"
                    spacing: cs.ui.px(6)
                    Rectangle {
                        width: cs.ui.px(24); height: width; radius: width / 2; color: cs.ui.c.raised; border.color: cs.ui.c.accentLo
                        anchors.verticalCenter: parent.verticalCenter
                        Text { anchors.centerIn: parent; text: "A"; color: cs.ui.c.text; font.family: cs.ui.bodyFont; font.pixelSize: cs.ui.px(13); font.bold: true }
                    }
                    Text { text: "Copy again"; color: cs.ui.c.soft; font.family: cs.ui.bodyFont; font.pixelSize: cs.ui.px(15); anchors.verticalCenter: parent.verticalCenter }
                    HoverHandler { onHoveredChanged: cs.ui.hover(parent, hovered) }
                    TapHandler { onTapped: cs.copyClicked() }
                }
                Row {
                    spacing: cs.ui.px(6)
                    Rectangle {
                        width: cs.ui.px(24); height: width; radius: width / 2; color: cs.ui.c.raised; border.color: cs.ui.c.accentLo
                        anchors.verticalCenter: parent.verticalCenter
                        Text { anchors.centerIn: parent; text: "B"; color: cs.ui.c.text; font.family: cs.ui.bodyFont; font.pixelSize: cs.ui.px(13); font.bold: true }
                    }
                    Text { text: "Close"; color: cs.ui.c.soft; font.family: cs.ui.bodyFont; font.pixelSize: cs.ui.px(15); anchors.verticalCenter: parent.verticalCenter }
                    HoverHandler { onHoveredChanged: cs.ui.hover(parent, hovered) }
                    TapHandler { onTapped: cs.closed() }
                }
            }
        }
    }
}
