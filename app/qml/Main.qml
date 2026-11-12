import QtQuick
import QtQuick.Window
import QtQuick.Shapes
import QtCore
import Turbine
import TurbineWeb

// Turbine Mod Manager's interface: the game rail, a game's page (its mods, Get Mods, the mod and game menus, the check
// before Play), Settings, game details, a notice for systems that aren't SteamOS and a 2,000 row list to test scrolling. Everything is sized from a design size the way
// Huginn's handheld mode is: 883 x 552 on a Steam Deck drawn at 1.45x, 914 x 514 on a 7 inch 1080p handheld at 2.1x.
Window {
    id: app
    // qtglue.cpp finds this window by name to deliver the controller pointer's mouse and wheel events.
    objectName: "turbineMain"
    visible: true
    title: "Turbine Mod Manager"
    width: shot.on ? shot.w : 1280
    height: shot.on ? shot.h : 800
    visibility: (Turbine.gamemode && !shot.on) ? Window.FullScreen : Window.Windowed
    color: c.ground

    // ---------------------------------------------------------------- settings that last between launches
    Settings {
        id: look
        category: "look"
        property string theme: "brass"      // brass, black, light
        property string accent: "brass"     // brass, iron, copper, jade, fjord, heather, ember
        property int size: 0                // Interface size, percent (50 to 150); 0 until set
        property bool highContrast: false
        property string vision: "typical"   // typical, red, green, blue, none
        property string layout: "handheld"  // handheld, desktop, tv (tv only with an external display)
        property string season: ""          // off, date, autumn, snow; "" until chosen (off on handhelds, by date on TV)
        property int fallAmount: 40         // 10 to 100
        property int fallSize: 100          // 50 to 200 percent
        property bool compactList: false    // Shorter rows in My mods
        property bool reduceMotion: false   // No sliding or growing (read once, into Animations as "reduced")
        property string animations: ""      // Full, reduced (fades only) or off (instant); "" until chosen
        property string confirmBy: "hold"   // "hold" (A held 5 s) or "twice" (A, then A again) for Reset and delete
        property string holdStartDoes: "choice" // Holding Start 3 s: "choice" (the Play choice) or "last" (plays the last choice)
    }
    // Labs mode (ten presses on the icon in Settings > About) and the owner's checklist answers.
    Settings {
        id: labsSet
        category: "labs"
        property bool on: false
        property string states: "{}"        // item id: "works" or "problem"; missing means not tested
    }
    Settings {
        id: seen
        category: "seen"
        property bool notSteamOS: false
        property string lastGame: ""
        // Kept by the engine now (seen.json, Turbine.adoptSeen): read in once at start, then cleared.
        property string launchDone: "[]"
        property string onceDone: "[]"
        property string reqQueue: ""
        property string depMoves: "{}"
        property string played: "[]"        // game ids, last played first (the rail's order)
        property bool tutorialDone: false
        property bool railHintDone: false   // "Up and Down pick a game, A opens it" was shown until a game was opened
        property string antiCheatDone: "[]" // games whose anti-cheat card was shown
        property string basicIntroDone: "[]" // Basic games whose first-visit card was shown
        property string playedWarn: "[]"
        property string updateWarned: "[]"
        property string modHold: "{}"       // Games played without mods until their mods update: {build, versions}
        property string playChoice: "{}"    // Per game, the last Play choice with Seamless Co-op: mods, coop or plain
        property string coopTileAsked: "[]" // Games whose "Add a Co-op tile to Steam?" was asked once
        property string nexusGameAsked: "[]" // Games whose "Which game on Nexus is this?" was asked once
    }
    // The rail: pinned games first, then the last played; hidden ones (tools like Lossless Scaling) left off.
    Settings {
        id: railSet
        category: "rail"
        property string pinned: "[]"
        property string hidden: "[]"
    }
    // Get mods: the last 8 mod pages opened, per game ({"<game id>": [{id, name, icon, source}]}).
    Settings {
        id: getSet
        category: "getmods"
        property string recent: "{}"
    }
    // Each mod's name and words from its Nexus page ("<game>|<mod id>": {name, author, summary, icon}), kept
    // when Get mods or a mod page shows them, and the names the player gave mods ("<game>|<mod id>": name).
    Settings {
        id: namesSet
        category: "names"
        property string nexus: "{}"
        property string renamed: "{}"
    }
    // Settings > Getting mods: the browser.
    Settings {
        id: browserSet
        category: "browser"
        property string openIn: "turbine"   // Desktop Mode: "turbine" (Nexus inside Turbine) or "mine" (the player's browser)
        property real zoom: 0               // the Nexus page's size; 0 until the player changes it
    }
    // Settings > Updates (and Labs > Update channel).
    Settings {
        id: updates
        category: "updates"
        property bool auto: true
        property bool prerelease: false
    }

    // ---------------------------------------------------------------- screenshot mode (used by the build checks)
    QtObject {
        id: shot
        readonly property var args: Qt.application.arguments
        function arg(name) {
            for (let i = 0; i < args.length; i++)
                if (args[i].startsWith("--" + name + "=")) return args[i].substring(name.length + 3)
            return ""
        }
        readonly property string path: arg("screenshot")
        readonly property bool on: path !== ""
        readonly property int w: parseInt(arg("size").split("x")[0]) || 1280
        readonly property int h: parseInt(arg("size").split("x")[1]) || 800
        readonly property string page: arg("page")
        // Pages that run a list of presses (the controls' acceptance checks) and take their picture at the end.
        readonly property bool scripted: ["controls", "legendcheck", "sheetcheck", "nexusguard", "presscount", "inputcheck", "sizeinputs", "coopcheck", "editbutton", "menufit", "heldcheck", "abcheck", "passthrough", "golden", "mmdguard", "startcheck", "bookcheck", "cooppw", "keycheck", "tilecheck", "oldercheck", "protoncheck", "reqcheck"].indexOf(page) >= 0
        // Pages where every sheet must come from a press (the build checks the log for "opened with no press").
        readonly property bool strictSheets: ["controls", "presscount", "nexusguard"].indexOf(page) >= 0
        // Pages that show Labs mode (it isn't saved from a picture).
        readonly property bool labs: ["about", "labs", "report", "perfcheck", "activity"].indexOf(page) >= 0
        // --layout=desktop or tv (not saved from a picture); --external-display pretends an HDMI display is there.
        readonly property string layout: arg("layout")
        readonly property bool external: args.indexOf("--external-display") >= 0
    }
    // The build's checks themselves (SelfCheck.qml), loaded at start in screenshot runs only; checkIds is what they
    // reach by id.
    Loader { id: selfCheck }
    readonly property var checkIds: !shot.on ? null : ({
        askView: askView, barLb: legend.barLb, barName: legend.barName, barRb: legend.barRb, confirmHold: confirmHold,
        confirmSheet: confirmSheet, edgeBack: edgeBack, edgeNext: edgeNext, gameMenuView: gameMenuView,
        getView: getView, ghostTimer: ghostTimer, hudWin: hudWin, infoView: infoView, inputProbe: inputProbe,
        legend: legend, loadOrderView: loadOrderView, look: look, lookList: settingsPage.lookList, mo: mo, modList: modList,
        panelEdit: modPanel.panelEdit, panelWin: panelWin, perfList: perfList, pickMenu: pickMenu, playBtn: legend.playBtn,
        rail: rail, railList: rail.railList, resetHold: resetHold, seen: seen, settingsView: settingsView,
        sheetLayer: sheetLayer, shot: shot, splash: splash, splashIcon: splash.splashIcon, startHint: legend.startHint,
        startHintKey: legend.startHintKey, startRing: legend.startRing, toast: toast
    })

    // ---------------------------------------------------------------- size
    // Layout: handheld, desktop (mouse in Desktop Mode: smaller, denser rows, 4 columns) or tv (only with an
    // external display). The whole interface is drawn from a design size and never grows past what fits the window:
    // 760 x 460 design units is the smallest it reflows into.
    readonly property string layoutNow: shot.layout !== "" ? shot.layout : (look.layout === "tv" && !externalShown) ? "handheld" : look.layout
    readonly property bool tv: layoutNow === "tv"
    readonly property bool desktopLayout: layoutNow === "desktop"
    readonly property int sizeDefault: tv ? 120 : desktopLayout ? 80 : 100
    readonly property real fit: Math.max(0.5, Math.min(width / 883, height / 514))
    // Interface size 50% to 150% in 10% steps, with no cap: at large sizes the layout reflows (fewer grid
    // columns, rows and sheets scroll, the bottom bar drops hints, text wraps) instead of stopping at what fits.
    readonly property int sizeMin: 50
    readonly property int sizeMax: 150
    readonly property var sizeSteps: { let l = []; for (let v = sizeMin; v <= sizeMax; v += 10) l.push(v); return l }
    // --uisize=N: a picture at that Interface size.
    readonly property int sizeNow: Math.max(sizeMin, Math.min(sizeMax, sizeTrial > 0 ? sizeTrial : parseInt(shot.arg("uisize")) > 0 ? parseInt(shot.arg("uisize")) : look.size > 0 ? look.size : sizeDefault))
    function sizeStep(d) {
        let i = sizeSteps.findIndex(v => v >= sizeNow)
        if (i < 0) i = sizeSteps.length - 1
        else if (sizeSteps[i] !== sizeNow && d < 0) i++   // between two steps: Left goes to the lower one
        return sizeSteps[Math.max(0, Math.min(sizeSteps.length - 1, i + d))]
    }
    readonly property real s: Math.max(0.35, fit * sizeNow / 100)
    function px(v) { return Math.round(v * s) }
    // Text: never under 12 pixels, and never smaller than 90% of its design size, so 50% stays readable (body text
    // keeps 14 pixels on a Steam Deck). fpxAt(v, scale): the same at another scale (the bottom bar's own).
    function fpxAt(v, sc) { return Math.max(12, Math.round(v * Math.max(sc, 0.9))) }
    function fpx(v) { return fpxAt(v, s) }
    // Something to press. Touch targets never shrink under a finger, whatever the Interface size: 44 pixels, or 6 mm on a dense
    // screen (the Ally X's 7 inch 1080p panel is about 12 pixels a millimetre).
    readonly property int touchFloor: Math.max(44, Math.round(Screen.pixelDensity * 6))
    function tpx(v) { return Math.max(touchFloor, Math.round(v * s)) }
    // Design width: 2 grid columns under 640 design units, 3 under 760, 4 from there.
    readonly property real designWidth: width / s
    readonly property int gridColumns: designWidth < 640 ? 2 : designWidth < 760 ? 3 : 4
    // An external display: HDMI or DisplayPort in /sys, or a Qt screen that isn't the built-in panel.
    property bool externalShown: shot.external
    function checkDisplays() {
        if (shot.on) return
        let ext = Turbine.externalDisplay()
        for (const sc of Qt.application.screens) if (sc.name && !sc.name.startsWith("eDP")) ext = true
        if (ext !== externalShown) note("window", ext ? "External display connected" : "No external display")
        const lost = externalShown && !ext && look.layout === "tv"
        externalShown = ext
        if (lost) { look.layout = "handheld"; toast.info("TV unplugged. Back to the handheld layout.") }
    }
    Connections { target: Qt.application; function onScreensChanged() { app.checkDisplays() } }
    // Pictures only: a size tried without saving it.
    property int sizeTrial: 0
    // Size and Layout apply live: no "Keep this size?" sheet (the slider never goes past what fits).
    function sizePreview(v) {
        v = Math.max(sizeMin, Math.min(sizeMax, v))
        if (v === sizeNow) return
        note("action", "Interface size " + sizeNow + "% to " + v + "%")
        // Pictures never save settings.
        if (shot.on) sizeTrial = v; else { look.size = v; sizeTrial = 0 }
    }
    // Interface size: every size from 50% to 150%, the current one marked. A picks, B cancels.
    function openSizePicker() {
        note("action", "Interface size list open at " + sizeNow + "%")
        openPick("Interface size", "Now " + sizeNow + "%. A picks a size, B keeps this one.", sizeSteps.map(v => ({
            icon: "info", title: v + "%" + (v === sizeDefault ? " (normal)" : ""), detail: v === sizeNow ? "Current size" : v < sizeNow ? "Smaller" : "Bigger",
            aLabel: v === sizeNow ? "Keep" : "Pick", run: () => sizePreview(v) })), "")
        pickBLabel = "Cancel"
        menuIndex = Math.max(0, sizeSteps.indexOf(sizeNow))
    }
    function layoutPreview(v) {
        note("action", "Layout " + v)
        if (!shot.on) look.layout = v
    }

    // ---------------------------------------------------------------- Labs mode and the activity log
    readonly property bool labsOn: labsSet.on || (shot.on && shot.labs)
    function note(kind, text) { Turbine.log(kind, text) }
    onPageChanged: { note("page", "Page " + page); stopDangerHolds(); barsHidden = false; if (page === "settings") settingsFresh = true; else settingsFrom = null; pageArrived() }
    // A whole page arriving (another game from the rail, Settings, Where do these files go?, Nexus, back from it):
    // it fades up as it rises a little into place (Arrive.qml, gentle). The state is already the new page's.
    function pageArrived() {
        if (page === "browser") browserIn.play(0, px(mo.nudge))
        else pageIn.play(0, px(mo.nudge))
    }
    // No top bar on the game screen. Going down a long list hides the bottom bar and the rail too, so the
    // content is truly full screen; going up, reaching the top, View, a tab change, a sheet or the rail bring them
    // back. Reduce motion: no slide, they just go and come.
    property bool barsHidden: false
    // ---- One motion system (Motion.qml): every duration and curve comes from it. Settings, Look, Animations:
    // Full, Reduced (fades only) or Off (instant). The build's checks run with it off, so no check races an
    // animation (--motion=full for the pass that only looks at where things end up; the splash pictures play it).
    readonly property string motionLevel: shot.on ? (shot.arg("motion") !== "" ? shot.arg("motion") : shot.page.indexOf("splash") === 0 ? "full" : "off")
                                          : look.animations !== "" ? look.animations : look.reduceMotion ? "reduced" : "full"
    Motion { id: mo; level: app.motionLevel }
    // A list or grid brought to its focused row: `jump` puts it there (positionViewAtIndex and the like); at Full
    // the view then glides from where it was instead of snapping. One at a time, never an overshoot, and a new
    // one takes over from wherever the last had got to. Only within about a screen: further (a list opening on a
    // row far down, a jump to the top or the end) it is simply there, as with animations off.
    NumberAnimation {
        id: glideAnim
        properties: "contentY"; duration: mo.moveStandard; easing.type: mo.glide
        // It ends exactly where a jump would have put it (rows of different heights can move the mark on the way).
        onFinished: { const j = app.glideJump; app.glideJump = null; if (j) j() }
    }
    property var glideJump: null
    property int glides: 0
    // Nothing still moving of what the build's motion check knows (page turns, the rail, pages and a mod's page
    // arriving, list glides).
    readonly property bool motionStill: turnTo < 0 && rail.slide === 0 && !pageIn.running && !browserIn.running && !modIn.running && !gridBack.running && !glideAnim.running
    function glide(view, jump) {
        if (!mo.moves || !view || !view.visible) { jump(); return }
        if (glideAnim.running) {
            const was = glideAnim.target, wasJump = glideJump
            glideAnim.stop(); glideJump = null
            // Another view's glide ends exactly where a jump puts it now (its rows may have been laid out since
            // its glide began).
            if (was && was !== view) { if (wasJump) wasJump(); else was.contentY = glideAnim.to }
        }
        const from = view.contentY
        jump()
        const to = view.contentY
        if (Math.abs(to - from) < 1 || Math.abs(to - from) > view.height) return
        view.contentY = from
        glideAnim.target = view; glideAnim.from = from; glideAnim.to = to
        glideJump = jump
        glides++
        glideAnim.start()
    }
    // A list brought to row i: the row fully in view with a little room around it, so it is never against an edge
    // or under a fade (the bottom bar is outside every list).
    function reveal(view, i) {
        if (!view || i < 0) return
        view.positionViewAtIndex(i, ListView.Contain)
        const it = view.itemAtIndex(i)
        // (Not laid out yet: the list has no height of its own.)
        if (!it || view.height <= it.height) return
        // Clear of the list's fades (ScrollFade, 26 design pixels).
        const m = Math.min(px(28), Math.max(0, (view.height - it.height) / 2))
        const lo = view.originY - view.topMargin, hi = Math.max(lo, view.originY + view.contentHeight + view.bottomMargin - view.height)
        if (it.y - m < view.contentY) view.contentY = Math.max(lo, it.y - m)
        else if (it.y + it.height + m > view.contentY + view.height) view.contentY = Math.min(hi, it.y + it.height + m - view.height)
    }
    onBarsHiddenChanged: { barsChangedAt = Date.now(); note("page", "Bars " + (barsHidden ? "hidden (full screen)" : "shown")) }
    // They only hide when there is more than a screen to see: the list is taller than its view and the focused row
    // is past the first screen. Game switches and a flick back to the top bring them back.
    function tuckFollow(now, before, longer) {
        if (page !== "game" || modPage) return
        if (now <= 0) barsHidden = false
        else if (now > before) { if (longer) barsHidden = true }
        else if (now < before) barsHidden = false
    }
    // A touch or wheel scroll: down hides, up shows (a little give, and never while the bars just changed, since
    // that changes the list's height too).
    property real barsChangedAt: 0
    function scrollFollow(y, before, longer) {
        if (page !== "game" || modPage || Date.now() - barsChangedAt < 400) return
        if (y <= px(4)) barsHidden = false
        else if (y > before + px(24) && longer) barsHidden = true
        else if (y < before - px(24)) barsHidden = false
    }
    function modRowPastFirstScreen(i) {
        if (!modList || modList.contentHeight <= modList.height) return false
        const it = modList.itemAtIndex(i)
        return !it || it.y + it.height > modList.height - modList.topMargin
    }
    property int tuckModBefore: 0
    onModIndexChanged: {
        tuckFollow(modIndex, tuckModBefore, modRowPastFirstScreen(modIndex)); tuckModBefore = modIndex
        if (keptRow !== "" && !(focusedItem && focusedItem.kind === "mod" && focusedItem.m.id === keptRow)) keptRow = ""
    }
    property int tuckGetBefore: 0
    onGetIndexChanged: {
        const cols = Math.max(1, gridColumns), row = Math.floor(getIndex / cols)
        // About two rows of cards fit on a screen: more than that is a longer grid.
        if (getArea === "grid") tuckFollow(row, Math.floor(tuckGetBefore / cols), Math.ceil(getItems.length / cols) > 2 && row >= 2)
        else barsHidden = false
        tuckGetBefore = getIndex
    }
    // ---- Get mods is a page of a book. No game list there: My mods is the page to the left (its name and an
    // arrow down the left edge) and Load order the page to the right. Left from the first column, LB, a swipe to
    // the right or a click on the edge turns back; Right from the last column, RB, a swipe to the left or the right
    // edge turns forward. The page coming in fades up as it slides in a short way from its side; the one it replaces
    // fades out quickly, drifting the other way (Reduced: the fades alone).
    readonly property bool bookTab: page === "game" && !!game && tab === 1
    readonly property bool bookPage: bookTab && !modPage
    readonly property bool bookHasNext: tabs.length > 2
    // The game list steps aside on Get mods (it stays while the focus is in it).
    readonly property bool railAway: barsHidden || (bookTab && zone !== "rail")
    property bool padRepeat: false        // the press being handled is a held D-pad or key repeating
    function turnPage(d) {
        // A held D-pad stops at the edge of the page; a new press turns it.
        if (padRepeat) return false
        const t = tab + d
        if (t < 0 || t >= tabs.length) return false
        note("action", "Page turned " + (d < 0 ? "back" : "forward") + " to " + tabs[t].replace(/ \(\d+\)$/, ""))
        switchTab(d)
        return true
    }
    property int turnFrom: -1             // the tab going while a page turns, else -1
    property int turnTo: -1               // the tab coming in while a page turns, else -1
    property int turnDir: 0               // -1: the page comes in from the left (back), 1: from the right
    // 0 to 1 while a page turns: the page coming in (its fade, its travel), the page going (gone at 1).
    property real turnInFade: 1
    property real turnInMove: 1
    property real turnOut: 1
    property int turnsShown: 0
    property int tabBefore: 0
    property real gameChangedAt: 0        // a tab that changes with the game is no page turn
    property int lastTurnDir: 0
    ParallelAnimation {
        id: turnAnim
        NumberAnimation { target: app; properties: "turnInFade"; from: 0; to: 1; duration: mo.standard; easing.type: mo.ease }
        NumberAnimation { target: app; properties: "turnInMove"; from: 0; to: 1; duration: mo.moveStandard; easing.type: mo.glide }
        NumberAnimation { target: app; properties: "turnOut"; from: 0; to: 1; duration: mo.quick; easing.type: mo.ease }
        onFinished: { app.turnFrom = -1; app.turnTo = -1 }
    }
    property bool turnModPage: false      // the page going is a mod's page (over Get mods)
    // Both pages start from how they look now (one may still be coming in or going from a turn a moment ago).
    property real turnOutFade: 1
    property real turnOutShift: 0
    property real turnInFrom: 0
    function turnStart(from, to) {
        turnOutFade = tabFade(from); turnOutShift = tabShift(from)
        turnInFrom = to === turnFrom ? tabFade(to) : 0
        turnAnim.stop()
        turnFrom = from; turnTo = to; turnDir = to > from ? 1 : -1
        turnModPage = from === 1 && modPage
        turnInFade = 0; turnInMove = 0; turnOut = 0
        turnsShown++
        turnAnim.start()
    }
    // What a tab's page draws: it shows on its tab, and stays while it turns away. Its opacity and its sideways
    // shift while a page turns (1 and 0 otherwise).
    function onTab(n) { return tab === n || turnFrom === n }
    function tabFade(n) { return n === turnTo ? turnInFrom + (1 - turnInFrom) * turnInFade : n === turnFrom ? turnOutFade * (1 - turnOut) : 1 }
    function tabShift(n) { return n === turnTo ? turnDir * (1 - turnInMove) * px(mo.shift) : n === turnFrom ? turnOutShift - turnDir * turnOut * px(mo.nudge) : 0 }
    onTabChanged: {
        // One page to the next in the same game: the turn. (Not in the build's pictures, but for its own check.)
        if (page === "game" && !!game && Date.now() - gameChangedAt > 150 && Math.abs(tab - tabBefore) === 1 && mo.on) turnStart(tabBefore, tab)
        else { turnAnim.stop(); turnFrom = -1; turnTo = -1 }
        lastTurnDir = tab > tabBefore ? 1 : -1
        tabBefore = tab
        barsHidden = false
        note("page", "Tab " + (tabs[tab] || tab))
        stopDangerHolds()
        selecting = false
        modPage = false
        history = []
        // A Basic game Turbine couldn't match on Nexus: asked once, when the player opens Get mods.
        if (tab === 1) Qt.callLater(() => app.maybeAskNexusGame("Get mods opened"))
        if (tab === 1 && !browseData) {
            // A first visit lands on the first card (Y opens Search, Sort and Category).
            if (!restoreFocus && getText === "") restoreFocus = { area: "grid", index: 0 }
            browseNow()
        }
        else if (tab === 2) { if (orderMemo < 0 && order.length > 0) orderMemo = orderIndex; orderLand = true; Turbine.loadOrder() }
    }
    onZoneChanged: { note("page", "Focus zone " + zone); if (zone !== "main" && barsHidden) barsHidden = false }
    onSetSectionChanged: {
        settingsFresh = true
        note("page", "Settings section " + (settingsSections[setSection] || setSection))
        if (settingsSections[setSection] === "Getting mods") { Turbine.handlerStatus(); if (!shot.on) { Turbine.bridgeRefresh(); signinLoad("Settings") } signinRefresh() }
        if (settingsSections[setSection] === "Help" && !decky.checked) deckyCheck()
    }
    onWidthChanged: sizeNote.restart()
    onHeightChanged: sizeNote.restart()
    Timer { id: sizeNote; interval: 500; onTriggered: app.note("window", "Window " + app.width + " x " + app.height + ", scale " + app.s.toFixed(2)) }
    // The first frame drawn: how long starting took (Labs > Performance check).
    property bool shown: false
    onFrameSwapped: { if (!shown) { shown = true; Turbine.markShown() } if (nxmWaitFrame) nxmShown("first frame") }

    // ---------------------------------------------------------------- fonts
    FontLoader { id: cinzel; source: "file:///app/share/turbine-mod-manager/fonts/Cinzel.ttf" }
    FontLoader { id: barlow; source: "file:///app/share/turbine-mod-manager/fonts/Barlow-Regular.ttf" }
    FontLoader { source: "file:///app/share/turbine-mod-manager/fonts/Barlow-Medium.ttf" }
    FontLoader { source: "file:///app/share/turbine-mod-manager/fonts/Barlow-SemiBold.ttf" }
    FontLoader { source: "file:///app/share/turbine-mod-manager/fonts/Barlow-Bold.ttf" }
    readonly property string titleFont: cinzel.status === FontLoader.Ready ? cinzel.font.family : "serif"
    readonly property string bodyFont: barlow.status === FontLoader.Ready ? barlow.font.family : "sans-serif"

    // ---------------------------------------------------------------- colors
    QtObject {
        id: c
        readonly property bool hc: look.highContrast
        readonly property bool light: look.theme === "light" && !hc
        readonly property bool black: look.theme === "black" || hc
        readonly property color ground: hc ? "#000000" : light ? "#F1EADB" : black ? "#000000" : "#0B0A08"
        readonly property color rail: hc ? "#000000" : light ? "#E6DCC6" : black ? "#000000" : "#080706"
        readonly property color panel: hc ? "#000000" : light ? "#E7DDC8" : black ? "#0C0B0A" : "#141109"
        readonly property color card: hc ? "#000000" : light ? "#F7F2E7" : black ? "#080707" : "#100E0A"
        readonly property color raised: hc ? "#1A1A1A" : light ? "#DDD1B8" : black ? "#161412" : "#211B12"
        readonly property color line: hc ? "#FFFFFF" : light ? "#CDBF9F" : black ? "#26221C" : "#33291B"
        readonly property color text: hc ? "#FFFFFF" : light ? "#2A2216" : "#EADFC6"
        readonly property color soft: hc ? "#FFFFFF" : light ? "#4A3E2A" : "#CDBE9F"
        readonly property color muted: hc ? "#E0E0E0" : light ? "#5E5038" : "#AC9873"
        readonly property var swatches: ({ brass: "#C99E52", iron: "#A9B4BE", copper: "#B07A5A", jade: "#6FAE8C",
                                           fjord: "#6E9CC8", heather: "#A58BC6", ember: "#CF6F54" })
        readonly property color accentBase: look.vision === "none" ? "#BDBDBD" : (swatches[look.accent] || swatches.brass)
        readonly property color accent: hc ? "#FFD54A" : light ? Qt.darker(accentBase, 1.8) : accentBase
        readonly property color accentHi: hc ? "#FFE680" : light ? Qt.darker(accentBase, 1.6) : Qt.lighter(accentBase, 1.22)
        readonly property color accentLo: light ? Qt.darker(accentBase, 2.1) : Qt.darker(accentBase, 1.35)
        readonly property color onAccent: light ? "#FFFFFF" : "#0B0A08"
        // Accent-colored words: at least 4.5:1 on every surface here, in every theme, accent and color vision mode.
        readonly property color accentText: hc ? "#FFD54A" : light ? Qt.darker(accentBase, 2.2) : Qt.lighter(accentBase, 1.12)
        readonly property color focusFill: hc ? "#333300" : light ? Qt.lighter(accentBase, 1.55) : Qt.darker(accentBase, 4.2)
        // Status colors stay apart for each kind of color vision; words always carry the meaning too. Each reads at
        // 4.5:1 or better as text on ground, panel, card, raised and focusFill.
        readonly property color ok: look.vision === "none" ? soft : (look.vision === "red" || look.vision === "green") ? (light ? "#25507E" : "#7FAAD3") : light ? "#3F5824" : "#9DBF7A"
        readonly property color warn: look.vision === "none" ? text : (look.vision === "red" || look.vision === "green") ? (light ? "#704700" : "#E0A040") : light ? "#843C20" : "#E6A98A"
        readonly property color stop: look.vision === "none" ? text : hc ? "#FF8A80" : light ? "#8E2A1E" : "#F0907A"
    }

    // ---------------------------------------------------------------- what the designer's components need
    readonly property var colors: c
    QtObject {
        id: kit
        readonly property var c: app.colors
        // Cinzel (app.titleFont) is only for the wordmark and the game's name: the components' titles are Barlow.
        readonly property string titleFont: app.bodyFont
        readonly property string bodyFont: app.bodyFont
        function px(v) { return app.px(v) }
        function fpx(v) { return app.fpx(v) }
        function tpx(v) { return app.tpx(v) }
        function hover(item, on) { app.hover(item, on) }
        // One look everywhere (COMPONENTS.md, "The shared looks"): corner radii, the focus ring (focus is an outline,
        // never a fill), the backdrop under everything that covers the window, and the gap between the window's
        // edges and a card.
        readonly property real radius: app.px(10)
        readonly property real radiusSmall: app.px(6)
        readonly property real ringWidth: Math.max(2, app.px(2))
        readonly property color ring: c.accentHi
        readonly property color scrim: Qt.rgba(c.ground.r * 0.6, c.ground.g * 0.6, c.ground.b * 0.6, c.light ? 0.82 : 0.9)
        readonly property real edge: app.px(16)
        // The bottom bar's top in the window (the toast sits above it).
        readonly property real barTop: legend.shown ? app.height - legend.height : app.height
        // A button's cap as the pad, the keyboard or the mouse names it ("A", "Enter"); no caps after the mouse.
        function glyph(k) { return app.glyph(k) }
        readonly property bool capsShown: !app.mouseLegend
        // Line icons on a 24 x 24 grid, one per kind of action, and the icon a menu row gets from its words.
        readonly property var icons: ({
            update: "M12 4v11M7 10l5 5 5-5M5 20h14",
            download: "M12 4v11M7 10l5 5 5-5M5 20h14",
            refresh: "M20 11a8 8 0 0 0-14.9-3.9L4 9 M4 4v5h5 M4 13a8 8 0 0 0 14.9 3.9L20 15 M20 20v-5h-5",
            power: "M12 3v8M6.5 6.5a8 8 0 1 0 11 0",
            overlap: "M5 3h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z M11 9h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2h-8a2 2 0 0 1-2-2v-8a2 2 0 0 1 2-2z",
            files: "M14 3H6a1 1 0 0 0-1 1v16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V8z M14 3v5h5",
            move: "M8 4v16M4 8l4-4 4 4M16 20V4M12 16l4 4 4-4",
            remove: "M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13",
            close: "M6 6l12 12 M18 6L6 18",
            play: "M7 4l13 8-13 8z",
            back: "M15 5l-7 7 7 7",
            forward: "M9 5l7 7-7 7",
            restore: "M4 4v6h6 M5 15a8 8 0 1 0 2-8.5L4 10",
            profile: "M12 4a4 4 0 1 1 0 8 4 4 0 0 1 0-8z M4 21c1-4 4-6 8-6s7 2 8 6",
            info: "M12 3a9 9 0 1 1 0 18 9 9 0 0 1 0-18z M12 11v6 M12 7.5v0.5",
            help: "M12 3a9 9 0 1 1 0 18 9 9 0 0 1 0-18z M9.5 9.2a2.5 2.5 0 1 1 3.6 2.2c-.7.4-1.1.9-1.1 1.7v.5 M12 16.8v.4",
            pencil: "M4 20h4L19 9l-4-4L4 16z M13.5 6.5l4 4",
            more: "M5 11a1 1 0 1 0 0 2 1 1 0 1 0 0-2z M12 11a1 1 0 1 0 0 2 1 1 0 1 0 0-2z M19 11a1 1 0 1 0 0 2 1 1 0 1 0 0-2z",
            list: "M9 6h11 M9 12h11 M9 18h11 M4.5 6v.2 M4.5 12v.2 M4.5 18v.2",
            sliders: "M4 21v-7 M4 10V3 M12 21v-9 M12 8V3 M20 21v-5 M20 12V3 M2 14h4 M10 8h4 M18 16h4",
            search: "M11 4a7 7 0 1 1 0 14 7 7 0 0 1 0-14z M20 20l-4-4",
            checkbox: "M5 4h14a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1z M8.5 12l2.5 2.5 4.5-5",
            eyeoff: "M3 3l18 18 M10.6 6.1A9 9 0 0 1 12 6c5 0 9 6 9 6a16 16 0 0 1-3 3.6 M6.6 6.7C4.3 8.2 3 12 3 12s4 6 9 6a9 9 0 0 0 4.3-1.1 M9.9 9.9a3 3 0 0 0 4.2 4.2",
            pin: "M12 16v5 M9 3h6l-1 6 4 4v2H6v-2l4-4z",
            external: "M14 4h6v6 M20 4l-9 9 M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5",
            folder: "M3 7a1 1 0 0 1 1-1h5l2 2h9a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1z",
            copy: "M9 9h11v11H9z M5 15H4V4h11v1",
            zoom: "M11 4a7 7 0 1 1 0 14 7 7 0 0 1 0-14z M20 20l-4-4 M8 11h6 M11 8v6",
            save: "M5 3h11l3 3v13a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z M7 3v5h8 M7 21v-7h10v7",
            sort: "M4 6h16 M7 12h10 M10 18h4",
            shield: "M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6z M9 12l2 2 4-4",
            log: "M6 3h9l4 4v13a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z M8 11h8 M8 15h8 M8 7h4"
        })
        // A menu row's icon: what its words say it does (the first match), else the icon it was given.
        readonly property var iconWords: [
            [/^(uninstall|delete)\b|^remove (it|the|this|from)/i, "remove"],
            [/^(not now|cancel|stay|forget|stop|close|keep it on|it isn't on|none of)/i, "close"],
            [/^rename\b/i, "pencil"],
            [/^(more|and \d+ more)\b/i, "more"],
            [/settings\b/i, "sliders"],
            [/^(search|find)\b/i, "search"],
            [/activity log/i, "log"],
            [/^select several/i, "checkbox"],
            [/^hide\b/i, "eyeoff"],
            [/^(pin|unpin)\b/i, "pin"],
            [/^check game folder/i, "shield"],
            [/^(check for updates|check again|look again)/i, "refresh"],
            [/on nexus$|nexus page|^browse\b|^open (it )?on nexus/i, "external"],
            [/^(install|get |download)/i, "download"],
            [/^(why|help|how to)\b/i, "help"],
            [/folder\b/i, "folder"],
            [/^copy\b/i, "copy"],
            [/^zoom\b/i, "zoom"],
            [/^(forward|go to|show in)\b/i, "forward"],
            [/^save\b/i, "save"],
            [/^(sort|show as)\b/i, "sort"],
            [/^(game )?details$/i, "list"]
        ]
        function iconFor(icon, label) {
            for (const w of iconWords) if (w[0].test(label || "")) return w[1]
            return icon || ""
        }
        function iconPath(name) { return name ? (icons[name] || name) : "" }
        // The motion system, and the glide to a focused row.
        readonly property Motion motion: mo
        // A page or a tab is on its way in: what shows up inside it comes with it (no fade of its own).
        readonly property bool arriving: app.turnTo >= 0 || pageIn.running
        function glide(view, jump) { app.glide(view, jump) }
        function reveal(view, i) { app.reveal(view, i) }
        // Words for the mod pages: "24.0 MB" ("" for none), "Mar 4, 2025" from "2025-03-04" (anything else stays as it
        // is), "3.1M" and "12K".
        function fileSize(b) {
            if (!b) return ""
            if (b >= 1073741824) return (b / 1073741824).toFixed(1) + " GB"
            if (b >= 1048576) return (b / 1048576).toFixed(1) + " MB"
            if (b >= 1024) return Math.round(b / 1024) + " KB"
            return b + " bytes"
        }
        function niceDate(t) {
            const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(t || "")
            return m ? Qt.formatDate(new Date(+m[1], +m[2] - 1, +m[3]), "MMM d, yyyy") : (t || "")
        }
        function shortCount(n) {
            n = n || 0
            if (n >= 1000000) return (n / 1000000).toFixed(1).replace(".0", "") + "M"
            if (n >= 1000) return Math.round(n / 1000) + "K"
            return String(n)
        }
        // Scrolls the Flickable view so the item it (somewhere inside content) is fully in view.
        function follow(view, content, it) {
            if (!it) return
            const y = it.mapToItem(content, 0, 0).y
            if (y < view.contentY) view.contentY = Math.max(0, y)
            else if (y + it.height > view.contentY + view.height) view.contentY = Math.min(Math.max(0, view.contentHeight - view.height), y + it.height - view.height)
        }
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
    }

    // ---------------------------------------------------------------- data
    property var games: []
    property int current: 0
    readonly property var game: games.length > 0 ? games[Math.min(current, games.length - 1)] : null
    function load() {
        Turbine.scan()
    }

    // The current game's mods, rules and Downloads, from the engine as JSON.
    property var mods: []
    property var removedMods: []
    property var setups: []
    property bool modsLoaded: false
    property var rules: null
    property var downloads: []
    property var checks: []
    readonly property bool ordered: rules !== null && rules.ordered === true
    // The name for tight places (the header): the game rules' short name when it has one.
    readonly property string gameTitle: game ? ((rules && rules.shortName) || game.name) : ""
    readonly property bool loaderMissing: rules !== null && rules.loader !== null && rules.loader !== undefined && !rules.loaderInstalled
    readonly property string loaderShort: rules && rules.loader ? rules.loader.name.split(" (")[0] : ""
    readonly property var needLoader: loaderMissing ? mods.filter(m => m.enabled && m.needsLoader) : []
    readonly property var tabs: ["My mods (" + mods.length + ")"].concat(acRefused ? [] : ["Get mods"]).concat(rules && rules.plugins ? ["Load order"] : [])
    // ---------------------------------------------------------------- anti-cheat games
    // The data file says how each is played with mods: "me3" (me3 starts it offline without the anti-cheat, mods in a
    // Turbine-owned profile, asked once), "official" (Halo's own switch), "offline", "refuse" (online only: no mods).
    readonly property var acInfo: rules && rules.antiCheat ? rules.antiCheat : null
    readonly property string acMode: acInfo ? (acInfo.mode || "warn") : ""
    readonly property bool acRefused: acMode === "refuse"
    property var acAnswered: ({})          // games answered this run (the engine keeps the answer)
    function acAskNeeded() { return !!game && acMode === "me3" && acInfo.needsAsk === true && !acInfo.off && !acAnswered[game.id] && mods.some(m => m.enabled) }
    readonly property bool acOffNow: acMode === "me3" && !!acInfo && (acInfo.off === true || (!!game && !!acAnswered[game.id])) && mods.some(m => m.enabled)
    // Turn off anti-cheat for <game>? A turns it off for mods (then `then`), B Not now.
    function openAcAsk(then) {
        if (!acInfo) return
        note("action", "Anti-cheat question: " + acInfo.askTitle)
        // The first modded Play of a me3 game is one question and one press. What would have been sheets
        // of their own (the game update warning, me3 coming down) is said here; A plays.
        const upd = gameUpdate && gameUpdate.risky && gameUpdate.risky.length > 0 ? gameUpdate.risky.map(r => r.name) : []
        if (upd.length > 0 && game && !shot.on) Turbine.markUpdateWarned(game.id + "|" + gameUpdate.now)
        // More bullets under the question's own.
        const extra = (acInfo.me3Ready ? "" : "\n• This once, Turbine first gets Mod Engine 3, the free tool that loads the mods.")
            + (upd.length > 0 ? "\n• " + gameTitle + " updated since mods were last on. These may not work until they update: " + upd.join(", ") + "." : "")
        openMsg({ title: acInfo.askTitle, text: acInfo.askText + extra, buttons: [
            { key: "a", label: "Play with mods, offline", run: () => {
                let a = Object.assign({}, acAnswered); a[game.id] = true; acAnswered = a
                Turbine.antiCheatOff(true)
                note("action", "Anti-cheat: off while mods are on")
                if (then) then()
            } },
            { key: "b", label: "Not now", run: () => note("action", "Anti-cheat: not now (mods wait)") }] })
    }
    function openAcGuide() {
        if (!acInfo) return
        note("action", "Anti-cheat guide: " + gameTitle)
        openMsg({ title: "Play " + gameTitle + " with mods", text: gameTitle + " has its own switch for this, in Steam's Play menu.", steps: acInfo.guide || [], buttons: [{ key: "b", label: "Close", run: () => {} }] })
    }
    // The engine marks the one Seamless Co-op the game loads ("seamless": true in the mods' JSON), whatever
    // it came from: Turbine's own download or a copy from Nexus count the same. Every test for it reads that mark.
    readonly property bool seamlessIn: mods.some(m => m.seamless === true)
    readonly property var seamlessMod: mods.find(m => m.seamless === true) || null
    // Play Seamless Co-op (Elden Ring through Steam and me3, its own .co2 saves). Turns it on first if it's off.
    // Every co-op start comes through here (the Play choice, Start held, Play options, the Co-op tile and
    // Steam's menu, "Use it again"): with no session password the mod stops with an error, so its settings open
    // on the password first, with the keyboard, and the start goes on once it is set.
    // ---- Two copies of Seamless Co-op in the list (one from Nexus and Turbine's own, or an old one). The
    // engine looks (Turbine.coopCheck, answered as coopChecked): the game loads the newest, and the others only
    // confuse. One sheet, on opening the game (once per run) and before a co-op start: Tidy up keeps the newest
    // and moves the other to Removed mods, the password kept; Not now leaves them. A version that disagrees with
    // its own file is a toast with both versions named (once per run).
    property var coopFound: null          // the last answer for the game on screen
    property var coopTidyAsked: ({})      // games asked on opening, this run
    property var coopVersionSaid: ({})
    property var coopAfterTidy: null      // the co-op start waiting for the sheet's answer
    property int coopLooks: 0
    property int coopModsSeen: -1
    function coopLook() { if (game && seamlessIn) { coopModsSeen = mods.length; Turbine.coopCheck() } }
    function coopLooked(gameId, d) {
        if (!game || game.id !== gameId || !d) return
        coopFound = Object.assign({ game: gameId }, d)
        coopLooks++
        const then = coopAfterTidy
        coopAfterTidy = null
        if ((d.tidied || []).length > 0) {
            note("action", "Seamless Co-op tidied: " + d.tidied.join(", ") + " moved to Removed mods")
            toast.done("Kept " + (d.name || "Seamless Co-op") + (d.listVersion ? " " + d.listVersion : "") + " and turned it on. " + d.tidied.join(", ") + " moved to Removed mods.")
        }
        // The co-op start that waited for the tidy goes on (whatever the tidy found to do).
        if (then) { then(); return }
        if ((d.tidied || []).length > 0) return
        if (d.versionWarning && !coopVersionSaid[gameId]) {
            let s = Object.assign({}, coopVersionSaid); s[gameId] = true; coopVersionSaid = s
            note("action", "Seamless Co-op version warning: list " + d.listVersion + ", file " + d.fileVersion)
            toast.info(d.versionWarning)
        }
        if ((d.others || []).length > 0 && !coopTidyAsked[gameId] && sheet === "" && !wheelOpen && page === "game") {
            let a = Object.assign({}, coopTidyAsked); a[gameId] = true; coopTidyAsked = a
            openCoopTidy(null)
        }
    }
    function coopCopies() { return coopFound && game && seamlessIn && coopFound.game === game.id ? (coopFound.others || []) : [] }
    function openCoopTidy(then) {
        const others = coopCopies()
        const n = others.length + 1
        const count = n === 2 ? "two" : n === 3 ? "three" : String(n)
        note("action", "Seamless Co-op: " + n + " copies (" + (coopFound.name || "Seamless Co-op") + " " + (coopFound.listVersion || "") + " is used; not used: " + others.join(", ") + ")" + (then ? ", before a co-op start" : ""))
        coopAfterTidy = null
        openMsg({ title: "You have " + count + " copies of Seamless Co-op",
                  text: "You have " + count + " copies of Seamless Co-op. Turbine keeps the newest and moves the other" + (n > 2 ? "s" : "") + " to Removed mods. Your password is kept.",
                  notes: ["Kept: " + (coopFound.name || "Seamless Co-op") + (coopFound.listVersion ? " " + coopFound.listVersion : "")].concat(others.map(o => "Moved to Removed mods: " + o)),
                  buttons: [{ key: "a", label: "Tidy up", run: () => { note("action", "Seamless Co-op: Tidy up"); coopAfterTidy = then; coopFound = Object.assign({}, coopFound, { others: [] }); Turbine.seamlessTidy() } },
                            { key: "b", label: "Not now", run: () => { note("action", "Seamless Co-op copies: not now"); coopFound = Object.assign({}, coopFound, { others: [], skipped: true }); if (then) then() } }] })
    }
    // ---- A Seamless Co-op file older than the one installed (an old download, or GitHub behind Nexus) is
    // never put over the newer one by itself. One question: Keep the one installed (A, the default; B goes back
    // and keeps it too) or Use the older file (X). The file is not shown as a mod that failed.
    property int olderAsks: 0
    function askOlderSeamless(gameId, have, thisOne) {
        const i = games.findIndex(g => g.id === gameId)
        if (i >= 0 && i !== current) current = i
        olderAsks++
        note("action", "Older Seamless Co-op: have " + have + ", this file is " + thisOne)
        const answer = use => { Turbine.olderSeamlessAnswer(use); Turbine.statusDone() }
        openMsg({ title: "This Seamless Co-op is older", reason: "an older Seamless Co-op file",
                  text: "You have Seamless Co-op " + have + ". This file is " + thisOne + ", which is older.",
                  buttons: [{ key: "a", label: "Keep " + have, run: () => { note("action", "Older Seamless Co-op: keep " + have); answer(false) } },
                            { key: "x", label: "Use " + thisOne, run: () => { note("action", "Older Seamless Co-op: use " + thisOne); answer(true) } },
                            { key: "b", label: "Back", run: () => { note("action", "Older Seamless Co-op: back (keep " + have + ")"); answer(false) } }] })
    }
    property int coopStops: 0
    function coopPasswordEmpty() { return !!seamlessMod && Turbine.coopPasswordState(seamlessMod.id) === 0 }
    function startCoop() {
        if (!game || !seamlessMod) return
        // Two copies in the list: the one sheet first (Tidy up or Not now), then this start goes on.
        if (coopCopies().length > 0) { openCoopTidy(() => startCoop()); return }
        if (coopPasswordEmpty()) { coopStops++; note("action", "Play Seamless Co-op: no session password, its settings open first"); openCoopPasswordFirst(); return }
        note("action", "Play Seamless Co-op")
        if (!seamlessMod.enabled) { Turbine.setEnabled(seamlessMod.id, true); mods = mods.map(x => x.id === seamlessMod.id ? Object.assign({}, x, { enabled: true }) : x) }
        startPlay(true)
    }
    property bool playCoopNext: false
    // ---- With Seamless Co-op installed, Play (Start or a tap) asks one small question: Play with mods, Play
    // Seamless Co-op, Play without mods, and Co-op settings (Y) with whether the password is set. The last choice is
    // remembered and has the ring, so A starts it at once; B goes back.
    readonly property string coopOnlyWords: "Seamless Co-op only loads in Co-op mode"
    readonly property string lastPlayChoice: game ? (parse(seen.playChoice, {})[game.id] || "mods") : "mods"
    property string coopPassword: "?"     // "set", "not set" or "?" (not read yet)
    function rememberPlay(k) { if (!game) return; let c = parse(seen.playChoice, {}); c[game.id] = k; seen.playChoice = JSON.stringify(c) }
    function playPressed() {
        if (!game) return
        if (!seamlessIn || acRefused) { startPlay(); return }
        readCoopPassword()
        const keys = ["mods", "coop", "plain"]
        // Said where the choice is made: Seamless Co-op loads only with Play Seamless Co-op.
        openPick("Play " + gameTitle, coopOnlyWords + ". A plays the highlighted one.", [
            { icon: "play", title: "Play with mods", detail: "Solo, " + plural(mods.filter(m => m.enabled && !isSeamless(m)).length, "mod", "mods") + " on. Seamless Co-op stays out.", key: "mods", run: () => { rememberPlay("mods"); startPlay() } },
            { icon: "play", title: "Play Seamless Co-op", detail: "With friends, and your mods. Everyone uses the same password and settings.", key: "coop", run: () => { rememberPlay("coop"); startCoopChecked() } },
            { icon: "play", title: "Play without mods", detail: "Your mods stay installed", key: "plain", run: () => { rememberPlay("plain"); Turbine.playOriginal() } },
            { icon: "info", title: "Co-op settings", detail: "Password: " + (coopPassword === "?" ? "not read yet" : coopPassword), key: "settings", aLabel: "Open", run: () => openCoopSettings() }
        ], "")
        pickY = { label: "Co-op settings", run: () => openCoopSettings() }
        menuIndex = Math.max(0, keys.indexOf(lastPlayChoice))
        note("action", "Play choice: " + pickItems.map(x => x.title).join("; ") + " | focus " + (pickItems[menuIndex] ? pickItems[menuIndex].title : ""))
    }
    function coopSettingsFile() {
        if (!seamlessMod) return null
        return settingsFilesOf(seamlessMod.id).find(x => /ersc_settings\.ini$/i.test(x.rel)) || null
    }
    // The password, read from Turbine's copy of ersc_settings.ini (the answer comes back through settingsLoaded).
    function readCoopPassword() {
        const f = coopSettingsFile()
        if (!f) { coopPassword = "?"; return }
        settingsWant = { mod: seamlessMod.id, rel: f.rel, title: "Seamless Co-op settings", pin: ["password"], back: "", peek: true }
        Turbine.settingsLoad(seamlessMod.id, f.rel)
    }
    // Co-op Play: the session password first. Empty: its settings open on the password, and Play goes on after Save.
    property bool coopAfterSettings: false
    function startCoopChecked() { startCoop() }
    // Co-op settings on the password, with Steam's keyboard up; Play goes on after it is set (coopGoOn).
    function openCoopPasswordFirst() {
        const f = coopSettingsFile()
        if (!f) { toast.problem("Seamless Co-op's settings file isn't there. Install Seamless Co-op again from the game menu."); return }
        if (sheet !== "" && sheet !== "settingsedit") sheet = ""
        settingsWant = { mod: seamlessMod.id, rel: f.rel, title: "Seamless Co-op settings", pin: ["password"], back: "", coop: true }
        Turbine.settingsLoad(seamlessMod.id, f.rel)
    }
    // A password as the mod reads it: no spaces round it, and a pair of quotes with nothing in it is empty.
    function plainPassword(v) {
        let t = String(v || "").trim()
        if (t.length >= 2 && ((t[0] === '"' && t[t.length - 1] === '"') || (t[0] === "'" && t[t.length - 1] === "'"))) t = t.substring(1, t.length - 1).trim()
        return t
    }
    function coopPasswordOf(d) {
        const pw = (d.fields || []).find(x => /password/i.test(x.key || ""))
        return pw ? plainPassword(pw.value) : ""
    }
    // The password row gets the focus and Steam's keyboard opens on it.
    function coopTypePassword() {
        const i = settingsView.shown.findIndex(r => r.f && /password/i.test(r.f.key || ""))
        if (i < 0) { note("action", "Co-op password: no password row on the page"); return }
        settingsView.focusIndex = i
        const row = settingsView.shown[i]
        note("action", "Co-op password: the keyboard opens on " + row.f.key)
        settingsTypeRow = row
        openAsk("settingvalue", "Session password", "Every co-op player types the same one.", String(settingsView.valueOf(row.f) || ""), "Session password", 2000, false, "Done")
    }
    // Play after the check: with mods, or Seamless Co-op when that was asked.
    function playNow() {
        if (!game) return
        if (playCoopNext) { playCoopNext = false; Turbine.playCoop() }
        else Turbine.play(game.id)
    }
    // A me3 game's Steam Launch Options don't run Turbine's helper yet. In Game Mode Steam shows a game only when
    // Steam started it, so the modded launch goes through Steam: the one-time step, with the exact text to copy.
    function openLaunchSetup(o) {
        note("action", "Launch Options step for " + o.name + ": needs \"" + o.option + "\", set \"" + o.set + "\"")
        if (o.undone === true) {
            openMsg({ title: "Use the launch options line again?", text: "You undid the launch options change for " + o.name + ", so Turbine doesn't start it with mods through Steam. Use the line again to play with mods, or play without them.",
                      buttons: [{ key: "a", label: "Use it again", run: () => { Turbine.launchRedo(o.game); note("action", "Launch Options: used again"); if (o.coop === true) startCoop(); else startPlay() } },
                                { key: "x", label: "Play without mods", run: () => Turbine.playOriginal() },
                                { key: "b", label: "Not now", run: () => {} }] })
            return
        }
        // Steam starts the game through Turbine's own Steam tool, "Turbine (mods)", picked once in the game's
        // Properties. One sheet: the steps in plain words (the engine's, Turbine.steamToolJson), Check again (A),
        // and Set it up myself (the Launch Options line to paste) as the other way. In Game Mode Turbine never
        // restarts Steam to write Steam's settings (Steam writes its own copy back); Desktop Mode offers that way.
        const tool = steamTool(o.game)
        if (tool.steps.length > 0) { openToolSteps(o, tool); return }
        if (Turbine.gamemode) { openLaunchPaste(o); return }
        // No Steam tool to offer here (Steam's folder wasn't found): the Desktop Mode way.
        askSteamRestart({ game: o.game, kind: "options", resume: o.coop === true ? "coop" : "play", name: o.name,
                          title: o.outdated === true ? "Update " + o.name + "'s launch setting" : "One step before " + o.name + " plays with mods",
                          intro: "In Game Mode a game shows only when Steam starts it, so Turbine starts " + o.name + " through Steam. Steam needs one setting for that, and Turbine can set it for you."
                                 // The player's own Launch Options step aside while mods are on; say which, and how they come back.
                                 + (o.set && o.outdated !== true ? " Your Launch Options now (" + o.set + ") step aside while mods are on; Settings, Help, Undo the launch options change brings them back." : ""),
                          aLabel: "Restart Steam and set it up", manualLabel: "Set it up myself", manual: () => openLaunchPaste(o) })
    }
    // (toolTest: the build's pictures put a made-up answer here to see it shown.)
    property var toolTest: null
    function steamTool(gameId) {
        if (toolTest) return Object.assign({ installed: true, listed: true, chosen: true, route: "tool", steps: [], restartFirst: false, proton: "", protonName: "", lastProton: "", nothingStarted: "", using: [], toolName: "Turbine (mods)", tile: "", tileWords: "" }, toolTest)
        return parse(Turbine.steamToolJson(gameId), { installed: false, listed: false, chosen: false, route: "none", steps: [], restartFirst: true, proton: "", protonName: "",
                                                      lastProton: "", nothingStarted: "", using: [], toolName: "Turbine (mods)", tile: "", tileWords: "" })
    }
    property int toolChecks: 0
    // ---- "Proton for modded play": the Proton Turbine's Steam tool runs the game with (Turbine.protonJson:
    // {name, dir, how, words, auto, list: [{name, dir, kind, usable, current}]}). A row in the game menu (This
    // game) and on the tool steps sheet; it opens the list: Automatic first, then every Proton installed, the ones
    // the tool can't start dimmed with the reason. A pick is kept (Turbine.protonSet; "" is Automatic).
    readonly property var protonNow: {
        steamTick
        if (!game || acMode !== "me3" || game.id.indexOf("steam:") !== 0) return null
        const p = parse(Turbine.protonJson(game.id), null)
        return p && (p.list || []).length > 0 ? p : null
    }
    function protonTitle(p) { return "Proton for modded play: " + (!p ? "" : p.how === "you" ? p.name : "Automatic" + (p.name ? " (" + p.name + ")" : "")) }
    readonly property var protonKinds: ({ stable: "", beta: "Beta", experimental: "Experimental", hotfix: "Hotfix", other: "Another tool's" })
    property int protonPicks: 0
    function openProtonPick(back) {
        const p = game ? parse(Turbine.protonJson(game.id), null) : null
        if (!p) return
        const gid = game.id
        const pick = (dir, name) => {
            const r = parse(Turbine.protonSet(gid, dir), { ok: false, words: "Turbine couldn't keep that choice." })
            note("action", "Proton for modded play: " + (name || "Automatic") + (r.ok ? "" : " refused: " + r.words))
            protonPicks++; steamTick++
            if (r.ok) toast.done(r.words); else toast.problem(r.words)
            // Opened from the tool's steps: back to them, with the new name.
            if (back === "msg" && msgData) { msgData = Object.assign({}, msgData, { footnote: protonNow ? protonTitle(protonNow) + " (" + protonNow.words + ")." : "", buttons: msgData.buttons.map(b => b.proton ? Object.assign({}, b, { label: protonNow ? protonTitle(protonNow).replace(" for modded play", "") : b.label }) : b) }); infoIndex = 0; sheet = "msg" }
        }
        note("action", "Proton for modded play: " + (p.name || "none") + " (" + p.how + "), " + p.list.length + " installed")
        openPick("Proton for modded play", gameTitle + ": " + (p.how === "you" ? p.name + ", your pick" : (p.name || "none yet") + ", " + p.words), [
            { icon: "restore", title: "Automatic", detail: p.auto ? "Turbine chooses: " + p.auto + " now" : "Turbine chooses", aLabel: "Pick", run: () => pick("", "") }
        ].concat(p.list.map(x => ({
            icon: "play", title: x.name,
            detail: !x.usable ? "Can't be used: the Steam Linux Runtime it needs isn't installed" : [protonKinds[x.kind] || "", x.current ? "In use now" : ""].filter(s => s !== "").join(" · "),
            dim: !x.usable, info: !x.usable, aLabel: "Pick", run: () => pick(x.dir, x.name) }))), back || "")
        menuIndex = p.how === "you" ? Math.max(0, 1 + p.list.findIndex(x => x.current)) : 0
    }
    // For the launch check and Why won't it start?: what Turbine's Steam tool did at the last launch, when it
    // said: that it started nothing (and why), and which Proton it ran the game with (and why that one).
    function toolRows(gameId) {
        const t = steamTool(gameId)
        let rows = []
        if (t.nothingStarted) rows.push({ icon: "info", title: "Turbine's Steam tool started nothing", detail: t.nothingStarted, info: true, run: () => {} })
        if (t.lastProton) rows.push({ icon: "info", title: "Proton at the last start", detail: t.lastProton, info: true, run: () => {} })
        return rows
    }
    function openToolSteps(o, tool) {
        const name = tool.toolName || "Turbine (mods)"
        note("action", "Steam tool steps for " + o.name + ": " + tool.steps.length + " steps, " + (tool.restartFirst ? "Steam has to start again first" : "Steam lists the tool") + ", route " + tool.route)
        openMsg({ title: "One step before " + o.name + " plays with mods",
                  text: "Steam starts " + o.name + " with your mods through Turbine's own Steam tool, \"" + name + "\". Pick it once in Steam; Turbine sees it when it is set."
                        + (tool.restartFirst ? "\n\nSteam shows \"" + name + "\" in its list after Steam's next start." : "")
                        + (o.set ? "\n\nYour Launch Options (" + o.set + ") stay as they are." : ""),
                  steps: tool.steps,
                  footnote: protonNow ? protonTitle(protonNow) + " (" + protonNow.words + ")." : "",
                  buttons: [
            // Check again keeps the steps up until Steam has it; then the Play that asked goes on.
            { key: "a", label: "Check again", keep: true, run: () => {
                const now = steamTool(o.game)
                toolChecks++
                note("action", "Steam tool: check again: " + (now.route === "tool" ? "picked" : now.chosen ? "picked, not usable yet" : "not picked yet") + (now.restartFirst ? ", Steam has to start again first" : ""))
                if (now.route === "tool" || now.route === "options") {
                    sheet = ""; steamTick++
                    toast.done(now.route === "tool" ? "Steam uses " + name + " for " + o.name + "." : o.name + " is set up.")
                    if (o.coop === true) startCoop(); else startPlay()
                    return
                }
                msgData = Object.assign({}, msgData, { steps: now.steps,
                    text: msgData.text.split("\n\n")[0] + (now.restartFirst ? "\n\nSteam shows \"" + name + "\" in its list after Steam's next start." : "") })
                toast.info(now.restartFirst ? "Steam has to restart before it lists " + name + "." : "Not picked yet. The steps are still here.") } },
            { key: "x", label: "Set it up myself", run: () => openLaunchPaste(o) }
            ].concat(protonNow ? [
            // The Proton the tool runs the game with: its button opens the list (B there comes back here).
            { key: "y", label: protonTitle(protonNow).replace(" for modded play", ""), keep: true, proton: true, run: () => openProtonPick("msg") }
            ] : Turbine.gamemode ? [] : [
            // Desktop Mode only: Turbine sets the Launch Options line itself with one Steam restart.
            { key: "y", label: "Restart Steam and set it up", run: () => runSteamRestart({ game: o.game, kind: "options", resume: o.coop === true ? "coop" : "play", name: o.name, manualLabel: "Set it up myself", manual: () => openLaunchPaste(o) }) }
            ]).concat([{ key: "b", label: "Not now", run: () => note("action", "Steam tool steps: not now") }]) })
    }
    // The paste steps (Set it up myself): the exact text to copy into Steam's Launch Options.
    function openLaunchPaste(o) {
        note("action", "Launch Options step: set it up myself")
        const steps = ["A copies the text above.",
                       "In Steam: " + o.name + ", the gear, Properties, General, Launch Options.",
                       o.outdated === true ? "Replace Turbine's older line there with this one (it starts the game normally if Turbine is ever removed)." : o.set !== "" ? "Replace what is there now (" + o.set + ") with it: paste, or type it." : "Paste it there, or type it.",
                       "Close Properties, come back to Turbine and press Play.",
                       "Steam's own Play button then starts " + o.name + " with your mods too. With every mod off it starts normally."]
        openMsg({ title: o.outdated === true ? "Update " + o.name + "'s launch options line" : "One step before " + o.name + " plays with mods",
                  text: "In Game Mode a game shows only when Steam starts it, so Turbine starts " + o.name + " through Steam. Steam needs this in the game's Launch Options, once:\n\n" + o.option,
                  steps: steps,
                  buttons: [
            // Copy keeps the steps on screen; the button says it's copied. B closes.
            { key: "a", label: "Copy it", keep: true, run: () => {
                Turbine.copyText(o.option); note("action", "Launch option copied: " + o.option)
                msgData = Object.assign({}, msgData, { buttons: msgData.buttons.map(x => x.key === "a" ? Object.assign({}, x, { label: "Copied (copy again)" }) : x) })
                toast.done("Copied. The steps stay here until B.") } },
            { key: "x", label: "Play without mods", run: () => { note("action", "Launch Options step: played without mods"); Turbine.playOriginal() } },
            { key: "b", label: "Not now", run: () => note("action", "Launch Options step: not now") }
            // Y is only ever Start outside Steam, and only in Desktop Mode (the activity log has its own place).
            ].concat(Turbine.gamemode ? [] : [{ key: "y", label: "Start outside Steam", run: () => {
                note("action", "Launch Options step: started outside Steam (Desktop Mode)"); Turbine.playOutsideSteam(o.coop === true) } }]) })
    }
    // ---------------------------------------------------------------- Edit settings
    property string settingsMod: ""          // the mod's id; "" for the game's own files
    property string settingsRel: ""
    property string settingsReturn: ""         // the sheet B goes back to
    property var settingsTypeRow: null
    // The file is read on its own thread (a big one never stops the interface); the page opens when it comes.
    property var settingsWant: null
    function openSettingsFile(modId, rel, title, pin, back) {
        settingsWant = { mod: modId, rel: rel, title: title, pin: pin || [], back: back || "" }
        Turbine.settingsLoad(modId, rel)
    }
    function settingsLoaded(d) {
        const w = settingsWant
        if (!w || d.mod !== w.mod || d.rel !== w.rel) return
        settingsWant = null
        if (d.error) { toast.problem(d.error); return }
        // The Play choice's password line (nothing opens).
        if (w.peek) {
            coopPassword = coopPasswordOf(d) !== "" ? "set" : "not set"
            if (sheet === "pick") pickItems = pickItems.map(x => x.key === "settings" ? Object.assign({}, x, { detail: "Password: " + coopPassword }) : x)
            note("action", "Co-op password: " + coopPassword)
            return
        }
        if (w.coop) {
            // Set meanwhile (the copy the game loads has one): on with the start.
            if (!coopPasswordEmpty()) { note("action", "Co-op password set: playing"); startCoop(); return }
            note("action", "Co-op password not set: its settings open on it")
            coopAfterSettings = true
            toast.info("Set the session password first. Every co-op player uses the same password and settings. Play goes on once it's set.")
        }
        settingsMod = w.mod; settingsRel = w.rel; settingsReturn = w.back
        settingsView.title = w.title
        settingsView.sub = w.rel + (d.edited ? " · your version (Reset to default goes back to " + (w.mod === "" ? "the original" : "the mod's own") + ")" : "")
                           + (d.big ? " · too big to edit here as settings, so it's shown as plain text" : d.fields.length > 0 ? " · " + plural(d.fields.length, "setting", "settings") : "")
        settingsView.pinFirst = w.pin
        settingsView.doc = d
        settingsView.reset()
        // After a mod update, the settings Turbine couldn't carry over for sure come first, with the words.
        if (d.review) {
            settingsView.sub = "Check again after the update: " + (d.reviewNames || []).join(", ") + " · " + settingsView.sub
            toast.info(d.review)
            note("action", "Edit settings: settings to check after the mod's update: " + (d.reviewNames || []).join(", "))
        }
        note("action", "Edit settings: " + w.title + " (" + w.rel + ", " + d.format + ", " + d.fields.length + " settings" + (d.edited ? ", edited" : "") + (d.big ? ", too big: plain text" : "") + ")")
        sheet = "settingsedit"
        if (w.coop && coopAfterSettings) coopTypePassword()
    }
    function settingsFilesOf(modId) { return parse(Turbine.settingsFiles(modId), []) }
    // X on a mod, Edit settings: one file opens straight away; several are listed first.
    function openModSettings(m) {
        const files = settingsFilesOf(m.id)
        if (files.length === 0) { toast.info(m.name + " has no settings file Turbine can edit."); return }
        const coop = isSeamless(m)
        if (files.length === 1 || coop) {
            const f = coop ? (files.find(x => /ersc_settings\.ini$/i.test(x.rel)) || files[0]) : files[0]
            openSettingsFile(m.id, f.rel, coop ? "Seamless Co-op settings" : "Settings: " + m.name, coop ? ["password"] : [], "")
            return
        }
        openPick("Settings: " + m.name, plural(files.length, "settings file", "settings files"), files.map(f => ({
            icon: "info", title: f.rel.split("/").pop(), detail: f.rel + (f.edited ? " · your version" : "") + (f.review ? " · check after the update" : ""), aLabel: "Edit",
            run: () => openSettingsFile(m.id, f.rel, "Settings: " + m.name, [], "pick") })), "modmenu")
    }
    // Game menu, Edit settings: the game's own settings files.
    function openGameSettings() {
        const files = settingsFilesOf("")
        if (files.length === 0) { toast.info("Turbine found no settings files of " + gameTitle + "'s own to edit."); return }
        openPick("Edit " + gameTitle + "'s settings", "Its own settings files. Turbine keeps the original and the copy before each save.", files.map(f => ({
            icon: "info", title: f.rel.split("/").pop(), detail: f.rel + (f.edited ? " · changed by you" : ""), aLabel: "Edit",
            run: () => openSettingsFile("", f.rel, gameTitle + ": " + f.rel.split("/").pop(), [], "") })), "gamemenu")
    }
    function isSeamless(m) { return !!m && m.seamless === true }
    function openCoopSettings() {
        if (!seamlessMod) { openMsg({ title: "Seamless Co-op isn't installed", text: "Get it first: Turbine downloads it from its official release and checks it.", steps: [], notes: [], footnote: "",
                                      buttons: [{ key: "a", label: "Get Seamless Co-op", run: () => { const i = gameMenuItems.findIndex(x => x.act === "seamless"); if (i >= 0) gameMenuRun(i) } }, { key: "b", label: "Not now", run: () => {} }] }); return }
        openModSettings(seamlessMod)
    }
    function settingsTyped(row, text) {
        settingsTypeRow = null
        if (row.kind === "line") { settingsView.setLine(row.n, text); return }
        const f = row.f
        if (f.kind === "number") {
            text = String(text).trim()
            const v = Number(text)
            if (!/^-?\d+(\.\d+)?$/.test(text) || !isFinite(v)) { toast.problem(f.key + " takes a number, like 5 or 1.5."); return }
            if (f.min !== null && v < f.min || f.max !== null && v > f.max) { toast.problem(f.key + " goes from " + f.min + " to " + f.max + "."); return }
        }
        settingsView.setValue(f, text)
    }
    // Every change saves itself a moment after it is made (the copy before it is kept, Reset to default
    // goes back), like everything else in Turbine: nothing to remember to save, and B never loses a change.
    Timer { id: settingsAutosave; interval: 700; onTriggered: if (app.sheet === "settingsedit" && settingsView.changes > 0) app.settingsSaveNow(true) }
    Connections { target: settingsView; function onChangesChanged() { if (settingsView.changes > 0 && app.sheet === "settingsedit") settingsAutosave.restart() } }
    function settingsSaveNow(quiet) {
        settingsAutosave.stop()
        if (settingsView.changes === 0) { if (quiet !== true) toast.info("Already saved: changes save automatically."); if (coopAfterSettings) coopGoOn(); return }
        const e = settingsView.edits, le = settingsView.lineEdits
        // Each setting by its identity and the value the page showed (the file may have changed since).
        const byId = id => settingsView.fields.find(f => f.id === parseInt(id))
        const body = { fields: Object.keys(e).map(id => { const f = byId(id); return { section: f.section, key: f.key, occ: f.occ || 0, was: f.value, value: e[id] } }),
                       lines: Object.keys(le).map(n => ({ line: parseInt(n), was: settingsView.lines[parseInt(n) - 1], text: le[n] })) }
        const r = parse(Turbine.settingsSave(settingsMod, settingsRel, JSON.stringify(body)), { ok: false, error: "Couldn't save it." })
        if (!r.ok) { toast.problem(r.error); return }
        note("action", "Edit settings saved: " + settingsRel + " (" + settingsView.changes + " changes)")
        if (settingsMod !== "") Turbine.applyNow()
        toast.done("Saved")
        if (quiet === true && Object.keys(le).length === 0) {
            // The page stays where it is: the saved values become the page's own.
            const d = settingsView.doc
            settingsView.doc = Object.assign({}, d, { edited: true, fields: d.fields.map(f => e[f.id] !== undefined ? Object.assign({}, f, { value: e[f.id] }) : f) })
            settingsView.edits = ({})
        } else if (!settingsLeaving) openSettingsFile(settingsMod, settingsRel, settingsView.title, settingsView.pinFirst, settingsReturn)
        if (coopAfterSettings && !settingsLeaving) coopGoOn()
    }
    // A co-op Play waiting for the password: on once it is set.
    property bool settingsLeaving: false
    function coopGoOn() {
        const pw = settingsView.fields.find(x => /password/i.test(x.key || ""))
        const v = pw ? plainPassword(settingsView.valueOf(pw)) : ""
        if (v === "") return
        // Saved for real (the file Turbine keeps): otherwise the page stays, and says so.
        if (coopPasswordEmpty()) { note("action", "Co-op password typed but not in the file yet"); toast.problem("The password didn't save. Try once more."); return }
        coopAfterSettings = false
        note("action", "Co-op password set: playing")
        sheet = ""
        startCoop()
    }
    function settingsResetAsk() {
        const title = settingsView.title, mod = settingsMod, rel = settingsRel, pin = settingsView.pinFirst, back = settingsReturn
        openMsg({ title: "Reset to default?", text: (mod === "" ? "Puts back " + rel + " as it was before you first changed it." : "Puts back the mod's own " + rel + ".") + " Your version is kept as a backup file.",
                  buttons: [{ key: "a", label: "Reset", run: () => {
                      const r = parse(Turbine.settingsReset(mod, rel), { ok: false, error: "Couldn't reset it." })
                      if (!r.ok) { toast.problem(r.error); return }
                      if (mod !== "") Turbine.applyNow()
                      toast.done("Back to the default.")
                      openSettingsFile(mod, rel, title, pin, back) } },
                            { key: "b", label: "Cancel", run: () => { sheet = "settingsedit" } }] })
    }
    // ---------------------------------------------------------------- One Steam restart, asked first
    // Steam's own settings (a game's launch setting, the co-op tile) change only while Steam is closed. Turbine asks
    // once, with the warning, then a small helper on the device closes Steam, makes the change and starts Steam
    // again; in Game Mode Turbine closes with Steam and comes back through its Steam entry, then goes on.
    property int steamTick: 0                  // bumps when Steam's settings changed (the game menu reads them again)
    property real steamSetupSince: 0
    function askSteamRestart(o) {
        // Never in Game Mode (Steam starts again at once there and writes its own copy back, so the change
        // was lost): the do-it-myself steps instead, or plain words when there are none.
        if (Turbine.gamemode) {
            note("action", "No Steam restart in Game Mode for " + o.name + " (" + o.kind + ")" + (o.manual ? ": the steps to do it by hand" : ""))
            if (o.manual) { o.manual(); return }
            openMsg({ title: o.title, text: o.intro + "\n\nSteam's own settings can't be changed from Game Mode. Open Turbine in Desktop Mode for this.", buttons: [{ key: "b", label: "Close", run: () => {} }] })
            return
        }
        note("action", "Steam restart asked for " + o.name + " (" + o.kind + ")")
        openMsg({ title: o.title,
                  text: o.intro,
                  notes: ["Turbine restarts Steam once to finish setting up " + o.name + ".", "Steam closes and opens again: any game or download in progress stops.", "Save your game first if one is running."],
                  buttons: [{ key: "a", label: o.aLabel, run: () => runSteamRestart(o) }]
                           .concat(o.manual ? [{ key: "x", label: o.manualLabel, run: o.manual }] : [])
                           .concat([{ key: "b", label: "Not now", run: () => note("action", "Steam restart: not now") }]) })
    }
    function runSteamRestart(o) {
        const r = parse(Turbine.steamSetupStart(o.game, o.kind, o.resume), { ok: false, state: "", words: "Turbine couldn't start the restart." })
        note("action", "Steam restart for " + o.name + ": " + (r.ok ? r.state : "refused: " + r.words))
        if (!r.ok) {
            openMsg({ title: "Steam can't restart right now", text: r.words,
                      buttons: (o.manual ? [{ key: "x", label: o.manualLabel, run: o.manual }] : []).concat([{ key: "b", label: "Close", run: () => {} }]) })
            return
        }
        if (r.state === "done") { steamSetupResult(parse(Turbine.steamSetupState(), { state: "done", resume: o.resume === "" ? "" : "play " + o.game + " " + (o.resume === "coop" ? "coop" : "solo") })); return }
        steamSetupSince = Date.now()
        steamSetupPoll.start()
        toast.info(Turbine.gamemode ? "Restarting Steam. Turbine closes with it and comes back when Steam is ready." : "Restarting Steam. Turbine goes on when Steam is back.")
    }
    Timer {
        id: steamSetupPoll; interval: 1000; repeat: true
        onTriggered: {
            const s = app.parse(Turbine.steamSetupState(), { state: "none" })
            if (s.state === "running" && Date.now() - app.steamSetupSince < 240000) return
            stop()
            if (s.state === "running") { app.note("action", "Steam restart: no result after 4 minutes"); s.state = "failed"; s.words = "The restart didn't finish. If Steam is closed, start it again, then try once more." }
            app.steamSetupResult(s)
        }
    }
    function steamSetupResult(s) {
        Turbine.steamSetupFinish()
        steamTick++
        note("action", "Steam restart: " + s.state + (s.words ? " (" + s.words + ")" : "") + (s.resume ? ", then " + s.resume : ""))
        if (s.state !== "done") {
            openMsg({ title: "Steam's setting wasn't changed", text: (s.words || "Something went wrong during the restart.") + " The activity log has each step.",
                      buttons: [{ key: "y", label: "Activity log", run: () => openActivity() }, { key: "b", label: "Close", run: () => {} }] })
            return
        }
        toast.done("Done: " + (s.what || "Steam's setting") + " is set.")
        // Go on with the Play that asked for it (once this game's mods are read).
        const parts = String(s.resume || "").split(" ")
        if (parts[0] === "play" && parts[1]) quickRequest("turbine:quick " + (parts[2] === "coop" ? "play-coop" : "play") + " " + encodeURIComponent(parts[1]), 0)
    }
    // At start: a restart that finished while Turbine was closed (Game Mode), and changes Steam wrote over.
    function steamSetupAtStart() {
        const s = parse(Turbine.steamSetupState(), { state: "none" })
        if (s.state === "running" && s.age < 300) { steamSetupSince = Date.now() - s.age * 1000; steamSetupPoll.start() }
        else if ((s.state === "done" || s.state === "failed") && s.age < 900) steamSetupResult(s)
        else if (s.state !== "none") { note("action", "Steam restart from " + s.age + " s ago: left (" + s.state + ")"); Turbine.steamSetupFinish() }
        const lost = parse(Turbine.steamChangesCheck(), { lost: [] }).lost
        if (lost.length > 0) toast.info("Steam put back its own settings for " + lost.map(n => fullGame(n)).join(", ") + " (it was still running). Play or the game menu sets them up again.")
    }
    // The co-op tile in Steam's library (with Seamless Co-op installed).
    readonly property bool coopTileThere: steamTick >= 0 && !!game && seamlessIn && parse(Turbine.coopTileJson(game.id), { there: false }).there === true
    // ---- How the tile can be added here and now is the engine's call (Turbine.coopTileOfferJson(game id),
    // {"how": "restart" | "direct" | "later" | "none", "words": what to tell the player, "label": the button}):
    // "direct" in Desktop Mode with Steam closed (Turbine.coopTileSet(game id, add) does it at once and answers
    // {"ok", "words"}), "restart" in Desktop Mode with Steam running (one restart, asked first), "later" in Game
    // Mode (words only, with OK: the tile is added in Desktop Mode), "none" for a game with no such tile (the rows
    // go).
    readonly property var coopTileOffer: {
        steamTick
        if (!game || !seamlessIn) return { how: "none", words: "", label: "" }
        if (typeof Turbine.coopTileOfferJson !== "function") return { how: "restart", words: "", label: "" }
        return parse(Turbine.coopTileOfferJson(game.id), { how: "restart", words: "", label: "" })
    }
    readonly property bool coopTileOffered: coopTileOffer.how !== "none" || coopTileThere
    readonly property string coopTileHow: coopTileOffer.how === "restart" ? "one Steam restart" : coopTileOffer.how === "direct" ? "no restart" : ""
    // Asked once, right after Seamless Co-op is installed: "Add a Co-op tile to Steam?" One press adds it (the
    // restart is said first); Not now leaves it for the game menu and the Co-op settings page.
    property string seamlessWanted: ""    // the game Get Seamless Co-op was pressed for
    property int coopTileAsks: 0
    // Get Seamless Co-op goes to its Nexus page first (eldenring mod 510, the engine's
    // anticheat::SEAMLESS_NEXUS: the newest version is there first), right in Turbine, where Slow download adds
    // it. Turbine's own download from the mod's GitHub release is the other way: when Nexus can't be opened in
    // Turbine or the player isn't signed in there. With a copy installed Turbine only looks whether a newer one
    // is out: as new or newer installed, it says so and does nothing.
    readonly property string seamlessNexusUrl: "https://www.nexusmods.com/eldenring/mods/510?tab=files"
    function seamlessGetPlan() {
        if (seamlessIn) return "check"
        const outside = !Turbine.gamemode && browserSet.openIn === "mine"
        return browserSignedIn && browserLive && !outside ? "nexus" : "github"
    }
    function installSeamless() {
        if (!game) return
        seamlessWanted = game.id
        signinRefresh()
        const plan = seamlessGetPlan()
        note("action", "Get Seamless Co-op: " + (plan === "nexus" ? "its Nexus page in Turbine" : plan === "check" ? "installed: looking for a newer one" : "Turbine's own download (" + (browserSignedIn ? "Nexus doesn't open in Turbine here" : "not signed in on Nexus") + ")"))
        if (plan === "nexus") {
            toast.info("Seamless Co-op on Nexus: press Slow download on its newest file. Turbine adds it.")
            if (openNexus(seamlessNexusUrl, "getit")) return
        }
        Turbine.installSeamless()
    }
    onSeamlessInChanged: {
        if (seamlessIn && game && seamlessWanted === game.id) { seamlessWanted = ""; coopTileAskTimer.tries = 0; coopTileAskTimer.restart() }
        // On opening a game with Seamless Co-op (and when it arrives): are there two copies?
        if (seamlessIn) Qt.callLater(coopLook); else coopFound = null
    }
    // Another mod came or went while Seamless Co-op is in: looked at again.
    onModsChanged: if (seamlessIn && modsLoaded && game && (mods.length !== coopModsSeen || !coopFound || coopFound.game !== game.id) && coopModsSeen >= 0) Qt.callLater(coopLook)
    Timer {
        id: coopTileAskTimer
        property int tries: 0
        interval: 1200
        // Once nothing else is on screen (the install's own toast and any sheet come first).
        onTriggered: { if (app.sheet !== "" || app.wheelOpen || app.page !== "game" || Turbine.busy) { if (++tries < 40) restart(); return } app.maybeAskCoopTile() }
    }
    function maybeAskCoopTile() {
        if (!game || !seamlessIn || coopTileThere) return false
        const asked = parse(seen.coopTileAsked, [])
        if (asked.indexOf(game.id) >= 0) return false
        const how = coopTileOffer.how
        // Nothing to press here: no question (the rows say where it can be added).
        if (how !== "restart" && how !== "direct") { note("action", "Co-op tile: not asked (" + how + ")"); return false }
        asked.push(game.id); seen.coopTileAsked = JSON.stringify(asked)
        coopTileAsks++
        note("action", "Co-op tile: asked once for " + game.name + " (" + how + ")")
        openCoopTile()
        return true
    }
    function openCoopTile() {
        if (!game) return
        const add = !coopTileThere
        const offer = coopTileOffer
        if (offer.how === "later" || offer.how === "none") {
            openMsg({ title: add ? "Add a Co-op tile to Steam" : "Remove the Co-op tile from Steam", text: offer.words || "The Co-op tile can't be changed from here.", buttons: [{ key: "b", label: "Close", run: () => {} }] })
            return
        }
        if (offer.how === "direct") {
            const tileIntro = add ? "A tile in your Steam library that starts " + gameTitle + " with Seamless Co-op in one press: Turbine gets the mods ready and Steam starts the game as usual."
                                  : "Takes the " + gameTitle + " Co-op tile and its pictures out of your Steam library."
            openMsg({ title: add ? "Add a Co-op tile to Steam?" : "Remove the Co-op tile from Steam?", text: tileIntro + (offer.words ? "\n\n" + offer.words : ""),
                      buttons: [{ key: "a", label: offer.label || (add ? "Add it" : "Remove it"), run: () => {
                                      const r = parse(Turbine.coopTileSet(game.id, add), { ok: false, words: "Turbine couldn't change the Co-op tile." })
                                      note("action", "Co-op tile " + (add ? "add" : "remove") + ": " + (r.ok ? "done" : "refused: " + r.words))
                                      steamTick++
                                      if (r.ok) toast.done(r.words || (add ? "Co-op tile added to your Steam library." : "Co-op tile removed.")); else toast.problem(r.words) } },
                                { key: "b", label: "Not now", run: () => note("action", "Co-op tile: not now") }] })
            return
        }
        askSteamRestart({ game: game.id, kind: add ? "tile" : "untile", resume: "", name: gameTitle,
                          title: add ? "Add a Co-op tile to Steam" : "Remove the Co-op tile from Steam",
                          intro: add ? "A tile in your Steam library that starts " + gameTitle + " with Seamless Co-op in one press: Turbine gets the mods ready and Steam starts the game as usual."
                                     : "Takes the " + gameTitle + " Co-op tile and its pictures out of your Steam library.",
                          aLabel: add ? "Restart Steam and add it" : "Restart Steam and remove it" })
    }
    // ---------------------------------------------------------------- Undo the launch options change
    function openLaunchUndo(current) {
        const list = parse(Turbine.launchUndoJson(current), [])
        note("action", "Undo the launch options change: " + list.map(x => x.name + (x.uses ? " (uses it)" : "")).join("; "))
        if (list.length === 0) { toast.info("No game's Steam Launch Options run Turbine's helper."); return }
        if (list.length === 1) { showLaunchUndo(list[0]); return }
        openPick("Undo the launch options change", "Pick the game", list.map(x => ({ icon: "restore", title: x.name, detail: x.original !== "" ? "Back to: " + x.original : "Back to empty", aLabel: "Show", run: () => showLaunchUndo(x) })), "")
    }
    function showLaunchUndo(x) {
        // Turbine puts the old setting back itself with one Steam restart; doing it by hand is the fallback.
        askSteamRestart({ game: x.game, kind: "undo", resume: "", name: fullGame(x.name),
                          title: "Undo the launch options change: " + fullGame(x.name),
                          intro: "Turbine puts back " + fullGame(x.name) + "'s launch setting as it was before: " + (x.original !== "" ? x.original : "empty") + ". Steam then starts the game normally, with its anti-cheat.",
                          aLabel: "Restart Steam and put it back", manualLabel: "Do it myself", manual: () => showLaunchUndoPaste(x) })
    }
    function showLaunchUndoPaste(x) {
        Turbine.launchUndo(x.game)
        const back = x.original
        openMsg({ title: "Undo the launch options change: " + fullGame(x.name),
                  text: "Turbine's helper starts nothing for " + fullGame(x.name) + " now. To take its line out of Steam too, put back what was there before:\n\n" + (back !== "" ? back : "(nothing: leave Launch Options empty)"),
                  steps: ["In Steam: " + fullGame(x.name) + ", the gear, Properties, General, Launch Options.",
                          back !== "" ? "Replace Turbine's line with the text above (A copies it)." : "Delete everything there.",
                          "Close Properties. Steam starts the game normally, with its anti-cheat."],
                  buttons: (back !== "" ? [{ key: "a", label: "Copy it", run: () => { Turbine.copyText(back); toast.done("Copied. Paste it in Steam: Properties, Launch Options.") } }] : [])
                           .concat([{ key: "b", label: "Close", run: () => {} }]) })
    }
    // ---------------------------------------------------------------- Elden Ring's saves, each kind apart
    function openErSaves() {
        const v = parse(Turbine.erSavesJson(), { folders: 0, kinds: [] })
        note("action", "Saves by kind: " + v.kinds.map(k => k.kind + " " + k.files.length + " files, " + k.backups.length + " backups").join("; "))
        if (v.folders === 0) { toast.info("Turbine can't find " + gameTitle + "'s save folder yet. Play once, then look again."); return }
        openPick("Saves: normal, modded and co-op", "Kept apart so a modded or co-op save never goes online. A shows each one's backups.", v.kinds.map(k => ({
            icon: "restore", title: k.label, aLabel: "Backups",
            detail: (k.files.length === 0 ? "No save yet" : k.files.map(f => f.rel.split("/").pop() + " · " + when(f.modified)).join(", ")) + " · " + plural(k.backups.length, "backup", "backups"),
            run: () => openErSaveKind(k) })), "gamemenu")
    }
    function openErSaveKind(k) {
        if (k.backups.length === 0) { toast.info("No backups of the " + k.label.toLowerCase() + " save yet. One is made each time you play."); return }
        openPick(k.label + ": put back a backup", "Only this kind goes back; the others stay. The saves as they are now are kept first.", k.backups.map(b => ({
            icon: "restore", title: when(b.when), detail: plural(b.count, "file", "files"), aLabel: "Put back",
            run: () => { note("action", "Put back " + k.kind + " save from " + b.id); Turbine.restoreSaveKind(k.kind, b.id) } })), "gamemenu")
    }
    // ---------------------------------------------------------------- The game update warning
    property var gameUpdate: null
    function maybeGameUpdate() {
        if (!game || !gameUpdate || !gameUpdate.risky || gameUpdate.risky.length === 0 || page !== "game" || sheet !== "" || wheelOpen) return
        // An update still waiting in Steam warns once too (before it installs), then again once it installed.
        const waiting = !!gameUpdate.pending && gameUpdate.last === gameUpdate.now
        const key = game.id + "|" + gameUpdate.now + (waiting ? "|waiting" : "")
        if (Turbine.updateWarned(key)) return
        if (!shot.on) Turbine.markUpdateWarned(key)
        const names = gameUpdate.risky.map(r => r.name)
        note("action", "Game update warning: " + game.name + " build " + gameUpdate.last + " to " + gameUpdate.now + "; may break: " + names.join(", "))
        if (waiting) {
            openMsg({ title: "An update is waiting in Steam", text: gameUpdate.pending + " These mods hook into the game and often stop working until their makers update them: " + names.join(", ") + ".",
                      buttons: [{ key: "a", label: "Play without mods until they update", run: () => holdMods() },
                                { key: "b", label: "Keep my mods on", run: () => note("action", "Waiting update warning: kept the mods on") }] })
            return
        }
        openMsg({ title: gameTitle + " updated", text: game.store === "steam" || String(game.id).indexOf("steam:") === 0
                    ? "Steam updated " + gameTitle + " since you last played with mods. These mods hook into the game and often stop working until their makers update them: " + names.join(", ") + "."
                    : gameTitle + " updated since you last played with mods. These mods hook into the game and often stop working until their makers update them: " + names.join(", ") + ".",
                  buttons: [{ key: "a", label: "Play without mods until they update", run: () => holdMods() },
                            { key: "b", label: "Keep my mods on", run: () => note("action", "Game update warning: kept the mods on") }] })
    }
    // Play without mods until the mods that may break are updated (or Game menu, Play with mods again).
    readonly property var modHold: game ? (parse(seen.modHold, {})[game.id] || null) : null
    function holdMods() {
        if (!game || !gameUpdate) return
        let all = parse(seen.modHold, {})
        let versions = {}
        for (const r of gameUpdate.risky) { const m = mods.find(x => x.id === r.id); versions[r.id] = m ? (m.version || "") : "" }
        all[game.id] = { build: gameUpdate.now, versions: versions }
        seen.modHold = JSON.stringify(all)
        Turbine.setModHold(true)
        note("action", "Mods held until they update: " + Object.keys(versions).join(", "))
        toast.act("done", "Mods paused until they update.", "Play with them now", () => releaseHold("you asked"))
    }
    // The paused mods, from the status (a tap or A on it) or the game menu: one choice.
    function openHoldChoice() {
        openMsg({ title: "Mods paused: " + gameTitle + " updated", text: "Play starts " + gameTitle + " without mods until the mods that may break are updated.", steps: [], notes: [], footnote: "",
                  buttons: [{ key: "a", label: "Play with mods again", run: () => releaseHold("you asked") }, { key: "b", label: "Keep them paused", run: () => {} }] })
    }
    function releaseHold(why) {
        if (!game) return
        let all = parse(seen.modHold, {})
        if (!all[game.id]) return
        delete all[game.id]
        seen.modHold = JSON.stringify(all)
        Turbine.setModHold(false)
        note("action", "Mods back on for Play: " + why)
        toast.done("Play uses your mods again: " + why + ".")
    }
    // A held mod got an update (or went): Play uses the mods again.
    function checkModHold() {
        const h = modHold
        if (!h || !modsLoaded) return
        const moved = Object.keys(h.versions).filter(id => { const m = mods.find(x => x.id === id); return !m || (m.version || "") !== h.versions[id] })
        if (moved.length > 0) {
            const m = mods.find(x => x.id === moved[0])
            releaseHold(m ? m.name + " updated" : "a held mod was removed")
        }
    }
    // The launch check: the game didn't show about 20 seconds after Play.
    // Said only while it is true. A game that shows up later (a slow start) takes the sheet away, and one
    // that started and then closed is told as "<game> closed after N seconds", with the likely cause and its
    // buttons, never as "didn't start".
    property string notStartedFor: ""     // the game the "didn't start" sheet on screen is about
    function notStartedShowing(gameId) { return sheet === "pick" && notStartedFor === gameId && / didn't start$/.test(pickTitle) }
    function gameSeenLate(gameId, seconds) {
        const g = games.find(x => x.id === gameId)
        const name = g ? fullGame(g.name) : gameTitle
        note("launch", name + " showed up " + seconds + " seconds after Play" + (notStartedShowing(gameId) ? ": the didn't start sheet goes" : ""))
        if (notStartedShowing(gameId)) { sheet = ""; toast.info(name + " is running now. It took " + seconds + " seconds to start.") }
        notStartedFor = ""
    }
    function openNotStarted(gameId) {
        const g = games.find(x => x.id === gameId)
        const name = g ? fullGame(g.name) : gameTitle
        note("action", "Launch check: " + name + " didn't start")
        if (sheet !== "" && sheet !== "pick") return
        notStartedFor = gameId
        const me3 = acMode === "me3" && game && game.id === gameId
        let rows = toolRows(gameId)
        if (me3) {
            const h = healthNow(), lf = launchFinding(h)
            rows.push(lf ? lf : { icon: "power", title: "Steam's Launch Options: set", detail: "Steam has what " + name + " needs", info: true, run: () => {} })
        }
        rows.push({ icon: "play", title: "Play without mods", detail: "If it starts this way, a mod is the cause", run: () => Turbine.playOriginal() })
        rows.push({ icon: "info", title: "Activity log", detail: "What Turbine and Steam did for this launch", run: () => openActivity() })
        rows.push({ icon: "info", title: "Why won't it start?", detail: "Checks the usual reasons", run: () => openWhy() })
        openPick(name + " didn't start", "It still wasn't running 20 seconds after Play. Try these:", rows, "")
    }
    // My mods: the pinned "+ Get mods" row, the mods, then one collapsed row for Recently removed.
    // Mods that are mainly what other mods need (another installed mod lists it, or it is a known framework or
    // loader) go into one collapsed row at the bottom, "Dependencies (5)"; A opens it. X options move a mod in or
    // out. Only the list changes: the load order stays exactly as it is. Moving, Select several and Find show the
    // whole list.
    readonly property bool depsGrouped: !moving && !orderMoving && !selecting && findText === ""
    property bool depsOpen: false
    readonly property var depMoveMap: { seenRev; return game ? parse(Turbine.depMovesJson(game.id), {}) : {} }
    function isDep(m) { return listView.depAll.indexOf(m.id) >= 0 }
    function moveDep(m, into) {
        Turbine.moveDep(game.id, m.id, into)
        seenRev++
        note("action", m.name + (into ? ": moved into Dependencies" : ": moved out of Dependencies"))
        toast.done(into ? m.name + " is in Dependencies now." : m.name + " is with your mods now.")
        const i = listItems.findIndex(x => x.kind === "mod" && x.m.id === m.id)
        if (i >= 0) modIndex = i
    }
    property var coreIds: []
    // The list from the engine (modlist.rs): Find, Show, Sort and the Dependencies row, as places in mods.
    readonly property var listView: {
        const hidden = Object.keys(overlapsMap).filter(k => overlapsMap[k] && overlapsMap[k].hidden === true)
        const warned = Object.keys(modWarns).filter(k => !!modWarns[k])
        const v = parse(Turbine.modListJson(JSON.stringify({
            mods: mods.map(m => ({ id: m.id, name: m.name, enabled: m.enabled, added: m.added, needsLoader: !!m.needsLoader })), links: linksMap,
            find: findText, show: listShow, sort: listSort, kept: keptRow, grouped: depsGrouped, open: depsOpen, lastPlayedAt: lastPlayedAt,
            updates: Object.keys(updateMap), core: coreIds, moves: depMoveMap, loaderMissing: loaderMissing, hidden: hidden, warned: warned })), null)
        return v || { shown: [], deps: [], rows: [], depAll: [] }
    }
    readonly property var depMods: listView.deps.map(i => mods[i])
    readonly property var listItems: {
        let l = listView.rows.map(r => r.deps ? { kind: "deps", m: { id: "", name: "" } } : r.dep ? { kind: "mod", m: mods[r.i], dep: true } : { kind: "mod", m: mods[r.i] })
        // Requirements still to get (Later, Stop, or Turbine closed meanwhile): one row at the top goes on.
        if (reqLeft && !reqRunning && !selecting && !moving && !orderMoving) l.unshift({ kind: "needsq", m: { id: "", name: "" } })
        // Mods turned off from Steam's menu (Mods off) say so first, with the way back on that row.
        if (heldOff.length > 0 && !selecting) l.unshift({ kind: "held", m: { id: "", name: "" } })
        // Just uninstalled: the row stays, dimmed, for 8 seconds ("Removed", A puts it back), then the next row.
        if (ghost && game && ghost.game === game.id) l.splice(Math.max(0, Math.min(ghost.index, l.length)), 0, { kind: "ghost", m: { id: ghost.id, name: ghost.name } })
        return l.concat(removedMods.length > 0 ? [{ kind: "removed", m: { id: "", name: "" } }] : [])
    }
    property var ghost: null              // {game, id, name, index}
    // The ids of mods that Mods off in Steam's menu turned off and that are still off.
    property var heldOff: []
    function turnHeldBackOn() {
        if (!game || heldOff.length === 0) return
        note("action", "Turn them back on (Mods off from Steam's menu, " + heldOff.length + ")")
        Turbine.quickMods(game.id, true)
    }
    readonly property string heldWords: "Mods off from Steam's menu: " + plural(heldOff.length, "mod", "mods") + " turned off"
    Timer { id: ghostTimer; interval: 8000; onTriggered: app.ghostExpired() }
    // The removed row just went (the next mod slid under the highlight): an A in the next 400 ms still means
    // "Put back", never the mod that slid in.
    property var ghostGone: null          // {id, at, index}
    function ghostExpired() {
        if (ghost && focusedItem && focusedItem.kind === "ghost") ghostGone = { id: ghost.id, at: Date.now(), index: modIndex }
        ghost = null
    }
    function ghostJustWent() { return !!ghostGone && Date.now() - ghostGone.at < 400 && modIndex === ghostGone.index }
    // Find: only the mods whose name has every word typed, as it is typed.
    property string findText: ""
    property string findBefore: ""
    // Show and Sort (in the Y pop-up): Show narrows the list; Sort orders it (other sorts than My order hide the
    // numbers, and reorder waits for My order).
    property string listShow: "all"       // all, on, off, attention, new, update
    property string listSort: "order"     // order, az, recent
    readonly property var showKeys: ["all", "on", "off", "attention", "new", "update"]
    readonly property var showNames: ({ all: "All", on: "On", off: "Off", attention: "Needs attention", new: "New", update: "Update ready" })
    readonly property var sortKeys2: ["order", "az", "recent"]
    readonly property var sortNames2: ({ order: "My order", az: "A to Z", recent: "Recently added" })
    readonly property var shownMods: listView.shown.map(i => mods[i])
    property string keptRow: ""
    onListShowChanged: keptRow = ""
    readonly property bool listFiltered: findText !== "" || listShow !== "all" || listSort !== "order"
    function openFind() {
        note("action", "Find a mod")
        findBefore = findText
        openAsk("find", "Find a mod", "Shows only the mods whose names have these words.", findText, "Part of a mod's name", 60, false, "Show them")
    }
    // ---------------------------------------------------------------- the Y pop-up
    // One pop-up for finding. Get mods: Search, Sort and Category (and From when the game has two sources). My
    // mods: Find, Show and Sort (Select several is in a mod's X menu). A on a choice puts it into effect at
    // once; B, Start or a tap outside is Done and keeps everything; X clears all. A on the field (or Y, or a tap) types
    // right in it; Enter shows the results. A long category list ends in More..., which lists every category.
    property var findPop: ({ kind: "get", text: "", sort: 0, cat: "", src: "", show: "all", order: "order" })
    property int findRow: 0               // 0 the field, then one row per section
    property int findCol: 0
    readonly property int findCatMax: 6
    function sortNamesFor(src) { return src === "nexus" ? ["Trending", "Most liked", "Most downloaded", "Recently updated", "Newest"] : ["Most downloaded", "Most liked", "Recently updated", "Newest"] }
    readonly property var findSections: {
        const f = findPop
        if (f.kind === "mods") return [
            { key: "show", title: "Show", options: showKeys.map(k => showNames[k]), chosen: showKeys.indexOf(f.show) },
            { key: "sort", title: "Sort", options: sortKeys2.map(k => sortNames2[k]), chosen: sortKeys2.indexOf(f.order) }
        ]
        let l = []
        const src = f.src || srcNow
        if (sources.length > 1) l.push({ key: "source", title: "From", options: sources.map(srcName), chosen: Math.max(0, sources.indexOf(src)) })
        l.push({ key: "sort", title: "Sort", options: sortNamesFor(src), chosen: f.sort })
        // Categories are known for the source on screen only.
        if (getCats.length > 0 && src === srcNow) {
            const ci = Math.max(0, getCats.indexOf(f.cat))
            let vals = []
            for (let i = 0; i < Math.min(getCats.length, findCatMax); i++) vals.push(i)
            // The chosen one always shows.
            if (ci >= findCatMax) vals[findCatMax - 1] = ci
            const more = getCats.length > findCatMax
            l.push({ key: "category", title: "Category", options: vals.map(i => getCats[i] || "All").concat(more ? ["More..."] : []), values: vals,
                     chosen: vals.indexOf(ci), more: more })
        }
        return l
    }
    function openFindPop() {
        if (!game || sheet !== "") return
        // Get mods' pop-up whenever Get mods is the tab (a mod page on it closes, with its history), My mods'
        // otherwise. Never nothing: with no mods yet it says so.
        const kind = tab === 1 ? "get" : "mods"
        if (kind === "mods" && mods.length === 0) { toast.info("No mods to find yet. Get mods finds new ones."); return }
        if (kind === "get" && modPage) { modPage = false; history = []; note("action", "Find from a mod page: the page closes") }
        findPop = { kind: kind, text: kind === "get" ? getText : findText, sort: getSortIndex, cat: getCategory, src: srcNow, show: listShow, order: listSort }
        findRow = 0; findCol = 0
        // The focus: the first section with a choice of its own, else the field.
        const secs = findSections
        for (let r = 0; r < secs.length; r++) if (secs[r].key !== "source" && secs[r].chosen > 0) { findRow = r + 1; findCol = secs[r].chosen; break }
        sheet = "findpop"
        note("action", "Find pop-up (" + (kind === "get" ? "Get mods" : "My mods") + "): " + ["field"].concat(findSections.map(x => x.title + " " + x.options.length + (x.more ? " and More" : ""))).join(", "))
    }
    // The defaults: no words, the first sort, every category, every mod shown, my order.
    function findPopDefaults(f) { return Object.assign({}, f, { text: "", sort: 0, cat: "", show: "all", order: "order" }) }
    function findPopChoose(r, c) {
        const sec = findSections[r - 1]
        if (!sec) return
        let f = Object.assign({}, findPop)
        if (sec.key === "category" && sec.more && c === sec.options.length - 1) {
            const now = Math.max(0, getCats.indexOf(f.cat))
            openPick("Category", gameTitle, getCats.map((x, i) => ({ icon: i === now ? "power" : "info", title: x || "All", detail: i === now ? "Chosen" : "", aLabel: "Choose",
                run: () => {
                    findPop = Object.assign({}, findPop, { cat: getCats[i] }); sheet = "findpop"
                    // The ring goes onto the chosen category, and it takes effect at once.
                    const sc = findSections[findRow - 1]
                    findCol = sc && sc.chosen >= 0 ? sc.chosen : 0
                    findPopApply(false)
                } })), "findpop")
            menuIndex = now
            return
        }
        if (sec.key === "source") { if (sources[c] !== (f.src || srcNow)) { f.src = sources[c]; f.sort = 0; f.cat = "" } }
        else if (sec.key === "sort") { if (f.kind === "mods") f.order = sortKeys2[c]; else f.sort = c }
        else if (sec.key === "show") f.show = showKeys[c]
        else if (sec.key === "category") f.cat = getCats[sec.values[c]]
        findPop = f
        note("action", "Find pop-up: " + sec.title + " " + sec.options[c])
        // A choice takes effect the moment it is made.
        findPopApply(false)
    }
    // Typing right in the pop-up: Steam's keyboard opens over it; Enter shows the results.
    function findPopField() {
        findRow = 0
        findView.focusField()
        keyboardUp = true
        note("action", "Find pop-up: typing")
        if (Turbine.gamemode && !shot.on) Qt.openUrlExternally("steam://open/keyboard")
    }
    function findPopTyping(text) {
        // My mods follows the words as they're typed.
        if (findPop.kind === "mods") { findPop = Object.assign({}, findPop, { text: text }); findText = text; modIndex = 0 }
    }
    function findPopEntered(text) {
        findPop = Object.assign({}, findPop, { text: text })
        note("action", "Find pop-up: words \"" + text + "\"")
        findPopApply(true)
    }
    function findPopEndTyping() {
        if (keyboardUp) { keyboardUp = false; if (Turbine.gamemode && !shot.on) Qt.openUrlExternally("steam://close/keyboard") }
        findView.stopTyping()
        keyRoot.forceActiveFocus()
    }
    // Done (B, Start, View, a tap outside, or A on Done): close, keeping everything. Words typed but not yet entered
    // count too.
    function findPopDone() {
        const typed = findView.typing ? findView.fieldText.trim() : findPop.text
        findPopEndTyping()
        if (typed !== findPop.text) { findPop = Object.assign({}, findPop, { text: typed }); findPopApply(true) }
        else { sheet = ""; note("action", "Find pop-up: Done") }
    }
    function findPopClear() {
        findPopEndTyping()
        findPop = findPopDefaults(findPop)
        findRow = 0; findCol = 0
        note("action", "Find pop-up: Clear all")
        findPopApply(false)
        toast.info("Cleared")
    }
    // Puts the pop-up's choices into effect; close says whether the pop-up closes (words entered, Done).
    Timer { id: findBrowse; interval: 400; onTriggered: app.browseNow() }
    function findPopApply(close) {
        const f = findPop
        if (close) { findPopEndTyping(); sheet = "" }
        if (f.kind === "mods") {
            findText = f.text; listShow = f.show; listSort = f.order
            if (tab !== 0) tab = 0
            zone = "main"; modIndex = 0
            note("action", "Find pop-up: \"" + f.text + "\", " + showNames[f.show] + ", " + sortNames2[f.order] + ": " + shownMods.length + " shown")
            return
        }
        if (srcNow === "nexus" && nexusPublic === "failed" && !account.hasKey) toast.info("Searching Nexus from Turbine isn't working right now. Open on Nexus searches there.")
        // Get mods: the results always end on the grid with no mod page over it.
        modPage = false; history = []
        const changed = (f.src && f.src !== srcNow) || f.text !== getText || f.sort !== getSortIndex || f.cat !== getCategory
        if (f.src && f.src !== srcNow) { getSource = f.src; browseData = null; catList = [] }
        getText = f.text; getSortIndex = f.sort; getCategory = f.cat
        if (tab !== 1) tab = 1
        zone = "main"; getArea = "grid"; getIndex = 0
        restoreFocus = { area: "grid", index: 0 }
        note("action", "Find pop-up: \"" + f.text + "\", " + (sortNamesFor(srcNow)[f.sort] || "") + ", " + (f.cat || "All") + (changed ? "" : " (no change)"))
        if (!changed) return
        // Stepping through chips doesn't fire a search per chip; words and Done search at once.
        browseLoading = true
        if (close) { findBrowse.stop(); browseNow() } else findBrowse.restart()
    }
    function findPopAct(b) {
        const sec = findSections[findRow - 1]
        const btnRow = findSections.length + 1
        const n = findRow === 0 ? 1 : findRow === btnRow ? 2 : (sec ? sec.options.length : 1)
        // Up and Down land on the row's chosen option.
        if (b === "up" || b === "down") {
            if (findView.typing) findPopEndTyping()
            findRow = Math.max(0, Math.min(btnRow, findRow + (b === "up" ? -1 : 1)))
            const s2 = findSections[findRow - 1]
            findCol = s2 && s2.chosen >= 0 ? s2.chosen : 0
        }
        else if (b === "left") findCol = Math.max(0, findCol - 1)
        else if (b === "right") findCol = Math.min(n - 1, findCol + 1)
        else if (b === "y" || (b === "a" && findRow === 0)) findPopField()
        else if (b === "a" && findRow === btnRow) { if (findCol === 0) findPopDone(); else findPopClear() }
        else if (b === "a") findPopChoose(findRow, findCol)
        else if (b === "x") findPopClear()
        else if (b === "b" || b === "view" || b === "start") findPopDone()
    }
    function findPopALabel() {
        if (findRow === 0) return "Type"
        if (findRow === findSections.length + 1) return findCol === 0 ? "Done" : "Clear all"
        const sec = findSections[findRow - 1]
        if (!sec) return ""
        if (sec.more && findCol === sec.options.length - 1) return "More"
        return "Choose"
    }
    // The pop-up's live result line.
    readonly property string findResult: findPop.kind === "mods" ? "Showing " + shownMods.length + " of " + mods.length
        : browseLoading ? "Looking..." : nexusFallback ? "" : getItems.length === 0 ? "Nothing matches" : plural(getItems.length, "mod", "mods") + (noMore ? "" : "+")
    // What is in effect on Get mods, over the cards (empty when everything is the default).
    readonly property string getSummary: {
        const parts = []
        if (getSortIndex > 0) parts.push(sortNamesFor(srcNow)[getSortIndex] || "")
        if (getCategory !== "") parts.push(getCategory)
        if (getText !== "") parts.push("\"" + getText + "\"")
        return parts.filter(x => x !== "").join(" · ")
    }
    // Select several (from the bar over My mods): A ticks, X acts on every ticked mod, B done.
    property bool selecting: false
    property var ticked: ({})
    readonly property int tickedCount: Object.keys(ticked).filter(k => ticked[k] && mods.some(m => m.id === k)).length
    function startSelect(m) { selecting = true; let t = {}; if (m) t[m.id] = true; ticked = t; note("action", "Select several") }
    function selectAct(b) {
        const it = focusedItem
        if (b === "up") modIndex = Math.max(0, modIndex - 1)
        else if (b === "down") modIndex = Math.min(Math.max(0, listItems.length - 1), modIndex + 1)
        else if (b === "a" && it && it.kind === "mod") { let t = Object.assign({}, ticked); t[it.m.id] = !t[it.m.id]; ticked = t }
        else if (b === "x" && tickedCount > 0) selectMenu()
        else if (b === "b") { selecting = false; ticked = {}; note("action", "Select several: done") }
        else if (b === "lb" || b === "rb" || b === "left") { selecting = false; ticked = {}; act(b) }
    }
    function selectMenu() {
        const list = mods.filter(m => ticked[m.id])
        const n = plural(list.length, "mod", "mods")
        openPick(n + " ticked", list.map(m => m.name).slice(0, 3).join(", ") + (list.length > 3 ? " and " + (list.length - 3) + " more" : ""), [
            { icon: "power", title: "Turn on all " + list.length, detail: "They go into the game", run: () => { for (const m of list) if (!m.enabled) toggle(m); selecting = false; ticked = {} } },
            { icon: "power", title: "Turn off all " + list.length, detail: "They stay installed", run: () => { for (const m of list) if (m.enabled) toggle(m); selecting = false; ticked = {} } },
            { icon: "remove", title: "Uninstall all " + list.length, detail: "Kept in Removed mods for 7 days", danger: true, run: () => {
                for (const m of list) { mods = mods.filter(x => x.id !== m.id); Turbine.removeMod(m.id) }
                edited(); selecting = false; ticked = {}; modIndex = Math.max(0, Math.min(modIndex, listItems.length - 1))
                toast.done(n + " moved to Removed mods (kept 7 days).") } }
        ], "")
    }
    // A small menu built on the spot: items {icon, title, detail, danger, run}.
    property var pickItems: []
    property string pickTitle: ""
    property string pickSub: ""
    property string pickBack: ""
    function openPick(title, sub, items, back) { pickTitle = title; pickSub = sub; pickItems = items; pickBack = back || ""; pickBLabel = ""; menuIndex = 0; sheet = "pick" }
    // A list that only tells (Why won't it start?): Start doesn't play from it, and rows marked info have no A.
    property string pickBLabel: ""          // what B says on this menu ("Not now"), when not Back or Close
    function pickRun(i) {
        const it = pickItems[i]
        if (!it || it.info) return
        note("action", pickTitle + ": " + it.title)
        sheet = ""
        it.run()
    }
    // Play options: Play with mods, Play without mods, or with another profile.
    function openPlayOptions() {
        if (!game || sheet !== "") return
        note("action", "Play options")
        Turbine.profiles()
        playOptionsNow()
    }
    function playOptionsNow() {
        const others = profileList.filter(p => !profileData || p.name !== profileData.active)
        openPick("Play options", gameTitle, [
            { icon: "play", title: "Play with mods", detail: seamlessIn ? plural(mods.filter(m => m.enabled && !isSeamless(m)).length, "mod", "mods") + " on. Seamless Co-op stays out." : plural(mods.filter(m => m.enabled).length, "mod", "mods") + " on", run: () => startPlay() },
            { icon: "play", title: "Play without mods", detail: "Your mods stay installed", run: () => Turbine.playOriginal() }
        ].concat(seamlessIn ? [
            { icon: "play", title: "Play Seamless Co-op", detail: "With your mods and Seamless Co-op; co-op keeps its own saves, apart from solo", run: () => startCoop() }
        ] : []).concat(canSkipLoader ? [
            { icon: "play", title: "Play without " + loaderShort,
              detail: "Leaves out " + loaderShort + " and the mods that need it, this time only", run: () => { autoApply.stop(); Turbine.playWithoutLoader() } }
        ] : []).concat(others.map(p => ({ icon: "play", title: "Play with loadout " + p.name, detail: "Switches to it, then plays", run: () => { Turbine.profileSwitch(p.name); startPlay() } }))), "")
    }
    readonly property bool canSkipLoader: !!rules && !!rules.loader && rules.loaderInstalled === true && mods.some(m => m.enabled)
    // X on a profile: Rename, Delete (hold A).
    function profileMenu(n) {
        const inUse = profileData && profileData.active === n
        openPick(n, inUse ? "In use now" : "Loadout", [
            { icon: "info", title: "Rename", detail: "Give it a new name", run: () => { sheet = "profiles"; openAsk("renameProfile:" + n, "Rename " + n, "A new name for this loadout.", n, "Loadout name", 60, false, "Rename") } }
        ].concat(inUse ? [] : [
            { icon: "remove", title: "Delete", detail: pressTwice ? "Press A twice to delete it permanently" : "Hold A to delete it permanently", danger: true, run: () => askDelete("profile:" + n, "Delete the loadout " + n + " permanently? Its mods stay installed.") }
        ]), "profiles")
    }
    // Load order: the mod that placed a plugin.
    property var orderOwners: ({})
    readonly property string pluginOwner: orderPlugin ? (orderOwners[orderPlugin.name.toLowerCase()] || "") : ""
    function goToMod(id) {
        note("action", "Go to its mod: " + id)
        // Always lands on My mods with the ring on the mod, from anywhere (Settings, Play, the
        // rail, the Nexus page).
        if (browserOpen) { browserFrom = null; closeBrowser("back") }
        sheet = ""; page = "game"; zone = "main"; tab = 0; findText = ""; listShow = "all"; modPage = false; history = []
        const i = listItems.findIndex(x => x.kind === "mod" && x.m.id === id)
        if (i >= 0) modIndex = i
    }
    // Add from Downloads: this game's files (Downloads and its Game Mods folder), or every file with Y.
    property bool addAll: false
    readonly property var addList: addAll ? downloads : downloads.filter(d => d.here !== false)
    // Mods added since the game was last played get a "New" badge.
    readonly property real lastPlayedAt: { const p = setups.filter(x => x.played === true).map(x => x.savedAt); return p.length > 0 ? Math.max.apply(null, p) : 0 }
    function isNew(m) { return m.added > lastPlayedAt }
    readonly property var focusedItem: listItems.length > 0 ? listItems[Math.min(modIndex, listItems.length - 1)] : null

    function parse(text, fallback) {
        if (!text) return fallback
        try { return JSON.parse(text) } catch (e) { return fallback }
    }
    Connections {
        target: Turbine
        function onGamesChanged() {
            // A store's name with an em or en dash shows with a colon ("Witcher 3: Remastered").
            app.allGames = app.parse(Turbine.games, []).map(g => Object.assign({}, g, { name: String(g.name || "").replace(/\s*[\u2014\u2013]\s*/g, ": ") }))
            app.orderGames(shot.arg("game") || seen.lastGame)
        }
        function onModsJsonChanged() {
            if (Turbine.modsJson === "") { app.modsLoaded = false; app.mods = []; app.removedMods = []; app.setups = []; return }
            if (app.moving) return
            const v = app.parse(Turbine.modsJson, {})
            // The list can come back in a new order (a mod turned on, put back, updated): the focus stays on its mod.
            const keep = app.modsLoaded && !app.landFocus && app.focusedItem && app.focusedItem.kind === "mod" ? app.focusedItem.m.id : ""
            app.mods = (v.mods || []).map(app.withNames)
            app.removedMods = v.removed || []
            app.setups = v.setups || []
            app.lastGoodId = v.lastGood || ""
            app.overlapsMap = v.overlaps || {}
            app.linksMap = v.links || {}
            app.coreIds = v.core || []
            app.modWarns = v.warns || {}
            app.kcdRestore = v.kcdRestore === true
            app.placements = v.placements || {}
            app.guardedMods = v.guarded || {}
            app.gameUpdate = v.gameUpdate || null
            app.heldOff = v.heldOff || []
            app.modsLoaded = true
            app.checkModHold()
            Qt.callLater(app.maybeGameUpdate)
            app.modIndex = Math.max(0, Math.min(app.modIndex, app.listItems.length - 1))
            if (keep !== "") {
                const k = app.listItems.findIndex(x => x.kind === "mod" && x.m.id === keep)
                if (k >= 0) app.modIndex = k
            }
            if (app.landFocus && app.game) {
                app.landFocus = false
                const want = app.lastFocusId[app.game.id]
                const i = want ? app.listItems.findIndex(x => x.kind === "mod" && x.m.id === want) : -1
                app.modIndex = i >= 0 ? i : 0
                app.note("focus", app.game.name + ": landed on row " + (app.modIndex + 1) + (want ? " (last focused " + want + (i < 0 ? ", not in the list" : "") + ")" : ""))
            }
            if (app.removedIndex > app.removedMods.length) app.removedIndex = app.removedMods.length
            if (app.sheet === "removed" && app.removedMods.length === 0) app.sheet = ""
            app.focusArrived()
            app.maybeLandOnGet()
            app.offerLoader()
            app.maybeCheckUpdates()
        }
        function onRulesJsonChanged() {
            app.rulesFor = app.game ? app.game.id : ""
            app.rules = app.parse(Turbine.rulesJson, null)
            // A game that opens on Get mods before its rules are read is asked about once they are.
            if (app.tab === 1) Qt.callLater(() => app.maybeAskNexusGame("its rules were read"))
            app.maybeLandOnGet()
            app.offerLoader()
            app.maybeAntiCheat()
            if (app.rules && app.tab === 1 && !app.browseData) app.browseNow()
        }
        function onDownloadsJsonChanged() {
            app.downloads = app.parse(Turbine.downloadsJson, [])
            app.addIndex = Math.max(0, Math.min(app.addIndex, app.addList.length - 1))
        }
        function onMessage(text, level) { app.say(text, level) }
        // Putting saves back couldn't keep the saves as they are now: ask before replacing them for good.
        function onSavesKeepFailed(id, why) {
            app.note("action", "Saves: " + why)
            app.askDelete("saves:" + id, "Turbine couldn't back up your current saves first (" + why.replace(/^.*?first: /, "") + "). Put the backup back anyway? Your current saves are replaced permanently.")
        }
        function onChecked(json) { app.checked(app.parse(json, { checks: [] })) }
        function onLaunch(url) {
            app.playAt = Date.now()
            // Heroic starts its games itself, outside Steam; in Game Mode the game
            // may open behind Turbine.
            if (url.indexOf("heroic://") === 0 && Turbine.gamemode) { app.note("action", "Play through Heroic in Game Mode"); toast.info("Heroic is starting it. If it doesn't show, press the Steam button and switch to it.") }
            if (!shot.on) Qt.openUrlExternally(url)
        }
        function onOlderSeamless(gameId, have, thisOne) { app.askOlderSeamless(gameId, have, thisOne) }
        function onCoopChecked(gameId, json) { app.coopLooked(gameId, app.parse(json, null)) }
        function onCoopPasswordNeeded(gameId) {
            app.note("action", "Play Seamless Co-op stopped: no session password in the settings the game loads")
            if (app.game && app.game.id === gameId && app.seamlessMod) { app.coopStops++; app.openCoopPasswordFirst() }
        }
        function onLaunchSetup(gameId, json) { const o = app.parse(json, null); if (o) app.openLaunchSetup(o) }
        function onNotStarted(gameId) { app.openNotStarted(gameId) }
        function onGameSeen(gameId, seconds) { app.gameSeenLate(gameId, seconds) }
        function onSettingsLoaded(json) { app.settingsLoaded(app.parse(json, { error: "Couldn't open it." })) }
        function onGameEnded(gameId, seconds, known) {
            if (known) { if (seconds < 60) app.quickExit(gameId, seconds) }
            else app.measureAway = gameId
        }
        function onPages(title, json) { app.openPages(title, app.parse(json, [])) }
        function onHandlerJsonChanged() { app.handler = app.parse(Turbine.handlerJson, { state: "unknown" }) }
        // Catching browser downloads (catch.rs).
        function onStatusJsonChanged() { if (!shot.on || shot.page === "slowhint" || shot.page === "regress") app.status = app.parse(Turbine.statusJson, { active: false }) }
        function onBrowserJsonChanged() {
            if (shot.on) return
            app.browser = app.parse(Turbine.browserJson, { checked: false })
            if (app.sheet === "nobrowser" && app.browserFound) { app.sheet = ""; toast.info("Found " + app.browser.result.name + ". Nexus pages open there.") }
        }
        function onBridgeJsonChanged() {
            if (shot.on) return
            const was = app.bridge.connected || 0
            app.bridge = app.parse(Turbine.bridgeJson, { running: false })
            if (app.bridge.connected > was && app.openState !== "")
                app.note("browser", "The browser helper " + (app.bridge.helper || "") + " connected " + (Date.now() - app.openAt) + " ms after Open")
            // The helper was paired on another port: the pairing page again, once.
            if (app.bridge.moved && !app.movedPaired) { app.movedPaired = true; app.note("bridge", "Port changed: opening the pairing page again"); app.pair(false) }
        }
        function onAdded(name, notes) { app.openNotes(name, app.parse(notes, [])) }
        function onAsked(json) {
            const f = app.parse(json, null)
            app.found = f && f.path ? f : null
            if (app.found && app.sheet === "") { app.infoIndex = 0; app.sheet = "found" }
            else if (!app.found && (app.sheet === "found" || app.sheet === "pickgame")) app.sheet = ""
        }
        function onPad(button, pressed) {
            app.padEvents++
            if (button === "wheel") { app.stickWheel(pressed ? 1 : -1); return }
            if (pressed) { app.lastInput = "pad"; app.pressStamp = Turbine.pressAt(); app.pressFresh = true }
            pressed ? app.press(button) : app.release(button)
        }
        // 0.3
        function onBrowseJsonChanged() {
            const v = app.parse(Turbine.browseJson, null)
            if (!v || !app.game || v.game !== app.game.id) return
            // An answer for an older question (a quick second choice, a source switch) is dropped, so the grid
            // never shows Trending while the pop-up says Most liked.
            if (v.qsrc !== undefined && (v.query !== app.getText || v.qcat !== app.getCategory || v.qsort !== (app.sortKeys[app.getSortIndex] || "") || v.qsrc !== app.srcNow)) {
                app.note("net", "Get mods: an older answer dropped (\"" + v.query + "\", " + (v.qsort || "no sort") + ", " + (v.qcat || "All") + ")")
                return
            }
            app.loadingMore = false
            if (!v.append && (v.items || []).length > 0) app.note("net", "Get mods for " + app.game.name + ": " + v.items.length + " mods, first " + v.items[0].name + (v.categories && v.categories.length > 0 ? ", " + v.categories.length + " categories" : ""))
            if (v.fallback === true) {
                // Browsing without a key failed: Open on Nexus instead, for the rest of this session.
                if (!app.browseData || !v.append) { app.nexusPublic = "failed"; app.browseData = v }
                app.browseLoading = false
                app.getArea = "fallback"; app.getIndex = 0
                return
            }
            if (v.append && app.browseData && app.browseData.items) {
                const more = Object.assign({}, v, { items: app.browseData.items.concat(v.items || []) })
                app.browseData = more
                app.noMore = (v.items || []).length === 0
                return
            }
            app.nexusPublic = v.source === "nexus" ? "ok" : app.nexusPublic
            if (!v.category && (v.categories || []).length > 0) app.catList = v.categories
            else if (v.source !== "nexus") app.catList = v.categories || []
            app.browseData = v
            if (v.source === "nexus") for (const x of (v.items || [])) {
                const mine = app.installedNexus[String(x.id)]
                if (mine !== undefined) app.rememberNexus(app.mods.find(y => y.id === mine), x.name, x.owner, x.summary, x.icon)
            }
            app.noMore = (v.items || []).length < 30
            app.browseLoading = false
            if (app.restoreFocus) {
                const r = app.restoreFocus
                app.restoreFocus = null
                app.getArea = r.area === "recent" && app.starterItems.length === 0 ? "grid" : r.area === "fallback" ? "grid" : r.area
                app.getIndex = Math.max(0, Math.min(r.index, (v.items || []).length - 1))
            } else if (app.getArea === "grid") app.getIndex = Math.max(0, Math.min(app.getIndex, (v.items || []).length - 1))
            else if (app.getArea === "fallback") { app.getArea = "grid"; app.getIndex = 0 }
        }
        function onDetailsJsonChanged() {
            let d = app.parse(Turbine.detailsJson, null)
            // A page that couldn't be read still shows: what the card knew, the reason in one line, Open on Nexus.
            if (d && d.error && app.detailsItem) d = app.partialPage(app.detailsItem, d.error)
            app.detailsData = d
            if (app.detailsData && app.detailsData.name && app.detailsItem) app.rememberViewed(app.detailsItem, app.detailsData)
            if (d && d.name && d.source === "nexus" && app.installedNexus[String(d.id)] !== undefined)
                app.rememberNexus(app.mods.find(y => y.id === app.installedNexus[String(d.id)]), d.name, d.owner, d.summary, d.icon, d.deprecated === true)
            if ((shot.page === "versions" || shot.page === "nexusversions") && app.otherVersions.length > 0) app.openVersions()
        }
        function onDownloadJsonChanged() { app.dl = app.parse(Turbine.downloadJson, { active: false }) }
        function onPlaceJsonChanged() {
            app.placeData = app.parse(Turbine.placeJson, null)
            if (app.placeData) app.maybePlace()
            else if (app.page === "place" && app.placeMode === "add") app.placeLeave()
        }
        function onNexusGamesJsonChanged() {
            const v = app.parse(Turbine.nexusGamesJson, { state: "idle", games: [] })
            if (v.state === "error") toast.problem("Nexus couldn't search its games: " + (v.error || ""))
            if (v.state !== "done") return
            app.nexusFound = v.games || []
            if (app.sheet === "" || (app.sheet === "pick" && app.pickTitle.indexOf("Which game on Nexus") === 0)) app.openNexusPick()
        }
        function onFomodJsonChanged() {
            app.fomod = app.parse(Turbine.fomodJson, null)
            if (app.fomod && app.sheet === "") app.sheet = "fomod"
            else if (!app.fomod && app.sheet === "fomod") app.sheet = ""
        }
        function onOrderJsonChanged() {
            const v = app.parse(Turbine.orderJson, null)
            if (!v || !app.game || v.game !== app.game.id) return
            if (app.orderMoving) return
            app.order = v.plugins || []
            app.orderOwners = v.owners || {}
            app.orderError = v.error || ""
            app.orderMessages = v.messages || {}
            if (v.note) app.orderNote = v.note
            // Focus starts on the first plugin that can move (the game's own files are fixed at the top).
            if (app.orderLand && app.order.length > 0) {
                app.orderLand = false
                app.orderIndex = app.orderMemo >= 0 && app.orderMemo < app.order.length ? app.orderMemo : Math.max(0, app.order.findIndex(x => !x.fixed))
                app.orderMemo = -1
            }
            if (shot.page === "lootsort" && v.sortedBy === "loot") app.orderIndex = app.order.length - 1
            if (shot.page === "lootnotes" && v.sortedBy === "loot") {
                const i = app.order.findIndex(x => (app.orderMessages[x.name.toLowerCase()] || []).length > 1)
                if (i >= 0) { app.orderIndex = i; app.sheet = "plugin" }
            }
            app.orderIndex = Math.max(-1, Math.min(app.orderIndex, app.order.length - 1))
        }
        function onProfilesJsonChanged() {
            const v = app.parse(Turbine.profilesJson, null)
            if (!v || !app.game || v.game !== app.game.id) return
            app.profileData = v.data
            if (app.sheet === "pick" && app.pickTitle === "Play options") app.playOptionsNow()
        }
        function onSharesJsonChanged() { app.shares = app.parse(Turbine.sharesJson, []) }
        function onImportJsonChanged() {
            const v = app.parse(Turbine.importJson, null)
            app.importData = v && (v.plan || v.error) ? v : null
            if (app.importData && app.sheet === "") { app.profIndex = 0; app.sheet = "profiles" }
            if (app.importData) app.profIndex = 0
        }
        function onUpdatesJsonChanged() {
            app.updatesData = app.parse(Turbine.updatesJson, null)
            // Nexus's own name, author, summary and picture for mods added from a file.
            const names = app.updatesData && app.updatesData.names && app.game && app.updatesData.game === app.game.id ? app.updatesData.names : {}
            for (const id in names) {
                const m = app.mods.find(x => x.id === id), w = names[id]
                if (m && w && w.name && !(app.nexusInfo[app.nameKey(id)] || {}).name) app.rememberNexus(m, w.name, w.author, w.summary, w.picture)
            }
        }
        function onAccountJsonChanged() { app.account = app.parse(Turbine.accountJson, { state: "none" }) }
        function onCodeJsonChanged() {
            app.codeData = app.parse(Turbine.codeJson, null)
            if (app.codeData && app.codeData.state === "ready") Turbine.copyText(app.codeData.code)
        }
        function onArrived(gameId, id, name, on) { app.modArrived(gameId, id, name, on) }
        function onLabsJsonChanged() {
            const v = app.parse(Turbine.labsJson, null)
            if (!v || !v.id) return
            app.labsResult = v
            app.labsChoose(app.labsItems.find(x => x.id === v.id), v.ok ? "works" : "problem", true)
            if (v.ok) toast.done(v.text); else toast.problem(v.text)
        }
        function onSelfJsonChanged() {
            app.selfUpd = app.parse(Turbine.selfJson, { state: "idle" })
            if (app.selfUpd.state === "available" && app.selfAuto) {
                app.selfAuto = false
                toast.info("Turbine " + app.selfUpd.version + " is ready to install: Settings, Updates.")
            }
            if (app.selfUpd.state === "ready" && app.backToStable) { app.backToStable = false; updates.prerelease = false }
        }
    }
    // The rail's order: pinned games, then the last played, then the rest; hidden games (tools) left off. The game on
    // screen stays the same game when the order changes.
    property var allGames: []
    readonly property var pinnedIds: parse(railSet.pinned, [])
    readonly property var hiddenIds: hiddenTest !== null ? hiddenTest : parse(railSet.hidden, [])
    // The build's check of a rail with every game hidden (the saved list is never touched there).
    property var hiddenTest: null
    function orderGames(want) {
        const keep = want !== undefined ? want : (game ? game.id : "")
        // A game asked for by name shows even when hidden (not in the build's check of a rail with every game hidden).
        const order = parse(Turbine.railOrderJson(JSON.stringify({ ids: allGames.map(g => g.id), played: parse(seen.played, []), pinned: pinnedIds,
                                                                   hidden: hiddenIds, always: hiddenTest === null ? shot.arg("game") : "" })), [])
        const list = order.map(i => allGames[i])
        const i = list.findIndex(g => g.id === keep)
        // Only the order changed: the game on screen isn't "switched" (its focus and Get mods stay as they are).
        sameGameReorder = !!game && i >= 0 && list[i].id === game.id
        games = list
        current = i >= 0 ? i : 0
        sameGameReorder = false
        if (zone === "rail") railIndex = current
    }
    property bool sameGameReorder: false
    function setRailList(which, id, on) {
        let l = parse(which === "pinned" ? railSet.pinned : railSet.hidden, []).filter(x => x !== id)
        if (on) l.push(id)
        if (which === "hidden" && hiddenTest !== null) hiddenTest = l
        else if (!shot.on) { if (which === "pinned") railSet.pinned = JSON.stringify(l); else railSet.hidden = JSON.stringify(l) }
        note("action", (which === "pinned" ? (on ? "Pin " : "Unpin ") : (on ? "Hide from rail " : "Show on rail ")) + id)
        orderGames()
    }
    // Focus memory per game and tab: which tab, Get mods' search and card, Load order's row (My mods' row is
    // lastFocusId). Saved as a game is left, put back when it is opened again.
    property var focusMemo: ({})
    property string memoGame: ""
    property int orderMemo: -1
    function saveFocus(gid) {
        let all = Object.assign({}, focusMemo)
        all[gid] = { tab: tab, get: { area: getArea === "fallback" ? "grid" : getArea, index: getIndex, text: getText }, order: orderIndex }
        focusMemo = all
    }
    // The mod last focused in each game: switching back lands on it (else on the first mod).
    property var lastFocusId: ({})
    property bool landFocus: true
    // Not while a game's mods are still on their way in (the list on screen is the last game's until then).
    // Whatever puts a different mod under the highlight (a move of the focus, or the list changing under it).
    onFocusedItemChanged: if (game && !moving && !landFocus && modsLoaded && page === "game" && tab === 0 && focusedItem && focusedItem.kind === "mod") {
        if (shot.scripted && lastFocusId[game.id] !== focusedItem.m.id) note("focus", game.name + ": remember " + focusedItem.m.name + " (row " + (modIndex + 1) + ", zone " + zone + ", tab " + tab + ", " + mods.length + " mods)")
        lastFocusId[game.id] = focusedItem.m.id
    }
    onGameChanged: {
        gameChangedAt = Date.now()
        if (game && !sameGameReorder && page === "game") pageArrived()
        allOffWas = []
        disarm()
        barsHidden = false
        if (!game || sameGameReorder) return
        if (memoGame !== "" && memoGame !== game.id) saveFocus(memoGame)
        memoGame = game.id
        if (!shot.on) seen.lastGame = game.id
        moving = false
        landFocus = true
        modsLoaded = false
        modIndex = 0
        checks = []
        modPage = false
        history = []
        landedOn = ""
        browseData = null; catList = []
        getArea = "grid"; getIndex = 0; getText = ""; getCategory = ""
        // A find, show or sort from one game never follows into the next.
        findText = ""; listShow = "all"; listSort = "order"; getSortIndex = 0; getSource = ""; keptRow = ""
        order = []; orderIndex = 0; orderMessages = {}; orderNote = ""
        profileData = null
        // Back to a game: its tab and the row it was on. First visit: My mods, the first mod.
        const f = focusMemo[game.id]
        orderMemo = f && f.order !== undefined ? f.order : -1
        if (f) {
            tab = f.tab
            if (f.get) { getText = f.get.text || ""; restoreFocus = { area: f.get.area, index: f.get.index } }
            note("focus", "Back to " + game.name + ": tab " + f.tab + ", row " + (f.tab === 0 ? (lastFocusId[game.id] || "first") : f.tab === 1 ? f.get.area + " " + f.get.index : f.order))
        } else tab = 0
        Turbine.select(game.id)
        // Its loadouts, for the header chip (shown with two or more).
        Turbine.profiles()
        if (tab === 1) Qt.callLater(browseNow)
    }
    // Desktop Mode with no controller seen is normal (Steam's desktop layout drives Turbine
    // as a mouse and a keyboard, and both work everywhere): nothing is shown, in a toast or in Settings. The
    // activity log has one line, once per run.
    property bool noPadNoted: false
    function noPadNote() {
        if (Turbine.pads || Turbine.gamemode || noPadNoted) return false
        noPadNoted = true
        note("pad", "Desktop Mode with no controller seen: nothing shown (the mouse and the keyboard drive Turbine; started from its Steam entry, Steam gives it the controller)")
        return true
    }
    Timer { interval: 4000; running: !shot.on && !Turbine.gamemode; onTriggered: app.noPadNote() }
    Component.onCompleted: {
        introSeen = parse(seen.basicIntroDone, [])
        adoptSeen()
        // Recorded samples instead of the internet (the build's screenshot checks); first, before anything asks.
        if (shot.arg("selftest-data") !== "") Turbine.setSampleData(shot.arg("selftest-data"))
        else if (!shot.on) {
            Turbine.accountStatus()
            Turbine.startCatching()
            Qt.callLater(steamSetupAtStart)
            if (updates.auto) { selfAuto = true; Turbine.selfCheckAtStart(labsOn && updates.prerelease) }
        }
        load()
        note("app", "Qt platform " + Qt.platform.pluginName + (Turbine.gamemode ? ", Game Mode" : ""))
        if (shot.page === "settings") { page = "settings"; zone = "settings"; setRow = firstSettingsRow() }
        else if (shot.page === "details") sheet = "details"
        else if (shot.page === "perf") { page = "perf"; zone = "perf" }
        else if (shot.page === "notice") sheet = "notice"
        else if (!shot.on && !seen.tutorialDone) openTutorial(0)
        else if (!shot.on && !Turbine.steamos && !seen.notSteamOS) sheet = "notice"
        checkDisplays()
        if (shot.on) { selfCheck.setSource("SelfCheck.qml", { app: app }); selfCheck.item.start() }
    }

    // ---------------------------------------------------------------- status line
    // Messages from the engine show until they're done ("work"), for a while ("ok", errors), then the line goes back
    // to saying how the game stands.
    property string msgText: ""
    property string msgLevel: ""
    Timer { id: msgTimer; interval: 4000 }
    function say(text, level) {
        msgText = text
        msgLevel = level
        msgTimer.interval = level === "error" ? 10000 : 4000
        if (level !== "work") msgTimer.restart()
        else msgTimer.stop()
        // An error is never only the bar's small second line. A short one is a toast; a long one
        // (Play couldn't start, with the reason) a sheet that stays until OK. In Settings, where the bar has no
        // status, every result shows as a toast.
        if (level === "error") {
            if (text.length > 80 && sheet === "" && !shot.strictSheets) openMsg({ title: "That didn't work", text: text, steps: [], notes: [], footnote: "", buttons: [{ key: "a", label: "OK" }] })
            else toast.problem(text)
        } else if (page === "settings" && (level === "ok" || level === "warn") && text !== "") toast.info(text)
        // The bar's status is a few words: a longer result is a toast, and the status says it shortly (msgShort).
        else if ((level === "ok" || level === "warn") && text.length > msgMost) toast.info(text)
    }
    readonly property int msgMost: 40
    readonly property string msgShort: msgText.length <= msgMost || msgLevel === "error" ? msgText : msgLevel === "work" ? "Working..." : msgLevel === "warn" ? "Something to check" : "Done"
    readonly property bool msgShown: msgText !== "" && (msgLevel === "work" || msgTimer.running)
    readonly property bool savingNow: Turbine.busy && msgShown && msgLevel === "work" && msgText.indexOf("Saving ") === 0
    function plural(n, one, many) { return n + " " + (n === 1 ? one : many) }
    readonly property var statusNow: {
        if (page === "settings") return { level: "ok", words: "Changes show right away", detail: "" }
        if (page === "perf") return { level: "ok", words: "2,000 rows", detail: "Hold the D-pad to scroll fast, or turn the left stick in a circle." }
        if (!game) return { level: Turbine.problems !== "" ? "warn" : "ok", words: "No games found yet", detail: "" }
        if (dl.active) return { level: "busy", words: "Downloading " + dl.label, detail: "", progress: dlPart >= 0 ? dlPart : 0,
                                 progressText: dl.total > 0 ? Math.round(dlPart * 100) + "% · " + sizeWords(dl.got) + " of " + sizeWords(dl.total) : sizeWords(dl.got) }
        // A free account presses Slow download on the page itself; the status says so (catch.rs, watchPage).
        const slow = status.active && !status.done && status.level === "wait" && status.text.indexOf("Slow download") >= 0
        if (openState === "opening") return { level: "busy", words: slow ? status.text : "Opening Nexus in " + openName + "...", detail: slow ? "Opening Nexus in " + openName + "..." : "" }
        if (openState === "open") return { level: slow ? "busy" : "ok", words: slow ? status.text : "Nexus is open in " + openName + ". Sign in if asked (free), press Slow download, then switch back to Turbine.", detail: "" }
        // An online-only anti-cheat game says so before anything else (the refused file's message is in the log).
        if (acRefused) return { level: "warn", words: "Needs its anti-cheat: no mods", detail: "" }
        // Mods held after a game update are never a hidden state.
        if (modHold && mods.some(m => m.enabled)) return { level: "warn", words: "Mods paused: " + gameTitle + " updated", detail: "", hold: true }
        if (msgShown) return { level: ({ work: "busy", ok: "ok", warn: "warn", error: "stop" })[msgLevel] || "ok", words: msgShort, detail: "" }
        if (needLoader.length > 0)
            return { level: "warn", words: plural(needLoader.length, "mod needs ", "mods need ") + loaderShort, detail: "The game still plays, without " + (needLoader.length === 1 ? "that one" : "those") }
        const warned = mods.filter(m => m.enabled && !!modWarns[m.id])
        if (warned.length > 0)
            // Short, like every status: the row and Before you play say what is wrong with which mod.
            return { level: "warn", words: plural(warned.length, "mod", "mods") + " may not work here", detail: "Before you play says why" }
        const on = mods.filter(m => m.enabled).length
        // Changes waiting to be saved show nothing (they go in by themselves); while saving, the line says Saving.
        return { level: "ok", words: "Ready to play", detail: !modsLoaded ? "" : mods.length === 0 ? "No mods yet" : plural(on, "mod", "mods") + " on" + (acOffNow ? " · " + acInfo.statusRow : ""),
                 pill: updateCount > 0 ? plural(updateCount, "update", "updates") + " ready"
                       : profileData && profileList.length > 1 ? "Loadout: " + profileData.active : "" }
    }

    // ---------------------------------------------------------------- applying changes
    // Changes go into the game 3 seconds after the last edit, when Turbine loses focus and when it closes.
    Timer { id: autoApply; interval: 3000; onTriggered: if (!shot.on) Turbine.applyNow() }
    function edited() { autoApply.restart() }
    onActiveChanged: windowActive(active)
    // The window came to the front (true) or went to the back (false). The build's input check calls it too.
    function windowActive(active) {
        note("window", active ? "Turbine in front" : "Turbine in the background")
        // Every hold and the D-pad repeat stop both ways: a release made while Turbine was in the background
        // can't be seen here, so nothing may still be counting when it comes back.
        stopHolds(active ? "back in front" : "in the background")
        if (active) {
            // Back from a game: presses made there (still queued) must not act here.
            Turbine.wake()
        } else {
            Turbine.sleep()
            if (!shot.on && Turbine.pending > 0) {
                autoApply.stop()
                Turbine.applyNow()
            }
        }
    }
    onClosing: Turbine.finish()

    // ---------------------------------------------------------------- mod actions
    function toggle(m, sure) {
        const on = !m.enabled
        const users = on ? [] : neededByOn(m)
        if (!on && !sure && users.length > 0) {
            openPick(plural(users.length, "mod needs", "mods need") + " " + m.name, users.map(x => x.name).slice(0, 3).join(", ") + (users.length > 3 ? " and " + (users.length - 3) + " more" : "") + ". They stop working.", [
                { icon: "power", title: "Turn off anyway", detail: "Those mods stay on but won't work.", danger: true, run: () => toggle(m, true) },
                { icon: "back", title: "Keep it on", detail: "", run: () => {} }
            ], "")
            return
        }
        note("action", "Turn " + (on ? "on " : "off ") + m.name)
        if (listShow !== "all" && tab === 0) {
            keptRow = m.id
            if ((listShow === "on" && !on) || (listShow === "off" && on)) toast.info((on ? "Turned on " : "Turned off ") + m.name + ". It leaves this list (Show: " + showNames[listShow] + ") when you move on.")
        }
        mods = mods.map(x => x.id === m.id ? Object.assign({}, x, { enabled: on }) : x)
        Turbine.setEnabled(m.id, on)
        edited()
        if (on && m.needsLoader && loaderMissing) { loaderFor = [m.name]; openLoader() }
    }
    // Touch reorder: with Move on, a tap on another row puts the mod there and sets it down.
    function moveByTap(row) {
        if (!moving || !tapOnce()) return
        const target = listItems[row]
        const to = target && target.kind === "mod" ? mods.findIndex(x => x.id === target.m.id) : -1
        note("touch", "Move by tap: to row " + (row + 1))
        if (to >= 0 && to !== modIndex) stepMove(to - modIndex)
        endMove()
    }
    function moveMod(m, to) {
        const from = mods.findIndex(x => x.id === m.id)
        if (from < 0 || to === from) return
        note("action", "Move " + m.name + " from " + (from + 1) + " to " + (to + 1))
        let list = mods.slice()
        list.splice(from, 1)
        list.splice(to, 0, m)
        mods = list
        // The ring follows the mod's own row (the list on screen may be filtered or sorted; controls #10).
        const row = listItems.findIndex(x => x.kind === "mod" && x.m.id === m.id)
        modIndex = row >= 0 ? row : to
        Turbine.moveTo(m.id, to)
        edited()
    }
    function uninstall(m) {
        note("action", "Uninstall " + m.name)
        const at = Math.max(0, listItems.findIndex(x => x.kind === "mod" && x.m.id === m.id))
        mods = mods.filter(x => x.id !== m.id)
        Turbine.removeMod(m.id)
        edited()
        // The same row, dimmed: A puts it back for 8 seconds; then the focus is on the next row.
        // Uninstalled from somewhere else (a Get mods card): My mods' ring stays put, and a toast says where it went
        //.
        if (tab !== 0 || modPage) { toast.done(m.name + " uninstalled. Removed mods, at the bottom of My mods, puts it back for 7 days."); return }
        ghost = { game: game ? game.id : "", id: m.id, name: m.name, index: at }
        ghostTimer.restart()
        modIndex = at
    }
    // ---------------------------------------------------------------- Recently removed (one collapsed row; a sheet)
    property int removedIndex: 0
    function openRemoved() { removedIndex = 0; sheet = "removed" }
    function removedRun(i) {
        // The mods first, Empty last, so the dangerous row is never the first one.
        if (i === removedMods.length) askDelete("all", "Delete all " + plural(removedMods.length, "removed mod", "removed mods") + " permanently? They can't be put back.")
        else if (removedMods[i]) putBack(removedMods[i].id)
    }
    function removedDelete(i) {
        const m = removedMods[i]
        if (m) askDelete(m.id, "Delete " + m.name + " permanently? It can't be put back.")
    }
    // Hold A to confirm (data that can't come back): the same hold the game menu's Reset uses.
    property string confirmWhat: ""
    property string confirmText: ""
    // 2 seconds for a mod already in Removed mods; 5 for a loadout or profile.
    readonly property int holdNeeded: confirmWhat.indexOf("profile:") === 0 || confirmWhat.indexOf("saves:") === 0 ? 5 : 2
    // What the confirm does, in its title and button: put saves back over the ones there, or delete for good.
    function confirmWords() { return confirmVerb === "delete" ? "delete permanently" : confirmVerb === "remove it" ? "remove the leftover file" : "replace your saves" }
    readonly property string confirmVerb: confirmWhat.indexOf("saves:") === 0 ? "replace them" : confirmWhat.indexOf("acleftover:") === 0 ? "remove it" : "delete"
    function askDelete(what, text) { disarm(); confirmWhat = what; confirmText = text; confirmIndex = 0; stopReset(); sheet = "confirm" }
    property int confirmIndex: 0
    function confirmDone() {
        const w = confirmWhat
        if (w.indexOf("check:") === 0) { note("check", "Delete for good would run now"); sheet = ""; return }
        if (w.indexOf("saves:") === 0) { note("action", "Put back saves without keeping the ones now"); sheet = ""; Turbine.restoreSavesAnyway(w.substring(6)); return }
        if (w.indexOf("acleftover:") === 0) { note("action", "Remove another tool's leftover file: " + w.substring(11)); sheet = ""; Turbine.removeAcLeftover(); return }
        if (w.indexOf("profile:") === 0) {
            const n = w.substring(8)
            note("action", "Delete profile " + n)
            Turbine.profileDelete(n)
            profIndex = 0; sheet = "profiles"
            return
        }
        sheet = removedMods.length > (w === "all" ? removedMods.length : 1) ? "removed" : ""
        if (w === "all") { forgetNames(removedMods.map(x => x.id)); Turbine.purgeAllRemoved(); toast.done("Removed mods deleted permanently.") }
        else { const m = removedMods.find(x => x.id === w); forgetNames([w]); Turbine.purgeRemoved(w); toast.done((m ? m.name : "The mod") + " deleted permanently.") }
        removedIndex = Math.min(removedIndex, Math.max(0, removedMods.length - 1))
    }
    function putBack(id) {
        note("action", "Put back " + id)
        if (ghost && ghost.id === id) { ghost = null; ghostTimer.stop() }
        Turbine.undoRemove(id)
        toast.hide()
        edited()
        if (sheet === "removed" && removedMods.length <= 1) sheet = ""
    }
    // Reorder mode (Y): the D-pad moves the mod, A, B or Y puts it down.
    property bool moving: false
    property int moveFrom: -1
    // In move mode the list is whole and in your order: the mod at modIndex is mods[modIndex].
    property var moveSaved: []
    function startMove() {
        if (!ordered || !focusedItem || focusedItem.kind !== "mod") return
        const id = focusedItem.m.id
        // The dimmed removed row goes first: it shifts the rows under it, and in move mode row n must be mods[n].
        if (ghost) { ghost = null; ghostTimer.stop() }
        if (listFiltered) {
            // Moving needs the whole list in your order: switch to it, keep the same mod focused, then move.
            findText = ""; listShow = "all"; listSort = "order"
            note("action", "Move: the full list in your order")
        }
        // Moving shows the whole list in order (the Dependencies group opens out), so row n is mods[n].
        moving = true
        const i = listItems.findIndex(x => x.kind === "mod" && x.m.id === id)
        if (i < 0 || !mods[i] || mods[i].id !== id) { moving = false; return }
        modIndex = i
        moveFrom = modIndex
        moveSaved = mods.slice()
    }
    // d rows (Up/Down 1, Left/Right 10); LT/RT pass a large number for top or bottom.
    function stepMove(d) {
        const from = modIndex
        const to = Math.max(0, Math.min(mods.length - 1, from + d))
        if (to === from) return
        let list = mods.slice()
        const m = list.splice(from, 1)[0]
        list.splice(to, 0, m)
        mods = list
        modIndex = to
    }
    function endMove() {
        moving = false
        const m = mods[modIndex]
        Qt.callLater(() => { if (m) { const r = listItems.findIndex(x => x.kind === "mod" && x.m.id === m.id); if (r >= 0) modIndex = r } })
        if (modIndex !== moveFrom && m) {
            note("action", "Reorder " + m.name + " from " + (moveFrom + 1) + " to " + (modIndex + 1))
            Turbine.moveTo(m.id, modIndex)
            edited()
        }
        note("focus", "Move done: focus on row " + (modIndex + 1))
    }
    // B: the mod goes back where it was.
    function cancelMove() {
        moving = false
        mods = moveSaved
        modIndex = moveFrom
        const back = mods[moveFrom]
        Qt.callLater(() => { if (back) { const r = listItems.findIndex(x => x.kind === "mod" && x.m.id === back.id); if (r >= 0) modIndex = r } })
        note("action", "Move cancelled")
    }
    // The loader prompt shows once per game while a mod that needs the missing loader is on.
    property string loaderOffered: ""
    property var loaderFor: []
    property bool loaderSourceShown: false
    property int loaderIndex: 0
    function openLoader() { loaderSourceShown = false; loaderIndex = 0; sheet = "loader" }
    // A loader Turbine can't download: A opens its page in Turbine (works in Game Mode), no two steps.
    readonly property var loaderLabels: loaderCanGet ? ["Install", "Not now"] : ["Open its page", "Not now"]
    function loaderRun(i) {
        note("action", "Loader prompt: " + (loaderLabels[i] || i))
        if (i === 0 && loaderCanGet) getLoader()
        else if (i === 0) {
            const src = rules && rules.loader ? rules.loader.source : ""
            sheet = ""
            if (src !== "") openNexus(src.indexOf("://") < 0 ? "https://" + src : src)
        }
        else sheet = ""
    }
    function offerLoader() {
        if (!game || !modsLoaded || loaderOffered === game.id || needLoader.length === 0) return
        if (shot.on && shot.page !== "loader") return
        if (sheet !== "" || page !== "game") return
        // Offered after a press (opening the game, turning a mod on), never out of nowhere.
        if (!pressRecent() && !shot.on) return
        loaderOffered = game.id
        loaderFor = needLoader.map(m => m.name)
        openLoader()
    }

    // ---------------------------------------------------------------- Play
    property int playAsks: 0              // times Play was asked for (the build's checks count them)
    function startPlay(coop) {
        if (!game) return
        playAsks++
        playCoopNext = coop === true
        // An anti-cheat game me3 starts: asked once before the first Play with mods.
        if (acAskNeeded()) { openAcAsk(() => startPlay(coop === true)); return }
        if (acRefused && mods.some(m => m.enabled)) { toast.problem(acInfo.refuseWords); return }
        // Held after a game update: without mods until they update.
        if (modHold && mods.some(m => m.enabled)) {
            note("action", "Play " + game.name + " without mods (held until they update after the game update)")
            toast.act("info", "Mods paused until they update: playing without them.", "Play with them now", () => releaseHold("you asked"))
            Turbine.playOriginal()
            return
        }
        note("action", "Play " + game.name)
        if (!shot.on) { let p = parse(seen.played, []).filter(x => x !== game.id); p.unshift(game.id); seen.played = JSON.stringify(p.slice(0, 50)) }
        if (moving) endMove()
        // Play applies the waiting changes itself; its job runs on the worker after any apply already running, so
        // the timer must not start another one meanwhile.
        autoApply.stop()
        say("Checking before Play...", "work")
        Turbine.check(JSON.stringify(checkCtx()))
    }
    // What was seen and answered is kept by the engine (seen.rs). The values Turbine kept here before are read in
    // once, then cleared. seenRev goes up after each change there, so what reads it is read again.
    property int seenRev: 0
    function adoptSeen() {
        const old = { launchDone: seen.launchDone, onceDone: seen.onceDone, playedWarn: seen.playedWarn, updateWarned: seen.updateWarned,
                      reqQueue: seen.reqQueue, depMoves: seen.depMoves }
        if (!Turbine.adoptSeen(JSON.stringify(old))) return
        seen.launchDone = "[]"; seen.onceDone = "[]"; seen.playedWarn = "[]"; seen.updateWarned = "[]"; seen.reqQueue = ""; seen.depMoves = "{}"
        seenRev++
    }
    // The mods as the engine's answers about them need them: id, the name shown, on or off.
    function modsBrief() { return mods.map(m => ({ id: m.id, name: m.name, enabled: m.enabled })) }
    // The engine shapes the check (precheck.rs): lines already read or answered go, missing requirements become
    // one warning, web addresses come out of the words, and it says whether these same warnings were played
    // through before. The interface opens the sheet or plays.
    readonly property string launchTitle: "One-time Steam setting for the mod loader"
    property string checkKey: ""          // what Play anyway remembers for these warnings with these mods
    // The game and its mods as shown, and whether Steam's Launch Options have the loader's line (read back from
    // Steam; null when they can't be read).
    function checkCtx() {
        const lf = launchFinding(healthNow())
        return { game: game ? game.id : "", mods: modsBrief(), links: linksMap, domain: rules ? rules.domain || "" : "",
                 launchSet: lf ? lf.info === true : null, update: gameUpdate ? String(gameUpdate.now) : null, hold: !!modHold }
    }
    function checked(r) {
        checks = r.checks || []
        checkKey = r.key || ""
        note("engine", "Check before Play: " + checks.map(x => x.level + " " + x.title).join("; "))
        note("engine", "Check before Play buttons: " + checkChoices.join("; "))
        if (msgLevel === "work" && msgText === "Checking before Play...") msgText = ""
        // Warnings already played through with these same mods: a note after launch instead of a stop.
        if (r.trouble && r.known && shot.page.indexOf("check") !== 0) {
            note("engine", "Check before Play: the same warnings were played through with these mods: playing, then a note")
            playNow()
            toast.info("Played with the same warnings as last time: " + checks.filter(x => x.level === "warn").map(x => x.title).join("; "))
            return
        }
        if (r.trouble || shot.page === "check" || shot.page === "checkend" || shot.page === "checkfocus") {
            // The highlight starts on the fix; else Not now when something stops play; else Play anyway.
            checkIndex = checkOffer ? 0 : checkStops ? checkChoices.indexOf("Not now") : Math.max(0, checkChoices.indexOf(checkPlayWord))
            sheet = "check"
        }
        else playNow()
    }
    readonly property bool checkStops: checks.some(x => x.level === "stop")
    readonly property string vcTitle: "The Visual C++ runtime is missing"
    // One fix the check can do in a press (Y): turn off the newer of two Witcher 3 mods that change the same script,
    // or install the missing loader (SKSE, BepInEx, SMAPI).
    readonly property var checkOffer: {
        const w = checks.find(x => x.offer && x.offer.mod && mods.some(m => m.id === x.offer.mod && m.enabled))
        if (w) return { label: w.offer.label, mod: w.offer.mod }
        // Cyberpunk's frameworks without Microsoft's Visual C++ runtime in the game's Proton setup (error 998).
        if (checks.some(x => x.title === vcTitle)) return { label: "Install the Visual C++ runtime", vc: true }
        if (checks.some(x => x.title === reqCheckTitle) && (reqOnAll().get.length > 0 || reqOnAll().off.length > 0)) return { label: "Get what they need", reqs: true }
        if (loaderMissing && needLoader.length > 0) return { label: (loaderCanGet ? "Install " : "Get ") + loaderShort, loader: true }
        // Low space, and Turbine is keeping removed mods: the way to free that space is one press.
        if (removedMods.length > 0 && checks.some(x => x.title === "Low space on the game's drive")) return { label: "Free space: Removed mods", removed: true }
        return null
    }
    // Fix it, Play anyway, Play without mods, Not now. A runs the highlighted one, B is Not now, Start Play anyway.
    // "Play anyway" only when something is wrong; with every line fine it is plain Play.
    readonly property string checkPlayWord: checks.some(x => x.level !== "ok") ? "Play anyway" : "Play"
    readonly property var checkChoices: (checkOffer ? [checkOffer.label] : []).concat(checkStops ? [] : [checkPlayWord]).concat(["Play without mods", "Not now"])
    // The lines as the sheet shows them: the one the fix button acts on first, so the main button never refers to a
    // line further down.
    readonly property var checksShown: {
        const o = checkOffer
        if (!o) return checks
        const lead = x => o.mod ? !!(x.offer && x.offer.mod === o.mod) : o.vc ? x.title === vcTitle : o.reqs ? x.title === reqCheckTitle
                          : o.removed ? x.title === "Low space on the game's drive" : false
        const i = checks.findIndex(lead)
        return i > 0 ? [checks[i]].concat(checks.slice(0, i), checks.slice(i + 1)) : checks
    }
    // Script paths from the last check (Game details shows them).
    readonly property var checkPaths: checks.filter(x => x.paths && x.paths.length > 0).map(x => x.paths.join(", "))
    property int checkIndex: 0
    function checkChoose(i) {
        const what = checkChoices[i]
        note("action", "Check before Play: " + what)
        sheet = ""
        if (checks.some(x => x.title === launchTitle) && what !== "Not now") Turbine.markLaunchSeen(game.id)
        // Other read-once lines (REDmod): seen once the player moves on from the check.
        const once = checks.filter(x => x.once && x.title !== launchTitle)
        if (once.length > 0 && what !== "Not now" && !shot.on) Turbine.markOnceSeen(game.id, JSON.stringify(once.map(x => x.title)))
        if (what === checkPlayWord) {
            if (!shot.on) Turbine.markPlayed(checkKey)
            playNow()
        }
        else if (what === "Play without mods") { playCoopNext = false; Turbine.playOriginal() }
        else if (checkOffer && what === checkOffer.label) runOffer(checkOffer)
        else playCoopNext = false
    }
    function runOffer(o) {
        if (o.mod) {
            // Turned off, then checked again: Play goes on when nothing else is in the way.
            const m = mods.find(x => x.id === o.mod)
            if (m && m.enabled) toggle(m, true)
            startPlay()
        } else if (o.loader) getLoader()
        else if (o.removed) openRemoved()
        else if (o.reqs) { const r = reqOnAll(); reqStart({ name: r.names[0] || "your mods" }, r) }
        else if (o.vc) { note("action", "Install the Visual C++ runtime for " + gameTitle); if (!shot.on) Turbine.installVcRuntime() }
    }
    // Install the missing loader (from the check, the mod menu or the game menu); where it can't be fetched, the
    // prompt says where it comes from.
    function getLoader() {
        loaderFor = needLoader.map(m => m.name)
        if (loaderCanGet) { sheet = ""; note("action", "Install the loader"); Turbine.installLoader() }
        else openLoader()
    }

    // ---------------------------------------------------------------- the game closed right after Play
    // The engine watches the game's process after a modded Play (gameEnded). Under a minute means a mod likely broke
    // it: Turbine offers the way back when the player returns. Without a process to watch, the time Turbine was away
    // stands in for it.
    property real playAt: 0
    property real awayAt: 0
    property string measureAway: ""
    property var quick: null              // {game, seconds, added: [mods], worked: setup or null}
    Connections {
        target: app
        function onActiveChanged() {
            if (!app.active && app.playAt > 0 && Date.now() - app.playAt < 120000) app.awayAt = Date.now()
            else if (app.active && app.measureAway !== "" && app.awayAt > 0) {
                const secs = Math.round((Date.now() - app.awayAt) / 1000)
                const g = app.measureAway
                app.measureAway = ""; app.awayAt = 0
                app.note("launch", "Back after " + secs + " seconds away")
                if (secs < 60) app.quickExit(g, secs)
            }
            if (app.active && app.quick && app.sheet === "") { app.infoIndex = 0; app.sheet = "quick" }
        }
    }
    function quickExit(gameId, secs) {
        const i = games.findIndex(g => g.id === gameId)
        if (i < 0) return
        const shortName = i === current ? gameTitle : fullGame(games[i].name)
        if (i !== current) current = i
        note("launch", games[i].name + " closed after " + secs + " seconds: offering a way back")
        // The setup that worked: the newest played one that isn't what just closed (store.rs), and the mods on now
        // that it didn't have on.
        const back = parse(Turbine.quickExitJson(JSON.stringify({ setups: setups, mods: mods.map(m => ({ id: m.id, enabled: m.enabled })) })), { worked: null, added: [] })
        const worked = back.worked
        const added = modsOf(back.added)
        if (!shot.on) Turbine.markNotWorking()
        const tool = steamTool(gameId)
        quick = { game: gameId, name: shortName, seconds: secs, added: added, worked: worked, proton: tool.lastProton || "", nothing: tool.nothingStarted || "" }
        page = "game"; zone = "main"; tab = 0; modPage = false
        // It started and closed: a "didn't start" sheet still up for it gives way to "closed after N seconds".
        if (notStartedShowing(gameId)) { note("launch", "The didn't start sheet gives way: it started, then closed"); sheet = "" }
        notStartedFor = ""
        if (app.active && sheet === "") { infoIndex = 0; sheet = "quick" }
    }
    readonly property string quickText: !quick ? "" : (quick.added.length === 1 ? "Since it last worked, you added " + quick.added[0].name + "."
        : quick.added.length > 1 ? "Since it last worked (" + when(quick.worked.savedAt) + "), you added or turned on " + nameList(quick.added.map(m => m.name)) + "."
        : "It started, then closed unexpectedly. A mod is the likely cause: playing without mods shows whether the game itself is fine.")
        // Which Proton Turbine's Steam tool ran it with, when the tool said.
        + (quick.proton ? " " + quick.proton : "")
    // "Bright Torches, Faster Roach and 1 more"
    function nameList(names) {
        if (names.length <= 1) return names.join("")
        if (names.length === 2) return names[0] + " and " + names[1]
        return names[0] + ", " + names[1] + " and " + (names.length - 2) + " more"
    }
    function quickRun(label) {
        const q = quick
        quick = null
        sheet = ""
        if (!q) return
        note("action", "Closed early: " + label)
        if (label === "Turn that off and play") { const m = mods.find(x => x.id === q.added[0].id); if (m && m.enabled) toggle(m, true); startPlay() }
        else if (label === "Turn off the newest and play") {
            const newest = q.added.slice().sort((a, b) => (b.added || 0) - (a.added || 0))[0]
            const m = newest ? mods.find(x => x.id === newest.id) : null
            if (m && m.enabled) { note("action", "Turn off the newest: " + m.name); toggle(m, true) }
            startPlay()
        }
        else if (label.indexOf("Go back to the setup that worked") === 0) { Turbine.restoreSetup(q.worked.id); startPlay() }
        else if (label === "Play without mods") Turbine.playOriginal()
    }
    // Free accounts: what a mod needs, one page at a time ("Next: Address Library").
    property var chainQueue: []
    property string chainAfter: ""
    function showChain(arrived) {
        if (chainQueue.length === 0) return
        chainAfter = arrived
        if (sheet === "") { infoIndex = 0; sheet = "chain" }
    }

    // ---------------------------------------------------------------- Requirements, lined up
    // A mod that needs others: one sheet, "This mod needs N others", with Get them all. The whole chain is followed
    // (each mod got adds what it needs in turn), installed mods that are off are turned on, and the rest are got one
    // after another: with Nexus Premium (or Thunderstore) with no press, with a free account on the mod's own file
    // page, lined up on its Slow download button; after that press and the add, the next page opens by itself. A
    // strip says "Getting 2 of 3: <name>" with Skip and Stop. The list is kept (Later, Stop, closing Turbine) and My
    // mods keeps a row to go on where it left off. Where requirements come from: each mod author's list on Nexus.
    property bool reqRunning: false
    property var reqLookFor: null         // {id, name}: a mod just added, looked at once its requirements are known
    // The list being got is kept by the engine (needs.rs ReqQueue, in seen.json).
    readonly property var reqQ: { seenRev; const q = parse(Turbine.reqQueueJson(), null); return q && q.items ? q : null }
    readonly property bool reqLeft: !!reqQ && !!game && reqQ.game === game.id && reqQ.index < reqQ.items.length
    readonly property var reqItem: reqQ && reqQ.index < reqQ.items.length ? reqQ.items[reqQ.index] : null
    readonly property string reqStripWords: reqRunning && reqItem ? "Getting " + (reqQ.index + 1) + " of " + reqQ.items.length + ": " + reqItem.name : ""
    function reqStartQueue(forName, items) { Turbine.reqQueueStart(game.id, forName, JSON.stringify(items)); seenRev++ }
    function reqClear() { Turbine.reqQueueClear(); seenRev++ }
    // Requirements as the engine reads them (needs.rs): each mod's missing needs and what of them is installed but
    // off (turned on by Get them all) or not installed (got), and what the mods that are on miss together.
    readonly property var needsView: parse(Turbine.needsJson(JSON.stringify({ mods: modsBrief(), links: linksMap, domain: rules ? rules.domain || "" : "" })), null)
                                     || { missing: {}, split: {}, all: { off: [], get: [], names: [], words: "" } }
    function modsOf(ids) { return ids.map(id => mods.find(x => x.id === id)).filter(x => !!x) }
    function reqSplit(m) { const r = needsView.split[m.id]; return r ? { off: modsOf(r.off), get: r.get } : { off: [], get: [] } }
    Timer { id: reqLook; interval: 900; onTriggered: app.reqLookNow() }
    function reqLookNow() {
        const w = reqLookFor
        if (w && sheet !== "" && !reqRunning) { reqLook.restart(); return }
        reqLookFor = null
        if (!w || !game) return
        const m = mods.find(x => x.id === w.id)
        if (!m) return
        const r = reqSplit(m)
        if (r.off.length === 0 && r.get.length === 0) { if (reqRunning) reqNextSoon(); return }
        if (reqRunning && reqQ) {
            // The chain: what this one needs comes right after it, before the rest of the list.
            for (const x of r.off) if (!x.enabled) toggle(x, true)
            const fresh = parse(Turbine.reqQueueInsertNext(JSON.stringify(r.get)), [])
            seenRev++
            note("action", "Requirements: " + m.name + " needs " + plural(fresh.length, "more mod", "more mods") + ": " + fresh.map(x => x.name).join(", "))
            reqNextSoon()
            return
        }
        reqOffer(m, r)
    }
    function reqOffer(m, r) {
        const n = r.off.length + r.get.length
        note("action", "Requirements: " + m.name + " needs " + n + ": " + r.off.map(x => x.name + " (off)").concat(r.get.map(x => x.name)).join(", "))
        openMsg({ title: "This mod needs " + (n === 1 ? "1 other" : n + " others"), reason: "requirements",
                  text: m.name + " needs these to work. Get them all gets each one in turn" + (account.premium === true ? "." : ": for each, press Slow download on its page and the next one opens automatically."),
                  notes: r.off.map(x => x.name + ": installed, turned off (it is turned on)").concat(r.get.map(x => x.name + ": not installed")),
                  footnote: "From the list the mod's author keeps on Nexus.",
                  buttons: [{ key: "a", label: "Get them all", run: () => reqStart(m, r) },
                            { key: "b", label: "Not now", run: () => { reqStartQueue(m.name, r.get); for (const x of r.off) if (!x.enabled) toggle(x, true); note("action", "Requirements: later (My mods keeps a row)") } }] })
    }
    function reqStart(m, r) {
        for (const x of r.off) if (!x.enabled) toggle(x, true)
        if (r.get.length === 0) { toast.done("Turned on what " + m.name + " needs."); return }
        reqStartQueue(m.name, r.get)
        reqRunning = true
        note("action", "Requirements: get them all for " + m.name + " (" + r.get.map(x => x.name).join(", ") + ")")
        reqNext()
    }
    function reqGoOn() {
        if (!reqLeft) return
        reqRunning = true
        note("action", "Requirements: go on where it left off (" + (reqQ.index + 1) + " of " + reqQ.items.length + ")")
        reqNext()
    }
    Timer { id: reqNextTimer; interval: 900; onTriggered: if (app.reqRunning) app.reqNext() }
    function reqNextSoon() { reqNextTimer.restart() }
    function reqNext() {
        if (!reqRunning) return
        const q = reqQ
        if (!q || !game || q.game !== game.id) { reqRunning = false; return }
        if (q.index >= q.items.length) { reqFinish(); return }
        const it = q.items[q.index]
        // Got meanwhile (another way): the next one.
        if (mods.some(x => linksMapHas(x, it.modId))) { reqAdvance("already there"); return }
        note("action", "Requirements: getting " + (q.index + 1) + " of " + q.items.length + ": " + it.name)
        // The build's checks stop here (no internet): the step is in the log.
        if (shot.on) return
        if (account.premium === true && it.domain === (rules ? rules.domain || "" : "")) {
            Turbine.install(String(it.modId), "nexus")
            edited()
            return
        }
        if (it.domain && rules && it.domain !== rules.domain) { openNexus("https://www.nexusmods.com/" + it.domain + "/mods/" + it.modId + "?tab=files", "page"); return }
        page = "game"; tab = 1; zone = "main"
        openDetails({ id: String(it.modId), name: it.name, source: "nexus", icon: "" }, modPage)
        pendingGet = true
    }
    // An installed mod that is this Nexus mod (its links entry names it as a provider for another mod, or its id).
    function linksMapHas(m, modId) { return nexusModOf(m) === String(modId) }
    function reqAdvance(why) {
        if (!reqQ) return
        const done = parse(Turbine.reqQueueAdvance(), null)
        seenRev++
        note("action", "Requirements: " + (done ? done.name : "") + " " + why)
        reqNextSoon()
    }
    function reqSkip() {
        if (!reqRunning) return
        if (browserOpen) closeBrowser("back")
        if (modPage) { modPage = false; history = [] }
        reqAdvance("skipped")
    }
    function reqStop() {
        if (!reqRunning) return
        reqRunning = false
        reqNextTimer.stop()
        if (browserOpen) closeBrowser("back")
        note("action", "Requirements: stopped at " + (reqQ ? (reqQ.index + 1) + " of " + reqQ.items.length : "the end"))
        toast.info("Stopped. My mods keeps a row to go on where you left off.")
    }
    function reqFinish() {
        const q = reqQ
        reqRunning = false
        reqClear()
        if (modPage) { modPage = false; history = [] }
        page = "game"; tab = 0; zone = "play"
        note("action", "Requirements: done for " + (q ? q.forName : ""))
        toast.done("Got everything " + (q ? q.forName : "it") + " needs. Hold " + glyph("start") + " to play.")
    }
    // Every mod that is on: what they miss together (each once), and words naming who needs what.
    readonly property string reqCheckTitle: "Mods need others that aren't on"
    function reqOnAll() { const a = needsView.all; return { off: modsOf(a.off), get: a.get, names: a.names, words: a.words } }
    // A mod arrived: its requirements are looked at once they are known (the list comes a moment after the add).
    function reqArrived(id, name) {
        reqLookFor = { id: id, name: name }
        reqLook.restart()
    }

    // ---------------------------------------------------------------- game menu
    // The setup to go back to: the newest one the game was played with (the engine's restore target, lastGood
    // in modsJson).
    property string lastGoodId: ""
    readonly property var lastGood: setups.find(x => x.id === lastGoodId) || null
    function when(sec) { return Qt.formatDateTime(new Date(sec * 1000), "MMM d, h:mm AP") }
    // Reset shows only when the engine found records it couldn't read for this game. It runs after A is held 5 s.
    readonly property bool canReset: game !== null && (Turbine.resetFor === game.id || shot.page === "inputcheck")
    property real resetHeld: 0
    Timer {
        id: resetHold
        interval: 100
        repeat: true
        onTriggered: {
            // Only while a real press is held (A, or a finger or the pointer on the row).
            if (!app.holdAllowed()) { app.stopReset(); return }
            app.resetHeld += 0.1
            if (app.resetHeld >= 5) {
                stop(); app.resetHeld = 0; app.sheet = ""
                app.holdsDone++
                if (shot.page === "inputcheck") app.note("check", "Reset would run now")
                else Turbine.resetGame()
            }
        }
    }
    function stopReset() { resetHold.stop(); resetHeld = 0 }
    // A real press is down: A (pad or Enter), or a finger or the pointer held on a hold button. A tap, a click,
    // R3 or a press whose release was lost never runs a hold to its end.
    property bool aHeld: false
    property bool clickHeld: false
    property int holdsDone: 0
    function holdAllowed() { return aHeld || clickHeld }
    // Confirm by: Press twice: the first press says what a second one does; a second within 4 seconds does it.
    readonly property bool pressTwice: look.confirmBy === "twice"
    // The first press arms one exact thing (the reset of one game, or the delete of one item) on one screen; any
    // move, sheet, page, tab or game change clears it, so a second press can only ever do what the first said.
    property string armedKey: ""
    property real armedAt: 0
    function disarm() { if (armedKey !== "") { armedKey = ""; note("action", "Press twice cleared") } }
    function resetKey() { return "reset|" + (game ? game.id : "") }
    function deleteKey() { return "delete|" + sheet + "|" + confirmWhat }
    onMenuIndexChanged: disarm()
    onConfirmIndexChanged: disarm()
    function confirmTwice(key, what, run) {
        if (armedKey === key && Date.now() - armedAt < 4000) { armedKey = ""; note("action", "Pressed twice: " + what); run(); return }
        armedKey = key; armedAt = Date.now()
        note("action", "First press: " + what)
        // The game menu's row and the confirm sheet's button say it already: no toast over the bottom bar.
        if (sheet !== "gamemenu" && sheet !== "confirm") toast.info("Press A again to " + what + ".")
    }
    function holdHint(what) { toast.info("Hold A for " + what + ". A tap doesn't do it.") }
    // Stops every hold and the D-pad repeat: the window going to the back, a controller going away.
    function stopHolds(why) {
        const was = resetHold.running || confirmHold.running || startHold.running || repeater.running
        stopReset(); stopConfirm()
        startHold.stop(); startHeld = 0
        repeater.stop(); repeatButton = ""
        aHeld = false; clickHeld = false
        if (was) note("pad", "Holds stopped (" + why + ")")
    }
    // The danger holds stop whenever the sheet or the page changes.
    function stopDangerHolds() { stopReset(); stopConfirm(); startHold.stop(); startHeld = 0; clickHeld = false; disarm() }
    // (onPageChanged and onTabChanged, near the top, call it too.)
    onModPageChanged: {
        stopDangerHolds(); pageSettings = false
        // A mod's page comes in from the right over Get mods; going back, the cards come in from the left.
        if (modPage) modIn.play(px(mo.shift), 0)
        else if (tab === 1 && turnFrom !== 1) gridBack.play(-px(mo.shift), 0)
    }
    readonly property var gameMenuItems: [
        { icon: "play", title: "Play without mods", detail: acMode === "me3" ? "Starts it the normal way, with its anti-cheat. Nothing is deleted." : "Starts the game with no mods. Nothing is deleted.", ok: true, act: "original" },
        // One row for both ways back: the last setup that worked, or (with more saved) the list, newest first.
        { icon: "back", title: "Go back to a setup that worked",
          detail: lastGood ? "Last: " + when(lastGood.savedAt) + " · " + plural(lastGood.modsOn, "mod", "mods") + " on" + (setups.length > 1 ? " · " + plural(setups.length, "setup", "setups") + " saved" : "")
                           : "Nothing saved yet. A setup is saved each time you play with mods.",
          ok: lastGood !== null, act: "restore" }
    ].concat(canReset ? [
        { icon: "restore", title: "Reset game files", act: "reset", danger: true, ok: true,
          detail: pressTwice ? (armedKey === resetKey() ? "Press A again to reset" : "Press A twice. Takes every mod file out and puts the game's own files back. Mods stay listed, turned off.")
                  : resetHold.running ? "Keep holding A: " + Math.max(1, Math.ceil(5 - resetHeld)) + "..."
                  : "Hold A for 5 seconds. Takes every mod file out and puts the game's own files back. Mods stay listed, turned off." }
    ] : []).concat(loaderMissing ? [
        { icon: "update", title: (loaderCanGet ? "Install " : "Get ") + loaderShort, detail: needLoader.length > 0 ? plural(needLoader.length, "mod needs", "mods need") + " it" : "The mod loader for " + gameTitle, ok: true, act: "loader" }
    ] : []).concat(kcdRestore ? [
        { icon: "restore", title: "Put back my mod_order.txt", detail: "As it was before Turbine changed it. Turbine re-adds its own mods that are on at your next change or Play.", ok: true, act: "kcdrestore" }
    ] : []).concat([
        { icon: "profile", title: "Loadouts", detail: profileData && profileList.length > 1 ? "In use: " + profileData.active : "Save setups by name, switch, share", ok: true, act: "profiles" }
    ]).concat(acMode === "me3" && acInfo.needsAsk ? [
        acInfo.off ? { icon: "power", title: "Anti-cheat: off while mods are on", detail: "A turns it back on: mods wait until you answer again", ok: true, act: "acon" }
                   : { icon: "info", title: acInfo.askTitle, detail: "Mods load through Mod Engine 3 (a free tool), offline only", ok: true, act: "acask" }
    ] : []).concat(acMode === "me3" && !acInfo.me3Ready ? [
        { icon: "update", title: "Get Mod Engine 3, the tool that loads mods", detail: "From its makers, checked. Turbine gets it at the first Play with mods anyway.", ok: true, act: "me3" }
    ] : []).concat(acMode === "me3" && acInfo.seamless && !seamlessIn ? [
        { icon: "update", title: "Get Seamless Co-op", detail: "From its makers, checked. Co-op keeps its own saves, apart from solo.", ok: true, act: "seamless" }
    ] : []).concat(acInfo && acInfo.leftover ? [
        { icon: "remove", title: "Remove the leftover file", detail: acInfo.leftover + ", left by another tool. Hold A.", ok: true, act: "acleftover" }
    ] : []).concat(modHold ? [
        { icon: "play", title: "Play with mods again", detail: "Play has started it without mods since the game updated", ok: true, act: "unhold" }
    ] : []).concat(seamlessIn ? [
        { icon: "info", title: "Co-op settings", detail: "Seamless Co-op: the session password, then every other option", ok: true, act: "coopsettings" }
    ] : []).concat(seamlessIn && game && String(game.id).indexOf("steam:") === 0 ? [
        !coopTileOffered ? null
        : coopTileThere ? { icon: "remove", title: "Remove the Co-op tile from Steam", detail: "Takes it out of your Steam library" + (coopTileHow !== "" ? " (" + coopTileHow + ")" : ""), ok: true, act: "cooptile" }
                      : { icon: "play", title: "Add a Co-op tile to Steam", detail: "Seamless Co-op in one press from your Steam library" + (coopTileHow !== "" ? " (" + coopTileHow + ")" : ""), ok: true, act: "cooptile" }
    ] : []).concat(acMode === "me3" && game && String(game.id).indexOf("steam:") === 0 ? [
        { icon: "restore", title: "Undo the launch options change", detail: "Shows what to paste back in Steam", ok: true, act: "launchundo" }
    ] : []).concat(acMode === "me3" && acInfo.seamless ? [
        { icon: "restore", title: "Saves: normal, modded and co-op", detail: "Each kind apart, with its backups", ok: true, act: "ersaves" }
    ] : []).concat(acMode === "me3" || acRefused ? [] : [
        { icon: "info", title: "Edit settings", detail: gameTitle + "'s own settings files", ok: true, act: "gamesettings" }
    ]).concat(acMode === "official" ? [
        { icon: "info", title: "How to play " + gameTitle + " with mods", detail: "Its own switch in Steam's Play menu", ok: true, act: "acguide" }
    ] : []).concat(overlapCount > 0 ? [
        { icon: "info", title: "Overlaps: " + plural(overlapCount, "mod shares", "mods share") + " files", detail: "Which mod's copy the game gets", ok: true, act: "overlaps" }
    ] : []).concat(updateCount > 0 ? [
        { icon: "update", title: "Update all", detail: plural(updateCount, "mod has", "mods have") + " a newer version", ok: true, act: "updateall" }
    ] : []).concat(webDls.length > 0 && !browserOpen ? [
        { icon: "update", title: "Nexus downloads", detail: webDlsRunning > 0 ? plural(webDlsRunning, "download", "downloads") + " running · added automatically" : "What Turbine's Nexus page downloaded", ok: true, act: "nexusdl" }
    ] : []).concat([
        // The obvious actions for a game.
        { icon: "play", title: "Mods for this game on Nexus", detail: rules && rules.domain ? "Opens its mod list in Turbine" : "Turbine doesn't know this game's Nexus page", ok: !!(rules && rules.domain), act: "nexuslist" },
        { icon: "update", title: "Check for mod updates now", detail: "Asks Nexus and Thunderstore", ok: mods.some(m => m.sourceKind === "nexus" || m.sourceKind === "thunderstore"), act: "checkupdates" },
        { icon: "files", title: "Add from Downloads", detail: "A mod file you downloaded", ok: true, act: "add" },
        outdatedMods.length > 0 ? { icon: "info", title: plural(outdatedMods.length, "mod may be", "mods may be") + " out of date", detail: outdatedMods.slice(0, 2).map(m => m.name).join(", "), ok: true, act: "outdated" }
                                : null,
        { icon: "info", title: "Why won't it start?", detail: "Checks the usual reasons, in plain words", ok: true, act: "why" },
        { icon: "info", title: "Check game folder", detail: "Are Turbine's files still all there?", ok: true, act: "verify" },
        { icon: "restore", title: "Saves: put back a backup", detail: "A copy is kept each time you play with mods", ok: true, act: "saves" },
        allOffWas.length > 0 && !mods.some(m => m.enabled)
            ? { icon: "power", title: "Turn them back on", detail: plural(allOffWas.length, "mod", "mods") + " you turned off together", ok: true, act: "allon" }
            : { icon: "power", title: "Turn every mod off", detail: "Turn them back on from here later", ok: mods.some(m => m.enabled), act: "alloff" },
        protonNow ? { icon: "play", title: protonTitle(protonNow), detail: "The Proton Turbine's Steam tool runs it with: " + protonNow.words, ok: true, act: "proton" } : null,
        { icon: "info", title: "Game details", detail: "What Turbine found for this game", ok: true, act: "details" },
        { icon: "info", title: "Activity log", detail: "What Turbine did, to read, copy or save", ok: true, act: "activity" },
        { icon: "info", title: "Help", detail: "The buttons and the beginner tutorial", ok: true, act: "help" }
    ]).filter(x => !!x).map(x => Object.assign({}, x, { group: gameMenuGroup(x.act) }))
      .sort((a, b) => gameMenuOrder.indexOf(a.act) - gameMenuOrder.indexOf(b.act))
    // The game menu in four sections, always in this order (rows that don't apply are left out; rows that
    // apply but can't be used now are shown dimmed, and A on one says why).
    readonly property var gameMenuOrder: ["unhold", "coopsettings", "cooptile", "original", "restore", "acguide", "acask", "acon",
        "add", "updateall", "checkupdates", "alloff", "allon", "profiles", "overlaps", "outdated", "nexusdl", "nexuslist",
        "why", "verify", "saves", "ersaves", "loader", "me3", "seamless", "acleftover", "kcdrestore", "launchundo", "reset",
        "gamesettings", "proton", "details", "activity", "help"]
    function gameMenuGroup(a) {
        const i = gameMenuOrder.indexOf(a)
        return i < gameMenuOrder.indexOf("add") ? "Play" : i < gameMenuOrder.indexOf("why") ? "Mods" : i < gameMenuOrder.indexOf("gamesettings") ? "Fix a problem" : "This game"
    }
    // ---------------------------------------------------------------- Health
    function healthNow() { return parse(Turbine.gameHealth(), {}) }
    // Launch option doctor: what the game needs in Steam's Launch Options, what is set, and the steps.
    function launchFinding(h) {
        const need = (h.launch && h.launch.needed) || "", set = (h.launch && h.launch.set) || ""
        if (need === "") return null
        if (h.launch.ok === true || (h.launch.ok === undefined && set.indexOf(need.replace(/ ?%command%/, "").trim()) >= 0)) return { icon: "power", title: "Steam's Launch Options: set", detail: "Steam has what " + gameTitle + " needs", info: true, run: () => {} }
        // Settings typed in front of Turbine's line without "export" stop Steam starting the game at all.
        if (h.launch.fix === true)
            return { icon: "info", title: "Steam's Launch Options: a setting in front breaks the line", detail: "Steam starts nothing like this. A copies the same line with export in front. Then in Steam: " + gameTitle + ", the gear, Properties, Launch Options, paste.",
                     run: () => { Turbine.copyText(need); note("action", "Launch option fix copied: " + need); toast.done("Copied. Paste it in Steam: Properties, Launch Options.") } }
        // The steps, not the 150 character line; the line is what A copies.
        return { icon: "info", title: set === "" ? "Steam's Launch Options: line missing" : "Steam's Launch Options: line changed", detail: "A copies the line. Then in Steam: " + gameTitle + ", the gear, Properties, Launch Options, paste.",
                 run: () => { Turbine.copyText(need); note("action", "Launch option copied: " + need); toast.done("Copied. Paste it in Steam: Properties, Launch Options.") } }
    }
    function openWhy() {
        const h = healthNow()
        let rows = []
        if (loaderMissing) rows.push({ icon: "update", title: loaderShort + " isn't installed", detail: "Mods that need it stop the game at start. A gets it.", run: () => { sheet = ""; getLoader() } })
        const lf = launchFinding(h); if (lf) rows.push(lf)
        if (game) rows = rows.concat(toolRows(game.id))
        if (h.verify && (h.verify.missing.length + h.verify.changed.length) > 0)
            rows.push({ icon: "info", title: plural(h.verify.missing.length + h.verify.changed.length, "mod file is", "mod files are") + " missing or changed", detail: "A puts them back", run: () => { sheet = ""; Turbine.applyNow(); toast.info("Putting the mod files back...") } })
        for (const c of (h.copies || [])) rows.push({ icon: "info", title: "Another copy of " + gameTitle, detail: c + ". Mods go only into the one shown here.", info: true, run: () => {} })
        // The bottom bar's warnings, each with its reason.
        for (const m of mods.filter(m => m.enabled && !!modWarns[m.id]).slice(0, 4))
            rows.push({ icon: "info", title: m.name, detail: modWarns[m.id], info: true, run: () => {} })
        if (rules && rules.antiCheat) rows.push({ icon: "info", title: "Anti-cheat: " + rules.antiCheat.name, detail: "Online play with mods can get you banned", info: true, run: () => {} })
        const on = mods.filter(m => m.enabled).length
        if (on > 0) rows.push({ icon: "play", title: "Try it without mods", detail: "Play without mods. If it starts, a mod is the cause.", run: () => { sheet = ""; Turbine.playOriginal() } })
        if (lastGood) rows.push({ icon: "restore", title: "Go back to a setup that worked", detail: "Last: " + when(lastGood.savedAt), run: () => { sheet = ""; Turbine.restoreSetup("") } })
        if (rows.length === 0) rows.push({ icon: "power", title: "Nothing found", detail: "Turbine sees no reason it shouldn't start", info: true, run: () => {} })
        note("action", "Why won't it start: " + rows.map(r => r.title).join("; "))
        openPick("Why won't it start?", gameTitle, rows, "gamemenu")
    }
    function openVerify() {
        const h = healthNow(), v = h.verify
        if (!v) { toast.info("Turbine hasn't put anything in this game yet."); return }
        const bad = v.missing.length + v.changed.length
        note("action", "Check game folder: " + v.ok + " fine, " + v.missing.length + " missing, " + v.changed.length + " changed")
        openPick("Check game folder", bad === 0 ? plural(v.ok, "mod file", "mod files") + " in place. Nothing to fix." : plural(bad, "file needs", "files need") + " fixing. A on the first row puts them back.",
            (bad > 0 ? [{ icon: "update", title: "Put them back", detail: "Puts mod files back. Game files are kept, never deleted", run: () => { sheet = ""; Turbine.applyNow(); toast.info("Putting the mod files back...") } }] : [])
            .concat(v.missing.slice(0, 20).map(f => ({ icon: "info", title: f, detail: "Missing", info: true, run: () => {} })))
            .concat(v.changed.slice(0, 20).map(f => ({ icon: "info", title: f, detail: "Changed since Turbine put it in", info: true, run: () => {} }))), "gamemenu")
    }
    function openSaves() {
        const v = parse(Turbine.savesJson(), { folders: 0, backups: [] })
        if (v.folders === 0) { toast.info("Turbine doesn't know where " + gameTitle + " keeps its saves yet."); return }
        openPick("Saves: put back a backup", v.backups.length === 0 ? "No backups yet. One is made each time you play with mods." : "The saves as they are now are kept first.",
            v.backups.map(b => ({ icon: "restore", title: when(b.when), detail: plural(b.files, "file", "files") + " · " + sizeWords(b.bytes), aLabel: "Put back",
                run: () => { note("action", "Put back saves from " + when(b.when)); Turbine.restoreSaves(b.id) } })), "gamemenu")
    }
    // Turn every mod off, with Undo on the toast (each one back on, as it was).
    function allOff() {
        if (!game) return
        const was = mods.filter(m => m.enabled).map(m => m.id)
        if (was.length === 0) return
        note("action", "Turn every mod off (" + was.length + ")")
        for (const id of was) Turbine.setEnabled(id, false)
        mods = mods.map(x => Object.assign({}, x, { enabled: false }))
        allOffWas = was
        toast.act("done", plural(was.length, "mod", "mods") + " turned off.", "Undo", allBackOn)
    }
    property var allOffWas: []
    function allBackOn() {
        const ids = allOffWas.filter(id => mods.some(m => m.id === id))
        allOffWas = []
        note("action", "Turn them back on (" + ids.length + ")")
        for (const id of ids) Turbine.setEnabled(id, true)
        mods = mods.map(x => ids.indexOf(x.id) >= 0 ? Object.assign({}, x, { enabled: true }) : x)
        toast.done(plural(ids.length, "mod", "mods") + " back on.")
    }
    function openSetups() {
        const list = setups.slice().sort((a, b) => b.savedAt - a.savedAt)
        openPick("Go back to a setup that worked", gameTitle, list.map(x => ({
            icon: "restore", title: when(x.savedAt), detail: plural(x.modsOn, "mod", "mods") + " on" + (x.played ? " · Played" : ""),
            run: () => { note("action", "Go back to the setup of " + when(x.savedAt)); Turbine.restoreSetup(x.id) } })), "gamemenu")
    }
    function gameMenuRun(i) {
        const item = gameMenuItems[i]
        if (!item) return
        // A dimmed row says why it can't be used.
        if (!item.ok) { note("action", "Game menu: " + item.title + " is dimmed: " + item.detail); toast.info(item.detail); return }
        note("action", "Game menu: " + item.title)
        if (item.act === "proton") { openProtonPick("gamemenu"); return }
        if (item.act === "original") { sheet = ""; Turbine.playOriginal() }
        else if (item.act === "restore") { if (setups.length > 1) openSetups(); else { sheet = ""; Turbine.restoreSetup("") } }
        else if (item.act === "nexuslist") { sheet = ""; openNexus("https://www.nexusmods.com/" + rules.domain + "/mods/") }
        else if (item.act === "checkupdates") { sheet = ""; let c = Object.assign({}, updatesChecked); c[game.id] = true; updatesChecked = c; Turbine.checkUpdates(); toast.info("Checking for updates...") }
        else if (item.act === "add") { sheet = ""; openAdd() }
        else if (item.act === "why") openWhy()
        else if (item.act === "setup") { note("action", "Get this setup"); Turbine.listShares(); shareIndex = 0; sharesBack = "gamemenu"; sheet = "shares" }
        else if (item.act === "outdated") openOutdated()
        else if (item.act === "verify") openVerify()
        else if (item.act === "saves") openSaves()
        else if (item.act === "alloff") { sheet = ""; allOff() }
        else if (item.act === "allon") { sheet = ""; allBackOn() }
        else if (item.act === "reset") {
            if (pressTwice) confirmTwice(resetKey(), "reset the game files", () => {
                sheet = ""; holdsDone++
                if (shot.page === "inputcheck") note("check", "Reset would run now"); else Turbine.resetGame() })
            else if (resetHold.running) stopReset()
            else if (holdAllowed()) { resetHeld = 0; resetHold.start() }
            else holdHint("5 seconds to reset the game files")
        }
        else if (item.act === "profiles") openProfiles()
        else if (item.act === "acask") openAcAsk(null)
        else if (item.act === "acon") { sheet = ""; let a = Object.assign({}, acAnswered); delete a[game.id]; acAnswered = a; Turbine.antiCheatOff(false); toast.info("Anti-cheat stays on. Mods wait until you answer again.") }
        else if (item.act === "me3") { sheet = ""; Turbine.installMe3(false) }
        else if (item.act === "seamless") { sheet = ""; installSeamless() }
        else if (item.act === "acguide") openAcGuide()
        else if (item.act === "unhold") { sheet = ""; releaseHold("as you asked") }
        else if (item.act === "coopsettings") openCoopSettings()
        else if (item.act === "cooptile") openCoopTile()
        else if (item.act === "ersaves") openErSaves()
        else if (item.act === "launchundo") openLaunchUndo(true)
        else if (item.act === "gamesettings") openGameSettings()
        else if (item.act === "acleftover") askDelete("acleftover:" + acInfo.leftover, acInfo.leftover + " was left in " + gameTitle + "'s folder by another program that used to start it without its anti-cheat. It isn't Steam's, and Steam's Verify doesn't remove it. Turbine removes only that one file; the anti-cheat starter stays as it is.")
        else if (item.act === "loader") { sheet = ""; getLoader() }
        else if (item.act === "kcdrestore") { sheet = ""; Turbine.kcdRestore() }
        else if (item.act === "setups") openSetups()
        else if (item.act === "overlaps") openOverlaps()
        else if (item.act === "activity") openActivity()
        else if (item.act === "nexusdl") openNexusDownloads()
        else if (item.act === "help") { sheet = ""; openSection("Help") }
        else if (item.act === "updateall") { sheet = ""; if (account.premium === true || updateList.every(x => x.u.kind !== "nexus")) Turbine.updateAll(); else openUpdates() }
        else sheet = "details"
    }

    // ---------------------------------------------------------------- mod menu
    function ordinal(n) {
        const s = ["th", "st", "nd", "rd"], v = n % 100
        return n + (s[(v - 20) % 10] || s[v] || s[0])
    }
    readonly property var menuMod: focusedItem && focusedItem.kind === "mod" ? focusedItem.m : null
    // A new mod arrived: switch to its game, My mods, focus it, and say so once.
    property var arrivedMod: null
    function modArrived(gameId, id, name, on) {
        note("action", "New mod " + name + " in " + gameId)
        const i = games.findIndex(g => g.id === gameId)
        const words = on ? name + ": added and on. Press Play to start " + (i >= 0 ? fullGame(games[i].name) : gameTitle) + " with it." : name + ": added, turned off. A turns it on."
        // The build's pictures add their test mods this way: only the new-mod picture (and the press count) follow them.
        if (shot.on && shot.page !== "newmod" && shot.page !== "presscount" && shot.page.indexOf("browser") !== 0) return
        // Nexus is open in Turbine: the landing waits for it to close (by itself 2.5 seconds after a single file).
        if (browserOpen && page === "browser" && !landing) { browserArrived(gameId, id, name, on); return }
        // The requirements being got: the next one comes (and what this one needs goes in after it).
        if (reqRunning && game && gameId === game.id) {
            note("focus", "New mod while getting requirements: " + name)
            if (reqItem) { Turbine.reqQueueAdvance(); seenRev++ }
            if (modPage) { modPage = false; history = [] }
            busyHere = false
            toast.done("Added " + name + ".")
            reqArrived(id, name)
            return
        }
        if (game && gameId === game.id) reqArrived(id, name)
        // A download that went on after Nexus was closed: the screen never jumps there by itself; a toast says so.
        if (!browserOpen && !landing && browserBackground.length > 0) {
            browserBackground = browserBackground.slice(1)
            const g = games[i]
            toast.done(game && game.id === gameId ? words : "Added to " + (g ? fullGame(g.name) : "its game") + ": " + name + ".")
            note("focus", "New mod from a download that went on after Nexus closed: the focus stays where it is")
            return
        }
        // Installed from the mod page of the game on screen: stay on the page (its button says Installed).
        if (modPage && game && game.id === gameId) {
            busyHere = false; toast.done(words)
            // While it still needs something, the focus stays on the page (A must never start the game there).
            if (chainQueue.length === 0 && pageMissing < 0) { zone = "play"; note("focus", "New mod: focus on Play") }
            else note("focus", "New mod: still needs something, focus stays on the page")
            showChain(name); return
        }
        if (i >= 0 && i !== current) current = i
        if (page !== "game") { page = "game" }
        if (sheet === "add" || sheet === "found") sheet = ""
        zone = "main"; tab = 0; modPage = false
        arrivedMod = { id: id, name: name }
        focusArrived()
        // Next is playing it: the Play button has the focus (B or Down goes to the list, on the new mod).
        zone = "play"
        note("focus", "New mod: focus on Play")
        toast.done(words)
        showChain(name)
    }
    function focusArrived() {
        if (!arrivedMod) return
        const i = listItems.findIndex(x => x.kind === "mod" && x.m.id === arrivedMod.id)
        if (i >= 0) { modIndex = i; arrivedMod = null }
    }
    // Who wins shared files (overlaps.rs) and requirements both ways (needs.rs), from the engine with the mods.
    property var overlapsMap: ({})
    property var linksMap: ({})
    // Mods that may not work in this game ({id: "Made for KCD"}), and whether the player's mod_order.txt can be put back.
    property var modWarns: ({})
    property bool kcdRestore: false
    function isHidden(m) { const o = overlapsMap[m.id]; return !!o && o.hidden === true }
    function missingNeeds(m) { return needsView.missing[m.id] || [] }
    function neededByOn(m) { const l = linksMap[m.id]; return l ? (l.neededBy || []).map(id => mods.find(x => x.id === id)).filter(x => !!x && x.enabled) : [] }
    readonly property int overlapCount: Object.keys(overlapsMap).filter(k => mods.some(m => m.id === k)).length
    function modName(id) { const m = mods.find(x => x.id === id); return m ? m.name : id }
    // The Overlaps sheet for one pair: the shared files, and "Make this one win" (ordered games) when it loses them.
    function openOverlap(m, other, loses) {
        const files = other.files || []
        const more = other.count > files.length ? [{ icon: "info", title: "And " + (other.count - files.length) + " more", detail: "", run: () => {} }] : []
        openPick(loses ? m.name + " loses to " + other.name : m.name + " wins over " + other.name,
                 plural(other.count, "shared file", "shared files") + (loses ? ". The game gets " + other.name + "'s copy." : ". The game gets this mod's copy."),
                 (loses && ordered ? [{ icon: "move", title: "Make this one win", detail: "Moves it just after " + other.name + " in the list", run: () => makeWin(m, other.id) }] : [])
                 .concat(files.map(f => ({ icon: "info", title: f, detail: "", run: () => {} }))).concat(more), "")
    }
    function makeWin(m, otherId) {
        const to = mods.findIndex(x => x.id === otherId)
        const from = mods.findIndex(x => x.id === m.id)
        if (to < 0 || from < 0) return
        note("action", "Make " + m.name + " win over " + modName(otherId))
        moveMod(m, from < to ? to : to + 1)
        toast.done(m.name + " now wins its shared files over " + modName(otherId) + ".")
    }
    function openOverlaps() {
        const list = mods.filter(m => !!overlapsMap[m.id])
        openPick("Overlaps", plural(list.length, "mod shares", "mods share") + " files. Lower in the list wins.", list.map(m => {
            const o = overlapsMap[m.id]
            const w = (o.wins || []).reduce((a, x) => a + x.count, 0), l = (o.loses || []).reduce((a, x) => a + x.count, 0)
            return { icon: "info", title: m.name, detail: (o.hidden ? "Hidden by another mod. " : "") + "Wins " + plural(w, "file", "files") + ", loses " + l,
                     run: () => { goToMod(m.id); sheet = "modmenu" } }
        }), "")
    }
    function openNeeds(m) {
        const l = linksMap[m.id] || { needs: [] }
        openPick(m.name + " needs", "", (l.needs || []).map(n => {
            const have = n.id ? mods.find(x => x.id === n.id) : null
            return { icon: have ? "power" : "info", title: n.name, detail: !have ? "Not installed. Its page on Nexus has it." : have.enabled ? "Installed and on" : "Installed, turned off",
                     run: () => { if (have) goToMod(have.id) } }
        }), "")
    }
    function openNeededBy(m) {
        const l = linksMap[m.id] || { neededBy: [] }
        openPick("Mods that need " + m.name, "", (l.neededBy || []).map(id => {
            const x = mods.find(y => y.id === id)
            return { icon: "info", title: x ? x.name : id, detail: x ? (x.enabled ? "On" : "Off") : "", run: () => goToMod(id) }
        }), "")
    }
    // Put it after...: type part of a mod's name; the mod goes just after the first one that matches.
    property var putAfterMod: null
    function putAfter(text) {
        const m = putAfterMod
        putAfterMod = null
        if (!m || text.trim() === "") return
        const words = text.toLowerCase().split(/\s+/).filter(w => w !== "")
        const target = mods.find(x => x.id !== m.id && words.every(w => (x.name || "").toLowerCase().indexOf(w) >= 0))
        if (!target) { toast.info("No mod has \"" + text + "\" in its name."); return }
        const from = mods.findIndex(x => x.id === m.id), to = mods.findIndex(x => x.id === target.id)
        moveMod(m, from < to ? to : to + 1)
        toast.done(m.name + " is now after " + target.name + ".")
    }
    // X on a mod: the quick menu, at most 6 rows. What to do first (the missing loader, Move, an update), then what
    // to know (a warning, shared files, requirements), Details (everything else, Put it after...) and Uninstall.
    readonly property var modMenuFacts: !menuMod ? [] : (modWarns[menuMod.id] ? [
        { icon: "info", title: modWarns[menuMod.id], detail: "Before you play says why. It may not work in " + gameTitle + ".", act: "check" }
    ] : []).concat(overlapsMap[menuMod.id] ? (overlapsMap[menuMod.id].loses || []).slice(0, 2).map(o => (
        { icon: "info", title: "Loses " + plural(o.count, "file", "files") + " to " + o.name, detail: ordered ? "A shows them, and Make this one win" : "The game gets " + o.name + "'s copy", act: "overlap", other: o, loses: true })) : [])
    .concat(overlapsMap[menuMod.id] ? (overlapsMap[menuMod.id].wins || []).slice(0, 2).map(o => (
        { icon: "info", title: "Wins " + plural(o.count, "file", "files") + " over " + o.name, detail: "The game gets this mod's copy", act: "overlap", other: o, loses: false })) : [])
    .concat(linksMap[menuMod.id] && (linksMap[menuMod.id].needs || []).length > 0 ? [
        { icon: "info", title: "Needs: " + linksMap[menuMod.id].needs.map(n => n.name).join(", "),
          detail: missingNeeds(menuMod).length === 0 ? "All installed and on" : missingNeeds(menuMod).length + " missing or turned off", act: "needs" }
    ] : []).concat(linksMap[menuMod.id] && (linksMap[menuMod.id].neededBy || []).length > 0 ? [
        { icon: "info", title: "Needed by: " + plural(linksMap[menuMod.id].neededBy.length, "mod", "mods"), detail: linksMap[menuMod.id].neededBy.map(modName).slice(0, 3).join(", "), act: "neededby" }
    ] : [])
    // What covers a hidden mod ("Covered by Performance Tweak").
    function coveredBy(m) {
        const o = overlapsMap[m.id]
        return o && o.hidden && (o.loses || []).length > 0 ? o.loses[0] : null
    }
    readonly property bool modMenuHasFacts: !!menuMod && (modMenuFacts.length > 0 || !!placements[menuMod.id] || !!guardedMods[menuMod.id] || ordered)
    readonly property var modMenuItems: {
        if (!menuMod) return []
        const cov = menuMod.enabled ? coveredBy(menuMod) : null
        // A mod another one covers: what to do about it comes first.
        const fix = cov ? (ordered ? [{ icon: "move", title: "Make this one win", detail: "Moves it just after " + cov.name, act: "win", other: cov }]
                                   : [{ icon: "power", title: "Turn it off", detail: cov.name + " covers all its files, so it does nothing now", act: "toggle" }]) : []
        // What it still needs stays one press away (also after Later on the requirements).
        const missing = menuMod.enabled ? missingNeeds(menuMod) : []
        const first = fix.concat(missing.length > 0 ? [
            { icon: "update", title: "Get what it needs: " + (missing[0].name || missing[0]), detail: missing.length > 1 ? "And " + plural(missing.length - 1, "more mod", "more mods") + "; finds it in Get mods" : "Finds it in Get mods", act: "needs", need: missing[0].name || missing[0] }
        ] : []).concat(menuMod.needsLoader && loaderMissing ? [
            { icon: "update", title: "Get " + loaderShort, detail: loaderCanGet ? "This mod needs it. Turbine gets it from its official page." : "This mod needs it. Shows where it comes from.", act: "loader" }
        ] : []).concat(updateMap[menuMod.id] ? [
            { icon: "update", title: "Update to " + updateMap[menuMod.id].to, detail: (updateMap[menuMod.id].from ? "You have " + updateMap[menuMod.id].from + ". " : "") + "Keeps its place, and stays on or off", act: "update" }
        ] : [])
        // Edit settings, for any mod with a settings file (.ini, .toml, .json, .cfg, .yaml, .xml).
        const cfgs = settingsFilesOf(menuMod.id)
        const edit = cfgs.length === 0 ? [] : [isSeamless(menuMod)
            ? { icon: "info", title: "Co-op settings", detail: "Seamless Co-op: the session password, then every other option", act: "settings" }
            : { icon: "info", title: "Edit settings", detail: (cfgs.some(f => f.review) ? "Check after the update · " : "") + (cfgs.length === 1 ? cfgs[0].rel.split("/").pop() + (cfgs[0].edited ? " · your version" : "") : plural(cfgs.length, "settings file", "settings files")), act: "settings" }]
        const src = menuMod.sourceKind === "nexus" && rules && rules.domain && nexusModOf(menuMod) !== "" ? "nexus"
                  : menuMod.sourceKind === "thunderstore" && menuMod.sourceId ? "thunderstore" : ""
        const page = src === "nexus" ? [{ icon: "play", title: "Its Nexus page", detail: "Opens it in Turbine", act: "page" }]
                   : src === "thunderstore" ? [{ icon: "play", title: "Its Thunderstore page", detail: "Opens it in Turbine", act: "files", src: src }] : []
        const check = src !== "" && !updateMap[menuMod.id] ? [{ icon: "update", title: "Check for an update", detail: "Asks " + (src === "nexus" ? "Nexus" : "Thunderstore") + " now", act: "checkupdate" }] : []
        // Actions only, always in the same order; missing rows are left out, never dimmed. The facts
        // (overlaps, needs, where it goes) are in Details.
        const order = ordered && mods.length > 1 ? [
            { icon: "move", title: "Move", detail: "Up and Down move it, A puts it down (or tap where it goes)", act: "move" }
        ] : []
        const names = [{ icon: "info", title: "Rename", detail: menuMod.renamed ? "Now " + menuMod.name : "A name of your own for the list", act: "rename" }]
        const select = mods.length > 1 ? [{ icon: "files", title: "Select several", detail: "Tick this and others to change them together", act: "select" }] : []
        // In or out of the Dependencies group (only the list changes, never the load order).
        const dep = isDep(menuMod) ? [{ icon: "move", title: "Move out of Dependencies", detail: "Show it with your mods", act: "depout" }]
                                   : [{ icon: "move", title: "Move into Dependencies", detail: "For a mod that is mainly what others need", act: "depin" }]
        const more = page.concat(check).concat(dep)
            .concat(src === "nexus" ? [{ icon: "update", title: "Other files", detail: "Another version or file from its page", act: "files", src: src }] : [])
            .concat([{ icon: "files", title: "Show its files", detail: plural(menuMod.files || 0, "file", "files") + " it puts in the game", act: "showfiles" }])
            .concat(placements[menuMod.id] ? [{ icon: "move", title: "Change where it goes", detail: "Now in " + folderWords(placements[menuMod.id].to), act: "place" }] : [])
            .concat(menuMod.renamed ? [{ icon: "restore", title: "Reset name", detail: "Back to " + (menuMod.nexusName || tidyName(menuMod.raw, menuMod.version)), act: "resetname" }] : [])
            // Qt can't open a folder in Game Mode: the row is left out there.
            .concat(Turbine.gamemode ? [] : [{ icon: "info", title: "Open folder", detail: "Where Turbine keeps this mod's files", act: "folder" }])
        const facts = modMenuFacts.length
        return first.concat(order).concat(edit).concat(names).concat(select)
            .concat(more.length > 0 ? [{ icon: "info", title: "More", detail: more.slice(0, 3).map(x => x.title).join(", "), act: "more", rest: more }] : [])
            .concat(modMenuHasFacts ? [{ icon: "info", title: "Details", act: "details",
                                         detail: facts > 0 ? modMenuFacts.slice(0, 2).map(f => f.title).join(" · ") : modWords(menuMod) || (placements[menuMod.id] ? "Where it goes" : "Its place in the order") }] : [])
            .concat([{ icon: "remove", title: "Uninstall", detail: "You can put it back for 7 days.", act: "remove", danger: true }])
    }
    // Details: everything about the mod in one list (and Put it after... for ordered games).
    function openModDetails(m) {
        openPick(m.name, [modWords(m), m.enabled ? "On" : "Off"].filter(x => x !== "").join(" · "), placementWords(m).map(f => ({
            icon: f.icon, title: f.title, detail: f.detail, run: () => {
                if (f.act === "place") openPlace("move", m)
                else if (f.act === "replaceon") { note("action", "Replace the game's files: " + m.name); Turbine.setReplace(m.id, true) }
                else if (f.act === "replaceoff") { note("action", "Leave the game's files alone: " + m.name); Turbine.setReplace(m.id, false) }
            } })).concat(modMenuFacts.map(f => ({
            icon: f.icon, title: f.title, detail: f.detail, run: () => {
                if (f.act === "overlap") openOverlap(m, f.other, f.loses)
                else if (f.act === "needs") openNeeds(m)
                else if (f.act === "neededby") openNeededBy(m)
                else if (f.act === "check") startPlay()
            } })).concat(ordered ? [
            { icon: "move", title: "Put it after...", detail: "Type part of another mod's name; it goes just after that one",
              run: () => { putAfterMod = m; openAsk("putafter", "Put " + m.name + " after...", "Part of the name of the mod it goes after.", "", "Mod name", 60, false, "Move it") } }
        ] : []).concat(modMenuFacts.length === 0 && !ordered && !placements[m.id] ? [{ icon: "info", title: "Nothing else to know", detail: "", run: () => {} }] : [])), "")
    }
    property int menuIndex: 0
    function modMenuRun(i) {
        const item = modMenuItems[i], m = menuMod
        if (!item || !m) return
        note("action", "Mod menu: " + item.title + " for " + m.name)
        sheet = ""
        modMenuDo(item, m)
    }
    // The menu's rows for the activity log, with what More holds.
    // The highlighted mod's settings (My mods' panel and the mod page): its files, the button's words, and the way in.
    readonly property var panelCfgs: menuMod && tab === 0 ? settingsFilesOf(menuMod.id) : []
    readonly property bool panelHasSettings: showModPanel && panelCfgs.length > 0
    readonly property string panelSettingsLabel: menuMod && isSeamless(menuMod) ? "Co-op settings" : "Edit settings"
    function panelEditOpen() { if (!menuMod) return; note("action", "Panel: " + panelSettingsLabel + " for " + menuMod.name); openModSettings(menuMod) }
    onPanelHasSettingsChanged: if (!panelHasSettings && zone === "panel") zone = "main"
    function modMenuTitles() { return modMenuItems.map(x => x.act === "more" ? "More (" + x.rest.map(r => r.title).join(", ") + ")" : x.act === "details" ? "Details (" + modMenuFacts.map(f => f.title).join(", ") + ")" : x.title).join("; ") }
    // How many things X offers for the highlighted mod (the More row's own rows counted, Details left out): the
    // details panel's "More options (9)".
    function optionsCount() {
        let all = []
        for (const x of modMenuItems) all = all.concat(x.act === "more" ? x.rest : [x])
        return all.filter(x => x.act !== "details").length
    }
    function modMenuDo(item, m) {
        if (item.act === "more") openPick(m.name, "More options", item.rest.map(x => ({ icon: x.icon, title: x.title, detail: x.detail, run: () => modMenuDo(x, m) })), "modmenu")
        else if (item.act === "toggle") toggle(m)
        else if (item.act === "loader") getLoader()
        else if (item.act === "needs") {
            note("action", "Get what it needs: " + item.need)
            sheet = ""
            // The same sheet as after an add (Get them all); a need Turbine can't name on Nexus is searched.
            const r = reqSplit(m)
            if (r.get.length > 0 || r.off.length > 0) { reqOffer(m, r); return }
            page = "game"; modPage = false; history = []; tab = 1; zone = "main"
            getText = String(item.need); getCategory = ""; browseNow()
        }
        else if (item.act === "depout") moveDep(m, false)
        else if (item.act === "depin") moveDep(m, true)
        else if (item.act === "check") startPlay()
        else if (item.act === "top") moveMod(m, 0)
        else if (item.act === "bottom") moveMod(m, mods.length - 1)
        else if (item.act === "remove") uninstall(m)
        else if (item.act === "select") startSelect(m)
        else if (item.act === "overlap") openOverlap(m, item.other, item.loses)
        else if (item.act === "needs") openNeeds(m)
        else if (item.act === "neededby") openNeededBy(m)
        else if (item.act === "putafter") { putAfterMod = m; openAsk("putafter", "Put " + m.name + " after...", "Part of the name of the mod it goes after.", "", "Mod name", 60, false, "Move it") }
        else if (item.act === "update") updateOne(m)
        else if (item.act === "move") startMove()
        else if (item.act === "details") openModDetails(m)
        else if (item.act === "place") openPlace("move", m)
        else if (item.act === "win") makeWin(m, item.other.id)
        else if (item.act === "page") openNexus(nexusPageOf(m))
        else if (item.act === "checkupdate") { let c = Object.assign({}, updatesChecked); c[game.id] = true; updatesChecked = c; Turbine.checkUpdates(); toast.info("Checking for updates...") }
        else if (item.act === "rename") { renameTarget = m; openAsk("rename", "Rename " + m.name, "Its name in your list. Leave it empty for its own name.", m.renamed ? m.name : "", "Name", 80, false, "Rename") }
        else if (item.act === "resetname") renameMod(m, "")
        else if (item.act === "showfiles") openModFiles(m)
        else if (item.act === "settings") openModSettings(m)
        else if (item.act === "files") {
            const id = item.src === "nexus" ? nexusModOf(m) : String(m.sourceId).split(":")[0]
            if (sources.indexOf(item.src) >= 0) getSource = item.src
            const back = { kind: "mymods", index: modIndex }
            page = "game"; tab = 1; zone = "main"
            pendingFiles = true
            // B comes back to My mods, on this mod (newbie M7), not to Get mods.
            history = [back]
            openDetails({ id: id, name: m.name, source: item.src, icon: "" }, true)
        }
        else if (item.act === "folder") {
            const dir = Turbine.modFolder(m.id)
            if (dir === "") { toast.info("This mod's folder isn't there."); return }
            note("action", "Open folder " + dir)
            if (!Qt.openUrlExternally("file://" + encodeURI(dir))) toast.info("Its files are in " + dir)
        }
    }
    // ---------------------------------------------------------------- Clean names
    // A mod's name: the one the player gave it, else its name on Nexus, else its file's name tidied up (underscores
    // to spaces, the version left off the end when it is the mod's version, spaces collapsed).
    property var nexusInfo: parse(namesSet.nexus, {})
    property var renamedMap: parse(namesSet.renamed, {})
    function nameKey(id) { return (game ? game.id : "") + "|" + id }
    function tidyName(raw, version) {
        let n = String(raw || "").replace(/_/g, " ").replace(/\s+/g, " ").trim()
        if (version) {
            const v = String(version).replace(/^[vV]/, "")
            const esc = v.replace(/[.*+?^${}()|[\]\\]/g, "\\$&").replace(/\\\./g, "[. ]")
            const t = n.replace(new RegExp("[ -]+[vV]?" + esc + "$"), "").trim()
            if (t.length >= 2) n = t
        }
        // A Nexus download's file tail ("-790-2-6-1-1699999999": mod id, version, upload time), then any version
        // left at the end ("1.2", "v2.0.1"). The version shows in the details panel instead.
        let t = n.replace(/-\d+-[0-9A-Za-z-]*-\d{9,11}$/, "").trim()
        t = t.replace(/[ _-]+[vV]?\d+(\.\d+){1,3}[a-z]?$/, "").trim()
        if (t.length >= 2) n = t
        return n !== "" ? n : String(raw || "Mod")
    }
    function cleanName(raw) { return tidyName(raw, "") }
    function withNames(m) {
        const k = nameKey(m.id), nx = nexusInfo[k]
        // The player's own name, else the page's title (Nexus), else the cleaned file name.
        const shown = renamedMap[k] || (nx && nx.name) || m.sourceTitle || tidyName(m.name, m.version)
        return Object.assign({}, m, { raw: m.name, name: shown, renamed: !!renamedMap[k], author: nx ? nx.author || "" : "",
                                      summary: nx ? nx.summary || "" : "", picture: (nx && nx.icon) || m.picture || "", nexusName: nx ? nx.name || "" : (m.sourceTitle || ""),
                                      gone: !!(nx && nx.gone) })
    }
    // Words from Nexus for an installed mod (from a card or its page); the list shows them at once.
    function rememberNexus(m, name, author, summary, icon, gone) {
        if (!m || !name) return
        const k = nameKey(m.id), old = nexusInfo[k] || {}
        const g = gone === undefined ? !!old.gone : !!gone
        if (old.name === name && old.author === (author || "") && old.summary === (summary || "") && (old.icon || "") === (icon || old.icon || "") && !!old.gone === g) return
        let all = Object.assign({}, nexusInfo)
        all[k] = { name: name, author: author || "", summary: summary || "", icon: icon || old.icon || "", gone: g }
        namesSet.nexus = JSON.stringify(all)
        nexusInfo = all
        mods = mods.map(x => x.id === m.id ? withNames(Object.assign({}, x, { name: x.raw || x.name })) : x)
    }
    // A mod deleted for good: its name and Nexus words go too, so a later mod given the same id starts fresh.
    function forgetNames(ids) {
        let r = Object.assign({}, renamedMap), x = Object.assign({}, nexusInfo), changed = false
        for (const id of ids) {
            const k = nameKey(id)
            if (k in r) { delete r[k]; changed = true }
            if (k in x) { delete x[k]; changed = true }
        }
        if (!changed) return
        namesSet.renamed = JSON.stringify(r); renamedMap = r
        namesSet.nexus = JSON.stringify(x); nexusInfo = x
    }
    function renameMod(m, text) {
        const k = nameKey(m.id)
        let all = Object.assign({}, renamedMap)
        if (text.trim() === "") delete all[k]; else all[k] = text.trim()
        namesSet.renamed = JSON.stringify(all)
        renamedMap = all
        mods = mods.map(x => x.id === m.id ? withNames(Object.assign({}, x, { name: x.raw || x.name })) : x)
        note("action", text.trim() === "" ? "Name reset: " + m.name : "Renamed " + (m.raw || m.name) + " to " + text.trim())
        toast.done(text.trim() === "" ? "Back to its own name." : "Renamed to " + text.trim() + ".")
    }
    property var renameTarget: null
    // Show its files: the files the mod puts in the game, in a list.
    function openModFiles(m) {
        const list = parse(Turbine.modFiles(m.id), [])
        openPick(m.name, plural(list.length, "file", "files") + " it puts in " + gameTitle, list.slice(0, 200).map(f => ({ icon: "files", title: f, detail: "", run: () => {} }))
                 .concat(list.length > 200 ? [{ icon: "info", title: "and " + (list.length - 200) + " more", detail: "", run: () => {} }] : []), "")
    }
    function nexusPageOf(m) {
        const id = nexusModOf(m)
        return id !== "" && rules && rules.domain ? "https://www.nexusmods.com/" + rules.domain + "/mods/" + id : ""
    }
    // The panel on the right of My mods: only on wide screens (16:9 and up), with something highlighted.
    readonly property bool showModPanel: tab === 0 && modPanelRoom
    // The panel's room on My mods, whatever the tab (it fades out with My mods while a page turns).
    readonly property bool modPanelRoom: page === "game" && !!game && modsLoaded && !modPage && mods.length > 0 && width / Math.max(1, height) >= 1.5 && designWidth >= 700
    // The details panel's first line: "Version 1.2 · On · added Nov 9 · 3 files" (and the author when known).
    function versionOf(m) {
        if (!m) return ""
        if (m.version) return String(m.version).replace(/^[vV]/, "")
        const v = String(m.raw || m.name || "").replace(/_/g, " ").match(/[ -][vV]?(\d+(\.\d+){1,3}[a-z]?)$/)
        return v ? v[1] : ""
    }
    function panelLine(m) {
        if (!m) return ""
        const v = versionOf(m)
        return [v !== "" ? "Version " + v : "", m.author ? "by " + m.author : "", m.enabled ? "On" : "Off",
                m.added ? "added " + Qt.formatDate(new Date(m.added * 1000), "MMM d") : "", m.files ? plural(m.files, "file", "files") : ""].filter(x => x !== "").join(" · ")
    }
    readonly property var modPanelFacts: {
        const m = menuMod
        if (!m) return []
        let out = []
        const line = rowLine(m)
        if (line.level !== "") out.push({ text: line.text, level: line.level })
        // Seamless Co-op's own fact, first when nothing is wrong (its row says it then).
        if (isSeamless(m) && line.text.indexOf("Only loads") !== 0) out.push({ text: coopOnlyWords + ".", level: "" })
        const o = overlapsMap[m.id]
        if (o) {
            for (const w of (o.wins || []).slice(0, 2)) out.push({ text: "Wins " + plural(w.count, "file", "files") + " over " + w.name, level: "" })
            for (const l of (o.loses || []).slice(0, 2)) out.push({ text: "Loses " + plural(l.count, "file", "files") + " to " + l.name, level: "" })
        }
        const l = linksMap[m.id]
        if (l && (l.needs || []).length > 0) out.push({ text: "Needs: " + l.needs.map(n => n.name).join(", "), level: missingNeeds(m).length > 0 ? "warn" : "" })
        if (placements[m.id]) out.push({ text: "Installed in " + folderWords(placements[m.id].to), level: "" })
        return out.slice(0, 5)
    }
    // Status badges: a mod its page took down or its author marked deprecated.
    function statusOf(m) {
        if (!m) return ""
        if (m.gone) return m.sourceKind === "nexus" ? "Removed from Nexus" : "Deprecated"
        const u = updateMap[m.id]
        return u && u.deprecated ? "Marked deprecated" : ""
    }
    readonly property var outdatedMods: mods.filter(m => statusOf(m) !== "")
    function openOutdated() {
        openPick(plural(outdatedMods.length, "mod may be", "mods may be") + " out of date", "Their pages say they are no longer kept up. A turns one off.",
            outdatedMods.map(m => ({ icon: "power", title: m.name, detail: statusOf(m) + (m.enabled ? " · On" : " · Off"), aLabel: m.enabled ? "Turn off" : "Turn on",
                run: () => { toggle(m) } })), "gamemenu")
    }
    // A row's second line: its problem when it has one (said plainly, with what to do), else who made it and when it
    // was added ("by STuRoK · added Nov 6"). Where it came from is not repeated on every row.
    function rowLine(m) {
        if (!m) return { text: "", level: "" }
        if (m.needsLoader && loaderMissing) return { text: "Needs " + loaderShort + ": X to get it", level: "warn" }
        if (modWarns[m.id]) return { text: modWarns[m.id], level: "warn" }
        if (m.enabled && missingNeeds(m).length > 0) return { text: "Needs a mod that isn't on: " + missingNeeds(m).map(n => n.name || n).slice(0, 2).join(", "), level: "warn" }
        const cov = m.enabled ? coveredBy(m) : null
        if (cov) return { text: "Covered by " + cov.name + ": X to fix", level: "info" }
        // Seamless Co-op's row says when it loads (on or off, it only loads with its own Play).
        if (isSeamless(m)) return { text: "Only loads in Co-op mode", level: "info" }
        const when = m.added > 0 ? "added " + Qt.formatDate(new Date(m.added * 1000), "MMM d") : ""
        return { text: [m.author ? "by " + m.author : "", when].filter(x => x !== "").join(" · "), level: "" }
    }
    function modWords(m) {
        let parts = []
        if (m.version) parts.push(m.version)
        // Files added by hand say nothing more ("Zip file" on every row told nothing).
        if (m.source && m.source !== "Zip file") parts.push(m.source)
        return parts.join(" · ")
    }

    // ---------------------------------------------------------------- Mods for any game
    // Games without rules of their own get Basic support: each mod goes where its files show they belong, or Turbine
    // asks once ("Where do these files go?"). The engine decides (basic.rs); these draw it and take the answers.
    readonly property bool basicGame: !!rules && !!rules.supportLevel && rules.supportLevel !== "full"
    // An anti-cheat game me3 starts says how its mods run; an online-only one says nothing here (the status does).
    // Rules can be null for a moment while the rail switches games (basicGame may not have caught up yet).
    readonly property string supportChip: acMode === "refuse" ? "" : acMode === "me3" ? "Mods offline" : !basicGame || !rules ? "" : rules.supportLevel === "custom" ? "Added by you" : "Basic support"
    property var placements: ({})         // mod id: {from, to, rule, replace, skipped} (Basic games)
    property var guardedMods: ({})        // mod id: {replace, skipped} (a player file's game, mods it laid out)
    property var placeData: null          // the waiting mod: {game, gameName, pending: {name, placement, sure, conflicts, refused, tree}}
    property string placeMode: ""         // "add" (a new mod) or "move" (Change where it goes)
    property var placeMoveMod: null
    property string placeGameRel: ""      // the game folder on screen (where the files go)
    property string placeModRel: ""       // the mod's folder on screen (what goes in)
    property int placePane: 0
    property int placeGameIndex: 0
    property int placeModIndex: 0
    property string placeFilter: ""
    property var placeNewFolders: []      // folders made on this page (made for real when the mod goes in)
    property var placeFoldersNow: []
    property var placeCheckNow: null
    property var placeFrom: null
    property var placeMemo: ({})          // the row last on in each folder, by "g:<folder>" and "m:<folder>"
    // The mod's own files only on demand (X, Only part of the file...).
    property bool placeModShown: false
    readonly property bool placeTwoPanes: placeModShown && (width >= 1280 || desktopLayout)
    readonly property string placeGameName: placeData ? fullGame(placeData.gameName) : gameTitle
    function folderWords(rel) { return rel === "" ? "the game folder" : rel }
    function parentOf(p) { const i = p.lastIndexOf("/"); return i < 0 ? "" : p.substring(0, i) }
    function baseName(p) { const i = p.lastIndexOf("/"); return i < 0 ? p : p.substring(i + 1) }
    readonly property var placeGameRows: {
        if (page !== "place") return []
        let rows = []
        const p = placeData && placeData.pending ? placeData.pending.placement : null
        // The guess first, as one press: "Put them in Content/Paks" (A or a tap places them there).
        if (placeMode === "add" && p && placeGameRel === "") {
            for (const s of [p.to].concat(p.others || [])) if (s !== "") rows.push({ kind: "suggest", label: "Put them in " + s, path: s, badge: "Suggested" })
        }
        rows.push({ kind: "here", label: "Put them here", detail: placeGameRel === "" ? "Right in " + placeGameName + "'s folder" : placeGameRel })
        const kids = placeFoldersNow.concat(placeNewFolders.filter(n => parentOf(n).toLowerCase() === placeGameRel.toLowerCase()
                                                                       && !placeFoldersNow.some(f => f.path.toLowerCase() === n.toLowerCase()))
                                                         .map(n => ({ name: baseName(n), path: n, turbine: false, made: true })))
        const f = placeFilter.toLowerCase()
        for (const k of kids) if (f === "" || k.name.toLowerCase().indexOf(f) >= 0)
            rows.push({ kind: k.made ? "new" : "folder", label: k.name, path: k.path, badge: k.made ? "New" : k.turbine ? "Made by Turbine" : "" })
        return rows
    }
    readonly property var placeTree: placeData && placeData.pending ? (placeData.pending.tree || []) : []
    readonly property var placeModRows: {
        if (page !== "place" || placeMode !== "add") return []
        const rows = placeTree.filter(t => t.path !== "" && parentOf(t.path).toLowerCase() === placeModRel.toLowerCase())
                              .map(t => ({ kind: "folder", label: baseName(t.path), detail: plural(t.files, "file", "files"), path: t.path }))
        return rows.length > 0 ? rows : [{ kind: "info", label: "No folders in here", detail: "Only files: they all go in together" }]
    }
    readonly property string placeTopLine: {
        if (page !== "place") return ""
        if (placeMode === "move") return (placeMoveMod ? placeMoveMod.name : "This mod") + ": pick the folder of " + placeGameName + " its files go into now. Every file keeps its place inside it."
        const pd = placeData && placeData.pending
        if (!pd) return ""
        return pd.placement.state === "guess" ? pd.name + ": Turbine thinks these files go in " + pd.placement.to + ". A puts them there. Pick another folder below if that's wrong."
             : pd.name + ": Turbine can't tell where these go in " + placeGameName + ". Open the folder they belong in, then press Put them here."
    }
    readonly property string placePreview: {
        if (page !== "place") return ""
        if (placeMode === "move") return (placeMoveMod ? placeMoveMod.name : "The mod") + "'s files go into " + folderWords(placeGameRel) + "."
        const r = placeCheckNow
        if (!r) return ""
        if (r.error) return r.error
        return plural(r.files, "file goes", "files go") + " into " + folderWords(placeGameRel) + "."
             + (r.conflicts.length > 0 ? " " + (r.conflicts.length === 1 ? "1 of them is" : r.conflicts.length + " of them are") + " already in the game." : "")
             + (r.refused.length > 0 ? " " + plural(r.refused.length, "file", "files") + " can't go in (Turbine never places it)." : "")
    }
    function placeRefresh() {
        placeFoldersNow = parse(Turbine.gameFolders(placeGameRel), [])
        placeCheckNow = placeMode === "add" ? parse(Turbine.placeCheck(placeModRel, placeGameRel), null) : null
    }
    // A waiting mod: the page (or only the question about the game's own files) when nothing else is on screen.
    function maybePlace() {
        if (!placeData || !placeData.pending || page === "place" || page === "browser" || sheet !== "" || wheelOpen) return
        const pd = placeData.pending
        if (pd.sure) openOverwrite(pd.name, pd.placement.from, pd.placement.to, pd.conflicts || [], pd.refused || [], false)
        else openPlace("add", null)
    }
    function openPlace(mode, mod) {
        placeFrom = { page: page === "place" ? "game" : page, tab: tab, zone: zone }
        placeMode = mode; placeMoveMod = mod || null
        placePane = 0; placeFilter = ""; placeNewFolders = []; placeMemo = ({}); placeModShown = false
        if (mode === "add") {
            const p = placeData.pending.placement
            placeModRel = p.from || ""
            placeGameRel = ""
            note("page", "Asked where files go: " + placeData.pending.name + " (" + p.state + (p.to !== "" ? ", suggested " + p.to : "") + ")")
        } else {
            const info = placements[mod.id]
            placeModRel = ""
            placeGameRel = info ? info.to : ""
            note("page", "Change where it goes: " + mod.name + " (now " + folderWords(placeGameRel) + ")")
        }
        if (game && placeData && mode === "add" && placeData.game !== game.id) {
            const i = games.findIndex(g => g.id === placeData.game)
            if (i >= 0) current = i
        }
        sheet = ""; page = "place"; zone = "main"
        placeRefresh()
        const s = placeGameRows.findIndex(r => r.kind === "suggest")
        placeGameIndex = s >= 0 ? s : 0
        placeModIndex = 0
    }
    function placeLeave() {
        const f = placeFrom || { page: "game", tab: 0, zone: "main" }
        page = f.page === "place" ? "game" : f.page; tab = f.page === "game" ? 0 : f.tab; zone = "main"
        placeMode = ""; placeMoveMod = null
        Qt.callLater(maybePlace)
    }
    function placeGo(rel) {
        placeMemo[(placePane === 0 ? "g:" : "m:") + (placePane === 0 ? placeGameRel : placeModRel)] = placePane === 0 ? placeGameIndex : placeModIndex
        if (placePane === 0) { placeGameRel = rel; placeGameIndex = 0 } else { placeModRel = rel; placeModIndex = 0 }
        placeFilter = ""
        placeRefresh()
    }
    function placeUp() {
        const rel = placePane === 0 ? placeGameRel : placeModRel
        if (rel === "") return false
        const up = parentOf(rel)
        placeGo(up)
        // Back on the folder it came out of.
        const rows = placePane === 0 ? placeGameRows : placeModRows
        const i = rows.findIndex(r => r.path && r.path.toLowerCase() === rel.toLowerCase() && r.kind !== "suggest")
        if (placePane === 0) placeGameIndex = Math.max(0, i); else placeModIndex = Math.max(0, i)
        return true
    }
    function placeHere() {
        const to = placeGameRel
        if (placeMode === "move") {
            if (!placeMoveMod) return
            note("action", "Change where it goes: " + placeMoveMod.name + " into " + folderWords(to))
            Turbine.changePlacement(placeMoveMod.id, to)
            placeLeave()
            return
        }
        if (!placeData || !placeData.pending) return
        const r = parse(Turbine.placeCheck(placeModRel, to), null)
        if (!r || r.error) { toast.problem(r && r.error ? r.error : "That folder can't be used."); return }
        if (r.conflicts.length > 0 || r.refused.length > 0) { openOverwrite(placeData.pending.name, placeModRel, to, r.conflicts, r.refused, true); return }
        note("action", "Put them here: " + folderWords(to))
        Turbine.finishPlace(placeModRel, to, false)
        placeLeave()
    }
    function placeRowRun(pane, i) {
        const rows = pane === 0 ? placeGameRows : placeModRows
        const r = rows[i]
        if (!r) return
        if (r.kind === "here") placeHere()
        else if (r.kind === "suggest") { note("action", "Where do these files go: the suggestion " + r.path); placeGo(r.path); placeHere() }
        else if (r.path !== undefined) placeGo(r.path)
    }
    function placeLeaveAsk() {
        note("action", "Where do these files go: left")
        if (placeMode === "add") { Turbine.cancelPlace(); toast.info("Not added. The file is still in Downloads.") }
        placeLeave()
    }
    function placeMenu() {
        const rows = placePane === 0 ? placeGameRows : placeModRows
        const r = rows[placePane === 0 ? placeGameIndex : placeModIndex]
        let items = [{ icon: "play", title: "Put them here", detail: "Into " + folderWords(placeGameRel), run: () => placeHere() }]
        if (r && r.path !== undefined) items.push({ icon: "move", title: "Open", detail: r.path === "" ? "The top" : r.path, run: () => placeGo(r.path) })
        if (placePane === 0) items.push({ icon: "update", title: "New folder here", detail: "Made when the mod goes in", run: () => openAsk("placefolder", "New folder in " + folderWords(placeGameRel), "Its name. Letters, numbers and spaces work best.", "", "Folder name", 60, false, "Make it") })
        if (placeMode === "add") items.push({ icon: "files", title: placeModShown ? "Hide the mod's files" : "Only part of the file...", detail: placeModShown ? "Back to the game's folders only" : "Pick a folder inside the mod to put in", run: () => { placeModShown = !placeModShown; if (!placeModShown) placePane = 0 } })
        if (placeMode === "add") items.push({ icon: "info", title: "Show the files inside", detail: "What goes in from " + (placeModRel === "" ? "the file" : placeModRel), run: () => placeShowFiles() })
        openPick("Where do these files go?", placeGameName, items, "")
    }
    function placeShowFiles() {
        const inside = placeTree.filter(t => t.path === "" ? placeModRel === "" : t.path.toLowerCase() === placeModRel.toLowerCase())
        const sub = placeTree.filter(t => t.path !== "" && parentOf(t.path).toLowerCase() === placeModRel.toLowerCase())
        openMsg({ title: placeModRel === "" ? "Everything in the file" : placeModRel,
                  text: (inside[0] ? plural(inside[0].files, "file", "files") : "No files") + " go in from here" + (sub.length > 0 ? ", in these folders:" : "."),
                  notes: sub.map(t => baseName(t.path) + ": " + plural(t.files, "file", "files")),
                  buttons: [{ key: "b", label: "Close", run: () => {} }] })
    }
    function placeNewFolder(name) {
        const n = name.trim()
        if (n === "" || n === "." || n === ".." || /[\/\\:*?"<>|]/.test(n)) { toast.problem("That name can't be a folder name."); return }
        const path = placeGameRel === "" ? n : placeGameRel + "/" + n
        placeNewFolders = placeNewFolders.concat([path])
        note("action", "New folder here: " + path)
        placeGo(path)
    }
    function placeAct(b) {
        const rows = placePane === 0 ? placeGameRows : placeModRows
        const idx = placePane === 0 ? placeGameIndex : placeModIndex
        const setIdx = i => { const v = Math.max(0, Math.min(rows.length - 1, i)); if (placePane === 0) placeGameIndex = v; else placeModIndex = v }
        if (b === "up") setIdx(idx - 1)
        else if (b === "down") setIdx(idx + 1)
        else if (b === "lt") setIdx(idx - pageJump)
        else if (b === "rt") setIdx(idx + pageJump)
        else if (b === "a") placeRowRun(placePane, idx)
        else if (b === "b") { if (!placeUp()) placeLeaveAsk() }
        else if (b === "x") placeMenu()
        else if (b === "y") openAsk("placefind", "Find a folder", "Part of its name. Only this folder's list is filtered.", placeFilter, "Folder name", 60, false, "Show them")
        // Start puts them where the ring is (never the folder on screen when the ring is on another one).
        else if (b === "start") {
            const r = rows[idx]
            if (placePane === 0 && r && (r.kind === "suggest" || r.kind === "folder" || r.kind === "new")) { placeGo(r.path); placeHere() }
            else placeHere()
        }
        else if ((b === "lb" || b === "rb" || b === "left" || b === "right") && placeMode === "add" && placeModShown) {
            const want = b === "lb" || b === "left" ? 0 : 1
            // The mod's files have no folders to pick: the focus stays where it is.
            if (want === 1 && placeModRows.every(r => r.kind === "info")) { note("action", "Where do these files go: Mod files has no folders"); return }
            if (want !== placePane) { placePane = want; note("action", "Where do these files go: " + (want === 0 ? "Game folder" : "Mod files")) }
        }
    }
    // B on Where do these files go?: the legend and the header button say the same.
    readonly property string placeBLabel: (placePane === 0 ? placeGameRel : placeModRel) !== "" ? "Up a folder" : "Leave"
    readonly property string placeALabel: {
        const rows = placePane === 0 ? placeGameRows : placeModRows
        const r = rows[placePane === 0 ? placeGameIndex : placeModIndex]
        return !r ? "" : r.kind === "here" || r.kind === "suggest" ? r.label : r.path !== undefined ? "Open" : ""
    }
    // What Start does from the ring's row.
    readonly property string placeStartLabel: {
        const r = placePane === 0 ? placeGameRows[placeGameIndex] : null
        return !r || r.kind === "here" || r.kind === "suggest" ? "" : r.path !== undefined ? "Put them in " + r.label : ""
    }
    // The question about files the game already has (and the ones never placed).
    function openOverwrite(name, from, to, conflicts, refused, fromPage) {
        const gname = placeData ? fullGame(placeData.gameName) : gameTitle
        const list = conflicts.slice(0, 5).concat(conflicts.length > 5 ? ["and " + (conflicts.length - 5) + " more"] : [])
        const refusedWords = refused.length === 0 ? ""
            : plural(refused.length, "file", "files") + " can't be " + (conflicts.length > 0 ? "replaced" : "placed") + ": "
              + refused.slice(0, 4).map(x => baseName(x.path) + " (" + x.why + ")").join(", ") + (refused.length > 4 ? " and " + (refused.length - 4) + " more" : "") + "."
        const done = () => { if (fromPage) placeLeave() }
        const cancel = () => {
            if (fromPage) return
            Turbine.cancelPlace(); toast.info("Not added. The file is still in Downloads.")
        }
        note("page", "Overwrite question: " + name + ": " + conflicts.length + " the game has, " + refused.length + " never placed")
        openMsg({
            title: conflicts.length > 0 ? gname + " already has " + plural(conflicts.length, "of these files", "of these files") : "Some of these files can't go in",
            text: conflicts.length > 0 ? "Turbine can leave the game's files alone and skip these, or replace them. If you replace them, Turbine keeps the game's originals and puts them back when you turn the mod off or remove it."
                                       : "Turbine adds the rest and leaves these out.",
            notes: list,
            footnote: refusedWords,
            reason: "a mod needs a choice",
            buttons: (conflicts.length > 0 ? [
                { key: "a", label: "Skip those files", run: () => { note("action", "Overwrite: skip"); Turbine.finishPlace(from, to, false); done() } },
                { key: "x", label: "Replace them", run: () => { note("action", "Overwrite: replace"); Turbine.finishPlace(from, to, true); done() } }
            ] : [
                { key: "a", label: "Add the rest", run: () => { Turbine.finishPlace(from, to, false); done() } }
            ]).concat([{ key: "b", label: "Cancel", run: cancel }])
        })
    }
    // A sheet with words and buttons ({title, text, steps, notes, footnote, buttons: [{key, label, run}], reason}).
    property var msgData: null
    function openMsg(d) { msgData = d; infoIndex = 0; sheet = "msg" }
    // "Which game on Nexus is this?": once, on opening Get mods for a game Turbine couldn't match; B asks again later.
    property var nexusAsked: ({})
    property var nexusFound: null         // the search's games, in place of the candidates
    // A game whose Nexus page Turbine can't tell for sure (several Nexus games fit its name: the engine's
    // candidates, rules.nexusCandidates with domainState "ask") is asked about once, with a simple pick, the first
    // time its Get mods shows: on a tab change, and when the game's rules arrive with Get mods already open (a
    // game that opens there was never asked before). Once per game, ever; Get mods keeps its Pick the Nexus game
    // button after a Not now.
    property string rulesFor: ""          // the game the rules on hand are for
    property int nexusAsks: 0
    function maybeAskNexusGame(why) {
        if (!game || !rules || rulesFor !== game.id) return false
        // (The build's pictures of other things stay as they are: only its own picture asks.)
        if (shot.on && shot.page !== "nexuspick") return false
        if (page !== "game" || tab !== 1 || modPage || sheet !== "" || wheelOpen) return false
        if (rules.domainState !== "ask" || (rules.nexusCandidates || []).length === 0) return false
        const asked = parse(seen.nexusGameAsked, [])
        if (nexusAsked[game.id] || asked.indexOf(game.id) >= 0) return false
        asked.push(game.id)
        if (!shot.on) seen.nexusGameAsked = JSON.stringify(asked)
        nexusAsks++
        note("action", "Which game on Nexus: asked once for " + game.name + " (" + why + ")")
        openNexusPick()
        return true
    }
    function openNexusPick() {
        if (!game || !rules) return
        let a = Object.assign({}, nexusAsked); a[game.id] = true; nexusAsked = a
        const cands = nexusFound !== null ? nexusFound : (rules.nexusCandidates || [])
        note("action", "Which game on Nexus: " + game.name + " (" + cands.length + (nexusFound !== null ? " found" : " candidates") + ")")
        openPick("Which game on Nexus is " + gameTitle + "?", nexusFound !== null ? "Search results" : "Pick it, or search Nexus", cands.map(c => ({
            icon: "play", title: c.name, detail: Number(c.mods).toLocaleString(Qt.locale("en_US"), "f", 0) + " mods", aLabel: "Pick",
            run: () => { nexusFound = null; Turbine.pickNexusGame(c.domain, c.name); toast.done("Get mods shows " + c.name + " mods.") } })).concat([
            { icon: "info", title: "Search Nexus", detail: "Type the game's name", run: () => nexusSearchAsk() },
            { icon: "remove", title: "It isn't on Nexus", detail: "You can still add mod files", run: () => { nexusFound = null; Turbine.pickNexusGame("", "") } }
        ]), "")
        pickY = { label: "Search", run: () => nexusSearchAsk() }
    }
    function nexusSearchAsk() { openAsk("nexusgame", "Search Nexus", "The game's name, or part of it.", game ? game.name : "", "Game name", 60, false, "Search") }
    // Y on the pick sheet (Search), when the sheet has one.
    property var pickY: null
    onPickTitleChanged: if (pickTitle.indexOf("Which game on Nexus") !== 0) pickY = null
    readonly property string nexusLine: basicGame && rules && rules.domain && rules.nexusName ? "From Nexus: " + rules.nexusName : ""
    readonly property bool nexusUnknown: basicGame && rules && !rules.domain && (rules.domainState === "ask" || rules.domainState === "unknown")
    // Anti-cheat: an info card once per game, and a line before Play while mods are on.
    function maybeAntiCheat() {
        if (!game || !rules || !rules.antiCheat || page !== "game" || sheet !== "" || shot.strictSheets) return
        const done = parse(seen.antiCheatDone, [])
        if (done.indexOf(game.id) >= 0) return
        if (!shot.on) { done.push(game.id); seen.antiCheatDone = JSON.stringify(done) }
        note("page", "Anti-cheat card: " + game.name + " (" + rules.antiCheat.name + ")")
        const a = rules.antiCheat
        openMsg({ title: a.mode === "refuse" ? gameTitle + " needs its anti-cheat" : gameTitle + " uses anti-cheat", reason: "anti-cheat in the game",
                  text: a.mode === "refuse" ? a.refuseWords
                        : a.mode === "official" ? a.officialWords
                        : a.mode === "me3" && a.needsAsk ? "Mods for " + gameTitle + " load through Mod Engine 3, which starts it offline without its anti-cheat. Turbine asks before the first Play with mods, and never changes the game's own files."
                        : a.mode === "me3" ? "Mods for " + gameTitle + " load through Mod Engine 3. Turbine never changes the game's own files."
                        : gameTitle + " uses anti-cheat (" + a.name + "). Mods that change its files can get you kicked or banned from online play. Use mods only in offline or single player modes.",
                  notes: a.mode === "refuse" ? (removedMods.length > 0 || downloads.some(d => d.here !== false) ? ["Any mod file for it stays in Downloads, untouched."] : []) : rules.antiCheat.note ? [rules.antiCheat.note] : [],
                  buttons: [{ key: "b", label: "Close", run: () => {} }] })
    }
    // The first visit to a Basic game with no mods: what Basic support means (once per game).
    property var introSeen: null          // the games seen before this run (read once, so the card stays this run)
    // This game has somewhere to get mods (Nexus or Thunderstore).
    readonly property bool canGetMods: !!rules && (!!rules.domain || sources.length > 0)
    // An empty My mods: the centered card (any game, after a Basic game's first-visit card).
    readonly property bool emptyMods: tab === 0 && emptyModsRoom
    // My mods' empty card and Basic first-visit card, whatever the tab (they fade out with My mods while a page turns).
    readonly property bool emptyModsRoom: page === "game" && !!game && modsLoaded && mods.length === 0 && !acRefused && !basicIntroRoom && !modPage
    onEmptyModsChanged: if (emptyMods) Qt.callLater(() => note("page", "No mods card: No mods for " + gameTitle + " yet | legend " + legendHints.map(h => h[0].toUpperCase() + " " + h[1]).join(" · ")))
    readonly property bool basicIntro: tab === 0 && basicIntroRoom
    readonly property bool basicIntroRoom: page === "game" && !!game && modsLoaded && mods.length === 0 && basicGame && !acRefused
                                       && (introSeen || []).indexOf(game.id) < 0
    onBasicIntroChanged: {
        if (introSeen === null) introSeen = parse(seen.basicIntroDone, [])
        if (basicIntro && !shot.on) { let d = parse(seen.basicIntroDone, []); if (d.indexOf(game.id) < 0) { d.push(game.id); seen.basicIntroDone = JSON.stringify(d) } }
        if (basicIntro) note("page", "First visit card: Mods for " + gameTitle)
    }
    function placementWords(m) {
        const p = placements[m.id] || guardedMods[m.id]
        if (!p) return []
        return (placements[m.id] ? [{ icon: "info", title: "Installed in " + folderWords(p.to), detail: p.rule === "you" ? "By you" : "By Turbine's rule " + p.rule, act: "place" }] : [])
            .concat((p.skipped || []).length > 0 ? [{ icon: "info", title: "Skipped: " + plural(p.skipped.length, "file the game already has", "files the game already has"),
                                                      detail: "A lets this mod replace them (the game's own are kept)", act: "replaceon" }]
                    : p.replace ? [{ icon: "info", title: "Replaces the game's own files", detail: "A leaves them alone again", act: "replaceoff" }] : [])
    }

    // ---------------------------------------------------------------- Add mods from Downloads
    property int addIndex: 0
    function openAdd() { Turbine.listDownloads(); addIndex = 0; addAll = false; sheet = "add" }
    // Add from Downloads' Find pop-up. The first row is which files show; B goes back to the list.
    function openAddFind() {
        const pickShow = (all) => { addAll = all; addIndex = 0; note("action", all ? "Add: all files" : "Add: this game's files"); sheet = "add" }
        openPick("Find mod files", "Which files Add from Downloads lists", [
            { icon: "files", title: "This game's files", detail: "In Downloads and Game Mods/" + gameTitle + (addAll ? "" : " · Showing now"), aLabel: "Show", run: () => pickShow(false) },
            { icon: "files", title: "All files", detail: "Every mod file in Downloads and its Game Mods folders" + (addAll ? " · Showing now" : ""), aLabel: "Show", run: () => pickShow(true) }], "add")
        menuIndex = addAll ? 1 : 0
    }
    // Edit settings' options (X): how the file shows, and Reset to default.
    function openSettingsOptions() {
        const rows = []
        if (settingsView.fields.length > 0)
            rows.push({ icon: "info", title: settingsView.plain ? "Show as a list" : "Show as text", detail: settingsView.plain ? "Each setting with its own control" : "The file's lines as they are", aLabel: "Show",
                        run: () => { settingsView.toggleView(); sheet = "settingsedit" } })
        rows.push({ icon: "restore", title: "Reset to default", detail: settingsMod === "" ? "Puts back the file as it was before you first changed it" : "Puts back the mod's own file", aLabel: "Reset", run: settingsResetAsk })
        openPick("Options", settingsView.title, rows, "settingsedit")
    }
    function addChosen() {
        const d = addList[addIndex]
        // Nothing to add: the sheet stays.
        if (!d) return
        sheet = ""
        note("action", "Add from Downloads: " + d.name)
        tab = 0
        Turbine.importZip(d.path)
        edited()
    }
    function age(sec) {
        const d = Math.floor((Date.now() / 1000 - sec) / 86400)
        return d <= 0 ? "Today" : d === 1 ? "Yesterday" : d + " days ago"
    }
    function sizeWords(b) {
        if (b >= 1048576) return (b / 1048576).toFixed(1) + " MB"
        return Math.max(1, Math.round(b / 1024)) + " KB"
    }

    // ---------------------------------------------------------------- controller and keyboard
    // zone: where the controller focus is: "rail" (games and buttons on the left), "main" (the game page),
    // "settings", "perf" and the like. sheet: what is open on top ("" for none; modmenu, gamemenu, pick, msg, ...).
    property string page: "game"          // game, settings, perf, place, browser
    property string zone: "main"
    property string sheet: ""
    property int railIndex: 0             // 0..games.length-1, then "add", then "settings"
    property int tab: 0                   // 0 Mods, 1 Get Mods, 2 Load order
    property int modIndex: 0
    property int setSection: 0            // an index into settingsSections
    property int setRow: 0
    property int perfIndex: 0
    // The games, then "Show hidden games" when every game is hidden, then Settings.
    readonly property int railExtra: games.length === 0 && hiddenIds.length > 0 ? 1 : 0
    readonly property int railCount: games.length + railExtra + 1
    function showHiddenGames() {
        note("action", "Show hidden games")
        if (hiddenTest !== null) hiddenTest = []
        else if (!shot.on) railSet.hidden = "[]"
        orderGames()
        zone = "rail"; railIndex = 0
        toast.done("Every game is back on the rail.")
    }
    // Sheets open from a press. The ones that come from outside Turbine (a download that arrived, an installer's
    // questions, install notes) and the first-run tutorial say why; anything else with no press near it is a bug.
    readonly property var sheetReasons: ({ found: "a download arrived", fomod: "the installer asks", notes: "install notes", chain: "the next requirement",
                                           tutorial: "first run", quick: "the game closed early",
                                           msg: "a mod or the game needs a word" })
    onSheetChanged: {
        if (sheet === "") note("page", "Sheet closed")
        else if (pressRecent() || (shot.on && !shot.strictSheets)) note("page", "Sheet " + sheet + " (after " + (lastInputName || "start") + ")")
        else if (sheetReasons[sheet]) note("page", "Sheet " + sheet + " (" + sheetReasons[sheet] + ")")
        else note("page", "Sheet " + sheet + " opened with no press")
        menuIndex = 0; stopDangerHolds()
        if (sheet !== "") closeWheel()
        if (sheet !== "" && barsHidden) barsHidden = false
        // Other files' place in the history goes with its sheet.
        if (sheet !== "versions" && history.some(x => x.kind === "files")) history = history.filter(x => x.kind !== "files")
        // The text prompt lives only while it is the sheet (another sheet replacing it closes it unanswered).
        if (sheet !== "ask" && askView.shown) { note("page", "Text entry closed: another sheet replaced it"); askView.close(); reportEditing(false) }
        // An installer's choices: the Nexus page closes so they show; any other sheet over the page gives the page
        // its keyboard focus back when it closes.
        if (sheet === "fomod" && page === "browser") closeBrowser("installer")
        else if (sheet === "" && page === "browser" && nexusView) Qt.callLater(() => { if (nexusView) nexusView.focusPage() })
        pickHover()
        if (sheet === "" && fomod) sheet = "fomod"
        else if (sheet === "" && found) { infoIndex = 0; sheet = "found" }
        else if (sheet === "" && pendingNotes && page !== "browser") { pendingNotes = false; infoIndex = 0; sheet = "notes" }
        else if (sheet === "") offerLoader()
        // "Which game on Nexus is this?" waited for the sheet that was up (the anti-cheat card comes first).
        if (sheet === "" && tab === 1) Qt.callLater(() => app.maybeAskNexusGame("the sheet before it closed"))
    }

    Timer { interval: 16; repeat: true; running: app.active || shot.on; onTriggered: Turbine.poll() }

    // Holding the D-pad repeats; after 1.5 s each repeat moves 5 rows.
    property string repeatButton: ""
    property real repeatSince: 0
    Timer {
        id: repeater
        interval: 380
        repeat: true
        onTriggered: {
            interval = 85
            const fast = Date.now() - app.repeatSince > 1500 && (app.repeatButton === "up" || app.repeatButton === "down")
            // On the Nexus page the sped-up repeat is a page up or down at a time (not five link hops).
            if (fast && app.page === "browser" && app.browserTab === 0 && app.sheet === "" && !app.wheelOpen) {
                if (interval !== 260) interval = 260
                Turbine.browserPageKey(app.repeatButton === "up" ? "pageup" : "pagedown")
                return
            }
            const n = fast ? 5 : 1
            app.padRepeat = true
            for (let i = 0; i < n; i++) app.act(app.repeatButton)
            app.padRepeat = false
        }
    }
    // A tap of Start opens Settings (again, or B, goes back). Holding Start for 3 seconds
    // plays: a brass ring fills on the bar's Play button meanwhile, and letting go early does nothing. What the hold
    // does is a setting (Look, "Hold Start does"): show the Play choice, or play the last choice at once.
    readonly property int startHoldMs: 3000
    readonly property int startTapMs: 500
    property real startSince: 0
    property real startHeld: 0            // 0 to 1 while Start is held
    readonly property bool startHoldPlays: look.holdStartDoes === "last"
    readonly property string startHoldWords: (keysLegend ? "hold Space: " : "hold: ") + (startHoldPlays ? "Play" : "Play options")
    Timer {
        id: startHold
        interval: 50
        repeat: true
        onTriggered: {
            const part = Math.min(1, (Date.now() - app.startSince) / app.startHoldMs)
            // Something opened over the page meanwhile, or there is nothing to play here: no ring, and no Play.
            if (!app.startHoldReady()) { app.startHeld = 0; if (part >= 1) stop(); return }
            app.startHeld = part
            if (part >= 1) { stop(); app.startHeld = 0; app.startHoldDone() }
        }
    }
    // Where a hold of Start counts: a game's screen or Settings with nothing over it, and the Nexus page after an add.
    function startHoldReady() {
        if (sheet !== "" || wheelOpen || moving || orderMoving) return false
        if (page === "browser") return browserAddedGame !== ""
        return (page === "game" || page === "settings") && !!game
    }
    function startHoldBegin() {
        startSince = Date.now(); startHeld = 0
        startHold.restart()
        startGlide = 0
        if (mo.on) startGlideAnim.restart()
    }
    // What the ring draws. The hold itself is timed by startHold alone; this only fills the ring evenly
    // between its ticks (with animations off the ring steps with the ticks, as before).
    property real startGlide: 0
    NumberAnimation { id: startGlideAnim; target: app; properties: "startGlide"; from: 0; to: 1; duration: app.startHoldMs; easing.type: mo.even }
    readonly property real startRingPart: mo.on && startGlideAnim.running ? Math.min(startGlide, startHeld + 0.05) : startHeld
    onStartHeldChanged: if (startHeld === 0) { startGlideAnim.stop(); startGlide = 0 }
    function startHoldDone() {
        note("action", "Start held 3 seconds: " + (page === "browser" ? "Play after the add" : startHoldPlays ? "Play my last choice" : "the Play choice"))
        if (page === "browser") { browserPlay(); return }
        if (!game) return
        if (page === "settings") settingsBack()
        if (page !== "game") return
        if (startHoldPlays) playLast(); else playChoice()
    }
    // A tap of Start: Settings, and back from it.
    function startTap() {
        if (page === "settings") { note("action", "Start: back from Settings"); settingsBack() }
        else if (page === "game") { note("action", "Start: Settings"); openSettings() }
    }
    // The Play choice: with Seamless Co-op its own question (the last choice has the ring), otherwise Play options.
    function playChoice() {
        if (!game) return
        if (seamlessIn && !acRefused) playPressed(); else openPlayOptions()
    }
    // The last choice, started at once.
    function playLast() {
        if (!game) return
        if (!seamlessIn || acRefused) { startPlay(); return }
        const k = lastPlayChoice
        note("action", "Play my last choice: " + k)
        if (k === "coop") startCoopChecked()
        else if (k === "plain") Turbine.playOriginal()
        else startPlay()
    }
    // Every press counts as input: a sheet that opens with no press near it is a bug (logged, and checked by the build).
    property int inputCount: 0
    property real lastInputAt: 0
    property string lastInputName: ""
    function pressRecent() { return Date.now() - lastInputAt < 2500 }
    // The back grips (L4 L5 R4 R5) send nothing in Steam's default layout; when a layout sends them: L4/R4 are LT/RT,
    // L5 is Y and R5 is View. They are never in the legend.
    function press(b) {
        // The start-up splash: any press skips it and does nothing else.
        if (splash.shown) { splash.finish("press " + b); ignoredRelease = b; return }
        inputCount++; lastInputAt = Date.now(); lastInputName = b
        if (lastInput === "touch") lastInput = "pad"
        const grip = { l4: "lt", r4: "rt", l5: "y", r5: "view" }
        if (grip[b]) b = grip[b]
        // The Nexus page closed by itself a moment ago: A, Start and RT were meant for the page (never Play).
        if ((b === "a" || b === "start" || b === "rt") && Date.now() - closedAddedAt < 600) {
            note("pad", "Press " + b + " ignored: the Nexus page closed by itself " + (Date.now() - closedAddedAt) + " ms ago")
            ignoredRelease = b
            return
        }
        if (page === "browser") browserPressed()
        if (b === "a") aHeld = true
        // R3 here means the pointer was hidden: the same as A on the focused item (never the start of a hold).
        if (b === "r3") { r3Click(); return }
        if (b === "start" && sheet === "" && !wheelOpen && !moving && !orderMoving && (page === "game" || page === "settings" || page === "browser")) { startHoldBegin(); return }
        if (["up", "down", "left", "right"].indexOf(b) >= 0) {
            repeatButton = b
            repeatSince = Date.now()
            repeater.interval = 380
            repeater.restart()
        }
        if (b === "view" && sheet === "" && !moving && !orderMoving) { wheelPress(); return }
        if (b === "a" && sheet === "confirm" && confirmIndex === 0) { startConfirm(); return }
        act(b)
    }
    property string ignoredRelease: ""
    function release(b) {
        const grip = { l4: "lt", r4: "rt", l5: "y", r5: "view" }
        if (grip[b]) b = grip[b]
        if (b === ignoredRelease) { ignoredRelease = ""; return }
        // A controller went away (pad.rs let go of its buttons first).
        if (b === "lost") { stopHolds("a controller went away"); return }
        if (b === "a") aHeld = false
        if (b === "start" && startHold.running) {
            const ms = Date.now() - startSince
            startHold.stop(); startHeld = 0
            startGlideAnim.stop(); startGlide = 0
            // A tap is Settings; a hold let go before the 3 seconds does nothing at all.
            if (ms < startTapMs) act("start")
            else note("pad", "Start let go after " + (ms / 1000).toFixed(1) + " s: nothing (a hold is 3 seconds)")
            return
        }
        if (b === repeatButton) { repeater.stop(); repeatButton = "" }
        if (b === "a" && resetHold.running) stopReset()
        if (b === "a" && confirmHold.running) stopConfirm()
        if (b === "view") wheelRelease()
    }
    // R3 with the pointer hidden: A on the focused item. The pointer stays hidden, so the triggers keep paging.
    // A hold (Delete for good, Reset) never starts from R3: it says to hold A instead.
    // While a toast with a next step shows ("R3 Undo"), R3 does that step instead.
    function r3Click() {
        if (toastActionLive()) { note("pad", "R3: the toast's " + toast.actionLabel); runToastAction(); return }
        note("pad", "R3: same as A on the focus")
        act("a")
    }
    function toastActionLive() { return !!toastAction && toast.shown && sheet === "" && !wheelOpen && page !== "browser" }
    // One tap, one action: a chip's own handler and its row's both see the same tap (the chip's comes first).
    property real lastTapAt: 0
    function tapOnce() { const now = Date.now(); if (now - lastTapAt < 150) return false; lastTapAt = now; return true }
    // A click or tap on a row: the focus goes there (focus()), then A runs, exactly as the button would.
    function rowA(focus, settings) {
        if (sheet !== "" || wheelOpen || !tapOnce()) return
        focus()
        if (!settings) zone = "main"; else settingsFresh = false
        note("action", "Click on a row: A")
        act("a")
    }
    // The left stick's wheel with the pointer hidden: at the focused item, which then follows what is in view.
    function stickWheel(d) {
        const p = focusCenter()
        Turbine.wheelAt(p.x, p.y, d)
    }
    // ---- The focus ring glides. Each thing draws its own ring where the focus rests, fading in as the focus
    // arrives and out as it leaves; when the focus moves, a ring travels from the last one to the new one (Motion's
    // quick, on the glide curve) and fades as it lands. Only at Full, only on the main screens (the game list, My
    // mods, Get mods, Play), and never while a list is still gliding to its row.
    property Item focusWas: null
    function focusItemNow() {
        if (sheet !== "" || wheelOpen || page === "browser" || page === "place") return null
        if (zone === "rail") return rail.itemAt(railIndex)
        if (zone === "play") return legend.playBtn.visible ? legend.playBtn : null
        if (page !== "game" || modPage || zone !== "main") return null
        if (tab === 0) return modIndex >= 0 && modList.visible ? modList.itemAtIndex(modIndex) : null
        if (tab === 1 && getArea === "grid" && getView.visible) return getView.gridItem(getIndex)
        return null
    }
    function focusMoved() { if (mo.moves) Qt.callLater(focusTrail) }
    function focusTrail() {
        let now = null
        try { now = focusItemNow() } catch (e) { now = null }
        const was = focusWas
        focusWas = now
        if (!mo.moves || !now || !was || was === now || glideAnim.running || turnTo >= 0) return
        try {
            if (!was.visible || !now.visible) return
            const a = was.mapToItem(null, 0, 0), b = now.mapToItem(null, 0, 0)
            // Both on screen (a row scrolled out of view has nowhere to glide from).
            if (a.y + was.height < 0 || a.y > height || b.y + now.height < 0 || b.y > height) return
            focusRing.x = a.x; focusRing.y = a.y; focusRing.width = was.width; focusRing.height = was.height
            ringX.to = b.x; ringY.to = b.y; ringW.to = now.width; ringH.to = now.height
            ringMove.restart()
        } catch (e) {}
    }
    Connections {
        target: app
        function onModIndexChanged() { app.focusMoved() }
        function onGetIndexChanged() { app.focusMoved() }
        function onGetAreaChanged() { app.focusMoved() }
        function onRailIndexChanged() { app.focusMoved() }
        function onZoneChanged() { app.focusMoved() }
        function onTabChanged() { app.focusWas = null }
        function onPageChanged() { app.focusWas = null }
        function onSheetChanged() { app.focusWas = null }
        function onCurrentChanged() { app.focusWas = null }
    }
    Rectangle {
        id: focusRing
        z: 45
        visible: opacity > 0
        opacity: 0
        color: "transparent"
        radius: app.px(6)
        border.color: c.accentHi
        border.width: Math.max(2, app.px(2))
        enabled: false
        SequentialAnimation {
            id: ringMove
            PropertyAction { target: focusRing; properties: "opacity"; value: 0.9 }
            ParallelAnimation {
                NumberAnimation { id: ringX; target: focusRing; properties: "x"; duration: mo.moveQuick; easing.type: mo.glide }
                NumberAnimation { id: ringY; target: focusRing; properties: "y"; duration: mo.moveQuick; easing.type: mo.glide }
                NumberAnimation { id: ringW; target: focusRing; properties: "width"; duration: mo.moveQuick; easing.type: mo.glide }
                NumberAnimation { id: ringH; target: focusRing; properties: "height"; duration: mo.moveQuick; easing.type: mo.glide }
            }
            NumberAnimation { target: focusRing; properties: "opacity"; to: 0; duration: mo.quick; easing.type: mo.ease }
        }
    }
    // The middle of whatever has the focus ring, in window pixels.
    function focusCenter() {
        let it = null
        try {
            if (sheet === "" && !wheelOpen) {
                if (zone === "rail") it = rail.itemAt(railIndex)
                else if (zone === "play") it = legend.playBtn
                else if (page === "game" && tab === 0 && !modPage && modIndex >= 0) it = modList.itemAtIndex(modIndex)
                else if (page === "game" && modPage) it = modView.focusItem ? modView.focusItem() : null
            }
            if (it) { const p = it.mapToItem(null, it.width / 2, it.height / 2); return { x: p.x, y: p.y } }
        } catch (e) {}
        return { x: width / 2, y: height / 2 }
    }
    property real confirmHeld: 0
    Timer {
        id: confirmHold
        interval: 100
        repeat: true
        onTriggered: {
            if (!app.holdAllowed()) { app.stopConfirm(); return }
            app.confirmHeld += 0.1
            if (app.confirmHeld >= app.holdNeeded) { stop(); app.confirmHeld = 0; app.holdsDone++; app.confirmDone() }
        }
    }
    function startConfirm() {
        if (pressTwice) { confirmTwice(deleteKey(), confirmWords(), () => { holdsDone++; confirmDone() }); return }
        if (confirmHold.running) return
        if (!holdAllowed()) { holdHint(holdNeeded + " seconds to " + confirmWords()); return }
        confirmHeld = 0; confirmHold.start()
    }
    function stopConfirm() { confirmHold.stop(); confirmHeld = 0 }

    // A key was pressed: the hints follow the keyboard from here.
    function keySeen(name) {
        if (lastInput !== "keys") lastInput = "keys"
        // When the key came. Qt hovers the items under a still mouse again when the screen changes
        // (a wheel or sheet closing after a key): that is not the mouse being used, so it doesn't turn the hints
        // to the mouse's (the toast after Esc on the wheel lost its "Ctrl+Z").
        keyAt = Date.now()
        inputCount++; lastInputAt = Date.now(); lastInputName = name
    }
    // Ctrl+= and Ctrl+-, and Ctrl with the mouse wheel: the interface size, a step at a time.
    function sizeBy(d, how) {
        const v = sizeStep(d)
        note("action", how + ": interface size " + sizeNow + "% to " + v + "%")
        if (v !== sizeNow) sizePreview(v)
    }
    // The mouse's Back button is B, wherever the pointer is.
    function mouseBack() {
        if (lastInput !== "mouse") lastInput = "mouse"
        inputCount++; lastInputAt = Date.now(); lastInputName = "b"
        note("mouse", "Mouse Back: B")
        act("b")
    }
    // Tab and Space each do one thing on a tap and another when held (see Keys.onPressed below).
    property bool tabDown: false
    property double keyAt: 0
    // The real mouse moved: the hints follow it. Qt's hover again right after a key (a wheel or sheet closing under
    // a resting mouse) is not the mouse being used.
    function mouseSeen() {
        if (Turbine.pointerShown || Date.now() - keyAt < 400) return
        if (lastInput !== "mouse") lastInput = "mouse"
    }
    property int hoverSeen: 0              // the checks' count of the real mouse's hovers
    property double hoverSeenAfterKey: -1
    property bool tabWheel: false
    property bool spaceDown: false
    readonly property int tabHoldMs: 220
    Timer {
        id: tabHold
        interval: app.tabHoldMs
        onTriggered: {
            if (!app.tabDown || app.sheet !== "" || app.wheelOpen || app.moving || app.orderMoving) return
            app.tabWheel = true
            app.lastInputName = "view"
            app.note("key", "Tab held: the quick wheel")
            app.wheelPress()
        }
    }
    // Where Esc has nothing to go back from: a game's first page (My mods), on the list or the games.
    readonly property bool escTop: page === "game" && sheet === "" && !wheelOpen && !moving && !orderMoving && !selecting && !modPage && tab === 0 && (zone === "main" || zone === "rail")
    // The mouse's Forward button is the next page (RB), wherever the pointer is.
    function mouseForward() {
        if (lastInput !== "mouse") lastInput = "mouse"
        inputCount++; lastInputAt = Date.now(); lastInputName = "rb"
        note("mouse", "Mouse Forward: the next page")
        act("rb")
    }
    // A double-click on a mod opens it: its details (what it needs, what it wins or loses, where it goes), or
    // its options when there is nothing more to tell. Each of the two clicks was A on the row, so the mod is first
    // put back the way it was before the first one.
    // Counted here, not by the row: the first click turns the mod on or off, which can make the list build the row
    // again, and a new row never sees the second click as a double-click.
    property var clickWas: null
    function rowClickSeen(i, m) {
        if (!m || !m.id) return false
        if (clickWas && clickWas.id === m.id && Date.now() - clickWas.at <= Qt.styleHints.mouseDoubleClickInterval) { rowOpen(i, m.id); return true }
        clickWas = { id: m.id, enabled: m.enabled === true, at: Date.now() }
        return false
    }
    function rowOpen(i, id) {
        if (moving || orderMoving || selecting || wheelOpen || sheet !== "") return
        const was = clickWas, now = mods.find(x => x.id === id)
        clickWas = null
        if (!now) return
        if (was && was.id === id && (now.enabled === true) !== was.enabled) toggle(now, true)
        if (sheet !== "") return
        modIndex = i; zone = "main"
        if (!menuMod || menuMod.id !== id) return
        note("mouse", "Double-click on a mod: " + (modMenuHasFacts ? "its details" : "its options"))
        if (modMenuHasFacts) openModDetails(menuMod); else sheet = "modmenu"
    }
    Item {
        id: keyRoot
        anchors.fill: parent
        focus: true
        // A real mouse (not the controller pointer, which Turbine draws itself) that moved since the last key.
        HoverHandler {
            onPointChanged: {
                app.hoverSeen++; app.hoverSeenAfterKey = Date.now() - app.keyAt
                app.mouseSeen()
            }
        }
        Keys.onPressed: (e) => {
            app.pressFresh = false
            // Ctrl+Enter is the keyboard's hold of Start (the Play choice, or Play).
            if ((e.modifiers & Qt.ControlModifier) && (e.key === Qt.Key_Return || e.key === Qt.Key_Enter)) {
                e.accepted = true
                if (e.isAutoRepeat) return
                app.note("key", "Key Ctrl+Enter")
                app.inputCount++; app.lastInputAt = Date.now(); app.lastInputName = "start"
                if (app.startHoldReady()) { app.startHoldDone() }
                return
            }
            // The keyboard is as good as the controller. Arrows move, Enter (or Space) is A, Esc or Backspace
            // is B, Tab and Shift+Tab (Page Down and Page Up too) flip pages, Ctrl+F or / finds, the Menu key or
            // Shift+F10 opens options, F1 is Help, Ctrl+= and Ctrl+- change the size, Ctrl+Z is a toast's step
            // (Undo), Home and End jump a screen. The letters stay: X, Y or F, Q and E, V the wheel, P Start.
            // The keys a PC player reaches for (many keyboards have no Page Up or Page Down): W A S D move
            // like the arrows, Q and E turn the pages ([ and ] too), F finds, R opens options, Esc on a game's
            // first page opens Settings (a tap of Start), Tab held opens the quick wheel (let go to choose; a
            // tap of Tab is still the next page) and Space held for 3 seconds plays, with Start's ring (a tap
            // of Space is still A). A text field that is being typed in has the keys itself: none of these
            // reach here then.
            const ctrl = (e.modifiers & Qt.ControlModifier) !== 0, shift = (e.modifiers & Qt.ShiftModifier) !== 0
            if (ctrl && (e.key === Qt.Key_Equal || e.key === Qt.Key_Plus || e.key === Qt.Key_Minus)) {
                e.accepted = true
                app.keySeen("size")
                app.sizeBy(e.key === Qt.Key_Minus ? -1 : 1, "Key Ctrl" + (e.key === Qt.Key_Minus ? "-" : "+"))
                return
            }
            if (e.key === Qt.Key_F1) {
                e.accepted = true
                if (e.isAutoRepeat) return
                app.keySeen("help"); app.note("key", "Key F1")
                if (app.sheet === "" && !app.wheelOpen && app.page !== "browser") app.openSection("Help")
                return
            }
            const map = {}
            map[Qt.Key_Up] = "up"; map[Qt.Key_Down] = "down"; map[Qt.Key_Left] = "left"; map[Qt.Key_Right] = "right"
            map[Qt.Key_Return] = "a"; map[Qt.Key_Enter] = "a"; map[Qt.Key_Space] = "a"
            map[Qt.Key_Escape] = "b"; map[Qt.Key_Backspace] = "b"
            map[Qt.Key_Tab] = "rb"; map[Qt.Key_Backtab] = "lb"; map[Qt.Key_PageDown] = "rb"; map[Qt.Key_PageUp] = "lb"
            map[Qt.Key_Home] = "lt"; map[Qt.Key_End] = "rt"
            map[Qt.Key_Slash] = "y"; map[Qt.Key_Menu] = "x"
            map[Qt.Key_X] = "x"; map[Qt.Key_Y] = "y"; map[Qt.Key_F] = "y"
            map[Qt.Key_Q] = "lb"; map[Qt.Key_E] = "rb"
            map[Qt.Key_P] = "start"; map[Qt.Key_V] = "view"
            map[Qt.Key_W] = "up"; map[Qt.Key_S] = "down"; map[Qt.Key_A] = "left"; map[Qt.Key_D] = "right"
            map[Qt.Key_R] = "x"; map[Qt.Key_BracketLeft] = "lb"; map[Qt.Key_BracketRight] = "rb"
            let b = undefined
            if (ctrl) b = e.key === Qt.Key_F ? "y" : e.key === Qt.Key_Z ? "r3" : undefined
            else if (shift && e.key === Qt.Key_F10) b = "x"
            else if (shift && e.key === Qt.Key_Tab) b = "lb"
            else b = map[e.key]
            if (!b) return
            e.accepted = true
            const plain = !ctrl && !shift && !(e.modifiers & Qt.AltModifier)
            // Esc on a game's first page (nothing to go back from) opens Settings, like a tap of Start.
            if (e.key === Qt.Key_Escape && plain && app.escTop) {
                if (e.isAutoRepeat) return
                app.note("key", "Key Esc: Settings"); app.keySeen("start")
                app.startTap()
                return
            }
            // Tab. A tap is the next page (on letting go); held, the quick wheel opens and letting go
            // chooses what was picked. With a sheet up it is the next page at once, as before.
            if (e.key === Qt.Key_Tab && plain) {
                if (e.isAutoRepeat) return
                app.note("key", "Key Tab"); app.keySeen("rb")
                if (app.wheelOpen) { app.act("view"); return }
                if (app.sheet !== "" || app.moving || app.orderMoving) { app.act("rb"); return }
                app.tabDown = true; app.tabWheel = false
                tabHold.restart()
                return
            }
            // Space. A tap is A (on letting go); held for 3 seconds where Start's hold counts, it plays,
            // with the same ring on the Play button. With a sheet up it is A at once, as before.
            if (e.key === Qt.Key_Space && plain) {
                if (e.isAutoRepeat && app.spaceDown) return
                if (!e.isAutoRepeat && app.startHoldReady()) {
                    app.note("key", "Key Space"); app.keySeen("a")
                    app.spaceDown = true
                    app.startHoldBegin()
                    return
                }
            }
            // Held down, only the arrows and the screen jumps repeat.
            if (e.isAutoRepeat && ["up", "down", "left", "right", "lt", "rt", "a"].indexOf(b) < 0) return
            if (!e.isAutoRepeat) app.note("key", "Key " + b)
            app.keySeen(b)
            if (b === "r3") { app.r3Click(); return }
            if (b === "start" || b === "view") { if (!e.isAutoRepeat) app.press(b); return }
            if (b === "a" && !e.isAutoRepeat) app.aHeld = true
            // Holding a key repeats it, except A (Reset waits for A to be held down).
            if (b === "a" && app.sheet === "confirm" && app.confirmIndex === 0) { if (!e.isAutoRepeat) app.startConfirm(); return }
            if (!(b === "a" && e.isAutoRepeat)) { app.padRepeat = e.isAutoRepeat; app.act(b); app.padRepeat = false }
        }
        Keys.onReleased: (e) => {
            if (!e.isAutoRepeat && e.key === Qt.Key_Tab && app.tabDown) {
                app.tabDown = false
                if (tabHold.running) { tabHold.stop(); app.act("rb") }
                else if (app.tabWheel) { app.tabWheel = false; app.wheelRelease() }
                return
            }
            if (!e.isAutoRepeat && e.key === Qt.Key_Space && app.spaceDown) {
                app.spaceDown = false
                if (startHold.running) {
                    const ms = Date.now() - app.startSince
                    startHold.stop(); app.startHeld = 0
                    startGlideAnim.stop(); app.startGlide = 0
                    // A tap is A; a hold let go before the 3 seconds does nothing at all.
                    if (ms < app.startTapMs) app.act("a")
                    else app.note("key", "Space let go after " + (ms / 1000).toFixed(1) + " s: nothing (a hold is 3 seconds)")
                }
                return
            }
            if (!e.isAutoRepeat && (e.key === Qt.Key_Return || e.key === Qt.Key_Enter || e.key === Qt.Key_Space)) app.release("a")
            if (!e.isAutoRepeat && e.key === Qt.Key_V) app.release("view")
            if (!e.isAutoRepeat && e.key === Qt.Key_P) app.release("start")
        }
    }

    // Sheets with a row of buttons (SheetButtons): the D-pad moves the highlight, A presses it, B is the cancel
    // button, Start the main one. No other keys. {labels, index, main, cancel, set(i), run(i), scrolls, scroll(d)}.
    readonly property var infoSheets: ["nobrowser", "helper", "notes", "found", "quick", "chain", "addbrowser", "msg"]
    function buttonSheet() {
        const s = sheet
        if (s === "check") return { labels: checkChoices, index: checkIndex, main: checkStops ? -1 : checkChoices.indexOf(checkPlayWord), cancel: checkChoices.indexOf("Not now"),
                                    skip: checkView.cancelHidden ? checkChoices.indexOf("Not now") : -1,
                                    set: i => { checkIndex = i }, run: i => checkChoose(i), scrolls: checkView.scrolls, scroll: d => checkView.scroll(d) }
        if (s === "loader") return { labels: loaderLabels, index: loaderIndex, main: 0, cancel: 1, skip: -1, set: i => { loaderIndex = i }, run: i => loaderRun(i), scrolls: false }
        if (infoSheets.indexOf(s) >= 0) {
            const cancel = infoButtons.findIndex(x => x.key === "b")
            return { labels: infoButtons.map(x => x.label), index: infoIndex, main: 0, cancel: cancel, skip: infoView.cancelHidden ? cancel : -1, set: i => { infoIndex = i }, run: i => infoRun(i),
                     scrolls: infoView.scrolls, scroll: d => infoView.scrollBy(d) }
        }
        if (s === "ask") return { labels: ["Clear", "Cancel", askView.doneLabel], index: askIndex, main: 2, cancel: 1, set: i => { askIndex = i }, run: i => askRun(i), scrolls: false }
        if (s === "confirm") return { labels: ["Delete", "Keep"], index: confirmIndex, main: -1, cancel: 1, set: i => { confirmIndex = i },
                                      run: i => { if (i === 1) { stopConfirm(); sheet = confirmWhat.indexOf("profile:") === 0 ? "profiles" : confirmWhat.indexOf("saves:") === 0 || confirmWhat.indexOf("acleftover:") === 0 ? "" : removedMods.length > 0 ? "removed" : "" } }, scrolls: false }
        return null
    }
    // The build's check of the button rule: with sheetDry on, a press only says which button it would run.
    property bool sheetDry: false
    function buttonSheetAct(bs, b) {
        const n = bs.labels.length
        // Text entry: the field first (A opens Steam's keyboard again), then Clear, Cancel and Done.
        if (sheet === "ask" && askIndex < 0 && (b === "down" || b === "right")) { askIndex = 2; return }
        if (sheet === "ask" && askIndex >= 0 && b === "up") { askIndex = -1; return }
        const runIt = i => {
            if (i < 0 || i >= n) return
            if (sheetDry) { lastDry = bs.labels[i]; note("check", "Sheet " + sheet + " " + b.toUpperCase() + ": " + bs.labels[i]); return }
            bs.run(i)
        }
        // A cancel button with no room in the row (B does it) is passed over.
        const step = d => { let i = bs.index + d; if (i === bs.skip) i += d; if (i >= 0 && i < n) bs.set(i) }
        if ((b === "up" || b === "down") && bs.scrolls) bs.scroll(b === "down" ? 1 : -1)
        else if (b === "left" || b === "up") step(-1)
        else if (b === "right" || b === "down") step(1)
        else if (b === "a") {
            if (sheet === "ask" && askIndex < 0) askKeyboard()
            else if (sheet !== "confirm" || bs.index === 1) runIt(Math.min(bs.index, n - 1))
            // A that isn't a held A (R3, a tap): Delete needs the hold.
            else if (!sheetDry && !confirmHold.running && !pressTwice) holdHint(holdNeeded + " seconds to " + confirmWords())
        }
        else if (b === "b") { if (bs.cancel >= 0) runIt(bs.cancel); else if (!sheetDry) sheet = "" }
        // Start does nothing on these (a tap of Start never plays; A on the highlighted button does).
    }
    function sheetBinds() {
        const bs = buttonSheet()
        if (bs) {
            const hl = sheet === "ask" && askIndex < 0 ? (Turbine.gamemode ? "Keyboard" : "") : sheet === "confirm" && confirmIndex === 0 ? "Delete" : bs.labels[bs.index] || ""
            let o = { a: bind(hl, () => sheetAct("a"), sheet === "confirm" && confirmIndex === 0 ? "hold" : "") }
            // B says nothing new when the highlighted button is the cancel one ("A Not now", not "A Not now B Not now").
            if (!(bs.cancel >= 0 && bs.index === bs.cancel && hl === bs.labels[bs.cancel])) o.b = bind(bs.cancel >= 0 ? bs.labels[bs.cancel] : "Close", () => sheetAct("b"))
            else o.b = bind("", () => sheetAct("b"))
            return o
        }
        const s = sheet
        const k = key => () => sheetAct(key)
        // Choices are already in effect, so B is Done; Start does the same but isn't shown (Start is Play).
        if (s === "findpop") return { a: bind(findPopALabel(), k("a")), b: bind("Done", k("b")), x: bind("Clear all", k("x")), y: bind("Keyboard", k("y")), start: bind("", k("start")) }
        if (s === "modmenu" || s === "gamemenu" || s === "cardmenu") {
            // A says what the highlighted row does ("A Uninstall", "A Rename").
            const row = s === "modmenu" ? modMenuItems[menuIndex] : s === "cardmenu" ? cardMenuItems[menuIndex] : gameMenuItems[menuIndex]
            // The game menu's A names its row too; a dimmed row says why instead.
            return { a: bind(row && s === "gamemenu" && !row.ok ? "Why not" : row && row.title.length <= 24 ? row.title : "Open", k("a")), b: bind("Close", k("b")) }
        }
        if (s === "pick") return Object.assign({ a: bind(!pickItems[menuIndex] || pickItems[menuIndex].info ? "" : pickItems[menuIndex].aLabel ? pickItems[menuIndex].aLabel : "Choose", k("a")),
                                                 b: bind(pickBLabel !== "" ? pickBLabel : pickBack !== "" ? "Back" : "Close", k("b")) },
                                               pickY ? { y: bind(pickY.label, k("y")) } : {})
        if (s === "removed") return { a: bind(removedIndex === removedMods.length ? "Delete all" : "Put back", k("a")), b: bind("Close", k("b")), x: bind(removedIndex === removedMods.length ? "" : "Delete now", k("x")) }
        if (s === "add") return { a: bind(addList[addIndex] ? "Add" : "", k("a")), b: bind("Close", k("b")), y: bind("Find", k("y")) }
        if (s === "shares") return { a: bind("Read", k("a")), b: bind("Back", k("b")) }
        if (s === "profiles") return { a: bind("Choose", k("a")), b: bind("Close", k("b")), x: bind(!importData && profIndex < profileList.length ? "Options" : "", k("x")) }
        if (s === "pages") return { a: bind("Open", k("a")), b: bind("Close", k("b")) }
        if (s === "versions") return { a: bind(otherVersions[versionIndex] && otherVersions[versionIndex].installed ? "" : detailsData && detailsData.premium === false ? "Get it on Nexus" : "Install", k("a")), b: bind("Close", k("b")) }
        if (s === "code") return { a: bind(codeData && codeData.state === "ready" ? "Copy" : "", k("a")), b: bind("Back", k("b")) }
        if (s === "plugin") return { a: bind(pluginOwner ? "Go to its mod" : "", k("a")), b: bind("Close", k("b")) }
        if (s === "pickgame") return { a: bind("Add here", k("a")), b: bind("Back", k("b")) }
        // The legend follows the installer: the stop question's buttons, Stop on the first page,
        // and Start goes on from any row.
        if (s === "fomod") return fomodView.askStop ? { a: bind(fomodView.stopFocus === 1 ? "Stop installing" : "Keep choosing", k("a")), b: bind("Keep choosing", k("b")) }
                                  : { a: bind(fomodView.onNextRow ? (fomodView.lastPage ? "Install" : "Next") : "Choose", k("a")), b: bind(fomodView.canStepBack ? "Back a step" : "Stop", k("b")),
                                      start: bind(fomodView.lastPage ? "Install" : "Next", k("start")) }
        if (s === "report") return reportView.editing ? { b: bind("Done typing", k("b")) }
                                   : { a: bind(reportView.focusAt === 2 ? "Save report" : reportView.focusAt === 3 ? "Not now" : reportView.focusAt === 1 ? "Type" : "Switch", k("a")),
                                       b: bind("Not now", k("b")), start: bind(reportView.text.trim() !== "" ? "Save report" : "", k("start")) }
        if (s === "activity") return { a: bind("", k("a")), b: bind("Close", k("b")), x: bind(activityView.full ? "What happened" : "Full log", k("x")) }
        if (s === "settingsedit") return { a: bind(settingsView.aWord, k("a")), b: bind(settingsView.leaveArmed ? "Leave" : "Back", k("b")),
                                           x: bind("Options", k("x")), start: bind("", k("start")) }
        // One button on the card (OK, Close): A says it, and B (the same) isn't said twice.
        if (s === "notice") return { a: bind("OK", k("a")), b: bind("", k("b")) }
        if (s === "details") return { a: bind("Close", k("a")), b: bind("", k("b")) }
        return { a: bind("", k("a")), b: bind("Close", k("b")) }
    }
    function sheetAct(b) {
        const bs = buttonSheet()
        if (bs) { buttonSheetAct(bs, b); return }
        if (sheet === "findpop") findPopAct(b)
        else if (sheet === "report") reportView.handleButton(b)
        else if (sheet === "perf") perfView.handleButton(b)
        else if (sheet === "activity") activityView.handleButton(b)
        else if (sheet === "settingsedit") settingsView.handleButton(b)
        else if (sheet === "details" || sheet === "notice") {
            if (b === "a" || b === "b" || b === "view") {
                if (sheet === "notice") seen.notSteamOS = true
                sheet = ""
            }
        } else if (sheet === "pick" && nxmAskQuiet(actStamp)) {
            note("nxm", "Press " + b + " ignored: made " + Math.round(actStamp - nxmShownAt) + " ms after the question showed")
        } else if (sheet === "pick" && nxmAsking && b === "a" && nxmRingMoved && actStamp < nxmShownAt + nxmRingMove.interval) {
            // An A made while the ring was still on Not now, handled after the timer moved it (a
            // stalled window): it meant Not now.
            note("nxm", "A made at " + Math.round(actStamp - nxmShownAt) + " ms, before the ring moved: Not now")
            menuIndex = 1; pickRun(1)
        } else if (sheet === "pick" && nxmAsking && (b === "left" || b === "right" || b === "up" || b === "down")) {
            // Right (or Up) to Get it, Left (or Down) back to Not now. The player chose: the ring never moves by
            // itself after this.
            nxmRingMove.stop()
            menuIndex = b === "right" || b === "up" ? 0 : 1
        } else if (sheet === "pick") {
            if (b === "up") menuIndex = Math.max(0, menuIndex - 1)
            else if (b === "down") menuIndex = Math.min(pickItems.length - 1, menuIndex + 1)
            else if ((b === "left" || b === "right") && pickItems[menuIndex] && pickItems[menuIndex].step) {
                // A row with a value (the Nexus menu's Zoom): Left and Right change it, the menu stays.
                const i = menuIndex
                pickItems[i].step(b === "left" ? -1 : 1)
                if (pickTitle === "Nexus") openBrowserMenu(i)
            }
            else if (b === "a") pickRun(menuIndex)
            else if (b === "y" && pickY) { const y = pickY; sheet = ""; y.run() }
            else if (b === "b" || b === "x") { if (pickBLabel === "Not now") nxmAskNo(); sheet = pickBack }
        } else if (sheet === "modmenu") {
            if (b === "up") menuIndex = Math.max(0, menuIndex - 1)
            else if (b === "down") menuIndex = Math.min(modMenuItems.length - 1, menuIndex + 1)
            else if (b === "a") modMenuRun(menuIndex)
            else if (b === "b" || b === "x") sheet = ""
        } else if (sheet === "gamemenu") {
            if (b === "up") { stopReset(); menuIndex = Math.max(0, menuIndex - 1) }
            else if (b === "down") { stopReset(); menuIndex = Math.min(gameMenuItems.length - 1, menuIndex + 1) }
            else if (b === "a") gameMenuRun(menuIndex)
            else if (b === "b" || b === "view" || b === "x") { stopReset(); sheet = "" }
        } else if (sheet === "add") {
            if (b === "up") addIndex = Math.max(0, addIndex - 1)
            else if (b === "down") addIndex = Math.min(addList.length - 1, addIndex + 1)
            else if (b === "a") addChosen()
            // Y: Find: its first row chooses this game's files or every mod file.
            else if (b === "y") openAddFind()
            else if (b === "b") sheet = ""
        } else if (sheet === "pages") {
            if (b === "up") pagesIndex = Math.max(0, pagesIndex - 1)
            else if (b === "down") pagesIndex = Math.min(pagesList.length - 1, pagesIndex + 1)
            else if (b === "a") openPage(pagesIndex)
            else if (b === "b") sheet = ""
        } else if (sheet === "versions") {
            if (b === "up") versionIndex = Math.max(0, versionIndex - 1)
            else if (b === "down") versionIndex = Math.min(otherVersions.length - 1, versionIndex + 1)
            else if (b === "a") installVersion(versionIndex)
            else if (b === "b" || b === "x") { sheet = ""; if (history.length > 0 && history[history.length - 1].kind === "files") history = history.slice(0, -1) }
        } else if (sheet === "code") {
            if (b === "a" && codeData && codeData.state === "ready") { Turbine.copyText(codeData.code); toast.done("Code copied.") }
            else if (b === "b") sheet = "profiles"
        } else if (sheet === "plugin") {
            if (b === "a" && pluginOwner) goToMod(pluginOwner)
            else if (b === "b" || b === "x") sheet = ""
        } else if (sheet === "pickgame") {
            if (b === "up") pickIndex = Math.max(0, pickIndex - 1)
            else if (b === "down") pickIndex = Math.min(games.length - 1, pickIndex + 1)
            else if (b === "a") pickGame(pickIndex)
            else if (b === "b") { infoIndex = 0; sheet = "found" }
        } else if (sheet === "removed") {
            if (b === "up") removedIndex = Math.max(0, removedIndex - 1)
            else if (b === "down") removedIndex = Math.min(removedMods.length, removedIndex + 1)
            else if (b === "a") removedRun(removedIndex)
            else if (b === "x") removedDelete(removedIndex)
            else if (b === "b") sheet = ""
        } else if (sheet === "cardmenu") {
            if (b === "up") menuIndex = Math.max(0, menuIndex - 1)
            else if (b === "down") menuIndex = Math.min(cardMenuItems.length - 1, menuIndex + 1)
            else if (b === "a") cardMenuRun(menuIndex)
            else if (b === "b" || b === "x") sheet = ""
        } else if (sheet === "fomod") fomodView.handleButton(b)
        else if (sheet === "profiles") {
            if (b === "up") profIndex = Math.max(0, profIndex - 1)
            else if (b === "down") profIndex = Math.min(profilesView.rows - 1, profIndex + 1)
            else if (b === "a") profileRun(profIndex)
            else if (b === "x" && !importData && profIndex < profileList.length) profileMenu(profileList[profIndex].name)
            else if (b === "b" || b === "view") { if (importData) { importData = null; profIndex = 0 } else sheet = "" }
        } else if (sheet === "shares") {
            if (b === "up") shareIndex = Math.max(0, shareIndex - 1)
            else if (b === "down") shareIndex = Math.min(shares.length - 1, shareIndex + 1)
            else if (b === "a" && shares[shareIndex]) { Turbine.profileImportFile(shares[shareIndex].path); sheet = "profiles" }
            else if (b === "b") sheet = sharesBack
        }
    }
    // B on the loadout files goes back to where they were opened from (the game menu or Loadouts).
    property string sharesBack: "profiles"
    // Turbine can fetch the loader itself (SKSE, BepInEx, SMAPI).
    readonly property bool loaderCanGet: !!rules && !!rules.loader && ["bethesda", "bepinex", "smapi"].indexOf(rules.kind) >= 0

    // ---------------------------------------------------------------- one table for every button
    // binds() says what each button does right now: {a, b, x, y, start, lb, rb, lt, rt, view} of {label, run, pre}.
    // act() runs it and the legend prints it, so the legend can't say something the button doesn't do. The D-pad
    // and the left stick move the focus (navigate()); they are never in the legend.
    function bind(label, run, pre) { return { label: label, run: run, pre: pre || "" } }
    // When the press being handled was made, on Turbine.monoMs's clock: a controller press by SDL's
    // own time stamp (a stalled window handles it late, but it was made early), anything else now.
    property real pressStamp: 0
    property bool pressFresh: false
    property real actStamp: 0
    function act(b) {
        actStamp = pressFresh ? pressStamp : Turbine.monoMs()
        pressFresh = false
        landFocus = false
        if (sheet === "tutorial") { tutorialAct(b); return }
        if (wheelOpen) { wheelAct(b); return }
        if (sheet !== "") { sheetAct(b); return }
        if (moving) {
            if (b === "up") stepMove(-1)
            else if (b === "down") stepMove(1)
            else if (b === "left") stepMove(-10)
            else if (b === "right") stepMove(10)
            else if (b === "lt") stepMove(-100000)
            else if (b === "rt") stepMove(100000)
            else if (b === "a") endMove()
            else if (b === "b") cancelMove()
            return
        }
        if (orderMoving) { orderAct(b); return }
        const e = binds()[b]
        if (e && e.run) { e.run(); return }
        navigate(b)
    }
    // How many rows LT and RT jump.
    readonly property int pageJump: Math.max(3, Math.floor((content.height - px(40)) / px(desktopLayout ? 44 : 58)))
    function binds() {
        // Y is always Find; a toast's own next step is on R3 (and a tap on the toast's button).
        return bindsCore()
    }
    function bindsCore() {
        let o = {}
        if (sheet === "tutorial") return { a: bind(tutCard < tutCount - 1 ? "Next" : "Pick a game", () => tutorialAct("a")), b: bind("Skip", () => tutorialAct("b")), start: bind("", () => tutorialAct("start")) }
        if (wheelOpen) {
            // Nothing picked (the stick at rest): A closes, like B and View.
            const s = wheelSlot >= 0 ? wheelSlots[wheelSlot] : null
            return { a: bind(s ? (s.ok ? s.label : "") : "Close", () => wheelAct("a")), b: bind("Close", () => wheelAct("b")), view: bind("", () => wheelAct("b")) }
        }
        if (sheet !== "") return sheetBinds()
        if (moving || orderMoving) return { a: bind("Done", () => act("a")), b: bind("Cancel", () => act("b")), lt: bind("", () => act("lt")), rt: bind("", () => act("rt")) }
        o.view = bind("", wheelPress)
        // A tap of Start is Settings, and back from it (press() times the hold that plays).
        if (page === "game") o.start = bind("", startTap)
        else if (page === "settings") o.start = bind("Close", startTap)
        // Nexus inside Turbine: A clicks at the pointer (or works the Downloads row), B back a page or close.
        if (page === "browser") {
            // B: back a page (held: close), or close on the first page.
            o.b = bind(browserTab === 1 ? "Page" : nexusView && nexusView.canGoBack ? "Back" : "Close", browserBack)
            o.lb = bind("", () => { browserTab = 0 })
            o.rb = bind("", () => { browserTab = 1 })
            // After an add: holding Start plays that game (a tap never plays).
            o.start = bind(browserAddedGame !== "" ? "Play" : "", () => { if (browserAddedGame !== "") toast.info("Hold " + glyph("start") + " for 3 seconds to play.") }, "hold")
            if (browserTab === 1) {
                o.a = bind(browserRowLabel(browserDlIndex), () => browserRowAct(browserDlIndex))
                // X only on a row (the menu has nothing to offer with no downloads).
                if (webDls[browserDlIndex]) o.x = bind("Options", () => browserRowMenu(browserDlIndex))
                return o
            }
            o.x = bind("Options", openBrowserMenu)
            // Google's sign-in page: Turbine's card answers A and Y.
            if (browserOnGoogle) {
                o.a = bind("Set a password", googleSetPassword)
                o.y = bind("Use Chrome", googleUseChrome)
                return o
            }
            // Steam's keyboard is up for a field in the page: B only ends the typing (it never goes back a page).
            if (browserTyping) {
                o.b = bind("Done typing", browserTypingDone)
                return o
            }
            // Mod Manager Download without a key, when the file's page is somewhere else: A goes there, B not now.
            if (browserNoKeyPage !== "") {
                o.a = bind("Slow download page", browserNoKeyGo)
                o.b = bind("Not now", () => { browserNoKeyPage = ""; browserNoteNow = null })
                return o
            }
            o.a = bind(Turbine.browserKeyNav && browserEnterNow ? "Press" : "Click", () => Turbine.browserClick())
            o.y = bind("Keyboard", browserKeyboard)
            o.lt = bind("", openBrowserMenu)
            o.rt = bind("", () => Turbine.browserClick())
            return o
        }
        // Where do these files go? (a full page; LB and RB switch pane).
        if (page === "place") {
            o.a = bind(placeALabel, () => placeAct("a"))
            o.b = bind(placeBLabel, () => placeAct("b"))
            o.x = bind("Options", () => placeAct("x"))
            o.y = bind("Find", () => placeAct("y"))
            // Start puts them where the ring is; shown when A says something else (a folder row: A opens it).
            o.start = bind(placeStartLabel, () => placeAct("start"))
            o.lb = bind("", () => placeAct("lb"))
            o.rb = bind("", () => placeAct("rb"))
            o.lt = bind("", () => placeAct("lt"))
            o.rt = bind("", () => placeAct("rt"))
            return o
        }
        if (zone === "rail") {
            const g = railIndex < games.length
            o.a = bind(g ? "Open" : railIndex < games.length + railExtra ? "Show hidden games" : "Settings", () => railSelect(railIndex))
            if (page !== "game" || game) o.b = bind("Back", () => { zone = page === "settings" ? "settings" : page === "perf" ? "perf" : "main" })
            if (g) o.x = bind("Options", openRailMenu)
            o.lt = bind("", () => { railIndex = Math.max(0, railIndex - 5) })
            o.rt = bind("", () => { railIndex = Math.min(railCount - 1, railIndex + 5) })
            return o
        }
        if (zone === "panel" && page === "game" && game && panelHasSettings) {
            o.a = bind(panelSettingsLabel, panelEditOpen)
            o.b = bind("Back", () => { zone = "main" })
            return o
        }
        if (zone === "play" && page === "game" && game) {
            o.a = bind("Play", playPressed)
            o.b = bind("Back", () => { zone = "main"; note("focus", "Back from Play to the list") })
            o.lb = bind("", () => switchTab(-1))
            o.rb = bind("", () => switchTab(1))
            return o
        }
        if (page === "settings") {
            const r = sec === "Look" || sec === "About" ? null : rowsNow[setRow]
            const kind = sec === "Look" ? "look" : sec === "About" ? "about" : r ? r.kind : ""
            const aLabel = kind === "look" || kind === "choice" || kind === "toggle" ? "Change" : kind === "item" ? "Choose"
                : kind === "about" ? (setRow === 0 ? "" : "Open") : kind === "action" ? (r.action || "") : ""
            if (aLabel !== "" || kind === "about") o.a = bind(aLabel, settingsA)
            o.b = bind("Back", settingsBack)
            o.lb = bind("", () => settingsSection(-1))
            o.rb = bind("", () => settingsSection(1))
            const rows = sec === "Look" ? lookRows.length : sec === "About" ? 1 : rowsNow.length
            o.lt = bind("", () => { setRow = settingsLand(Math.max(0, setRow - pageJump), -1, rows) })
            o.rt = bind("", () => { setRow = settingsLand(Math.min(rows - 1, setRow + pageJump), 1, rows) })
            return o
        }
        if (page === "perf") {
            o.b = bind("Back", () => { page = "settings"; zone = "settings" })
            o.lt = bind("", () => { perfIndex = Math.max(0, perfIndex - pageJump) })
            o.rt = bind("", () => { perfIndex = Math.min(1999, perfIndex + pageJump) })
            return o
        }
        if (!game) { o.a = railExtra > 0 ? bind("Show games", showHiddenGames) : bind("Look again", load); return o }
        if (modPage) {
            o.a = bind(pageSettings ? (isSeamless(pageMod) ? "Co-op settings" : "Edit settings") : pageALabel, () => modPageAct("a"))
            o.b = bind("Back", getBack)
            if (pageMenuItems.length > 0) o.x = bind("Options", () => { menuIndex = 0; openPick(detailsData && detailsData.name ? detailsData.name : "This mod", "", pageMenuItems, "") })
            // Y finds here too (the mod page closes onto the results).
            if (sources.length > 0) o.y = bind("Find", openFindPop)
            o.lb = bind("", () => modPageAct("lb"))
            o.rb = bind("", () => modPageAct("rb"))
            o.lt = bind("", () => modView.scroll(-4))
            o.rt = bind("", () => modView.scroll(4))
            return o
        }
        // The tab bar: LB and RB (never another game).
        o.lb = bind("", () => switchTab(-1))
        o.rb = bind("", () => switchTab(1))
        if (tab === 0) {
            if (selecting) {
                const it = focusedItem
                if (it && it.kind === "mod") o.a = bind(ticked[it.m.id] ? "Untick" : "Tick", () => selectAct("a"))
                o.b = bind("Done", () => selectAct("b"))
                if (tickedCount > 0) o.x = bind("Act on " + tickedCount, () => selectAct("x"))
                return o
            }
            if (acRefused && mods.length === 0) {
                o.b = bind("Games", () => { zone = "rail"; railIndex = current })
                return o
            }
            // No mods yet (any game): A Get mods, X Add from Downloads (the centered card says the same).
            if (mods.length === 0 && modsLoaded && !(focusedItem && (focusedItem.kind === "removed" || focusedItem.kind === "ghost"))) {
                if (canGetMods) { o.a = bind("Get mods", () => { note("action", "No mods yet: Get mods"); tab = 1 }); o.x = bind("Add from Downloads", openAdd) }
                else o.a = bind("Add from Downloads", openAdd)
                o.b = bind("Games", () => { zone = "rail"; railIndex = current })
                return o
            }
            // Y: the pop-up with Find, Show, Sort and Select several.
            if (mods.length > 0) o.y = bind("Find", openFindPop)
            o.lt = bind("", () => { modIndex = Math.max(0, modIndex - pageJump) })
            o.rt = bind("", () => { modIndex = Math.min(listItems.length - 1, modIndex + pageJump) })
            const it = focusedItem
            if (it && it.kind === "mod") {
                o.a = bind(it.m.enabled ? "Turn off" : "Turn on", () => toggle(it.m))
                o.x = bind("Options", () => { menuIndex = 0; sheet = "modmenu" })
            } else if (it && it.kind === "ghost") o.a = bind("Put back", () => putBack(it.m.id))
            else if (it && it.kind === "held") o.a = bind("Turn them back on", turnHeldBackOn)
            else if (it && it.kind === "deps") o.a = bind(depsOpen ? "Close" : "Open", () => { depsOpen = !depsOpen; note("action", "Dependencies " + (depsOpen ? "opened" : "closed")) })
            else if (it && it.kind === "needsq") {
                o.a = bind("Go on", reqGoOn)
                o.x = bind("Options", () => openPick("Requirements for " + reqQ.forName, plural(reqQ.items.length - reqQ.index, "mod left", "mods left"), [
                    { icon: "update", title: "Go on", detail: reqItem ? "Next: " + reqItem.name : "", run: () => reqGoOn() },
                    { icon: "remove", title: "Forget them", detail: "The row goes; the mod's warning stays until they are in", run: () => { reqClear(); note("action", "Requirements: forgotten") } }
                ], ""))
            }
            else if (it && it.kind === "removed") {
                o.a = bind("Open", openRemoved)
                o.x = bind("Options", openRemoved)
            }
            // The removed row went a moment ago and another row slid under the highlight: A still means Put back.
            if (ghostJustWent()) o.a = bind("Put back", () => { note("action", "A just as the removed row went: put back"); const id = ghostGone.id; ghostGone = null; putBack(id) })
            // B only ever goes back. Find, Show and Sort stay (the line over the list says
            // them); Y, then X Clear all, clears them.
            o.b = bind("Games", () => { zone = "rail"; railIndex = current })
            return o
        }
        if (tab === 1 && sources.length === 0) {
            if (nexusUnknown) { o.a = bind("Pick game", openNexusPick); o.x = bind("Add from Downloads", openAdd) }
            else o.a = bind("Add from Downloads", openAdd)
            o.b = bind("My mods", () => { tab = 0 })
            return o
        }
        if (tab === 1) {
            // B goes back (a search stays, named over the cards; Y, X clears it).
            o.b = bind("My mods", getBack)
            if (getArea === "fallback") {
                o.a = bind("Open on Nexus", openNexusMods)
                o.x = bind("Try again here", retryBrowse)
                // Y is Find here too (a search on Nexus); Add from Downloads is the card's own button
                // and the wheel's.
                if (sources.length > 0) o.y = bind("Find", openFindPop)
                return o
            }
            // Y: the pop-up with Search, Sort and Category.
            o.y = bind("Find", openFindPop)
            if (getArea === "recent") o.a = bind("Open", () => getAct("a"))
            else {
                if (getItems[getIndex]) o.a = bind("Open", () => getAct("a"))
                if (getItems[getIndex]) o.x = bind("Options", () => { menuIndex = 0; sheet = "cardmenu" })
                // A screen of rows; the rows stop at both ends.
                const jump = gridColumns * 2
                o.lt = bind("", () => { getIndex = Math.max(getIndex % gridColumns, getIndex - jump) })
                o.rt = bind("", () => { getIndex = Math.max(0, Math.min(getItems.length - 1, getIndex + jump)); if (getIndex >= getItems.length - gridColumns) browseMore() })
            }
            return o
        }
        // Load order.
        const p = orderPlugin
        if (orderIndex === -1) o.a = bind("Sort", () => orderAct("a"))
        else if (p && !p.fixed) o.a = bind(p.active ? "Turn off" : "Turn on", () => orderAct("a"))
        o.b = bind("My mods", () => { tab = 0 })
        if (p) o.x = bind("Options", openPluginMenu)
        // One name for Y: Find (here it finds a plugin by name).
        if (order.length > 0) o.y = bind("Find", openFindPlugin)
        // A page up or down, landing on a plugin that can be changed (the game's own are locked).
        o.lt = bind("", () => { orderIndex = orderLandOn(Math.max(-1, orderIndex - pageJump), -1) })
        o.rt = bind("", () => { orderIndex = orderLandOn(Math.min(order.length - 1, orderIndex + pageJump), 1) })
        return o
    }
    // The legend: A, B, X, Y as binds() has them, then one more: the pointer's buttons while it shows, else Start
    // when a sheet covers the Play button. LB/RB, View and Start are caps on the tab bar, the wheel button and Play.
    function legendFor(o) {
        let h = []
        for (const k of ["a", "b", "x", "y"]) if (o[k] && o[k].label) h.push([k, o[k].label, o[k].pre || ""])
        // R3 is A on the focused thing, except while a toast shows a next step (Undo, Watch it): then R3 does
        // that step, and the legend says so, right after A and B, for as long as the toast is up.
        if (toastActionLive()) h.splice(Math.min(2, h.length), 0, ["r3", toast.actionLabel, ""])
        // The Nexus page: A already says Click, so the one more is Start Play after an add, otherwise how to move the
        // pointer (the right stick). X Options covers what LT opens too.
        if (page === "browser" && sheet === "" && !wheelOpen) {
            if (o.start && o.start.label) h.push(["start", o.start.label, o.start.pre || ""])
            else if (browserTab === 0 && !browserTyping && !keysLegend) h.push(["rs", "Pointer", ""])
            return h.slice(0, 5)
        }
        // LT is the right button (options) only where X has options: elsewhere the legend says RT Click alone.
        if (Turbine.pointerShown && !keyboardUp) h.push(o.x && o.x.label ? ["rt", "Click", "", "lt", "Menu"] : ["rt", "Click", ""])
        else if (o.start && o.start.label) h.push(["start", o.start.label, o.start.pre || ""])
        return h.slice(0, 5)
    }
    // The first press after a page opens never changes a setting (the owner's Left on the way into Settings, a
    // press carried over from the quick wheel, set the Nexus page to 140%). Left or Right changes a value only after
    // the focus has been moved (Up, Down, a tap) or something chosen since the page or section opened.
    property bool settingsFresh: false
    function navigate(b) {
        // The controls audit's blocker: the D-pad on "Where do these files go?" moves its own folders, never the
        // list hidden behind the page.
        if (page === "place") { placeAct(b); return }
        if (page === "browser") {
            if (browserTab === 1) {
                if (b === "up") browserDlIndex = Math.max(0, browserDlIndex - 1)
                else if (b === "down") browserDlIndex = Math.min(Math.max(0, webDls.length - 1), browserDlIndex + 1)
                return
            }
            // Arrow keys into the page: its own focus ring moves between links and buttons, or it scrolls.
            if (["up", "down", "left", "right"].indexOf(b) >= 0) Turbine.browserKey(b)
            return
        }
        if (zone === "rail") {
            if (b === "up") railIndex = Math.max(0, railIndex - 1)
            else if (b === "down") railIndex = Math.min(railCount - 1, railIndex + 1)
            else if (b === "right") zone = page === "settings" ? "settings" : page === "perf" ? "perf" : "main"
            return
        }
        if (page === "settings") {
            const rows = sec === "Look" ? lookRows.length : sec === "About" ? 1 : rowsNow.length
            const r = sec === "Look" || sec === "About" ? null : rowsNow[setRow]
            const kind = r ? r.kind : ""
            if (b === "up" || b === "down") { settingsFresh = false; settingsMove(b === "up" ? -1 : 1, rows) }
            else if (settingsFresh && (b === "left" || b === "right")) {
                settingsFresh = false
                note("action", "Settings: " + b + " just after opening, nothing changed")
                toast.info("Left and Right change the highlighted setting. Press again to change it.")
            }
            // Left and Right change values; Left never leaves Settings (B does).
            else if (sec === "Look" && (b === "left" || b === "right")) lookRows[setRow].step(b === "left" ? -1 : 1)
            else if (kind === "item" && b === "left") labsChip = Math.max(0, labsChip - 1)
            else if (kind === "item" && b === "right") labsChip = Math.min(2, labsChip + 1)
            else if ((kind === "choice" || kind === "toggle") && (b === "left" || b === "right")) rowStep(r, b === "left" ? -1 : 1)
            return
        }
        if (page === "perf") {
            if (b === "up") perfIndex = Math.max(0, perfIndex - 1)
            else if (b === "down") perfIndex = Math.min(1999, perfIndex + 1)
            else if (b === "left") zone = "rail"
            return
        }
        if (!game) { if (b === "left") { zone = "rail"; railIndex = 0 } return }
        if (zone === "panel") {
            if (b === "left") zone = "main"
            else if (b === "up" || b === "down") { zone = "main"; navigate(b) }
            return
        }
        if (zone === "play") {
            if (b === "up" || b === "down") zone = "main"
            else if (b === "left") { zone = "rail"; railIndex = current }
            return
        }
        if (modPage) { modPageAct(b); return }
        // Play is in the bottom bar (Start); Up at the top stays there and brings the bars back.
        if (tab === 1 && sources.length > 0) { getAct(b); return }
        if (tab === 2) {
            if (b === "up" && orderIndex === -1) { barsHidden = false; return }
            orderAct(b); return
        }
        if (tab === 1 || listItems.length === 0) {
            // Get mods with nowhere to get them: still a page of the book.
            if (tab === 1) { if (b === "left") turnPage(-1); else if (b === "right") turnPage(1) }
            else if (b === "left") { zone = "rail"; railIndex = current }
            return
        }
        if (selecting) { if (b === "left") { selecting = false; ticked = {}; zone = "rail"; railIndex = current } else selectAct(b); return }
        if (modIndex < 0) modIndex = 0
        if (b === "up") { if (modIndex === 0) barsHidden = false; modIndex = Math.max(0, modIndex - 1) }
        else if (b === "down") modIndex = Math.min(listItems.length - 1, modIndex + 1)
        else if (b === "left") { zone = "rail"; railIndex = current }
        // Right: the panel's Edit settings, when the highlighted mod has settings.
        else if (b === "right" && panelHasSettings) zone = "panel"
    }
    function switchTab(d) {
        const t = Math.max(0, Math.min(tabs.length - 1, tab + d))
        if (t === tab) return
        if (zone !== "main") zone = "main"
        tab = t
    }
    function settingsSection(d) {
        const s = Math.max(0, Math.min(settingsSections.length - 1, setSection + d))
        if (s !== setSection) { setSection = s; setRow = firstSettingsRow() }
    }
    // Rows with only words (kind "info") are never focused: the D-pad passes them, and the list still scrolls them
    // into view.
    function settingsFocusable(i) {
        if (sec === "Look" || sec === "About") return true
        const r = rowsNow[i]
        return !!r && r.kind !== "info"
    }
    function firstSettingsRow() {
        if (sec === "Look" || sec === "About") return 0
        const i = rowsNow.findIndex(r => r.kind !== "info")
        return Math.max(0, i)
    }
    // The nearest focusable row from i, looking in direction d first.
    function settingsLand(i, d, rows) {
        for (let j = i; j >= 0 && j < rows; j += d) if (settingsFocusable(j)) return j
        for (let j = i; j >= 0 && j < rows; j -= d) if (settingsFocusable(j)) return j
        return setRow
    }
    function settingsMove(d, rows) {
        let i = setRow + d
        while (i >= 0 && i < rows && !settingsFocusable(i)) i += d
        // The words passed over (or above the first row, or below the last) come into view.
        const labsList = settingsPage.labsList
        if (labsList.visible) {
            if (i < 0) labsList.positionViewAtBeginning()
            else if (i >= rows) labsList.positionViewAtEnd()
            else for (let j = setRow + d; j !== i; j += d) labsList.positionViewAtIndex(j, ListView.Contain)
        }
        if (i >= 0 && i < rows) setRow = i
    }
    function settingsA() {
        const r = sec === "Look" || sec === "About" ? null : rowsNow[setRow]
        const kind = r ? r.kind : ""
        if (sec === "Look") { const lr = lookRows[setRow]; if (lr.a) lr.a(); else lr.step(1) }
        else if (sec === "About") labsTap()
        else if (kind === "item") labsChoose(r, ["works", "problem", ""][labsChip])
        else if (kind === "choice" || kind === "toggle") rowStep(r, 1)
        else if (kind === "action") rowRun(r)
    }
    // B from Settings: the screen and row it was opened from.
    property var settingsFrom: null
    function settingsBack() {
        const f = settingsFrom
        settingsFrom = null
        // Opened from the Nexus page (its quick wheel): back to the page.
        if (f && f.browser && browserOpen) { page = "browser"; note("focus", "Back from Settings to the Nexus page"); return }
        page = "game"
        if (f) { tab = f.tab; zone = f.zone === "settings" ? "main" : f.zone; if (f.zone === "rail") railIndex = f.railIndex; modPage = f.modPage; note("focus", "Back from Settings to " + f.zone + ", tab " + f.tab) }
        else zone = "main"
    }
    // The rail's quick menu (X): Play options, Loadouts, Pin, Hide from rail.
    function openRailMenu() {
        const g = games[railIndex]
        if (!g) return
        const pinned = pinnedIds.indexOf(g.id) >= 0
        note("action", "Rail options: " + g.name)
        openPick(g.name, g.launcher || "", [
            { icon: "play", title: "Play options", detail: "With or without mods, or another loadout", run: () => { current = railIndex; page = "game"; zone = "main"; Qt.callLater(openPlayOptions) } },
            { icon: "profile", title: "Loadouts", detail: "Save setups by name, switch, share", run: () => { current = railIndex; page = "game"; zone = "main"; Qt.callLater(openProfiles) } },
            { icon: "move", title: pinned ? "Unpin" : "Pin to the top", detail: pinned ? "Back in last played order" : "Stays at the top of the rail", run: () => setRailList("pinned", g.id, !pinned) },
            { icon: "info", title: "Hide from the rail", detail: "For tools that aren't games. You can show it again.", run: () => { setRailList("hidden", g.id, true); toast.act("done", fullGame(g.name) + " is hidden from the rail.", "Undo", () => setRailList("hidden", g.id, false)) } }
        ].concat(g.support && g.support !== "full" ? [
            { icon: "update", title: "Nexus game", detail: "Which game on Nexus Mods this is", run: () => { current = railIndex; page = "game"; zone = "main"; nexusFound = null; Qt.callLater(openNexusPick) } }
        ] : []), "")
    }
    // Load order's quick menu (X): Move, Details, Go to its mod.
    function openPluginMenu() {
        const p = orderPlugin
        if (!p) return
        openPick(p.name, p.fixed ? "Always loaded by the game" : (p.active ? "On" : "Off"), (p.fixed ? [] : [
            { icon: "move", title: "Move", detail: "D-pad moves it, A puts it down", run: () => { orderMoving = true; orderFrom = orderIndex; orderSaved = order.slice() } }
        ]).concat([
            { icon: "info", title: "Details", detail: "What it needs, and the community list's notes", run: () => { note("action", "Plugin details: " + p.name); sheet = "plugin" } }
        ]).concat(pluginOwner ? [
            { icon: "info", title: "Go to its mod", detail: modName(pluginOwner), run: () => goToMod(pluginOwner) }
        ] : []), "")
    }
    function openFindPlugin() {
        openAsk("findplugin", "Find a plugin", "Jumps to the first plugin whose name has these words.", "", "Part of a plugin's name", 60, false, "Find")
    }
    function findPlugin(text) {
        const words = text.toLowerCase().split(/\s+/).filter(w => w !== "")
        const i = order.findIndex(p => words.every(w => p.name.toLowerCase().indexOf(w) >= 0))
        if (i >= 0) { orderIndex = i; note("action", "Find plugin \"" + text + "\": row " + (i + 1)) }
        else toast.info("No plugin has \"" + text + "\" in its name.")
    }
    // The rail's label: the game's name, and where it comes from when two copies share a name (Steam and Heroic).
    // Short names on the rail, so long ones aren't cut to "Kingdom Come: D..." twice.
    readonly property var shortNames: ({ "The Elder Scrolls V: Skyrim Special Edition": "Skyrim SE", "Skyrim Special Edition": "Skyrim SE",
        "Kingdom Come: Deliverance": "KCD", "Kingdom Come: Deliverance II": "KCD II", "The Witcher 3: Wild Hunt": "Witcher 3",
        "The Witcher 3": "Witcher 3", "Cyberpunk 2077": "Cyberpunk", "Risk of Rain 2": "Risk of Rain 2", "Stardew Valley": "Stardew Valley" })
    // Words on a filled button: near-black or white, whichever reads better on that fill (any accent).
    function onFill(bg) {
        const f = v => v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4)
        const l = 0.2126 * f(bg.r) + 0.7152 * f(bg.g) + 0.0722 * f(bg.b)
        return (l + 0.05) / 0.05 >= 1.05 / (l + 0.05) ? "#0B0A08" : "#FFFFFF"
    }
    function shortGame(name) {
        if (shortNames[name]) return shortNames[name]
        return titleCase(String(name || "").replace(/^The Elder Scrolls [IVX]+: /, "").replace(/: .*$/, ""))
    }
    // A game's name in titles and sentences (COMPONENTS.md, Words): the name its rules use when they give one
    // (gameTitle for the game on screen), else Steam's name in title case.
    readonly property var fullNames: ({ "The Elder Scrolls V: Skyrim Special Edition": "Skyrim Special Edition", "The Witcher 3: Wild Hunt": "The Witcher 3",
        "ELDEN RING": "Elden Ring", "ELDEN RING NIGHTREIGN": "Nightreign", "ARMORED CORE VI FIRES OF RUBICON": "Armored Core VI", "DARK SOULS III": "Dark Souls III",
        "Sekiro: Shadows Die Twice": "Sekiro", "Halo: The Master Chief Collection": "Halo MCC" })
    function fullGame(name) {
        if (game && name === game.name) return gameTitle
        return fullNames[name] || titleCase(String(name || ""))
    }
    // Steam's all-capitals names ("ELDEN RING") read as "Elden Ring"; Roman numerals stay ("ARMORED CORE VI").
    function titleCase(n) {
        if (!/[A-Z]{3}/.test(n) || n !== n.toUpperCase()) return n
        return n.split(" ").map(w => /^[IVX]+$/.test(w) ? w : w.charAt(0) + w.substring(1).toLowerCase()).join(" ")
    }
    function railName(i) {
        const g = games[i]
        if (!g) return ""
        const twins = games.filter(x => x.name === g.name).length > 1
        return titleCase(g.name) + (twins ? " · " + (g.launcher === "Steam" ? "Steam" : (g.storeLabel || g.launcher) + " (Heroic)") : "")
    }
    function railSelect(i) {
        note("action", "Rail " + (i < games.length ? games[i].name : "Settings"))
        if (i < games.length) { current = i; page = "game"; zone = "main"; if (!shot.on) seen.railHintDone = true }
        else if (i < games.length + railExtra) showHiddenGames()
        else openSettings()
    }
    // ---------------------------------------------------------------- the quick wheel (View)
    // Eight fixed slots, the same places on every screen; a slot with nothing to do here is dimmed, never moved.
    // Tap View: it stays open, the D-pad picks (Up then Right is Up-right), A does it, B closes. Or hold View, push a
    // direction and let go of View. The middle is neutral: it closes.
    property bool wheelOpen: false
    // ---- One input layer. In Qt 6 a MouseArea that takes a press stops the items under it but not their
    // pointer handlers, so an overlay's backdrop alone can't stop a tap reaching a card beneath. Everything under
    // the top layer is turned off instead (a disabled item and its handlers never see a press): 0 the page (main
    // row and the Nexus page), 2 a sheet, menu or prompt, 3 the quick wheel, 4 the tutorial. The bottom bar stays
    // on up to the wheel (its Close is in the bar).
    readonly property int topLayer: sheet === "tutorial" ? 4 : wheelOpen ? 3
                                    : (sheet !== "" || askView.shown || perfView.visible || reportView.visible) ? 2 : 0
    // A press that landed on something drawn over the layers without turning them off (the bar over a sheet, a
    // toast over anything): the handlers under it ignore that press. Cleared once the press is done.
    property int pressCover: -1
    function coverPress(layer) { pressCover = layer; coverClear.restart() }
    // After the release has reached every handler (passive ones get it after the backdrop that took the press).
    function coverEnd() { Qt.callLater(() => { app.pressCover = -1 }) }
    Timer { id: coverClear; interval: 8000; onTriggered: app.pressCover = -1 }
    // Every tap wired from the page, the bar or a sheet asks first. In normal use it never refuses (the layers are
    // off); when it does, the build's checks fail on the LEAK line.
    function gate(layer, name) {
        if (topLayer > layer) { note("touch", "LEAK " + name + " (layer " + layer + " under " + topLayer + ")"); return false }
        if (pressCover > layer) { note("touch", "Covered: " + name + " under a press on layer " + pressCover); return false }
        return true
    }
    property int wheelSlot: -1            // 0 Up, 1 Up-right, 2 Right, 3 Down-right, 4 Down, 5 Down-left, 6 Left, 7 Up-left
    property bool wheelHeld: false
    property bool wheelPicked: false
    // While it is open the left stick is only a D-pad (no scrolling), so Up then on round to Right reaches Up-right.
    onWheelOpenChanged: Turbine.stickDpad = wheelOpen
    readonly property string wheelKind: page === "browser" ? "browser" : !game || page !== "game" ? "none" : tab === 1 || modPage ? "get" : tab === 2 ? "order" : "mods"
    // Each wedge: its name, whether it does something here, what it does (the middle of the wheel says it), and
    // the action. A wedge is never empty: when it can't be used here it keeps its name, dimmed.
    function slot(label, ok, run, desc) { return { label: label, ok: ok, run: run, desc: desc || "" } }
    readonly property var wheelSlots: {
        const g = !!game
        const k = wheelKind
        // The Nexus page: Up Search Nexus, Up-right Zoom in, Right Forward, Down-right Reload, Down Game menu (the
        // game menu, as everywhere: Activity log and Help included), Down-left Settings, Left Downloads, Up-left
        // Zoom out.
        if (k === "browser") return [slot("Search Nexus", true, browserSearch, "Find a mod on Nexus"), slot("Zoom in", true, () => browserZoomBy(0.1), "Bigger page"),
                                     slot("Forward", !!nexusView && nexusView.canGoForward, () => nexusView.forward(), "The page you came back from"), slot("Reload", !!nexusView, () => nexusView.reload(), "Load this page again"),
                                     slot("Game menu", g, () => { menuIndex = 0; sheet = "gamemenu" }, "Everything for this game"), slot("Settings", true, openSettings, "Look, controls and accounts"),
                                     slot("Downloads", true, () => { browserTab = 1 }, "What Nexus is sending"), slot("Zoom out", true, () => browserZoomBy(-0.1), "Smaller page")]
        const common = [slot("Play options", g, openPlayOptions, "With or without mods, or another loadout"), slot("Game menu", g, () => { menuIndex = 0; sheet = "gamemenu" }, "Everything for this game"),
                        slot("Settings", true, openSettings, "Look, controls and accounts"), slot("Games", games.length > 0, openGamesPick, "Every game, last played first")]
        // Up is the Y pop-up (Find, Show, Sort; Search, Sort, Category), Right Add from Downloads and Up-left
        // Loadouts on every tab; Up-right is the tab's own.
        const tail = [slot("Add from Downloads", g, openAdd, "A mod file you downloaded")].concat(common).concat([slot("Loadouts", g, openProfiles, "Save or switch sets of mods")])
        if (k === "get") return [slot("Find", sources.length > 0, openFindPop, "Search, sort and category"),
                                 slot("Recently viewed", recentViewed.length > 0, openRecentPick, recentViewed.length > 0 ? plural(recentViewed.length, "mod page", "mod pages") + " you opened" : "Mod pages you open show here")].concat(tail)
        if (k === "order") return [slot("Find", order.length > 0, openFindPlugin, "Find a plugin by name"), slot("Sort", order.length > 0, () => { orderIndex = -1; Turbine.orderSort() }, "Put plugins in a safe order")].concat(tail)
        return [slot("Find", k === "mods" && mods.length > 0, openFindPop, "Find, show, sort, select several"),
                slot(updateList.length > 0 ? "Updates (" + updateList.length + ")" : "Updates", k === "mods" && updateList.length > 0, openUpdates, updateList.length > 0 ? "Mods with a newer version" : "No updates ready")].concat(tail)
    }
    // The wheel's Recently viewed: the last mod pages opened in this game.
    function openRecentPick() {
        openPick("Recently viewed", gameTitle, recentViewed.map(r => ({ icon: "info", title: cleanName(r.name), detail: "", aLabel: "Open",
            run: () => { tab = 1; zone = "main"; openDetails(r) } })), "")
    }
    // The middle is neutral. The stick at rest picks nothing, and the middle says Close: A, B, View, a tap on
    // the middle, or letting go of a held View with nothing picked all close it. A short tap of View keeps it open.
    property bool wheelByStick: false     // the wedge was picked by the stick's angle (it goes when the stick rests)
    property real wheelPressedAt: 0
    readonly property string wheelHint: lastInput === "touch" ? "Tap one to choose" : lastInput === "mouse" ? "Click one to choose" : lastInput === "keys" ? "W A S D or the arrows choose" : "Push the stick to choose"
    function wheelPress() {
        if (wheelOpen) { closeWheel(); return }
        // View brings the bars back too.
        barsHidden = false
        wheelOpen = true; wheelSlot = -1; wheelHeld = true; wheelPicked = false; wheelByStick = false; wheelPressedAt = Date.now()
        note("action", "Quick wheel open (" + wheelKind + "): " + wheelSlots.map(x => x.label + (x.ok ? "" : " (dim)")).join("; "))
    }
    function wheelRelease() {
        if (!wheelHeld) return
        wheelHeld = false
        // Held, pushed, let go: that slot.
        if (wheelOpen && wheelPicked && wheelSlot >= 0) wheelFire(wheelSlot)
        // Held and let go with nothing picked: it closes. (A short tap of View leaves it open.)
        else if (wheelOpen && wheelSlot < 0 && Date.now() - wheelPressedAt >= 350) { note("action", "Quick wheel: View let go with nothing picked"); closeWheel() }
    }
    function closeWheel() { if (wheelOpen) { wheelOpen = false; wheelHeld = false; note("action", "Quick wheel closed") } }
    // The left stick picks a wedge by its angle while the wheel is open (0 is Up, clockwise; -1 centred).
    Connections {
        target: Turbine
        function onWheelAngleChanged() {
            if (!app.wheelOpen) return
            // The stick came back to rest: the wedge it picked lets go (one picked with the D-pad stays).
            if (Turbine.wheelAngle < 0) { if (app.wheelByStick) { app.wheelByStick = false; app.wheelSlot = -1; app.wheelPicked = false } return }
            app.wheelByStick = true
            const a = Turbine.wheelAngle, cur = app.wheelSlot
            // A little give at the edges: the wedge picked stays until the stick is 6 degrees past its border, so a
            // stick resting on a border doesn't flicker between two.
            const off = cur >= 0 ? Math.abs(((a - cur * 45) % 360 + 540) % 360 - 180) : 999
            const i = off <= 22.5 + 6 ? cur : Math.round(a / 45) % 8
            if (i !== app.wheelSlot) app.wheelSlot = i
            app.wheelPicked = true
        }
    }
    function wheelMove(b) {
        const card = { up: 0, right: 2, down: 4, left: 6 }[b]
        if (card === undefined) return
        const now = wheelSlot
        if (now < 0 || now % 2 === 1 || now === card || (now + 4) % 8 === card) wheelSlot = card
        else wheelSlot = (now === 0 && card === 6) || (now === 6 && card === 0) ? 7 : (now + card) / 2
        wheelPicked = true
        // The stick sends D-pad presses too while the wheel is open: only a real D-pad press keeps its wedge.
        wheelByStick = Turbine.wheelAngle >= 0
    }
    function wheelFire(i) {
        const s = wheelSlots[i]
        if (!s || !s.ok) { note("action", "Quick wheel: " + (s ? s.label : i) + " is dimmed here"); if (s) toast.info(s.label + " isn't available here."); return }
        closeWheel()
        note("action", "Quick wheel: " + s.label)
        s.run()
    }
    function wheelAct(b) {
        if (["up", "down", "left", "right"].indexOf(b) >= 0) wheelMove(b)
        else if (b === "a") { if (wheelSlot >= 0) wheelFire(wheelSlot); else { note("action", "Quick wheel: A with nothing picked"); closeWheel() } }
        else if (b === "b" || b === "view") closeWheel()
        else if (b === "start") closeWheel()
    }
    // The wheel's Games: every game, last played first.
    function openGamesPick() {
        openPick("Games", plural(games.length, "game", "games"), games.map((g, i) => ({
            icon: i === current ? "power" : "play", title: railName(i), detail: i === current ? "On screen now" : "",
            run: () => { railIndex = i; railSelect(i) } })), "")
        menuIndex = current
    }

    // Counted for the build's checks (SelfCheck.qml).
    property int launchCount: 0
    property string lastDry: ""
    // Presses and releases that reached the interface from the controller (the input check counts them).
    property int padEvents: 0
    property bool probeOn: false

    // ---------------------------------------------------------------- the beginner tutorial (first run)
    // Four cards with the real buttons drawn large: A and B, X and Y; View and the quick wheel; getting a mod; Play.
    // A or Right: next card; Left: back one; B or Start: skip. It ends on the rail ("Pick a game to start"), runs
    // once (remembered) and Settings, Help shows it again.
    readonly property int tutCount: 5
    property int tutCard: 0
    function openTutorial(n) {
        tutCard = n || 0
        tutorialFrom = page
        // Remembered as soon as it shows, so a sheet or a quit part way doesn't bring it back at the next start.
        if (!shot.on) seen.tutorialDone = true
        note("action", "Beginner tutorial, card " + (tutCard + 1))
        sheet = "tutorial"
    }
    function tutorialAct(b) {
        if (b === "a" || b === "right") {
            if (tutCard < tutCount - 1) { tutCard++; note("action", "Tutorial card " + (tutCard + 1)) }
            else tutorialDone("finished")
        }
        else if (b === "left") tutCard = Math.max(0, tutCard - 1)
        else if (b === "b") tutorialDone("skipped")
        // Start finishes it: straight to picking a game.
        else if (b === "start") tutorialDone("finished with Start")
    }
    property string tutorialFrom: ""
    function tutorialDone(how) {
        if (!shot.on) seen.tutorialDone = true
        sheet = ""
        // Shown again from Settings, Help: back to Settings.
        if (tutorialFrom === "settings" && page === "settings") { tutorialFrom = ""; note("action", "Tutorial " + how + ": back to Settings"); return }
        tutorialFrom = ""
        note("action", "Tutorial " + how + ": pick a game")
        page = "game"
        zone = "rail"
        railIndex = Math.min(current, Math.max(0, games.length - 1))
    }

    // ---------------------------------------------------------------- Get mods
    // Where this game's mods come from: Thunderstore for its games, Nexus Mods for the rest (no key needed).
    readonly property var sources: !rules ? [] : [].concat(rules.community ? ["thunderstore"] : []).concat(rules.domain ? ["nexus"] : [])
    property string getSource: ""
    readonly property string srcNow: sources.indexOf(getSource) >= 0 ? getSource : (sources[0] || "")
    function srcName(s) { return s === "nexus" ? "Nexus Mods" : s === "thunderstore" ? "Thunderstore" : "" }
    property string getText: ""
    property string getCategory: ""
    property int getSortIndex: 0
    readonly property var sortKeys: srcNow === "nexus" ? ["trending", "endorsements", "downloads", "updated", "newest"] : ["downloads", "rating", "updated", "newest"]
    property var browseData: null
    property bool browseLoading: false
    property bool loadingMore: false
    property bool noMore: false
    property string getArea: "grid"       // recent (Good first mods), grid, fallback
    property int getIndex: 0
    // The card B left from (Down from the bar goes back to it); -1 when there is none.
    // Nexus browsing without a key: "" until tried this session, "ok", or "failed" (Open on Nexus instead).
    property string nexusPublic: ""
    readonly property bool nexusFallback: srcNow === "nexus" && (nexusPublic === "failed" || (browseData && browseData.fallback === true))
    // Categories come with every Nexus answer; kept from the last one without a category, so the list stays whole
    // while one is picked.
    property var catList: []
    readonly property var getCats: catList.length > 0 ? [""].concat(catList) : []
    // The Nexus mod each installed mod came from ("<domain>:<mod>[:<file>]"), so cards say Installed or Update ready
    // even for mods added after the grid was read.
    function nexusModOf(m) { if (!m || m.sourceKind !== "nexus") return ""; const p = String(m.sourceId || "").split(":"); return p.length >= 2 ? p[1] : p[0] }
    readonly property var installedNexus: { let o = {}; for (const m of mods) { const k = nexusModOf(m); if (k !== "") o[k] = m.id } return o }
    readonly property var getItems: (browseData && browseData.items ? browseData.items : []).map(x => {
        const mine = srcNow === "nexus" ? installedNexus[String(x.id)] : undefined
        if (mine !== undefined) {
            const mm = mods.find(y => y.id === mine)
            return Object.assign({}, x, { installed: true, on: mm ? mm.enabled === true : undefined, update: !!updateMap[mine] })
        }
        return !x.installed ? x : Object.assign({}, x, { installed: true })
    })
    // Updates for your mods, on the Get mods start page.
    readonly property var updateList: Object.keys(updateMap).map(k => ({ u: updateMap[k], m: mods.find(x => x.id === k) })).filter(x => !!x.m)
    // Good first mods (from the game's data file) on the Get mods start page, for this source.
    readonly property var starterItems: getSummary === "" && rules && rules.starters ? rules.starters.filter(x => x.source === srcNow)
        .map(x => ({ id: x.id, name: x.name, icon: "", source: x.source, what: x.what })) : []
    // Recently viewed: the last 8 mod pages opened in this game.
    property var recentSeed: null
    readonly property var recentViewed: recentSeed ? recentSeed : (game ? (parse(getSet.recent, {})[game.id] || []).filter(x => x.source === srcNow) : [])
    function rememberViewed(item, d) {
        if (!game || shot.on) return
        let all = parse(getSet.recent, {})
        let list = (all[game.id] || []).filter(x => !(x.id === String(item.id) && x.source === srcNow))
        list.unshift({ id: String(item.id), name: d.name || item.name, icon: item.icon || "", source: srcNow })
        all[game.id] = list.slice(0, 8)
        getSet.recent = JSON.stringify(all)
    }
    // Entering a game with no mods lands on Get mods (once per visit).
    property string landedOn: ""
    property bool landAgain: false
    function maybeLandOnGet() {
        // Needs the game's rules too (where its mods come from): called again when they arrive.
        if (!game || !modsLoaded || !rules || page !== "game") return
        if (landedOn === game.id && !(landAgain && mods.length === 0)) return
        landAgain = false
        landedOn = game.id
        if (mods.length === 0 && sources.length > 0 && tab === 0 && !basicIntro && shot.page !== "emptygamemods" && shot.page.indexOf("mods") !== 0 && shot.page !== "removed") {
            // Straight onto the first mod: A opens its page.
            if (!restoreFocus) restoreFocus = { area: "grid", index: 0 }
            note("focus", "No mods yet: Get mods, first card")
            tab = 1
        }
    }
    // Get mods history: B goes back exactly one step and puts everything back as it was.
    property var history: []
    property var restoreFocus: null
    function pushResults() {
        history = history.concat([{ kind: "results", text: getText, sort: getSortIndex, category: getCategory, source: srcNow, area: getArea, index: getIndex }])
    }
    function getBack() {
        if (sheet !== "") return
        if (modPage) {
            // Other files' step is never a place to go back to.
            const h = history.filter(x => x.kind !== "files")
            const top = h.pop()
            history = h
            if (h.length > 0 && h[h.length - 1].kind === "mymods") {
                const mm = h[h.length - 1]
                history = []; modPage = false; tab = 0; zone = "main"; modIndex = mm.index
                note("action", "Back to My mods")
                return
            }
            if (top && top.kind === "mod" && h.length > 0 && h[h.length - 1].kind === "mod") {
                // Back to the mod page this requirement page came from.
                const prev = h[h.length - 1]
                history = h.slice(0, -1)
                openDetails(prev.item, true)
                return
            }
            modPage = false
            const res = h.length > 0 && h[h.length - 1].kind === "results" ? h[h.length - 1] : null
            if (res) {
                history = h.slice(0, -1)
                const same = res.text === getText && res.sort === getSortIndex && res.category === getCategory && res.source === srcNow && browseData
                getText = res.text; getSortIndex = res.sort; getCategory = res.category; getSource = res.source
                if (same) { getArea = res.area; getIndex = res.index }
                else { restoreFocus = { area: res.area, index: res.index }; browseNow() }
            }
            note("action", "Back to the results")
            return
        }
        // B goes to My mods; a search stays as it is (the line over the cards names it).
        tab = 0
    }
    function browseNow() {
        if (!game || !rules || srcNow === "") return
        if (srcNow === "nexus" && nexusPublic === "failed" && !account.hasKey) {
            browseLoading = false
            browseData = { game: game.id, source: "nexus", items: [], categories: [], fallback: true }
            getArea = "fallback"; getIndex = 0
            return
        }
        browseLoading = true
        noMore = false
        Turbine.browse(getText, getCategory, sortKeys[getSortIndex] || "", srcNow)
    }
    function browseMore() {
        if (loadingMore || noMore || !browseData || srcNow !== "nexus" || getItems.length < 30) return
        loadingMore = true
        note("action", "Get mods: the next 30 from " + getItems.length)
        Turbine.browsePage(getText, getCategory, sortKeys[getSortIndex] || "", srcNow, getItems.length)
    }
    // Try again here: browsing without a key once more.
    function retryBrowse() {
        note("action", "Try again here")
        nexusPublic = ""
        browseData = null
        getArea = "grid"; getIndex = 0
        browseNow()
    }
    function getAct(b) {
        const cols = gridColumns
        if (b === "b") { getBack(); return }
        if (getArea === "fallback") {
            if (b === "a") openNexusMods()
            else if (b === "x") retryBrowse()
            else if (b === "add") openAdd()
            else if (b === "left") turnPage(-1)
            else if (b === "right") turnPage(1)
            return
        }
        if (getArea !== "recent" && getArea !== "grid") { getArea = "grid"; getIndex = 0 }
        if (getArea === "recent") {
            if (b === "left") { if (getIndex > 0) getIndex--; else turnPage(-1) }
            else if (b === "right") { if (getIndex < starterItems.length - 1) getIndex++; else turnPage(1) }
            else if (b === "up") barsHidden = false
            // Down keeps the column (the strip's 4th card goes to the grid's 4th).
            else if (b === "down" && getItems.length > 0) { getArea = "grid"; getIndex = Math.min(getItems.length - 1, getIndex % cols) }
            else if (b === "a" && starterItems[getIndex]) openDetails(starterItems[getIndex])
        } else {
            if (b === "left") { if (getIndex % cols > 0) getIndex--; else turnPage(-1) }
            // The last column (or the last card): Right turns the page forward.
            else if (b === "right") { if (getIndex % cols < cols - 1 && getIndex < getItems.length - 1) getIndex++; else turnPage(1) }
            else if (b === "up") { if (getIndex >= cols) getIndex -= cols; else if (starterItems.length > 0) { getArea = "recent"; getIndex = Math.min(starterItems.length - 1, getIndex) } else barsHidden = false }
            else if (b === "down") getIndex = Math.max(0, Math.min(getItems.length - 1, getIndex + cols))
            else if (b === "a" && getItems[getIndex]) openDetails(getItems[getIndex])
            else if (b === "x" && getItems[getIndex]) { menuIndex = 0; sheet = "cardmenu" }
            // Infinite scroll: the next 30 when the focus reaches the last row.
            if (getIndex >= getItems.length - cols) browseMore()
        }
    }
    function openUpdates() {
        openPick("Updates for your mods", plural(updateList.length, "mod has", "mods have") + " a newer version", updateList.map(x => ({
            icon: "update", title: x.m.name + " " + x.u.to, detail: (x.u.from ? "You have " + x.u.from + ". " : "") + "Keeps its place, and stays on or off",
            run: () => updateOne(x.m) })), "")
    }
    // Update one mod: Premium and Thunderstore download it; a free Nexus account opens the new file's page, and that
    // download takes the old one's place.
    function updateOne(m) {
        const u = updateMap[m.id]
        if (!u) return
        if (u.kind === "nexus" && account.premium !== true) {
            if (!u.page) return
            note("action", "Update " + m.name + " on Nexus")
            Turbine.expectUpdate(m.id)
            openNexus(u.page)
            return
        }
        Turbine.updateMod(m.id); edited()
    }
    // X on a card: the mod page's own words (Get it on Nexus for a free account, else Install), Other files, Open on Nexus.
    readonly property var cardItem: getArea === "grid" ? getItems[getIndex] : null
    // A card you already have offers what you can do with it, not getting it again.
    // Nexus cards by the Nexus mod id; Thunderstore cards by the package ("Owner-Name", with or without a version).
    readonly property var cardMine: !cardItem || !cardItem.installed ? null
        : srcNow === "nexus" ? (installedNexus[String(cardItem.id)] !== undefined ? mods.find(m => m.id === installedNexus[String(cardItem.id)]) || null : null)
        : mods.find(m => m.sourceKind === srcNow && (String(m.sourceId) === String(cardItem.id) || String(m.sourceId || "").split(":")[0] === String(cardItem.id))) || null
    readonly property var cardMenuItems: !cardItem ? [] : cardMine ? [
        { icon: "info", title: "Show in My mods", detail: cardMine.name, act: "show" },
        { icon: "power", title: cardMine.enabled ? "Turn off" : "Turn on", detail: cardMine.enabled ? "On now" : "Off now", act: "toggle" }
    ].concat(updateMap[cardMine.id] ? [{ icon: "update", title: "Update to " + updateMap[cardMine.id].to, detail: "Keeps its place, and stays on or off", act: "update" }] : [])
     // The mod's own actions, the same as on its row in My mods.
     .concat(settingsFilesOf(cardMine.id).length > 0 ? [{ icon: "info", title: isSeamless(cardMine) ? "Co-op settings" : "Edit settings", detail: "Its settings file", act: "m:settings" }] : [])
     .concat(ordered && mods.length > 1 ? [{ icon: "move", title: "Move", detail: "In My mods: Up and Down move it, A puts it down", act: "m:move" }] : [])
     .concat([{ icon: "info", title: "Rename", detail: cardMine.renamed ? "Now " + cardMine.name : "A name of your own for the list", act: "m:rename" }])
     .concat([srcNow === "nexus" ? { icon: "play", title: "Its Nexus page", detail: "Opens it in Turbine", act: "nexus" }
                                 : { icon: "play", title: "Its Thunderstore page", detail: "Opens it in Turbine", act: "page" },
              { icon: "remove", title: "Uninstall", detail: "You can put it back for 7 days.", act: "remove", danger: true }]) : [
        { icon: "update", title: srcNow === "nexus" && account.premium !== true ? "Get it on Nexus" : "Install", detail: getWords(), act: "get" },
        { icon: "info", title: "Other files", detail: "Older versions and extra files, on its page", act: "files" }
    ].concat(srcNow === "nexus" ? [{ icon: "play", title: "Its Nexus page", detail: "Opens it in Turbine", act: "nexus" }] : [])
    property bool pendingFiles: false
    function cardMenuRun(i) {
        const it = cardMenuItems[i], c = cardItem
        if (!it || !c) return
        sheet = ""
        note("action", "Card menu: " + it.title + " for " + c.name)
        if (it.act === "files") { pendingFiles = true; openDetails(c) }
        else if (it.act === "get") quickGet(c)
        else if (it.act === "show" && cardMine) { const id = cardMine.id; tab = 0; zone = "main"; Qt.callLater(() => goToMod(id)) }
        else if (it.act === "toggle" && cardMine) toggle(cardMine)
        else if (it.act === "update" && cardMine) updateOne(cardMine)
        else if (it.act === "remove" && cardMine) uninstall(cardMine)
        else if (it.act === "nexus") openNexus("https://www.nexusmods.com/" + rules.domain + "/mods/" + c.id)
        else if (it.act === "page") openDetails(c)
        else if (it.act.indexOf("m:") === 0 && cardMine) {
            const m = cardMine, a = it.act.slice(2)
            // Move happens in the list itself: My mods, on this mod, then Move.
            if (a === "move") { tab = 0; zone = "main"; modPage = false; history = []; goToMod(m.id); Qt.callLater(() => modMenuDo({ act: "move" }, m)) }
            else modMenuDo({ act: a }, m)
        }
    }
    function getWords() {
        if (srcNow === "thunderstore") return "Installs it with what it needs"
        return account.premium === true ? "Installs it in one press" : "Opens its file page on Nexus"
    }
    // Y on a card (and the card menu's Get it on Nexus or Install): the same as the mod page's main button.
    function quickGet(c) {
        if (c.installed) { toast.info("Already in your mods.") ; return }
        if (srcNow === "nexus" && account.premium !== true) {
            // Free: the page knows the exact file, so open the mod page first.
            openDetails(c)
            pendingGet = true
            return
        }
        note("action", "Get " + c.name + " from " + srcNow)
        Turbine.install(c.id, srcNow)
        edited()
    }
    property bool pendingGet: false

    // ---------------------------------------------------------------- the mod page (pushed over Get mods)
    property bool modPage: false
    property var detailsData: null
    property var detailsItem: null
    property int pageIndex: 0
    function openDetails(item, fromHistory) {
        note("action", "Mod page: " + item.name)
        if (!fromHistory) {
            if (!modPage) pushResults()
        }
        history = history.concat([{ kind: "mod", id: String(item.id), item: item }])
        // Another mod's page over this one (LB, RB, a requirement, back): the new page settles in place.
        if (modPage && (!detailsItem || String(detailsItem.id) !== String(item.id))) modIn.play(0, px(mo.nudge))
        detailsItem = item
        detailsData = { loading: true }
        modPage = true
        // The main button ("Get it on Nexus", "Install") has the focus.
        pageIndex = 0
        Turbine.details(String(item.id), item.source || srcNow)
    }
    function partialPage(item, problem) {
        const src = item.source || srcNow
        const page = src === "nexus" && rules && rules.domain ? "https://www.nexusmods.com/" + rules.domain + "/mods/" + item.id : (item.page || "")
        note("net", "Mod page read only in part: " + problem)
        return { source: src, id: String(item.id), name: item.name || "", owner: item.owner || "", summary: item.summary || "", icon: item.icon || "",
                 version: item.version || "", downloads: item.downloads || 0, rating: item.rating || 0, updated: item.updated || "",
                 premium: false, page: page, filePage: page, partial: true, problem: problem, versions: [], needs: null }
    }
    onDetailsDataChanged: {
        if (pendingGet && detailsData && !detailsData.loading) { pendingGet = false; if (!detailsData.error) installFromDetails(); else if (reqRunning) reqAdvance("couldn't be opened (" + detailsData.error + ")") }
        if (pendingFiles && detailsData && !detailsData.loading) { pendingFiles = false; if (otherVersions.length > 0) openVersions(); else toast.info("No other files for this mod.") }
    }
    readonly property bool pageInstalled: !!detailsData && (detailsData.installed === true || (detailsItem && detailsItem.installed === true))
    // The installed mod this page is about (to say On or Off, turn it on, or show it in My mods).
    readonly property var pageMod: {
        if (!detailsData || !detailsData.id) return null
        const id = String(detailsData.id), src = detailsData.source || srcNow
        return mods.find(m => src === "nexus" ? nexusModOf(m) === id : m.sourceKind === src && (m.sourceId === id || String(m.sourceId || "").split(":")[0] === id)) || null
    }
    readonly property bool pageModOff: pageInstalled && !!pageMod && !pageMod.enabled
    readonly property string mainLabel: !detailsData || detailsData.loading ? "" : detailsData.error ? "" : detailsData.partial ? (detailsData.page ? "Open on Nexus" : "") : pageInstalled ? (pageModOff ? "Installed · Off" : "Show in My mods")
        : busyHere ? (dl.active ? "Installing " + Math.round(Math.max(0, dlPart) * 100) + "%" : "Installing...")
        : detailsData.source === "nexus" && detailsData.premium === false ? (launching ? "Opening Nexus..." : nexusFallback ? "Open on Nexus" : "Get it on Nexus")
        : "Install"
    readonly property string mainLine: !detailsData || detailsData.loading || detailsData.error ? ""
        : detailsData.partial ? (detailsData.page ? "Some details didn't load. A opens it on Nexus." : "Some details didn't load.")
        : pageInstalled ? (pageModOff ? "In your mods, turned off. A turns it on." : "In your mods and on.")
        : detailsData.source === "nexus" && detailsData.premium === false ? "Opens Nexus right here. Sign in if asked (free), then press Slow download. It's added automatically."
        : ""
    property bool busyHere: false
    readonly property var pageNeeds: {
        const n = detailsData && detailsData.needs ? detailsData.needs : null
        if (!n) return []
        return (n.installed || []).concat(n.missing || []).concat(n.unchecked || [])
    }
    readonly property int pageRows: 2 + pageNeeds.length
    // The first requirement not installed yet (Y opens its page), or -1.
    readonly property int pageMissing: {
        const n = detailsData && detailsData.needs ? detailsData.needs : null
        if (!n || !n.missing || n.missing.length === 0) return -1
        return (n.installed || []).length
    }
    readonly property string pageALabel: pageIndex === 0 ? mainLabel : pageIndex === 1 ? "Other files" : "Open its page"
    // X on the mod page: Other files, Get what it needs, Open on Nexus.
    readonly property var pageMenuItems: !detailsData || detailsData.loading ? [] : (otherVersions.length > 0 ? [
        { icon: "update", title: "Other files", detail: (detailsData.mainLine ? detailsData.mainLine + " · " : "") + plural(otherVersions.length, "other file", "other files"), run: () => openVersions() }
    ] : []).concat(pageMissing >= 0 ? [
        { icon: "update", title: "Get what it needs", detail: pageNeeds[pageMissing] ? pageNeeds[pageMissing].name : "", run: () => { pageIndex = pageMissing + 2; openNeed(pageMissing) } }
    ] : []).concat((detailsData.source || srcNow) === "nexus" && detailsData.page ? [
        { icon: "play", title: "Open on Nexus", detail: "Its page, in the browser", run: () => openNexus(detailsData.page) }
    ] : [])
    // The installed mod's Edit settings on its page (Left reaches it, A opens it).
    readonly property bool pageHasSettings: modPage && !!pageMod && settingsFilesOf(pageMod.id).length > 0
    property bool pageSettings: false
    function modPageAct(b) {
        if (b === "b") { getBack(); return }
        if (pageSettings) {
            if (b === "a") { note("action", "Mod page: settings of " + pageMod.name); openModSettings(pageMod) }
            else if (b === "left") { pageSettings = false; zone = "rail"; railIndex = current }
            else if (["right", "up", "down"].indexOf(b) >= 0) pageSettings = false
            if (b !== "lb" && b !== "rb") return
        }
        if (b === "left" && pageHasSettings) { pageSettings = true; return }
        if (b === "a") {
            if (pageIndex === 0) installFromDetails()
            else if (pageIndex === 1) openVersions()
            else openNeed(pageIndex - 2)
        }
        else if (b === "up") {
            if (pageIndex > 0) pageIndex = pageIndex === 2 ? 0 : pageIndex - 1
            else modView.scroll(-1)
        }
        else if (b === "down") {
            // The Other files row (1) isn't on the page (it is in X Options): skip it, and scroll when no row is left.
            const next = pageIndex === 0 ? 2 : pageIndex + 1
            if (next <= pageRows - 1) pageIndex = next; else modView.scroll(1)
        }
        else if (b === "left") { zone = "rail"; railIndex = current }
        // LB and RB: the previous or next mod in the results.
        else if (b === "lb" || b === "rb") {
            const i = getItems.findIndex(x => detailsItem && String(x.id) === String(detailsItem.id))
            const j = i + (b === "lb" ? -1 : 1)
            if (i >= 0 && j >= 0 && j < getItems.length) {
                history = history.slice(0, -1)
                getIndex = j
                const h = history.slice()
                if (h.length > 0 && h[h.length - 1].kind === "results") { h[h.length - 1] = Object.assign({}, h[h.length - 1], { area: "grid", index: j }); history = h }
                openDetails(getItems[j], true)
            }
        }
    }
    // A on a requirement: its page, pushed on the history.
    function openNeed(i) {
        const r = pageNeeds[i]
        if (!r) return
        if (r.modId > 0 && (!r.domain || r.domain === rules.domain)) openDetails({ id: String(r.modId), name: r.name, source: "nexus", icon: "" }, true)
        else if (r.url) openNexus(r.url)
    }
    function installFromDetails() {
        if (!detailsData || detailsData.loading || detailsData.error) return
        // Installing already: a second press never starts a second install.
        if (busyHere) { note("action", "Install pressed while installing: ignored"); toast.info("Installing " + detailsData.name + " already."); return }
        if (detailsData.partial) { if (detailsData.page) openNexus(detailsData.page); return }
        if (pageInstalled) {
            // Off: A turns it on (the button then says Show in My mods). On: A shows it in My mods.
            if (pageModOff) toggle(pageMod)
            else if (pageMod) goToMod(pageMod.id)
            // Installed, but its row isn't found by id: My mods all the same.
            else { modPage = false; history = []; tab = 0; zone = "main"; toast.info("It's in My mods.") }
            return
        }
        if (detailsData.source === "nexus" && detailsData.premium === false) {
            const url = detailsData.filePage || detailsData.page
            note("action", "Get it on Nexus: " + detailsData.name + " (" + url + ")")
            // What it needs is offered once it is in ("This mod needs N others", Get them all).
            chainQueue = []
            openNexus(url, "getit")
            return
        }
        note("action", "Install " + detailsData.name + " from " + srcNow)
        Turbine.install(detailsData.id, detailsData.source || srcNow)
        edited()
        busyHere = true
    }
    Connections { target: app; function onDlChanged() { if (!app.dl.active && app.busyHere && !Turbine.busy) app.busyHere = false } }

    // Pages to open on Nexus Mods (free accounts, SKSE, a profile's Nexus mods).
    property string pagesTitle: ""
    property var pagesList: []
    property int pagesIndex: 0
    function openPages(title, list) {
        if (!list || list.length === 0) return
        pagesTitle = title; pagesList = list; pagesIndex = 0
        Turbine.handlerStatus()
        sheet = "pages"
    }
    function openPage(i) {
        const p = pagesList[i]
        if (!p) return
        note("action", "Open page: " + p.name)
        // The list steps aside so the page shows and takes the presses; it comes back, on the
        // next page, when the page closes.
        pagesReturn = true
        sheet = ""
        const opened = openNexus(p.url)
        // Opened outside Turbine (or not at all): the list stays, on the next page when this one opened.
        if (!browserOpen) { pagesReturn = false; if (opened && pagesIndex < pagesList.length - 1) pagesIndex++; sheet = "pages" }
    }
    property bool pagesReturn: false

    // ---------------------------------------------------------------- Nexus inside Turbine (NexusBrowser.qml)
    // Nexus opens full screen in Turbine's window: the player signs in once, presses Slow download (or Mod Manager
    // Download) himself, and the file lands in Downloads/Game Mods/<Game>/ and is added. Desktop Mode can keep the
    // player's own browser (Settings, Getting mods, Open Nexus in); Steam's browser is in Labs.
    property bool browserOpen: false
    property string browserStart: ""
    property string browserOpener: ""      // "getit" (a mod's Get it), "page" (any other Nexus page), "key", "signin"
    property string browserGame: ""         // the game it was opened for
    property var browserFrom: null          // where B goes back to: {page, zone, tab, modPage, pageIndex}
    property int browserTab: 0              // 0 Page, 1 Downloads
    property int browserDlIndex: 0
    property var webDls: []                 // downloads: {obj, path, name, game, state, got, total, speed, words, at, lastGot, lastAt}
    property int webDlTick: 0
    property var browserNoteNow: null       // {text, level, at}: a refusal, the no-key message, a sign-in result
    property string browserAddedGame: ""    // a mod was added while it was open: holding Start plays that game
    property var browserArrival: null       // the landing waiting for the browser to close: {game, id, name, on}
    property bool browserLiveForced: false  // the build's checks: the page is made even without the stand-in
    property bool forceMyBrowser: false     // the build's slowhint picture: the Desktop "My browser" way
    readonly property bool browserLive: !shot.on || shot.arg("browser-test") !== "" || browserLiveForced
    readonly property string browserTest: shot.arg("browser-test")
    property real zoomTrial: 0              // the build's pictures change the size without saving it
    readonly property real browserZoom: zoomTrial > 0 ? zoomTrial : !shot.on && browserSet.zoom > 0 ? browserSet.zoom : Turbine.browserZoomDefault(height, sizeNow)
    // The controller works the page (A clicks at the pointer) only while nothing of Turbine's is over it.
    readonly property bool browserHasPad: page === "browser" && sheet === "" && !wheelOpen && browserTab === 0 && !keyboardUp && browserNoKeyPage === "" && !browserOnGoogle
    onBrowserHasPadChanged: Turbine.browserInput = browserHasPad
    readonly property bool browserEnterNow: page === "browser" && !!nexusView && nexusView.focusKind === 1
    onBrowserEnterNowChanged: Turbine.browserEnter = browserEnterNow
    // The build's stand-in page takes the place of www.nexusmods.com in its test runs.
    function browserAddress(url) {
        if (browserTest === "" || url.indexOf("https://www.nexusmods.com/") !== 0) return url
        const m = /^(http:\/\/127\.0\.0\.1:[0-9]+)/.exec(browserTest)
        return m ? m[1] + url.substring("https://www.nexusmods.com".length) : url
    }
    // Every "open Nexus" goes through here: in Turbine by default (always in Game Mode), the player's own browser
    // when he chose it in Desktop Mode.
    function openNexus(url, opener) {
        const outside = forceMyBrowser || (!Turbine.gamemode && browserSet.openIn === "mine")
        if (outside) return openNexusOutside(url)
        return openBrowser(url, opener || "page")
    }
    function openBrowser(url, opener) {
        if (launching && url === openUrlNow && browserOpen) {
            note("browser", "A ignored: already opening this page (" + (Date.now() - openAt) + " ms ago)")
            return true
        }
        openUrlNow = url; openAt = Date.now()
        launching = true; launchGuard.restart()
        launchCount++
        const addr = browserAddress(url)
        note("browser", "Nexus in Turbine: " + Turbine.browserLogUrl(addr) + " (" + opener + ")")
        if (page !== "browser") browserFrom = { page: page, zone: zone, tab: tab, modPage: modPage, pageIndex: pageIndex }
        browserOpener = opener
        browserGame = game ? game.id : ""
        browserStartMod = Turbine.browserPageMod(addr)
        browserAddedGame = ""
        browserArrival = null
        browserNoteNow = null
        browserNoKeyPage = ""
        browserBackground = []
        browserTab = 0
        // Already open: a fresh page with no history, so B closes on the first page Turbine opened (not after
        // stepping back through the pages before it).
        if (browserOpen && nexusView) nexusView.restart(addr)
        browserStart = addr
        browserOpen = true
        page = "browser"
        Turbine.browserMode(true)
        Qt.callLater(() => { if (nexusView) nexusView.focusPage() })
        return true
    }
    // Closes the page (downloads go on). why: "back" (B on the first page), "added" (auto close after an add),
    // "home", "installer", "menu".
    function closeBrowser(why) {
        if (!browserOpen) return
        note("browser", "Nexus page closed (" + why + ")" + (webDlsRunning > 0 ? ": " + plural(webDlsRunning, "download goes", "downloads go") + " on" : ""))
        browserLastUrl = nexusView && nexusView.url ? nexusView.url : browserStart
        browserOpen = false
        launching = false
        browserAutoClose.stop()
        browserNoKeyPage = ""
        if (browserTyping) browserTypingDone()
        Turbine.browserMode(false)
        Turbine.browserInput = false
        // The page closed by itself after an add: A, Start and RT pressed a moment later were meant for the page,
        // not for the Play button that now has the focus.
        if (why === "added") closedAddedAt = Date.now()
        const f = browserFrom
        browserFrom = null
        if (page === "browser") page = f && f.page !== "browser" ? f.page : "game"
        if (f) { tab = f.tab; modPage = f.modPage; pageIndex = f.pageIndex; zone = f.zone === "settings" && page !== "settings" ? "main" : f.zone }
        keyRoot.forceActiveFocus()
        const a = browserArrival
        browserArrival = null
        // The page's own mod went in and the page closed by itself: the focus goes to it (Play). Closed by hand: the
        // focus stays where Nexus was opened from, and a toast says what was added.
        if (a && why === "added") { landing = true; modArrived(a.game, a.id, a.name, a.on); landing = false }
        else {
            note("focus", "Back from Nexus: " + zone + (modPage ? ", the mod page's " + (pageIndex === 0 ? "main button" : "row " + pageIndex) : ""))
            if (a && why !== "play" && why !== "home") toast.done("Added " + a.name + "." + (game && a.game === game.id ? " Hold " + glyph("start") + " to play." : ""))
        }
        // Still downloading: it goes on, and the player is told so and where to see it (and when it lands, the
        // screen doesn't jump to it).
        browserBackground = webDls.filter(x => isRunning(x)).map(x => x.path)
        if (webDlsRunning > 0 && why !== "play") {
            const t = "Still downloading " + plural(webDlsRunning, "mod", "mods") + (webDlsRunning === 1 ? ". It's" : ". They're") + " added automatically."
            note("browser", t)
            toast.act("info", t, "Watch it", openNexusDownloads)
        }
        // The list of pages it was opened from comes back on the next one.
        if (pagesReturn) {
            pagesReturn = false
            if (pagesIndex < pagesList.length - 1 && why !== "play" && why !== "home" && why !== "installer") { pagesIndex = pagesIndex + 1; Qt.callLater(() => { if (app.sheet === "") app.sheet = "pages" }) }
        }
        // Install notes that came while the page was open.
        if (pendingNotes && sheet === "") { pendingNotes = false; infoIndex = 0; sheet = "notes" }
    }
    property string browserLastUrl: ""
    property var browserBackground: []      // downloads still running when the page closed (their paths)
    property real browserStartMod: 0        // the Nexus mod of the page Turbine opened (0: not a mod's page)
    property real closedAddedAt: 0
    // A mod's file page opened by Get it: its Slow download button is scrolled into view and the pointer put on
    // it (the page draws it a moment after it loads, so it is looked for a few times). Pressing stays the player's.
    property int slowTries: 0
    Timer {
        id: slowLineUp
        interval: 700
        onTriggered: {
            if (!app.browserOpen || !app.nexusView || app.browserOpener !== "getit") return
            app.nexusView.lineUpSlow(p => {
                if (p && p.x >= 0) {
                    app.note("browser", "Slow download lined up: scrolled into view" + (shot.on ? " (the build's check leaves the pointer where its steps put it)" : ", the pointer on it"))
                    if (!shot.on) Turbine.pointerShowAt(p.x, p.y)
                } else if (++app.slowTries < 4) slowLineUp.restart()
            })
        }
    }
    // View, Game menu, Nexus downloads (and a tap on "Getting 1 mod" at the top): the page again, on its Downloads tab.
    function openNexusDownloads() {
        sheet = ""
        const u = browserLastUrl !== "" ? browserLastUrl : browserStart
        if (u === "") return
        openBrowser(u, "page")
        browserTab = 1
    }
    property bool landing: false
    // B: back one page, or close on the first page Turbine opened.
    function browserBack() {
        if (browserTab === 1) { browserTab = 0; return }
        if (nexusView && nexusView.canGoBack) { note("browser", "Back one page"); nexusView.back() }
        else closeBrowser("back")
    }
    // A mod arrived while the page is open: the pill says so, Start plays it, and a single file Get it opened for
    // closes the page 2.5 seconds later with the focus on Play.
    function browserArrived(gameId, id, name, on) {
        browserAddedGame = gameId
        browserArrival = { game: gameId, id: id, name: name, on: on }
        // Closes by itself only for the page's own mod (the newest file that landed came from the mod Get it opened).
        const last = webDls.filter(x => x.state === "done" || x.state === "added").pop()
        const own = !!last && browserStartMod > 0 && last.mod === browserStartMod
        const auto = browserOpener === "getit" && webDlsRunning === 0 && gameId === browserGame && own
        note("browser", "Added while Nexus is open: " + name + (auto ? "; closing after 2.5 seconds with no press" : own ? "" : " (not the page's own mod: the page stays)"))
        if (auto) browserAutoClose.restart()
    }
    // 2.5 seconds with no press after the add (each press starts it again).
    Timer { id: browserAutoClose; interval: 2500; onTriggered: if (app.browserOpen && app.webDlsRunning === 0) app.closeBrowser("added") }
    Connections {
        target: Turbine
        function onBrowserPressCountChanged() { app.browserPressed() }
    }
    // A press in the page (A, RT, R3, a click or tap, the D-pad): the auto close waits again, and a note goes.
    function browserPressed() {
        if (browserAutoClose.running) browserAutoClose.restart()
        if (browserNoteNow && browserNoteNow.level !== "busy" && browserNoKeyPage === "" && Date.now() - browserNoteNow.at > 400) browserNoteNow = null
    }
    readonly property var nexusView: browserLoader.item

    // ---- downloads (the profile's downloadRequested: the request is only good during the signal)
    Connections {
        target: app.browserLive && app.browserOpen ? NexusProfile : null
        function onDownloadRequested(d) { app.webDownload(d) }
    }
    function webDownload(d) {
        const pageUrl = nexusView ? nexusView.url : browserStart
        // What running downloads still have to come counts against the free space too.
        let pending = 0
        for (const x of webDls) if (isRunning(x) && x.total > 0) pending += Math.max(0, x.total - x.got)
        const r = parse(Turbine.browserPlan(d.url.toString(), d.suggestedFileName || "", pageUrl, d.totalBytes, pending), { ok: false, error: "Turbine couldn't take that download." })
        if (!r.ok) {
            d.cancel()
            browserSay(r.error, "warn")
            return
        }
        d.downloadDirectory = r.dir
        d.downloadFileName = r.name
        d.accept()
        webDls = webDls.concat([{ obj: d, path: r.path, name: r.name, game: r.game, sure: r.sure === true, mod: r.mod || 0, state: "requested", got: 0, total: d.totalBytes, speed: 0,
                                  words: r.sure ? "Starting" : "Starting. Not from Nexus's own file host: Turbine asks which game it is for", at: Date.now(), lastGot: 0, lastAt: Date.now(), reported: false }])
        browserDlIndex = webDls.length - 1
    }
    // "about 2 min left" from the speed so far.
    function timeLeft(x) {
        if (!(x.total > 0) || !(x.speed > 0) || x.got >= x.total) return ""
        const sec = (x.total - x.got) / x.speed
        return sec < 60 ? "under a minute left" : sec < 3600 ? "about " + Math.round(sec / 60) + " min left" : "about " + Math.round(sec / 360) / 10 + " h left"
    }
    // Twice a second: each download's state, bytes and speed (Qt keeps no history, so rows stay until Turbine closes).
    function isRunning(x) { return x.state === "requested" || x.state === "progress" || x.state === "paused" }
    readonly property int webDlsRunning: { webDlTick; return webDls.filter(x => isRunning(x)).length }
    // All running downloads' share done (-1 when a size isn't known), for "Getting 1 mod · 40%" at the top.
    readonly property real webDlPart: {
        webDlTick
        const run = webDls.filter(x => isRunning(x))
        if (run.length === 0 || run.some(x => !(x.total > 0))) return -1
        let got = 0, total = 0
        for (const x of run) { got += x.got; total += x.total }
        return total > 0 ? Math.min(1, got / total) : -1
    }
    Timer {
        interval: 500
        repeat: true
        running: app.webDls.length > 0
        onTriggered: app.webDlPoll()
    }
    function webDlPoll() {
        let changed = false
        const now = Date.now()
        for (const x of webDls) {
            const o = x.obj
            if (!o) { if (x.state !== "cancelled" && x.state !== "added" && x.state !== "done") { x.state = "interrupted"; x.words = "Stopped"; changed = true } continue }
            // WebEngineDownloadRequest states: 0 requested, 1 in progress, 2 completed, 3 cancelled, 4 interrupted.
            const st = o.state
            if (st === 1 || st === 0) {
                const got = o.receivedBytes, total = o.totalBytes
                if (now - x.lastAt >= 1000) { x.speed = Math.max(0, (got - x.lastGot) * 1000 / (now - x.lastAt)); x.lastGot = got; x.lastAt = now }
                const s = o.isPaused ? "paused" : "progress"
                if (got !== x.got || total !== x.total || s !== x.state) changed = true
                x.got = got; x.total = total; x.state = s
                // Past 64 GB, or the disk nearly full: stopped now, and its partial file goes at once.
                const stop = Turbine.browserDlCheck(x.path, got)
                if (stop !== "") {
                    x.stopWhy = stop
                    note("browser", "Stopping " + x.name + ": " + stop)
                    o.cancel()
                    x.state = "cancelled"; x.words = "Stopped: " + stop; x.goneAt = now; changed = true
                    Turbine.browserFailed(x.path, "stopped: " + stop)
                    browserSay(x.name + " stopped: " + stop + ". Its partial file was removed.", "warn")
                    continue
                }
                const left = s === "progress" ? timeLeft(x) : ""
                x.words = (s === "paused" ? "Paused · " : "") + sizeWords(got) + (total > 0 ? " of " + sizeWords(total) : "") + (s === "progress" && x.speed > 0 ? " · " + sizeWords(Math.round(x.speed)) + "/s" : "") + (left ? " · " + left : "")
            } else if (st === 2 && x.state !== "done" && x.state !== "added") {
                x.state = "done"; x.got = o.receivedBytes; x.total = Math.max(o.totalBytes, o.receivedBytes); x.words = "Adding..."
                changed = true
                Turbine.browserDone(x.path)
            } else if (st === 3 && x.state !== "cancelled") {
                x.state = "cancelled"; x.words = "Cancelled"; x.goneAt = now; changed = true
                browserBackground = browserBackground.filter(p => p !== x.path)
                Turbine.browserFailed(x.path, "cancelled")
            } else if (st === 4 && x.state !== "interrupted" && x.state !== "cancelled") {
                x.why = o.interruptReasonString || ""; changed = true
                // The disk filled up: stopped for good, and its partial file goes at once (not at the next start).
                // WebEngineDownloadRequest's interruptReason 3 is FileNoSpace.
                if (o.interruptReason === 3 || /no space|disk.*full|not enough space/i.test(x.why)) {
                    o.cancel()
                    x.state = "cancelled"; x.words = "Stopped: the disk is full"; x.goneAt = now
                    Turbine.browserFailed(x.path, "nospace: " + x.why)
                    browserSay(x.name + " stopped: the disk is full. Its partial file was removed.", "warn")
                    continue
                }
                x.state = "interrupted"
                // Nexus's file address has expired (the server refuses it): a new press of Slow download is needed.
                x.server = /server|forbidden|unauthori|bad request|not found/i.test(x.why)
                x.words = x.server ? "The address expired: press Slow download on the page again" : "Stopped" + (x.why ? " (" + x.why + ")" : "") + ". A: Try again"
                Turbine.browserFailed(x.path, "interrupted: " + x.why)
            }
        }
        // Cancelled rows go after 8 seconds.
        const keep = webDls.filter(x => !(x.state === "cancelled" && now - (x.goneAt || now) > 8000))
        if (keep.length !== webDls.length) { webDls = keep; browserDlIndex = Math.min(browserDlIndex, Math.max(0, webDls.length - 1)); changed = true }
        if (changed) webDlTick++
    }
    // A finished download went in (or Turbine asks about it, or it failed): its own row says so, matched by its
    // path (two finishing together, or another status, never label the wrong row).
    Connections {
        target: Turbine
        function onAddResultJsonChanged() {
            const r = app.parse(Turbine.addResultJson, {})
            const x = app.webDls.find(d => d.path === r.path)
            if (!x) return
            if (r.ask) { x.state = "asked"; x.words = r.text }
            else if (r.ok) { x.state = "added"; x.words = r.text }
            else { x.state = "failed"; x.words = r.text }
            // Nothing lands for a question or a failure: no arrival to wait for.
            if (!r.ok) app.browserBackground = app.browserBackground.filter(p => p !== x.path)
            if (app.browserOpen) app.browserSay(r.text, r.ok ? "ok" : "warn")
            app.webDlTick++
        }
    }
    function logUrl(u) { return Turbine.browserLogUrl(u) }
    function browserSay(text, level) {
        browserNoteNow = { text: text, level: level, at: Date.now() }
        note("browser", "Pill: " + text)
    }
    // The pill's words: downloads first, then what just happened, then what to press.
    readonly property var browserPill: {
        webDlTick
        // Waiting for the player's answer (A the file's page, B OK) comes first.
        if (browserNoKeyPage !== "" && browserNoteNow) return { text: browserNoteNow.text, level: "warn", progress: -1 }
        const run = webDls.filter(x => isRunning(x))
        if (run.length > 1) return { text: run.length + " downloading", level: "busy", progress: -1 }
        if (dl.active && run.length === 0) return { text: "Downloading " + dl.label + (dl.total > 0 ? " " + Math.min(99, Math.floor(dl.got * 100 / dl.total)) + "%" : ""), level: "busy", progress: dl.total > 0 ? dl.got / dl.total : -1 }
        if (run.length === 1) {
            const x = run[0]
            const pct = x.total > 0 ? Math.min(99, Math.floor(x.got * 100 / x.total)) : -1
            return { text: "Downloading " + x.name + (pct >= 0 ? " " + pct + "%" : ""), level: "busy", progress: pct >= 0 ? pct / 100 : -1 }
        }
        if (webDls.some(x => x.state === "done")) return { text: "Adding...", level: "busy", progress: -1 }
        const n = browserNoteNow
        if (n) return { text: n.text, level: n.level, progress: -1 }
        if (signinWaitSince > 0 && signinWaitText !== "") return { text: signinWaitText, level: "busy", progress: -1 }
        const stopped = webDls.filter(x => x.state === "interrupted")
        if (stopped.length > 0) return { text: stopped[stopped.length - 1].server ? "Press Slow download on the page again" : stopped[stopped.length - 1].name + " stopped. Downloads: Try again", level: "warn", progress: -1 }
        if (browserIsFilePage && account.premium !== true)
            return { text: browserSignedIn ? "Press Slow download (free). Mod Manager Download needs a Nexus key." : "Sign in first (free), then press Slow download", level: "hint", progress: -1 }
        return { text: "", level: "ok", progress: -1 }
    }
    // Signed in to Nexus in Turbine's browser (its sign-in cookie), looked at while the page is open.
    // One sign-in status everywhere (Settings, the page's hints, the Nexus menu): whether Turbine's browser
    // holds Nexus's sign-in. It is read all the time, not only while a Nexus page is open, and the browser's kept
    // sign-in is read a moment after start, so Settings never says "Not signed in" before Nexus is opened.
    property bool browserSignedIn: false
    function signinRefresh() { const v = Turbine.browserSignedIn(); if (v !== browserSignedIn) { browserSignedIn = v; note("web", "Sign-in status: " + (v ? "signed in" : "not signed in")) } }
    Timer { interval: app.browserOpen ? 1000 : 3000; repeat: true; running: true; triggeredOnStart: true; onTriggered: app.signinRefresh() }
    property bool signinRead: false
    function signinLoad(why) {
        if (signinRead) return
        signinRead = true
        // Touching the browser's profile makes it and reads its kept cookies (webglue.cpp); the status follows.
        const p = NexusProfile
        note("web", "Nexus sign-in read from Turbine's browser (" + why + "): " + (p ? "its profile is ready" : "no profile"))
        signinRefresh()
    }
    Timer { interval: 3500; running: !shot.on; onTriggered: app.signinLoad("after start") }
    // "Google sign-in" (the player's own browser through the helper) shows only with a paired helper and while the
    // page isn't signed in.
    readonly property bool browserSigninShown: bridge.paired === true && !browserSignedIn
    // A message under the bar (cut short in the pill) shows for 6 seconds, then only the pill keeps it; a done or
    // info message goes after 15 seconds.
    property real noteClock: 0
    Timer { interval: 500; repeat: true; running: app.browserOpen && !!app.browserNoteNow; onTriggered: {
        app.noteClock = Date.now()
        const n = app.browserNoteNow
        if (n && n.level !== "warn" && app.browserNoKeyPage === "" && Date.now() - n.at > 15000) app.browserNoteNow = null
    } }
    readonly property bool browserNoteFresh: !browserNoteNow || noteClock === 0 || noteClock - browserNoteNow.at < 6000
    readonly property bool browserIsFilePage: !!nexusView && /[?&]tab=files/.test(nexusView.url) && Turbine.browserIsNexus(nexusView.url)
    // The Downloads tab's rows.
    readonly property var browserRows: { webDlTick; return webDls.map(x => ({ name: x.name, game: x.game, state: x.state, got: x.got, total: x.total, words: x.words })) }
    function browserRowAct(i) {
        const x = webDls[i]
        if (!x || !x.obj) return
        if (x.state === "interrupted") { note("action", "Try again: " + x.name); x.obj.resume(); x.state = "progress"; x.words = "Trying again"; webDlTick++ }
        else if (x.state === "progress") { note("action", "Pause: " + x.name); x.obj.pause() }
        else if (x.state === "paused") { note("action", "Resume: " + x.name); x.obj.resume() }
    }
    function browserRowLabel(i) {
        const x = webDls[i]
        return !x ? "" : x.state === "interrupted" && !x.server ? "Try again" : x.state === "progress" ? "Pause" : x.state === "paused" ? "Resume" : ""
    }
    function browserRowMenu(i) {
        const x = webDls[i]
        if (!x) return
        const dir = x.path.substring(0, x.path.lastIndexOf("/"))
        openPick(x.name, x.game || "Other", (isRunning(x) ? [
            { icon: "remove", title: "Stop", detail: "Stops this download", danger: true, run: () => { if (x.obj) x.obj.cancel() } }
        ] : []).concat(x.state === "added" ? [
            { icon: "info", title: "Show in Turbine", detail: "Its game's My mods", run: () => closeBrowser("menu") }
        ] : []).concat(Turbine.gamemode ? [] : [
            // A file manager means nothing in Game Mode: Desktop Mode only.
            { icon: "info", title: "Open folder", detail: dir.replace(/^.*\/Downloads\//, "Downloads/"), run: () => { if (!shot.on && !Qt.openUrlExternally("file://" + encodeURI(dir))) toast.info("It's in " + dir) } }
        ]), "")
    }
    // X and LT (a right click on the page): Turbine's quick menu for the page, at most 6 rows. Back is B, so it isn't
    // here; Forward only when there is a page to go forward to.
    function openBrowserMenu(keep) {
        if (!browserOpen) return
        const zoomWords = Math.round(browserZoom * 100) + "% · Left, Right change it" + (Math.abs(browserZoom - browserZoomSaved) > 0.001 ? " · A keeps it" : "")
        openPick("Nexus", nexusView ? nexusView.title : "", (reqRunning ? [
            { icon: "update", title: "Skip this one", detail: reqStripWords, run: () => reqSkip() },
            { icon: "remove", title: "Stop getting them", detail: "My mods keeps a row to go on later", run: () => reqStop() }
        ] : []).concat(nexusView && nexusView.canGoForward ? [
            { icon: "info", title: "Forward", detail: "The page you went back from", run: () => { if (nexusView) nexusView.forward() } }
        ] : []).concat([
            { icon: "update", title: "Reload", detail: "", run: () => { if (nexusView) nexusView.reload() } },
            { icon: "info", title: "Zoom", detail: zoomWords, keep: true, step: (d) => browserZoomBy(d * 0.1), run: () => browserZoomKeep() },
            { icon: "restore", title: "Back to the first page", detail: "The page Nexus opened on", run: () => { if (nexusView) nexusView.url = browserStart } }
        ]).concat(labsOn ? [
            { icon: "info", title: "Copy page address", detail: nexusView ? Turbine.browserLogUrl(nexusView.url) : "", run: () => { Turbine.copyText(nexusView ? nexusView.url : browserStart); toast.done("Address copied.") } }
        ] : []).concat(browserSigninShown ? [
            { icon: "play", title: "Google sign-in", detail: "Signs in on Nexus in Chrome, then Turbine takes the sign-in", run: () => signinStart() }
        ] : []).concat([
            { icon: "restore", title: "Close", detail: webDlsRunning > 0 ? "Downloads go on and are added automatically" : "Back to where you opened it", run: () => closeBrowser("menu") }
        ]), "")
        if (keep !== undefined) menuIndex = Math.min(keep, pickItems.length - 1)
    }
    // The page size: this time only (Left and Right), or kept for next time (A on Zoom, or Settings).
    readonly property real browserZoomSaved: !shot.on && browserSet.zoom > 0 ? browserSet.zoom : Turbine.browserZoomDefault(height, sizeNow)
    function browserZoomBy(d) {
        const z = Math.max(0.5, Math.min(3, Math.round((browserZoom + d) * 10) / 10))
        zoomTrial = z
        note("action", "Nexus zoom " + Math.round(z * 100) + "%")
    }
    function browserZoomKeep() {
        if (!shot.on) browserSet.zoom = browserZoom
        note("action", "Nexus zoom kept: " + Math.round(browserZoom * 100) + "%")
        toast.done("Nexus pages now start at " + Math.round(browserZoom * 100) + "%.")
    }
    // Y, and by itself once each time a text field gets the focus: Steam's keyboard.
    property int browserFieldSeen: 0
    function browserKeyboard() {
        note("action", "Nexus: keyboard")
        if (Turbine.gamemode && !shot.on) {
            Qt.openUrlExternally("steam://open/keyboard")
            // Until the field loses the focus (or B, Done typing), the pad's presses don't work the page: a B meant
            // for Steam's keyboard never goes back a page.
            browserTyping = true
            keyboardUp = true
        }
    }
    property bool browserTyping: false
    function browserTypingDone() {
        if (!browserTyping) return
        browserTyping = false
        keyboardUp = false
        note("browser", "Done typing on the page")
        if (Turbine.gamemode && !shot.on) Qt.openUrlExternally("steam://close/keyboard")
    }
    Connections {
        target: app.nexusView
        function onFocusKindChanged() {
            if (!app.nexusView) return
            if (app.nexusView.focusKind === 2) { app.browserFieldSeen++; app.note("browser", "A text field has the focus: Steam's keyboard opens"); app.browserKeyboard() }
            else if (app.browserTyping) app.browserTypingDone()
        }
        function onPageEvent(what, detail) {
            if (what === "url") return
            // Addresses go into the log as host and path only (signed file links and sign-in codes stay out).
            if (what === "loaded" || what === "failed" || what === "blocked" || what === "certificate" || what === "upload" || what === "crashed" || what === "newwindow" || what === "popupblocked" || what === "nxmrefused")
                app.note("browser", "Page " + what + (detail ? ": " + (/^[a-z]+:/i.test(detail) ? app.logUrl(detail) : detail) : ""))
            if (what === "popupblocked") app.browserSay("The page tried to open a window. Turbine only opens windows you ask for.", "warn")
            if (what === "blocked") app.browserSay("Turbine doesn't open " + detail + " links.", "warn")
            if (what === "certificate") app.browserSay("That site's security certificate isn't valid, so Turbine won't open it.", "warn")
            if (what === "upload") app.browserSay("Uploading files isn't possible here.", "warn")
            if (what === "crashed") app.browserSay("The page stopped. X, Reload brings it back.", "warn")
            if (what === "loaded" && app.launching) app.launching = false
            if (what === "loaded" && app.browserOpener === "getit") { app.slowTries = 0; slowLineUp.restart() }
        }
        function onDialogWanted(request) { app.browserDialog(request) }
        function onMenuWanted() { app.openBrowserMenu() }
        function onSigninClicked() { if (!app.gate(0, "browser onSigninClicked")) return; app.signinStart() }
        function onGooglePassword() { if (!app.gate(0, "browser onGooglePassword")) return; app.googleSetPassword() }
        function onGoogleChrome() { if (!app.gate(0, "browser onGoogleChrome")) return; app.googleUseChrome() }
        function onPagePressed() { Turbine.browserPressed() }
        function onPageDown() { Turbine.browserPointerDown() }
        function onCloseClicked() { if (!app.gate(0, "browser onCloseClicked")) return; app.closeBrowser("back") }
        function onHintClicked(k) { if (!app.gate(0, "browser onHintClicked")) return; app.note("action", "Legend " + k + " (the Nexus page's bar)"); app.act(k) }
        function onTabClicked(i) { if (!app.gate(0, "browser onTabClicked")) return; app.browserTab = i }
        function onDownloadClicked(i) { if (!app.gate(0, "browser onDownloadClicked")) return; app.browserDlIndex = i; app.browserRowAct(i) }
    }
    // The page's alert and questions, in Turbine's own menu (the answer goes back to the page).
    function browserDialog(request) {
        // While Turbine asks about a Mod Manager Download, the page's own alert or question never replaces it (the
        // page's sheet would have no one-second guard, and the link would be lost): it is answered No, and logged.
        if (nxmAsking) { note("nxm", "The page's " + (["alert", "question", "prompt", "leave question"][request.type] || "dialog") + " while Turbine asks: dropped"); request.dialogReject(); return }
        // 0 alert, 1 confirm, 2 prompt, 3 before leaving the page.
        if (request.type === 0) { openPick("The page says", "", [{ icon: "info", title: "OK", detail: request.message, run: () => request.dialogAccept() }], ""); return }
        if (request.type === 2) { request.dialogReject(); return }
        openPick(request.type === 3 ? "Leave this page?" : "The page asks", "", [
            { icon: "play", title: request.type === 3 ? "Leave" : "OK", detail: request.message, run: () => request.dialogAccept() },
            { icon: "remove", title: request.type === 3 ? "Stay" : "Cancel", detail: "", run: () => request.dialogReject() }
        ], "")
    }

    // Start held after an add: Play that game (the page closes first).
    function browserPlay() {
        const g = browserAddedGame
        if (g === "") return
        closeBrowser("play")
        const i = games.findIndex(x => x.id === g)
        if (i >= 0 && i !== current) current = i
        startPlay()
    }
    // The quick wheel's Search Nexus: Steam's keyboard, then this game's search on Nexus.
    function browserSearch() {
        openAsk("nexussearch", "Search Nexus", "Words from the mod's name or what it does. Shows " + (gameTitle || "this game") + "'s mods on Nexus.", "", "Search Nexus", 100, false, "Search")
    }
    // ---- Mod Manager Download without a key, in the browser's pill (A goes to the file's own page).
    property string browserNoKeyPage: ""
    function browserNoKey(page, link) {
        const m = /^nxm:\/\/([a-z0-9_-]+)\/mods\/([0-9]+)\/files\/([0-9]+)/i.exec(link || "")
        const exact = m ? Turbine.browserStartUrl(m[1].toLowerCase(), parseInt(m[2]), parseInt(m[3])) : ""
        const target = exact !== "" ? exact : page
        const here = nexusView ? nexusView.url : ""
        // Already on that file's page: only a note (A still clicks, B still goes back); it goes after 8 seconds or
        // at the next click. Only a file page somewhere else takes A over (A goes there, B not now).
        if (target === "" || (here !== "" && Turbine.browserSamePage(browserAddress(target), here))) {
            browserNoKeyPage = ""
            browserSay("Mod Manager Download needs a Nexus key (Settings, Getting mods, Faster downloads). Press Slow download here instead. It's free.", "warn")
            noKeyNoteClear.restart()
            return
        }
        browserNoKeyPage = target
        browserSay("Mod Manager Download needs a Nexus key (Settings, Getting mods, Faster downloads). A opens this file's page, where Slow download is free.", "warn")
    }
    Timer { id: noKeyNoteClear; interval: 8000; onTriggered: if (app.browserNoteNow && app.browserNoKeyPage === "" && /needs a Nexus key/.test(app.browserNoteNow.text)) app.browserNoteNow = null }
    // A on that message: the file's own page, where Slow download is.
    function browserNoKeyGo() {
        const u = browserNoKeyPage
        browserNoKeyPage = ""; browserNoteNow = null
        note("action", "Nexus: the file's page for Slow download")
        if (nexusView && u !== "") nexusView.load(browserAddress(u))
    }

    // ---- Sign in with my browser (Google and others)
    // Google won't sign in inside an app's browser. On its sign-in page Turbine's card offers the two ways that work:
    // a Nexus password (Nexus emails a link; then email and password work right here), or Chrome with the helper.
    readonly property bool browserOnGoogle: page === "browser" && browserTab === 0 && !!nexusView && /^https:\/\/accounts\.google\.com\//i.test(nexusView.url)
    readonly property string googlePasswordUrl: "https://users.nexusmods.com/auth/password/new"
    function googleSetPassword() {
        note("action", "Google page: set a Nexus password (stays here)")
        if (nexusView) nexusView.load(googlePasswordUrl)
    }
    function googleUseChrome() {
        note("action", "Google page: use Chrome instead")
        if (bridge.paired === true) { signinStart(); return }
        const t = Turbine.gamemode
            ? "Chrome needs Desktop Mode once: add Chrome to Steam (Settings, Labs, Use Chrome in Game Mode) and set up the browser helper (Settings, Getting mods, Using your own browser). Then Google sign-in shows here."
            : "Chrome needs the browser helper once (Settings, Getting mods, Using your own browser). Then Google sign-in shows here."
        browserSay(t, "warn")
        toast.info(t)
    }
    // Waiting for the sign-in from the player's browser: the pill keeps saying so (until it comes, fails, or the
    // helper's 15 minutes are over; then it says how to try again).
    property double signinWaitSince: 0
    property string signinWaitText: ""
    Timer {
        interval: 5000; repeat: true; running: app.signinWaitSince > 0
        onTriggered: if (Date.now() - app.signinWaitSince > 15 * 60 * 1000) {
            app.signinWaitSince = 0
            app.note("browser", "Sign in with my browser: nothing came back within 15 minutes")
            app.browserSay("The sign-in didn't come back from the browser in 15 minutes. Press Google sign-in to try again.", "warn")
        }
    }
    // After a sign-in from the browser: did Turbine really come to the front? Said in the activity log either way.
    Timer {
        id: signinFrontCheck; interval: 5000
        onTriggered: app.note("browser", app.active
            ? "Sign in with my browser: Turbine is in front again"
            : "Sign in with my browser: Turbine didn't come to the front within 5 seconds (the browser's page says: press the Steam button, then pick Turbine)")
    }
    function signinStart() {
        const r = parse(Turbine.signinStart(), { how: "error", text: "Turbine couldn't start the sign-in." })
        note("browser", "Sign in with my browser: " + r.how)
        if (r.how === "nohelper" || r.how === "error") { browserSay(r.text, "warn"); toast.info(r.text); return }
        if (r.how === "guarded") { if (signinWaitSince === 0) signinWaitSince = Date.now(); signinWaitText = r.text; browserSay(r.text, "hint"); return }
        if (r.how === "desktop" || r.how === "shortcut" || r.how === "helper") { signinWaitSince = Date.now(); signinWaitText = r.text }
        browserSay(r.text, r.how === "steam" ? "warn" : "hint")
        if (shot.on) return
        if (r.how === "desktop") Qt.openUrlExternally(r.url)
        else if (r.steam) Qt.openUrlExternally(r.steam)
    }
    Connections {
        target: Turbine
        function onWebSigninNote(text, level) {
            if (level === "wait") {
                if (app.signinWaitSince > 0) { app.signinWaitText = text; app.browserSay(text, "hint") }
                return
            }
            if (level === "warn") app.signinWaitSince = 0
            app.browserSay(text, level === "ok" ? "ok" : "warn")
            if (level === "ok") toast.done(text)
            else toast.problem(text)
        }
        function onWebSignedIn(text, back) {
            app.signinWaitSince = 0
            app.note("browser", text)
            if (app.nexusView) app.nexusView.reload()
            app.browserSay(text, "ok")
            toast.done(text)
            // Back to the front: Game Mode through Steam (Turbine's own id); otherwise the window comes forward.
            if (shot.on) return
            if (back !== "") Qt.openUrlExternally(back)
            else if (Turbine.gamemode) toast.info(text + ". Press the Steam button, then Turbine.")
            app.raise(); app.requestActivate()
            signinFrontCheck.restart()
        }
    }

    // ---------------------------------------------------------------- the player's own browser (catch.rs)
    // What the browser check found, the browser helper's link, and the status shown over the browser.
    property var browser: ({ checked: false })
    property var bridge: ({ running: false })
    property var status: ({ active: false })
    property bool movedPaired: false
    readonly property bool browserFound: !browser.checked || (!!browser.result && browser.result.found === true)
    // Desktop Mode with Open Nexus in: My browser: opens a Nexus page in the player's browser and watches Downloads
    // for its file. False when there is no browser. The status pill says what is happening for up to 8 seconds.
    property string openState: ""         // "", "opening", "open"
    property string openName: "the browser"
    property string openUrlNow: ""
    property double openAt: 0
    property string addMode: ""           // "noshortcut" or "helper": what the Add Chrome to Steam sheet says
    // A pressed again while the browser is still starting does nothing for 8 seconds (one browser, not three);
    // launch.rs keeps the same guard for the same page.
    property bool launching: false
    Timer { id: launchGuard; interval: 8000; onTriggered: app.launching = false }
    function openNexusOutside(url) {
        if (!browserFound) { noBrowser(); return false }
        if (launching && url === openUrlNow) {
            note("browser", "A ignored: already opening this page (" + (Date.now() - openAt) + " ms ago)")
            return true
        }
        const w = parse(Turbine.watchPage(url), {})
        if (w.watching) note("catch", "Watching Downloads for this mod (" + w.game + ")")
        openUrlNow = url
        openAt = Date.now()
        launching = true; launchGuard.restart()
        launchCount++
        note("browser", "Browser launch: " + url)
        openName = browser.result && browser.result.name ? browser.result.name : shot.on ? "Chrome" : "the browser"
        if (shot.on) { openState = "opening"; openGone.restart(); return true }
        const r = parse(Turbine.openUrl(url), { how: "desktop" })
        if (r.how === "guarded") return true
        openState = "opening"
        openGone.restart()
        Qt.openUrlExternally(url)
        return true
    }
    function showAddBrowser(mode) {
        addMode = mode
        note("browser", mode === "noshortcut" ? "Showing Add Chrome to Steam (once)" : "Showing Set up the browser helper too (once)")
        infoIndex = 0
        sheet = "addbrowser"
    }
    // Recheck on Add Chrome to Steam (and Labs): is a Chrome-family browser among Steam's non-Steam games now?
    function recheckBrowserShortcut() {
        const r = parse(Turbine.browserShortcut(), { found: false })
        if (r.found) {
            sheet = ""
            toast.info("Found " + r.browser + " in Steam as " + r.name + ". Nexus pages open there now.")
            if (openUrlNow !== "" && !shot.on) openNexus(openUrlNow)
        } else toast.info("Not in Steam yet. Add Google Chrome as a non-Steam game in Desktop Mode, then Recheck.")
        return r
    }
    function openNexusMods() {
        if (!rules || !rules.domain) return
        note("action", "Open " + gameTitle + " mods on Nexus Mods")
        openNexus("https://www.nexusmods.com/" + rules.domain + "/mods")
    }
    function steamOpen() {
        note("action", "Open in Steam's browser")
        openState = ""
        if (shot.on) return
        const link = Turbine.openInSteam(openUrlNow)
        if (link === "") { toast.info("Steam's browser can't open that address."); return }
        // Opened like Play's steam://rungameid, so Game Mode shows it.
        Qt.openUrlExternally(link)
        openName = "Steam's browser"
        openState = "opening"
        openGone.restart()
    }
    Timer { id: openGone; interval: 8000; onTriggered: app.openState = "" }
    Connections {
        target: app
        function onActiveChanged() {
            if (!app.active && app.openState === "opening") {
                app.note("browser", app.openName + " came up: Turbine went to the background " + (Date.now() - app.openAt) + " ms after Open")
                app.openState = "open"; openGone.stop()
            }
            else if (app.active && app.openState === "open") openGone.restart()
        }
    }
    function noBrowser() {
        note("browser", "No browser found: showing how to get one")
        infoIndex = 0
        sheet = "nobrowser"
    }
    // The browser helper: copied into Downloads, then the four steps; Pair opens the pairing page.
    function setupHelper() {
        const r = parse(Turbine.helperSetup(), { error: "Turbine couldn't put the helper in Downloads." })
        if (r.error) { toast.info(r.error); return }
        infoIndex = 0
        sheet = "helper"
    }
    function pair(again) {
        if (!browserFound) { noBrowser(); return }
        const url = again ? Turbine.bridgePairAgain() : Turbine.bridgePairUrl()
        if (url === "") { toast.info("Turbine's link for the browser helper isn't running. Close Turbine and open it again."); return }
        if (!shot.on) Qt.openUrlExternally(url)
        toast.info("Opened the pairing page in the browser. It says Paired with Turbine when the helper is set up.")
    }
    readonly property string browserWords: !browser.checked ? "Looking for a web browser..."
        : browserFound ? browser.result.name + (browser.result.how === "flatpak" ? " (installed from Discover)" : browser.result.how === "default" ? " (your default browser)" : "")
          + ". Nexus pages open there."
        : "None found. Turbine opens Nexus Mods in your browser; A shows how to get one."
    readonly property string bridgeTitle: bridge.connected > 0 ? "Browser helper: Connected" : bridge.paired ? "Browser helper: Paired" : "Browser helper: Not paired"
    readonly property string bridgeWords: !bridge.running ? (bridge.error ? "Turbine's link isn't running: " + bridge.error + "." : "Turbine's link isn't running.")
        : bridge.connected > 0 ? "The helper is talking to Turbine now: downloads, progress and installer choices show on Nexus pages."
        : bridge.paired ? "Paired. It connects when the browser is open."
        : "Optional. After setting it up, Pair opens a page that links the helper to this Turbine."
    // Install notes after adding a mod, and "Add this file?" for a download the watcher could only guess.
    property string notesName: ""
    property var notesList: []
    property var found: null
    property int infoIndex: 0
    // X on "Found <file>": which installed game it goes into.
    property int pickIndex: 0
    function openPickGame() {
        if (!found || games.length === 0) return
        pickIndex = Math.max(0, games.findIndex(g => g.id === found.game))
        note("action", "Pick the game for " + found.name)
        sheet = "pickgame"
    }
    function pickGame(i) {
        const g = games[i], f = found
        if (!g || !f) return
        found = null; sheet = ""
        Turbine.catchPick(f.path, g.id)
    }
    function openNotes(name, list) {
        if (!list || list.length === 0) return
        notesName = name; notesList = list
        note("engine", "Install notes for " + name + ": " + list.length)
        // Never over the Nexus page: they wait for it to close.
        if ((sheet === "" || sheet === "notes") && page !== "browser") { infoIndex = 0; sheet = "notes" }
        else pendingNotes = true
    }
    property bool pendingNotes: false
    readonly property var infoButtons: sheet === "quick" && quick ? (quick.added.length === 1 ? [{ key: "a", label: "Turn that off and play" }]
                                                                   : quick.added.length > 1 ? [{ key: "a", label: "Turn off the newest and play" }].concat(quick.worked ? [{ key: "y", label: "Go back to the setup that worked" }] : []) : [])
                                          .concat([{ key: "x", label: "Play without mods" }, { key: "b", label: "Keep my mods" }])
        : sheet === "chain" ? [{ key: "a", label: "Open its page" }, { key: "b", label: "Not now" }]
        : sheet === "nobrowser" ? [{ key: "a", label: "Check again" }, { key: "b", label: "Close" }]
        : sheet === "helper" ? [{ key: "a", label: "Pair" }, { key: "b", label: "Close" }]
        : sheet === "addbrowser" && addMode === "helper" ? [{ key: "a", label: "Set up the helper" }, { key: "x", label: "Steam's browser" }, { key: "b", label: "Close" }]
        : sheet === "addbrowser" ? [{ key: "a", label: "Recheck" }, { key: "x", label: "Steam's browser" }, { key: "b", label: "Close" }]
        // B is Not now (it asks again next time); Not a mod, which is for good,
        // is its own button.
        : sheet === "found" ? (found && found.game ? [{ key: "a", label: "Add it" }] : []).concat([{ key: "x", label: found && found.game ? "Other game" : "Pick the game" }, { key: "y", label: "Not a mod" }, { key: "b", label: "Not now" }])
        : sheet === "msg" && msgData ? msgData.buttons.map(x => ({ key: x.key, label: x.label }))
        : [{ key: "a", label: "OK" }]
    function infoRun(i) {
        const label = infoButtons[i] ? infoButtons[i].label : ""
        note("action", "Sheet " + sheet + ": " + label)
        if (sheet === "msg") {
            const d = msgData, bt = d && d.buttons ? d.buttons[i] : null
            // A button that keeps the sheet (Copy it) runs with the sheet still up.
            if (bt && bt.keep === true) { if (bt.run) bt.run(); return }
            sheet = ""
            msgData = null
            if (bt && bt.run) bt.run()
            Qt.callLater(maybePlace)
            return
        }
        if (sheet === "quick") { quickRun(label); return }
        if (sheet === "chain") {
            const next = chainQueue[0]
            sheet = ""
            if (label === "Open its page" && next) {
                chainQueue = chainQueue.slice(1)
                note("action", "Next requirement: " + next.name)
                if (next.modId > 0 && rules && rules.domain) {
                    // Its mod page first, so the main file is known: then its file page opens straight away, the same
                    // as the first mod (no files tab to pick from).
                    page = "game"; tab = 1; zone = "main"
                    openDetails({ id: String(next.modId), name: next.name, source: "nexus", icon: "" }, modPage)
                    pendingGet = true
                } else openNexus(next.url)
            }
            else { chainQueue = []; note("action", "Requirements: later") }
            return
        }
        if (sheet === "nobrowser") { if (label === "Check again") { Turbine.browserCheck(); toast.info("Looking for a browser...") } else sheet = "" }
        else if (sheet === "helper") { if (label === "Pair") { sheet = ""; pair(false) } else sheet = "" }
        else if (sheet === "addbrowser") {
            if (label === "Recheck") recheckBrowserShortcut()
            else if (label === "Set up the helper") { sheet = ""; setupHelper() }
            else if (label === "Steam's browser") { sheet = ""; if (openUrlNow !== "") steamOpen() }
            else sheet = ""
        }
        else if (sheet === "found") {
            const f = found
            if (!f) { sheet = ""; return }
            if (label === "Other game" || label === "Pick the game") { openPickGame(); return }
            found = null; sheet = ""
            if (label === "Not now") { note("action", "Found a download: not now (asked again next time)"); return }
            Turbine.watchConfirm(f.path, label === "Add it")
        }
        else sheet = ""
    }

    // The status card over other apps: only while Turbine isn't the window in front (the player is in the
    // browser). Game Mode uses the input-less
    // overlay window gamescope paints over the browser; Desktop Mode a small panel kept above the browser.
    readonly property var transfer: status.active ? status
        : dl.active ? { active: true, text: "Downloading " + dl.label, level: "busy", progress: dlPart, game: gameTitle, added: "", done: false, ask: "" }
        : null
    readonly property bool overlayWanted: !shot.on && !app.active && transfer !== null
    onOverlayWantedChanged: showOverlay(overlayWanted, Turbine.gamemode)
    property bool hudMarked: false
    function showOverlay(on, gameMode) {
        if (gameMode) {
            if (on) { Turbine.overlayMark(true); hudMarked = true; hudWin.visible = true }
            else if (hudWin.visible || hudMarked) { hudWin.visible = false; Turbine.overlayMark(false); hudMarked = false }
        } else panelWin.visible = on
        note("overlay", (on ? "Status shown over other apps" : "Status overlay hidden") + (gameMode ? " (Game Mode)" : " (Desktop)"))
    }
    // A result stays 6 seconds, then the card goes.
    Timer { id: statusHide; interval: 6000; onTriggered: Turbine.statusDone() }
    onStatusChanged: if (status.active && status.done && !shot.on) statusHide.restart(); else statusHide.stop()

    // Nexus links (Mod Manager Download): from the command line, or from a later launch through the running Turbine.
    property var handler: ({ state: "unknown" })
    Timer { interval: 300; running: true; repeat: true; onTriggered: { const l = Turbine.takeLink(); if (l !== "") app.linkArrived(l) } }
    function linkArrived(link) {
        // Turbine launched again with no link: this one comes to the front (the second one has closed).
        if (link === "turbine:show") { if (!shot.on) { app.raise(); app.requestActivate() } return }
        // A request from Steam's menu (the Decky plugin) for one game.
        if (link.indexOf("turbine:quick ") === 0) { quickRequest(link, 0); return }
        const r = parse(Turbine.openLink(link), { error: "That Nexus link can't be read." })
        if (!shot.on) { app.raise(); app.requestActivate() }
        if (r.needsKey && browserOpen) {
            note("nxm", "Mod Manager Download without a Nexus key (in Turbine's browser)")
            browserNoKey(r.page, link)
            return
        }
        if (r.needsKey) {
            // No Nexus key: Slow download works instead, and that file goes into this mod's game by itself.
            note("nxm", "Mod Manager Download without a Nexus key")
            if (r.page) Turbine.watchPage(r.page)
            say(r.error, "warn")
            msgTimer.interval = 12000; msgTimer.restart()
            toast.info(r.error)
            return
        }
        if (r.error) { note("nxm", "Link refused: " + r.error); say(r.error, "error"); if (browserOpen) browserSay(r.error, "warn"); return }
        // Every Mod Manager Download link asks first, wherever it came from (Turbine's page, the
        // browser, a second launch, the command line): nothing downloads until the player says Get it.
        if (!(browserOpen && page === "browser")) { const gi = games.findIndex(g => g.id === r.game); if (gi >= 0 && gi !== current) current = gi; page = "game"; zone = "main"; modPage = false }
        nxmAsk(link, r)
    }
    // After the quiet second the ring moves to Get it by itself, so A says yes (B is still Not now).
    // Started from the question's first frame on screen (nxmShown); a press of a direction or a tap stops it.
    Timer { id: nxmRingMove; interval: 1200; onTriggered: if (app.nxmAsking && app.sheet === "pick" && app.menuIndex === 1) { app.menuIndex = 0; app.nxmRingMoved = true; app.note("nxm", "The ring moved to Get it") } }
    property bool nxmRingMoved: false
    property int nxmGets: 0
    // The quiet second and the ring's move count from the first frame that shows the question.
    property real nxmShownAt: 0
    property bool nxmWaitFrame: false
    Timer { id: nxmFrameFallback; interval: 250; onTriggered: if (app.nxmWaitFrame) app.nxmShown("no frame in 250 ms") }
    function nxmShown(how) {
        nxmWaitFrame = false
        if (!nxmAsking) return
        nxmShownAt = Turbine.monoMs()
        nxmRingMoved = false
        nxmRingMove.restart()
        nxmAskFrame = nxmAskState()
        note("nxm", "The question is on screen (" + how + ")")
    }

    // Steam's menu (the Decky plugin, decky/) asks Turbine, open or just started for it, to do one thing for one
    // game: open on it, Report a bug, Mods on or off, or a Play. The same buttons' own steps run, on screen. A request
    // that arrives before the games are read waits for them (up to 10 seconds); a Play waits for the game's mods.
    property var quickWaiting: null
    Timer { id: quickRetry; interval: 500; onTriggered: { const w = app.quickWaiting; app.quickWaiting = null; if (w) app.quickRequest(w.link, w.tries) } }
    function quickRequest(link, tries) {
        const parts = link.split(" ")
        const what = parts[1] || ""
        let id = ""
        try { id = decodeURIComponent(parts[2] || "") } catch (e) { id = "" }
        const i = games.findIndex(g => g.id === id)
        if (i < 0 && tries < 20 && allGames.length === 0) { quickWaiting = { link: link, tries: tries + 1 }; quickRetry.restart(); return }
        note("action", "Steam's menu: " + what + " " + id)
        if (i < 0) { say("Turbine doesn't list that game (it may be hidden in Settings).", "warn"); return }
        if (what === "mods-on" || what === "mods-off") { Turbine.quickMods(id, what === "mods-on"); return }
        if (!shot.on) { app.raise(); app.requestActivate() }
        if (i !== current) current = i
        page = "game"; zone = "main"; modPage = false
        if (sheet !== "fomod") sheet = ""
        if (what === "open") return
        if (what === "report") { openReport(null, "Problem with " + fullGame(games[i].name) + ": ", ""); return }
        // A Play: once this game's mods are read (the seamless check and the check before Play need them).
        if (!modsLoaded && tries < 20) { quickWaiting = { link: link, tries: tries + 1 }; quickRetry.restart(); return }
        if (what === "play") startPlay()
        else if (what === "play-coop") { if (seamlessMod) startCoop(); else toast.problem("Seamless Co-op isn't installed for " + fullGame(games[i].name) + ". Get it from the game menu.") }
        else if (what === "play-plain") Turbine.playOriginal()
    }

    // "Get <mod> (<file>) for <game>?", asked for every Mod Manager Download link from Turbine's page. It opens on
    // Not now and takes no input at all for its first second (a page saying "press A again" can't have the next
    // press answer it); Right (or a tap on Get it) then A gets it. One question at a time: a second link while it
    // is open is dropped. The names come from Nexus a moment later.
    property string nxmAskLink: ""
    property var nxmAskInfo: null
    property string nxmAskMod: ""
    property string nxmAskFile: ""
    readonly property bool nxmAsking: sheet === "pick" && nxmAskLink !== "" && pickBLabel === "Not now"
    // No input for the question's first second on screen; `at`: when the press was made (default: now).
    function nxmAskQuiet(at) { return nxmAsking && (nxmShownAt === 0 || (at === undefined ? Turbine.monoMs() : at) - nxmShownAt < 1000) }
    function nxmAskTitle() {
        const m = /^nxm:\/\/[a-z0-9_-]+\/mods\/([0-9]+)\/files\/([0-9]+)/i.exec(nxmAskLink)
        // "Download SkyUI 5.2SE?" (the file's name, else the mod's, else their numbers); the game is in the rows.
        return "Download " + (nxmAskFile !== "" ? nxmAskFile : nxmAskMod !== "" ? nxmAskMod : "mod " + (m ? m[1] : "") + " (file " + (m ? m[2] : "") + ")") + "?"
    }
    function nxmAsk(link, r) {
        if (nxmAsking) { note("nxm", "Another Mod Manager Download link while asking about one: dropped"); return }
        note("nxm", "Asking before it downloads: a Mod Manager Download link from Turbine's page")
        nxmAskLink = link; nxmAskInfo = r; nxmAskMod = ""; nxmAskFile = ""; nxmAskFirst = null; nxmAskFrame = null; nxmAsks++
        // The same link asked again: its names are known already (the answer only tells when it changes).
        nxmNameGot(parse(Turbine.nxmNameJson, {}), true)
        Turbine.nxmAskName(link)
        openPick(nxmAskTitle(), "Turbine asks before each Mod Manager Download. Highlight Get it, then press A.", [
            { icon: "play", title: "Get it", detail: "Downloads it and adds it to " + (r.name ? fullGame(r.name) : "the game"), aLabel: "Get it", run: () => nxmAskGet() },
            { icon: "remove", title: "Not now", detail: "Nothing is downloaded", aLabel: "Not now", run: () => nxmAskNo() }
        ], "")
        pickBLabel = "Not now"
        menuIndex = 1
        nxmShownAt = 0; nxmWaitFrame = true
        nxmRingMove.stop(); nxmFrameFallback.restart()
        keyRoot.forceActiveFocus()
        // The build's checks read what the question offered the moment it opened (not after the quiet second, when
        // the ring has moved to Get it): its A and B, and whether input was still off. Read here, in the same turn
        // that opens it, never a moment later: a check that looks right after the question opened sees it.
        nxmAskFirst = nxmAskState()
    }
    property var nxmAskFirst: null
    // The same at the first frame that shows it (nxmShown), and how many questions opened.
    property var nxmAskFrame: null
    property int nxmAsks: 0
    function nxmAskState() { const o = binds(); return { a: o.a ? o.a.label : "", b: o.b ? o.b.label : "", quiet: nxmAskQuiet(), ring: menuIndex } }
    // The names of the link's mod and file, when Nexus has said them. They only change the title: the ring and the
    // quiet second stay as they are.
    function nxmNameGot(j, opening) {
        if (!j || j.link !== nxmAskLink || nxmAskLink === "" || (!opening && !nxmAsking)) return
        nxmAskMod = j.name || ""; nxmAskFile = j.file || ""
        if (!opening) pickTitle = nxmAskTitle()
    }
    function nxmAskWords() {
        const f = nxmAskFirst || {}, o = binds()
        return pickTitle + ", at first A " + f.a + ", B " + f.b + (f.quiet ? ", input off" : ", input ON") + "; after the quiet second A " + (o.a ? o.a.label : "") + ", B " + (o.b ? o.b.label : "")
    }
    function nxmAskGet() {
        const link = nxmAskLink, r = nxmAskInfo
        nxmAskLink = ""; nxmAskInfo = null
        if (link === "" || !r) return
        nxmGets++
        note("nxm", "Get it: " + r.name)
        if (browserOpen && page === "browser") { Turbine.nxmDownload(link, r.game); edited(); return }
        const i = games.findIndex(g => g.id === r.game)
        if (i >= 0 && i !== current) current = i
        Turbine.nxmDownload(link, r.game)
        edited()
    }
    function nxmAskNo() {
        if (nxmAskLink !== "") note("nxm", "Not now: nothing downloaded")
        nxmAskLink = ""; nxmAskInfo = null
    }
    Connections {
        target: Turbine
        function onNxmNameJsonChanged() {
            app.nxmNameGot(app.parse(Turbine.nxmNameJson, {}), false)
        }
    }

    // Other versions of the mod on the details page (X).
    property int versionIndex: 0
    readonly property var otherVersions: detailsData && detailsData.versions ? detailsData.versions : []
    function openVersions() {
        if (otherVersions.length === 0) return
        note("action", "Other files of " + detailsData.name)
        versionIndex = 0
        history = history.concat([{ kind: "files" }])
        sheet = "versions"
    }
    function installVersion(i) {
        const v = otherVersions[i]
        if (!v) return
        if (v.installed) { toast.info("That one is already in My mods."); return }
        sheet = ""
        if (detailsData.premium === false && detailsData.page) {
            note("action", "Get it on Nexus: " + v.name + " " + (v.version || ""))
            openNexus(detailsData.page + "&file_id=" + v.id)
            return
        }
        note("action", "Install version " + (v.version || v.name) + " of " + detailsData.name)
        Turbine.installVersion(detailsData.id, detailsData.source || srcNow, v.id)
        edited()
        busyHere = true
    }

    // Profiles > Create a Thunderstore code.
    property var codeData: null

    // Downloads: the status line and the rail's Downloads ring.
    property var dl: ({ active: false })
    readonly property real dlPart: dl.active && dl.total > 0 ? Math.min(1, dl.got / dl.total) : -1

    // Installer choices (FOMOD).
    property var fomod: null

    // Load order (Skyrim).
    property var order: []
    property string orderError: ""
    property var orderMessages: ({})      // LOOT's notes: plugin name lowercased -> [{level, text}]
    property string orderNote: ""         // what the last Sort automatically did
    readonly property var orderPlugin: orderIndex >= 0 && orderIndex < order.length ? order[orderIndex] : null
    property int orderIndex: 0
    property bool orderMoving: false
    property bool orderLand: false
    property var orderSaved: []
    property int orderFrom: -1
    function orderLandOn(i, d) {
        if (i < 0) return -1
        for (let j = i; j >= 0 && j < order.length; j += d) if (!order[j].fixed) return j
        for (let j = i; j >= 0 && j < order.length; j -= d) if (!order[j].fixed) return j
        return d < 0 ? -1 : orderIndex
    }
    function orderAct(b) {
        const p = order[orderIndex]
        if (orderMoving) {
            if (b === "b") { order = orderSaved; orderIndex = orderFrom; orderMoving = false; note("action", "Move cancelled"); return }
            const steps = { up: -1, down: 1, left: -10, right: 10, lt: -100000, rt: 100000 }
            if (steps[b] !== undefined) {
                // Plugins the game always loads stay at the top: a move stops at the first one that can move.
                const first = order.findIndex(x => !x.fixed)
                const to = Math.max(first < 0 ? 0 : first, Math.min(order.length - 1, orderIndex + steps[b]))
                if (to === orderIndex || order[to].fixed) return
                let list = order.slice(); const x = list.splice(orderIndex, 1)[0]; list.splice(to, 0, x)
                order = list; orderIndex = to
            } else if (b === "a" || b === "y") {
                orderMoving = false
                if (orderIndex !== orderFrom && order[orderIndex]) Turbine.orderMove(order[orderIndex].name, orderIndex)
            }
            return
        }
        // The game's own plugins are locked: Up and Down skip them.
        if (b === "up") { let i = orderIndex - 1; while (i >= 0 && order[i].fixed) i--; orderIndex = Math.max(-1, i) }
        else if (b === "down") { let i = orderIndex + 1; while (i < order.length && order[i].fixed) i++; if (i < order.length) orderIndex = i }
        else if (b === "a" && orderIndex === -1) Turbine.orderSort()
        else if (b === "a" && p && !p.fixed) {
            order = order.map((x, i) => i === orderIndex ? Object.assign({}, x, { active: !x.active }) : x)
            Turbine.orderToggle(p.name, !p.active)
        }
        else if (b === "y" && p && !p.fixed) { orderMoving = true; orderFrom = orderIndex; orderSaved = order.slice() }
        else if (b === "x" && p) { note("action", "Plugin details: " + p.name); sheet = "plugin" }
        else if (b === "left") { zone = "rail"; railIndex = current }
    }

    // Profiles.
    property var profileData: null
    property var shares: []
    property int shareIndex: 0
    property var importData: null
    property int profIndex: 0
    readonly property var profileList: profileData ? profileData.profiles : []
    readonly property var profileActions: [
        { key: "new", label: "New loadout", detail: "From the mods as they are now" },
        { key: "export", label: "Save to a loadout file", detail: "Puts \"" + (profileData ? profileData.active : "") + "\" in Downloads to share" },
        { key: "importFile", label: "Open a loadout file", detail: "One someone shared with you, from Downloads" }
    ].concat(rules && rules.community ? [
        { key: "code", label: "Create a Thunderstore code", detail: "Uploads \"" + (profileData ? profileData.active : "") + "\" to Thunderstore and shows a code to share" },
        { key: "importCode", label: "Import a Thunderstore profile code", detail: "A code someone shared with you" }
    ] : [])
    function openProfiles() { profIndex = 0; importData = null; Turbine.profiles(); sheet = "profiles" }
    function profileRun(i) {
        if (importData) {
            if (i === 0 && !importData.error) {
                note("action", "Import profile: get them")
                Turbine.profileImportGo(); sheet = ""
                const steps = account.premium ? (importData.nexusPremium || []) : (importData.nexusFree || [])
                const open = steps.filter(x => x.action === "open").map(x => ({ name: x.mod.name, url: x.url, note: "From the shared loadout" }))
                if (open.length > 0) { importData = null; openPages("Open these pages for the loadout", open); return }
            }
            importData = null
            if (sheet === "profiles") profIndex = 0
            return
        }
        if (i < profileList.length) {
            const n = profileList[i].name
            if (profileData && n === profileData.active) return
            note("action", "Switch profile to " + n)
            Turbine.profileSwitch(n)
            sheet = ""
            return
        }
        const a = profileActions[i - profileList.length]
        if (!a) return
        note("action", "Profiles: " + a.label)
        if (a.key === "new") openAsk("newProfile", "New loadout", "A name for the mods as they are now.", "", "Loadout name", 60, false, "Make it")
        else if (a.key === "export" && profileData) Turbine.profileExport(profileData.active)
        else if (a.key === "code" && profileData) { Turbine.profileCode(profileData.active); sheet = "code" }
        else if (a.key === "importFile") { Turbine.listShares(); shareIndex = 0; sharesBack = "profiles"; sheet = "shares" }
        else if (a.key === "importCode") openAsk("code", "Thunderstore profile code", "The code looks like 0190a2b4-7c1d-4e8f-9a0b-1c2d3e4f5a6b.", "", "Profile code", 40, false, "Read it")
    }

    // Mod updates: badges, the mod menu and Update all.
    property var updatesData: null
    readonly property var updateMap: {
        let m = {}
        if (updatesData && game && updatesData.game === game.id) for (const u of updatesData.items || []) m[u.mod] = u
        return m
    }
    readonly property int updateCount: Object.keys(updateMap).length
    property var updatesChecked: ({})
    function maybeCheckUpdates() {
        if (!game || updatesChecked[game.id] || (shot.on && shot.page !== "getmods")) return
        const any = mods.some(m => m.source === "Thunderstore" || m.source === "Nexus Mods")
        if (!any) return
        let c = Object.assign({}, updatesChecked); c[game.id] = true; updatesChecked = c
        Turbine.checkUpdates()
    }

    // Accounts and Turbine's own updates.
    property var account: ({ state: "none" })
    property var selfUpd: ({ state: "idle" })
    property bool selfAuto: false
    property bool backToStable: false

    // A line of text (search, profile name, code, Nexus key), typed with Steam's keyboard in Game Mode.
    property string askPurpose: ""
    property string askReturn: ""
    property int askIndex: -1            // -1 the field, then Clear, Cancel, Done
    function askKeyboard() { note("action", "Text entry: keyboard"); if (Turbine.gamemode && !shot.on) Qt.openUrlExternally("steam://open/keyboard") }
    function askRun(i) {
        if (i === 0) { askView.clear(); askIndex = -1 }
        else if (i === 1) askView.cancel()
        else askView.accept()
    }
    function openAsk(purpose, title, noteText, start, placeholder, max, secret, done) {
        askIndex = -1
        askPurpose = purpose
        askReturn = sheet
        askView.open(title, noteText, start, placeholder, max, secret, done)
        sheet = "ask"
        reportEditing(true)
    }
    function askDone(text) {
        reportEditing(false)
        sheet = askReturn === "ask" ? "" : askReturn
        if (askPurpose === "search") { getText = text; getArea = "grid"; getIndex = 0; note("action", "Get Mods search"); browseNow() }
        else if (askPurpose === "nexusKey" && text !== "") Turbine.setNexusKey(text)
        else if (askPurpose === "newProfile" && text !== "") Turbine.profileCreate(text)
        else if (askPurpose === "putafter") putAfter(text)
        else if (askPurpose === "settingvalue" && settingsTypeRow) settingsTyped(settingsTypeRow, text)
        else if (askPurpose === "findplugin" && text !== "") findPlugin(text)
        else if (askPurpose === "popfind") { findPop = Object.assign({}, findPop, { text: text }); findRow = 0; note("action", "Find pop-up: words \"" + text + "\"") }
        else if (askPurpose === "find") { findText = text; modIndex = 0; note("action", "Find \"" + text + "\": " + shownMods.length + " shown") }
        else if (askPurpose.indexOf("renameProfile:") === 0 && text !== "" && text !== askPurpose.substring(14)) Turbine.profileRename(askPurpose.substring(14), text)
        else if (askPurpose === "code" && text !== "") Turbine.profileImportCode(text)
        else if (askPurpose === "placefind") { placeFilter = text; if (placePane === 0) placeGameIndex = 0; else placeModIndex = 0 }
        else if (askPurpose === "placefolder" && text !== "") placeNewFolder(text)
        else if (askPurpose === "rename" && renameTarget) { const m = renameTarget; renameTarget = null; renameMod(m, text) }
        else if (askPurpose === "nexusgame" && text.trim() !== "") { note("action", "Search Nexus games: " + text); Turbine.searchNexusGames(text) }
        else if (askPurpose === "nexussearch" && text !== "" && nexusView) nexusView.load(browserAddress("https://www.nexusmods.com/games/" + ((rules && rules.domain) || "") + "/mods?keyword=" + encodeURIComponent(text)))
    }
    function askCancelled() {
        reportEditing(false)
        if (askPurpose === "find") findText = findBefore
        sheet = askReturn === "ask" ? "" : askReturn
    }

    // ---------------------------------------------------------------- Labs
    readonly property var settingsSections: ["Getting mods", "Look", "Updates", "Help", "About"].concat(labsOn ? ["Labs"] : [])
    readonly property string sec: settingsSections[setSection] || "Getting mods"
    // With a newer Turbine ready, Settings opens on Updates.
    function openSettings() {
        // B comes back here.
        // Where B goes back to is taken every time Settings opens, never left over from before.
        if (page !== "settings") settingsFrom = { zone: zone, tab: tab, modPage: modPage, railIndex: railIndex, browser: page === "browser" }
        page = "settings"; zone = "settings"
        if (selfUpd.state === "available" && settingsSections.indexOf("Updates") >= 0) setSection = settingsSections.indexOf("Updates")
        setRow = firstSettingsRow()
        if (sec === "Look") checkDisplays()
    }
    function openSection(name) { openSettings(); setSection = Math.max(0, settingsSections.indexOf(name)); setRow = firstSettingsRow() }
    onLabsOnChanged: if (!labsOn && setSection >= settingsSections.length) { setSection = settingsSections.indexOf("About"); setRow = 0 }
    // Getting mods, Updates, Labs and Help: rows of one kind each. action (A does it), toggle and choice (Left, Right or A
    // change it), item (a Labs checklist item), info (words only).
    readonly property var rowsNow: sec === "Getting mods" ? accountRows : sec === "Updates" ? updateRows : sec === "Labs" ? labsRows : sec === "Help" ? helpRows : []
    // Help: the buttons in one place, the beginner tutorial again, and games hidden from the rail.
    readonly property var helpRows: [
        { kind: "action", key: "tutorial", title: "Beginner tutorial", how: "Five cards: the buttons, the wheel, getting a mod, START, and what Turbine sets up for you", action: "Show it" },
        { kind: "action", key: "launchundo", title: "Undo the launch options change", how: "For games Turbine starts through Steam with Mod Engine 3: puts Steam's Launch Options back as they were", action: "Show" },
        { kind: "action", key: "problem", title: "Report a problem", how: "Saves a report file in Downloads to send: what Turbine did lately, its settings and your games. No keys or passwords.", action: "Save report" },
        { kind: "action", key: "decky", title: "Steam menu plugin (Decky)", how: deckyWords, action: decky.plugin === true ? "How to use it" : "Show how" },
        { kind: "info", key: "buttons", title: "The buttons",
          how: "A: do it. B: back. X: options. Y: find and sort. LB RB: tabs. LT RT: page up or down. VIEW: the wheel. START: Settings (hold 3 seconds: Play options, with Play Seamless Co-op when it's installed). Right stick: pointer, RT clicks. R3: the same as A; while a message offers a step (Undo), R3 does that step." },
        { kind: "info", key: "keys", title: "Keyboard and mouse",
          how: "Move: W A S D or the arrows. Enter or Space: do it. Esc or Backspace: back (Esc on a game's first page: Settings). Q and E: the last and the next page (Tab too). F: find. R or a right-click: options. Hold Tab: the wheel (pick with the mouse or W A S D, let go to choose). Hold Space for 3 seconds: Play. A double-click opens a mod. The mouse wheel scrolls; Ctrl and the mouse wheel, or Ctrl+= and Ctrl+-, change the size. F1: Help. The mouse's Back button goes back and its Forward button turns the page." },
        { kind: "info", key: "touch", title: "Touch",
          how: "Tap does it. Hold for options (Move is there: then tap where the mod goes). Swipe sideways on the bottom bar to switch tabs." }
    ].concat(steamArt.found ? [
        { kind: "action", key: "steamart", title: "Steam library art", how: steamArt.on ? "Turbine's pictures in your Steam library (your own pictures are never replaced)" : "Off: Steam shows its own pictures for Turbine", action: steamArt.on ? "Remove" : "Add" }
    ] : []).concat(hiddenIds.length > 0 ? [
        { kind: "action", key: "unhide", title: "Games hidden from the rail: " + hiddenIds.length, how: "Puts them back on the rail", action: "Show them" }
    ] : []).concat(unseenFolders.map(f => (
        // A Steam library Turbine isn't allowed to read, with the one press that allows it.
        { kind: "action", key: "allowfolder", folder: f, title: "Turbine can't see " + f, how: "Your games there don't show. Allow lets Turbine read that folder (it restarts once).", action: "Allow" })))
    // Steam libraries the scan found but isn't allowed to read (engine steam.rs, UNSEEN_LIBRARY).
    readonly property var unseenFolders: String(Turbine.problems || "").split("\n").map(l => (l.match(/^Can't see the Steam library at (.*) \(Turbine isn't allowed to read that folder\)$/) || [])[1]).filter(x => !!x)
    // The optional Steam menu plugin (decky/): Turbine's panel in the Quick Access menu, through Decky Loader.
    // Whether Decky and the plugin are on this device is read on the host when Help opens and when the row is pressed.
    property var decky: ({ checked: false })
    function deckyCheck() { if (!shot.on) decky = parse(Turbine.deckyJson(), { checked: true, decky: null, plugin: false, url: "" }) }
    readonly property string deckyWords: decky.plugin === true ? "Installed. In a game: the Quick Access button, Decky, then Turbine."
        : decky.decky === true ? "Decky is on this device: one step adds Turbine to the Quick Access menu."
        : decky.decky === false ? "Optional, for the Quick Access menu in a game. It needs Decky Loader, which isn't on this device yet."
        : "Optional: Turbine in the Quick Access menu in a game. Mods on or off and Play, without opening Turbine."
    function openDecky() {
        deckyCheck()
        const url = decky.url || "https://github.com/Ravenor907/Turbine-Mod-Manager/releases/latest/download/Turbine-Decky.zip"
        note("action", "Steam menu plugin: Decky " + decky.decky + ", plugin " + decky.plugin)
        if (decky.plugin === true) {
            openMsg({ title: "Turbine is in the Quick Access menu",
                      text: "In a game, press the Quick Access button, open Decky (the plug), then Turbine. It shows that game's mods: Mods on or off, Play with mods, Play Seamless Co-op when it's installed, Play without mods, Open Turbine on the game, and how the last launch went.",
                      buttons: [{ key: "b", label: "Close", run: () => {} }] })
            return
        }
        if (decky.decky === true) {
            openMsg({ title: "Add Turbine to the Quick Access menu",
                      text: "Decky is on this device, so it's one step. Decky gets Turbine's plugin from this address:\n\n" + url,
                      steps: ["Quick Access button, Decky (the plug), its gear, Developer, Install Plugin From URL: type or paste the address above, then Install. (No Developer tab? In General, turn on Developer Mode first.)"],
                      buttons: [{ key: "a", label: "Copy the address", keep: true, run: () => { Turbine.copyText(url); note("action", "Steam menu plugin: address copied"); toast.done("Copied. The step stays here until B.") } },
                                { key: "b", label: "Close", run: () => {} }] })
            return
        }
        openMsg({ title: "Steam menu plugin (optional)",
                  text: "With Decky Loader, Turbine gets a panel in the Quick Access menu: Mods on or off and Play with or without mods for the game you're in, without opening Turbine. Turbine works the same without it." + (decky.decky === false ? " Decky isn't on this device yet." : ""),
                  steps: ["Switch to Desktop Mode: the Steam button, Power, Switch to Desktop.",
                          "In a browser, open decky.xyz, download the installer, run it and pick the release version.",
                          "Back in Game Mode, open this row again: it shows the one step left."],
                  buttons: [{ key: "b", label: "Close", run: () => {} }] })
    }
    // The Nexus key's rows show folded until asked for (or while a key is set, or the key is being checked).
    property bool keyRowsOpen: false
    // Settings, Getting mods: the rows for the player's own browser are folded until asked for (open in Desktop Mode
    // when he chose his own browser, and in the build's pictures of them).
    property bool ownRowsOpen: false
    readonly property bool ownRowsShown: ownRowsOpen || (!Turbine.gamemode && browserSet.openIn === "mine") || bridge.outdated === true
                                         || ["browser", "helper", "accounts", "nobrowser"].indexOf(shot.page) >= 0
    readonly property bool keyRowsShown: keyRowsOpen || account.hasKey === true || account.state === "checking" || account.state === "error" || shot.page === "accounts"
    readonly property string accountWords: account.state === "checking" ? "Checking the key with Nexus Mods..."
        : account.state === "ok" ? "Key works: " + account.name + (account.premium ? " · Premium: installs in one press with no browser" : " · Free account: Get it on Nexus opens the file's page")
        : account.state === "error" ? account.message
        : account.message ? account.message + " " + keyWhere : keyWhere
    // Where the Nexus key is, exactly (the Request Api Key buttons on that page are for other apps).
    readonly property string keyWhere: "On nexusmods.com, open Site preferences, then API Keys. Scroll to the bottom to Personal API Key and copy it. Don't use the Request Api Key buttons: those are for other apps."
    readonly property string keyPageUrl: "https://www.nexusmods.com/settings/api-keys"
    // Paste: the key straight from the clipboard, checked with Nexus, in one press.
    function pasteKey() {
        const t = shot.on ? "" : Turbine.clipboardText().trim()
        if (t.length >= 16 && t.length <= 1024 && !/\s/.test(t)) { note("action", "Paste key: checking it with Nexus"); Turbine.setNexusKey(t); toast.info("Checking the key with Nexus Mods...") }
        else toast.info(t === "" ? "Nothing to paste yet. Copy your Personal API Key on nexusmods.com first, then press Paste." : "That isn't a key. Copy your Personal API Key on nexusmods.com, then press Paste.")
    }
    // Getting mods: first a verdict, not a setting; then what is optional.
    readonly property var accountRows: [
        // First, whether you're signed in to Nexus, and the one button for it.
        { kind: "action", key: "nexusSign", title: "Nexus Mods: " + (browserSignedIn ? "Signed in" : "Not signed in"),
          how: browserSignedIn ? "Get it on Nexus opens the file's page here and adds the download automatically."
                               : account.state === "ok" ? "Your Nexus key works (" + account.name + "), but Turbine's Nexus pages aren't signed in. Sign in (free) to use Slow download there."
                               : "Not needed to look around. Sign in (free) when you get your first mod.",
          action: browserSignedIn ? "Sign out" : "Sign in" }
    ].concat(!Turbine.gamemode || shot.page === "accounts" ? [
        { kind: "choice", key: "openIn", title: "Open Nexus in",
          how: browserSet.openIn === "mine" ? "Your own browser. Turbine watches Downloads and adds the file when it lands." : "Turbine itself: sign in once, press Slow download, and the mod is added. (Desktop Mode only; Game Mode always uses Turbine.)",
          value: browserSet.openIn, options: ["turbine", "mine"], names: ["Turbine", "My browser"] }
    ] : []).concat(bridge.paired === true ? [
        { kind: "action", key: "signinMine", title: "Google sign-in",
          how: "Signs in on Nexus in Chrome, then Turbine takes the sign-in from there. Email and password also work right in Turbine's page.",
          action: "Sign in" }
    ] : []).concat([
        // A stepper like Interface size: the size in the title, Left and Right in steps of 10.
        { kind: "choice", key: "zoom", title: "Nexus page size: " + Math.round(browserZoom * 100) + "%", how: "How large Nexus pages show in Turbine. Left and Right change it (also on the page: X, Zoom)",
          value: "", options: ["-", "+"], names: ["-10%", "+10%"] }
    ]).concat(keyRowsShown ? [
        { kind: "action", key: "pasteKey", head: "Faster downloads (optional)", title: "Nexus key", how: accountWords, action: account.hasKey ? "Paste a new key" : "Paste" },
        { kind: "action", key: "keyPage", title: "Open the API Keys page", how: "Opens " + keyPageUrl.replace("https://", "") + " in the browser. Sign in first if Nexus asks.", action: "Open" },
        { kind: "action", key: "nexusKey", title: "Type the key instead", how: "With Steam's keyboard, if pasting isn't possible", action: "Type" },
        { kind: "info", key: "keyWhat", title: "What a key does",
          how: "Free key: mod details, file lists and update checks. Premium key: also one-press downloads straight from Turbine. Without Premium, Get it on Nexus opens the file's page and Turbine adds the download." }
    ] : [
        // Folded: one row, so the words a beginner doesn't need (API key, Premium) stay out of the way.
        { kind: "action", key: "keyFold", title: "Faster downloads (optional)", how: "A Nexus key: Premium installs in one press.", action: "Show" }
    ]).concat(account.hasKey ? [{ kind: "action", key: "signOut", title: "Remove the key", how: "Deletes the key from this device. You stay signed in on Nexus pages.", action: "Remove" }] : []).concat(ownRowsShown || (Turbine.gamemode && shot.page !== "accounts") ? [] : [
        // Folded: the rows for the player's own browser (Desktop Mode) stay out of the way.
        { kind: "action", key: "ownFold", title: "Using your own browser (Desktop Mode)",
          how: "Not needed in Game Mode: Nexus opens right in Turbine. These are for Chrome or Firefox in Desktop Mode.", action: "Show" }
    ]).concat(!ownRowsShown ? [] : [
        // The helper stays only for signing in (Google blocks sign-in inside Turbine's page).
        { kind: "action", key: "helper", title: "Browser helper (for signing in)", action: "Set up",
          how: "For Chrome: brings a Google (or other) sign-in on Nexus back to Turbine." }
    ]).concat(ownRowsShown && bridge.outdated ? [
        { kind: "info", key: "helperOld", title: "Your browser helper is out of date" + (bridge.helper ? " (" + bridge.helper + ", Turbine has " + bridge.bundled + ")" : ""),
          how: "Turbine put the new version in its folder and asks it to reload when it connects." }
    ] : []).concat(ownRowsShown && (bridge.paired || bridge.connected > 0 || shot.page === "browser") ? [
        { kind: "action", key: "pair", title: bridgeTitle, how: bridgeWords, action: bridge.paired ? "Pair again" : "Pair" }
    ] : []).concat(!ownRowsShown ? [] : [
        { kind: "action", key: "browser", title: "Web browser", how: browserWords, action: browserFound ? "Check again" : "How to get one" }
    ])
    readonly property string selfWords: selfUpd.state === "checking" ? "Looking for a newer Turbine..."
        : selfUpd.state === "available" ? "Turbine " + selfUpd.version + (selfUpd.prerelease ? " (pre-release)" : "") + " is ready to install"
        : selfUpd.state === "stable" ? "The latest stable Turbine is " + selfUpd.version
        : selfUpd.state === "current" ? "Turbine " + Turbine.version + " is the newest" + (labsOn && updates.prerelease ? ", pre-releases included" : "")
        : selfUpd.state === "installing" ? "Downloading and installing Turbine " + selfUpd.version + "..."
        : selfUpd.state === "ready" ? "Turbine " + selfUpd.version + " is installed. Restart to use it."
        : selfUpd.state === "error" ? selfUpd.message
        : "This is Turbine " + Turbine.version
    readonly property var updateRows: [
        { kind: "action", key: "checkNow", title: "Check for updates", how: selfWords, action: selfUpd.state === "checking" ? "" : "Check now" }
    ].concat(selfUpd.state === "available" || selfUpd.state === "stable" ? [
        { kind: "action", key: "install", title: "Install Turbine " + selfUpd.version, how: (selfUpd.notes || "").split("\n")[0] || "Downloads it, checks it, then installs it", action: "Install" }
    ] : []).concat(selfUpd.state === "ready" ? [
        // In Game Mode only what Steam starts shows, so Turbine closes and Steam starts it again.
        Turbine.gamemode ? { kind: "action", key: "restart", title: "Close Turbine", how: "Then start it again from Steam to use the new version", action: "Close" }
                         : { kind: "action", key: "restart", title: "Restart Turbine", how: "Closes Turbine and opens the new version", action: "Restart" }
    ] : []).concat([
        { kind: "toggle", key: "auto", title: "Check when Turbine starts", how: "Checks at startup, at most every 6 h. Nothing installs without you.", value: updates.auto, options: [false, true], names: ["Off", "On"] },
        { kind: "info", key: "channel", title: "Update channel: " + (labsOn && updates.prerelease ? "Pre-release" : "Stable"), how: labsOn ? "Change it in Labs" : "Stable releases only" }
    ])
    function rowStep(r, d) {
        if (r.key === "auto") updates.auto = !updates.auto
        else if (r.key === "openIn") { if (!shot.on) browserSet.openIn = browserSet.openIn === "mine" ? "turbine" : "mine" }
        else if (r.key === "zoom") { browserZoomBy(d * 0.1); if (!shot.on) browserSet.zoom = browserZoom }
        else if (r.key === "noGpu") { webNoGpu = !webNoGpu; Turbine.setWebNoGpu(webNoGpu); toast.info("Takes effect the next time Turbine starts.") }
        else if (r.key === "protonLog") { protonLog = !protonLog; Turbine.setProtonLog(protonLog); toast.info(protonLog ? "On: the next Steam launch writes Proton's log. Report a problem includes it." : "Off.") }
        else if (r.key === "channel") { updates.prerelease = !updates.prerelease; Turbine.selfCheck(updates.prerelease, false) }
        note("action", "Setting " + r.key + " changed")
    }
    // Every Settings action row's key has its handler here (the build checks that each row in every section has
    // one, so a row can't silently do nothing).
    readonly property var rowActions: ({
        report: () => openReport(null, "", ""),
        perf: () => openPerf(),
        scrolltest: () => { page = "perf"; zone = "perf"; perfIndex = 0 },
        activity: () => openActivity(),
        problem: () => saveProblemReport(),
        decky: () => openDecky(),
        launchundo: () => openLaunchUndo(false),
        steamart: () => { const w = Turbine.steamArtSet(!steamArt.on); steamArt = parse(Turbine.steamArtJson(), { found: false, on: true }); note("action", "Steam library art: " + (steamArt.on ? "added" : "removed")); toast.info(w) },
        steamBrowser: (r) => labsRun(r), keyless: (r) => labsRun(r),
        nexusKey: () => openAsk("nexusKey", "Nexus key", keyWhere + " It's kept in a file only you can read.", "", "Nexus key", 1024, true, "Check it"),
        pasteKey: () => pasteKey(),
        keyFold: () => { keyRowsOpen = true; setRow = accountRows.findIndex(x => x.key === "pasteKey") },
        ownFold: () => { ownRowsOpen = true; setRow = accountRows.findIndex(x => x.key === "helper") },
        keyPage: () => { if (openNexus(keyPageUrl)) toast.info("Opened API Keys. Copy the Personal API Key at the bottom, come back and press Paste.") },
        signOut: () => Turbine.nexusSignOut(),
        nexusSign: () => {
            if (browserSignedIn) { Turbine.browserSignOut(); browserSignedIn = false; toast.done("Signed out of Nexus in Turbine. Your key, if you have one, stays.") }
            else if (openNexus("https://users.nexusmods.com/auth/sign_in")) toast.info("Sign in on the page. Turbine keeps you signed in.")
        },
        signinMine: () => signinStart(),
        tutorial: () => openTutorial(0),
        unhide: () => { if (!shot.on) railSet.hidden = "[]"; orderGames(); toast.done("Every game is back on the rail.") },
        allowfolder: r => {
            const why = shot.on ? "" : Turbine.allowFolder(r.folder)
            if (why !== "") { toast.problem(why); return }
            openMsg({ title: "Turbine can read " + r.folder + " now", text: "It sees the games there after a restart.",
                      buttons: [{ key: "a", label: "Restart Turbine", run: () => { Turbine.restart(); Qt.quit() } }, { key: "b", label: "Not now", run: () => {} }] })
        },
        addChrome: () => showAddBrowser("noshortcut"),
        browser: () => { if (browserFound) Turbine.browserCheck(); else noBrowser() },
        helper: () => setupHelper(),
        pair: () => pair(bridge.paired),
        checkNow: () => Turbine.selfCheck(labsOn && updates.prerelease, false),
        install: () => Turbine.selfInstall(),
        restart: () => { Turbine.finish(); if (!Turbine.gamemode) Turbine.restart(); else note("action", "Close Turbine for the update (Game Mode: start it again from Steam)"); Qt.quit() },
        stable: () => { backToStable = true; Turbine.selfCheck(false, true); openSection("Updates") }
    })
    function rowRun(r) {
        note("action", "Settings: " + r.title)
        const f = rowActions[r.key]
        if (f) f(r)
        else { note("action", "Settings row with no action: " + r.key); toast.problem("This row has nothing to do yet.") }
    }
    // Settings > Report a problem: one text file in Downloads; where it went is shown.
    function saveProblemReport() {
        const lines = [
            "Interface size: " + sizeNow + "%", "Layout: " + layoutNow, "Theme: " + look.theme, "Accent: " + look.accent,
            "Contrast: " + (look.highContrast ? "High" : "Standard"), "Color vision: " + look.vision, "Compact list: " + look.compactList,
            "Animations: " + mo.level, "Confirm by: " + look.confirmBy, "Seasonal: " + seasonPick,
            "Open Nexus in: " + browserSet.openIn, "Status over other apps: true",
            "Check for updates at start: " + updates.auto, "Pre-release: " + (labsOn && updates.prerelease), "Labs: " + labsOn,
            "Nexus account: " + (account.state || "unknown") + (account.hasKey ? ", a key is set" : ", no key") + (account.premium === true ? ", Premium" : ""),
            "Last input: " + lastInput, "Controller connected: " + Turbine.pads ]
        const screen = width + " x " + height + " (design " + Math.round(designWidth) + " wide, scale " + s.toFixed(2) + ")"
        const r = parse(Turbine.saveProblemReport(screen, lines.join("\n")), { error: "Couldn't save it." })
        note("action", "Report a problem: " + (r.error ? r.error : r.file))
        if (r.error) { toast.problem(r.error); return }
        openMsg({ title: "Report saved", text: "Saved as " + r.file + " in " + r.folder + ". Send that file with a few words about what happened. It has no keys, tokens or cookies.",
                  buttons: [{ key: "a", label: "OK", run: () => {} }] })
    }
    // Ten presses on the icon in Settings > About, a few seconds apart at most, turn Labs mode on or off.
    property int labsTaps: 0
    Timer { id: labsTapReset; interval: 3000; onTriggered: app.labsTaps = 0 }
    function labsTap() {
        labsTaps++
        labsTapReset.restart()
        if (labsTaps < 10) return
        labsTaps = 0
        labsSet.on = !labsSet.on
        note("labs", labsSet.on ? "Labs mode on" : "Labs mode off")
        toast.info(labsSet.on ? "Labs mode on. Labs is at the end of Settings." : "Labs mode off")
    }
    // Things to confirm on the device: what the build server can't check (LabsChecklist.qml).
    LabsChecklist { id: labsChecklist }
    readonly property var labsItems: labsChecklist.items
    readonly property var labsRows: [
        { kind: "action", key: "report", title: "Report a bug", how: "Screenshot, what went wrong, and details Turbine adds automatically", action: "Report" },
        { kind: "action", key: "perf", title: "Performance check", how: "Frame times, processor, startup, controller", action: "Open" },
        { kind: "action", key: "scrolltest", title: "Scrolling test", how: "2,000 mod rows, to feel how smoothly long lists scroll", action: "Open" },
        { kind: "choice", key: "channel", title: "Update channel", how: "Pre-release gets test builds first", value: updates.prerelease, options: [false, true], names: ["Stable", "Pre-release"] },
        { kind: "action", key: "keyless", title: "Nexus without a key", how: labsResult ? labsResult.text : "Asks Nexus for this game's mods with no key and says what came back", action: "Check" },
        { kind: "action", key: "steamBrowser", title: "Open Nexus in Steam's browser", how: "For the Steam browser check below: opens nexusmods.com in Steam and watches Downloads for the game on screen", action: "Open" },
        { kind: "action", key: "activity", title: "Activity log", how: "The last 500 lines of what Turbine did, to read, copy or save to Downloads", action: "Open" },
        { kind: "toggle", key: "noGpu", title: "Nexus page: draw without the GPU", how: "For a black or frozen Nexus page in Turbine. Takes effect the next time Turbine starts.",
          value: webNoGpu, options: [false, true], names: ["Off", "On"] },
        { kind: "toggle", key: "protonLog", title: "Proton log for Mod Engine 3 games", how: "Proton writes its own log for the next Steam launches of Elden Ring and the other Mod Engine 3 games. Report a problem includes it.",
          value: protonLog, options: [false, true], names: ["Off", "On"] },
        { kind: "action", key: "addChrome", title: "Use Chrome in Game Mode", how: "How to add Chrome to Steam once (for signing in with Google)", action: "Show how" }
    ].concat(updates.prerelease ? [
        { kind: "action", key: "stable", title: "Go back to stable", how: "Installs the latest stable Turbine, even if it's older, and turns Pre-release off", action: "Go back" }
    ] : []).concat(labsItems.map((x, i) => Object.assign({ kind: "item", head: i === 0 ? "To confirm on your device" : "" }, x)))
    property var labsAnswers: parse(labsSet.states, {})
    property bool webNoGpu: Turbine.webNoGpu()
    property bool protonLog: Turbine.protonLogOn()
    property var steamArt: parse(Turbine.steamArtJson(), { found: false, on: true })
    readonly property string labsDone: {
        const n = labsItems.filter(x => labsAnswers[x.id] === "works").length
        const p = labsItems.filter(x => labsAnswers[x.id] === "problem").length
        return n + " of " + labsItems.length + " work" + (p > 0 ? ", " + p + " with a problem" : "")
    }
    function labsState(id) { return labsAnswers[id] || "" }
    // Where Left and Right are on a checklist row: 0 Works, 1 Problem, 2 Not tested.
    property int labsChip: 0
    function labsFocusRow() {
        const r = labsRows[setRow]
        if (r && r.kind === "item") { const st = labsState(r.id); labsChip = st === "works" ? 0 : st === "problem" ? 1 : 2 }
    }
    onSetRowChanged: if (sec === "Labs") labsFocusRow()
    property var labsResult: null
    function labsChoose(item, st, quiet) {
        if (!item) return
        let a = Object.assign({}, labsAnswers)
        if (st === "") delete a[item.id]
        else a[item.id] = st
        labsAnswers = a
        if (!shot.on) labsSet.states = JSON.stringify(a)
        note("labs", "Checklist: " + item.title + " = " + (st || "not tested"))
        if (st === "problem" && !quiet) openReport(item, "", "")
    }
    function labsRun(row) {
        note("action", "Labs: " + row.title)
        if (row.key === "keyless") { Turbine.nexusKeylessCheck(); toast.info("Asking Nexus...") }
        else if (row.key === "steamBrowser") {
            const url = Turbine.watchSteamBrowser()
            if (!shot.on) Qt.openUrlExternally(url)
            toast.info("Opened Nexus Mods in Steam's browser. Try a Slow download; Turbine asks before adding what it finds.")
        }
    }
    // Labs > Activity log: the last 500 lines in a sheet.
    property var activityLines: []
    function openActivity() {
        activityLines = Turbine.activityLog(500).split("\n").filter(l => l !== "")
        note("labs", "Activity log open: " + activityLines.length + " lines")
        sheet = "activity"
    }
    function labsItemText(item) {
        if (!item) return ""
        return "Item: " + item.title + "\nHow to try it: " + item.how + "\nWhat should happen: " + item.expect
               + "\nChecklist answer: " + (labsState(item.id) || "not tested")
    }

    // What the interface looks like right now, for reports.
    function focusWords() {
        if (sheet === "modmenu") return "Mod menu item " + (modMenuItems[menuIndex] ? modMenuItems[menuIndex].title : menuIndex)
        if (sheet === "gamemenu") return "Game menu item " + (gameMenuItems[menuIndex] ? gameMenuItems[menuIndex].title : menuIndex)
        if (sheet === "add") return "Add mods row " + (addList[addIndex] ? addList[addIndex].name : addIndex)
        if (sheet === "check") return "Check before Play button " + (checkChoices[checkIndex] || checkIndex)
        if (sheet !== "") return "Sheet " + sheet
        if (zone === "rail") return "Rail item " + railIndex + (railIndex < games.length ? " (" + games[railIndex].name + ")" : "")
        if (page === "settings") return "Settings " + (settingsSections[setSection] || setSection) + " row " + setRow
               + (sec === "Look" && lookRows[setRow] ? " (" + lookRows[setRow].label + ")" : rowsNow[setRow] ? " (" + rowsNow[setRow].title + ")" : "")
        if (page === "perf") return "Scrolling test row " + perfIndex
        if (tab === 0 && focusedItem) return (focusedItem.kind === "mod" ? "Mod row " : "Removed row ") + (modIndex + 1) + " (" + focusedItem.m.name + ")"
        return "Tab " + (tabs[tab] || tab)
    }
    function interfaceText() {
        return [
            "Page: " + page + (sheet !== "" ? ", sheet " + sheet : "") + ", tab " + (tabs[tab] || tab) + (moving ? ", moving a mod" : ""),
            "Focus: " + focusWords() + " (zone " + zone + ")",
            "Window: " + width + " x " + height + (visibility === Window.FullScreen ? ", full screen" : ", windowed") + (active ? ", in front" : ", in the background"),
            "Screen: " + Screen.width + " x " + Screen.height + " px, pixel ratio " + Screen.devicePixelRatio.toFixed(2),
            "Scale: fit " + fit.toFixed(2) + "x, interface size " + sizeNow + "%, everything " + s.toFixed(2) + "x, layout " + layoutNow,
            "Look: theme " + look.theme + ", accent " + look.accent + ", contrast " + (look.highContrast ? "high" : "standard")
                + ", color vision " + look.vision + ", seasonal " + seasonPick + (seasonKind !== "" ? " (" + seasonKind + ", amount " + look.fallAmount + ", size " + look.fallSize + "%)" : ""),
            "Status line: " + statusNow.words + (statusNow.detail ? " · " + statusNow.detail : ""),
            "Waiting changes: " + Turbine.pending + ", busy " + (Turbine.busy ? "yes" : "no"),
            "Labs mode: " + (labsOn ? "on" : "off")
        ].join("\n")
    }

    // Report a bug: the screenshot is taken first, so the report screen isn't in it.
    property string reportContext: ""
    property var reportItem: null
    property bool discardArmed: false
    Timer { id: discardReset; interval: 3000; onTriggered: app.discardArmed = false }
    function openReport(item, startText, perfText) {
        reportContext = interfaceText()
        reportItem = item
        const tmp = Turbine.shotTemp()
        const open = function (ok) {
            reportView.shotUrl = ok ? fileUrl(tmp) + "?" + Date.now() : ""
            reportView.aboutLine = "Save report saves it to Downloads and copies it. It includes Turbine " + Turbine.version
                + ", this device, the game, its mods and the last 300 lines of the activity log."
            // A draft B kept comes back (unless this report starts with words of its own).
            const draft = reportDraft !== "" && startText === "" && !perfText ? reportDraft : ""
            reportView.open(item ? item.title : "", draft !== "" ? draft : startText !== "" ? startText : item ? "Problem: " + item.title + ". " : "",
                            perfText, Turbine.hasCrashLog())
            if (draft !== "") { reportView.focusAt = 2; toast.info("Your draft is back. A saves it.") }
            perfView.visible = false
            sheet = "report"
        }
        if (!app.contentItem.grabToImage(function (r) { open(r.saveToFile(tmp)) })) open(false)
    }
    function sendReport() {
        if (reportView.text.trim() === "") { toast.info("Say what went wrong first."); return }
        const stamp = Turbine.stamp()
        const withShot = reportView.include && reportView.shotUrl !== ""
        const shotName = withShot ? "Turbine-screenshot-" + stamp + ".png" : ""
        const text = Turbine.report(labsItemText(reportItem), reportView.text, reportContext,
                                    reportView.includePerf ? reportView.perf : "", reportView.includeLog && reportView.hasLog, shotName)
        const res = parse(Turbine.saveReport(text, stamp, withShot ? Turbine.shotTemp() : ""), { error: "Couldn't save the report." })
        if (res.error !== "") { toast.info(res.error); return }
        Turbine.copyText(text)
        reportDraft = ""
        closeReport()
        note("labs", "Report saved: Downloads/" + res.report)
        // Where it went, on screen until OK, so it is plain that it saved.
        openMsg({ title: "Report saved", text: "It's in Downloads as " + res.report + (withShot ? ", with the screenshot beside it" : "") + ". The words are copied too, ready to paste.",
                  steps: [], notes: [], footnote: "In Desktop Mode, open Downloads to send the file with a few words.",
                  buttons: [{ key: "a", label: "OK" }] })
    }
    function closeReport() {
        reportView.close()
        discardArmed = false
        if (sheet === "report") sheet = ""
    }
    property string reportDraft: ""
    function dismissReport() {
        // B never throws words away: they stay as a draft for the next Report a bug.
        if (reportView.text.trim() !== "" && !/^Problem: [^.]*\. ?$/.test(reportView.text.trim())) {
            reportDraft = reportView.text
            note("labs", "Report closed, kept as a draft")
            closeReport()
            toast.info("Not saved yet. Your words are kept: Report a bug brings them back.")
            return
        }
        note("labs", "Report closed, nothing typed")
        closeReport()
    }
    // Steam's on-screen keyboard is up: the controller pointer leaves the sticks and triggers to it.
    property bool keyboardUp: false
    onKeyboardUpChanged: if (!shot.on) Turbine.pointerPaused = keyboardUp
    function reportEditing(on) {
        keyboardUp = on
        note("labs", on ? "Typing the report" : "Done typing the report")
        if (Turbine.gamemode && !shot.on) Qt.openUrlExternally(on ? "steam://open/keyboard" : "steam://close/keyboard")
        if (!on) keyRoot.forceActiveFocus()
    }
    function openPerf() { perfView.version = Turbine.version; perfView.extra = perfExtra(); perfView.open(); sheet = "perf" }
    function perfExtra() {
        return "Window: " + width + " x " + height + ", " + (Turbine.gamemode ? "Game Mode" : "Desktop") + "\n"
            + "Turbine scale: fit " + fit.toFixed(2) + "x, interface size " + sizeNow + "%, everything " + s.toFixed(2) + "x, layout " + layoutNow + "\n"
            + "Look: theme " + look.theme + ", high contrast " + (look.highContrast ? "on" : "off") + ", seasonal " + seasonPick
            + (seasonKind !== "" ? " (" + seasonKind + (fallen.moving ? ", moving" : ", still") + ")" : "")
    }

    // The app icon.
    readonly property string iconFull: "file:///app/share/icons/hicolor/128x128/apps/io.github.ravenor907.TurbineModManager.png"
    // The brass mark on a clear background, at every size; the splash takes the large one.
    readonly property string iconBig: "file:///app/share/icons/hicolor/512x512/apps/io.github.ravenor907.TurbineModManager.png"

    // ---------------------------------------------------------------- Look settings rows
    readonly property var layoutOptions: externalShown ? ["handheld", "desktop", "tv"] : ["handheld", "desktop"]
    readonly property var lookRows: [
        { label: "Interface size", key: "size", slider: true, min: sizeMin, max: sizeMax, unit: "%", now: sizeNow,
          step: (d) => sizePreview(sizeStep(d)),
          // A (and a tap or click on the row) opens the list of sizes; Left and Right, and the - and + buttons,
          // step down and up.
          a: () => openSizePicker() },
        { label: "Layout", options: layoutOptions, names: layoutOptions.map(o => o === "tv" ? "TV" : o.charAt(0).toUpperCase() + o.substring(1)), key: "layout", now: layoutNow,
          step: (d) => layoutPreview(layoutOptions[(layoutOptions.indexOf(layoutNow) + d + layoutOptions.length) % layoutOptions.length]) },
        { label: "Theme", options: ["brass", "black", "light"], names: ["Brass", "Black", "Light"], key: "theme",
          step: (d) => cycle("theme", ["brass", "black", "light"], d) },
        { label: "Accent", options: Object.keys(c.swatches), key: "accent", swatch: true,
          step: (d) => cycle("accent", ["brass", "iron", "copper", "jade", "fjord", "heather", "ember"], d) },

        { label: "Contrast", options: [false, true], names: ["Standard", "High"], key: "highContrast",
          step: (d) => { look.highContrast = !look.highContrast } },
        { label: "Color vision", options: ["typical", "red", "green", "blue", "none"], names: ["Typical", "Red-weak", "Green-weak", "Blue-weak", "No color"], key: "vision",
          step: (d) => cycle("vision", ["typical", "red", "green", "blue", "none"], d) },

        { label: "Compact list", options: [false, true], names: ["Off", "On"], key: "compactList",
          step: (d) => { look.compactList = !look.compactList; note("action", "Look: compact list " + look.compactList) } },
        { label: "Confirm deletes by", options: ["hold", "twice"], names: ["Holding A", "Pressing A twice"], key: "confirmBy",
          step: (d) => cycle("confirmBy", ["hold", "twice"], d) },
        { label: "Hold START does", options: ["choice", "last"], names: ["Show Play options", "Play my last choice"], key: "holdStartDoes",
          step: (d) => cycle("holdStartDoes", ["choice", "last"], d) },
        // Full, Reduced (fades only: nothing travels or grows) or Off (instant).
        { label: "Animations", options: ["full", "reduced", "off"], names: ["Full", "Reduced", "Off"], key: "animations", now: mo.level,
          step: (d) => { const l = ["full", "reduced", "off"]; look.animations = l[(l.indexOf(mo.level) + d + l.length) % l.length]; note("action", "Look: animations " + look.animations) } },

        { label: "Seasonal", options: ["off", "date", "autumn", "snow"], names: ["Off", "By date", "Autumn", "Snow"], key: "season",
          now: seasonPick, step: (d) => { look.season = seasonPick; cycle("season", ["off", "date", "autumn", "snow"], d) } }
    ].concat(seasonKind !== "" ? [
        { label: "Amount", key: "fallAmount", slider: true, min: 10, max: 100, unit: "",
          step: (d) => slide("fallAmount", 10, 100, 10, d), a: () => { if (look.fallAmount >= 100) look.fallAmount = 10; else slide("fallAmount", 10, 100, 10, 1) } },
        { label: "Size", key: "fallSize", slider: true, min: 50, max: 200, unit: "%",
          step: (d) => slide("fallSize", 50, 200, 25, d), a: () => { if (look.fallSize >= 200) look.fallSize = 50; else slide("fallSize", 50, 200, 25, 1) } }
    ] : [])
    function slide(key, lo, hi, by, d) {
        look[key] = Math.max(lo, Math.min(hi, look[key] + d * by))
        note("action", "Look: " + key + " " + look[key])
    }
    // Seasonal: off on handhelds and by date on TV until chosen. By date: snow from December 1 to the end of
    // February, leaves the rest of the year. Checked every hour, so a new season starts without a restart.
    readonly property string seasonPick: shot.arg("season") !== "" ? shot.arg("season") : look.season !== "" ? look.season : (tv ? "date" : "off")
    property date today: new Date()
    Timer { interval: 3600000; running: true; repeat: true; onTriggered: app.today = new Date() }
    readonly property string seasonKind: {
        if (seasonPick === "autumn" || seasonPick === "snow") return seasonPick
        if (seasonPick !== "date") return ""
        const md = (today.getMonth() + 1) * 100 + today.getDate()
        return md >= 1201 || md <= 229 ? "snow" : "autumn"
    }
    function cycle(key, list, d) {
        const i = list.indexOf(look[key])
        look[key] = list[(i + d + list.length) % list.length]
        note("action", "Look: " + key + " " + look[key])
    }

    // ---------------------------------------------------------------- glyphs (what Key.qml and Hint.qml show)
    // Glyphs follow the pad: an Xbox 360 pad says Back and Start; Series, Steam Deck and Ally pads say View and
    // Menu. With no pad seen (or one Turbine can't tell), View and Start, the words Steam's layouts use.
    readonly property string padKind: {
        const n = (Turbine.padName || "").toLowerCase()
        if (n.indexOf("360") >= 0 && n.indexOf("steam") < 0 && n.indexOf("virtual") < 0) return "360"
        if (/xbox|series|steam deck|ally|steam virtual/.test(n)) return "menu"
        return ""
    }
    // Desktop Mode with a mouse or keyboard: the legend shows keys ("Enter Open · Esc Back · Q E Switch game").
    // Keys count only when Turbine sees the pad itself (otherwise Steam may be sending the pad as keys).
    property string lastInput: "pad"      // pad, keys, mouse
    // The hints follow the last input used: the keyboard's keys after a key, the mouse's buttons after the
    // mouse, the controller's after a press on it. (Never in Game Mode; the build's pictures keep the controller's
    // unless a check asks.)
    property bool keysTest: false
    readonly property bool keysLegend: !Turbine.gamemode && (!shot.on || keysTest) && (lastInput === "mouse" || lastInput === "keys")
    // The keys a PC player expects (Page Up and Page Down still work, and are never shown).
    readonly property var keyNames: ({ a: "Enter", b: "Esc", x: "R", y: "F", lb: "Q", rb: "E", lt: "Home", rt: "End", view: "Tab", start: "Space", r3: "Ctrl+Z" })
    readonly property var mouseNames: ({ a: "Click", x: "Right-click" })
    // After the mouse, no key caps at all: the pages and the actions are buttons with their names.
    readonly property bool mouseLegend: keysLegend && lastInput === "mouse"
    function glyph(k) {
        if (k === "rs") return "R stick"
        if (keysLegend && lastInput === "mouse" && mouseNames[k]) return mouseNames[k]
        if (keysLegend && keyNames[k]) return keyNames[k]
        if (k === "view" && padKind === "360") return "BACK"
        if (k === "start" && padKind === "menu") return "MENU"
        return k.toUpperCase()
    }
    function fileUrl(p) { return p ? "file://" + p.split("/").map(encodeURIComponent).join("/") : "" }
    function initials(name) {
        const words = (name || "?").replace(/[^A-Za-z0-9 ]/g, " ").split(" ").filter(w => w.length > 0 && w !== "The" && w !== "of")
        return (words.length > 1 ? words[0][0] + words[1][0] : (words[0] || "?").substring(0, 2)).toUpperCase()
    }
    function runtimeWords(g) {
        if (g.runtime === "windows") return g.store === "steam" ? "Windows game, runs through Proton" : "Windows game, runs through Wine or Proton"
        if (g.runtime === "native") return "Native Linux game"
        return "Not started yet"
    }
    // ---------------------------------------------------------------- layout
    // The ground under everything (the window's own color isn't in the build's pictures, so empty areas there showed
    // as see-through).
    Rectangle { anchors.fill: parent; color: c.ground }
    // Seasonal leaves or snow, behind everything. Still while Turbine isn't in front (a game is running).
    Seasonal {
        id: fallen
        anchors.fill: parent
        kind: app.seasonKind
        amount: look.fallAmount
        size: look.fallSize
        scale: app.s
        light: c.light
        moving: (app.active || shot.on) && mo.moves
    }
    // The rail and the page beside it (not a Row: the rail keeps drawing while it slides out of a room of 0).
    Item {
        id: mainRow
        anchors.fill: parent
        // Off while anything covers it (see topLayer).
        enabled: app.topLayer === 0

        // ===== Rail (GameRail.qml): the games (last played first, portrait art), Settings pinned at the bottom.
        GameRail {
            id: rail
            app: app
            kit: kit
        }

        // ===== Main
        Item {
            id: main
            x: rail.width
            width: parent.width - rail.width
            height: parent.height
            // Laid out for the rail's new room at once; moved with the rail's edge while it slides (GameRail.qml).
            transform: Translate { x: rail.slide }

            Column {
                id: mainCol
                anchors.fill: parent
                // Every page starts a little down from the window's top edge.
                anchors.topMargin: app.px(12)
                anchors.leftMargin: app.px(14)
                anchors.rightMargin: app.px(14)
                anchors.bottomMargin: legend.shown ? legend.height + app.px(8) : app.px(8)
                spacing: app.px(8)

                // No tab row. The bottom bar names the tabs (LB and RB switch them, a tap or a sideways swipe too);
                // My mods' Find, Show, Sort and Select several are in the Y pop-up.
                // Page content
                Item {
                    id: content
                    width: parent.width
                    height: mainCol.height - y
                    clip: true
                    opacity: pageIn.part
                    transform: Arrive { id: pageIn; motion: mo; fadeTime: mo.gentle; moveTime: mo.moveGentle }
                    // Seasonal leaves and snow fade out behind the lists and cards (they show in the margins).
                    Rectangle { anchors.fill: parent; color: c.ground; opacity: app.seasonKind !== "" ? 0.85 : 0 }

                    // The page edges on Get mods (PageEdge.qml).
                    PageEdge { id: edgeBack; win: app; visible: app.bookPage || (app.turnFrom === 1 && !app.modPage); x: 0; label: "My mods"; dir: -1 }
                    PageEdge { id: edgeNext; win: app; visible: (app.bookPage || (app.turnFrom === 1 && !app.modPage)) && app.bookHasNext; x: parent.width - width; label: (app.tabs[2] || "Load order").replace(/ \(\d+\)$/, ""); dir: 1 }

                    // No games, mods being read.
                    EmptyState {
                        id: middle
                        readonly property bool none: !app.game
                        readonly property bool loading: app.game !== null && app.tab === 0 && !app.modsLoaded
                        ui: kit
                        anchors.fill: parent
                        visible: app.page === "game" && (none || loading)
                        title: none ? (app.railExtra > 0 ? "Every game is hidden" : "No games yet") : "Reading mods..."
                        body: none ? (app.railExtra > 0 ? "They were hidden from the rail. Show hidden games puts them back."
                                      : "Turbine looks in Steam's libraries, including SD cards, and in Heroic. Install a game and it shows up here.") : ""
                        buttonKey: none ? "a" : ""
                        buttonLabel: none ? (app.railExtra > 0 ? "Show hidden games" : "Look again") : ""
                        onActivated: { if (!app.gate(0, "onActivated")) return; app.railExtra > 0 ? app.showHiddenGames() : app.load() }
                    }

                    // My mods: the mods, then the collapsed Removed row (Find, Show, Sort and Select several are in
                    // the Y pop-up).
                    // My mods' head: the page's name and how many, or what Move, Select several or a filter is doing.
                    Row {
                        id: moveLine
                        z: app.turnTo === 0 ? 3 : 0
                        opacity: app.tabFade(0)
                        transform: Translate { x: app.tabShift(0) }
                        enabled: app.turnFrom !== 0
                        visible: app.page === "game" && app.onTab(0) && !app.modPage && app.game !== null && app.modsLoaded && (app.mods.length > 0 || app.moving || app.selecting || app.listFiltered)
                        height: visible ? app.px(30) : 0
                        x: app.px(6)
                        width: parent.width - app.px(12)
                        spacing: app.px(10)
                        Label { id: moveHead; win: app; anchors.verticalCenter: parent.verticalCenter; text: "My mods"; font.pixelSize: app.fpx(17); font.bold: true }
                        Label {
                            win: app
                            anchors.verticalCenter: parent.verticalCenter
                            width: parent.width - moveHead.width - parent.spacing
                            text: app.moving ? "Lower in the list wins when two mods change the same file."
                                  : app.selecting ? app.plural(app.tickedCount, "mod", "mods") + " ticked. A ticks, X acts on them, B done."
                                  : app.listFiltered ? "Showing " + app.plural(app.shownMods.length, "mod", "mods") + (app.findText !== "" ? " with \"" + app.findText + "\"" : "") + (app.listShow !== "all" ? " · " + app.showNames[app.listShow] : "") + (app.listSort !== "order" ? " · " + app.sortNames2[app.listSort] : "") + " · Y changes it"
                                  : app.plural(app.mods.length, "mod", "mods") + " · " + app.mods.filter(m => m.enabled).length + " on"
                            color: app.moving || app.selecting || app.listFiltered ? c.soft : c.muted
                            font.pixelSize: app.fpx(14)
                        }
                    }
                    // The details panel (ModPanel.qml): the highlighted mod, on the right of My mods when there is room.
                    ModPanel {
                        id: modPanel
                        app: app
                        kit: kit
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom
                        anchors.topMargin: moveLine.height + app.px(4)
                    }
                    // The list itself (ModList.qml).
                    ModList {
                        id: modList
                        app: app
                        kit: kit
                        look: look
                        anchors.fill: parent
                        anchors.rightMargin: modPanel.visible ? modPanel.width + app.px(10) : 0
                        anchors.topMargin: moveLine.height
                    }

                    // Get mods: a game with no source can still add a file.
                    EmptyState {
                        ui: kit
                        anchors.fill: parent
                        visible: app.page === "game" && app.game !== null && app.onTab(1) && app.sources.length === 0
                        opacity: app.tabFade(1)
                        transform: Translate { x: app.tabShift(1) }
                        enabled: app.turnFrom !== 1
                        title: "Get mods"
                        body: app.nexusUnknown ? "Turbine couldn't find " + app.gameTitle + " on Nexus Mods. Pick it yourself, or add a mod file (X)."
                              : "This game isn't on Nexus Mods or Thunderstore. You can still add a mod file."
                        buttonKey: "a"
                        buttonLabel: app.nexusUnknown ? "Pick the Nexus game" : "Add from Downloads"
                        buttonFocused: app.zone === "main"
                        onActivated: { if (!app.gate(0, "onActivated")) return; app.nexusUnknown ? app.openNexusPick() : app.openAdd() }
                    }
                    // An online-only anti-cheat game: no mods, and why.
                    EmptyState {
                        ui: kit
                        anchors.fill: parent
                        visible: app.page === "game" && app.acRefused && app.onTab(0) && app.mods.length === 0 && !app.modPage
                        opacity: app.tabFade(0)
                        transform: Translate { x: app.tabShift(0) }
                        enabled: app.turnFrom !== 0
                        title: app.gameTitle + " needs its anti-cheat"
                        body: app.acInfo ? app.acInfo.refuseWords : ""
                        buttonKey: ""
                        buttonLabel: ""
                    }
                    // An empty My mods (any game): a centered card with a big Get mods (A) and Add a file (X)
                    // (NoModsCard.qml).
                    NoModsCard {
                        app: app
                        kit: kit
                    }
                    // A Basic game's first visit with no mods: what Basic support means.
                    EmptyState {
                        ui: kit
                        anchors.fill: parent
                        visible: app.onTab(0) && app.basicIntroRoom && !app.modPage
                        opacity: app.tabFade(0)
                        transform: Translate { x: app.tabShift(0) }
                        enabled: app.turnFrom !== 0
                        title: "Mods for " + app.gameTitle
                        body: "Turbine has no rules for this game yet. It puts each mod where its files show they belong, and asks when it can't tell. Turning a mod off always puts the game back."
                        buttonKey: "a"
                        buttonLabel: app.rules && app.rules.domain ? "Get mods on Nexus" : "Add from Downloads"
                        buttonFocused: app.zone === "main"
                        note: app.rules && app.rules.domain ? "X: Add from Downloads" : ""
                        onActivated: { if (!app.gate(0, "onActivated")) return; app.rules && app.rules.domain ? app.tab = 1 : app.openAdd() }
                    }
                    // Where do these files go? (Basic games), and Change where it goes.
                    PlaceFiles {
                        ui: kit
                        anchors.fill: parent
                        anchors.topMargin: app.px(4)
                        visible: app.page === "place"
                        active: app.sheet === "" && !app.wheelOpen
                        moveOnly: app.placeMode === "move"
                        twoPanes: app.placeTwoPanes
                        topLine: app.placeTopLine
                        gamePath: [app.placeGameName].concat(app.placeGameRel === "" ? [] : app.placeGameRel.split("/")).join(" > ")
                        modPath: "Putting in: everything " + (app.placeModRel === "" ? "in the file" : "inside " + app.placeModRel)
                        gameRows: app.placeGameRows
                        modRows: app.placeModRows
                        pane: app.placePane
                        gameIndex: app.placeGameIndex
                        modIndex: app.placeModIndex
                        preview: app.placePreview
                        onRowClicked: (pane, index) => { if (!app.gate(0, "onRowClicked")) return; app.placePane = pane; if (pane === 0) app.placeGameIndex = index; else app.placeModIndex = index; app.placeAct("a") }
                        onRowHeld: (pane, index) => { if (!app.gate(0, "onRowHeld")) return; app.placePane = pane; if (pane === 0) app.placeGameIndex = index; else app.placeModIndex = index; app.placeAct("x") }
                        onSwiped: (dir) => { if (!app.gate(0, "onSwiped")) return; app.placeAct(dir < 0 ? "lb" : "rb") }
                    }
                    GetMods {
                        id: getView
                        ui: kit
                        anchors.fill: parent
                        anchors.topMargin: app.px(4)
                        // Between the page edges (My mods on the left; Load order on the right where the game has one).
                        anchors.leftMargin: app.tpx(34) + app.px(6)
                        anchors.rightMargin: app.bookHasNext ? app.tpx(34) + app.px(6) : 0
                        visible: app.page === "game" && app.game !== null && app.onTab(1) && app.sources.length > 0 && !app.modPage && !(app.turnFrom === 1 && app.turnModPage)
                        z: app.turnTo === 1 ? 3 : 0
                        opacity: app.tabFade(1) * gridBack.part
                        transform: [Translate { x: app.tabShift(1) }, Arrive { id: gridBack; motion: mo }]
                        enabled: app.turnFrom !== 1
                        // Touch: a swipe to the right turns back to My mods, to the left forward to Load order.
                        DragHandler {
                            enabled: app.bookPage && app.sheet === "" && !app.wheelOpen
                            target: null
                            xAxis.enabled: true; yAxis.enabled: false
                            acceptedDevices: PointerDevice.TouchScreen
                            grabPermissions: PointerHandler.TakeOverForbidden
                            onActiveChanged: if (!active && Math.abs(activeTranslation.x) > app.px(90)) { app.note("touch", "Swipe on Get mods"); app.turnPage(activeTranslation.x > 0 ? -1 : 1) }
                        }
                        // The cards with Installed and Update ready worked out from My mods (getItems).
                        result: app.browseData ? Object.assign({}, app.browseData, { items: app.getItems }) : null
                        loading: app.browseLoading
                        loadingMore: app.loadingMore
                        summary: app.getSummary
                        recent: app.starterItems
                        recentTitle: "Good first mods"
                        fallback: app.nexusFallback
                        fallbackTitle: "Browse " + app.gameTitle + " mods on Nexus Mods"
                        columns: app.gridColumns
                        focusArea: app.getArea
                        focusIndex: app.getIndex
                        active: app.zone === "main" && app.sheet === ""
                        onContentScrolled: (y, before, longer) => app.scrollFollow(y, before, longer)
                        onRecentClicked: (i) => { if (!app.gate(0, "onRecentClicked")) return; app.getArea = "recent"; app.getIndex = i; app.getAct("a") }
                        onItemClicked: (i) => { if (!app.gate(0, "onItemClicked")) return; app.getArea = "grid"; app.getIndex = i; app.openDetails(app.getItems[i]) }
                        onItemMenu: (i) => { if (!app.gate(0, "onItemMenu")) return; if (app.sheet === "") { app.zone = "main"; app.getArea = "grid"; app.getIndex = i; app.menuIndex = 0; app.sheet = "cardmenu" } }
                        onFallbackClicked: (k) => { if (!app.gate(0, "onFallbackClicked")) return; app.getArea = "fallback"; app.getAct(k) }
                        // After the left stick's wheel (or a swipe): the focus comes into view with the cards.
                        onScrolled: (first, last) => {
                            if (app.getArea !== "grid" || first < 0) return
                            const cols = app.gridColumns, col = app.getIndex % cols
                            if (app.getIndex < first) { let i = first - first % cols + col; if (i < first) i += cols; app.getIndex = Math.min(app.getItems.length - 1, i) }
                            else if (last >= 0 && app.getIndex > last) { let i = last - last % cols + col; if (i > last) i -= cols; app.getIndex = Math.max(0, i) }
                        }
                    }
                    ModDetails {
                        id: modView
                        ui: kit
                        anchors.fill: parent
                        anchors.topMargin: app.px(4)
                        visible: app.page === "game" && (app.modPage || (app.turnFrom === 1 && app.turnModPage))
                        opacity: app.tabFade(1) * modIn.part
                        transform: [Translate { x: app.tabShift(1) }, Arrive { id: modIn; motion: mo }]
                        enabled: app.turnFrom !== 1
                        info: app.detailsData
                        fallbackPicture: app.detailsItem && app.detailsItem.icon ? app.detailsItem.icon : ""
                        focused: app.zone === "main" && app.sheet === ""
                        mainLabel: app.mainLabel
                        mainLine: app.mainLine
                        mainProgress: app.busyHere && app.dl.active ? Math.max(0, app.dlPart) : -1
                        focusIndex: app.pageIndex
                        onInstallClicked: { if (!app.gate(0, "onInstallClicked")) return; app.pageIndex = 0; app.installFromDetails() }
                        onNeedClicked: (i) => { if (!app.gate(0, "onNeedClicked")) return; app.pageIndex = i + 2; app.openNeed(i) }
                        settingsLabel: app.pageHasSettings ? (app.isSeamless(app.pageMod) ? "Co-op settings" : "Edit settings") : ""
                        settingsFocused: app.pageSettings && app.zone === "main" && app.sheet === ""
                        onSettingsClicked: { if (!app.gate(0, "onSettingsClicked")) return; app.pageSettings = true; app.openModSettings(app.pageMod) }
                        // Right-click (LT with the pointer): the page's options, like X.
                        TapHandler { acceptedButtons: Qt.RightButton; onTapped: { if (!app.gate(0, "sheet (right)")) return; if (app.sheet === "" && app.pageMenuItems.length > 0) { app.zone = "main"; app.act("x") } } }
                    }
                    // Load order tab (Skyrim)
                    LoadOrderList {
                        id: loadOrderView
                        ui: kit
                        anchors.fill: parent
                        anchors.topMargin: app.px(4)
                        visible: app.page === "game" && app.game !== null && app.onTab(2) && !app.modPage
                        z: app.turnTo === 2 ? 3 : 0
                        opacity: app.tabFade(2)
                        transform: Translate { x: app.tabShift(2) }
                        enabled: app.turnFrom !== 2
                        plugins: app.order
                        error: app.orderError
                        messages: app.orderMessages
                        note: app.orderNote
                        owners: { let o = {}; for (const k in app.orderOwners) o[k] = app.modName(app.orderOwners[k]); return o }
                        focusIndex: app.orderIndex
                        moving: app.orderMoving
                        active: app.zone === "main" && app.sheet === ""
                        onSortClicked: { if (!app.gate(0, "onSortClicked")) return; app.orderIndex = -1; Turbine.orderSort() }
                        onRowClicked: (i) => { if (!app.gate(0, "onRowClicked")) return; app.rowA(() => { if (!app.orderMoving) app.orderIndex = i }) }
                        onRowMenu: (i) => { if (!app.gate(0, "onRowMenu")) return; if (app.sheet === "") { app.zone = "main"; app.orderIndex = i; app.openPluginMenu() } }
                        onScrolled: (first, last) => {
                            if (app.orderMoving || first < 0) return
                            if (app.orderIndex < first) app.orderIndex = app.orderLandOn(first, 1)
                            else if (last >= 0 && app.orderIndex > last) app.orderIndex = app.orderLandOn(last, -1)
                        }
                    }

                    // Settings (SettingsPage.qml)
                    SettingsPage {
                        id: settingsPage
                        app: app
                        kit: kit
                        look: look
                    }

                    // Scrolling test: 2,000 mod rows (ScrollTest.qml).
                    ScrollTest {
                        id: perfList
                        app: app
                        kit: kit
                    }
                }
            }

        }
    }

    // ---------------------------------------------------------------- the Nexus page (over the whole window)
    // Made when Nexus opens and destroyed when it closes (the profile, and with it the sign-in, stays). Hidden while
    // Settings is open from its quick wheel. Sheets and menus come after it, so they draw over it.
    Loader {
        id: browserLoader
        anchors.fill: parent
        active: app.browserOpen
        visible: app.page === "browser"
        enabled: app.topLayer === 0
        opacity: browserIn.part
        transform: Arrive { id: browserIn; motion: mo; fadeTime: mo.gentle; moveTime: mo.moveGentle }
        sourceComponent: Component {
            NexusBrowser {
                ui: kit
                live: app.browserLive
                startUrl: app.browserStart
                gameName: app.game ? app.shortGame(app.game.name) : ""
                gameIcon: app.game && (app.game.cover || app.game.header) ? app.fileUrl(app.game.cover || app.game.header) : ""
                zoom: app.browserZoom
                tab: app.browserTab
                downloads: app.browserRows
                dlIndex: app.browserDlIndex
                // Words a toast already says aren't said again in the pill.
                pillText: toast.shown && toast.text === app.browserPill.text ? "" : app.browserPill.text
                pillLevel: app.browserPill.level
                pillProgress: app.browserPill.progress
                legend: app.sheet === "" && !app.wheelOpen ? app.legendHints : []
                glyph: app.glyph
                padLegend: app.lastInput !== "touch"
                signinShown: app.browserSigninShown
                inputBlocked: app.sheet !== "" || app.wheelOpen
                noteFresh: app.browserNoteFresh
                googleShown: app.browserOnGoogle
                googlePasswordWords: "Nexus emails you a link. Open it (on your phone is fine), set a password, then sign in here with your email and that password. Nothing else to set up."
                googleChromeWords: app.bridge.paired === true ? "Opens Nexus's sign-in in Chrome. Sign in with Google there; Turbine takes the sign-in and comes back."
                                   : Turbine.gamemode ? "Needs Desktop Mode once: Chrome added to Steam (Settings, Labs, Use Chrome in Game Mode) and the browser helper in Chrome (Settings, Getting mods, Using your own browser)."
                                   : "Needs the browser helper set up in Chrome once (Settings, Getting mods, Using your own browser)."
            }
        }
    }

    // The status card over other apps (see showOverlay).
    StatusOverlay {
        id: panelWin
        ui: kit
        hud: false
        status: shot.on ? app.status : app.transfer
        fomod: app.fomod
        onConfirm: (yes) => { if (app.status.ask) Turbine.watchConfirm(app.status.ask, yes); app.found = null; if (app.sheet === "found" || app.sheet === "pickgame") app.sheet = "" }
        // Picking a game needs the list: Turbine comes to the front with it.
        onPick: { app.raise(); app.requestActivate(); if (app.found) app.openPickGame() }
        onChoose: (g, o) => Turbine.fomodChoose(g, o)
        onNext: Turbine.fomodNext()
        onBack: Turbine.fomodBack()
        onStop: Turbine.fomodCancel()
        onOpenTurbine: { app.raise(); app.requestActivate() }
    }
    StatusOverlay {
        id: hudWin
        objectName: "turbineHud"
        ui: kit
        hud: true
        status: shot.on ? app.status : app.transfer
        fomod: app.fomod
    }
    // ---------------------------------------------------------------- sheets
    // Every sheet, menu and prompt in one layer (SheetLayer.qml): off under the quick wheel and the tutorial (see
    // topLayer).
    SheetLayer {
        id: sheetLayer
        app: app
        kit: kit
        toast: toast
        seen: seen
        shot: shot
        rail: rail
        resetHold: resetHold
        confirmHold: confirmHold
        nxmRingMove: nxmRingMove
        // Every sheet and menu ends above the bottom bar, so its last row is never under it (150%, 1024 wide).
        anchors.fill: parent
        anchors.bottomMargin: legend.visible ? legend.height : 0
    }
    // The sheets the logic here drives by id.
    property alias askView: sheetLayer.askView
    property alias activityView: sheetLayer.activityView
    property alias checkView: sheetLayer.checkView
    property alias confirmSheet: sheetLayer.confirmSheet
    property alias findView: sheetLayer.findView
    property alias fomodView: sheetLayer.fomodView
    property alias gameMenuView: sheetLayer.gameMenuView
    property alias infoView: sheetLayer.infoView
    property alias perfView: sheetLayer.perfView
    property alias pickMenu: sheetLayer.pickMenu
    property alias profilesView: sheetLayer.profilesView
    property alias reportView: sheetLayer.reportView
    property alias settingsView: sheetLayer.settingsView

    // The bottom bar (BottomBar.qml, the only bar on the game screen). Left to right: the game's name with the tab
    // and its status under it (a warning there is tappable), LB RB and the tabs, then the legend: what binds() says A,
    // B, X and Y do here and one more (the pointer's buttons, or Start when a sheet covers Play), at most five, then
    // View Wheel, and Start Play on the right. Elsewhere it is the legend and View alone. After a touch the button
    // hints hide until the next press (the game's name, the tabs and Play stay). Scrolling down hides it with the
    // rail. Declared after the menus so their backdrop never dims it.
    readonly property var legendHints: legendFor(binds())
    // The bar keeps one shape on every game page, the mod page included (Play bottom right).
    readonly property bool barGame: page === "game" && game !== null
    // Where the player is, for the bar's left when it isn't a game screen (Settings, Where do these files go?).
    // On a mod page, its place in the results (LB and RB step through them).
    readonly property int pageResultAt: modPage && detailsItem ? getItems.findIndex(x => String(x.id) === String(detailsItem.id)) : -1
    readonly property string pageResultWords: pageResultAt >= 0 && getItems.length > 1 ? " · Mod " + (pageResultAt + 1) + " of " + getItems.length : ""
    readonly property string barPlace: page === "settings" ? "Settings" : page === "place" ? (placeMode === "move" ? "Change where it goes" : "Where do these files go?") : page === "perf" ? "Scrolling test" : ""
    // "My mods · Basic support · 2 mods on", or the problem in the status's own color.
    // The chip comes last, so a long line cuts it before the status words.
    // No tab name (the lit tab is right beside it), so the status itself has the room.
    // While anything downloads: "Getting 2 mods · 64%"; otherwise updates ready or the loadout.
    readonly property string pillWords: dl.active ? "Getting " + plural(Math.max(1, Turbine.netBusy), "mod", "mods") + (dlPart >= 0 ? " · " + Math.round(dlPart * 100) + "%" : "")
                                        : !browserOpen && webDlsRunning > 0 ? "Getting " + plural(webDlsRunning, "mod", "mods") + (webDlPart >= 0 ? " · " + Math.round(webDlPart * 100) + "%" : "")
                                        : statusNow.pill || ""
    // On Load order, a game with no problem says what that tab lists: its plugins and how many are on.
    readonly property string barStatus: (statusNow.level === "ok" ? (tab === 2 && order.length > 0 ? plural(order.length, "plugin", "plugins") + " · " + order.filter(p => p.active).length + " on"
                                                                     : (statusNow.detail || statusNow.words)) : statusNow.words) + (pillWords !== "" ? " · " + pillWords : "")
        + (supportChip !== "" ? " · " + supportChip : "")
    // Where LB or RB goes when there is no room for every tab's name: RB to the next tab, or LB back from the last.
    readonly property int barTarget: tabs.length < 2 ? -1 : tab < tabs.length - 1 ? tab + 1 : tab - 1
    readonly property bool barTappable: statusNow.level === "warn" || statusNow.level === "stop" || pillWords !== ""
    // A tap on the status line: what is waiting (downloads, updates, the loadout), else why there is a problem.
    function statusTap() {
        note("action", "Status tapped: " + barStatus)
        if (modHold && mods.some(m => m.enabled)) { openHoldChoice(); return }
        if (!dl.active && !browserOpen && webDlsRunning > 0) openNexusDownloads()
        else if (updateCount > 0 && !dl.active && statusNow.level === "ok") openUpdates()
        else if (pillWords.indexOf("Loadout: ") === 0 && statusNow.level === "ok") openProfiles()
        else if (statusNow.level === "warn" || statusNow.level === "stop") openWhy()
    }
    BottomBar {
        id: legend
        app: app
        kit: kit
        // Beside the rail's edge where it shows, so it never covers the rail while the rail slides.
        anchors.leftMargin: rail.width + rail.slide
    }
    QuickWheel {
        ui: kit
        visible: app.wheelOpen
        slots: app.wheelSlots
        focusIndex: app.wheelSlot
        hint: app.wheelHint
        onActivated: (i) => { app.wheelSlot = i; app.wheelFire(i) }
        // A mouse over a wedge picks it; over the middle (or off the pie) nothing is picked.
        onPointed: (i) => { app.wheelByStick = false; app.wheelSlot = i; app.wheelPicked = i >= 0 }
        onClosed: app.closeWheel()
        onMiddleTapped: { app.note("touch", "Quick wheel: middle tapped"); app.wheelSlot = -1; app.closeWheel() }
    }
    Tutorial {
        ui: kit
        visible: app.sheet === "tutorial"
        card: app.tutCard
        count: app.tutCount
        viewGlyph: app.glyph("view")
        startGlyph: app.glyph("start")
        keys: app.keysLegend
        thunderstore: app.sources.length > 0 && app.sources[0] === "thunderstore"
        onNext: app.tutorialAct("a")
        onSkip: app.tutorialAct("b")
        onBack: app.tutorialAct("left")
    }
    // Touch anywhere: the pointer goes and the legend hides (the next press brings it back). A tap still does its
    // thing; this only watches.
    Item {
        anchors.fill: parent
        z: 9999
        PointHandler {
            acceptedDevices: PointerDevice.TouchScreen
            onActiveChanged: if (active) app.touched()
        }
    }
    // The mouse's Back button is B everywhere, and Ctrl with the wheel changes the interface size. Over
    // everything; other buttons and a plain wheel pass straight through.
    Item {
        anchors.fill: parent
        z: 9998
        MouseArea {
            anchors.fill: parent
            acceptedButtons: Qt.BackButton | Qt.ForwardButton
            onClicked: (m) => { if (m.button === Qt.ForwardButton) app.mouseForward(); else app.mouseBack() }
        }
        WheelHandler {
            property real turned: 0
            acceptedModifiers: Qt.ControlModifier
            acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
            onWheel: (w) => {
                turned += w.angleDelta.y
                if (Math.abs(turned) < 120) return
                const d = turned > 0 ? 1 : -1
                turned = 0
                if (app.lastInput !== "mouse") app.lastInput = "mouse"
                app.sizeBy(d, "Ctrl and the wheel")
            }
        }
    }
    function touched() {
        if (lastInput !== "touch") note("touch", "Touch: pointer gone (the bar's buttons stay)")
        lastInput = "touch"
        Turbine.pointerHide()
    }

    // ---------------------------------------------------------------- toast
    // Something that just happened, in one of three looks: info, done, problem (real failures only). Never words
    // already on screen; the same words aren't shown twice in a row. (Uninstall says nothing here: its row stays,
    // dimmed, and A puts it back.)
    property string lastToast: ""
    property var toastAction: null
    function runToastAction() { const f = toastAction; if (!f) return; note("action", "Toast: " + toast.actionLabel); toastAction = null; toast.hide(); f() }
    property real lastToastAt: 0

    // ---------------------------------------------------------------- requirements
    // "Getting 2 of 3: <name>" while requirements are got one after another, with Skip and Stop (also in X
    // options on the Nexus page): RequirementsStrip.qml.
    RequirementsStrip {
        app: app
        kit: kit
    }
    Toast {
        id: toast
        ui: kit
        // Centred in the content: right of the rail, left of the details panel or a side sheet; 14 px over the bar.
        areaLeft: rail.visible ? rail.width : 0
        // The side sheets' widths (SideSheet.qml's users), to stay left of them.
        readonly property real sideWidth: ["modmenu", "cardmenu", "pick"].indexOf(app.sheet) >= 0 ? Math.min(app.width - app.px(40), app.px(392))
                                          : ["add", "shares"].indexOf(app.sheet) >= 0 ? Math.min(app.width - app.px(40), app.px(420))
                                          : ["versions", "plugin"].indexOf(app.sheet) >= 0 ? Math.min(app.width * 0.6, app.px(560))
                                          : app.sheet === "profiles" ? Math.min(app.width * 0.55, app.px(520)) : 0
        // A dialog in the middle of the window, or a side sheet that leaves too little room beside it: the toast goes
        // over the bottom bar (the sheet's buttons carry their keys meanwhile), never over the sheet. There it is a
        // banner as wide as the bar, so no hint shows half beside it.
        readonly property bool overBar: app.sheet !== "" && app.sheet !== "fomod" && app.sheet !== "tutorial" && legend.shown
                                        && (sideWidth === 0 || app.width - sideWidth - areaLeft < app.px(380))
        banner: overBar
        bannerHeight: legend.height - app.px(8)
        areaRight: app.width - (overBar ? 0 : app.sheet === "" && app.page === "game" && app.showModPanel && modPanel.visible ? modPanel.width + app.px(24) : sideWidth)
        // (The Nexus page's own bar is at the bottom while the bottom bar is away.)
        bottomMargin: overBar ? Math.max(app.px(4), (legend.height - height) / 2)
                      : (app.page === "browser" && !legend.visible ? app.px(48) : app.height - kit.barTop) + app.px(14)
        function put(kind, m) {
            if (m === app.lastToast && Date.now() - app.lastToastAt < 6000 && shown) return
            app.lastToast = m; app.lastToastAt = Date.now()
            toast.kind = kind
            actionLabel = ""; app.toastAction = null
            timeout = shot.page === "newmod" ? 0 : 4000
            show(m)
        }
        // A toast with its next step as a button; R3 (Y is always Find) or a tap on the
        // button does it while it shows.
        function act(kind, m, label, run) {
            put(kind, m)
            actionLabel = label; actionGlyph = app.mouseLegend ? "" : app.glyph("r3"); app.toastAction = run
            timeout = 8000; show(m)
        }
        onActionClicked: { if (!app.gate(5, "toast action")) return; app.runToastAction() }
        onHidden: app.toastAction = null
        function info(m) { put("info", m) }
        function done(m) { put("done", m) }
        function problem(m) { put("problem", m) }
        onPressedOn: app.coverPress(5)
        onPressEnded: app.coverEnd()
        // The new-mod picture keeps its toast up (it arrives a few seconds before the picture).
        timeout: shot.page === "newmod" ? 0 : 4000
    }
    // ---------------------------------------------------------------- The start-up splash
    // The only branding (Splash.qml): Turbine's icon coming in, then fading into the app, about 1 s. Once per
    // launch, never when coming back from a game or the background; any press, tap or click skips it.
    Splash {
        id: splash
        app: app
        kit: kit
        shot: shot
    }
    property bool splashDone: false

    // The build's input check: a plain mouse area over everything, to see the left button go down and up.
    MouseArea { id: inputProbe; anchors.fill: parent; z: 100000; visible: app.probeOn; enabled: app.probeOn; acceptedButtons: Qt.LeftButton }  // overlay-check: the build's input probe only
    // ---------------------------------------------------------------- the controller pointer
    // Steam's desktop layout: the right stick moves this pointer, RT clicks (held, it drags), LT right-clicks, the
    // left stick turned in a circle is a scroll wheel. Turbine sends real mouse and wheel events into this window
    // (backend.rs, qtglue.cpp), so everything that takes a tap or a mouse also takes the pointer; PointerLayer.qml
    // draws it, over everything (sheets included), and outlines the button or row under it. It hides 3 seconds after
    // the right stick stops; A and the D-pad work as always.
    property Item hoverItem: null
    property rect hoverBox: Qt.rect(0, 0, 0, 0)
    // Everything under the pointer that takes a tap; the outline goes on the innermost one (a row's ON pill
    // rather than the row). With a sheet open, the page under it doesn't count.
    property var hoverList: []
    function hover(item, on) {
        // A row or button the real mouse comes onto is the mouse being used too: the hover reaches that item's own
        // handler and not keyRoot's (the check saw no hover there while the row was outlined).
        if (on) mouseSeen()
        let list = hoverList.filter(x => !!x && x !== item)
        if (on) list.push(item)
        hoverList = list
        pickHover()
    }
    function inside(item, top) {
        for (let p = item; p; p = p.parent) if (p === top) return true
        return false
    }
    function pickHover() {
        const live = hoverList.filter(x => !!x && x.visible && !((sheet !== "" || page === "browser" || wheelOpen) && inside(x, mainRow)))
        let best = null
        for (const x of live) if (!live.some(y => y !== x && inside(y, x))) { best = x; break }
        hoverItem = best
        placeHover()
    }
    function placeHover() {
        try {
            if (!hoverItem || !hoverItem.visible) { hoverBox = Qt.rect(0, 0, 0, 0); return }
            const p = hoverItem.mapToItem(null, 0, 0)
            hoverBox = Qt.rect(p.x, p.y, hoverItem.width, hoverItem.height)
        } catch (e) {
            hoverBox = Qt.rect(0, 0, 0, 0)
        }
    }
    Connections {
        target: Turbine
        function onPointerXChanged() { app.placeHover(); app.pointerStill = false; pointerStillTimer.restart() }
        function onPointerYChanged() { app.placeHover(); app.pointerStill = false; pointerStillTimer.restart() }
        function onPointerShownChanged() { app.note("pad", Turbine.pointerShown ? "Pointer showing" : "Pointer hidden") }
    }
    property bool pointerStill: false
    Timer { id: pointerStillTimer; interval: 3000; onTriggered: app.pointerStill = true }
    PointerLayer {
        app: app
        kit: kit
    }
}
