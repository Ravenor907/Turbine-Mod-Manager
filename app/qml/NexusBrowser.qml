import QtQuick
import QtWebEngine
import Turbine
import TurbineWeb

// Nexus inside Turbine: one web page, full screen in Turbine's window, under a slim brass bar. The page is the
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
    property bool signinShown: true
    property bool padLegend: true
    // What the page is doing, for Main.qml.
    readonly property var view: loader.item
    readonly property string url: view ? view.url.toString() : startUrl
    readonly property string title: view && view.title ? view.title : "Nexus Mods"
    readonly property bool loading: view ? view.loading : false
    readonly property bool canGoBack: view ? view.canGoBack : false
    readonly property bool canGoForward: view ? view.canGoForward : false
    property int focusKind: 0             // 0 nothing, 1 a link or button, 2 a text field
    readonly property bool signInPage: /^https:\/\/users\.nexusmods\.com\/(auth|login|sign)/i.test(url)

    signal signinClicked()
    signal closeClicked()
    signal menuWanted()
    signal tabClicked(int index)
    signal downloadClicked(int index)
    signal pageEvent(string what, string detail)

    function back() { if (view && view.canGoBack) view.goBack() }
    function forward() { if (view && view.canGoForward) view.goForward() }
    function reload() { if (view) view.reload() }
    function load(u) { if (view) view.url = u; else startUrl = u }
    function focusPage() { if (view) view.forceActiveFocus() }
    // Where a point in the page (CSS pixels) is in the window: for the build's scripted presses only.
    function pagePoint(cx, cy) { return view ? view.mapToItem(null, cx * view.zoomFactor, cy * view.zoomFactor) : Qt.point(-1, -1) }

    Rectangle { anchors.fill: parent; color: nb.ui.c.ground }
    // Nothing under the page reacts to the pointer, a tap or the wheel while it is on screen.
    MouseArea { anchors.fill: parent; hoverEnabled: true; acceptedButtons: Qt.AllButtons; onWheel: (w) => { w.accepted = true } }

    // ---------------------------------------------------------------- the bar (40 design pixels)
    Rectangle {
        id: bar
        anchors.left: parent.left; anchors.right: parent.right; anchors.top: parent.top
        height: nb.ui.px(40)
        color: nb.ui.c.panel
        z: 2
        Rectangle { anchors.left: parent.left; anchors.right: parent.right; anchors.bottom: parent.bottom; height: Math.max(1, nb.ui.px(1)); color: nb.ui.c.accentLo }
        // Room for Sign in with my browser: 4 the whole words and the game's name, 3 the whole words (the game's
        // icon only), 2 "My browser sign-in", 1 "Sign in", 0 none (the X menu, Settings and Nexus's own sign-in page
        // still have it). Worked out from sizes that never depend on it (the pill and the legend sit at the right).
        TextMetrics { id: signinFull; font: signinText.font; text: "Sign in with my browser" }
        TextMetrics { id: signinMid; font: signinText.font; text: "My browser sign-in" }
        TextMetrics { id: signinShort; font: signinText.font; text: "Sign in" }
        readonly property real tabsEnd: nb.ui.px(10) + nb.ui.px(26) + nb.ui.px(8) + nb.ui.px(12) + tabs.width + nb.ui.px(12)
        readonly property real signinRoom: pill.x - nb.ui.px(8) - tabsEnd
        // The pill takes what it needs up to the room left of the legend, never over the tabs; its whole words show
        // under the bar when they don't fit (pillFull).
        readonly property real pillWant: pillMetrics.advanceWidth + nb.ui.px(24)
        readonly property real pillRoom: legendRow.x - nb.ui.px(10) - tabsEnd
        readonly property real nameW: Math.min(nameText.implicitWidth, nb.width * 0.16)
        readonly property int signinFit: {
            if (!nb.signinShown) return 0
            const pad = nb.ui.px(16)
            if (signinFull.advanceWidth + pad + nameW <= signinRoom) return 4
            if (signinFull.advanceWidth + pad <= signinRoom) return 3
            if (signinMid.advanceWidth + pad <= signinRoom) return 2
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
                visible: bar.signinFit === 4 || (bar.signinFit === 0 && bar.tabsEnd + bar.nameW <= pill.x - nb.ui.px(10))
                text: nb.gameName !== "" ? nb.gameName : "Nexus Mods"
                color: nb.ui.c.accentText
                font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.px(15); font.weight: Font.DemiBold
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
            Cap { k: "lb"; visible: nb.padLegend }
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
                    Text { id: tbText; anchors.centerIn: parent; text: tb.modelData; color: nb.tab === tb.index ? nb.ui.c.accentText : nb.ui.c.soft; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.px(14) }
                    HoverHandler { onHoveredChanged: nb.ui.hover(tb, hovered) }
                    TapHandler { onTapped: nb.tabClicked(tb.index) }
                }
            }
            Cap { k: "rb"; visible: nb.padLegend }
        }
        // Middle: the page's title, and its site (another site than Nexus: in a warning chip).
        Row {
            id: titleRow
            anchors.left: tabs.right; anchors.leftMargin: nb.ui.px(12)
            anchors.right: signin.visible ? signin.left : pill.left; anchors.rightMargin: nb.ui.px(10)
            anchors.verticalCenter: parent.verticalCenter
            spacing: nb.ui.px(8)
            clip: true
            Rectangle {
                id: hostChip
                readonly property bool nexus: Turbine.browserIsNexus(nb.url)
                readonly property string host: { const m = /^[a-z]+:\/\/([^\/:?#]+)/i.exec(nb.url); return m ? m[1].toLowerCase() : "" }
                visible: host !== ""
                width: hostText.implicitWidth + nb.ui.px(14); height: nb.ui.px(24); radius: nb.ui.px(12)
                color: nexus ? nb.ui.c.raised : nb.ui.c.focusFill
                border.color: nexus ? nb.ui.c.line : nb.ui.c.warn
                border.width: Math.max(1, nb.ui.px(1))
                anchors.verticalCenter: parent.verticalCenter
                Text { id: hostText; anchors.centerIn: parent; text: hostChip.nexus ? "nexusmods.com" : hostChip.host; color: hostChip.nexus ? nb.ui.c.soft : nb.ui.c.warn; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.px(13) }
            }
            Text {
                text: nb.loading ? "Loading..." : nb.title
                color: nb.ui.c.text
                font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.px(14)
                width: Math.max(0, titleRow.width - hostChip.width - nb.ui.px(8)); elide: Text.ElideRight
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
            Text { id: signinText; anchors.centerIn: parent; text: bar.signinFit >= 3 ? "Sign in with my browser" : bar.signinFit === 2 ? "My browser sign-in" : "Sign in"; color: nb.ui.c.text; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.px(13) }
            HoverHandler { onHoveredChanged: nb.ui.hover(signin, hovered) }
            TapHandler { onTapped: nb.signinClicked() }
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
                font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.px(13)
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
                    Text { visible: (lh.modelData[2] || "") !== ""; text: lh.modelData[2] || ""; color: nb.ui.c.soft; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.px(13); anchors.verticalCenter: parent.verticalCenter }
                    Cap { k: lh.modelData[0]; anchors.verticalCenter: parent.verticalCenter }
                    Text { text: lh.modelData[1] + ((lh.modelData[3] || "") !== "" ? "," : ""); color: nb.ui.c.soft; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.px(13); anchors.verticalCenter: parent.verticalCenter }
                    Cap { visible: (lh.modelData[3] || "") !== ""; k: lh.modelData[3] || ""; anchors.verticalCenter: parent.verticalCenter }
                    Text { visible: (lh.modelData[4] || "") !== ""; text: lh.modelData[4] || ""; color: nb.ui.c.soft; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.px(13); anchors.verticalCenter: parent.verticalCenter }
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
            Text { anchors.centerIn: parent; text: "×"; color: nb.ui.c.text; font.pixelSize: nb.ui.px(18); font.family: nb.ui.bodyFont }
            HoverHandler { onHoveredChanged: nb.ui.hover(closeBtn, hovered) }
            TapHandler { onTapped: nb.closeClicked() }
        }
    }

    // The pill's whole words under the bar when they are cut short there (a message to read: done or a warning).
    Rectangle {
        id: pillFull
        visible: pill.visible && bar.pillWant > pill.width + 1 && (nb.pillLevel === "warn" || nb.pillLevel === "ok")
        anchors.top: bar.bottom; anchors.topMargin: nb.ui.px(4)
        anchors.right: parent.right; anchors.rightMargin: nb.ui.px(10)
        width: Math.min(bar.pillWant, nb.width - nb.ui.px(20)); height: fullText.implicitHeight + nb.ui.px(12)
        radius: nb.ui.px(6)
        color: nb.ui.c.panel
        border.color: nb.pillLevel === "warn" ? nb.ui.c.warn : nb.ui.c.ok
        border.width: Math.max(1, nb.ui.px(1.5))
        z: 3
        Text {
            id: fullText
            x: nb.ui.px(12); y: nb.ui.px(6); width: parent.width - nb.ui.px(24)
            text: nb.pillText
            wrapMode: Text.WordWrap
            color: nb.pillLevel === "warn" ? nb.ui.c.warn : nb.ui.c.text
            font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.px(13)
        }
    }

    component Cap: Rectangle {
        property string k: ""
        implicitWidth: Math.max(nb.ui.px(22), capText.implicitWidth + nb.ui.px(10))
        implicitHeight: nb.ui.px(22)
        radius: k.length > 1 ? nb.ui.px(5) : height / 2
        color: nb.ui.c.raised
        border.color: nb.ui.c.accentLo; border.width: Math.max(1, nb.ui.px(1))
        Text { id: capText; anchors.centerIn: parent; text: nb.glyph(parent.k); color: nb.ui.c.text; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.px(12); font.bold: true }
    }

    // ---------------------------------------------------------------- the page
    Item {
        id: pageArea
        anchors.top: bar.bottom; anchors.left: parent.left; anchors.right: parent.right; anchors.bottom: parent.bottom
        visible: nb.tab === 0
        Loader {
            id: loader
            anchors.fill: parent
            active: nb.live
            sourceComponent: pageComponent
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
            font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.px(16)
        }
        // On Nexus's sign-in page: Turbine's own button (drawn by Turbine over its corner, not put into the page).
        Rectangle {
            id: signinBig
            visible: nb.signInPage && nb.signinShown
            anchors.right: parent.right; anchors.bottom: parent.bottom; anchors.margins: nb.ui.px(16)
            width: bigCol.implicitWidth + nb.ui.px(28); height: bigCol.implicitHeight + nb.ui.px(20)
            radius: nb.ui.px(8)
            color: nb.ui.c.panel
            border.color: nb.ui.c.accent; border.width: Math.max(2, nb.ui.px(2))
            Column {
                id: bigCol
                anchors.centerIn: parent
                spacing: nb.ui.px(4)
                Text { text: "Sign in with my browser (Google and others)"; color: nb.ui.c.accentText; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.px(16); font.weight: Font.DemiBold }
                Text { text: "Email and password work right here too."; color: nb.ui.c.soft; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.px(13) }
            }
            HoverHandler { onHoveredChanged: nb.ui.hover(signinBig, hovered) }
            TapHandler { onTapped: nb.signinClicked() }
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
            onUrlChanged: nb.pageEvent("url", url.toString())
            onLoadingChanged: (info) => nb.pageEvent(info.status === WebEngineView.LoadSucceededStatus ? "loaded" : info.status === WebEngineView.LoadFailedStatus ? "failed" : "loading", info.url.toString() + (info.errorString ? " (" + info.errorString + ")" : ""))
            // Chromium's own right-click menu never shows: LT (a right click) opens Turbine's quick menu instead.
            onContextMenuRequested: (request) => { request.accepted = true; nb.menuWanted() }
            // Pop-ups open here, in the same view.
            onNewWindowRequested: (request) => { nb.pageEvent("newwindow", request.requestedUrl.toString()); request.openIn(web) }
            // Only web pages. A Mod Manager Download link goes to Turbine (the scheme handler sees it too: one
            // counts); every other kind of link is refused.
            onNavigationRequested: (request) => {
                const u = request.url.toString()
                if (/^(https?|about|data|blob):/i.test(u)) return
                request.reject()
                if (/^nxm:/i.test(u)) { Turbine.browserNxm(u); return }
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
        anchors.top: bar.bottom; anchors.left: parent.left; anchors.right: parent.right; anchors.bottom: parent.bottom
        anchors.margins: nb.ui.px(14)
        visible: nb.tab === 1
        Text {
            visible: nb.downloads.length === 0
            anchors.centerIn: parent
            text: "Nothing downloaded yet. Press Slow download on a mod's file page."
            color: nb.ui.c.muted; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.px(16)
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
                        Text { text: row.modelData.name; color: nb.ui.c.text; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.px(16); font.weight: Font.DemiBold; elide: Text.ElideRight; width: Math.min(implicitWidth, dlList.width * 0.5) }
                        Text { text: row.modelData.game || "Other"; color: nb.ui.c.muted; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.px(14) }
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
                    Text { text: row.modelData.words; color: nb.ui.c.soft; font.family: nb.ui.bodyFont; font.pixelSize: nb.ui.px(13) }
                }
                HoverHandler { onHoveredChanged: nb.ui.hover(row, hovered) }
                TapHandler { onTapped: nb.downloadClicked(row.index) }
            }
        }
    }
}
