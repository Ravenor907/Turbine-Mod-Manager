import QtQuick
import QtQuick.Window
import QtQuick.Shapes
import QtCore
import Turbine

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
        property int textSize: 100          // percent (before 0.5; moved into size once)
        property int size: 0                // Interface size, percent (80 to 150); 0 until set or moved over
        property bool highContrast: false
        property string vision: "typical"   // typical, red, green, blue, none
        property string layout: "handheld"  // handheld, desktop, tv (tv only with an external display)
        property string season: ""          // off, date, autumn, snow; "" until chosen (off on handhelds, by date on TV)
        property int fallAmount: 40         // 10 to 100
        property int fallSize: 100          // 50 to 200 percent
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
        property string launchDone: "[]"    // games whose one-time Steam Launch Option note was seen
        property string onceDone: "[]"      // "<game id>|<title>": check lines read once (REDmod)
        property string played: "[]"        // game ids, last played first (the rail's order)
        property string bannerDone: "[]"    // games whose "No mods yet" line on Get mods was seen
        property bool tutorialDone: false
        property string playedWarn: "[]"    // "<game>|<mods on>|<warnings>" played through with Play anyway   // the four first-run cards were seen (Settings, Help shows them again)
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
    // Settings > Accounts > Browser.
    Settings {
        id: browserSet
        category: "browser"
        property bool overlay: true         // Show status over other apps
        property bool addShown: false       // "Add Chrome to Steam" was shown once (Game Mode)
        property bool helperHintShown: false // "Set up the browser helper too" was shown once (Game Mode)
        property string oldHelperShown: ""  // "Your browser helper is out of date" was shown for this helper version
        property bool alwaysSteam: false    // Game Mode opens Nexus in Steam's browser straight away
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
        // Pages that wait for the engine (imports, menus, the check) get more time before the picture.
        readonly property bool slow: ["pointer", "pointerscroll", "checkend", "caught", "nexusnokey", "mods",
                                      "getmods", "getmodsfallback", "modpagefree", "modpagepremium", "getmodsback", "emptygame", "emptygamemods",
                                      "removed", "newmod", "opening", "opening8s", "desktop", "desktopgrid", "tv", "rail", "menu", "gamemenu", "check", "add", "loader", "report", "browse", "moddetails", "fomod",
                                      "loadorder", "profiles", "accounts", "updates", "lootsort", "lootnotes", "versions", "code",
                                      "nexusdetails", "nexusversions", "nexusfree", "nxm", "browser", "helper",
                                      "playoptions", "findmods", "selectmods", "overlaps", "quickexit", "chain", "listbar",
                                      "checkfocus", "menuskse", "addbrowser", "slowhint",
                                      "wheel", "ghost", "railmenu"].indexOf(page) >= 0 || scripted
        // 0.54: pages that run a list of presses (the controls' acceptance checks) and take their picture at the end.
        readonly property bool scripted: ["controls", "legendcheck", "sheetcheck", "nexusguard", "inlinebrowser", "presscount"].indexOf(page) >= 0
        // Pages where every sheet must come from a press (the build checks the log for "opened with no press").
        readonly property bool strictSheets: ["controls", "presscount", "opening8s", "nexusguard"].indexOf(page) >= 0
        // Pages that show Labs mode (it isn't saved from a picture).
        readonly property bool labs: ["about", "labs", "report", "perfcheck", "activity"].indexOf(page) >= 0
        // --layout=desktop or tv (not saved from a picture); --external-display pretends an HDMI display is there.
        readonly property string layout: arg("layout")
        readonly property bool external: args.indexOf("--external-display") >= 0
        function start() {
            const files = arg("selftest-import")
            // "file|thunderstore|Owner-Name|1.0.3" adds a zip as if it came from Thunderstore.
            if (files !== "") for (const f of files.split(",")) {
                const p = f.split("|")
                if (p.length === 4) Turbine.selftestImport(p[0], p[1], p[2], p[3]); else Turbine.importZip(f)
            }
            if (page === "gamemenu") app.sheet = "gamemenu"
            else if (page === "add") { app.tab = 1; app.openAdd() }
            else if (page === "about") app.openSection("About")
            else if (page === "labs") {
                app.labsAnswers = { "pad-quiet": "works", "proton-bepinex": "problem" }
                app.openSection("Labs"); app.setRow = 5
            }
            else if (page === "browse" || page === "moddetails" || page === "versions") { app.tab = 1; app.zone = "main" }
            else if (page === "nexusdetails" || page === "nexusversions" || page === "nexusfree") {
                if (page === "nexusfree") Turbine.setSampleFree()
                app.tab = 1; app.zone = "main"
            }
            else if (page === "loadorder" || page === "lootsort" || page === "lootnotes") { app.tab = 2; app.zone = "main" }
            else if (page === "accounts") app.openSection("Getting mods")
            // The browser: Settings > Accounts with a browser found and the helper connected; the no-browser
            // warning; the helper's setup steps; the status card in both its windows.
            else if (page === "browser" || page === "helper" || page === "nobrowser") {
                app.browser = page === "nobrowser" ? { checked: true, result: { found: false, name: "", how: "" }, title: "No web browser found",
                    text: "Turbine opens Nexus Mods in your web browser, and this system doesn't have one yet. Installing one takes a minute.",
                    steps: ["Switch to Desktop Mode: press the Steam button, open Power, and pick Switch to Desktop.",
                            "In Desktop Mode, open Discover, search for Google Chrome or Firefox, and press Install.",
                            "Optional, so it also opens in Game Mode: open the app menu, right-click the browser, and pick Add to Steam.",
                            "Switch back to Game Mode and open Turbine again."] }
                    : { checked: true, result: { found: true, name: "Google Chrome", how: "flatpak" } }
                app.bridge = { running: true, port: 47701, paired: true, connected: page === "browser" ? 1 : 0, ext: "iniglfblobfbekihldmgekchfehncbeb", error: "",
                               helper: page === "browser" ? "0.52" : "", bundled: "0.54", outdated: page === "browser", manual: page === "browser" }
                app.openSection("Getting mods")
                if (page === "helper") { app.helperFolder = "/home/deck/Downloads/Turbine browser helper"; app.infoIndex = 0; app.sheet = "helper" }
                else if (page === "nobrowser") app.noBrowser()
            }
            // Game Mode's Add Chrome to Steam (the home has no shortcuts.vdf, so Recheck finds nothing) and the
            // Getting mods key rows.
            else if (page === "addbrowser") {
                app.browser = { checked: true, result: { found: true, name: "Google Chrome", how: "flatpak" } }
                const r = app.parse(Turbine.browserShortcut(), { found: false })
                app.note("shot", "Browser in Steam: " + (r.found ? r.browser : "none"))
                app.openUrlNow = "https://www.nexusmods.com/skyrimspecialedition/mods/12604"
                app.showAddBrowser("noshortcut")
            }
            else if (page === "overlay" || page === "overlaygm") {
                app.status = { active: true, text: "Added to Cyberpunk 2077", level: "ok", progress: -1, game: "Cyberpunk 2077",
                               added: "Cyber Engine Tweaks 1.32.0", done: true, ask: "" }
                app.showOverlay(true, page === "overlaygm")
            }
            else if (page === "updates") { app.openSection("Updates"); Turbine.selfCheck(false, false) }
            // "Found <file>. Add it to <game>?" for a download Turbine wasn't sure about.
            else if (page === "caught") {
                app.found = { path: "/home/deck/Downloads/Appearance Menu Mod-790-2-6-1.zip", name: "Appearance Menu Mod-790-2-6-1.zip", size: 18874368,
                              game: app.game ? app.game.id : "", gameName: app.game ? app.game.name : "", why: "it is the game last open in Turbine", more: 1 }
                app.status = { active: true, text: "Found " + app.found.name + ". Add it to " + app.found.gameName + "?", level: "warn", progress: -1,
                               game: app.found.gameName, added: "", done: false, ask: app.found.path }
                app.infoIndex = 0; app.sheet = "found"
            }
            else if (page === "caughtgm") {
                app.status = { active: true, text: "Found Appearance Menu Mod-790-2-6-1.zip. Add it to Cyberpunk 2077?", level: "warn", progress: -1,
                               game: "Cyberpunk 2077", added: "", done: false, ask: "/home/deck/Downloads/Appearance Menu Mod-790-2-6-1.zip" }
                app.showOverlay(true, true)
            }
            else if (page === "nexusnokey" || page === "getmodsfallback" || page === "opening" || page === "opening8s") {
                Turbine.setSampleNexusFail(); app.tab = 1; app.zone = "main"
            }
            else if (page === "getmods" || page === "getmodsback" || page === "desktopgrid" || page === "tv") {
                // Two seeded Recently viewed mods.
                if (arg("game") === "steam:489830") app.recentSeed = [{ id: "2", name: "Lantern Glow SKSE", icon: app.fileUrl(arg("selftest-data") + "/thumb2.png"), source: "nexus" }, { id: "3", name: "Quiet Snow", icon: app.fileUrl(arg("selftest-data") + "/thumb3.png"), source: "nexus" }]
                app.tab = 1; app.zone = "main"
            }
            else if (page === "modpagefree" || page === "modpagepremium") {
                if (page === "modpagefree") Turbine.setSampleFree()
                app.tab = 1; app.zone = "main"
            }
            else if (page === "removed") app.modIndex = app.listItems.length - 1
            else if (page === "rail") { app.zone = "rail"; app.railIndex = 0 }
            else if (page === "size150" || page === "tvhidden") {
                app.openSection("Look")
                if (page === "size150") { app.sizePreview(150); app.setRow = 0 }
                else app.setRow = app.lookRows.findIndex(r => r.key === "layout")
            }
            else if (page === "overlaywait") {
                app.status = { active: true, text: "Press Save in the browser", level: "wait", progress: -1, game: "", added: "", done: false, ask: "" }
                app.showOverlay(true, false)
            }
            else if (page === "pointerscroll") { app.page = "perf"; app.zone = "perf" }
            else if (page === "perfcheck") app.openPerf()
            else if (page === "activity") { app.openSection("Labs"); app.openActivity() }
            else if (page === "slowhint") Turbine.setSampleFree()
            // 0.54: the beginner tutorial's cards; pages for the new controls.
            else if (page.indexOf("tutorial") === 0) app.openTutorial(parseInt(page.substring(8)) - 1)
            else if (page === "nexusguard" || page === "inlinebrowser") { Turbine.setSampleFree(); app.tab = 1; app.zone = "main" }
            else if (page === "presscount") Turbine.setSampleFree()
            else if (page === "settings" && arg("season") !== "") app.setRow = 6
        }
    }
    Timer {
        running: shot.on && shot.slow
        interval: 2500
        onTriggered: {
            if (shot.page === "menu" && app.menuMod) app.sheet = "modmenu"
            else if (shot.page === "check" || shot.page === "checkend") { app.startPlay(); if (shot.page === "checkend") checkEndShot.start() }
            else if (shot.page === "moddetails" && app.browseData && app.browseData.items && app.browseData.items.length > 0) app.openDetails(app.browseData.items[0])
            else if (shot.page === "versions" && app.browseData && app.browseData.items && app.browseData.items.length > 0)
                app.openDetails(app.browseData.items.find(x => x.id === "x753-More_Suits") || app.browseData.items[0])
            else if ((shot.page === "nexusdetails" || shot.page === "nexusversions" || shot.page === "nexusfree") && app.getItems.length > 0) app.openDetails(app.getItems[0])
            else if (shot.page === "code") { app.openProfiles(); Turbine.profileCode("Default"); app.sheet = "code" }
            else if (shot.page === "profiles") app.openProfiles()
            else if (shot.page === "browser") app.setRow = app.accountRows.findIndex(r => r.key === "overlay")
            else if (shot.page === "lootsort" || shot.page === "lootnotes") { app.orderIndex = -1; Turbine.orderSort() }
            // The controller pointer: over the second mod's ON pill, clicked through the same mouse events the right
            // stick and RT make (the mod turns off), then the 2,000 row list scrolled by wheel steps.
            else if (shot.page === "pointer") app.pointerCheck()
            else if (shot.page === "pointerscroll") app.pointerScrollCheck()
            // After the first frames, so the screenshot inside the report has something in it.
            else if (shot.page === "report") app.openReport(app.labsItems.find(x => x.id === "uninstall-undo"), "Undo put the mod back at the bottom of the list instead of where it was.", "")
            else if ((shot.page === "modpagefree" || shot.page === "modpagepremium") && app.getItems.length > 0) app.openDetails(app.getItems[0])
            else if (shot.page === "getmodsback") { app.getText = "lantern"; app.browseNow(); getBackShot.start() }
            else if (shot.page === "emptygame" || shot.page === "emptygamemods" || shot.page === "removed") {
                // One removed mod (two for the Removed sheet): imported, then removed.
                for (const m of app.mods) Turbine.removeMod(m.id)
                app.landAgain = true
                if (shot.page === "emptygamemods") { app.tab = 0; app.modIndex = 0 }
                if (shot.page === "removed") removedShot.start()
            }
            else if (shot.page === "opening") { app.getArea = "fallback"; app.getAct("a") }
            // Hold Start: Play options; R3 Find with "glow" typed; Select several with one mod ticked.
            else if (shot.page === "playoptions") { app.press("start"); playOptionsShot.start() }
            else if (shot.page === "findmods") { app.tab = 0; app.zone = "main"; app.act("y"); askView.open("Find a mod", "", "glow", "", 60, false, "Show them"); askView.accept() }
            else if (shot.page === "selectmods") { app.tab = 0; app.zone = "main"; app.modIndex = 0; app.startSelect(app.mods[0]); app.act("down") }
            // 0.54: a mod's file page opened on a free account: the status says to press Slow download.
            else if (shot.page === "slowhint") app.openNexus("https://www.nexusmods.com/witcher3/mods/2109?tab=files&file_id=1")
            else if (shot.page === "opening8s") { app.getArea = "fallback"; app.getAct("a"); app.openHelpNow() }
            // 0.51: the mod menu of a mod that loses a file; the closed-early prompt; the next requirement; the list bar.
            else if (shot.page === "overlaps") { app.tab = 0; app.zone = "main"; app.modIndex = 0; app.sheet = "modmenu"; app.note("shot", "Mod menu: " + app.modMenuItems.map(x => x.title).join("; ")) }
            else if (shot.page === "quickexit" && app.game && app.mods.length > 0) {
                // A setup from yesterday that worked, with only the first mod on.
                app.setups = [{ id: "s1", savedAt: Math.floor(Date.now() / 1000) - 86400, modsOn: 1, played: true, on: [app.mods[0].id] }]
                app.quickExit(app.game.id, 12)
            }
            else if (shot.page === "chain") {
                app.chainQueue = [{ name: "Community Patch", url: "https://www.nexusmods.com/witcher3/mods/3652?tab=files" }]
                app.showChain("Better Torches")
            }
            else if (shot.page === "listbar") { app.tab = 0; app.zone = "main"; app.listShow = "on"; app.modIndex = -1; app.listBarIndex = 1 }
            // 0.52: A on Before you play runs the highlighted button; the mod and game menus offer the missing loader.
            else if (shot.page === "checkfocus") { app.startPlay(); checkFocusShot.start() }
            else if (shot.page === "menuskse") {
                app.tab = 0; app.zone = "main"
                app.modIndex = Math.max(0, app.listItems.findIndex(x => x.kind === "mod" && x.m.needsLoader))
                app.note("shot", "Game menu: " + app.gameMenuItems.map(x => x.title).join("; "))
                app.sheet = "modmenu"
                app.note("shot", "Mod menu: " + app.modMenuItems.map(x => x.title).join("; "))
            }
            else if (shot.scripted) app.runScript(shot.page)
            else if (shot.page === "wheel") { app.tab = 0; app.zone = "main"; app.modIndex = 0; app.pad("view"); app.pad("up"); app.pad("right") }
            else if (shot.page === "ghost") { app.tab = 0; app.zone = "main"; app.modIndex = 0; if (app.mods[0]) app.uninstall(app.mods[0]) }
            else if (shot.page === "railmenu") { app.zone = "rail"; app.railIndex = app.current; app.pad("x") }
            if (shot.page === "mods") app.note("shot", "Row warnings: " + app.mods.filter(m => !!app.modWarns[m.id]).map(m => m.name + ": " + app.modWarns[m.id]).join("; "))
        }
    }
    Timer {
        running: shot.on
        interval: shot.scripted ? 16000 : shot.page.indexOf("loot") === 0 || shot.page.indexOf("nxm") === 0 || shot.page === "getmodsback" ? 6500 : shot.slow ? 4500 : 1500
        // The overlay pictures are of the overlay window itself.
        onTriggered: (shot.page === "overlay" || shot.page === "overlaywait" ? panelWin : shot.page === "overlaygm" || shot.page === "caughtgm" ? hudWin : app).contentItem.grabToImage(function (r) { r.saveToFile(shot.path); Qt.quit() })
    }
    Timer { id: removedShot; interval: 900; onTriggered: app.openRemoved() }
    // checkfocus: the highlight moved to Not now, A pressed (the check must close without playing); then the check
    // again with the highlight on Not now for the picture.
    Timer {
        id: checkFocusShot
        interval: 800
        property int stepNo: 0
        onTriggered: {
            if (app.sheet !== "check") { app.note("shot", "Check focus: no check sheet"); return }
            for (let i = 0; i < app.checkChoices.length; i++) app.act("right")
            app.note("shot", "Check focus on: " + app.checkChoices[app.checkIndex])
            if (stepNo === 0) { app.act("a"); app.note("shot", "After A: sheet \"" + app.sheet + "\""); stepNo = 1; app.startPlay(); start() }
        }
    }
    Timer { id: playOptionsShot; interval: 900; onTriggered: { app.note("shot", "Play options open: " + (app.sheet === "pick" && app.pickTitle === "Play options")); app.release("start") } }
    // getmodsback: the 5th card of the "lantern" search, its page, then B.
    Timer {
        id: getBackShot
        interval: 1500
        property int stepNo: 0
        onTriggered: {
            if (stepNo === 0) { app.getArea = "grid"; app.getIndex = Math.min(4, app.getItems.length - 1); app.getAct("a"); stepNo = 1; start() }
            else { app.act("b"); app.note("shot", "Back from the mod page: search \"" + app.getText + "\", card " + (app.getIndex + 1)) }
        }
    }

    // ---------------------------------------------------------------- size
    // Layout: handheld, desktop (mouse in Desktop Mode: smaller, denser rows, 4 columns) or tv (only with an
    // external display). The whole interface is drawn from a design size and never grows past what fits the window:
    // 760 x 460 design units is the smallest it reflows into.
    readonly property string layoutNow: shot.layout !== "" ? shot.layout : (look.layout === "tv" && !externalShown) ? "handheld" : look.layout
    readonly property bool tv: layoutNow === "tv"
    readonly property bool desktopLayout: layoutNow === "desktop"
    readonly property int sizeDefault: tv ? 120 : desktopLayout ? 80 : 100
    readonly property real fit: Math.max(0.5, Math.min(width / 883, height / 514))
    // The largest Interface size that still fits this window (the interface never grows past 760 x 460 design units):
    // about 116% at 1280 x 800 and 112% at 1920 x 1080. The slider ends there ("Largest that fits").
    readonly property int sizeCap: Math.max(80, Math.min(150, Math.floor(Math.min(width / 760, height / 460) / fit * 100)))
    readonly property int sizeNow: Math.min(sizeCap, sizeTrial > 0 ? sizeTrial : look.size > 0 ? look.size : sizeDefault)
    // The sizes the slider steps through: every 10% up to the cap, then the cap itself.
    readonly property var sizeSteps: { let l = []; for (let v = 80; v < sizeCap; v += 10) l.push(v); l.push(sizeCap); return l }
    function sizeStep(d) {
        let i = sizeSteps.findIndex(v => v >= sizeNow)
        if (i < 0) i = sizeSteps.length - 1
        else if (sizeSteps[i] !== sizeNow && d < 0) i++   // between two steps: Left goes to the lower one
        return sizeSteps[Math.max(0, Math.min(sizeSteps.length - 1, i + d))]
    }
    readonly property real s: Math.max(0.4, Math.min(fit * sizeNow / 100, width / 760, height / 460))
    function px(v) { return Math.round(v * s) }
    // Design width: 3 grid columns on a handheld, 4 from 1100 (TV and Desktop layouts), 2 below 860.
    readonly property real designWidth: width / s
    readonly property int gridColumns: desktopLayout || designWidth >= 1100 ? 4 : designWidth < 860 ? 2 : 3
    readonly property int rowHeight: desktopLayout ? 38 : 0
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
        v = Math.max(80, Math.min(sizeCap, v))
        if (v === sizeNow) return
        note("action", "Try size " + v + "% (largest that fits " + sizeCap + "%)")
        // Pictures never save settings.
        if (shot.on) sizeTrial = v; else { look.size = v; sizeTrial = 0 }
    }
    function layoutPreview(v) {
        note("action", "Layout " + v)
        if (!shot.on) look.layout = v
    }

    // ---------------------------------------------------------------- Labs mode and the activity log
    readonly property bool labsOn: labsSet.on || (shot.on && shot.labs)
    function note(kind, text) { Turbine.log(kind, text) }
    onPageChanged: note("page", "Page " + page)
    onTabChanged: {
        note("page", "Tab " + (tabs[tab] || tab))
        selecting = false
        modPage = false
        history = []
        if (tab === 1 && !browseData) browseNow()
        else if (tab === 2) { if (orderMemo < 0 && order.length > 0) orderMemo = orderIndex; orderLand = true; Turbine.loadOrder() }
    }
    onZoneChanged: note("page", "Focus zone " + zone)
    onSetSectionChanged: {
        note("page", "Settings section " + (settingsSections[setSection] || setSection))
        if (settingsSections[setSection] === "Getting mods") { Turbine.handlerStatus(); if (!shot.on) Turbine.bridgeRefresh() }
    }
    onWidthChanged: sizeNote.restart()
    onHeightChanged: sizeNote.restart()
    Timer { id: sizeNote; interval: 500; onTriggered: app.note("window", "Window " + app.width + " x " + app.height + ", scale " + app.s.toFixed(2)) }
    // The first frame drawn: how long starting took (Labs > Performance check).
    property bool shown: false
    onFrameSwapped: if (!shown) { shown = true; Turbine.markShown() }

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
        // Cinzel is only for the wordmark and the game's name in the header: every other title is Barlow.
        readonly property string titleFont: app.bodyFont
        readonly property string bodyFont: app.bodyFont
        function px(v) { return app.px(v) }
        function hover(item, on) { app.hover(item, on) }
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
    readonly property var tabs: ["My mods (" + mods.length + ")", "Get mods"].concat(rules && rules.plugins ? ["Load order"] : [])
    // My mods: the pinned "+ Get mods" row, the mods, then one collapsed row for Recently removed.
    readonly property var listItems: {
        let l = shownMods.map(m => ({ kind: "mod", m: m }))
        // Just uninstalled: the row stays, dimmed, for 8 seconds ("Removed", A puts it back), then the next row.
        if (ghost && game && ghost.game === game.id) l.splice(Math.max(0, Math.min(ghost.index, l.length)), 0, { kind: "ghost", m: { id: ghost.id, name: ghost.name } })
        return l.concat(removedMods.length > 0 ? [{ kind: "removed", m: { id: "", name: "" } }] : [])
    }
    property var ghost: null              // {game, id, name, index}
    Timer { id: ghostTimer; interval: 8000; onTriggered: app.ghost = null }
    // Find (R3): only the mods whose name has every word typed, as it is typed.
    property string findText: ""
    property string findBefore: ""
    // The bar over My mods: [Find] at any size, plus [Show] and [Sort] from 15 mods. Show narrows the list; Sort
    // orders it (other sorts than Load order hide the numbers, and reorder waits for Load order).
    property string listShow: "all"       // all, on, off, attention, new, update
    property string listSort: "order"     // order, az, recent
    readonly property var showKeys: ["all", "on", "off", "attention", "new", "update"]
    readonly property var showNames: ({ all: "All", on: "On", off: "Off", attention: "Needs attention", new: "New", update: "Update ready" })
    readonly property var sortKeys2: ["order", "az", "recent"]
    readonly property var sortNames2: ({ order: "My order", az: "A to Z", recent: "Recently added" })
    readonly property bool listBarFull: mods.length >= 15 || listShow !== "all" || listSort !== "order" || shot.page === "listbar"
    readonly property var listBar: [{ key: "find", label: findText !== "" ? "Find: " + findText : "Find" }]
        .concat(listBarFull ? [{ key: "show", label: "Show: " + showNames[listShow] }, { key: "sort", label: "Sort: " + sortNames2[listSort] }] : [])
        // Select several lives with the other whole-list tools, not in one mod's menu.
        .concat(mods.length > 1 ? [{ key: "select", label: "Select several" }] : [])
    property int listBarIndex: 0
    function needsAttention(m) { return (m.needsLoader && loaderMissing) || (m.enabled && (missingNeeds(m).length > 0 || isHidden(m) || !!modWarns[m.id])) }
    readonly property var shownMods: {
        const words = findText.toLowerCase().split(/\s+/).filter(w => w !== "")
        let list = mods.filter(m => {
            const n = (m.name || "").toLowerCase()
            if (!words.every(w => n.indexOf(w) >= 0)) return false
            if (listShow === "on") return m.enabled
            if (listShow === "off") return !m.enabled
            if (listShow === "attention") return needsAttention(m)
            if (listShow === "new") return isNew(m)
            if (listShow === "update") return !!updateMap[m.id]
            return true
        })
        if (listSort === "az") list = list.slice().sort((a, b) => (a.name || "").toLowerCase().localeCompare((b.name || "").toLowerCase()))
        else if (listSort === "recent") list = list.slice().sort((a, b) => b.added - a.added)
        return list
    }
    readonly property bool listFiltered: findText !== "" || listShow !== "all" || listSort !== "order"
    function listBarRun(i) {
        const k = listBar[i] ? listBar[i].key : ""
        if (k === "find") openFind()
        else if (k === "select") { startSelect(null); modIndex = 0; toast.showPlain("A ticks a mod, X acts on every ticked mod, B is done.") }
        else if (k === "show" || k === "sort") {
            const keys = k === "show" ? showKeys : sortKeys2, names = k === "show" ? showNames : sortNames2
            const now = keys.indexOf(k === "show" ? listShow : listSort)
            openPick(k === "show" ? "Show" : "Sort", gameTitle, keys.map((key, i) => ({
                icon: i === now ? "power" : "info", title: names[key], detail: i === now ? "Showing now" : "",
                run: () => { if (k === "show") listShow = key; else listSort = key; note("action", "My mods: " + k + " " + names[key]) } })), "")
            menuIndex = now
        }
    }
    function listCycle(k, d) {
        if (k === "show") { listShow = showKeys[(showKeys.indexOf(listShow) + d + showKeys.length) % showKeys.length]; note("action", "My mods: show " + showNames[listShow]) }
        else if (k === "sort") { listSort = sortKeys2[(sortKeys2.indexOf(listSort) + d + sortKeys2.length) % sortKeys2.length]; note("action", "My mods: sort " + sortNames2[listSort]) }
    }
    function openFind() {
        note("action", "Find a mod")
        findBefore = findText
        openAsk("find", "Find a mod", "Shows only the mods whose names have these words.", findText, "Part of a mod's name", 60, false, "Show them")
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
    function openPick(title, sub, items, back) { pickTitle = title; pickSub = sub; pickItems = items; pickBack = back || ""; menuIndex = 0; sheet = "pick" }
    function pickRun(i) {
        const it = pickItems[i]
        if (!it) return
        note("action", pickTitle + ": " + it.title)
        sheet = ""
        it.run()
    }
    // Hold Start: Play with mods, Play without mods, or with another profile.
    function openPlayOptions() {
        if (!game || sheet !== "") return
        note("action", "Play options")
        Turbine.profiles()
        playOptionsNow()
    }
    function playOptionsNow() {
        const others = profileList.filter(p => !profileData || p.name !== profileData.active)
        openPick("Play options", gameTitle, [
            { icon: "play", title: "Play with mods", detail: plural(mods.filter(m => m.enabled).length, "mod", "mods") + " on", run: () => startPlay() },
            { icon: "play", title: "Play without mods", detail: "Your mods stay installed and come back next time", run: () => Turbine.playOriginal() }
        ].concat(canSkipLoader ? [
            { icon: "play", title: rules.kind === "bethesda" ? "Play without the script extender" : "Play without " + loaderShort,
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
            { icon: "remove", title: "Delete", detail: "Hold A to delete it for good", danger: true, run: () => askDelete("profile:" + n, "Delete the loadout " + n + " for good? Its mods stay installed.") }
        ]), "profiles")
    }
    // Load order: the mod that placed a plugin.
    property var orderOwners: ({})
    readonly property string pluginOwner: orderPlugin ? (orderOwners[orderPlugin.name.toLowerCase()] || "") : ""
    function goToMod(id) {
        note("action", "Go to its mod: " + id)
        sheet = ""; tab = 0; findText = ""; listShow = "all"; modPage = false
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
            app.allGames = app.parse(Turbine.games, [])
            app.orderGames(shot.arg("game") || seen.lastGame)
        }
        function onModsJsonChanged() {
            if (Turbine.modsJson === "") { app.modsLoaded = false; app.mods = []; app.removedMods = []; app.setups = []; return }
            if (app.moving) return
            const v = app.parse(Turbine.modsJson, {})
            app.mods = v.mods || []
            app.removedMods = v.removed || []
            app.setups = v.setups || []
            app.overlapsMap = v.overlaps || {}
            app.linksMap = v.links || {}
            app.modWarns = v.warns || {}
            app.kcdRestore = v.kcdRestore === true
            app.modsLoaded = true
            app.modIndex = Math.max(0, Math.min(app.modIndex, app.listItems.length - 1))
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
            app.rules = app.parse(Turbine.rulesJson, null)
            app.maybeLandOnGet()
            app.offerLoader()
            if (app.rules && app.tab === 1 && !app.browseData) app.browseNow()
        }
        function onDownloadsJsonChanged() {
            app.downloads = app.parse(Turbine.downloadsJson, [])
            app.addIndex = Math.max(0, Math.min(app.addIndex, app.addList.length - 1))
        }
        function onMessage(text, level) { app.say(text, level) }
        function onChecked(json) { app.checked(app.parse(json, [])) }
        function onLaunch(url) { app.playAt = Date.now(); app.playGame = app.game ? app.game.id : ""; if (!shot.on) Qt.openUrlExternally(url) }
        function onGameEnded(gameId, seconds, known) {
            if (known) { if (seconds < 60) app.quickExit(gameId, seconds) }
            else app.measureAway = gameId
        }
        function onPages(title, json) { app.openPages(title, app.parse(json, [])) }
        function onHandlerJsonChanged() { app.handler = app.parse(Turbine.handlerJson, { state: "unknown" }) }
        // Catching browser downloads (catch.rs).
        function onStatusJsonChanged() { if (!shot.on || shot.page === "slowhint") app.status = app.parse(Turbine.statusJson, { active: false }) }
        function onBrowserJsonChanged() {
            if (shot.on) return
            app.browser = app.parse(Turbine.browserJson, { checked: false })
            if (app.sheet === "nobrowser" && app.browserFound) { app.sheet = ""; toast.showPlain("Found " + app.browser.result.name + ". Nexus pages open there.") }
        }
        function onBridgeJsonChanged() {
            if (shot.on) return
            const was = app.bridge.connected || 0
            app.bridge = app.parse(Turbine.bridgeJson, { running: false })
            if (app.bridge.connected > was && app.openState !== "")
                app.note("browser", "The browser helper " + (app.bridge.helper || "") + " connected " + (Date.now() - app.openAt) + " ms after Open")
            // An old helper that can't reload itself: how to bring it up to date, once (Settings, Getting mods says it too).
            if (app.bridge.connected > was && app.bridge.manual) app.oldHelperOnce(app.bridge.helper)
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
        function onPad(button, pressed) { if (pressed) app.lastInput = "pad"; pressed ? app.press(button) : app.release(button) }
        // 0.3
        function onBrowseJsonChanged() {
            const v = app.parse(Turbine.browseJson, null)
            if (!v || !app.game || v.game !== app.game.id) return
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
            app.gridReturn = -1
            if (!v.category && (v.categories || []).length > 0) app.catList = v.categories
            else if (v.source !== "nexus") app.catList = v.categories || []
            app.browseData = v
            app.noMore = (v.items || []).length < 30
            app.browseLoading = false
            if (app.restoreFocus) {
                const r = app.restoreFocus
                app.restoreFocus = null
                app.getArea = r.area; app.getIndex = Math.max(0, Math.min(r.index, (v.items || []).length - 1))
            } else if (app.getArea === "grid") app.getIndex = Math.max(0, Math.min(app.getIndex, (v.items || []).length - 1))
            else if (app.getArea === "fallback") { app.getArea = "bar"; app.getIndex = 0 }
        }
        function onDetailsJsonChanged() {
            let d = app.parse(Turbine.detailsJson, null)
            // A page that couldn't be read still shows: what the card knew, the reason in one line, Open on Nexus.
            if (d && d.error && app.detailsItem) d = app.partialPage(app.detailsItem, d.error)
            app.detailsData = d
            if (app.detailsData && app.detailsData.name && app.detailsItem) app.rememberViewed(app.detailsItem, app.detailsData)
            if ((shot.page === "versions" || shot.page === "nexusversions") && app.otherVersions.length > 0) app.openVersions()
        }
        function onDownloadJsonChanged() { app.dl = app.parse(Turbine.downloadJson, { active: false }) }
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
        function onUpdatesJsonChanged() { app.updatesData = app.parse(Turbine.updatesJson, null) }
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
    readonly property var hiddenIds: parse(railSet.hidden, [])
    function orderGames(want) {
        const keep = want !== undefined ? want : (game ? game.id : "")
        const order = parse(seen.played, []), pins = pinnedIds, hide = hiddenIds
        let list = allGames.map((g, i) => ({ g: g, i: i })).filter(x => hide.indexOf(x.g.id) < 0 || x.g.id === shot.arg("game"))
        const rank = x => { const p = pins.indexOf(x.g.id), o = order.indexOf(x.g.id); return p >= 0 ? p : 1000 + (o < 0 ? 1e6 + x.i : o) }
        list.sort((a, b) => rank(a) - rank(b))
        list = list.map(x => x.g)
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
        if (!shot.on) { if (which === "pinned") railSet.pinned = JSON.stringify(l); else railSet.hidden = JSON.stringify(l) }
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
        all[gid] = { tab: tab, get: { area: getArea === "fallback" ? "bar" : getArea, index: getIndex, text: getText }, order: orderIndex }
        focusMemo = all
    }
    // The mod last focused in each game: switching back lands on it (else on the first mod).
    property var lastFocusId: ({})
    property bool landFocus: true
    // Not while a game's mods are still on their way in (the list on screen is the last game's until then).
    onModIndexChanged: if (game && !moving && !landFocus && focusedItem && focusedItem.kind === "mod") lastFocusId[game.id] = focusedItem.m.id
    onGameChanged: {
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
        getArea = "bar"; getIndex = 0; getText = ""; getCategory = ""
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
    Component.onCompleted: {
        // Recorded samples instead of the internet (the build's screenshot checks); first, before anything asks.
        if (shot.arg("selftest-data") !== "") Turbine.setSampleData(shot.arg("selftest-data"))
        else if (!shot.on) {
            Turbine.accountStatus()
            Turbine.startCatching()
            if (updates.auto) { selfAuto = true; Turbine.selfCheck(labsOn && updates.prerelease, false) }
        }
        load()
        note("app", "Qt platform " + Qt.platform.pluginName + (Turbine.gamemode ? ", Game Mode" : ""))
        if (shot.page === "settings") { page = "settings"; zone = "settings" }
        else if (shot.page === "details") sheet = "details"
        else if (shot.page === "perf") { page = "perf"; zone = "perf" }
        else if (shot.page === "notice") sheet = "notice"
        else if (!shot.on && !seen.tutorialDone) openTutorial(0)
        else if (!shot.on && !Turbine.steamos && !seen.notSteamOS) sheet = "notice"
        if (look.size === 0 && look.textSize !== 100) look.size = Math.max(80, Math.min(150, Math.round(look.textSize / 10) * 10))
        checkDisplays()
        if (shot.on) shot.start()
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
    }
    readonly property bool msgShown: msgText !== "" && (msgLevel === "work" || msgTimer.running)
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
        if (openState === "open") return { level: slow ? "busy" : "ok", words: slow ? status.text : "Nexus is open in " + openName + ". Turbine adds your download by itself.", detail: "" }
        if (msgShown) return { level: ({ work: "busy", ok: "ok", warn: "warn", error: "stop" })[msgLevel] || "ok", words: msgText, detail: "" }
        if (needLoader.length > 0)
            return { level: "warn", words: plural(needLoader.length, "mod needs ", "mods need ") + loaderShort, detail: "The game still plays, without " + (needLoader.length === 1 ? "that one" : "those") }
        const warned = mods.filter(m => m.enabled && !!modWarns[m.id])
        if (warned.length > 0)
            return { level: "warn", words: warned.length === 1 ? warned[0].name + ": " + modWarns[warned[0].id] : plural(warned.length, "mod", "mods") + " may not work here", detail: "Before you play says why" }
        const on = mods.filter(m => m.enabled).length
        // Changes still on their way in show as a pill (only on the game they belong to); the line says how it stands.
        const waiting = Turbine.pending > 0 && Turbine.pendingGame === game.id
        return { level: "ok", words: "Ready to play", detail: !modsLoaded ? "" : mods.length === 0 ? "No mods yet" : plural(on, "mod", "mods") + " on",
                 pill: waiting ? plural(Turbine.pending, "change", "changes") + " going in" : updateCount > 0 ? plural(updateCount, "update", "updates") + " ready"
                       : profileData && profileList.length > 1 ? "Loadout: " + profileData.active : "" }
    }

    // ---------------------------------------------------------------- applying changes
    // Changes go into the game 3 seconds after the last edit, when Turbine loses focus and when it closes.
    Timer { id: autoApply; interval: 3000; onTriggered: if (!shot.on) Turbine.applyNow() }
    function edited() { autoApply.restart() }
    onActiveChanged: {
        note("window", active ? "Turbine in front" : "Turbine in the background")
        if (active) {
            // Back from a game: presses made there (still queued) must not act here.
            repeater.stop()
            repeatButton = ""
            Turbine.wake()
        } else if (!shot.on && Turbine.pending > 0) {
            autoApply.stop()
            Turbine.applyNow()
        }
    }
    onClosing: Turbine.finish()

    // ---------------------------------------------------------------- mod actions
    function toggle(m, sure) {
        const on = !m.enabled
        const users = on ? [] : neededByOn(m)
        if (!on && !sure && users.length > 0) {
            openPick(plural(users.length, "mod needs", "mods need") + " " + m.name, users.map(x => x.name).slice(0, 3).join(", ") + (users.length > 3 ? " and " + (users.length - 3) + " more" : "") + ". They will stop working.", [
                { icon: "power", title: "Turn off anyway", detail: "Those mods stay on but won't work.", danger: true, run: () => toggle(m, true) },
                { icon: "back", title: "Keep it on", detail: "", run: () => {} }
            ], "")
            return
        }
        note("action", "Turn " + (on ? "on " : "off ") + m.name)
        mods = mods.map(x => x.id === m.id ? Object.assign({}, x, { enabled: on }) : x)
        Turbine.setEnabled(m.id, on)
        edited()
        if (on && m.needsLoader && loaderMissing) { loaderFor = [m.name]; openLoader() }
    }
    function moveMod(m, to) {
        const from = mods.findIndex(x => x.id === m.id)
        if (from < 0 || to === from) return
        note("action", "Move " + m.name + " from " + (from + 1) + " to " + (to + 1))
        let list = mods.slice()
        list.splice(from, 1)
        list.splice(to, 0, m)
        mods = list
        modIndex = to
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
        ghost = { game: game ? game.id : "", id: m.id, name: m.name, index: at }
        ghostTimer.restart()
        modIndex = at
    }
    // ---------------------------------------------------------------- Recently removed (one collapsed row; a sheet)
    property int removedIndex: 0
    function openRemoved() { removedIndex = removedMods.length > 0 ? 1 : 0; sheet = "removed" }
    function removedRun(i) {
        if (i === 0) askDelete("all", "Delete all " + plural(removedMods.length, "removed mod", "removed mods") + " for good? They can't be put back.")
        else if (removedMods[i - 1]) putBack(removedMods[i - 1].id)
    }
    function removedDelete(i) {
        const m = removedMods[i - 1]
        if (m) askDelete(m.id, "Delete " + m.name + " for good? It can't be put back.")
        else if (i === 0) removedRun(0)
    }
    // Hold A to confirm (data that can't come back): the same hold the game menu's Reset uses.
    property string confirmWhat: ""
    property string confirmText: ""
    // 2 seconds for a mod already in Removed mods; 5 for a loadout or profile.
    readonly property int holdNeeded: confirmWhat.indexOf("profile:") === 0 ? 5 : 2
    function askDelete(what, text) { confirmWhat = what; confirmText = text; confirmIndex = 0; stopReset(); sheet = "confirm" }
    property int confirmIndex: 0
    function confirmDone() {
        const w = confirmWhat
        if (w.indexOf("profile:") === 0) {
            const n = w.substring(8)
            note("action", "Delete profile " + n)
            Turbine.profileDelete(n)
            profIndex = 0; sheet = "profiles"
            return
        }
        sheet = removedMods.length > (w === "all" ? removedMods.length : 1) ? "removed" : ""
        if (w === "all") { Turbine.purgeAllRemoved(); toast.done("Removed mods deleted for good.") }
        else { const m = removedMods.find(x => x.id === w); Turbine.purgeRemoved(w); toast.done((m ? m.name : "The mod") + " deleted for good.") }
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
        if (listFiltered) {
            // Moving needs the whole list in your order: switch to it, keep the same mod focused, then move.
            const id = focusedItem.m.id
            findText = ""; listShow = "all"; listSort = "order"
            const i = listItems.findIndex(x => x.kind === "mod" && x.m.id === id)
            if (i < 0) return
            modIndex = i
            note("action", "Move: the full list in your order")
        }
        moving = true
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
        note("action", "Move cancelled")
    }
    // The loader prompt shows once per game while a mod that needs the missing loader is on.
    property string loaderOffered: ""
    property var loaderFor: []
    property bool loaderSourceShown: false
    property int loaderIndex: 0
    function openLoader() { loaderSourceShown = false; loaderIndex = 0; sheet = "loader" }
    readonly property var loaderLabels: loaderCanGet ? ["Install", "Not now"] : [loaderSourceShown ? "OK" : "Show where to get it", "Close"]
    function loaderRun(i) {
        note("action", "Loader prompt: " + (loaderLabels[i] || i))
        if (i === 0 && loaderCanGet) getLoader()
        else if (i === 0 && !loaderSourceShown) loaderSourceShown = true
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
    function startPlay() {
        if (!game) return
        note("action", "Play " + game.name)
        if (!shot.on) { let p = parse(seen.played, []).filter(x => x !== game.id); p.unshift(game.id); seen.played = JSON.stringify(p.slice(0, 50)) }
        if (moving) endMove()
        // Play applies the waiting changes itself; its job runs on the worker after any apply already running, so
        // the timer must not start another one meanwhile.
        autoApply.stop()
        say("Checking before Play...", "work")
        Turbine.check()
    }
    readonly property string launchTitle: "One-time Steam setting for the mod loader"
    function launchSeen() { return game && parse(seen.launchDone, []).indexOf(game.id) >= 0 }
    function onceSeen(title) { return game && parse(seen.onceDone, []).indexOf(game.id + "|" + title) >= 0 }
    function checked(list) {
        if (launchSeen()) list = list.filter(x => x.title !== launchTitle)
        list = list.filter(x => !(x.once && x.title !== launchTitle && onceSeen(x.title)))
        if (list.length === 0) list = [{ level: "ok", title: "Ready to play", detail: "Everything checks out." }]
        checks = list
        note("engine", "Check before Play: " + list.map(x => x.level + " " + x.title).join("; "))
        note("engine", "Check before Play buttons: " + checkChoices.join("; "))
        const trouble = list.some(x => x.level === "stop" || x.level === "warn")
        if (msgLevel === "work" && msgText === "Checking before Play...") msgText = ""
        // Warnings already played through with these same mods: a note after launch instead of a stop.
        const warnKey = playKey(list)
        const known = !list.some(x => x.level === "stop") && parse(seen.playedWarn, []).indexOf(warnKey) >= 0
        if (trouble && known && shot.page.indexOf("check") !== 0) {
            note("engine", "Check before Play: the same warnings were played through with these mods: playing, then a note")
            Turbine.play(game.id)
            toast.info("Played with the same warnings as last time: " + list.filter(x => x.level === "warn").map(x => x.title).join("; "))
            return
        }
        if (trouble || shot.page === "check" || shot.page === "checkend" || shot.page === "checkfocus") {
            // The highlight starts on the fix; else Not now when something stops play; else Play anyway.
            checkIndex = checkOffer ? 0 : checkStops ? checkChoices.indexOf("Not now") : Math.max(0, checkChoices.indexOf("Play anyway"))
            sheet = "check"
        }
        else Turbine.play(game.id)
    }
    function playKey(list) {
        return (game ? game.id : "") + "|" + mods.filter(m => m.enabled).map(m => m.id).sort().join(",") + "|" + list.filter(x => x.level === "warn").map(x => x.title).sort().join(",")
    }
    readonly property bool checkStops: checks.some(x => x.level === "stop")
    // One fix the check can do in a press (Y): turn off the newer of two Witcher 3 mods that change the same script,
    // or install the missing loader (SKSE, BepInEx, SMAPI).
    readonly property var checkOffer: {
        const w = checks.find(x => x.offer && x.offer.mod && mods.some(m => m.id === x.offer.mod && m.enabled))
        if (w) return { label: w.offer.label, mod: w.offer.mod }
        if (loaderMissing && needLoader.length > 0) return { label: (loaderCanGet ? "Install " : "Get ") + loaderShort, loader: true }
        return null
    }
    // Fix it, Play anyway, Play without mods, Not now. A runs the highlighted one, B is Not now, Start Play anyway.
    readonly property var checkChoices: (checkOffer ? [checkOffer.label] : []).concat(checkStops ? [] : ["Play anyway"]).concat(["Play without mods", "Not now"])
    // Script paths from the last check (Game details shows them).
    readonly property var checkPaths: checks.filter(x => x.paths && x.paths.length > 0).map(x => x.paths.join(", "))
    property int checkIndex: 0
    function checkChoose(i) {
        const what = checkChoices[i]
        note("action", "Check before Play: " + what)
        sheet = ""
        if (checks.some(x => x.title === launchTitle) && what !== "Not now") {
            let done = parse(seen.launchDone, []); done.push(game.id); seen.launchDone = JSON.stringify(done)
        }
        // Other read-once lines (REDmod): seen once the player moves on from the check.
        const once = checks.filter(x => x.once && x.title !== launchTitle)
        if (once.length > 0 && what !== "Not now" && !shot.on) {
            let done = parse(seen.onceDone, []); for (const x of once) done.push(game.id + "|" + x.title); seen.onceDone = JSON.stringify(done)
        }
        if (what === "Play anyway") {
            if (!shot.on) { let k = parse(seen.playedWarn, []).filter(x => x.indexOf(game.id + "|") !== 0); k.push(playKey(checks)); seen.playedWarn = JSON.stringify(k.slice(-50)) }
            Turbine.play(game.id)
        }
        else if (what === "Play without mods") Turbine.playOriginal()
        else if (checkOffer && what === checkOffer.label) runOffer(checkOffer)
    }
    function runOffer(o) {
        if (o.mod) {
            // Turned off, then checked again: Play goes on when nothing else is in the way.
            const m = mods.find(x => x.id === o.mod)
            if (m && m.enabled) toggle(m, true)
            startPlay()
        } else if (o.loader) getLoader()
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
    property string playGame: ""
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
        const shortName = i === current ? gameTitle : games[i].name
        if (i !== current) current = i
        note("launch", games[i].name + " closed after " + secs + " seconds: offering a way back")
        const onNow = mods.filter(m => m.enabled)
        const key = list => list.slice().sort().join("|")
        const nowKey = key(onNow.map(m => m.id))
        // The setup that worked: the newest played one that isn't what just closed.
        const worked = setups.filter(x => x.played === true && key(x.on || []) !== nowKey).sort((a, b) => b.savedAt - a.savedAt)[0] || null
        const added = worked ? onNow.filter(m => (worked.on || []).indexOf(m.id) < 0) : []
        if (!shot.on) Turbine.markNotWorking()
        quick = { game: gameId, name: shortName, seconds: secs, added: added, worked: worked }
        page = "game"; zone = "main"; tab = 0; modPage = false
        if (app.active && sheet === "") { infoIndex = 0; sheet = "quick" }
    }
    readonly property string quickText: !quick ? "" : quick.added.length === 1 ? "Since it last worked, you added " + quick.added[0].name + "."
        : quick.added.length > 1 ? "Since it last worked (" + when(quick.worked.savedAt) + "), you added or turned on " + nameList(quick.added.map(m => m.name)) + "."
        : "A mod may have stopped it from starting. Playing without mods shows whether the game itself is fine."
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

    // ---------------------------------------------------------------- game menu
    // The setup to go back to: the newest one the game was played with (the engine's restore target).
    readonly property var lastGood: {
        const played = setups.filter(x => x.played === true).sort((a, b) => b.savedAt - a.savedAt)
        return played.length > 0 ? played[0] : null
    }
    function when(sec) { return Qt.formatDateTime(new Date(sec * 1000), "MMM d, h:mm AP") }
    // Reset shows only when the engine found records it couldn't read for this game. It runs after A is held 5 s.
    readonly property bool canReset: game !== null && Turbine.resetFor === game.id
    property real resetHeld: 0
    Timer {
        id: resetHold
        interval: 100
        repeat: true
        onTriggered: {
            app.resetHeld += 0.1
            if (app.resetHeld >= 5) { stop(); app.resetHeld = 0; app.sheet = ""; Turbine.resetGame() }
        }
    }
    function stopReset() { resetHold.stop(); resetHeld = 0 }
    readonly property var gameMenuItems: [
        { icon: "play", title: "Play without mods", detail: "Starts the game with no mods. Nothing is deleted.", ok: true },
        { icon: "back", title: "Restore last working setup",
          detail: lastGood ? "From " + when(lastGood.savedAt) + " · " + plural(lastGood.modsOn, "mod", "mods") + " on" : "Nothing saved yet. A setup is saved each time you play with mods.",
          ok: lastGood !== null }
    ].concat(canReset ? [
        { icon: "restore", title: "Reset game files", act: "reset", danger: true, ok: true,
          detail: resetHold.running ? "Keep holding A: " + Math.max(1, Math.ceil(5 - resetHeld)) + "..."
                  : "Hold A for 5 seconds. Takes every mod file out and puts the game's own files back. Mods stay listed, turned off." }
    ] : []).concat(loaderMissing ? [
        { icon: "update", title: (loaderCanGet ? "Install " : "Get ") + loaderShort, detail: needLoader.length > 0 ? plural(needLoader.length, "mod needs", "mods need") + " it" : "The mod loader for " + gameTitle, ok: true, act: "loader" }
    ] : []).concat(setups.length > 1 ? [
        { icon: "restore", title: "Go back to an earlier setup", detail: plural(setups.length, "setup", "setups") + " saved, newest first", ok: true, act: "setups" }
    ] : []).concat(kcdRestore ? [
        { icon: "restore", title: "Put back my mod_order.txt", detail: "As it was before Turbine changed it. Turbine re-adds its own mods that are on at your next change or Play.", ok: true, act: "kcdrestore" }
    ] : []).concat([
        { icon: "profile", title: "Loadouts", detail: profileData && profileList.length > 1 ? "In use: " + profileData.active : "Save setups by name, switch, share", ok: true, act: "profiles" }
    ]).concat(overlapCount > 0 ? [
        { icon: "info", title: "Overlaps: " + plural(overlapCount, "mod shares", "mods share") + " files", detail: "Which mod's copy the game gets", ok: true, act: "overlaps" }
    ] : []).concat(updateCount > 0 ? [
        { icon: "update", title: "Update all", detail: plural(updateCount, "mod has", "mods have") + " a newer version", ok: true, act: "updateall" }
    ] : []).concat([
        { icon: "info", title: "Game details", detail: "What Turbine found for this game", ok: true, act: "details" },
        { icon: "info", title: "Activity log", detail: "What Turbine did, to read, copy or save", ok: true, act: "activity" },
        { icon: "info", title: "Help", detail: "The buttons, and the beginner tutorial again", ok: true, act: "help" }
    ])
    // Every saved setup, newest first ("Nov 4, 9:12 PM · 16 mods on · Played"); A puts one back.
    function openSetups() {
        const list = setups.slice().sort((a, b) => b.savedAt - a.savedAt)
        openPick("Go back to an earlier setup", gameTitle, list.map(x => ({
            icon: "restore", title: when(x.savedAt), detail: plural(x.modsOn, "mod", "mods") + " on" + (x.played ? " · Played" : ""),
            run: () => { note("action", "Go back to the setup of " + when(x.savedAt)); Turbine.restoreSetup(x.id) } })), "gamemenu")
    }
    function gameMenuRun(i) {
        const item = gameMenuItems[i]
        if (!item || !item.ok) return
        note("action", "Game menu: " + item.title)
        if (i === 0) { sheet = ""; Turbine.playOriginal() }
        else if (i === 1) { sheet = ""; Turbine.restoreSetup("") }
        else if (item.act === "reset") { if (resetHold.running) stopReset(); else { resetHeld = 0; resetHold.start() } }
        else if (item.act === "profiles") openProfiles()
        else if (item.act === "loader") { sheet = ""; getLoader() }
        else if (item.act === "kcdrestore") { sheet = ""; Turbine.kcdRestore() }
        else if (item.act === "setups") openSetups()
        else if (item.act === "overlaps") openOverlaps()
        else if (item.act === "activity") openActivity()
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
        const words = "New: " + name + (on ? " (on)" : " (off)") + ". Start: Play"
        // The build's pictures add their test mods this way: only the new-mod picture (and the press count) follow them.
        if (shot.on && shot.page !== "newmod" && shot.page !== "presscount") return
        // Installed from the mod page of the game on screen: stay on the page (its button says Installed).
        if (modPage && game && game.id === gameId) { busyHere = false; toast.done(words); zone = "play"; note("focus", "New mod: focus on Play"); showChain(name); return }
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
    function missingNeeds(m) { const l = linksMap[m.id]; return l ? (l.needs || []).filter(n => !n.id || !mods.some(x => x.id === n.id && x.enabled)) : [] }
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
    readonly property var modMenuItems: {
        if (!menuMod) return []
        const first = (menuMod.needsLoader && loaderMissing ? [
            { icon: "update", title: "Get " + loaderShort, detail: loaderCanGet ? "This mod needs it. Turbine gets it from its official page." : "This mod needs it. Shows where it comes from.", act: "loader" }
        ] : []).concat(ordered && mods.length > 1 ? [
            { icon: "move", title: "Move", detail: "D-pad moves it, LT and RT to the top or bottom, A puts it down", act: "move" }
        ] : []).concat(updateMap[menuMod.id] ? [
            { icon: "update", title: "Update to " + updateMap[menuMod.id].to, detail: (updateMap[menuMod.id].from ? "You have " + updateMap[menuMod.id].from + ". " : "") + "Keeps its place and on or off", act: "update" }
        ] : [])
        const room = Math.max(0, 6 - first.length - 2)
        return first.concat(modMenuFacts.slice(0, room)).concat([
            { icon: "info", title: "Details", detail: [modWords(menuMod), menuMod.enabled ? "On" : "Off"].filter(x => x !== "").join(" · "), act: "details" },
            { icon: "remove", title: "Uninstall", detail: "The row stays for 8 seconds: A puts it back. Then kept in Removed mods for 7 days.", act: "remove", danger: true }
        ])
    }
    // Details: everything about the mod in one list (and Put it after... for ordered games).
    function openModDetails(m) {
        openPick(m.name, [modWords(m), m.enabled ? "On" : "Off"].filter(x => x !== "").join(" · "), modMenuFacts.map(f => ({
            icon: f.icon, title: f.title, detail: f.detail, run: () => {
                if (f.act === "overlap") openOverlap(m, f.other, f.loses)
                else if (f.act === "needs") openNeeds(m)
                else if (f.act === "neededby") openNeededBy(m)
                else if (f.act === "check") startPlay()
            } })).concat(ordered ? [
            { icon: "move", title: "Put it after...", detail: "Type part of another mod's name; it goes just after that one",
              run: () => { putAfterMod = m; openAsk("putafter", "Put " + m.name + " after...", "Part of the name of the mod it goes after.", "", "Mod name", 60, false, "Move it") } }
        ] : []).concat(modMenuFacts.length === 0 && !ordered ? [{ icon: "info", title: "Nothing else to know", detail: "", run: () => {} }] : []), "")
    }
    property int menuIndex: 0
    function modMenuRun(i) {
        const item = modMenuItems[i], m = menuMod
        if (!item || !m) return
        note("action", "Mod menu: " + item.title + " for " + m.name)
        sheet = ""
        if (item.act === "toggle") toggle(m)
        else if (item.act === "loader") getLoader()
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
    }
    function modWords(m) {
        let parts = []
        if (m.version) parts.push(m.version)
        // Files added by hand say nothing more ("Zip file" on every row told nothing).
        if (m.source && m.source !== "Zip file") parts.push(m.source)
        return parts.join(" · ")
    }

    // ---------------------------------------------------------------- Add mods from Downloads
    property int addIndex: 0
    function openAdd() { Turbine.listDownloads(); addIndex = 0; addAll = false; sheet = "add" }
    function addChosen() {
        const d = addList[addIndex]
        sheet = ""
        if (!d) return
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
        if (b >= 1024) return Math.round(b / 1024) + " KB"
        return b + " bytes"
    }

    // ---------------------------------------------------------------- controller and keyboard
    // zone: where the controller focus is. "rail" (games and buttons on the left), "main" (the game page),
    // "settings", "perf". sheet: an overlay on top (details, notice, modmenu, gamemenu, add, check, loader).
    property string page: "game"          // game, settings, perf
    property string zone: "main"
    property string sheet: ""
    property int railIndex: 0             // 0..games.length-1, then "add", then "settings"
    property int tab: 0                   // 0 Mods, 1 Get Mods, 2 Load order
    property int modIndex: 0
    property int setSection: 0            // 0 Look, 1 About
    property int setRow: 0
    property int perfIndex: 0
    readonly property int railCount: games.length + 1   // the games, then Settings
    // Sheets open from a press. The ones that come from outside Turbine (a download that arrived, an installer's
    // questions, install notes) and the first-run tutorial say why; anything else with no press near it is a bug.
    readonly property var sheetReasons: ({ found: "a download arrived", fomod: "the installer asks", notes: "install notes", chain: "the next requirement",
                                           tutorial: "first run", quick: "the game closed early", oldhelper: "the browser helper is old" })
    onSheetChanged: {
        if (sheet === "") note("page", "Sheet closed")
        else if (pressRecent() || (shot.on && !shot.strictSheets)) note("page", "Sheet " + sheet + " (after " + (lastInputName || "start") + ")")
        else if (sheetReasons[sheet]) note("page", "Sheet " + sheet + " (" + sheetReasons[sheet] + ")")
        else note("page", "Sheet " + sheet + " opened with no press")
        menuIndex = 0; stopReset(); stopConfirm()
        if (sheet !== "") closeWheel()
        pickHover()
        if (sheet === "" && fomod) sheet = "fomod"
        else if (sheet === "" && found) { infoIndex = 0; sheet = "found" }
        else if (sheet === "" && pendingNotes) { pendingNotes = false; infoIndex = 0; sheet = "notes" }
        else if (sheet === "") offerLoader()
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
            const n = Date.now() - app.repeatSince > 1500 && (app.repeatButton === "up" || app.repeatButton === "down") ? 5 : 1
            for (let i = 0; i < n; i++) app.act(app.repeatButton)
        }
    }
    // Hold B for 1 second: every sheet and page closes, back to the game's list.
    Timer {
        id: homeHold
        interval: 1000
        onTriggered: app.goHome()
    }
    function goHome() {
        note("action", "Hold B: home")
        if (sheet === "ask") askView.cancel ? askView.cancel() : null
        sheet = ""; moving = false; orderMoving = false; modPage = false; history = []
        page = "game"; zone = "main"; tab = 0; modIndex = 0
    }
    // Hold Start (0.6 s): Play options. A short press is Play, as always.
    Timer { id: startHold; interval: 600; onTriggered: app.openPlayOptions() }
    // Every press counts as input: a sheet that opens with no press near it is a bug (logged, and checked by the build).
    property int inputCount: 0
    property real lastInputAt: 0
    property string lastInputName: ""
    function pressRecent() { return Date.now() - lastInputAt < 2500 }
    // The back grips (L4 L5 R4 R5) send nothing in Steam's default layout; when a layout sends them: L4/R4 are LT/RT,
    // L5 is Y and R5 is View. They are never in the legend.
    function press(b) {
        inputCount++; lastInputAt = Date.now(); lastInputName = b
        if (lastInput === "touch") lastInput = "pad"
        const grip = { l4: "lt", r4: "rt", l5: "y", r5: "view" }
        if (grip[b]) b = grip[b]
        // R3 here means the pointer was hidden: it shows at the focused item and clicks it, the same as A.
        if (b === "r3") { r3Click(); return }
        if (b === "start" && sheet === "" && !wheelOpen && !moving && !orderMoving && page === "game" && game) { startHold.restart(); return }
        if (["up", "down", "left", "right"].indexOf(b) >= 0) {
            repeatButton = b
            repeatSince = Date.now()
            repeater.interval = 380
            repeater.restart()
        }
        if (b === "view" && sheet === "" && !moving && !orderMoving) { wheelPress(); return }
        if (b === "b") homeHold.restart()
        if (b === "a" && sheet === "confirm" && confirmIndex === 0) { startConfirm(); return }
        act(b)
    }
    function release(b) {
        const grip = { l4: "lt", r4: "rt", l5: "y", r5: "view" }
        if (grip[b]) b = grip[b]
        if (b === "start" && startHold.running) { startHold.stop(); act("start"); return }
        if (b === repeatButton) { repeater.stop(); repeatButton = "" }
        if (b === "a" && resetHold.running) stopReset()
        if (b === "a" && confirmHold.running) stopConfirm()
        if (b === "b") homeHold.stop()
        if (b === "view") wheelRelease()
    }
    // R3 with the pointer hidden: the pointer appears on the focused item and A runs (a click there does the same).
    function r3Click() {
        const p = focusCenter()
        Turbine.pointerShowAt(p.x, p.y)
        note("pad", "R3: pointer shown on the focus, same as A")
        act("a")
    }
    // The middle of whatever has the focus ring, in window pixels.
    function focusCenter() {
        let it = null
        try {
            if (sheet === "" && !wheelOpen) {
                if (zone === "rail") it = railIndex < games.length ? railList.itemAtIndex(railIndex) : gear
                else if (zone === "play") it = playBtn
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
            app.confirmHeld += 0.1
            if (app.confirmHeld >= app.holdNeeded) { stop(); app.confirmHeld = 0; app.confirmDone() }
        }
    }
    function startConfirm() { if (!confirmHold.running) { confirmHeld = 0; confirmHold.start() } }
    function stopConfirm() { confirmHold.stop(); confirmHeld = 0 }

    Item {
        id: keyRoot
        anchors.fill: parent
        focus: true
        // A real mouse (not the controller pointer, which Turbine draws itself).
        HoverHandler { onPointChanged: if (!Turbine.pointerShown && app.lastInput !== "mouse") app.lastInput = "mouse" }
        Keys.onPressed: (e) => {
            const map = {}
            map[Qt.Key_Up] = "up"; map[Qt.Key_Down] = "down"; map[Qt.Key_Left] = "left"; map[Qt.Key_Right] = "right"
            map[Qt.Key_Return] = "a"; map[Qt.Key_Enter] = "a"; map[Qt.Key_Space] = "a"
            map[Qt.Key_Escape] = "b"; map[Qt.Key_Backspace] = "b"
            // With no pad seen directly, Steam's desktop layout drives Turbine as a keyboard: Enter A, Escape or
            // Backspace B, Tab X, arrows the D-pad, Page Up and Page Down LT and RT.
            map[Qt.Key_X] = "x"; map[Qt.Key_Tab] = "x"; map[Qt.Key_Y] = "y"
            map[Qt.Key_PageUp] = "lt"; map[Qt.Key_PageDown] = "rt"
            map[Qt.Key_Q] = "lb"; map[Qt.Key_E] = "rb"
            map[Qt.Key_P] = "start"; map[Qt.Key_V] = "view"; map[Qt.Key_F] = "y"
            const b = map[e.key]
            if (!b) return
            e.accepted = true
            if (Turbine.pads && app.lastInput !== "keys") app.lastInput = "keys"
            if (!e.isAutoRepeat) app.note("key", "Key " + b)
            app.inputCount++; app.lastInputAt = Date.now(); app.lastInputName = b
            if (b === "start" || b === "view") { if (!e.isAutoRepeat) app.press(b); return }
            if (b === "b" && !e.isAutoRepeat) homeHold.restart()
            // Holding a key repeats it, except A (Reset waits for A to be held down).
            if (b === "a" && app.sheet === "confirm" && app.confirmIndex === 0) { if (!e.isAutoRepeat) app.startConfirm(); return }
            if (!(b === "a" && e.isAutoRepeat)) app.act(b)
        }
        Keys.onReleased: (e) => {
            if (!e.isAutoRepeat && (e.key === Qt.Key_Return || e.key === Qt.Key_Enter || e.key === Qt.Key_Space)) app.release("a")
            if (!e.isAutoRepeat && e.key === Qt.Key_V) app.release("view")
            if (!e.isAutoRepeat && e.key === Qt.Key_P) app.release("start")
            if (!e.isAutoRepeat && (e.key === Qt.Key_Escape || e.key === Qt.Key_Backspace)) homeHold.stop()
        }
    }

    // Sheets with a row of buttons (SheetButtons): the D-pad moves the highlight, A presses it, B is the cancel
    // button, Start the main one. No other keys. {labels, index, main, cancel, set(i), run(i), scrolls, scroll(d)}.
    readonly property var infoSheets: ["nobrowser", "helper", "notes", "found", "quick", "chain", "addbrowser", "oldhelper"]
    function buttonSheet() {
        const s = sheet
        if (s === "check") return { labels: checkChoices, index: checkIndex, main: checkStops ? -1 : checkChoices.indexOf("Play anyway"), cancel: checkChoices.indexOf("Not now"),
                                    set: i => { checkIndex = i }, run: i => checkChoose(i), scrolls: checkView.scrolls, scroll: d => checkView.scroll(d) }
        if (s === "loader") return { labels: loaderLabels, index: loaderIndex, main: 0, cancel: 1, set: i => { loaderIndex = i }, run: i => loaderRun(i), scrolls: false }
        if (infoSheets.indexOf(s) >= 0) {
            const cancel = infoButtons.findIndex(x => x.key === "b")
            return { labels: infoButtons.map(x => x.label), index: infoIndex, main: 0, cancel: cancel, set: i => { infoIndex = i }, run: i => infoRun(i),
                     scrolls: infoView.scrolls, scroll: d => infoView.scrollBy(d) }
        }
        if (s === "ask") return { labels: ["Clear", "Cancel", askView.doneLabel], index: askIndex, main: 2, cancel: 1, set: i => { askIndex = i }, run: i => askRun(i), scrolls: false }
        if (s === "confirm") return { labels: ["Delete", "Keep"], index: confirmIndex, main: -1, cancel: 1, set: i => { confirmIndex = i },
                                      run: i => { if (i === 1) { stopConfirm(); sheet = confirmWhat.indexOf("profile:") === 0 ? "profiles" : removedMods.length > 0 ? "removed" : "" } }, scrolls: false }
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
        if ((b === "up" || b === "down") && bs.scrolls) bs.scroll(b === "down" ? 1 : -1)
        else if (b === "left" || b === "up") bs.set(Math.max(0, bs.index - 1))
        else if (b === "right" || b === "down") bs.set(Math.min(n - 1, bs.index + 1))
        else if (b === "a") { if (sheet === "ask" && askIndex < 0) askKeyboard(); else if (sheet !== "confirm" || bs.index === 1) runIt(Math.min(bs.index, n - 1)) }
        else if (b === "b") { if (bs.cancel >= 0) runIt(bs.cancel); else if (!sheetDry) sheet = "" }
        else if (b === "start") runIt(bs.main)
    }
    function sheetBinds() {
        const bs = buttonSheet()
        if (bs) {
            const hl = sheet === "ask" && askIndex < 0 ? (Turbine.gamemode ? "Keyboard" : "") : sheet === "confirm" && confirmIndex === 0 ? "Delete" : bs.labels[bs.index] || ""
            let o = { a: bind(hl, () => sheetAct("a"), sheet === "confirm" && confirmIndex === 0 ? "hold" : ""), b: bind(bs.cancel >= 0 ? bs.labels[bs.cancel] : "Close", () => sheetAct("b")) }
            if (bs.main >= 0) o.start = bind(bs.labels[bs.main], () => sheetAct("start"))
            return o
        }
        const s = sheet
        const k = key => () => sheetAct(key)
        if (s === "modmenu" || s === "gamemenu" || s === "cardmenu") return { a: bind("Choose", k("a")), b: bind("Close", k("b")), start: bind("Play", k("start")) }
        if (s === "pick") return { a: bind("Choose", k("a")), b: bind(pickBack !== "" ? "Back" : "Close", k("b")) }
        if (s === "removed") return { a: bind(removedIndex === 0 ? "Delete all" : "Put back", k("a")), b: bind("Close", k("b")), x: bind(removedIndex === 0 ? "" : "Delete now", k("x")), start: bind("Play", k("start")) }
        if (s === "add") return { a: bind("Add", k("a")), b: bind("Close", k("b")), y: bind(addAll ? "This game's files" : "All files", k("y")) }
        if (s === "shares") return { a: bind("Read", k("a")), b: bind("Back", k("b")) }
        if (s === "profiles") return { a: bind("Choose", k("a")), b: bind("Close", k("b")), x: bind(!importData && profIndex < profileList.length ? "Options" : "", k("x")) }
        if (s === "pages") return { a: bind("Open", k("a")), b: bind("Close", k("b")) }
        if (s === "versions") return { a: bind(detailsData && detailsData.premium === false ? "Get it on Nexus" : "Install", k("a")), b: bind("Close", k("b")) }
        if (s === "code") return { a: bind(codeData && codeData.state === "ready" ? "Copy" : "", k("a")), b: bind("Back", k("b")) }
        if (s === "plugin") return { a: bind(pluginOwner ? "Go to its mod" : "", k("a")), b: bind("Close", k("b")) }
        if (s === "pickgame") return { a: bind("Add here", k("a")), b: bind("Back", k("b")) }
        if (s === "fomod") return { a: bind("Choose", k("a")), b: bind("Back a step", k("b")), start: bind("Next", k("start")) }
        if (s === "report") return { b: bind(reportView.editing ? "Done typing" : "Close", k("b")) }
        return { a: bind("", k("a")), b: bind("Close", k("b")) }
    }
    function sheetAct(b) {
        const bs = buttonSheet()
        if (bs) { buttonSheetAct(bs, b); return }
        if (sheet === "report") reportView.handleButton(b)
        else if (sheet === "perf") perfView.handleButton(b)
        else if (sheet === "activity") activityView.handleButton(b)
        else if (sheet === "details" || sheet === "notice") {
            if (b === "a" || b === "b" || b === "view") {
                if (sheet === "notice") seen.notSteamOS = true
                sheet = ""
            }
        } else if (["modmenu", "gamemenu", "cardmenu", "removed"].indexOf(sheet) >= 0 && b === "start") {
            // Start plays from any menu (the legend shows it while a menu covers the Play button).
            stopReset(); sheet = ""; startPlay()
        } else if (sheet === "pick") {
            if (b === "up") menuIndex = Math.max(0, menuIndex - 1)
            else if (b === "down") menuIndex = Math.min(pickItems.length - 1, menuIndex + 1)
            else if (b === "a") pickRun(menuIndex)
            else if (b === "b" || b === "x") sheet = pickBack
        } else if (sheet === "modmenu") {
            if (b === "up") menuIndex = Math.max(0, menuIndex - 1)
            else if (b === "down") menuIndex = Math.min(modMenuItems.length - 1, menuIndex + 1)
            else if (b === "a") modMenuRun(menuIndex)
            else if (b === "b" || b === "x") sheet = ""
        } else if (sheet === "gamemenu") {
            if (b === "up") { stopReset(); menuIndex = Math.max(0, menuIndex - 1) }
            else if (b === "down") { stopReset(); menuIndex = Math.min(gameMenuItems.length - 1, menuIndex + 1) }
            else if (b === "a") gameMenuRun(menuIndex)
            else if (b === "b" || b === "view" || b === "x") sheet = ""
        } else if (sheet === "add") {
            if (b === "up") addIndex = Math.max(0, addIndex - 1)
            else if (b === "down") addIndex = Math.min(addList.length - 1, addIndex + 1)
            else if (b === "a") addChosen()
            // Y: every mod file, not only the ones for this game.
            else if (b === "y") { addAll = !addAll; addIndex = 0; note("action", addAll ? "Add: all files" : "Add: this game's files") }
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
            else if (b === "b" || b === "x" || b === "a") sheet = ""
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
            else if (b === "b") sheet = "profiles"
        }
    }
    // Turbine can fetch the loader itself (SKSE, BepInEx, SMAPI).
    readonly property bool loaderCanGet: !!rules && !!rules.loader && ["bethesda", "bepinex", "smapi"].indexOf(rules.kind) >= 0

    // ---------------------------------------------------------------- one table for every button
    // binds() says what each button does right now: {a, b, x, y, start, lb, rb, lt, rt, view} of {label, run, pre}.
    // act() runs it and the legend prints it, so the legend can't say something the button doesn't do. The D-pad
    // and the left stick move the focus (navigate()); they are never in the legend.
    function bind(label, run, pre) { return { label: label, run: run, pre: pre || "" } }
    function act(b) {
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
        let o = {}
        if (sheet === "tutorial") return { a: bind(tutCard < tutCount - 1 ? "Next" : "Pick a game", () => tutorialAct("a")), b: bind("Skip", () => tutorialAct("b")), start: bind("", () => tutorialAct("start")) }
        if (wheelOpen) {
            const s = wheelSlot >= 0 ? wheelSlots[wheelSlot] : null
            return { a: bind(s ? (s.ok ? s.label : "") : (labsOn ? "Report" : ""), () => wheelAct("a")), b: bind("Close", () => wheelAct("b")), view: bind("", () => wheelAct("b")) }
        }
        if (sheet !== "") return sheetBinds()
        if (moving || orderMoving) return { a: bind("Done", () => act("a")), b: bind("Cancel", () => act("b")), lt: bind("", () => act("lt")), rt: bind("", () => act("rt")) }
        o.view = bind("", wheelPress)
        if (page === "game" && game) o.start = bind("", startPlay)
        if (zone === "rail") {
            const g = railIndex < games.length
            o.a = bind(g ? "Open" : "Settings", () => railSelect(railIndex))
            if (page !== "game" || game) o.b = bind("Back", () => { zone = page === "settings" ? "settings" : page === "perf" ? "perf" : "main" })
            if (g) o.x = bind("Options", openRailMenu)
            o.lt = bind("", () => { railIndex = Math.max(0, railIndex - 5) })
            o.rt = bind("", () => { railIndex = Math.min(railCount - 1, railIndex + 5) })
            return o
        }
        if (zone === "play" && page === "game" && game) {
            o.a = bind("Play", startPlay)
            o.b = bind("Back", () => { zone = "main"; note("focus", "Back from Play to the list") })
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
            const rows = sec === "Look" ? lookRows.length : sec === "About" ? 2 : rowsNow.length
            o.lt = bind("", () => { setRow = Math.max(0, setRow - pageJump) })
            o.rt = bind("", () => { setRow = Math.min(rows - 1, setRow + pageJump) })
            return o
        }
        if (page === "perf") {
            o.b = bind("Back", () => { page = "settings"; zone = "settings" })
            o.lt = bind("", () => { perfIndex = Math.max(0, perfIndex - pageJump) })
            o.rt = bind("", () => { perfIndex = Math.min(1999, perfIndex + pageJump) })
            return o
        }
        if (!game) { o.a = bind("Look again", load); return o }
        if (modPage) {
            o.a = bind(pageALabel, () => modPageAct("a"))
            o.b = bind("Back", getBack)
            if (pageMenuItems.length > 0) o.x = bind("Options", () => { menuIndex = 0; openPick(detailsData && detailsData.name ? detailsData.name : "This mod", "", pageMenuItems, "") })
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
            o.y = bind("Find", openFind)
            o.lt = bind("", () => { modIndex = Math.max(0, modIndex - pageJump) })
            o.rt = bind("", () => { modIndex = Math.min(listItems.length - 1, modIndex + pageJump) })
            if (modIndex < 0) {
                if (listBar[listBarIndex]) o.a = bind("Choose", () => listBarRun(listBarIndex))
                o.b = bind("Back", () => { modIndex = 0 })
                return o
            }
            const it = focusedItem
            if (it && it.kind === "mod") {
                o.a = bind(it.m.enabled ? "Turn off" : "Turn on", () => toggle(it.m))
                o.x = bind("Options", () => { menuIndex = 0; sheet = "modmenu" })
            } else if (it && it.kind === "ghost") o.a = bind("Put back", () => putBack(it.m.id))
            else if (it && it.kind === "removed") {
                o.a = bind("Open", openRemoved)
                o.x = bind("Options", openRemoved)
            }
            o.b = bind(listFiltered ? "Show all" : "Games", () => {
                if (listFiltered) { findText = ""; listShow = "all"; listSort = "order"; modIndex = 0; note("action", "Find, Show and Sort cleared") }
                else { zone = "rail"; railIndex = current }
            })
            return o
        }
        if (tab === 1 && sources.length === 0) {
            o.a = bind("Add a file", openAdd)
            o.b = bind("My mods", () => { tab = 0 })
            return o
        }
        if (tab === 1) {
            o.b = bind(getArea === "bar" ? (getText !== "" ? "Clear search" : "My mods") : getArea === "fallback" ? "My mods" : "Back", getBack)
            if (getArea === "fallback") {
                o.a = bind("Open on Nexus", openNexusMods)
                o.x = bind("Try again here", retryBrowse)
                return o
            }
            o.y = bind("Search", () => getBarRun(Math.max(0, getBar.findIndex(x => x.key === "search"))))
            if (getArea === "bar") o.a = bind("Choose", () => getBarRun(getIndex))
            else if (getArea === "recent") o.a = bind("Open", () => getAct("a"))
            else {
                if (getItems[getIndex]) o.a = bind("Open", () => getAct("a"))
                if (getItems[getIndex]) o.x = bind("Options", () => { menuIndex = 0; sheet = "cardmenu" })
                // A screen of rows; the rows stop at both ends.
                const jump = gridColumns * 2
                o.lt = bind("", () => { getIndex = Math.max(getIndex % gridColumns, getIndex - jump) })
                o.rt = bind("", () => { getIndex = Math.min(getItems.length - 1, getIndex + jump); if (getIndex >= getItems.length - gridColumns) browseMore() })
            }
            return o
        }
        // Load order.
        const p = orderPlugin
        if (orderIndex === -1) o.a = bind("Sort", () => orderAct("a"))
        else if (p && !p.fixed) o.a = bind(p.active ? "Turn off" : "Turn on", () => orderAct("a"))
        o.b = bind("My mods", () => { tab = 0 })
        if (p) o.x = bind("Options", openPluginMenu)
        o.y = bind("Find plugin", openFindPlugin)
        o.lt = bind("", () => { orderIndex = Math.max(-1, orderIndex - pageJump) })
        o.rt = bind("", () => { orderIndex = Math.min(order.length - 1, orderIndex + pageJump) })
        return o
    }
    // The legend: A, B, X, Y as binds() has them, then one more: the pointer's buttons while it shows, else Start
    // when a sheet covers the Play button. LB/RB, View and Start are caps on the tab bar, the wheel button and Play.
    function legendFor(o) {
        let h = []
        for (const k of ["a", "b", "x", "y"]) if (o[k] && o[k].label) h.push([k, o[k].label, o[k].pre || ""])
        if (Turbine.pointerShown && !keyboardUp) h.push(["rt", "Click", "", "lt", "Menu"])
        else if (o.start && o.start.label) h.push(["start", o.start.label, ""])
        return h.slice(0, 5)
    }
    function navigate(b) {
        if (zone === "rail") {
            if (b === "up") railIndex = Math.max(0, railIndex - 1)
            else if (b === "down") railIndex = Math.min(railCount - 1, railIndex + 1)
            else if (b === "right") zone = page === "settings" ? "settings" : page === "perf" ? "perf" : "main"
            return
        }
        if (page === "settings") {
            const rows = sec === "Look" ? lookRows.length : sec === "About" ? 2 : rowsNow.length
            const r = sec === "Look" || sec === "About" ? null : rowsNow[setRow]
            const kind = r ? r.kind : ""
            if (b === "up") setRow = Math.max(0, setRow - 1)
            else if (b === "down") setRow = Math.min(rows - 1, setRow + 1)
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
        if (zone === "play") {
            if (b === "down") zone = "main"
            else if (b === "left") { zone = "rail"; railIndex = current }
            return
        }
        if (modPage) { modPageAct(b); return }
        if (tab === 1 && sources.length > 0) {
            // Up from the top of Get mods' bar: the Play button.
            if (b === "up" && getArea === "bar") { zone = "play"; return }
            getAct(b); return
        }
        if (tab === 2) {
            if (b === "up" && orderIndex === -1) { zone = "play"; return }
            orderAct(b); return
        }
        if (tab === 1 || listItems.length === 0) {
            if (b === "left") { zone = "rail"; railIndex = current }
            else if (b === "up") zone = "play"
            return
        }
        if (selecting) { if (b === "left") { selecting = false; ticked = {}; zone = "rail"; railIndex = current } else selectAct(b); return }
        if (modIndex < 0) {
            // The bar over the list (Find, Show, Sort, Select several).
            if (b === "left") { if (listBarIndex > 0) listBarIndex--; else { zone = "rail"; railIndex = current } }
            else if (b === "right") listBarIndex = Math.min(listBar.length - 1, listBarIndex + 1)
            else if (b === "down") modIndex = 0
            else if (b === "up") zone = "play"
            return
        }
        if (b === "up" && modIndex === 0) { modIndex = -1; listBarIndex = Math.min(listBarIndex, listBar.length - 1) }
        else if (b === "up") modIndex = Math.max(0, modIndex - 1)
        else if (b === "down") modIndex = Math.min(listItems.length - 1, modIndex + 1)
        else if (b === "left") { zone = "rail"; railIndex = current }
    }
    function switchTab(d) {
        const t = Math.max(0, Math.min(tabs.length - 1, tab + d))
        if (t === tab) return
        if (zone !== "main") zone = "main"
        tab = t
    }
    function settingsSection(d) {
        const s = Math.max(0, Math.min(settingsSections.length - 1, setSection + d))
        if (s !== setSection) { setSection = s; setRow = 0 }
    }
    function settingsA() {
        const r = sec === "Look" || sec === "About" ? null : rowsNow[setRow]
        const kind = r ? r.kind : ""
        if (sec === "Look") lookRows[setRow].step(1)
        else if (sec === "About") { if (setRow === 0) labsTap(); else { page = "perf"; zone = "perf"; perfIndex = 0 } }
        else if (kind === "item") labsChoose(r, ["works", "problem", ""][labsChip])
        else if (kind === "choice" || kind === "toggle") rowStep(r, 1)
        else if (kind === "action") rowRun(r)
    }
    // B from Settings: the screen and row it was opened from.
    property var settingsFrom: null
    function settingsBack() {
        const f = settingsFrom
        settingsFrom = null
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
            { icon: "info", title: "Hide from the rail", detail: "For tools that aren't games. Settings, Help shows them again.", run: () => setRailList("hidden", g.id, true) }
        ], "")
    }
    // Load order's quick menu (X): Move, Details, Go to its mod.
    function openPluginMenu() {
        const p = orderPlugin
        if (!p) return
        openPick(p.name, p.fixed ? "Always loaded by the game" : (p.active ? "On" : "Off"), (p.fixed ? [] : [
            { icon: "move", title: "Move", detail: "D-pad moves it, A puts it down", run: () => { orderMoving = true; orderFrom = orderIndex; orderSaved = order.slice() } }
        ]).concat([
            { icon: "info", title: "Details", detail: "What LOOT says about it", run: () => { note("action", "Plugin details: " + p.name); sheet = "plugin" } }
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
    function railName(i) {
        const g = games[i]
        if (!g) return ""
        const twins = games.filter(x => x.name === g.name).length > 1
        return g.name + (twins ? " · " + (g.launcher === "Steam" ? "Steam" : (g.storeLabel || g.launcher) + " (Heroic)") : "")
    }
    function railSelect(i) {
        note("action", "Rail " + (i < games.length ? games[i].name : "Settings"))
        if (i < games.length) { current = i; page = "game"; zone = "main" }
        else openSettings()
    }
    // ---------------------------------------------------------------- the quick wheel (View)
    // Eight fixed slots, the same places on every screen; a slot with nothing to do here is dimmed, never moved.
    // Tap View: it stays open, the D-pad picks (Up then Right is Up-right), A does it, B closes. Or hold View, push a
    // direction and let go of View. In Labs the middle takes the report shot.
    property bool wheelOpen: false
    property int wheelSlot: -1            // 0 Up, 1 Up-right, 2 Right, 3 Down-right, 4 Down, 5 Down-left, 6 Left, 7 Up-left
    property bool wheelHeld: false
    property bool wheelPicked: false
    readonly property string wheelKind: !game || page !== "game" ? "none" : tab === 1 || modPage ? "get" : tab === 2 ? "order" : "mods"
    function slot(label, ok, run) { return { label: label, ok: ok, run: run } }
    readonly property var wheelSlots: {
        const g = !!game
        const k = wheelKind
        const common = [slot("Play options", g, openPlayOptions), slot("Full menu", g, () => { menuIndex = 0; sheet = "gamemenu" }),
                        slot("Settings", true, openSettings), slot("Games", games.length > 0, openGamesPick)]
        if (k === "get") return [slot("Search", sources.length > 0, () => { if (modPage) getBack(); getBarRun(Math.max(0, getBar.findIndex(x => x.key === "search"))) }),
                                 slot("Sort", sources.length > 0, () => pickBar("sort")),
                                 slot("Category", getCats.length > 0, () => pickBar("category"))].concat(common).concat([slot("Add a file", g, openAdd)])
        if (k === "order") return [slot("Find plugin", order.length > 0, openFindPlugin), slot("Sort", order.length > 0, () => { orderIndex = -1; Turbine.orderSort() }),
                                   slot("Show", false, () => {})].concat(common).concat([slot("Loadouts", g, openProfiles)])
        return [slot("Find", k === "mods" && mods.length > 0, openFind), slot("Sort", k === "mods" && mods.length > 1, () => listBarRunKey("sort")),
                slot("Show", k === "mods" && mods.length > 0, () => listBarRunKey("show"))].concat(common)
                .concat([slot(updateList.length > 0 ? "Updates (" + updateList.length + ")" : "Updates", k === "mods" && updateList.length > 0, openUpdates)])
    }
    function listBarRunKey(key) {
        const keys = key === "show" ? showKeys : sortKeys2, names = key === "show" ? showNames : sortNames2
        const now = keys.indexOf(key === "show" ? listShow : listSort)
        openPick(key === "show" ? "Show" : "Sort", gameTitle, keys.map((kk, i) => ({
            icon: i === now ? "power" : "info", title: names[kk], detail: i === now ? "Showing now" : "",
            run: () => { if (key === "show") listShow = kk; else listSort = kk; note("action", "My mods: " + key + " " + names[kk]) } })), "")
        menuIndex = now
    }
    function wheelPress() {
        if (wheelOpen) { closeWheel(); return }
        wheelOpen = true; wheelSlot = -1; wheelHeld = true; wheelPicked = false
        note("action", "Quick wheel open (" + wheelKind + "): " + wheelSlots.map(x => x.label + (x.ok ? "" : " (dim)")).join("; "))
    }
    function wheelRelease() {
        if (!wheelHeld) return
        wheelHeld = false
        // Held, pushed, let go: that slot.
        if (wheelOpen && wheelPicked && wheelSlot >= 0) wheelFire(wheelSlot)
    }
    function closeWheel() { if (wheelOpen) { wheelOpen = false; wheelHeld = false; note("action", "Quick wheel closed") } }
    function wheelMove(b) {
        const card = { up: 0, right: 2, down: 4, left: 6 }[b]
        if (card === undefined) return
        const now = wheelSlot
        if (now < 0 || now % 2 === 1 || now === card || (now + 4) % 8 === card) wheelSlot = card
        else wheelSlot = (now === 0 && card === 6) || (now === 6 && card === 0) ? 7 : (now + card) / 2
        wheelPicked = true
    }
    function wheelFire(i) {
        const s = wheelSlots[i]
        if (!s || !s.ok) { note("action", "Quick wheel: " + (s ? s.label : i) + " is dimmed here"); return }
        closeWheel()
        note("action", "Quick wheel: " + s.label)
        s.run()
    }
    function wheelAct(b) {
        if (["up", "down", "left", "right"].indexOf(b) >= 0) wheelMove(b)
        else if (b === "a") { if (wheelSlot >= 0) wheelFire(wheelSlot); else if (labsOn) { closeWheel(); quickReport() } }
        else if (b === "b" || b === "view") closeWheel()
        else if (b === "start") { closeWheel(); act("start") }
    }
    // The wheel's Games: every game, last played first.
    function openGamesPick() {
        openPick("Games", plural(games.length, "game", "games"), games.map((g, i) => ({
            icon: i === current ? "power" : "play", title: railName(i), detail: i === current ? "On screen now" : "",
            run: () => { railIndex = i; railSelect(i) } })), "")
        menuIndex = current
    }

    // ---------------------------------------------------------------- the build's scripted checks (0.54 controls)
    // Each page is a list of steps run 0.6 s apart; a step that answers false waits and is tried again (up to 12
    // times). Presses go through Turbine.selftestPad, the same path a controller's take (pointer included).
    property var script: []
    property int scriptStep: 0
    property int scriptTries: 0
    property int launchCount: 0
    property var scriptMemo: ({})
    property string lastDry: ""
    Timer { id: scriptTimer; interval: 600; repeat: true; onTriggered: app.scriptTick() }
    function pad(b) { Turbine.selftestPad(b, true); Turbine.selftestPad(b, false) }
    function check(text) { note("check", text) }
    function scriptTick() {
        if (scriptStep >= script.length) { scriptTimer.stop(); return }
        let ok = true
        try { ok = script[scriptStep]() !== false } catch (e) { check("Step " + (scriptStep + 1) + " stopped: " + e); ok = true }
        if (ok || ++scriptTries > 12) {
            if (!ok) check("Step " + (scriptStep + 1) + " gave up waiting (game " + (game ? game.name : "none") + ", " + mods.length + " mods, tab " + tab + ", zone " + zone
                           + ", Get mods " + getArea + " " + getIndex + " of " + getItems.length + ", mod page " + modPage + ", sheet \"" + sheet + "\")")
            scriptStep++; scriptTries = 0
        }
    }
    function runScript(name) {
        const m = scriptMemo
        if (name === "controls") script = [
            () => { if (mods.length < 2) return false; page = "game"; zone = "main"; tab = 0; modIndex = 0 },
            // 2: LB and RB in Settings change the section, never the game.
            () => { m.g = game.id; openSettings(); pad("rb"); pad("rb"); pad("lb"); check("Settings LB/RB: section " + sec + ", game " + (game.id === m.g ? "unchanged" : "changed to " + game.id)); pad("b") },
            // 7: pointer shown: RT is one click (the ON pill), LT the quick menu; after a D-pad press LT and RT page.
            () => { const it = modList.itemAtIndex(0); if (!it || !it.pill) return false
                    const p = it.pill.mapToItem(null, it.pill.width / 2, it.pill.height / 2); m.on = mods[0].enabled; m.toggles = 0
                    Turbine.pointerShowAt(p.x, p.y); pad("rt") },
            () => { check("Pointer RT: " + (mods[0].enabled !== m.on ? "one click, the mod turned " + (mods[0].enabled ? "on" : "off") : "no click reached the mod")) },
            () => { const it = modList.itemAtIndex(0); const p = it.mapToItem(null, it.width * 0.4, it.height / 2); Turbine.pointerShowAt(p.x, p.y); pad("lt") },
            () => { check("Pointer LT: sheet \"" + sheet + "\""); pad("b") },
            () => { const it = modList.itemAtIndex(0); const p = it.mapToItem(null, it.width * 0.4, it.height / 2); Turbine.pointerShowAt(p.x, p.y); modIndex = 0; pad("down") },
            () => { m.row = modIndex; m.shown = Turbine.pointerShown; pad("lt"); m.row2 = modIndex; pad("rt")
                    check("After a D-pad press: pointer " + (m.shown ? "still showing" : "hidden") + ", LT row " + (m.row + 1) + " to " + (m.row2 + 1) + ", RT to " + (modIndex + 1) + ", sheet \"" + sheet + "\"") },
            // R3 with the pointer hidden: shows it on the focus and does what A does; with it showing, a left click.
            () => { modIndex = 0; m.on = mods[0].enabled; pad("r3") },
            () => { check("R3 hidden: pointer " + (Turbine.pointerShown ? "shown" : "not shown") + ", " + (mods[0].enabled !== m.on ? "same as A (the mod turned " + (mods[0].enabled ? "on" : "off") + ")" : "A didn't run")); pad("r3") },
            () => { pad("down"); pad("up") },
            // 8: the focus table: toggle keeps the row; a menu closes back to it; tabs and a game round trip keep it.
            () => { modIndex = 1; pad("a"); check("Toggle: focus on row " + (modIndex + 1)); pad("x"); m.sheet = sheet; pad("b"); check("Menu " + m.sheet + " closed: focus on row " + (modIndex + 1)) },
            () => { modIndex = 1; pad("rb"); m.tab = tab; pad("lb"); check("Tabs round trip: tab " + m.tab + " and back to " + tab + ", row " + (modIndex + 1)) },
            () => { m.home = game.id; m.other = games.findIndex(g => g.id !== game.id); if (m.other < 0) { check("Game round trip: only one game"); return }
                    tab = 0; zone = "main"; modIndex = 1; m.want = mods[1] ? mods[1].name : ""; zone = "rail"; railIndex = m.other; pad("a") },
            () => { if (!game || game.id === m.home) return false; zone = "rail"; railIndex = games.findIndex(g => g.id === m.home); pad("a") },
            () => { if (!game || game.id !== m.home || !modsLoaded || mods.length < 2) return false
                    check("Game round trip: back on " + game.name + ", tab " + tab + ", row " + (modIndex + 1) + " (" + (focusedItem && focusedItem.m ? focusedItem.m.name : "") + ", was " + m.want + "; remembered " + lastFocusId[game.id] + ")") },
            // The quick wheel: tap View, Up then Right is Up-right (Sort); hold View, push Left, let go: Games.
            () => { pad("view"); pad("up"); pad("right"); check("Wheel: slot " + wheelSlot + " " + (wheelSlots[wheelSlot] ? wheelSlots[wheelSlot].label : "")); pad("b") },
            () => { Turbine.selftestPad("view", true); pad("left"); Turbine.selftestPad("view", false); check("Wheel held, Left, let go: sheet " + sheet + " \"" + pickTitle + "\""); pad("b") },
            // Uninstall: the same row stays (dimmed, A puts it back).
            () => { zone = "main"; tab = 0; modIndex = 0; m.name = mods[0].name; pad("x"); menuIndex = modMenuItems.findIndex(x => x.act === "remove"); pad("a") },
            () => { check("Uninstall: row " + (modIndex + 1) + " is " + (focusedItem ? focusedItem.kind : "nothing") + (focusedItem && focusedItem.kind === "ghost" ? " (" + focusedItem.m.name + ")" : "")); pad("a") },
            () => { if (!mods.some(x => x.name === m.name)) return false; check("Put back: " + m.name + " is back"); pad("x") }
        ]
        else if (name === "legendcheck") script = [
            () => { if (mods.length < 2) return false
                const ctxs = [
                    ["My mods", () => { sheet = ""; page = "game"; zone = "main"; tab = 0; modIndex = 0 }],
                    ["My mods bar", () => { modIndex = -1 }],
                    ["Rail", () => { modIndex = 0; zone = "rail"; railIndex = current }],
                    ["Play", () => { zone = "play" }],
                    ["Get mods bar", () => { zone = "main"; tab = 1; getArea = "bar"; getIndex = 0 }],
                    ["Get mods grid", () => { getArea = "grid"; getIndex = 0 }],
                    ["Load order", () => { tab = 2; orderIndex = 0 }],
                    ["Settings", () => { tab = 0; openSettings() }],
                    ["Settings Look", () => { setSection = settingsSections.indexOf("Look"); setRow = 0 }],
                    ["Settings Help", () => { setSection = settingsSections.indexOf("Help"); setRow = 0 }],
                    ["Mod menu", () => { settingsBack(); zone = "main"; tab = 0; modIndex = 0; sheet = "modmenu" }],
                    ["Game menu", () => { sheet = "gamemenu" }],
                    ["Before you play", () => { sheet = ""; checks = [{ level: "warn", title: "A test warning", detail: "" }]; checkIndex = 0; sheet = "check" }],
                    ["Loader", () => { sheet = ""; loaderFor = ["Lantern Glow SKSE"]; openLoader() }],
                    ["Delete for good", () => { sheet = ""; askDelete("all", "Test") }],
                    ["Text entry", () => { sheet = ""; openFind() }],
                    ["Text entry buttons", () => { askIndex = 2 }],
                    ["Play options", () => { askView.close(); sheet = ""; openPlayOptions() }],
                    ["Removed", () => { sheet = ""; openRemoved() }],
                    ["Quick wheel", () => { sheet = ""; wheelPress(); wheelHeld = false }],
                    ["Quick wheel slot", () => { wheelMove("up") }],
                    ["Tutorial", () => { closeWheel(); openTutorial(0) }],
                    ["Moving", () => { sheet = ""; modIndex = 0; startMove() }],
                    ["Pointer", () => { cancelMove(); const it = modList.itemAtIndex(0); const p = it.mapToItem(null, 20, 20); Turbine.pointerShowAt(p.x, p.y) }]
                ]
                let problems = 0
                for (const c of ctxs) {
                    c[1]()
                    const o = binds(), h = legendFor(o), shown = legendHints
                    let bad = []
                    if (h.length > 5) bad.push("more than 5")
                    const keys = h.map(x => x[0])
                    if (keys.some((k, i) => keys.indexOf(k) !== i)) bad.push("a key twice")
                    if (keys.some(k => ["lb", "rb", "view"].indexOf(k) >= 0)) bad.push("a cap in the legend")
                    if (keys.indexOf("start") >= 0 && sheet === "" && !wheelOpen) bad.push("Start in the legend with Play in view")
                    for (const k of ["a", "b", "x", "y"]) {
                        if (o[k] && o[k].label && keys.indexOf(k) < 0) bad.push(k.toUpperCase() + " missing")
                        if (keys.indexOf(k) >= 0 && !(o[k] && o[k].run)) bad.push(k.toUpperCase() + " does nothing")
                    }
                    if (JSON.stringify(shown) !== JSON.stringify(h)) bad.push("the legend on screen differs")
                    problems += bad.length > 0 ? 1 : 0
                    check("Legend " + c[0] + ": " + h.map(x => (x[2] ? x[2] + " " : "") + x[0].toUpperCase() + " " + x[1] + (x[3] ? ", " + x[3].toUpperCase() + " " + x[4] : "")).join(" · ") + (bad.length > 0 ? "  PROBLEM: " + bad.join(", ") : ""))
                }
                Turbine.pointerHide()
                check("Legend check: " + ctxs.length + " contexts, " + problems + " problems")
                sheet = ""
            }
        ]
        else if (name === "sheetcheck") script = [
            () => { if (mods.length < 2) return false
                const cases = [
                    ["check", () => { checks = [{ level: "warn", title: "A test warning", detail: "" }]; checkIndex = 0; sheet = "check" }],
                    ["loader", () => { loaderFor = ["Lantern Glow SKSE"]; openLoader() }],
                    ["chain", () => { chainQueue = [{ name: "Test Patch", url: "https://www.nexusmods.com/skyrimspecialedition/mods/1?tab=files" }]; infoIndex = 0; sheet = "chain" }],
                    ["found", () => { found = { path: "/tmp/x.zip", name: "x.zip", size: 1024, game: game.id, gameName: game.name, why: "test", more: 0 }; infoIndex = 0; sheet = "found" }],
                    ["notes", () => { notesName = "Test"; notesList = ["A note"]; infoIndex = 0; sheet = "notes" }],
                    ["confirm", () => { askDelete("all", "Test") }],
                    ["ask", () => { openFind() }]
                ]
                sheetDry = true
                let problems = 0
                for (const c of cases) {
                    sheet = ""; c[1]()
                    let bs = buttonSheet()
                    if (!bs) { check("Sheet rule " + c[0] + ": not a button sheet"); problems++; continue }
                    act("right")
                    bs = buttonSheet()
                    const want = { a: bs.labels[bs.index] || "", b: bs.cancel >= 0 ? bs.labels[bs.cancel] : "", start: bs.main >= 0 ? bs.labels[bs.main] : "" }
                    let got = {}
                    for (const k of ["a", "b", "start"]) { lastDry = ""; act(k); got[k] = lastDry }
                    const ok = got.a === want.a && got.b === want.b && got.start === want.start
                    if (!ok) problems++
                    check("Sheet rule " + c[0] + ": highlighted " + want.a + ", A ran " + (got.a || "nothing") + ", B ran " + (got.b || "nothing") + ", Start ran " + (got.start || "nothing") + (ok ? "" : "  PROBLEM"))
                }
                sheetDry = false; found = null; askView.close(); sheet = ""
                check("Sheet rule: " + cases.length + " sheets, " + problems + " problems")
            }
        ]
        else if (name === "nexusguard" || name === "inlinebrowser") script = [
            () => { if (getItems.length === 0) return false; openDetails(getItems[0]) },
            () => { if (!detailsData || detailsData.loading || mainLabel !== "Get it on Nexus") return false; m.before = launchCount
                    pad("a"); if (name === "nexusguard") { pad("a"); pad("a") } },
            () => { if (name === "nexusguard") { check("Nexus guard: 3 presses of A, " + (launchCount - m.before) + " browser launch" + (launchCount - m.before === 1 ? "" : "es") + ", button \"" + mainLabel + "\""); return }
                    openHelpNow(); pad("down") },
            () => { if (name === "inlinebrowser") check("Inline browser buttons: " + pageHelpLabels.join("; ") + ", focus on " + pageALabel + ", sheet \"" + sheet + "\"") }
        ]
        else if (name === "presscount") script = [
            // After the tutorial: the rail, the game with no mods focused. Count every press to a mod added and Play.
            () => { if (!game || !modsLoaded) return false; sheet = ""; zone = "rail"; railIndex = current; m.start = inputCount; m.log = [] },
            () => { pad("a"); m.log.push("A open " + game.name) },
            () => { if (tab !== 1 || getArea !== "grid" || getItems.length === 0) return false; pad("a"); m.log.push("A on " + getItems[getIndex].name) },
            () => { if (!detailsData || detailsData.loading || pageIndex !== 0 || mainLabel === "") return false; m.log.push("A " + mainLabel); pad("a") },
            // On Nexus: Slow download (counted, outside Turbine); the download arrives and Turbine adds it.
            () => { m.log.push("Slow download on Nexus"); Turbine.importZip(shot.arg("presscount-file")) },
            () => { if (zone !== "play") return false; m.log.push("A Play (focused)"); pad("a") },
            () => { const n = inputCount - m.start
                    check("Presses from the rail to a mod added and Play: " + n + " in Turbine + 1 on Nexus = " + (n + 1) + " (" + m.log.join(", ") + ")") }
        ]
        scriptStep = 0; scriptTries = 0
        scriptTimer.start()
    }

    // ---------------------------------------------------------------- the beginner tutorial (first run)
    // Four cards with the real buttons drawn large: A and B, X and Y; View and the quick wheel; getting a mod; Play.
    // A or Right: next card; Left: back one; B or Start: skip. It ends on the rail ("Pick a game to start"), runs
    // once (remembered) and Settings, Help shows it again.
    readonly property int tutCount: 4
    property int tutCard: 0
    function openTutorial(n) {
        tutCard = n || 0
        note("action", "Beginner tutorial, card " + (tutCard + 1))
        sheet = "tutorial"
    }
    function tutorialAct(b) {
        if (b === "a" || b === "right") {
            if (tutCard < tutCount - 1) { tutCard++; note("action", "Tutorial card " + (tutCard + 1)) }
            else tutorialDone("finished")
        }
        else if (b === "left") tutCard = Math.max(0, tutCard - 1)
        else if (b === "b" || b === "start") tutorialDone("skipped")
    }
    function tutorialDone(how) {
        if (!shot.on) seen.tutorialDone = true
        note("action", "Tutorial " + how + ": pick a game")
        sheet = ""
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
    readonly property var sortNames: srcNow === "nexus" ? ["Trending", "Most endorsed", "Most downloaded", "Recently updated", "Newest"]
                                                        : ["Most downloaded", "Top rated", "Recently updated", "Newest"]
    property var browseData: null
    property bool browseLoading: false
    property bool loadingMore: false
    property bool noMore: false
    property string getArea: "bar"        // bar, recent, grid, fallback
    property int getIndex: 0
    // The card B left from (Down from the bar goes back to it); -1 when there is none.
    property int gridReturn: -1
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
        return (mine === undefined && !x.installed) ? x : Object.assign({}, x, { installed: true, update: mine !== undefined && !!updateMap[mine] })
    })
    // Updates for your mods, on the Get mods start page.
    readonly property var updateList: Object.keys(updateMap).map(k => ({ u: updateMap[k], m: mods.find(x => x.id === k) })).filter(x => !!x.m)
    readonly property var getBar: [{ key: "search", label: getText !== "" ? getText : "Search " + gameTitle + " mods" },
                                   { key: "sort", label: "Sort: " + (sortNames[getSortIndex] || "") }]
        .concat(getCats.length > 0 ? [{ key: "category", label: "Category: " + (getCategory || "All") }] : [])
        .concat(sources.length > 1 ? [{ key: "source", label: "From: " + srcName(srcNow) }] : [])
        .concat([{ key: "add", label: "Add a file" }])
        // Last, so the search field keeps its room.
        .concat(updateList.length > 0 && getText === "" ? [{ key: "updates", label: "Updates (" + updateList.length + ")" }] : [])
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
    // The one-time line over the grid for a game with no mods yet.
    readonly property bool bannerOn: !!game && modsLoaded && mods.length === 0 && parse(seen.bannerDone, []).indexOf(game.id) < 0
    readonly property string getBanner: bannerOn ? "No mods for " + gameTitle + " yet. Pick one below, or search." : ""
    // Entering a game with no mods lands on Get mods (once per visit).
    property string landedOn: ""
    property bool landAgain: false
    function maybeLandOnGet() {
        // Needs the game's rules too (where its mods come from): called again when they arrive.
        if (!game || !modsLoaded || !rules || page !== "game") return
        if (landedOn === game.id && !(landAgain && mods.length === 0)) return
        landAgain = false
        landedOn = game.id
        if (mods.length === 0 && sources.length > 0 && tab === 0 && shot.page !== "emptygamemods" && shot.page.indexOf("mods") !== 0 && shot.page !== "removed") {
            // Straight onto the first mod: A opens its page.
            if (!restoreFocus) restoreFocus = { area: "grid", index: 0 }
            note("focus", "No mods yet: Get mods, first card")
            tab = 1
        }
    }
    onModsChanged: if (game && mods.length > 0 && parse(seen.bannerDone, []).indexOf(game.id) < 0 && !shot.on) {
        let d = parse(seen.bannerDone, []); d.push(game.id); seen.bannerDone = JSON.stringify(d)
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
            const h = history.slice()
            const top = h.pop()
            history = h
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
        if (getArea === "grid" || getArea === "recent") {
            // Back to the search bar; Down comes back to the same card.
            if (getArea === "grid") gridReturn = getIndex
            getArea = "bar"; getIndex = Math.max(0, getBar.findIndex(x => x.key === "search")); return
        }
        if (getText !== "") { getText = ""; getArea = "bar"; getIndex = 0; note("action", "Search cleared"); browseNow(); return }
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
        getArea = "bar"; getIndex = 0
        browseNow()
    }
    function getAct(b) {
        const cols = gridColumns
        if (b === "b") { getBack(); return }
        if (getArea === "fallback") {
            if (b === "a") openNexusMods()
            else if (b === "x") retryBrowse()
            else if (b === "y") openAdd()
            else if (b === "left") { zone = "rail"; railIndex = current }
            return
        }
        if (getArea === "bar") {
            const k = getBar[getIndex] ? getBar[getIndex].key : ""
            if (b === "left") { if (getIndex > 0) getIndex--; else { zone = "rail"; railIndex = current } }
            else if (b === "right") getIndex = Math.min(getBar.length - 1, getIndex + 1)
            else if (b === "down") {
                if (gridReturn >= 0 && gridReturn < getItems.length) { getArea = "grid"; getIndex = gridReturn; gridReturn = -1 }
                else if (recentViewed.length > 0 && getText === "") { getArea = "recent"; getIndex = 0 }
                else if (getItems.length > 0) { getArea = "grid"; getIndex = 0 }
            }
            else if (b === "a") getBarRun(getIndex)
        } else if (getArea === "recent") {
            if (b === "left") { if (getIndex > 0) getIndex--; else { zone = "rail"; railIndex = current } }
            else if (b === "right") getIndex = Math.min(recentViewed.length - 1, getIndex + 1)
            else if (b === "up") { getArea = "bar"; getIndex = 0 }
            else if (b === "down" && getItems.length > 0) { getArea = "grid"; getIndex = 0 }
            else if (b === "a" && recentViewed[getIndex]) openDetails(recentViewed[getIndex])
        } else {
            if (b === "left") { if (getIndex % cols > 0) getIndex--; else { zone = "rail"; railIndex = current } }
            else if (b === "right") { if (getIndex % cols < cols - 1) getIndex = Math.min(getItems.length - 1, getIndex + 1) }
            else if (b === "up") { if (getIndex >= cols) getIndex -= cols; else if (recentViewed.length > 0 && getText === "") { getArea = "recent"; getIndex = 0 } else { getArea = "bar"; getIndex = 0 } }
            else if (b === "down") getIndex = Math.min(getItems.length - 1, getIndex + cols)
            else if (b === "a" && getItems[getIndex]) openDetails(getItems[getIndex])
            else if (b === "x" && getItems[getIndex]) { menuIndex = 0; sheet = "cardmenu" }
            // Infinite scroll: the next 30 when the focus reaches the last row.
            if (getIndex >= getItems.length - cols) browseMore()
        }
    }
    // A on Sort, Category or From: every choice in one list; one read when one is chosen.
    function pickBar(k) {
        const names = k === "sort" ? sortNames : k === "category" ? getCats.map(x => x || "All") : sources.map(srcName)
        const now = k === "sort" ? getSortIndex : k === "category" ? Math.max(0, getCats.indexOf(getCategory)) : Math.max(0, sources.indexOf(srcNow))
        openPick(k === "sort" ? "Sort" : k === "category" ? "Category" : "Get mods from", gameTitle, names.map((n, i) => ({
            icon: i === now ? "power" : "info", title: n, detail: i === now ? "Showing now" : "",
            run: () => {
                if (i === now) return
                if (k === "sort") getSortIndex = i
                else if (k === "category") getCategory = getCats[i]
                else { getSource = sources[i]; getCategory = ""; getSortIndex = 0; browseData = null }
                note("action", "Get mods " + k + " " + n)
                browseNow()
            } })), "")
        menuIndex = now
    }
    // Steps Sort, Category or From (kept for the pointer and tests).
    function cycleBar(k, d) {
        if (k === "sort") { getSortIndex = (getSortIndex + d + sortNames.length) % sortNames.length; note("action", "Get mods sort " + sortNames[getSortIndex]) }
        else if (k === "category") { getCategory = getCats[(getCats.indexOf(getCategory) + d + getCats.length) % getCats.length]; note("action", "Get mods category " + (getCategory || "All")) }
        else if (k === "source") { getSource = sources[(sources.indexOf(srcNow) + d + sources.length) % sources.length]; getCategory = ""; getSortIndex = 0; note("action", "Get mods source " + getSource); browseData = null }
        browseNow()
    }
    function getBarRun(i) {
        const k = getBar[i] ? getBar[i].key : ""
        getArea = "bar"; getIndex = i
        if (k === "search") openAsk("search", "Search " + gameTitle + " mods", "Words from the mod's name or what it does.", getText, "Search", 100, false, "Search")
        else if (k === "sort" || k === "category" || k === "source") pickBar(k)
        else if (k === "add") openAdd()
        else if (k === "updates") openUpdates()
    }
    function openUpdates() {
        openPick("Updates for your mods", plural(updateList.length, "mod has", "mods have") + " a newer version", updateList.map(x => ({
            icon: "update", title: x.m.name + " " + x.u.to, detail: (x.u.from ? "You have " + x.u.from + ". " : "") + "Keeps its place and on or off",
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
    // X on a card: Get it, Other files, Open on Nexus.
    readonly property var cardItem: getArea === "grid" ? getItems[getIndex] : null
    readonly property var cardMenuItems: !cardItem ? [] : [
        { icon: "update", title: "Get it", detail: getWords(), act: "get" },
        { icon: "info", title: "Other files", detail: "Older versions and extra files, on its page", act: "files" }
    ].concat(srcNow === "nexus" ? [{ icon: "play", title: "Open on Nexus", detail: "In your browser", act: "nexus" }] : [])
    property bool pendingFiles: false
    function cardMenuRun(i) {
        const it = cardMenuItems[i], c = cardItem
        if (!it || !c) return
        sheet = ""
        note("action", "Card menu: " + it.title + " for " + c.name)
        if (it.act === "files") { pendingFiles = true; openDetails(c) }
        else if (it.act === "get") quickGet(c)
        else if (it.act === "nexus") openNexus("https://www.nexusmods.com/" + rules.domain + "/mods/" + c.id)
    }
    function getWords() {
        if (srcNow === "thunderstore") return "Installs it with what it needs"
        return account.premium === true ? "Installs it in one press" : "Opens its file page on Nexus"
    }
    // Y on a card (and the card menu's Get it): the same as the mod page's main button.
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
        detailsItem = item
        detailsData = { loading: true }
        modPage = true
        // The main button ("Get it on Nexus", "Install") has the focus.
        pageIndex = 0; pageInline = -1
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
        if (pendingGet && detailsData && !detailsData.loading) { pendingGet = false; if (!detailsData.error) installFromDetails() }
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
        : detailsData.source === "nexus" && detailsData.premium === false ? "Opens this file's page. Press Slow download there and wait the few seconds Nexus asks. Turbine adds it by itself."
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
    // The two browser buttons under the main one, 8 seconds after Open (openHelp).
    readonly property bool pageHelp: openHelp && modPage && !!detailsData && !detailsData.loading
    readonly property var pageHelpLabels: pageHelp ? ["Open in Steam's browser", "Always use Steam's browser"] : []
    onPageHelpChanged: if (!pageHelp) pageInline = -1
    readonly property string pageALabel: pageInline >= 0 ? (pageHelpLabels[pageInline] || "") : pageIndex === 0 ? mainLabel : pageIndex === 1 ? "Other files" : "Open its page"
    // X on the mod page: Other files, Get what it needs, Open on Nexus.
    readonly property var pageMenuItems: !detailsData || detailsData.loading ? [] : (otherVersions.length > 0 ? [
        { icon: "update", title: "Other files", detail: plural(otherVersions.length, "other file", "other files"), run: () => openVersions() }
    ] : []).concat(pageMissing >= 0 ? [
        { icon: "update", title: "Get what it needs", detail: pageNeeds[pageMissing] ? pageNeeds[pageMissing].name : "", run: () => { pageIndex = pageMissing + 2; openNeed(pageMissing) } }
    ] : []).concat((detailsData.source || srcNow) === "nexus" && detailsData.page ? [
        { icon: "play", title: "Open on Nexus", detail: "Its page, in the browser", run: () => openNexus(detailsData.page) }
    ] : [])
    function modPageAct(b) {
        if (b === "b") { getBack(); return }
        if (b === "a") {
            if (pageInline === 0) steamOpen()
            else if (pageInline === 1) alwaysSteamNow()
            else if (pageIndex === 0) installFromDetails()
            else if (pageIndex === 1) openVersions()
            else openNeed(pageIndex - 2)
        }
        else if (b === "up") {
            if (pageInline === 1) pageInline = 0
            else if (pageInline === 0) pageInline = -1
            else if (pageHelp && pageIndex > 0 && (pageIndex === 1 || (pageIndex === 2 && otherVersions.length === 0))) { pageIndex = 0; pageInline = 1 }
            else if (pageIndex > 0) pageIndex = pageIndex === 2 && otherVersions.length === 0 ? 0 : pageIndex - 1
            else modView.scroll(-1)
        }
        else if (b === "down") {
            if (pageInline < 0 && pageIndex === 0 && pageHelp) { pageInline = 0; return }
            if (pageInline === 0) { pageInline = 1; return }
            // The Other files row (1) is hidden when there are none: skip it, and scroll when no row is left.
            const next = pageIndex === 0 && otherVersions.length === 0 ? 2 : pageIndex + 1
            if (next <= pageRows - 1) { pageIndex = next; pageInline = -1 } else modView.scroll(1)
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
        if (detailsData.partial) { if (detailsData.page) openNexus(detailsData.page); return }
        if (pageInstalled) {
            // Off: A turns it on (the button then says Show in My mods). On: A shows it in My mods.
            if (pageModOff) toggle(pageMod)
            else if (pageMod) goToMod(pageMod.id)
            return
        }
        if (detailsData.source === "nexus" && detailsData.premium === false) {
            const url = detailsData.filePage || detailsData.page
            note("action", "Get it on Nexus: " + detailsData.name + " (" + url + ")")
            const miss = (detailsData.needs && detailsData.needs.missing ? detailsData.needs.missing : []).filter(r => r.modId > 0 && r.url)
            chainQueue = miss.map(r => ({ name: r.name, url: r.url + (r.url.indexOf("?") < 0 ? "?tab=files" : "") }))
            openNexus(url)
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
        openNexus(p.url)
    }

    // ---------------------------------------------------------------- the browser (catch.rs)
    // What the browser check found, the browser helper's link, and the status shown over the browser.
    property var browser: ({ checked: false })
    property var bridge: ({ running: false })
    property var status: ({ active: false })
    property bool movedPaired: false
    readonly property bool browserFound: !browser.checked || (!!browser.result && browser.result.found === true)
    // Opens a Nexus page in the player's browser and watches Downloads for its file. False when there is no browser.
    // In Game Mode gamescope only shows what Steam starts, so Turbine starts the player's browser through Steam (a
    // non-Steam game in their library, launch.rs) and the browser helper opens the page in a new tab. No browser in
    // Steam: "Add Chrome to Steam" once, then Steam's own browser. The status pill says what is happening, and if
    // Turbine is still in front 8 seconds later, "Didn't see Nexus open?" offers Steam's browser.
    property string openState: ""         // "", "opening", "open"
    property string openName: "the browser"
    property string openUrlNow: ""
    property double openAt: 0
    property string addMode: ""           // "noshortcut" or "helper": what the Add Chrome to Steam sheet says
    // A pressed again while the browser is still starting does nothing for 8 seconds (one browser, not three);
    // launch.rs keeps the same guard for the same page.
    property bool launching: false
    Timer { id: launchGuard; interval: 8000; onTriggered: app.launching = false }
    // After 8 seconds with Turbine still in front: "Open in Steam's browser" and "Always use Steam's browser" under
    // the mod page's button (no sheet).
    property bool openHelp: false
    property int pageInline: -1           // the mod page: -1, or which of those two buttons has the focus
    function openNexus(url) {
        if (!browserFound) { noBrowser(); return false }
        if (launching && url === openUrlNow) {
            note("browser", "A ignored: already opening this page (" + (Date.now() - openAt) + " ms ago)")
            return true
        }
        const w = parse(Turbine.watchPage(url), {})
        if (w.watching) note("catch", "Watching Downloads for this mod (" + w.game + ")")
        openUrlNow = url
        openAt = Date.now()
        openHelp = false; pageInline = -1
        launching = true; launchGuard.restart()
        launchCount++
        note("browser", "Browser launch: " + url)
        openName = browser.result && browser.result.name ? browser.result.name : shot.on ? "Chrome" : "the browser"
        if (shot.on) { openState = "opening"; openTimer.restart(); return true }
        // Settings, Getting mods: Always use Steam's browser (Game Mode).
        if (Turbine.gamemode && browserSet.alwaysSteam) { note("browser", "Always use Steam's browser: on"); steamOpen(); return true }
        const r = parse(Turbine.openUrl(url), { how: "desktop" })
        if (r.how === "guarded") return true
        if (r.how === "noshortcut") {
            // Nothing to set up first: Steam's browser opens the page now; adding Chrome is optional (said once).
            note("browser", "No browser in Steam: Steam's browser instead")
            steamOpen()
            if (!browserSet.addShown) { browserSet.addShown = true; toast.showPlain("Opened in Steam's browser. To use Chrome instead: Settings, Getting mods, Use Chrome in Game Mode.") }
            return true
        }
        if (r.how === "oldhelper") {
            // The helper in the browser is too old to open pages: Steam's browser opens the page now, and the
            // player hears once how to bring the helper up to date.
            oldHelperOnce(r.helperVersion)
            steamOpen()
            return true
        }
        if (r.how === "shortcut") {
            openName = r.name || openName
            // The same way Play starts a game: Steam gets the link through the desktop portal and starts the
            // browser where Game Mode shows it.
            note("browser", "Asked Steam to start " + openName + ": " + r.steam)
            Qt.openUrlExternally(r.steam)
            openState = "opening"
            openTimer.restart()
            if (!r.helper) {
                if (!browserSet.helperHintShown) { browserSet.helperHintShown = true; note("browser", "Said once: set up the browser helper too") }
                toast.showPlain(openName + " opens on its start page: the browser helper takes it to the mod (Settings, Getting mods).")
            }
            return true
        }
        openState = "opening"
        openTimer.restart()
        Qt.openUrlExternally(url)
        return true
    }
    // The helper in the browser is older than Turbine's and can't reload itself: said once per helper version.
    function oldHelperOnce(v) {
        if (shot.on || browserSet.oldHelperShown === (v || "?")) return
        browserSet.oldHelperShown = v || "?"
        note("browser", "Showing: the browser helper " + (v || "") + " is out of date (once)")
        infoIndex = 0
        sheet = "oldhelper"
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
            toast.showPlain("Found " + r.browser + " in Steam as " + r.name + ". Nexus pages open there now.")
            if (openUrlNow !== "" && !shot.on) openNexus(openUrlNow)
        } else toast.showPlain("Not in Steam yet. Add Google Chrome as a non-Steam game in Desktop Mode, then Recheck.")
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
        openHelp = false; pageInline = -1
        if (shot.on) return
        const link = Turbine.openInSteam(openUrlNow)
        if (link === "") { toast.showPlain("Steam's browser can't open that address."); return }
        // Opened like Play's steam://rungameid, so Game Mode shows it.
        Qt.openUrlExternally(link)
        openName = "Steam's browser"
        openState = "opening"
        openTimer.restart()
    }
    // Still in front 8 seconds after Open: the two buttons under the mod page's button; elsewhere a line saying where
    // they are. Never a sheet.
    function openHelpNow() {
        openTimer.stop()
        openHelp = true
        note("browser", "Still in front 8 seconds after Open: Open in Steam's browser and Always use Steam's browser are under the button" + (modPage ? "" : " (mod page)"))
        if (!modPage) toast.showPlain("Nexus didn't show? Settings, Getting mods: Always use Steam's browser.")
    }
    function alwaysSteamNow() {
        if (!shot.on) browserSet.alwaysSteam = true
        note("action", "Always use Steam's browser")
        toast.showPlain("Game Mode opens Nexus in Steam's browser from now on. Settings, Getting mods changes it.")
        steamOpen()
    }
    // 8 seconds after Open, Turbine still in front and no browser window: the safety net (Game Mode only).
    Timer {
        id: openTimer
        interval: 8000
        onTriggered: {
            if (app.openState === "opening" && app.active && Turbine.gamemode) {
                // Steam's browser is already the safety net: say so in the log rather than offer it again.
                if (app.openName === "Steam's browser") app.note("browser", "Still in front 8 seconds after asking Steam's browser to open Nexus")
                else app.openHelpNow()
            }
            if (app.openState === "opening") app.openState = ""
        }
    }
    Timer { id: openGone; interval: 8000; onTriggered: app.openState = "" }
    Connections {
        target: app
        function onActiveChanged() {
            if (!app.active && app.openState === "opening") {
                app.note("browser", app.openName + " came up: Turbine went to the background " + (Date.now() - app.openAt) + " ms after Open")
                app.openState = "open"; openTimer.stop()
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
    property string helperFolder: ""
    function setupHelper() {
        const r = parse(Turbine.helperSetup(), { error: "Turbine couldn't put the helper in Downloads." })
        if (r.error) { toast.showPlain(r.error); return }
        helperFolder = r.folder
        infoIndex = 0
        sheet = "helper"
    }
    function pair(again) {
        if (!browserFound) { noBrowser(); return }
        const url = again ? Turbine.bridgePairAgain() : Turbine.bridgePairUrl()
        if (url === "") { toast.showPlain("Turbine's link for the browser helper isn't running. Close Turbine and open it again."); return }
        if (!shot.on) Qt.openUrlExternally(url)
        toast.showPlain("Opened the pairing page in the browser. It says Paired with Turbine when the helper is set up.")
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
        if (sheet === "" || sheet === "notes") { infoIndex = 0; sheet = "notes" }
        else pendingNotes = true
    }
    property bool pendingNotes: false
    readonly property var infoButtons: sheet === "quick" && quick ? (quick.added.length === 1 ? [{ key: "a", label: "Turn that off and play" }]
                                                                   : quick.added.length > 1 ? [{ key: "a", label: "Turn off the newest and play" }].concat(quick.worked ? [{ key: "y", label: "Go back to the setup that worked" }] : []) : [])
                                          .concat([{ key: "x", label: "Play without mods" }, { key: "b", label: "Keep my mods" }])
        : sheet === "chain" ? [{ key: "a", label: "Open its page" }, { key: "b", label: "Later" }]
        : sheet === "nobrowser" ? [{ key: "a", label: "Check again" }, { key: "b", label: "Close" }]
        : sheet === "helper" ? [{ key: "a", label: "Pair" }, { key: "b", label: "Close" }]
        : sheet === "addbrowser" && addMode === "helper" ? [{ key: "a", label: "Set up the helper" }, { key: "x", label: "Steam's browser" }, { key: "b", label: "Close" }]
        : sheet === "addbrowser" ? [{ key: "a", label: "Recheck" }, { key: "x", label: "Steam's browser" }, { key: "b", label: "Close" }]
        : sheet === "found" ? (found && found.game ? [{ key: "a", label: "Add it" }] : []).concat([{ key: "x", label: found && found.game ? "Other game" : "Pick the game" }, { key: "b", label: "Not a mod" }])
        : [{ key: "a", label: "OK" }]
    function infoRun(i) {
        const label = infoButtons[i] ? infoButtons[i].label : ""
        note("action", "Sheet " + sheet + ": " + label)
        if (sheet === "quick") { quickRun(label); return }
        if (sheet === "chain") {
            const next = chainQueue[0]
            sheet = ""
            if (label === "Open its page" && next) { chainQueue = chainQueue.slice(1); note("action", "Next requirement: " + next.name); openNexus(next.url) }
            else { chainQueue = []; note("action", "Requirements: later") }
            return
        }
        if (sheet === "nobrowser") { if (label === "Check again") { Turbine.browserCheck(); toast.showPlain("Looking for a browser...") } else sheet = "" }
        else if (sheet === "helper") { if (label === "Pair") { sheet = ""; pair(false) } else sheet = "" }
        else if (sheet === "addbrowser") {
            if (label === "Recheck") recheckBrowserShortcut()
            else if (label === "Set up the helper") { sheet = ""; setupHelper() }
            else if (label === "Steam's browser") { sheet = ""; openTimer.stop(); if (openUrlNow !== "") steamOpen() }
            else sheet = ""
        }
        else if (sheet === "found") {
            const f = found
            if (!f) { sheet = ""; return }
            if (label === "Other game" || label === "Pick the game") { openPickGame(); return }
            found = null; sheet = ""
            Turbine.watchConfirm(f.path, label === "Add it")
        }
        else sheet = ""
    }

    // The status card over other apps: only while Turbine isn't the window in front (the player is in the
    // browser), only with Settings > Accounts > Show status over other apps on. Game Mode uses the input-less
    // overlay window gamescope paints over the browser; Desktop Mode a small panel kept above the browser.
    readonly property var transfer: status.active ? status
        : dl.active ? { active: true, text: "Downloading " + dl.label, level: "busy", progress: dlPart, game: gameTitle, added: "", done: false, ask: "" }
        : null
    readonly property bool overlayWanted: !shot.on && browserSet.overlay && !app.active && transfer !== null
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
        const r = parse(Turbine.openLink(link), { error: "That Nexus link can't be read." })
        if (!shot.on) { app.raise(); app.requestActivate() }
        if (r.needsKey) {
            // No Nexus key: Slow download works instead, and that file goes into this mod's game by itself.
            note("nxm", "Mod Manager Download without a Nexus key")
            if (r.page) Turbine.watchPage(r.page)
            say(r.error, "warn")
            msgTimer.interval = 12000; msgTimer.restart()
            toast.showPlain(r.error)
            return
        }
        if (r.error) { note("nxm", "Link refused: " + r.error); say(r.error, "error"); return }
        const i = games.findIndex(g => g.id === r.game)
        if (i >= 0 && i !== current) current = i
        page = "game"; zone = "main"; tab = 0; modPage = false
        if (sheet !== "fomod") sheet = ""
        note("nxm", "Link for " + r.name)
        Turbine.nxmDownload(link, r.game)
        edited()
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
        if (!v || v.installed) return
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
        { key: "export", label: "Export to a share file", detail: "Saves \"" + (profileData ? profileData.active : "") + "\" to Downloads, to share" },
        { key: "importFile", label: "Import a share file", detail: "A Turbine profile file in Downloads" }
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
        else if (a.key === "importFile") { Turbine.listShares(); shareIndex = 0; sheet = "shares" }
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
        if (askPurpose === "search") { getText = text; getArea = "bar"; getIndex = 0; note("action", "Get Mods search"); browseNow() }
        else if (askPurpose === "nexusKey" && text !== "") Turbine.setNexusKey(text)
        else if (askPurpose === "newProfile" && text !== "") Turbine.profileCreate(text)
        else if (askPurpose === "putafter") putAfter(text)
        else if (askPurpose === "findplugin" && text !== "") findPlugin(text)
        else if (askPurpose === "find") { findText = text; modIndex = 0; note("action", "Find \"" + text + "\": " + shownMods.length + " shown") }
        else if (askPurpose.indexOf("renameProfile:") === 0 && text !== "" && text !== askPurpose.substring(14)) Turbine.profileRename(askPurpose.substring(14), text)
        else if (askPurpose === "code" && text !== "") Turbine.profileImportCode(text)
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
        if (page !== "settings" && !settingsFrom) settingsFrom = { zone: zone, tab: tab, modPage: modPage, railIndex: railIndex }
        page = "settings"; zone = "settings"; setRow = 0
        if (selfUpd.state === "available" && settingsSections.indexOf("Updates") >= 0) setSection = settingsSections.indexOf("Updates")
        if (sec === "Look") checkDisplays()
    }
    function openSection(name) { openSettings(); setSection = Math.max(0, settingsSections.indexOf(name)); setRow = 0 }
    onLabsOnChanged: if (!labsOn && setSection >= settingsSections.length) { setSection = settingsSections.indexOf("About"); setRow = 0 }
    // Accounts, Updates and Labs: rows of one kind each. action (A does it), toggle and choice (Left, Right or A
    // change it), item (a Labs checklist item), info (words only).
    readonly property var rowsNow: sec === "Getting mods" ? accountRows : sec === "Updates" ? updateRows : sec === "Labs" ? labsRows : sec === "Help" ? helpRows : []
    // Help: the buttons in one place, the beginner tutorial again, and games hidden from the rail.
    readonly property var helpRows: [
        { kind: "action", key: "tutorial", title: "Beginner tutorial", how: "Four cards: the buttons, the quick wheel, getting a mod, and Play", action: "Show it" },
        { kind: "info", key: "buttons", title: "The buttons",
          how: "A does the highlighted thing. B goes back (hold it to go home). X: options for what's highlighted. Y: Find or Search. LB and RB: tabs. LT and RT: a page up or down. View: the quick wheel. Start: Play (hold it for Play options). Right stick: the pointer; RT or R3 clicks, LT opens options." },
        { kind: "info", key: "touch", title: "Touch",
          how: "Tap does it. Hold a moment for options. Hold, then drag to reorder. Swipe sideways on the tabs to switch them." }
    ].concat(hiddenIds.length > 0 ? [
        { kind: "action", key: "unhide", title: "Games hidden from the rail: " + hiddenIds.length, how: "Puts them back on the rail", action: "Show them" }
    ] : [])
    readonly property string accountWords: account.state === "checking" ? "Checking the key with Nexus Mods..."
        : account.state === "ok" ? "Signed in as " + account.name + (account.premium ? " · Premium: installs in one press with no browser" : " · Free account: Get it on Nexus opens the file's page")
        : account.state === "error" ? account.message
        : account.message ? account.message + " " + keyWhere : keyWhere
    // Where the Nexus key is, exactly (the Request Api Key buttons on that page are for other apps).
    readonly property string keyWhere: "On nexusmods.com, open Site preferences, then API Keys. Scroll to the bottom to Personal API Key and copy it. Don't use the Request Api Key buttons, those are for other apps."
    readonly property string keyPageUrl: "https://www.nexusmods.com/settings/api-keys"
    // Paste: the key straight from the clipboard, checked with Nexus, in one press.
    function pasteKey() {
        const t = shot.on ? "" : Turbine.clipboardText().trim()
        if (t.length >= 16 && t.length <= 1024 && !/\s/.test(t)) { note("action", "Paste key: checking it with Nexus"); Turbine.setNexusKey(t); toast.showPlain("Checking the key with Nexus Mods...") }
        else toast.showPlain(t === "" ? "Nothing to paste yet. Copy your Personal API Key on nexusmods.com first, then press Paste." : "That isn't a key. Copy your Personal API Key on nexusmods.com, then press Paste.")
    }
    // Getting mods: first a verdict, not a setting; then what is optional.
    readonly property var accountRows: [
        { kind: "info", key: "verdict", title: nexusPublic === "failed" ? "You're ready. Get mods opens Nexus in your browser, nothing to set up." : "You're ready. Browse and get mods now, nothing to set up.",
          how: "Get mods on each game's page. Thunderstore mods install in one press; Nexus mods open their file page, and Turbine adds the download by itself." },
        { kind: "action", key: "pasteKey", head: "Optional, makes it faster", title: "Nexus Mods key", how: accountWords, action: account.hasKey ? "Paste a new key" : "Paste" },
        { kind: "action", key: "keyPage", title: "Open the API Keys page", how: "Opens " + keyPageUrl.replace("https://", "") + " in the browser. Sign in first if Nexus asks.", action: "Open" },
        { kind: "action", key: "nexusKey", title: "Type the key instead", how: "With Steam's keyboard, if pasting isn't possible", action: "Type" },
        { kind: "info", key: "keyWhat", title: "What a key does",
          how: "Free key: mod details, file lists and update checks. Premium key: also one-press downloads straight from Turbine. Without Premium, Get it on Nexus opens the file's page and Turbine adds the download." }
    ].concat(account.hasKey ? [{ kind: "action", key: "signOut", title: "Sign out of Nexus Mods", how: "Deletes the key from this device", action: "Sign out" }] : []).concat([
        { kind: "action", key: "helper", title: "Browser helper", action: "Set up",
          how: "Saves Nexus downloads with no Save box, sorted into Game Mods folders. Tip: in Chrome's settings, turn off Ask where to save each file." }
    ]).concat(bridge.outdated ? [
        { kind: "info", key: "helperOld", title: "Your browser helper is out of date" + (bridge.helper ? " (" + bridge.helper + ", Turbine has " + bridge.bundled + ")" : ""),
          how: bridge.manual ? "In Chrome, open chrome://extensions and press the reload arrow on Turbine browser helper. After that it updates by itself."
                             : "Turbine put the new version in its folder and asks it to reload when it connects." }
    ] : []).concat([
        { kind: "toggle", key: "overlay", title: "Show status over other apps",
          how: "A small Turbine card shows download progress and Added while you're in the browser.",
          value: browserSet.overlay, options: [false, true], names: ["Off", "On"] }
    ]).concat(bridge.paired || bridge.connected > 0 || shot.page === "browser" ? [
        { kind: "action", key: "pair", title: bridgeTitle, how: bridgeWords, action: bridge.paired ? "Pair again" : "Pair" }
    ] : []).concat([
        { kind: "action", key: "browser", title: "Web browser", how: browserWords, action: browserFound ? "Check again" : "How to get one" },
        { kind: "toggle", key: "alwaysSteam", title: "Always use Steam's browser (Game Mode)",
          how: "Nexus opens in Steam's own browser right away, instead of your browser through Steam.",
          value: browserSet.alwaysSteam, options: [false, true], names: ["Off", "On"] },
        { kind: "action", key: "addChrome", title: "Use Chrome in Game Mode", how: "How to add Chrome to Steam once, so Nexus opens in it right over Turbine", action: "Show how" },
        { kind: "action", key: "nxmHandler", title: "Use Turbine for Mod Manager Download",
          how: handler.state === "yes" ? "Turbine gets Mod Manager Download links from the browser (they need a Nexus key)."
             : handler.state === "no" ? "Mod Manager Download links now go to " + (handler.current || "another program") + ". A makes Turbine get them."
             : "Makes the browser's Mod Manager Download button send files to Turbine (it needs your Nexus key; Slow download works without one).",
          action: handler.state === "yes" ? "Set again" : "Use Turbine" }
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
        { kind: "action", key: "restart", title: "Restart Turbine", how: "Closes Turbine and opens the new version", action: "Restart" }
    ] : []).concat([
        { kind: "toggle", key: "auto", title: "Check when Turbine starts", how: "Looks once at each start. Nothing installs without you.", value: updates.auto, options: [false, true], names: ["Off", "On"] },
        { kind: "info", key: "channel", title: "Update channel: " + (labsOn && updates.prerelease ? "Pre-release" : "Stable"), how: labsOn ? "Change it in Labs" : "Stable releases only" }
    ])
    function rowStep(r, d) {
        if (r.key === "auto") updates.auto = !updates.auto
        else if (r.key === "overlay") browserSet.overlay = !browserSet.overlay
        else if (r.key === "alwaysSteam") { if (!shot.on) browserSet.alwaysSteam = !browserSet.alwaysSteam }
        else if (r.key === "channel") { updates.prerelease = !updates.prerelease; Turbine.selfCheck(updates.prerelease, false) }
        note("action", "Setting " + r.key + " changed")
    }
    function rowRun(r) {
        note("action", "Settings: " + r.title)
        if (r.key === "report") openReport(null, "", "")
        else if (r.key === "perf") openPerf()
        else if (r.key === "steamBrowser" || r.key === "keyless" || r.key === "gmBrowser") labsRun(r)
        else if (r.key === "nexusKey") openAsk("nexusKey", "Nexus Mods API key", keyWhere + " It's kept in a file only you can read.", "", "API key", 1024, true, "Check it")
        else if (r.key === "pasteKey") pasteKey()
        else if (r.key === "keyPage") { if (openNexus(keyPageUrl)) toast.showPlain("Opened API Keys. Copy the Personal API Key at the bottom, come back and press Paste.") }
        else if (r.key === "signOut") Turbine.nexusSignOut()
        else if (r.key === "tutorial") openTutorial(0)
        else if (r.key === "unhide") { if (!shot.on) railSet.hidden = "[]"; orderGames(); toast.done("Every game is back on the rail.") }
        else if (r.key === "addChrome") showAddBrowser("noshortcut")
        else if (r.key === "nxmHandler") Turbine.handlerSet()
        else if (r.key === "browser") { if (browserFound) Turbine.browserCheck(); else noBrowser() }
        else if (r.key === "helper") setupHelper()
        else if (r.key === "pair") pair(bridge.paired)
        else if (r.key === "checkNow") Turbine.selfCheck(labsOn && updates.prerelease, false)
        else if (r.key === "install") Turbine.selfInstall()
        else if (r.key === "restart") { Turbine.finish(); Turbine.restart(); Qt.quit() }
        else if (r.key === "stable") { backToStable = true; Turbine.selfCheck(false, true); openSection("Updates") }
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
        toast.showPlain(labsSet.on ? "Labs mode on. Labs is at the end of Settings." : "Labs mode off")
    }
    // Things to confirm on the device: what the build server can't check. title, how to try it, what should happen.
    readonly property var labsItems: [
        { id: "pad-quiet", title: "Controller quiet after a game", how: "Play a game, press buttons as you quit it, then come back to Turbine.",
          expect: "Nothing happens in Turbine from buttons pressed in the game." },
        { id: "proton-bepinex", title: "Proton setting on a BepInEx game", how: "Add a BepInEx mod to Valheim, Lethal Company or Risk of Rain 2, then Play.",
          expect: "The check before Play says BepInEx gets turned on in Proton, and the game starts with the mod working." },
        { id: "running-check", title: "Running game check", how: "With a BepInEx game running, switch back to Turbine and press Play.",
          expect: "Turbine says the game is still running and changes nothing until it's closed." },
        { id: "nexus-keyless", title: "Get mods without a key", how: "Labs, Nexus without a key: Check. Then open Get mods in Skyrim, The Witcher 3 or Cyberpunk 2077.",
          expect: "The check says Nexus answered, and Get mods shows the game's mods as pictures with no key." },
        { id: "nexus-gamemode", title: "Nexus opens in Game Mode", how: "Game Mode: a mod page, A Get it on Nexus.",
          expect: "The browser opens over Turbine on that file's page. If not, after 8 seconds two buttons under Get it offer Steam's browser." },
        { id: "game-mods-folder", title: "Slow download with no Save box", how: "Browser helper set up in Chrome. Press Slow download on a mod's file page.",
          expect: "No Save box; the file lands in Downloads, Game Mods, the game's folder, and Turbine adds it to that game." },
        { id: "desktop-pad", title: "Controller in Desktop Mode", how: "Desktop Mode with Steam running: use Turbine with the controller.",
          expect: "Buttons work (as Steam's mouse and keyboard if Turbine sees no pad itself), and no line says no controller was found." },
        { id: "text-size", title: "Interface size in Game Mode", how: "In Game Mode, look at every page at Interface size 80% to 150%.",
          expect: "Everything is readable; nothing is cut off or overlapping." },
        { id: "add-downloads", title: "Add mods from Downloads", how: "Put a mod's zip in Downloads, then Get mods > Add a file from Downloads.",
          expect: "The mod shows in the list, turned on, with its name and version." },
        { id: "auto-apply", title: "Auto-apply, then start from Steam", how: "Turn a mod on or off, wait 5 seconds, close Turbine, start the game from the Steam library.",
          expect: "The game has the change without pressing Play in Turbine." },
        { id: "uninstall-undo", title: "Uninstall and put back", how: "Uninstall a mod from its options (X), then press A on its dimmed row.",
          expect: "The row stays, dimmed, for 8 seconds; A puts the mod back in the same place. After that it is in Removed (n) at the bottom." },
        { id: "play-original", title: "Play without mods", how: "Game menu (View) > Play without mods.",
          expect: "The game starts with no mods. The mods stay in Turbine and go back in at the next Play." },
        { id: "restore", title: "Restore last setup played", how: "Play with mods, change some, then Game menu (View) > Restore last working setup.",
          expect: "The mods go back to how they were the last time the game was played." },
        { id: "sd-card", title: "Game on the SD card", how: "Add a mod to a game on the SD card and play it; then try with the card out.",
          expect: "The mod works. With the card out, Turbine says the game is offline, never that mods are gone." },
        { id: "heroic-play", title: "Heroic game Play", how: "Pick an Epic or GOG game from Heroic and press Play.",
          expect: "Heroic starts the game with the mods in place." },
        { id: "seasonal", title: "Seasonal leaves and snow", how: "Settings > Look > Seasonal: Autumn, then Snow; try Amount and Size.",
          expect: "They fall smoothly behind the panels and stop while a game runs." },
        { id: "quick-report", title: "Screenshot and report", how: "Labs on: tap View and press A on the middle of the wheel (or hold the icon at the top of the rail for 7 seconds).",
          expect: "A brass ring fills, then a screenshot and report land in Downloads and the report pastes from the clipboard." },
        { id: "ts-install", title: "Thunderstore install", how: "Valheim, Lethal Company or Risk of Rain 2: Get mods, pick a mod, A Install.",
          expect: "It downloads with progress on the status line and the rail ring, and shows in Mods with what it needs." },
        { id: "loader-install", title: "Loader install", how: "Turn on a mod that needs the loader, then A Install in the prompt (SKSE, BepInEx or SMAPI).",
          expect: "The loader downloads from its official home, shows as a mod, and the Needs badges go away." },
        { id: "fomod", title: "Installer choices (FOMOD)", how: "Add a Skyrim mod with an installer, from Get mods or Downloads.",
          expect: "One question per page with pictures; B goes back; the files of the chosen options go in." },
        { id: "load-order", title: "Load order", how: "Skyrim: Load order tab. Turn a plugin off, move one (X, Move), then Sort automatically.",
          expect: "The game loads plugins in that order; masters stay first; missing masters are named." },
        { id: "ts-code", title: "Thunderstore code", how: "Lethal Company or another Thunderstore game, online: game menu (View), Loadouts, Create a Thunderstore code.",
          expect: "A code shows big and is copied; importing it on another device (or Loadouts, Import a Thunderstore profile code) gets the same mods." },
        { id: "other-versions", title: "Other versions", how: "Get mods, a mod page, X Other files, then A on an older version.",
          expect: "That version downloads and takes the installed one's place, same spot and on or off." },
        { id: "nxm-handler", title: "Mod Manager Download", how: "Settings, Accounts: Use Turbine for Nexus downloads. Then in Desktop Mode press Mod Manager Download on a Nexus mod's Files tab, with Turbine open and with it closed.",
          expect: "Turbine comes to the front on that game, the download shows on the status line, and the mod is added (with installer choices when it has them)." },
        { id: "nexus-free", title: "Free Nexus account", how: "With a free account: a Nexus mod's details, A Open on Nexus; also SKSE's Install in Skyrim.",
          expect: "The page opens in the browser; Mod Manager Download there sends the file to Turbine. In Game Mode Turbine says it works best in Desktop Mode." },
        { id: "nexus-requirements", title: "Nexus requirements", how: "Skyrim: details of a Nexus mod that needs others (like SkyUI needs SKSE).",
          expect: "Also needs lists them as Have it, Missing, Game DLC or Outside Nexus; Premium installs the missing ones with the mod, free accounts get their pages." },
        { id: "profile-nexus", title: "Loadout import with Nexus mods", how: "Import a share file that has Nexus mods.",
          expect: "Premium: Turbine downloads them too. Free: it lists the pages to open, and the loadout is made with what's there." },
        { id: "loot-sort", title: "LOOT sorting", how: "Skyrim with a few plugin mods, online: Load order tab, A on Sort automatically, then X on a plugin with a badge.",
          expect: "The line at the top says Sorted with LOOT's masterlist; the game starts with that order; badges like Dirty show LOOT's notes, and X lists them." },
        { id: "nexus-key", title: "Nexus Mods key", how: "Settings, Accounts: add your personal API key with Steam's keyboard.",
          expect: "It says your name and Premium or Free; Nexus Mods browsing works; Sign out removes the key." },
        { id: "mod-updates", title: "Mod updates", how: "Have an older Thunderstore or Nexus mod, open its game.",
          expect: "Update ready shows on it; X Update or the game menu's Update all brings the new version, same place and on or off." },
        { id: "profiles", title: "Loadouts", how: "Game menu (View), Loadouts: make one, switch, export a share file, import it or a Thunderstore code.",
          expect: "Switching changes which mods are on right away; an import gets the mods and makes the loadout." },
        { id: "self-update", title: "Turbine updates itself", how: "Settings, Updates: Check now, Install, then Restart.",
          expect: "The new version installs and opens. With Pre-release in Labs, test builds show; Go back to stable returns to stable." },
        { id: "archives", title: "7z and rar mods", how: "Add a .7z and a .rar mod from Downloads.",
          expect: "Both unpack and install like zip files." },
        { id: "heroic-art", title: "Heroic covers", how: "Look at Heroic (Epic and GOG) games in the rail.",
          expect: "They show their cover art; games without art show their initials." },
        { id: "launch-option", title: "Valheim Launch Option", how: "Valheim with a BepInEx mod: press Play and read the check, then set the Launch Option in Steam.",
          expect: "The check explains the one-time setting once; after setting it, mods load when the game starts from Steam." },
        { id: "no-browser", title: "No browser warning", how: "On a system without a browser, open a Nexus page from Turbine (a mod's details, A Open on Nexus).",
          expect: "A warning says no browser was found, with the Discover and Add to Steam steps; Check again finds one once it's installed." },
        { id: "slow-download", title: "Slow download from Downloads", how: "With no browser helper and no setup: open Nexus any way you like (Get mods, a bookmark, typing the address), find a mod and press Slow download.",
          expect: "When the file finishes, Turbine adds it to the right game, or shows Found <file>. Add it to <game>? with A Add, X another game, B Not a mod. The file stays in Downloads." },
        { id: "catch-closed", title: "Downloads while Turbine is closed", how: "Close Turbine, download a mod with Slow download, then open Turbine.",
          expect: "Turbine catches up on it at start (adds it, or asks). A file you answered Not a mod is never asked about again." },
        { id: "nxm-no-key", title: "Mod Manager Download without a key", how: "With no Nexus key in Settings, Accounts, press Mod Manager Download on a Nexus mod.",
          expect: "Turbine says Mod Manager Download needs your Nexus API key and to use Slow download; a Slow download of that mod then goes into its game by itself." },
        { id: "nexus-no-key", title: "Open on Nexus when browsing fails", how: "If Get mods shows Browse ... mods on Nexus Mods: A Open on Nexus, then X Try again here.",
          expect: "The game's mods open in the browser and Turbine says: Download with Slow download. Turbine adds it when it lands in Downloads." },
        { id: "pointer", title: "Controller pointer", how: "Push the right stick, then press RT on a button, a mod's ON pill, a tab and a Settings row; press LT on a mod. Hold RT and move to drag. Try Game Mode and Desktop Mode, built-in controls and an Xbox pad.",
          expect: "A brass pointer moves smoothly, outlines what it's over, RT clicks it, LT on a mod opens its options. It hides 3 seconds after the stick stops, or at once on any other button; then LT and RT page up and down." },
        { id: "r3-click", title: "R3 is the left mouse button", how: "Show the pointer and press R3 on a mod's ON pill; hold R3 and move the right stick. Then let the pointer hide and press R3 on a mod.",
          expect: "With the pointer: R3 clicks like RT and holding it drags. Hidden: the pointer shows on the highlighted mod and R3 does what A does." },
        { id: "left-stick", title: "Left stick: D-pad and wheel", how: "On My mods push the left stick down and hold it, then let go. Then sweep it round the edge both ways over a long list.",
          expect: "Pushed and held, it moves like the D-pad (one row, then repeating). Swept round, it scrolls: clockwise down, a few rows every 30 degrees, and the highlight stays on a row you can see." },
        { id: "bumpers-tabs", title: "LB and RB are tabs", how: "Press LB and RB on a game, in Settings and on a mod page from Get mods.",
          expect: "Game tabs, Settings sections and the previous or next mod change. The game never changes; the rail does that." },
        { id: "quick-wheel", title: "Quick wheel", how: "Tap View, press Up then Right, then A. Then hold View, push Left, let go.",
          expect: "Tap: the wheel stays open and Sort opens. Hold and push: Games opens when you let go. Empty places are dimmed, never moved." },
        { id: "tutorial", title: "Beginner tutorial", how: "Settings, Help: Beginner tutorial, Show it. Go through the four cards with A; open it again and skip with B.",
          expect: "Big pictures of the real buttons; it ends on the rail with \"Pick a game to start\" and doesn't come back by itself." },
        { id: "focus-memory", title: "Focus comes back", how: "Highlight the third mod, open another game from the rail, come back. Open X on a mod and close it. Open Settings from the wheel and press B.",
          expect: "Each time the highlight is where you left it, on the same tab." },
        { id: "inline-browser", title: "Didn't see Nexus open?", how: "Game Mode: a mod page, A Get it on Nexus, and stay in Turbine for 8 seconds. Press A again quickly the first time.",
          expect: "Only one browser starts. After 8 seconds two buttons show under Get it: Open in Steam's browser and Always use Steam's browser. No box pops up." },
        { id: "rail-menu", title: "Pin and hide on the rail", how: "On the rail press X on a game: Pin to the top; on a tool like Lossless Scaling: Hide from the rail. Settings, Help shows it again.",
          expect: "The pinned game stays at the top; the hidden one is gone from the rail until shown again." },
        { id: "touch-hold", title: "Touch: hold for options", how: "Touch a mod row, a Get mods card and a plugin and hold half a second; swipe sideways on the tabs.",
          expect: "Holding opens that item's options; the swipe changes the tab; the legend hides until the next button press." },
        { id: "fewer-steps", title: "Fewer presses to a mod", how: "A game with no mods, free Nexus account, Game Mode: A on the game in the rail, A on a mod, A Get it on Nexus, Slow download, then Play.",
          expect: "The game opens on Get mods with the first mod highlighted, Get it is highlighted on its page, and Play is highlighted when the mod arrives." },
        { id: "scroll-stick", title: "Left stick in Steam's keyboard", how: "Open Find (Y) so Steam's keyboard shows, move both sticks and press the triggers.",
          expect: "While Steam's keyboard is up, the sticks and triggers do nothing in Turbine." },
        { id: "md5-identify", title: "Files named from Nexus", how: "With a Nexus key, add a mod you downloaded from Nexus by hand (Add from Downloads).",
          expect: "It shows with its Nexus name and version, and gets Update ready badges like mods installed from Get mods." },
        { id: "overlay-desktop", title: "Status over the browser (Desktop Mode)", how: "Desktop Mode: start a Nexus download from Turbine, then stay in the browser.",
          expect: "A small Turbine card stays above the browser with progress, then Added to the game, and goes 6 seconds later. Its buttons work with a tap or click." },
        { id: "overlay-gamemode", title: "Status over the browser (Game Mode)", how: "Game Mode: open a Nexus page from Turbine (it opens over Turbine), download a mod, watch the bottom left corner.",
          expect: "The brass card shows over the browser and never takes the controller; it goes when idle, and the Performance Overlay still works after." },
        { id: "gamemode-window", title: "Turbine's window in Game Mode", how: "Open Turbine in Game Mode and use it for a minute (this build draws through X11 on SteamOS).",
          expect: "Turbine fills the screen, looks sharp, and the controller works as before." },
        { id: "browser-helper", title: "Browser helper (optional)", how: "Settings, Accounts: Browser helper (optional), Set up, follow the steps, then Pair. Download a mod on Nexus.",
          expect: "The pairing page says Paired; Settings says Connected; the Nexus page shows Turbine's progress, Added, and installer choices that work from either side." },
        { id: "gm-browser-steam", title: "Browser opens in Game Mode", how: "Add Google Chrome to Steam (Desktop Mode, Steam, Games, Add a Non-Steam Game), set up and pair the browser helper, then in Game Mode press Get it on Nexus on a mod.",
          expect: "Chrome shows over Turbine and opens the mod's page in a new tab by itself. Without Chrome in Steam, Add Chrome to Steam shows once and Recheck finds it after adding. The activity log lists each step (found, started through Steam, sent to the helper)." },
        { id: "steam-browser", title: "Steam's built-in browser", how: "Game Mode: Labs, Open Nexus in Steam's browser, sign in, then Slow download a mod for the game on screen.",
          expect: "The file arrives in Downloads and Turbine asks to add it. If it doesn't, say in the report where the file went (or that Steam didn't download it)." },
        { id: "cyberpunk", title: "Cyberpunk 2077 mods", how: "Cyberpunk 2077: add Cyber Engine Tweaks, RED4ext and a mod that needs them, plus a REDmod mod; press Play.",
          expect: "The check names missing frameworks and explains REDmod once; Proton settings get winmm and version; the mods load in the game." },
        { id: "kcd-mods", title: "KCD mods load", how: "Kingdom Come: Deliverance: add a mod from Nexus (a folder with mod.manifest and Data), reorder two mods, press Play.",
          expect: "Mods go in Mods/<name>; Mods/mod_order.txt lists them in Turbine's order; the mods work in the game." },
        { id: "kcd2-mods", title: "KCD II mods load", how: "Kingdom Come: Deliverance II: add two mods from Nexus, turn one off, press Play.",
          expect: "Mods go in the game's mods folder; mod_order.txt lists only the one that is on; it works in the game." },
        { id: "witcher3", title: "The Witcher 3 mods", how: "The Witcher 3: add two mods, reorder them, press Play (start the game once first if it never ran).",
          expect: "Mods go in mods/modName; their order is written to mods.settings; scripts that clash show in the check; settings notes show after adding." },
        { id: "skyrim-nexus", title: "Skyrim from Nexus", how: "Skyrim Special Edition: get SKSE, Address Library and an SKSE plugin mod from Nexus (Mod Manager Download or Slow download).",
          expect: "SKSE goes in as the loader; without Address Library the check says it's missing; with it, the plugin loads." }
    ]
    readonly property var labsRows: [
        { kind: "action", key: "report", title: "Report a bug", how: "Screenshot, what went wrong, details Turbine adds by itself", action: "Report" },
        { kind: "action", key: "perf", title: "Performance check", how: "Frame times, processor, startup, controller", action: "Open" },
        { kind: "choice", key: "channel", title: "Update channel", how: "Pre-release gets test builds first", value: updates.prerelease, options: [false, true], names: ["Stable", "Pre-release"] },
        { kind: "action", key: "keyless", title: "Nexus without a key", how: labsResult ? labsResult.text : "Asks Nexus for this game's mods with no key and says what came back", action: "Check" },
        { kind: "action", key: "steamBrowser", title: "Open Nexus in Steam's browser", how: "For the Steam's built-in browser check below: opens nexusmods.com in Steam and watches Downloads for the game on screen", action: "Open" },
        { kind: "action", key: "gmBrowser", title: "Browser through Steam (Game Mode)",
          how: labsShortcut ? labsShortcut : "Looks for Chrome among Steam's non-Steam games, starts it through Steam, and asks the helper to open nexusmods.com", action: "Try it" },
        { kind: "action", key: "activity", title: "Activity log", how: "The last 500 lines of what Turbine did, to read, copy or save to Downloads", action: "Open" }
    ].concat(updates.prerelease ? [
        { kind: "action", key: "stable", title: "Go back to stable", how: "Installs the latest stable Turbine, even if it's older, and turns Pre-release off", action: "Go back" }
    ] : []).concat(labsItems.map((x, i) => Object.assign({ kind: "item", head: i === 0 ? "To confirm on your device" : "" }, x)))
    property var labsAnswers: parse(labsSet.states, {})
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
    property string labsShortcut: ""
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
        if (row.key === "report") openReport(null, "", "")
        else if (row.key === "perf") openPerf()
        else if (row.key === "activity") openActivity()
        else if (row.key === "keyless") { Turbine.nexusKeylessCheck(); toast.info("Asking Nexus...") }
        else if (row.key === "gmBrowser") {
            const r = parse(Turbine.browserShortcut(), { found: false })
            labsShortcut = r.found ? "Found " + r.browser + " in Steam as " + r.name + " (" + r.url + ")" : "No Chrome-family browser among Steam's non-Steam games"
            if (!r.found) showAddBrowser("noshortcut")
            else openNexus("https://www.nexusmods.com/")
        }
        else if (row.key === "steamBrowser") {
            const url = Turbine.watchSteamBrowser()
            if (!shot.on) Qt.openUrlExternally(url)
            toast.showPlain("Opened Nexus Mods in Steam's browser. Try a Slow download; Turbine asks before adding what it finds.")
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
            reportView.aboutLine = "Send saves the report to Downloads and copies it, with Turbine " + Turbine.version
                + ", this device, the game and its mods, and the last 300 lines of the activity log."
            reportView.open(item ? item.title : "", startText !== "" ? startText : item ? "Problem: " + item.title + ". " : "",
                            perfText, Turbine.hasCrashLog())
            perfView.visible = false
            sheet = "report"
        }
        if (!app.contentItem.grabToImage(function (r) { open(r.saveToFile(tmp)) })) open(false)
    }
    function sendReport() {
        if (reportView.text.trim() === "") { toast.showPlain("Say what went wrong first."); return }
        const stamp = Turbine.stamp()
        const withShot = reportView.include && reportView.shotUrl !== ""
        const shotName = withShot ? "Turbine-screenshot-" + stamp + ".png" : ""
        const text = Turbine.report(labsItemText(reportItem), reportView.text, reportContext,
                                    reportView.includePerf ? reportView.perf : "", reportView.includeLog && reportView.hasLog, shotName)
        const res = parse(Turbine.saveReport(text, stamp, withShot ? Turbine.shotTemp() : ""), { error: "Couldn't save the report." })
        if (res.error !== "") { toast.showPlain(res.error); return }
        Turbine.copyText(text)
        closeReport()
        toast.showPlain("Report saved to Downloads as " + res.report + ". The report is copied.")
    }
    function closeReport() {
        reportView.close()
        discardArmed = false
        if (sheet === "report") sheet = ""
    }
    function dismissReport() {
        if (reportView.text.trim() === "" || discardArmed) { note("labs", "Report closed without saving"); closeReport(); return }
        discardArmed = true
        discardReset.restart()
        toast.showPlain("Press B again to throw this report away.")
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

    // Labs: hold View (or the icon at the top of the rail) for 7 seconds: a screenshot and a report go to Downloads
    // and the report is copied. Letting go early stops it.
    property real holdMs: 0
    Timer {
        id: holdTimer
        interval: 50
        repeat: true
        onTriggered: {
            app.holdMs += interval
            if (app.holdMs >= 7000) { stop(); app.holdMs = 0; app.quickReport() }
        }
    }
    readonly property bool holdShown: holdTimer.running && holdMs >= 300
    function holdStart(how) {
        if (!labsOn || holdTimer.running) return
        holdMs = 0
        holdTimer.start()
        note("labs", "Holding " + how + " for a screenshot and report")
    }
    function holdStop() {
        if (!holdTimer.running) return
        holdTimer.stop()
        holdMs = 0
        note("labs", "Let go early: no screenshot")
    }
    function quickReport() {
        const stamp = Turbine.stamp()
        const tmp = Turbine.shotTemp()
        const done = function (ok) {
            const shotName = ok ? "Turbine-screenshot-" + stamp + ".png" : ""
            const text = Turbine.report("", "", interfaceText(), "", false, shotName)
            const res = parse(Turbine.saveReport(text, stamp, ok ? tmp : ""), { error: "Couldn't save the report." })
            if (res.error !== "") { toast.showPlain(res.error); return }
            Turbine.copyText(text)
            toast.showPlain(ok ? "Screenshot and report saved to Downloads. The report is copied." : "Report saved to Downloads (the screenshot didn't work). The report is copied.")
        }
        // A moment for the ring to go before the picture.
        Qt.callLater(function () { if (!app.contentItem.grabToImage(function (r) { done(r.saveToFile(tmp)) })) done(false) })
    }

    // The app icon: the small, simpler one where the full one would be under 64 physical pixels.
    readonly property string iconFull: "file:///app/share/icons/hicolor/128x128/apps/io.github.ravenor907.TurbineModManager.png"
    readonly property string iconSmall: "file:///app/share/turbine-mod-manager/icon-small.png"

    // ---------------------------------------------------------------- Look settings rows
    readonly property var layoutOptions: externalShown ? ["handheld", "desktop", "tv"] : ["handheld", "desktop"]
    readonly property var lookRows: [
        { label: "Interface size", key: "size", slider: true, min: 80, max: sizeCap, unit: "%", now: sizeNow,
          words: sizeNow === sizeCap && sizeCap < 150 ? "Largest that fits" : "",
          step: (d) => sizePreview(sizeStep(d)) },
        { label: "Layout", options: layoutOptions, names: layoutOptions.map(o => o === "tv" ? "TV" : o.charAt(0).toUpperCase() + o.substring(1)), key: "layout", now: layoutNow,
          step: (d) => layoutPreview(layoutOptions[(layoutOptions.indexOf(layoutNow) + d + layoutOptions.length) % layoutOptions.length]) },
        { label: "Theme", options: ["brass", "black", "light"], names: ["Brass", "Black", "Light"], key: "theme",
          step: (d) => cycle("theme", ["brass", "black", "light"], d) },
        { label: "Accent", options: Object.keys(c.swatches), key: "accent", swatch: true,
          step: (d) => cycle("accent", ["brass", "iron", "copper", "jade", "fjord", "heather", "ember"], d) },

        { label: "Contrast", options: [false, true], names: ["Standard", "High"], key: "highContrast",
          step: (d) => { look.highContrast = !look.highContrast } },
        { label: "Color vision", options: ["typical", "red", "green", "blue", "none"], names: ["Typical", "Red", "Green", "Blue", "None"], key: "vision",
          step: (d) => cycle("vision", ["typical", "red", "green", "blue", "none"], d) },

        { label: "Seasonal", options: ["off", "date", "autumn", "snow"], names: ["Off", "By date", "Autumn", "Snow"], key: "season",
          now: seasonPick, step: (d) => { look.season = seasonPick; cycle("season", ["off", "date", "autumn", "snow"], d) } }
    ].concat(seasonKind !== "" ? [
        { label: "Amount", key: "fallAmount", slider: true, min: 10, max: 100, unit: "",
          step: (d) => slide("fallAmount", 10, 100, 10, d) },
        { label: "Size", key: "fallSize", slider: true, min: 50, max: 200, unit: "%",
          step: (d) => slide("fallSize", 50, 200, 25, d) }
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

    // ---------------------------------------------------------------- shared pieces
    component Label: Text {
        color: c.text
        font.family: app.bodyFont
        font.pixelSize: app.px(15)
        elide: Text.ElideRight
        verticalAlignment: Text.AlignVCenter
    }
    component Key: Rectangle {
        property string k: ""
        property bool square: k.length > 1
        implicitWidth: Math.max(app.px(24), keyText.implicitWidth + app.px(12))
        implicitHeight: app.px(24)
        radius: square ? app.px(5) : height / 2
        color: c.raised
        border.color: c.accentLo
        border.width: Math.max(1, app.px(1))
        Text {
            id: keyText
            anchors.centerIn: parent
            text: app.glyph(parent.k)
            color: c.text
            font.family: app.bodyFont
            font.pixelSize: app.px(14)
            font.bold: true
        }
    }
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
    readonly property bool keysLegend: !Turbine.gamemode && !shot.on && (lastInput === "mouse" || (lastInput === "keys" && Turbine.pads))
    readonly property var keyNames: ({ a: "Enter", b: "Esc", x: "X", y: "Y", lb: "Q", rb: "E", lt: "PgUp", rt: "PgDn", view: "V", start: "P", r3: "F" })
    function glyph(k) {
        if (keysLegend && keyNames[k]) return keyNames[k]
        if (k === "view" && padKind === "360") return "BACK"
        if (k === "start" && padKind === "menu") return "MENU"
        return k.toUpperCase()
    }
    // One or more keys ("lb+rb") and what they do.
    component Hint: Row {
        id: hint
        property string k: ""
        property string label: ""
        property string pre: ""           // "hold": the legend reads "hold A Delete"
        property string k2: ""            // a second key and its words in one hint ("RT Click, LT Menu")
        property string label2: ""
        // In the legend, a hint for one button is also a button: a tap or click does what the button does.
        readonly property bool clickable: parent === legendRow && ["a", "b", "x", "y"].indexOf(k) >= 0
        spacing: app.px(6)
        TapHandler { enabled: hint.clickable; onTapped: { app.note("action", "Legend " + hint.k + " (" + hint.label + ")"); app.act(hint.k) } }
        HoverHandler { enabled: hint.clickable; onHoveredChanged: app.hover(hint, hovered) }
        Label { visible: hint.pre !== ""; text: hint.pre; font.pixelSize: app.px(15); color: c.soft; anchors.verticalCenter: parent.verticalCenter }
        Repeater {
            model: hint.k.split("+")
            Key { required property string modelData; k: modelData; visible: modelData !== ""; anchors.verticalCenter: parent.verticalCenter }
        }
        Label { visible: hint.label !== ""; text: hint.label + (hint.k2 !== "" ? "," : ""); font.pixelSize: app.px(15); color: c.soft; anchors.verticalCenter: parent.verticalCenter }
        Key { visible: hint.k2 !== ""; k: hint.k2; anchors.verticalCenter: parent.verticalCenter }
        Label { visible: hint.label2 !== ""; text: hint.label2; font.pixelSize: app.px(15); color: c.soft; anchors.verticalCenter: parent.verticalCenter }
    }
    component Icon: Shape {
        property string d: ""
        property color tint: c.muted
        width: app.px(22); height: app.px(22)
        preferredRendererType: Shape.CurveRenderer
        ShapePath {
            strokeColor: tint
            strokeWidth: 2
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            scale: Qt.size(app.px(22) / 24, app.px(22) / 24)
            PathSvg { path: d }
        }
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
    // Seasonal leaves or snow, behind everything. Still while Turbine isn't in front (a game is running).
    Seasonal {
        id: fallen
        anchors.fill: parent
        kind: app.seasonKind
        amount: look.fallAmount
        size: look.fallSize
        scale: app.s
        light: c.light
        moving: app.active || shot.on
    }
    Row {
        id: mainRow
        anchors.fill: parent

        // ===== Rail: the logo, the games (last played first, portrait art), Settings pinned at the bottom.
        Rectangle {
            id: rail
            width: app.px(72)
            height: parent.height
            color: c.rail
            z: 5
            Column {
                id: railTop
                anchors.top: parent.top
                anchors.topMargin: app.px(10)
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: app.px(10)
                // The app icon (the simpler small one under 64 physical pixels); a plain "T" if it can't be loaded.
                // In Labs mode, holding it for 7 seconds saves a screenshot and a report.
                Image {
                    id: logo
                    readonly property bool small: app.px(44) * Screen.devicePixelRatio < 64
                    width: app.px(44); height: app.px(44)
                    anchors.horizontalCenter: parent.horizontalCenter
                    source: small ? app.iconSmall : app.iconFull
                    sourceSize.width: app.px(88)
                    smooth: true
                    mipmap: true
                    visible: status === Image.Ready
                    HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                    TapHandler {
                        onPressedChanged: pressed ? app.holdStart("the rail icon") : app.holdStop()
                    }
                }
                Rectangle {
                    visible: logo.status !== Image.Ready
                    width: app.px(44); height: app.px(44); radius: width / 2
                    anchors.horizontalCenter: parent.horizontalCenter
                    gradient: Gradient {
                        GradientStop { position: 0; color: c.accentHi }
                        GradientStop { position: 1; color: c.accent }
                    }
                    Text {
                        anchors.centerIn: parent
                        text: "T"
                        color: c.onAccent
                        font.family: app.titleFont
                        font.pixelSize: app.px(24)
                        font.bold: true
                    }
                }
                Rectangle { width: app.px(28); height: app.px(2); radius: 1; color: c.line; anchors.horizontalCenter: parent.horizontalCenter }
            }
            ListView {
                id: railList
                anchors.top: railTop.bottom
                anchors.topMargin: app.px(10)
                anchors.bottom: gear.top
                anchors.bottomMargin: app.px(10)
                width: parent.width
                clip: true
                spacing: app.px(8)
                model: app.games.length
                currentIndex: app.zone === "rail" ? Math.min(app.railIndex, app.games.length - 1) : app.current
                highlightFollowsCurrentItem: false
                onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                delegate: Item {
                    id: tileItem
                    required property int index
                    readonly property var g: app.games[index]
                    readonly property bool here: index === app.current && app.page === "game"
                    readonly property bool focused: app.zone === "rail" && app.railIndex === index
                    width: railList.width
                    height: app.px(66)
                    Rectangle {
                        visible: tileItem.here
                        x: app.px(1); width: app.px(4); height: app.px(36); radius: app.px(2)
                        anchors.verticalCenter: parent.verticalCenter
                        color: c.accent
                    }
                    Rectangle {
                        id: tile
                        width: app.px(44); height: app.px(66)
                        anchors.horizontalCenter: parent.horizontalCenter
                        radius: app.px(6)
                        color: c.raised
                        border.width: tileItem.here || tileItem.focused ? app.px(2) : 0
                        border.color: tileItem.focused ? c.text : c.accent
                        clip: true
                        Image {
                            id: art
                            anchors.fill: parent
                            anchors.margins: tile.border.width
                            visible: status === Image.Ready
                            source: tileItem.g ? app.fileUrl(tileItem.g.cover || tileItem.g.header) : ""
                            fillMode: Image.PreserveAspectCrop
                            sourceSize.height: app.px(132)
                            asynchronous: true
                        }
                        // No art: the whole name (shrunk to fit, never cut, so "Deliverance II" keeps its II) and
                        // the store under it, so two copies or two games with long names still look different.
                        Text {
                            id: tileName
                            visible: !art.visible
                            x: app.px(3); y: app.px(4)
                            width: parent.width - app.px(6)
                            height: parent.height - app.px(8) - tileStore.height
                            horizontalAlignment: Text.AlignHCenter
                            verticalAlignment: Text.AlignVCenter
                            wrapMode: Text.WordWrap
                            maximumLineCount: 6
                            elide: Text.ElideNone
                            // Long words ("Cyberpunk", "Deliverance") shrink to fit the tile instead of being cut.
                            fontSizeMode: Text.Fit
                            minimumPixelSize: Math.max(5, app.px(5))
                            text: tileItem.g ? tileItem.g.name : ""
                            color: c.soft
                            font.family: app.bodyFont
                            font.pixelSize: app.px(10)
                            font.bold: true
                        }
                        Text {
                            id: tileStore
                            visible: !art.visible
                            anchors.bottom: parent.bottom
                            anchors.bottomMargin: app.px(3)
                            width: parent.width
                            horizontalAlignment: Text.AlignHCenter
                            text: tileItem.g ? (tileItem.g.launcher === "Steam" ? "Steam" : (tileItem.g.storeLabel || tileItem.g.launcher || "")) : ""
                            color: c.muted
                            font.family: app.bodyFont
                            font.pixelSize: app.px(8)
                            fontSizeMode: Text.HorizontalFit
                            minimumPixelSize: Math.max(5, app.px(5))
                        }
                    }
                    // A mod waiting or an update ready.
                    Rectangle {
                        visible: tileItem.here && app.updateCount > 0
                        anchors.right: tile.right; anchors.top: tile.top
                        anchors.margins: -app.px(3)
                        width: app.px(10); height: width; radius: width / 2
                        color: c.accentHi
                        border.color: c.rail; border.width: app.px(2)
                    }
                    HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                    TapHandler { onTapped: { app.railIndex = tileItem.index; app.railSelect(tileItem.index) } }
                }
            }
            Rectangle {
                id: gear
                anchors.bottom: parent.bottom
                anchors.bottomMargin: app.px(10)
                anchors.horizontalCenter: parent.horizontalCenter
                width: app.px(44); height: app.px(44); radius: app.px(8)
                readonly property bool focused: app.zone === "rail" && app.railIndex === app.games.length
                color: app.page === "settings" || app.page === "perf" ? c.raised : "transparent"
                border.width: focused ? app.px(2) : 0
                border.color: c.text
                Icon {
                    anchors.centerIn: parent
                    tint: app.page === "settings" ? c.accent : c.muted
                    d: "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.6 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.6a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"
                }
                // A newer Turbine is ready to install.
                Rectangle {
                    visible: app.selfUpd.state === "available"
                    anchors.right: parent.right; anchors.top: parent.top
                    anchors.margins: -app.px(2)
                    width: app.px(10); height: width; radius: width / 2
                    color: c.accentHi
                    border.color: c.rail; border.width: app.px(2)
                }
                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                TapHandler { onTapped: app.openSettings() }
            }
        }

        // ===== Main
        Item {
            id: main
            width: parent.width - rail.width
            height: parent.height

            Column {
                id: mainCol
                anchors.fill: parent
                anchors.leftMargin: app.px(14)
                anchors.rightMargin: app.px(14)
                anchors.bottomMargin: legend.height + app.px(16)
                spacing: app.px(8)

                // Header: the game's wide art darkened under the words, its name, the status under it, Start Play.
                Rectangle {
                    id: header
                    width: parent.width + app.px(28)
                    x: -app.px(14)
                    height: app.px(72)
                    color: c.panel
                    clip: true
                    Image {
                        id: hero
                        anchors.fill: parent
                        visible: app.page === "game" && status === Image.Ready
                        source: app.page === "game" && app.game && app.game.header ? app.fileUrl(app.game.header) : ""
                        fillMode: Image.PreserveAspectCrop
                        asynchronous: true
                    }
                    Rectangle {
                        visible: hero.visible
                        anchors.fill: parent
                        gradient: Gradient {
                            orientation: Gradient.Horizontal
                            GradientStop { position: 0; color: Qt.rgba(c.ground.r, c.ground.g, c.ground.b, 0.85) }
                            GradientStop { position: 1; color: Qt.rgba(c.ground.r, c.ground.g, c.ground.b, 0.40) }
                        }
                    }
                    Rectangle {
                        id: backBtn
                        visible: app.page !== "game"
                        x: app.px(14)
                        anchors.verticalCenter: parent.verticalCenter
                        width: backRow.implicitWidth + app.px(20); height: app.px(32); radius: app.px(6)
                        color: c.raised
                        Hint { id: backRow; anchors.centerIn: parent; k: "b"; label: "Back" }
                        HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                        TapHandler { onTapped: app.act("b") }
                    }
                    Column {
                        x: backBtn.visible ? backBtn.x + backBtn.width + app.px(14) : app.px(16)
                        width: parent.width - x - playBtn.width - app.px(40) - (resultCaps.visible ? resultCaps.width + app.px(14) : 0)
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: app.px(2)
                        Text {
                            id: headerTitle
                            width: parent.width
                            elide: Text.ElideRight
                            text: app.page === "settings" ? "Settings" : app.page === "perf" ? "Scrolling test"
                                  : app.modPage ? "Get mods > " + (app.detailsData && app.detailsData.name ? app.detailsData.name : app.detailsItem ? app.detailsItem.name : "")
                                  : app.game ? app.gameTitle : "Turbine Mod Manager"
                            color: c.text
                            font.family: app.page === "game" && app.game && !app.modPage ? app.titleFont : app.bodyFont
                            font.pixelSize: app.px(20)
                            font.weight: Font.DemiBold
                        }
                        Row {
                            width: parent.width
                            spacing: app.px(8)
                            Rectangle {
                                width: app.px(8); height: width; radius: width / 2
                                anchors.verticalCenter: parent.verticalCenter
                                color: app.statusNow.level === "warn" ? c.warn : app.statusNow.level === "stop" ? c.stop : app.statusNow.level === "busy" ? c.accentHi : c.ok
                            }
                            Text {
                                id: statusWords
                                width: Math.min(implicitWidth, parent.width - app.px(16) - (pill.visible ? pill.width + app.px(8) : 0))
                                elide: Text.ElideRight
                                anchors.verticalCenter: parent.verticalCenter
                                text: app.statusNow.words + (app.statusNow.detail ? " · " + app.statusNow.detail : "")
                                color: app.statusNow.level === "stop" ? c.stop : app.statusNow.level === "warn" ? c.warn : c.soft
                                font.family: app.bodyFont
                                font.pixelSize: app.px(14)
                            }
                            // While anything downloads: "Getting 2 mods · 64%"; otherwise updates ready.
                            Rectangle {
                                id: pill
                                readonly property string words: app.dl.active ? "Getting " + app.plural(Math.max(1, Turbine.netBusy), "mod", "mods") + (app.dlPart >= 0 ? " · " + Math.round(app.dlPart * 100) + "%" : "")
                                                                 : app.statusNow.pill || ""
                                visible: words !== ""
                                anchors.verticalCenter: parent.verticalCenter
                                width: pillText.implicitWidth + app.px(16); height: app.px(22); radius: height / 2
                                color: c.raised
                                border.color: c.accentLo
                                Text { id: pillText; anchors.centerIn: parent; text: pill.words; color: c.accentText; font.family: app.bodyFont; font.pixelSize: app.px(13); font.bold: true }
                                // Updates ready: the list of them (a free account picks each one's page there).
                                TapHandler { onTapped: if (app.updateCount > 0 && !app.dl.active && !(Turbine.pending > 0 && Turbine.pendingGame === app.game.id)) app.openUpdates(); else if (pill.words.indexOf("Loadout: ") === 0) app.openProfiles() }
                            }
                        }
                    }
                    // The mod page: LB and RB step through the results.
                    Row {
                        id: resultCaps
                        visible: app.page === "game" && app.modPage && app.getItems.length > 1
                        anchors.right: playBtn.left
                        anchors.rightMargin: app.px(14)
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: app.px(6)
                        Key { k: "lb"; anchors.verticalCenter: parent.verticalCenter }
                        Label { text: "Mod " + (Math.max(0, app.getItems.findIndex(x => app.detailsItem && String(x.id) === String(app.detailsItem.id))) + 1) + " of " + app.getItems.length; font.pixelSize: app.px(14); color: c.soft; anchors.verticalCenter: parent.verticalCenter }
                        Key { k: "rb"; anchors.verticalCenter: parent.verticalCenter }
                    }
                    Rectangle {
                        id: playBtn
                        visible: app.page === "game" && app.game !== null
                        anchors.right: parent.right
                        anchors.rightMargin: app.px(14)
                        anchors.verticalCenter: parent.verticalCenter
                        width: playRow.implicitWidth + app.px(24); height: app.px(38); radius: app.px(8)
                        opacity: Turbine.busy ? 0.7 : 1
                        // The focus ring when Play has the focus (a mod just arrived, or Up from the top).
                        border.color: c.text
                        border.width: app.zone === "play" && app.sheet === "" && !app.wheelOpen ? app.px(3) : 0
                        gradient: Gradient {
                            GradientStop { position: 0; color: c.accentHi }
                            GradientStop { position: 1; color: c.accent }
                        }
                        Row {
                            id: playRow
                            anchors.centerIn: parent
                            spacing: app.px(8)
                            Key { k: "start"; anchors.verticalCenter: parent.verticalCenter }
                            Text { text: "Play"; color: c.onAccent; font.family: app.bodyFont; font.pixelSize: app.px(16); font.bold: true; anchors.verticalCenter: parent.verticalCenter }
                        }
                        HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                        TapHandler { onTapped: app.startPlay() }
                    }
                }

                // Tabs (game page only): My mods, Get mods, and Load order for plugin games. LB and RB switch them.
                // The mod page is its own screen: no tabs over it.
                Rectangle {
                    visible: app.page === "game" && app.game !== null && !app.modPage
                    width: parent.width
                    height: app.px(38)
                    radius: app.px(6)
                    color: c.panel
                    Row {
                        id: tabRow
                        anchors.fill: parent
                        anchors.margins: app.px(3)
                        spacing: app.px(4)
                        Key { id: ltKey; k: "lb"; anchors.verticalCenter: parent.verticalCenter }
                        Repeater {
                            model: app.tabs
                            Rectangle {
                                required property int index
                                required property string modelData
                                width: (tabRow.width - ltKey.width * 2 - tabRow.spacing * (app.tabs.length + 1)) / app.tabs.length
                                height: tabRow.height
                                radius: app.px(5)
                                color: app.tab === index ? c.raised : "transparent"
                                Rectangle { visible: app.tab === parent.index; anchors.bottom: parent.bottom; width: parent.width; height: app.px(2); color: c.accent }
                                Label { anchors.centerIn: parent; text: parent.modelData; font.weight: Font.DemiBold; color: app.tab === parent.index ? c.text : c.muted }
                                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                TapHandler { onTapped: { app.modPage = false; app.tab = parent.index } }
                            }
                        }
                        Key { k: "rb"; anchors.verticalCenter: parent.verticalCenter }
                    }
                    // Touch: a sideways swipe on the tabs is LB or RB.
                    DragHandler {
                        target: null
                        xAxis.enabled: true; yAxis.enabled: false
                        acceptedDevices: PointerDevice.TouchScreen
                        onActiveChanged: if (!active && Math.abs(activeTranslation.x) > app.px(60)) { app.note("touch", "Swipe on the tabs"); app.switchTab(activeTranslation.x < 0 ? 1 : -1) }
                    }
                }
                // Page content
                Item {
                    id: content
                    width: parent.width
                    height: mainCol.height - y
                    clip: true

                    // No games, mods being read.
                    EmptyState {
                        id: middle
                        readonly property bool none: !app.game
                        readonly property bool loading: app.game !== null && app.tab === 0 && !app.modsLoaded
                        ui: kit
                        anchors.fill: parent
                        visible: app.page === "game" && (none || loading)
                        title: none ? "No games yet" : "Reading mods..."
                        body: none ? "Turbine looks in Steam's libraries, including SD cards, and in Heroic. Install a game and it shows up here." : ""
                        buttonKey: none ? "a" : ""
                        buttonLabel: none ? "Look again" : ""
                        onActivated: app.load()
                    }

                    // My mods: "+ Get mods for <game>", the mods, then the collapsed Removed row.
                    Row {
                        id: listBarRow
                        visible: app.page === "game" && app.game !== null && app.tab === 0 && app.modsLoaded && !app.modPage && app.mods.length > 0
                        height: visible ? app.px(36) : 0
                        x: app.px(4)
                        spacing: app.px(8)
                        Repeater {
                            model: app.listBar
                            Rectangle {
                                required property int index
                                required property var modelData
                                readonly property bool foc: app.zone === "main" && app.sheet === "" && app.modIndex < 0 && app.listBarIndex === index
                                width: chipText.implicitWidth + app.px(28); height: app.px(32); radius: app.px(6)
                                color: foc ? c.focusFill : c.card
                                border.color: foc ? c.accent : c.line
                                border.width: foc ? Math.max(1, app.px(2)) : Math.max(1, app.px(1))
                                Text { id: chipText; anchors.centerIn: parent; text: parent.modelData.label; color: c.text; font.family: app.bodyFont; font.pixelSize: app.px(14); font.weight: Font.DemiBold }
                                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                TapHandler { onTapped: { app.modIndex = -1; app.listBarIndex = parent.index; app.listBarRun(parent.index) } }
                            }
                        }
                    }
                    Label {
                        id: moveLine
                        y: listBarRow.height
                        visible: app.page === "game" && app.tab === 0 && (app.moving || app.findText !== "" || app.selecting || app.listShow !== "all")
                        height: visible ? app.px(26) : 0
                        x: app.px(6)
                        text: app.moving ? "Lower in the list wins when two mods change the same file."
                              : app.selecting ? app.plural(app.tickedCount, "mod", "mods") + " ticked. A ticks, X acts on them, B done."
                              : "Showing " + app.plural(app.shownMods.length, "mod", "mods") + (app.findText !== "" ? " with \"" + app.findText + "\"" : "") + (app.listShow !== "all" ? " · " + app.showNames[app.listShow] : "") + " · B Show all"
                        color: c.soft
                        font.pixelSize: app.px(14)
                    }
                    ListView {
                        id: modList
                        visible: app.page === "game" && app.game !== null && app.tab === 0 && app.modsLoaded && !app.modPage
                        anchors.fill: parent
                        anchors.topMargin: listBarRow.height + moveLine.height
                        topMargin: app.px(4)
                        bottomMargin: app.px(4)
                        clip: true
                        spacing: app.px(app.desktopLayout ? 3 : 6)
                        model: app.listItems
                        currentIndex: Math.max(0, app.modIndex)
                        highlightFollowsCurrentItem: false
                        onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                        // After the left stick scrolls, focus the nearest row still in view.
                        onMovementEnded: {
                            const top = indexAt(10, contentY + app.px(10)), bottom = indexAt(10, contentY + height - app.px(10))
                            if (top >= 0 && app.modIndex < top) app.modIndex = top
                            else if (bottom >= 0 && app.modIndex > bottom) app.modIndex = bottom
                        }
                        delegate: Item {
                            id: entry
                            required property int index
                            required property var modelData
                            readonly property string kind: modelData.kind
                            readonly property var m: modelData.m
                            readonly property bool foc: app.zone === "main" && app.sheet === "" && app.modIndex === index
                            readonly property Item pill: row.pillItem
                            width: modList.width
                            height: kind === "mod" ? row.implicitHeight : app.px(app.desktopLayout ? 38 : 46)
                            ModRow {
                                id: row
                                visible: entry.kind === "mod"
                                ui: kit
                                x: app.px(4)
                                width: parent.width - app.px(8)
                                name: entry.m.name || ""
                                detail: app.modWords(entry.m)
                                isOn: entry.m.enabled === true
                                ordered: app.ordered && app.listSort === "order"
                                compact: app.desktopLayout
                                order: entry.kind === "mod" ? app.mods.findIndex(x => x.id === entry.m.id) + 1 : 0
                                ticked: app.selecting && entry.kind === "mod" ? (app.ticked[entry.m.id] ? 1 : 0) : -1
                                aLabel: app.selecting ? (app.ticked[entry.m.id] ? "Untick" : "Tick") : (entry.m.enabled === true ? "Turn off" : "Turn on")
                                xLabel: app.selecting ? (app.tickedCount > 0 ? "Act on " + app.tickedCount : "") : "More"
                                focused: entry.foc
                                badge: app.moving && entry.foc ? "Moving" : (entry.m.needsLoader && app.loaderMissing ? "Needs " + app.loaderShort
                                       : entry.kind === "mod" && app.modWarns[entry.m.id] ? app.modWarns[entry.m.id]
                                       : entry.kind === "mod" && entry.m.enabled && app.missingNeeds(entry.m).length > 0 ? "Missing requirement"
                                       : app.updateMap[entry.m.id] ? "Update ready" : entry.kind === "mod" && entry.m.enabled && app.isHidden(entry.m) ? "Hidden by another mod"
                                       : entry.kind === "mod" && app.isNew(entry.m) ? "New" : "")
                                badgeLevel: app.moving && entry.foc ? "info" : (entry.m.needsLoader && app.loaderMissing) || (entry.kind === "mod" && entry.m.enabled && (!!app.modWarns[entry.m.id] || app.missingNeeds(entry.m).length > 0)) ? "warn" : "info"
                                // The legend says what A and X do: nothing printed twice.
                                showHints: false
                                onClicked: app.modIndex = entry.index
                                onToggleClicked: { app.modIndex = entry.index; app.toggle(entry.m) }
                                // A right-click (LT with the pointer) opens the mod's menu, like X.
                                onMenuClicked: if (app.sheet === "" && !app.moving) { app.modIndex = entry.index; app.zone = "main"; app.sheet = "modmenu" }
                            }
                            Rectangle {
                                visible: entry.kind !== "mod"
                                x: app.px(4)
                                width: parent.width - app.px(8)
                                height: parent.height
                                radius: app.px(6)
                                // Removed (n) at the bottom; a mod just uninstalled stays dimmed in its place for 8 seconds.
                                color: entry.foc ? c.focusFill : "transparent"
                                border.color: entry.foc ? c.accent : c.line
                                border.width: entry.foc ? Math.max(1, app.px(2)) : Math.max(1, app.px(1))
                                opacity: entry.kind === "ghost" && !entry.foc ? 0.6 : 1
                                Label {
                                    x: app.px(14)
                                    anchors.verticalCenter: parent.verticalCenter
                                    width: parent.width - app.px(28)
                                    elide: Text.ElideRight
                                    text: entry.kind === "ghost" ? "Removed: " + entry.m.name + ". A puts it back."
                                          : "Removed (" + app.removedMods.length + "), kept 7 days"
                                    color: entry.kind === "ghost" ? c.soft : c.muted
                                    font.pixelSize: app.px(15)
                                    font.italic: entry.kind === "ghost"
                                }
                                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                TapHandler { onTapped: { app.modIndex = entry.index; app.act("a") } }
                            }
                        }
                    }

                    // Get mods: a game with no source can still add a file.
                    EmptyState {
                        ui: kit
                        anchors.fill: parent
                        visible: app.page === "game" && app.game !== null && app.tab === 1 && app.sources.length === 0
                        title: "Get mods"
                        body: "This game isn't on Nexus Mods or Thunderstore. You can still add a mod file."
                        buttonKey: "a"
                        buttonLabel: "Add a file from Downloads"
                        buttonFocused: app.zone === "main"
                        onActivated: app.openAdd()
                    }
                    GetMods {
                        id: getView
                        ui: kit
                        anchors.fill: parent
                        anchors.topMargin: app.px(4)
                        visible: app.page === "game" && app.game !== null && app.tab === 1 && app.sources.length > 0 && !app.modPage
                        // The cards with Installed and Update ready worked out from My mods (getItems).
                        result: app.browseData ? Object.assign({}, app.browseData, { items: app.getItems }) : null
                        loading: app.browseLoading
                        loadingMore: app.loadingMore
                        search: app.getText
                        category: app.getCategory
                        bar: app.getBar
                        recent: app.recentViewed
                        banner: app.getBanner
                        fallback: app.nexusFallback
                        fallbackTitle: "Browse " + app.gameTitle + " mods on Nexus Mods"
                        columns: app.gridColumns
                        focusArea: app.getArea
                        focusIndex: app.getIndex
                        active: app.zone === "main" && app.sheet === ""
                        onBarClicked: (i) => app.getBarRun(i)
                        onRecentClicked: (i) => { app.getArea = "recent"; app.getIndex = i; app.getAct("a") }
                        onItemClicked: (i) => { app.getArea = "grid"; app.getIndex = i; app.openDetails(app.getItems[i]) }
                        onItemMenu: (i) => { if (app.sheet === "") { app.zone = "main"; app.getArea = "grid"; app.getIndex = i; app.menuIndex = 0; app.sheet = "cardmenu" } }
                        onFallbackClicked: (k) => { app.getArea = "fallback"; app.getAct(k) }
                    }
                    ModDetails {
                        id: modView
                        ui: kit
                        anchors.fill: parent
                        anchors.topMargin: app.px(4)
                        visible: app.page === "game" && app.modPage
                        info: app.detailsData
                        fallbackPicture: app.detailsItem && app.detailsItem.icon ? app.detailsItem.icon : ""
                        focused: app.zone === "main" && app.sheet === ""
                        mainLabel: app.mainLabel
                        mainLine: app.mainLine
                        mainProgress: app.busyHere && app.dl.active ? Math.max(0, app.dlPart) : -1
                        focusIndex: app.pageInline >= 0 ? -1 : app.pageIndex
                        helpLabels: app.pageHelpLabels
                        helpFocus: app.pageInline
                        onHelpClicked: (i) => { app.pageInline = i; app.modPageAct("a") }
                        onInstallClicked: { app.pageIndex = 0; app.pageInline = -1; app.installFromDetails() }
                        onVersionsClicked: { app.pageIndex = 1; app.openVersions() }
                        onNeedClicked: (i) => { app.pageIndex = i + 2; app.openNeed(i) }
                    }
                    // Load order tab (Skyrim)
                    LoadOrderList {
                        ui: kit
                        anchors.fill: parent
                        anchors.topMargin: app.px(4)
                        visible: app.page === "game" && app.game !== null && app.tab === 2 && !app.modPage
                        plugins: app.order
                        error: app.orderError
                        messages: app.orderMessages
                        note: app.orderNote
                        owners: { let o = {}; for (const k in app.orderOwners) o[k] = app.modName(app.orderOwners[k]); return o }
                        focusIndex: app.orderIndex
                        moving: app.orderMoving
                        active: app.zone === "main" && app.sheet === ""
                        onSortClicked: { app.orderIndex = -1; Turbine.orderSort() }
                        onRowClicked: (i) => app.orderIndex = i
                        onRowMenu: (i) => { if (app.sheet === "") { app.orderIndex = i; app.openPluginMenu() } }
                        onToggleClicked: (i) => { app.orderIndex = i; app.orderAct("a") }
                    }

                    // Settings
                    Row {
                        visible: app.page === "settings"
                        anchors.fill: parent
                        spacing: app.px(14)
                        Column {
                            width: app.px(170)
                            spacing: app.px(4)
                            // LB and RB move between the sections (caps at both ends of the list).
                            Key { k: "lb"; anchors.horizontalCenter: parent.horizontalCenter }
                            Repeater {
                                model: app.settingsSections
                                Rectangle {
                                    required property int index
                                    required property string modelData
                                    width: parent.width; height: app.px(40); radius: app.px(6)
                                    color: app.setSection === index ? c.raised : "transparent"
                                    Rectangle { visible: app.setSection === parent.index; width: app.px(3); height: parent.height; color: c.accent; radius: app.px(2) }
                                    Label { x: app.px(14); anchors.verticalCenter: parent.verticalCenter; text: parent.modelData; font.pixelSize: app.px(16); font.bold: app.setSection === parent.index; color: app.setSection === parent.index ? c.text : c.muted }
                                    HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                    TapHandler { onTapped: { app.setSection = parent.index; app.setRow = 0 } }
                                }
                            }
                            Key { k: "rb"; anchors.horizontalCenter: parent.horizontalCenter }
                        }
                        // Look
                        ListView {
                            id: lookList
                            width: parent.width - app.px(184)
                            height: parent.height
                            visible: app.sec === "Look"
                            clip: true
                            spacing: app.px(4)
                            model: app.lookRows
                            currentIndex: app.sec === "Look" ? app.setRow : 0
                            highlightFollowsCurrentItem: false
                            onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                            delegate: Rectangle {
                                id: srow
                                required property int index
                                required property var modelData
                                readonly property bool foc: app.zone === "settings" && app.setRow === index
                                readonly property var now: srow.modelData.now !== undefined ? srow.modelData.now : look[srow.modelData.key]
                                width: lookList.width; height: app.px(50); radius: app.px(6)
                                color: foc ? c.focusFill : c.card
                                border.color: c.accent
                                border.width: foc ? app.px(2) : 0
                                opacity: srow.modelData.dim ? 0.6 : 1
                                Label { x: app.px(14); width: app.px(140); anchors.verticalCenter: parent.verticalCenter; text: srow.modelData.label; font.pixelSize: app.px(16); font.bold: true }
                                Item {
                                    x: app.px(160); width: parent.width - app.px(174); height: parent.height
                                    // Choices
                                    Row {
                                        visible: !srow.modelData.swatch && !srow.modelData.slider
                                        anchors.verticalCenter: parent.verticalCenter
                                        width: parent.width
                                        spacing: app.px(4)
                                        Repeater {
                                            model: srow.modelData.names || []
                                            Rectangle {
                                                required property int index
                                                required property string modelData
                                                readonly property bool on: srow.now === srow.modelData.options[index]
                                                width: (parent.width - app.px(4) * ((srow.modelData.names || []).length - 1)) / (srow.modelData.names || [1]).length
                                                height: app.px(32); radius: app.px(5)
                                                color: on ? c.raised : c.ground
                                                Rectangle { visible: parent.on; anchors.bottom: parent.bottom; width: parent.width; height: app.px(2); color: c.accent }
                                                Label { anchors.centerIn: parent; text: parent.modelData; font.pixelSize: app.px(14); font.bold: parent.on; color: parent.on ? c.text : c.muted }
                                                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                                TapHandler {
                                                    onTapped: {
                                                        app.setRow = srow.index
                                                        const v = srow.modelData.options[parent.index]
                                                        if (srow.modelData.key === "layout") app.layoutPreview(v)
                                                        else if (srow.modelData.key === "season") { look.season = v; app.note("action", "Look: season " + v) }
                                                        else { look[srow.modelData.key] = v; app.note("action", "Look: " + srow.modelData.key + " " + v) }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    // Accent swatches
                                    Row {
                                        visible: srow.modelData.swatch === true
                                        anchors.verticalCenter: parent.verticalCenter
                                        spacing: app.px(10)
                                        Repeater {
                                            model: ["brass", "iron", "copper", "jade", "fjord", "heather", "ember"]
                                            Rectangle {
                                                required property string modelData
                                                width: app.px(24); height: width; radius: width / 2
                                                anchors.verticalCenter: parent.verticalCenter
                                                color: c.swatches[modelData]
                                                border.color: c.text
                                                border.width: look.accent === modelData ? app.px(2) : 0
                                                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                                TapHandler { onTapped: { look.accent = parent.modelData; app.note("action", "Look: accent " + look.accent) } }
                                            }
                                        }
                                        Label { text: look.accent.charAt(0).toUpperCase() + look.accent.substring(1); color: c.soft; font.pixelSize: app.px(14); anchors.verticalCenter: parent.verticalCenter }
                                    }
                                    // Sliders: Text size, Seasonal amount and size
                                    Row {
                                        visible: srow.modelData.slider === true
                                        anchors.verticalCenter: parent.verticalCenter
                                        width: parent.width
                                        spacing: app.px(12)
                                        Rectangle {
                                            id: track
                                            readonly property real part: srow.modelData.slider ? (srow.now - srow.modelData.min) / (srow.modelData.max - srow.modelData.min) : 0
                                            width: parent.width - app.px(srow.modelData.words ? 200 : 70); height: app.px(6); radius: height / 2
                                            anchors.verticalCenter: parent.verticalCenter
                                            color: c.line
                                            Rectangle { width: parent.width * track.part; height: parent.height; radius: parent.radius; color: c.accent }
                                            Rectangle {
                                                x: parent.width * track.part - width / 2
                                                anchors.verticalCenter: parent.verticalCenter
                                                width: app.px(18); height: width; radius: width / 2; color: c.accentHi
                                            }
                                        }
                                        Label { text: srow.now + (srow.modelData.unit || "") + (srow.modelData.words ? " · " + srow.modelData.words : ""); color: c.accentText; font.bold: true; anchors.verticalCenter: parent.verticalCenter }
                                    }
                                }
                                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                TapHandler { onTapped: app.setRow = srow.index }
                            }
                        }
                        // About: the app icon (ten presses turn Labs mode on or off), version, the scrolling test.
                        Column {
                            width: parent.width - app.px(184)
                            spacing: app.px(8)
                            visible: app.sec === "About"
                            Row {
                                spacing: app.px(14)
                                Rectangle {
                                    id: aboutIcon
                                    readonly property bool foc: app.zone === "settings" && app.setRow === 0
                                    width: app.px(76); height: width; radius: app.px(10)
                                    color: foc ? c.focusFill : "transparent"
                                    border.color: c.accent
                                    border.width: foc ? app.px(2) : 0
                                    Image {
                                        anchors.centerIn: parent
                                        width: app.px(64); height: width
                                        source: app.iconFull
                                        sourceSize.width: app.px(128)
                                        smooth: true
                                        mipmap: true
                                    }
                                    HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                    TapHandler { onTapped: { app.setRow = 0; app.labsTap() } }
                                }
                                Column {
                                    anchors.verticalCenter: parent.verticalCenter
                                    spacing: app.px(4)
                                    Label { text: "Turbine Mod Manager " + Turbine.version; font.pixelSize: app.px(19); font.weight: Font.DemiBold }
                                    Label { visible: app.labsOn; text: "Labs mode on"; color: c.accentText; font.pixelSize: app.px(15); font.bold: true }
                                }
                            }
                            Label { visible: !Turbine.pads; width: parent.width; wrapMode: Text.WordWrap; elide: Text.ElideNone; color: c.muted; font.pixelSize: app.px(14); text: "Using Steam's controller settings (no controller seen directly)." }
                            Label { width: parent.width; wrapMode: Text.WordWrap; elide: Text.ElideNone; color: c.soft; text: "A mod manager for SteamOS handhelds. Free software under the GNU General Public License, version 3 or later. Licenses of the parts it uses are in the licenses folder of the project." }
                            Rectangle {
                                readonly property bool foc: app.zone === "settings" && app.setRow === 1
                                width: perfRow.implicitWidth + app.px(28); height: app.px(40); radius: app.px(8)
                                color: foc ? c.focusFill : c.card
                                border.color: c.accent
                                border.width: foc ? app.px(2) : 0
                                Hint { id: perfRow; anchors.centerIn: parent; k: "a"; label: "Scrolling test (2,000 rows)" }
                                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                TapHandler { onTapped: { app.page = "perf"; app.zone = "perf" } }
                            }
                        }
                        // Labs: Report a bug, Performance check, and the checklist of things to confirm on the device.
                        ListView {
                            id: labsList
                            width: parent.width - app.px(184)
                            height: parent.height
                            visible: app.sec === "Getting mods" || app.sec === "Updates" || app.sec === "Labs" || app.sec === "Help"
                            clip: true
                            spacing: app.px(4)
                            model: app.rowsNow
                            currentIndex: visible ? app.setRow : 0
                            highlightFollowsCurrentItem: false
                            onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                            onVisibleChanged: if (visible) Qt.callLater(() => { positionViewAtBeginning(); positionViewAtIndex(currentIndex, ListView.Contain) })
                            delegate: Item {
                                id: lrow
                                required property int index
                                required property var modelData
                                readonly property bool foc: app.zone === "settings" && app.setRow === index
                                readonly property bool isItem: modelData.kind === "item"
                                readonly property bool hasChips: isItem || modelData.kind === "choice" || modelData.kind === "toggle"
                                readonly property string answer: isItem ? app.labsState(modelData.id) : ""
                                width: labsList.width
                                height: (modelData.head ? labsHead.height : 0) + box.height
                                Label {
                                    id: labsHead
                                    visible: !!lrow.modelData.head
                                    height: visible ? app.px(34) : 0
                                    x: app.px(6)
                                    verticalAlignment: Text.AlignBottom
                                    bottomPadding: app.px(6)
                                    text: (lrow.modelData.head || "") + (lrow.modelData.head && lrow.isItem ? " · " + app.labsDone : "")
                                    color: c.muted
                                    font.pixelSize: app.px(14)
                                    font.bold: true
                                    font.letterSpacing: app.px(1)
                                }
                                Rectangle {
                                    id: box
                                    y: labsHead.height
                                    width: parent.width
                                    height: Math.max(lrow.isItem ? app.px(66) : app.px(50), labsWords.implicitHeight + app.px(18))
                                    radius: app.px(6)
                                    color: lrow.foc ? c.focusFill : c.card
                                    border.color: c.accent
                                    border.width: lrow.foc ? app.px(2) : 0
                                    Column {
                                        id: labsWords
                                        x: app.px(14)
                                        anchors.verticalCenter: parent.verticalCenter
                                        width: parent.width - app.px(28) - (lrow.hasChips ? chips.width + app.px(10) : actionHint.width + app.px(10))
                                        spacing: app.px(2)
                                        Label { width: parent.width; text: lrow.modelData.title; font.pixelSize: app.px(16); font.bold: true }
                                        Label { width: parent.width; text: lrow.modelData.how; color: c.soft; font.pixelSize: app.px(14); wrapMode: Text.WordWrap; maximumLineCount: 3; elide: Text.ElideRight }
                                    }
                                    Hint {
                                        id: actionHint
                                        visible: lrow.modelData.kind === "action"
                                        anchors.right: parent.right; anchors.rightMargin: app.px(12)
                                        anchors.verticalCenter: parent.verticalCenter
                                        k: lrow.foc ? "a" : ""
                                        label: lrow.modelData.action || ""
                                    }
                                    Row {
                                        id: chips
                                        visible: lrow.hasChips
                                        anchors.right: parent.right; anchors.rightMargin: app.px(10)
                                        anchors.verticalCenter: parent.verticalCenter
                                        spacing: app.px(4)
                                        Repeater {
                                            model: lrow.isItem ? [["works", "Works"], ["problem", "Problem"], ["", "Not tested"]]
                                                   : lrow.hasChips ? lrow.modelData.options.map((o, i) => [o, lrow.modelData.names[i]]) : []
                                            Rectangle {
                                                required property int index
                                                required property var modelData
                                                readonly property bool on: lrow.isItem ? lrow.answer === modelData[0] : lrow.modelData.value === modelData[0]
                                                readonly property bool cursor: lrow.isItem && lrow.foc && app.labsChip === index
                                                width: chipText.implicitWidth + app.px(18); height: app.px(32); radius: app.px(5)
                                                color: on ? c.raised : c.ground
                                                border.color: cursor ? c.text : on ? c.accent : c.line
                                                border.width: cursor ? app.px(2) : Math.max(1, app.px(1))
                                                Label {
                                                    id: chipText
                                                    anchors.centerIn: parent
                                                    text: parent.modelData[1]
                                                    font.pixelSize: app.px(14)
                                                    font.bold: parent.on
                                                    color: !parent.on ? c.muted : !lrow.isItem ? c.text : parent.index === 0 ? c.ok : parent.index === 1 ? c.stop : c.text
                                                }
                                                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                                TapHandler {
                                                    onTapped: {
                                                        app.setRow = lrow.index
                                                        if (lrow.isItem) { app.labsChip = parent.index; app.labsChoose(lrow.modelData, parent.modelData[0]) }
                                                        else if (!parent.on) app.rowStep(lrow.modelData, 1)
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                    TapHandler { onTapped: { app.setRow = lrow.index; if (lrow.modelData.kind === "action") app.rowRun(lrow.modelData) } }
                                }
                            }
                        }
                    }

                    // Scrolling test: 2,000 mod rows.
                    ListView {
                        id: perfList
                        visible: app.page === "perf"
                        anchors.fill: parent
                        clip: true
                        spacing: app.px(4)
                        model: 2000
                        currentIndex: app.perfIndex
                        highlightMoveDuration: 80
                        onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                        delegate: Rectangle {
                            id: prow
                            required property int index
                            readonly property bool foc: app.perfIndex === index
                            readonly property bool off: index % 7 === 3
                            width: perfList.width
                            height: app.px(52)
                            radius: app.px(6)
                            color: foc ? c.focusFill : c.card
                            border.color: c.accent
                            border.width: foc ? app.px(2) : 0
                            Row {
                                anchors.verticalCenter: parent.verticalCenter
                                x: app.px(12)
                                spacing: app.px(12)
                                Rectangle {
                                    width: app.px(38); height: app.px(22); radius: app.px(5)
                                    anchors.verticalCenter: parent.verticalCenter
                                    color: prow.off ? c.ground : c.raised
                                    Label { anchors.centerIn: parent; text: prow.off ? "OFF" : "ON"; font.pixelSize: app.px(14); font.bold: true; color: prow.off ? c.muted : c.accentText }
                                }
                                Column {
                                    anchors.verticalCenter: parent.verticalCenter
                                    Label { text: "Test mod " + (prow.index + 1); font.pixelSize: app.px(16); font.bold: true; color: prow.off ? c.muted : c.text }
                                    Label { text: "Row " + (prow.index + 1) + " of 2,000"; font.pixelSize: app.px(14); color: c.muted }
                                }
                            }
                        }
                    }
                }
            }

        }
    }

    // ---------------------------------------------------------------- sheets
    Rectangle {
        anchors.fill: parent
        visible: app.sheet === "details" || app.sheet === "notice"
        color: "#B0000000"
        TapHandler { onTapped: app.act("b") }

        // Game details: what Turbine found, so the owner can check detection on the device.
        Rectangle {
            visible: app.sheet === "details" && app.game !== null
            anchors.centerIn: parent
            width: Math.min(parent.width - app.px(40), app.px(600))
            height: detailCol.implicitHeight + app.px(32)
            radius: app.px(10)
            color: c.panel
            border.color: c.line
            Column {
                id: detailCol
                x: app.px(18); y: app.px(16)
                width: parent.width - app.px(36)
                spacing: app.px(8)
                Label { text: app.game ? app.game.name : ""; font.pixelSize: app.px(19); font.weight: Font.DemiBold; width: parent.width }
                Repeater {
                    model: app.game ? [
                        ["Store", app.game.storeLabel + (app.game.launcher !== "Steam" ? " (through Heroic)" : "")],
                        ["Store ID", app.game.storeId],
                        ["Runs as", app.runtimeWords(app.game)],
                        ["Stored on", app.game.drive],
                        ["Folder", app.game.installDir],
                        ["Version", app.game.build || "Not reported"],
                        ["Cover art", app.game.cover || app.game.header ? "Found on the device" : "None cached"]
                    ].concat(app.checkPaths.length > 0 ? [
                        ["Same scripts", app.checkPaths.join("; ")]
                    ] : []).concat(app.rules && app.rules.launchOption ? [
                        ["Loader setup", app.rules.launchHelp || ""],
                        ["Launch Option", app.rules.launchOption]
                    ] : []) : []
                    Row {
                        required property var modelData
                        width: detailCol.width
                        spacing: app.px(12)
                        Label { width: app.px(110); text: parent.modelData[0]; color: c.muted; font.pixelSize: app.px(14) }
                        Label { width: parent.width - app.px(122); text: parent.modelData[1]; wrapMode: Text.WrapAnywhere; elide: Text.ElideNone; font.pixelSize: app.px(15) }
                    }
                }
                Hint { k: "b"; label: "Close" }
            }
        }

        // Not SteamOS: said once, then Turbine runs as it is.
        Rectangle {
            visible: app.sheet === "notice"
            anchors.centerIn: parent
            width: Math.min(parent.width - app.px(40), app.px(520))
            height: noticeCol.implicitHeight + app.px(36)
            radius: app.px(10)
            color: c.panel
            border.color: c.line
            Column {
                id: noticeCol
                x: app.px(20); y: app.px(18)
                width: parent.width - app.px(40)
                spacing: app.px(10)
                Label { text: "Made for SteamOS"; font.pixelSize: app.px(19); font.weight: Font.DemiBold }
                Label { width: parent.width; wrapMode: Text.WordWrap; elide: Text.ElideNone; color: c.soft; text: "This system isn't supported. Turbine Mod Manager is built and tested for SteamOS handhelds only, so some things may not work here." }
                Hint { k: "a"; label: "OK" }
            }
        }
    }

    // ---------------------------------------------------------------- menus and prompts (they cover the window)
    ModMenu {
        ui: kit
        visible: app.sheet === "modmenu"
        title: app.menuMod ? app.menuMod.name : ""
        subtitle: !app.menuMod ? ""
                  : [app.modWords(app.menuMod), (app.menuMod.enabled ? "On" : "Off") + (app.ordered ? ", " + app.ordinal(app.mods.indexOf(app.menuMod) + 1) + " in load order" : "")].filter(s => s !== "").join(" · ")
        items: app.modMenuItems.map(m => ({ label: m.title, detail: m.detail, icon: m.icon, danger: m.danger === true }))
        focusIndex: app.menuIndex
        onActivated: (index) => { app.menuIndex = index; app.modMenuRun(index) }
        onClosed: app.act("b")
    }
    GameMenu {
        ui: kit
        visible: app.sheet === "gamemenu"
        gameName: app.gameTitle
        restoreDate: app.lastGood ? "From " + app.when(app.lastGood.savedAt) + " · " + app.plural(app.lastGood.modsOn, "mod", "mods") + " on" : ""
        restoreEmptyText: "Nothing saved yet. A setup is saved each time you play with mods."
        extraItems: app.gameMenuItems.slice(2).map(m => ({ key: m.act, label: m.title, detail: m.detail, icon: m.icon, danger: m.danger === true }))
        focusIndex: app.menuIndex
        onActivated: (index, key) => { app.menuIndex = index; app.gameMenuRun(index) }
        onClosed: app.act("b")
    }
    AddMods {
        ui: kit
        visible: app.sheet === "add"
        subtitle: app.addAll ? "Every mod file in Downloads and its Game Mods folders, newest first"
                             : "Mod files in Downloads and Game Mods/" + app.gameTitle + ", newest first"
        files: app.addList.map(d => ({ name: d.name, size: app.sizeWords(d.size), age: (d.modified > 0 ? app.age(d.modified) : "") + (app.addAll && d.folder ? " · " + d.folder : "") }))
        focusIndex: app.addIndex
        emptyText: app.addAll || app.downloads.length === 0 ? "No mod files in Downloads yet. Get one with Get mods, or download a mod's zip file in Desktop Mode." : "None for " + app.gameTitle + ". Y shows every mod file in Downloads."
        laterText: ""
        onActivated: (index) => { app.addIndex = index; app.addChosen() }
        onClosed: app.act("b")
    }
    PrePlayCheck {
        id: checkView
        ui: kit
        visible: app.sheet === "check"
        checks: app.checks
        buttons: app.checkChoices.map(label => ({ key: "", label: label, main: !app.checkStops && label === "Play anyway" }))
        focusIndex: app.checkIndex
        note: app.checkStops ? "Play without mods starts the game with no mods. Nothing is deleted." : ""
        onActivated: (index, key) => { app.checkIndex = index; app.checkChoose(index) }
        onClosed: app.act("b")
    }
    LoaderPrompt {
        ui: kit
        visible: app.sheet === "loader"
        title: (app.loaderShort || "Mod loader") + " needed"
        loaderName: app.loaderShort
        loaderFullName: app.rules && app.rules.loader && app.rules.loader.name.indexOf(" (") > 0 ? app.rules.loader.name.split(" (")[1].replace(")", "") : ""
        explain: app.rules && app.rules.loader ? app.rules.loader.explain : ""
        neededBy: app.loaderFor
        sourceUrl: app.rules && app.rules.loader ? app.rules.loader.source : ""
        sourceShown: app.loaderSourceShown
        canDownload: app.loaderCanGet
        buttons: app.loaderLabels.map(l => ({ key: "", label: l }))
        focusIndex: app.loaderIndex
        onActivated: (index, key) => { app.loaderIndex = index; app.act(key || "a") }
        onClosed: app.act("b")
    }

    // Installer choices: over the main area, the rail stays in view.
    FomodSheet {
        id: fomodView
        ui: kit
        visible: app.sheet === "fomod" && app.fomod !== null
        x: rail.width
        width: parent.width - rail.width
        height: parent.height - legend.height - app.px(16)
        info: app.fomod
        onChoose: (g, o) => { app.note("action", "Installer: question " + (g + 1) + ", option " + (o + 1)); Turbine.fomodChoose(g, o) }
        onNext: { app.note("action", "Installer: next step"); Turbine.fomodNext() }
        onBack: { app.note("action", "Installer: back a step"); Turbine.fomodBack() }
        onStop: { app.note("action", "Installer: stopped"); Turbine.fomodCancel() }
    }
    VersionsSheet {
        ui: kit
        visible: app.sheet === "versions"
        modName: app.detailsData && app.detailsData.name ? app.detailsData.name : ""
        versions: app.otherVersions
        focusIndex: app.versionIndex
        note: app.detailsData && app.detailsData.versionsNote ? app.detailsData.versionsNote : ""
        onActivated: (i) => { app.versionIndex = i; app.installVersion(i) }
        onClosed: app.act("b")
    }
    PagesSheet {
        ui: kit
        visible: app.sheet === "pages"
        title: app.pagesTitle
        pages: app.pagesList
        focusIndex: app.pagesIndex
        gameMode: Turbine.gamemode
        handlerSet: app.handler.state !== "no"
        onActivated: (i) => { app.pagesIndex = i; app.openPage(i) }
        onClosed: app.act("b")
    }
    CodeSheet {
        ui: kit
        visible: app.sheet === "code"
        info: app.codeData
        onCopyClicked: app.act("a")
        onClosed: app.act("b")
    }
    PluginSheet {
        ui: kit
        visible: app.sheet === "plugin" && app.orderPlugin !== null
        plugin: app.orderPlugin
        position: app.orderIndex + 1
        notes: app.orderPlugin ? (app.orderMessages[app.orderPlugin.name.toLowerCase()] || []) : []
        onClosed: app.act("b")
    }
    ProfilesSheet {
        id: profilesView
        ui: kit
        visible: app.sheet === "profiles"
        gameName: app.gameTitle
        profiles: app.profileList
        active: app.profileData ? app.profileData.active : ""
        actions: app.profileActions
        plan: app.importData
        premium: app.account.premium === true
        focusIndex: app.profIndex
        onActivated: (i) => { app.profIndex = i; app.profileRun(i) }
        onClosed: app.act("b")
    }
    AddMods {
        ui: kit
        visible: app.sheet === "shares"
        title: "Import a share file"
        subtitle: "Shared loadout files in your Downloads folder, newest first"
        files: app.shares.map(x => ({ name: x.name, size: "", age: "" }))
        focusIndex: app.shareIndex
        actionLabel: "Read"
        emptyText: "No shared loadout files in Downloads. Someone makes one with Loadouts, Export to a share file."
        laterText: ""
        onActivated: (index) => { app.shareIndex = index; app.act("a") }
        onClosed: app.act("b")
    }
    // The no-browser warning, the browser helper's steps, install notes, "Add this file?" and Add Chrome to Steam.
    InfoSheet {
        id: infoView
        ui: kit
        visible: ["nobrowser", "helper", "notes", "found", "quick", "chain", "addbrowser", "oldhelper"].indexOf(app.sheet) >= 0
        title: app.sheet === "quick" && app.quick ? app.quick.name + " closed after " + app.plural(app.quick.seconds, "second", "seconds")
               : app.sheet === "chain" ? "Next: " + (app.chainQueue[0] ? app.chainQueue[0].name : "")
               : app.sheet === "nobrowser" ? (app.browser.title || "No web browser found")
               : app.sheet === "helper" ? "Set up the browser helper"
               : app.sheet === "addbrowser" && app.addMode === "helper" ? "Set up the browser helper too"
               : app.sheet === "addbrowser" ? "Add Chrome to Steam so it opens in Game Mode"
               : app.sheet === "found" ? "Found a download"
               : app.sheet === "oldhelper" ? "Your browser helper is out of date"
               : "Added " + app.notesName
        text: app.sheet === "quick" ? app.quickText
              : app.sheet === "chain" ? (app.chainAfter ? app.chainAfter + " is in. " : "") + "It also needs " + (app.chainQueue[0] ? app.chainQueue[0].name : "") + ". A opens its files on Nexus: pick the file for your " + app.gameTitle + " version and press Slow download. Turbine adds it too."
              : app.sheet === "nobrowser" ? (app.browser.text || "")
              : app.sheet === "addbrowser" && app.addMode === "helper" ? "Turbine started " + app.openName + " through Steam. Without the browser helper it opens on its start page, not on the mod. The helper opens the mod's page for you and saves the download in the right game's folder."
              : app.sheet === "addbrowser" ? "Game Mode only shows apps Steam starts. Add your browser to Steam once, and Turbine opens Nexus in it right over Turbine. Once:"
              : app.sheet === "helper" ? "Optional. Skips the Save box and puts each download in the right game's folder. Turbine put it in Downloads, in the folder Turbine browser helper. Then, once:"
              : app.sheet === "found" && app.found ? "Found " + app.found.name + " (" + app.sizeWords(app.found.size) + ")."
                    + (app.found.game ? " Add it to " + app.found.gameName + "?" : " Which game is it for?")
              : app.sheet === "notes" ? "Something to know about this mod:"
              : app.sheet === "oldhelper" ? "In Chrome, open chrome://extensions and press the reload arrow on Turbine browser helper. Turbine already put the new version in its folder. After this once, the helper updates by itself."
              : ""
        steps: app.sheet === "nobrowser" ? (app.browser.steps || [])
               : app.sheet === "helper" ? ["In Desktop Mode, open chrome://extensions in the browser.",
                                           "Turn on Developer mode (top right).",
                                           "Press Load unpacked.",
                                           "Choose Downloads, then the folder Turbine browser helper."]
               : app.sheet === "addbrowser" && app.addMode !== "helper" ? ["Switch to Desktop Mode: press the Steam button, open Power, and pick Switch to Desktop.",
                                           "In Desktop Mode, open Steam, then Games, then Add a Non-Steam Game to My Library.",
                                           "Tick Google Chrome and press Add Selected Programs.",
                                           "Switch back to Game Mode, open Turbine and press Recheck."]
               : []
        notes: app.sheet === "notes" ? app.notesList : []
        footnote: app.sheet === "helper" ? "Then press Pair: the pairing page opens in the browser and links the helper to Turbine."
                  : app.sheet === "addbrowser" && app.addMode !== "helper" ? (app.bridge.paired ? "Until then, Steam's browser opens Nexus instead." : "Also set up the browser helper (Settings, Getting mods): without it the browser opens, but not on the mod's page. Until then, Steam's browser opens Nexus instead.")
                  : app.sheet === "addbrowser" ? "Steam's browser opens the mod's page without the helper."
                  : app.sheet === "oldhelper" ? "Until then, Nexus pages open in Steam's browser." + (app.bridge.helper ? " Helper " + app.bridge.helper + ", Turbine's " + (app.bridge.bundled || "") + "." : "")
                  : app.sheet === "found" && app.found ? (app.found.game ? "Why " + app.found.gameName + ": " + app.found.why + ". " : "")
                    + "The file stays in Downloads either way." + (app.found.more > 0 ? " " + app.plural(app.found.more, "more file waits", "more files wait") + " after this one." : "")
                  : ""
        buttons: app.infoButtons
        focusIndex: app.infoIndex
        onActivated: (i) => { app.infoIndex = i; app.infoRun(i) }
        onClosed: app.act("b")
    }
    // Removed mods (A Put back, X Delete now) and the hold-to-confirm sheet for deleting for good.
    RemovedSheet {
        ui: kit
        visible: app.sheet === "removed"
        mods: app.removedMods
        focusIndex: app.removedIndex
        now: app.today
        onActivated: (i) => { app.removedIndex = i; app.removedRun(i) }
        onDeleteClicked: (i) => { app.removedIndex = i; app.removedDelete(i) }
        onClosed: app.act("b")
    }
    InfoSheet {
        ui: kit
        visible: app.sheet === "confirm"
        title: "Delete for good?"
        text: app.confirmText
        buttons: [{ key: "", label: confirmHold.running ? "Keep holding A: " + Math.max(1, Math.ceil(app.holdNeeded - app.confirmHeld)) + "..." : "Hold A to delete" },
                  { key: "", label: "Keep" }]
        focusIndex: app.confirmIndex
        onActivated: (i) => { app.confirmIndex = i; if (i === 0) app.startConfirm(); else app.act("b") }
        onClosed: app.act("b")
    }
    // X on a Get mods card.
    ModMenu {
        ui: kit
        visible: app.sheet === "cardmenu"
        title: app.cardItem ? app.cardItem.name : ""
        subtitle: app.cardItem ? (app.cardItem.owner || "") : ""
        items: app.cardMenuItems.map(m => ({ label: m.title, detail: m.detail, icon: m.icon, danger: false }))
        focusIndex: app.menuIndex
        onActivated: (index) => { app.menuIndex = index; app.cardMenuRun(index) }
        onClosed: app.act("b")
    }
    // A small menu made on the spot: Play options (hold Start), a profile's Rename and Delete, Select several's actions.
    ModMenu {
        ui: kit
        visible: app.sheet === "pick"
        title: app.pickTitle
        subtitle: app.pickSub
        items: app.pickItems.map(m => ({ label: m.title, detail: m.detail, icon: m.icon, danger: m.danger === true }))
        focusIndex: app.menuIndex
        onActivated: (index) => { app.menuIndex = index; app.pickRun(index) }
        onClosed: app.act("b")
    }
    // The focused game's name beside the rail.
    Rectangle {
        id: railLabel
        readonly property bool on: app.zone === "rail" && app.railIndex < app.games.length
        visible: on && app.sheet === ""
        z: 20
        x: rail.width + app.px(4)
        y: railList.y + app.railIndex * (app.px(66) + railList.spacing) - railList.contentY + (app.px(66) - height) / 2
        width: railLabelText.implicitWidth + app.px(24)
        height: app.px(32)
        radius: app.px(6)
        color: c.raised
        border.color: c.accent
        border.width: Math.max(1, app.px(1))
        Text { id: railLabelText; anchors.centerIn: parent; text: app.railName(app.railIndex); color: c.text; font.family: app.bodyFont; font.pixelSize: app.px(15); font.weight: Font.DemiBold }
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
    // X on "Found <file>": which game it goes into.
    GamePicker {
        ui: kit
        visible: app.sheet === "pickgame"
        fileName: app.found ? app.found.name : ""
        games: app.games
        focusIndex: app.pickIndex
        onActivated: (i) => { app.pickIndex = i; app.pickGame(i) }
        onClosed: app.act("b")
    }
    AskText {
        id: askView
        ui: kit
        focusIndex: app.askIndex
        onAccepted: (text) => app.askDone(text)
        onEdited: (text) => { if (app.askPurpose === "find") { app.findText = text; app.modIndex = 0 } }
        onCancelled: app.askCancelled()
    }

    ActivityLog {
        id: activityView
        ui: kit
        visible: app.sheet === "activity"
        lines: app.activityLines
        onCopyClicked: {
            Turbine.copyText(app.activityLines.slice(-500).join("\n") + "\n")
            toast.showPlain("Copied the last " + Math.min(500, app.activityLines.length) + " lines of the activity log.")
        }
        onSaveClicked: {
            const r = app.parse(Turbine.saveActivityLog(), { error: "Couldn't save it." })
            toast.showPlain(r.error ? r.error : "Saved to Downloads as " + r.file + ".")
        }
        onCloseClicked: app.sheet = ""
    }
    PerfCheck {
        id: perfView
        ui: kit
        win: app
        onCopyClicked: (text) => { Turbine.copyText(text); toast.showPlain("Performance check copied.") }
        onReportClicked: (text) => app.openReport(null, "Performance check", text)
        onCloseClicked: { perfView.close(); app.sheet = "" }
        Component.onCompleted: if (shot.page === "perfcheck") reportText = "Turbine performance check\nTurbine: 0.2 (build sample)\nVerdict: Smooth\n"
            + "Still: 60 fps, median 16.7 ms, worst 18.9 ms, spikes 0.0%, CPU 9%\nScrolling: 59 fps, median 16.7 ms, worst 24.0 ms, spikes 0.4%, CPU 31%\n"
            + "Idle: 0 frames in 4.0 s, CPU 0.2%\n\nStartup: first frame 640 ms after start\nDrawing: Vulkan, software drawing no"
    }
    LabsReport {
        id: reportView
        ui: kit
        onSendClicked: app.sendReport()
        onCloseClicked: app.dismissReport()
        onEditStarted: app.reportEditing(true)
        onEditEnded: app.reportEditing(false)
    }

    // Legend: what binds() says A, B, X and Y do here, then one more (the pointer's buttons, or Start when a sheet
    // covers Play). At most five; LB/RB, View and Start are caps on the tab bar, the wheel button and Play. Hidden
    // after a touch until the next press. Declared after the menus so their backdrop never dims it.
    readonly property var legendHints: legendFor(binds())
    Rectangle {
        id: legend
        anchors.bottom: parent.bottom
        anchors.bottomMargin: app.px(8)
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: rail.width + app.px(14)
        anchors.rightMargin: app.px(14)
        height: app.px(36)
        radius: app.px(6)
        color: c.panel
        z: 40
        visible: app.lastInput !== "touch" && app.sheet !== "tutorial"
        Row {
            id: legendRow
            anchors.verticalCenter: parent.verticalCenter
            x: app.px(12)
            spacing: app.px(16)
            Repeater {
                model: app.legendHints
                Hint {
                    required property var modelData
                    k: modelData[0]
                    label: modelData[1]
                    pre: modelData[2] || ""
                    k2: modelData[3] || ""
                    label2: modelData[4] || ""
                }
            }
        }
        // The quick wheel's button: View's cap (a tap opens it too).
        Rectangle {
            id: wheelBtn
            anchors.right: parent.right
            anchors.rightMargin: app.px(4)
            anchors.verticalCenter: parent.verticalCenter
            width: wheelRow.implicitWidth + app.px(16); height: app.px(30); radius: app.px(6)
            color: app.wheelOpen ? c.focusFill : c.raised
            visible: app.sheet === "" || app.wheelOpen
            Row {
                id: wheelRow
                anchors.centerIn: parent
                spacing: app.px(6)
                Key { k: "view"; anchors.verticalCenter: parent.verticalCenter }
                Label { text: "Quick wheel"; font.pixelSize: app.px(14); color: c.soft; anchors.verticalCenter: parent.verticalCenter }
            }
            HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
            TapHandler { onTapped: app.wheelPress() }
        }
    }
    QuickWheel {
        ui: kit
        visible: app.wheelOpen
        slots: app.wheelSlots
        focusIndex: app.wheelSlot
        labs: app.labsOn
        viewGlyph: app.glyph("view")
        onActivated: (i) => { app.wheelSlot = i; app.wheelFire(i) }
        onClosed: app.closeWheel()
    }
    Tutorial {
        ui: kit
        visible: app.sheet === "tutorial"
        card: app.tutCard
        count: app.tutCount
        viewGlyph: app.glyph("view")
        startGlyph: app.glyph("start")
        onNext: app.tutorialAct("a")
        onSkip: app.tutorialAct("b")
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
    function touched() {
        if (lastInput !== "touch") note("touch", "Touch: legend hidden, pointer gone")
        lastInput = "touch"
        Turbine.pointerHide()
    }

    // ---------------------------------------------------------------- toast
    // Something that just happened, in one of three looks: info, done, problem (real failures only). Never words
    // already on screen; the same words aren't shown twice in a row. After Uninstall: "Removed: name · Kept 7 days ·
    // Y Undo".
    property string undoId: ""
    readonly property bool undoShown: toast.shown && undoId !== "" && sheet === ""
    property string lastToast: ""
    property real lastToastAt: 0
    Toast {
        id: toast
        ui: kit
        function put(kind, m) {
            if (m === app.lastToast && Date.now() - app.lastToastAt < 6000 && shown) return
            app.lastToast = m; app.lastToastAt = Date.now()
            app.undoId = ""
            toast.kind = kind
            show(m)
        }
        function info(m) { put("info", m) }
        function done(m) { put("done", m) }
        function problem(m) { put("problem", m) }
        function showUndo(id, name) { app.undoId = id; toast.kind = "info"; show("Removed:", name, "Kept 7 days", "y", "Undo") }
        function showPlain(m) { put("info", m) }
        // The new-mod picture keeps its toast up (it arrives a few seconds before the picture).
        timeout: shot.page === "newmod" ? 0 : app.undoId !== "" ? 7000 : 4000
        onActionClicked: if (app.undoId !== "") app.putBack(app.undoId)
        onHidden: app.undoId = ""
    }
    // Labs: the brass ring that fills around the rail icon while View (or the icon) is held for a report.
    Shape {
        id: holdRing
        visible: app.holdShown
        readonly property real r: app.px(28)
        x: rail.width / 2 - r - app.px(3)
        y: app.px(10) + app.px(22) - r - app.px(3)
        width: (r + app.px(3)) * 2; height: width
        preferredRendererType: Shape.CurveRenderer
        ShapePath {
            strokeColor: c.line
            strokeWidth: app.px(4)
            fillColor: "transparent"
            PathAngleArc { centerX: holdRing.width / 2; centerY: holdRing.height / 2; radiusX: holdRing.r; radiusY: holdRing.r; startAngle: 0; sweepAngle: 360 }
        }
        ShapePath {
            strokeColor: c.accentHi
            strokeWidth: app.px(4)
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            PathAngleArc { centerX: holdRing.width / 2; centerY: holdRing.height / 2; radiusX: holdRing.r; radiusY: holdRing.r; startAngle: -90; sweepAngle: 360 * Math.min(1, app.holdMs / 7000) }
        }
    }

    // ---------------------------------------------------------------- the controller pointer
    // Steam's desktop layout: the right stick moves this pointer, RT clicks (held, it drags), LT right-clicks, the
    // left stick turned in a circle is a scroll wheel. Turbine sends real mouse and wheel events into this window
    // (backend.rs, qtglue.cpp), so everything that takes a tap or a mouse also takes the pointer; this draws it, over
    // everything (sheets included), and outlines the button or row under it. It hides 3 seconds after the right
    // stick stops; A and the D-pad work as always.
    property Item hoverItem: null
    property rect hoverBox: Qt.rect(0, 0, 0, 0)
    // Everything under the pointer that takes a tap; the outline goes on the innermost one (a row's ON pill
    // rather than the row). With a sheet open, the page under it doesn't count.
    property var hoverList: []
    function hover(item, on) {
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
        const live = hoverList.filter(x => !!x && x.visible && !(sheet !== "" && inside(x, mainRow)))
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
        function onPointerXChanged() { app.placeHover() }
        function onPointerYChanged() { app.placeHover() }
        function onPointerShownChanged() { app.note("pad", Turbine.pointerShown ? "Pointer showing" : "Pointer hidden") }
    }
    property int pointerTries: 0
    function pointerCheck() {
        const second = modList.itemAtIndex(1), first = modList.itemAtIndex(0)
        const it = second && second.kind === "mod" ? second : first && first.kind === "mod" ? first : null
        if (!it || !it.pill) {
            // The test mods may still be going in: look again shortly.
            if (++pointerTries < 6) pointerRetry.start(); else note("pad", "Pointer check: no mod row to click")
            return
        }
        const p = it.pill.mapToItem(null, it.pill.width / 2, it.pill.height / 2)
        Turbine.selftestPointer(p.x, p.y, true, 0)
    }
    Timer { id: pointerRetry; interval: 300; onTriggered: app.pointerCheck() }
    function pointerScrollCheck() {
        const p = perfList.mapToItem(null, perfList.width / 2, perfList.height / 2)
        Turbine.selftestPointer(p.x, p.y, false, 12)
        scrollCheckNote.start()
    }
    // The build's picture of the end of the check before Play: wheel steps over it.
    Timer { id: checkEndShot; interval: 900; onTriggered: Turbine.selftestPointer(app.width / 2, app.height * 0.45, false, 40) }
    Timer { id: scrollCheckNote; interval: 1500; onTriggered: app.note("pad", "Scroll check: the list moved " + Math.round(perfList.contentY) + " px") }
    Item {
        id: pointerLayer
        anchors.fill: parent
        z: 10000
        visible: Turbine.pointerShown
        enabled: false
        Rectangle {
            visible: app.hoverItem !== null && app.hoverBox.width > 0
            x: app.hoverBox.x - app.px(2); y: app.hoverBox.y - app.px(2)
            width: app.hoverBox.width + app.px(4); height: app.hoverBox.height + app.px(4)
            radius: app.px(7)
            color: Qt.rgba(c.accent.r, c.accent.g, c.accent.b, 0.10)
            border.width: Math.max(1, app.px(1.5))
            border.color: c.accentHi
        }
        // The arrow: brass with a dark edge, its tip on the spot that gets clicked, about 24 design pixels tall.
        Shape {
            x: Turbine.pointerX
            y: Turbine.pointerY
            width: app.px(24); height: app.px(24)
            preferredRendererType: Shape.CurveRenderer
            ShapePath {
                strokeColor: "#0B0A08"
                strokeWidth: Math.max(1.5, app.px(1.6))
                joinStyle: ShapePath.RoundJoin
                fillColor: "#D8AE5E"
                scale: Qt.size(app.px(24) / 24, app.px(24) / 24)
                PathSvg { path: "M1 1 L1 20 L6 15.5 L9.6 23 L13 21.4 L9.5 14 L16 14 Z" }
            }
        }
    }
}
