import QtQuick

// The beginner tutorial: five cards on first run, each with the real buttons drawn large. 1: A picks, B goes back,
// X options, Y find. 2: View opens the quick wheel. 3: getting a mod (Get mods, A on a mod, sign in to Nexus right in
// Turbine with a free account, Slow download, Added, what it needs lined up after it). 4: START: a tap is Settings, a
// 3 second hold is Play options (Play Seamless Co-op there too). 5 (0.64): good to know: Steam's own Play through
// Turbine (mods), games on an SD card or a second drive, Cyberpunk's Visual C++ runtime, updates. Then "Pick a game to
// start". Main.qml moves between the cards (A or Right next, Left back, B skips, Start finishes) and remembers it was
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
    property bool keys: false             // 0.63: the keyboard or the mouse was used last: the cards name the keys
    property bool thunderstore: false     // the game gets its mods from Thunderstore (Install, not Nexus)
    signal next()
    signal skip()
    signal back()

    anchors.fill: parent
    z: 100

    Rectangle { anchors.fill: parent; color: tut.ui.c.ground; opacity: 0.97 }

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

    Column {
        id: body
        anchors.centerIn: parent
        anchors.verticalCenterOffset: -tut.ui.px(20)
        width: Math.min(parent.width - tut.ui.px(60), tut.ui.px(760))
        spacing: tut.ui.px(18)

        Text {
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
            text: ["The buttons", "The quick wheel", "Getting a mod", "Start", "Good to know"][tut.card] || ""
            color: tut.ui.c.accentText
            font.family: tut.ui.titleFont; font.pixelSize: tut.ui.fpx(28); font.bold: true
        }
        Text {
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
            // 0.59 (T16): short, touch included, the game's own way to get mods, and the real Start glyph.
            text: [tut.keys ? "Enter or Space picks, Esc goes back. Move with W A S D or the arrows. R: options for the highlighted thing. F: find and sort. Q and E: the pages. The mouse works too: click, double-click a mod to open it, right-click for options."
                            : "A picks, B goes back. X: options for the highlighted thing. Y: find and sort. LB and RB: tabs (on a mod page, the next result). Touch works too: tap, or hold for options.",
                   tut.keys ? "Hold Tab for the quick wheel from anywhere, pick with the mouse or W A S D, and let go: Find, Add from Downloads, Settings, your games and more."
                            : "View opens the quick wheel from anywhere: Find, Add from Downloads, Settings, your games and more.",
                   tut.thunderstore ? "Open Get mods, A on a mod, then A on Install. Turbine adds it with what it needs."
                                    : "Open Get mods, A on a mod, then A on Get it on Nexus. Nexus opens right here: sign in once (a free account is enough; with Premium it downloads with no press) and press Slow download. Turbine adds it, and lines up anything it needs.",
                   tut.keys ? "Hold Space for 3 seconds for Play options: play with your mods, or Play Seamless Co-op where it is in. Esc on a game's first page opens Settings. A click on the Play button plays too."
                            : "Hold " + tut.startGlyph + " for 3 seconds for Play options: play with your mods, or Play Seamless Co-op where it is in. A tap of " + tut.startGlyph + " opens Settings. A on the Play button plays too.",
                   "Turbine sets up what it can by itself and asks only when it must."][tut.card] || ""
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
            Face { k: "B"; label: "Goes back"; anchors.right: parent.right; anchors.rightMargin: tut.ui.px(30); anchors.verticalCenter: parent.verticalCenter }
            Face { k: "A"; label: "Picks"; lit: true; anchors.horizontalCenter: parent.horizontalCenter; anchors.bottom: parent.bottom }
        }
        // 2: View and the wheel's eight places.
        Row {
            visible: tut.card === 1
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: tut.ui.px(40)
            Pill { k: tut.viewGlyph; anchors.verticalCenter: parent.verticalCenter }
            Item {
                width: tut.ui.px(250); height: width
                Rectangle { anchors.fill: parent; radius: width / 2; color: tut.ui.c.card; border.color: tut.ui.c.line; border.width: Math.max(1, tut.ui.px(2)) }
                Rectangle { anchors.centerIn: parent; width: tut.ui.px(54); height: width; radius: width / 2; color: tut.ui.c.raised }
                Repeater {
                    model: ["Find", "Updates", "Add from Downloads", "Play options", "Game menu", "Settings", "Games", "Loadouts"]
                    Text {
                        required property int index
                        required property string modelData
                        readonly property real a: (index * 45 - 90) * Math.PI / 180
                        x: parent.width / 2 + Math.cos(a) * tut.ui.px(88) - width / 2
                        y: parent.height / 2 + Math.sin(a) * tut.ui.px(88) - height / 2
                        text: modelData
                        color: index === 5 ? tut.ui.c.accentText : tut.ui.c.text
                        font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.fpx(14); font.weight: Font.DemiBold
                    }
                }
            }
        }
        // 3: the five steps of getting a mod, all inside Turbine.
        Row {
            visible: tut.card === 2
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: tut.ui.px(8)
            Step { n: 1; tile: "Get mods"; words: "Open Get mods (RB)" }
            Step { n: 2; tile: "A"; face: true; words: "A on a mod, then A on Get it on Nexus" }
            Step { n: 3; tile: "Slow download"; words: "Sign in if it asks (free), press Slow download" }
            Step { n: 4; tile: "Added"; words: "Turbine adds it; what it needs comes next" }
            Step { n: 5; tile: "A"; face: true; words: "The ring is on Play: A plays" }
        }
        // 4: Start.
        Column {
            visible: tut.card === 3
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: tut.ui.px(12)
            Pill { k: tut.startGlyph; anchors.horizontalCenter: parent.horizontalCenter; width: tut.ui.px(150); height: tut.ui.px(64) }
            Text { anchors.horizontalCenter: parent.horizontalCenter; text: "Hold 3 seconds: Play options"; color: tut.ui.c.text; font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.fpx(20); font.weight: Font.DemiBold }
            Text { anchors.horizontalCenter: parent.horizontalCenter; text: "Tap: Settings"; color: tut.ui.c.soft; font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.fpx(18) }
        }
    }

    // 5 (0.64): what happens once, by itself or with one press.
    Column {
        visible: tut.card === 4
        anchors.top: body.bottom
        anchors.topMargin: -tut.ui.px(4)
        anchors.horizontalCenter: parent.horizontalCenter
        width: body.width
        spacing: tut.ui.px(10)
        Repeater {
            model: [
                ["Steam's Play button", "To play with mods from Steam, pick Turbine (mods) once in the game's Properties, Compatibility. Turbine shows the steps when it is needed."],
                ["SD card or second drive", "Games there are found by themselves, like the rest."],
                ["Cyberpunk 2077", "Its frameworks need Microsoft's Visual C++ runtime. Before you play offers it in one press."],
                ["Updates", "Turbine looks for a new version when it starts and installs it only when you say yes."]
            ]
            Row {
                required property var modelData
                width: parent.width
                spacing: tut.ui.px(12)
                Rectangle { width: tut.ui.px(10); height: width; radius: width / 2; color: tut.ui.c.accent; anchors.verticalCenter: parent.verticalCenter }
                Text {
                    width: parent.width - tut.ui.px(22)
                    wrapMode: Text.WordWrap
                    textFormat: Text.PlainText
                    text: parent.modelData[0] + ": " + parent.modelData[1]
                    color: tut.ui.c.text
                    font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.fpx(17)
                }
            }
        }
    }

    // Where you are, and what A and B do here.
    Column {
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
            // Back a card by touch (0.59; Left does it with the controller).
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
