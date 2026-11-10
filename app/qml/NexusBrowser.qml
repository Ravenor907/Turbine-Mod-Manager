import QtQuick
import QtWebEngine
import Turbine
import TurbineWeb

// Nexus inside Turbine: one web page, full screen in Turbine's window, over a slim brass bar at the bottom. The page is the
// player's: Turbine never clicks, fills in, hides or changes anything on it. The only clicks come from his own
// presses (A, RT, R3 at the pointer: backend.rs and web.rs), and the D-pad's arrows go to the page's own focus ring.
// One read-only script reads, a few times a second, whether a text field has the focus (for Steam's keyboard and the
// Y hint); it changes nothing.
//
// Main.qml owns everything else: the downloads (NexusProfile's downloadRequested), the pill's words, the legend,
// and what each button does (binds). This file draws the bar, the Page and Downloads tabs and the page, and reports
// what the page does.
Item {
    id: nb
    required property var ui
    // Make the page (false in the build's pictures that have no stand-in page: they show the frame only).
    property bool live: true
    property string startUrl: ""
    property string gameName: ""
    property string gameIcon: ""
    property real zoom: 1.25
    property int tab: 0                   // 0 Page, 1 Downloads
    property var downloads: []            // [{name, game, state, got, total, speed, words}]
    property int dlIndex: 0
    property string pillText: ""
    property string pillLevel: "ok"       // ok, busy, hint, warn
    property real pillProgress: -1
    property var legend: []               // [[key, label, pre, key2, label2]]
    property var glyph: function (k) { return k.toUpperCase() }
    // "Google sign-in" (through the player's own browser and the helper): only with a paired helper and while the
    // page isn't signed in (Main.qml decides).
    property bool signinShown: false
    property bool padLegend: true
    // A message cut short in the pill shows whole under the bar for its first 6 seconds only.
    property bool noteFresh: true
    // A sheet of Turbine's is over the page: the page takes no input and can't take the focus.
    property bool inputBlocked: false
    // Google's sign-in page: Turbine's card with the two ways that work (Main.qml has the words and the buttons).
    property bool googleShown: false
    property string googlePasswordWords: ""
    property string googleChromeWords: ""
    // What the page is doing, for Main.qml.
    readonly property var view: loader.item
    readonly property string url: view ? view.url.toString() : startUrl
    readonly property string title: view && view.title ? view.title : "Nexus Mods"
    readonly property bool loading: view ? view.loading : false
    readonly property bool canGoBack: view ? view.canGoBack : false
    readonly property bool canGoForward: view ? view.canGoForward : false
    property int focusKind: 0             // 0 nothing, 1 a link or button, 2 a text field
    readonly property bool signInPage: /^https:\/\/users\.nexusmods\.com\/(auth|login|sign)/i.test(url) && !/password/i.test(url)

    signal signinClicked()
    signal googlePassword()
    signal pagePressed()
    signal pageDown()
    signal googleChrome()
    signal closeClicked()
    signal menuWanted()
    signal tabClicked(int index)
    signal downloadClicked(int index)
    signal pageEvent(string what, string detail)

    function back() { if (view && view.canGoBack) view.goBack() }
    function forward() { if (view && view.canGoForward) view.goForward() }
    function reload() { if (view) view.reload() }
    function load(u) { if (view) view.url = u; else startUrl = u }
    // A fresh view at this address (the profile, and with it the sign-in, stays): no history, so B closes on it.
    function restart(u) {
        startUrl = u
        if (!nb.live) return
        loader.active = false
        loader.active = Qt.binding(() => nb.live)
    }
    function focusPage() { if (view) view.forceActiveFocus() }
    // Where a point in the page (CSS pixels) is in the window: for the build's scripted presses only.
    // The bar's touch targets and their sizes in design pixels ([name, "WxH"]), for the build's check (44 at least).
    function tapSizes() {
        const out = []
        const walk = (it) => {
            if (!it || !it.visible) return
            if (typeof it.objectName === "string" && it.objectName.indexOf("tap:") === 0)
                out.push([it.objectName.substring(4), Math.round(it.width / nb.ui.px(1)) + "x" + Math.round(it.height / nb.ui.px(1))])
            for (let i = 0; i < it.children.length; i++) walk(it.children[i])
        }
        walk(bar)
        return out
    }
    // 0.64: the file page's Slow download button brought into view (scrolled to the middle, never pressed). `done`
    // gets its middle in window pixels, or null when the page has none.
    function lineUpSlow(done) {
        if (!view) { done(null); return }
        view.runJavaScript(
            "(function(){var l=document.querySelectorAll('button,a');for(var i=0;i<l.length;i++){var e=l[i];" +
            "if(/^\\s*slow download\\s*$/i.test(e.textContent||'')&&e.offsetParent!==null){e.scrollIntoView({block:'center'});" +
            "var r=e.getBoundingClientRect();return [r.left+r.width/2,r.top+r.height/2]}}return null})()",
            WebEngineScript.ApplicationWorld,
            function (r) { done(r && r.length === 2 ? pagePoint(r[0], r[1]) : null) })
    }
    function pagePoint(cx, cy) { return view ? view.mapToItem(null, cx * view.zoomFactor, cy * view.zoomFactor) : Qt.point(-1, -1) }

    Rectangle { anchors.fill: parent; color: nb.ui.c.ground }
    // Nothing under the page reacts to the pointer, a tap or the wheel while it is on screen.
    MouseArea { anchors.fill: parent; hoverEnabled: true; acceptedButtons: Qt.AllButtons; onWheel: (w) => { w.accepted = true } }  // overlay-check: the page itself (layer 0), not an overlay

    // ---------------------------------------------------------------- the bar (40 design pixels)
    // 0.6 (intent I20): at the bottom, like every other screen's bar (no top bar anywhere); the page fills the rest.
    Rectangle {
        id: bar
        anchors.left: parent.left; anchors.right: parent.right; anchors.bottom: parent.bottom
        height: nb.ui.px(40)
        color: nb.ui.c.panel
        z: 2
        Rectangle { anchors.left: parent.left; anchors.right: parent.right; anchors.top: parent.top; height: Math.max(1, nb.ui.px(1)); color: nb.ui.c.accentLo }
        // Room for "Google sign-in": 2 with the game's name, 1 the button alone (the game's icon only), 0 none. The
        // pill gives way first (pillRoom keeps the button's room), so it hides only on the narrowest bars.
        TextMetrics { id: signinShort; font: signinText.font; text: "Google sign-in" }
        readonly property real signinNeed: nb.signinShown ? signinShort.advanceWidth + nb.ui.px(16) + nb.ui.px(8) : 0
        // On another site than Nexus its host always shows (a warning chip): the room after the tabs keeps it.
        readonly property real tabsEnd: nb.ui.px(10) + nb.ui.px(26) + nb.ui.px(8) + nb.ui.px(12) + tabs.width + nb.ui.px(12)
                                        + (hostChip.host !== "" && !hostChip.nexus ? hostChip.width + nb.ui.px(10) : 0)
        readonly property real signinRoom: pill.x - nb.ui.px(8) - tabsEnd
        // The pill takes what it needs up to the room left of the legend, never over the tabs; its whole words show
        // over the bar when they don't fit (pillFull).
        readonly property real pillWant: pillMetrics.advanceWidth + nb.ui.px(24)
        // The page's title keeps a little room too (a long hint in the pill used to take it all).
        readonly property real titleMin: nb.width * 0.09
        readonly property real pillRoom: legendRow.x - nb.ui.px(10) - tabsEnd - signinNeed - titleMin
        readonly property real nameW: Math.min(nameText.implicitWidth, nb.width * 0.16)
        readonly property int signinFit: {
            if (!nb.signinShown) return 0
            const pad = nb.ui.px(16)
            if (signinShort.advanceWidth + pad + nameW <= signinRoom) return 2
            if (signinShort.advanceWidth + pad <= signinRoom) return 1
            return 0
        }
        // Left: the game.
        Row {
            id: gameBox
            anchors.left: parent.left; anchors.leftMargin: nb.ui.px(10)
            anchors.verticalCenter: parent.verticalCenter
            spacing: nb.ui.px(8)
            Image {
                visible: status === Image.Ready
                source: nb.gameIcon
                width: nb.ui.px(26); height: nb.ui.px(26)
                fillMode: Image.PreserveAspectCrop
                anchors.verticalCenter: parent.verticalCenter
            }
            Text {
                id: nameText
                visible: bar.signinFit === 2 || (bar.signinFit === 0 && bar.tabsEnd + bar.nameW <= (signin.visible ? signin.x : pill.x) - nb.ui.px(10))
                text: nb.gameName !== "" ? nb.gameName : "Nexus Mods"
                color: nb.ui.c.accentText
                font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.fpx(15); font.weight: Font.DemiBold
                width: Math.min(implicitWidth, nb.width * 0.16); elide: Text.ElideRight
                anchors.verticalCenter: parent.verticalCenter
            }
        }
        // Page | Downloads (n): LB and RB caps on the ends.
        Row {
            id: tabs
            anchors.left: gameBox.right; anchors.leftMargin: nb.ui.px(12)
            anchors.verticalCenter: parent.verticalCenter
            spacing: nb.ui.px(4)
            Cap { k: "lb"; visible: nb.padLegend; anchors.verticalCenter: parent.verticalCenter }
            Repeater {
                model: ["Page", "Downloads" + (nb.downloads.length > 0 ? " (" + nb.downloads.length + ")" : "")]
                Rectangle {
                    id: tb
                    required property int index
                    required property string modelData
                    width: tbText.implicitWidth + nb.ui.px(16); height: nb.ui.px(28); radius: nb.ui.px(5)
                    color: nb.tab === index ? nb.ui.c.focusFill : "transparent"
                    border.color: nb.tab === index ? nb.ui.c.accent : "transparent"
                    border.width: Math.max(1, nb.ui.px(1))
                    Text { id: tbText; anchors.centerIn: parent; text: tb.modelData; color: nb.tab === tb.index ? nb.ui.c.accentText : nb.ui.c.soft; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.fpx(14) }
                    anchors.verticalCenter: parent.verticalCenter
                    HoverHandler { onHoveredChanged: nb.ui.hover(tb, hovered) }
                    // A touch target of at least 44 design pixels around the 28 drawn.
                    Item { objectName: "tap:" + tb.modelData; anchors.fill: parent; anchors.margins: -nb.ui.px(8); TapHandler { onTapped: nb.tabClicked(tb.index) } }
                }
            }
            Cap { k: "rb"; visible: nb.padLegend; anchors.verticalCenter: parent.verticalCenter }
        }
        // Middle: the page's title, and its site (another site than Nexus: in a warning chip).
        Row {
            id: titleRow
            anchors.left: tabs.right; anchors.leftMargin: nb.ui.px(12)
            anchors.right: signin.visible ? signin.left : pill.left; anchors.rightMargin: nb.ui.px(10)
            anchors.verticalCenter: parent.verticalCenter
            spacing: nb.ui.px(8)
            clip: true
            // Off Nexus the site's warning chip must show whole (else the row stays empty); on Nexus the chip goes
            // first and the page's title shows, cut short as needed.
            visible: hostChip.nexus ? width > nb.ui.px(40) : width >= hostChip.width
            Rectangle {
                id: hostChip
                readonly property bool nexus: Turbine.browserIsNexus(nb.url)
                readonly property string host: { const m = /^[a-z]+:\/\/([^\/:?#]+)/i.exec(nb.url); return m ? m[1].toLowerCase() : "" }
                readonly property bool fits: !nexus || titleRow.width >= width + nb.ui.px(8) + nb.ui.px(120)
                visible: host !== "" && fits
                width: hostText.implicitWidth + nb.ui.px(14); height: nb.ui.px(24); radius: nb.ui.px(12)
                color: nexus ? nb.ui.c.raised : nb.ui.c.focusFill
                border.color: nexus ? nb.ui.c.line : nb.ui.c.warn
                border.width: Math.max(1, nb.ui.px(1))
                anchors.verticalCenter: parent.verticalCenter
                Text { id: hostText; anchors.centerIn: parent; text: hostChip.nexus ? "nexusmods.com" : hostChip.host; color: hostChip.nexus ? nb.ui.c.soft : nb.ui.c.warn; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.fpx(13) }
            }
            Text {
                text: nb.loading ? "Loading..." : nb.title
                color: nb.ui.c.text
                font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.fpx(14)
                width: Math.max(0, titleRow.width - (hostChip.visible ? hostChip.width + nb.ui.px(8) : 0)); elide: Text.ElideRight
                anchors.verticalCenter: parent.verticalCenter
            }
        }
        // Sign in with my browser (Google and others): in the bar when there is room (signinFit).
        Rectangle {
            id: signin
            visible: bar.signinFit > 0
            anchors.right: pill.left; anchors.rightMargin: nb.ui.px(8)
            anchors.verticalCenter: parent.verticalCenter
            width: signinText.implicitWidth + nb.ui.px(16); height: nb.ui.px(28); radius: nb.ui.px(5)
            color: nb.ui.c.raised
            border.color: nb.ui.c.accentLo; border.width: Math.max(1, nb.ui.px(1))
            Text { id: signinText; anchors.centerIn: parent; text: "Google sign-in"; color: nb.ui.c.text; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.fpx(13) }
            HoverHandler { onHoveredChanged: nb.ui.hover(signin, hovered) }
            Item { objectName: "tap:Google sign-in"; anchors.fill: parent; anchors.margins: -nb.ui.px(8); TapHandler { onTapped: nb.signinClicked() } }
        }
        // The download pill.
        Rectangle {
            id: pill
            visible: nb.pillText !== ""
            anchors.right: legendRow.left; anchors.rightMargin: nb.ui.px(10)
            anchors.verticalCenter: parent.verticalCenter
            width: visible ? Math.max(0, Math.min(bar.pillWant, bar.pillRoom)) : 0
            TextMetrics { id: pillMetrics; font: pillText.font; text: nb.pillText }
            height: nb.ui.px(28); radius: height / 2
            color: nb.pillLevel === "warn" ? nb.ui.c.focusFill : nb.ui.c.card
            border.color: nb.pillLevel === "ok" ? nb.ui.c.ok : nb.pillLevel === "warn" ? nb.ui.c.warn : nb.ui.c.accent
            border.width: Math.max(1, nb.ui.px(1.5))
            clip: true
            Rectangle {
                visible: nb.pillProgress >= 0
                anchors.left: parent.left; anchors.top: parent.top; anchors.bottom: parent.bottom
                width: parent.width * Math.max(0, Math.min(1, nb.pillProgress))
                radius: parent.radius
                color: Qt.rgba(nb.ui.c.accent.r, nb.ui.c.accent.g, nb.ui.c.accent.b, 0.22)
            }
            Text {
                id: pillText
                anchors.verticalCenter: parent.verticalCenter
                x: nb.ui.px(12); width: parent.width - nb.ui.px(24)
                text: nb.pillText
                color: nb.pillLevel === "warn" ? nb.ui.c.warn : nb.ui.c.text
                font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.fpx(13)
                elide: Text.ElideRight
            }
        }
        // The legend lives in the bar (the page needs the height).
        Row {
            id: legendRow
            anchors.right: closeBtn.left; anchors.rightMargin: nb.ui.px(8)
            anchors.verticalCenter: parent.verticalCenter
            spacing: nb.ui.px(10)
            Repeater {
                model: nb.legend
                Row {
                    id: lh
                    required property var modelData
                    spacing: nb.ui.px(4)
                    Text { visible: (lh.modelData[2] || "") !== ""; text: lh.modelData[2] || ""; color: nb.ui.c.soft; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.fpx(13); anchors.verticalCenter: parent.verticalCenter }
                    Cap { k: lh.modelData[0]; anchors.verticalCenter: parent.verticalCenter }
                    Text { text: lh.modelData[1] + ((lh.modelData[3] || "") !== "" ? "," : ""); color: nb.ui.c.soft; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.fpx(13); anchors.verticalCenter: parent.verticalCenter }
                    Cap { visible: (lh.modelData[3] || "") !== ""; k: lh.modelData[3] || ""; anchors.verticalCenter: parent.verticalCenter }
                    Text { visible: (lh.modelData[4] || "") !== ""; text: lh.modelData[4] || ""; color: nb.ui.c.soft; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.fpx(13); anchors.verticalCenter: parent.verticalCenter }
                }
            }
        }
        // Close: B on the first page does the same.
        Rectangle {
            id: closeBtn
            anchors.right: parent.right; anchors.rightMargin: nb.ui.px(6)
            anchors.verticalCenter: parent.verticalCenter
            width: nb.ui.px(32); height: nb.ui.px(28); radius: nb.ui.px(5)
            color: nb.ui.c.raised
            Text { anchors.centerIn: parent; text: "×"; color: nb.ui.c.text; font.pixelSize: nb.ui.fpx(18); font.family: nb.ui.bodyFont }
            HoverHandler { onHoveredChanged: nb.ui.hover(closeBtn, hovered) }
            Item { objectName: "tap:Close"; anchors.fill: parent; anchors.margins: -nb.ui.px(8); TapHandler { onTapped: nb.closeClicked() } }
        }
    }

    // The pill's whole words under the bar when they are cut short there (a message to read: done, a warning or what
    // to press), for their first 6 seconds; then only the pill keeps them, so nothing stays over the page's corner.
    property bool fullFresh: false
    onPillTextChanged: { fullFresh = pillText !== ""; fullTimer.restart() }
    Timer { id: fullTimer; interval: 6000; onTriggered: nb.fullFresh = false }
    Rectangle {
        id: pillFull
        visible: pill.visible && nb.noteFresh && nb.fullFresh && bar.pillWant > pill.width + 1 && (nb.pillLevel === "warn" || nb.pillLevel === "ok" || nb.pillLevel === "hint")
        anchors.bottom: bar.top; anchors.bottomMargin: nb.ui.px(4)
        anchors.right: parent.right; anchors.rightMargin: nb.ui.px(10)
        width: Math.min(bar.pillWant, nb.width - nb.ui.px(20)); height: fullText.implicitHeight + nb.ui.px(12)
        radius: nb.ui.px(6)
        color: nb.ui.c.panel
        border.color: nb.pillLevel === "warn" ? nb.ui.c.warn : nb.pillLevel === "hint" ? nb.ui.c.accent : nb.ui.c.ok
        border.width: Math.max(1, nb.ui.px(1.5))
        z: 3
        Text {
            id: fullText
            x: nb.ui.px(12); y: nb.ui.px(6); width: parent.width - nb.ui.px(24)
            text: nb.pillText
            wrapMode: Text.WordWrap
            color: nb.pillLevel === "warn" ? nb.ui.c.warn : nb.ui.c.text
            font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.fpx(13)
        }
    }

    component Cap: Rectangle {
        property string k: ""
        implicitWidth: Math.max(nb.ui.px(22), capText.implicitWidth + nb.ui.px(10))
        implicitHeight: nb.ui.px(22)
        radius: k.length > 1 ? nb.ui.px(5) : height / 2
        color: nb.ui.c.raised
        border.color: nb.ui.c.accentLo; border.width: Math.max(1, nb.ui.px(1))
        Text { id: capText; anchors.centerIn: parent; text: nb.glyph(parent.k); color: nb.ui.c.text; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.fpx(12); font.bold: true }
    }

    // ---------------------------------------------------------------- the page
    Item {
        id: pageArea
        anchors.top: parent.top; anchors.left: parent.left; anchors.right: parent.right; anchors.bottom: bar.top
        visible: nb.tab === 0
        Loader {
            id: loader
            anchors.fill: parent
            active: nb.live
            sourceComponent: pageComponent
            enabled: !nb.inputBlocked
        }
        // The build's pictures without a stand-in page: the frame only.
        Text {
            visible: !nb.live
            anchors.centerIn: parent
            width: parent.width * 0.7
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
            text: "The Nexus page shows here.\n" + nb.startUrl
            color: nb.ui.c.muted
            font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.fpx(16)
        }
        // On Nexus's sign-in page: Turbine's own button (drawn by Turbine over its corner, not put into the page).
        Rectangle {
            id: signinBig
            visible: nb.signInPage
            anchors.right: parent.right; anchors.bottom: parent.bottom; anchors.margins: nb.ui.px(16)
            width: bigCol.implicitWidth + nb.ui.px(28); height: bigCol.implicitHeight + nb.ui.px(20)
            radius: nb.ui.px(8)
            color: nb.ui.c.panel
            border.color: nb.ui.c.accent; border.width: Math.max(2, nb.ui.px(2))
            Column {
                id: bigCol
                anchors.centerIn: parent
                spacing: nb.ui.px(4)
                Text { visible: nb.signinShown; text: "Google sign-in (in Chrome)"; color: nb.ui.c.accentText; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.fpx(16); font.weight: Font.DemiBold }
                Text { text: nb.signinShown ? "Email and password work right here too." : "Email and password work here."; color: nb.signinShown ? nb.ui.c.soft : nb.ui.c.text; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.fpx(nb.signinShown ? 13 : 15) }
            }
            HoverHandler { enabled: nb.signinShown; onHoveredChanged: nb.ui.hover(signinBig, hovered) }
            TapHandler { enabled: nb.signinShown; onTapped: nb.signinClicked() }
        }
        // Google's own sign-in page: Google refuses to sign in inside apps, so Turbine says so and offers the two
        // ways that work, each with what it needs (drawn by Turbine over the page, never put into it).
        Rectangle {
            id: googleCard
            visible: nb.googleShown
            anchors.centerIn: parent
            width: Math.min(parent.width - nb.ui.px(40), nb.ui.px(620))
            height: googleCol.implicitHeight + nb.ui.px(32)
            radius: nb.ui.px(10)
            color: nb.ui.c.panel
            border.color: nb.ui.c.accent; border.width: Math.max(2, nb.ui.px(2))
            z: 4
            Column {
                id: googleCol
                x: nb.ui.px(18); y: nb.ui.px(16); width: parent.width - nb.ui.px(36)
                spacing: nb.ui.px(10)
                Text { width: parent.width; wrapMode: Text.WordWrap; text: "Google won't sign in inside apps"; color: nb.ui.c.accentText; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.fpx(19); font.weight: Font.DemiBold }
                Repeater {
                    model: [["a", "Set a Nexus password (stays here)", nb.googlePasswordWords, 0], ["y", "Use Chrome instead", nb.googleChromeWords, 1]]
                    Rectangle {
                        id: gRow
                        required property var modelData
                        width: googleCol.width; height: Math.max(nb.ui.px(56), gText.implicitHeight + nb.ui.px(16)); radius: nb.ui.px(8)
                        color: nb.ui.c.card; border.color: nb.ui.c.line; border.width: Math.max(1, nb.ui.px(1))
                        Cap { id: gCap; k: gRow.modelData[0]; visible: nb.padLegend; x: nb.ui.px(12); anchors.verticalCenter: parent.verticalCenter }
                        Column {
                            id: gText
                            anchors.left: gCap.visible ? gCap.right : parent.left; anchors.leftMargin: nb.ui.px(12)
                            anchors.right: parent.right; anchors.rightMargin: nb.ui.px(12)
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: nb.ui.px(2)
                            Text { width: parent.width; wrapMode: Text.WordWrap; text: gRow.modelData[1]; color: nb.ui.c.text; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.fpx(16); font.weight: Font.DemiBold }
                            Text { width: parent.width; wrapMode: Text.WordWrap; text: gRow.modelData[2]; color: nb.ui.c.soft; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.fpx(13) }
                        }
                        HoverHandler { onHoveredChanged: nb.ui.hover(gRow, hovered) }
                        TapHandler { onTapped: gRow.modelData[3] === 0 ? nb.googlePassword() : nb.googleChrome() }
                    }
                }
            }
        }
        // A tap or a click on the page counts as the player's press (a download or a Mod Manager Download link in the
        // next moments is his); a scroll or a drag doesn't. Only watched: the page still gets it.
        Item {
            anchors.fill: parent
            z: 3
            // A PointHandler only watches (a TapHandler here kept the page from getting its clicks): a press let go
            // within a second, moved less than a finger's width, is a tap or a click. It counts from where it went
            // down (Rust drops it if the page changed while it was held).
            PointHandler {
                acceptedButtons: Qt.LeftButton
                enabled: !nb.inputBlocked
                property real downAt: 0
                property real moved: 0
                onTranslationChanged: moved = Math.max(moved, Math.sqrt(translation.x * translation.x + translation.y * translation.y))
                onActiveChanged: {
                    if (active) { downAt = Date.now(); moved = 0; nb.pageDown() }
                    else if (moved < nb.ui.px(12) && Date.now() - downAt < 1000) nb.pagePressed()
                }
            }
        }
    }

    Component {
        id: pageComponent
        WebEngineView {
            id: web
            profile: NexusProfile
            url: nb.startUrl
            zoomFactor: nb.zoom
            focus: true
            settings.focusOnNavigationEnabled: true
            settings.javascriptCanOpenWindows: true
            settings.pluginsEnabled: false
            settings.playbackRequiresUserGesture: true
            settings.localContentCanAccessRemoteUrls: false
            settings.localContentCanAccessFileUrls: false
            onZoomFactorChanged: if (Math.abs(zoomFactor - nb.zoom) > 0.001) zoomFactor = nb.zoom
            // The main frame's address: off Nexus, the player's last press no longer counts, and a press made there
            // never lets a download or a Mod Manager Download link through without a question (Rust).
            onUrlChanged: { Turbine.browserMainFrame(url.toString()); nb.pageEvent("url", url.toString()) }
            onLoadingChanged: (info) => nb.pageEvent(info.status === WebEngineView.LoadSucceededStatus ? "loaded" : info.status === WebEngineView.LoadFailedStatus ? "failed" : "loading", info.url.toString() + (info.errorString ? " (" + info.errorString + ")" : ""))
            // Chromium's own right-click menu never shows: LT (a right click) opens Turbine's quick menu instead.
            onContextMenuRequested: (request) => { request.accepted = true; nb.menuWanted() }
            // Pop-ups open here, in the same view, and only when the player's own press asked for one.
            onNewWindowRequested: (request) => {
                if (!request.userInitiated) { nb.pageEvent("popupblocked", request.requestedUrl.toString()); return }
                nb.pageEvent("newwindow", request.requestedUrl.toString())
                request.openIn(web)
            }
            // Only web pages. A Mod Manager Download link goes on (to webglue.cpp's scheme handler, which checks who
            // started it) only when the page itself goes there while it is on Nexus: never from a frame inside it.
            // Every other kind of link is refused.
            onNavigationRequested: (request) => {
                const u = request.url.toString()
                if (/^(https?|about|data|blob):/i.test(u)) return
                if (/^nxm:/i.test(u)) {
                    if (Turbine.browserNxm(u, request.isMainFrame, web.url.toString())) return
                    request.reject()
                    return
                }
                request.reject()
                nb.pageEvent("blocked", u.split(":")[0])
            }
            onCertificateError: (error) => { error.rejectCertificate(); nb.pageEvent("certificate", error.url.toString()) }
            onFileDialogRequested: (request) => { request.accepted = true; request.dialogReject(); nb.pageEvent("upload", "") }
            onJavaScriptDialogRequested: (request) => { request.accepted = true; nb.pageEvent("dialog", ""); nb.dialog(request) }
            onAuthenticationDialogRequested: (request) => { request.accepted = true; request.dialogReject() }
            onColorDialogRequested: (request) => { request.accepted = true; request.dialogReject() }
            onFullScreenRequested: (request) => request.reject()
            onPermissionRequested: (permission) => permission.deny()
            // The page's own console stays out of Turbine's log.
            onJavaScriptConsoleMessage: (level, message, line, source) => {}
            onRenderProcessTerminated: (status, code) => nb.pageEvent("crashed", String(code))
        }
    }

    // A JavaScript alert or question from the page: Main.qml shows it in Turbine's own sheet.
    signal dialogWanted(var request)
    function dialog(request) { dialogWanted(request) }

    // The read-only look at what has the page's focus: 0 nothing, 1 a link or button, 2 a text field. It runs in
    // its own world (the page's scripts can't see or change it) and only reads.
    Timer {
        interval: 300
        repeat: true
        running: nb.visible && nb.tab === 0 && !!nb.view && !nb.loading
        onTriggered: nb.view.runJavaScript(
            "(function(){var a=document.activeElement;if(!a||a===document.body||a===document.documentElement)return 0;" +
            "var t=(a.tagName||'').toLowerCase();var ty=(a.type||'').toLowerCase();" +
            "if(a.isContentEditable||t==='textarea'||(t==='input'&&!/^(button|submit|checkbox|radio|range|color|file|image|reset|hidden)$/.test(ty)))return 2;return 1})()",
            WebEngineScript.ApplicationWorld,
            function (r) { const k = r === 2 ? 2 : r === 1 ? 1 : 0; if (k !== nb.focusKind) nb.focusKind = k })
    }
    onLoadingChanged: if (loading) focusKind = 0

    // ---------------------------------------------------------------- the Downloads tab
    Item {
        anchors.top: parent.top; anchors.left: parent.left; anchors.right: parent.right; anchors.bottom: bar.top
        anchors.margins: nb.ui.px(14)
        visible: nb.tab === 1
        Text {
            visible: nb.downloads.length === 0
            anchors.centerIn: parent
            text: "Nothing downloaded yet. Press Slow download on a mod's file page."
            color: nb.ui.c.muted; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.fpx(16)
        }
        ListView {
            id: dlList
            anchors.fill: parent
            clip: true
            spacing: nb.ui.px(8)
            model: nb.downloads
            currentIndex: nb.dlIndex
            delegate: Rectangle {
                id: row
                required property int index
                required property var modelData
                readonly property bool foc: index === nb.dlIndex
                width: dlList.width; height: nb.ui.px(66); radius: nb.ui.px(8)
                color: foc ? nb.ui.c.focusFill : nb.ui.c.card
                border.color: foc ? nb.ui.c.accentHi : nb.ui.c.line
                border.width: foc ? Math.max(2, nb.ui.px(2.5)) : Math.max(1, nb.ui.px(1))
                Column {
                    anchors.left: parent.left; anchors.right: parent.right; anchors.verticalCenter: parent.verticalCenter
                    anchors.leftMargin: nb.ui.px(14); anchors.rightMargin: nb.ui.px(14)
                    spacing: nb.ui.px(6)
                    Row {
                        spacing: nb.ui.px(10)
                        Text { text: row.modelData.name; color: nb.ui.c.text; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.fpx(16); font.weight: Font.DemiBold; elide: Text.ElideRight; width: Math.min(implicitWidth, dlList.width * 0.5) }
                        Text { text: row.modelData.game || "Other"; color: nb.ui.c.muted; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.fpx(14) }
                    }
                    Rectangle {
                        width: parent.width; height: nb.ui.px(6); radius: height / 2
                        color: nb.ui.c.raised
                        Rectangle {
                            height: parent.height; radius: parent.radius
                            width: parent.width * (row.modelData.total > 0 ? Math.min(1, row.modelData.got / row.modelData.total) : row.modelData.state === "done" || row.modelData.state === "added" ? 1 : 0)
                            color: row.modelData.state === "interrupted" || row.modelData.state === "cancelled" ? nb.ui.c.warn : nb.ui.c.accent
                        }
                    }
                    Text { text: row.modelData.words; color: nb.ui.c.soft; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.fpx(13) }
                }
                HoverHandler { onHoveredChanged: nb.ui.hover(row, hovered) }
                TapHandler { onTapped: nb.downloadClicked(row.index) }
            }
        }
    }
}
