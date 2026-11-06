import QtQuick
import QtQuick.Window
import Turbine

// Labs > Performance check: Run check measures about 13 seconds (standing still with one small animation, scrolling a
// long list, then 4 seconds with nothing to draw) and gives a short verdict, frame times for each part, the processor
// time Turbine used, how it draws (and whether that's software drawing), startup time, screen and scale numbers,
// memory and how long controller presses waited. The report shows here before Copy report or Report a bug (which
// sends it along). Left and Right choose a button, A uses it, B closes (or stops a check under way).
Item {
    id: perf
    required property var ui
    required property var win
    anchors.fill: parent
    visible: false

    property string version: ""
    property string extra: ""             // lines from Main.qml: window, scale, look
    property int focusAt: 0               // 0 Run check, 1 Copy report, 2 Report a bug
    property bool running: false
    property string phase: ""             // "", "still", "scroll", "idle"
    property var results: []
    property string verdict: ""
    property string reportText: ""

    signal copyClicked(string text)
    signal reportClicked(string text)
    signal closeClicked()

    readonly property var buttons: ["Run check", "Copy report", "Report a bug"]

    function open() { focusAt = 0; visible = true; if (reportText === "") reportText = makeReport() }
    function close() { stop(); visible = false }
    function handleButton(b) {
        if (b === "left") focusAt = Math.max(0, focusAt - 1)
        else if (b === "right") focusAt = Math.min(buttons.length - 1, focusAt + 1)
        else if (b === "a") activate(focusAt)
        else if (b === "b") { if (running) stop(); else closeClicked() }
    }
    function activate(i) {
        focusAt = i
        if (i === 0) { if (running) stop(); else start() }
        else if (i === 1) copyClicked(reportText)
        else reportClicked(reportText)
    }

    // ------------------------------------------------------------- measuring
    readonly property var plan: [{ key: "still", name: "Still", ms: 3000 }, { key: "scroll", name: "Scrolling", ms: 4000 },
                                 { key: "idle", name: "Idle", ms: 4000 }]
    property int step: -1
    property bool warming: false
    property var frames: []
    property real lastFrame: 0
    property real cpuAt: 0
    property real wallAt: 0

    function start() {
        Turbine.log("labs", "Performance check started")
        results = []
        verdict = ""
        running = true
        step = -1
        next()
    }
    function stop() {
        if (!running) return
        running = false
        phase = ""
        clock.stop()
        Turbine.log("labs", "Performance check stopped")
    }
    function next() {
        step++
        if (step >= plan.length) { finish(); return }
        phase = plan[step].key
        warming = true
        clock.interval = 500
        clock.restart()
    }
    function begin() {
        warming = false
        frames = []
        lastFrame = 0
        const f = JSON.parse(Turbine.perfFacts())
        cpuAt = f.cpuMs
        wallAt = Turbine.nowUs()
        clock.interval = plan[step].ms
        clock.restart()
    }
    function measure() {
        const f = JSON.parse(Turbine.perfFacts())
        const wall = (Turbine.nowUs() - wallAt) / 1000
        const list = frames.slice().sort((a, b) => a - b)
        const n = list.length
        const median = n > 0 ? list[Math.floor(n / 2)] : 0
        const worst = n > 0 ? list[n - 1] : 0
        const spikes = n > 0 ? list.filter(v => v > Math.max(25, median * 2)).length * 100 / n : 0
        const r = { name: plan[step].name, frames: n, fps: n * 1000 / Math.max(1, wall), median: median, worst: worst,
                    spikes: spikes, cpu: (f.cpuMs - cpuAt) * 100 / Math.max(1, wall), secs: wall / 1000 }
        results = results.concat([r])
        next()
    }
    function finish() {
        running = false
        phase = ""
        const still = results[0], scroll = results[1], idle = results[2]
        let v = []
        if (still.fps < 45) v.push("Slow even standing still")
        if (scroll.spikes > 5 || scroll.fps < 45) v.push("Stutters while scrolling")
        if (idle.cpu > 5) v.push("Busy while nothing happens")
        verdict = v.length ? v.join("; ") : "Smooth"
        reportText = makeReport()
        Turbine.log("labs", "Performance check done: " + verdict)
    }
    Timer { id: clock; onTriggered: perf.warming ? perf.begin() : perf.measure() }
    Connections {
        target: perf.win
        enabled: perf.running && !perf.warming
        function onFrameSwapped() {
            const now = Turbine.nowUs() / 1000
            if (perf.lastFrame > 0) perf.frames.push(now - perf.lastFrame)
            perf.lastFrame = now
        }
    }

    function fixed(v, d) { return Number(v).toFixed(d === undefined ? 1 : d) }
    function apiName(a) {
        switch (a) {
        case GraphicsInfo.Software: return "Software"
        case GraphicsInfo.OpenGL: return "OpenGL"
        case GraphicsInfo.Vulkan: return "Vulkan"
        case GraphicsInfo.Direct3D11: return "Direct3D 11"
        case GraphicsInfo.Direct3D12: return "Direct3D 12"
        case GraphicsInfo.Metal: return "Metal"
        case GraphicsInfo.Null: return "None"
        default: return "Not known yet"
        }
    }
    function makeReport() {
        const f = JSON.parse(Turbine.perfFacts())
        let out = ["Turbine performance check", "Turbine: " + version + " (build " + f.build + ")"]
        if (results.length === 3) {
            out.push("Verdict: " + verdict)
            for (const r of results.slice(0, 2))
                out.push(r.name + ": " + Math.round(r.fps) + " fps, median " + fixed(r.median) + " ms, worst " + fixed(r.worst)
                         + " ms, spikes " + fixed(r.spikes) + "%, CPU " + Math.round(r.cpu) + "%")
            const idle = results[2]
            out.push("Idle: " + idle.frames + " frames in " + fixed(idle.secs) + " s, CPU " + fixed(idle.cpu) + "%")
        } else {
            out.push("No check run yet.")
        }
        out.push("")
        out.push("Startup: first frame " + Math.round(f.startupMs) + " ms after start")
        const api = apiName(GraphicsInfo.api)
        out.push("Drawing: " + api + ", software drawing " + (GraphicsInfo.api === GraphicsInfo.Software ? "yes" : "no"))
        out.push("Screen: " + Screen.width + " x " + Screen.height + " px, pixel ratio " + fixed(Screen.devicePixelRatio, 2))
        if (extra !== "") out.push(extra)
        out.push("Memory: " + Math.round(f.memoryMb) + " MB, processor time so far " + Math.round(f.cpuMs) + " ms, " + f.threads + " threads")
        out.push("Controller: " + (f.padCount > 0 ? "presses waited median " + f.padMedian + " ms, worst " + f.padWorst + " ms over "
                                   + f.padCount + " presses (read every 16 ms)" : "no presses measured yet")
                 + (f.pads !== "" ? "; " + f.pads : "; none connected"))
        out.push("Device: " + f.device)
        out.push("System: " + f.system)
        return out.join("\n")
    }

    // ------------------------------------------------------------- the screen
    Rectangle { anchors.fill: parent; color: "#C8000000" }
    MouseArea { anchors.fill: parent; onClicked: perf.closeClicked() }
    Rectangle {
        id: box
        anchors.centerIn: parent
        width: Math.min(parent.width - perf.ui.px(32), perf.ui.px(660))
        height: Math.min(parent.height - perf.ui.px(24), col.implicitHeight + perf.ui.px(36))
        radius: perf.ui.px(10)
        color: perf.ui.c.panel
        border.color: perf.ui.c.line
        clip: true
        MouseArea { anchors.fill: parent }
        Column {
            id: col
            x: perf.ui.px(18); y: perf.ui.px(18)
            width: parent.width - perf.ui.px(36)
            spacing: perf.ui.px(10)
            Text {
                text: "PERFORMANCE CHECK"
                color: perf.ui.c.text
                font.family: perf.ui.titleFont
                font.pixelSize: perf.ui.px(18)
                font.bold: true
                font.letterSpacing: perf.ui.px(1.5)
            }
            Text {
                width: parent.width
                wrapMode: Text.WordWrap
                color: perf.ui.c.soft
                font.family: perf.ui.bodyFont
                font.pixelSize: perf.ui.px(15)
                text: perf.running ? (perf.phase === "still" ? "Measuring: standing still..." : perf.phase === "scroll" ? "Measuring: scrolling a long list..." : "Measuring: nothing to draw...")
                      : perf.verdict !== "" ? "Verdict: " + perf.verdict
                      : "Run check measures for about 13 seconds. Leave the controller alone meanwhile."
            }
            Row {
                spacing: perf.ui.px(10)
                Repeater {
                    model: perf.buttons
                    Rectangle {
                        id: btn
                        required property int index
                        required property string modelData
                        readonly property bool focused: perf.focusAt === index
                        width: label.implicitWidth + perf.ui.px(28)
                        height: perf.ui.px(40)
                        radius: perf.ui.px(8)
                        opacity: index > 0 && perf.running ? 0.5 : 1
                        color: focused ? perf.ui.c.focusFill : perf.ui.c.raised
                        border.color: focused ? perf.ui.c.text : perf.ui.c.line
                        border.width: focused ? perf.ui.px(2) : Math.max(1, perf.ui.px(1))
                        Text {
                            id: label
                            anchors.centerIn: parent
                            text: btn.index === 0 && perf.running ? "Stop" : btn.modelData
                            color: perf.ui.c.text
                            font.family: perf.ui.bodyFont
                            font.pixelSize: perf.ui.px(15)
                            font.bold: true
                        }
                        TapHandler { onTapped: perf.activate(btn.index) }
                    }
                }
            }
            // What gets drawn while measuring: a turning ring (still), a long list (scrolling), nothing (idle).
            Rectangle {
                width: parent.width
                height: perf.running ? perf.ui.px(120) : reportView.implicitHeight + perf.ui.px(20)
                radius: perf.ui.px(6)
                color: perf.ui.c.card
                clip: true
                Rectangle {
                    visible: perf.phase === "still"
                    anchors.centerIn: parent
                    width: perf.ui.px(40); height: width; radius: perf.ui.px(4)
                    color: perf.ui.c.accent
                    RotationAnimation on rotation { running: perf.phase === "still"; from: 0; to: 360; duration: 1200; loops: Animation.Infinite }
                }
                ListView {
                    id: scroller
                    visible: perf.phase === "scroll"
                    anchors.fill: parent
                    model: perf.phase === "scroll" ? 400 : 0
                    interactive: false
                    delegate: Rectangle {
                        required property int index
                        width: scroller.width; height: perf.ui.px(30)
                        color: index % 2 ? perf.ui.c.card : perf.ui.c.raised
                        Text { x: perf.ui.px(10); anchors.verticalCenter: parent.verticalCenter; text: "Row " + (parent.index + 1); color: perf.ui.c.soft; font.family: perf.ui.bodyFont; font.pixelSize: perf.ui.px(14) }
                    }
                    NumberAnimation on contentY {
                        running: perf.phase === "scroll"
                        from: 0; to: perf.ui.px(30) * 300; duration: 4600; loops: Animation.Infinite
                    }
                }
                Text {
                    id: reportView
                    visible: !perf.running
                    x: perf.ui.px(10); y: perf.ui.px(10)
                    width: parent.width - perf.ui.px(20)
                    wrapMode: Text.WordWrap
                    textFormat: Text.PlainText
                    text: perf.reportText
                    color: perf.ui.c.text
                    font.family: "monospace"
                    font.pixelSize: perf.ui.px(13)
                }
            }

        }
    }
}
