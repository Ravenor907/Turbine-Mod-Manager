import QtQuick

// "These mods need SKSE (Skyrim Script Extender)": said once, in plain words, when mods need a loader the game
// doesn't have. Two variants:
//   canDownload: true   the later one: A Install, B Not now
//   canDownload: false  Turbine can't get it itself: an info line, A Open its page (in Turbine), B Not now
// A centered card over a dimmed backdrop. It fills its parent; show and hide it with visible.
//
// ui (required): an object that gives
//   ui.c          colors: ground, panel, card, raised, line, text, soft, muted, accent, accentHi, accentLo,
//                 onAccent, focusFill, ok, warn, light (bool); stop is optional (a fallback is used without it)
//   ui.px(n)      design pixels to screen pixels
//   ui.titleFont  font family for titles
//   ui.bodyFont   font family for everything else
Overlay {
    id: root
    onOutsideTapped: root.closed()

    required property var ui
    motion: ui.motion

    property string title: "Mod loader needed"
    property string loaderName: ""        // short name, e.g. "SKSE"
    property string loaderFullName: ""    // e.g. "Skyrim Script Extender"
    property string gameName: ""          // e.g. "Skyrim"
    property string explain: ""           // what the loader is, in plain words
    property var neededBy: []             // mod names
    property string sourceUrl: ""         // official home, e.g. "skse.silverlock.org"
    property bool canDownload: false
    property bool sourceShown: false
    property string infoText: "Turbine can't get " + (loaderName || "it") + " by itself. Open its page, download it there, and Turbine adds it when it lands in Downloads."
    // Buttons: { key, label }. Empty uses the defaults for the variant.
    property var buttons: []
    property int focusIndex: 0
    // The buttons stay in view when the words don't fit (0.59).
    onFocusIndexChanged: Qt.callLater(() => scroller.follow(loaderButtons))
    onVisibleChanged: if (visible) Qt.callLater(() => scroller.follow(loaderButtons))

    signal activated(int index, string key)
    signal closed()

    readonly property var shownButtons: (buttons || []).length > 0 ? buttons
        : canDownload ? [{ key: "a", label: "Install" }, { key: "b", label: "Not now" }]
                      : [{ key: "a", label: "Open its page" }, { key: "b", label: "Not now" }]
    readonly property string message: "These mods need " + (loaderName || "a mod loader")
        + (loaderFullName !== "" ? " (" + loaderFullName + ")" : "") + "."
        + (canDownload ? " Install it from its official source?" : "")

    // Contrast: the same hue, darkened on light grounds or lightened on dark ones until it reads at 4.5:1 or better.
    function lum(col) {
        const f = v => v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4)
        return 0.2126 * f(col.r) + 0.7152 * f(col.g) + 0.0722 * f(col.b)
    }
    function readable(col, bg) {
        let out = col
        const lb = lum(bg)
        for (let i = 0; i < 14; i++) {
            const lc = lum(out)
            if ((Math.max(lc, lb) + 0.05) / (Math.min(lc, lb) + 0.05) >= 4.5) break
            out = lb < 0.18 ? Qt.lighter(out, 1.12) : Qt.darker(out, 1.12)
        }
        return out
    }

    anchors.fill: parent

    Rectangle {
        anchors.fill: parent
        color: Qt.rgba(0.02, 0.016, 0.012, 0.74)
    }

    Rectangle {
        id: card
        anchors.centerIn: parent
        width: Math.min(parent.width - root.ui.px(40), root.ui.px(520))
        height: Math.min(content.implicitHeight + root.ui.px(40), parent.height - root.ui.px(24))
        radius: root.ui.px(6)
        color: root.ui.c.panel
        border.width: Math.max(1, root.ui.px(1.5))
        border.color: root.ui.c.accentLo
        clip: true

        CardGuard {}   // taps on the card don't close it

        // 0.59: scrolls by touch and follows the focus when the rows don't fit (150%, a long list).
        Flickable {
            id: scroller
            anchors.fill: parent
            anchors.margins: root.ui.px(20)
            contentHeight: content.implicitHeight
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            function follow(it) {
                if (!it) return
                const y = it.mapToItem(content, 0, 0).y
                if (y < contentY) contentY = Math.max(0, y)
                else if (y + it.height > contentY + height) contentY = Math.min(Math.max(0, contentHeight - height), y + it.height - height)
            }
        Column {
            id: content
            width: scroller.width
            spacing: root.ui.px(8)

            Text {
                width: parent.width
                text: root.title || ""
                elide: Text.ElideRight
                color: root.ui.c.text
                font.family: root.ui.titleFont
                font.pixelSize: root.ui.fpx(18)
                font.bold: true
            }
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

            // 0.2: nothing is downloaded, so say so plainly.
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
                            color: root.readable(root.ui.c.accentHi, root.ui.c.raised)
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
                        wrapMode: Text.WrapAnywhere
                        color: root.readable(root.ui.c.accentHi, root.ui.c.card)
                        font.family: root.ui.bodyFont
                        font.pixelSize: root.ui.fpx(17)
                        font.weight: Font.DemiBold
                    }
                }
            }

            SheetButtons {
                id: loaderButtons
                ui: root.ui
                width: parent.width
                topPadding: root.ui.px(6)
                labels: (root.shownButtons || []).slice(0, 3).map(b => b.label || "")
                focusIndex: root.focusIndex
                mainIndex: 0
                onActivated: (i) => root.activated(i, "")
            }
        }
        }
    }

    // A button: brass when focused, plain otherwise, with its controller key in front.
    component ActionButton: Rectangle {
        id: btn
        required property var ui
        property string k: ""
        property string label: ""
        property bool focused: false
        signal clicked()
        width: btnRow.implicitWidth + ui.px(36)
        height: ui.px(44)
        radius: ui.px(6)
        color: ui.c.raised
        border.width: Math.max(1, ui.px(1.5))
        border.color: focused ? ui.c.accentHi : ui.c.line

        Rectangle {
            visible: btn.focused
            anchors.fill: parent
            anchors.margins: -btn.ui.px(3)
            radius: btn.radius + btn.ui.px(3)
            color: "transparent"
            border.width: btn.ui.px(3)
            border.color: Qt.rgba(btn.ui.c.accent.r, btn.ui.c.accent.g, btn.ui.c.accent.b, 0.22)
        }
        Rectangle {
            visible: btn.focused
            anchors.fill: parent
            radius: btn.radius
            border.width: Math.max(1, btn.ui.px(1.5))
            border.color: btn.ui.c.accentHi
            gradient: Gradient {
                GradientStop { position: 0; color: btn.ui.c.accentHi }
                GradientStop { position: 0.5; color: btn.ui.c.accent }
                GradientStop { position: 1; color: btn.ui.c.accentLo }
            }
        }
        HoverHandler { onHoveredChanged: btn.ui.hover(parent, hovered) }
        TapHandler { onTapped: btn.clicked() }

        Row {
            id: btnRow
            anchors.centerIn: parent
            spacing: btn.ui.px(6)
            Rectangle {
                visible: btn.k !== ""
                width: Math.max(btn.ui.px(24), btnKeyText.implicitWidth + btn.ui.px(12))
                height: btn.ui.px(24)
                radius: btn.k.length > 1 ? btn.ui.px(6) : height / 2
                anchors.verticalCenter: parent.verticalCenter
                color: btn.focused ? Qt.rgba(0, 0, 0, 0.2) : btn.ui.c.ground
                border.width: btn.focused ? 0 : Math.max(1, btn.ui.px(1))
                border.color: btn.ui.c.accentLo
                Text {
                    id: btnKeyText
                    anchors.centerIn: parent
                    text: btn.k.toUpperCase()
                    color: btn.focused ? btn.ui.c.onAccent : btn.ui.c.text
                    font.family: btn.ui.bodyFont
                    font.pixelSize: btn.ui.fpx(14)
                    font.bold: true
                }
            }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: btn.label
                color: btn.focused ? btn.ui.c.onAccent : btn.ui.c.text
                font.family: btn.ui.bodyFont
                font.pixelSize: btn.ui.fpx(16)
                font.weight: Font.DemiBold
            }
        }
    }
}
