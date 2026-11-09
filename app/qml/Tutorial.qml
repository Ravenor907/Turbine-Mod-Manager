import QtQuick

// The beginner tutorial: four cards on first run, each with the real buttons drawn large. 1: A picks, B goes back,
// X options, Y find. 2: View opens the quick wheel. 3: getting a mod (Get mods, A on a mod, Slow download on Nexus
// right in Turbine, Added, START plays). 4: Press START to play, then "Pick a game to start". Main.qml moves between the cards (A or Right next,
// Left back, B skips, Start finishes) and remembers it was seen; Settings, Help shows it again.
Item {
    id: tut
    required property var ui
    property int card: 0
    property int count: 4
    // The pad's names for View and Start ("VIEW", "MENU", "BACK", "START"), from Main.qml's glyphs.
    property string viewGlyph: "VIEW"
    property string startGlyph: "START"
    signal next()
    signal skip()

    anchors.fill: parent
    z: 100

    Rectangle { anchors.fill: parent; color: tut.ui.c.ground; opacity: 0.97 }
    MouseArea { anchors.fill: parent }

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
            Text { anchors.centerIn: parent; text: face.k; color: face.lit ? tut.ui.c.onAccent : tut.ui.c.text; font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.px(30); font.bold: true }
        }
        Text { anchors.horizontalCenter: parent.horizontalCenter; text: face.label; color: tut.ui.c.text; font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.px(17); font.weight: Font.DemiBold }
    }
    // A wide button (View, Start, LB): a rounded pill with its name.
    component Pill: Rectangle {
        property string k: ""
        property bool lit: true
        width: Math.max(tut.ui.px(96), pillText.implicitWidth + tut.ui.px(36)); height: tut.ui.px(52); radius: tut.ui.px(14)
        color: lit ? tut.ui.c.accent : tut.ui.c.raised
        border.color: lit ? tut.ui.c.accentHi : tut.ui.c.accentLo
        border.width: Math.max(2, tut.ui.px(3))
        Text { id: pillText; anchors.centerIn: parent; text: parent.k; color: parent.lit ? tut.ui.c.onAccent : tut.ui.c.text; font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.px(22); font.bold: true }
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
                Text { id: stepTop; anchors.centerIn: parent; text: st.tile; color: tut.ui.c.onAccent; font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.px(st.face ? 24 : 14); font.bold: true }
            }
        }
        Text { width: st.width; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap; text: st.n + ". " + st.words; color: tut.ui.c.text; font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.px(16) }
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
            text: ["The buttons", "The quick wheel", "Getting a mod", "Play"][tut.card] || ""
            color: tut.ui.c.accentText
            font.family: tut.ui.titleFont; font.pixelSize: tut.ui.px(28); font.bold: true
        }
        Text {
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
            text: ["A picks and B goes back. X shows options for what's highlighted, and Y searches this list. LB and RB switch tabs. The right stick moves a pointer, and RT clicks.",
                   "Press View for the quick wheel: Find, Add from Downloads, Settings, your games and more, from anywhere. Games: Left to the rail, or View then Left.",
                   "Open Get mods, A on a mod, then A on Get it on Nexus. Sign in if it asks (free) and press Slow download. Turbine adds it and closes the page.",
                   "Press START to play the game with your mods on. That's all you need."][tut.card] || ""
            color: tut.ui.c.soft
            font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.px(18)
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
                        font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.px(14); font.weight: Font.DemiBold
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
            Step { n: 4; tile: "Added"; words: "Turbine adds it and closes the page" }
            Step { n: 5; tile: tut.startGlyph; words: tut.startGlyph + " plays" }
        }
        // 4: Start.
        Column {
            visible: tut.card === 3
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: tut.ui.px(12)
            Pill { k: tut.startGlyph; anchors.horizontalCenter: parent.horizontalCenter; width: tut.ui.px(150); height: tut.ui.px(64) }
            Text { anchors.horizontalCenter: parent.horizontalCenter; text: "Play"; color: tut.ui.c.text; font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.px(20); font.weight: Font.DemiBold }
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
            Row {
                spacing: tut.ui.px(10)
                Rectangle {
                    width: tut.ui.px(40); height: width; radius: width / 2; color: tut.ui.c.accent
                    Text { anchors.centerIn: parent; text: "A"; color: tut.ui.c.onAccent; font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.px(20); font.bold: true }
                }
                Text { anchors.verticalCenter: parent.verticalCenter; text: tut.card < tut.count - 1 ? "Next" : "Pick a game to start"; color: tut.ui.c.text; font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.px(20); font.weight: Font.DemiBold }
                TapHandler { onTapped: tut.next() }
            }
            Row {
                spacing: tut.ui.px(10)
                visible: tut.card < tut.count - 1
                Rectangle {
                    width: tut.ui.px(40); height: width; radius: width / 2; color: tut.ui.c.raised; border.color: tut.ui.c.accentLo; border.width: Math.max(1, tut.ui.px(2))
                    Text { anchors.centerIn: parent; text: "B"; color: tut.ui.c.text; font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.px(20); font.bold: true }
                }
                Text { anchors.verticalCenter: parent.verticalCenter; text: "Skip"; color: tut.ui.c.soft; font.family: tut.ui.bodyFont; font.pixelSize: tut.ui.px(20) }
                TapHandler { onTapped: tut.skip() }
            }
        }
    }
}
