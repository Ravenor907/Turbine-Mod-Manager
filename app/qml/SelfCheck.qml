import QtQuick
import Turbine

// The build's self-checks (tools/ci-shots.sh): what each --page= picture sets up, the timers that press through it
// and take the picture, the scripted control checks and the checks of Nexus inside Turbine. Main.qml loads this only
// in screenshot runs (shot.on). Everything in the interface is reached through app (Main.qml's window) and the ids
// Main.qml lists in checkIds.
Item {
    id: sc
    visible: false
    required property var app
    readonly property var askView: app.checkIds.askView
    readonly property var barLb: app.checkIds.barLb
    readonly property var barName: app.checkIds.barName
    readonly property var barRb: app.checkIds.barRb
    readonly property var confirmHold: app.checkIds.confirmHold
    readonly property var confirmSheet: app.checkIds.confirmSheet
    readonly property var edgeBack: app.checkIds.edgeBack
    readonly property var edgeNext: app.checkIds.edgeNext
    readonly property var gameMenuView: app.checkIds.gameMenuView
    readonly property var getView: app.checkIds.getView
    readonly property var ghostTimer: app.checkIds.ghostTimer
    readonly property var hudWin: app.checkIds.hudWin
    readonly property var infoView: app.checkIds.infoView
    readonly property var inputProbe: app.checkIds.inputProbe
    readonly property var legend: app.checkIds.legend
    readonly property var loadOrderView: app.checkIds.loadOrderView
    readonly property var look: app.checkIds.look
    readonly property var lookList: app.checkIds.lookList
    readonly property var mo: app.checkIds.mo
    readonly property var modList: app.checkIds.modList
    readonly property var panelEdit: app.checkIds.panelEdit
    readonly property var panelWin: app.checkIds.panelWin
    readonly property var perfList: app.checkIds.perfList
    readonly property var pickMenu: app.checkIds.pickMenu
    readonly property var playBtn: app.checkIds.playBtn
    readonly property var rail: app.checkIds.rail
    readonly property var railList: app.checkIds.railList
    readonly property var resetHold: app.checkIds.resetHold
    readonly property var seen: app.checkIds.seen
    readonly property var settingsView: app.checkIds.settingsView
    readonly property var sheetLayer: app.checkIds.sheetLayer
    readonly property var splash: app.checkIds.splash
    readonly property var splashIcon: app.checkIds.splashIcon
    readonly property var startHint: app.checkIds.startHint
    readonly property var startHintKey: app.checkIds.startHintKey
    readonly property var startRing: app.checkIds.startRing
    readonly property var toast: app.checkIds.toast
    readonly property var shot: app.checkIds.shot

    // Pages that wait for the engine (imports, menus, the check) get more time before the picture.
    readonly property bool slow: ["pointer", "pointerscroll", "checkend", "caught", "nexusnokey", "mods",
                                  "getmods", "getmodsfallback", "modpagefree", "modpagepremium", "getmodsback", "emptygame", "emptygamemods",
                                  "removed", "newmod", "opening", "desktop", "desktopgrid", "tv", "rail", "menu", "gamemenu", "check", "add", "loader", "report", "browse", "moddetails", "fomod",
                                  "loadorder", "profiles", "accounts", "accountsgm", "updates", "lootsort", "lootnotes", "versions", "code",
                                  "nexusdetails", "nexusversions", "nexusfree", "nxm", "browser", "helper",
                                  "playoptions", "findmods", "selectmods", "overlaps", "quickexit", "chain", "listbar",
                                  "checkfocus", "menuskse", "addbrowser", "slowhint",
                                  "wheel", "ghost", "railmenu", "browserframe", "browsertest", "browserpersist", "regress",
                                  "basicgame", "place", "placestart", "overwrite", "overwriteskip", "nexuspick", "basicmenu", "tucked", "why", "findpop", "fullscroll", "acask", "acdeploy", "acguide", "acrefuse", "modsempty", "notstarted", "launchsetup", "modsettings", "coopsettings", "gameupdate", "ersaves"].indexOf(shot.page) >= 0 || shot.scripted
    function start() {
        const files = shot.arg("selftest-import")
        // "file|thunderstore|Owner-Name|1.0.3" adds a zip as if it came from Thunderstore.
        if (files !== "") for (const f of files.split(",")) {
            const p = f.split("|")
            if (p.length === 4) Turbine.selftestImport(p[0], p[1], p[2], p[3]); else Turbine.importZip(f)
        }
        if (shot.page === "gamemenu") app.sheet = "gamemenu"
        else if (shot.page === "add") { app.tab = 1; app.openAdd() }
        else if (shot.page === "about") app.openSection("About")
        else if (shot.page === "labs") {
            app.labsAnswers = { "pad-quiet": "works", "proton-bepinex": "problem" }
            app.openSection("Labs"); app.setRow = 5
        }
        else if (shot.page === "browse" || shot.page === "moddetails" || shot.page === "versions") { app.tab = 1; app.zone = "main" }
        else if (shot.page === "nexusdetails" || shot.page === "nexusversions" || shot.page === "nexusfree") {
            if (shot.page === "nexusfree") Turbine.setSampleFree()
            app.tab = 1; app.zone = "main"
        }
        else if (shot.page === "loadorder" || shot.page === "lootsort" || shot.page === "lootnotes") { app.tab = 2; app.zone = "main" }
        else if (shot.page === "accounts" || shot.page === "accountsgm") app.openSection("Getting mods")
        // The browser: Settings > Getting mods with a browser found and the helper connected; the no-browser
        // warning; the helper's setup steps; the status card in both its windows.
        else if (shot.page === "browser" || shot.page === "helper" || shot.page === "nobrowser") {
            app.browser = shot.page === "nobrowser" ? { checked: true, result: { found: false, name: "", how: "" }, title: "No web browser found",
                text: "Turbine opens Nexus Mods in your web browser, and this system doesn't have one yet. Installing one takes a minute.",
                steps: ["Switch to Desktop Mode: press the Steam button, open Power, and pick Switch to Desktop.",
                        "In Desktop Mode, open Discover, search for Google Chrome or Firefox, and press Install.",
                        "Optional, so it also opens in Game Mode: open the app menu, right-click the browser, and pick Add to Steam.",
                        "Switch back to Game Mode and open Turbine again."] }
                : { checked: true, result: { found: true, name: "Google Chrome", how: "flatpak" } }
            app.bridge = { running: true, port: 47701, paired: true, connected: shot.page === "browser" ? 1 : 0, ext: "iniglfblobfbekihldmgekchfehncbeb", error: "",
                           helper: shot.page === "browser" ? "0.63" : "", bundled: "0.64", outdated: shot.page === "browser" }
            app.openSection("Getting mods")
            if (shot.page === "helper") { app.infoIndex = 0; app.sheet = "helper" }
            else if (shot.page === "nobrowser") app.noBrowser()
        }
        // Game Mode's Add Chrome to Steam (the home has no shortcuts.vdf, so Recheck finds nothing) and the
        // Getting mods key rows.
        else if (shot.page === "addbrowser") {
            app.browser = { checked: true, result: { found: true, name: "Google Chrome", how: "flatpak" } }
            const r = app.parse(Turbine.browserShortcut(), { found: false })
            app.note("shot", "Browser in Steam: " + (r.found ? r.browser : "none"))
            app.openUrlNow = "https://www.nexusmods.com/skyrimspecialedition/mods/12604"
            app.showAddBrowser("noshortcut")
        }
        else if (shot.page === "overlay" || shot.page === "overlaygm") {
            app.status = { active: true, text: "Added to Cyberpunk 2077", level: "ok", progress: -1, game: "Cyberpunk 2077",
                           added: "Cyber Engine Tweaks 1.32.0", done: true, ask: "" }
            app.showOverlay(true, shot.page === "overlaygm")
        }
        else if (shot.page === "updates") { app.openSection("Updates"); Turbine.selfCheck(false, false) }
        // "Found <file>. Add it to <game>?" for a download Turbine wasn't sure about.
        else if (shot.page === "caught") {
            app.found = { path: "/home/deck/Downloads/Appearance Menu Mod-790-2-6-1.zip", name: "Appearance Menu Mod-790-2-6-1.zip", size: 18874368,
                          game: app.game ? app.game.id : "", gameName: app.game ? app.game.name : "", why: "it is the game last open in Turbine", more: 1 }
            app.status = { active: true, text: "Found " + app.found.name + ". Add it to " + app.found.gameName + "?", level: "warn", progress: -1,
                           game: app.found.gameName, added: "", done: false, ask: app.found.path }
            app.infoIndex = 0; app.sheet = "found"
        }
        else if (shot.page === "caughtgm") {
            app.status = { active: true, text: "Found Appearance Menu Mod-790-2-6-1.zip. Add it to Cyberpunk 2077?", level: "warn", progress: -1,
                           game: "Cyberpunk 2077", added: "", done: false, ask: "/home/deck/Downloads/Appearance Menu Mod-790-2-6-1.zip" }
            app.showOverlay(true, true)
        }
        else if (shot.page === "nexusnokey" || shot.page === "getmodsfallback" || shot.page === "opening") {
            Turbine.setSampleNexusFail(); app.tab = 1; app.zone = "main"
        }
        else if (shot.page === "getmods" || shot.page === "getmodsback" || shot.page === "desktopgrid" || shot.page === "tv") {
            // Two seeded Recently viewed mods.
            if (shot.arg("game") === "steam:489830") app.recentSeed = [{ id: "2", name: "Lantern Glow SKSE", icon: app.fileUrl(shot.arg("selftest-data") + "/thumb2.png"), source: "nexus" }, { id: "3", name: "Quiet Snow", icon: app.fileUrl(shot.arg("selftest-data") + "/thumb3.png"), source: "nexus" }]
            app.tab = 1; app.zone = "main"
        }
        else if (shot.page === "modpagefree" || shot.page === "modpagepremium") {
            if (shot.page === "modpagefree") Turbine.setSampleFree()
            app.tab = 1; app.zone = "main"
        }
        else if (shot.page === "removed") app.modIndex = app.listItems.length - 1
        else if (shot.page === "rail") { app.zone = "rail"; app.railIndex = 0 }
        else if (shot.page === "tvhidden") { app.openSection("Look"); app.setRow = app.lookRows.findIndex(r => r.key === "layout") }
        else if (shot.page === "overlaywait") {
            app.status = { active: true, text: "Press Save in the browser", level: "wait", progress: -1, game: "", added: "", done: false, ask: "" }
            app.showOverlay(true, false)
        }
        else if (shot.page === "pointerscroll") { app.page = "perf"; app.zone = "perf" }
        else if (shot.page === "perfcheck") app.openPerf()
        // The Activity log opened the way the player opens it: Settings, Labs, its row, A. Every
        // Settings action row has a handler, too.
        else if (shot.page === "activity") {
            app.openSection("Labs"); app.zone = "settings"
            app.setRow = Math.max(0, app.labsRows.findIndex(r => r.key === "activity"))
            app.note("shot", "Settings row focused: " + (app.rowsNow[app.setRow] ? app.rowsNow[app.setRow].key : "none"))
            app.act("a")
            app.note("check", "Activity log sheet open after A on its Settings row: " + (app.sheet === "activity"))
            sc.settingsRowsCheck()
        }
        else if (shot.page === "slowhint") { Turbine.setSampleFree(); app.forceMyBrowser = true }
        // The beginner tutorial's cards; pages for the new controls.
        else if (shot.page.indexOf("tutorial") === 0) app.openTutorial(parseInt(shot.page.substring(8)) - 1)
        else if (shot.page === "nexusguard") { Turbine.setSampleFree(); app.tab = 1; app.zone = "main" }
        else if (shot.page === "presscount") Turbine.setSampleFree()
        // Nexus inside Turbine (the frame alone; the stand-in page under X11; the second run; the watcher).
        else if (shot.page === "browserframe" || shot.page === "browsertest") { Turbine.setSampleFree(); app.tab = 1; app.zone = "main" }
        else if (shot.page === "regress") Turbine.setSampleFree()
        // Mods for any game: Get mods for a game Nexus has under another name.
        else if (shot.page === "nexuspick") { app.tab = 1; app.zone = "main" }
        else if (shot.page === "settings" && shot.arg("season") !== "") app.setRow = 6
    }

    Timer {
        running: shot.on && sc.slow
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
            else if (shot.page === "browser") app.setRow = -1   // no row focused
            // Settings, Getting mods in Game Mode: "Open Nexus in" shows as a fact, the own-browser rows fold.
            else if (shot.page === "accountsgm") app.note("shot", "Getting mods rows (Game Mode " + Turbine.gamemode + "): " + app.accountRows.map(r => r.key).join(", "))
            else if (shot.page === "lootsort" || shot.page === "lootnotes") { app.orderIndex = -1; Turbine.orderSort() }
            // The controller pointer: over the second mod's ON pill, clicked through the same mouse events the right
            // stick and RT make (the mod turns off), then the 2,000 row list scrolled by wheel steps.
            else if (shot.page === "pointer") sc.pointerCheck()
            else if (shot.page === "pointerscroll") sc.pointerScrollCheck()
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
            // Play options; R3 Find with "glow" typed; Select several with one mod ticked.
            else if (shot.page === "playoptions") { app.openPlayOptions(); playOptionsShot.start() }
            else if (shot.page === "findmods") { app.tab = 0; app.zone = "main"; app.act("y"); app.act("y"); askView.open("Find a mod", "", "glow", "", 60, false, "Done"); askView.accept(); app.act("start") }
            else if (shot.page === "selectmods") { app.tab = 0; app.zone = "main"; app.modIndex = 0; app.startSelect(app.mods[0]); app.act("down") }
            // A mod's file page opened on a free account: the status says to press Slow download.
            else if (shot.page === "slowhint") app.openNexus("https://www.nexusmods.com/witcher3/mods/2109?tab=files&file_id=1")
            // The mod menu of a mod that loses a file; the closed-early prompt; the next requirement; the list bar.
            else if (shot.page === "overlaps") { app.tab = 0; app.zone = "main"; app.modIndex = 0; app.sheet = "modmenu"; app.note("shot", "Mod menu: " + app.modMenuTitles()) }
            else if (shot.page === "quickexit" && app.game && app.mods.length > 0) {
                // A setup from yesterday that worked, with only the first mod on.
                app.setups = [{ id: "s1", savedAt: Math.floor(Date.now() / 1000) - 86400, modsOn: 1, played: true, on: [app.mods[0].id] }]
                app.quickExit(app.game.id, 12)
            }
            else if (shot.page === "chain") {
                app.chainQueue = [{ name: "Community Patch", url: "https://www.nexusmods.com/witcher3/mods/3652?tab=files" }]
                app.showChain("Better Torches")
            }
            // My mods' Y pop-up (Find, Show, Sort, Select several), with Show "On" chosen.
            else if (shot.page === "listbar") { app.tab = 0; app.zone = "main"; app.modIndex = 0; app.listShow = "on"; findPopShot.start() }
            // Anti-cheat games: Play asks once (Elden Ring); the mod goes into the me3 profile, and Before you
            // play; Halo's guide; an online-only game refusing mods.
            else if (["acask", "acdeploy", "acguide", "acrefuse"].indexOf(shot.page) >= 0) { app.tab = 0; app.zone = "main"; acShot.start() }
            else if (["modsettings", "coopsettings", "gameupdate", "ersaves"].indexOf(shot.page) >= 0) { app.tab = 0; app.zone = "main"; lateShot.start() }
            // Get mods' Y pop-up and Get mods scrolled down to full screen, once the cards are in.
            else if (shot.page === "findpop" || shot.page === "fullscroll") { app.zone = "main"; app.tab = 1; findPopShot.start() }
            // A on Before you play runs the highlighted button; the mod and game menus offer the missing loader.
            else if (shot.page === "checkfocus") { app.startPlay(); checkFocusShot.start() }
            else if (shot.page === "menuskse") {
                app.tab = 0; app.zone = "main"
                app.modIndex = Math.max(0, app.listItems.findIndex(x => x.kind === "mod" && x.m.needsLoader))
                app.note("shot", "Game menu: " + app.gameMenuItems.map(x => x.title).join("; "))
                app.sheet = "modmenu"
                app.note("shot", "Mod menu: " + app.modMenuTitles())
            }
            else if (shot.scripted) sc.runScript(shot.page)
            else if (shot.page === "wheel") { app.tab = 0; app.zone = "main"; app.modIndex = 0; sc.pad("view"); sc.pad("up"); sc.pad("right") }
            else if (shot.page === "ghost") { app.tab = 0; app.zone = "main"; app.modIndex = 0; if (app.mods[0]) app.uninstall(app.mods[0]) }
            else if (shot.page === "railmenu") { app.zone = "rail"; app.railIndex = app.current; sc.pad("x") }
            else if (shot.page === "browserframe") app.openNexus(sc.btFiles)
            else if (shot.page === "browsertest") sc.browserCheckScript()
            else if (shot.page === "browserpersist") sc.browserPersistScript()
            else if (shot.page === "regress") sc.regressScript()
            // Start on Where do these files go? (Put them here); A on the question about the game's own files
            // (Skip those files); the Nexus game question; and a Basic game's mod menu.
            else if (shot.page === "placestart") { app.note("shot", "Place page open: " + (app.page === "place")); app.placeAct("start"); Turbine.applyNow() }
            else if (shot.page === "overwriteskip") { app.note("shot", "Overwrite sheet open: " + (app.sheet === "msg")); app.act("a"); Turbine.applyNow() }
            else if (shot.page === "nexuspick") {
                // The game opened on Get mods with its anti-cheat card up: the question waits for the card,
                // and comes by itself once B closes it (nexusPickShot looks).
                app.note("shot", "Nexus game question, before: asked by itself " + app.nexusAsks + " times, sheet \"" + app.sheet + "\"" + (app.sheet === "msg" && app.msgData ? " (" + app.msgData.title + ")" : ""))
                if (app.sheet !== "") sc.pad("b")
                nexusPickShot.start()
            }
            else if (shot.page === "why") app.openWhy()
            // The one-time Launch Options step for a me3 game, and the launch check's sheet.
            else if (shot.page === "launchsetup") { launchSetupShot.start(); app.openLaunchSetup({ game: app.game.id, name: "Elden Ring", option: "if [ -x \"/home/deck/.var/app/io.github.ravenor907.TurbineModManager/data/steam-launch/turbine-launch.sh\" ]; then \"/home/deck/.var/app/io.github.ravenor907.TurbineModManager/data/steam-launch/turbine-launch.sh\" %command%; else %command%; fi", set: "", listed: true, coop: false }) }
            else if (shot.page === "notstarted") {
                app.openNotStarted(app.game.id); app.note("shot", "Launch check sheet: " + app.pickTitle + " | " + app.pickItems.map(r => r.title).join("; "))
                // What Turbine's Steam tool said about the last launch shows first, when it said anything.
                app.toolTest = { lastProton: "Started with Proton 9.0 (Beta): Steam used it for this game last time.", nothingStarted: "no Proton was found, so nothing started" }
                app.sheet = ""; app.openNotStarted(app.game.id)
                app.note("shot", "Launch check with the tool's words: " + app.pickItems.map(r => r.title + (r.info ? " (" + r.detail + ")" : "")).join("; "))
                app.sheet = ""; app.openWhy()
                app.note("shot", "Why won't it start with the tool's words: " + app.pickItems.filter(r => /Proton|Steam tool/.test(r.title)).map(r => r.title + ": " + r.detail).join("; "))
                app.sheet = ""; app.openNotStarted(app.game.id)
                // It shows up late: the sheet goes. It started and closed: "closed after N seconds", never "didn't start".
                app.gameSeenLate(app.game.id, 31)
                app.note("shot", "Launch check, the game showed up late: sheet \"" + app.sheet + "\"")
                app.openNotStarted(app.game.id); app.quickExit(app.game.id, 24)
                app.note("shot", "Launch check, it started then closed: sheet " + app.sheet + ", " + infoView.title + " | " + app.quickText + " | " + app.infoButtons.map(b => b.label).join("; "))
                app.toolTest = null
                // The picture is the sheet on its own: the late start's toast belongs to the step before.
                app.quick = null; app.sheet = ""; toast.hide(); app.openNotStarted(app.game.id)
            }
            // A mod renamed, then the list scrolled down a row (the bars and the rail go; the row stays in view).
            else if (shot.page === "tucked" && app.mods.length > 1) {
                app.tab = 0; app.zone = "main"; app.modIndex = 0
                app.renameMod(app.mods[0], "Lanterns of the Road")
                app.modIndex = 1
                // Two mods fit on one screen: Down doesn't hide the bars. The picture shows them hidden anyway.
                app.note("shot", "Short list keeps the bars: " + !app.barsHidden)
                app.note("shot", "Bottom bar: " + barName.text + " | " + app.barStatus + " | tabs " + (legend.tabWordsFit ? app.tabs.length : "caps only") + " | Play " + (playBtn.visible ? "shown" : "hidden") + " | top bar none")
                app.barsHidden = true
                app.note("shot", "Bars hidden: " + app.barsHidden + "; panel: " + app.showModPanel + " (design width " + Math.round(app.designWidth) + ", " + app.width + "x" + app.height + ", tab " + app.tab + ", page " + app.page + "); row 2 line: " + app.rowLine(app.mods[1]).text)
            }
            else if (shot.page === "basicmenu") {
                app.tab = 0; app.zone = "main"; app.modIndex = 0; app.sheet = "modmenu"
                app.note("shot", "Mod menu: " + app.modMenuTitles())
            }
            if (shot.page === "mods") app.note("shot", "Row warnings: " + app.mods.filter(m => !!app.modWarns[m.id]).map(m => m.name + ": " + app.modWarns[m.id]).join("; "))
        }
    }
    Timer {
        id: shotTimer
        running: shot.on
        // Pictures that wait for the cards or the engine, then press, then wait for what the press did.
        // A scripted page takes its picture 1.5 s after its last step (shotAfterScript), 50 s at the latest.
        interval: shot.page === "splashmid" ? 520 : shot.page === "splashafter" ? 2400 : ["findpop", "fullscroll", "listbar", "acask", "acdeploy", "acguide", "acrefuse", "modsettings", "coopsettings", "gameupdate", "ersaves"].indexOf(shot.page) >= 0 ? 9000 : shot.page === "browsertest" ? 200000 : shot.page === "regress" ? 48000 : shot.page === "browserpersist" ? 110000 : shot.page === "inputcheck" ? 45000 : shot.scripted ? 50000 : shot.page.indexOf("loot") === 0 || shot.page.indexOf("nxm") === 0 || shot.page === "getmodsback" ? 6500 : sc.slow ? 4500 : 1500
        // The overlay pictures are of the overlay window itself.
        onTriggered: {
            // Every picture checks the bottom bar (nothing overlaps Play or goes past the edge).
            legend.check(shot.page)
            if (shot.page.indexOf("splash") === 0) app.note("check", "Splash check (" + shot.page + "): shown " + splash.shown + ", icon opacity " + splashIcon.opacity.toFixed(2) + ", scale " + splashIcon.scale.toFixed(2) + ", rail icon none")
            // A picture with animations asked for (--motion=full or reduced): everything has come to rest by now.
            if (shot.arg("motion") !== "" && mo.on && shot.page.indexOf("splash") !== 0) app.note("check", "Motion check: " + shot.page + (app.motionStill ? " settled" : " still moving") + " (animations " + mo.level + ")")
            ;(shot.page === "overlay" || shot.page === "overlaywait" ? panelWin : shot.page === "overlaygm" || shot.page === "caughtgm" ? hudWin : app).contentItem.grabToImage(function (r) { r.saveToFile(shot.path); Qt.quit() })
        }
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
    // The Steam tool's steps: what the one sheet says, then A (Check again) with the tool not picked yet.
    Timer {
        id: launchSetupShot; interval: 700
        onTriggered: {
            const d = app.msgData, t = app.steamTool(app.game.id)
            app.note("shot", "Tool steps sheet: " + app.sheet + " | " + (d ? d.title : "") + " | " + (d && d.steps ? d.steps.length : 0) + " steps, first: " + (d && d.steps && d.steps[0] ? d.steps[0] : "")
                     + " | says after Steam's next start: " + (!!d && /after Steam's next start/.test(d.text)) + " (restart first " + t.restartFirst + ") | buttons " + app.infoButtons.map(b => b.label).join("; ") + " | Game Mode " + Turbine.gamemode)
            const asks = app.playAsks
            sc.pad("a")
            app.note("shot", "Tool steps, Check again: sheet " + app.sheet + ", checks " + app.toolChecks + ", Play asked " + (app.playAsks - asks) + " times, route " + t.route)
        }
    }
    // The Nexus game question came by itself once; a second try doesn't ask again.
    Timer {
        id: nexusPickShot; interval: 700
        onTriggered: {
            const rows = app.sheet === "pick" ? app.pickItems.length : 0, was = app.sheet === "pick" ? app.pickTitle : "\"" + app.sheet + "\""
            if (app.sheet === "pick") sc.pad("b")
            const again = app.maybeAskNexusGame("a second time")
            app.note("shot", "Nexus game question: asked by itself " + app.nexusAsks + " times, sheet " + was + ", " + rows + " rows; a second time: " + (again ? "asked again" : "not asked"))
            app.sheet = ""; app.openNexusPick()
        }
    }
    Timer { id: playOptionsShot; interval: 900; onTriggered: { app.note("shot", "Play options open: " + (app.sheet === "pick" && app.pickTitle === "Play options")) } }
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

    Timer {
        id: acShot
        interval: 3500
        onTriggered: {
            const a = app.acInfo
            app.note("shot", "Anti-cheat rules: " + (a ? a.mode + ", ask " + a.needsAsk + ", off " + a.off + ", me3 ready " + a.me3Ready : "none") + ", " + app.mods.length + " mods; bar: " + barName.text + " | " + app.barStatus)
            if (shot.page === "acask") {
                app.startPlay()
                app.note("shot", "Anti-cheat sheet: " + app.sheet + " " + (app.msgData ? app.msgData.title : "") + " | buttons " + app.infoButtons.map(x => x.label).join(", "))
            } else if (shot.page === "acdeploy") {
                Turbine.applyNow()
                app.acAnswered = ({ "steam:1245620": true })
                Turbine.antiCheatOff(true)
                app.startPlay()
            } else if (shot.page === "acguide") {
                app.note("shot", "Game menu: " + app.gameMenuItems.map(x => x.title).join("; "))
                app.openAcGuide()
            } else {
                app.note("shot", "Refused: tabs " + app.tabs.length + ", status " + app.statusNow.words + ", Play " + (playBtn.visible ? "shown" : "hidden"))
            }
        }
    }
    // Edit settings on a mod (Right, then Start saves), Seamless Co-op settings (the password first), the game
    // update warning (A holds the mods), and Elden Ring's saves by kind.
    Timer {
        id: lateShot
        interval: 3500
        property int step: 0
        onTriggered: {
            if (shot.page === "modsettings") {
                const m = app.mods.find(x => x.name.indexOf("Glow") >= 0)
                if (step === 0) {
                    app.tab = 0; app.zone = "main"; app.modIndex = Math.max(0, app.listItems.findIndex(x => x.kind === "mod" && x.m === m))
                    app.note("shot", "Mod menu: " + app.modMenuTitles())
                    // It comes in off (SKSE isn't there): on, so the edited copy goes into the game.
                    if (m && !m.enabled) Turbine.setEnabled(m.id, true)
                    if (m) app.openModSettings(m)
                    step = 1; interval = 1200; restart()
                } else if (step === 1) {
                    const r = settingsView.shown[settingsView.focusIndex]
                    app.note("shot", "Settings page: " + app.sheet + " | " + settingsView.shown.length + " rows | focus " + (r && r.f ? r.f.key + " " + r.f.kind + " " + settingsView.valueOf(r.f) : "none"))
                    const v0 = r && r.f ? settingsView.valueOf(r.f) : ""
                    app.act("right")
                    app.note("check", "First Right after Edit settings opened: " + (r && r.f && settingsView.valueOf(r.f) === v0 && settingsView.changes === 0 ? "nothing changed" : "PROBLEM: it changed " + v0 + " to " + (r && r.f ? settingsView.valueOf(r.f) : "")))
                    app.act("right")
                    app.note("shot", "After Right: " + (r && r.f ? settingsView.valueOf(r.f) : "") + ", " + settingsView.changes + " change; legend " + app.legendHints.map(h => h[0].toUpperCase() + " " + h[1]).join(" · "))
                    app.act("start")
                    step = 2; interval = 1500; restart()
                } else {
                    app.note("shot", "After Save: sheet " + app.sheet + ", " + settingsView.changes + " changes, edited " + settingsView.doc.edited)
                    // X opens the options; its first row shows the file as text; again as a list.
                    app.act("x")
                    const first = app.sheet === "pick" && app.pickItems[0] ? app.pickItems[0].title : ""
                    app.act("a")
                    const text = app.sheet === "settingsedit" && settingsView.plain
                    app.act("x"); app.act("a")
                    app.note("check", "Edit settings X: " + app.pickTitle + ", first " + first + "; then " + (text ? "as text" : "not as text") + ", back " + (app.sheet === "settingsedit" && !settingsView.plain ? "as a list" : "PROBLEM"))
                }
            } else if (shot.page === "coopsettings") {
                // The bar is looked at once it has laid itself out again with no sheet open.
                if (step === 0) { app.sheet = ""; step = 1; interval = 800; restart(); return }
                if (step === 1) {
                    app.note("shot", "Seamless in: " + app.seamlessIn + "; game menu: " + app.gameMenuItems.map(x => x.title).join("; "))
                    app.note("shot", "Co-op chosen at Play: " + app.seamlessIn + "; Play options: " + (app.seamlessIn ? "Play Seamless Co-op" : "none"))
                    legend.check("coopbar")
                    app.openCoopSettings()
                    step = 2; interval = 1200; restart(); return
                }
                app.note("shot", "Seamless page: " + settingsView.title + " | first " + (settingsView.shown[0] && settingsView.shown[0].f ? settingsView.shown[0].f.key : "none")
                         + " | " + settingsView.shown.filter(r => r.f).map(r => r.f.key + ":" + r.f.kind).join(", "))
            } else if (shot.page === "gameupdate") {
                const m = app.mods.find(x => x.name.indexOf("Glow") >= 0)
                app.gameUpdate = { last: "99", now: "100", risky: m ? [{ id: m.id, name: m.name }] : [] }
                app.maybeGameUpdate()
                app.note("shot", "Update sheet: " + app.sheet + " " + (app.msgData ? app.msgData.title + " | " + app.infoButtons.map(x => x.label).join(", ") : ""))
            } else if (shot.page === "ersaves") app.openErSaves()
        }
    }
    // findpop: Y on Get mods once the cards are in, then Down to Sort. fullscroll: Down through the cards until the
    // bars and the rail are gone.
    Timer {
        id: findPopShot
        interval: 3500
        onTriggered: {
            if (shot.page === "listbar") {
                app.zone = "main"; app.modIndex = 0
                app.act("y"); app.act("down")
                app.note("shot", "My mods pop-up legend: " + app.legendHints.map(x => x[0].toUpperCase() + " " + x[1]).join(" · "))
            } else if (shot.page === "findpop") {
                app.act("y"); app.act("down")
                app.note("shot", "Get mods pop-up: " + app.findSections.map(x => x.title + ": " + x.options.join("/")).join("; ") + " | legend " + app.legendHints.map(x => x[0].toUpperCase() + " " + x[1]).join(" · ") + " | sheet " + app.sheet)
                app.note("shot", "Get mods page: search row " + (app.getArea === "bar" ? "focused" : "none") + ", top bar none")
            } else {
                for (let i = 0; i < 3; i++) app.act("down")
                fullBackShot.start()
            }
        }
    }
    Timer {
        id: fullBackShot
        interval: 600
        onTriggered: {
            app.note("shot", "Full screen: bars hidden " + app.barsHidden + ", rail " + (rail.visible ? "shown" : "hidden") + ", bottom bar " + (legend.shown ? "shown" : "hidden") + ", card " + (app.getIndex + 1) + " of " + app.getItems.length)
            // Up a row brings them back; then down again for the picture.
            app.act("up")
            app.note("shot", "Full screen, Up: bars hidden " + app.barsHidden)
            app.act("down")
            app.note("shot", "Full screen, Down again: bars hidden " + app.barsHidden)
        }
    }

    // ---------------------------------------------------------------- the build's scripted checks (the controls)
    // Each page is a list of steps run 0.6 s apart; a step that answers false waits and is tried again (up to 12
    // times). Presses go through Turbine.selftestPad, the same path a controller's take (pointer included).
    property var script: []
    property int scriptStep: 0
    property int scriptTries: 0
    property var scriptMemo: ({})
    Timer { id: scriptTimer; interval: 600; repeat: true; onTriggered: sc.scriptTick() }
    function pad(b) { Turbine.selftestPad(b, true); Turbine.selftestPad(b, false) }
    function check(text) { app.note("check", text) }
    function scriptTick() {
        if (scriptStep >= script.length) { scriptTimer.stop(); shotAfterScript(); return }
        let ok = true
        try { ok = script[scriptStep]() !== false } catch (e) { check("Step " + (scriptStep + 1) + " stopped: " + e); ok = true }
        if (ok || ++scriptTries > 12) {
            if (!ok) check("Step " + (scriptStep + 1) + " gave up waiting (game " + (app.game ? app.game.name : "none") + ", " + app.mods.length + " mods, tab " + app.tab + ", zone " + app.zone
                           + ", Get mods " + app.getArea + " " + app.getIndex + " of " + app.getItems.length + ", mod page " + app.modPage + ", sheet \"" + app.sheet + "\")")
            scriptStep++; scriptTries = 0
        }
    }
    // The picture of a scripted page comes once its steps are done, however long they took (a fixed time cut
    // the longer ones short: Menu fit's last menus never ran).
    function shotAfterScript() {
        if (!shot.scripted || shot.page === "inputcheck") return
        check("Script done: " + script.length + " steps")
        shotTimer.interval = 1500
        shotTimer.restart()
    }
    function runScript(name) {
        const m = scriptMemo
        if (name === "controls") script = [
            () => { if (app.mods.length < 2) return false; app.page = "game"; app.zone = "main"; app.tab = 0; app.modIndex = 0 },
            // 2: LB and RB in Settings change the section, never the game.
            () => { m.g = app.game.id; app.openSettings(); pad("rb"); pad("rb"); pad("lb"); check("Settings LB/RB: section " + app.sec + ", game " + (app.game.id === m.g ? "unchanged" : "changed to " + app.game.id)); pad("b") },
            // 7: pointer shown: RT is one click (the ON pill), LT the quick menu; after a D-pad press LT and RT page.
            () => { const it = modList.itemAtIndex(0); if (!it || !it.pill) return false
                    const p = it.pill.mapToItem(null, it.pill.width / 2, it.pill.height / 2); m.on = app.mods[0].enabled; m.toggles = 0
                    Turbine.pointerShowAt(p.x, p.y); pad("rt") },
            () => { check("Pointer RT: " + (app.mods[0].enabled !== m.on ? "one click, the mod turned " + (app.mods[0].enabled ? "on" : "off") : "no click reached the mod")) },
            () => { const it = modList.itemAtIndex(0); const p = it.mapToItem(null, it.width * 0.4, it.height / 2); Turbine.pointerShowAt(p.x, p.y); pad("lt") },
            () => { check("Pointer LT: sheet \"" + app.sheet + "\""); pad("b") },
            () => { const it = modList.itemAtIndex(0); const p = it.mapToItem(null, it.width * 0.4, it.height / 2); Turbine.pointerShowAt(p.x, p.y); app.modIndex = 0; pad("down") },
            () => { m.row = app.modIndex; m.shown = Turbine.pointerShown; pad("lt"); m.row2 = app.modIndex; pad("rt")
                    check("After a D-pad press: pointer " + (m.shown ? "still showing" : "hidden") + ", LT row " + (m.row + 1) + " to " + (m.row2 + 1) + ", RT to " + (app.modIndex + 1) + ", sheet \"" + app.sheet + "\"") },
            // R3 with the pointer hidden: does what A does (the pointer stays hidden); with it showing, a left click.
            () => { app.modIndex = 0; m.on = app.mods[0].enabled; pad("r3") },
            () => { check("R3 hidden: pointer " + (Turbine.pointerShown ? "shown" : "stays hidden") + ", " + (app.mods[0].enabled !== m.on ? "same as A (the mod turned " + (app.mods[0].enabled ? "on" : "off") + ")" : "A didn't run"))
                    const it = modList.itemAtIndex(0); const p = it.mapToItem(null, it.width * 0.4, it.height / 2); Turbine.pointerShowAt(p.x, p.y); pad("r3") },
            () => { pad("down"); pad("up") },
            // 8: the focus table: toggle keeps the row; a menu closes back to it; tabs and a game round trip keep it.
            () => { app.modIndex = 1; pad("a"); check("Toggle: focus on row " + (app.modIndex + 1)); pad("x"); m.sheet = app.sheet; pad("b"); check("Menu " + m.sheet + " closed: focus on row " + (app.modIndex + 1)) },
            () => { app.modIndex = 1; pad("rb"); m.tab = app.tab; pad("lb"); check("Tabs round trip: tab " + m.tab + " and back to " + app.tab + ", row " + (app.modIndex + 1)) },
            () => { m.home = app.game.id; m.other = app.games.findIndex(g => g.id !== app.game.id); if (m.other < 0) { check("Game round trip: only one game"); return }
                    app.tab = 0; app.zone = "main"; app.modIndex = 1; m.want = app.focusedItem && app.focusedItem.m ? app.focusedItem.m.name : ""; app.zone = "rail"; app.railIndex = m.other; pad("a") },
            () => { if (!app.game || app.game.id === m.home) return false; app.zone = "rail"; app.railIndex = app.games.findIndex(g => g.id === m.home); pad("a") },
            () => { if (!app.game || app.game.id !== m.home || !app.modsLoaded || app.mods.length < 2) return false
                    check("Game round trip: back on " + app.game.name + ", tab " + app.tab + ", row " + (app.modIndex + 1) + " (" + (app.focusedItem && app.focusedItem.m ? app.focusedItem.m.name : "") + ", was " + m.want + "; remembered " + app.lastFocusId[app.game.id] + ")" + (app.focusedItem && app.focusedItem.m && app.focusedItem.m.name === m.want ? ": the same mod" : ": a different mod")) },
            // The quick wheel: tap View, Up then Right is Up-right (Sort); hold View, push Left, let go: Games.
            () => { pad("view"); pad("up"); pad("right"); check("Wheel: slot " + app.wheelSlot + " " + (app.wheelSlots[app.wheelSlot] ? app.wheelSlots[app.wheelSlot].label : "")); pad("b") },
            () => { Turbine.selftestPad("view", true); pad("left"); Turbine.selftestPad("view", false); check("Wheel held, Left, let go: sheet " + app.sheet + " \"" + app.pickTitle + "\""); pad("b") },
            // The neutral middle. A tap of View opens it with nothing picked, and A closes it.
            () => { app.sheet = ""; pad("view"); m.open = app.wheelOpen; m.slot = app.wheelSlot; m.a = app.binds().a.label; m.b = app.binds().b.label; pad("a")
                    check("Wheel neutral: a tap of View leaves it " + (m.open ? "open" : "closed") + " with slot " + m.slot + ", A " + m.a + ", B " + m.b + "; A closes it: " + !app.wheelOpen) },
            // The stick pushed Right picks that wedge; back at rest nothing is picked. A D-pad pick stays. B closes.
            () => { pad("view"); Turbine.wheelAngle = 90; m.s1 = app.wheelSlot; Turbine.wheelAngle = -1; m.s2 = app.wheelSlot; pad("up"); m.s3 = app.wheelSlot; pad("b")
                    check("Wheel stick: pushed Right picks slot " + m.s1 + ", at rest slot " + m.s2 + "; D-pad Up picks slot " + m.s3 + " and it stays; B closes it: " + !app.wheelOpen) },
            // View held with nothing pushed, then let go: it closes.
            () => { m.t = Date.now(); Turbine.selftestPad("view", true); m.open = app.wheelOpen },
            () => { if (Date.now() - m.t < 600) return false; Turbine.selftestPad("view", false)
                    check("Wheel held, nothing pushed, let go: it was " + (m.open ? "open" : "closed") + ", now " + (app.wheelOpen ? "still open" : "closed") + ", sheet \"" + app.sheet + "\"") },
            // A click (or a tap) on the middle closes it.
            () => { pad("view"); m.open = app.wheelOpen
                    Turbine.selftestMouse(0, app.width / 2, app.height / 2, 0, 0, 0); Turbine.selftestMouse(1, app.width / 2, app.height / 2, Qt.LeftButton, 0, 0); Turbine.selftestMouse(2, app.width / 2, app.height / 2, Qt.LeftButton, 0, 0) },
            () => { check("Wheel middle clicked: it was " + (m.open ? "open" : "closed") + ", now " + (app.wheelOpen ? "still open" : "closed")) ; app.closeWheel() },
            // Uninstall: the same row stays (dimmed, A puts it back).
            () => { app.zone = "main"; app.tab = 0; app.modIndex = 0; m.name = app.mods[0].name; pad("x"); app.menuIndex = app.modMenuItems.findIndex(x => x.act === "remove"); pad("a") },
            () => { check("Uninstall: row " + (app.modIndex + 1) + " is " + (app.focusedItem ? app.focusedItem.kind : "nothing") + (app.focusedItem && app.focusedItem.kind === "ghost" ? " (" + app.focusedItem.m.name + ")" : "")); pad("a") },
            () => { if (!app.mods.some(x => x.name === m.name)) return false; check("Put back: " + m.name + " is back"); pad("x") }
        ]
        else if (name === "legendcheck") script = [
            () => { if (app.mods.length < 2) return false
                const ctxs = [
                    ["My mods", () => { app.sheet = ""; app.page = "game"; app.zone = "main"; app.tab = 0; app.modIndex = 0 }],
                    ["Find pop-up (My mods)", () => { app.openFindPop() }],
                    ["Rail", () => { app.sheet = ""; app.modIndex = 0; app.zone = "rail"; app.railIndex = app.current }],
                    ["Play", () => { app.zone = "play" }],
                    ["Find pop-up (Get mods)", () => { app.zone = "main"; app.tab = 1; app.getArea = "grid"; app.getIndex = 0; app.openFindPop() }],
                    ["Get mods grid", () => { app.sheet = ""; app.getArea = "grid"; app.getIndex = 0 }],
                    ["Load order", () => { app.tab = 2; app.orderIndex = 0 }],
                    ["Settings", () => { app.tab = 0; app.openSettings() }],
                    ["Settings Look", () => { app.setSection = app.settingsSections.indexOf("Look"); app.setRow = 0 }],
                    ["Settings Help", () => { app.setSection = app.settingsSections.indexOf("Help"); app.setRow = 0 }],
                    ["Mod menu", () => { app.settingsBack(); app.zone = "main"; app.tab = 0; app.modIndex = 0; app.sheet = "modmenu" }],
                    ["Game menu", () => { app.sheet = "gamemenu" }],
                    ["Before you play", () => { app.sheet = ""; app.checks = [{ level: "warn", title: "A test warning", detail: "" }]; app.checkIndex = 0; app.sheet = "check" }],
                    ["Loader", () => { app.sheet = ""; app.loaderFor = ["Lantern Glow SKSE"]; app.openLoader() }],
                    ["Delete for good", () => { app.sheet = ""; app.askDelete("all", "Test") }],
                    ["Text entry", () => { app.sheet = ""; app.openFind() }],
                    ["Text entry buttons", () => { app.askIndex = 2 }],
                    ["Play options", () => { askView.close(); app.sheet = ""; app.openPlayOptions() }],
                    ["Removed", () => { app.sheet = ""; app.openRemoved() }],
                    ["Quick wheel", () => { app.sheet = ""; app.wheelPress(); app.wheelHeld = false }],
                    ["Quick wheel slot", () => { app.wheelMove("up") }],
                    ["Tutorial", () => { app.closeWheel(); app.openTutorial(0) }],
                    ["Moving", () => { app.sheet = ""; app.modIndex = 0; app.startMove() }],
                    ["Pointer", () => { app.cancelMove(); const it = modList.itemAtIndex(0); const p = it.mapToItem(null, 20, 20); Turbine.pointerShowAt(p.x, p.y) }],
                    ["Nexus page", () => { app.openBrowser(btFiles, "page") }],
                    ["Nexus downloads", () => { app.browserTab = 1 }],
                    ["Nexus menu", () => { app.browserTab = 0; app.openBrowserMenu() }],
                    ["Nexus quick wheel", () => { app.sheet = ""; app.wheelPress(); app.wheelHeld = false }]
                ]
                let problems = 0
                for (const c of ctxs) {
                    c[1]()
                    const o = app.binds(), h = app.legendFor(o), shown = app.legendHints
                    const bad = legendProblems(o, h, keys => [].concat(keys.some(k => ["lb", "rb", "view"].indexOf(k) >= 0) ? ["a cap in the legend"] : [],
                        keys.indexOf("start") >= 0 && app.sheet === "" && !app.wheelOpen && app.page !== "browser" && app.page !== "place" && app.page !== "settings" ? ["Start in the legend with Play in view"] : []))
                    if (JSON.stringify(shown) !== JSON.stringify(h)) bad.push("the legend on screen differs")
                    problems += bad.length > 0 ? 1 : 0
                    check("Legend " + c[0] + ": " + h.map(x => (x[2] ? x[2] + " " : "") + x[0].toUpperCase() + " " + x[1] + (x[3] ? ", " + x[3].toUpperCase() + " " + x[4] : "")).join(" · ") + (bad.length > 0 ? "  PROBLEM: " + bad.join(", ") : ""))
                }
                app.closeWheel(); app.sheet = ""; app.closeBrowser("back")
                Turbine.pointerHide()
                check("Legend check: " + ctxs.length + " contexts, " + problems + " problems")
                app.sheet = ""
            }
        ]
        // Seamless Co-op chosen at Play: the choice, the password step, the remembered choice, its settings
        // from all three places, and a toggle, a number and the password saved into Turbine's copy of the file.
        else if (name === "coopcheck") script = [
            () => { if (!app.seamlessIn || !app.modsLoaded) return false; app.sheet = ""; app.page = "game"; app.zone = "main"; app.tab = 0; app.acAnswered = ({ "steam:1245620": true }); Turbine.selftestPad("start", true) },
            () => { if (app.sheet !== "pick") return false
                    Turbine.selftestPad("start", false)
                    check("Co-op only: the Play choice says \"" + app.pickSub + "\"; Play with mods: " + app.pickItems[0].detail + "; its row in My mods: " + app.rowLine(app.seamlessMod).text)
                    check("Co-op choice: " + app.pickItems.map(x => x.title).join("; ") + " | focus " + (app.pickItems[app.menuIndex] ? app.pickItems[app.menuIndex].title : "") + " | Y " + (app.pickY ? app.pickY.label : "none"))
                    pad("down"); m.focus = app.pickItems[app.menuIndex] ? app.pickItems[app.menuIndex].title : ""; pad("a") },
            () => { if (app.sheet !== "ask") return false
                    const r = settingsView.shown[settingsView.focusIndex]
                    m.asks = app.playAsks
                    check("Co-op password step: picked " + m.focus + ", sheet " + app.askReturn + " with the keyboard (" + app.sheet + "), focus " + (r && r.f ? r.f.key : "none") + ", waiting " + app.coopAfterSettings + ", Play asked " + app.playAsks + " times")
                    askView.open("Session password", "", "dragon", "", 2000, false, "Done"); askView.accept() },
            () => { if (app.sheet === "settingsedit" || app.sheet === "ask") return false; check("Co-op after the password: sheet \"" + app.sheet + "\", waiting " + app.coopAfterSettings + ", Play asked " + (app.playAsks - m.asks) + " times, co-op " + (app.playCoopNext || app.playAsks > m.asks)) },
            () => { app.sheet = ""; app.page = "game"; app.zone = "play"; pad("a") },
            () => { if (app.sheet !== "pick") return false
                    check("Co-op choice remembered: focus " + (app.pickItems[app.menuIndex] ? app.pickItems[app.menuIndex].title : "") + ", Play says " + (app.lastPlayChoice === "coop" ? "Co-op" : "Solo"))
                    pad("y") },
            () => { if (app.sheet !== "settingsedit") return false; check("Co-op settings from the Play choice (Y): " + settingsView.title); pad("b") },
            () => { app.sheet = ""; app.menuIndex = app.gameMenuItems.findIndex(x => x.act === "coopsettings"); m.gi = app.menuIndex; app.sheet = "gamemenu"; app.menuIndex = m.gi; check("Game menu first row: " + (app.gameMenuItems[0] ? app.gameMenuItems[0].title : "")); pad("a") },
            () => { if (app.sheet !== "settingsedit") return false; check("Co-op settings from the game menu: " + settingsView.title); pad("b") },
            () => { app.sheet = ""; app.tab = 0; app.zone = "main"; app.modIndex = Math.max(0, app.listItems.findIndex(x => x.kind === "mod" && app.isSeamless(x.m))); pad("x"); app.menuIndex = app.modMenuItems.findIndex(x => x.act === "settings"); check("Mod menu: " + app.modMenuTitles()); pad("a") },
            () => { if (app.sheet !== "settingsedit") return false
                    check("Co-op settings from the mod's X menu: " + settingsView.title)
                    const t = settingsView.fields.find(x => x.key === "allow_invaders"), n = settingsView.fields.find(x => x.key === "enemy_health_scaling")
                    if (t) settingsView.setValue(t, t.value === "1" ? "0" : "1")
                    if (n) settingsView.setValue(n, "50")
                    check("Co-op edits: allow_invaders to " + (t ? settingsView.valueOf(t) : "?") + ", enemy_health_scaling to 50") },
            () => { if (settingsView.changes > 0) return false; check("Co-op edits saved by themselves: 0 waiting"); pad("b") },
            // The Co-op tile is asked about once after Seamless Co-op is installed (never by itself otherwise),
            // and has its row on the Co-op settings page.
            () => { if (app.sheet !== "") return false; m.asks0 = app.coopTileAsks; m.r1 = app.maybeAskCoopTile()
                    m.t = app.sheet === "msg" && app.msgData ? app.msgData.title + " | " + app.msgData.buttons.map(x => x.label).join("; ") : "sheet \"" + app.sheet + "\""; pad("b") },
            () => { m.s = app.sheet; m.r2 = app.maybeAskCoopTile()
                    check("Co-op tile question: asked by itself " + m.asks0 + " times; after the install: " + (m.r1 ? "asked" : "not asked") + ", " + m.t + "; B: sheet \"" + m.s + "\"; a second time: " + (m.r2 ? "asked again" : "not asked"))
                    app.sheet = ""; app.openCoopSettings() },
            () => { if (app.sheet !== "settingsedit") return false
                    const i = settingsView.shown.findIndex(r => r.kind === "action"); m.row = i >= 0 ? settingsView.shown[i] : null
                    if (i >= 0) settingsView.focusIndex = i
                    m.a = app.binds().a.label; pad("a") },
            () => { check("Co-op settings page: row " + (m.row ? m.row.title + " [" + m.row.label + "]" : "missing") + ", A " + m.a + ", A opens: " + (app.sheet === "msg" && app.msgData ? app.msgData.title : "sheet \"" + app.sheet + "\"")); pad("b") }
        ]
        // Edit settings on the mod's panel: a tap and Right then A open that mod's settings; no button
        // for a mod without settings.
        // A selects and B goes back one step, on every screen and sheet, by real presses. Each
        // case starts on My mods, opens its screen, reads what A is bound to, presses B and checks where it landed.
        else if (name === "abcheck") {
            const where = () => [app.page, "tab " + app.tab, "zone " + app.zone, "sheet " + (app.sheet || "none"), app.modPage ? "mod page" : "", app.wheelOpen ? "wheel" : ""].filter(x => x).join(", ")
            const home = () => { app.sheet = ""; if (app.wheelOpen) app.wheelOpen = false; app.modPage = false; app.history = []; app.page = "game"; app.tab = 0; app.zone = "main"; app.modIndex = 0 }
            const sheetCase = (label, open) => [label, open, () => app.sheet === "" && app.page === "game" && app.tab === 0 && !app.modPage]
            const cases = [
                sheetCase("Mod menu", () => { app.menuIndex = 0; app.sheet = "modmenu" }),
                sheetCase("Game menu", () => { app.menuIndex = 0; app.sheet = "gamemenu" }),
                sheetCase("Add from Downloads", () => app.openAdd()),
                sheetCase("Loadouts", () => app.openProfiles()),
                sheetCase("Play options", () => app.openPlayOptions()),
                sheetCase("Find pop-up", () => app.openFindPop()),
                sheetCase("Before you play", () => { app.checks = [{ level: "warn", title: "A test warning", detail: "" }]; app.checkIndex = 0; app.sheet = "check" }),
                sheetCase("Mod loader", () => { app.loaderFor = ["Lantern Glow SKSE"]; app.openLoader() }),
                sheetCase("Activity log", () => app.openActivity()),
                sheetCase("Removed mods", () => app.openRemoved()),
                ["Quick wheel", () => pad("view"), () => !app.wheelOpen && app.sheet === "" && app.page === "game" && app.tab === 0],
                ["Settings", () => app.openSettings(), () => app.page === "game" && app.tab === 0 && app.sheet === ""],
                ["Get mods", () => { app.tab = 1; app.zone = "main" }, () => app.tab === 0 && app.page === "game"],
                ["A mod's page", () => { app.tab = 1; app.zone = "main"; if (app.getItems.length > 0) app.openDetails(app.getItems[0]) }, () => app.tab === 1 && !app.modPage && app.page === "game"],
                ["Play button", () => { app.zone = "play" }, () => app.zone === "main" && app.page === "game"]
            ]
            let steps = [() => { if (app.mods.length < 2 || !app.modsLoaded) return false; m.bad = 0; m.n = 0; home() }]
            for (const cs of cases) {
                steps.push(() => { home(); cs[1]() })
                steps.push(() => {
                    if (cs[0] === "A mod's page" && !app.modPage) return false
                    if (cs[0] === "Quick wheel" && !app.wheelOpen) return false
                    if (cs[0].indexOf("menu") >= 0 && cs[0] !== "Mod menu" && cs[0] !== "Game menu" && app.sheet === "") return false
                    const o = app.binds()
                    m.a = o.a && o.a.label ? o.a.label : (app.wheelOpen ? "(the wedge)" : "")
                    m.bLabel = o.b && o.b.label !== undefined ? (o.b.label || "(unlabelled)") : ""
                    m.at = where()
                    pad("b")
                })
                steps.push(() => {
                    // A view with nothing to choose (the activity log only reads) has no A; every other one does.
                    const ok = cs[2]() && (m.a !== "" || cs[0] === "Activity log") && m.bLabel !== ""
                    m.n++
                    if (!ok) m.bad++
                    check("A and B, " + cs[0] + ": A " + (m.a || "nothing") + ", B " + (m.bLabel || "nothing") + " from " + m.at + " to " + where() + (ok ? "" : "  PROBLEM"))
                })
            }
            steps.push(() => { home(); check("A and B (" + app.width + "x" + app.height + "): " + m.n + " screens, " + m.bad + " problems") })
            script = steps
        }
        // Nothing goes through a pop-up. With each sheet, menu or the wheel open, a real click
        // on Play in the bar and on the first game in the rail (both under the pop-up's backdrop) must start nothing
        // and change nothing beneath.
        else if (name === "passthrough") {
            const cases = [
                ["Mod menu", () => { app.menuIndex = 0; app.sheet = "modmenu" }],
                ["Game menu", () => { app.menuIndex = 0; app.sheet = "gamemenu" }],
                ["Add from Downloads", () => app.openAdd()],
                ["Loadouts", () => app.openProfiles()],
                ["Play options", () => app.openPlayOptions()],
                ["Find pop-up", () => app.openFindPop()],
                ["Before you play", () => { app.checks = [{ level: "warn", title: "A test warning", detail: "" }]; app.checkIndex = 0; app.sheet = "check" }],
                ["Mod loader", () => { app.loaderFor = ["Lantern Glow SKSE"]; app.openLoader() }],
                ["Activity log", () => app.openActivity()],
                ["Delete for good", () => app.askDelete("check:none", "Test")],
                ["Quick wheel", () => pad("view")]
            ]
            const state = () => [app.game ? app.game.id : "", app.tab, app.page, app.mods.map(x => x.enabled ? 1 : 0).join(""), app.playAt].join("|")
            let steps = [() => { if (app.mods.length < 2 || !app.modsLoaded) return false; m.bad = 0; m.n = 0; app.sheet = ""; app.page = "game"; app.tab = 0; app.zone = "main" }]
            for (const cs of cases) {
                for (const target of ["Play", "the rail"]) {
                    steps.push(() => { Turbine.pointerHide(); app.sheet = ""; if (app.wheelOpen) app.wheelOpen = false; app.page = "game"; app.tab = 0; app.zone = "main"; cs[1]() })
                    steps.push(() => {
                        if (app.sheet === "" && !app.wheelOpen) return false
                        // The rail: a tile of another game that is on screen now (tiles out of view aren't made).
                        const railTile = () => { for (let i = 0; i < app.games.length; i++) { if (app.game && app.games[i].id === app.game.id) continue
                                                 const t = railList.itemAtIndex(i); if (!t || !t.visible) continue
                                                 const y0 = t.mapToItem(null, 0, 0).y; if (y0 >= 0 && y0 + t.height <= app.height) return t } return null }
                        const it = target === "Play" ? playBtn : railTile()
                        if (!it || !it.visible) { m.skip = target + " not shown"; return }
                        m.skip = ""
                        m.before = state(); m.open = app.sheet || "wheel"
                        const p = it.mapToItem(null, it.width / 2, it.height / 2)
                        Turbine.selftestPointer(p.x, p.y, true, 0)
                    })
                    steps.push(() => {
                        if (m.skip) { check("Pass-through, " + cs[0] + ", a click on " + target + ": skipped (" + m.skip + ")"); return }
                        const after = state()
                        const ok = after === m.before && !(app.sheet === "check" && m.open !== "check")
                        m.n++
                        if (!ok) m.bad++
                        check("Pass-through, " + cs[0] + ", a click on " + target + ": " + (ok ? "nothing beneath changed" : "PROBLEM: " + m.before + " to " + after) + ", " + m.open + " then " + (app.sheet || (app.wheelOpen ? "wheel" : "closed")))
                    })
                }
            }
            steps.push(() => { Turbine.pointerHide(); app.sheet = ""; check("Pass-through (" + app.width + "x" + app.height + "): " + m.n + " clicks, " + m.bad + " problems") })
            script = steps
        }
        // The golden path, by real presses only: an empty game, X Add from Downloads, A on the file, the mod is on
        // in My mods, Start held 3 seconds and A play it (Before you play's Play anyway when it asks); then the mod off and on with A, X
        // Uninstall and A Put back.
        else if (name === "golden") script = [
            () => { if (!app.modsLoaded || !app.game) return false; app.sheet = ""; app.page = "game"; app.tab = 0; app.zone = "main"; m.played = app.playAt
                    check("Journey start: " + app.game.name + ", " + app.mods.length + " mods, legend " + app.legendHints.map(h => h[0].toUpperCase() + " " + h[1]).join(" · ")); pad("x") },
            () => { if (app.sheet !== "add" || app.addList.length === 0) return false
                    app.addIndex = Math.max(0, app.addList.findIndex(d => d.name.indexOf("Brighter Lanterns") === 0)); check("Journey add: sheet add, " + app.addList.length + " files, A " + (app.binds().a ? app.binds().a.label : "")); pad("a") },
            () => { if (app.mods.length < 1 || app.page === "place") return false
                    check("Journey added: " + app.mods.map(x => x.name + (x.enabled ? " on" : " off")).join(", ") + ", sheet " + (app.sheet || "none")); app.sheet = ""; Turbine.selftestPad("start", true) },
            () => { if (app.sheet !== "pick") return false
                    Turbine.selftestPad("start", false)
                    check("Journey Start held: " + app.pickTitle + ", focus " + (app.pickItems[app.menuIndex] ? app.pickItems[app.menuIndex].title : "")); pad("a") },
            () => { if (app.sheet === "check") { const o = app.binds(); check("Journey Before you play: A " + (o.a ? o.a.label : "") + " | " + app.checks.map(c => c.level + " " + c.title).join("; ")); if (app.checks.some(c => c.level === "stop")) return; app.act("right"); pad("a"); return false }
                    if (app.playAt === m.played) return false
                    check("Journey played: Play started " + app.game.name + " with " + app.mods.filter(x => x.enabled).length + " mods on") },
            () => { app.sheet = ""; app.zone = "main"; app.tab = 0; app.modIndex = Math.max(0, app.listItems.findIndex(x => x.kind === "mod")); m.on = app.focusedItem && app.focusedItem.m ? app.focusedItem.m.enabled : null; pad("a") },
            () => { if (!app.focusedItem || !app.focusedItem.m || app.focusedItem.m.enabled === m.on) return false; m.mid = app.focusedItem.m.id; pad("a") },
            () => { const r = app.mods.find(x => x.id === m.mid); if (!r || r.enabled !== m.on) return false
                    check("Journey A off and on: " + r.name + " " + (r.enabled ? "on" : "off") + " again"); pad("x") },
            () => { if (app.sheet !== "modmenu") return false; const i = app.modMenuItems.findIndex(x => /^Uninstall/.test(x.title)); if (i < 0) { check("Journey uninstall: no Uninstall row"); return }
                    app.menuIndex = i; pad("a") },
            () => { if (!app.focusedItem || app.focusedItem.kind !== "ghost") return false; check("Journey uninstall: the row says " + app.focusedItem.m.name + " removed, A " + (app.binds().a ? app.binds().a.label : "")); pad("a") },
            () => { if (!app.mods.some(x => x.id === m.mid)) return false; check("Journey put back: " + app.mods.find(x => x.id === m.mid).name + " is back in My mods") }
        ]
        // The Mod Manager Download question by real presses, timed from its first frame: no
        // input in its first second (also a press made then but handled late), the ring moves to Get it at 1.2 s
        // only when nothing was pressed, and a Left after the first second keeps Not now for good.
        else if (name === "mmdguard") {
            const ask = () => app.nxmAsk("nxm://skyrimspecialedition/mods/1/files/2", { name: app.game ? app.game.name : "", game: app.game ? app.game.id : "" })
            const since = () => app.nxmShownAt > 0 ? Turbine.monoMs() - app.nxmShownAt : -1
            const ask2 = () => app.nxmAsk("nxm://skyrimspecialedition/mods/1/files/3", { name: app.game ? app.game.name : "", game: app.game ? app.game.id : "" })
            const words = f => f ? "ring on " + (["Get it", "Not now"][f.ring] || "nothing") + ", A " + f.a + ", B " + f.b + ", input " + (f.quiet ? "off" : "ON") : "nothing read"
            script = [
                () => { if (!app.game || !app.modsLoaded) return false; app.sheet = ""; app.page = "game"; app.tab = 0; app.zone = "main"; ask() },
                () => { if (app.nxmShownAt === 0) return false; const f = app.nxmAskFirst || {}
                        pad("a"); check("Guard at first: A " + f.a + ", B " + f.b + ", input " + (f.quiet ? "off" : "ON") + "; A at " + Math.round(since()) + " ms: " + (app.nxmAsking ? "ignored" : "TAKEN")) },
                // A press made 0.5 s after the question showed, handled 0.9 s later (a stalled window): still ignored.
                () => { if (since() < 1400) return false
                        Turbine.selftestPadAged("a", Math.round(since() - 500))
                        check("Guard late press: made at 500 ms, handled at " + Math.round(since()) + " ms: " + (app.nxmAsking ? "ignored" : "TAKEN")) },
                () => { const o = app.binds(); check("Guard after the quiet second: ring on " + (app.pickItems[app.menuIndex] ? app.pickItems[app.menuIndex].title : "") + ", A " + (o.a ? o.a.label : "")); pad("b") },
                () => { if (app.sheet !== "") return false; ask() },
                () => { if (since() < 1050) return false; pad("right"); pad("left") },
                () => { if (since() < 1700) return false; const o = app.binds()
                        check("Guard Left at about 1 s: ring on " + (app.pickItems[app.menuIndex] ? app.pickItems[app.menuIndex].title : "") + " at " + Math.round(since()) + " ms, A " + (o.a ? o.a.label : ""))
                        pad("a"); check("Guard A then: " + (app.nxmAsking ? "still asking" : app.sheet === "" && app.nxmAskLink === "" ? "closed, nothing downloaded" : "other")) },
                // An A made at 1.1 s (ring still on Not now) but handled after the ring moved: Not now.
                () => { if (app.sheet !== "") return false; ask() },
                () => { if (since() < 1400) return false
                        m.gets = app.nxmGets; m.ring = app.pickItems[app.menuIndex] ? app.pickItems[app.menuIndex].title : ""
                        Turbine.selftestPadAged("a", Math.round(since() - 1100))
                        check("Guard stalled A: made at 1100 ms, ring on " + m.ring + ", handled at " + Math.round(since()) + " ms: " + (app.nxmAsking ? "still asking" : app.nxmGets === m.gets && app.sheet === "" ? "Not now, nothing downloaded" : "GOT IT")) },
                // Asked twice in a row and with names Nexus hasn't said yet (a file never asked about), read in
                // the very turn it opens and again at its first frame: on Not now, A and B Not now, input off, each
                // time. A second link while it asks is dropped and changes nothing; the names coming late change the
                // title only.
                () => { if (app.sheet !== "") return false
                        m.n = app.nxmAsks; ask2(); m.one = words(app.nxmAskFirst); m.title = app.pickTitle
                        ask2(); check("Guard twice, the first: " + m.title + ": " + m.one + "; a second link at once: " + words(app.nxmAskState()) + ", " + (app.nxmAsks === m.n + 1 ? "dropped" : "ASKED AGAIN")) },
                () => { if (app.nxmShownAt === 0) return false
                        check("Guard twice, at its first frame: " + words(app.nxmAskFrame))
                        m.ring = app.menuIndex; m.at = app.nxmShownAt
                        app.nxmNameGot({ link: app.nxmAskLink, name: "SkyUI", file: "SkyUI 5.2SE" }, false)
                        check("Guard names late: " + app.pickTitle + ", ring " + (app.menuIndex === m.ring ? "unchanged" : "MOVED") + ", the quiet second " + (app.nxmShownAt === m.at ? "unchanged" : "RESTARTED"))
                        Turbine.selftestPadAged("a", Math.max(0, Math.round(since() - 300)))
                        check("Guard names late, A made at 300 ms: " + (app.nxmAsking ? "ignored" : "TAKEN")) },
                () => { if (since() < 1050) return false; pad("left"); pad("b") },
                // Straight after it closed, the same link again (no wait between the two questions).
                () => { if (app.sheet !== "") return false
                        m.gets2 = app.nxmGets; m.n = app.nxmAsks; ask2(); check("Guard again at once: " + (app.nxmAsks === m.n + 1 ? "asked" : "NOT ASKED") + ": " + words(app.nxmAskFirst)); pad("a")
                        check("Guard again at once, A: " + (app.nxmAsking && app.nxmGets === m.gets2 ? "ignored" : "TAKEN")) },
                () => { if (app.nxmShownAt === 0) return false; check("Guard again, at its first frame: " + words(app.nxmAskFrame)) },
                () => { if (since() < 1050) return false; pad("left"); pad("b") }
            ]
        }
        // Mods off from Steam's menu shows in My mods with Turn them back on, by A and by a tap on the row.
        else if (name === "heldcheck") script = [
            () => { if (app.mods.length < 2 || !app.modsLoaded) return false; app.sheet = ""; app.page = "game"; app.zone = "main"; app.tab = 0; m.n = app.mods.length; Turbine.quickMods(app.game.id, false) },
            () => { if (app.heldOff.length < m.n) { check("Held wait: " + app.heldOff.length + " held, " + app.mods.filter(x => x.enabled).length + " of " + app.mods.length + " on"); return false }
                    app.modIndex = 0
                    check("Held note: row 1 " + (app.listItems[0] ? app.listItems[0].kind : "none") + " | " + app.heldWords + " | legend A " + (app.legendHints[0] ? app.legendHints[0][1] : "")) },
            () => { pad("a") },
            () => { if (app.heldOff.length > 0 || app.mods.some(x => !x.enabled)) return false
                    check("Held back on by A: " + app.mods.filter(x => x.enabled).length + " of " + app.mods.length + " on, note " + (app.listItems.some(x => x.kind === "held") ? "still there" : "gone")); Turbine.quickMods(app.game.id, false) },
            () => { if (app.heldOff.length < m.n) return false; app.modIndex = 1
                    const it = modList.itemAtIndex(0); if (!it) return false
                    const p = it.mapToItem(null, it.width - app.px(40), it.height / 2); Turbine.selftestPointer(p.x, p.y, true, 0) },
            () => { if (app.heldOff.length > 0 || app.mods.some(x => !x.enabled)) return false
                    check("Held back on by a tap: " + app.mods.filter(x => x.enabled).length + " of " + app.mods.length + " on, note " + (app.listItems.some(x => x.kind === "held") ? "still there" : "gone")); Turbine.pointerHide() }
        ]
        else if (name === "editbutton") script = [
            () => { if (app.mods.length < 2 || !app.modsLoaded) return false; app.sheet = ""; app.page = "game"; app.zone = "main"; app.tab = 0
                    m.with = app.mods.find(x => app.settingsFilesOf(x.id).length > 0); m.without = app.mods.find(x => app.settingsFilesOf(x.id).length === 0)
                    if (!m.with) { check("Edit button: no mod with settings"); return }
                    app.modIndex = app.listItems.findIndex(x => x.kind === "mod" && x.m.id === m.with.id) },
            () => { if (!panelEdit.visible) { if (!app.showModPanel) { check("Edit button: the panel doesn't fit here"); scriptStep = script.length; return } return false }
                    const p = panelEdit.mapToItem(null, panelEdit.width / 2, panelEdit.height / 2); Turbine.selftestPointer(p.x, p.y, true, 0) },
            () => { if (app.sheet !== "settingsedit") return false
                    check("Edit button tapped: sheet settingsedit for " + (app.settingsMod === m.with.id ? m.with.name : "another mod (" + app.settingsMod + ")")); Turbine.pointerHide(); pad("b") },
            () => { if (app.sheet !== "") return false; app.modIndex = app.listItems.findIndex(x => x.kind === "mod" && x.m.id === m.with.id); app.zone = "main"; pad("right"); check("Edit button by Right: zone " + app.zone + ", legend A " + (app.legendHints[0] ? app.legendHints[0][1] : "")); pad("a") },
            () => { if (app.sheet !== "settingsedit") return false; check("Edit button by Right then A: sheet settingsedit for " + (app.settingsMod === m.with.id ? m.with.name : app.settingsMod)); pad("b") },
            () => { if (app.sheet !== "") return false; app.zone = "main"
                    if (m.without) { app.modIndex = app.listItems.findIndex(x => x.kind === "mod" && x.m.id === m.without.id) }
                    check("Edit button for a mod without settings (" + (m.without ? m.without.name : "none") + "): " + (panelEdit.visible ? "PROBLEM: shown" : "none")) }
        ]
        // Every sheet and menu ends above the bottom bar at this Interface size and window: its card, and its
        // last row once scrolled to the end.
        else if (name === "menufit") {
            const cases = [
                ["Mod menu", () => { app.tab = 0; app.zone = "main"; app.modIndex = 0; app.menuIndex = 0; app.sheet = "modmenu" }],
                ["Game menu", () => { app.menuIndex = 0; app.sheet = "gamemenu" }],
                ["Game menu, last row", () => { app.sheet = "gamemenu"; for (let i = 0; i < app.gameMenuItems.length; i++) app.act("down") }],
                ["Add from Downloads", () => { app.openAdd() }],
                ["Loadouts", () => { app.openProfiles() }],
                ["Play options", () => { app.openPlayOptions() }],
                ["Find pop-up", () => { app.openFindPop() }],
                ["Before you play", () => { app.checks = [{ level: "warn", title: "A test warning", detail: "" }, { level: "warn", title: "Another", detail: "" }]; app.checkIndex = 0; app.sheet = "check" }],
                ["Loader", () => { app.loaderFor = ["Lantern Glow SKSE"]; app.openLoader() }],
                ["Delete for good", () => { app.askDelete("check:none", "Test") }],
                ["Activity log", () => { app.openActivity() }]
            ]
            let steps = [() => { if (app.mods.length < 2 || !app.modsLoaded) return false; app.page = "game"; app.zone = "main"; app.tab = 0; m.bad = 0; m.n = 0 }]
            for (const cs of cases) {
                steps.push(() => { app.sheet = ""; cs[1]() })
                steps.push(() => {
                    const barTop = legend.visible ? legend.mapToItem(null, 0, 0).y : app.height
                    let worst = -1e9, what = ""
                    const walk = (it, depth) => {
                        if (!it || !it.visible || depth > 3) return
                        for (const ch of it.children) {
                            if (!ch.visible || ch.width <= 0 || ch.height <= 0) continue
                            const b = ch.mapToItem(null, 0, ch.height).y
                            if (b > worst) { worst = b; what = (ch.objectName || ch.toString().split("(")[0]) }
                        }
                    }
                    for (const ov of sheetLayer.children) if (ov.visible) walk(ov, 0)
                    m.n++
                    const ok = worst <= barTop + 1
                    if (!ok) m.bad++
                    check("Menu fit " + cs[0] + ": bottom " + Math.round(worst) + ", bar top " + Math.round(barTop) + (ok ? "" : "  PROBLEM: under the bar (" + what + ")"))
                })
            }
            steps.push(() => { app.sheet = ""; check("Menu fit (" + app.width + "x" + app.height + ", size " + app.sizeNow + "%): " + m.n + " menus, " + m.bad + " problems") })
            script = steps
        }
        else if (name === "sheetcheck") script = [
            () => { if (app.mods.length < 2) return false
                const cases = [
                    ["check", () => { app.checks = [{ level: "warn", title: "A test warning", detail: "" }]; app.checkIndex = 0; app.sheet = "check" }],
                    ["loader", () => { app.loaderFor = ["Lantern Glow SKSE"]; app.openLoader() }],
                    ["chain", () => { app.chainQueue = [{ name: "Test Patch", url: "https://www.nexusmods.com/skyrimspecialedition/mods/1?tab=files" }]; app.infoIndex = 0; app.sheet = "chain" }],
                    ["found", () => { app.found = { path: "/tmp/x.zip", name: "x.zip", size: 1024, game: app.game.id, gameName: app.game.name, why: "test", more: 0 }; app.infoIndex = 0; app.sheet = "found" }],
                    ["notes", () => { app.notesName = "Test"; app.notesList = ["A note"]; app.infoIndex = 0; app.sheet = "notes" }],
                    ["confirm", () => { app.askDelete("all", "Test") }],
                    ["ask", () => { app.openFind() }]
                ]
                app.sheetDry = true
                let problems = 0
                for (const c of cases) {
                    app.sheet = ""; c[1]()
                    let bs = app.buttonSheet()
                    if (!bs) { check("Sheet rule " + c[0] + ": not a button sheet"); problems++; continue }
                    app.act("right")
                    bs = app.buttonSheet()
                    // Start runs nothing on a sheet (a tap of Start never plays).
                    const want = { a: bs.labels[bs.index] || "", b: bs.cancel >= 0 ? bs.labels[bs.cancel] : "", start: "" }
                    let got = {}
                    for (const k of ["a", "b", "start"]) { app.lastDry = ""; app.act(k); got[k] = app.lastDry }
                    const ok = got.a === want.a && got.b === want.b && got.start === want.start
                    if (!ok) problems++
                    check("Sheet rule " + c[0] + ": highlighted " + want.a + ", A ran " + (got.a || "nothing") + ", B ran " + (got.b || "nothing") + ", Start ran " + (got.start || "nothing") + (ok ? "" : "  PROBLEM"))
                }
                app.sheetDry = false; app.found = null; askView.close(); app.sheet = ""
                check("Sheet rule: " + cases.length + " sheets, " + problems + " problems")
            }
        ]
        // Interface size goes smaller by every input: D-pad Left, the - button clicked with the pointer, and A's
        // list (B cancels it). Its buttons are at least 44 pixels.
        else if (name === "sizeinputs") script = [
            () => { app.openSection("Look"); app.zone = "settings"; app.setRow = Math.max(0, app.lookRows.findIndex(r => r.key === "size")); app.sizeTrial = 100 },
            () => { m.row = lookList.itemAtIndex(app.setRow); if (!m.row || !m.row.minus) return false
                    check("Size buttons: - " + Math.round(m.row.minus.width) + " x " + Math.round(m.row.minus.height) + ", + " + Math.round(m.row.plus.width) + " x " + Math.round(m.row.plus.height) + " pixels" + (m.row.minus.width >= 44 && m.row.plus.height >= 44 ? "" : "  PROBLEM: under 44"))
                    m.was = app.sizeNow; pad("left"); check("Size first Left after Settings opened: " + m.was + "% to " + app.sizeNow + "%" + (app.sizeNow === m.was ? ", unchanged" : "  PROBLEM: the first press changed it"))
                    m.was = app.sizeNow; pad("left"); check("Size by D-pad Left: " + m.was + "% to " + app.sizeNow + "%" + (app.sizeNow < m.was ? ", smaller" : "  PROBLEM: not smaller")) },
            () => { m.row = lookList.itemAtIndex(app.setRow); if (!m.row) return false
                    const p = m.row.minus.mapToItem(null, m.row.minus.width / 2, m.row.minus.height / 2)
                    m.was = app.sizeNow; Turbine.selftestPointer(p.x, p.y, true, 0); m.t = Date.now() },
            () => { if (Date.now() - m.t < 400) return false
                    check("Size by the - button (pointer click): " + m.was + "% to " + app.sizeNow + "%" + (app.sizeNow < m.was ? ", smaller" : "  PROBLEM: not smaller")); Turbine.pointerHide() },
            () => { m.row = lookList.itemAtIndex(app.setRow); if (!m.row) return false
                    const p = m.row.plus.mapToItem(null, m.row.plus.width / 2, m.row.plus.height / 2)
                    m.was = app.sizeNow; Turbine.selftestPointer(p.x, p.y, true, 0); m.t = Date.now() },
            () => { if (Date.now() - m.t < 400) return false
                    check("Size by the + button (pointer click): " + m.was + "% to " + app.sizeNow + "%" + (app.sizeNow > m.was ? ", bigger" : "  PROBLEM: not bigger")); Turbine.pointerHide()
                    m.was = app.sizeNow; pad("a") },
            () => { check("Size A: " + (app.sheet === "pick" && app.pickTitle === "Interface size" ? "the list of sizes, on " + (app.pickItems[app.menuIndex] ? app.pickItems[app.menuIndex].title : "?") + ", " + app.pickItems.length + " sizes" : "PROBLEM: no list (sheet \"" + app.sheet + "\")"))
                    pad("up"); pad("b") },
            () => { check("Size list B: " + m.was + "% to " + app.sizeNow + "%" + (app.sizeNow === m.was && app.sheet === "" ? ", unchanged" : "  PROBLEM: changed or still open")); pad("a") },
            () => { pad("up"); pad("up"); pad("a") },
            () => { check("Size by A's list: " + m.was + "% to " + app.sizeNow + "%" + (app.sizeNow < m.was ? ", smaller" : "  PROBLEM: not smaller"))
                    m.was = app.sizeNow; pad("right"); check("Size by D-pad Right: " + m.was + "% to " + app.sizeNow + "%" + (app.sizeNow > m.was ? ", bigger" : "  PROBLEM: not bigger"))
                    app.sizeTrial = 50; check("Size range: " + app.sizeSteps[0] + "% to " + app.sizeSteps[app.sizeSteps.length - 1] + "% in " + app.sizeSteps.length + " steps") },
            () => { check("At 50%: text at least " + app.fpx(10) + " pixels, buttons " + app.tpx(36) + " pixels") }
        ]
        else if (name === "inputcheck") script = [
            () => { if (app.mods.length < 2) return false
                    app.page = "game"; app.zone = "main"; app.tab = 0; app.modIndex = 0; app.sheet = ""
                    m.gid = app.game ? app.game.id : ""
                    m.bad = 0
                    m.ok = (good, text) => { if (!good) m.bad++; check(text + (good ? "" : "  PROBLEM")) } },
            // 1. Holds run only while a real press is held: never from a click, a tap, R3, or a lost release. Each step
            // opens its own Delete for good sheet ("check:" only logs) and counts what finished during it.
            () => { app.askDelete("check:none", "Test") },
            () => { const it = confirmSheet.buttonItem(0); if (!it || !confirmSheet.visible) return false
                    m.d = app.holdsDone; const p = it.mapToItem(null, it.width / 2, it.height / 2); Turbine.selftestPointer(p.x, p.y, true, 0); m.t = Date.now() },
            () => { if (Date.now() - m.t < 2500) return false
                    m.ok(app.holdsDone === m.d && !confirmHold.running && app.sheet === "confirm", "Hold check: a click on Hold A to delete: " + (confirmHold.running || app.holdsDone > m.d ? "the hold ran" : "only the highlight moved"))
                    Turbine.pointerHide(); app.askDelete("check:none", "Test"); m.d = app.holdsDone; pad("r3"); m.t = Date.now() },
            () => { if (Date.now() - m.t < 2500) return false
                    m.ok(app.holdsDone === m.d && !confirmHold.running, "Hold check: R3 on Hold A to delete: " + (confirmHold.running || app.holdsDone > m.d ? "the hold ran" : "no hold"))
                    app.askDelete("check:none", "Test"); Turbine.selftestPad("a", true); m.run = confirmHold.running; app.windowActive(false); app.windowActive(true)
                    m.ok(m.run && !confirmHold.running, "Hold check: A held, then Turbine in the background: " + (m.run ? "the hold started, " : "the hold never started, ") + (confirmHold.running ? "still running" : "stopped"))
                    Turbine.selftestPad("a", false) },
            () => { app.askDelete("check:none", "Test"); Turbine.selftestPad("a", true); m.run = confirmHold.running; Turbine.selftestPad("lost", false)
                    m.ok(m.run && !confirmHold.running, "Hold check: A held, then the controller went away: " + (m.run ? "" : "never started, ") + (confirmHold.running ? "still running" : "stopped"))
                    Turbine.selftestPad("a", false) },
            () => { app.askDelete("check:none", "Test"); Turbine.selftestPad("a", true); m.run = confirmHold.running; app.sheet = "removed"
                    m.ok(m.run && !confirmHold.running, "Hold check: A held, then the sheet changed: " + (m.run ? "" : "never started, ") + (confirmHold.running ? "still running" : "stopped"))
                    Turbine.selftestPad("a", false) },
            () => { app.askDelete("check:none", "Test"); m.d = app.holdsDone; Turbine.selftestPad("a", true); m.t = Date.now() },
            () => { if (Date.now() - m.t < 2600) return false
                    Turbine.selftestPad("a", false)
                    m.ok(app.holdsDone === m.d + 1, "Hold check: A really held for 2 seconds: " + (app.holdsDone === m.d + 1 ? "Delete for good ran" : "nothing ran")) },
            () => { app.sheet = ""; app.sheet = "gamemenu"; m.ri = app.gameMenuItems.findIndex(x => x.act === "reset"); if (m.ri < 0) return false
                    m.d = app.holdsDone; app.menuIndex = m.ri; pad("a"); m.t = Date.now() },
            () => { if (Date.now() - m.t < 1000) return false
                    m.ok(app.holdsDone === m.d && !resetHold.running, "Hold check: a tap of A on Reset game files: " + (resetHold.running ? "still counting" : "stopped at the release"))
                    app.menuIndex = m.ri; Turbine.selftestPad("a", true); m.run = resetHold.running; pad("b")
                    m.ok(m.run && !resetHold.running && app.sheet === "", "Hold check: A held on Reset, then B closed the menu: " + (m.run ? "" : "never started, ") + (resetHold.running ? "still counting" : "stopped"))
                    Turbine.selftestPad("a", false) },
            () => { app.sheet = "gamemenu"; app.menuIndex = m.ri },
            () => { const it = gameMenuView.entryItem(m.ri); if (!it || !gameMenuView.visible) return false
                    m.d = app.holdsDone; const p = it.mapToItem(null, it.width / 2, it.height / 2); Turbine.selftestPointer(p.x, p.y, true, 0); m.t = Date.now() },
            () => { if (Date.now() - m.t < 1000) return false
                    m.ok(app.holdsDone === m.d && !resetHold.running, "Hold check: a click on Reset game files: " + (resetHold.running ? "counting" : "no hold"))
                    Turbine.pointerHide(); app.sheet = "gamemenu"; app.menuIndex = m.ri; pad("r3"); m.t = Date.now() },
            () => { if (Date.now() - m.t < 1000) return false
                    m.ok(app.holdsDone === m.d && !resetHold.running, "Hold check: R3 on Reset game files: " + (resetHold.running ? "counting" : "no hold"))
                    pad("b") },
            // 7. RT and R3 hold the left button each on their own.
            () => { app.sheet = ""; app.probeOn = true; Turbine.pointerShowAt(app.width / 2, app.height / 2); m.pe = app.padEvents
                    Turbine.selftestPad("rt", true); const a = inputProbe.pressed
                    Turbine.selftestPad("r3", true); Turbine.selftestPad("r3", false); const b = inputProbe.pressed
                    Turbine.selftestPad("rt", false); const c = inputProbe.pressed
                    Turbine.selftestPad("r3", true); const d = inputProbe.pressed
                    Turbine.selftestPad("rt", true); Turbine.selftestPad("rt", false); const e = inputProbe.pressed
                    Turbine.selftestPad("r3", false); const f = inputProbe.pressed
                    m.ok(a && b && !c && d && e && !f && app.padEvents === m.pe, "RT and R3: R3 let go while RT holds: " + (b ? "still held" : "dropped") + "; RT let go: " + (c ? "stuck" : "let go")
                         + "; RT let go while R3 holds: " + (e ? "still held" : "dropped") + "; R3 let go: " + (f ? "stuck" : "let go") + "; presses sent on to the interface: " + (app.padEvents - m.pe))
                    app.probeOn = false; Turbine.pointerHide() },
            // 2. A click anywhere on a row is A; rows are at least 44 px.
            () => { app.page = "game"; app.zone = "main"; app.tab = 0; app.modIndex = 0; const it = modList.itemAtIndex(1); if (!it || app.mods.length < 2) return false
                    m.id = app.listItems[1].m.id; m.on = app.mods.find(x => x.id === m.id).enabled
                    const p = it.mapToItem(null, it.width * 0.6, it.height / 2); Turbine.selftestPointer(p.x, p.y, true, 0) },
            () => { const x = app.mods.find(y => y.id === m.id), it = modList.itemAtIndex(1)
                    m.ok(!!x && x.enabled !== m.on && app.modIndex === 1, "Click check: a click on a mod row's name: " + (!!x && x.enabled !== m.on ? "turned it " + (x.enabled ? "on" : "off") : "did nothing") + ", focus on row " + (app.modIndex + 1))
                    m.ok(!!it && it.height >= 44, "Click check: a mod row is " + (it ? Math.round(it.height) : 0) + " px tall")
                    Turbine.pointerHide() },
            () => { if (!app.ordered || !(app.rules && app.rules.plugins)) return
                    app.tab = 2; app.zone = "main"; if (app.order.length === 0) return false
                    m.pi = app.order.findIndex(x => !x.fixed); if (m.pi < 0) return
                    m.pa = app.order[m.pi].active; const it = loadOrderView.rowItem(m.pi); if (!it) return false
                    const p = it.mapToItem(null, it.width * 0.7, it.height / 2); Turbine.selftestPointer(p.x, p.y, true, 0) },
            () => { if (app.tab !== 2 || m.pi === undefined || m.pi < 0) return
                    const it = loadOrderView.rowItem(m.pi)
                    m.ok(app.order[m.pi] && app.order[m.pi].active !== m.pa, "Click check: a click on a load order row: " + (app.order[m.pi] && app.order[m.pi].active !== m.pa ? "turned it " + (app.order[m.pi].active ? "on" : "off") : "did nothing"))
                    m.ok(!!it && it.height >= 44, "Click check: a load order row is " + (it ? Math.round(it.height) : 0) + " px tall")
                    Turbine.pointerHide(); app.tab = 0 },
            () => { app.openSettings(); app.setSection = app.settingsSections.indexOf("Look"); m.row = app.lookRows.findIndex(r => r.key === "theme"); m.theme = look.theme },
            // (The next look: the Settings row is laid out where it shows.)
            () => { const it = lookList.itemAtIndex(m.row); if (!it || !lookList.visible) return false
                    const p = it.mapToItem(null, app.px(40), it.height / 2); Turbine.selftestPointer(p.x, p.y, true, 0) },
            () => { const it = lookList.itemAtIndex(m.row)
                    m.ok(look.theme !== m.theme && app.setRow === m.row, "Click check: a click on a Settings choice row: " + (look.theme !== m.theme ? "changed it" : "did nothing"))
                    m.ok(!!it && it.height >= 44, "Click check: a Settings row is " + (it ? Math.round(it.height) : 0) + " px tall")
                    look.theme = m.theme; Turbine.pointerHide(); app.settingsBack(); app.zone = "main"; app.tab = 0 },
            // 3. The removed row never sends a move or an A to the wrong mod.
            () => { if (app.mods.length < 2) return false; app.modIndex = 0; m.goneId = app.mods[0].id; app.uninstall(app.mods[0]) },
            () => { if (!app.ghost) return false; app.modIndex = 1; m.want = app.focusedItem && app.focusedItem.m ? app.focusedItem.m.id : ""; app.startMove()
                    const now = app.moving && app.mods[app.modIndex] ? app.mods[app.modIndex] : null
                    m.ok(!!now && now.id === m.want && !app.ghost, "Ghost check: Move with the removed row showing moves " + (now ? now.name : "nothing") + (now && now.id === m.want ? ", the focused mod" : ", not the focused mod"))
                    app.cancelMove(); app.putBack(m.goneId) },
            () => { if (!app.mods.some(x => x.id === m.goneId)) return false; app.modIndex = app.mods.findIndex(x => x.id === m.goneId); app.uninstall(app.mods[app.modIndex]) },
            () => { if (!app.ghost) return false; app.modIndex = app.ghost.index; ghostTimer.stop(); app.ghostExpired()
                    m.next = app.focusedItem && app.focusedItem.kind === "mod" ? app.focusedItem.m.id : ""; m.nextOn = m.next ? app.mods.find(x => x.id === m.next).enabled : false
                    pad("a") },
            () => { if (!app.mods.some(x => x.id === m.goneId)) return false
                    const nx = app.mods.find(x => x.id === m.next)
                    m.ok(!nx || nx.enabled === m.nextOn, "Ghost check: A just as the removed row went: put back, the mod that slid up " + (nx && nx.enabled !== m.nextOn ? "changed too" : "untouched")) },
            // 10. A sheet that comes up during the tutorial waits for it.
            () => { app.openTutorial(0); app.openNotes("Tutorial check", ["A note"])
                    m.ok(app.sheet === "tutorial" && app.pendingNotes, "Tutorial check: the install notes sheet " + (app.sheet === "tutorial" ? "waits for the tutorial" : "replaced it"))
                    app.tutorialAct("b")
                    m.ok(app.sheet === "notes", "Tutorial check: after the tutorial: sheet \"" + app.sheet + "\""); app.sheet = "" },
            // 11. Every game hidden: the rail offers them back.
            () => { app.hiddenTest = app.allGames.map(g => g.id); app.orderGames(); app.zone = "rail"; app.railIndex = 0
                    const o = app.binds()
                    m.ok(app.games.length === 0 && app.railExtra === 1 && !!o.a && o.a.label === "Show hidden games", "Hidden check: every game hidden, the rail shows " + (app.railExtra === 1 ? "Show hidden games" : "nothing to bring them back"))
                    pad("a") },
            () => { m.ok(app.games.length > 0, "Hidden check: Show hidden games put back " + app.plural(app.games.length, "game", "games"))
                    app.hiddenTest = null; app.orderGames() },
            // 11b. Confirm by pressing twice: the sheet's button says "Press A again", no toast repeats it.
            () => { m.cb = look.confirmBy; look.confirmBy = "twice"; toast.hide(); app.askDelete("check:none", "Test") },
            () => { if (app.sheet !== "confirm") return false
                    pad("a")
                    m.ok(app.sheet === "confirm" && !(toast.shown && toast.text.indexOf("Press A again") === 0), "Press twice check: first A on Delete for good: " + (toast.shown && toast.text.indexOf("Press A again") === 0 ? "a toast repeats it" : "the button says it, no toast"))
                    app.disarm(); app.sheet = ""; look.confirmBy = m.cb },
            // 11c. Add from Downloads' Y is Find; its first row chooses which files show.
            () => { if (!app.game) return false; app.sheet = ""; app.openAdd() },
            () => { if (app.sheet !== "add") return false
                    const o = app.binds(); m.addY = o.y ? o.y.label : ""; pad("y") },
            () => { if (app.sheet !== "pick") return false
                    m.addRows = app.pickItems.map(x => x.title).join(", "); pad("down"); pad("a") },
            () => { if (app.sheet !== "add") return false
                    m.ok(m.addY === "Find" && m.addRows === "This game's files, All files" && app.addAll, "Find check: Add from Downloads Y " + m.addY + " (" + m.addRows + "), All files " + (app.addAll ? "showing" : "not showing"))
                    pad("b") },
            // 12. Y is Find even while a toast offers Undo; R3 runs the toast's Undo.
            // The hidden-games check above can leave another game selected: go back to the one with the two mods.
            () => { if (!app.game) return false
                    if (m.gid && app.game.id !== m.gid) { const gi = app.games.findIndex(x => x.id === m.gid); if (gi >= 0) app.railSelect(gi); return false }
                    if (app.mods.length === 0) return false
                    app.page = "game"; app.zone = "main"; app.tab = 0; app.modIndex = 0; app.sheet = ""; Turbine.pointerHide()
                    if (!app.mods.some(x => x.enabled)) { Turbine.setEnabled(app.mods[0].id, true); app.mods = app.mods.map((x, i) => i === 0 ? Object.assign({}, x, { enabled: true }) : x) }
                    m.wasOn = app.mods.filter(x => x.enabled).length
                    app.allOff()
                    m.ok(!!app.toastAction && toast.actionGlyph === app.glyph("r3"), "Toast check: Turn every mod off shows " + toast.actionGlyph + " " + toast.actionLabel)
                    m.ok(app.legendHints.length > 2 && app.legendHints[2][0] === "r3" && app.legendHints[2][1] === "Undo", "Toast check: the legend while it shows: " + app.legendHints.map(h => h[0].toUpperCase() + " " + h[1]).join(" · "))
                    pad("y") },
            () => { m.ok(app.sheet === "findpop" && !!app.toastAction, "Toast check: Y with Undo showing: " + (app.sheet === "findpop" ? "Find opened" : "sheet \"" + app.sheet + "\"") + (app.toastAction ? ", Undo kept" : ", Undo gone"))
                    pad("b") },
            () => { if (app.sheet !== "") return false
                    pad("r3")
                    const on = app.mods.filter(x => x.enabled).length
                    m.ok(on === m.wasOn && app.sheet === "", "Toast check: R3 with Undo showing: " + app.plural(on, "mod", "mods") + " back on of " + m.wasOn)
                    m.ok(!app.legendHints.some(h => h[0] === "r3"), "Toast check: the legend after it: " + app.legendHints.map(h => h[0].toUpperCase() + " " + h[1]).join(" · "))
                    check("Input check: " + m.bad + " problems") }
        ]
        else if (name === "nexusguard") script = [
            () => { if (app.getItems.length === 0) return false; app.openDetails(app.getItems[0]) },
            () => { if (!app.detailsData || app.detailsData.loading || app.mainLabel !== "Get it on Nexus") return false; m.before = app.launchCount
                    pad("a"); pad("a"); pad("a") },
            () => { check("Nexus guard: 3 presses of A, " + (app.launchCount - m.before) + " browser launch" + (app.launchCount - m.before === 1 ? "" : "es") + ", button \"" + app.mainLabel + "\"") }
        ]
        // Start. A tap opens Settings and a tap or B goes back; a hold let go early does nothing; a 3 second
        // hold shows the Play choice (or plays, by the setting); A on Play plays; Ctrl+Enter is the hold; Start on a
        // menu never plays; a tap on the bar's Start hint is a tap of Start. All by real presses, keys and clicks.
        else if (name === "startcheck") {
            m.bad = 0
            const say = (ok, text) => { if (!ok) m.bad++; check("Start check: " + text + (ok ? "" : "  PROBLEM")) }
            const holdRow = () => app.lookRows.findIndex(r => r.key === "holdStartDoes")
            script = [
                () => { if (app.mods.length < 2 || !app.modsLoaded) return false; app.sheet = ""; app.page = "game"; app.zone = "main"; app.tab = 0; app.modIndex = 0; m.asks = app.playAsks },
                () => { pad("start"); say(app.page === "settings" && app.playAsks === m.asks, "a tap opens " + (app.page === "settings" ? "Settings" : app.page) + ", legend " + app.legendHints.map(h => h[0].toUpperCase() + " " + h[1]).join(" · ")) },
                () => { pad("start"); say(app.page === "game" && app.zone === "main", "a tap again goes back to " + (app.page === "game" ? "My mods" : app.page)) },
                () => { pad("start"); m.was = app.page; pad("b"); say(m.was === "settings" && app.page === "game", "B from Settings goes back to " + (app.page === "game" ? "My mods" : app.page)) },
                // A hold let go at 1.5 s: the ring was filling, and nothing happens.
                () => { m.t = Date.now(); Turbine.selftestPad("start", true) },
                () => { if (Date.now() - m.t < 1500) return false
                        m.part = app.startHeld; m.ring = startRing.visible
                        Turbine.selftestPad("start", false)
                        say(m.ring && m.part > 0.3 && m.part < 0.9 && app.page === "game" && app.sheet === "" && app.playAsks === m.asks && app.startHeld === 0,
                            "let go at 1.5 s: the ring was " + (m.ring ? "showing" : "hidden") + " at " + Math.round(m.part * 100) + "%, then page " + app.page + ", sheet \"" + app.sheet + "\", Play asked " + (app.playAsks - m.asks) + " times") },
                // Held for the 3 seconds: the Play choice, not before.
                () => { m.t = Date.now(); Turbine.selftestPad("start", true) },
                () => { if (Date.now() - m.t < 2300) return false; m.early = app.sheet },
                () => { if (app.sheet !== "pick" && Date.now() - m.t < 6000) return false
                        const secs = (Date.now() - m.t) / 1000
                        Turbine.selftestPad("start", false)
                        say(m.early === "" && app.sheet === "pick" && app.pickTitle === "Play options" && app.playAsks === m.asks && secs >= 2.9,
                            "held 3 seconds: nothing at 2.3 s (sheet \"" + m.early + "\"), then " + (app.sheet === "pick" ? app.pickTitle : "sheet \"" + app.sheet + "\"") + ", focus " + (app.pickItems[app.menuIndex] ? app.pickItems[app.menuIndex].title : "none") + ", Play asked " + (app.playAsks - m.asks) + " times")
                        pad("b") },
                () => { say(app.sheet === "" && app.page === "game", "B closes the Play choice: sheet \"" + app.sheet + "\"") },
                // The setting, changed with A on its row in Settings, Look.
                () => { app.openSection("Look"); app.zone = "settings"; app.setRow = holdRow(); m.words = app.startHoldWords; pad("a") },
                () => { say(holdRow() >= 0 && look.holdStartDoes === "last" && app.startHoldWords === "hold: Play", "Hold Start does: row " + (holdRow() >= 0 ? "there" : "missing") + ", A changed it to " + look.holdStartDoes + ", the bar says " + m.words + " then " + app.startHoldWords)
                        pad("b") },
                () => { if (app.page !== "game") return false; app.zone = "main"; m.asks = app.playAsks; m.t = Date.now(); Turbine.selftestPad("start", true) },
                () => { if (app.playAsks === m.asks && Date.now() - m.t < 6000) return false
                        const secs = (Date.now() - m.t) / 1000
                        Turbine.selftestPad("start", false)
                        say(app.playAsks === m.asks + 1 && secs >= 2.9, "Play my last choice: held 3 seconds, Play asked " + (app.playAsks - m.asks) + " times") },
                // A on the bar's Play button still plays.
                () => { if (Turbine.busy) return false; app.sheet = ""; app.page = "game"; app.zone = "play"; m.asks = app.playAsks; pad("a") },
                () => { say(app.playAsks === m.asks + 1, "A on Play: Play asked " + (app.playAsks - m.asks) + " times") },
                // Back to the Play choice, by A on the row again; Ctrl+Enter is the keyboard's hold.
                () => { if (Turbine.busy) return false; app.sheet = ""; app.openSection("Look"); app.zone = "settings"; app.setRow = holdRow(); pad("a") },
                () => { pad("b"); app.sheet = ""; app.zone = "main"; m.asks = app.playAsks
                        Turbine.selftestKey(Qt.Key_Return, Qt.ControlModifier, true); Turbine.selftestKey(Qt.Key_Return, Qt.ControlModifier, false) },
                () => { say(look.holdStartDoes === "choice" && app.sheet === "pick" && app.pickTitle === "Play options" && app.playAsks === m.asks, "Ctrl+Enter: " + (app.sheet === "pick" ? app.pickTitle : "sheet \"" + app.sheet + "\"") + ", setting " + look.holdStartDoes)
                        Turbine.selftestKey(Qt.Key_Escape, 0, true); Turbine.selftestKey(Qt.Key_Escape, 0, false) },
                // Start on a menu never plays, and never opens Settings under it.
                () => { say(app.sheet === "", "Esc closes the Play choice: sheet \"" + app.sheet + "\""); app.sheet = ""; app.zone = "main"; app.tab = 0; app.modIndex = 0; pad("x") },
                () => { m.sheet = app.sheet; m.asks = app.playAsks; pad("start")
                        say(m.sheet === "modmenu" && app.sheet === "modmenu" && app.page === "game" && app.playAsks === m.asks, "Start on the mod menu: sheet \"" + app.sheet + "\", page " + app.page + ", Play asked " + (app.playAsks - m.asks) + " times, legend " + app.legendHints.map(h => h[0].toUpperCase() + " " + h[1]).join(" · "))
                        pad("b") },
                // The bar: the Start hint is a button for a finger or a click, and nothing overlaps.
                () => { if (app.sheet !== "") return false
                        const p = startHint.mapToItem(null, startHint.width / 2, startHint.height / 2)
                        m.hint = startHint.visible; m.h = Math.round(startHint.height)
                        Turbine.selftestMouse(0, p.x, p.y, 0, 0, 0); Turbine.selftestMouse(1, p.x, p.y, Qt.LeftButton, 0, 0); Turbine.selftestMouse(2, p.x, p.y, Qt.LeftButton, 0, 0) },
                () => { say(m.hint && m.h >= 44 && app.page === "settings", "a click on the Start hint (" + m.h + " px tall): " + (app.page === "settings" ? "Settings" : app.page)); pad("start") },
                () => { say(app.page === "game", "and Start goes back: " + app.page); app.zone = "main"; legend.check("startcheck")
                        check("Start check done: " + m.bad + " problems") }
            ]
        }
        // Get mods as a page of a book, by real presses and clicks: no game list there, the edges name the pages
        // beside it, Left from the first column and Right from the last turn the page (a held D-pad stops at the
        // edge), LB and RB and a click on an edge turn it too, and the page coming in slides in as the other fades.
        else if (name === "bookcheck") {
            m.bad = 0
            const say = (ok, text) => { if (!ok) m.bad++; check("Book check: " + text + (ok ? "" : "  PROBLEM")) }
            const tabName = () => (app.tabs[app.tab] || "").replace(/ \(\d+\)$/, "")
            const click = it => { const p = it.mapToItem(null, it.width / 2, it.height / 2)
                                  Turbine.selftestMouse(0, p.x, p.y, 0, 0, 0); Turbine.selftestMouse(1, p.x, p.y, Qt.LeftButton, 0, 0); Turbine.selftestMouse(2, p.x, p.y, Qt.LeftButton, 0, 0) }
            script = [
                () => { if (!app.game || !app.modsLoaded || app.mods.length < 1) return false; app.sheet = ""; app.page = "game"; app.zone = "main"; app.tab = 0; app.modIndex = 0 },
                () => { say(rail.width > 0 && !edgeBack.visible, "My mods: the game list is " + (rail.width > 0 ? "there" : "gone") + ", page edges " + (edgeBack.visible ? "shown" : "none")); m.turns = app.turnsShown; pad("rb")
                        say(app.tab === 1 && app.lastTurnDir === 1 && (mo.moves ? app.turnTo === 1 && app.turnInMove < 1 && app.turnsShown === m.turns + 1 : app.turnTo < 0), "RB: " + tabName() + " comes in from the " + (app.lastTurnDir > 0 ? "right" : "left")) },
                () => { if (app.getItems.length < app.gridColumns || app.turnTo >= 0 || rail.width > 0) return false
                        say(rail.width === 0 && edgeBack.visible && edgeBack.width >= 44 && edgeNext.visible === app.bookHasNext && app.bookHasNext,
                            "Get mods: the game list is " + (rail.width === 0 ? "gone" : "there") + ", left edge " + (edgeBack.visible ? edgeBack.label : "none") + " (" + Math.round(edgeBack.width) + " px), right edge " + (edgeNext.visible ? edgeNext.label : "none"))
                        app.zone = "main"; app.getArea = "grid"; app.getIndex = 1; pad("left")
                        say(app.tab === 1 && app.getIndex === 0, "Left inside the row: card " + (app.getIndex + 1) + ", still " + tabName())
                        pad("left")
                        say(app.tab === 0 && app.zone === "main" && app.lastTurnDir === -1 && modList.visible && (mo.moves ? app.turnTo === 0 && getView.visible && app.turnInMove < 1 : !getView.visible),
                            "Left from the first column: " + tabName() + " comes in from the " + (app.lastTurnDir < 0 ? "left" : "right") + ", zone " + app.zone) },
                () => { if (app.turnTo >= 0 || rail.width < app.px(72)) return false
                        say(app.tab === 0 && !getView.visible && rail.width > 0, "The turn done: " + tabName() + ", Get mods " + (getView.visible ? "still showing" : "put away") + ", the game list back"); pad("rb") },
                () => { if (app.turnTo >= 0 || app.tab !== 1) return false
                        app.getArea = "grid"; app.getIndex = 0; pad("right")
                        say(app.tab === 1 && app.getIndex === 1, "Right inside the row: card " + (app.getIndex + 1) + ", still " + tabName())
                        app.getIndex = app.gridColumns - 1; pad("right")
                        say(app.tab === 2 && app.lastTurnDir === 1, "Right from the last column: " + tabName() + " comes in from the " + (app.lastTurnDir > 0 ? "right" : "left")) },
                () => { if (app.turnTo >= 0) return false; pad("lb"); say(app.tab === 1 && app.lastTurnDir === -1, "LB from Load order: " + tabName() + " comes in from the " + (app.lastTurnDir < 0 ? "left" : "right")) },
                // A held D-pad runs to the first column and stops there: the page stays.
                () => { if (app.turnTo >= 0) return false; app.getArea = "grid"; app.getIndex = app.gridColumns - 1; m.t = Date.now(); Turbine.selftestPad("left", true) },
                () => { if (Date.now() - m.t < 1500) return false; Turbine.selftestPad("left", false)
                        say(app.tab === 1 && app.getIndex === 0, "Left held 1.5 s: card " + (app.getIndex + 1) + ", " + (app.tab === 1 ? "the page stayed" : "the page turned to " + tabName())) },
                // The edges are buttons.
                () => { click(edgeBack) },
                () => { if (app.turnTo >= 0) return false; say(app.tab === 0, "A click on the left edge: " + tabName()); pad("rb") },
                () => { if (app.turnTo >= 0 || app.tab !== 1) return false; click(edgeNext) },
                () => { if (app.turnTo >= 0) return false; say(app.tab === 2, "A click on the right edge: " + tabName()); pad("lb") },
                () => { if (app.turnTo >= 0 || app.tab !== 1 || rail.width > 0) return false; app.getArea = "grid"; app.getIndex = 0; app.zone = "main"
                        legend.check("bookcheck")
                        say(app.legendHints.some(h => h[0] === "b" && h[1] === "My mods"), "The bar on Get mods: " + app.legendHints.map(h => h[0].toUpperCase() + " " + h[1]).join(" · "))
                        check("Book check done: " + m.bad + " problems, animations " + mo.level + ", " + app.turnsShown + " page turns shown, " + app.glides + " glides") }
            ]
        }
        // Seamless Co-op started with an empty session password and the mod stopped with
        // an error. Every way a co-op start comes, with the password empty: nothing starts, Co-op settings open on
        // the password with the keyboard, and the start waits for it.
        else if (name === "cooppw") {
            m.bad = 0
            const say = (ok, text) => { if (!ok) m.bad++; check("Co-op password check: " + text + (ok ? "" : "  PROBLEM")) }
            const stopped = () => app.sheet === "ask" && app.askReturn === "settingsedit" && app.coopAfterSettings && app.playAsks === m.asks
            const focusKey = () => { const r = settingsView.shown[settingsView.focusIndex]; return r && r.f ? r.f.key : "none" }
            const told = how => say(stopped() && focusKey() === "cooppassword", how + ": " + (app.playAsks === m.asks ? "nothing started" : "Play asked " + (app.playAsks - m.asks) + " times") + ", sheet " + app.askReturn + " with the keyboard (" + app.sheet + "), focus " + focusKey() + ", waiting " + app.coopAfterSettings)
            const waitAsk = () => app.sheet === "ask" || Date.now() - m.t > 6500
            // B on the keyboard sheet, then B on the settings: back on the game, nothing waiting, nothing started.
            const close1 = () => { if (app.sheet === "ask") pad("b") }
            const close2 = how => () => { if (app.sheet === "settingsedit") pad("b")
                                          say(app.sheet === "" && !app.coopAfterSettings && app.playAsks === m.asks, how + ", then B and B: sheet \"" + app.sheet + "\", waiting " + app.coopAfterSettings + ", Play asked " + (app.playAsks - m.asks) + " times") }
            script = [
                () => { if (!app.seamlessIn || !app.modsLoaded) return false; app.sheet = ""; app.page = "game"; app.zone = "main"; app.tab = 0; app.acAnswered = ({ "steam:1245620": true }); m.asks = app.playAsks
                        say(Turbine.coopPasswordState(app.seamlessMod.id) === 0, "at the start the password is " + ["unknown", "empty", "set"][Turbine.coopPasswordState(app.seamlessMod.id) + 1])
                        // This copy came from Nexus, not Turbine's own download: it counts just the same.
                        say(app.seamlessMod.seamless === true && app.seamlessMod.sourceKind === "nexus" && app.isSeamless(app.seamlessMod) && app.rowLine(app.seamlessMod).text === "Only loads in Co-op mode" && app.settingsFilesOf(app.seamlessMod.id).length > 0,
                            "Seamless Co-op from " + app.seamlessMod.sourceKind + ": marked " + (app.seamlessMod.seamless === true ? "seamless" : "not seamless") + ", its row says " + app.rowLine(app.seamlessMod).text + ", settings files " + app.settingsFilesOf(app.seamlessMod.id).length) },
                // 1: the Play choice (A on Play, then Play Seamless Co-op).
                () => { app.zone = "play"; pad("a") },
                () => { if (app.sheet !== "pick") return false; app.menuIndex = app.pickItems.findIndex(x => x.key === "coop"); m.t = Date.now(); pad("a") },
                () => { if (!waitAsk()) return false; told("The Play choice") },
                close1, close2("The Play choice"),
                // 2: Start held 3 seconds with "Play my last choice" (the last choice is co-op now).
                () => { look.holdStartDoes = "last"; app.zone = "main"; m.t = Date.now(); Turbine.selftestPad("start", true) },
                () => { if (app.sheet !== "ask" && Date.now() - m.t < 6500) return false; Turbine.selftestPad("start", false); told("Start held (last choice " + app.lastPlayChoice + ")"); look.holdStartDoes = "choice" },
                close1, close2("Start held"),
                // 3: Play options (the quick wheel), Play Seamless Co-op.
                () => { app.zone = "main"; pad("view"); pad("down"); pad("right"); m.slot = app.wheelSlots[app.wheelSlot] ? app.wheelSlots[app.wheelSlot].label : ""; pad("a") },
                () => { if (app.sheet !== "pick") return false; app.menuIndex = app.pickItems.findIndex(x => x.title === "Play Seamless Co-op"); m.t = Date.now(); pad("a") },
                () => { if (!waitAsk()) return false; told("Play options (wheel slot " + m.slot + ")") },
                close1, close2("Play options"),
                // 4: the Co-op tile and Steam's menu, when Turbine is open: their request comes in as this line.
                () => { m.t = Date.now(); app.linkArrived("turbine:quick play-coop " + encodeURIComponent(app.game.id)) },
                () => { if (!waitAsk()) return false; told("The Co-op tile or Steam's menu") },
                close1, close2("The tile"),
                // 5: a start that got as far as the Play job itself: it stops there too, and says so.
                () => { m.stops = app.coopStops; m.t = Date.now(); Turbine.playCoop() },
                () => { if (!waitAsk()) return false; say(app.coopStops === m.stops + 1, "The Play job stopped it itself: " + (app.coopStops - m.stops) + " times"); told("The Play job") },
                // 6: a pair of quotes with nothing in it is still empty: typed in, nothing starts.
                () => { askView.open("Session password", "", "\"\"", "", 2000, false, "Done"); askView.accept(); m.t = Date.now() },
                () => { if (Date.now() - m.t < 1800) return false
                        say(app.playAsks === m.asks && Turbine.coopPasswordState(app.seamlessMod.id) === 0, "A password of two quotes: " + (app.playAsks === m.asks ? "nothing started" : "Play asked") + ", the password counts as " + ["unknown", "empty", "set"][Turbine.coopPasswordState(app.seamlessMod.id) + 1])
                        if (app.sheet === "ask") pad("b") },
                () => { if (app.sheet === "settingsedit") pad("b") },
                // Two copies of Seamless Co-op. First the engine's own look, through the bridge and back.
                () => { if (app.sheet !== "") return false; m.looks = app.coopLooks; app.coopFound = null; app.coopLook() },
                () => { if (app.coopLooks === m.looks && !app.coopFound) return false
                        say(!!app.coopFound && app.coopFound.installed === true && (app.coopFound.others || []).length === 0 && app.sheet === "", "the engine's look at Seamless Co-op: " + (app.coopFound && app.coopFound.installed ? "installed (" + app.coopFound.name + " " + app.coopFound.listVersion + ")" : "not found") + ", " + (app.coopFound ? (app.coopFound.others || []).length : "?") + " other copies, sheet \"" + app.sheet + "\"")
                        // Then an answer with a second copy and a version that disagrees: the one sheet, and the toast.
                        app.coopTidyAsked = ({}); app.coopVersionSaid = ({})
                        m.two = { installed: true, name: "Seamless Co-op", listVersion: "2.0.1", fileVersion: "1.9.0", others: ["Seamless Co-op 1.9.1"], tidied: [],
                                  versionWarning: "Your list says Seamless Co-op 2.0.1, but its file says it is 1.9.0. The mod doesn't run when it is out of date: get it again from its page if the game says so." }
                        app.coopLooked(app.game.id, m.two) },
                () => { say(app.sheet === "msg" && !!app.msgData && app.msgData.title === "You have two copies of Seamless Co-op" && /2\.0\.1.*1\.9\.0/.test(app.lastToast),
                            "two copies, on opening the game: " + (app.sheet === "msg" && app.msgData ? app.msgData.text + " | " + app.msgData.buttons.map(b => b.label).join("; ") : "sheet \"" + app.sheet + "\"") + " | toast: " + app.lastToast.substring(0, 70))
                        pad("b") },
                // Before a co-op start: the same sheet first; Tidy up, and the start goes on (to the password here).
                () => { say(app.sheet === "" && app.coopCopies().length === 0, "Not now: sheet \"" + app.sheet + "\"")
                        app.coopLooked(app.game.id, m.two); m.auto = app.sheet; m.asks = app.playAsks; app.zone = "play"; pad("a") },
                () => { if (app.sheet !== "pick") return false; app.menuIndex = app.pickItems.findIndex(x => x.key === "coop"); pad("a") },
                () => { say(m.auto === "" && app.sheet === "msg" && !!app.msgData && app.msgData.title === "You have two copies of Seamless Co-op" && app.playAsks === m.asks,
                            "two copies, before a co-op start: asked again by itself: " + (m.auto === "" ? "no" : "yes") + "; at the start: " + (app.sheet === "msg" && app.msgData ? app.msgData.title : "sheet \"" + app.sheet + "\"") + ", Play asked " + (app.playAsks - m.asks) + " times")
                        m.t = Date.now(); pad("a") },
                () => { if (app.sheet !== "ask" && Date.now() - m.t < 6500) return false
                        say(app.sheet === "ask" && app.coopAfterSettings && app.playAsks === m.asks, "Tidy up, then the start goes on: " + (app.sheet === "ask" ? "to the session password (still empty)" : "sheet \"" + app.sheet + "\""))
                        if (app.sheet === "ask") pad("b") },
                () => { if (app.sheet === "settingsedit") pad("b"); check("Co-op password check done: " + m.bad + " problems") }
            ]
        }
        // And the keys a PC player expects: W A S D, Q and E, F, R, Esc for Settings on the first page, Tab
        // held for the quick wheel, Space held for Play, letters left to a text field being typed in; the mouse's
        // Forward button, a double-click on a mod, and no key caps after the mouse.
        // The keyboard and the mouse drive every screen, by real key presses and clicks: arrows, Enter, Esc
        // and Backspace, Tab and Shift+Tab, Page Down and Page Up, Ctrl+F and /, the Menu key and Shift+F10, F1,
        // Ctrl+= and Ctrl+-, P, V, Ctrl+Enter, Ctrl+Z; hover, click, right-click, the Back button, Ctrl with the
        // wheel; and the bar's hints follow the last input used.
        else if (name === "keycheck") {
            m.bad = 0
            const say = (ok, text) => { if (!ok) m.bad++; check("Key check: " + text + (ok ? "" : "  PROBLEM")) }
            const K = (k, mods) => { Turbine.selftestKey(k, mods || 0, true); Turbine.selftestKey(k, mods || 0, false) }
            const at = (it, fx) => it.mapToItem(null, it.width * (fx || 0.5), it.height / 2)
            const mouse = (p, button) => { Turbine.selftestMouse(0, p.x, p.y, 0, 0, 0); Turbine.selftestMouse(1, p.x, p.y, button, 0, 0); Turbine.selftestMouse(2, p.x, p.y, button, 0, 0) }
            const hints = () => app.glyph("a") + ", " + app.glyph("b") + ", " + app.glyph("x") + ", " + app.glyph("y") + ", " + app.glyph("lb") + ", " + app.glyph("rb")
            script = [
                () => { if (app.mods.length < 2 || !app.modsLoaded) return false; app.keysTest = true; app.sheet = ""; app.page = "game"; app.zone = "main"; app.tab = 0; app.modIndex = 0
                        if (!app.mods[0].enabled) { Turbine.setEnabled(app.mods[0].id, true); app.mods = app.mods.map((x, i) => i === 0 ? Object.assign({}, x, { enabled: true }) : x) } },
                // The keyboard.
                () => { K(Qt.Key_Down); m.r1 = app.modIndex; K(Qt.Key_Up)
                        say(m.r1 === 1 && app.modIndex === 0 && app.lastInput === "keys", "Down and Up: row " + (m.r1 + 1) + " then row " + (app.modIndex + 1) + ", last input " + app.lastInput)
                        say(hints() === "Enter, Esc, R, F, Q, E", "the hints after a key: " + hints())
                        m.on = app.mods[0].enabled; K(Qt.Key_Return) },
                () => { say(app.mods[0].enabled !== m.on, "Enter is A: the mod turned " + (app.mods[0].enabled ? "on" : "off")); K(Qt.Key_Return) },
                () => { K(Qt.Key_Menu); m.s1 = app.sheet; K(Qt.Key_Escape); m.s2 = app.sheet; K(Qt.Key_F10, Qt.ShiftModifier); m.s3 = app.sheet; K(Qt.Key_Backspace)
                        say(m.s1 === "modmenu" && m.s2 === "" && m.s3 === "modmenu" && app.sheet === "", "Menu key: sheet \"" + m.s1 + "\", Esc: \"" + m.s2 + "\", Shift+F10: \"" + m.s3 + "\", Backspace: \"" + app.sheet + "\"") },
                () => { K(Qt.Key_F, Qt.ControlModifier); m.s1 = app.sheet; K(Qt.Key_Escape); m.s2 = app.sheet; K(Qt.Key_Slash); m.s3 = app.sheet; K(Qt.Key_Escape)
                        say(m.s1 === "findpop" && m.s2 === "" && m.s3 === "findpop" && app.sheet === "", "Ctrl+F: sheet \"" + m.s1 + "\", Esc: \"" + m.s2 + "\", /: \"" + m.s3 + "\", Esc: \"" + app.sheet + "\"") },
                () => { K(Qt.Key_Tab); say(app.tab === 1, "Tab: " + (app.tabs[app.tab] || "")) },
                () => { if (app.getItems.length < app.gridColumns + 2 || app.turnTo >= 0) return false; app.zone = "main"; app.getArea = "grid"; app.getIndex = 0
                        K(Qt.Key_Right); m.i1 = app.getIndex; K(Qt.Key_Down); m.i2 = app.getIndex; K(Qt.Key_Return) },
                () => { say(m.i1 === 1 && m.i2 === 1 + app.gridColumns && app.modPage, "Get mods: Right to card " + (m.i1 + 1) + ", Down to card " + (m.i2 + 1) + ", Enter opens " + (app.modPage ? "the mod's page" : "nothing"))
                        K(Qt.Key_Escape); say(!app.modPage && app.tab === 1, "Esc from the mod's page: " + (app.modPage ? "still there" : "back on " + (app.tabs[app.tab] || ""))) },
                () => { K(Qt.Key_PageDown); m.t1 = app.tab; K(Qt.Key_PageUp); m.t2 = app.tab; K(Qt.Key_Backtab, Qt.ShiftModifier)
                        say(m.t1 === 2 && m.t2 === 1 && app.tab === 0, "Page Down: tab " + m.t1 + ", Page Up: tab " + m.t2 + ", Shift+Tab: tab " + app.tab) },
                () => { K(Qt.Key_F1); m.p = app.page + " " + app.sec; K(Qt.Key_Escape); say(m.p === "settings Help" && app.page === "game", "F1: " + m.p + ", Esc: " + app.page) },
                () => { m.z = app.sizeNow; K(Qt.Key_Equal, Qt.ControlModifier); m.z1 = app.sizeNow; K(Qt.Key_Minus, Qt.ControlModifier)
                        say(m.z1 === m.z + 10 && app.sizeNow === m.z, "Ctrl+=: " + m.z + "% to " + m.z1 + "%, Ctrl+-: back to " + app.sizeNow + "%") },
                () => { K(Qt.Key_P); m.p = app.page; K(Qt.Key_P); m.p2 = app.page; K(Qt.Key_V); m.w = app.wheelOpen; K(Qt.Key_Right); m.ws = app.wheelSlot; K(Qt.Key_Escape)
                        say(m.p === "settings" && m.p2 === "game" && m.w && m.ws === 2 && !app.wheelOpen, "P: " + m.p + ", P again: " + m.p2 + "; V: the wheel " + (m.w ? "open" : "closed") + ", Right picks slot " + m.ws + ", Esc: " + (app.wheelOpen ? "still open" : "closed")) },
                () => { K(Qt.Key_Return, Qt.ControlModifier); m.s1 = app.sheet + " " + app.pickTitle; K(Qt.Key_Escape); say(m.s1 === "pick Play options" && app.sheet === "", "Ctrl+Enter: " + m.s1 + ", Esc: sheet \"" + app.sheet + "\"") },
                // The keys a PC player expects.
                () => { app.zone = "main"; app.tab = 0; app.modIndex = 0; K(Qt.Key_S); m.r1 = app.modIndex; K(Qt.Key_W); m.r2 = app.modIndex; K(Qt.Key_R); m.s1 = app.sheet; K(Qt.Key_Escape); K(Qt.Key_F); m.s2 = app.sheet; K(Qt.Key_Escape)
                        say(m.r1 === 1 && m.r2 === 0 && m.s1 === "modmenu" && m.s2 === "findpop" && app.sheet === "", "W A S D, R and F: S to row " + (m.r1 + 1) + ", W to row " + (m.r2 + 1) + ", R: sheet \"" + m.s1 + "\", F: sheet \"" + m.s2 + "\"") },
                () => { K(Qt.Key_E); m.t1 = app.tab; K(Qt.Key_BracketRight); m.t2 = app.tab; K(Qt.Key_BracketLeft); K(Qt.Key_Q)
                        say(m.t1 === 1 && m.t2 === 2 && app.tab === 0, "Q and E turn the pages: E: tab " + m.t1 + ", ]: tab " + m.t2 + ", [ and Q: tab " + app.tab) },
                () => { if (app.turnTo >= 0) return false; app.zone = "main"; K(Qt.Key_Escape); m.p = app.page; K(Qt.Key_Escape); m.p2 = app.page; K(Qt.Key_Backspace)
                        say(m.p === "settings" && m.p2 === "game" && app.page === "game", "Esc on the first page: " + m.p + ", Esc again: " + m.p2 + "; Backspace stays B: " + app.page)
                        app.zone = "main" },
                // A field being typed in has the letters itself.
                () => { K(Qt.Key_F); if (app.sheet !== "findpop") { say(false, "F didn't open Find"); return }
                        app.findPopField(); m.t = app.tab
                        K(Qt.Key_Q); K(Qt.Key_E); K(Qt.Key_R); K(Qt.Key_W); K(Qt.Key_F); K(Qt.Key_Space)
                        say(app.sheet === "findpop" && app.tab === m.t && !app.wheelOpen && !app.spaceDown, "typing in Find: Q, E, R, W, F and Space are the field's: sheet \"" + app.sheet + "\", tab " + app.tab)
                        app.findPopEndTyping(); K(Qt.Key_Escape) },
                // Space: a tap is A, a 3 second hold is Start's hold (the Play choice).
                () => { if (app.sheet !== "") { say(false, "Find stayed open: sheet \"" + app.sheet + "\""); app.sheet = "" } app.zone = "main"; app.tab = 0; app.modIndex = 0; m.on = app.mods[0].enabled; K(Qt.Key_Space) },
                () => { say(app.mods[0].enabled !== m.on, "a tap of Space is A: the mod turned " + (app.mods[0].enabled ? "on" : "off")); K(Qt.Key_Space) },
                () => { m.on = app.mods[0].enabled; m.t = Date.now(); Turbine.selftestKey(Qt.Key_Space, 0, true) },
                () => { if (app.sheet !== "pick" && Date.now() - m.t < 6000) return false
                        m.s1 = app.sheet + " " + app.pickTitle; m.ms = Date.now() - m.t; Turbine.selftestKey(Qt.Key_Space, 0, false); m.s2 = app.sheet; K(Qt.Key_Escape)
                        say(m.s1 === "pick Play options" && m.ms >= 2900 && m.s2 === "pick" && app.mods[0].enabled === m.on && app.sheet === "", "Space held: " + m.s1 + " after " + (m.ms >= 2900 ? "3 seconds" : (m.ms / 1000).toFixed(1) + " s") + ", the mod left as it was, Esc: sheet \"" + app.sheet + "\"") },
                // Tab held: the quick wheel; W A S D pick.
                () => { m.t = Date.now(); m.tb = app.tab; Turbine.selftestKey(Qt.Key_Tab, 0, true) },
                () => { if (!app.wheelOpen && Date.now() - m.t < 3000) return false
                        m.w = app.wheelOpen; K(Qt.Key_D); m.ws = app.wheelSlot; K(Qt.Key_Escape); m.w2 = app.wheelOpen; Turbine.selftestKey(Qt.Key_Tab, 0, false)
                        say(m.w && m.ws === 2 && !m.w2 && !app.wheelOpen && app.tab === m.tb, "Tab held: the wheel " + (m.w ? "open" : "closed") + ", D picks slot " + m.ws + ", Esc: " + (m.w2 || app.wheelOpen ? "still open" : "closed") + ", the page " + (app.tab === m.tb ? "stayed" : "turned")) },
                // A toast's step (Undo) is Ctrl+Z.
                () => { app.zone = "main"; app.tab = 0; m.wasOn = app.mods.filter(x => x.enabled).length; app.allOff(); m.g = toast.actionGlyph },
                () => { K(Qt.Key_Z, Qt.ControlModifier); const on = app.mods.filter(x => x.enabled).length
                        say(m.g === "Ctrl+Z" && on === m.wasOn && m.wasOn > 0, "the toast says " + m.g + " Undo; Ctrl+Z: " + on + " of " + m.wasOn + " back on") },
                // The mouse: hover, click, right-click, Back, Ctrl with the wheel.
                // A real mouse comes to the row from somewhere else (it may already rest on that spot from an earlier check).
                () => { const it = modList.itemAtIndex(1); if (!it) return false
                        m.hv = app.hoverSeen; const p = at(it, 0.4); Turbine.selftestMouse(0, p.x + 40, p.y + 6, 0, 0, 0) },
                () => { const p = at(modList.itemAtIndex(1), 0.4); Turbine.selftestMouse(0, p.x, p.y, 0, 0, 0) },
                () => { if (app.lastInput !== "mouse") app.note("check", "Key check note: hovers seen " + (app.hoverSeen - m.hv) + ", the last " + app.hoverSeenAfterKey + " ms after a key, pointer shown " + Turbine.pointerShown)
                        say(app.lastInput === "mouse" && !!app.hoverItem && hints() === "Click, Esc, Right-click, F, Q, E", "the mouse over a row: last input " + app.lastInput + ", " + (app.hoverItem ? "the row is outlined" : "nothing outlined") + ", hints " + hints())
                        m.on = app.mods[1].enabled; mouse(at(modList.itemAtIndex(1), 0.4), Qt.LeftButton) },
                () => { say(app.mods[1].enabled !== m.on && app.modIndex === 1, "a click on a row is A: row " + (app.modIndex + 1) + " turned " + (app.mods[1].enabled ? "on" : "off"))
                        mouse(at(modList.itemAtIndex(0), 0.4), Qt.RightButton) },
                () => { m.s1 = app.sheet; m.row = app.modIndex; mouse({ x: app.width / 2, y: app.height / 3 }, Qt.BackButton)
                        say(m.s1 === "modmenu" && m.row === 0 && app.sheet === "", "a right-click on row 1: sheet \"" + m.s1 + "\" for row " + (m.row + 1) + "; the Back button: sheet \"" + app.sheet + "\"")
                        app.tab = 1 },
                () => { if (app.getItems.length < 2 || app.turnTo >= 0) return false
                        const cell = getView.gridItem(1)
                        if (!cell) { say(false, "no card to right-click"); return }
                        mouse(at(cell), Qt.RightButton) },
                () => { m.s1 = app.sheet; m.gi = app.getIndex; mouse({ x: app.width / 2, y: app.height / 3 }, Qt.BackButton); m.s2 = app.sheet; mouse({ x: app.width / 2, y: app.height / 3 }, Qt.BackButton)
                        say(m.s1 === "cardmenu" && m.gi === 1 && m.s2 === "" && app.tab === 0, "a right-click on card 2: sheet \"" + m.s1 + "\"; Back: sheet \"" + m.s2 + "\"; Back again: " + (app.tabs[app.tab] || "").replace(/ \(\d+\)$/, "")) },
                () => { m.z = app.sizeNow; Turbine.selftestMouse(3, app.width / 2, app.height / 3, 0, 0, 1); m.z0 = app.sizeNow
                        Turbine.selftestMouse(3, app.width / 2, app.height / 3, 0, Qt.ControlModifier, -1); m.z1 = app.sizeNow; Turbine.selftestMouse(3, app.width / 2, app.height / 3, 0, Qt.ControlModifier, 1)
                        say(m.z0 === m.z && m.z1 === m.z + 10 && app.sizeNow === m.z, "the wheel alone: " + m.z0 + "%; Ctrl and the wheel up: " + m.z1 + "%, down: " + app.sizeNow + "%") },
                // After the mouse no key caps; Forward is the next page; a double-click opens a mod.
                () => { const it = modList.itemAtIndex(1); if (!it) return false
                        const p = at(it, 0.4); Turbine.selftestMouse(0, p.x, p.y, 0, 0, 0) },
                () => { legend.check("keycheck mouse")
                        say(app.mouseLegend && barLb.width === 0 && barRb.width === 0 && startHintKey.width === 0, "after the mouse: " + (barLb.width === 0 && barRb.width === 0 && startHintKey.width === 0 ? "no key caps" : "key caps") + ", the pages by name: " + app.tabs.map(t => t.replace(/ \(\d+\)$/, "")).join(" / "))
                        m.tb = app.tab; mouse({ x: app.width / 2, y: app.height / 3 }, Qt.ForwardButton); m.t1 = app.tab; mouse({ x: app.width / 2, y: app.height / 3 }, Qt.BackButton)
                        say(m.t1 === m.tb + 1 && app.tab === m.tb, "the Forward button: tab " + m.t1 + "; Back: tab " + app.tab) },
                // A mod whose click asks nothing (one that needs a missing loader asks for it on the first click).
                () => { if (app.turnTo >= 0) return false
                        m.di = app.mods.findIndex(x => !x.needsLoader); const it = m.di >= 0 ? modList.itemAtIndex(m.di) : null; if (!it) return false
                        m.on = app.mods[m.di].enabled; m.name = app.mods[m.di].name; const p = at(it, 0.4); mouse(p, Qt.LeftButton); mouse(p, Qt.LeftButton) },
                () => { m.s1 = app.sheet; m.ok = (app.sheet === "pick" && app.pickTitle === m.name) || app.sheet === "modmenu"
                        say(m.ok && app.modIndex === m.di && app.mods[m.di].enabled === m.on, "a double-click on a mod: " + (m.ok ? "the mod opened" : "sheet \"" + app.sheet + "\"") + ", left " + (app.mods[m.di].enabled === m.on ? "as it was" : "changed"))
                        mouse({ x: app.width / 2, y: app.height / 3 }, Qt.BackButton) },
                () => { if (app.sheet !== "") mouse({ x: app.width / 2, y: app.height / 3 }, Qt.BackButton) },
                () => { say(app.sheet === "", "Back from the opened mod: sheet \"" + app.sheet + "\"") },
                // The hints follow the last input: a key, then the controller.
                () => { K(Qt.Key_Down); m.h1 = hints() },
                () => { legend.check("keycheck"); pad("up"); m.h2 = hints()
                        say(m.h1 === "Enter, Esc, R, F, Q, E" && m.h2 === "A, B, X, Y, LB, RB", "hints after a key: " + m.h1 + "; after a controller press: " + m.h2)
                        app.keysTest = false
                        // Desktop Mode with no controller: one line in the log, nothing on screen.
                        m.toastBefore = app.lastToastAt; m.noted = app.noPadNote(); m.again = app.noPadNote()
                        check("No controller in Desktop Mode: " + (Turbine.pads ? "a controller is seen" : Turbine.gamemode ? "Game Mode" : m.noted && !m.again ? "logged once" : "not logged") + ", " + (app.lastToastAt === m.toastBefore ? "no toast" : "a toast showed"))
                        // One sign-in status: Settings' row and the Nexus page's hints read the same thing.
                        app.signinRefresh()
                        say((app.accountRows[0].title === "Nexus Mods: Signed in") === Turbine.browserSignedIn() && app.browserSigninShown === (app.bridge.paired === true && !Turbine.browserSignedIn()),
                            "sign-in status: Settings says " + app.accountRows[0].title + ", Turbine's browser is " + (Turbine.browserSignedIn() ? "signed in" : "not signed in"))
                        // Get Seamless Co-op goes to Nexus first when Turbine's Nexus pages are signed in, and to
                        // Turbine's own download otherwise (nothing is opened or downloaded here).
                        m.p1 = app.seamlessGetPlan(); m.was = app.browserSignedIn; app.browserSignedIn = true; app.browserLiveForced = true; m.p2 = app.seamlessGetPlan()
                        m.url = Turbine.browserPageMod(app.seamlessNexusUrl); app.browserSignedIn = m.was; app.browserLiveForced = false
                        check("Seamless Co-op plan: not signed in on Nexus: " + m.p1 + "; signed in: " + m.p2 + " (mod " + m.url + " of " + app.seamlessNexusUrl.split("/")[3] + ")")
                        check("Key check done: " + m.bad + " problems") }
            ]
        }
        // The Co-op tile's rows follow the engine's answer for where Turbine runs: in Desktop Mode one press
        // (at once with Steam closed, or one restart, asked first), in Game Mode words only with OK.
        // Requirements lined up and the Dependencies group, by real presses. The build has no internet, so the
        // requirement is given to the first mod here (as Nexus's list would after an add); the queue's first step
        // is read from the log instead of a page opening.
        else if (name === "reqcheck") {
            m.bad = 0
            const say = (ok, text) => { if (!ok) m.bad++; check("Requirements check: " + text + (ok ? "" : "  PROBLEM")) }
            const give = () => { const l = Object.assign({}, app.linksMap); l[app.mods[0].id] = { needs: [{ name: "Test Library", id: null, domain: app.rules.domain, modId: 4242 }], neededBy: [] }
                                 l[app.mods[1].id] = { needs: [], neededBy: [app.mods[0].id] }; l[app.mods[0].id].needs.push({ name: app.mods[1].name, id: app.mods[1].id, domain: "", modId: 0 }); app.linksMap = l }
            const rowKinds = () => app.listItems.map(x => x.kind).join(",")
            script = [
                () => { if (app.mods.length < 2 || !app.modsLoaded) return false; app.sheet = ""; app.page = "game"; app.zone = "main"; app.tab = 0; app.modIndex = 0
                        if (!app.mods[0].enabled) app.toggle(app.mods[0], true)
                        if (app.mods[1].enabled) app.toggle(app.mods[1], true)
                        app.reqClear(); app.reqRunning = false },
                // An add of a mod that needs one mod that isn't installed and one that is but is off.
                () => { give(); app.reqArrived(app.mods[0].id, app.mods[0].name); m.t = Date.now() },
                () => { if (app.sheet !== "msg" && Date.now() - m.t < 4000) return false
                        say(app.sheet === "msg" && !!app.msgData && app.msgData.title === "This mod needs 2 others" && app.msgData.buttons.map(b => b.label).join("; ") === "Get them all; Not now",
                            "after the add: " + (app.sheet === "msg" && app.msgData ? app.msgData.title + " | " + app.msgData.buttons.map(b => b.label).join("; ") + " | " + (app.msgData.notes || []).join("; ") : "sheet \"" + app.sheet + "\""))
                        pad("a") },
                () => { say(app.reqRunning && app.reqStripWords === "Getting 1 of 1: Test Library" && app.mods[1].enabled, "Get them all: " + (app.reqStripWords || "nothing running") + ", the installed one turned " + (app.mods[1].enabled ? "on" : "off"))
                        app.reqStop() },
                () => { say(!app.reqRunning && app.listItems[0] && app.listItems[0].kind === "needsq", "Stop: " + (app.reqRunning ? "still running" : "stopped") + "; My mods rows: " + rowKinds())
                        app.modIndex = 0; app.zone = "main" },
                // The loader question opened when Get them all turned on Lantern Glow SKSE (it needs SKSE): B closes it
                // first, then A on the row.
                () => { if (app.sheet !== "") pad("b") },
                () => { app.modIndex = 0; app.zone = "main"; pad("a") },
                () => { say(app.reqRunning && app.reqStripWords === "Getting 1 of 1: Test Library", "A on the row goes on: " + (app.reqStripWords || "nothing running"))
                        app.reqSkip() },
                () => { if (app.reqRunning) return false },
                () => { say(!app.reqRunning && !app.reqLeft, "Skip on the last one: " + (app.reqRunning ? "still running" : "done") + ", rows: " + rowKinds()) },
                // Before you play names what is missing, with Get what they need first.
                () => { give(); m.c = app.playAsks; app.startPlay() },
                () => { if (app.sheet !== "check") return false
                        const row = app.checks.find(x => x.title === app.reqCheckTitle)
                        say(!!row && app.checkChoices[0] === "Get what they need", "Before you play: " + (row ? row.title + ": " + row.detail : "no row") + " | " + app.checkChoices.join("; "))
                        pad("b") },
                // Dependencies: the mod another one needs goes into the group at the bottom, A opens it, X moves it out.
                () => { if (app.sheet !== "") return false; give(); app.depsOpen = false; app.zone = "main"
                        m.k1 = rowKinds(); app.modIndex = app.listItems.findIndex(x => x.kind === "deps"); pad("a") },
                () => { const i = app.listItems.findIndex(x => x.kind === "mod" && x.dep)
                        say(m.k1 === "mod,deps" && app.depsOpen && i >= 0, "Dependencies: rows " + m.k1 + "; A opens it: " + rowKinds() + (i >= 0 ? " (" + app.listItems[i].m.name + ")" : ""))
                        if (i >= 0) { app.modIndex = i; pad("x") } },
                () => { if (app.sheet !== "modmenu") return false
                        const more = app.modMenuItems.find(x => x.act === "more"), rows = more ? more.rest : []
                        const out = rows.find(x => x.act === "depout")
                        say(!!out, "X options of a dependency: " + (out ? out.title : "no Move out row") + "; order before: " + app.mods.map(x => x.name).join(", "))
                        m.order = app.mods.map(x => x.id).join(",")
                        if (out) app.modMenuDo(out, app.focusedItem.m); app.sheet = "" },
                () => { give()
                        say(rowKinds() === "mod,mod" && app.mods.map(x => x.id).join(",") === m.order, "moved out: rows " + rowKinds() + ", the load order " + (app.mods.map(x => x.id).join(",") === m.order ? "unchanged" : "CHANGED"))
                        const i = app.listItems.findIndex(x => x.kind === "mod" && x.m.id === app.mods[1].id); app.modIndex = i; m.warnBefore = app.sheet; pad("a") },
                () => { say(app.sheet === "pick" && /need/.test(app.pickTitle), "turning off a mod another needs warns: " + (app.sheet === "pick" ? app.pickTitle : "sheet \"" + app.sheet + "\""))
                        pad("b"); Turbine.clearDepMoves(); app.seenRev++ },
                () => { check("Requirements check done: " + m.bad + " problems") }
            ]
        }
        else if (name === "tilecheck") script = [
            () => { if (!app.seamlessIn || !app.modsLoaded) return false; app.sheet = ""; app.page = "game"; app.zone = "main"; app.tab = 0
                    const o = app.coopTileOffer
                    check("Tile check: Game Mode " + Turbine.gamemode + ", how " + o.how + ", label \"" + o.label + "\", words: " + o.words)
                    const row = app.gameMenuItems.find(x => x.act === "cooptile")
                    check("Tile check: game menu row: " + (row ? row.title + " | " + row.detail : "none"))
                    m.asked = app.maybeAskCoopTile()
                    m.s1 = app.sheet === "msg" && app.msgData ? app.msgData.title + " | " + app.msgData.buttons.map(b => b.label).join("; ") : "sheet \"" + app.sheet + "\""
                    if (app.sheet !== "") pad("b") },
            () => { check("Tile check: asked once after the install: " + (m.asked ? "yes, " + m.s1 : "no (" + app.coopTileOffer.how + ")"))
                    app.sheet = ""; app.openCoopTile() },
            () => { check("Tile check: the row opens: " + (app.sheet === "msg" && app.msgData ? app.msgData.title + " | " + app.msgData.text.substring(0, 90) + " | " + app.msgData.buttons.map(b => b.label).join("; ") : "sheet \"" + app.sheet + "\""))
                    pad("b") },
            () => { check("Tile check done: sheet \"" + app.sheet + "\"") }
        ]
        // An older Seamless Co-op file is asked about, never added by itself and never a failed mod: A keeps the
        // one installed, B too, X uses the older file.
        else if (name === "oldercheck") {
            const file = shot.arg("older-file")
            const ver = () => app.seamlessMod ? app.versionOf(app.seamlessMod) : "none"
            const ask = () => app.sheet === "msg" && app.msgData ? app.msgData.title + " | " + app.msgData.text + " | " + app.msgData.buttons.map(b => b.label).join("; ") + " | highlighted " + (app.infoButtons[app.infoIndex] ? app.infoButtons[app.infoIndex].label : "") : "sheet \"" + app.sheet + "\""
            const pwRow = () => settingsView.shown.find(r => r.f && /password/i.test(r.f.key || ""))
            const pwState = () => app.seamlessMod ? ["unknown", "empty", "set"][Turbine.coopPasswordState(app.seamlessMod.id) + 1] : "no Seamless Co-op"
            script = [
                () => { if (!app.seamlessIn || !app.modsLoaded) return false; app.sheet = ""; app.page = "game"; app.zone = "main"; app.tab = 0; m.n = app.mods.length; m.newer = app.seamlessMod.id
                        check("Older check: installed Seamless Co-op " + ver() + ", " + app.mods.length + " mods; Get Seamless Co-op would: " + app.seamlessGetPlan())
                        app.openCoopSettings() },
                // A session password on the copy installed, typed on its settings page and saved by itself.
                () => { if (app.sheet !== "settingsedit" || !pwRow()) return false; app.coopTypePassword() },
                () => { if (app.sheet !== "ask") return false; askView.open("Session password", "", "dragon", "", 2000, false, "Done"); askView.accept() },
                () => { if (app.sheet !== "settingsedit" || settingsView.changes > 0 || Turbine.coopPasswordState(app.seamlessMod.id) !== 1) return false; pad("b") },
                () => { if (app.sheet !== "") return false
                        check("Older check: the password on " + ver() + " is " + pwState()); m.asks = app.olderAsks; Turbine.selftestImport(file, "loader", "seamless", "1.9.0") },
                () => { if (app.olderAsks === m.asks) return false
                        check("Older check: asked: " + ask() + " | error said: " + (app.msgLevel === "error" ? app.msgText : "none")); pad("a") },
                () => { if (Turbine.busy) return false
                        check("Older check: A keeps it: sheet \"" + app.sheet + "\", Seamless Co-op " + ver() + ", " + app.mods.length + " mods, failed: " + (app.msgLevel === "error" ? app.msgText : "none"))
                        m.asks = app.olderAsks; Turbine.selftestImport(file, "loader", "seamless", "1.9.0") },
                () => { if (app.olderAsks === m.asks) return false; pad("b") },
                () => { if (Turbine.busy) return false
                        check("Older check: B goes back and keeps it: sheet \"" + app.sheet + "\", Seamless Co-op " + ver() + ", " + app.mods.length + " mods")
                        m.asks = app.olderAsks; Turbine.selftestImport(file, "loader", "seamless", "1.9.0") },
                // Use is the second button: Right to it, then A (a message's buttons are picked with the D-pad).
                () => { if (app.olderAsks === m.asks || app.sheet !== "msg") return false
                        pad("right"); m.on = app.infoButtons[app.infoIndex] ? app.infoButtons[app.infoIndex].label : ""; pad("a") },
                // The older file is a new mod in the list, on; the newer copy is in Removed mods.
                () => { if (Turbine.busy || ver() !== "1.9.0") return false
                        const used = app.mods.filter(x => x.seamless === true)
                        check("Older check: Right then A on " + m.on + ": in the list " + used.map(x => "Seamless Co-op " + app.versionOf(x) + (x.enabled ? " on" : " off")).join(", ") + " (" + app.mods.length + " mods), Removed mods " + app.removedMods.map(x => x.id === m.newer ? "the 2.0.1 copy" : x.name).join("; ")
                              + ", failed: " + (app.msgLevel === "error" ? app.msgText : "none") + ", sheet \"" + app.sheet + "\"")
                        check("Older check: the password on " + ver() + " is " + pwState())
                        if (app.sheet !== "") app.sheet = ""
                        app.openCoopSettings() },
                () => { if (app.sheet !== "settingsedit" || !pwRow()) return false
                        check("Older check: the password carried over: " + (String(settingsView.valueOf(pwRow().f)) === "dragon" ? "yes" : "no, it reads \"" + settingsView.valueOf(pwRow().f) + "\"")); pad("b") },
                () => { if (app.sheet !== "") return false; app.openRemoved() },
                () => { if (app.sheet !== "removed") return false
                        const r = app.removedMods[app.removedIndex], o = app.binds()
                        m.back = r ? r.id : ""
                        check("Older check: Removed mods offers: " + (r ? (r.id === m.newer ? "the 2.0.1 copy" : r.name) : "nothing") + ", A " + (o.a ? o.a.label : "") + ", used twice: " + (app.olderAsks === m.asks + 1 ? "no" : "asked again"))
                        pad("a") },
                // What Put back then does (two copies are the two copies check's own subject): said, not required.
                () => { if (Turbine.busy) return false
                        check("Older check: after Put back: in the list " + app.mods.filter(x => x.seamless === true || x.id === m.back).map(x => x.name + " " + app.versionOf(x) + (x.enabled ? " on" : " off")).join(", ") + "; Removed mods " + app.removedMods.length + ", sheet \"" + app.sheet + "\"") },
                () => { check("Older check done: asked " + app.olderAsks + " times") }
            ]
        }
        // "Proton for modded play": the row in the game menu (This game), its list (Automatic first, an
        // unusable Proton dimmed with the reason), a pick kept and Automatic again, and the same from the tool's
        // steps sheet (its button, and B comes back to the steps).
        else if (name === "protoncheck") {
            const rowsNow = () => app.pickItems.map(r => r.title + (r.dim ? " [dim: " + r.detail + "]" : r.detail ? " [" + r.detail + "]" : "")).join("; ")
            const openList = () => { app.sheet = "gamemenu"; app.menuIndex = app.gameMenuItems.findIndex(x => x.act === "proton"); pad("a") }
            script = [
                () => { if (!app.game || !app.modsLoaded || !app.rules) return false; app.sheet = ""; app.page = "game"; app.zone = "main"; app.tab = 0
                        const row = app.gameMenuItems.find(x => x.act === "proton")
                        check("Proton check: game menu row: " + (row ? row.title + " | section " + row.group : "none") + " | " + (app.protonNow ? app.protonNow.list.length + " installed, how " + app.protonNow.how : "no Proton found"))
                        openList() },
                () => { if (app.sheet !== "pick") return false
                        check("Proton check: the list: " + app.pickTitle + " | " + rowsNow() + " | focus " + (app.pickItems[app.menuIndex] ? app.pickItems[app.menuIndex].title : ""))
                        m.picks = app.protonPicks; app.menuIndex = app.pickItems.findIndex(r => r.dim === true); m.dimRow = app.pickItems[app.menuIndex] ? app.pickItems[app.menuIndex].title : "none"; pad("a") },
                () => { check("Proton check: A on the dimmed one (" + m.dimRow + "): picked " + (app.protonPicks - m.picks) + " times, still " + app.protonTitle(app.protonNow))
                        if (app.sheet !== "pick") openList() },
                () => { if (app.sheet !== "pick") return false; app.menuIndex = app.pickItems.findIndex((r, i) => i > 0 && r.dim !== true); m.want = app.pickItems[app.menuIndex] ? app.pickItems[app.menuIndex].title : ""; pad("a") },
                () => { check("Proton check: picked " + m.want + ": " + app.protonTitle(app.protonNow) + ", how " + (app.protonNow ? app.protonNow.how : "?") + ", sheet \"" + app.sheet + "\", toast: " + app.lastToast)
                        openList() },
                () => { if (app.sheet !== "pick") return false; m.focus = app.pickItems[app.menuIndex] ? app.pickItems[app.menuIndex].title : ""; app.menuIndex = 0; pad("a") },
                () => { check("Proton check: the list opened on " + m.focus + "; Automatic again: " + app.protonTitle(app.protonNow) + ", how " + (app.protonNow ? app.protonNow.how : "?"))
                        app.sheet = ""; app.openLaunchSetup({ game: app.game.id, name: app.gameTitle, option: "x %command%", set: "", listed: true, coop: false }) },
                () => { if (app.sheet !== "msg") return false
                        check("Proton check: the tool's steps sheet: " + (app.msgData ? app.msgData.buttons.map(b => b.label).join("; ") + " | " + app.msgData.footnote : ""))
                        app.infoIndex = app.msgData ? app.msgData.buttons.findIndex(b => b.proton === true) : 0; pad("a") },
                () => { m.s1 = app.sheet + " " + app.pickTitle; pad("b"); check("Proton check: its button on the steps: " + m.s1 + "; B: sheet " + app.sheet + (app.sheet === "msg" && app.msgData ? " (" + app.msgData.title + ")" : "")) },
                () => { app.sheet = ""; check("Proton check done") }
            ]
        }
        else if (name === "presscount") script = [
            // After the tutorial: the rail, the game with no mods focused. Count every press to a mod added and Play.
            () => { if (!app.game || !app.modsLoaded) return false; app.sheet = ""; app.zone = "rail"; app.railIndex = app.current; m.start = app.inputCount; m.log = [] },
            () => { pad("a"); m.log.push("A open " + app.game.name) },
            () => { if (app.tab !== 1 || app.getArea !== "grid" || app.getItems.length === 0) return false; pad("a"); m.log.push("A on " + app.getItems[app.getIndex].name) },
            () => { if (!app.detailsData || app.detailsData.loading || app.pageIndex !== 0 || app.mainLabel === "") return false; m.log.push("A " + app.mainLabel); pad("a") },
            // On Nexus: Slow download (counted, outside Turbine); the download arrives and Turbine adds it.
            () => { m.log.push("Slow download on Nexus"); Turbine.importZip(shot.arg("presscount-file")) },
            () => { if (app.zone !== "play") return false; m.log.push("A Play (focused)"); pad("a") },
            () => { const n = app.inputCount - m.start
                    check("Presses from the rail to a mod added and Play: " + n + " in Turbine + 1 on Nexus = " + (n + 1) + " (" + m.log.join(", ") + ")") }
        ]
        scriptStep = 0; scriptTries = 0
        scriptTimer.start()
    }

    // ---------------------------------------------------------------- the build's checks of Nexus inside Turbine
    // A stand-in Nexus page on 127.0.0.1 (tools/nexus-stand-in.py) with Slow download and Mod Manager Download at fixed
    // places: each step waits for its condition (up to its max), then acts. Presses go through Turbine.selftestPad,
    // the same path a controller's take; the pointer is placed on the page's buttons by their place in the page.
    property var btSteps: []
    property int btStep: 0
    property real btSince: 0
    property var bt: ({})
    Timer { id: btTimer; interval: 250; repeat: true; onTriggered: sc.btTick() }
    Timer { id: btFinal; interval: 1200; onTriggered: app.contentItem.grabToImage(function (r) { r.saveToFile(shot.path); Qt.quit() }) }
    function btRun(steps) { btSteps = steps; btStep = 0; btSince = Date.now(); btTimer.start() }
    function btTick() {
        const st = btSteps[btStep]
        if (!st) { btTimer.stop(); check("Browser check: done"); if (shot.on) btFinal.start(); return }
        const waited = Date.now() - btSince
        if (st.after && waited < st.after) return
        let ready = true
        try { ready = !st.wait || st.wait() === true } catch (e) { ready = false }
        if (!ready && waited < (st.max || 15000)) return
        if (!ready) check("Browser check: step " + (btStep + 1) + " (" + st.name + ") gave up waiting: page " + app.page + ", url " + (app.nexusView ? app.nexusView.url : "none") + ", sheet \"" + app.sheet + "\"")
        try { if (st.act) st.act() } catch (e) { check("Browser check: step " + (btStep + 1) + " (" + st.name + ") stopped: " + e) }
        btStep++; btSince = Date.now()
    }
    // A picture of this moment beside the run's own (shots/<page>-<size>-<tag>.png).
    function btShot(tag) {
        const p = shot.path.replace(/\.png$/, "-" + tag + ".png")
        app.contentItem.grabToImage(function (r) { r.saveToFile(p); sc.check("Browser check: picture " + tag) })
    }
    // The pointer onto a place in the page (CSS pixels), then RT pressed and let go: one click.
    function btClick(cx, cy) {
        const p = app.nexusView.pagePoint(cx, cy)
        Turbine.pointerShowAt(p.x, p.y)
        Turbine.selftestPad("rt", true); Turbine.selftestPad("rt", false)
    }
    function btMark(name) {
        const m = /^(http:\/\/127\.0\.0\.1:[0-9]+)/.exec(app.browserTest)
        if (!m) return
        const x = new XMLHttpRequest()
        x.open("GET", m[1] + "/mark?name=" + encodeURIComponent(name))
        x.send()
        check("Browser check: mark " + name)
    }
    // The legend checks' problems with a legend h for the binds o: too long, a key twice, more (the caller's own
    // checks, given the keys), a bound key missing or a shown key doing nothing.
    function legendProblems(o, h, more) {
        const keys = h.map(x => x[0])
        let bad = []
        if (h.length > 5) bad.push("more than 5")
        if (keys.some((k, i) => keys.indexOf(k) !== i)) bad.push("a key twice")
        if (more) bad = bad.concat(more(keys))
        for (const k of ["a", "b", "x", "y"]) {
            if (o[k] && o[k].label && keys.indexOf(k) < 0) bad.push(k.toUpperCase() + " missing")
            if (keys.indexOf(k) >= 0 && !(o[k] && o[k].run)) bad.push(k.toUpperCase() + " does nothing")
        }
        return bad
    }
    function btLegend(what) {
        const o = app.binds(), h = app.legendFor(o)
        const bad = legendProblems(o, h)
        if (JSON.stringify(app.nexusView ? app.legendHints : []) !== JSON.stringify(h)) bad.push("the legend on screen differs")
        check("Browser legend (" + what + "): " + h.map(x => x[0].toUpperCase() + " " + x[1]).join(" · ") + ", " + bad.length + " problems" + (bad.length ? ": " + bad.join(", ") : ""))
    }
    readonly property string btFiles: "https://www.nexusmods.com/skyrimspecialedition/mods/1?tab=files&file_id=2"
    function browserCheckScript() {
        const big = app.width >= 1900
        const pageIs = (re) => !!app.nexusView && !!app.nexusView.view && !app.nexusView.loading && re.test(app.nexusView.url)
        btRun([
            { name: "Get mods", max: 20000, wait: () => !!app.game && app.modsLoaded && app.getItems.length > 0,
              act: () => { Turbine.selftestCatch("watch")
                           // The sign-in pass-through: a helper's answer goes into the profile (checked on disk later).
                           Turbine.selftestCatch("signin"); app.browserNoteNow = null; toast.hide()
                           app.openDetails(app.getItems[0]) } },
            { name: "mod page", max: 15000, wait: () => !!app.detailsData && !app.detailsData.loading && app.mainLabel === "Get it on Nexus",
              act: () => { check("Browser check: mod page " + app.detailsData.name + ", button " + app.mainLabel); app.openNexus(btFiles) } },
            { name: "file page", max: 45000, wait: () => pageIs(/tab=files/),
              act: () => { check("Browser check: the file page loaded in Turbine: " + app.nexusView.title + " (" + app.nexusView.url + ")"); btShot("page"); if (!big) btMark("idle-start") } },
            { name: "idle", after: big ? 500 : 30000,
              act: () => { if (!big) btMark("idle-end"); btLegend("no field") } },
            { name: "field", act: () => btClick(190, 500) },
            { name: "field focus", max: 6000, wait: () => app.nexusView.focusKind === 2,
              act: () => { btLegend("a field"); check("Browser check: Steam's keyboard opens for a field: " + (app.browserFieldSeen > 0 ? "yes" : "no"))
                           // A clicked a moment ago: still a click at the pointer, never Enter on what has the focus.
                           check("Browser check: A after a click is a click: " + (Turbine.browserKeyNav ? "no" : "yes")); pad("down") } },
            { name: "d-pad", after: 300, act: () => { check("Browser check: A after the D-pad presses Enter: " + (Turbine.browserKeyNav ? "yes" : "no")); btClick(110, 432) } },
            { name: "on the second page", max: 15000, wait: () => pageIs(/tab=description/),
              act: () => { check("Browser check: A after the pointer clicked is a click again: " + (Turbine.browserKeyNav ? "no" : "yes")); pad("b") } },
            { name: "back", max: 15000, wait: () => pageIs(/tab=files/),
              act: () => { check("Browser check: B from a second page went back one page (" + app.nexusView.url + ")"); btClick(150, 312) } },
            { name: "refused", max: 10000, wait: () => !!app.browserNoteNow && app.browserNoteNow.text.indexOf("zip, 7z or rar") >= 0,
              act: () => { check("Browser check: refused: " + app.browserNoteNow.text); app.browserNoteNow = null; btClick(150, 372) } },
            { name: "downloading", max: 15000, wait: () => app.webDls.some(x => x.name.indexOf("Dupe Test") === 0 && x.state === "progress"),
              act: () => { const x = app.webDls.find(d => d.name.indexOf("Dupe Test") === 0); check("Browser check: second download of a name there already is " + x.name); pad("rb") } },
            // A picture is taken at the next frame: what changes the screen waits for the step after it.
            { name: "downloads tab", after: 400, act: () => btShot("downloads") },
            { name: "page tab", after: 500, act: () => pad("lb") },
            { name: "quick menu", after: 400, act: () => { app.openBrowserMenu(); check("Browser check: the quick menu has " + app.pickItems.length + " rows (at most 6): " + app.pickItems.map(x => x.title).join(", "))
                                                         check("Browser check: touch targets " + app.nexusView.tapSizes().map(x => x[0] + " " + x[1]).join(", ")) } },
            { name: "quick menu closed", after: 400, act: () => pad("b") },
            { name: "added", max: 25000, wait: () => app.webDls.some(x => x.name.indexOf("Dupe Test") === 0 && x.state === "added"),
              act: () => { check("Browser check: pill " + app.browserPill.text); btShot("added") } },
            { name: "mod manager download", after: 500, act: () => { bt.nxmAt = Date.now(); btClick(460, 188)
                check("Browser check: the Added note cleared at the next click: " + (!app.browserNoteNow || app.browserNoteNow.text.indexOf("Added") < 0 ? "yes" : "no")) } },
            // Every Mod Manager Download link from the page asks: on Not now, deaf for a second, then Right and A.
            { name: "mmd asks", max: 15000, wait: () => app.nxmAsking,
              // What it offered the moment it opened (nxmAskFirst), and an A made 300 ms after it showed (judged by when it
              // was made, so a slow runner can't turn it into a late press).
              act: () => { const f = app.nxmAskFirst || {}
                           check("Browser check: Mod Manager Download asks: " + app.pickTitle + ", focus " + (f.ring === 1 ? "Not now" : "Get it") + ", A " + f.a + ", B " + f.b + ", page input " + (app.nexusView.view.enabled ? "on" : "off"))
                           Turbine.selftestPadAged("a", app.nxmShownAt > 0 ? Math.max(0, Math.round(Turbine.monoMs() - app.nxmShownAt - 300)) : 0)
                           check("Browser check: A at once on the question: " + (app.nxmAsking ? "ignored" : "taken")) } },
            // Two taps on Get it don't get it, and a page alert or confirm doesn't replace the question.
            { name: "mmd taps", after: 1200, act: () => {
                pickMenu.activated(0); pickMenu.activated(0)
                check("Browser check: two taps on Get it: " + (app.nxmAsking ? "still asking" : "got it"))
                app.browserDialog({ type: 0, message: "Tap here twice", dialogAccept: () => {}, dialogReject: () => {} })
                app.browserDialog({ type: 1, message: "OK?", dialogAccept: () => {}, dialogReject: () => {} })
                check("Browser check: a page alert and confirm while asking: " + (app.nxmAsking && app.pickTitle.indexOf("Download ") === 0 ? "the question stays" : "replaced it")) } },
            { name: "mmd get", after: 300, act: () => { pad("right"); pad("a"); check("Browser check: Right then A after a second gets it: " + (!app.nxmAsking ? "yes" : "no")) } },
            { name: "nxm", after: 2500, act: () => {
                check("Browser check: after Mod Manager Download the page is still the file page: " + (pageIs(/tab=files/) ? "yes" : "no, " + (app.nexusView ? app.nexusView.url : "")))
                app.browserNoKey("", "nxm://skyrimspecialedition/mods/1/files/2?key=abc&expires=4102444800&user_id=1") } },
            { name: "no key", after: 600, act: () => { btShot("nokey"); btLegend("no key message")
                const o = app.binds(); check("Browser check: the no-key note on its own file page leaves A " + o.a.label + " and B " + o.b.label)
                app.browserNoteNow = null; app.browserNoKey("", "nxm://skyrimspecialedition/mods/1/files/9?key=abc&expires=4102444800&user_id=1") } },
            { name: "no key elsewhere", after: 400, act: () => { const o = app.binds(); check("Browser check: the no-key note for another file's page: A " + o.a.label + ", B " + o.b.label); pad("b") } },
            { name: "close", after: 400, act: () => { check("Browser check: B on " + (app.nexusView && app.nexusView.canGoBack ? "a later page" : "the first page")); pad("b") } },
            { name: "closed", max: 5000, wait: () => app.page !== "browser",
              act: () => { check("Browser check: B on the first page closed it; focus " + (app.modPage && app.zone === "main" && app.pageIndex === 0 ? "on Get it" : "on " + app.zone + (app.modPage ? " (mod page row " + app.pageIndex + ")" : ""))) } },
            // This check's mod needs nothing else (the sample mod asks for SKSE, which keeps the focus on its page
            // and offers the next requirement: that way is checked by the chain pictures).
            { name: "get it", after: 500, act: () => { app.detailsData = Object.assign({}, app.detailsData, { needs: null }); app.chainQueue = []; check("Browser check: (the mod for the auto close needs nothing else)"); pad("a") } },
            { name: "file page again", max: 30000, wait: () => app.browserOpener === "getit" && pageIs(/tab=files/),
              act: () => { bt.pressAt = Date.now(); btClick(160, 188) } },
            { name: "slow download", max: 15000, wait: () => app.webDls.some(x => x.name === "TurbineTest-Bigger_Pockets-1.0.3.zip"),
              act: () => { check("Browser check: Slow download started " + Math.round((Date.now() - bt.pressAt) / 100) / 10 + " s after the press") } },
            { name: "auto close", max: 30000, wait: () => app.page !== "browser",
              act: () => { check("Browser check: after the add the page closed by itself; focus " + (app.zone === "play" ? "on Play" : "on " + app.zone)
                                 + "; My mods " + (app.mods.some(m => /bigger.pockets|turbinetest/i.test(m.name + " " + (m.sourceId || ""))) ? "lists it" : "doesn't list it (" + app.mods.map(m => m.name).join(", ") + ")"))
                           check("Browser check: the pointer is hidden after the close: " + (Turbine.pointerShown ? "no" : "yes"))
                           bt.launches = app.launchCount; pad("a") } },
            { name: "after the close", after: 300, act: () => check("Browser check: A just after the auto close started nothing: " + (app.launchCount === bt.launches && app.sheet === "" ? "yes" : "no")) }
        ])
    }
    // The second run: the profile kept the sign-in; then a page that tries everything by itself with no press (a Mod
    // Manager Download link from a script and from a frame, a pop-up, a download): nothing happens. Then a Slow
    // download, and the page closed while it runs: Turbine says it goes on and where to watch it.
    function browserPersistScript() {
        const pageIs = (re) => !!app.nexusView && !!app.nexusView.view && !app.nexusView.loading && re.test(app.nexusView.url)
        btRun([
            { name: "open", act: () => app.openNexus(btFiles + "&auto=1") },
            { name: "loaded", max: 45000, wait: () => !!app.nexusView && !!app.nexusView.view && !app.nexusView.loading && /tab=files/.test(app.nexusView.url),
              act: () => check("Browser check: second run loaded " + app.nexusView.url) },
            { name: "tries", after: 9000, act: () => { check("Browser check: no press, nothing downloaded: " + (app.webDls.length === 0 && !app.dl.active ? "yes" : "no (" + app.webDls.map(x => x.name).join(", ") + ")"))
                                                         // A link on mod 1's page to another mod's page that goes to its nxm link by itself.
                                                         btClick(700, 512) } },
            { name: "another mod asks", max: 15000, wait: () => app.nxmAsking && app.menuIndex === 0 && app.nxmShownAt > 0 && Turbine.monoMs() - app.nxmShownAt > 1400,
              act: () => { check("Browser check: another mod's link asks: " + app.nxmAskWords()); pad("b") } },
            { name: "auto page again", after: 600, act: () => { check("Browser check: Not now closed it: " + (app.sheet === "" ? "yes" : "no")); app.nexusView.load(app.browserAddress(btFiles + "&links=1")) } },
            // A on a link to an outside site that sends the view straight back to a Nexus page that goes to its
            // Mod Manager Download link by itself (after the page's own tries are over).
            { name: "outside link", max: 20000, wait: () => pageIs(/links=1/), after: 800, act: () => btClick(700, 452) },
            { name: "bounced", max: 20000, wait: () => pageIs(/file_id=55/), after: 6000,
              act: () => check("Browser check: after a bounce off Nexus and back, nothing downloaded: " + (app.webDls.length === 0 && !app.dl.active ? "yes" : "no")) },
            // An outside page read for a while; a tap on it sends the view to mod 1's page that does the same.
            { name: "outside page", after: 300, act: () => app.nexusView.load("http://127.0.0.1:47801/tapme") },
            { name: "tap outside", max: 15000, wait: () => pageIs(/tapme/), after: 800, act: () => btClick(190, 80) },
            { name: "tap off nexus asks", max: 15000, wait: () => app.nxmAsking && app.menuIndex === 0 && app.nxmShownAt > 0 && Turbine.monoMs() - app.nxmShownAt > 1400,
              act: () => { check("Browser check: a tap off Nexus asks: " + app.nxmAskWords()); pad("b") } },
            { name: "slow", max: 15000, wait: () => app.sheet === "" && pageIs(/file_id=56/), after: 600,
              act: () => { check("Browser check: after Not now, nothing downloaded: " + (app.webDls.length === 0 && !app.dl.active ? "yes" : "no")); bt.pressAt = Date.now(); btClick(160, 188) } },
            // The page closes while the download runs (the bounce left pages to go back to, so it closes outright).
            { name: "running", max: 20000, wait: () => app.webDlsRunning > 0, act: () => app.closeBrowser("back") },
            { name: "closed", max: 5000, wait: () => app.page !== "browser",
              act: () => check("Browser check: closed while downloading; at the top: " + (app.webDlsRunning > 0 ? "Getting " + app.plural(app.webDlsRunning, "mod", "mods") : "nothing") + "; game menu has Nexus downloads: " + (app.gameMenuItems.some(x => x.act === "nexusdl") ? "yes" : "no")) },
            { name: "landed", max: 20000, wait: () => app.webDls.some(x => x.state === "added" || x.state === "failed" || x.state === "asked"),
              act: () => check("Browser check: the download went on after the close: " + app.webDls.map(x => x.name + " " + x.state).join(", ")) }
        ])
    }
    // Helper reports: one with no finish, a refused finish, and the Slow download hint replaced.
    function regressScript() {
        const dl = shot.arg("regress-downloads")
        const page = "https://www.nexusmods.com/skyrimspecialedition/mods/1?tab=files&file_id=2"
        const started = (id, name) => Turbine.selftestCatch("bridge:" + JSON.stringify({ type: "download.started", id: id, url: "https://cf-files.nexus-cdn.com/x/" + encodeURIComponent(name),
            finalUrl: "https://cf-files.nexus-cdn.com/x/" + encodeURIComponent(name), filename: dl + "/" + name, referrer: page, mime: "application/zip", totalBytes: 1000 }))
        btRun([
            { name: "watch", act: () => { Turbine.selftestCatch("watch"); Turbine.selftestCatch("page:" + page) } },
            { name: "partial 1", after: 1500, act: () => Turbine.selftestCatch("partial:Regress Three 1.0.zip.crdownload") },
            { name: "partial 2", after: 1100, act: () => Turbine.selftestCatch("partial:Regress Three 1.0.zip.crdownload") },
            { name: "partial 3", after: 1100, act: () => Turbine.selftestCatch("partial:Regress Three 1.0.zip.crdownload") },
            { name: "no finish", after: 1500, act: () => { started("r1", "Regress One 1.0.zip"); Turbine.selftestCatch("drop:" + dl + "/Brighter Lanterns 1.2.zip|Regress One 1.0.zip") } },
            { name: "refused finish", after: 1000, act: () => { started("r2", "Regress Two 1.0.zip"); Turbine.selftestCatch("drop:" + dl + "/Lantern Glow SKSE 2.0.zip|Regress Two 1.0.zip") } },
            { name: "finish", after: 4500, act: () => Turbine.selftestCatch("bridge:" + JSON.stringify({ type: "download.finished", id: "r2", path: "/nonexistent/Regress Two 1.0.zip", size: 1000 })) },
            { name: "wait", after: 26000, act: () => check("Regress check: done") }
        ])
    }

    // The build's check: every action row in every Settings section (Labs on) has a handler.
    function settingsRowsCheck() {
        const all = app.accountRows.concat(app.updateRows, app.labsRows, app.helpRows)
        const acts = all.filter(r => r.kind === "action")
        const missing = acts.filter(r => !app.rowActions[r.key]).map(r => r.key)
        app.note("check", "Settings rows check: " + acts.length + " action rows, " + missing.length + " without an action" + (missing.length ? " (" + missing.join(", ") + ")" : ""))
    }
    property int pointerTries: 0
    function pointerCheck() {
        // A game whose store was still empty when it opened lands on Get mods (the test mods go in a moment later):
        // the click is meant for My mods, so it goes back there first, the way B does, and clicks once the list is
        // on screen (the hidden list's rows still answer itemAtIndex, at the place of the Get mods cards).
        if (!modList.visible || app.turnTo >= 0) {
            if (app.tab !== 0 && app.sheet === "" && !app.modPage) {
                app.note("pad", "Pointer check: My mods first (the game opened on " + (app.tabs[app.tab] || app.tab) + ")")
                app.tab = 0; app.zone = "main"
            }
            if (++pointerTries < 6) pointerRetry.start(); else app.note("pad", "Pointer check: My mods isn't on screen")
            return
        }
        const second = modList.itemAtIndex(1), first = modList.itemAtIndex(0)
        const it = second && second.kind === "mod" ? second : first && first.kind === "mod" ? first : null
        if (!it || !it.pill) {
            // The test mods may still be going in: look again shortly.
            if (++pointerTries < 6) pointerRetry.start(); else app.note("pad", "Pointer check: no mod row to click")
            return
        }
        const p = it.pill.mapToItem(null, it.pill.width / 2, it.pill.height / 2)
        Turbine.selftestPointer(p.x, p.y, true, 0)
    }
    Timer { id: pointerRetry; interval: 300; onTriggered: sc.pointerCheck() }
    function pointerScrollCheck() {
        const p = perfList.mapToItem(null, perfList.width / 2, perfList.height / 2)
        Turbine.selftestPointer(p.x, p.y, false, 12)
        scrollCheckNote.start()
    }
    // The build's picture of the end of the check before Play: wheel steps over it.
    Timer { id: checkEndShot; interval: 900; onTriggered: Turbine.selftestPointer(app.width / 2, app.height * 0.45, false, 40) }
    Timer { id: scrollCheckNote; interval: 1500; onTriggered: app.note("pad", "Scroll check: the list moved " + Math.round(perfList.contentY) + " px") }

    // ---------------------------------------------------------------- layout checks
    // Just before each picture is saved (on shotTimer and btFinal, beside their own handlers) a walk over the visible
    // items of the window in the picture logs one line per layout problem, then a summary (tools/ci-shots.sh collects
    // both):
    //   check   Layout problem (<page>): <kind>: <path> "<text>" [and <n> more like it]: <what> [<numbers>]
    //   check   Layout (<page>, <WxH>, size <n>%): <k> problems | <kind> <count>, ... | <a> allowed, <t> texts
    // <path> is the item's components from the window down ("SheetLayer > InfoSheet > Label"); the same problem on
    // several items with one path is one line. The kinds:
    //   cut       text elided or past its maximum lines (Text.truncated), or cut by a clip that isn't a scrolling list
    //   midword   prose broken inside a word (WrapAnywhere on more than one word, or a word wider than its line)
    //   overflow  text or a filled box past the box it sits in (a rounded Rectangle with a fill or a border), or past
    //             the window
    //   scroll    a sheet's form (a Flickable that isn't a list) with words or boxes cut or hidden by its edge
    //   bar       the bottom bar drops the A or B hint while the page or sheet binds that button (legendProblems)
    //   small     text drawn under layoutMinText pixels (fpx() never goes under 12)
    //   overlap   two sibling texts drawn over each other
    // An intentional case is allowed one of two ways. On the item, or any item above it, in its own QML file:
    // `property bool layoutOk: true` (every kind) or `property string layoutOk: "cut overflow"` (those kinds). Or
    // here in layoutAllow: the kind, a pattern for the path, optional patterns for the text and for what is wrong,
    // least (for an elided line: the share of it that must still show), and why. Allowed problems are counted in the
    // summary; --layoutall=1 lists them too ("Layout allowed (<page>): ...").
    readonly property int layoutMinText: 12
    readonly property int layoutMostLines: 30
    readonly property var layoutAllow: [
        { kind: "cut", path: /(^|> )GetMods >/, what: /^elided$/, why: "Get mods cards elide long names and summaries; the mod page shows them whole" },
        { kind: "cut", path: /(^|> )GameRail >/, what: /^elided$/, why: "the rail's names on the art elide; the bar and the game menu name the game whole" },
        { kind: "cut", path: /(^|> )(AddMods|ModDetails|ModPanel|ModRow|LoadOrderList) >/, what: /^elided$/, least: 0.5,
          why: "a long mod or file name on one line elides in its row, most of it still showing" },
        { kind: "cut", path: /^BottomBar > Text$/, what: /^elided$/, least: 0.5, why: "the bar's status line elides, most of it still showing; a tap or the toast says it whole" },
        { kind: "midword", path: /(^|> )ActivityLog >/, why: "log lines break anywhere so long paths fit" },
        { kind: "scroll", path: /(^|> )(GameMenu|ModMenu) >/, why: "a menu of rows scrolls with the focus, its last row fading as the cue" }
    ]
    FontMetrics { id: layoutFont }
    Connections { target: shotTimer; function onTriggered() { sc.layoutCheck() } }
    Connections { target: btFinal; function onTriggered() { sc.layoutCheck() } }

    // The type of an item: a component's file name, or the QtQuick type without its QQuick.
    function layoutType(it) {
        const m = /^(\w+?)((?:_QMLTYPE_\d+|_QML_\d+)*)\(/.exec(String(it))
        const n = m ? m[1] : "Item"
        return { name: n.replace(/^QQuick/, ""), component: !!(m && m[2]) && !/^QQuick|^QQml/.test(n) }
    }
    // An item's box in window coordinates (rotation and scale included): {x, y, r, b}.
    function layoutBox(it, x, y, w, h) {
        const a = it.mapToItem(null, x, y), b = it.mapToItem(null, x + w, y), c = it.mapToItem(null, x, y + h), d = it.mapToItem(null, x + w, y + h)
        return { x: Math.min(a.x, b.x, c.x, d.x), y: Math.min(a.y, b.y, c.y, d.y), r: Math.max(a.x, b.x, c.x, d.x), b: Math.max(a.y, b.y, c.y, d.y) }
    }
    function layoutCut(a, b) { return { x: Math.max(a.x, b.x), y: Math.max(a.y, b.y), r: Math.min(a.r, b.r), b: Math.min(a.b, b.b) } }
    function layoutEmpty(a) { return a.r - a.x < 0.5 || a.b - a.y < 0.5 }
    // How far a goes past b on each side, worst first ("12 px at the bottom"), or "" within tol px (1.5).
    function layoutPast(a, b, tol) {
        const sides = [["left", b.x - a.x], ["right", a.r - b.r], ["top", b.y - a.y], ["bottom", a.b - b.b]].filter(s => s[1] > (tol || 1.5))
        sides.sort((p, q) => q[1] - p[1])
        return sides.map(s => Math.round(s[1]) + " px at the " + s[0]).join(", ")
    }
    function layoutPlain(s) { return String(s).replace(/<[^>]*>/g, "").replace(/&nbsp;/g, " ").replace(/&amp;/g, "&").replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/\s+/g, " ").trim() }
    // A Text's drawn words in its own coordinates (alignment and padding taken in).
    function layoutTextArea(t) {
        const w = t.contentWidth, h = t.contentHeight
        const l = t.leftPadding || 0, r = t.rightPadding || 0, top = t.topPadding || 0, bot = t.bottomPadding || 0
        const ha = t.effectiveHorizontalAlignment !== undefined ? t.effectiveHorizontalAlignment : t.horizontalAlignment
        const x = ha === Text.AlignRight ? t.width - r - w : ha === Text.AlignHCenter ? l + (t.width - l - r - w) / 2 : l
        const y = t.verticalAlignment === Text.AlignBottom ? t.height - bot - h : t.verticalAlignment === Text.AlignVCenter ? top + (t.height - top - bot - h) / 2 : top
        return { x: x, y: y, w: w, h: h }
    }
    // Where a wrapped Text breaks inside a word ("It p|uts"), or "": its lines laid out again with its font.
    function layoutMidWord(t, s) {
        layoutFont.font = t.font
        const width = t.width - (t.leftPadding || 0) - (t.rightPadding || 0)
        if (width <= 0) return ""
        if (t.wrapMode === Text.Wrap) {
            // Only a word longer than a whole line breaks inside itself.
            for (const w of s.split(" ")) if (layoutFont.advanceWidth(w) > width + 0.5) return w.slice(0, 24) + "| (wider than its line)"
            return ""
        }
        let pos = 0
        for (let line = 0; pos < s.length && line < 60; line++) {
            let lo = pos + 1, hi = s.length
            while (lo < hi) { const mid = Math.ceil((lo + hi) / 2); if (layoutFont.advanceWidth(s.substring(pos, mid)) <= width + 0.5) lo = mid; else hi = mid - 1 }
            if (lo < s.length && /\S/.test(s[lo - 1]) && /\S/.test(s[lo])) return s.substring(Math.max(pos, lo - 12), lo) + "|" + s.substring(lo, lo + 6)
            pos = lo
            while (s[pos] === " ") pos++
        }
        return ""
    }

    function layoutCheck() {
        const page = shot.page
        const win = page === "overlay" || page === "overlaywait" ? panelWin : page === "overlaygm" || page === "caughtgm" ? hudWin : app
        if (!win || !win.contentItem) return
        const W = win.width, H = win.height
        const whole = { x: 0, y: 0, r: W, b: H }
        let problems = [], allowed = 0, texts = 0
        const okFor = (ok, kind) => ok === true || (typeof ok === "string" && ok.split(/[\s,]+/).indexOf(kind) >= 0)
        const quote = (text) => text !== "" ? " \"" + text.slice(0, 40).replace(/"/g, "'") + (text.length > 40 ? "..." : "") + "\"" : ""
        const add = (kind, path, text, what, numbers, ok, share) => {
            const line = kind + ": " + path + quote(text) + ": " + what + (numbers ? " [" + numbers + "]" : "")
            if (okFor(ok, kind) || layoutAllow.some(a => a.kind === kind && a.path.test(path) && (!a.text || a.text.test(text)) && (!a.what || a.what.test(what))
                                                    && (!a.least || (share !== undefined && share >= a.least)))) {
                allowed++
                if (shot.arg("layoutall") !== "") app.note("check", "Layout allowed (" + page + "): " + line)
                return
            }
            // The same problem on several items of one kind (every name on the rail) is one line.
            const same = problems.find(p => p.kind === kind && p.path === path && p.what === what)
            if (same) { same.more++; return }
            problems.push({ kind: kind, path: path, text: text, what: what, numbers: numbers, more: 0 })
        }
        const at = (a) => Math.round(a.r - a.x) + "x" + Math.round(a.b - a.y) + " at " + Math.round(a.x) + "," + Math.round(a.y)
        // The clip that cuts a box: the innermost whose own rect it goes past ({rect, kind, by}), or null.
        const cutBy = (a, clips, tol) => { for (let i = clips.length - 1; i >= 0; i--) if (layoutPast(a, clips[i].rect, i === 0 ? tol : 1.5) !== "") return clips[i]; return null }
        // ctx: op (opacity so far), clip (what is still seen), clips (each clip on the way down: the window, then
        // "box", "list" or "form" ones), box (the nearest rounded box: rect, path, clips), form (the sheet's form
        // around it, counting what its edge cuts), sheet (inside the sheet layer), path, ok (layoutOk from above).
        const walk = (it, ctx) => {
            if (!it || !it.visible || it === sc) return
            const op = ctx.op * it.opacity
            if (op < 0.05) return
            const ty = layoutType(it)
            const path = ty.component && it !== win.contentItem ? ctx.path.concat([ty.name]) : ctx.path
            const where = path.concat(ty.component ? [] : [ty.name]).join(" > ") + (it.objectName ? " \"" + it.objectName + "\"" : "")
            const ok = it.layoutOk === undefined ? ctx.ok : ctx.ok === true || it.layoutOk === true ? true : ((ctx.ok || "") + " " + (it.layoutOk || "")).trim()
            const r = layoutBox(it, 0, 0, it.width, it.height)
            const isText = typeof it.truncated === "boolean" && typeof it.elide === "number" && typeof it.text === "string"
            const isRect = ty.name === "Rectangle" && it.border !== undefined
            const filled = isRect && it.color.a > 0.1
            const isBox = isRect && it.radius > 0 && (filled || (it.border.width > 0 && it.border.color.a > 0.1)) && it.width >= 24 && it.height >= 16 && (r.r - r.x < W - 1 || r.b - r.y < H - 1)
            const flick = it.contentY !== undefined && it.contentHeight !== undefined && it.flicking !== undefined
            const list = flick && it.model !== undefined
            if (isText && it.color.a > 0.05 && layoutPlain(it.text) !== "") {
                texts++
                const s = layoutPlain(it.text)
                const ta = layoutTextArea(it)
                const drawn = layoutBox(it, ta.x, ta.y, ta.w, ta.h)
                if (ctx.form && layoutPast(drawn, ctx.form.rect) !== "" && ctx.form.cut++ === 0) ctx.form.first = "\"" + s.slice(0, 30) + "\""
                const seen = layoutCut(drawn, ctx.clip)
                if (!layoutEmpty(seen)) {
                    const share = it.wrapMode === Text.NoWrap && it.implicitWidth > 0 ? Math.min(1, it.width / it.implicitWidth) : undefined
                    if (it.truncated) add("cut", where, s, it.elide !== Text.ElideNone ? "elided" : "more than " + it.maximumLineCount + " lines",
                                          (share !== undefined ? "shows " + Math.round(share * 100) + "%, " : "") + at(r), ok, share)
                    else if (it.clip && (it.contentWidth > it.width + 1 || it.contentHeight > it.height + 1)) add("cut", where, s, "cut by its own clip", "words " + Math.round(it.contentWidth) + "x" + Math.round(it.contentHeight) + ", box " + at(r), ok)
                    const by = cutBy(drawn, ctx.clips, 1.5)
                    const past = by ? layoutPast(drawn, by.rect) : ""
                    if (by && (by.kind === "box" || by.kind === "window"))
                        add(by.kind === "window" ? "overflow" : "cut", where, s, by.kind === "window" ? "past the window" : "cut by " + by.by + "'s clip", past, ok)
                    else if (past === "" && ctx.box && !ctx.box.clips) {
                        const pb = layoutPast(drawn, ctx.box.rect)
                        if (pb !== "") add("overflow", where, s, "past its box (" + ctx.box.path + ")", pb, ok)
                    }
                    if ((it.wrapMode === Text.WrapAnywhere || it.wrapMode === Text.Wrap) && it.lineCount > 1 && (it.wrapMode === Text.Wrap || / \S/.test(s))) {
                        const m = layoutMidWord(it, s)
                        if (m !== "") add("midword", where, s, "breaks inside a word: " + m, (it.wrapMode === Text.WrapAnywhere ? "WrapAnywhere, " : "Wrap, ") + it.lineCount + " lines", ok)
                    }
                    const a = it.mapToItem(null, 0, 0), b = it.mapToItem(null, 100, 0)
                    const px = (it.font.pixelSize > 0 ? it.font.pixelSize : it.font.pointSize * 4 / 3) * Math.hypot(b.x - a.x, b.y - a.y) / 100
                    if (px < layoutMinText - 0.25) add("small", where, s, Math.round(px * 10) / 10 + " px, under " + layoutMinText, "", ok)
                }
            } else if (filled && it !== win.contentItem) {
                if (ctx.form && layoutPast(r, ctx.form.rect) !== "" && ctx.form.cut++ === 0) ctx.form.first = "a box " + at(r)
                const seen = layoutCut(r, ctx.clip)
                if (!layoutEmpty(seen)) {
                    // A side sheet may put its border and round corners past the window's edge.
                    const by = cutBy(r, ctx.clips, Math.max(it.border.width, it.radius) + 1.5)
                    const past = by ? layoutPast(r, by.rect, by.kind === "window" ? Math.max(it.border.width, it.radius) + 1.5 : 1.5) : ""
                    if (by && (by.kind === "box" || by.kind === "window"))
                        add(by.kind === "window" ? "overflow" : "cut", where, "", by.kind === "window" ? "a box past the window" : "a box cut by " + by.by + "'s clip", past + ", " + at(r), ok)
                    else if (past === "" && ctx.box && !ctx.box.clips) {
                        const pb = layoutPast(r, ctx.box.rect)
                        if (pb !== "") add("overflow", where, "", "a box past its box (" + ctx.box.path + ")", pb + ", " + at(r), ok)
                    }
                }
            }
            // Sibling texts drawn over each other.
            let sib = []
            for (let i = 0; i < it.children.length; i++) {
                const c = it.children[i]
                if (!c.visible || op * c.opacity < 0.05 || typeof c.truncated !== "boolean" || typeof c.text !== "string" || layoutPlain(c.text) === "" || c.color.a <= 0.05) continue
                const ca = layoutTextArea(c)
                const cr = layoutCut(layoutBox(c, ca.x, ca.y, ca.w, ca.h), ctx.clip)
                if (!layoutEmpty(cr)) sib.push({ t: c, r: cr })
            }
            for (let i = 0; i < sib.length; i++) for (let j = i + 1; j < sib.length; j++) {
                const o = layoutCut(sib[i].r, sib[j].r)
                if (o.r - o.x > 2 && o.b - o.y > 2) {
                    const cok = sib[i].t.layoutOk !== undefined ? sib[i].t.layoutOk : sib[j].t.layoutOk !== undefined ? sib[j].t.layoutOk : ok
                    add("overlap", where, layoutPlain(sib[i].t.text), "drawn over \"" + layoutPlain(sib[j].t.text).slice(0, 30) + "\"", Math.round(o.r - o.x) + "x" + Math.round(o.b - o.y) + " px", cok)
                }
            }
            // What the children see.
            let next = { op: op, clip: ctx.clip, clips: ctx.clips, box: ctx.box, form: ctx.form, sheet: ctx.sheet || it === sheetLayer, path: path, ok: ok }
            if (it.clip) {
                next.clip = layoutCut(ctx.clip, r)
                next.clips = ctx.clips.concat([{ rect: r, kind: list ? "list" : flick ? "form" : "box", by: where }])
                if (layoutEmpty(next.clip)) return
            }
            if (isBox) next.box = { rect: r, path: where, clips: !!it.clip }
            // A sheet's form: what its edge cuts or hides is counted on the way down.
            const form = flick && !list && ctx.sheet && it.interactive !== false ? { rect: r, cut: 0, first: "" } : null
            if (form) next.form = form
            for (let i = 0; i < it.children.length; i++) walk(it.children[i], next)
            if (form && form.cut > 0)
                add("scroll", where, "", form.cut + (form.cut === 1 ? " item" : " items") + " cut or hidden by its edge, first " + form.first,
                    "content " + Math.round(it.contentWidth) + "x" + Math.round(it.contentHeight) + ", box " + at(r) + ", scrolled to " + Math.round(it.contentY), ok)
        }
        walk(win.contentItem, { op: 1, clip: whole, clips: [{ rect: whole, kind: "window", by: "the window" }], box: null, form: null, sheet: false, path: [], ok: undefined })
        // The bottom bar: the hints on screen, checked by the legend checks' own rule (A and B only).
        if (win === app && legend && legend.visible && legend.shown) {
            let shown = []
            const hints = (it, op) => {
                for (let i = 0; i < it.children.length; i++) {
                    const c = it.children[i]
                    if (!c.visible || op * c.opacity < 0.05) continue
                    if (c.inBar === true && typeof c.k === "string") {
                        // Shown when whole inside the bar and every clip between them.
                        const cr = layoutBox(c, 0, 0, c.width, c.height)
                        let whole = true
                        for (let p = c.parent; p && whole; p = p === legend ? null : p.parent)
                            if (p.clip || p === legend) whole = layoutPast(cr, layoutBox(p, 0, 0, p.width, p.height)) === ""
                        if (whole) shown.push([c.k, c.label])
                    } else hints(c, op * c.opacity)
                }
            }
            hints(legend, legend.opacity)
            const o = app.binds()
            for (const x of sc.legendProblems(o, shown).filter(x => /^[AB] missing$/.test(x))) {
                const k = x[0].toLowerCase()
                add("bar", "BottomBar", "", "drops the " + x[0] + " hint (" + o[k].label + ") that " + (app.sheet !== "" ? "the sheet \"" + app.sheet + "\"" : "the page") + " binds",
                    "shown: " + (shown.map(h => h[0].toUpperCase()).join(" ") || "none") + " of " + app.legendHints.map(h => h[0].toUpperCase()).join(" "), undefined)
            }
        }
        const tag = page + ", " + W + "x" + H + ", size " + app.sizeNow + "%"
        for (let i = 0; i < Math.min(problems.length, layoutMostLines); i++) {
            const p = problems[i]
            app.note("check", "Layout problem (" + page + "): " + p.kind + ": " + p.path + quote(p.text) + (p.more ? " and " + p.more + " more like it" : "") + ": " + p.what + (p.numbers ? " [" + p.numbers + "]" : ""))
        }
        if (problems.length > layoutMostLines) app.note("check", "Layout problem (" + page + "): and " + (problems.length - layoutMostLines) + " more")
        let kinds = {}
        for (const p of problems) kinds[p.kind] = (kinds[p.kind] || 0) + 1
        app.note("check", "Layout (" + tag + "): " + problems.length + " problems" + (problems.length ? " | " + Object.keys(kinds).map(k => k + " " + kinds[k]).join(", ") : "")
                 + " | " + allowed + " allowed, " + texts + " texts")
    }
}
