import QtQuick

// The beginner tutorial: five cards on first run, each with the real buttons drawn large. 1: A picks, B goes back,
// X options, Y find. 2: VIEW opens the wheel. 3: getting a mod (Get mods, A on a mod, sign in to Nexus right in
// Turbine with a free account, Slow download, Added, what it needs lined up after it). 4: START: a tap is Settings, a
// 3 second hold is Play options (Play Seamless Co-op there too). 5: good to know: Steam's own Play through
// Turbine (mods), games on an SD card or a second drive, Cyberpunk's Visual C++ runtime, updates. Then "Pick a game to
// start". Each card is its title, one or two lines and a picture, centred above the page dots and the buttons.
// Main.qml moves between the cards (A or Right next, Left back, B skips, Start finishes) and remembers it was
// seen; Settings, Help shows it again.
Overlay {
    id: tut
    closeOnOutside: false
    required property var ui
    motion: ui.motion
    property int card: 0
    property int count: 5
    // The pad's names for View and Start ("VIEW", "MENU", "BACK", "START"), from Main.qml's glyphs.
    property string viewGlyph: "VIEW"
    property string startGlyph: "START"
    property bool keys: false             // the keyboard or the mouse was used last: the cards name the keys
    property bool thunderstore: false     // the game gets its mods from Thunderstore (Install, not Nexus)
    signal next()
    signal skip()
    signal back()

    anchors.fill: parent
    z: 100
    // It fades in whole over the page, and each card comes in from the side it was asked from (Next from the right,
    // Back from the left).
    fadesWhole: true
    backdrop: ground
    property int cardWas: 0
    onCardChanged: { if (visible) cardIn.play((card < cardWas ? -1 : 1) * ui.px(ui.motion.shift), 0); cardWas = card }

    // The cards cover the window entirely: nothing behind them competes with their words.
    Rectangle { id: ground; anchors.fill: parent; color: tut.ui.c.ground }

    // A face button as it looks on the pad: round, its letter in the middle.
    component Face: Column {
        id: face
        property string k: ""
        property string label: ""
        property bool lit: false
        spacing: tut.ui.px(6)
        Rectangle {
            anchors.horizontalCenter: parent.horizontalCenter
            width: tut.ui.px(64); height: width; radius: width / 2
            color: face.lit ? tut.ui.c.accent : tut.ui.c.raised
            border.color: face.lit ? tut.ui.c.accentHi : tut.ui.c.accentLo
            border.width: Math.max(2, tut.ui.px(3))
            Text { anchors.centerIn: parent; text: face.k; color: face.lit ? tut.ui.c.onAccent : tut.ui.c.text; font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.fpx(30); font.bold: true }
        }
        Text { anchors.horizontalCenter: parent.horizontalCenter; text: face.label; color: tut.ui.c.text; font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.fpx(17); font.weight: Font.DemiBold }
    }
    // A wide button (View, Start, LB): a rounded pill with its name.
    component Pill: Rectangle {
        property string k: ""
        property bool lit: true
        width: Math.max(tut.ui.px(96), pillText.implicitWidth + tut.ui.px(36)); height: tut.ui.px(52); radius: tut.ui.px(14)
        color: lit ? tut.ui.c.accent : tut.ui.c.raised
        border.color: lit ? tut.ui.c.accentHi : tut.ui.c.accentLo
        border.width: Math.max(2, tut.ui.px(3))
        Text { id: pillText; anchors.centerIn: parent; text: parent.k; color: parent.lit ? tut.ui.c.onAccent : tut.ui.c.text; font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.fpx(22); font.bold: true }
    }
    // One step of getting a mod: a number, a picture-like tile and its words.
    component Step: Column {
        id: st
        property int n: 1
        property string tile: ""
        property string words: ""
        property bool face: false
        width: tut.ui.px(136)
        spacing: tut.ui.px(8)
        Rectangle {
            anchors.horizontalCenter: parent.horizontalCenter
            width: tut.ui.px(126); height: tut.ui.px(72); radius: tut.ui.px(10)
            color: tut.ui.c.card
            border.color: tut.ui.c.accent; border.width: Math.max(1, tut.ui.px(2))
            Rectangle {
                anchors.centerIn: parent
                width: st.face ? tut.ui.px(46) : Math.min(tut.ui.px(116), stepTop.implicitWidth + tut.ui.px(16))
                height: tut.ui.px(46); radius: st.face ? width / 2 : tut.ui.px(8)
                color: tut.ui.c.accent
                Text { id: stepTop; anchors.centerIn: parent; text: st.tile; color: tut.ui.c.onAccent; font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.fpx(st.face ? 24 : 14); font.bold: true }
            }
        }
        Text { width: st.width; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap; text: st.n + ". " + st.words; color: tut.ui.c.text; font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.fpx(16) }
    }

    // The card: the space above the dots and buttons, its words and picture centred in it (never under them).
    Item {
        id: stage
        anchors.top: parent.top
        anchors.topMargin: tut.ui.px(24)
        anchors.bottom: footer.top
        anchors.bottomMargin: tut.ui.px(20)
        anchors.left: parent.left
        anchors.right: parent.right
    }
    Column {
        id: body
        opacity: cardIn.part
        transform: Arrive { id: cardIn; motion: tut.ui.motion }
        anchors.centerIn: stage
        width: Math.min(parent.width - tut.ui.px(60), tut.ui.px(760))
        spacing: tut.ui.px(18)

        Text {
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
            text: ["The buttons", "The wheel", "Getting a mod", "Play options", "Good to know"][tut.card] || ""
            color: tut.ui.c.accentText
            font.family: tut.ui.titleFont; font.pixelSize: tut.ui.fpx(28); font.bold: true
        }
        // One or two lines: the picture under it says the rest.
        Text {
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
            text: [tut.keys ? "Enter or Space picks, Esc goes back, R shows options, F finds and sorts, Q and E switch tabs. The mouse works too: right-click for options."
                            : "LB and RB switch tabs. Touch works too: tap to pick, hold for options.",
                   tut.keys ? "Hold Tab anywhere for the wheel, point with the mouse or W A S D, then let go."
                            : "Press " + tut.viewGlyph + " anywhere for the wheel, then push the stick to a place.",
                   tut.thunderstore ? "It all happens in Turbine, from Thunderstore."
                                    : "It all happens in Turbine. A free Nexus account is enough.",
                   tut.keys ? "Hold Space for 3 seconds to choose how to play: with or without mods, or Seamless Co-op."
                            : "Hold " + tut.startGlyph + " for 3 seconds to choose how to play: with or without mods, or Seamless Co-op.",
                   "Turbine sets up what it can and asks only when it must."][tut.card] || ""
            color: tut.ui.c.soft
            font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.fpx(18)
        }

        // 1: the four face buttons, laid out as on the pad.
        Item {
            visible: tut.card === 0
            anchors.horizontalCenter: parent.horizontalCenter
            width: tut.ui.px(380); height: tut.ui.px(260)
            Face { k: "Y"; label: "Find"; anchors.horizontalCenter: parent.horizontalCenter; y: 0 }
            Face { k: "X"; label: "Options"; x: tut.ui.px(40); anchors.verticalCenter: parent.verticalCenter }
            Face { k: "B"; label: "Back"; anchors.right: parent.right; anchors.rightMargin: tut.ui.px(30); anchors.verticalCenter: parent.verticalCenter }
            Face { k: "A"; label: "Pick"; lit: true; anchors.horizontalCenter: parent.horizontalCenter; anchors.bottom: parent.bottom }
        }
        // 2: View and the wheel's eight places, each inside its wedge.
        Row {
            visible: tut.card === 1
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: tut.ui.px(40)
            Pill { k: tut.viewGlyph; anchors.verticalCenter: parent.verticalCenter }
            Item {
                id: wheelPic
                width: tut.ui.px(300); height: width
                Rectangle { anchors.fill: parent; radius: width / 2; color: tut.ui.c.card; border.color: tut.ui.c.line; border.width: Math.max(1, tut.ui.px(2)) }
                // The wedges' borders, between the names.
                Repeater {
                    model: 8
                    Rectangle {
                        required property int index
                        x: wheelPic.width / 2 - width / 2; y: wheelPic.height / 2 - height
                        width: Math.max(1, tut.ui.px(2)); height: wheelPic.height / 2 - tut.ui.px(2)
                        color: tut.ui.c.line
                        transformOrigin: Item.Bottom
                        rotation: index * 45 + 22.5
                    }
                }
                Rectangle { anchors.centerIn: parent; width: tut.ui.px(70); height: width; radius: width / 2; color: tut.ui.c.raised; border.color: tut.ui.c.line; border.width: Math.max(1, tut.ui.px(2)) }
                Repeater {
                    model: ["Find", "Updates", "Add from Downloads", "Play options", "Game menu", "Settings", "Games", "Loadouts"]
                    Text {
                        required property int index
                        required property string modelData
                        readonly property real a: (index * 45 - 90) * Math.PI / 180
                        width: tut.ui.px(index % 2 === 1 ? 70 : 84)
                        x: wheelPic.width / 2 + Math.cos(a) * tut.ui.px(98) - width / 2
                        y: wheelPic.height / 2 + Math.sin(a) * tut.ui.px(98) - height / 2
                        horizontalAlignment: Text.AlignHCenter
                        wrapMode: Text.WordWrap
                        text: modelData
                        color: index === 5 ? tut.ui.c.accentText : tut.ui.c.text
                        font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.fpx(13); font.weight: Font.DemiBold
                    }
                }
            }
        }
        // 3: the steps of getting a mod, all inside Turbine.
        Row {
            visible: tut.card === 2
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: tut.ui.px(8)
            Repeater {
                model: tut.thunderstore
                       ? [["Get mods", false, "Open Get mods with RB"], ["A", true, "Pick a mod, then Install"], ["Added", false, "Turbine adds it and what it needs"], ["A", true, "Play is ready: press A"]]
                       : [["Get mods", false, "Open Get mods with RB"], ["A", true, "Pick a mod, then Get it on Nexus"], ["Slow download", false, "Sign in if asked (free), then press Slow download"],
                          ["Added", false, "Turbine adds it and what it needs"], ["A", true, "Play is ready: press A"]]
                Step { required property int index; required property var modelData; n: index + 1; tile: modelData[0]; face: modelData[1]; words: modelData[2] }
            }
        }
        // 4: Start: a hold is Play options, a tap Settings.
        Column {
            visible: tut.card === 3
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: tut.ui.px(12)
            Pill { k: tut.startGlyph; anchors.horizontalCenter: parent.horizontalCenter; width: tut.ui.px(150); height: tut.ui.px(64) }
            Text { anchors.horizontalCenter: parent.horizontalCenter; text: "Hold 3 seconds: Play options"; color: tut.ui.c.text; font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.fpx(20); font.weight: Font.DemiBold }
            Text { anchors.horizontalCenter: parent.horizontalCenter; text: tut.keys ? "Esc on a game's first page: Settings" : "Tap: Settings"; color: tut.ui.c.soft; font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.fpx(18) }
        }
        // 5: what happens once, automatically or with one press.
        Column {
            visible: tut.card === 4
            width: parent.width
            spacing: tut.ui.px(12)
            Repeater {
                model: [
                    ["Steam's Play button", "to play with mods from Steam, pick Turbine (mods) once under Properties, Compatibility. Turbine walks you through it."],
                    ["SD card or second drive", "games there show up like the rest."],
                    ["Cyberpunk 2077", "its mod frameworks need Microsoft's Visual C++ runtime. Before you play installs it in one press."],
                    ["Updates", "Turbine checks for a new version at startup and installs it only when you say yes."]
                ]
                Item {
                    required property var modelData
                    width: parent.width
                    height: bullet.implicitHeight
                    Rectangle {
                        width: tut.ui.px(10); height: width; radius: width / 2; color: tut.ui.c.accent
                        y: (bulletMetrics.height - height) / 2
                    }
                    Text {
                        id: bullet
                        x: tut.ui.px(22)
                        width: parent.width - x
                        wrapMode: Text.WordWrap
                        textFormat: Text.StyledText
                        text: "<b>" + parent.modelData[0] + ":</b> " + parent.modelData[1]
                        color: tut.ui.c.text
                        font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.fpx(17)
                    }
                    FontMetrics { id: bulletMetrics; font: bullet.font }
                }
            }
        }
    }

    // Where you are, and what A and B do here.
    Column {
        id: footer
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: tut.ui.px(28)
        spacing: tut.ui.px(14)
        Row {
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: tut.ui.px(10)
            Repeater {
                model: tut.count
                Rectangle { required property int index; width: tut.ui.px(10); height: width; radius: width / 2; color: index === tut.card ? tut.ui.c.accent : tut.ui.c.line }
            }
        }
        Row {
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: tut.ui.px(28)
            // Back a card by touch (Left does it with the controller).
            Row {
                spacing: tut.ui.px(10)
                height: tut.ui.tpx(44)
                visible: tut.card > 0
                Text { anchors.verticalCenter: parent.verticalCenter; text: "Back"; color: tut.ui.c.soft; font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.fpx(20) }
                TapHandler { onTapped: tut.back() }
            }
            Row {
                spacing: tut.ui.px(10)
                height: tut.ui.tpx(44)
                Rectangle {
                    anchors.verticalCenter: parent.verticalCenter
                    width: tut.ui.px(40); height: width; radius: width / 2; color: tut.ui.c.accent
                    Text { anchors.centerIn: parent; text: "A"; color: tut.ui.c.onAccent; font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.fpx(20); font.bold: true }
                }
                Text { anchors.verticalCenter: parent.verticalCenter; text: tut.card < tut.count - 1 ? "Next" : "Pick a game to start"; color: tut.ui.c.text; font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.fpx(20); font.weight: Font.DemiBold }
                TapHandler { onTapped: tut.next() }
            }
            Row {
                spacing: tut.ui.px(10)
                height: tut.ui.tpx(44)
                visible: tut.card < tut.count - 1
                Rectangle {
                    anchors.verticalCenter: parent.verticalCenter
                    width: tut.ui.px(40); height: width; radius: width / 2; color: tut.ui.c.raised; border.color: tut.ui.c.accentLo; border.width: Math.max(1, tut.ui.px(2))
                    Text { anchors.centerIn: parent; text: "B"; color: tut.ui.c.text; font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.fpx(20); font.bold: true }
                }
                Text { anchors.verticalCenter: parent.verticalCenter; text: "Skip"; color: tut.ui.c.soft; font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.fpx(20) }
                TapHandler { onTapped: tut.skip() }
            }
        }
    }
}
