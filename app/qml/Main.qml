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
        property int textSize: 100          // percent (before 0.5; moved into size once)
        property int size: 0                // Interface size, percent (80 to 150); 0 until set or moved over
        property bool highContrast: false
        property string vision: "typical"   // typical, red, green, blue, none
        property string layout: "handheld"  // handheld, desktop, tv (tv only with an external display)
        property string season: ""          // off, date, autumn, snow; "" until chosen (off on handhelds, by date on TV)
        property int fallAmount: 40         // 10 to 100
        property int fallSize: 100          // 50 to 200 percent
        property bool compactList: false    // 0.56: shorter rows in My mods
        property bool reduceMotion: false   // 0.56: no sliding or growing (0.62: read once, as Animations "reduced")
        property string animations: ""      // 0.62: full, reduced (fades only) or off (instant); "" until chosen
        property string confirmBy: "hold"   // 0.56: "hold" (A held 5 s) or "twice" (A, then A again) for Reset and delete
        property string holdStartDoes: "choice" // 0.62: holding Start 3 s: "choice" (the Play choice) or "last" (plays the last choice)
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
        property string reqQueue: ""        // 0.64: the requirements being got: {game, forName, items: [{name, modId, domain}], index}
        property string depMoves: "{}"      // 0.64: {game id: {mod id: "in" | "out"}}: the player moved a mod into or out of Dependencies
        property string played: "[]"        // game ids, last played first (the rail's order)
        property string bannerDone: "[]"    // games whose "No mods yet" line on Get mods was seen
        property bool tutorialDone: false
        property bool railHintDone: false   // "Up and Down pick a game, A opens it" was shown until a game was opened
        property string antiCheatDone: "[]" // games whose anti-cheat card was shown
        property string basicIntroDone: "[]" // Basic games whose first-visit card was shown
        property string playedWarn: "[]"    // "<game>|<mods on>|<warnings>" played through with Play anyway   // the four first-run cards were seen (Settings, Help shows them again)
        property string updateWarned: "[]"  // 0.58: "<game>|<build>": the game update warning was shown
        property string modHold: "{}"       // 0.58: games played without mods until their mods update: {build, versions}
        property string playChoice: "{}"    // 0.6: per game, the last Play choice with Seamless Co-op: mods, coop or plain
        property string coopTileAsked: "[]" // 0.62: games whose "Add a Co-op tile to Steam?" was asked once
        property string nexusGameAsked: "[]" // 0.62: games whose "Which game on Nexus is this?" was asked once
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
    // 0.56: each mod's name and words from its Nexus page ("<game>|<mod id>": {name, author, summary, icon}), kept
    // when Get mods or a mod page shows them, and the names the player gave mods ("<game>|<mod id>": name).
    Settings {
        id: namesSet
        category: "names"
        property string nexus: "{}"
        property string renamed: "{}"
    }
    // Settings > Accounts > Browser.
    Settings {
        id: browserSet
        category: "browser"
        property bool overlay: true         // Show status over other apps
        property bool addShown: false       // "Add Chrome to Steam" was shown once (Game Mode)
        property bool helperHintShown: false // "Set up the browser helper too" was shown once (Game Mode)
        property string oldHelperShown: ""  // "Your browser helper is out of date" was shown for this helper version
        property bool alwaysSteam: false    // Labs: Game Mode opens Nexus in Steam's browser straight away
        property string openIn: "turbine"   // Desktop Mode: "turbine" (Nexus inside Turbine) or "mine" (the player's browser)
        property bool labsChrome: false     // Labs: Game Mode opens Nexus in Chrome through Steam (0.53's way)
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
        // Pages that wait for the engine (imports, menus, the check) get more time before the picture.
        readonly property bool slow: ["pointer", "pointerscroll", "checkend", "caught", "nexusnokey", "mods",
                                      "getmods", "getmodsfallback", "modpagefree", "modpagepremium", "getmodsback", "emptygame", "emptygamemods",
                                      "removed", "newmod", "opening", "opening8s", "desktop", "desktopgrid", "tv", "rail", "menu", "gamemenu", "check", "add", "loader", "report", "browse", "moddetails", "fomod",
                                      "loadorder", "profiles", "accounts", "accountsgm", "updates", "lootsort", "lootnotes", "versions", "code",
                                      "nexusdetails", "nexusversions", "nexusfree", "nxm", "browser", "helper",
                                      "playoptions", "findmods", "selectmods", "overlaps", "quickexit", "chain", "listbar",
                                      "checkfocus", "menuskse", "addbrowser", "slowhint",
                                      "wheel", "ghost", "railmenu", "browserframe", "browsertest", "browserpersist", "regress",
                                      "basicgame", "place", "placestart", "overwrite", "overwriteskip", "nexuspick", "basicmenu", "tucked", "why", "findpop", "fullscroll", "acask", "acdeploy", "acguide", "acrefuse", "modsempty", "notstarted", "launchsetup", "modsettings", "coopsettings", "gameupdate", "ersaves"].indexOf(page) >= 0 || scripted
        // 0.54: pages that run a list of presses (the controls' acceptance checks) and take their picture at the end.
        readonly property bool scripted: ["controls", "legendcheck", "sheetcheck", "nexusguard", "inlinebrowser", "presscount", "inputcheck", "sizeinputs", "coopcheck", "editbutton", "menufit", "heldcheck", "abcheck", "passthrough", "golden", "mmdguard", "startcheck", "bookcheck", "cooppw", "keycheck", "tilecheck", "oldercheck", "protoncheck", "reqcheck"].indexOf(page) >= 0
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
            else if (page === "accounts" || page === "accountsgm") app.openSection("Getting mods")
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
            // 0.58: the Activity log opened the way the player opens it: Settings, Labs, its row, A (in 0.57 that
            // press did nothing). Every Settings action row has a handler, too.
            else if (page === "activity") {
                app.openSection("Labs"); app.zone = "settings"
                app.setRow = Math.max(0, app.labsRows.findIndex(r => r.key === "activity"))
                app.note("shot", "Settings row focused: " + (app.rowsNow[app.setRow] ? app.rowsNow[app.setRow].key : "none"))
                app.act("a")
                app.note("check", "Activity log sheet open after A on its Settings row: " + (app.sheet === "activity"))
                app.settingsRowsCheck()
            }
            else if (page === "slowhint") { Turbine.setSampleFree(); app.forceMyBrowser = true }
            // 0.54: the beginner tutorial's cards; pages for the new controls.
            else if (page.indexOf("tutorial") === 0) app.openTutorial(parseInt(page.substring(8)) - 1)
            else if (page === "nexusguard" || page === "inlinebrowser") { Turbine.setSampleFree(); app.tab = 1; app.zone = "main" }
            else if (page === "presscount") Turbine.setSampleFree()
            // 0.55: Nexus inside Turbine (the frame alone; the stand-in page under X11; the second run; the watcher).
            else if (page === "browserframe" || page === "browsertest") { Turbine.setSampleFree(); app.tab = 1; app.zone = "main" }
            else if (page === "regress") Turbine.setSampleFree()
            // 0.56, Mods for any game: Get mods for a game Nexus has under another name.
            else if (page === "nexuspick") { app.tab = 1; app.zone = "main" }
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
            // 0.55: Settings, Getting mods in Game Mode: "Open Nexus in" shows as a fact, the own-browser rows fold.
            else if (shot.page === "accountsgm") app.note("shot", "Getting mods rows (Game Mode " + Turbine.gamemode + "): " + app.accountRows.map(r => r.key).join(", "))
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
            // Play options; R3 Find with "glow" typed; Select several with one mod ticked.
            else if (shot.page === "playoptions") { app.openPlayOptions(); playOptionsShot.start() }
            else if (shot.page === "findmods") { app.tab = 0; app.zone = "main"; app.act("y"); app.act("y"); askView.open("Find a mod", "", "glow", "", 60, false, "Done"); askView.accept(); app.act("start") }
            else if (shot.page === "selectmods") { app.tab = 0; app.zone = "main"; app.modIndex = 0; app.startSelect(app.mods[0]); app.act("down") }
            // 0.54: a mod's file page opened on a free account: the status says to press Slow download.
            else if (shot.page === "slowhint") app.openNexus("https://www.nexusmods.com/witcher3/mods/2109?tab=files&file_id=1")
            else if (shot.page === "opening8s") { app.getArea = "fallback"; app.getAct("a"); app.openHelpNow() }
            // 0.51: the mod menu of a mod that loses a file; the closed-early prompt; the next requirement; the list bar.
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
            // 0.57: My mods' Y pop-up (Find, Show, Sort, Select several), with Show "On" chosen.
            else if (shot.page === "listbar") { app.tab = 0; app.zone = "main"; app.modIndex = 0; app.listShow = "on"; findPopShot.start() }
            // 0.57, anti-cheat games: Play asks once (Elden Ring); the mod goes into the me3 profile, and Before you
            // play; Halo's guide; an online-only game refusing mods.
            else if (["acask", "acdeploy", "acguide", "acrefuse"].indexOf(shot.page) >= 0) { app.tab = 0; app.zone = "main"; acShot.start() }
            else if (["modsettings", "coopsettings", "gameupdate", "ersaves"].indexOf(shot.page) >= 0) { app.tab = 0; app.zone = "main"; lateShot.start() }
            // 0.57: Get mods' Y pop-up and Get mods scrolled down to full screen, once the cards are in.
            else if (shot.page === "findpop" || shot.page === "fullscroll") { app.zone = "main"; app.tab = 1; findPopShot.start() }
            // 0.52: A on Before you play runs the highlighted button; the mod and game menus offer the missing loader.
            else if (shot.page === "checkfocus") { app.startPlay(); checkFocusShot.start() }
            else if (shot.page === "menuskse") {
                app.tab = 0; app.zone = "main"
                app.modIndex = Math.max(0, app.listItems.findIndex(x => x.kind === "mod" && x.m.needsLoader))
                app.note("shot", "Game menu: " + app.gameMenuItems.map(x => x.title).join("; "))
                app.sheet = "modmenu"
                app.note("shot", "Mod menu: " + app.modMenuTitles())
            }
            else if (shot.scripted) app.runScript(shot.page)
            else if (shot.page === "wheel") { app.tab = 0; app.zone = "main"; app.modIndex = 0; app.pad("view"); app.pad("up"); app.pad("right") }
            else if (shot.page === "ghost") { app.tab = 0; app.zone = "main"; app.modIndex = 0; if (app.mods[0]) app.uninstall(app.mods[0]) }
            else if (shot.page === "railmenu") { app.zone = "rail"; app.railIndex = app.current; app.pad("x") }
            else if (shot.page === "browserframe") app.openNexus(app.btFiles)
            else if (shot.page === "browsertest") app.browserCheckScript()
            else if (shot.page === "browserpersist") app.browserPersistScript()
            else if (shot.page === "regress") app.regressScript()
            // 0.56: Start on Where do these files go? (Put them here); A on the question about the game's own files
            // (Skip those files); the Nexus game question; and a Basic game's mod menu.
            else if (shot.page === "placestart") { app.note("shot", "Place page open: " + (app.page === "place")); app.placeAct("start"); Turbine.applyNow() }
            else if (shot.page === "overwriteskip") { app.note("shot", "Overwrite sheet open: " + (app.sheet === "msg")); app.act("a"); Turbine.applyNow() }
            else if (shot.page === "nexuspick") {
                // 0.62: the game opened on Get mods with its anti-cheat card up: the question waits for the card,
                // and comes by itself once B closes it (nexusPickShot looks).
                app.note("shot", "Nexus game question, before: asked by itself " + app.nexusAsks + " times, sheet \"" + app.sheet + "\"" + (app.sheet === "msg" && app.msgData ? " (" + app.msgData.title + ")" : ""))
                if (app.sheet !== "") app.pad("b")
                nexusPickShot.start()
            }
            else if (shot.page === "why") app.openWhy()
            // 0.58: the one-time Launch Options step for a me3 game, and the launch check's sheet.
            else if (shot.page === "launchsetup") { launchSetupShot.start(); app.openLaunchSetup({ game: app.game.id, name: "Elden Ring", option: "if [ -x \"/home/deck/.var/app/io.github.ravenor907.TurbineModManager/data/steam-launch/turbine-launch.sh\" ]; then \"/home/deck/.var/app/io.github.ravenor907.TurbineModManager/data/steam-launch/turbine-launch.sh\" %command%; else %command%; fi", set: "", listed: true, coop: false }) }
            else if (shot.page === "notstarted") {
                app.openNotStarted(app.game.id); app.note("shot", "Launch check sheet: " + app.pickTitle + " | " + app.pickItems.map(r => r.title).join("; "))
                // 0.62: what Turbine's Steam tool said about the last launch shows first, when it said anything.
                app.toolTest = { lastProton: "Started with Proton 9.0 (Beta): Steam used it for this game last time.", nothingStarted: "no Proton was found, so nothing started" }
                app.sheet = ""; app.openNotStarted(app.game.id)
                app.note("shot", "Launch check with the tool's words: " + app.pickItems.map(r => r.title + (r.info ? " (" + r.detail + ")" : "")).join("; "))
                app.sheet = ""; app.openWhy()
                app.note("shot", "Why won't it start with the tool's words: " + app.pickItems.filter(r => /Proton|Steam tool/.test(r.title)).map(r => r.title + ": " + r.detail).join("; "))
                app.sheet = ""; app.openNotStarted(app.game.id)
                // 0.62: it shows up late: the sheet goes. It started and closed: "closed after N seconds", never "didn't start".
                app.gameSeenLate(app.game.id, 31)
                app.note("shot", "Launch check, the game showed up late: sheet \"" + app.sheet + "\"")
                app.openNotStarted(app.game.id); app.quickExit(app.game.id, 24)
                app.note("shot", "Launch check, it started then closed: sheet " + app.sheet + ", " + infoView.title + " | " + app.quickText + " | " + app.infoButtons.map(b => b.label).join("; "))
                app.toolTest = null
                app.quick = null; app.sheet = ""; app.openNotStarted(app.game.id)
            }
            // 0.56: a mod renamed, then the list scrolled down a row (the bars and the rail go; the row stays in view).
            else if (shot.page === "tucked" && app.mods.length > 1) {
                app.tab = 0; app.zone = "main"; app.modIndex = 0
                app.renameMod(app.mods[0], "Lanterns of the Road")
                app.modIndex = 1
                // Two mods fit on one screen: Down doesn't hide the bars. The picture shows them hidden anyway.
                app.note("shot", "Short list keeps the bars: " + !app.barsHidden)
                app.note("shot", "Bottom bar: " + barName.text + " | " + app.barStatus + " | tabs " + (legend.tabWordsFit ? app.tabs.length : "caps only") + " | Play " + (playBtn.visible ? "shown" : "hidden") + " | top bar " + (header.height > 0 ? "shown" : "none"))
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
        // 0.57: pictures that wait for the cards or the engine, then press, then wait for what the press did.
        // 0.6: a scripted page takes its picture 1.5 s after its last step (shotAfterScript), 50 s at the latest.
        interval: shot.page === "splashmid" ? 520 : shot.page === "splashafter" ? 2400 : ["findpop", "fullscroll", "listbar", "acask", "acdeploy", "acguide", "acrefuse", "modsettings", "coopsettings", "gameupdate", "ersaves"].indexOf(shot.page) >= 0 ? 9000 : shot.page === "browsertest" ? 200000 : shot.page === "regress" ? 48000 : shot.page === "browserpersist" ? 110000 : shot.page === "inputcheck" ? 45000 : shot.scripted ? 50000 : shot.page.indexOf("loot") === 0 || shot.page.indexOf("nxm") === 0 || shot.page === "getmodsback" ? 6500 : shot.slow ? 4500 : 1500
        // The overlay pictures are of the overlay window itself.
        onTriggered: {
            // 0.58: every picture checks the bottom bar (nothing overlaps Play or goes past the edge).
            legend.check(shot.page)
            if (shot.page.indexOf("splash") === 0) app.note("check", "Splash check (" + shot.page + "): shown " + splash.shown + ", icon opacity " + splashIcon.opacity.toFixed(2) + ", scale " + splashIcon.scale.toFixed(2) + ", rail icon none")
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
    // 0.62: the Steam tool's steps: what the one sheet says, then A (Check again) with the tool not picked yet.
    Timer {
        id: launchSetupShot; interval: 700
        onTriggered: {
            const d = app.msgData, t = app.steamTool(app.game.id)
            app.note("shot", "Tool steps sheet: " + app.sheet + " | " + (d ? d.title : "") + " | " + (d && d.steps ? d.steps.length : 0) + " steps, first: " + (d && d.steps && d.steps[0] ? d.steps[0] : "")
                     + " | says after Steam's next start: " + (!!d && /after Steam's next start/.test(d.text)) + " (restart first " + t.restartFirst + ") | buttons " + app.infoButtons.map(b => b.label).join("; ") + " | Game Mode " + Turbine.gamemode)
            const asks = app.playAsks
            app.pad("a")
            app.note("shot", "Tool steps, Check again: sheet " + app.sheet + ", checks " + app.toolChecks + ", Play asked " + (app.playAsks - asks) + " times, route " + t.route)
        }
    }
    // 0.62: the Nexus game question came by itself once; a second try doesn't ask again.
    Timer {
        id: nexusPickShot; interval: 700
        onTriggered: {
            const rows = app.sheet === "pick" ? app.pickItems.length : 0, was = app.sheet === "pick" ? app.pickTitle : "\"" + app.sheet + "\""
            if (app.sheet === "pick") app.pad("b")
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
    // 0.58: Edit settings on a mod (Right, then Start saves), Seamless Co-op settings (the password first), the game
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
                    // 0.61 (I18): X opens the options; its first row shows the file as text; again as a list.
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
                    app.note("shot", "Co-op chosen at Play: " + (app.seamlessIn && !coopBtn.visible) + "; Play options: " + (app.seamlessIn ? "Play Seamless Co-op" : "none"))
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
                app.note("shot", "Get mods page: search row " + (app.getArea === "bar" ? "focused" : "none") + ", top bar " + (header.height > 0 ? "shown" : "none"))
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

    // ---------------------------------------------------------------- size
    // Layout: handheld, desktop (mouse in Desktop Mode: smaller, denser rows, 4 columns) or tv (only with an
    // external display). The whole interface is drawn from a design size and never grows past what fits the window:
    // 760 x 460 design units is the smallest it reflows into.
    readonly property string layoutNow: shot.layout !== "" ? shot.layout : (look.layout === "tv" && !externalShown) ? "handheld" : look.layout
    readonly property bool tv: layoutNow === "tv"
    readonly property bool desktopLayout: layoutNow === "desktop"
    readonly property int sizeDefault: tv ? 120 : desktopLayout ? 80 : 100
    readonly property real fit: Math.max(0.5, Math.min(width / 883, height / 514))
    // 0.58: Interface size 50% to 150% in 10% steps, with no cap: at large sizes the layout reflows (fewer grid
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
    // Text: never under 12 pixels, so 50% stays readable.
    function fpx(v) { return Math.max(12, Math.round(v * s)) }
    // Something to press: at least 44 physical pixels where the design size allows it.
    // 0.59: touch targets never shrink under a finger, whatever the Interface size: 44 pixels, or 6 mm on a dense
    // screen (the Ally X's 7 inch 1080p panel is about 12 pixels a millimetre).
    readonly property int touchFloor: Math.max(44, Math.round(Screen.pixelDensity * 6))
    function tpx(v) { return Math.max(touchFloor, Math.round(v * s)) }
    // Design width: 3 grid columns on a handheld, 4 from 1100 (TV and Desktop layouts), 2 below 860.
    readonly property real designWidth: width / s
    readonly property int gridColumns: designWidth < 640 ? 2 : designWidth < 760 ? 3 : 4
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
    onPageChanged: { note("page", "Page " + page); stopDangerHolds(); barsHidden = false; if (page === "settings") settingsFresh = true; else settingsFrom = null }
    // 0.57: no top bar on the game screen. Going down a long list hides the bottom bar and the rail too, so the
    // content is truly full screen; going up, reaching the top, View, a tab change, a sheet or the rail bring them
    // back. Reduce motion: no slide, they just go and come.
    property bool barsHidden: false
    readonly property bool gameScreen: page === "game" && !modPage
    // ---- 0.62: one motion system (Motion.qml): every duration and curve comes from it. Settings, Look, Animations:
    // Full, Reduced (fades only) or Off (instant). The build's checks run with it off, so no check races an
    // animation (--motion=full for the pass that only looks at where things end up; the splash pictures play it).
    readonly property string motionLevel: shot.on ? (shot.arg("motion") !== "" ? shot.arg("motion") : shot.page.indexOf("splash") === 0 ? "full" : "off")
                                          : look.animations !== "" ? look.animations : look.reduceMotion ? "reduced" : "full"
    Motion { id: mo; level: app.motionLevel }
    readonly property Motion motion: mo
    readonly property bool reduceMotion: !mo.moves
    // A list or grid brought to its focused row: `jump` puts it there (positionViewAtIndex and the like); at Full
    // the view then glides from where it was instead of snapping. One at a time, never an overshoot, and a new
    // one takes over from wherever the last had got to.
    NumberAnimation {
        id: glideAnim
        properties: "contentY"; duration: mo.moveBase; easing.type: mo.enter
        // It ends exactly where a jump would have put it (rows of different heights can move the mark on the way).
        onFinished: { const j = app.glideJump; app.glideJump = null; if (j) j() }
    }
    property var glideJump: null
    property int glides: 0
    function glide(view, jump) {
        if (!mo.moves || !view || !view.visible) { jump(); return }
        if (glideAnim.running) {
            glideAnim.stop(); glideJump = null
            // Another view's glide ends where it was going.
            if (glideAnim.target && glideAnim.target !== view) glideAnim.target.contentY = glideAnim.to
        }
        const from = view.contentY
        jump()
        const to = view.contentY
        if (Math.abs(to - from) < 1) return
        view.contentY = from
        glideAnim.target = view; glideAnim.from = from; glideAnim.to = to
        glideJump = jump
        glides++
        glideAnim.start()
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
    // ---- 0.62: Get mods is a page of a book. No game list there: My mods is the page to the left (its name and an
    // arrow down the left edge) and Load order the page to the right. Left from the first column, LB, a swipe to
    // the right or a click on the edge turns back; Right from the last column, RB, a swipe to the left or the right
    // edge turns forward. The page coming in slides over the one it covers, a brass line at its edge.
    readonly property bool bookTab: page === "game" && !!game && tab === 1
    readonly property bool bookPage: bookTab && !modPage
    readonly property bool bookHasNext: tabs.length > 2
    // The game list steps aside on Get mods (it stays while the focus is in it).
    readonly property bool railAway: barsHidden || (bookTab && zone !== "rail")
    property bool padRepeat: false        // the press being handled is a held D-pad or key repeating
    property int pageTurns: 0
    function turnPage(d) {
        // A held D-pad stops at the edge of the page; a new press turns it.
        if (padRepeat) return false
        const t = tab + d
        if (t < 0 || t >= tabs.length) return false
        pageTurns++
        note("action", "Page turned " + (d < 0 ? "back" : "forward") + " to " + tabs[t].replace(/ \(\d+\)$/, ""))
        switchTab(d)
        return true
    }
    property int turnFrom: -1             // the tab being covered while a page turns, else -1
    property int turnTo: -1
    property int turnDir: 0               // -1: the page comes in from the left (back), 1: from the right
    property real turnPart: 1
    property int turnsShown: 0
    property int tabBefore: 0
    property real gameChangedAt: 0        // a tab that changes with the game is no page turn
    // Full: the page slides in. Reduced: it fades in where it is (turnFade).
    readonly property real turnX: turnTo < 0 || !mo.moves ? 0 : turnDir * (1 - turnPart) * content.width
    readonly property real turnFade: turnTo >= 0 && !mo.moves ? 1 - turnPart : 0
    property int lastTurnDir: 0
    NumberAnimation {
        id: turnAnim
        target: app; properties: "turnPart"
        from: 0; to: 1; duration: mo.page; easing.type: mo.enter
        onFinished: { app.turnFrom = -1; app.turnTo = -1 }
    }
    function turnStart(from, to) {
        turnAnim.stop()
        turnFrom = from; turnTo = to; turnDir = to > from ? 1 : -1
        turnPart = 0
        turnsShown++
        turnAnim.start()
    }
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
        // Cinzel is only for the wordmark and the game's name in the header: every other title is Barlow.
        readonly property string titleFont: app.bodyFont
        readonly property string bodyFont: app.bodyFont
        function px(v) { return app.px(v) }
        function fpx(v) { return app.fpx(v) }
        function tpx(v) { return app.tpx(v) }
        function hover(item, on) { app.hover(item, on) }
        // 0.62: the motion system, and the glide to a focused row.
        readonly property Motion motion: mo
        function glide(view, jump) { app.glide(view, jump) }
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
    // ---------------------------------------------------------------- anti-cheat games (0.57)
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
        // 0.59 (I2): the first modded Play of a me3 game is one question and one press. What would have been sheets
        // of their own (the game update warning, me3 coming down) is said here; A plays.
        const upd = gameUpdate && gameUpdate.risky && gameUpdate.risky.length > 0 ? gameUpdate.risky.map(r => r.name) : []
        if (upd.length > 0 && game && !shot.on) { let w = parse(seen.updateWarned, []); w.push(game.id + "|" + gameUpdate.now); seen.updateWarned = JSON.stringify(w.slice(-50)) }
        const extra = (acInfo.me3Ready ? "" : " Turbine gets me3 (the free tool that loads the mods) now, once.")
            + (upd.length > 0 ? " " + gameTitle + " updated since mods were last on; these may not work until they update: " + upd.join(", ") + "." : "")
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
    // 0.62: the engine marks the one Seamless Co-op the game loads ("seamless": true in the mods' JSON), whatever
    // it came from: Turbine's own download or a copy from Nexus count the same. Every test for it reads that mark.
    readonly property bool seamlessIn: mods.some(m => m.seamless === true)
    readonly property var seamlessMod: mods.find(m => m.seamless === true) || null
    // 0.58: Play Seamless Co-op (Elden Ring through Steam and me3, its own .co2 saves). Turns it on first if it's off.
    // 0.62: every co-op start comes through here (the Play choice, Start held, Play options, the Co-op tile and
    // Steam's menu, "Use it again"): with no session password the mod stops with an error, so its settings open
    // on the password first, with the keyboard, and the start goes on once it is set.
    // ---- 0.62: two copies of Seamless Co-op in the list (one from Nexus and Turbine's own, or an old one). The
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
    // ---- 0.62: a Seamless Co-op file older than the one installed (an old download, or GitHub behind Nexus) is
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
        // 0.62: two copies in the list: the one sheet first (Tidy up or Not now), then this start goes on.
        if (coopCopies().length > 0) { openCoopTidy(() => startCoop()); return }
        if (coopPasswordEmpty()) { coopStops++; note("action", "Play Seamless Co-op: no session password, its settings open first"); openCoopPasswordFirst(); return }
        note("action", "Play Seamless Co-op")
        if (!seamlessMod.enabled) { Turbine.setEnabled(seamlessMod.id, true); mods = mods.map(x => x.id === seamlessMod.id ? Object.assign({}, x, { enabled: true }) : x) }
        startPlay(true)
    }
    property bool playCoopNext: false
    // ---- 0.6 (I3): with Seamless Co-op installed, Play (Start or a tap) asks one small question: Play with mods, Play
    // Seamless Co-op, Play without mods, and Co-op settings (Y) with whether the password is set. The last choice is
    // remembered and has the ring, so A starts it at once; B goes back.
    readonly property string coopOnlyWords: "Seamless Co-op loads only with Play Seamless Co-op"
    readonly property string lastPlayChoice: game ? (parse(seen.playChoice, {})[game.id] || "mods") : "mods"
    property string coopPassword: "?"     // "set", "not set" or "?" (not read yet)
    function rememberPlay(k) { if (!game) return; let c = parse(seen.playChoice, {}); c[game.id] = k; seen.playChoice = JSON.stringify(c) }
    function playPressed() {
        if (!game) return
        if (!seamlessIn || acRefused) { startPlay(); return }
        readCoopPassword()
        const keys = ["mods", "coop", "plain"]
        // 0.62: said where the choice is made: Seamless Co-op loads only with Play Seamless Co-op.
        openPick("Play " + gameTitle, coopOnlyWords + ". A plays the highlighted one.", [
            { icon: "play", title: "Play with mods", detail: "Solo, " + plural(mods.filter(m => m.enabled && !isSeamless(m)).length, "mod", "mods") + " on. Seamless Co-op stays out.", key: "mods", run: () => { rememberPlay("mods"); startPlay() } },
            { icon: "play", title: "Play Seamless Co-op", detail: "With friends, and your mods. Everyone uses the same password and settings.", key: "coop", run: () => { rememberPlay("coop"); startCoopChecked() } },
            { icon: "play", title: "Play without mods", detail: "Your mods stay installed and come back next time", key: "plain", run: () => { rememberPlay("plain"); Turbine.playOriginal() } },
            { icon: "info", title: "Co-op settings", detail: "Password: " + (coopPassword === "?" ? "not read yet" : coopPassword), key: "settings", aLabel: "Open", run: () => openCoopSettings() }
        ], "")
        pickY = { label: "Co-op settings", run: () => openCoopSettings() }
        pickNoStart = true
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
    // 0.58: a me3 game's Steam Launch Options don't run Turbine's helper yet. In Game Mode Steam shows a game only when
    // Steam started it, so the modded launch goes through Steam: the one-time step, with the exact text to copy.
    property var launchSetupData: null
    function openLaunchSetup(o) {
        launchSetupData = o
        note("action", "Launch Options step for " + o.name + ": needs \"" + o.option + "\", set \"" + o.set + "\"")
        if (o.undone === true) {
            openMsg({ title: "Use the launch options line again?", text: "You undid the launch options change for " + o.name + ", so Turbine doesn't start it with mods through Steam. Use the line again to play with mods, or play without them.",
                      buttons: [{ key: "a", label: "Use it again", run: () => { Turbine.launchRedo(o.game); note("action", "Launch Options: used again"); if (o.coop === true) startCoop(); else startPlay() } },
                                { key: "x", label: "Play without mods", run: () => Turbine.playOriginal() },
                                { key: "b", label: "Not now", run: () => {} }] })
            return
        }
        // 0.62: Steam starts the game through Turbine's own Steam tool, "Turbine (mods)", picked once in the game's
        // Properties. One sheet: the steps in plain words (the engine's, Turbine.steamToolJson), Check again (A),
        // and Set it up myself (the Launch Options line to paste) as the other way. Turbine no longer restarts
        // Steam to write Steam's settings in Game Mode (Steam wrote its own copy back); in Desktop Mode that way
        // is still offered.
        const tool = steamTool(o.game)
        if (tool.steps.length > 0) { openToolSteps(o, tool); return }
        if (Turbine.gamemode) { openLaunchPaste(o); return }
        // No Steam tool to offer here (Steam's folder wasn't found): the Desktop Mode way, as before.
        askSteamRestart({ game: o.game, kind: "options", resume: o.coop === true ? "coop" : "play", name: o.name,
                          title: o.outdated === true ? "Update " + o.name + "'s launch setting" : "One step before " + o.name + " plays with mods",
                          intro: "In Game Mode a game shows only when Steam starts it, so Turbine starts " + o.name + " through Steam. Steam needs one setting for that, and Turbine can set it for you."
                                 // 0.6 (break L9): the player's own Launch Options step aside while mods are on; say which, and how they come back.
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
    // ---- 0.62: "Proton for modded play": the Proton Turbine's Steam tool runs the game with (Turbine.protonJson:
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
        pickNoStart = true
        menuIndex = p.how === "you" ? Math.max(0, 1 + p.list.findIndex(x => x.current)) : 0
    }
    // 0.62, for the launch check and Why won't it start?: what Turbine's Steam tool did at the last launch, when it
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
                toast.info(now.restartFirst ? "Not yet: Steam has to start again before it lists " + name + "." : "Not picked yet. The steps are still here.") } },
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
            // 0.59 (newbie H7): Copy keeps the steps on screen; the button says it's copied. B closes.
            { key: "a", label: "Copy it", keep: true, run: () => {
                Turbine.copyText(o.option); note("action", "Launch option copied: " + o.option)
                msgData = Object.assign({}, msgData, { buttons: msgData.buttons.map(x => x.key === "a" ? Object.assign({}, x, { label: "Copied (copy again)" }) : x) })
                toast.done("Copied. The steps stay here until B.") } },
            { key: "x", label: "Play without mods", run: () => { note("action", "Launch Options step: played without mods"); Turbine.playOriginal() } },
            { key: "b", label: "Not now", run: () => note("action", "Launch Options step: not now") }
            // 0.6 (I21): Y is only ever Start outside Steam, and only in Desktop Mode (the activity log has its own place).
            ].concat(Turbine.gamemode ? [] : [{ key: "y", label: "Start outside Steam", run: () => {
                note("action", "Launch Options step: started outside Steam (Desktop Mode)"); Turbine.playOutsideSteam(o.coop === true) } }]) })
    }
    // ---------------------------------------------------------------- 0.58: Edit settings
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
        // 0.6: after a mod update, the settings Turbine couldn't carry over for sure come first, with the words.
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
    // 0.6 (I12): every change saves itself a moment after it is made (the copy before it is kept, Reset to default
    // goes back), like everything else in Turbine: nothing to remember to save, and B never loses a change.
    Timer { id: settingsAutosave; interval: 700; onTriggered: if (app.sheet === "settingsedit" && settingsView.changes > 0) app.settingsSaveNow(true) }
    Connections { target: settingsView; function onChangesChanged() { if (settingsView.changes > 0 && app.sheet === "settingsedit") settingsAutosave.restart() } }
    function settingsSaveNow(quiet) {
        settingsAutosave.stop()
        if (settingsView.changes === 0) { if (quiet !== true) toast.info("Saved already: changes save by themselves."); if (coopAfterSettings) coopGoOn(); return }
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
    // ---------------------------------------------------------------- 0.6 (I1): one Steam restart, asked first
    // Steam's own settings (a game's launch setting, the co-op tile) change only while Steam is closed. Turbine asks
    // once, with the warning, then a small helper on the device closes Steam, makes the change and starts Steam
    // again; in Game Mode Turbine closes with Steam and comes back through its Steam entry, then goes on.
    property int steamTick: 0                  // bumps when Steam's settings changed (the game menu reads them again)
    property real steamSetupSince: 0
    function askSteamRestart(o) {
        // 0.62: never in Game Mode (Steam starts again at once there and writes its own copy back, so the change
        // was lost): the do-it-myself steps instead, or plain words when there are none.
        if (Turbine.gamemode) {
            note("action", "No Steam restart in Game Mode for " + o.name + " (" + o.kind + ")" + (o.manual ? ": the steps to do it by hand" : ""))
            if (o.manual) { o.manual(); return }
            openMsg({ title: o.title, text: o.intro + "\n\nSteam's own settings can't be changed from Game Mode. Open Turbine in Desktop Mode for this.", buttons: [{ key: "b", label: "OK", run: () => {} }] })
            return
        }
        note("action", "Steam restart asked for " + o.name + " (" + o.kind + ")")
        openMsg({ title: o.title,
                  text: o.intro + "\n\nTurbine needs to restart Steam once to finish setting up " + o.name + ". Steam will close and open again; any game or download in progress will stop. Save your game first if one is running.",
                  buttons: [{ key: "a", label: o.aLabel, run: () => runSteamRestart(o) }]
                           .concat(o.manual ? [{ key: "x", label: o.manualLabel, run: o.manual }] : [])
                           .concat([{ key: "b", label: "Not now", run: () => note("action", "Steam restart: not now") }]) })
    }
    function runSteamRestart(o) {
        const r = parse(Turbine.steamSetupStart(o.game, o.kind, o.resume), { ok: false, state: "", words: "Turbine couldn't start the restart." })
        note("action", "Steam restart for " + o.name + ": " + (r.ok ? r.state : "refused: " + r.words))
        if (!r.ok) {
            openMsg({ title: "Steam can't restart right now", text: r.words,
                      buttons: (o.manual ? [{ key: "x", label: o.manualLabel, run: o.manual }] : []).concat([{ key: "b", label: "OK", run: () => {} }]) })
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
                      buttons: [{ key: "y", label: "Activity log", run: () => openActivity() }, { key: "b", label: "OK", run: () => {} }] })
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
        if (lost.length > 0) toast.info("Steam put back its own settings for " + lost.join(", ") + " (it was still running). Play or the game menu sets them up again.")
    }
    // The co-op tile in Steam's library (with Seamless Co-op installed).
    readonly property bool coopTileThere: steamTick >= 0 && !!game && seamlessIn && parse(Turbine.coopTileJson(game.id), { there: false }).there === true
    // ---- 0.62: how the tile can be added here and now is the engine's call (Turbine.coopTileOfferJson(game id),
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
    // 0.62: Get Seamless Co-op goes to its Nexus page first (eldenring mod 510, the engine's
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
        // 0.62: on opening a game with Seamless Co-op (and when it arrives): are there two copies?
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
            openMsg({ title: add ? "Add a Co-op tile to Steam" : "Remove the Co-op tile from Steam", text: offer.words || "The Co-op tile can't be changed from here.", buttons: [{ key: "b", label: "OK", run: () => {} }] })
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
    // ---------------------------------------------------------------- 0.58: Undo the launch options change
    function openLaunchUndo(current) {
        const list = parse(Turbine.launchUndoJson(current), [])
        note("action", "Undo the launch options change: " + list.map(x => x.name + (x.uses ? " (uses it)" : "")).join("; "))
        if (list.length === 0) { toast.info("No game's Steam Launch Options run Turbine's helper."); return }
        if (list.length === 1) { showLaunchUndo(list[0]); return }
        openPick("Undo the launch options change", "Pick the game", list.map(x => ({ icon: "restore", title: x.name, detail: x.original !== "" ? "Back to: " + x.original : "Back to empty", aLabel: "Show", run: () => showLaunchUndo(x) })), "")
    }
    function showLaunchUndo(x) {
        // 0.6 (I1): Turbine puts the old setting back itself with one Steam restart; doing it by hand is the fallback.
        askSteamRestart({ game: x.game, kind: "undo", resume: "", name: shortGame(x.name),
                          title: "Undo the launch options change: " + shortGame(x.name),
                          intro: "Turbine puts back " + shortGame(x.name) + "'s launch setting as it was before: " + (x.original !== "" ? x.original : "empty") + ". Steam then starts the game normally, with its anti-cheat.",
                          aLabel: "Restart Steam and put it back", manualLabel: "Do it myself", manual: () => showLaunchUndoPaste(x) })
    }
    function showLaunchUndoPaste(x) {
        Turbine.launchUndo(x.game)
        const back = x.original
        openMsg({ title: "Undo the launch options change: " + shortGame(x.name),
                  text: "Turbine's helper starts nothing for " + shortGame(x.name) + " now. To take its line out of Steam too, put back what was there before:\n\n" + (back !== "" ? back : "(nothing: leave Launch Options empty)"),
                  steps: ["In Steam: " + shortGame(x.name) + ", the gear, Properties, General, Launch Options.",
                          back !== "" ? "Replace Turbine's line with the text above (A copies it)." : "Delete everything there.",
                          "Close Properties. Steam starts the game normally, with its anti-cheat."],
                  buttons: (back !== "" ? [{ key: "a", label: "Copy it", run: () => { Turbine.copyText(back); toast.done("Copied. Paste it in Steam: Properties, Launch Options.") } }] : [])
                           .concat([{ key: "b", label: "Close", run: () => {} }]) })
    }
    // ---------------------------------------------------------------- 0.58: Elden Ring's saves, each kind apart
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
    // ---------------------------------------------------------------- 0.58: the game update warning
    property var gameUpdate: null
    function maybeGameUpdate() {
        if (!game || !gameUpdate || !gameUpdate.risky || gameUpdate.risky.length === 0 || page !== "game" || sheet !== "" || wheelOpen) return
        // 0.6: an update still waiting in Steam warns once too (before it installs), then again once it installed.
        const waiting = !!gameUpdate.pending && gameUpdate.last === gameUpdate.now
        const key = game.id + "|" + gameUpdate.now + (waiting ? "|waiting" : "")
        let warned = parse(seen.updateWarned, [])
        if (warned.indexOf(key) >= 0) return
        if (!shot.on) { warned.push(key); seen.updateWarned = JSON.stringify(warned.slice(-50)) }
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
    // 0.58, the launch check: the game didn't show about 20 seconds after Play.
    // 0.62: said only while it is true. A game that shows up later (a slow start) takes the sheet away, and one
    // that started and then closed is told as "<game> closed after N seconds", with the likely cause and its
    // buttons, never as "didn't start".
    property string notStartedFor: ""     // the game the "didn't start" sheet on screen is about
    function notStartedShowing(gameId) { return sheet === "pick" && notStartedFor === gameId && / didn't start$/.test(pickTitle) }
    function gameSeenLate(gameId, seconds) {
        const g = games.find(x => x.id === gameId)
        const name = g ? shortGame(g.name) : gameTitle
        note("launch", name + " showed up " + seconds + " seconds after Play" + (notStartedShowing(gameId) ? ": the didn't start sheet goes" : ""))
        if (notStartedShowing(gameId)) { sheet = ""; toast.info(name + " is running now. It took " + seconds + " seconds to start.") }
        notStartedFor = ""
    }
    function openNotStarted(gameId) {
        const g = games.find(x => x.id === gameId)
        const name = g ? shortGame(g.name) : gameTitle
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
        openPick(name + " didn't start", "It wasn't running about 20 seconds after Play. The likely fixes:", rows, "")
        pickNoStart = true
    }
    // My mods: the pinned "+ Get mods" row, the mods, then one collapsed row for Recently removed.
    // 0.64: mods that are mainly what other mods need (another installed mod lists it, or it is a known framework or
    // loader) go into one collapsed row at the bottom, "Dependencies (5)"; A opens it. X options move a mod in or
    // out. Only the list changes: the load order stays exactly as it is. Moving, Select several and Find show the
    // whole list.
    readonly property bool depsGrouped: !moving && !orderMoving && !selecting && findText === ""
    property bool depsOpen: false
    readonly property var depMoveMap: { const all = parse(seen.depMoves, {}); return game && all[game.id] ? all[game.id] : {} }
    function isDep(m) {
        const mv = depMoveMap[m.id]
        if (mv === "out") return false
        if (mv === "in") return true
        if (coreIds.indexOf(m.id) >= 0) return true
        const l = linksMap[m.id]
        return !!l && (l.neededBy || []).some(id => mods.some(x => x.id === id))
    }
    function moveDep(m, into) {
        let all = parse(seen.depMoves, {})
        let g = Object.assign({}, all[game.id] || {})
        g[m.id] = into ? "in" : "out"
        all[game.id] = g
        seen.depMoves = JSON.stringify(all)
        note("action", m.name + (into ? ": moved into Dependencies" : ": moved out of Dependencies"))
        toast.done(into ? m.name + " is in Dependencies now." : m.name + " is with your mods now.")
        const i = listItems.findIndex(x => x.kind === "mod" && x.m.id === m.id)
        if (i >= 0) modIndex = i
    }
    property var coreIds: []
    readonly property var depMods: depsGrouped ? shownMods.filter(m => isDep(m)) : []
    readonly property var listItems: {
        const deps = depMods
        let l = shownMods.filter(m => deps.indexOf(m) < 0).map(m => ({ kind: "mod", m: m }))
        if (deps.length > 0) l = l.concat([{ kind: "deps", m: { id: "", name: "" } }]).concat(depsOpen ? deps.map(m => ({ kind: "mod", m: m, dep: true })) : [])
        // 0.64: requirements still to get (Later, Stop, or Turbine closed meanwhile): one row at the top goes on.
        if (reqLeft && !reqRunning && !selecting) l.unshift({ kind: "needsq", m: { id: "", name: "" } })
        // 0.6: mods turned off from Steam's menu (Mods off) say so first, with the way back on that row.
        if (heldOff.length > 0 && !selecting) l.unshift({ kind: "held", m: { id: "", name: "" } })
        // Just uninstalled: the row stays, dimmed, for 8 seconds ("Removed", A puts it back), then the next row.
        if (ghost && game && ghost.game === game.id) l.splice(Math.max(0, Math.min(ghost.index, l.length)), 0, { kind: "ghost", m: { id: ghost.id, name: ghost.name } })
        return l.concat(removedMods.length > 0 ? [{ kind: "removed", m: { id: "", name: "" } }] : [])
    }
    property var ghost: null              // {game, id, name, index}
    // 0.6: the ids of mods that Mods off in Steam's menu turned off and that are still off.
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
    // Find (R3): only the mods whose name has every word typed, as it is typed.
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
    function needsAttention(m) { return (m.needsLoader && loaderMissing) || (m.enabled && (missingNeeds(m).length > 0 || isHidden(m) || !!modWarns[m.id])) }
    readonly property var shownMods: {
        const words = findText.toLowerCase().split(/\s+/).filter(w => w !== "")
        let list = mods.filter(m => {
            // A mod just turned on or off under Show stays in its row until the ring moves (controls #9), so the next
            // mod never slides under the ring and a second A never changes it.
            if (keptRow === m.id) return true
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
    property string keptRow: ""
    onListShowChanged: keptRow = ""
    readonly property bool listFiltered: findText !== "" || listShow !== "all" || listSort !== "order"
    function openFind() {
        note("action", "Find a mod")
        findBefore = findText
        openAsk("find", "Find a mod", "Shows only the mods whose names have these words.", findText, "Part of a mod's name", 60, false, "Show them")
    }
    // ---------------------------------------------------------------- the Y pop-up (0.57)
    // One pop-up for finding. Get mods: Search, Sort and Category (and From when the game has two sources). My
    // mods: Find, Show and Sort (Select several is in a mod's X menu). 0.59 (R1): A on a choice puts it into effect at
    // once; B, Start or a tap outside is Done and keeps everything; X clears all. A on the field (or Y, or a tap) types
    // right in it; Enter shows the results. A long category list ends in More..., which lists every category.
    property var findPop: ({ kind: "get", text: "", sort: 0, cat: "", src: "", show: "all", order: "order" })
    property int findRow: 0               // 0 the field, then one row per section
    property int findCol: 0
    readonly property int findCatMax: 6
    function sortNamesFor(src) { return src === "nexus" ? ["Trending", "Most liked", "Most downloaded", "Recently updated", "Newest"] : ["Most downloaded", "Top rated", "Recently updated", "Newest"] }
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
    readonly property int findRows: 2 + findSections.length   // the field, the sections, Done and Clear all
    function openFindPop() {
        if (!game || sheet !== "") return
        // 0.59: Get mods' pop-up whenever Get mods is the tab (a mod page on it closes, with its history), My mods'
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
                    // The ring goes onto the chosen category (controls #25), and it takes effect at once.
                    const sc = findSections[findRow - 1]
                    findCol = sc && sc.chosen >= 0 ? sc.chosen : 0
                    findPopApply(false)
                } })), "findpop")
            pickNoStart = true
            menuIndex = now
            return
        }
        if (sec.key === "source") { if (sources[c] !== (f.src || srcNow)) { f.src = sources[c]; f.sort = 0; f.cat = "" } }
        else if (sec.key === "sort") { if (f.kind === "mods") f.order = sortKeys2[c]; else f.sort = c }
        else if (sec.key === "show") f.show = showKeys[c]
        else if (sec.key === "category") f.cat = getCats[sec.values[c]]
        findPop = f
        note("action", "Find pop-up: " + sec.title + " " + sec.options[c])
        // 0.59 (R1): a choice takes effect the moment it is made.
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
        // Get mods: the results always end on the grid with no mod page over it (controls #2).
        modPage = false; history = []
        const changed = (f.src && f.src !== srcNow) || f.text !== getText || f.sort !== getSortIndex || f.cat !== getCategory
        if (f.src && f.src !== srcNow) { getSource = f.src; browseData = null; catList = [] }
        getText = f.text; getSortIndex = f.sort; getCategory = f.cat
        if (tab !== 1) tab = 1
        zone = "main"; getArea = "grid"; getIndex = 0; gridReturn = -1
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
    function openPick(title, sub, items, back) { pickTitle = title; pickSub = sub; pickItems = items; pickBack = back || ""; pickBLabel = ""; pickNoStart = false; menuIndex = 0; sheet = "pick" }
    // A list that only tells (Why won't it start?): Start doesn't play from it, and rows marked info have no A.
    property bool pickNoStart: false
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
            { icon: "play", title: "Play without mods", detail: "Your mods stay installed and come back next time", run: () => Turbine.playOriginal() }
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
            { icon: "remove", title: "Delete", detail: pressTwice ? "Press A twice to delete it for good" : "Hold A to delete it for good", danger: true, run: () => askDelete("profile:" + n, "Delete the loadout " + n + " for good? Its mods stay installed.") }
        ]), "profiles")
    }
    // Load order: the mod that placed a plugin.
    property var orderOwners: ({})
    readonly property string pluginOwner: orderPlugin ? (orderOwners[orderPlugin.name.toLowerCase()] || "") : ""
    function goToMod(id) {
        note("action", "Go to its mod: " + id)
        // 0.59 (controls #14): always lands on My mods with the ring on the mod, from anywhere (Settings, Play, the
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
            // 0.6 (integration I11): a store's name with an em or en dash shows with a colon ("Witcher 3: Remastered").
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
            // 0.62: a game that opens on Get mods before its rules are read is asked about once they are.
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
            app.askDelete("saves:" + id, "Turbine couldn't keep your saves as they are now first (" + why.replace(/^.*?first: /, "") + "). Put the backup back anyway? Your saves as they are now are replaced for good.")
        }
        function onChecked(json) { app.checked(app.parse(json, [])) }
        function onLaunch(url) {
            app.playAt = Date.now(); app.playGame = app.game ? app.game.id : ""
            // 0.59 (newbie H9, check on the device): Heroic starts its games itself, outside Steam; in Game Mode the game
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
            // 0.59: an answer for an older question (a quick second choice, a source switch) is dropped, so the grid
            // never shows Trending while the pop-up says Most liked (controls #7).
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
            app.gridReturn = -1
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
            // 0.6 (intent I16): Nexus's own name, author, summary and picture for mods added from a file.
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
        const order = parse(seen.played, []), pins = pinnedIds, hide = hiddenIds
        let list = allGames.map((g, i) => ({ g: g, i: i })).filter(x => hide.indexOf(x.g.id) < 0 || (x.g.id === shot.arg("game") && hiddenTest === null))
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
        // 0.59 (controls #17): a find, show or sort from one game never follows into the next.
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
    // 0.62 (the owner's setup): Desktop Mode with no controller seen is normal (Steam's desktop layout drives Turbine
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
        // 0.59 (newbie H2): an error is never only the bar's small second line. A short one is a toast; a long one
        // (Play couldn't start, with the reason) a sheet that stays until OK. In Settings, where the bar has no
        // status, every result shows as a toast.
        if (level === "error") {
            if (text.length > 80 && sheet === "" && !shot.strictSheets) openMsg({ title: "That didn't work", text: text, steps: [], notes: [], footnote: "", buttons: [{ key: "a", label: "OK" }] })
            else toast.problem(text)
        } else if (page === "settings" && (level === "ok" || level === "warn") && text !== "") toast.info(text)
    }
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
        if (openState === "open") return { level: slow ? "busy" : "ok", words: slow ? status.text : "Nexus is open in " + openName + ". Sign in if asked (free), press Slow download, then Steam button, Turbine.", detail: "" }
        // 0.57: an online-only anti-cheat game says so before anything else (the refused file's message is in the log).
        if (acRefused) return { level: "warn", words: "Needs its anti-cheat: no mods", detail: "" }
        // 0.6 (I6): mods held after a game update are never a hidden state.
        if (modHold && mods.some(m => m.enabled)) return { level: "warn", words: "Mods paused: " + gameTitle + " updated", detail: "", hold: true }
        if (msgShown) return { level: ({ work: "busy", ok: "ok", warn: "warn", error: "stop" })[msgLevel] || "ok", words: msgText, detail: "" }
        if (needLoader.length > 0)
            return { level: "warn", words: plural(needLoader.length, "mod needs ", "mods need ") + loaderShort, detail: "The game still plays, without " + (needLoader.length === 1 ? "that one" : "those") }
        const warned = mods.filter(m => m.enabled && !!modWarns[m.id])
        if (warned.length > 0)
            return { level: "warn", words: warned.length === 1 ? warned[0].name + ": " + modWarns[warned[0].id] : plural(warned.length, "mod", "mods") + " may not work here", detail: "Before you play says why" }
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
            openPick(plural(users.length, "mod needs", "mods need") + " " + m.name, users.map(x => x.name).slice(0, 3).join(", ") + (users.length > 3 ? " and " + (users.length - 3) + " more" : "") + ". They will stop working.", [
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
    // Touch reorder (0.59, newbie H4): with Move on, a tap on another row puts the mod there and sets it down.
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
        // (controls #15).
        if (tab !== 0 || modPage) { toast.done(m.name + " uninstalled. Removed mods, at the bottom of My mods, puts it back for 7 days."); return }
        ghost = { game: game ? game.id : "", id: m.id, name: m.name, index: at }
        ghostTimer.restart()
        modIndex = at
    }
    // ---------------------------------------------------------------- Recently removed (one collapsed row; a sheet)
    property int removedIndex: 0
    function openRemoved() { removedIndex = 0; sheet = "removed" }
    function removedRun(i) {
        // 0.59 (T10): the mods first, Empty last, so the dangerous row is never the first one.
        if (i === removedMods.length) askDelete("all", "Delete all " + plural(removedMods.length, "removed mod", "removed mods") + " for good? They can't be put back.")
        else if (removedMods[i]) putBack(removedMods[i].id)
    }
    function removedDelete(i) {
        const m = removedMods[i]
        if (m) askDelete(m.id, "Delete " + m.name + " for good? It can't be put back.")
    }
    // Hold A to confirm (data that can't come back): the same hold the game menu's Reset uses.
    property string confirmWhat: ""
    property string confirmText: ""
    // 2 seconds for a mod already in Removed mods; 5 for a loadout or profile.
    readonly property int holdNeeded: confirmWhat.indexOf("profile:") === 0 || confirmWhat.indexOf("saves:") === 0 ? 5 : 2
    // What the confirm does, in its title and button: put saves back over the ones there, or delete for good.
    function confirmWords() { return confirmVerb === "delete" ? "delete for good" : confirmVerb === "remove it" ? "remove the leftover file" : "replace your saves" }
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
        if (w === "all") { forgetNames(removedMods.map(x => x.id)); Turbine.purgeAllRemoved(); toast.done("Removed mods deleted for good.") }
        else { const m = removedMods.find(x => x.id === w); forgetNames([w]); Turbine.purgeRemoved(w); toast.done((m ? m.name : "The mod") + " deleted for good.") }
        removedIndex = Math.min(removedIndex, Math.max(0, removedMods.length - 1))
    }
    function putBack(id) {
        note("action", "Put back " + id)
        nudgeRow(id, 0)
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
        const i = listItems.findIndex(x => x.kind === "mod" && x.m.id === id)
        if (i < 0 || !mods[i] || mods[i].id !== id) return
        modIndex = i
        moving = true
        moveFrom = modIndex
        moveSaved = mods.slice()
    }
    // d rows (Up/Down 1, Left/Right 10); LT/RT pass a large number for top or bottom.
    // 0.62: a row that just moved, came back or arrived slides or fades into its place (the list is rebuilt when
    // it changes, so the row is told where it came from: {id, dir: -1 from above, 1 from below, 0 a fade, at}).
    property var rowNudge: null
    function nudgeRow(id, dir) { rowNudge = mo.on ? { id: id, dir: dir, at: Date.now() } : null }
    function stepMove(d) {
        const from = modIndex
        const to = Math.max(0, Math.min(mods.length - 1, from + d))
        if (to === from) return
        let list = mods.slice()
        const m = list.splice(from, 1)[0]
        list.splice(to, 0, m)
        nudgeRow(m.id, to > from ? -1 : 1)
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
    // 0.59 (T1): a loader Turbine can't download: A opens its page in Turbine (works in Game Mode), no two steps.
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
        // 0.57: an anti-cheat game me3 starts: asked once before the first Play with mods.
        if (acAskNeeded()) { openAcAsk(() => startPlay(coop === true)); return }
        if (acRefused && mods.some(m => m.enabled)) { toast.problem(acInfo.refuseWords); return }
        // 0.58: held after a game update: without mods until they update.
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
        Turbine.check()
    }
    readonly property string launchTitle: "One-time Steam setting for the mod loader"
    function launchSeen() { return game && parse(seen.launchDone, []).indexOf(game.id) >= 0 }
    function onceSeen(title) { return game && parse(seen.onceDone, []).indexOf(game.id + "|" + title) >= 0 }
    function checked(list) {
        // 0.59 (newbie H6): the loader's one-time Steam setting goes only once Steam's Launch Options really have the
        // line (read back from Steam); when they can't be read, once it has been seen.
        const lfNow = launchFinding(healthNow())
        if (lfNow ? lfNow.info === true : launchSeen()) list = list.filter(x => x.title !== launchTitle)
        list = list.filter(x => !(x.once && x.title !== launchTitle && onceSeen(x.title)))
        // 0.64: mods that are on but miss what they need (Virtual Car Dealer without Codeware and TweakXL).
        const need = reqOnAll()
        if (need.words !== "") list = list.filter(x => x.level !== "ok").concat([{ level: "warn", title: reqCheckTitle, detail: need.words + " Without them those mods may not load, or stop the game." }])
        // 0.6 (I14): one warning for one game update. Once its sheet was answered (Play without mods until they
        // update, or Keep my mods on), Before you play doesn't ask again about the same update.
        if (game && gameUpdate && (modHold || parse(seen.updateWarned, []).some(k => k.indexOf(game.id + "|" + gameUpdate.now) === 0)))
            list = list.filter(x => !/ updated this game$/.test(String(x.title || "")))
        if (list.length === 0) list = [{ level: "ok", title: "Ready to play", detail: "Everything checks out." }]
        // 0.56: plain words. Web addresses come out of the text (Game menu, Mods for this game on Nexus, or the
        // loader's own row get there), and "Missing master" says what it means.
        list = list.map(x => {
            const url = (String(x.detail || "").match(/https?:\/\/[^\s)]+/) || [""])[0].replace(/[.,]$/, "")
            const detail = String(x.detail || "").replace(/\s*Get it from https?:\/\/[^\s)]+[.,]?/g, url ? " Turbine shows where it comes from." : "")
                                                  .replace(/\s*\(?https?:\/\/[^\s)]+\)?/g, "").replace(/\s+/g, " ").trim()
            const title = x.title === "Missing master" ? "Needs a mod that isn't on" : x.title
            return Object.assign({}, x, { title: title, detail: detail, url: url })
        })
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
            playNow()
            toast.info("Played with the same warnings as last time: " + list.filter(x => x.level === "warn").map(x => x.title).join("; "))
            return
        }
        if (trouble || shot.page === "check" || shot.page === "checkend" || shot.page === "checkfocus") {
            // The highlight starts on the fix; else Not now when something stops play; else Play anyway.
            checkIndex = checkOffer ? 0 : checkStops ? checkChoices.indexOf("Not now") : Math.max(0, checkChoices.indexOf("Play anyway"))
            sheet = "check"
        }
        else playNow()
    }
    function playKey(list) {
        return (game ? game.id : "") + "|" + mods.filter(m => m.enabled).map(m => m.id).sort().join(",") + "|" + list.filter(x => x.level === "warn").map(x => x.title).sort().join(",")
    }
    readonly property bool checkStops: checks.some(x => x.level === "stop")
    readonly property string vcTitle: "The Visual C++ runtime is missing"
    // One fix the check can do in a press (Y): turn off the newer of two Witcher 3 mods that change the same script,
    // or install the missing loader (SKSE, BepInEx, SMAPI).
    readonly property var checkOffer: {
        const w = checks.find(x => x.offer && x.offer.mod && mods.some(m => m.id === x.offer.mod && m.enabled))
        if (w) return { label: w.offer.label, mod: w.offer.mod }
        // 0.64: Cyberpunk's frameworks without Microsoft's Visual C++ runtime in the game's Proton setup (error 998).
        if (checks.some(x => x.title === vcTitle)) return { label: "Install the Visual C++ runtime", vc: true }
        if (checks.some(x => x.title === reqCheckTitle) && (reqOnAll().get.length > 0 || reqOnAll().off.length > 0)) return { label: "Get what they need", reqs: true }
        if (loaderMissing && needLoader.length > 0) return { label: (loaderCanGet ? "Install " : "Get ") + loaderShort, loader: true }
        // 0.6 (I25): low space, and Turbine is keeping removed mods: the way to free that space is one press.
        if (removedMods.length > 0 && checks.some(x => x.title === "Low space on the game's drive")) return { label: "Free space: Removed mods", removed: true }
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
        const tool = steamTool(gameId)
        quick = { game: gameId, name: shortName, seconds: secs, added: added, worked: worked, proton: tool.lastProton || "", nothing: tool.nothingStarted || "" }
        page = "game"; zone = "main"; tab = 0; modPage = false
        // 0.62: it started and closed: a "didn't start" sheet still up for it gives way to "closed after N seconds".
        if (notStartedShowing(gameId)) { note("launch", "The didn't start sheet gives way: it started, then closed"); sheet = "" }
        notStartedFor = ""
        if (app.active && sheet === "") { infoIndex = 0; sheet = "quick" }
    }
    readonly property string quickText: !quick ? "" : (quick.added.length === 1 ? "Since it last worked, you added " + quick.added[0].name + "."
        : quick.added.length > 1 ? "Since it last worked (" + when(quick.worked.savedAt) + "), you added or turned on " + nameList(quick.added.map(m => m.name)) + "."
        : "It started, then closed by itself. A mod is the likely cause: playing without mods shows whether the game itself is fine.")
        // 0.62: which Proton Turbine's Steam tool ran it with, when the tool said.
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
    property bool chainKeep: false
    property string chainAfter: ""
    function showChain(arrived) {
        if (chainQueue.length === 0) return
        chainAfter = arrived
        if (sheet === "") { infoIndex = 0; sheet = "chain" }
    }

    // ---------------------------------------------------------------- 0.64: requirements, lined up
    // A mod that needs others: one sheet, "This mod needs N others", with Get them all. The whole chain is followed
    // (each mod got adds what it needs in turn), installed mods that are off are turned on, and the rest are got one
    // after another: with Nexus Premium (or Thunderstore) with no press, with a free account on the mod's own file
    // page, lined up on its Slow download button; after that press and the add, the next page opens by itself. A
    // strip says "Getting 2 of 3: <name>" with Skip and Stop. The list is kept (Later, Stop, closing Turbine) and My
    // mods keeps a row to go on where it left off. Where requirements come from: each mod author's list on Nexus.
    property bool reqRunning: false
    property var reqLookFor: null         // {id, name}: a mod just added, looked at once its requirements are known
    readonly property var reqQ: { const q = parse(seen.reqQueue, null); return q && q.items ? q : null }
    readonly property bool reqLeft: !!reqQ && !!game && reqQ.game === game.id && reqQ.index < reqQ.items.length
    readonly property var reqItem: reqQ && reqQ.index < reqQ.items.length ? reqQ.items[reqQ.index] : null
    readonly property string reqStripWords: reqRunning && reqItem ? "Getting " + (reqQ.index + 1) + " of " + reqQ.items.length + ": " + reqItem.name : ""
    function reqSave(q) { seen.reqQueue = q ? JSON.stringify(q) : "" }
    // What a mod needs that isn't on: installed but off (turned on by Get them all), and not installed (got).
    function reqSplit(m) {
        const miss = missingNeeds(m)
        return { off: miss.filter(n => !!n.id && mods.some(x => x.id === n.id)).map(n => mods.find(x => x.id === n.id)),
                 get: miss.filter(n => !n.id && n.modId > 0).map(n => ({ name: n.name, modId: n.modId, domain: n.domain || (rules ? rules.domain : "") })) }
    }
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
            const q = Object.assign({}, reqQ)
            const fresh = r.get.filter(x => !q.items.some(y => y.modId === x.modId))
            q.items = q.items.slice(0, q.index).concat(fresh).concat(q.items.slice(q.index))
            reqSave(q)
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
                  text: m.name + " needs these to work. Get them all gets each one in turn" + (account.premium === true ? "." : ": for each, press Slow download on its page and the next one opens by itself."),
                  notes: r.off.map(x => x.name + ": installed, turned off (it is turned on)").concat(r.get.map(x => x.name + ": not installed")),
                  footnote: "From the list the mod's author keeps on Nexus.",
                  buttons: [{ key: "a", label: "Get them all", run: () => reqStart(m, r) },
                            { key: "b", label: "Later", run: () => { reqSave({ game: game.id, forName: m.name, items: r.get, index: 0 }); for (const x of r.off) if (!x.enabled) toggle(x, true); note("action", "Requirements: later (My mods keeps a row)") } }] })
    }
    function reqStart(m, r) {
        for (const x of r.off) if (!x.enabled) toggle(x, true)
        if (r.get.length === 0) { toast.done("Turned on what " + m.name + " needs."); return }
        reqSave({ game: game.id, forName: m.name, items: r.get, index: 0 })
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
        if (account.premium === true && it.domain === (rules ? rules.domain : "")) {
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
        const q = reqQ
        if (!q) return
        const done = q.items[q.index]
        reqSave(Object.assign({}, q, { index: q.index + 1 }))
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
        reqSave(null)
        if (modPage) { modPage = false; history = [] }
        page = "game"; tab = 0; zone = "play"
        note("action", "Requirements: done for " + (q ? q.forName : ""))
        toast.done("Got everything " + (q ? q.forName : "it") + " needs. Hold " + glyph("start") + " to play.")
    }
    // Every mod that is on: what they miss together (each once), and words naming who needs what.
    readonly property string reqCheckTitle: "Mods need others that aren't on"
    function reqOnAll() {
        let off = [], get = [], names = [], lines = []
        for (const m of mods.filter(x => x.enabled)) {
            const miss = missingNeeds(m)
            if (miss.length === 0) continue
            names.push(m.name)
            if (lines.length < 3) lines.push(m.name + " needs " + miss.map(n => n.name).join(" and ") + ".")
            const r = reqSplit(m)
            for (const x of r.off) if (!off.some(y => y.id === x.id)) off.push(x)
            for (const x of r.get) if (!get.some(y => y.modId === x.modId)) get.push(x)
        }
        if (names.length > 3) lines.push("And " + plural(names.length - 3, "more mod", "more mods") + ".")
        return { off: off, get: get, names: names, words: lines.join(" ") }
    }
    // A mod arrived: its requirements are looked at once they are known (the list comes a moment after the add).
    function reqArrived(id, name) {
        reqLookFor = { id: id, name: name }
        reqLook.restart()
    }

    // ---------------------------------------------------------------- game menu
    // The setup to go back to: the newest one the game was played with (the engine's restore target).
    readonly property var lastGood: {
        const played = setups.filter(x => x.played === true).sort((a, b) => b.savedAt - a.savedAt)
        return played.length > 0 ? played[0] : null
    }
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
    property bool bHeld: false
    property bool clickHeld: false
    property int holdsDone: 0
    function holdAllowed() { return aHeld || clickHeld }
    // 0.56, Confirm by: Press twice: the first press says what a second one does; a second within 4 seconds does it.
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
        // The game menu's row and the confirm sheet's button say it already (T39): no toast over the bottom bar.
        if (sheet !== "gamemenu" && sheet !== "confirm") toast.info("Press A again to " + what + ".")
    }
    function holdHint(what) { toast.info("Hold A for " + what + ". A tap doesn't do it.") }
    // Stops every hold and the D-pad repeat: the window going to the back, a controller going away.
    function stopHolds(why) {
        const was = resetHold.running || confirmHold.running || homeHold.running || startHold.running || repeater.running
        stopReset(); stopConfirm()
        homeHold.stop(); startHold.stop(); startHeld = 0
        repeater.stop(); repeatButton = ""
        aHeld = false; bHeld = false; clickHeld = false
        if (was) note("pad", "Holds stopped (" + why + ")")
    }
    // The danger holds stop whenever the sheet or the page changes.
    function stopDangerHolds() { stopReset(); stopConfirm(); startHold.stop(); startHeld = 0; clickHeld = false; disarm() }
    // (onPageChanged and onTabChanged, near the top, call it too.)
    onModPageChanged: { stopDangerHolds(); pageSettings = false }
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
                   : { icon: "info", title: acInfo.askTitle, detail: "Mods load through me3 (the free tool that loads them), offline only", ok: true, act: "acask" }
    ] : []).concat(acMode === "me3" && !acInfo.me3Ready ? [
        { icon: "update", title: "Get me3, the tool that loads the mods", detail: "From its makers, checked. Turbine gets it at the first Play with mods anyway.", ok: true, act: "me3" }
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
        { icon: "info", title: "How to play " + gameTitle + " with mods", detail: "Its own Play choice without anti-cheat", ok: true, act: "acguide" }
    ] : []).concat(overlapCount > 0 ? [
        { icon: "info", title: "Overlaps: " + plural(overlapCount, "mod shares", "mods share") + " files", detail: "Which mod's copy the game gets", ok: true, act: "overlaps" }
    ] : []).concat(updateCount > 0 ? [
        { icon: "update", title: "Update all", detail: plural(updateCount, "mod has", "mods have") + " a newer version", ok: true, act: "updateall" }
    ] : []).concat(webDls.length > 0 && !browserOpen ? [
        { icon: "update", title: "Nexus downloads", detail: webDlsRunning > 0 ? plural(webDlsRunning, "download", "downloads") + " running · added by themselves" : "What Turbine's Nexus page downloaded", ok: true, act: "nexusdl" }
    ] : []).concat([
        // 0.56: the obvious actions for a game.
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
        { icon: "info", title: "Help", detail: "The buttons, and the beginner tutorial again", ok: true, act: "help" }
    ]).filter(x => !!x).map(x => Object.assign({}, x, { group: gameMenuGroup(x.act) }))
      .sort((a, b) => gameMenuOrder.indexOf(a.act) - gameMenuOrder.indexOf(b.act))
    // 0.59 (R4): the game menu in four sections, always in this order (rows that don't apply are left out; rows that
    // apply but can't be used now are shown dimmed, and A on one says why).
    readonly property var gameMenuOrder: ["unhold", "coopsettings", "cooptile", "original", "restore", "acguide", "acask", "acon",
        "add", "updateall", "checkupdates", "alloff", "allon", "profiles", "overlaps", "outdated", "nexusdl", "nexuslist",
        "why", "verify", "saves", "ersaves", "loader", "me3", "seamless", "acleftover", "kcdrestore", "launchundo", "reset",
        "gamesettings", "proton", "details", "activity", "help"]
    function gameMenuGroup(a) {
        const i = gameMenuOrder.indexOf(a)
        return i < gameMenuOrder.indexOf("add") ? "Play" : i < gameMenuOrder.indexOf("why") ? "Mods" : i < gameMenuOrder.indexOf("gamesettings") ? "Fix a problem" : "This game"
    }
    // Every saved setup, newest first ("Nov 4, 9:12 PM · 16 mods on · Played"); A puts one back.
    // ---------------------------------------------------------------- 0.56: health
    function healthNow() { return parse(Turbine.gameHealth(), {}) }
    // Launch option doctor: what the game needs in Steam's Launch Options, what is set, and the steps.
    function launchFinding(h) {
        const need = (h.launch && h.launch.needed) || "", set = (h.launch && h.launch.set) || ""
        if (need === "") return null
        if (h.launch.ok === true || (h.launch.ok === undefined && set.indexOf(need.replace(/ ?%command%/, "").trim()) >= 0)) return { icon: "power", title: "Steam's Launch Options: set", detail: "Steam has what " + gameTitle + " needs", info: true, run: () => {} }
        // 0.59: settings typed in front of Turbine's line without "export" stop Steam starting the game at all.
        if (h.launch.fix === true)
            return { icon: "info", title: "Steam's Launch Options: a setting in front breaks the line", detail: "Steam starts nothing like this. A copies the same line with export in front. Then in Steam: " + gameTitle + ", the gear, Properties, Launch Options, paste.",
                     run: () => { Turbine.copyText(need); note("action", "Launch option fix copied: " + need); toast.done("Copied. Paste it in Steam: Properties, Launch Options.") } }
        // 0.59 (T14): the steps, not the 150 character line; the line is what A copies.
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
        pickNoStart = true
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
        // A dimmed row says why it can't be used (R4, T5).
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
        const words = on ? name + ": added and on. A on Play starts it, or hold " + glyph("start") + "." : name + ": added, turned off. A turns it on."
        // The build's pictures add their test mods this way: only the new-mod picture (and the press count) follow them.
        if (shot.on && shot.page !== "newmod" && shot.page !== "presscount" && shot.page.indexOf("browser") !== 0) return
        // Nexus is open in Turbine: the landing waits for it to close (by itself 2.5 seconds after a single file).
        if (browserOpen && page === "browser" && !landing) { browserArrived(gameId, id, name, on); return }
        // 0.64: the requirements being got: the next one comes (and what this one needs goes in after it).
        if (reqRunning && game && gameId === game.id) {
            note("focus", "New mod while getting requirements: " + name)
            if (reqItem) reqSave(Object.assign({}, reqQ, { index: reqQ.index + 1 }))
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
            toast.done(game && game.id === gameId ? words : "Added to " + (g ? g.name : "its game") + ": " + name + ".")
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
    // 0.56: what covers a hidden mod ("Covered by Performance Tweak").
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
        // 0.6 (I24): what it still needs stays one press away (also after Later on the requirements).
        const missing = menuMod.enabled ? missingNeeds(menuMod) : []
        const first = fix.concat(missing.length > 0 ? [
            { icon: "update", title: "Get what it needs: " + (missing[0].name || missing[0]), detail: missing.length > 1 ? "And " + plural(missing.length - 1, "more mod", "more mods") + "; finds it in Get mods" : "Finds it in Get mods", act: "needs", need: missing[0].name || missing[0] }
        ] : []).concat(menuMod.needsLoader && loaderMissing ? [
            { icon: "update", title: "Get " + loaderShort, detail: loaderCanGet ? "This mod needs it. Turbine gets it from its official page." : "This mod needs it. Shows where it comes from.", act: "loader" }
        ] : []).concat(updateMap[menuMod.id] ? [
            { icon: "update", title: "Update to " + updateMap[menuMod.id].to, detail: (updateMap[menuMod.id].from ? "You have " + updateMap[menuMod.id].from + ". " : "") + "Keeps its place and on or off", act: "update" }
        ] : [])
        // 0.58: Edit settings, for any mod with a settings file (.ini, .toml, .json, .cfg, .yaml, .xml).
        const cfgs = settingsFilesOf(menuMod.id)
        const edit = cfgs.length === 0 ? [] : [isSeamless(menuMod)
            ? { icon: "info", title: "Co-op settings", detail: "Seamless Co-op: the session password, then every other option", act: "settings" }
            : { icon: "info", title: "Edit settings", detail: (cfgs.some(f => f.review) ? "Check after the update · " : "") + (cfgs.length === 1 ? cfgs[0].rel.split("/").pop() + (cfgs[0].edited ? " · your version" : "") : plural(cfgs.length, "settings file", "settings files")), act: "settings" }]
        const src = menuMod.sourceKind === "nexus" && rules && rules.domain && nexusModOf(menuMod) !== "" ? "nexus"
                  : menuMod.sourceKind === "thunderstore" && menuMod.sourceId ? "thunderstore" : ""
        const page = src === "nexus" ? [{ icon: "play", title: "Its Nexus page", detail: "Opens it in Turbine", act: "page" }]
                   : src === "thunderstore" ? [{ icon: "play", title: "Its Thunderstore page", detail: "Opens in Turbine", act: "files", src: src }] : []
        const check = src !== "" && !updateMap[menuMod.id] ? [{ icon: "update", title: "Check for an update", detail: "Asks " + (src === "nexus" ? "Nexus" : "Thunderstore") + " now", act: "checkupdate" }] : []
        // 0.59 (R3): actions only, always in the same order; missing rows are left out, never dimmed. The facts
        // (overlaps, needs, where it goes) are in Details.
        const order = ordered && mods.length > 1 ? [
            { icon: "move", title: "Move", detail: "Up and Down move it, A puts it down (or tap where it goes)", act: "move" }
        ] : []
        const names = [{ icon: "info", title: "Rename", detail: menuMod.renamed ? "Now " + menuMod.name : "A name of your own for the list", act: "rename" }]
        const select = mods.length > 1 ? [{ icon: "files", title: "Select several", detail: "Tick this and others to change them together", act: "select" }] : []
        // 0.64: in or out of the Dependencies group (only the list changes, never the load order).
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
    // The details panel's "X Options:" line: the first row (often a fix), Rename, Uninstall, and how many more.
    // The menu's rows for the activity log, with what More holds.
    // The highlighted mod's settings (My mods' panel and the mod page): its files, the button's words, and the way in.
    readonly property var panelCfgs: menuMod && tab === 0 ? settingsFilesOf(menuMod.id) : []
    readonly property bool panelHasSettings: showModPanel && panelCfgs.length > 0
    readonly property string panelSettingsLabel: menuMod && isSeamless(menuMod) ? "Co-op settings" : "Edit settings"
    function panelEditOpen() { if (!menuMod) return; note("action", "Panel: " + panelSettingsLabel + " for " + menuMod.name); openModSettings(menuMod) }
    onPanelHasSettingsChanged: if (!panelHasSettings && zone === "panel") zone = "main"
    function modMenuTitles() { return modMenuItems.map(x => x.act === "more" ? "More (" + x.rest.map(r => r.title).join(", ") + ")" : x.act === "details" ? "Details (" + modMenuFacts.map(f => f.title).join(", ") + ")" : x.title).join("; ") }
    function optionsLine() {
        let all = []
        for (const x of modMenuItems) all = all.concat(x.act === "more" ? x.rest : [x])
        all = all.filter(x => x.act !== "details")
        const pick = []
        if (all.length > 0 && all[0].act !== "rename" && all[0].act !== "remove") pick.push(all[0])
        for (const a of ["rename", "remove"]) { const r = all.find(x => x.act === a); if (r) pick.push(r) }
        const rest = all.length - pick.length
        return pick.map(x => x.title).join(" · ") + (rest > 0 ? " and " + rest + " more" : "")
    }
    function modMenuDo(item, m) {
        if (item.act === "more") openPick(m.name, "More options", item.rest.map(x => ({ icon: x.icon, title: x.title, detail: x.detail, run: () => modMenuDo(x, m) })), "modmenu")
        else if (item.act === "toggle") toggle(m)
        else if (item.act === "loader") getLoader()
        else if (item.act === "needs") {
            note("action", "Get what it needs: " + item.need)
            sheet = ""
            // 0.64: the same sheet as after an add (Get them all); a need Turbine can't name on Nexus is searched.
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
            if (!Qt.openUrlExternally("file://" + encodeURI(dir))) toast.showPlain("Its files are in " + dir)
        }
    }
    // ---------------------------------------------------------------- 0.56: clean names
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
        // 0.6: the player's own name, else the page's title (Nexus), else the cleaned file name.
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
    readonly property bool showModPanel: page === "game" && !!game && tab === 0 && modsLoaded && !modPage && mods.length > 0 && width / Math.max(1, height) >= 1.5 && designWidth >= 700
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
        // 0.62: Seamless Co-op's own fact, first when nothing is wrong (its row says it then).
        if (isSeamless(m) && line.text.indexOf("Loads only") !== 0) out.push({ text: coopOnlyWords + ".", level: "" })
        const o = overlapsMap[m.id]
        if (o) {
            for (const w of (o.wins || []).slice(0, 2)) out.push({ text: "Wins " + plural(w.count, "file", "files") + " over " + w.name, level: "" })
            for (const l of (o.loses || []).slice(0, 2)) out.push({ text: "Loses " + plural(l.count, "file", "files") + " to " + l.name, level: "" })
        }
        const l = linksMap[m.id]
        if (l && (l.needs || []).length > 0) out.push({ text: "Needs: " + l.needs.map(n => n.name).join(", "), level: missingNeeds(m).length > 0 ? "warn" : "" })
        if (placements[m.id]) out.push({ text: "Placed in " + folderWords(placements[m.id].to), level: "" })
        return out.slice(0, 5)
    }
    // 0.56, status badges: a mod its page took down or its author marked deprecated.
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
        // 0.62: Seamless Co-op's row says when it loads (on or off, it only loads with its own Play).
        if (isSeamless(m)) return { text: "Loads only with Play Seamless Co-op", level: "info" }
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

    // ---------------------------------------------------------------- 0.56: mods for any game
    // Games without rules of their own get Basic support: each mod goes where its files show they belong, or Turbine
    // asks once ("Where do these files go?"). The engine decides (basic.rs); these draw it and take the answers.
    readonly property bool basicGame: !!rules && !!rules.supportLevel && rules.supportLevel !== "full"
    // 0.57: an anti-cheat game me3 starts says how its mods run; an online-only one says nothing here (the status does).
    // 0.59: rules can be null for a moment while the rail switches games (basicGame may not have caught up yet).
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
    // 0.59 (R2): the mod's own files only on demand (X, Only part of the file...).
    property bool placeModShown: false
    readonly property bool placeTwoPanes: placeModShown && (width >= 1280 || desktopLayout)
    readonly property string placeGameName: placeData ? placeData.gameName : gameTitle
    function folderWords(rel) { return rel === "" ? "the game folder" : rel }
    function parentOf(p) { const i = p.lastIndexOf("/"); return i < 0 ? "" : p.substring(0, i) }
    function baseName(p) { const i = p.lastIndexOf("/"); return i < 0 ? p : p.substring(i + 1) }
    readonly property var placeGameRows: {
        if (page !== "place") return []
        let rows = []
        const p = placeData && placeData.pending ? placeData.pending.placement : null
        // 0.59 (R2): the guess first, as one press: "Put them in Content/Paks" (A or a tap places them there).
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
        if (placeMode === "add") { Turbine.cancelPlace(); toast.showPlain("Not added. The file is still in Downloads.") }
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
        const gname = placeData ? placeData.gameName : gameTitle
        const list = conflicts.slice(0, 5).concat(conflicts.length > 5 ? ["and " + (conflicts.length - 5) + " more"] : [])
        const refusedWords = refused.length === 0 ? ""
            : plural(refused.length, "file", "files") + " can't be " + (conflicts.length > 0 ? "replaced" : "placed") + ": "
              + refused.slice(0, 4).map(x => baseName(x.path) + " (" + x.why + ")").join(", ") + (refused.length > 4 ? " and " + (refused.length - 4) + " more" : "") + "."
        const done = () => { if (fromPage) placeLeave() }
        const cancel = () => {
            if (fromPage) return
            Turbine.cancelPlace(); toast.showPlain("Not added. The file is still in Downloads.")
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
    // 0.62: a game whose Nexus page Turbine can't tell for sure (several Nexus games fit its name: the engine's
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
                        : a.mode === "me3" && a.needsAsk ? "Mods for " + gameTitle + " load through me3, which starts it offline without its anti-cheat. Turbine asks before the first Play with mods, and never changes the game's own files."
                        : a.mode === "me3" ? "Mods for " + gameTitle + " load through me3. Turbine never changes the game's own files."
                        : gameTitle + " uses anti-cheat (" + a.name + "). Mods that change its files can get you kicked or banned from online play. Use mods only in offline or single player modes.",
                  notes: a.mode === "refuse" ? (removedMods.length > 0 || downloads.some(d => d.here !== false) ? ["Any mod file for it stays in Downloads, untouched."] : []) : rules.antiCheat.note ? [rules.antiCheat.note] : [],
                  buttons: [{ key: "b", label: "OK", run: () => {} }] })
    }
    // The first visit to a Basic game with no mods: what Basic support means (once per game).
    property var introSeen: null          // the games seen before this run (read once, so the card stays this run)
    // 0.58: this game has somewhere to get mods (Nexus or Thunderstore).
    readonly property bool canGetMods: !!rules && (!!rules.domain || sources.length > 0)
    // An empty My mods: the centered card (any game, after a Basic game's first-visit card).
    readonly property bool emptyMods: page === "game" && !!game && tab === 0 && modsLoaded && mods.length === 0 && !acRefused && !basicIntro && !modPage
    onEmptyModsChanged: if (emptyMods) Qt.callLater(() => note("page", "No mods card: No mods for " + gameTitle + " yet | legend " + legendHints.map(h => h[0].toUpperCase() + " " + h[1]).join(" · ")))
    readonly property bool basicIntro: page === "game" && !!game && tab === 0 && modsLoaded && mods.length === 0 && basicGame && !acRefused
                                       && (introSeen || []).indexOf(game.id) < 0
    onBasicIntroChanged: {
        if (introSeen === null) introSeen = parse(seen.basicIntroDone, [])
        if (basicIntro && !shot.on) { let d = parse(seen.basicIntroDone, []); if (d.indexOf(game.id) < 0) { d.push(game.id); seen.basicIntroDone = JSON.stringify(d) } }
        if (basicIntro) note("page", "First visit card: Mods for " + gameTitle)
    }
    function placementWords(m) {
        const p = placements[m.id] || guardedMods[m.id]
        if (!p) return []
        return (placements[m.id] ? [{ icon: "info", title: "Placed in: " + folderWords(p.to), detail: p.rule === "you" ? "By you" : "By Turbine's rule " + p.rule, act: "place" }] : [])
            .concat((p.skipped || []).length > 0 ? [{ icon: "info", title: "Skipped: " + plural(p.skipped.length, "file the game already has", "files the game already has"),
                                                      detail: "A lets this mod replace them (the game's own are kept)", act: "replaceon" }]
                    : p.replace ? [{ icon: "info", title: "Replaces the game's own files", detail: "A leaves them alone again", act: "replaceoff" }] : [])
    }

    // ---------------------------------------------------------------- Add mods from Downloads
    property int addIndex: 0
    function openAdd() { Turbine.listDownloads(); addIndex = 0; addAll = false; sheet = "add" }
    // 0.61 (I18): Add from Downloads' Find pop-up. The first row is which files show; B goes back to the list.
    function openAddFind() {
        const pickShow = (all) => { addAll = all; addIndex = 0; note("action", all ? "Add: all files" : "Add: this game's files"); sheet = "add" }
        openPick("Find mod files", "Which files Add from Downloads lists", [
            { icon: "files", title: "This game's files", detail: "In Downloads and Game Mods/" + gameTitle + (addAll ? "" : " · Showing now"), aLabel: "Show", run: () => pickShow(false) },
            { icon: "files", title: "All files", detail: "Every mod file in Downloads and its Game Mods folders" + (addAll ? " · Showing now" : ""), aLabel: "Show", run: () => pickShow(true) }], "add")
        pickNoStart = true
        menuIndex = addAll ? 1 : 0
    }
    // 0.61 (I18): Edit settings' options (X): how the file shows, and Reset to default.
    function openSettingsOptions() {
        const rows = []
        if (settingsView.fields.length > 0)
            rows.push({ icon: "info", title: settingsView.plain ? "Show as a list" : "Show as text", detail: settingsView.plain ? "Each setting with its own control" : "The file's lines as they are", aLabel: "Show",
                        run: () => { settingsView.toggleView(); sheet = "settingsedit" } })
        rows.push({ icon: "restore", title: "Reset to default", detail: settingsMod === "" ? "Puts back the file as it was before you first changed it" : "Puts back the mod's own file", aLabel: "Reset", run: settingsResetAsk })
        openPick("Options", settingsView.title, rows, "settingsedit")
        pickNoStart = true
    }
    function addChosen() {
        const d = addList[addIndex]
        // Nothing to add: the sheet stays (controls #19).
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
                                           tutorial: "first run", quick: "the game closed early", oldhelper: "the browser helper is old",
                                           msg: "a mod or the game needs a word" })
    onSheetChanged: {
        if (sheet === "") note("page", "Sheet closed")
        else if (pressRecent() || (shot.on && !shot.strictSheets)) note("page", "Sheet " + sheet + " (after " + (lastInputName || "start") + ")")
        else if (sheetReasons[sheet]) note("page", "Sheet " + sheet + " (" + sheetReasons[sheet] + ")")
        else note("page", "Sheet " + sheet + " opened with no press")
        menuIndex = 0; stopDangerHolds()
        if (sheet !== "") closeWheel()
        if (sheet !== "" && barsHidden) barsHidden = false
        // Other files' place in the history goes with its sheet (controls #13).
        if (sheet !== "versions" && history.some(x => x.kind === "files")) history = history.filter(x => x.kind !== "files")
        // 0.59: the text prompt lives only while it is the sheet (another sheet replacing it closes it unanswered).
        if (sheet !== "ask" && askView.shown) { note("page", "Text entry closed: another sheet replaced it"); askView.close(); reportEditing(false) }
        // An installer's choices: the Nexus page closes so they show; any other sheet over the page gives the page
        // its keyboard focus back when it closes.
        if (sheet === "fomod" && page === "browser") closeBrowser("installer")
        else if (sheet === "" && page === "browser" && nexusView) Qt.callLater(() => { if (nexusView) nexusView.focusPage() })
        pickHover()
        if (sheet === "" && fomod) sheet = "fomod"
        else if (sheet === "" && found) { infoIndex = 0; sheet = "found" }
        else if (sheet === "" && pendingNotes && page !== "browser") { pendingNotes = false; infoIndex = 0; sheet = "notes" }
        else if (sheet === "" && pendingOldHelper) { pendingOldHelper = false; infoIndex = 0; sheet = "oldhelper" }
        else if (sheet === "") offerLoader()
        // 0.62: "Which game on Nexus is this?" waited for the sheet that was up (the anti-cheat card comes first).
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
    // 0.59 (the owner's rule: A selects, B goes back one step, nothing else): holding B no longer goes home. goHome
    // stays for the build's checks.
    Timer { id: homeHold; interval: 1000 }
    function goHome() {
        note("action", "Home")
        if (sheet === "ask") askView.cancel ? askView.cancel() : null
        sheet = ""; moving = false; orderMoving = false; modPage = false; history = []
        if (browserOpen) { browserFrom = null; browserArrival = null; closeBrowser("home") }
        page = "game"; zone = "main"; tab = 0; modIndex = 0
    }
    // 0.62 (the owner's rule): a tap of Start opens Settings (again, or B, goes back). Holding Start for 3 seconds
    // plays: a brass ring fills on the bar's Play button meanwhile, and letting go early does nothing. What the hold
    // does is a setting (Look, "Hold Start does"): show the Play choice, or play the last choice at once.
    readonly property int startHoldMs: 3000
    readonly property int startTapMs: 500
    property real startSince: 0
    property real startHeld: 0            // 0 to 1 while Start is held
    property int startHoldsDone: 0
    readonly property bool startHoldPlays: look.holdStartDoes === "last"
    readonly property string startHoldWords: (keysLegend ? "hold Space: " : "hold: ") + (startHoldPlays ? "Play" : "Play choice")
    Timer {
        id: startHold
        interval: 50
        repeat: true
        onTriggered: {
            const part = Math.min(1, (Date.now() - app.startSince) / app.startHoldMs)
            // Something opened over the page meanwhile, or there is nothing to play here: no ring, and no Play.
            if (!app.startHoldReady()) { app.startHeld = 0; if (part >= 1) stop(); return }
            app.startHeld = part
            if (part >= 1) { stop(); app.startHeld = 0; app.startHoldsDone++; app.startHoldDone() }
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
    // 0.62: what the ring draws. The hold itself is timed by startHold alone; this only fills the ring evenly
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
        if (b === "b") bHeld = true
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
        if (b === "b") bHeld = false
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
        if (b === "b") homeHold.stop()
        if (b === "view") wheelRelease()
    }
    // R3 with the pointer hidden: A on the focused item. The pointer stays hidden, so the triggers keep paging.
    // A hold (Delete for good, Reset) never starts from R3: it says to hold A instead.
    // 0.61 (M6): while a toast with a next step shows ("R3 Undo"), R3 does that step instead.
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
    // ---- 0.62: the focus ring glides. Each thing draws its own ring where the focus rests; when the focus moves,
    // a ring travels from the last one to the new one (120 ms) and fades as it lands. Only at Full, only on the
    // main screens (the game list, My mods, Get mods, Play), and never while a list is still gliding to its row.
    property Item focusWas: null
    property int focusGlides: 0
    function focusItemNow() {
        if (sheet !== "" || wheelOpen || page === "browser" || page === "place") return null
        if (zone === "rail") return railIndex < games.length ? railList.itemAtIndex(railIndex) : railIndex < games.length + railExtra ? hiddenTile : gear
        if (zone === "play") return playBtn.visible ? playBtn : null
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
            focusGlides++
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
                NumberAnimation { id: ringX; target: focusRing; properties: "x"; duration: mo.moveFast; easing.type: mo.enter }
                NumberAnimation { id: ringY; target: focusRing; properties: "y"; duration: mo.moveFast; easing.type: mo.enter }
                NumberAnimation { id: ringW; target: focusRing; properties: "width"; duration: mo.moveFast; easing.type: mo.enter }
                NumberAnimation { id: ringH; target: focusRing; properties: "height"; duration: mo.moveFast; easing.type: mo.enter }
            }
            NumberAnimation { target: focusRing; properties: "opacity"; to: 0; duration: mo.fast; easing.type: mo.leave }
        }
    }
    // The middle of whatever has the focus ring, in window pixels.
    function focusCenter() {
        let it = null
        try {
            if (sheet === "" && !wheelOpen) {
                if (zone === "rail") it = railIndex < games.length ? railList.itemAtIndex(railIndex) : railIndex < games.length + railExtra ? hiddenTile : gear
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
        // 0.63: when the key came. Qt hovers the items under a still mouse again when the screen changes
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
    // 0.63: Tab and Space each do one thing on a tap and another when held (see Keys.onPressed below).
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
    // 0.63: a double-click on a mod opens it: its details (what it needs, what it wins or loses, where it goes), or
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
            id: realMouse
            onPointChanged: {
                app.hoverSeen++; app.hoverSeenAfterKey = Date.now() - app.keyAt
                app.mouseSeen()
            }
        }
        Keys.onPressed: (e) => {
            app.pressFresh = false
            // 0.62: Ctrl+Enter is the keyboard's hold of Start (the Play choice, or Play).
            if ((e.modifiers & Qt.ControlModifier) && (e.key === Qt.Key_Return || e.key === Qt.Key_Enter)) {
                e.accepted = true
                if (e.isAutoRepeat) return
                app.note("key", "Key Ctrl+Enter")
                app.inputCount++; app.lastInputAt = Date.now(); app.lastInputName = "start"
                if (app.startHoldReady()) { app.startHoldsDone++; app.startHoldDone() }
                return
            }
            // 0.62: the keyboard is as good as the controller. Arrows move, Enter (or Space) is A, Esc or Backspace
            // is B, Tab and Shift+Tab (Page Down and Page Up too) flip pages, Ctrl+F or / finds, the Menu key or
            // Shift+F10 opens options, F1 is Help, Ctrl+= and Ctrl+- change the size, Ctrl+Z is a toast's step
            // (Undo), Home and End jump a screen. The letters stay: X, Y or F, Q and E, V the wheel, P Start.
            // 0.63: the keys a PC player reaches for (many keyboards have no Page Up or Page Down): W A S D move
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
            // 0.63: Esc on a game's first page (nothing to go back from) opens Settings, like a tap of Start.
            if (e.key === Qt.Key_Escape && plain && app.escTop) {
                if (e.isAutoRepeat) return
                app.note("key", "Key Esc: Settings"); app.keySeen("start")
                app.startTap()
                return
            }
            // 0.63: Tab. A tap is the next page (on letting go); held, the quick wheel opens and letting go
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
            // 0.63: Space. A tap is A (on letting go); held for 3 seconds where Start's hold counts, it plays,
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
            if (b === "b" && !e.isAutoRepeat) app.bHeld = true
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
            if (!e.isAutoRepeat && (e.key === Qt.Key_Escape || e.key === Qt.Key_Backspace)) app.bHeld = false
        }
    }

    // Sheets with a row of buttons (SheetButtons): the D-pad moves the highlight, A presses it, B is the cancel
    // button, Start the main one. No other keys. {labels, index, main, cancel, set(i), run(i), scrolls, scroll(d)}.
    readonly property var infoSheets: ["nobrowser", "helper", "notes", "found", "quick", "chain", "addbrowser", "oldhelper", "msg"]
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
        if ((b === "up" || b === "down") && bs.scrolls) bs.scroll(b === "down" ? 1 : -1)
        else if (b === "left" || b === "up") bs.set(Math.max(0, bs.index - 1))
        else if (b === "right" || b === "down") bs.set(Math.min(n - 1, bs.index + 1))
        else if (b === "a") {
            if (sheet === "ask" && askIndex < 0) askKeyboard()
            else if (sheet !== "confirm" || bs.index === 1) runIt(Math.min(bs.index, n - 1))
            // A that isn't a held A (R3, a tap): Delete needs the hold.
            else if (!sheetDry && !confirmHold.running && !pressTwice) holdHint(holdNeeded + " seconds to " + confirmWords())
        }
        else if (b === "b") { if (bs.cancel >= 0) runIt(bs.cancel); else if (!sheetDry) sheet = "" }
        // 0.62: Start does nothing on these (a tap of Start never plays; A on the highlighted button does).
    }
    function sheetBinds() {
        const bs = buttonSheet()
        if (bs) {
            const hl = sheet === "ask" && askIndex < 0 ? (Turbine.gamemode ? "Keyboard" : "") : sheet === "confirm" && confirmIndex === 0 ? "Delete" : bs.labels[bs.index] || ""
            let o = { a: bind(hl, () => sheetAct("a"), sheet === "confirm" && confirmIndex === 0 ? "hold" : ""), b: bind(bs.cancel >= 0 ? bs.labels[bs.cancel] : "Close", () => sheetAct("b")) }
            return o
        }
        const s = sheet
        const k = key => () => sheetAct(key)
        // 0.59 (R1): choices are already in effect, so B is Done; Start does the same but isn't shown (Start is Play).
        if (s === "findpop") return { a: bind(findPopALabel(), k("a")), b: bind("Done", k("b")), x: bind("Clear all", k("x")), y: bind("Keyboard", k("y")), start: bind("", k("start")) }
        if (s === "modmenu" || s === "gamemenu" || s === "cardmenu") {
            // A says what the highlighted row does ("A Uninstall", "A Rename").
            const row = s === "modmenu" ? modMenuItems[menuIndex] : s === "cardmenu" ? cardMenuItems[menuIndex] : gameMenuItems[menuIndex]
            // The game menu's A names its row too (T37); a dimmed row says why instead.
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
        // 0.59: the legend follows the installer (controls #6): the stop question's buttons, Stop on the first page,
        // and Start goes on from any row.
        if (s === "fomod") return fomodView.askStop ? { a: bind(fomodView.stopFocus === 1 ? "Stop installing" : "Keep choosing", k("a")), b: bind("Keep choosing", k("b")) }
                                  : { a: bind(fomodView.onNextRow ? (fomodView.lastPage ? "Install" : "Next") : "Choose", k("a")), b: bind(fomodView.canStepBack ? "Back a step" : "Stop", k("b")),
                                      start: bind(fomodView.lastPage ? "Install" : "Next", k("start")) }
        if (s === "report") return reportView.editing ? { b: bind("Done typing", k("b")) }
                                   : { a: bind(reportView.focusAt === 2 ? "Save report" : reportView.focusAt === 3 ? "Not now" : reportView.focusAt === 1 ? "Type" : "Switch", k("a")),
                                       b: bind("Close, keep it", k("b")), start: bind(reportView.text.trim() !== "" ? "Save report" : "", k("start")) }
        if (s === "activity") return { a: bind("", k("a")), b: bind("Close", k("b")), x: bind(activityView.full ? "What happened" : "Full log", k("x")) }
        if (s === "settingsedit") return { a: bind(settingsView.aWord, k("a")), b: bind(settingsView.leaveArmed ? "Leave" : "Back", k("b")),
                                           x: bind("Options", k("x")), start: bind("Save", k("start")) }
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
            // 0.61 (break N3): an A made while the ring was still on Not now, handled after the timer moved it (a
            // stalled window): it meant Not now.
            note("nxm", "A made at " + Math.round(actStamp - nxmShownAt) + " ms, before the ring moved: Not now")
            menuIndex = 1; pickRun(1)
        } else if (sheet === "pick" && nxmAsking && (b === "left" || b === "right" || b === "up" || b === "down")) {
            // Right (or Up) to Get it, Left (or Down) back to Not now. The player chose: the ring never moves by
            // itself after this (break M1).
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
            // Y: Find (0.61, I18): its first row chooses this game's files or every mod file.
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
    // Newbie L2: B on the loadout files goes back to where they were opened from (the game menu or Loadouts).
    property string sharesBack: "profiles"
    // Turbine can fetch the loader itself (SKSE, BepInEx, SMAPI).
    readonly property bool loaderCanGet: !!rules && !!rules.loader && ["bethesda", "bepinex", "smapi"].indexOf(rules.kind) >= 0

    // ---------------------------------------------------------------- one table for every button
    // binds() says what each button does right now: {a, b, x, y, start, lb, rb, lt, rt, view} of {label, run, pre}.
    // act() runs it and the legend prints it, so the legend can't say something the button doesn't do. The D-pad
    // and the left stick move the focus (navigate()); they are never in the legend.
    function bind(label, run, pre) { return { label: label, run: run, pre: pre || "" } }
    // 0.6 (break M2): when the press being handled was made, on Turbine.monoMs's clock: a controller press by SDL's
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
        // 0.61 (M6): Y is always Find; a toast's own next step is on R3 (and a tap on the toast's button).
        return bindsCore()
    }
    function bindsCore() {
        let o = {}
        if (sheet === "tutorial") return { a: bind(tutCard < tutCount - 1 ? "Next" : "Pick a game", () => tutorialAct("a")), b: bind("Skip", () => tutorialAct("b")), start: bind("", () => tutorialAct("start")) }
        if (wheelOpen) {
            // 0.62: nothing picked (the stick at rest): A closes, like B and View.
            const s = wheelSlot >= 0 ? wheelSlots[wheelSlot] : null
            return { a: bind(s ? (s.ok ? s.label : "") : "Close", () => wheelAct("a")), b: bind("Close", () => wheelAct("b")), view: bind("", () => wheelAct("b")) }
        }
        if (sheet !== "") return sheetBinds()
        if (moving || orderMoving) return { a: bind("Done", () => act("a")), b: bind("Cancel", () => act("b")), lt: bind("", () => act("lt")), rt: bind("", () => act("rt")) }
        o.view = bind("", wheelPress)
        // 0.62: a tap of Start is Settings, and back from it (press() times the hold that plays).
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
            const rows = sec === "Look" ? lookRows.length : sec === "About" ? (labsOn ? 2 : 1) : rowsNow.length
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
            // 0.59 (newbie M1): Y finds here too (the mod page closes onto the results).
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
            // 0.58: no mods yet (any game): A Get mods, X Add from Downloads (the centered card says the same).
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
                    { icon: "remove", title: "Forget them", detail: "The row goes; the mod's warning stays until they are in", run: () => { reqSave(null); note("action", "Requirements: forgotten") } }
                ], ""))
            }
            else if (it && it.kind === "removed") {
                o.a = bind("Open", openRemoved)
                o.x = bind("Options", openRemoved)
            }
            // The removed row went a moment ago and another row slid under the highlight: A still means Put back.
            if (ghostJustWent()) o.a = bind("Put back", () => { note("action", "A just as the removed row went: put back"); const id = ghostGone.id; ghostGone = null; putBack(id) })
            // 0.59 (the owner's rule): B only ever goes back. Find, Show and Sort stay (the line over the list says
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
                // 0.61 (I18): Y is Find here too (a search on Nexus); Add from Downloads is the card's own button
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
        // 0.61 (I18): one name for Y: Find (here it finds a plugin by name).
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
        // 0.62: R3 is A on the focused thing, except while a toast shows a next step (Undo, Watch it): then R3 does
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
    // 0.59: the first press after a page opens never changes a setting (the owner's Left on the way into Settings, a
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
            const rows = sec === "Look" ? lookRows.length : sec === "About" ? (labsOn ? 2 : 1) : rowsNow.length
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
        // 0.57: Play is in the bottom bar (Start); Up at the top stays there and brings the bars back.
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
        // Right: the panel's Edit settings (0.6), when the highlighted mod has settings.
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
        else if (sec === "About") { if (setRow === 0) labsTap(); else if (labsOn) { page = "perf"; zone = "perf"; perfIndex = 0 } }
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
            { icon: "info", title: "Hide from the rail", detail: "For tools that aren't games. You can show it again.", run: () => { setRailList("hidden", g.id, true); toast.act("done", g.name + " is hidden from the rail.", "Undo", () => setRailList("hidden", g.id, false)) } }
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
    // 0.56: short names on the rail, so long ones aren't cut to "Kingdom Come: D..." twice.
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
    // ---- 0.59: one input layer. In Qt 6 a MouseArea that takes a press stops the items under it but not their
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
    property int beneathHits: 0
    property int coveredHits: 0
    function gate(layer, name) {
        if (topLayer > layer) { beneathHits++; note("touch", "LEAK " + name + " (layer " + layer + " under " + topLayer + ")"); return false }
        if (pressCover > layer) { coveredHits++; note("touch", "Covered: " + name + " under a press on layer " + pressCover); return false }
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
        // 0.57: Up is the Y pop-up (Find, Show, Sort; Search, Sort, Category), Right Add from Downloads and Up-left
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
    // 0.62: the middle is neutral. The stick at rest picks nothing, and the middle says Close: A, B, View, a tap on
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
    // Presses and releases that reached the interface from the controller (the input check counts them).
    property int padEvents: 0
    property bool probeOn: false
    function check(text) { note("check", text) }
    function scriptTick() {
        if (scriptStep >= script.length) { scriptTimer.stop(); shotAfterScript(); return }
        let ok = true
        try { ok = script[scriptStep]() !== false } catch (e) { check("Step " + (scriptStep + 1) + " stopped: " + e); ok = true }
        if (ok || ++scriptTries > 12) {
            if (!ok) check("Step " + (scriptStep + 1) + " gave up waiting (game " + (game ? game.name : "none") + ", " + mods.length + " mods, tab " + tab + ", zone " + zone
                           + ", Get mods " + getArea + " " + getIndex + " of " + getItems.length + ", mod page " + modPage + ", sheet \"" + sheet + "\")")
            scriptStep++; scriptTries = 0
        }
    }
    // 0.6: the picture of a scripted page comes once its steps are done, however long they took (a fixed time cut
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
            // R3 with the pointer hidden: does what A does (the pointer stays hidden); with it showing, a left click.
            () => { modIndex = 0; m.on = mods[0].enabled; pad("r3") },
            () => { check("R3 hidden: pointer " + (Turbine.pointerShown ? "shown" : "stays hidden") + ", " + (mods[0].enabled !== m.on ? "same as A (the mod turned " + (mods[0].enabled ? "on" : "off") + ")" : "A didn't run"))
                    const it = modList.itemAtIndex(0); const p = it.mapToItem(null, it.width * 0.4, it.height / 2); Turbine.pointerShowAt(p.x, p.y); pad("r3") },
            () => { pad("down"); pad("up") },
            // 8: the focus table: toggle keeps the row; a menu closes back to it; tabs and a game round trip keep it.
            () => { modIndex = 1; pad("a"); check("Toggle: focus on row " + (modIndex + 1)); pad("x"); m.sheet = sheet; pad("b"); check("Menu " + m.sheet + " closed: focus on row " + (modIndex + 1)) },
            () => { modIndex = 1; pad("rb"); m.tab = tab; pad("lb"); check("Tabs round trip: tab " + m.tab + " and back to " + tab + ", row " + (modIndex + 1)) },
            () => { m.home = game.id; m.other = games.findIndex(g => g.id !== game.id); if (m.other < 0) { check("Game round trip: only one game"); return }
                    tab = 0; zone = "main"; modIndex = 1; m.want = focusedItem && focusedItem.m ? focusedItem.m.name : ""; zone = "rail"; railIndex = m.other; pad("a") },
            () => { if (!game || game.id === m.home) return false; zone = "rail"; railIndex = games.findIndex(g => g.id === m.home); pad("a") },
            () => { if (!game || game.id !== m.home || !modsLoaded || mods.length < 2) return false
                    check("Game round trip: back on " + game.name + ", tab " + tab + ", row " + (modIndex + 1) + " (" + (focusedItem && focusedItem.m ? focusedItem.m.name : "") + ", was " + m.want + "; remembered " + lastFocusId[game.id] + ")" + (focusedItem && focusedItem.m && focusedItem.m.name === m.want ? ": the same mod" : ": a different mod")) },
            // The quick wheel: tap View, Up then Right is Up-right (Sort); hold View, push Left, let go: Games.
            () => { pad("view"); pad("up"); pad("right"); check("Wheel: slot " + wheelSlot + " " + (wheelSlots[wheelSlot] ? wheelSlots[wheelSlot].label : "")); pad("b") },
            () => { Turbine.selftestPad("view", true); pad("left"); Turbine.selftestPad("view", false); check("Wheel held, Left, let go: sheet " + sheet + " \"" + pickTitle + "\""); pad("b") },
            // 0.62: the neutral middle. A tap of View opens it with nothing picked, and A closes it.
            () => { sheet = ""; pad("view"); m.open = wheelOpen; m.slot = wheelSlot; m.a = binds().a.label; m.b = binds().b.label; pad("a")
                    check("Wheel neutral: a tap of View leaves it " + (m.open ? "open" : "closed") + " with slot " + m.slot + ", A " + m.a + ", B " + m.b + "; A closes it: " + !wheelOpen) },
            // The stick pushed Right picks that wedge; back at rest nothing is picked. A D-pad pick stays. B closes.
            () => { pad("view"); Turbine.wheelAngle = 90; m.s1 = wheelSlot; Turbine.wheelAngle = -1; m.s2 = wheelSlot; pad("up"); m.s3 = wheelSlot; pad("b")
                    check("Wheel stick: pushed Right picks slot " + m.s1 + ", at rest slot " + m.s2 + "; D-pad Up picks slot " + m.s3 + " and it stays; B closes it: " + !wheelOpen) },
            // View held with nothing pushed, then let go: it closes.
            () => { m.t = Date.now(); Turbine.selftestPad("view", true); m.open = wheelOpen },
            () => { if (Date.now() - m.t < 600) return false; Turbine.selftestPad("view", false)
                    check("Wheel held, nothing pushed, let go: it was " + (m.open ? "open" : "closed") + ", now " + (wheelOpen ? "still open" : "closed") + ", sheet \"" + sheet + "\"") },
            // A click (or a tap) on the middle closes it.
            () => { pad("view"); m.open = wheelOpen
                    Turbine.selftestMouse(0, width / 2, height / 2, 0, 0, 0); Turbine.selftestMouse(1, width / 2, height / 2, Qt.LeftButton, 0, 0); Turbine.selftestMouse(2, width / 2, height / 2, Qt.LeftButton, 0, 0) },
            () => { check("Wheel middle clicked: it was " + (m.open ? "open" : "closed") + ", now " + (wheelOpen ? "still open" : "closed")) ; closeWheel() },
            // Uninstall: the same row stays (dimmed, A puts it back).
            () => { zone = "main"; tab = 0; modIndex = 0; m.name = mods[0].name; pad("x"); menuIndex = modMenuItems.findIndex(x => x.act === "remove"); pad("a") },
            () => { check("Uninstall: row " + (modIndex + 1) + " is " + (focusedItem ? focusedItem.kind : "nothing") + (focusedItem && focusedItem.kind === "ghost" ? " (" + focusedItem.m.name + ")" : "")); pad("a") },
            () => { if (!mods.some(x => x.name === m.name)) return false; check("Put back: " + m.name + " is back"); pad("x") }
        ]
        else if (name === "legendcheck") script = [
            () => { if (mods.length < 2) return false
                const ctxs = [
                    ["My mods", () => { sheet = ""; page = "game"; zone = "main"; tab = 0; modIndex = 0 }],
                    ["Find pop-up (My mods)", () => { openFindPop() }],
                    ["Rail", () => { sheet = ""; modIndex = 0; zone = "rail"; railIndex = current }],
                    ["Play", () => { zone = "play" }],
                    ["Find pop-up (Get mods)", () => { zone = "main"; tab = 1; getArea = "grid"; getIndex = 0; openFindPop() }],
                    ["Get mods grid", () => { sheet = ""; getArea = "grid"; getIndex = 0 }],
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
                    ["Pointer", () => { cancelMove(); const it = modList.itemAtIndex(0); const p = it.mapToItem(null, 20, 20); Turbine.pointerShowAt(p.x, p.y) }],
                    ["Nexus page", () => { openBrowser(btFiles, "page") }],
                    ["Nexus downloads", () => { browserTab = 1 }],
                    ["Nexus menu", () => { browserTab = 0; openBrowserMenu() }],
                    ["Nexus quick wheel", () => { sheet = ""; wheelPress(); wheelHeld = false }]
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
                    if (keys.indexOf("start") >= 0 && sheet === "" && !wheelOpen && page !== "browser" && page !== "place" && page !== "settings") bad.push("Start in the legend with Play in view")
                    for (const k of ["a", "b", "x", "y"]) {
                        if (o[k] && o[k].label && keys.indexOf(k) < 0) bad.push(k.toUpperCase() + " missing")
                        if (keys.indexOf(k) >= 0 && !(o[k] && o[k].run)) bad.push(k.toUpperCase() + " does nothing")
                    }
                    if (JSON.stringify(shown) !== JSON.stringify(h)) bad.push("the legend on screen differs")
                    problems += bad.length > 0 ? 1 : 0
                    check("Legend " + c[0] + ": " + h.map(x => (x[2] ? x[2] + " " : "") + x[0].toUpperCase() + " " + x[1] + (x[3] ? ", " + x[3].toUpperCase() + " " + x[4] : "")).join(" · ") + (bad.length > 0 ? "  PROBLEM: " + bad.join(", ") : ""))
                }
                closeWheel(); sheet = ""; closeBrowser("back")
                Turbine.pointerHide()
                check("Legend check: " + ctxs.length + " contexts, " + problems + " problems")
                sheet = ""
            }
        ]
        // 0.6 (I3): Seamless Co-op chosen at Play: the choice, the password step, the remembered choice, its settings
        // from all three places, and a toggle, a number and the password saved into Turbine's copy of the file.
        else if (name === "coopcheck") script = [
            () => { if (!seamlessIn || !modsLoaded) return false; sheet = ""; page = "game"; zone = "main"; tab = 0; acAnswered = ({ "steam:1245620": true }); Turbine.selftestPad("start", true) },
            () => { if (sheet !== "pick") return false
                    Turbine.selftestPad("start", false)
                    check("Co-op only: the Play choice says \"" + pickSub + "\"; Play with mods: " + pickItems[0].detail + "; its row in My mods: " + rowLine(seamlessMod).text)
                    check("Co-op choice: " + pickItems.map(x => x.title).join("; ") + " | focus " + (pickItems[menuIndex] ? pickItems[menuIndex].title : "") + " | Y " + (pickY ? pickY.label : "none"))
                    pad("down"); m.focus = pickItems[menuIndex] ? pickItems[menuIndex].title : ""; pad("a") },
            () => { if (sheet !== "ask") return false
                    const r = settingsView.shown[settingsView.focusIndex]
                    m.asks = playAsks
                    check("Co-op password step: picked " + m.focus + ", sheet " + askReturn + " with the keyboard (" + sheet + "), focus " + (r && r.f ? r.f.key : "none") + ", waiting " + coopAfterSettings + ", Play asked " + playAsks + " times")
                    askView.open("Session password", "", "dragon", "", 2000, false, "Done"); askView.accept() },
            () => { if (sheet === "settingsedit" || sheet === "ask") return false; check("Co-op after the password: sheet \"" + sheet + "\", waiting " + coopAfterSettings + ", Play asked " + (playAsks - m.asks) + " times, co-op " + (playCoopNext || playAsks > m.asks)) },
            () => { sheet = ""; page = "game"; zone = "play"; pad("a") },
            () => { if (sheet !== "pick") return false
                    check("Co-op choice remembered: focus " + (pickItems[menuIndex] ? pickItems[menuIndex].title : "") + ", Play says " + (lastPlayChoice === "coop" ? "Co-op" : "Solo"))
                    pad("y") },
            () => { if (sheet !== "settingsedit") return false; check("Co-op settings from the Play choice (Y): " + settingsView.title); pad("b") },
            () => { sheet = ""; menuIndex = gameMenuItems.findIndex(x => x.act === "coopsettings"); m.gi = menuIndex; sheet = "gamemenu"; menuIndex = m.gi; check("Game menu first row: " + (gameMenuItems[0] ? gameMenuItems[0].title : "")); pad("a") },
            () => { if (sheet !== "settingsedit") return false; check("Co-op settings from the game menu: " + settingsView.title); pad("b") },
            () => { sheet = ""; tab = 0; zone = "main"; modIndex = Math.max(0, listItems.findIndex(x => x.kind === "mod" && isSeamless(x.m))); pad("x"); menuIndex = modMenuItems.findIndex(x => x.act === "settings"); check("Mod menu: " + modMenuTitles()); pad("a") },
            () => { if (sheet !== "settingsedit") return false
                    check("Co-op settings from the mod's X menu: " + settingsView.title)
                    const t = settingsView.fields.find(x => x.key === "allow_invaders"), n = settingsView.fields.find(x => x.key === "enemy_health_scaling")
                    if (t) settingsView.setValue(t, t.value === "1" ? "0" : "1")
                    if (n) settingsView.setValue(n, "50")
                    check("Co-op edits: allow_invaders to " + (t ? settingsView.valueOf(t) : "?") + ", enemy_health_scaling to 50") },
            () => { if (settingsView.changes > 0) return false; check("Co-op edits saved by themselves: 0 waiting"); pad("b") },
            // 0.62: the Co-op tile is asked about once after Seamless Co-op is installed (never by itself otherwise),
            // and has its row on the Co-op settings page.
            () => { if (sheet !== "") return false; m.asks0 = coopTileAsks; m.r1 = maybeAskCoopTile()
                    m.t = sheet === "msg" && msgData ? msgData.title + " | " + msgData.buttons.map(x => x.label).join("; ") : "sheet \"" + sheet + "\""; pad("b") },
            () => { m.s = sheet; m.r2 = maybeAskCoopTile()
                    check("Co-op tile question: asked by itself " + m.asks0 + " times; after the install: " + (m.r1 ? "asked" : "not asked") + ", " + m.t + "; B: sheet \"" + m.s + "\"; a second time: " + (m.r2 ? "asked again" : "not asked"))
                    sheet = ""; openCoopSettings() },
            () => { if (sheet !== "settingsedit") return false
                    const i = settingsView.shown.findIndex(r => r.kind === "action"); m.row = i >= 0 ? settingsView.shown[i] : null
                    if (i >= 0) settingsView.focusIndex = i
                    m.a = binds().a.label; pad("a") },
            () => { check("Co-op settings page: row " + (m.row ? m.row.title + " [" + m.row.label + "]" : "missing") + ", A " + m.a + ", A opens: " + (sheet === "msg" && msgData ? msgData.title : "sheet \"" + sheet + "\"")); pad("b") }
        ]
        // 0.6 (I8): Edit settings on the mod's panel: a tap and Right then A open that mod's settings; no button
        // for a mod without settings.
        // 0.6 (the owner's rule): A selects and B goes back one step, on every screen and sheet, by real presses. Each
        // case starts on My mods, opens its screen, reads what A is bound to, presses B and checks where it landed.
        else if (name === "abcheck") {
            const where = () => [page, "tab " + tab, "zone " + zone, "sheet " + (sheet || "none"), modPage ? "mod page" : "", wheelOpen ? "wheel" : ""].filter(x => x).join(", ")
            const home = () => { sheet = ""; if (wheelOpen) wheelOpen = false; modPage = false; history = []; page = "game"; tab = 0; zone = "main"; modIndex = 0 }
            const sheetCase = (label, open) => [label, open, () => sheet === "" && page === "game" && tab === 0 && !modPage]
            const cases = [
                sheetCase("Mod menu", () => { menuIndex = 0; sheet = "modmenu" }),
                sheetCase("Game menu", () => { menuIndex = 0; sheet = "gamemenu" }),
                sheetCase("Add from Downloads", () => openAdd()),
                sheetCase("Loadouts", () => openProfiles()),
                sheetCase("Play options", () => openPlayOptions()),
                sheetCase("Find pop-up", () => openFindPop()),
                sheetCase("Before you play", () => { checks = [{ level: "warn", title: "A test warning", detail: "" }]; checkIndex = 0; sheet = "check" }),
                sheetCase("Mod loader", () => { loaderFor = ["Lantern Glow SKSE"]; openLoader() }),
                sheetCase("Activity log", () => openActivity()),
                sheetCase("Removed mods", () => openRemoved()),
                ["Quick wheel", () => pad("view"), () => !wheelOpen && sheet === "" && page === "game" && tab === 0],
                ["Settings", () => openSettings(), () => page === "game" && tab === 0 && sheet === ""],
                ["Get mods", () => { tab = 1; zone = "main" }, () => tab === 0 && page === "game"],
                ["A mod's page", () => { tab = 1; zone = "main"; if (getItems.length > 0) openDetails(getItems[0]) }, () => tab === 1 && !modPage && page === "game"],
                ["Play button", () => { zone = "play" }, () => zone === "main" && page === "game"]
            ]
            let steps = [() => { if (mods.length < 2 || !modsLoaded) return false; m.bad = 0; m.n = 0; home() }]
            for (const cs of cases) {
                steps.push(() => { home(); cs[1]() })
                steps.push(() => {
                    if (cs[0] === "A mod's page" && !modPage) return false
                    if (cs[0] === "Quick wheel" && !wheelOpen) return false
                    if (cs[0].indexOf("menu") >= 0 && cs[0] !== "Mod menu" && cs[0] !== "Game menu" && sheet === "") return false
                    const o = binds()
                    m.a = o.a && o.a.label ? o.a.label : (wheelOpen ? "(the wedge)" : "")
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
            steps.push(() => { home(); check("A and B (" + width + "x" + height + "): " + m.n + " screens, " + m.bad + " problems") })
            script = steps
        }
        // 0.6 (the owner's rule): nothing goes through a pop-up. With each sheet, menu or the wheel open, a real click
        // on Play in the bar and on the first game in the rail (both under the pop-up's backdrop) must start nothing
        // and change nothing beneath.
        else if (name === "passthrough") {
            const cases = [
                ["Mod menu", () => { menuIndex = 0; sheet = "modmenu" }],
                ["Game menu", () => { menuIndex = 0; sheet = "gamemenu" }],
                ["Add from Downloads", () => openAdd()],
                ["Loadouts", () => openProfiles()],
                ["Play options", () => openPlayOptions()],
                ["Find pop-up", () => openFindPop()],
                ["Before you play", () => { checks = [{ level: "warn", title: "A test warning", detail: "" }]; checkIndex = 0; sheet = "check" }],
                ["Mod loader", () => { loaderFor = ["Lantern Glow SKSE"]; openLoader() }],
                ["Activity log", () => openActivity()],
                ["Delete for good", () => askDelete("check:none", "Test")],
                ["Quick wheel", () => pad("view")]
            ]
            const state = () => [game ? game.id : "", tab, page, mods.map(x => x.enabled ? 1 : 0).join(""), playAt].join("|")
            let steps = [() => { if (mods.length < 2 || !modsLoaded) return false; m.bad = 0; m.n = 0; sheet = ""; page = "game"; tab = 0; zone = "main" }]
            for (const cs of cases) {
                for (const target of ["Play", "the rail"]) {
                    steps.push(() => { Turbine.pointerHide(); sheet = ""; if (wheelOpen) wheelOpen = false; page = "game"; tab = 0; zone = "main"; cs[1]() })
                    steps.push(() => {
                        if (sheet === "" && !wheelOpen) return false
                        // The rail: a tile of another game that is on screen now (tiles out of view aren't made).
                        const railTile = () => { for (let i = 0; i < games.length; i++) { if (game && games[i].id === game.id) continue
                                                 const t = railList.itemAtIndex(i); if (!t || !t.visible) continue
                                                 const y0 = t.mapToItem(null, 0, 0).y; if (y0 >= 0 && y0 + t.height <= height) return t } return null }
                        const it = target === "Play" ? playBtn : railTile()
                        if (!it || !it.visible) { m.skip = target + " not shown"; return }
                        m.skip = ""
                        m.before = state(); m.open = sheet || "wheel"
                        const p = it.mapToItem(null, it.width / 2, it.height / 2)
                        Turbine.selftestPointer(p.x, p.y, true, 0)
                    })
                    steps.push(() => {
                        if (m.skip) { check("Pass-through, " + cs[0] + ", a click on " + target + ": skipped (" + m.skip + ")"); return }
                        const after = state()
                        const ok = after === m.before && !(sheet === "check" && m.open !== "check")
                        m.n++
                        if (!ok) m.bad++
                        check("Pass-through, " + cs[0] + ", a click on " + target + ": " + (ok ? "nothing beneath changed" : "PROBLEM: " + m.before + " to " + after) + ", " + m.open + " then " + (sheet || (wheelOpen ? "wheel" : "closed")))
                    })
                }
            }
            steps.push(() => { Turbine.pointerHide(); sheet = ""; check("Pass-through (" + width + "x" + height + "): " + m.n + " clicks, " + m.bad + " problems") })
            script = steps
        }
        // 0.6: the golden path, by real presses only: an empty game, X Add from Downloads, A on the file, the mod is on
        // in My mods, Start held 3 seconds and A play it (Before you play's Play anyway when it asks); then the mod off and on with A, X
        // Uninstall and A Put back.
        else if (name === "golden") script = [
            () => { if (!modsLoaded || !game) return false; sheet = ""; page = "game"; tab = 0; zone = "main"; m.played = playAt
                    check("Journey start: " + game.name + ", " + mods.length + " mods, legend " + legendHints.map(h => h[0].toUpperCase() + " " + h[1]).join(" · ")); pad("x") },
            () => { if (sheet !== "add" || addList.length === 0) return false
                    addIndex = Math.max(0, addList.findIndex(d => d.name.indexOf("Brighter Lanterns") === 0)); check("Journey add: sheet add, " + addList.length + " files, A " + (binds().a ? binds().a.label : "")); pad("a") },
            () => { if (mods.length < 1 || page === "place") return false
                    check("Journey added: " + mods.map(x => x.name + (x.enabled ? " on" : " off")).join(", ") + ", sheet " + (sheet || "none")); sheet = ""; Turbine.selftestPad("start", true) },
            () => { if (sheet !== "pick") return false
                    Turbine.selftestPad("start", false)
                    check("Journey Start held: " + pickTitle + ", focus " + (pickItems[menuIndex] ? pickItems[menuIndex].title : "")); pad("a") },
            () => { if (sheet === "check") { const o = binds(); check("Journey Before you play: A " + (o.a ? o.a.label : "") + " | " + checks.map(c => c.level + " " + c.title).join("; ")); if (checks.some(c => c.level === "stop")) return; act("right"); pad("a"); return false }
                    if (playAt === m.played) return false
                    check("Journey played: Play started " + game.name + " with " + mods.filter(x => x.enabled).length + " mods on") },
            () => { sheet = ""; zone = "main"; tab = 0; modIndex = Math.max(0, listItems.findIndex(x => x.kind === "mod")); m.on = focusedItem && focusedItem.m ? focusedItem.m.enabled : null; pad("a") },
            () => { if (!focusedItem || !focusedItem.m || focusedItem.m.enabled === m.on) return false; m.mid = focusedItem.m.id; pad("a") },
            () => { const r = mods.find(x => x.id === m.mid); if (!r || r.enabled !== m.on) return false
                    check("Journey A off and on: " + r.name + " " + (r.enabled ? "on" : "off") + " again"); pad("x") },
            () => { if (sheet !== "modmenu") return false; const i = modMenuItems.findIndex(x => /^Uninstall/.test(x.title)); if (i < 0) { check("Journey uninstall: no Uninstall row"); return }
                    menuIndex = i; pad("a") },
            () => { if (!focusedItem || focusedItem.kind !== "ghost") return false; check("Journey uninstall: the row says " + focusedItem.m.name + " removed, A " + (binds().a ? binds().a.label : "")); pad("a") },
            () => { if (!mods.some(x => x.id === m.mid)) return false; check("Journey put back: " + mods.find(x => x.id === m.mid).name + " is back in My mods") }
        ]
        // 0.6 (break M1, M2, M8): the Mod Manager Download question by real presses, timed from its first frame: no
        // input in its first second (also a press made then but handled late), the ring moves to Get it at 1.2 s
        // only when nothing was pressed, and a Left after the first second keeps Not now for good.
        else if (name === "mmdguard") {
            const ask = () => nxmAsk("nxm://skyrimspecialedition/mods/1/files/2", { name: game ? game.name : "", game: game ? game.id : "" })
            const since = () => nxmShownAt > 0 ? Turbine.monoMs() - nxmShownAt : -1
            const ask2 = () => nxmAsk("nxm://skyrimspecialedition/mods/1/files/3", { name: game ? game.name : "", game: game ? game.id : "" })
            const words = f => f ? "ring on " + (["Get it", "Not now"][f.ring] || "nothing") + ", A " + f.a + ", B " + f.b + ", input " + (f.quiet ? "off" : "ON") : "nothing read"
            script = [
                () => { if (!game || !modsLoaded) return false; sheet = ""; page = "game"; tab = 0; zone = "main"; ask() },
                () => { if (nxmShownAt === 0) return false; const f = nxmAskFirst || {}
                        pad("a"); check("Guard at first: A " + f.a + ", B " + f.b + ", input " + (f.quiet ? "off" : "ON") + "; A at " + Math.round(since()) + " ms: " + (nxmAsking ? "ignored" : "TAKEN")) },
                // A press made 0.5 s after the question showed, handled 0.9 s later (a stalled window): still ignored.
                () => { if (since() < 1400) return false
                        Turbine.selftestPadAged("a", Math.round(since() - 500))
                        check("Guard late press: made at 500 ms, handled at " + Math.round(since()) + " ms: " + (nxmAsking ? "ignored" : "TAKEN")) },
                () => { const o = binds(); check("Guard after the quiet second: ring on " + (pickItems[menuIndex] ? pickItems[menuIndex].title : "") + ", A " + (o.a ? o.a.label : "")); pad("b") },
                () => { if (sheet !== "") return false; ask() },
                () => { if (since() < 1050) return false; pad("right"); pad("left") },
                () => { if (since() < 1700) return false; const o = binds()
                        check("Guard Left at about 1 s: ring on " + (pickItems[menuIndex] ? pickItems[menuIndex].title : "") + " at " + Math.round(since()) + " ms, A " + (o.a ? o.a.label : ""))
                        pad("a"); check("Guard A then: " + (nxmAsking ? "still asking" : sheet === "" && nxmAskLink === "" ? "closed, nothing downloaded" : "other")) },
                // 0.61 (N3): an A made at 1.1 s (ring still on Not now) but handled after the ring moved: Not now.
                () => { if (sheet !== "") return false; ask() },
                () => { if (since() < 1400) return false
                        m.gets = nxmGets; m.ring = pickItems[menuIndex] ? pickItems[menuIndex].title : ""
                        Turbine.selftestPadAged("a", Math.round(since() - 1100))
                        check("Guard stalled A: made at 1100 ms, ring on " + m.ring + ", handled at " + Math.round(since()) + " ms: " + (nxmAsking ? "still asking" : nxmGets === m.gets && sheet === "" ? "Not now, nothing downloaded" : "GOT IT")) },
                // 0.62: asked twice in a row and with names Nexus hasn't said yet (a file never asked about), read in
                // the very turn it opens and again at its first frame: on Not now, A and B Not now, input off, each
                // time. A second link while it asks is dropped and changes nothing; the names coming late change the
                // title only.
                () => { if (sheet !== "") return false
                        m.n = nxmAsks; ask2(); m.one = words(nxmAskFirst); m.title = pickTitle
                        ask2(); check("Guard twice, the first: " + m.title + ": " + m.one + "; a second link at once: " + words(nxmAskState()) + ", " + (nxmAsks === m.n + 1 ? "dropped" : "ASKED AGAIN")) },
                () => { if (nxmShownAt === 0) return false
                        check("Guard twice, at its first frame: " + words(nxmAskFrame))
                        m.ring = menuIndex; m.at = nxmShownAt
                        nxmNameGot({ link: nxmAskLink, name: "SkyUI", file: "SkyUI 5.2SE" }, false)
                        check("Guard names late: " + pickTitle + ", ring " + (menuIndex === m.ring ? "unchanged" : "MOVED") + ", the quiet second " + (nxmShownAt === m.at ? "unchanged" : "RESTARTED"))
                        Turbine.selftestPadAged("a", Math.max(0, Math.round(since() - 300)))
                        check("Guard names late, A made at 300 ms: " + (nxmAsking ? "ignored" : "TAKEN")) },
                () => { if (since() < 1050) return false; pad("left"); pad("b") },
                // Straight after it closed, the same link again (no wait between the two questions).
                () => { if (sheet !== "") return false
                        m.gets2 = nxmGets; m.n = nxmAsks; ask2(); check("Guard again at once: " + (nxmAsks === m.n + 1 ? "asked" : "NOT ASKED") + ": " + words(nxmAskFirst)); pad("a")
                        check("Guard again at once, A: " + (nxmAsking && nxmGets === m.gets2 ? "ignored" : "TAKEN")) },
                () => { if (nxmShownAt === 0) return false; check("Guard again, at its first frame: " + words(nxmAskFrame)) },
                () => { if (since() < 1050) return false; pad("left"); pad("b") }
            ]
        }
        // 0.6: Mods off from Steam's menu shows in My mods with Turn them back on, by A and by a tap on the row.
        else if (name === "heldcheck") script = [
            () => { if (mods.length < 2 || !modsLoaded) return false; sheet = ""; page = "game"; zone = "main"; tab = 0; m.n = mods.length; Turbine.quickMods(game.id, false) },
            () => { if (heldOff.length < m.n) { check("Held wait: " + heldOff.length + " held, " + mods.filter(x => x.enabled).length + " of " + mods.length + " on"); return false }
                    modIndex = 0
                    check("Held note: row 1 " + (listItems[0] ? listItems[0].kind : "none") + " | " + heldWords + " | legend A " + (legendHints[0] ? legendHints[0][1] : "")) },
            () => { pad("a") },
            () => { if (heldOff.length > 0 || mods.some(x => !x.enabled)) return false
                    check("Held back on by A: " + mods.filter(x => x.enabled).length + " of " + mods.length + " on, note " + (listItems.some(x => x.kind === "held") ? "still there" : "gone")); Turbine.quickMods(game.id, false) },
            () => { if (heldOff.length < m.n) return false; modIndex = 1
                    const it = modList.itemAtIndex(0); if (!it) return false
                    const p = it.mapToItem(null, it.width - app.px(40), it.height / 2); Turbine.selftestPointer(p.x, p.y, true, 0) },
            () => { if (heldOff.length > 0 || mods.some(x => !x.enabled)) return false
                    check("Held back on by a tap: " + mods.filter(x => x.enabled).length + " of " + mods.length + " on, note " + (listItems.some(x => x.kind === "held") ? "still there" : "gone")); Turbine.pointerHide() }
        ]
        else if (name === "editbutton") script = [
            () => { if (mods.length < 2 || !modsLoaded) return false; sheet = ""; page = "game"; zone = "main"; tab = 0
                    m.with = mods.find(x => settingsFilesOf(x.id).length > 0); m.without = mods.find(x => settingsFilesOf(x.id).length === 0)
                    if (!m.with) { check("Edit button: no mod with settings"); return }
                    modIndex = listItems.findIndex(x => x.kind === "mod" && x.m.id === m.with.id) },
            () => { if (!panelEdit.visible) { if (!showModPanel) { check("Edit button: the panel doesn't fit here"); scriptStep = script.length; return } return false }
                    const p = panelEdit.mapToItem(null, panelEdit.width / 2, panelEdit.height / 2); Turbine.selftestPointer(p.x, p.y, true, 0) },
            () => { if (sheet !== "settingsedit") return false
                    check("Edit button tapped: sheet settingsedit for " + (settingsMod === m.with.id ? m.with.name : "another mod (" + settingsMod + ")")); Turbine.pointerHide(); pad("b") },
            () => { if (sheet !== "") return false; modIndex = listItems.findIndex(x => x.kind === "mod" && x.m.id === m.with.id); zone = "main"; pad("right"); check("Edit button by Right: zone " + zone + ", legend A " + (legendHints[0] ? legendHints[0][1] : "")); pad("a") },
            () => { if (sheet !== "settingsedit") return false; check("Edit button by Right then A: sheet settingsedit for " + (settingsMod === m.with.id ? m.with.name : settingsMod)); pad("b") },
            () => { if (sheet !== "") return false; zone = "main"
                    if (m.without) { modIndex = listItems.findIndex(x => x.kind === "mod" && x.m.id === m.without.id) }
                    check("Edit button for a mod without settings (" + (m.without ? m.without.name : "none") + "): " + (panelEdit.visible ? "PROBLEM: shown" : "none")) }
        ]
        // 0.6: every sheet and menu ends above the bottom bar at this Interface size and window: its card, and its
        // last row once scrolled to the end.
        else if (name === "menufit") {
            const cases = [
                ["Mod menu", () => { tab = 0; zone = "main"; modIndex = 0; menuIndex = 0; sheet = "modmenu" }],
                ["Game menu", () => { menuIndex = 0; sheet = "gamemenu" }],
                ["Game menu, last row", () => { sheet = "gamemenu"; for (let i = 0; i < gameMenuItems.length; i++) act("down") }],
                ["Add from Downloads", () => { openAdd() }],
                ["Loadouts", () => { openProfiles() }],
                ["Play options", () => { openPlayOptions() }],
                ["Find pop-up", () => { openFindPop() }],
                ["Before you play", () => { checks = [{ level: "warn", title: "A test warning", detail: "" }, { level: "warn", title: "Another", detail: "" }]; checkIndex = 0; sheet = "check" }],
                ["Loader", () => { loaderFor = ["Lantern Glow SKSE"]; openLoader() }],
                ["Delete for good", () => { askDelete("check:none", "Test") }],
                ["Activity log", () => { openActivity() }]
            ]
            let steps = [() => { if (mods.length < 2 || !modsLoaded) return false; page = "game"; zone = "main"; tab = 0; m.bad = 0; m.n = 0 }]
            for (const cs of cases) {
                steps.push(() => { sheet = ""; cs[1]() })
                steps.push(() => {
                    const barTop = legend.visible ? legend.mapToItem(null, 0, 0).y : height
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
            steps.push(() => { sheet = ""; check("Menu fit (" + width + "x" + height + ", size " + sizeNow + "%): " + m.n + " menus, " + m.bad + " problems") })
            script = steps
        }
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
                    // 0.62: Start runs nothing on a sheet (a tap of Start never plays).
                    const want = { a: bs.labels[bs.index] || "", b: bs.cancel >= 0 ? bs.labels[bs.cancel] : "", start: "" }
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
        // 0.58: Interface size goes smaller by every input: D-pad Left, the - button clicked with the pointer, and A's
        // list (B cancels it). Its buttons are at least 44 pixels.
        else if (name === "sizeinputs") script = [
            () => { openSection("Look"); zone = "settings"; setRow = Math.max(0, lookRows.findIndex(r => r.key === "size")); sizeTrial = 100 },
            () => { m.row = lookList.itemAtIndex(setRow); if (!m.row || !m.row.minus) return false
                    check("Size buttons: - " + Math.round(m.row.minus.width) + " x " + Math.round(m.row.minus.height) + ", + " + Math.round(m.row.plus.width) + " x " + Math.round(m.row.plus.height) + " pixels" + (m.row.minus.width >= 44 && m.row.plus.height >= 44 ? "" : "  PROBLEM: under 44"))
                    m.was = sizeNow; pad("left"); check("Size first Left after Settings opened: " + m.was + "% to " + sizeNow + "%" + (sizeNow === m.was ? ", unchanged" : "  PROBLEM: the first press changed it"))
                    m.was = sizeNow; pad("left"); check("Size by D-pad Left: " + m.was + "% to " + sizeNow + "%" + (sizeNow < m.was ? ", smaller" : "  PROBLEM: not smaller")) },
            () => { m.row = lookList.itemAtIndex(setRow); if (!m.row) return false
                    const p = m.row.minus.mapToItem(null, m.row.minus.width / 2, m.row.minus.height / 2)
                    m.was = sizeNow; Turbine.selftestPointer(p.x, p.y, true, 0); m.t = Date.now() },
            () => { if (Date.now() - m.t < 400) return false
                    check("Size by the - button (pointer click): " + m.was + "% to " + sizeNow + "%" + (sizeNow < m.was ? ", smaller" : "  PROBLEM: not smaller")); Turbine.pointerHide() },
            () => { m.row = lookList.itemAtIndex(setRow); if (!m.row) return false
                    const p = m.row.plus.mapToItem(null, m.row.plus.width / 2, m.row.plus.height / 2)
                    m.was = sizeNow; Turbine.selftestPointer(p.x, p.y, true, 0); m.t = Date.now() },
            () => { if (Date.now() - m.t < 400) return false
                    check("Size by the + button (pointer click): " + m.was + "% to " + sizeNow + "%" + (sizeNow > m.was ? ", bigger" : "  PROBLEM: not bigger")); Turbine.pointerHide()
                    m.was = sizeNow; pad("a") },
            () => { check("Size A: " + (sheet === "pick" && pickTitle === "Interface size" ? "the list of sizes, on " + (pickItems[menuIndex] ? pickItems[menuIndex].title : "?") + ", " + pickItems.length + " sizes" : "PROBLEM: no list (sheet \"" + sheet + "\")"))
                    pad("up"); pad("b") },
            () => { check("Size list B: " + m.was + "% to " + sizeNow + "%" + (sizeNow === m.was && sheet === "" ? ", unchanged" : "  PROBLEM: changed or still open")); pad("a") },
            () => { pad("up"); pad("up"); pad("a") },
            () => { check("Size by A's list: " + m.was + "% to " + sizeNow + "%" + (sizeNow < m.was ? ", smaller" : "  PROBLEM: not smaller"))
                    m.was = sizeNow; pad("right"); check("Size by D-pad Right: " + m.was + "% to " + sizeNow + "%" + (sizeNow > m.was ? ", bigger" : "  PROBLEM: not bigger"))
                    sizeTrial = 50; check("Size range: " + sizeSteps[0] + "% to " + sizeSteps[sizeSteps.length - 1] + "% in " + sizeSteps.length + " steps") },
            () => { check("At 50%: text at least " + fpx(10) + " pixels, buttons " + tpx(36) + " pixels") }
        ]
        else if (name === "inputcheck") script = [
            () => { if (mods.length < 2) return false
                    page = "game"; zone = "main"; tab = 0; modIndex = 0; sheet = ""
                    m.gid = game ? game.id : ""
                    m.bad = 0
                    m.ok = (good, text) => { if (!good) m.bad++; check(text + (good ? "" : "  PROBLEM")) } },
            // 1. Holds run only while a real press is held: never from a click, a tap, R3, or a lost release. Each step
            // opens its own Delete for good sheet ("check:" only logs) and counts what finished during it.
            () => { askDelete("check:none", "Test") },
            () => { const it = confirmSheet.buttonItem(0); if (!it || !confirmSheet.visible) return false
                    m.d = holdsDone; const p = it.mapToItem(null, it.width / 2, it.height / 2); Turbine.selftestPointer(p.x, p.y, true, 0); m.t = Date.now() },
            () => { if (Date.now() - m.t < 2500) return false
                    m.ok(holdsDone === m.d && !confirmHold.running && sheet === "confirm", "Hold check: a click on Hold A to delete: " + (confirmHold.running || holdsDone > m.d ? "the hold ran" : "only the highlight moved"))
                    Turbine.pointerHide(); askDelete("check:none", "Test"); m.d = holdsDone; pad("r3"); m.t = Date.now() },
            () => { if (Date.now() - m.t < 2500) return false
                    m.ok(holdsDone === m.d && !confirmHold.running, "Hold check: R3 on Hold A to delete: " + (confirmHold.running || holdsDone > m.d ? "the hold ran" : "no hold"))
                    askDelete("check:none", "Test"); Turbine.selftestPad("a", true); m.run = confirmHold.running; windowActive(false); windowActive(true)
                    m.ok(m.run && !confirmHold.running, "Hold check: A held, then Turbine in the background: " + (m.run ? "the hold started, " : "the hold never started, ") + (confirmHold.running ? "still running" : "stopped"))
                    Turbine.selftestPad("a", false) },
            () => { askDelete("check:none", "Test"); Turbine.selftestPad("a", true); m.run = confirmHold.running; Turbine.selftestPad("lost", false)
                    m.ok(m.run && !confirmHold.running, "Hold check: A held, then the controller went away: " + (m.run ? "" : "never started, ") + (confirmHold.running ? "still running" : "stopped"))
                    Turbine.selftestPad("a", false) },
            () => { askDelete("check:none", "Test"); Turbine.selftestPad("a", true); m.run = confirmHold.running; sheet = "removed"
                    m.ok(m.run && !confirmHold.running, "Hold check: A held, then the sheet changed: " + (m.run ? "" : "never started, ") + (confirmHold.running ? "still running" : "stopped"))
                    Turbine.selftestPad("a", false) },
            () => { askDelete("check:none", "Test"); m.d = holdsDone; Turbine.selftestPad("a", true); m.t = Date.now() },
            () => { if (Date.now() - m.t < 2600) return false
                    Turbine.selftestPad("a", false)
                    m.ok(holdsDone === m.d + 1, "Hold check: A really held for 2 seconds: " + (holdsDone === m.d + 1 ? "Delete for good ran" : "nothing ran")) },
            () => { sheet = ""; sheet = "gamemenu"; m.ri = gameMenuItems.findIndex(x => x.act === "reset"); if (m.ri < 0) return false
                    m.d = holdsDone; menuIndex = m.ri; pad("a"); m.t = Date.now() },
            () => { if (Date.now() - m.t < 1000) return false
                    m.ok(holdsDone === m.d && !resetHold.running, "Hold check: a tap of A on Reset game files: " + (resetHold.running ? "still counting" : "stopped at the release"))
                    menuIndex = m.ri; Turbine.selftestPad("a", true); m.run = resetHold.running; pad("b")
                    m.ok(m.run && !resetHold.running && sheet === "", "Hold check: A held on Reset, then B closed the menu: " + (m.run ? "" : "never started, ") + (resetHold.running ? "still counting" : "stopped"))
                    Turbine.selftestPad("a", false) },
            () => { sheet = "gamemenu"; menuIndex = m.ri },
            () => { const it = gameMenuView.entryItem(m.ri); if (!it || !gameMenuView.visible) return false
                    m.d = holdsDone; const p = it.mapToItem(null, it.width / 2, it.height / 2); Turbine.selftestPointer(p.x, p.y, true, 0); m.t = Date.now() },
            () => { if (Date.now() - m.t < 1000) return false
                    m.ok(holdsDone === m.d && !resetHold.running, "Hold check: a click on Reset game files: " + (resetHold.running ? "counting" : "no hold"))
                    Turbine.pointerHide(); sheet = "gamemenu"; menuIndex = m.ri; pad("r3"); m.t = Date.now() },
            () => { if (Date.now() - m.t < 1000) return false
                    m.ok(holdsDone === m.d && !resetHold.running, "Hold check: R3 on Reset game files: " + (resetHold.running ? "counting" : "no hold"))
                    pad("b") },
            // 7. RT and R3 hold the left button each on their own.
            () => { sheet = ""; probeOn = true; Turbine.pointerShowAt(width / 2, height / 2); m.pe = padEvents
                    Turbine.selftestPad("rt", true); const a = inputProbe.pressed
                    Turbine.selftestPad("r3", true); Turbine.selftestPad("r3", false); const b = inputProbe.pressed
                    Turbine.selftestPad("rt", false); const c = inputProbe.pressed
                    Turbine.selftestPad("r3", true); const d = inputProbe.pressed
                    Turbine.selftestPad("rt", true); Turbine.selftestPad("rt", false); const e = inputProbe.pressed
                    Turbine.selftestPad("r3", false); const f = inputProbe.pressed
                    m.ok(a && b && !c && d && e && !f && padEvents === m.pe, "RT and R3: R3 let go while RT holds: " + (b ? "still held" : "dropped") + "; RT let go: " + (c ? "stuck" : "let go")
                         + "; RT let go while R3 holds: " + (e ? "still held" : "dropped") + "; R3 let go: " + (f ? "stuck" : "let go") + "; presses sent on to the interface: " + (padEvents - m.pe))
                    probeOn = false; Turbine.pointerHide() },
            // 2. A click anywhere on a row is A; rows are at least 44 px.
            () => { page = "game"; zone = "main"; tab = 0; modIndex = 0; const it = modList.itemAtIndex(1); if (!it || mods.length < 2) return false
                    m.id = listItems[1].m.id; m.on = mods.find(x => x.id === m.id).enabled
                    const p = it.mapToItem(null, it.width * 0.6, it.height / 2); Turbine.selftestPointer(p.x, p.y, true, 0) },
            () => { const x = mods.find(y => y.id === m.id), it = modList.itemAtIndex(1)
                    m.ok(!!x && x.enabled !== m.on && modIndex === 1, "Click check: a click on a mod row's name: " + (!!x && x.enabled !== m.on ? "turned it " + (x.enabled ? "on" : "off") : "did nothing") + ", focus on row " + (modIndex + 1))
                    m.ok(!!it && it.height >= 44, "Click check: a mod row is " + (it ? Math.round(it.height) : 0) + " px tall")
                    Turbine.pointerHide() },
            () => { if (!ordered || !(rules && rules.plugins)) return
                    tab = 2; zone = "main"; if (order.length === 0) return false
                    m.pi = order.findIndex(x => !x.fixed); if (m.pi < 0) return
                    m.pa = order[m.pi].active; const it = loadOrderView.rowItem(m.pi); if (!it) return false
                    const p = it.mapToItem(null, it.width * 0.7, it.height / 2); Turbine.selftestPointer(p.x, p.y, true, 0) },
            () => { if (tab !== 2 || m.pi === undefined || m.pi < 0) return
                    const it = loadOrderView.rowItem(m.pi)
                    m.ok(order[m.pi] && order[m.pi].active !== m.pa, "Click check: a click on a load order row: " + (order[m.pi] && order[m.pi].active !== m.pa ? "turned it " + (order[m.pi].active ? "on" : "off") : "did nothing"))
                    m.ok(!!it && it.height >= 44, "Click check: a load order row is " + (it ? Math.round(it.height) : 0) + " px tall")
                    Turbine.pointerHide(); tab = 0 },
            () => { openSettings(); setSection = settingsSections.indexOf("Look"); m.row = lookRows.findIndex(r => r.key === "theme"); m.theme = look.theme },
            // (The next look: the Settings row is laid out where it shows.)
            () => { const it = lookList.itemAtIndex(m.row); if (!it || !lookList.visible) return false
                    const p = it.mapToItem(null, app.px(40), it.height / 2); Turbine.selftestPointer(p.x, p.y, true, 0) },
            () => { const it = lookList.itemAtIndex(m.row)
                    m.ok(look.theme !== m.theme && setRow === m.row, "Click check: a click on a Settings choice row: " + (look.theme !== m.theme ? "changed it" : "did nothing"))
                    m.ok(!!it && it.height >= 44, "Click check: a Settings row is " + (it ? Math.round(it.height) : 0) + " px tall")
                    look.theme = m.theme; Turbine.pointerHide(); settingsBack(); zone = "main"; tab = 0 },
            // 3. The removed row never sends a move or an A to the wrong mod.
            () => { if (mods.length < 2) return false; modIndex = 0; m.goneId = mods[0].id; uninstall(mods[0]) },
            () => { if (!ghost) return false; modIndex = 1; m.want = focusedItem && focusedItem.m ? focusedItem.m.id : ""; startMove()
                    const now = moving && mods[modIndex] ? mods[modIndex] : null
                    m.ok(!!now && now.id === m.want && !ghost, "Ghost check: Move with the removed row showing moves " + (now ? now.name : "nothing") + (now && now.id === m.want ? ", the focused mod" : ", not the focused mod"))
                    cancelMove(); putBack(m.goneId) },
            () => { if (!mods.some(x => x.id === m.goneId)) return false; modIndex = mods.findIndex(x => x.id === m.goneId); uninstall(mods[modIndex]) },
            () => { if (!ghost) return false; modIndex = ghost.index; ghostTimer.stop(); ghostExpired()
                    m.next = focusedItem && focusedItem.kind === "mod" ? focusedItem.m.id : ""; m.nextOn = m.next ? mods.find(x => x.id === m.next).enabled : false
                    pad("a") },
            () => { if (!mods.some(x => x.id === m.goneId)) return false
                    const nx = mods.find(x => x.id === m.next)
                    m.ok(!nx || nx.enabled === m.nextOn, "Ghost check: A just as the removed row went: put back, the mod that slid up " + (nx && nx.enabled !== m.nextOn ? "changed too" : "untouched")) },
            // 10. A sheet that comes up during the tutorial waits for it.
            () => { openTutorial(0); oldHelperOnce("0.1")
                    m.ok(sheet === "tutorial" && pendingOldHelper, "Tutorial check: the out of date helper sheet " + (sheet === "tutorial" ? "waits for the tutorial" : "replaced it"))
                    tutorialAct("b")
                    m.ok(sheet === "oldhelper", "Tutorial check: after the tutorial: sheet \"" + sheet + "\""); sheet = "" },
            // 11. Every game hidden: the rail offers them back.
            () => { hiddenTest = allGames.map(g => g.id); orderGames(); zone = "rail"; railIndex = 0
                    const o = binds()
                    m.ok(games.length === 0 && railExtra === 1 && !!o.a && o.a.label === "Show hidden games", "Hidden check: every game hidden, the rail shows " + (railExtra === 1 ? "Show hidden games" : "nothing to bring them back"))
                    pad("a") },
            () => { m.ok(games.length > 0, "Hidden check: Show hidden games put back " + plural(games.length, "game", "games"))
                    hiddenTest = null; orderGames() },
            // 11b. 0.61 (T39): Confirm by pressing twice: the sheet's button says "Press A again", no toast repeats it.
            () => { m.cb = look.confirmBy; look.confirmBy = "twice"; toast.hide(); askDelete("check:none", "Test") },
            () => { if (sheet !== "confirm") return false
                    pad("a")
                    m.ok(sheet === "confirm" && !(toast.shown && toast.text.indexOf("Press A again") === 0), "Press twice check: first A on Delete for good: " + (toast.shown && toast.text.indexOf("Press A again") === 0 ? "a toast repeats it" : "the button says it, no toast"))
                    disarm(); sheet = ""; look.confirmBy = m.cb },
            // 11c. 0.61 (I18): Add from Downloads' Y is Find; its first row chooses which files show.
            () => { if (!game) return false; sheet = ""; openAdd() },
            () => { if (sheet !== "add") return false
                    const o = binds(); m.addY = o.y ? o.y.label : ""; pad("y") },
            () => { if (sheet !== "pick") return false
                    m.addRows = pickItems.map(x => x.title).join(", "); pad("down"); pad("a") },
            () => { if (sheet !== "add") return false
                    m.ok(m.addY === "Find" && m.addRows === "This game's files, All files" && addAll, "Find check: Add from Downloads Y " + m.addY + " (" + m.addRows + "), All files " + (addAll ? "showing" : "not showing"))
                    pad("b") },
            // 12. 0.61 (M6): Y is Find even while a toast offers Undo; R3 runs the toast's Undo.
            // The hidden-games check above can leave another game selected: go back to the one with the two mods.
            () => { if (!game) return false
                    if (m.gid && game.id !== m.gid) { const gi = games.findIndex(x => x.id === m.gid); if (gi >= 0) railSelect(gi); return false }
                    if (mods.length === 0) return false
                    page = "game"; zone = "main"; tab = 0; modIndex = 0; sheet = ""; Turbine.pointerHide()
                    if (!mods.some(x => x.enabled)) { Turbine.setEnabled(mods[0].id, true); mods = mods.map((x, i) => i === 0 ? Object.assign({}, x, { enabled: true }) : x) }
                    m.wasOn = mods.filter(x => x.enabled).length
                    allOff()
                    m.ok(!!toastAction && toast.actionGlyph === glyph("r3"), "Toast check: Turn every mod off shows " + toast.actionGlyph + " " + toast.actionLabel)
                    m.ok(legendHints.length > 2 && legendHints[2][0] === "r3" && legendHints[2][1] === "Undo", "Toast check: the legend while it shows: " + legendHints.map(h => h[0].toUpperCase() + " " + h[1]).join(" · "))
                    pad("y") },
            () => { m.ok(sheet === "findpop" && !!toastAction, "Toast check: Y with Undo showing: " + (sheet === "findpop" ? "Find opened" : "sheet \"" + sheet + "\"") + (toastAction ? ", Undo kept" : ", Undo gone"))
                    pad("b") },
            () => { if (sheet !== "") return false
                    pad("r3")
                    const on = mods.filter(x => x.enabled).length
                    m.ok(on === m.wasOn && sheet === "", "Toast check: R3 with Undo showing: " + plural(on, "mod", "mods") + " back on of " + m.wasOn)
                    m.ok(!legendHints.some(h => h[0] === "r3"), "Toast check: the legend after it: " + legendHints.map(h => h[0].toUpperCase() + " " + h[1]).join(" · "))
                    check("Input check: " + m.bad + " problems") }
        ]
        else if (name === "nexusguard" || name === "inlinebrowser") script = [
            () => { if (getItems.length === 0) return false; openDetails(getItems[0]) },
            () => { if (!detailsData || detailsData.loading || mainLabel !== "Get it on Nexus") return false; m.before = launchCount
                    pad("a"); if (name === "nexusguard") { pad("a"); pad("a") } },
            () => { if (name === "nexusguard") { check("Nexus guard: 3 presses of A, " + (launchCount - m.before) + " browser launch" + (launchCount - m.before === 1 ? "" : "es") + ", button \"" + mainLabel + "\""); return }
                    openHelpNow() },
            () => { if (name === "inlinebrowser") check("Inline browser buttons: " + (pageHelpLabels.length > 0 ? pageHelpLabels.join("; ") : "none") + " (Nexus " + (page === "browser" ? "open in Turbine" : "not open") + "), sheet \"" + sheet + "\"") }
        ]
        // 0.62: Start. A tap opens Settings and a tap or B goes back; a hold let go early does nothing; a 3 second
        // hold shows the Play choice (or plays, by the setting); A on Play plays; Ctrl+Enter is the hold; Start on a
        // menu never plays; a tap on the bar's Start hint is a tap of Start. All by real presses, keys and clicks.
        else if (name === "startcheck") {
            m.bad = 0
            const say = (ok, text) => { if (!ok) m.bad++; check("Start check: " + text + (ok ? "" : "  PROBLEM")) }
            const holdRow = () => lookRows.findIndex(r => r.key === "holdStartDoes")
            script = [
                () => { if (mods.length < 2 || !modsLoaded) return false; sheet = ""; page = "game"; zone = "main"; tab = 0; modIndex = 0; m.asks = playAsks },
                () => { pad("start"); say(page === "settings" && playAsks === m.asks, "a tap opens " + (page === "settings" ? "Settings" : page) + ", legend " + legendHints.map(h => h[0].toUpperCase() + " " + h[1]).join(" · ")) },
                () => { pad("start"); say(page === "game" && zone === "main", "a tap again goes back to " + (page === "game" ? "My mods" : page)) },
                () => { pad("start"); m.was = page; pad("b"); say(m.was === "settings" && page === "game", "B from Settings goes back to " + (page === "game" ? "My mods" : page)) },
                // A hold let go at 1.5 s: the ring was filling, and nothing happens.
                () => { m.t = Date.now(); Turbine.selftestPad("start", true) },
                () => { if (Date.now() - m.t < 1500) return false
                        m.part = startHeld; m.ring = startRing.visible
                        Turbine.selftestPad("start", false)
                        say(m.ring && m.part > 0.3 && m.part < 0.9 && page === "game" && sheet === "" && playAsks === m.asks && startHeld === 0,
                            "let go at 1.5 s: the ring was " + (m.ring ? "showing" : "hidden") + " at " + Math.round(m.part * 100) + "%, then page " + page + ", sheet \"" + sheet + "\", Play asked " + (playAsks - m.asks) + " times") },
                // Held for the 3 seconds: the Play choice, not before.
                () => { m.t = Date.now(); Turbine.selftestPad("start", true) },
                () => { if (Date.now() - m.t < 2300) return false; m.early = sheet },
                () => { if (sheet !== "pick" && Date.now() - m.t < 6000) return false
                        const secs = (Date.now() - m.t) / 1000
                        Turbine.selftestPad("start", false)
                        say(m.early === "" && sheet === "pick" && pickTitle === "Play options" && playAsks === m.asks && secs >= 2.9,
                            "held 3 seconds: nothing at 2.3 s (sheet \"" + m.early + "\"), then " + (sheet === "pick" ? pickTitle : "sheet \"" + sheet + "\"") + ", focus " + (pickItems[menuIndex] ? pickItems[menuIndex].title : "none") + ", Play asked " + (playAsks - m.asks) + " times")
                        pad("b") },
                () => { say(sheet === "" && page === "game", "B closes the Play choice: sheet \"" + sheet + "\"") },
                // The setting, changed with A on its row in Settings, Look.
                () => { openSection("Look"); zone = "settings"; setRow = holdRow(); m.words = startHoldWords; pad("a") },
                () => { say(holdRow() >= 0 && look.holdStartDoes === "last" && startHoldWords === "hold: Play", "Hold Start does: row " + (holdRow() >= 0 ? "there" : "missing") + ", A changed it to " + look.holdStartDoes + ", the bar says " + m.words + " then " + startHoldWords)
                        pad("b") },
                () => { if (page !== "game") return false; zone = "main"; m.asks = playAsks; m.t = Date.now(); Turbine.selftestPad("start", true) },
                () => { if (playAsks === m.asks && Date.now() - m.t < 6000) return false
                        const secs = (Date.now() - m.t) / 1000
                        Turbine.selftestPad("start", false)
                        say(playAsks === m.asks + 1 && secs >= 2.9, "Play my last choice: held 3 seconds, Play asked " + (playAsks - m.asks) + " times") },
                // A on the bar's Play button still plays.
                () => { if (Turbine.busy) return false; sheet = ""; page = "game"; zone = "play"; m.asks = playAsks; pad("a") },
                () => { say(playAsks === m.asks + 1, "A on Play: Play asked " + (playAsks - m.asks) + " times") },
                // Back to the Play choice, by A on the row again; Ctrl+Enter is the keyboard's hold.
                () => { if (Turbine.busy) return false; sheet = ""; openSection("Look"); zone = "settings"; setRow = holdRow(); pad("a") },
                () => { pad("b"); sheet = ""; zone = "main"; m.asks = playAsks
                        Turbine.selftestKey(Qt.Key_Return, Qt.ControlModifier, true); Turbine.selftestKey(Qt.Key_Return, Qt.ControlModifier, false) },
                () => { say(look.holdStartDoes === "choice" && sheet === "pick" && pickTitle === "Play options" && playAsks === m.asks, "Ctrl+Enter: " + (sheet === "pick" ? pickTitle : "sheet \"" + sheet + "\"") + ", setting " + look.holdStartDoes)
                        Turbine.selftestKey(Qt.Key_Escape, 0, true); Turbine.selftestKey(Qt.Key_Escape, 0, false) },
                // Start on a menu never plays, and never opens Settings under it.
                () => { say(sheet === "", "Esc closes the Play choice: sheet \"" + sheet + "\""); sheet = ""; zone = "main"; tab = 0; modIndex = 0; pad("x") },
                () => { m.sheet = sheet; m.asks = playAsks; pad("start")
                        say(m.sheet === "modmenu" && sheet === "modmenu" && page === "game" && playAsks === m.asks, "Start on the mod menu: sheet \"" + sheet + "\", page " + page + ", Play asked " + (playAsks - m.asks) + " times, legend " + legendHints.map(h => h[0].toUpperCase() + " " + h[1]).join(" · "))
                        pad("b") },
                // The bar: the Start hint is a button for a finger or a click, and nothing overlaps.
                () => { if (sheet !== "") return false
                        const p = startHint.mapToItem(null, startHint.width / 2, startHint.height / 2)
                        m.hint = startHint.visible; m.h = Math.round(startHint.height)
                        Turbine.selftestMouse(0, p.x, p.y, 0, 0, 0); Turbine.selftestMouse(1, p.x, p.y, Qt.LeftButton, 0, 0); Turbine.selftestMouse(2, p.x, p.y, Qt.LeftButton, 0, 0) },
                () => { say(m.hint && m.h >= 44 && page === "settings", "a click on the Start hint (" + m.h + " px tall): " + (page === "settings" ? "Settings" : page)); pad("start") },
                () => { say(page === "game", "and Start goes back: " + page); zone = "main"; legend.check("startcheck")
                        check("Start check done: " + m.bad + " problems") }
            ]
        }
        // 0.62: Get mods as a page of a book, by real presses and clicks: no game list there, the edges name the pages
        // beside it, Left from the first column and Right from the last turn the page (a held D-pad stops at the
        // edge), LB and RB and a click on an edge turn it too, and the page coming in slides over the other.
        else if (name === "bookcheck") {
            m.bad = 0
            const say = (ok, text) => { if (!ok) m.bad++; check("Book check: " + text + (ok ? "" : "  PROBLEM")) }
            const tabName = () => (tabs[tab] || "").replace(/ \(\d+\)$/, "")
            const click = it => { const p = it.mapToItem(null, it.width / 2, it.height / 2)
                                  Turbine.selftestMouse(0, p.x, p.y, 0, 0, 0); Turbine.selftestMouse(1, p.x, p.y, Qt.LeftButton, 0, 0); Turbine.selftestMouse(2, p.x, p.y, Qt.LeftButton, 0, 0) }
            script = [
                () => { if (!game || !modsLoaded || mods.length < 1) return false; sheet = ""; page = "game"; zone = "main"; tab = 0; modIndex = 0 },
                () => { say(rail.width > 0 && !edgeBack.visible, "My mods: the game list is " + (rail.width > 0 ? "there" : "gone") + ", page edges " + (edgeBack.visible ? "shown" : "none")); m.turns = turnsShown; pad("rb")
                        say(tab === 1 && lastTurnDir === 1 && (mo.moves ? turnTo === 1 && turnLine.visible && turnsShown === m.turns + 1 : !turnLine.visible), "RB: " + tabName() + " comes in from the " + (lastTurnDir > 0 ? "right" : "left")) },
                () => { if (getItems.length < gridColumns || turnTo >= 0 || rail.width > 0) return false
                        say(rail.width === 0 && edgeBack.visible && edgeBack.width >= 44 && edgeNext.visible === bookHasNext && bookHasNext,
                            "Get mods: the game list is " + (rail.width === 0 ? "gone" : "there") + ", left edge " + (edgeBack.visible ? edgeBack.label : "none") + " (" + Math.round(edgeBack.width) + " px), right edge " + (edgeNext.visible ? edgeNext.label : "none"))
                        zone = "main"; getArea = "grid"; getIndex = 1; pad("left")
                        say(tab === 1 && getIndex === 0, "Left inside the row: card " + (getIndex + 1) + ", still " + tabName())
                        pad("left")
                        say(tab === 0 && zone === "main" && lastTurnDir === -1 && modList.visible && (mo.moves ? turnTo === 0 && getView.visible && turnLine.visible : !getView.visible),
                            "Left from the first column: " + tabName() + " comes in from the " + (lastTurnDir < 0 ? "left" : "right") + ", zone " + zone) },
                () => { if (turnTo >= 0 || rail.width < px(72)) return false
                        say(tab === 0 && !getView.visible && rail.width > 0, "The turn done: " + tabName() + ", Get mods " + (getView.visible ? "still showing" : "put away") + ", the game list back"); pad("rb") },
                () => { if (turnTo >= 0 || tab !== 1) return false
                        getArea = "grid"; getIndex = 0; pad("right")
                        say(tab === 1 && getIndex === 1, "Right inside the row: card " + (getIndex + 1) + ", still " + tabName())
                        getIndex = gridColumns - 1; pad("right")
                        say(tab === 2 && lastTurnDir === 1, "Right from the last column: " + tabName() + " comes in from the " + (lastTurnDir > 0 ? "right" : "left")) },
                () => { if (turnTo >= 0) return false; pad("lb"); say(tab === 1 && lastTurnDir === -1, "LB from Load order: " + tabName() + " comes in from the " + (lastTurnDir < 0 ? "left" : "right")) },
                // A held D-pad runs to the first column and stops there: the page stays.
                () => { if (turnTo >= 0) return false; getArea = "grid"; getIndex = gridColumns - 1; m.t = Date.now(); Turbine.selftestPad("left", true) },
                () => { if (Date.now() - m.t < 1500) return false; Turbine.selftestPad("left", false)
                        say(tab === 1 && getIndex === 0, "Left held 1.5 s: card " + (getIndex + 1) + ", " + (tab === 1 ? "the page stayed" : "the page turned to " + tabName())) },
                // The edges are buttons.
                () => { click(edgeBack) },
                () => { if (turnTo >= 0) return false; say(tab === 0, "A click on the left edge: " + tabName()); pad("rb") },
                () => { if (turnTo >= 0 || tab !== 1) return false; click(edgeNext) },
                () => { if (turnTo >= 0) return false; say(tab === 2, "A click on the right edge: " + tabName()); pad("lb") },
                () => { if (turnTo >= 0 || tab !== 1 || rail.width > 0) return false; getArea = "grid"; getIndex = 0; zone = "main"
                        legend.check("bookcheck")
                        say(legendHints.some(h => h[0] === "b" && h[1] === "My mods"), "The bar on Get mods: " + legendHints.map(h => h[0].toUpperCase() + " " + h[1]).join(" · "))
                        check("Book check done: " + m.bad + " problems, animations " + mo.level + ", " + turnsShown + " page turns shown, " + glides + " glides") }
            ]
        }
        // 0.62 (the owner's device): Seamless Co-op started with an empty session password and the mod stopped with
        // an error. Every way a co-op start comes, with the password empty: nothing starts, Co-op settings open on
        // the password with the keyboard, and the start waits for it.
        else if (name === "cooppw") {
            m.bad = 0
            const say = (ok, text) => { if (!ok) m.bad++; check("Co-op password check: " + text + (ok ? "" : "  PROBLEM")) }
            const stopped = () => sheet === "ask" && askReturn === "settingsedit" && coopAfterSettings && playAsks === m.asks
            const focusKey = () => { const r = settingsView.shown[settingsView.focusIndex]; return r && r.f ? r.f.key : "none" }
            const told = how => say(stopped() && focusKey() === "cooppassword", how + ": " + (playAsks === m.asks ? "nothing started" : "Play asked " + (playAsks - m.asks) + " times") + ", sheet " + askReturn + " with the keyboard (" + sheet + "), focus " + focusKey() + ", waiting " + coopAfterSettings)
            const waitAsk = () => sheet === "ask" || Date.now() - m.t > 6500
            // B on the keyboard sheet, then B on the settings: back on the game, nothing waiting, nothing started.
            const close1 = () => { if (sheet === "ask") pad("b") }
            const close2 = how => () => { if (sheet === "settingsedit") pad("b")
                                          say(sheet === "" && !coopAfterSettings && playAsks === m.asks, how + ", then B and B: sheet \"" + sheet + "\", waiting " + coopAfterSettings + ", Play asked " + (playAsks - m.asks) + " times") }
            script = [
                () => { if (!seamlessIn || !modsLoaded) return false; sheet = ""; page = "game"; zone = "main"; tab = 0; acAnswered = ({ "steam:1245620": true }); m.asks = playAsks
                        say(Turbine.coopPasswordState(seamlessMod.id) === 0, "at the start the password is " + ["unknown", "empty", "set"][Turbine.coopPasswordState(seamlessMod.id) + 1])
                        // 0.62: this copy came from Nexus, not Turbine's own download: it counts just the same.
                        say(seamlessMod.seamless === true && seamlessMod.sourceKind === "nexus" && isSeamless(seamlessMod) && rowLine(seamlessMod).text === "Loads only with Play Seamless Co-op" && settingsFilesOf(seamlessMod.id).length > 0,
                            "Seamless Co-op from " + seamlessMod.sourceKind + ": marked " + (seamlessMod.seamless === true ? "seamless" : "not seamless") + ", its row says " + rowLine(seamlessMod).text + ", settings files " + settingsFilesOf(seamlessMod.id).length) },
                // 1: the Play choice (A on Play, then Play Seamless Co-op).
                () => { zone = "play"; pad("a") },
                () => { if (sheet !== "pick") return false; menuIndex = pickItems.findIndex(x => x.key === "coop"); m.t = Date.now(); pad("a") },
                () => { if (!waitAsk()) return false; told("The Play choice") },
                close1, close2("The Play choice"),
                // 2: Start held 3 seconds with "Play my last choice" (the last choice is co-op now).
                () => { look.holdStartDoes = "last"; zone = "main"; m.t = Date.now(); Turbine.selftestPad("start", true) },
                () => { if (sheet !== "ask" && Date.now() - m.t < 6500) return false; Turbine.selftestPad("start", false); told("Start held (last choice " + lastPlayChoice + ")"); look.holdStartDoes = "choice" },
                close1, close2("Start held"),
                // 3: Play options (the quick wheel), Play Seamless Co-op.
                () => { zone = "main"; pad("view"); pad("down"); pad("right"); m.slot = wheelSlots[wheelSlot] ? wheelSlots[wheelSlot].label : ""; pad("a") },
                () => { if (sheet !== "pick") return false; menuIndex = pickItems.findIndex(x => x.title === "Play Seamless Co-op"); m.t = Date.now(); pad("a") },
                () => { if (!waitAsk()) return false; told("Play options (wheel slot " + m.slot + ")") },
                close1, close2("Play options"),
                // 4: the Co-op tile and Steam's menu, when Turbine is open: their request comes in as this line.
                () => { m.t = Date.now(); linkArrived("turbine:quick play-coop " + encodeURIComponent(game.id)) },
                () => { if (!waitAsk()) return false; told("The Co-op tile or Steam's menu") },
                close1, close2("The tile"),
                // 5: a start that got as far as the Play job itself: it stops there too, and says so.
                () => { m.stops = coopStops; m.t = Date.now(); Turbine.playCoop() },
                () => { if (!waitAsk()) return false; say(coopStops === m.stops + 1, "The Play job stopped it itself: " + (coopStops - m.stops) + " times"); told("The Play job") },
                // 6: a pair of quotes with nothing in it is still empty: typed in, nothing starts.
                () => { askView.open("Session password", "", "\"\"", "", 2000, false, "Done"); askView.accept(); m.t = Date.now() },
                () => { if (Date.now() - m.t < 1800) return false
                        say(playAsks === m.asks && Turbine.coopPasswordState(seamlessMod.id) === 0, "A password of two quotes: " + (playAsks === m.asks ? "nothing started" : "Play asked") + ", the password counts as " + ["unknown", "empty", "set"][Turbine.coopPasswordState(seamlessMod.id) + 1])
                        if (sheet === "ask") pad("b") },
                () => { if (sheet === "settingsedit") pad("b") },
                // 0.62: two copies of Seamless Co-op. First the engine's own look, through the bridge and back.
                () => { if (sheet !== "") return false; m.looks = coopLooks; coopFound = null; coopLook() },
                () => { if (coopLooks === m.looks && !coopFound) return false
                        say(!!coopFound && coopFound.installed === true && (coopFound.others || []).length === 0 && sheet === "", "the engine's look at Seamless Co-op: " + (coopFound && coopFound.installed ? "installed (" + coopFound.name + " " + coopFound.listVersion + ")" : "not found") + ", " + (coopFound ? (coopFound.others || []).length : "?") + " other copies, sheet \"" + sheet + "\"")
                        // Then an answer with a second copy and a version that disagrees: the one sheet, and the toast.
                        coopTidyAsked = ({}); coopVersionSaid = ({})
                        m.two = { installed: true, name: "Seamless Co-op", listVersion: "2.0.1", fileVersion: "1.9.0", others: ["Seamless Co-op 1.9.1"], tidied: [],
                                  versionWarning: "Your list says Seamless Co-op 2.0.1, but its file says it is 1.9.0. The mod doesn't run when it is out of date: get it again from its page if the game says so." }
                        coopLooked(game.id, m.two) },
                () => { say(sheet === "msg" && !!msgData && msgData.title === "You have two copies of Seamless Co-op" && /2\.0\.1.*1\.9\.0/.test(lastToast),
                            "two copies, on opening the game: " + (sheet === "msg" && msgData ? msgData.text + " | " + msgData.buttons.map(b => b.label).join("; ") : "sheet \"" + sheet + "\"") + " | toast: " + lastToast.substring(0, 70))
                        pad("b") },
                // Before a co-op start: the same sheet first; Tidy up, and the start goes on (to the password here).
                () => { say(sheet === "" && coopCopies().length === 0, "Not now: sheet \"" + sheet + "\"")
                        coopLooked(game.id, m.two); m.auto = sheet; m.asks = playAsks; zone = "play"; pad("a") },
                () => { if (sheet !== "pick") return false; menuIndex = pickItems.findIndex(x => x.key === "coop"); pad("a") },
                () => { say(m.auto === "" && sheet === "msg" && !!msgData && msgData.title === "You have two copies of Seamless Co-op" && playAsks === m.asks,
                            "two copies, before a co-op start: asked again by itself: " + (m.auto === "" ? "no" : "yes") + "; at the start: " + (sheet === "msg" && msgData ? msgData.title : "sheet \"" + sheet + "\"") + ", Play asked " + (playAsks - m.asks) + " times")
                        m.t = Date.now(); pad("a") },
                () => { if (sheet !== "ask" && Date.now() - m.t < 6500) return false
                        say(sheet === "ask" && coopAfterSettings && playAsks === m.asks, "Tidy up, then the start goes on: " + (sheet === "ask" ? "to the session password (still empty)" : "sheet \"" + sheet + "\""))
                        if (sheet === "ask") pad("b") },
                () => { if (sheet === "settingsedit") pad("b"); check("Co-op password check done: " + m.bad + " problems") }
            ]
        }
        // 0.63: and the keys a PC player expects: W A S D, Q and E, F, R, Esc for Settings on the first page, Tab
        // held for the quick wheel, Space held for Play, letters left to a text field being typed in; the mouse's
        // Forward button, a double-click on a mod, and no key caps after the mouse.
        // 0.62: the keyboard and the mouse drive every screen, by real key presses and clicks: arrows, Enter, Esc
        // and Backspace, Tab and Shift+Tab, Page Down and Page Up, Ctrl+F and /, the Menu key and Shift+F10, F1,
        // Ctrl+= and Ctrl+-, P, V, Ctrl+Enter, Ctrl+Z; hover, click, right-click, the Back button, Ctrl with the
        // wheel; and the bar's hints follow the last input used.
        else if (name === "keycheck") {
            m.bad = 0
            const say = (ok, text) => { if (!ok) m.bad++; check("Key check: " + text + (ok ? "" : "  PROBLEM")) }
            const K = (k, mods) => { Turbine.selftestKey(k, mods || 0, true); Turbine.selftestKey(k, mods || 0, false) }
            const at = (it, fx) => it.mapToItem(null, it.width * (fx || 0.5), it.height / 2)
            const mouse = (p, button) => { Turbine.selftestMouse(0, p.x, p.y, 0, 0, 0); Turbine.selftestMouse(1, p.x, p.y, button, 0, 0); Turbine.selftestMouse(2, p.x, p.y, button, 0, 0) }
            const hints = () => glyph("a") + ", " + glyph("b") + ", " + glyph("x") + ", " + glyph("y") + ", " + glyph("lb") + ", " + glyph("rb")
            script = [
                () => { if (mods.length < 2 || !modsLoaded) return false; keysTest = true; sheet = ""; page = "game"; zone = "main"; tab = 0; modIndex = 0
                        if (!mods[0].enabled) { Turbine.setEnabled(mods[0].id, true); mods = mods.map((x, i) => i === 0 ? Object.assign({}, x, { enabled: true }) : x) } },
                // The keyboard.
                () => { K(Qt.Key_Down); m.r1 = modIndex; K(Qt.Key_Up)
                        say(m.r1 === 1 && modIndex === 0 && lastInput === "keys", "Down and Up: row " + (m.r1 + 1) + " then row " + (modIndex + 1) + ", last input " + lastInput)
                        say(hints() === "Enter, Esc, R, F, Q, E", "the hints after a key: " + hints())
                        m.on = mods[0].enabled; K(Qt.Key_Return) },
                () => { say(mods[0].enabled !== m.on, "Enter is A: the mod turned " + (mods[0].enabled ? "on" : "off")); K(Qt.Key_Return) },
                () => { K(Qt.Key_Menu); m.s1 = sheet; K(Qt.Key_Escape); m.s2 = sheet; K(Qt.Key_F10, Qt.ShiftModifier); m.s3 = sheet; K(Qt.Key_Backspace)
                        say(m.s1 === "modmenu" && m.s2 === "" && m.s3 === "modmenu" && sheet === "", "Menu key: sheet \"" + m.s1 + "\", Esc: \"" + m.s2 + "\", Shift+F10: \"" + m.s3 + "\", Backspace: \"" + sheet + "\"") },
                () => { K(Qt.Key_F, Qt.ControlModifier); m.s1 = sheet; K(Qt.Key_Escape); m.s2 = sheet; K(Qt.Key_Slash); m.s3 = sheet; K(Qt.Key_Escape)
                        say(m.s1 === "findpop" && m.s2 === "" && m.s3 === "findpop" && sheet === "", "Ctrl+F: sheet \"" + m.s1 + "\", Esc: \"" + m.s2 + "\", /: \"" + m.s3 + "\", Esc: \"" + sheet + "\"") },
                () => { K(Qt.Key_Tab); say(tab === 1, "Tab: " + (tabs[tab] || "")) },
                () => { if (getItems.length < gridColumns + 2 || turnTo >= 0) return false; zone = "main"; getArea = "grid"; getIndex = 0
                        K(Qt.Key_Right); m.i1 = getIndex; K(Qt.Key_Down); m.i2 = getIndex; K(Qt.Key_Return) },
                () => { say(m.i1 === 1 && m.i2 === 1 + gridColumns && modPage, "Get mods: Right to card " + (m.i1 + 1) + ", Down to card " + (m.i2 + 1) + ", Enter opens " + (modPage ? "the mod's page" : "nothing"))
                        K(Qt.Key_Escape); say(!modPage && tab === 1, "Esc from the mod's page: " + (modPage ? "still there" : "back on " + (tabs[tab] || ""))) },
                () => { K(Qt.Key_PageDown); m.t1 = tab; K(Qt.Key_PageUp); m.t2 = tab; K(Qt.Key_Backtab, Qt.ShiftModifier)
                        say(m.t1 === 2 && m.t2 === 1 && tab === 0, "Page Down: tab " + m.t1 + ", Page Up: tab " + m.t2 + ", Shift+Tab: tab " + tab) },
                () => { K(Qt.Key_F1); m.p = page + " " + sec; K(Qt.Key_Escape); say(m.p === "settings Help" && page === "game", "F1: " + m.p + ", Esc: " + page) },
                () => { m.z = sizeNow; K(Qt.Key_Equal, Qt.ControlModifier); m.z1 = sizeNow; K(Qt.Key_Minus, Qt.ControlModifier)
                        say(m.z1 === m.z + 10 && sizeNow === m.z, "Ctrl+=: " + m.z + "% to " + m.z1 + "%, Ctrl+-: back to " + sizeNow + "%") },
                () => { K(Qt.Key_P); m.p = page; K(Qt.Key_P); m.p2 = page; K(Qt.Key_V); m.w = wheelOpen; K(Qt.Key_Right); m.ws = wheelSlot; K(Qt.Key_Escape)
                        say(m.p === "settings" && m.p2 === "game" && m.w && m.ws === 2 && !wheelOpen, "P: " + m.p + ", P again: " + m.p2 + "; V: the wheel " + (m.w ? "open" : "closed") + ", Right picks slot " + m.ws + ", Esc: " + (wheelOpen ? "still open" : "closed")) },
                () => { K(Qt.Key_Return, Qt.ControlModifier); m.s1 = sheet + " " + pickTitle; K(Qt.Key_Escape); say(m.s1 === "pick Play options" && sheet === "", "Ctrl+Enter: " + m.s1 + ", Esc: sheet \"" + sheet + "\"") },
                // 0.63: the keys a PC player expects.
                () => { zone = "main"; tab = 0; modIndex = 0; K(Qt.Key_S); m.r1 = modIndex; K(Qt.Key_W); m.r2 = modIndex; K(Qt.Key_R); m.s1 = sheet; K(Qt.Key_Escape); K(Qt.Key_F); m.s2 = sheet; K(Qt.Key_Escape)
                        say(m.r1 === 1 && m.r2 === 0 && m.s1 === "modmenu" && m.s2 === "findpop" && sheet === "", "W A S D, R and F: S to row " + (m.r1 + 1) + ", W to row " + (m.r2 + 1) + ", R: sheet \"" + m.s1 + "\", F: sheet \"" + m.s2 + "\"") },
                () => { K(Qt.Key_E); m.t1 = tab; K(Qt.Key_BracketRight); m.t2 = tab; K(Qt.Key_BracketLeft); K(Qt.Key_Q)
                        say(m.t1 === 1 && m.t2 === 2 && tab === 0, "Q and E turn the pages: E: tab " + m.t1 + ", ]: tab " + m.t2 + ", [ and Q: tab " + tab) },
                () => { if (turnTo >= 0) return false; zone = "main"; K(Qt.Key_Escape); m.p = page; K(Qt.Key_Escape); m.p2 = page; K(Qt.Key_Backspace)
                        say(m.p === "settings" && m.p2 === "game" && page === "game", "Esc on the first page: " + m.p + ", Esc again: " + m.p2 + "; Backspace stays B: " + page)
                        zone = "main" },
                // A field being typed in has the letters itself.
                () => { K(Qt.Key_F); if (sheet !== "findpop") { say(false, "F didn't open Find"); return }
                        findPopField(); m.t = tab
                        K(Qt.Key_Q); K(Qt.Key_E); K(Qt.Key_R); K(Qt.Key_W); K(Qt.Key_F); K(Qt.Key_Space)
                        say(sheet === "findpop" && tab === m.t && !wheelOpen && !spaceDown, "typing in Find: Q, E, R, W, F and Space are the field's: sheet \"" + sheet + "\", tab " + tab)
                        findPopEndTyping(); K(Qt.Key_Escape) },
                // Space: a tap is A, a 3 second hold is Start's hold (the Play choice).
                () => { if (sheet !== "") { say(false, "Find stayed open: sheet \"" + sheet + "\""); sheet = "" } zone = "main"; tab = 0; modIndex = 0; m.on = mods[0].enabled; K(Qt.Key_Space) },
                () => { say(mods[0].enabled !== m.on, "a tap of Space is A: the mod turned " + (mods[0].enabled ? "on" : "off")); K(Qt.Key_Space) },
                () => { m.on = mods[0].enabled; m.t = Date.now(); Turbine.selftestKey(Qt.Key_Space, 0, true) },
                () => { if (sheet !== "pick" && Date.now() - m.t < 6000) return false
                        m.s1 = sheet + " " + pickTitle; m.ms = Date.now() - m.t; Turbine.selftestKey(Qt.Key_Space, 0, false); m.s2 = sheet; K(Qt.Key_Escape)
                        say(m.s1 === "pick Play options" && m.ms >= 2900 && m.s2 === "pick" && mods[0].enabled === m.on && sheet === "", "Space held: " + m.s1 + " after " + (m.ms >= 2900 ? "3 seconds" : (m.ms / 1000).toFixed(1) + " s") + ", the mod left as it was, Esc: sheet \"" + sheet + "\"") },
                // Tab held: the quick wheel; W A S D pick.
                () => { m.t = Date.now(); m.tb = tab; Turbine.selftestKey(Qt.Key_Tab, 0, true) },
                () => { if (!wheelOpen && Date.now() - m.t < 3000) return false
                        m.w = wheelOpen; K(Qt.Key_D); m.ws = wheelSlot; K(Qt.Key_Escape); m.w2 = wheelOpen; Turbine.selftestKey(Qt.Key_Tab, 0, false)
                        say(m.w && m.ws === 2 && !m.w2 && !wheelOpen && tab === m.tb, "Tab held: the wheel " + (m.w ? "open" : "closed") + ", D picks slot " + m.ws + ", Esc: " + (m.w2 || wheelOpen ? "still open" : "closed") + ", the page " + (tab === m.tb ? "stayed" : "turned")) },
                // A toast's step (Undo) is Ctrl+Z.
                () => { zone = "main"; tab = 0; m.wasOn = mods.filter(x => x.enabled).length; allOff(); m.g = toast.actionGlyph },
                () => { K(Qt.Key_Z, Qt.ControlModifier); const on = mods.filter(x => x.enabled).length
                        say(m.g === "Ctrl+Z" && on === m.wasOn && m.wasOn > 0, "the toast says " + m.g + " Undo; Ctrl+Z: " + on + " of " + m.wasOn + " back on") },
                // The mouse: hover, click, right-click, Back, Ctrl with the wheel.
                // A real mouse comes to the row from somewhere else (it may already rest on that spot from an earlier check).
                () => { const it = modList.itemAtIndex(1); if (!it) return false
                        m.hv = hoverSeen; const p = at(it, 0.4); Turbine.selftestMouse(0, p.x + 40, p.y + 6, 0, 0, 0) },
                () => { const p = at(modList.itemAtIndex(1), 0.4); Turbine.selftestMouse(0, p.x, p.y, 0, 0, 0) },
                () => { if (lastInput !== "mouse") note("check", "Key check note: hovers seen " + (hoverSeen - m.hv) + ", the last " + hoverSeenAfterKey + " ms after a key, pointer shown " + Turbine.pointerShown)
                        say(lastInput === "mouse" && !!hoverItem && hints() === "Click, Esc, Right-click, F, Q, E", "the mouse over a row: last input " + lastInput + ", " + (hoverItem ? "the row is outlined" : "nothing outlined") + ", hints " + hints())
                        m.on = mods[1].enabled; mouse(at(modList.itemAtIndex(1), 0.4), Qt.LeftButton) },
                () => { say(mods[1].enabled !== m.on && modIndex === 1, "a click on a row is A: row " + (modIndex + 1) + " turned " + (mods[1].enabled ? "on" : "off"))
                        mouse(at(modList.itemAtIndex(0), 0.4), Qt.RightButton) },
                () => { m.s1 = sheet; m.row = modIndex; mouse({ x: width / 2, y: height / 3 }, Qt.BackButton)
                        say(m.s1 === "modmenu" && m.row === 0 && sheet === "", "a right-click on row 1: sheet \"" + m.s1 + "\" for row " + (m.row + 1) + "; the Back button: sheet \"" + sheet + "\"")
                        tab = 1 },
                () => { if (getItems.length < 2 || turnTo >= 0) return false
                        const cell = getView.gridItem(1)
                        if (!cell) { say(false, "no card to right-click"); return }
                        mouse(at(cell), Qt.RightButton) },
                () => { m.s1 = sheet; m.gi = getIndex; mouse({ x: width / 2, y: height / 3 }, Qt.BackButton); m.s2 = sheet; mouse({ x: width / 2, y: height / 3 }, Qt.BackButton)
                        say(m.s1 === "cardmenu" && m.gi === 1 && m.s2 === "" && tab === 0, "a right-click on card 2: sheet \"" + m.s1 + "\"; Back: sheet \"" + m.s2 + "\"; Back again: " + (tabs[tab] || "").replace(/ \(\d+\)$/, "")) },
                () => { m.z = sizeNow; Turbine.selftestMouse(3, width / 2, height / 3, 0, 0, 1); m.z0 = sizeNow
                        Turbine.selftestMouse(3, width / 2, height / 3, 0, Qt.ControlModifier, -1); m.z1 = sizeNow; Turbine.selftestMouse(3, width / 2, height / 3, 0, Qt.ControlModifier, 1)
                        say(m.z0 === m.z && m.z1 === m.z + 10 && sizeNow === m.z, "the wheel alone: " + m.z0 + "%; Ctrl and the wheel up: " + m.z1 + "%, down: " + sizeNow + "%") },
                // 0.63: after the mouse no key caps; Forward is the next page; a double-click opens a mod.
                () => { const it = modList.itemAtIndex(1); if (!it) return false
                        const p = at(it, 0.4); Turbine.selftestMouse(0, p.x, p.y, 0, 0, 0) },
                () => { legend.check("keycheck mouse")
                        say(mouseLegend && barLb.width === 0 && barRb.width === 0 && startHintKey.width === 0, "after the mouse: " + (barLb.width === 0 && barRb.width === 0 && startHintKey.width === 0 ? "no key caps" : "key caps") + ", the pages by name: " + tabs.map(t => t.replace(/ \(\d+\)$/, "")).join(" / "))
                        m.tb = tab; mouse({ x: width / 2, y: height / 3 }, Qt.ForwardButton); m.t1 = tab; mouse({ x: width / 2, y: height / 3 }, Qt.BackButton)
                        say(m.t1 === m.tb + 1 && tab === m.tb, "the Forward button: tab " + m.t1 + "; Back: tab " + tab) },
                // A mod whose click asks nothing (one that needs a missing loader asks for it on the first click).
                () => { if (turnTo >= 0) return false
                        m.di = mods.findIndex(x => !x.needsLoader); const it = m.di >= 0 ? modList.itemAtIndex(m.di) : null; if (!it) return false
                        m.on = mods[m.di].enabled; m.name = mods[m.di].name; const p = at(it, 0.4); mouse(p, Qt.LeftButton); mouse(p, Qt.LeftButton) },
                () => { m.s1 = sheet; m.ok = (sheet === "pick" && pickTitle === m.name) || sheet === "modmenu"
                        say(m.ok && modIndex === m.di && mods[m.di].enabled === m.on, "a double-click on a mod: " + (m.ok ? "the mod opened" : "sheet \"" + sheet + "\"") + ", left " + (mods[m.di].enabled === m.on ? "as it was" : "changed"))
                        mouse({ x: width / 2, y: height / 3 }, Qt.BackButton) },
                () => { if (sheet !== "") mouse({ x: width / 2, y: height / 3 }, Qt.BackButton) },
                () => { say(sheet === "", "Back from the opened mod: sheet \"" + sheet + "\"") },
                // The hints follow the last input: a key, then the controller.
                () => { K(Qt.Key_Down); m.h1 = hints() },
                () => { legend.check("keycheck"); pad("up"); m.h2 = hints()
                        say(m.h1 === "Enter, Esc, R, F, Q, E" && m.h2 === "A, B, X, Y, LB, RB", "hints after a key: " + m.h1 + "; after a controller press: " + m.h2)
                        keysTest = false
                        // 0.62: Desktop Mode with no controller: one line in the log, nothing on screen.
                        m.toastBefore = lastToastAt; m.noted = noPadNote(); m.again = noPadNote()
                        check("No controller in Desktop Mode: " + (Turbine.pads ? "a controller is seen" : Turbine.gamemode ? "Game Mode" : m.noted && !m.again ? "logged once" : "not logged") + ", " + (lastToastAt === m.toastBefore ? "no toast" : "a toast showed"))
                        // 0.62: one sign-in status: Settings' row and the Nexus page's hints read the same thing.
                        signinRefresh()
                        say((accountRows[0].title === "Nexus Mods: Signed in") === Turbine.browserSignedIn() && browserSigninShown === (bridge.paired === true && !Turbine.browserSignedIn()),
                            "sign-in status: Settings says " + accountRows[0].title + ", Turbine's browser is " + (Turbine.browserSignedIn() ? "signed in" : "not signed in"))
                        // 0.62: Get Seamless Co-op goes to Nexus first when Turbine's Nexus pages are signed in, and to
                        // Turbine's own download otherwise (nothing is opened or downloaded here).
                        m.p1 = seamlessGetPlan(); m.was = browserSignedIn; browserSignedIn = true; browserLiveForced = true; m.p2 = seamlessGetPlan()
                        m.url = Turbine.browserPageMod(seamlessNexusUrl); browserSignedIn = m.was; browserLiveForced = false
                        check("Seamless Co-op plan: not signed in on Nexus: " + m.p1 + "; signed in: " + m.p2 + " (mod " + m.url + " of " + seamlessNexusUrl.split("/")[3] + ")")
                        check("Key check done: " + m.bad + " problems") }
            ]
        }
        // 0.62: the Co-op tile's rows follow the engine's answer for where Turbine runs: in Desktop Mode one press
        // (at once with Steam closed, or one restart, asked first), in Game Mode words only with OK.
        // 0.64: requirements lined up and the Dependencies group, by real presses. The build has no internet, so the
        // requirement is given to the first mod here (as Nexus's list would after an add); the queue's first step
        // is read from the log instead of a page opening.
        else if (name === "reqcheck") {
            m.bad = 0
            const say = (ok, text) => { if (!ok) m.bad++; check("Requirements check: " + text + (ok ? "" : "  PROBLEM")) }
            const give = () => { const l = Object.assign({}, linksMap); l[mods[0].id] = { needs: [{ name: "Test Library", id: null, domain: rules.domain, modId: 4242 }], neededBy: [] }
                                 l[mods[1].id] = { needs: [], neededBy: [mods[0].id] }; l[mods[0].id].needs.push({ name: mods[1].name, id: mods[1].id, domain: "", modId: 0 }); linksMap = l }
            const rowKinds = () => listItems.map(x => x.kind).join(",")
            script = [
                () => { if (mods.length < 2 || !modsLoaded) return false; sheet = ""; page = "game"; zone = "main"; tab = 0; modIndex = 0
                        if (!mods[0].enabled) toggle(mods[0], true)
                        if (mods[1].enabled) toggle(mods[1], true)
                        reqSave(null); reqRunning = false },
                // An add of a mod that needs one mod that isn't installed and one that is but is off.
                () => { give(); reqArrived(mods[0].id, mods[0].name); m.t = Date.now() },
                () => { if (sheet !== "msg" && Date.now() - m.t < 4000) return false
                        say(sheet === "msg" && !!msgData && msgData.title === "This mod needs 2 others" && msgData.buttons.map(b => b.label).join("; ") === "Get them all; Later",
                            "after the add: " + (sheet === "msg" && msgData ? msgData.title + " | " + msgData.buttons.map(b => b.label).join("; ") + " | " + (msgData.notes || []).join("; ") : "sheet \"" + sheet + "\""))
                        pad("a") },
                () => { say(reqRunning && reqStripWords === "Getting 1 of 1: Test Library" && mods[1].enabled, "Get them all: " + (reqStripWords || "nothing running") + ", the installed one turned " + (mods[1].enabled ? "on" : "off"))
                        reqStop() },
                () => { say(!reqRunning && listItems[0] && listItems[0].kind === "needsq", "Stop: " + (reqRunning ? "still running" : "stopped") + "; My mods rows: " + rowKinds())
                        modIndex = 0; zone = "main"; pad("a") },
                () => { say(reqRunning && reqStripWords === "Getting 1 of 1: Test Library", "A on the row goes on: " + (reqStripWords || "nothing running"))
                        reqSkip() },
                () => { if (reqRunning) return false },
                () => { say(!reqRunning && !reqLeft, "Skip on the last one: " + (reqRunning ? "still running" : "done") + ", rows: " + rowKinds()) },
                // Before you play names what is missing, with Get what they need first.
                () => { give(); m.c = playAsks; startPlay() },
                () => { if (sheet !== "check") return false
                        const row = checks.find(x => x.title === reqCheckTitle)
                        say(!!row && checkChoices[0] === "Get what they need", "Before you play: " + (row ? row.title + ": " + row.detail : "no row") + " | " + checkChoices.join("; "))
                        pad("b") },
                // Dependencies: the mod another one needs goes into the group at the bottom, A opens it, X moves it out.
                () => { if (sheet !== "") return false; give(); depsOpen = false; zone = "main"
                        m.k1 = rowKinds(); modIndex = listItems.findIndex(x => x.kind === "deps"); pad("a") },
                () => { const i = listItems.findIndex(x => x.kind === "mod" && x.dep)
                        say(m.k1 === "mod,deps" && depsOpen && i >= 0, "Dependencies: rows " + m.k1 + "; A opens it: " + rowKinds() + (i >= 0 ? " (" + listItems[i].m.name + ")" : ""))
                        if (i >= 0) { modIndex = i; pad("x") } },
                () => { if (sheet !== "modmenu") return false
                        const more = modMenuItems.find(x => x.act === "more"), rows = more ? more.rest : []
                        const out = rows.find(x => x.act === "depout")
                        say(!!out, "X options of a dependency: " + (out ? out.title : "no Move out row") + "; order before: " + mods.map(x => x.name).join(", "))
                        m.order = mods.map(x => x.id).join(",")
                        if (out) modMenuDo(out, focusedItem.m); sheet = "" },
                () => { give()
                        say(rowKinds() === "mod,mod" && mods.map(x => x.id).join(",") === m.order, "moved out: rows " + rowKinds() + ", the load order " + (mods.map(x => x.id).join(",") === m.order ? "unchanged" : "CHANGED"))
                        const i = listItems.findIndex(x => x.kind === "mod" && x.m.id === mods[1].id); modIndex = i; m.warnBefore = sheet; pad("a") },
                () => { say(sheet === "pick" && /need/.test(pickTitle), "turning off a mod another needs warns: " + (sheet === "pick" ? pickTitle : "sheet \"" + sheet + "\""))
                        pad("b"); seen.depMoves = "{}" },
                () => { check("Requirements check done: " + m.bad + " problems") }
            ]
        }
        else if (name === "tilecheck") script = [
            () => { if (!seamlessIn || !modsLoaded) return false; sheet = ""; page = "game"; zone = "main"; tab = 0
                    const o = coopTileOffer
                    check("Tile check: Game Mode " + Turbine.gamemode + ", how " + o.how + ", label \"" + o.label + "\", words: " + o.words)
                    const row = gameMenuItems.find(x => x.act === "cooptile")
                    check("Tile check: game menu row: " + (row ? row.title + " | " + row.detail : "none"))
                    m.asked = maybeAskCoopTile()
                    m.s1 = sheet === "msg" && msgData ? msgData.title + " | " + msgData.buttons.map(b => b.label).join("; ") : "sheet \"" + sheet + "\""
                    if (sheet !== "") pad("b") },
            () => { check("Tile check: asked once after the install: " + (m.asked ? "yes, " + m.s1 : "no (" + coopTileOffer.how + ")"))
                    sheet = ""; openCoopTile() },
            () => { check("Tile check: the row opens: " + (sheet === "msg" && msgData ? msgData.title + " | " + msgData.text.substring(0, 90) + " | " + msgData.buttons.map(b => b.label).join("; ") : "sheet \"" + sheet + "\""))
                    pad("b") },
            () => { check("Tile check done: sheet \"" + sheet + "\"") }
        ]
        // 0.62: an older Seamless Co-op file is asked about, never added by itself and never a failed mod: A keeps the
        // one installed, B too, X uses the older file.
        else if (name === "oldercheck") {
            const file = shot.arg("older-file")
            const ver = () => seamlessMod ? versionOf(seamlessMod) : "none"
            const ask = () => sheet === "msg" && msgData ? msgData.title + " | " + msgData.text + " | " + msgData.buttons.map(b => b.label).join("; ") + " | highlighted " + (infoButtons[infoIndex] ? infoButtons[infoIndex].label : "") : "sheet \"" + sheet + "\""
            const pwRow = () => settingsView.shown.find(r => r.f && /password/i.test(r.f.key || ""))
            const pwState = () => seamlessMod ? ["unknown", "empty", "set"][Turbine.coopPasswordState(seamlessMod.id) + 1] : "no Seamless Co-op"
            script = [
                () => { if (!seamlessIn || !modsLoaded) return false; sheet = ""; page = "game"; zone = "main"; tab = 0; m.n = mods.length; m.newer = seamlessMod.id
                        check("Older check: installed Seamless Co-op " + ver() + ", " + mods.length + " mods; Get Seamless Co-op would: " + seamlessGetPlan())
                        openCoopSettings() },
                // A session password on the copy installed, typed on its settings page and saved by itself.
                () => { if (sheet !== "settingsedit" || !pwRow()) return false; coopTypePassword() },
                () => { if (sheet !== "ask") return false; askView.open("Session password", "", "dragon", "", 2000, false, "Done"); askView.accept() },
                () => { if (sheet !== "settingsedit" || settingsView.changes > 0 || Turbine.coopPasswordState(seamlessMod.id) !== 1) return false; pad("b") },
                () => { if (sheet !== "") return false
                        check("Older check: the password on " + ver() + " is " + pwState()); m.asks = olderAsks; Turbine.selftestImport(file, "loader", "seamless", "1.9.0") },
                () => { if (olderAsks === m.asks) return false
                        check("Older check: asked: " + ask() + " | error said: " + (msgLevel === "error" ? msgText : "none")); pad("a") },
                () => { if (Turbine.busy) return false
                        check("Older check: A keeps it: sheet \"" + sheet + "\", Seamless Co-op " + ver() + ", " + mods.length + " mods, failed: " + (msgLevel === "error" ? msgText : "none"))
                        m.asks = olderAsks; Turbine.selftestImport(file, "loader", "seamless", "1.9.0") },
                () => { if (olderAsks === m.asks) return false; pad("b") },
                () => { if (Turbine.busy) return false
                        check("Older check: B goes back and keeps it: sheet \"" + sheet + "\", Seamless Co-op " + ver() + ", " + mods.length + " mods")
                        m.asks = olderAsks; Turbine.selftestImport(file, "loader", "seamless", "1.9.0") },
                // Use is the second button: Right to it, then A (a message's buttons are picked with the D-pad).
                () => { if (olderAsks === m.asks || sheet !== "msg") return false
                        pad("right"); m.on = infoButtons[infoIndex] ? infoButtons[infoIndex].label : ""; pad("a") },
                // The older file is a new mod in the list, on; the newer copy is in Removed mods.
                () => { if (Turbine.busy || ver() !== "1.9.0") return false
                        const used = mods.filter(x => x.seamless === true)
                        check("Older check: Right then A on " + m.on + ": in the list " + used.map(x => "Seamless Co-op " + versionOf(x) + (x.enabled ? " on" : " off")).join(", ") + " (" + mods.length + " mods), Removed mods " + removedMods.map(x => x.id === m.newer ? "the 2.0.1 copy" : x.name).join("; ")
                              + ", failed: " + (msgLevel === "error" ? msgText : "none") + ", sheet \"" + sheet + "\"")
                        check("Older check: the password on " + ver() + " is " + pwState())
                        if (sheet !== "") sheet = ""
                        openCoopSettings() },
                () => { if (sheet !== "settingsedit" || !pwRow()) return false
                        check("Older check: the password carried over: " + (String(settingsView.valueOf(pwRow().f)) === "dragon" ? "yes" : "no, it reads \"" + settingsView.valueOf(pwRow().f) + "\"")); pad("b") },
                () => { if (sheet !== "") return false; openRemoved() },
                () => { if (sheet !== "removed") return false
                        const r = removedMods[removedIndex], o = binds()
                        m.back = r ? r.id : ""
                        check("Older check: Removed mods offers: " + (r ? (r.id === m.newer ? "the 2.0.1 copy" : r.name) : "nothing") + ", A " + (o.a ? o.a.label : "") + ", used twice: " + (olderAsks === m.asks + 1 ? "no" : "asked again"))
                        pad("a") },
                // What Put back then does (two copies are the two copies check's own subject): said, not required.
                () => { if (Turbine.busy) return false
                        check("Older check: after Put back: in the list " + mods.filter(x => x.seamless === true || x.id === m.back).map(x => x.name + " " + versionOf(x) + (x.enabled ? " on" : " off")).join(", ") + "; Removed mods " + removedMods.length + ", sheet \"" + sheet + "\"") },
                () => { check("Older check done: asked " + olderAsks + " times") }
            ]
        }
        // 0.62: "Proton for modded play": the row in the game menu (This game), its list (Automatic first, an
        // unusable Proton dimmed with the reason), a pick kept and Automatic again, and the same from the tool's
        // steps sheet (its button, and B comes back to the steps).
        else if (name === "protoncheck") {
            const rowsNow = () => pickItems.map(r => r.title + (r.dim ? " [dim: " + r.detail + "]" : r.detail ? " [" + r.detail + "]" : "")).join("; ")
            const openList = () => { sheet = "gamemenu"; menuIndex = gameMenuItems.findIndex(x => x.act === "proton"); pad("a") }
            script = [
                () => { if (!game || !modsLoaded || !rules) return false; sheet = ""; page = "game"; zone = "main"; tab = 0
                        const row = gameMenuItems.find(x => x.act === "proton")
                        check("Proton check: game menu row: " + (row ? row.title + " | section " + row.group : "none") + " | " + (protonNow ? protonNow.list.length + " installed, how " + protonNow.how : "no Proton found"))
                        openList() },
                () => { if (sheet !== "pick") return false
                        check("Proton check: the list: " + pickTitle + " | " + rowsNow() + " | focus " + (pickItems[menuIndex] ? pickItems[menuIndex].title : ""))
                        m.picks = protonPicks; menuIndex = pickItems.findIndex(r => r.dim === true); m.dimRow = pickItems[menuIndex] ? pickItems[menuIndex].title : "none"; pad("a") },
                () => { check("Proton check: A on the dimmed one (" + m.dimRow + "): picked " + (protonPicks - m.picks) + " times, still " + protonTitle(protonNow))
                        if (sheet !== "pick") openList() },
                () => { if (sheet !== "pick") return false; menuIndex = pickItems.findIndex((r, i) => i > 0 && r.dim !== true); m.want = pickItems[menuIndex] ? pickItems[menuIndex].title : ""; pad("a") },
                () => { check("Proton check: picked " + m.want + ": " + protonTitle(protonNow) + ", how " + (protonNow ? protonNow.how : "?") + ", sheet \"" + sheet + "\", toast: " + lastToast)
                        openList() },
                () => { if (sheet !== "pick") return false; m.focus = pickItems[menuIndex] ? pickItems[menuIndex].title : ""; menuIndex = 0; pad("a") },
                () => { check("Proton check: the list opened on " + m.focus + "; Automatic again: " + protonTitle(protonNow) + ", how " + (protonNow ? protonNow.how : "?"))
                        sheet = ""; openLaunchSetup({ game: game.id, name: gameTitle, option: "x %command%", set: "", listed: true, coop: false }) },
                () => { if (sheet !== "msg") return false
                        check("Proton check: the tool's steps sheet: " + (msgData ? msgData.buttons.map(b => b.label).join("; ") + " | " + msgData.footnote : ""))
                        infoIndex = msgData ? msgData.buttons.findIndex(b => b.proton === true) : 0; pad("a") },
                () => { m.s1 = sheet + " " + pickTitle; pad("b"); check("Proton check: its button on the steps: " + m.s1 + "; B: sheet " + sheet + (sheet === "msg" && msgData ? " (" + msgData.title + ")" : "")) },
                () => { sheet = ""; check("Proton check done") }
            ]
        }
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

    // ---------------------------------------------------------------- the build's checks of Nexus inside Turbine
    // A stand-in Nexus page on 127.0.0.1 (tools/nexus-stand-in.py) with Slow download and Mod Manager Download at fixed
    // places: each step waits for its condition (up to its max), then acts. Presses go through Turbine.selftestPad,
    // the same path a controller's take; the pointer is placed on the page's buttons by their place in the page.
    property var btSteps: []
    property int btStep: 0
    property real btSince: 0
    property var bt: ({})
    Timer { id: btTimer; interval: 250; repeat: true; onTriggered: app.btTick() }
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
        if (!ready) check("Browser check: step " + (btStep + 1) + " (" + st.name + ") gave up waiting: page " + page + ", url " + (nexusView ? nexusView.url : "none") + ", sheet \"" + sheet + "\"")
        try { if (st.act) st.act() } catch (e) { check("Browser check: step " + (btStep + 1) + " (" + st.name + ") stopped: " + e) }
        btStep++; btSince = Date.now()
    }
    // A picture of this moment beside the run's own (shots/<page>-<size>-<tag>.png).
    function btShot(tag) {
        const p = shot.path.replace(/\.png$/, "-" + tag + ".png")
        app.contentItem.grabToImage(function (r) { r.saveToFile(p); app.check("Browser check: picture " + tag) })
    }
    // The pointer onto a place in the page (CSS pixels), then RT pressed and let go: one click.
    function btClick(cx, cy) {
        const p = nexusView.pagePoint(cx, cy)
        Turbine.pointerShowAt(p.x, p.y)
        Turbine.selftestPad("rt", true); Turbine.selftestPad("rt", false)
    }
    function btMark(name) {
        const m = /^(http:\/\/127\.0\.0\.1:[0-9]+)/.exec(browserTest)
        if (!m) return
        const x = new XMLHttpRequest()
        x.open("GET", m[1] + "/mark?name=" + encodeURIComponent(name))
        x.send()
        check("Browser check: mark " + name)
    }
    function btLegend(what) {
        const o = binds(), h = legendFor(o)
        let bad = []
        if (h.length > 5) bad.push("more than 5")
        const keys = h.map(x => x[0])
        if (keys.some((k, i) => keys.indexOf(k) !== i)) bad.push("a key twice")
        for (const k of ["a", "b", "x", "y"]) {
            if (o[k] && o[k].label && keys.indexOf(k) < 0) bad.push(k.toUpperCase() + " missing")
            if (keys.indexOf(k) >= 0 && !(o[k] && o[k].run)) bad.push(k.toUpperCase() + " does nothing")
        }
        if (JSON.stringify(nexusView ? legendHints : []) !== JSON.stringify(h)) bad.push("the legend on screen differs")
        check("Browser legend (" + what + "): " + h.map(x => x[0].toUpperCase() + " " + x[1]).join(" · ") + ", " + bad.length + " problems" + (bad.length ? ": " + bad.join(", ") : ""))
    }
    readonly property string btFiles: "https://www.nexusmods.com/skyrimspecialedition/mods/1?tab=files&file_id=2"
    function browserCheckScript() {
        const big = width >= 1900
        const pageIs = (re) => !!nexusView && !!nexusView.view && !nexusView.loading && re.test(nexusView.url)
        btRun([
            { name: "Get mods", max: 20000, wait: () => !!game && modsLoaded && getItems.length > 0,
              act: () => { Turbine.selftestCatch("watch")
                           // The sign-in pass-through: a helper's answer goes into the profile (checked on disk later).
                           Turbine.selftestCatch("signin"); browserNoteNow = null; toast.hide()
                           openDetails(getItems[0]) } },
            { name: "mod page", max: 15000, wait: () => !!detailsData && !detailsData.loading && mainLabel === "Get it on Nexus",
              act: () => { check("Browser check: mod page " + detailsData.name + ", button " + mainLabel); openNexus(btFiles) } },
            { name: "file page", max: 45000, wait: () => pageIs(/tab=files/),
              act: () => { check("Browser check: the file page loaded in Turbine: " + nexusView.title + " (" + nexusView.url + ")"); btShot("page"); if (!big) btMark("idle-start") } },
            { name: "idle", after: big ? 500 : 30000,
              act: () => { if (!big) btMark("idle-end"); btLegend("no field") } },
            { name: "field", act: () => btClick(190, 500) },
            { name: "field focus", max: 6000, wait: () => nexusView.focusKind === 2,
              act: () => { btLegend("a field"); check("Browser check: Steam's keyboard opens for a field: " + (browserFieldSeen > 0 ? "yes" : "no"))
                           // A clicked a moment ago: still a click at the pointer, never Enter on what has the focus.
                           check("Browser check: A after a click is a click: " + (Turbine.browserKeyNav ? "no" : "yes")); pad("down") } },
            { name: "d-pad", after: 300, act: () => { check("Browser check: A after the D-pad presses Enter: " + (Turbine.browserKeyNav ? "yes" : "no")); btClick(110, 432) } },
            { name: "on the second page", max: 15000, wait: () => pageIs(/tab=description/),
              act: () => { check("Browser check: A after the pointer clicked is a click again: " + (Turbine.browserKeyNav ? "no" : "yes")); pad("b") } },
            { name: "back", max: 15000, wait: () => pageIs(/tab=files/),
              act: () => { check("Browser check: B from a second page went back one page (" + nexusView.url + ")"); btClick(150, 312) } },
            { name: "refused", max: 10000, wait: () => !!browserNoteNow && browserNoteNow.text.indexOf("zip, 7z or rar") >= 0,
              act: () => { check("Browser check: refused: " + browserNoteNow.text); browserNoteNow = null; btClick(150, 372) } },
            { name: "downloading", max: 15000, wait: () => webDls.some(x => x.name.indexOf("Dupe Test") === 0 && x.state === "progress"),
              act: () => { const x = webDls.find(d => d.name.indexOf("Dupe Test") === 0); check("Browser check: second download of a name there already is " + x.name); pad("rb") } },
            // A picture is taken at the next frame: what changes the screen waits for the step after it.
            { name: "downloads tab", after: 400, act: () => btShot("downloads") },
            { name: "page tab", after: 500, act: () => pad("lb") },
            { name: "quick menu", after: 400, act: () => { openBrowserMenu(); check("Browser check: the quick menu has " + pickItems.length + " rows (at most 6): " + pickItems.map(x => x.title).join(", "))
                                                         check("Browser check: touch targets " + nexusView.tapSizes().map(x => x[0] + " " + x[1]).join(", ")) } },
            { name: "quick menu closed", after: 400, act: () => pad("b") },
            { name: "added", max: 25000, wait: () => webDls.some(x => x.name.indexOf("Dupe Test") === 0 && x.state === "added"),
              act: () => { check("Browser check: pill " + browserPill.text); btShot("added") } },
            { name: "mod manager download", after: 500, act: () => { bt.nxmAt = Date.now(); btClick(460, 188)
                check("Browser check: the Added note cleared at the next click: " + (!browserNoteNow || browserNoteNow.text.indexOf("Added") < 0 ? "yes" : "no")) } },
            // Every Mod Manager Download link from the page asks: on Not now, deaf for a second, then Right and A.
            { name: "mmd asks", max: 15000, wait: () => nxmAsking,
              // What it offered the moment it opened (nxmAskFirst), and an A made 300 ms after it showed (judged by when it
              // was made, so a slow runner can't turn it into a late press).
              act: () => { const f = nxmAskFirst || {}
                           check("Browser check: Mod Manager Download asks: " + pickTitle + ", focus " + (f.ring === 1 ? "Not now" : "Get it") + ", A " + f.a + ", B " + f.b + ", page input " + (nexusView.view.enabled ? "on" : "off"))
                           Turbine.selftestPadAged("a", nxmShownAt > 0 ? Math.max(0, Math.round(Turbine.monoMs() - nxmShownAt - 300)) : 0)
                           check("Browser check: A at once on the question: " + (nxmAsking ? "ignored" : "taken")) } },
            // 0.56 (L1, L2): two taps on Get it don't get it, and a page alert or confirm doesn't replace the question.
            { name: "mmd taps", after: 1200, act: () => {
                pickMenu.activated(0); pickMenu.activated(0)
                check("Browser check: two taps on Get it: " + (nxmAsking ? "still asking" : "got it"))
                browserDialog({ type: 0, message: "Tap here twice", dialogAccept: () => {}, dialogReject: () => {} })
                browserDialog({ type: 1, message: "OK?", dialogAccept: () => {}, dialogReject: () => {} })
                check("Browser check: a page alert and confirm while asking: " + (nxmAsking && pickTitle.indexOf("Get ") === 0 ? "the question stays" : "replaced it")) } },
            { name: "mmd get", after: 300, act: () => { pad("right"); pad("a"); check("Browser check: Right then A after a second gets it: " + (!nxmAsking ? "yes" : "no")) } },
            { name: "nxm", after: 2500, act: () => {
                check("Browser check: after Mod Manager Download the page is still the file page: " + (pageIs(/tab=files/) ? "yes" : "no, " + (nexusView ? nexusView.url : "")))
                browserNoKey("", "nxm://skyrimspecialedition/mods/1/files/2?key=abc&expires=4102444800&user_id=1") } },
            { name: "no key", after: 600, act: () => { btShot("nokey"); btLegend("no key message")
                const o = binds(); check("Browser check: the no-key note on its own file page leaves A " + o.a.label + " and B " + o.b.label)
                browserNoteNow = null; browserNoKey("", "nxm://skyrimspecialedition/mods/1/files/9?key=abc&expires=4102444800&user_id=1") } },
            { name: "no key elsewhere", after: 400, act: () => { const o = binds(); check("Browser check: the no-key note for another file's page: A " + o.a.label + ", B " + o.b.label); pad("b") } },
            { name: "close", after: 400, act: () => { check("Browser check: B on " + (nexusView && nexusView.canGoBack ? "a later page" : "the first page")); pad("b") } },
            { name: "closed", max: 5000, wait: () => page !== "browser",
              act: () => { check("Browser check: B on the first page closed it; focus " + (modPage && zone === "main" && pageIndex === 0 ? "on Get it" : "on " + zone + (modPage ? " (mod page row " + pageIndex + ")" : ""))) } },
            // This check's mod needs nothing else (the sample mod asks for SKSE, which keeps the focus on its page
            // and offers the next requirement: that way is checked by the chain pictures).
            { name: "get it", after: 500, act: () => { detailsData = Object.assign({}, detailsData, { needs: null }); chainQueue = []; check("Browser check: (the mod for the auto close needs nothing else)"); pad("a") } },
            { name: "file page again", max: 30000, wait: () => browserOpener === "getit" && pageIs(/tab=files/),
              act: () => { bt.pressAt = Date.now(); btClick(160, 188) } },
            { name: "slow download", max: 15000, wait: () => webDls.some(x => x.name === "TurbineTest-Bigger_Pockets-1.0.3.zip"),
              act: () => { check("Browser check: Slow download started " + Math.round((Date.now() - bt.pressAt) / 100) / 10 + " s after the press") } },
            { name: "auto close", max: 30000, wait: () => page !== "browser",
              act: () => { check("Browser check: after the add the page closed by itself; focus " + (zone === "play" ? "on Play" : "on " + zone)
                                 + "; My mods " + (mods.some(m => /bigger.pockets|turbinetest/i.test(m.name + " " + (m.sourceId || ""))) ? "lists it" : "doesn't list it (" + mods.map(m => m.name).join(", ") + ")"))
                           check("Browser check: the pointer is hidden after the close: " + (Turbine.pointerShown ? "no" : "yes"))
                           bt.launches = launchCount; pad("a") } },
            { name: "after the close", after: 300, act: () => check("Browser check: A just after the auto close started nothing: " + (launchCount === bt.launches && sheet === "" ? "yes" : "no")) }
        ])
    }
    // The second run: the profile kept the sign-in; then a page that tries everything by itself with no press (a Mod
    // Manager Download link from a script and from a frame, a pop-up, a download): nothing happens. Then a Slow
    // download, and the page closed while it runs: Turbine says it goes on and where to watch it.
    function browserPersistScript() {
        const pageIs = (re) => !!nexusView && !!nexusView.view && !nexusView.loading && re.test(nexusView.url)
        btRun([
            { name: "open", act: () => openNexus(btFiles + "&auto=1") },
            { name: "loaded", max: 45000, wait: () => !!nexusView && !!nexusView.view && !nexusView.loading && /tab=files/.test(nexusView.url),
              act: () => check("Browser check: second run loaded " + nexusView.url) },
            { name: "tries", after: 9000, act: () => { check("Browser check: no press, nothing downloaded: " + (webDls.length === 0 && !dl.active ? "yes" : "no (" + webDls.map(x => x.name).join(", ") + ")"))
                                                         // M5: a link on mod 1's page to another mod's page that goes to its nxm link by itself.
                                                         btClick(700, 512) } },
            { name: "another mod asks", max: 15000, wait: () => nxmAsking && menuIndex === 0 && nxmShownAt > 0 && Turbine.monoMs() - nxmShownAt > 1400,
              act: () => { check("Browser check: another mod's link asks: " + nxmAskWords()); pad("b") } },
            { name: "auto page again", after: 600, act: () => { check("Browser check: Not now closed it: " + (sheet === "" ? "yes" : "no")); nexusView.load(browserAddress(btFiles + "&links=1")) } },
            // H1b: A on a link to an outside site that sends the view straight back to a Nexus page that goes to its
            // Mod Manager Download link by itself (after the page's own tries are over).
            { name: "outside link", max: 20000, wait: () => pageIs(/links=1/), after: 800, act: () => btClick(700, 452) },
            { name: "bounced", max: 20000, wait: () => pageIs(/file_id=55/), after: 6000,
              act: () => check("Browser check: after a bounce off Nexus and back, nothing downloaded: " + (webDls.length === 0 && !dl.active ? "yes" : "no")) },
            // H1c: an outside page read for a while; a tap on it sends the view to mod 1's page that does the same.
            { name: "outside page", after: 300, act: () => nexusView.load("http://127.0.0.1:47801/tapme") },
            { name: "tap outside", max: 15000, wait: () => pageIs(/tapme/), after: 800, act: () => btClick(190, 80) },
            { name: "tap off nexus asks", max: 15000, wait: () => nxmAsking && menuIndex === 0 && nxmShownAt > 0 && Turbine.monoMs() - nxmShownAt > 1400,
              act: () => { check("Browser check: a tap off Nexus asks: " + nxmAskWords()); pad("b") } },
            { name: "slow", max: 15000, wait: () => sheet === "" && pageIs(/file_id=56/), after: 600,
              act: () => { check("Browser check: after Not now, nothing downloaded: " + (webDls.length === 0 && !dl.active ? "yes" : "no")); bt.pressAt = Date.now(); btClick(160, 188) } },
            // The page closes while the download runs (the bounce left pages to go back to, so it closes outright).
            { name: "running", max: 20000, wait: () => webDlsRunning > 0, act: () => closeBrowser("back") },
            { name: "closed", max: 5000, wait: () => page !== "browser",
              act: () => check("Browser check: closed while downloading; at the top: " + (webDlsRunning > 0 ? "Getting " + plural(webDlsRunning, "mod", "mods") : "nothing") + "; game menu has Nexus downloads: " + (gameMenuItems.some(x => x.act === "nexusdl") ? "yes" : "no")) },
            { name: "landed", max: 20000, wait: () => webDls.some(x => x.state === "added" || x.state === "failed" || x.state === "asked"),
              act: () => check("Browser check: the download went on after the close: " + webDls.map(x => x.name + " " + x.state).join(", ")) }
        ])
    }
    // The 0.54 fix: a helper report with no finish, a refused finish, and the Slow download hint replaced.
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

    // ---------------------------------------------------------------- the beginner tutorial (first run)
    // Four cards with the real buttons drawn large: A and B, X and Y; View and the quick wheel; getting a mod; Play.
    // A or Right: next card; Left: back one; B or Start: skip. It ends on the rail ("Pick a game to start"), runs
    // once (remembered) and Settings, Help shows it again.
    readonly property int tutCount: 4
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
        // Shown again from Settings, Help: back to Settings (controls #5).
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
    readonly property var sortNames: srcNow === "nexus" ? ["Trending", "Most liked", "Most downloaded", "Recently updated", "Newest"]
                                                        : ["Most downloaded", "Top rated", "Recently updated", "Newest"]
    property var browseData: null
    property bool browseLoading: false
    property bool loadingMore: false
    property bool noMore: false
    property string getArea: "grid"       // recent (Good first mods), grid, fallback
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
        if (mine !== undefined) {
            const mm = mods.find(y => y.id === mine)
            return Object.assign({}, x, { installed: true, on: mm ? mm.enabled === true : undefined, update: !!updateMap[mine] })
        }
        return !x.installed ? x : Object.assign({}, x, { installed: true })
    })
    // Updates for your mods, on the Get mods start page.
    readonly property var updateList: Object.keys(updateMap).map(k => ({ u: updateMap[k], m: mods.find(x => x.id === k) })).filter(x => !!x.m)
    // 0.56: Good first mods (from the game's data file) on the Get mods start page, for this source.
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
            // Other files' step (controls #13) is never a place to go back to.
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
        // 0.59: B goes to My mods; a search stays as it is (the line over the cards names it).
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
    // X on a card: the mod page's own words (Get it on Nexus for a free account, else Install), Other files, Open on Nexus.
    readonly property var cardItem: getArea === "grid" ? getItems[getIndex] : null
    // 0.56: a card you already have offers what you can do with it, not getting it again.
    // Nexus cards by the Nexus mod id; Thunderstore cards by the package ("Owner-Name", with or without a version).
    readonly property var cardMine: !cardItem || !cardItem.installed ? null
        : srcNow === "nexus" ? (installedNexus[String(cardItem.id)] !== undefined ? mods.find(m => m.id === installedNexus[String(cardItem.id)]) || null : null)
        : mods.find(m => m.sourceKind === srcNow && (String(m.sourceId) === String(cardItem.id) || String(m.sourceId || "").split(":")[0] === String(cardItem.id))) || null
    readonly property var cardMenuItems: !cardItem ? [] : cardMine ? [
        { icon: "info", title: "Show in My mods", detail: cardMine.name, act: "show" },
        { icon: "power", title: cardMine.enabled ? "Turn off" : "Turn on", detail: cardMine.enabled ? "On now" : "Off now", act: "toggle" }
    ].concat(updateMap[cardMine.id] ? [{ icon: "update", title: "Update to " + updateMap[cardMine.id].to, detail: "Keeps its place and on or off", act: "update" }] : [])
     // 0.6 (I23): the mod's own actions, the same as on its row in My mods.
     .concat(settingsFilesOf(cardMine.id).length > 0 ? [{ icon: "info", title: isSeamless(cardMine) ? "Co-op settings" : "Edit settings", detail: "Its settings file", act: "m:settings" }] : [])
     .concat(ordered && mods.length > 1 ? [{ icon: "move", title: "Move", detail: "In My mods: Up and Down move it, A puts it down", act: "m:move" }] : [])
     .concat([{ icon: "info", title: "Rename", detail: cardMine.renamed ? "Now " + cardMine.name : "A name of your own for the list", act: "m:rename" }])
     .concat([srcNow === "nexus" ? { icon: "play", title: "Its Nexus page", detail: "Opens it in Turbine", act: "nexus" }
                                 : { icon: "play", title: "Its Thunderstore page", detail: "Opens in Turbine", act: "page" },
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
        : detailsData.source === "nexus" && detailsData.premium === false ? "Opens Nexus right here. Sign in if asked (free), then press Slow download. It's added by itself."
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
    readonly property bool pageHelp: openHelp && outsideOpen && modPage && !!detailsData && !detailsData.loading
    readonly property var pageHelpLabels: pageHelp ? ["Open in Steam's browser", "Always use Steam's browser"] : []
    onPageHelpChanged: if (!pageHelp) pageInline = -1
    readonly property string pageALabel: pageInline >= 0 ? (pageHelpLabels[pageInline] || "") : pageIndex === 0 ? mainLabel : pageIndex === 1 ? "Other files" : "Open its page"
    // X on the mod page: Other files, Get what it needs, Open on Nexus.
    // The Other files row on the page itself (off: it is in X Options, with the main file's line).
    readonly property bool pageOtherRow: false
    readonly property var pageMenuItems: !detailsData || detailsData.loading ? [] : (otherVersions.length > 0 ? [
        { icon: "update", title: "Other files", detail: (detailsData.mainLine ? detailsData.mainLine + " · " : "") + plural(otherVersions.length, "other file", "other files"), run: () => openVersions() }
    ] : []).concat(pageMissing >= 0 ? [
        { icon: "update", title: "Get what it needs", detail: pageNeeds[pageMissing] ? pageNeeds[pageMissing].name : "", run: () => { pageIndex = pageMissing + 2; openNeed(pageMissing) } }
    ] : []).concat((detailsData.source || srcNow) === "nexus" && detailsData.page ? [
        { icon: "play", title: "Open on Nexus", detail: "Its page, in the browser", run: () => openNexus(detailsData.page) }
    ] : [])
    // 0.6: the installed mod's Edit settings on its page (Left reaches it, A opens it).
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
            if (pageInline === 0) steamOpen()
            else if (pageInline === 1) alwaysSteamNow()
            else if (pageIndex === 0) installFromDetails()
            else if (pageIndex === 1) openVersions()
            else openNeed(pageIndex - 2)
        }
        else if (b === "up") {
            if (pageInline === 1) pageInline = 0
            else if (pageInline === 0) pageInline = -1
            else if (pageHelp && pageIndex > 0 && (pageIndex === 1 || (pageIndex === 2 && !pageOtherRow))) { pageIndex = 0; pageInline = 1 }
            else if (pageIndex > 0) pageIndex = pageIndex === 2 && !pageOtherRow ? 0 : pageIndex - 1
            else modView.scroll(-1)
        }
        else if (b === "down") {
            if (pageInline < 0 && pageIndex === 0 && pageHelp) { pageInline = 0; return }
            if (pageInline === 0) { pageInline = 1; return }
            // The Other files row (1) is hidden when there are none: skip it, and scroll when no row is left.
            const next = pageIndex === 0 && !pageOtherRow ? 2 : pageIndex + 1
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
        // Installing already (controls #12): a second press never starts a second install.
        if (busyHere) { note("action", "Install pressed while installing: ignored"); toast.info("Installing " + detailsData.name + " already."); return }
        if (detailsData.partial) { if (detailsData.page) openNexus(detailsData.page); return }
        if (pageInstalled) {
            // Off: A turns it on (the button then says Show in My mods). On: A shows it in My mods.
            if (pageModOff) toggle(pageMod)
            else if (pageMod) goToMod(pageMod.id)
            // Installed, but its row isn't found by id (controls #26): My mods all the same.
            else { modPage = false; history = []; tab = 0; zone = "main"; toast.info("It's in My mods.") }
            return
        }
        if (detailsData.source === "nexus" && detailsData.premium === false) {
            const url = detailsData.filePage || detailsData.page
            note("action", "Get it on Nexus: " + detailsData.name + " (" + url + ")")
            // 0.64: what it needs is offered once it is in ("This mod needs N others", Get them all).
            chainQueue = []
            chainKeep = false
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
        // 0.59 (controls #4): the list steps aside so the page shows and takes the presses; it comes back, on the
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
    // player's own browser (Settings, Getting mods, Open Nexus in); Chrome through Steam and Steam's browser are in Labs.
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
    property real pointerMovedAt: 0
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
    // Every "open Nexus" goes through here: in Turbine by default, the player's own browser when he chose it
    // (Desktop Mode), or the old Game Mode ways when Labs has them on.
    function openNexus(url, opener) {
        // 0.6 (I13): Game Mode always opens Nexus inside Turbine (Chrome and Steam's browser routes are retired).
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
        outsideOpen = false
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
        browserBackground = webDls.filter(x => x.state === "requested" || x.state === "progress" || x.state === "paused").map(x => x.path)
        if (webDlsRunning > 0 && why !== "play") {
            const t = "Still downloading " + plural(webDlsRunning, "mod", "mods") + ". It's added by itself."
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
    // 0.64: a mod's file page opened by Get it: its Slow download button is scrolled into view and the pointer put on
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
    // The Desktop Mode setting, and the Labs ones for Game Mode.
    property bool outsideOpen: false

    // ---- downloads (the profile's downloadRequested: the request is only good during the signal)
    Connections {
        target: app.browserLive && app.browserOpen ? NexusProfile : null
        function onDownloadRequested(d) { app.webDownload(d) }
    }
    function webDownload(d) {
        const pageUrl = nexusView ? nexusView.url : browserStart
        // What running downloads still have to come counts against the free space too.
        let pending = 0
        for (const x of webDls) if ((x.state === "requested" || x.state === "progress" || x.state === "paused") && x.total > 0) pending += Math.max(0, x.total - x.got)
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
    readonly property int webDlsRunning: { webDlTick; return webDls.filter(x => x.state === "requested" || x.state === "progress" || x.state === "paused").length }
    // All running downloads' share done (-1 when a size isn't known), for "Getting 1 mod · 40%" at the top.
    readonly property real webDlPart: {
        webDlTick
        const run = webDls.filter(x => x.state === "requested" || x.state === "progress" || x.state === "paused")
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
        const run = webDls.filter(x => x.state === "requested" || x.state === "progress" || x.state === "paused")
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
            return { text: browserSignedIn ? "Press Slow download (free. Mod Manager Download needs your Nexus key)" : "Sign in first (free), then press Slow download", level: "hint", progress: -1 }
        return { text: "", level: "ok", progress: -1 }
    }
    // Signed in to Nexus in Turbine's browser (its sign-in cookie), looked at while the page is open.
    // 0.62: one sign-in status everywhere (Settings, the page's hints, the Nexus menu): whether Turbine's browser
    // holds Nexus's sign-in. It is read all the time, not only while a Nexus page is open, and the browser's kept
    // sign-in is read a moment after start (before 0.62 Settings said "Not signed in" until Nexus was opened once).
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
        openPick(x.name, x.game || "Other", (x.state === "progress" || x.state === "paused" || x.state === "requested" ? [
            { icon: "remove", title: "Cancel", detail: "Stops this download", danger: true, run: () => { if (x.obj) x.obj.cancel() } }
        ] : []).concat(x.state === "added" ? [
            { icon: "info", title: "Show in Turbine", detail: "Its game's My mods", run: () => closeBrowser("menu") }
        ] : []).concat(Turbine.gamemode ? [] : [
            // A file manager means nothing in Game Mode: Desktop Mode only.
            { icon: "info", title: "Open folder", detail: dir.replace(/^.*\/Downloads\//, "Downloads/"), run: () => { if (!shot.on && !Qt.openUrlExternally("file://" + encodeURI(dir))) toast.showPlain("It's in " + dir) } }
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
            { icon: "restore", title: "Close", detail: webDlsRunning > 0 ? "Downloads go on and are added by themselves" : "Back to where you opened it", run: () => closeBrowser("menu") }
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
            if (what === "popupblocked") app.browserSay("The page tried to open a window by itself. Turbine only opens one when you press.", "warn")
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
            browserSay("Mod Manager Download needs your Nexus key (Settings, Getting mods, Faster downloads). Press Slow download here instead: it's free", "warn")
            noKeyNoteClear.restart()
            return
        }
        browserNoKeyPage = target
        browserSay("Mod Manager Download needs your Nexus key (Settings, Getting mods, Faster downloads). A opens this file's page, where Slow download is free", "warn")
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
        toast.showPlain(t)
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
        if (r.how === "nohelper" || r.how === "error") { browserSay(r.text, "warn"); toast.showPlain(r.text); return }
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
            else if (Turbine.gamemode) toast.showPlain(text + ". Press the Steam button, then Turbine.")
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
    function openNexusOutside(url) {
        outsideOpen = true
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
        if ((shot.on && shot.page !== "inputcheck") || browserSet.oldHelperShown === (v || "?")) return
        if (!shot.on) browserSet.oldHelperShown = v || "?"
        note("browser", "Showing: the browser helper " + (v || "") + " is out of date (once)")
        // Never over another sheet (the tutorial above all): it waits for that one to close.
        if (sheet !== "") { pendingOldHelper = true; return }
        infoIndex = 0
        sheet = "oldhelper"
    }
    property bool pendingOldHelper: false
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
        // The focus goes to Open in Steam's browser (the main button's guard ends now: A there would reopen the
        // browser that just failed to show).
        if (modPage && pageIndex === 0 && pageInline < 0) pageInline = 0
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
        // Never over the Nexus page: they wait for it to close.
        if ((sheet === "" || sheet === "notes") && page !== "browser") { infoIndex = 0; sheet = "notes" }
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
        // 0.59 (newbie M4, the owner's B rule): B is Not now (it asks again next time); Not a mod, which is for good,
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
                    chainKeep = true
                    openDetails({ id: String(next.modId), name: next.name, source: "nexus", icon: "" }, modPage)
                    pendingGet = true
                } else openNexus(next.url)
            }
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
            if (label === "Not now") { note("action", "Found a download: not now (asked again next time)"); return }
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
        // Turbine launched again with no link: this one comes to the front (the second one has closed).
        if (link === "turbine:show") { if (!shot.on) { app.raise(); app.requestActivate() } return }
        // 0.6: a request from Steam's menu (the Decky plugin) for one game.
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
            toast.showPlain(r.error)
            return
        }
        if (r.error) { note("nxm", "Link refused: " + r.error); say(r.error, "error"); if (browserOpen) browserSay(r.error, "warn"); return }
        // 0.6 (I9): every Mod Manager Download link asks first, wherever it came from (Turbine's page, the
        // browser, a second launch, the command line): nothing downloads until the player says Get it.
        if (!(browserOpen && page === "browser")) { const gi = games.findIndex(g => g.id === r.game); if (gi >= 0 && gi !== current) current = gi; page = "game"; zone = "main"; modPage = false }
        nxmAsk(link, r)
    }
    // 0.6 (I17): after the quiet second the ring moves to Get it by itself, so A says yes (B is still Not now).
    // Started from the question's first frame on screen (nxmShown); a press of a direction or a tap stops it.
    Timer { id: nxmRingMove; interval: 1200; onTriggered: if (app.nxmAsking && app.sheet === "pick" && app.menuIndex === 1) { app.menuIndex = 0; app.nxmRingMoved = true; app.note("nxm", "The ring moved to Get it") } }
    property bool nxmRingMoved: false
    property int nxmGets: 0
    // 0.6 (break M2): the quiet second and the ring's move count from the first frame that shows the question.
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

    // 0.6: Steam's menu (the Decky plugin, decky/) asks Turbine, open or just started for it, to do one thing for one
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
        if (what === "report") { openReport(null, "Problem with " + games[i].name + ": ", ""); return }
        // A Play: once this game's mods are read (the seamless check and the check before Play need them).
        if (!modsLoaded && tries < 20) { quickWaiting = { link: link, tries: tries + 1 }; quickRetry.restart(); return }
        if (what === "play") startPlay()
        else if (what === "play-coop") { if (seamlessMod) startCoop(); else toast.problem("Seamless Co-op isn't installed for " + games[i].name + ". Get it from the game menu.") }
        else if (what === "play-plain") Turbine.playOriginal()
    }

    // "Get <mod> (<file>) for <game>?", asked for every Mod Manager Download link from Turbine's page. It opens on
    // Not now and takes no input at all for its first second (a page saying "press A again" can't have the next
    // press answer it); Right (or a tap on Get it) then A gets it. One question at a time: a second link while it
    // is open is dropped. The names come from Nexus a moment later.
    property string nxmAskLink: ""
    property var nxmAskInfo: null
    property real nxmAskAt: 0
    property string nxmAskMod: ""
    property string nxmAskFile: ""
    readonly property bool nxmAsking: sheet === "pick" && nxmAskLink !== "" && pickBLabel === "Not now"
    // No input for the question's first second on screen; `at`: when the press was made (default: now).
    function nxmAskQuiet(at) { return nxmAsking && (nxmShownAt === 0 || (at === undefined ? Turbine.monoMs() : at) - nxmShownAt < 1000) }
    function nxmAskTitle() {
        const m = /^nxm:\/\/[a-z0-9_-]+\/mods\/([0-9]+)\/files\/([0-9]+)/i.exec(nxmAskLink)
        return "Get " + (nxmAskMod !== "" ? nxmAskMod : "mod " + (m ? m[1] : "")) + " (" + (nxmAskFile !== "" ? nxmAskFile : "file " + (m ? m[2] : "")) + ") for "
            + (nxmAskInfo && nxmAskInfo.name ? nxmAskInfo.name : "this game") + "?"
    }
    function nxmAsk(link, r) {
        if (nxmAsking) { note("nxm", "Another Mod Manager Download link while asking about one: dropped"); return }
        note("nxm", "Asking before it downloads: a Mod Manager Download link from Turbine's page")
        nxmAskLink = link; nxmAskInfo = r; nxmAskMod = ""; nxmAskFile = ""; nxmAskFirst = null; nxmAskFrame = null; nxmAsks++
        // The same link asked again: its names are known already (the answer only tells when it changes).
        nxmNameGot(parse(Turbine.nxmNameJson, {}), true)
        Turbine.nxmAskName(link)
        openPick(nxmAskTitle(), "Turbine asks before every Mod Manager Download from a page. Pick Get it (Right, or a tap), then press A.", [
            { icon: "play", title: "Get it", detail: "Downloads it and adds it to " + (r.name || "the game"), aLabel: "Get it", run: () => nxmAskGet() },
            { icon: "remove", title: "Not now", detail: "Nothing is downloaded", aLabel: "Not now", run: () => nxmAskNo() }
        ], "")
        pickBLabel = "Not now"
        menuIndex = 1
        nxmAskAt = Date.now()
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
        // 0.59 (controls #5): where B goes back to is taken every time Settings opens, never left over from before.
        if (page !== "settings") settingsFrom = { zone: zone, tab: tab, modPage: modPage, railIndex: railIndex, browser: page === "browser" }
        page = "settings"; zone = "settings"
        if (selfUpd.state === "available" && settingsSections.indexOf("Updates") >= 0) setSection = settingsSections.indexOf("Updates")
        setRow = firstSettingsRow()
        if (sec === "Look") checkDisplays()
    }
    function openSection(name) { openSettings(); setSection = Math.max(0, settingsSections.indexOf(name)); setRow = firstSettingsRow() }
    onLabsOnChanged: if (!labsOn && setSection >= settingsSections.length) { setSection = settingsSections.indexOf("About"); setRow = 0 }
    // Accounts, Updates and Labs: rows of one kind each. action (A does it), toggle and choice (Left, Right or A
    // change it), item (a Labs checklist item), info (words only).
    readonly property var rowsNow: sec === "Getting mods" ? accountRows : sec === "Updates" ? updateRows : sec === "Labs" ? labsRows : sec === "Help" ? helpRows : []
    // Help: the buttons in one place, the beginner tutorial again, and games hidden from the rail.
    readonly property var helpRows: [
        { kind: "action", key: "tutorial", title: "Beginner tutorial", how: "Four cards: the buttons, the quick wheel, getting a mod, and Start", action: "Show it" },
        { kind: "action", key: "launchundo", title: "Undo the launch options change", how: "For games Turbine starts through Steam with me3: puts Steam's Launch Options back as they were", action: "Show" },
        { kind: "action", key: "problem", title: "Report a problem", how: "Saves a report file in Downloads to send: what Turbine did lately, its settings and your games. No keys or passwords.", action: "Save report" },
        { kind: "action", key: "decky", title: "Steam menu plugin (Decky)", how: deckyWords, action: decky.plugin === true ? "How to use it" : "Show how" },
        { kind: "info", key: "buttons", title: "The buttons",
          how: "A: do it. B: back. X: options. Y: find and sort. LB RB: tabs. LT RT: page up or down. View: quick wheel. Start: Settings (hold 3 seconds: Play). Right stick: pointer, RT clicks. R3: the same as A; while a message offers a step (Undo), R3 does that step." },
        { kind: "info", key: "keys", title: "Keyboard and mouse",
          how: "Move: W A S D or the arrows. Enter or Space: do it. Esc or Backspace: back (Esc on a game's first page: Settings). Q and E: the last and the next page (Tab too). F: find. R or a right-click: options. Hold Tab: quick wheel (pick with the mouse or W A S D, let go to choose). Hold Space for 3 seconds: Play. A double-click opens a mod. The wheel scrolls; Ctrl and the wheel, or Ctrl+= and Ctrl+-, change the size. F1: Help. The mouse's Back button goes back and its Forward button turns the page." },
        { kind: "info", key: "touch", title: "Touch",
          how: "Tap does it. Hold for options (Move is there: then tap where the mod goes). Swipe sideways on the bottom bar to switch tabs." }
    ].concat(steamArt.found ? [
        { kind: "action", key: "steamart", title: "Steam library art", how: steamArt.on ? "Turbine's pictures in your Steam library (your own pictures are never replaced)" : "Off: Steam shows its own pictures for Turbine", action: steamArt.on ? "Remove" : "Add" }
    ] : []).concat(hiddenIds.length > 0 ? [
        { kind: "action", key: "unhide", title: "Games hidden from the rail: " + hiddenIds.length, how: "Puts them back on the rail", action: "Show them" }
    ] : []).concat(unseenFolders.map(f => (
        // 0.6 (integration I5): a Steam library Turbine isn't allowed to read, with the one press that allows it.
        { kind: "action", key: "allowfolder", folder: f, title: "Turbine can't see " + f, how: "Your games there don't show. Allow lets Turbine read that folder (it restarts once).", action: "Allow" })))
    // Steam libraries the scan found but isn't allowed to read (engine steam.rs, UNSEEN_LIBRARY).
    readonly property var unseenFolders: String(Turbine.problems || "").split("\n").map(l => (l.match(/^Can't see the Steam library at (.*) \(Turbine isn't allowed to read that folder\)$/) || [])[1]).filter(x => !!x)
    // 0.6: the optional Steam menu plugin (decky/): Turbine's panel in the Quick Access menu, through Decky Loader.
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
        // 0.59 (R5): first, whether you're signed in to Nexus, and the one button for it.
        { kind: "action", key: "nexusSign", title: "Nexus Mods: " + (browserSignedIn ? "Signed in" : "Not signed in"),
          how: browserSignedIn ? "Get it on Nexus opens the file's page here and adds the download by itself."
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
          value: "", options: ["-", "+"], names: ["- 10%", "+ 10%"] }
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
        // 0.6 (I13): the helper stays only for signing in (Google blocks sign-in inside Turbine's page).
        { kind: "action", key: "helper", title: "Browser helper (for signing in)", action: "Set up",
          how: "For Chrome: brings a Google (or other) sign-in on Nexus back to Turbine." }
    ]).concat(ownRowsShown && bridge.outdated ? [
        { kind: "info", key: "helperOld", title: "Your browser helper is out of date" + (bridge.helper ? " (" + bridge.helper + ", Turbine has " + bridge.bundled + ")" : ""),
          how: bridge.manual ? "In Chrome, open chrome://extensions and press the reload arrow on Turbine browser helper. After that it updates by itself."
                             : "Turbine put the new version in its folder and asks it to reload when it connects." }
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
        // 0.59 (newbie H8): in Game Mode only what Steam starts shows, so Turbine closes and Steam starts it again.
        Turbine.gamemode ? { kind: "action", key: "restart", title: "Close Turbine", how: "Then start it again from Steam to use the new version", action: "Close" }
                         : { kind: "action", key: "restart", title: "Restart Turbine", how: "Closes Turbine and opens the new version", action: "Restart" }
    ] : []).concat([
        { kind: "toggle", key: "auto", title: "Check when Turbine starts", how: "Looks at a start, at most once every 6 hours. Nothing installs without you.", value: updates.auto, options: [false, true], names: ["Off", "On"] },
        { kind: "info", key: "channel", title: "Update channel: " + (labsOn && updates.prerelease ? "Pre-release" : "Stable"), how: labsOn ? "Change it in Labs" : "Stable releases only" }
    ])
    function rowStep(r, d) {
        if (r.key === "auto") updates.auto = !updates.auto
        else if (r.key === "overlay") browserSet.overlay = !browserSet.overlay
        else if (r.key === "alwaysSteam") { if (!shot.on) browserSet.alwaysSteam = !browserSet.alwaysSteam }
        else if (r.key === "labsChrome") { if (!shot.on) browserSet.labsChrome = !browserSet.labsChrome }
        else if (r.key === "openIn") { if (!shot.on) browserSet.openIn = browserSet.openIn === "mine" ? "turbine" : "mine" }
        else if (r.key === "zoom") { browserZoomBy(d * 0.1); if (!shot.on) browserSet.zoom = browserZoom }
        else if (r.key === "noGpu") { webNoGpu = !webNoGpu; Turbine.setWebNoGpu(webNoGpu); toast.showPlain("Takes effect the next time Turbine starts.") }
        else if (r.key === "protonLog") { protonLog = !protonLog; Turbine.setProtonLog(protonLog); toast.showPlain(protonLog ? "On: the next Steam launch writes Proton's log. Report a problem includes it." : "Off.") }
        else if (r.key === "channel") { updates.prerelease = !updates.prerelease; Turbine.selfCheck(updates.prerelease, false) }
        note("action", "Setting " + r.key + " changed")
    }
    // 0.58: every Settings action row's key has its handler here (the build checks that each row in every section has
    // one, so a row can't silently do nothing again: Labs > Activity log did in 0.57).
    readonly property var rowActions: ({
        report: () => openReport(null, "", ""),
        perf: () => openPerf(),
        activity: () => openActivity(),
        problem: () => saveProblemReport(),
        decky: () => openDecky(),
        launchundo: () => openLaunchUndo(false),
        steamart: () => { const w = Turbine.steamArtSet(!steamArt.on); steamArt = parse(Turbine.steamArtJson(), { found: false, on: true }); note("action", "Steam library art: " + (steamArt.on ? "added" : "removed")); toast.info(w) },
        steamBrowser: (r) => labsRun(r), keyless: (r) => labsRun(r), gmBrowser: (r) => labsRun(r),
        nexusKey: () => openAsk("nexusKey", "Nexus key", keyWhere + " It's kept in a file only you can read.", "", "API key", 1024, true, "Check it"),
        pasteKey: () => pasteKey(),
        keyFold: () => { keyRowsOpen = true; setRow = accountRows.findIndex(x => x.key === "pasteKey") },
        ownFold: () => { ownRowsOpen = true; setRow = accountRows.findIndex(x => x.key === "helper") },
        keyPage: () => { if (openNexus(keyPageUrl)) toast.showPlain("Opened API Keys. Copy the Personal API Key at the bottom, come back and press Paste.") },
        signOut: () => Turbine.nexusSignOut(),
        webSignOut: () => Turbine.browserSignOut(),
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
            openMsg({ title: "Turbine can read " + r.folder + " now", text: "It sees the games there after it starts again.",
                      buttons: [{ key: "a", label: "Restart Turbine", run: () => { Turbine.restart(); Qt.quit() } }, { key: "b", label: "Later", run: () => {} }] })
        },
        addChrome: () => showAddBrowser("noshortcut"),
        nxmHandler: () => Turbine.handlerSet(),
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
    // 0.58, Settings > Report a problem: one text file in Downloads; where it went is shown.
    function saveProblemReport() {
        const lines = [
            "Interface size: " + sizeNow + "%", "Layout: " + layoutNow, "Theme: " + look.theme, "Accent: " + look.accent,
            "Contrast: " + (look.highContrast ? "High" : "Standard"), "Color vision: " + look.vision, "Compact list: " + look.compactList,
            "Animations: " + mo.level, "Confirm by: " + look.confirmBy, "Seasonal: " + seasonPick,
            "Open Nexus in: " + browserSet.openIn, "Status over other apps: " + browserSet.overlay,
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
    // The build's check: every action row in every Settings section (Labs on) has a handler.
    function settingsRowsCheck() {
        const all = accountRows.concat(updateRows, labsRows, helpRows)
        const acts = all.filter(r => r.kind === "action")
        const missing = acts.filter(r => !rowActions[r.key]).map(r => r.key)
        note("check", "Settings rows check: " + acts.length + " action rows, " + missing.length + " without an action" + (missing.length ? " (" + missing.join(", ") + ")" : ""))
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
        // 0.58: modded launches through Steam, Seamless Co-op, Edit settings, the bar.
        { id: "steam-me3", title: "Elden Ring with mods in Game Mode", how: "Game Mode, Elden Ring with a mod on: press Play. Answer Restart Steam and set it up: Steam closes, comes back, and Turbine goes on with the Play.",
          expect: "The game shows in Game Mode (not Steam's blank launching screen) with the mod, offline. Steam's own Play button starts it with mods too; with every mod off it starts normally online." },
        { id: "steam-plain", title: "Play without mods after the step", how: "With the Launch Options set and mods on, Play options, Play without mods.",
          expect: "Elden Ring starts normally with its anti-cheat; the next Play starts it with mods again." },
        { id: "coop-play", title: "Play Seamless Co-op", how: "Elden Ring with Seamless Co-op: press Play and choose Play Seamless Co-op (an empty session password opens its settings first).",
          expect: "Seamless Co-op starts with your password and its own co-op save; Play with mods starts the solo modded game." },
        { id: "edit-settings", title: "Edit settings with the controller", how: "X on a mod with an .ini or .cfg file, Edit settings. Change a toggle, a number and a text (Steam keyboard); each saves by itself.",
          expect: "The game uses the new values; Reset to default puts the mod's own back; a mod update keeps your values." },
        { id: "launch-check", title: "The didn't start sheet", how: "Press Play on a game that can't start (for example Elden Ring with the Launch Options text removed in Steam).",
          expect: "About 20 seconds later a sheet says it didn't start, with the Launch Options step, Play without mods and the Activity log." },
        // 0.57: anti-cheat games.
        { id: "ac-me3", title: "Elden Ring with mods through me3", how: "Elden Ring (or Nightreign, Armored Core VI): add a mod, press Play, answer Turn it off for mods. Turbine gets me3 the first time.",
          expect: "The game starts offline with the mod, without the anti-cheat. The game folder has no new files; your normal save is untouched (a separate save is used)." },
        { id: "ac-original", title: "Normal launch after mods", how: "After playing with mods, turn every mod off (or Play without mods).",
          expect: "Steam starts it the normal way, with its anti-cheat, and online play works. Steam's Verify finds nothing to repair." },
        { id: "ac-seamless", title: "Seamless Co-op through me3", how: "Elden Ring, game menu, Get Seamless Co-op. Then Play.",
          expect: "Seamless Co-op starts with the game and uses its own co-op saves; turning it off in My mods takes it out." },
        { id: "ac-halo", title: "Halo MCC guide", how: "Halo MCC: add a mod, press Play.",
          expect: "Before you play says to pick Anti-Cheat Disabled (Mods and Limited Services) in Steam's Play menu, and the mod works there." },
        { id: "ac-refuse", title: "Online-only games refuse mods", how: "Open Dead by Daylight, Apex Legends or Destiny 2 and try Add from Downloads.",
          expect: "Turbine says the game needs its anti-cheat and adds nothing." },
        { id: "pad-quiet", title: "Controller quiet after a game", how: "Play a game, press buttons as you quit it, then come back to Turbine.",
          expect: "Nothing happens in Turbine from buttons pressed in the game." },
        { id: "proton-bepinex", title: "Proton setting on a BepInEx game", how: "Add a BepInEx mod to Valheim, Lethal Company or Risk of Rain 2, then Play.",
          expect: "The check before Play says BepInEx gets turned on in Proton, and the game starts with the mod working." },
        { id: "running-check", title: "Running game check", how: "With a BepInEx game running, switch back to Turbine and press Play.",
          expect: "Turbine says the game is still running and changes nothing until it's closed." },
        { id: "nexus-keyless", title: "Get mods without a key", how: "Labs, Nexus without a key: Check. Then open Get mods in Skyrim, The Witcher 3 or Cyberpunk 2077.",
          expect: "The check says Nexus answered, and Get mods shows the game's mods as pictures with no key." },
        { id: "nexus-inside", title: "Nexus inside Turbine", how: "Game Mode: a mod page, A Get it on Nexus. Sign in, press Slow download with the pointer (right stick, RT).",
          expect: "The file's page opens full screen in Turbine. After Nexus's wait the pill shows the download, then Added; the page closes by itself and Play has the focus." },
        { id: "nexus-signed-in", title: "Nexus remembers the sign-in", how: "Sign in on the Nexus page in Turbine, close Turbine, open it and open a mod's Nexus page again.",
          expect: "Still signed in." },
        { id: "nexus-google", title: "Sign in with my browser (Google)", how: "Browser helper set up in Chrome. On the Nexus page press Sign in with my browser, sign in with Google in Chrome.",
          expect: "Turbine comes back to the front and says Signed in to Nexus; the page shows you signed in." },
        { id: "nexus-pad", title: "The controller on the Nexus page", how: "D-pad between links, A on a link, Y in a text field, B back, LB and RB tabs, View the quick wheel.",
          expect: "Each does what the legend says; Steam's keyboard opens in a text field; the pointer never hides." },
        { id: "game-mods-folder", title: "Slow download with no Save box (My browser)", how: "Desktop Mode, Open Nexus in: My browser, browser helper set up in Chrome. Press Slow download on a mod's file page.",
          expect: "No Save box; the file lands in Downloads, Game Mods, the game's folder, and Turbine adds it to that game." },
        { id: "desktop-pad", title: "Controller in Desktop Mode", how: "Desktop Mode with Steam running: use Turbine with the controller.",
          expect: "Buttons work (as Steam's mouse and keyboard if Turbine sees no pad itself), and no line says no controller was found." },
        { id: "text-size", title: "Interface size in Game Mode", how: "In Game Mode, look at every page at Interface size 50% to 150% (tap - and +, D-pad Left and Right, or A for the list).",
          expect: "Everything is readable; nothing is cut off or overlapping." },
        { id: "add-downloads", title: "Add mods from Downloads", how: "Put a mod's zip in Downloads, then View, Add from Downloads.",
          expect: "The mod shows in the list, turned on, with its name and version." },
        { id: "auto-apply", title: "Auto-apply, then start from Steam", how: "Turn a mod on or off, wait 5 seconds, close Turbine, start the game from the Steam library.",
          expect: "The game has the change without pressing Play in Turbine." },
        { id: "uninstall-undo", title: "Uninstall and put back", how: "Uninstall a mod from its options (X), then press A on its dimmed row.",
          expect: "The row stays, dimmed, for 8 seconds; A puts the mod back in the same place. After that it is in Removed mods at the bottom of the list." },
        { id: "play-original", title: "Play without mods", how: "Game menu (View) > Play without mods.",
          expect: "The game starts with no mods. The mods stay in Turbine and go back in at the next Play." },
        { id: "restore", title: "Restore last setup played", how: "Play with mods, change some, then Game menu (View) > Go back to a setup that worked.",
          expect: "The mods go back to how they were the last time the game was played." },
        { id: "sd-card", title: "Game on the SD card", how: "Add a mod to a game on the SD card and play it; then try with the card out.",
          expect: "The mod works. With the card out, Turbine says the game is offline, never that mods are gone." },
        { id: "heroic-play", title: "Heroic game Play", how: "Pick an Epic or GOG game from Heroic and press Play.",
          expect: "Heroic starts the game with the mods in place. In Game Mode, check it shows (Turbine says how to switch to it)." },
        { id: "seasonal", title: "Seasonal leaves and snow", how: "Settings > Look > Seasonal: Autumn, then Snow; try Amount and Size.",
          expect: "They fall smoothly behind the panels and stop while a game runs." },
        { id: "quick-report", title: "Screenshot and report", how: "Labs on: hold View for 7 seconds.",
          expect: "A brass ring fills, then a screenshot and report land in Downloads and the report pastes from the clipboard." },
        { id: "ts-install", title: "Thunderstore install", how: "Valheim, Lethal Company or Risk of Rain 2: Get mods, pick a mod, A Install.",
          expect: "It downloads with progress on the status line and the rail ring, and shows in My mods with what it needs." },
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
        { id: "nxm-handler", title: "Mod Manager Download", how: "Settings, Getting mods, Using your own browser: Use Turbine for Mod Manager Download. Then in Desktop Mode press Mod Manager Download on a Nexus mod's Files tab, with Turbine open and with it closed.",
          expect: "Turbine comes to the front on that game, the download shows on the status line, and the mod is added (with installer choices when it has them)." },
        { id: "nexus-free", title: "Free Nexus account", how: "With a free account: a Nexus mod's details, A Open on Nexus; also SKSE's Install in Skyrim.",
          expect: "The page opens in the browser; Mod Manager Download there sends the file to Turbine. In Game Mode the page opens inside Turbine." },
        { id: "nexus-requirements", title: "Nexus requirements", how: "Skyrim: details of a Nexus mod that needs others (like SkyUI needs SKSE).",
          expect: "Also needs lists them as Have it, Missing, Game DLC or Outside Nexus; Premium installs the missing ones with the mod, free accounts get their pages." },
        { id: "profile-nexus", title: "Loadout import with Nexus mods", how: "Open a loadout file that has Nexus mods.",
          expect: "Premium: Turbine downloads them too. Free: it lists the pages to open, and the loadout is made with what's there." },
        { id: "loot-sort", title: "Community list sorting", how: "Skyrim with a few plugin mods, online: Load order tab, A on Sort automatically, then X on a plugin with a badge.",
          expect: "The line at the top says Sorted with the community sort list; the game starts with that order; badges like Needs cleaning show notes, and X lists them." },
        { id: "nexus-key", title: "Nexus Mods key", how: "Settings, Getting mods, Faster downloads: add your personal API key with Steam's keyboard.",
          expect: "It says your name and Premium or Free; Nexus Mods browsing works; Sign out removes the key." },
        { id: "mod-updates", title: "Mod updates", how: "Have an older Thunderstore or Nexus mod, open its game.",
          expect: "Update ready shows on it; X Update or the game menu's Update all brings the new version, same place and on or off." },
        { id: "profiles", title: "Loadouts", how: "Game menu (View), Loadouts: make one, switch, save a loadout file, open it or a Thunderstore code.",
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
          expect: "When the file finishes, Turbine adds it to the right game, or shows Found <file>. Add it to <game>? with A Add, X another game, Y Not a mod, B Not now. The file stays in Downloads." },
        { id: "catch-closed", title: "Downloads while Turbine is closed", how: "Close Turbine, download a mod with Slow download, then open Turbine.",
          expect: "Turbine catches up on it at start (adds it, or asks). A file you answered Not a mod is never asked about again." },
        { id: "nxm-no-key", title: "Mod Manager Download without a key", how: "With no Nexus key in Settings, Getting mods, Faster downloads, press Mod Manager Download on a Nexus mod.",
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
          expect: "Only one browser starts. After 8 seconds two buttons show under Get it on Nexus: Open in Steam's browser (highlighted) and Always use Steam's browser. No box pops up." },
        { id: "rail-menu", title: "Pin and hide on the rail", how: "On the rail press X on a game: Pin to the top; on a tool like Lossless Scaling: Hide from the rail. Settings, Help shows it again.",
          expect: "The pinned game stays at the top; the hidden one is gone from the rail until shown again." },
        { id: "touch-hold", title: "Touch: hold for options", how: "Touch a mod row, a Get mods card and a plugin and hold half a second; swipe sideways on the tabs.",
          expect: "Holding opens that item's options; the swipe changes the tab; the legend hides until the next button press." },
        { id: "fewer-steps", title: "Fewer presses to a mod", how: "A game with no mods, free Nexus account, Game Mode: A on the game in the rail, A on a mod, A Get it on Nexus, Slow download, then Play.",
          expect: "The game opens on Get mods with the first mod highlighted, Get it on Nexus is highlighted on its page, and Play is highlighted when the mod arrives." },
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
        { id: "browser-helper", title: "Browser helper (optional)", how: "Settings, Getting mods, Using your own browser: Browser helper, Set up, follow the steps, then Pair. Download a mod on Nexus.",
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
        { kind: "action", key: "activity", title: "Activity log", how: "The last 500 lines of what Turbine did, to read, copy or save to Downloads", action: "Open" },
        { kind: "toggle", key: "noGpu", title: "Nexus page: draw without the GPU", how: "For a black or frozen Nexus page in Turbine. Takes effect the next time Turbine starts.",
          value: webNoGpu, options: [false, true], names: ["Off", "On"] },
        { kind: "toggle", key: "protonLog", title: "Proton log for me3 games", how: "Proton writes its own log for the next Steam launches of Elden Ring and the other me3 games. Report a problem includes it.",
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
            reportView.aboutLine = "Save report puts it in Downloads and copies it, with Turbine " + Turbine.version
                + ", this device, the game and its mods, and the last 300 lines of the activity log."
            // 0.59: a draft B kept comes back (unless this report starts with words of its own).
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
        if (reportView.text.trim() === "") { toast.showPlain("Say what went wrong first."); return }
        const stamp = Turbine.stamp()
        const withShot = reportView.include && reportView.shotUrl !== ""
        const shotName = withShot ? "Turbine-screenshot-" + stamp + ".png" : ""
        const text = Turbine.report(labsItemText(reportItem), reportView.text, reportContext,
                                    reportView.includePerf ? reportView.perf : "", reportView.includeLog && reportView.hasLog, shotName)
        const res = parse(Turbine.saveReport(text, stamp, withShot ? Turbine.shotTemp() : ""), { error: "Couldn't save the report." })
        if (res.error !== "") { toast.showPlain(res.error); return }
        Turbine.copyText(text)
        reportDraft = ""
        closeReport()
        note("labs", "Report saved: Downloads/" + res.report)
        // Where it went, on screen until OK (0.59: the owner couldn't tell it had saved).
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
        // 0.59: B never throws words away: they stay as a draft for the next Report a bug.
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
    // 0.62: the brass mark on a clear background, at every size; the splash takes the large one.
    readonly property string iconBig: "file:///app/share/icons/hicolor/512x512/apps/io.github.ravenor907.TurbineModManager.png"
    readonly property string iconSmall: "file:///app/share/turbine-mod-manager/icon-small.png"

    // ---------------------------------------------------------------- Look settings rows
    readonly property var layoutOptions: externalShown ? ["handheld", "desktop", "tv"] : ["handheld", "desktop"]
    readonly property var lookRows: [
        { label: "Interface size", key: "size", slider: true, min: sizeMin, max: sizeMax, unit: "%", now: sizeNow,
          step: (d) => sizePreview(sizeStep(d)),
          // 0.58: A (and a tap or click on the row) opens the list of sizes; Left and Right, and the - and + buttons,
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
        { label: "Hold Start does", options: ["choice", "last"], names: ["Show the Play choice", "Play my last choice"], key: "holdStartDoes",
          step: (d) => cycle("holdStartDoes", ["choice", "last"], d) },
        // 0.62: Full, Reduced (fades only: nothing travels or grows) or Off (instant).
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

    // ---------------------------------------------------------------- shared pieces
    component Label: Text {
        color: c.text
        font.family: app.bodyFont
        font.pixelSize: app.fpx(15)
        elide: Text.ElideRight
        verticalAlignment: Text.AlignVCenter
    }
    component Key: Rectangle {
        property string k: ""
        property bool square: k.length > 1
        // 0.63: no caps after the mouse (and `off`: this cap has nothing to say here). Without a size a row
        // leaves it out, so the words beside it close up.
        property bool off: false
        readonly property bool gone: app.mouseLegend || off
        implicitWidth: gone ? 0 : Math.max(app.px(24), keyText.implicitWidth + app.px(12))
        implicitHeight: gone ? 0 : app.px(24)
        clip: true
        radius: square ? app.px(5) : height / 2
        color: c.raised
        border.color: c.accentLo
        border.width: Math.max(1, app.px(1))
        Text {
            id: keyText
            anchors.centerIn: parent
            text: app.glyph(parent.k)
            visible: !parent.gone
            color: c.text
            font.family: app.bodyFont
            font.pixelSize: app.fpx(14)
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
    // 0.62: the hints follow the last input used: the keyboard's keys after a key, the mouse's buttons after the
    // mouse, the controller's after a press on it. (Never in Game Mode; the build's pictures keep the controller's
    // unless a check asks.)
    property bool keysTest: false
    readonly property bool keysLegend: !Turbine.gamemode && (!shot.on || keysTest) && (lastInput === "mouse" || lastInput === "keys")
    // 0.63: the keys a PC player expects (Page Up and Page Down still work, and are never shown).
    readonly property var keyNames: ({ a: "Enter", b: "Esc", x: "R", y: "F", lb: "Q", rb: "E", lt: "Home", rt: "End", view: "Tab", start: "Space", r3: "Ctrl+Z" })
    readonly property var mouseNames: ({ a: "Click", x: "Right-click" })
    // 0.63: after the mouse, no key caps at all: the pages and the actions are buttons with their names.
    readonly property bool mouseLegend: keysLegend && lastInput === "mouse"
    function glyph(k) {
        if (k === "rs") return "R stick"
        if (keysLegend && lastInput === "mouse" && mouseNames[k]) return mouseNames[k]
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
        readonly property bool clickable: (parent === legendRow || parent === hintMeasure) && ["a", "b", "x", "y", "r3"].indexOf(k) >= 0
        // 0.63: after the mouse the hint is a button with its name on it and no key cap.
        readonly property bool named: app.mouseLegend && clickable && label !== "" && k2 === ""
        // A finger-sized target in the bar (0.59: the key caps alone are 24 px).
        height: clickable ? Math.max(app.tpx(44), implicitHeight) : implicitHeight
        spacing: app.px(6)
        TapHandler { enabled: hint.clickable; onTapped: { if (!app.gate(3, "hint " + hint.k)) return; app.note("action", "Legend " + hint.k + " (" + hint.label + ")"); if (hint.k === "r3") app.r3Click(); else app.act(hint.k) } }
        HoverHandler { enabled: hint.clickable; onHoveredChanged: app.hover(hint, hovered) }
        Label { visible: hint.pre !== "" && !hint.named; text: hint.pre; font.pixelSize: app.fpx(15); color: c.soft; anchors.verticalCenter: parent.verticalCenter }
        Repeater {
            model: hint.k.split("+")
            Key { required property string modelData; k: modelData; visible: modelData !== ""; anchors.verticalCenter: parent.verticalCenter }
        }
        Rectangle {
            visible: hint.named
            width: namedText.implicitWidth + app.px(22); height: Math.max(app.px(32), app.tpx(26)); radius: app.px(6)
            anchors.verticalCenter: parent.verticalCenter
            color: c.raised; border.color: c.accentLo; border.width: Math.max(1, app.px(1))
            Text { id: namedText; anchors.centerIn: parent; text: (hint.pre !== "" ? hint.pre + " " : "") + hint.label; color: c.text; font.family: app.bodyFont; font.pixelSize: app.fpx(14) }
        }
        Label { visible: hint.label !== "" && !hint.named; text: hint.label + (hint.k2 !== "" ? "," : ""); font.pixelSize: app.fpx(15); color: c.soft; anchors.verticalCenter: parent.verticalCenter }
        Key { visible: hint.k2 !== ""; k: hint.k2; anchors.verticalCenter: parent.verticalCenter }
        Label { visible: hint.label2 !== ""; text: hint.label2; font.pixelSize: app.fpx(15); color: c.soft; anchors.verticalCenter: parent.verticalCenter }
    }
    // The page edges on Get mods: the page to the left and the page to the right, each a button for a
    // click or a finger (at least 44 px wide).
    component PageEdge: Rectangle {
        id: edge
        property string label: ""
        property int dir: -1
        width: app.tpx(34); height: parent.height
        z: app.turnTo === 1 ? 3 : 0
        transform: Translate { x: app.turnTo === 1 ? app.turnX : 0 }
        color: edgeHover.hovered ? c.focusFill : c.panel
        Rectangle { x: edge.dir < 0 ? parent.width - width : 0; width: Math.max(1, app.px(1)); height: parent.height; color: c.line }
        Text {
            anchors.centerIn: parent
            rotation: edge.dir < 0 ? -90 : 90
            text: edge.label
            color: edge.dir < 0 ? c.accentText : c.soft
            font.family: app.bodyFont; font.pixelSize: app.fpx(14); font.weight: Font.DemiBold
        }
        // The arrow, pointing off the page.
        Shape {
            width: app.px(14); height: app.px(18)
            anchors.horizontalCenter: parent.horizontalCenter
            y: parent.height * 0.76
            preferredRendererType: Shape.CurveRenderer
            ShapePath {
                strokeWidth: 0; strokeColor: "transparent"
                fillColor: edge.dir < 0 ? c.accentText : c.soft
                startX: edge.dir < 0 ? app.px(14) : 0; startY: 0
                PathLine { x: edge.dir < 0 ? 0 : app.px(14); y: app.px(9) }
                PathLine { x: edge.dir < 0 ? app.px(14) : 0; y: app.px(18) }
                PathLine { x: edge.dir < 0 ? app.px(14) : 0; y: 0 }
            }
        }
        Accessible.role: Accessible.Button
        Accessible.name: edge.label + (edge.dir < 0 ? ", the page to the left" : ", the page to the right")
        HoverHandler { id: edgeHover }
        TapHandler { onTapped: { if (!app.gate(0, "page edge")) return; app.note("touch", "Page edge: " + edge.label); app.turnPage(edge.dir) } }
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
    Row {
        id: mainRow
        anchors.fill: parent
        // 0.59: off while anything covers it (see topLayer).
        enabled: app.topLayer === 0

        // ===== Rail: the logo, the games (last played first, portrait art), Settings pinned at the bottom.
        Rectangle {
            id: rail
            // 0.57: full screen while scrolling down (the bottom bar goes too).
            // 0.62: and on Get mods (a page of the book: no game list).
            width: app.railAway ? 0 : app.px(72)
            Behavior on width { enabled: mo.moves; NumberAnimation { duration: mo.moveBase; easing.type: mo.enter } }
            visible: width > 0
            clip: true
            height: parent.height
            color: c.rail
            z: 5
            Column {
                id: railTop
                anchors.top: parent.top
                anchors.topMargin: app.px(10)
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: app.px(10)
                // 0.59: no app icon here (the owner's room for the games); Turbine shows itself once, at start (splash),
                // and in Settings, About.
                Rectangle { width: app.px(28); height: app.px(2); radius: 1; color: c.line; anchors.horizontalCenter: parent.horizontalCenter }
            }
            // A tile scrolled half under the divider fades out instead of showing a cut edge.
            Rectangle {
                z: 2
                anchors.top: railList.top
                width: parent.width; height: app.px(12)
                visible: !railList.atYBeginning
                gradient: Gradient {
                    GradientStop { position: 0; color: c.rail }
                    GradientStop { position: 1; color: Qt.rgba(c.rail.r, c.rail.g, c.rail.b, 0) }
                }
            }
            ListView {
                id: railList
                anchors.top: railTop.bottom
                anchors.topMargin: app.px(10)
                // 0.6 (I19): the hidden-games tile sits after the last game, just above the gear, where its focus is.
                anchors.bottom: hiddenTile.visible ? hiddenTile.top : gear.top
                anchors.bottomMargin: app.px(10)
                width: parent.width
                clip: true
                spacing: app.px(8)
                model: app.games.length
                currentIndex: app.zone === "rail" ? Math.min(app.railIndex, app.games.length - 1) : app.current
                highlightFollowsCurrentItem: false
                onCurrentIndexChanged: app.glide(railList, () => railList.positionViewAtIndex(railList.currentIndex, ListView.Contain))
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
                        // Only the focus ring outlines a tile; the game on screen has the brass bar on its left.
                        border.width: tileItem.focused ? app.px(2) : 0
                        border.color: c.text
                        clip: true
                        Image {
                            id: art
                            anchors.fill: parent
                            anchors.margins: tile.border.width
                            visible: status === Image.Ready
                            opacity: status === Image.Ready ? 1 : 0
                            Behavior on opacity { enabled: mo.on; NumberAnimation { duration: mo.base; easing.type: mo.enter } }
                            source: tileItem.g ? app.fileUrl(tileItem.g.cover || tileItem.g.header) : ""
                            fillMode: Image.PreserveAspectCrop
                            sourceSize.height: app.px(132)
                            asynchronous: true
                        }
                        // The game's name on its art, so the rail reads without the label beside it.
                        Rectangle {
                            visible: art.visible
                            anchors.left: parent.left; anchors.right: parent.right; anchors.bottom: parent.bottom
                            anchors.margins: tile.border.width
                            height: artName.implicitHeight + app.px(4)
                            color: "#C8000000"
                            Text {
                                id: artName
                                anchors.centerIn: parent
                                width: parent.width - app.px(4)
                                horizontalAlignment: Text.AlignHCenter
                                wrapMode: Text.WordWrap
                                maximumLineCount: 2
                                elide: Text.ElideRight
                                text: tileItem.g ? app.shortGame(tileItem.g.name) : ""
                                color: "#FFFFFF"
                                font.family: app.bodyFont
                                font.pixelSize: Math.max(7, app.px(8))
                                font.bold: true
                            }
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
                            font.pixelSize: app.fpx(10)
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
                            font.pixelSize: app.fpx(8)
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
                    TapHandler { onTapped: { if (!app.gate(0, "railIndex")) return; { app.railIndex = tileItem.index; app.railSelect(tileItem.index) } } }
                    // Right-click (LT with the pointer): the game's options, like X.
                    TapHandler { acceptedButtons: Qt.RightButton; onTapped: { if (!app.gate(0, "sheet (right)")) return; if (app.sheet === "") { app.zone = "rail"; app.railIndex = tileItem.index; app.openRailMenu() } } }
                }
            }
            // Every game hidden from the rail: one way back, right here.
            Rectangle {
                id: hiddenTile
                visible: app.railExtra > 0
                anchors.bottom: gear.top
                anchors.bottomMargin: app.px(10)
                anchors.horizontalCenter: parent.horizontalCenter
                width: app.px(60); height: app.px(66); radius: app.px(6)
                readonly property bool focused: app.zone === "rail" && app.railIndex === app.games.length
                color: c.raised
                border.width: focused ? app.px(2) : Math.max(1, app.px(1))
                border.color: focused ? c.text : c.line
                Text {
                    anchors.fill: parent
                    anchors.margins: app.px(4)
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                    wrapMode: Text.WordWrap
                    text: "Show hidden games"
                    color: c.text
                    font.family: app.bodyFont
                    font.pixelSize: app.fpx(11)
                    font.bold: true
                }
                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                TapHandler { onTapped: { if (!app.gate(0, "showHiddenGames")) return; app.showHiddenGames() } }
            }
            Rectangle {
                id: gear
                anchors.bottom: parent.bottom
                anchors.bottomMargin: app.px(10)
                anchors.horizontalCenter: parent.horizontalCenter
                width: app.px(44); height: app.px(44); radius: app.px(8)
                readonly property bool focused: app.zone === "rail" && app.railIndex === app.games.length + app.railExtra
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
                TapHandler { onTapped: { if (!app.gate(0, "openSettings")) return; app.openSettings() } }
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
                anchors.bottomMargin: legend.shown ? legend.height + app.px(8) : app.px(8)
                spacing: app.px(8)

                // Header, on every page but the game screen (0.57: no top bar there; the bottom bar has the game's name,
                // its status and Play): the page's name, B Back for touch, and on a mod page Start Play.
                Rectangle {
                    id: header
                    width: parent.width + app.px(28)
                    x: -app.px(14)
                    // 0.6 (I11): no top bar anywhere: the bottom bar names the place, has the result caps and Play.
                    height: 0
                    visible: false
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
                        // For touch and the mouse; with a pad or keys the legend already says B Back.
                        // 0.59 (newbie H1): on a mod page too, so touch has a way back.
                        visible: (app.page !== "game" || app.modPage) && app.lastInput !== "pad" && app.lastInput !== "keys"
                        x: app.px(14)
                        anchors.verticalCenter: parent.verticalCenter
                        width: backRow.implicitWidth + app.px(20); height: app.tpx(44); radius: app.px(6)
                        color: c.raised
                        Hint { id: backRow; anchors.centerIn: parent; k: "b"; label: app.page === "place" ? app.placeBLabel : "Back" }
                        HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                        TapHandler { onTapped: { if (!app.gate(0, "act")) return; app.act("b") } }
                    }
                    Column {
                        x: backBtn.visible ? backBtn.x + backBtn.width + app.px(14) : app.px(16)
                        width: parent.width - x - pagePlayBtn.width - app.px(40) - (resultCaps.visible ? resultCaps.width + app.px(14) : 0)
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: app.px(2)
                        Text {
                            id: headerTitle
                            width: parent.width
                            elide: Text.ElideRight
                            text: app.page === "settings" ? "Settings" : app.page === "perf" ? "Scrolling test"
                                  : app.page === "place" ? "My mods > " + (app.placeMode === "move" ? "Change where it goes" : "Where do these files go?")
                                  : app.modPage ? "Get mods > " + (app.detailsData && app.detailsData.name ? app.detailsData.name : app.detailsItem ? app.detailsItem.name : "")
                                  : app.game ? app.gameTitle : "Turbine Mod Manager"
                            color: c.text
                            font.family: app.page === "game" && app.game && !app.modPage ? app.titleFont : app.bodyFont
                            font.pixelSize: app.fpx(20)
                            font.weight: Font.DemiBold
                            // Basic support: a small outlined chip after the name (never focusable).
                            rightPadding: supportChipBox.visible ? supportChipBox.width + app.px(10) : 0
                            Rectangle {
                                id: supportChipBox
                                visible: app.page === "game" && !app.modPage && app.supportChip !== ""
                                x: Math.min(parent.contentWidth + app.px(10), parent.width - width)
                                anchors.verticalCenter: parent.verticalCenter
                                width: chipWords.implicitWidth + app.px(16); height: app.px(22); radius: height / 2
                                color: "transparent"
                                border.color: c.accentLo
                                Accessible.role: Accessible.StaticText
                                Accessible.name: app.supportChip
                                Text { id: chipWords; anchors.centerIn: parent; text: app.supportChip; color: c.accentText; font.family: app.bodyFont; font.pixelSize: app.fpx(12); font.bold: true }
                            }
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
                                font.pixelSize: app.fpx(14)
                            }
                            // While anything downloads: "Getting 2 mods · 64%"; otherwise updates ready.
                            Rectangle {
                                id: pill
                                readonly property string words: app.dl.active ? "Getting " + app.plural(Math.max(1, Turbine.netBusy), "mod", "mods") + (app.dlPart >= 0 ? " · " + Math.round(app.dlPart * 100) + "%" : "")
                                                                 : !app.browserOpen && app.webDlsRunning > 0 ? "Getting " + app.plural(app.webDlsRunning, "mod", "mods") + (app.webDlPart >= 0 ? " · " + Math.round(app.webDlPart * 100) + "%" : "")
                                                                 : app.statusNow.pill || ""
                                visible: words !== ""
                                anchors.verticalCenter: parent.verticalCenter
                                width: pillText.implicitWidth + app.px(16); height: app.px(22); radius: height / 2
                                color: c.raised
                                border.color: c.accentLo
                                Text { id: pillText; anchors.centerIn: parent; text: pill.words; color: c.accentText; font.family: app.bodyFont; font.pixelSize: app.fpx(13); font.bold: true }
                                // Updates ready: the list of them (a free account picks each one's page there).
                                TapHandler { onTapped: { if (!app.gate(0, "dl")) return; if (!app.dl.active && !app.browserOpen && app.webDlsRunning > 0) app.openNexusDownloads(); else if (app.updateCount > 0 && !app.dl.active) app.openUpdates(); else if (pill.words.indexOf("Loadout: ") === 0) app.openProfiles() } }
                            }
                        }
                    }
                    // The mod page: LB and RB step through the results.
                    Row {
                        id: resultCaps
                        // Only when this mod is one of the results (controls #8).
                        readonly property int at: app.getItems.findIndex(x => app.detailsItem && String(x.id) === String(app.detailsItem.id))
                        visible: app.page === "game" && app.modPage && app.getItems.length > 1 && at >= 0
                        anchors.right: pagePlayBtn.left
                        anchors.rightMargin: app.px(14)
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: app.px(6)
                        Key { k: "lb"; anchors.verticalCenter: parent.verticalCenter }
                        Label { text: "Mod " + (resultCaps.at + 1) + " of " + app.getItems.length; font.pixelSize: app.fpx(14); color: c.soft; anchors.verticalCenter: parent.verticalCenter }
                        Key { k: "rb"; anchors.verticalCenter: parent.verticalCenter }
                    }
                    Rectangle {
                        id: pagePlayBtn
                        visible: app.page === "game" && app.game !== null
                        anchors.right: parent.right
                        anchors.rightMargin: app.px(14)
                        anchors.verticalCenter: parent.verticalCenter
                        width: pagePlayRow.implicitWidth + app.px(24); height: app.px(38); radius: app.px(8)
                        opacity: Turbine.busy ? 0.7 : 1
                        // The focus ring when Play has the focus (a mod just arrived, or Up from the top).
                        border.color: c.text
                        border.width: app.zone === "play" && app.sheet === "" && !app.wheelOpen ? app.px(3) : 0
                        gradient: Gradient {
                            GradientStop { position: 0; color: c.accentHi }
                            GradientStop { position: 1; color: c.accent }
                        }
                        Row {
                            id: pagePlayRow
                            anchors.centerIn: parent
                            spacing: app.px(8)
                            // Play waits for changes being saved: it says so.
                            Text { text: (app.savingNow ? "Play (saving...)" : app.modHold && app.mods.some(m => m.enabled) ? "Play without mods" : "Play") + (app.seamlessIn && !app.savingNow ? (app.lastPlayChoice === "coop" ? " · Co-op" : " · Solo") : ""); color: app.onFill(c.accent); font.family: app.bodyFont; font.pixelSize: app.fpx(16); font.bold: true; anchors.verticalCenter: parent.verticalCenter }
                        }
                        HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                        TapHandler { onTapped: { if (!app.gate(0, "startPlay")) return; app.playPressed() } }
                    }
                }

                // 0.57: no tab row. The bottom bar names the tabs (LB and RB switch them, a tap or a sideways swipe too);
                // My mods' Find, Show, Sort and Select several are in the Y pop-up.
                // Page content
                Item {
                    id: content
                    width: parent.width
                    height: mainCol.height - y
                    clip: true
                    // Seasonal leaves and snow fade out behind the lists and cards (they show in the margins).
                    Rectangle { anchors.fill: parent; color: c.ground; opacity: app.seasonKind !== "" ? 0.85 : 0 }

                    // ---- 0.62: a page turning. The page being covered dims, the one coming in slides over it on its own
                    // ground, and a brass line marks its edge.
                    Rectangle { anchors.fill: parent; z: 1; visible: app.turnTo >= 0 && mo.moves; color: c.ground; opacity: 0.55 }
                    Rectangle { id: turnGround; x: app.turnX; width: parent.width; height: parent.height; z: 2; visible: app.turnTo >= 0 && mo.moves; color: c.ground }
                    // Reduced: the new page fades in from the ground.
                    Rectangle { anchors.fill: parent; z: 6; visible: app.turnFade > 0; color: c.ground; opacity: app.turnFade }
                    Rectangle {
                        id: turnLine
                        z: 4
                        visible: app.turnTo >= 0 && mo.moves
                        x: app.turnDir < 0 ? app.turnX + parent.width - width : app.turnX
                        width: Math.max(2, app.px(3)); height: parent.height
                        color: c.accentHi
                    }
                    // The page edges on Get mods (PageEdge, with the shared pieces).
                    PageEdge { id: edgeBack; visible: app.bookPage; x: 0; label: "My mods"; dir: -1 }
                    PageEdge { id: edgeNext; visible: app.bookPage && app.bookHasNext; x: parent.width - width; label: (app.tabs[2] || "Load order").replace(/ \(\d+\)$/, ""); dir: 1 }

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

                    // My mods: the mods, then the collapsed Removed row (0.57: Find, Show, Sort and Select several are in
                    // the Y pop-up).
                    Item { id: listBarRow; height: 0 }
                    Label {
                        id: moveLine
                        y: listBarRow.height
                        z: app.turnTo === 0 ? 3 : 0
                        transform: Translate { x: app.turnTo === 0 ? app.turnX : 0 }
                        visible: app.page === "game" && app.tab === 0 && (app.moving || app.selecting || app.listFiltered)
                        height: visible ? app.px(26) : 0
                        x: app.px(6)
                        text: app.moving ? "Lower in the list wins when two mods change the same file."
                              : app.selecting ? app.plural(app.tickedCount, "mod", "mods") + " ticked. A ticks, X acts on them, B done."
                              : "Showing " + app.plural(app.shownMods.length, "mod", "mods") + (app.findText !== "" ? " with \"" + app.findText + "\"" : "") + (app.listShow !== "all" ? " · " + app.showNames[app.listShow] : "") + (app.listSort !== "order" ? " · " + app.sortNames2[app.listSort] : "") + " · Y changes it"
                        color: c.soft
                        font.pixelSize: app.fpx(14)
                    }
                    // 0.56: the highlighted mod, on the right of My mods when there is room: its picture, name,
                    // version and maker, what it does, what it wins or loses, and what X offers.
                    Rectangle {
                        id: modPanel
                        readonly property var m: app.menuMod
                        visible: app.showModPanel
                        z: app.turnTo === 0 ? 3 : 0
                        transform: Translate { x: app.turnTo === 0 ? app.turnX : 0 }
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom
                        anchors.topMargin: listBarRow.height + moveLine.height + app.px(4)
                        width: visible ? Math.round(parent.width * 0.36) : 0
                        radius: app.px(8)
                        color: c.panel
                        border.color: c.line; border.width: Math.max(1, app.px(1))
                        Accessible.role: Accessible.Pane
                        Accessible.name: m ? "Details: " + m.name : "No mod highlighted"
                        Column {
                            x: app.px(14); y: app.px(14)
                            width: parent.width - app.px(28)
                            spacing: app.px(8)
                            Rectangle {
                                width: parent.width; height: Math.min(width * 9 / 16, modPanel.height * 0.38); radius: app.px(6)
                                color: c.raised
                                clip: true
                                Image {
                                    id: panelPic
                                    anchors.fill: parent
                                    source: modPanel.m && modPanel.m.picture ? modPanel.m.picture : ""
                                    fillMode: Image.PreserveAspectCrop
                                    asynchronous: true
                                    visible: status === Image.Ready
                                    opacity: status === Image.Ready ? 1 : 0
                                    Behavior on opacity { enabled: mo.on; NumberAnimation { duration: mo.base; easing.type: mo.enter } }
                                }
                                Text {
                                    visible: !panelPic.visible
                                    anchors.centerIn: parent
                                    text: modPanel.m ? (modPanel.m.name || "?").substring(0, 1).toUpperCase() : ""
                                    color: c.muted; font.family: app.bodyFont; font.pixelSize: app.fpx(40); font.bold: true
                                }
                            }
                            Text {
                                width: parent.width; wrapMode: Text.WordWrap; maximumLineCount: 2; elide: Text.ElideRight
                                text: modPanel.m ? modPanel.m.name : ""
                                color: c.text; font.family: app.bodyFont; font.pixelSize: app.fpx(19); font.bold: true
                            }
                            Text {
                                width: parent.width; elide: Text.ElideRight
                                text: app.panelLine(modPanel.m)
                                color: c.soft; font.family: app.bodyFont; font.pixelSize: app.fpx(14)
                            }
                            Text {
                                visible: text !== ""
                                width: parent.width; wrapMode: Text.WordWrap; maximumLineCount: 4; elide: Text.ElideRight
                                text: modPanel.m ? (modPanel.m.summary || "") : ""
                                color: c.text; font.family: app.bodyFont; font.pixelSize: app.fpx(14)
                            }
                            Repeater {
                                model: app.modPanelFacts
                                Text {
                                    required property var modelData
                                    width: parent.width; wrapMode: Text.WordWrap; maximumLineCount: 2; elide: Text.ElideRight
                                    text: modelData.text
                                    color: modelData.level === "warn" ? c.warn : c.accentText
                                    font.family: app.bodyFont; font.pixelSize: app.fpx(14); font.weight: Font.DemiBold
                                }
                            }
                            // 0.6 (I8, the owner's video): Edit settings right on the mod, for a mod with a settings file
                            // (Seamless Co-op: its settings, password first). Right from the list, or a tap; nothing when
                            // the mod has none.
                            Rectangle {
                                id: panelEdit
                                objectName: "panelEdit"
                                readonly property bool foc: app.zone === "panel" && app.sheet === ""
                                visible: app.panelHasSettings
                                width: Math.min(parent.width, panelEditText.implicitWidth + app.px(28)); height: app.tpx(44); radius: app.px(6)
                                color: foc ? c.focusFill : c.raised
                                border.color: foc ? c.accentHi : c.accent; border.width: foc ? app.px(2) : Math.max(1, app.px(1.5))
                                Text { id: panelEditText; anchors.centerIn: parent; text: app.panelSettingsLabel; color: c.text; font.family: app.bodyFont; font.pixelSize: app.fpx(15); font.bold: true }
                                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                TapHandler { onTapped: { if (!app.gate(0, "panel Edit settings")) return; app.zone = "panel"; app.panelEditOpen() } }
                            }
                        }
                        Text {
                            anchors.left: parent.left; anchors.right: parent.right; anchors.bottom: parent.bottom
                            anchors.margins: app.px(14)
                            wrapMode: Text.WordWrap; maximumLineCount: 2; elide: Text.ElideRight
                            text: modPanel.m ? app.glyph("x") + " Options: " + app.optionsLine() : ""
                            color: c.muted; font.family: app.bodyFont; font.pixelSize: app.fpx(13)
                            // A tap opens the options too (0.6, I7: touch has every action).
                            TapHandler { onTapped: { if (!app.gate(0, "panel options")) return; if (app.menuMod) { app.menuIndex = 0; app.sheet = "modmenu" } } }
                        }
                    }
                    ListView {
                        id: modList
                        visible: app.page === "game" && app.game !== null && app.tab === 0 && app.modsLoaded && !app.modPage
                        z: app.turnTo === 0 ? 3 : 0
                        transform: Translate { x: app.turnTo === 0 ? app.turnX : 0 }
                        anchors.fill: parent
                        anchors.rightMargin: modPanel.visible ? modPanel.width + app.px(10) : 0
                        anchors.topMargin: listBarRow.height + moveLine.height
                        // The header sliding away or back changes the list's height: the focused row stays fully in view.
                        onHeightChanged: if (app.modIndex >= 0) positionViewAtIndex(app.modIndex, ListView.Contain)
                        // A touch or wheel scroll hides the bars going down and brings them back going up or at the top.
                        property real lastY: 0
                        onContentYChanged: { if (moving) app.scrollFollow(contentY + topMargin, lastY + topMargin, contentHeight > height + app.px(40)); lastY = contentY }
                        topMargin: app.px(4)
                        bottomMargin: app.px(4)
                        clip: true
                        spacing: app.px(app.desktopLayout ? 3 : 6)
                        model: app.listItems
                        currentIndex: Math.max(0, app.modIndex)
                        highlightFollowsCurrentItem: false
                        onCurrentIndexChanged: app.glide(modList, () => modList.positionViewAtIndex(modList.currentIndex, ListView.Contain))
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
                            // 0.62: just moved (it slides in from where it was) or put back (it fades in).
                            transform: Translate { id: entryShift; y: 0 }
                            NumberAnimation { id: entrySlide; target: entryShift; properties: "y"; to: 0; duration: mo.moveFast; easing.type: mo.enter }
                            NumberAnimation { id: entryFade; target: entry; properties: "opacity"; from: 0; to: 1; duration: mo.base; easing.type: mo.enter }
                            Component.onCompleted: {
                                const n = app.rowNudge
                                if (!n || !entry.m || n.id !== entry.m.id || Date.now() - n.at > 400) return
                                if (n.dir === 0) entryFade.start()
                                else if (mo.moves) { entryShift.y = n.dir * (entry.height + modList.spacing); entrySlide.start() }
                            }
                            height: kind === "mod" ? row.implicitHeight : kind === "held" ? Math.max(app.touchFloor + app.px(8), app.px(52)) : app.px(app.desktopLayout ? 38 : 46)
                            ModRow {
                                id: row
                                visible: entry.kind === "mod"
                                ui: kit
                                // 0.64: rows inside the open Dependencies group sit a little in.
                                x: app.px(entry.modelData.dep ? 24 : 4)
                                width: parent.width - app.px(entry.modelData.dep ? 28 : 8)
                                name: entry.m.name || ""
                                detail: entry.kind === "mod" ? app.rowLine(entry.m).text : ""
                                detailLevel: entry.kind === "mod" ? app.rowLine(entry.m).level : ""
                                isOn: entry.m.enabled === true
                                ordered: app.ordered && app.listSort === "order"
                                compact: app.desktopLayout || look.compactList
                                order: entry.kind === "mod" ? app.mods.findIndex(x => x.id === entry.m.id) + 1 : 0
                                position: entry.index + 1
                                count: app.shownMods.length
                                ticked: app.selecting && entry.kind === "mod" ? (app.ticked[entry.m.id] ? 1 : 0) : -1
                                aLabel: app.selecting ? (app.ticked[entry.m.id] ? "Untick" : "Tick") : (entry.m.enabled === true ? "Turn off" : "Turn on")
                                xLabel: app.selecting ? (app.tickedCount > 0 ? "Act on " + app.tickedCount : "") : "Options"
                                focused: entry.foc
                                // Problems are the row's second line (rowLine); the badge is for Moving, an update and New.
                                badge: app.moving && entry.foc ? "Moving" : app.updateMap[entry.m.id] && !app.updateMap[entry.m.id].deprecated ? "Update ready"
                                       : entry.kind === "mod" && app.statusOf(entry.m) !== "" ? app.statusOf(entry.m)
                                       : entry.kind === "mod" && app.isNew(entry.m) ? "New" : ""
                                badgeLevel: entry.kind === "mod" && !app.moving && app.statusOf(entry.m) !== "" && !(app.updateMap[entry.m.id] && !app.updateMap[entry.m.id].deprecated) ? "warn" : "info"
                                // The legend says what A and X do: nothing printed twice.
                                showHints: false
                                // A click or tap anywhere on the row is A on it (turns it on or off; ticks it in Select several).
                                onClicked: { if (!app.gate(0, "onClicked")) return; if (app.moving && entry.kind === "mod") { app.moveByTap(entry.index); return } if (entry.kind === "mod" && !app.selecting && app.rowClickSeen(entry.index, entry.m)) return; app.rowA(() => { if (!app.moving) app.modIndex = entry.index }) }
                                // A right-click (LT with the pointer) opens the mod's menu, like X.
                                onMenuClicked: { if (!app.gate(0, "onMenuClicked")) return; if (app.sheet === "" && !app.moving) { app.modIndex = entry.index; app.zone = "main"; app.sheet = "modmenu" } }
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
                                    width: parent.width - app.px(28) - (heldButton.visible ? heldButton.width + app.px(12) : 0)
                                    elide: Text.ElideRight
                                    text: entry.kind === "ghost" ? "Removed: " + entry.m.name + ". A puts it back."
                                          : entry.kind === "held" ? app.heldWords
                                          : entry.kind === "deps" ? "Dependencies (" + app.depMods.length + ") · what your mods need" + (app.depsOpen ? "" : " · A opens it")
                                          : entry.kind === "needsq" && app.reqQ ? app.reqQ.forName + " still needs " + app.plural(app.reqQ.items.length - app.reqQ.index, "mod", "mods") + " · A goes on where you left off"
                                          : "Removed mods (" + app.removedMods.length + ") · kept 7 days"
                                    color: entry.kind === "ghost" ? c.soft : entry.kind === "held" || entry.kind === "needsq" ? c.warn : entry.kind === "deps" ? c.soft : c.muted
                                    font.pixelSize: app.fpx(15)
                                    font.italic: entry.kind === "ghost"
                                }
                                // The held note's way back, on the row itself (a tap anywhere on the row does it too).
                                Rectangle {
                                    id: heldButton
                                    visible: entry.kind === "held"
                                    anchors.right: parent.right
                                    anchors.rightMargin: app.px(6)
                                    anchors.verticalCenter: parent.verticalCenter
                                    height: parent.height - app.px(8)
                                    width: heldButtonText.implicitWidth + app.px(24)
                                    radius: app.px(6)
                                    color: c.accent
                                    Label {
                                        id: heldButtonText
                                        anchors.centerIn: parent
                                        text: "Turn them back on"
                                        color: c.onAccent
                                        font.pixelSize: app.fpx(15)
                                        font.bold: true
                                    }
                                }
                                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                TapHandler { onTapped: { if (!app.gate(0, "rowA")) return; app.rowA(() => { app.modIndex = entry.index }) } }
                            }
                        }
                    }

                    // Get mods: a game with no source can still add a file.
                    EmptyState {
                        ui: kit
                        anchors.fill: parent
                        visible: app.page === "game" && app.game !== null && app.tab === 1 && app.sources.length === 0
                        title: "Get mods"
                        body: app.nexusUnknown ? "Turbine couldn't find " + app.gameTitle + " on Nexus Mods. Pick it yourself, or add a mod file (X)."
                              : "This game isn't on Nexus Mods or Thunderstore. You can still add a mod file."
                        buttonKey: "a"
                        buttonLabel: app.nexusUnknown ? "Pick the Nexus game" : "Add from Downloads"
                        buttonFocused: app.zone === "main"
                        onActivated: { if (!app.gate(0, "onActivated")) return; app.nexusUnknown ? app.openNexusPick() : app.openAdd() }
                    }
                    // 0.57: an online-only anti-cheat game: no mods, and why.
                    EmptyState {
                        ui: kit
                        anchors.fill: parent
                        visible: app.page === "game" && app.acRefused && app.tab === 0 && app.mods.length === 0 && !app.modPage
                        title: app.gameTitle + " needs its anti-cheat"
                        body: app.acInfo ? app.acInfo.refuseWords : ""
                        buttonKey: ""
                        buttonLabel: ""
                    }
                    // 0.58: an empty My mods (any game): a centered card with a big Get mods (A) and Add a file (X).
                    Rectangle {
                        id: emptyCard
                        visible: app.emptyMods
                        z: app.turnTo === 0 ? 3 : 0
                        transform: Translate { x: app.turnTo === 0 ? app.turnX : 0 }
                        anchors.centerIn: parent
                        width: Math.min(parent.width - app.px(32), app.px(520))
                        height: emptyCol.implicitHeight + app.px(48)
                        radius: app.px(12)
                        color: c.card
                        border.color: c.line
                        Column {
                            id: emptyCol
                            anchors.centerIn: parent
                            width: parent.width - app.px(48)
                            spacing: app.px(14)
                            Text {
                                width: parent.width; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.Wrap
                                text: "No mods for " + app.gameTitle + " yet"
                                color: c.text; font.family: app.bodyFont; font.pixelSize: app.fpx(22); font.bold: true
                            }
                            Text {
                                width: parent.width; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.Wrap
                                text: app.canGetMods ? "Get mods finds them for you, or add a mod file you downloaded." : "Add a mod file you downloaded: zip, 7z or rar."
                                color: c.soft; font.family: app.bodyFont; font.pixelSize: app.fpx(15)
                            }
                            Flow {
                                anchors.horizontalCenter: parent.horizontalCenter
                                width: Math.min(parent.width, emptyGet.width + emptyAdd.width + spacing + 1)
                                spacing: app.px(14)
                                Rectangle {
                                    id: emptyGet
                                    visible: app.canGetMods
                                    width: emptyGetRow.implicitWidth + app.px(36); height: Math.max(app.px(52), app.tpx(44)); radius: app.px(8)
                                    color: app.zone === "main" ? c.focusFill : c.raised
                                    border.color: c.accent; border.width: app.px(app.zone === "main" ? 2 : 1)
                                    Row { id: emptyGetRow; anchors.centerIn: parent; spacing: app.px(10)
                                        Key { k: "a"; anchors.verticalCenter: parent.verticalCenter }
                                        Text { text: "Get mods"; color: c.text; font.family: app.bodyFont; font.pixelSize: app.fpx(18); font.bold: true; anchors.verticalCenter: parent.verticalCenter } }
                                    Accessible.role: Accessible.Button; Accessible.name: "Get mods"
                                    HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                    TapHandler { onTapped: { if (!app.gate(0, "note")) return; { app.note("action", "No mods yet: Get mods (tap)"); app.tab = 1 } } }
                                }
                                Rectangle {
                                    id: emptyAdd
                                    width: emptyAddRow.implicitWidth + app.px(36); height: Math.max(app.px(52), app.tpx(44)); radius: app.px(8)
                                    color: !app.canGetMods && app.zone === "main" ? c.focusFill : c.raised
                                    border.color: c.line; border.width: Math.max(1, app.px(1))
                                    Row { id: emptyAddRow; anchors.centerIn: parent; spacing: app.px(10)
                                        Key { k: app.canGetMods ? "x" : "a"; anchors.verticalCenter: parent.verticalCenter }
                                        Text { text: "Add from Downloads"; color: c.text; font.family: app.bodyFont; font.pixelSize: app.fpx(18); font.bold: true; anchors.verticalCenter: parent.verticalCenter } }
                                    Accessible.role: Accessible.Button; Accessible.name: "Add from Downloads"
                                    HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                    TapHandler { onTapped: { if (!app.gate(0, "note")) return; { app.note("action", "No mods yet: Add a file (tap)"); app.openAdd() } } }
                                }
                            }
                        }
                    }
                    // A Basic game's first visit with no mods: what Basic support means.
                    EmptyState {
                        ui: kit
                        anchors.fill: parent
                        visible: app.basicIntro && !app.modPage
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
                        // 0.62: between the page edges (My mods on the left; Load order on the right where the game has one).
                        anchors.leftMargin: app.tpx(34) + app.px(6)
                        anchors.rightMargin: app.bookHasNext ? app.tpx(34) + app.px(6) : 0
                        visible: app.page === "game" && app.game !== null && (app.tab === 1 || (app.turnFrom === 1 && mo.moves)) && app.sources.length > 0 && !app.modPage
                        z: app.turnTo === 1 ? 3 : 0
                        transform: Translate { x: app.turnTo === 1 ? app.turnX : 0 }
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
                        search: app.getText
                        category: app.getCategory
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
                        onHelpClicked: (i) => { if (!app.gate(0, "onHelpClicked")) return; app.pageInline = i; app.modPageAct("a") }
                        onInstallClicked: { if (!app.gate(0, "onInstallClicked")) return; app.pageIndex = 0; app.pageInline = -1; app.installFromDetails() }
                        onVersionsClicked: { if (!app.gate(0, "onVersionsClicked")) return; app.pageIndex = 1; app.openVersions() }
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
                        visible: app.page === "game" && app.game !== null && (app.tab === 2 || (app.turnFrom === 2 && mo.moves)) && !app.modPage
                        z: app.turnTo === 2 ? 3 : 0
                        transform: Translate { x: app.turnTo === 2 ? app.turnX : 0 }
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
                                    Label { x: app.px(14); anchors.verticalCenter: parent.verticalCenter; text: parent.modelData; font.pixelSize: app.fpx(16); font.bold: app.setSection === parent.index; color: app.setSection === parent.index ? c.text : c.muted }
                                    HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                    TapHandler { onTapped: { if (!app.gate(0, "setSection")) return; { app.setSection = parent.index; app.setRow = 0 } } }
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
                            onCurrentIndexChanged: app.glide(lookList, () => lookList.positionViewAtIndex(lookList.currentIndex, ListView.Contain))
                            delegate: Rectangle {
                                id: srow
                                required property int index
                                required property var modelData
                                readonly property bool foc: app.zone === "settings" && app.setRow === index
                                readonly property var now: srow.modelData.now !== undefined ? srow.modelData.now : look[srow.modelData.key]
                                // For the build's size check.
                                readonly property Item minus: minusBtn
                                readonly property Item plus: plusBtn
                                width: lookList.width
                                height: srow.modelData.slider ? Math.max(app.px(62), app.tpx(36) + app.fpx(13) + app.px(16)) : app.px(50)
                                radius: app.px(6)
                                color: foc ? c.focusFill : c.card
                                border.color: c.accent
                                border.width: foc ? app.px(2) : 0
                                opacity: srow.modelData.dim ? 0.6 : 1
                                Label { x: app.px(14); width: app.px(140); anchors.verticalCenter: parent.verticalCenter; text: srow.modelData.label; font.pixelSize: app.fpx(16); font.bold: true }
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
                                                Behavior on color { enabled: mo.on; ColorAnimation { duration: mo.fast; easing.type: mo.enter } }
                                                Rectangle { visible: parent.on; anchors.bottom: parent.bottom; width: parent.width; height: app.px(2); color: c.accent }
                                                Label { anchors.centerIn: parent; text: parent.modelData; font.pixelSize: app.fpx(14); font.bold: parent.on; color: parent.on ? c.text : c.muted }
                                                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                                TapHandler {
                                                    onTapped: {
                                                        if (!app.gate(0, "look choice " + srow.modelData.key) || !app.tapOnce()) return
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
                                                TapHandler { onTapped: { if (!app.gate(0, "tapOnce")) return; { if (!app.tapOnce()) return; app.setRow = srow.index; look.accent = parent.modelData; app.note("action", "Look: accent " + look.accent) } } }
                                            }
                                        }
                                        Label { text: look.accent.charAt(0).toUpperCase() + look.accent.substring(1); color: c.soft; font.pixelSize: app.fpx(14); anchors.verticalCenter: parent.verticalCenter }
                                    }
                                    // Sliders: Interface size, Seasonal amount and size. Minus and plus at the ends, and a line
                                    // under it saying Left and Right change it (A steps up, then wraps back down).
                                    Row {
                                        id: sliderRow
                                        visible: srow.modelData.slider === true
                                        anchors.verticalCenter: parent.verticalCenter
                                        anchors.verticalCenterOffset: -app.px(8)
                                        width: parent.width
                                        spacing: app.px(8)
                                        // 0.58: real buttons, at least 44 pixels, for a tap, a click or the pointer.
                                        Rectangle {
                                            id: minusBtn
                                            objectName: "sizeMinus"
                                            width: app.tpx(36); height: width; radius: app.px(6)
                                            anchors.verticalCenter: parent.verticalCenter
                                            color: c.raised; border.color: c.line; border.width: Math.max(1, app.px(1))
                                            Label { anchors.centerIn: parent; text: "\u2212"; color: c.text; font.pixelSize: app.fpx(22); font.bold: true }
                                            Accessible.role: Accessible.Button
                                            Accessible.name: srow.modelData.label + ": smaller"
                                            HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                            TapHandler { onTapped: { if (!app.gate(0, "tapOnce")) return; { if (!app.tapOnce()) return; app.setRow = srow.index; app.zone = "settings"; app.note("action", srow.modelData.label + ": - tapped"); srow.modelData.step(-1) } } }
                                        }
                                        Rectangle {
                                            id: track
                                            readonly property real part: srow.modelData.slider ? (srow.now - srow.modelData.min) / (srow.modelData.max - srow.modelData.min) : 0
                                            width: Math.max(app.px(40), parent.width - app.tpx(36) * 2 - app.px(90)); height: app.px(6); radius: height / 2
                                            anchors.verticalCenter: parent.verticalCenter
                                            color: c.line
                                            // 0.62: the slider's fill and knob glide to the new value.
                                            Rectangle {
                                                width: parent.width * track.part; height: parent.height; radius: parent.radius; color: c.accent
                                                Behavior on width { enabled: mo.moves; NumberAnimation { duration: mo.moveFast; easing.type: mo.enter } }
                                            }
                                            Rectangle {
                                                x: parent.width * track.part - width / 2
                                                Behavior on x { enabled: mo.moves; NumberAnimation { duration: mo.moveFast; easing.type: mo.enter } }
                                                anchors.verticalCenter: parent.verticalCenter
                                                width: app.px(18); height: width; radius: width / 2; color: c.accentHi
                                            }
                                        }
                                        Rectangle {
                                            id: plusBtn
                                            objectName: "sizePlus"
                                            width: app.tpx(36); height: width; radius: app.px(6)
                                            anchors.verticalCenter: parent.verticalCenter
                                            color: c.raised; border.color: c.line; border.width: Math.max(1, app.px(1))
                                            Label { anchors.centerIn: parent; text: "+"; color: c.text; font.pixelSize: app.fpx(22); font.bold: true }
                                            Accessible.role: Accessible.Button
                                            Accessible.name: srow.modelData.label + ": bigger"
                                            HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                            TapHandler { onTapped: { if (!app.gate(0, "tapOnce")) return; { if (!app.tapOnce()) return; app.setRow = srow.index; app.zone = "settings"; app.note("action", srow.modelData.label + ": + tapped"); srow.modelData.step(1) } } }
                                        }
                                        Label { text: srow.now + (srow.modelData.unit || ""); color: c.accentText; font.bold: true; anchors.verticalCenter: parent.verticalCenter }
                                    }
                                    Label {
                                        visible: srow.modelData.slider === true
                                        anchors.top: sliderRow.bottom
                                        anchors.topMargin: app.px(2)
                                        text: "Left smaller · Right bigger"
                                        color: c.muted
                                        font.pixelSize: app.fpx(13)
                                    }
                                }
                                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                TapHandler { onTapped: { if (!app.gate(0, "rowA")) return; app.rowA(() => { app.setRow = srow.index; app.zone = "settings" }, true) } }
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
                                    TapHandler { onTapped: { if (!app.gate(0, "setRow")) return; { app.setRow = 0; app.labsTap() } } }
                                }
                                Column {
                                    anchors.verticalCenter: parent.verticalCenter
                                    spacing: app.px(4)
                                    Label { text: "Turbine Mod Manager " + Turbine.version; font.pixelSize: app.fpx(19); font.weight: Font.DemiBold }
                                    Label { visible: app.labsOn; text: "Labs mode on"; color: c.accentText; font.pixelSize: app.fpx(15); font.bold: true }
                                }
                            }
                            // 0.62: only in Game Mode; in Desktop Mode no controller is normal and nothing is said.
                            Label { visible: !Turbine.pads && Turbine.gamemode; width: parent.width; wrapMode: Text.WordWrap; elide: Text.ElideNone; color: c.muted; font.pixelSize: app.fpx(14); text: "Using Steam's controller settings (no controller seen directly)." }
                            Label { width: parent.width; wrapMode: Text.WordWrap; elide: Text.ElideNone; color: c.soft; text: "A mod manager for SteamOS handhelds. Free software under the GNU General Public License, version 3 or later. Licenses of the parts it uses are in the licenses folder of the project." }
                            // 0.6 (I26): a test for the build's makers, so only with Labs on.
                            Rectangle {
                                visible: app.labsOn
                                readonly property bool foc: app.zone === "settings" && app.setRow === 1
                                width: perfRow.implicitWidth + app.px(28); height: app.px(40); radius: app.px(8)
                                color: foc ? c.focusFill : c.card
                                border.color: c.accent
                                border.width: foc ? app.px(2) : 0
                                Hint { id: perfRow; anchors.centerIn: parent; k: "a"; label: "Scrolling test (2,000 rows)" }
                                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                TapHandler { onTapped: { if (!app.gate(0, "page")) return; { app.page = "perf"; app.zone = "perf" } } }
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
                            // After the left stick's wheel: the focus comes into view (a row with only words is passed).
                            onMovementEnded: {
                                const top = indexAt(10, contentY + app.px(10)), bottom = indexAt(10, contentY + height - app.px(10))
                                if (top >= 0 && app.setRow < top) app.setRow = app.settingsLand(top, 1, app.rowsNow.length)
                                else if (bottom >= 0 && app.setRow > bottom) app.setRow = app.settingsLand(bottom, -1, app.rowsNow.length)
                            }
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
                                    font.pixelSize: app.fpx(14)
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
                                        Label { width: parent.width; text: lrow.modelData.title; font.pixelSize: app.fpx(16); font.bold: true }
                                        Label { width: parent.width; text: lrow.modelData.how; color: c.soft; font.pixelSize: app.fpx(14); wrapMode: Text.WordWrap; maximumLineCount: 3; elide: Text.ElideRight }
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
                                                    font.pixelSize: app.fpx(14)
                                                    font.bold: parent.on
                                                    color: !parent.on ? c.muted : !lrow.isItem ? c.text : parent.index === 0 ? c.ok : parent.index === 1 ? c.stop : c.text
                                                }
                                                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                                TapHandler {
                                                    onTapped: {
                                                        if (!app.gate(0, "labs choice") || !app.tapOnce()) return
                                                        app.setRow = lrow.index
                                                        if (lrow.isItem) { app.labsChip = parent.index; app.labsChoose(lrow.modelData, parent.modelData[0]) }
                                                        else if (!parent.on) app.rowStep(lrow.modelData, lrow.modelData.key === "zoom" && parent.index === 0 ? -1 : 1)
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                                    TapHandler { onTapped: { if (!app.gate(0, "settingsFocusable")) return; { if (app.settingsFocusable(lrow.index)) app.rowA(() => { app.setRow = lrow.index; app.zone = "settings" }, true) } } }
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
                        highlightMoveDuration: mo.moveFast
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
                                    Label { anchors.centerIn: parent; text: prow.off ? "OFF" : "ON"; font.pixelSize: app.fpx(14); font.bold: true; color: prow.off ? c.muted : c.accentText }
                                }
                                Column {
                                    anchors.verticalCenter: parent.verticalCenter
                                    Label { text: "Test mod " + (prow.index + 1); font.pixelSize: app.fpx(16); font.bold: true; color: prow.off ? c.muted : c.text }
                                    Label { text: "Row " + (prow.index + 1) + " of 2,000"; font.pixelSize: app.fpx(14); color: c.muted }
                                }
                            }
                        }
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
        sourceComponent: Component {
            NexusBrowser {
                ui: kit
                live: app.browserLive
                startUrl: app.browserStart
                gameName: app.game ? app.gameTitle : ""
                gameIcon: app.game && (app.game.cover || app.game.header) ? app.fileUrl(app.game.cover || app.game.header) : ""
                zoom: app.browserZoom
                tab: app.browserTab
                downloads: app.browserRows
                dlIndex: app.browserDlIndex
                pillText: app.browserPill.text
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
    // 0.59: every sheet, menu and prompt in one layer: off under the quick wheel and the tutorial (see topLayer).
    Item {
        id: sheetLayer
        // 0.6: every sheet and menu ends above the bottom bar, so its last row is never under it (150%, 1024 wide).
        anchors.fill: parent
        anchors.bottomMargin: legend.visible ? legend.height : 0
        enabled: app.topLayer <= 2
        Overlay {
            visible: app.sheet === "details" || app.sheet === "notice"
            onOutsideTapped: { if (!app.gate(2, "sheet onOutsideTapped")) return; app.act("b") }
            Rectangle { anchors.fill: parent; color: "#B0000000" }

            // Game details: what Turbine found, so the owner can check detection on the device.
            Rectangle {
                visible: app.sheet === "details" && app.game !== null
                anchors.centerIn: parent
                width: Math.min(parent.width - app.px(40), app.px(600))
                height: detailCol.implicitHeight + app.px(32)
                radius: app.px(10)
                color: c.panel
                border.color: c.line
                CardGuard {}
                Column {
                    id: detailCol
                    x: app.px(18); y: app.px(16)
                    width: parent.width - app.px(36)
                    spacing: app.px(8)
                    Label { text: app.game ? app.game.name : ""; font.pixelSize: app.fpx(19); font.weight: Font.DemiBold; width: parent.width }
                    Repeater {
                        model: app.game ? [
                            ["Store", app.game.storeLabel + (app.game.launcher !== "Steam" ? " (through Heroic)" : "")],
                            ["Runs as", app.runtimeWords(app.game)],
                            ["Stored on", (app.game.launcher === "Steam" ? "Steam library, " : "") + app.game.drive],
                            ["Folder", String(app.game.installDir || "").split("/").filter(x => x !== "").slice(-1)[0] || ""],
                            // A Steam build number means nothing to people; the number stays for reports, in Labs.
                            // 0.59 (T31): "Reported by Steam" said nothing: a Steam game's build is for reports (Labs), so no row.
                            ["Game version", app.game.launcher === "Steam" ? "" : (app.game.build || "Not reported")],
                        ].concat(app.labsOn ? [["Store ID", app.game.storeId], ["Build", app.game.build || "none"], ["Full folder", app.game.installDir]] : []).concat([

                            ["Cover art", app.game.cover || app.game.header ? "Found on the device" : "None cached"],
                            ["Support", !app.rules ? "" : app.rules.supportLevel === "basic" ? "Basic support. Turbine has no rules of its own for this game yet. It puts each mod where its files show they belong, or asks you once."
                                        : app.rules.supportLevel === "custom" ? "Added by you (from your games folder)" : "Full support"]
                        ]).concat(app.rules && app.rules.supportLevel !== "full" ? [
                            ["Nexus game", app.rules.domain ? (app.rules.nexusName || app.rules.domain) + (app.rules.nexusHow ? " (" + app.rules.nexusHow + ")" : "") : app.rules.domainState === "none" ? "Not on Nexus (your pick)" : "Not found yet"]
                        ] : []).concat(app.rules && app.rules.antiCheat ? [
                            ["Anti-cheat", app.rules.antiCheat.name + ". Mods only in offline or single player modes."]
                        ] : []).concat(app.checkPaths.length > 0 ? [
                            ["Also checks", app.checkPaths.join("; ")]
                        ] : []).concat(app.rules && app.rules.launchOption ? [
                            ["Loader setup", app.rules.launchHelp || ""],
                            ["Launch Option", app.rules.launchOption]
                        ] : []) : []
                        Row {
                            required property var modelData
                            visible: modelData[1] !== ""
                            width: detailCol.width
                            spacing: app.px(12)
                            Label { width: app.px(110); text: parent.modelData[0]; color: c.muted; font.pixelSize: app.fpx(14) }
                            Label { width: parent.width - app.px(122); text: parent.modelData[1]; wrapMode: Text.WrapAnywhere; elide: Text.ElideNone; font.pixelSize: app.fpx(15) }
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
                CardGuard {}
                Column {
                    id: noticeCol
                    x: app.px(20); y: app.px(18)
                    width: parent.width - app.px(40)
                    spacing: app.px(10)
                    Label { text: "Made for SteamOS"; font.pixelSize: app.fpx(19); font.weight: Font.DemiBold }
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
            onActivated: (index) => { if (!app.gate(2, "sheet onActivated")) return; app.menuIndex = index; app.modMenuRun(index) }
            onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
        }
        GameMenu {
            id: gameMenuView
            ui: kit
            visible: app.sheet === "gamemenu"
            gameName: app.gameTitle
            items: app.gameMenuItems.map(m => ({ key: m.act || "", label: m.title, detail: m.detail, icon: m.icon, danger: m.danger === true, enabled: m.ok !== false, group: m.group, more: m.act === "restore" && app.setups.length > 1 }))
            focusIndex: app.menuIndex
            // Reset runs only while a finger or the pointer stays down on it (a tap says to hold).
            onActivated: (index, key) => { if (!app.gate(2, "sheet onActivated")) return; app.menuIndex = index; if (key !== "reset" || !resetHold.running) app.gameMenuRun(index) }
            onHeld: (index, down) => {
                const it = app.gameMenuItems[index]
                if (!it || it.act !== "reset") return
                app.menuIndex = index
                app.clickHeld = down
                if (down && !resetHold.running && !app.pressTwice) { app.resetHeld = 0; resetHold.start() }
                else if (!down) app.stopReset()
            }
            onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
        }
        AddMods {
            ui: kit
            visible: app.sheet === "add"
            subtitle: app.addAll ? "Every mod file in Downloads and its Game Mods folders, newest first"
                                 : "Mod files in Downloads and Game Mods/" + app.gameTitle + ", newest first"
            files: app.addList.map(d => ({ name: d.name, size: app.sizeWords(d.size), age: (d.modified > 0 ? app.age(d.modified) : "") + (app.addAll && d.folder ? " · " + d.folder : "") }))
            focusIndex: app.addIndex
            emptyText: app.addAll || app.downloads.length === 0 ? "No mod files in Downloads yet. Get one with Get mods, or download a mod's zip file in Desktop Mode." : "None for " + app.gameTitle + ". Find, All files lists the rest."
            otherLabel: app.downloads.length === 0 ? "" : "Find"
            onOtherClicked: { if (!app.gate(2, "sheet onOtherClicked")) return; app.act("y") }
            laterText: ""
            onActivated: (index) => { if (!app.gate(2, "sheet onActivated")) return; app.addIndex = index; app.addChosen() }
            onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
        }
        PrePlayCheck {
            id: checkView
            ui: kit
            visible: app.sheet === "check"
            checks: app.checks
            buttons: app.checkChoices.map(label => ({ key: "", label: label, main: !app.checkStops && label === "Play anyway" }))
            focusIndex: app.checkIndex
            note: app.checkStops ? "Play without mods starts the game with no mods. Nothing is deleted." : ""
            onActivated: (index, key) => { if (!app.gate(2, "sheet onActivated")) return; app.checkIndex = index; app.checkChoose(index) }
            onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
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
            onActivated: (index, key) => { if (!app.gate(2, "sheet onActivated")) return; app.loaderIndex = index; app.act(key || "a") }
            onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
        }

        // Installer choices: over the main area, the rail stays in view.
        FomodSheet {
            id: fomodView
            ui: kit
            visible: app.sheet === "fomod" && app.fomod !== null
            x: rail.width
            width: parent.width - rail.width
            height: parent.height - app.px(8)
            info: app.fomod
            onChoose: (g, o) => { if (!app.gate(2, "sheet onChoose")) return; app.note("action", "Installer: question " + (g + 1) + ", option " + (o + 1)); Turbine.fomodChoose(g, o) }
            onNext: { if (!app.gate(2, "sheet onNext")) return; app.note("action", "Installer: next step"); Turbine.fomodNext() }
            onBack: { if (!app.gate(2, "sheet onBack")) return; app.note("action", "Installer: back a step"); Turbine.fomodBack() }
            onStop: { if (!app.gate(2, "sheet onStop")) return; app.note("action", "Installer: stopped"); Turbine.fomodCancel() }
        }
        VersionsSheet {
            ui: kit
            visible: app.sheet === "versions"
            modName: app.detailsData && app.detailsData.name ? app.detailsData.name : ""
            versions: app.otherVersions
            focusIndex: app.versionIndex
            note: app.detailsData && app.detailsData.versionsNote ? app.detailsData.versionsNote : ""
            onActivated: (i) => { if (!app.gate(2, "sheet onActivated")) return; app.versionIndex = i; app.installVersion(i) }
            onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
        }
        PagesSheet {
            ui: kit
            visible: app.sheet === "pages"
            title: app.pagesTitle
            pages: app.pagesList
            focusIndex: app.pagesIndex
            gameMode: Turbine.gamemode
            handlerSet: app.handler.state !== "no"
            onActivated: (i) => { if (!app.gate(2, "sheet onActivated")) return; app.pagesIndex = i; app.openPage(i) }
            onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
        }
        CodeSheet {
            ui: kit
            visible: app.sheet === "code"
            info: app.codeData
            onCopyClicked: { if (!app.gate(2, "sheet onCopyClicked")) return; app.act("a") }
            onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
        }
        PluginSheet {
            ui: kit
            visible: app.sheet === "plugin" && app.orderPlugin !== null
            plugin: app.orderPlugin
            position: app.orderIndex + 1
            notes: app.orderPlugin ? (app.orderMessages[app.orderPlugin.name.toLowerCase()] || []) : []
            onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
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
            onActivated: (i) => { if (!app.gate(2, "sheet onActivated")) return; app.profIndex = i; app.profileRun(i) }
            onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
        }
        AddMods {
            ui: kit
            visible: app.sheet === "shares"
            title: "Open a loadout file"
            subtitle: "Shared loadout files in your Downloads folder, newest first"
            files: app.shares.map(x => ({ name: x.name, size: "", age: "" }))
            focusIndex: app.shareIndex
            actionLabel: "Read"
            emptyText: "No loadout files in Downloads. Someone makes one with Loadouts, Save to a loadout file."
            laterText: ""
            onActivated: (index) => { if (!app.gate(2, "sheet onActivated")) return; app.shareIndex = index; app.act("a") }
            onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
        }
        // The no-browser warning, the browser helper's steps, install notes, "Add this file?" and Add Chrome to Steam.
        InfoSheet {
            id: infoView
            ui: kit
            visible: ["nobrowser", "helper", "notes", "found", "quick", "chain", "addbrowser", "oldhelper", "msg"].indexOf(app.sheet) >= 0
            title: app.sheet === "quick" && app.quick ? app.quick.name + " closed after " + app.plural(app.quick.seconds, "second", "seconds")
                   : app.sheet === "chain" ? "Next: " + (app.chainQueue[0] ? app.chainQueue[0].name : "")
                   : app.sheet === "nobrowser" ? (app.browser.title || "No web browser found")
                   : app.sheet === "helper" ? "Set up the browser helper"
                   : app.sheet === "addbrowser" && app.addMode === "helper" ? "Set up the browser helper too"
                   : app.sheet === "addbrowser" ? "Add Chrome to Steam so it opens in Game Mode"
                   : app.sheet === "found" ? "Found a download"
                   : app.sheet === "oldhelper" ? "Your browser helper is out of date"
                   : app.sheet === "msg" && app.msgData ? app.msgData.title
                   : "Added " + app.notesName
            text: app.sheet === "quick" ? app.quickText
                  : app.sheet === "chain" ? (app.chainAfter ? app.chainAfter + " is in. " : "") + "It also needs " + (app.chainQueue[0] ? app.chainQueue[0].name : "") + ". " + (app.chainQueue[0] && app.chainQueue[0].modId > 0 ? "A opens its file in Turbine. Press Slow download; it's added too."
                                        : "A opens its files on Nexus. Pick the top file under Main files, press Manual download, then Slow download. It's added too.")
                  : app.sheet === "nobrowser" ? (app.browser.text || "")
                  : app.sheet === "addbrowser" && app.addMode === "helper" ? "Turbine started " + app.openName + " through Steam. Without the browser helper it opens on its start page, not on the mod. The helper opens the mod's page for you and saves the download in the right game's folder."
                  : app.sheet === "addbrowser" ? "Game Mode only shows apps Steam starts. Add your browser to Steam once, and Turbine opens Nexus in it right over Turbine. Once:"
                  : app.sheet === "helper" ? "Optional. Skips the Save box and puts each download in the right game's folder. Turbine put it in Downloads, in the folder Turbine browser helper. Then, once:"
                  : app.sheet === "found" && app.found ? "Found " + app.found.name + " (" + app.sizeWords(app.found.size) + ")."
                        + (app.found.game ? " Add it to " + app.found.gameName + "?" : " Which game is it for?")
                  : app.sheet === "notes" ? "Something to know about this mod:"
                  : app.sheet === "oldhelper" ? "In Chrome, open chrome://extensions and press the reload arrow on Turbine browser helper. Turbine already put the new version in its folder. After this once, the helper updates by itself."
                  : app.sheet === "msg" && app.msgData ? (app.msgData.text || "")
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
                   : app.sheet === "msg" && app.msgData ? (app.msgData.steps || [])
                   : []
            notes: app.sheet === "notes" ? app.notesList : app.sheet === "msg" && app.msgData ? (app.msgData.notes || []) : []
            footnote: app.sheet === "msg" && app.msgData ? (app.msgData.footnote || "")
                      : app.sheet === "helper" ? "Then press Pair: the pairing page opens in the browser and links the helper to Turbine."
                      : app.sheet === "addbrowser" && app.addMode !== "helper" ? (app.bridge.paired ? "Until then, Steam's browser opens Nexus instead." : "Also set up the browser helper (Settings, Getting mods): without it the browser opens, but not on the mod's page. Until then, Steam's browser opens Nexus instead.")
                      : app.sheet === "addbrowser" ? "Steam's browser opens the mod's page without the helper."
                      : app.sheet === "oldhelper" ? "Until then, Nexus pages open in Steam's browser." + (app.bridge.helper ? " Helper " + app.bridge.helper + ", Turbine's " + (app.bridge.bundled || "") + "." : "")
                      : app.sheet === "found" && app.found ? (app.found.game ? "Why " + app.found.gameName + ": " + app.found.why + ". " : "")
                        + "The file stays in Downloads either way." + (app.found.more > 0 ? " " + app.plural(app.found.more, "more file waits", "more files wait") + " after this one." : "")
                      : ""
            buttons: app.infoButtons
            focusIndex: app.infoIndex
            onActivated: (i) => { if (!app.gate(2, "sheet onActivated")) return; app.infoIndex = i; app.infoRun(i) }
            onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
        }
        // Removed mods (A Put back, X Delete now) and the hold-to-confirm sheet for deleting for good.
        RemovedSheet {
            ui: kit
            visible: app.sheet === "removed"
            mods: app.removedMods
            focusIndex: app.removedIndex
            now: app.today
            onActivated: (i) => { if (!app.gate(2, "sheet onActivated")) return; app.removedIndex = i; app.removedRun(i) }
            onDeleteClicked: (i) => { if (!app.gate(2, "sheet onDeleteClicked")) return; app.removedIndex = i; app.removedDelete(i) }
            onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
        }
        InfoSheet {
            id: confirmSheet
            ui: kit
            visible: app.sheet === "confirm"
            title: app.confirmVerb === "delete" ? "Delete for good?" : app.confirmVerb === "remove it" ? "Remove the leftover file?" : "Replace your saves?"
            text: app.confirmText
            buttons: [{ key: "", label: app.pressTwice ? (app.armedKey === app.deleteKey() ? "Press A again to " + app.confirmVerb : "Press A twice to " + app.confirmVerb)
                                     : confirmHold.running ? "Keep holding A: " + Math.max(1, Math.ceil(app.holdNeeded - app.confirmHeld)) + "..." : "Hold A to " + app.confirmVerb },
                      { key: "", label: "Keep" }]
            focusIndex: app.confirmIndex
            // Hold mode: a tap on Delete only moves the highlight; holding a finger or the pointer on it counts like
            // holding A. Press twice mode: a tap counts like a press of A.
            onActivated: (i) => { if (!app.gate(2, "sheet onActivated")) return
                if (i !== 0) { app.confirmIndex = i; app.act("b"); return }
                if (app.pressTwice) { app.confirmIndex = 0; app.startConfirm(); return }
                app.confirmIndex = i
                if (!confirmHold.running) app.holdHint(app.holdNeeded + " seconds to " + app.confirmWords())
            }
            onHeld: (i, down) => {
                if (i !== 0 || app.pressTwice) return
                app.confirmIndex = 0
                app.clickHeld = down
                if (down) app.startConfirm(); else app.stopConfirm()
            }
            onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
        }
        // X on a Get mods card.
        ModMenu {
            ui: kit
            visible: app.sheet === "cardmenu"
            title: app.cardItem ? app.cardItem.name : ""
            subtitle: app.cardItem ? (app.cardItem.owner || "") : ""
            items: app.cardMenuItems.map(m => ({ label: m.title, detail: m.detail, icon: m.icon, danger: false }))
            focusIndex: app.menuIndex
            onActivated: (index) => { if (!app.gate(2, "sheet onActivated")) return; app.menuIndex = index; app.cardMenuRun(index) }
            onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
        }
        // Y's pop-up: Search, Sort and Category on Get mods; Find, Show, Sort and Select several on My mods.
        FindSheet {
            id: findView
            ui: kit
            visible: app.sheet === "findpop"
            title: app.findPop.kind === "get" ? "Find mods for " + app.gameTitle : "Find in my mods"
            subtitle: app.findPop.kind === "get" ? app.nexusLine : ""
            fieldLabel: app.findPop.kind === "get" ? "Search" : "Find"
            text: app.findPop.text
            placeholder: app.findPop.kind === "get" ? "Search " + app.gameTitle + " mods" : "Part of a mod's name"
            result: app.findResult
            sections: app.findSections
            focusRow: app.findRow
            focusCol: app.findCol
            onFieldClicked: { if (!app.gate(2, "sheet onFieldClicked")) return; app.findPopField() }
            onTyped: (text) => app.findPopTyping(text)
            onEntered: (text) => app.findPopEntered(text)
            onDoneClicked: { if (!app.gate(2, "sheet onDoneClicked")) return; app.findPopDone() }
            onClearClicked: { if (!app.gate(2, "sheet onClearClicked")) return; app.findPopClear() }
            onOptionClicked: (r, c) => { if (!app.gate(2, "sheet onOptionClicked")) return; app.findRow = r; app.findCol = c; app.findPopChoose(r, c) }
            onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.findPopDone() }
        }
        // A small menu made on the spot: Play options (hold Start), a profile's Rename and Delete, Select several's actions.
        ModMenu {
            id: pickMenu
            ui: kit
            visible: app.sheet === "pick"
            title: app.pickTitle
            subtitle: app.pickSub
            items: app.pickItems.map(m => ({ label: m.title, detail: m.detail, icon: m.icon, danger: m.danger === true, enabled: m.dim !== true }))
            focusIndex: app.menuIndex
            onActivated: (index) => { if (!app.gate(2, "sheet onActivated")) return
                if (app.nxmAsking) nxmRingMove.stop()   // a tap is the player's choice: the ring stays (break M1)
                // The Mod Manager Download question: nothing for its first second, and a tap (or two, or a click) on Get
                // it only picks it: only A gets it, so a page can't get a mod by asking for taps where the row appears.
                if (app.nxmAsking && (app.nxmAskQuiet() || index === 0)) {
                    if (!app.nxmAskQuiet()) app.menuIndex = 0
                    app.note("nxm", "Tap on the question: " + (app.nxmAskQuiet() ? "ignored (it just opened)" : "Get it picked; only A gets it"))
                    // Said on screen too (newbie M3): touch alone can't get it, on purpose.
                    if (!app.nxmAskQuiet()) toast.info("Get it is picked. Press A to get it: a tap only picks it, so a page can never get a mod by itself.")
                    return
                }
                app.menuIndex = index; app.pickRun(index)
            }
            onClosed: { if (!app.gate(2, "sheet onClosed")) return; if (app.nxmAskQuiet()) return; app.act("b") }
        }
        // The focused game's name beside the rail, up in the header (never over the tabs), and until a game has been
        // opened from the rail once, how to pick one.
        Rectangle {
            id: railLabel
            readonly property bool on: app.zone === "rail" && app.railIndex < app.games.length
            readonly property bool hint: !seen.railHintDone || shot.page === "rail"
            visible: on && app.sheet === ""
            z: 20
            x: rail.width + app.px(4)
            y: app.px(6)
            width: Math.max(railLabelText.implicitWidth, railHintText.visible ? railHintText.implicitWidth : 0) + app.px(24)
            height: railLabelText.implicitHeight + (railHintText.visible ? railHintText.implicitHeight + app.px(2) : 0) + app.px(12)
            radius: app.px(6)
            color: c.raised
            border.color: c.accent
            border.width: Math.max(1, app.px(1))
            Column {
                anchors.centerIn: parent
                spacing: app.px(2)
                Text { id: railLabelText; anchors.horizontalCenter: parent.horizontalCenter; text: app.railName(app.railIndex); color: c.text; font.family: app.bodyFont; font.pixelSize: app.fpx(15); font.weight: Font.DemiBold }
                Text { id: railHintText; visible: railLabel.hint; anchors.horizontalCenter: parent.horizontalCenter; text: "Up and Down pick a game, A opens it"; color: c.soft; font.family: app.bodyFont; font.pixelSize: app.fpx(13) }
            }
        }
        // X on "Found <file>": which game it goes into.
        GamePicker {
            ui: kit
            visible: app.sheet === "pickgame"
            fileName: app.found ? app.found.name : ""
            games: app.games
            focusIndex: app.pickIndex
            onActivated: (i) => { if (!app.gate(2, "sheet onActivated")) return; app.pickIndex = i; app.pickGame(i) }
            onClosed: { if (!app.gate(2, "sheet onClosed")) return; app.act("b") }
        }
        AskText {
            id: askView
            ui: kit
            // 0.59: drawn only while it is the sheet, so another sheet can never open under it.
            visible: shown && app.sheet === "ask"
            focusIndex: app.askIndex
            onAccepted: (text) => app.askDone(text)
            onEdited: (text) => { if (app.askPurpose === "find") { app.findText = text; app.modIndex = 0 } }
            onCancelled: app.askCancelled()
            onFieldTapped: { app.askIndex = -1; app.askKeyboard() }
        }

        // 0.58: Edit settings (a mod's config files, the game's own, and Seamless Co-op's).
        SettingsEditor {
            id: settingsView
            ui: kit
            visible: app.sheet === "settingsedit"
            onSaveClicked: { if (!app.gate(2, "sheet onSaveClicked")) return; app.settingsSaveNow() }
            onResetClicked: { if (!app.gate(2, "sheet onResetClicked")) return; app.settingsResetAsk() }
            onOptionsClicked: { if (!app.gate(2, "sheet onOptionsClicked")) return; app.openSettingsOptions() }
            onCloseClicked: {
                if (!app.gate(2, "sheet onCloseClicked")) return
                // Anything still waiting to save goes in first: B never loses a change.
                app.settingsLeaving = true
                if (changes > 0) app.settingsSaveNow(true)
                app.settingsLeaving = false
                if (app.coopAfterSettings) { app.coopAfterSettings = false; toast.info("Co-op waits for a session password.") }
                app.note("action", "Edit settings closed"); app.sheet = app.settingsReturn
            }
            // 0.62: the Co-op tile's row, on Seamless Co-op's own settings page.
            extraRows: app.sheet === "settingsedit" && app.seamlessMod && app.settingsMod === app.seamlessMod.id && app.coopTileOffered ? [{
                key: "cooptile", title: "Co-op tile in Steam",
                detail: app.coopTileThere ? "In your Steam library: it starts " + app.gameTitle + " with Seamless Co-op in one press."
                                          : "A tile in your Steam library that starts " + app.gameTitle + " with Seamless Co-op in one press" + (app.coopTileHow !== "" ? " (" + app.coopTileHow + ")." : "."),
                label: app.coopTileThere ? "Remove" : "Add" }] : []
            onActionClicked: (key) => { if (!app.gate(2, "sheet onActionClicked")) return
                if (key !== "cooptile") return
                // Anything still waiting to save goes in first, then the tile's own question.
                app.settingsLeaving = true
                if (changes > 0) app.settingsSaveNow(true)
                app.settingsLeaving = false
                app.coopAfterSettings = false
                app.note("action", "Co-op settings: the Co-op tile row")
                app.openCoopTile()
            }
            onTypeAsked: (row, value, label) => { if (!app.gate(2, "sheet onTypeAsked")) return
                app.settingsTypeRow = row
                const help = row.kind === "line" ? "The whole line, as it is in the file." : (row.f.help || "") + (row.f.kind === "number" ? " A number." : "")
                app.openAsk("settingvalue", label, help.trim(), value, label, 2000, false, "Done")
            }
        }
        ActivityLog {
            id: activityView
            ui: kit
            visible: app.sheet === "activity"
            lines: app.activityLines
            onCopyClicked: { if (!app.gate(2, "sheet onCopyClicked")) return
                Turbine.copyText(app.activityLines.slice(-500).join("\n") + "\n")
                toast.showPlain("Copied the last " + Math.min(500, app.activityLines.length) + " lines of the activity log.")
            }
            onSaveClicked: { if (!app.gate(2, "sheet onSaveClicked")) return
                const r = app.parse(Turbine.saveActivityLog(), { error: "Couldn't save it." })
                toast.showPlain(r.error ? r.error : "Saved to Downloads as " + r.file + ".")
            }
            onCloseClicked: { if (!app.gate(2, "sheet onCloseClicked")) return; app.sheet = "" }
        }
        PerfCheck {
            id: perfView
            ui: kit
            win: app
            onCopyClicked: (text) => { if (!app.gate(2, "sheet onCopyClicked")) return; Turbine.copyText(text); toast.showPlain("Performance check copied.") }
            onReportClicked: (text) => { if (!app.gate(2, "sheet onReportClicked")) return; app.openReport(null, "Performance check", text) }
            onCloseClicked: { if (!app.gate(2, "sheet onCloseClicked")) return; perfView.close(); app.sheet = "" }
            Component.onCompleted: if (shot.page === "perfcheck") reportText = "Turbine performance check\nTurbine: 0.2 (build sample)\nVerdict: Smooth\n"
                + "Still: 60 fps, median 16.7 ms, worst 18.9 ms, spikes 0.0%, CPU 9%\nScrolling: 59 fps, median 16.7 ms, worst 24.0 ms, spikes 0.4%, CPU 31%\n"
                + "Idle: 0 frames in 4.0 s, CPU 0.2%\n\nStartup: first frame 640 ms after start\nDrawing: Vulkan, software drawing no"
        }
        LabsReport {
            id: reportView
            ui: kit
            onSendClicked: { if (!app.gate(2, "sheet onSendClicked")) return; app.sendReport() }
            onCloseClicked: { if (!app.gate(2, "sheet onCloseClicked")) return; app.dismissReport() }
            onEditStarted: app.reportEditing(true)
            onEditEnded: app.reportEditing(false)
        }
    }

    // The bottom bar (0.57: the only bar on the game screen). Left to right: the game's name with the tab and its
    // status under it (a warning there is tappable), LB RB and the tabs, then the legend: what binds() says A, B, X
    // and Y do here and one more (the pointer's buttons, or Start when a sheet covers Play), at most five, then View
    // Wheel, and Start Play on the right. Elsewhere it is the legend and View alone. After a touch the button hints
    // hide until the next press (the game's name, the tabs and Play stay). Scrolling down hides it with the rail.
    // Declared after the menus so their backdrop never dims it.
    readonly property var legendHints: legendFor(binds())
    // 0.6 (I11): the bar keeps one shape on every game page, the mod page included (Play bottom right).
    readonly property bool barGame: page === "game" && game !== null
    // Where the player is, for the bar's left when it isn't a game screen (Settings, Where do these files go?).
    // On a mod page, its place in the results (LB and RB step through them).
    readonly property int pageResultAt: modPage && detailsItem ? getItems.findIndex(x => String(x.id) === String(detailsItem.id)) : -1
    readonly property string pageResultWords: pageResultAt >= 0 && getItems.length > 1 ? " · Mod " + (pageResultAt + 1) + " of " + getItems.length : ""
    readonly property string barPlace: page === "settings" ? "Settings" : page === "place" ? (placeMode === "move" ? "Change where it goes" : "Where do these files go?") : page === "perf" ? "Scrolling test" : ""
    // "My mods · Basic support · 2 mods on", or the problem in the status's own color.
    // The chip comes last, so a long line cuts it before the status words.
    // 0.59 (T19): no tab name (the lit tab is right beside it), so the status itself has the room.
    readonly property string barStatus: (statusNow.level === "ok" ? (statusNow.detail || statusNow.words) : statusNow.words) + (pill.words !== "" ? " · " + pill.words : "")
        + (supportChip !== "" ? " · " + supportChip : "")
    // Where LB or RB goes when there is no room for every tab's name: RB to the next tab, or LB back from the last.
    readonly property int barTarget: tabs.length < 2 ? -1 : tab < tabs.length - 1 ? tab + 1 : tab - 1
    readonly property bool barTappable: statusNow.level === "warn" || statusNow.level === "stop" || pill.words !== ""
    // A tap on the status line: what is waiting (downloads, updates, the loadout), else why there is a problem.
    function statusTap() {
        note("action", "Status tapped: " + barStatus)
        if (modHold && mods.some(m => m.enabled)) { openHoldChoice(); return }
        if (!dl.active && !browserOpen && webDlsRunning > 0) openNexusDownloads()
        else if (updateCount > 0 && !dl.active && statusNow.level === "ok") openUpdates()
        else if (pill.words.indexOf("Loadout: ") === 0 && statusNow.level === "ok") openProfiles()
        else if (statusNow.level === "warn" || statusNow.level === "stop") openWhy()
    }
    Rectangle {
        id: legend
        // Shown unless the content went full screen (a sheet or the wheel always brings it back).
        readonly property bool shown: visible && !(app.barsHidden && app.sheet === "" && !app.wheelOpen)
        anchors.bottom: parent.bottom
        anchors.bottomMargin: shown ? 0 : -height - app.px(2)
        Behavior on anchors.bottomMargin { enabled: mo.moves; NumberAnimation { duration: mo.moveBase; easing.type: mo.enter } }
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: rail.width
        // 0.58: a taller bar with room between its groups.
        height: Math.max(app.px(60), app.tpx(30) + app.px(10), barGameCol.implicitHeight + app.px(12))
        color: c.panel
        // Over the quick wheel's backdrop too (the wheel is z 70), so it never dims.
        z: app.wheelOpen ? 80 : 40
        // The Nexus page has its legend in its own bar.
        // 0.6 (I7): the bar stays for touch too: its hints are the touch buttons.
        visible: app.sheet !== "tutorial" && (app.barGame || app.page !== "browser" || app.wheelOpen || app.sheet !== "")
        // 0.59: on up to the quick wheel (its Close is here); the tutorial covers it.
        enabled: app.topLayer <= 3
        // The bar takes every press on it, so a tap on the bar never reaches a sheet's backdrop or a button beneath
        // (one tap on a hint is one press of that button, like the controller).
        MouseArea {
            anchors.fill: parent
            z: -1
            acceptedButtons: Qt.AllButtons
            preventStealing: true
            onWheel: (w) => { w.accepted = true }
            onPressed: app.coverPress(3)
            onReleased: app.coverEnd()
            onCanceled: app.coverEnd()
        }
        Rectangle { width: parent.width; height: Math.max(1, app.px(1)); color: c.line }
        readonly property real gap: app.px(18)
        readonly property real edge: app.px(16)
        // Touch: a sideways swipe on the bar is LB or RB.
        DragHandler {
            enabled: app.barGame && app.sheet === "" && !app.wheelOpen
            target: null
            xAxis.enabled: true; yAxis.enabled: false
            acceptedDevices: PointerDevice.TouchScreen
            onActiveChanged: if (!active && Math.abs(activeTranslation.x) > app.px(60)) { app.note("touch", "Swipe on the bottom bar"); app.switchTab(activeTranslation.x < 0 ? 1 : -1) }
        }
        // ---- Left: the game (its whole name, wrapping to two lines before it is ever cut) and the tabs.
        Row {
            id: barRow
            x: legend.edge
            height: parent.height
            spacing: legend.gap
            Column {
                id: barGameCol
                visible: app.barGame || app.barPlace !== ""
                // Measured from the words themselves (never from the Texts' own sizes, which follow this width).
                TextMetrics { id: barNameMetrics; font: barName.font; text: barName.text }
                TextMetrics { id: barSubMetrics; font: barSub.font; text: barSub.text }
                // The name gets up to 34% of the bar; past that it wraps onto a second line.
                readonly property real most: Math.max(app.px(150), legend.width * 0.34)
                readonly property real want: Math.min(most, Math.ceil(Math.max(barNameMetrics.advanceWidth, Math.min(barSubMetrics.advanceWidth, most))) + app.px(2))
                width: visible ? want : 0
                anchors.verticalCenter: parent.verticalCenter
                Text {
                    id: barName
                    width: barGameCol.want
                    wrapMode: Text.Wrap
                    maximumLineCount: 2
                    elide: Text.ElideRight
                    text: app.barPlace !== "" ? app.barPlace : app.game ? app.shortGame(app.game.name) : ""
                    color: c.accentText; font.family: app.bodyFont; font.pixelSize: app.fpx(15); font.bold: true
                }
                Text {
                    id: barSub
                    width: barGameCol.want; elide: Text.ElideRight
                    // On a mod page: where it is ("Get mods > SkyUI", "Mod 3 of 30"); Settings: the section.
                    text: app.page === "settings" ? app.sec : app.page === "place" ? (app.placeData && app.placeData.pending ? app.placeData.pending.name : "")
                          : app.modPage ? "Get mods > " + (app.detailsData && app.detailsData.name ? app.detailsData.name : app.detailsItem ? app.detailsItem.name : "") + app.pageResultWords : app.barStatus
                    color: app.statusNow.level === "stop" ? c.stop : app.statusNow.level === "warn" ? c.warn : c.soft
                    font.family: app.bodyFont; font.pixelSize: app.fpx(12)
                    font.underline: app.barTappable && (app.statusNow.level === "warn" || app.statusNow.level === "stop")
                }
                Accessible.role: app.barTappable ? Accessible.Button : Accessible.StaticText
                Accessible.name: barName.text + ", " + app.barStatus
                HoverHandler { enabled: app.barTappable && app.sheet === "" && !app.wheelOpen; onHoveredChanged: app.hover(parent, hovered) }
                TapHandler { enabled: app.barTappable && app.sheet === "" && !app.wheelOpen; onTapped: app.statusTap() }
            }
            Rectangle { visible: app.barGame; width: Math.max(1, app.px(1)); height: app.px(34); color: c.line; anchors.verticalCenter: parent.verticalCenter }
            // LB RB and the tabs (the one on screen brighter); a tap on a name opens that tab.
            Row {
                id: barTabs
                // On a mod page LB and RB step through the results (the line on the left says which).
                visible: app.barGame && !app.modPage
                anchors.verticalCenter: parent.verticalCenter
                // 0.63: after the mouse the pages are buttons by name (My mods, Get mods, Load order), no caps.
                spacing: app.mouseLegend ? app.px(22) : app.px(6)
                leftPadding: app.mouseLegend ? app.px(10) : 0
                rightPadding: leftPadding
                // Dimmed where they do nothing (one tab, or a sheet open).
                Key { id: barLb; k: "lb"; opacity: app.tabs.length < 2 || app.sheet !== "" || app.tab === 0 ? 0.35 : 1; anchors.verticalCenter: parent.verticalCenter }
                Key { id: barRb; k: "rb"; opacity: app.tabs.length < 2 || app.sheet !== "" || app.tab >= app.tabs.length - 1 ? 0.35 : 1; anchors.verticalCenter: parent.verticalCenter }
                // No room for every name: only the tab LB or RB goes to (and nothing when even that doesn't fit).
                Text {
                    visible: !legend.tabWordsFit && legend.tabShortFits && app.barTarget >= 0 && !app.mouseLegend
                    text: app.barTarget >= 0 ? (app.barTarget < app.tab ? "back to " : "") + app.tabs[app.barTarget].replace(/ \(\d+\)$/, "") : ""
                    color: c.soft; font.family: app.bodyFont; font.pixelSize: app.fpx(14)
                    anchors.verticalCenter: parent.verticalCenter
                    TapHandler { enabled: app.sheet === "" && !app.wheelOpen; onTapped: { app.note("touch", "Tab tapped: " + app.tabs[app.barTarget]); app.modPage = false; app.tab = app.barTarget } }
                }
                Repeater {
                    model: legend.tabWordsFit || app.mouseLegend ? app.tabs : []
                    Row {
                        required property int index
                        required property string modelData
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: app.px(6)
                        Text { visible: parent.index > 0 && !app.mouseLegend; text: "/"; color: c.muted; font.family: app.bodyFont; font.pixelSize: app.fpx(14); anchors.verticalCenter: parent.verticalCenter }
                        Text {
                            Rectangle {
                                visible: app.mouseLegend
                                z: -1
                                anchors.centerIn: parent
                                width: parent.width + app.px(20); height: Math.max(app.px(32), app.tpx(26)); radius: app.px(6)
                                color: app.tab === parent.parent.index ? c.focusFill : c.raised
                                border.color: app.tab === parent.parent.index ? c.accent : c.accentLo; border.width: Math.max(1, app.px(1))
                            }
                            text: parent.modelData.replace(/ \(\d+\)$/, "")
                            color: app.tab === parent.index ? c.text : c.muted
                            font.family: app.bodyFont; font.pixelSize: app.fpx(14); font.bold: app.tab === parent.index
                            anchors.verticalCenter: parent.verticalCenter
                            Accessible.role: Accessible.PageTab
                            Accessible.name: text + (app.tab === parent.index ? ", selected" : "") + ", " + (parent.index + 1) + " of " + app.tabs.length
                            HoverHandler { enabled: app.sheet === "" && !app.wheelOpen; onHoveredChanged: app.hover(parent, hovered) }
                            TapHandler { enabled: app.sheet === "" && !app.wheelOpen; onTapped: { const i = parent.parent.index; app.note("touch", "Tab tapped: " + app.tabs[i]); app.modPage = false; app.tab = i } }
                        }
                    }
                }
            }
            Rectangle { visible: app.barGame; width: Math.max(1, app.px(1)); height: app.px(34); color: c.line; anchors.verticalCenter: parent.verticalCenter }
        }
        // ---- Right: Play (and Seamless Co-op where it is on), always whole; the wheel's button just before them.
        Row {
            id: playGroup
            anchors.right: parent.right
            anchors.rightMargin: legend.edge
            anchors.verticalCenter: parent.verticalCenter
            spacing: app.px(10)
            // 0.58: Play Seamless Co-op, next to Play, where Seamless Co-op is installed.
            Rectangle {
                id: coopBtn
                // 0.6 (I3): co-op is chosen at Play; the separate button is gone.
                visible: false
                width: coopText.implicitWidth + app.px(24); height: Math.max(app.px(40), app.tpx(30)); radius: app.px(6)
                color: c.raised; border.color: c.accent; border.width: Math.max(1, app.px(1.5))
                opacity: Turbine.busy ? 0.55 : 1
                Text { id: coopText; anchors.centerIn: parent; text: "Co-op"; color: c.text; font.family: app.bodyFont; font.pixelSize: app.fpx(15); font.bold: true }
                Accessible.role: Accessible.Button
                Accessible.name: "Play Seamless Co-op"
                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                TapHandler { onTapped: app.startCoop() }
            }
            // 0.62: what Start does: a tap is Settings, a 3 second hold plays (or shows the Play choice). A tap on
            // it is a tap of Start. The words go when the bar has no room for them; the cap stays.
            Rectangle {
                id: startHint
                visible: app.barGame && app.sheet === "" && !app.wheelOpen
                anchors.verticalCenter: parent.verticalCenter
                width: startHintRow.implicitWidth + app.px(16); height: Math.max(app.px(34), app.tpx(26)); radius: app.px(6)
                color: c.raised
                Row {
                    id: startHintRow
                    anchors.centerIn: parent
                    spacing: app.px(6)
                    // 0.63: with the keyboard Settings is Esc, on a game's first page (elsewhere Esc goes back, so no cap).
                    Key { id: startHintKey; k: app.keysLegend ? "b" : "start"; off: app.keysLegend && !app.escTop; anchors.verticalCenter: parent.verticalCenter }
                    Column {
                        visible: legend.startWords || startHintKey.gone
                        anchors.verticalCenter: parent.verticalCenter
                        Text { text: "Settings"; color: c.soft; font.family: app.bodyFont; font.pixelSize: app.fpx(13) }
                        Text { visible: !app.mouseLegend; text: app.startHoldWords; color: c.muted; font.family: app.bodyFont; font.pixelSize: app.fpx(12) }
                    }
                }
                Accessible.role: Accessible.Button
                Accessible.name: "Settings. Hold Start for 3 seconds: " + (app.startHoldPlays ? "Play" : "Play choice")
                HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                TapHandler { onTapped: { if (!app.gate(3, "Start hint")) return; app.note("touch", "Start hint tapped"); app.startTap() } }
            }
            Rectangle {
                id: playBtn
                // A sheet whose own Start button is in the legend ("START Save"): Play steps aside.
                visible: app.barGame && !(app.sheet !== "" && app.legendHints.some(h => h[0] === "start"))
                width: playRow.implicitWidth + app.px(28); height: Math.max(app.px(40), app.tpx(30)); radius: app.px(6)
                // Dimmed while a sheet or the wheel is open.
                opacity: Turbine.busy || app.sheet !== "" || app.wheelOpen ? 0.55 : 1
                // The focus ring when Play has the focus (a mod just arrived).
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
                    // Play waits for changes being saved: it says so.
                    Text { text: (app.savingNow ? "Play (saving...)" : app.modHold && app.mods.some(m => m.enabled) ? "Play without mods" : "Play") + (app.seamlessIn && !app.savingNow ? (app.lastPlayChoice === "coop" ? " · Co-op" : " · Solo") : ""); color: app.onFill(c.accent); font.family: app.bodyFont; font.pixelSize: app.fpx(16); font.bold: true; anchors.verticalCenter: parent.verticalCenter }
                }
                // 0.62: Start held: a brass ring fills here over 3 seconds (shown once the press is clearly a hold).
                Rectangle {
                    id: startRing
                    readonly property bool shown: app.startHeld > 0.1
                    visible: shown
                    anchors.centerIn: parent
                    width: parent.height - app.px(6); height: width; radius: width / 2
                    color: c.ground
                    Shape {
                        anchors.fill: parent
                        preferredRendererType: Shape.CurveRenderer
                        ShapePath {
                            strokeColor: c.accentHi
                            strokeWidth: Math.max(2, app.px(4))
                            fillColor: "transparent"
                            capStyle: ShapePath.RoundCap
                            PathAngleArc {
                                centerX: startRing.width / 2; centerY: startRing.height / 2
                                radiusX: startRing.width / 2 - app.px(5); radiusY: radiusX
                                startAngle: -90
                                sweepAngle: 360 * app.startRingPart
                            }
                        }
                    }
                }
                Accessible.role: Accessible.Button
                Accessible.name: "Play"
                HoverHandler { enabled: app.sheet === "" && !app.wheelOpen; onHoveredChanged: app.hover(parent, hovered) }
                // 0.6 (I22): a long press is Play options (the wheel's Play options slot shows it too).
                TapHandler { enabled: app.sheet === "" && !app.wheelOpen; onTapped: { if (!app.gate(3, "Play")) return; app.playPressed() }
                             onLongPressed: { if (!app.gate(3, "Play")) return; app.note("touch", "Play held: Play options"); app.openPlayOptions() } }
            }
        }
        // The quick wheel's button: View's cap (a tap opens it too).
        Rectangle {
            id: wheelBtn
            anchors.right: playGroup.left
            anchors.rightMargin: playGroup.width > 0 ? legend.gap : 0
            anchors.verticalCenter: parent.verticalCenter
            width: wheelRow.implicitWidth + app.px(16); height: Math.max(app.px(34), app.tpx(26)); radius: app.px(6)
            color: app.wheelOpen ? c.focusFill : c.raised
            visible: app.sheet === "" || app.wheelOpen
            Row {
                id: wheelRow
                anchors.centerIn: parent
                spacing: app.px(6)
                Key { k: "view"; anchors.verticalCenter: parent.verticalCenter }
                Label { visible: legend.wheelWord || app.wheelOpen || app.mouseLegend; text: app.wheelOpen ? "Close" : "Wheel"; font.pixelSize: app.fpx(14); color: c.soft; anchors.verticalCenter: parent.verticalCenter }
            }
            HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
            TapHandler { onTapped: app.wheelPress() }
        }
        // ---- Middle: the hints, at most 4 (3 beside the wheel's button), in priority order (A, B, X, Y, then the
        // rest); the ones that don't fit are left out, never squeezed under Play or past the edge.
        Item {
            id: hintsBox
            readonly property real rightEdge: wheelBtn.visible ? wheelBtn.x - legend.gap : playGroup.width > 0 ? playGroup.x - legend.gap : legend.width - legend.edge
            x: barRow.x + barRow.width + (app.barGame ? 0 : 0)
            width: Math.max(0, rightEdge - x)
            height: parent.height
            clip: true
            Row {
                id: legendRow
                visible: true
                // 0.62: the hints fade over when they change (another screen, another row, the pad, keys or the mouse).
                readonly property string said: app.legendHints.slice(0, legend.hintsFit).map(h => app.glyph(h[0]) + " " + h[1] + (h[3] ? app.glyph(h[3]) + h[4] : "")).join("|")
                onSaidChanged: if (mo.on && legend.visible) hintsFade.restart()
                NumberAnimation { id: hintsFade; target: legendRow; properties: "opacity"; from: 0.25; to: 1; duration: mo.base; easing.type: mo.enter }
                anchors.verticalCenter: parent.verticalCenter
                spacing: app.px(16)
                Repeater {
                    model: app.legendHints.slice(0, legend.hintsFit)
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
        }
        // Every hint measured where nobody sees it, so the bar knows how many fit before showing them.
        Row {
            id: hintMeasure
            x: -100000; y: 0
            opacity: 0
            enabled: false
            spacing: legendRow.spacing
            Repeater {
                id: measureRep
                model: app.legendHints
                onItemAdded: legend.measureTick++
                onItemRemoved: legend.measureTick++
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
        readonly property int hintsMost: wheelBtn.visible ? 3 : 4
        property int measureTick: 0
        function hintsWidth(n) {
            let w = measureTick * 0
            for (let i = 0; i < Math.min(n, measureRep.count); i++) { const it = measureRep.itemAt(i); if (it) w += it.implicitWidth + (i > 0 ? legendRow.spacing : 0) }
            return w
        }
        readonly property int hintsFit: {
            const most = Math.min(app.legendHints.length, hintsMost, measureRep.count)
            let n = 0
            for (let i = 1; i <= most; i++) if (hintsWidth(i) <= hintsBox.width) n = i
            return n
        }
        // The tab names show when they fit beside two hints; otherwise LB and RB with the tab they go to; otherwise
        // LB and RB alone (the status line names the tab).
        TextMetrics { id: tabWordsMetrics; font.family: app.bodyFont; font.pixelSize: app.fpx(14); font.bold: true; text: app.tabs.map(t => t.replace(/ \(\d+\)$/, "")).join(" / ") + "  " }
        TextMetrics { id: wheelMetrics; font.family: app.bodyFont; font.pixelSize: app.fpx(14); text: "Wheel" }
        TextMetrics { id: tabShortMetrics; font.family: app.bodyFont; font.pixelSize: app.fpx(14); text: app.barTarget >= 0 ? "back to " + app.tabs[app.barTarget].replace(/ \(\d+\)$/, "") : "" }
        // (The keyboard's names for LB and RB are wider than the controller's.)
        readonly property real capsW: Math.max(app.px(70), barLb.implicitWidth + barRb.implicitWidth + app.px(12))
        // 0.62: the Start hint's words ("Settings", "hold: Play choice") show when they fit beside the game, the
        // caps, the wheel's cap, Play and two hints. Measured from the words, never from the hint's own size.
        TextMetrics { id: startWordsMetrics; font.family: app.bodyFont; font.pixelSize: app.fpx(13); text: app.startHoldWords + "  " }
        readonly property bool startWords: width - edge * 2 - barGameCol.width - capsW - playBtn.width - startHintKey.implicitWidth - app.px(16 + 50) - gap * 6
                                           - hintsWidth(Math.min(2, app.legendHints.length)) >= startWordsMetrics.advanceWidth + app.px(6)
        // Room for the tab words: the bar less the game, the caps, Play, the wheel's cap and two hints.
        readonly property real barFree: width - edge * 2 - barGameCol.width - capsW - playGroup.width - app.px(50) - gap * 5 - hintsWidth(Math.min(2, app.legendHints.length))
        readonly property real tabWordsNeed: tabWordsMetrics.advanceWidth + app.px(6) * app.tabs.length * 2
        readonly property real wheelWordNeed: wheelMetrics.advanceWidth + app.px(6)
        readonly property real tabShortNeed: tabShortMetrics.advanceWidth + app.px(6)
        readonly property bool tabWordsFit: barFree >= tabWordsNeed
        readonly property bool tabShortFits: barFree >= tabShortNeed
        // The tab LB or RB goes to comes before the word Wheel.
        readonly property bool wheelWord: !app.barGame ? barFreeNoGame >= wheelWordNeed : barFree >= tabWordsNeed + wheelWordNeed || (!tabWordsFit && barFree >= tabShortNeed + wheelWordNeed)
        readonly property real barFreeNoGame: width - edge * 2 - app.px(50) - gap * 2 - hintsWidth(Math.min(hintsMost, app.legendHints.length))
        // 0.58, the build's check: nothing in the bar overlaps anything else, Play, or the edge, and the game's name is
        // whole. Logged as "Bar check (<size>): <n> items, <p> problems".
        function check(tag) {
            if (!visible || !shown) return
            let items = []
            const add = (name, it) => { if (it && it.visible && it.width > 0) { const p = it.mapToItem(legend, 0, 0); items.push({ name: name, x: p.x, y: p.y, w: it.width, h: it.height }) } }
            add("game", barGameCol); add("tabs", barTabs)
            for (let i = 0; i < legendRow.children.length; i++) { const h = legendRow.children[i]; if (h.k !== undefined && h.visible) add("hint " + h.k.toUpperCase(), h) }
            add("start", startHint); add("wheel", wheelBtn); add("co-op", coopBtn); add("Play", playBtn)
            let bad = []
            for (const a of items) {
                if (a.x < -0.5 || a.x + a.w > legend.width + 0.5) bad.push(a.name + " past the edge (" + Math.round(a.x) + " to " + Math.round(a.x + a.w) + " of " + Math.round(legend.width) + ")")
                if (a.y < -0.5 || a.y + a.h > legend.height + 0.5) bad.push(a.name + " taller than the bar")
            }
            for (let i = 0; i < items.length; i++) for (let j = i + 1; j < items.length; j++) {
                const a = items[i], b = items[j]
                if (a.x < b.x + b.w - 0.5 && b.x < a.x + a.w - 0.5) bad.push(a.name + " overlaps " + b.name)
            }
            if (barGameCol.visible && barName.truncated) bad.push("the game's name is cut")
            app.note("check", "Bar check (" + tag + ", " + app.width + "x" + app.height + ", size " + app.sizeNow + "%): " + items.length + " items, " + bad.length + " problems"
                     + (bad.length ? ": " + bad.join("; ") : "") + " | " + items.map(a => a.name).join(", ") + " | hints " + hintsFit + " of " + app.legendHints.length)
        }
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
    // 0.62: the mouse's Back button is B everywhere, and Ctrl with the wheel changes the interface size. Over
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
    // 0.64: "Getting 2 of 3: <name>" while requirements are got one after another, with Skip and Stop (also in X
    // options on the Nexus page).
    Rectangle {
        id: reqStrip
        visible: app.reqStripWords !== ""
        z: 95
        anchors.horizontalCenter: parent.horizontalCenter
        y: app.px(8)
        width: reqStripRow.implicitWidth + app.px(20)
        height: Math.max(app.px(40), app.tpx(34))
        radius: app.px(8)
        color: c.panel
        border.color: c.accent
        border.width: Math.max(1, app.px(1.5))
        Row {
            id: reqStripRow
            anchors.centerIn: parent
            spacing: app.px(12)
            Label { text: app.reqStripWords; font.pixelSize: app.fpx(15); color: c.text; anchors.verticalCenter: parent.verticalCenter }
            Repeater {
                model: [["Skip", () => app.reqSkip()], ["Stop", () => app.reqStop()]]
                Rectangle {
                    required property var modelData
                    width: reqBtnText.implicitWidth + app.px(20); height: Math.max(app.px(30), app.tpx(30)); radius: app.px(6)
                    anchors.verticalCenter: parent.verticalCenter
                    color: c.raised; border.color: c.accentLo; border.width: Math.max(1, app.px(1))
                    Label { id: reqBtnText; anchors.centerIn: parent; text: parent.modelData[0]; font.pixelSize: app.fpx(14); color: c.text }
                    Accessible.role: Accessible.Button
                    Accessible.name: parent.modelData[0]
                    HoverHandler { onHoveredChanged: app.hover(parent, hovered) }
                    TapHandler { onTapped: { app.note("touch", "Requirements strip: " + parent.modelData[0]); parent.modelData[1]() } }
                }
            }
        }
    }
    Toast {
        id: toast
        ui: kit
        // With the details panel open, over the list column (not the panel's edge).
        x: app.page === "game" && app.showModPanel && modPanel.visible
           ? Math.max(app.px(10), (rail.width + parent.width - modPanel.width - app.px(20)) / 2 - width / 2) : (parent.width - width) / 2
        function put(kind, m) {
            if (m === app.lastToast && Date.now() - app.lastToastAt < 6000 && shown) return
            app.lastToast = m; app.lastToastAt = Date.now()
            toast.kind = kind
            actionLabel = ""; app.toastAction = null
            timeout = shot.page === "newmod" ? 0 : 4000
            show(m)
        }
        // 0.6 (I10): a toast with its next step as a button; R3 (0.61, M6: Y is always Find) or a tap on the
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
        function showPlain(m) { put("info", m) }
        onPressedOn: app.coverPress(5)
        onPressEnded: app.coverEnd()
        // The new-mod picture keeps its toast up (it arrives a few seconds before the picture).
        timeout: shot.page === "newmod" ? 0 : 4000
    }
    // ---------------------------------------------------------------- 0.59: the start-up splash
    // The only branding: Turbine's icon in the middle, zooming in as it fades in, then out as it fades out, about
    // 1.2 s in all (0.62: it follows Animations: a plain short fade at Reduced, nothing at Off). Once per launch, never when coming back from a game or the
    // background; any press, tap or click skips it. Everything loads underneath while it plays.
    Item {
        id: splash
        objectName: "splash"
        anchors.fill: parent
        z: 90000
        property bool shown: (!shot.on || shot.page.indexOf("splash") === 0) && !app.splashDone
        visible: shown
        readonly property int inMs: mo.splashIn
        readonly property int holdMs: mo.splashHold
        readonly property int outMs: mo.splashOut
        property real started: 0
        function finish(why) {
            if (!shown) return
            app.splashDone = true
            app.note("app", "Splash done (" + why + ") after " + Math.round(Date.now() - started) + " ms")
        }
        // Animations off: no splash at all.
        Component.onCompleted: { started = Date.now(); if (shown && !mo.on) finish("animations off"); else if (shown) { splashAnim.start(); app.note("app", "Splash: start") } }
        Rectangle { anchors.fill: parent; color: c.ground; opacity: splashIcon.opacity }
        Image {
            id: splashIcon
            anchors.centerIn: parent
            width: Math.min(parent.width, parent.height) * 0.28; height: width
            source: app.iconBig
            sourceSize.width: 512
            smooth: true; mipmap: true
            opacity: 0
            scale: mo.splashFrom
        }
        SequentialAnimation {
            id: splashAnim
            ParallelAnimation {
                NumberAnimation { target: splashIcon; properties: "opacity"; from: 0; to: 1; duration: splash.inMs; easing.type: mo.enter }
                NumberAnimation { target: splashIcon; properties: "scale"; from: mo.splashFrom; to: 1; duration: splash.inMs; easing.type: mo.enter }
            }
            PauseAnimation { duration: splash.holdMs }
            ParallelAnimation {
                NumberAnimation { target: splashIcon; properties: "opacity"; to: 0; duration: splash.outMs; easing.type: mo.leave }
                NumberAnimation { target: splashIcon; properties: "scale"; to: mo.splashTo; duration: splash.outMs; easing.type: mo.leave }
            }
            ScriptAction { script: splash.finish("played") }
        }
        // A tap or click skips it and goes nowhere else.
        MouseArea {  // overlay-check: the splash takes every press while it plays
            anchors.fill: parent
            acceptedButtons: Qt.AllButtons
            preventStealing: true
            onWheel: (w) => { w.accepted = true; splash.finish("wheel") }
            onPressed: { app.coverPress(100000); splash.finish("tap") }
            onReleased: app.coverEnd()
            onCanceled: app.coverEnd()
        }
    }
    property bool splashDone: false

    // The build's input check: a plain mouse area over everything, to see the left button go down and up.
    MouseArea { id: inputProbe; anchors.fill: parent; z: 100000; visible: app.probeOn; enabled: app.probeOn; acceptedButtons: Qt.LeftButton }  // overlay-check: the build's input probe only
    // Labs: the brass ring that fills around the rail icon while View (or the icon) is held for a report.
    Shape {
        id: holdRing
        visible: app.holdShown
        readonly property real r: app.px(28)
        // Labs' report hold: in the middle of the window (0.59: the rail icon it went round is gone).
        x: app.width / 2 - r - app.px(3)
        y: app.height / 2 - r - app.px(3)
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
        function onPointerXChanged() { app.placeHover(); app.pointerMovedAt = Date.now(); app.pointerStill = false; pointerStillTimer.restart() }
        function onPointerYChanged() { app.placeHover(); app.pointerMovedAt = Date.now(); app.pointerStill = false; pointerStillTimer.restart() }
        function onPointerShownChanged() { app.note("pad", Turbine.pointerShown ? "Pointer showing" : "Pointer hidden") }
    }
    property bool pointerStill: false
    Timer { id: pointerStillTimer; interval: 3000; onTriggered: app.pointerStill = true }
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
        // On the Nexus page the pointer never hides: still for 3 seconds, it dims.
        opacity: app.page === "browser" && app.pointerStill ? 0.45 : 1
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
