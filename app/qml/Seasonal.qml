import QtQuick

// Seasonal background: falling leaves (Autumn) or snow, drawn behind the panels in one Canvas. Settings > Look >
// Seasonal picks Off, By date (snow from December 1 to the end of February, leaves the rest of the year), Autumn or
// Snow; Amount and Size set how many and how big. It moves at 30 frames a second only while Turbine's window is in
// front, and holds still otherwise, so it costs nothing while a game runs.
//
// kind: "", "autumn" or "snow". amount: 10 to 100 (how many on a 1280 x 800 window). size: 50 to 200 (percent).
// scale: design pixels to screen pixels. moving: false holds everything still.
Canvas {
    id: fall
    property string kind: ""
    property int amount: 40
    property int size: 100
    property real scale: 1
    property bool moving: true
    property bool light: false
    visible: kind !== ""
    renderStrategy: Canvas.Cooperative

    // How many fall here: Amount is for a 1280 x 800 window; bigger windows get more, never more than 160.
    readonly property int count: Math.max(1, Math.min(160, Math.round(amount * Math.max(0.5, width * height / (1280 * 800)))))
    property var pieces: []
    property real last: 0

    readonly property var leafTones: light ? ["#C27A2C", "#A9561F", "#B88A2E", "#8E4A1C", "#C9952F"]
                                           : ["#C99E52", "#D6763A", "#B9452A", "#E8B45A", "#A86A2A"]
    readonly property color flake: light ? "#7C8EA6" : "#E8EEF5"

    // The same index always gives the same piece, so Amount adds or takes away pieces without reshuffling.
    function make(i, anywhere) {
        let s = (i * 2654435761 + 97) >>> 0
        const r = function () { s = (s + 0x6D2B79F5) >>> 0; let t = s; t = Math.imul(t ^ (t >>> 15), t | 1); t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296 }
        const depth = r()
        return {
            x: r() * width, y: anywhere ? r() * height : -40 * scale,
            w: (kind === "snow" ? 3 + depth * 4 : 9 + depth * 9) * scale * size / 100,
            speed: (kind === "snow" ? 18 + depth * 30 : 22 + depth * 34) * scale,
            sway: (kind === "snow" ? 10 : 22) * scale * (0.5 + r()), swayT: 2.5 + r() * 3, phase: r() * 6.28,
            spin: (r() - 0.5) * 2.4, angle: r() * 6.28, tone: Math.floor(r() * 5), op: 0.45 + depth * 0.45, seed: i
        }
    }
    function reset() {
        const list = []
        for (let i = 0; i < count; ++i) list.push(make(i, true))
        pieces = list
        requestPaint()
    }
    onCountChanged: reset()
    onKindChanged: reset()
    onSizeChanged: reset()
    onWidthChanged: resetSoon.restart()
    onHeightChanged: resetSoon.restart()
    Timer { id: resetSoon; interval: 200; onTriggered: fall.reset() }
    Component.onCompleted: reset()

    Timer {
        interval: 33
        repeat: true
        running: fall.visible && fall.moving && fall.width > 0
        onRunningChanged: fall.last = 0
        onTriggered: {
            const now = Date.now()
            const dt = fall.last > 0 ? Math.min(0.1, (now - fall.last) / 1000) : 0.033
            fall.last = now
            const t = now / 1000
            for (let i = 0; i < fall.pieces.length; ++i) {
                const p = fall.pieces[i]
                p.y += p.speed * dt
                p.angle += p.spin * dt
                p.drawX = p.x + Math.sin(t / p.swayT * 6.28 + p.phase) * p.sway
                if (p.y > fall.height + 40 * fall.scale) {
                    const n = fall.make(p.seed + 7919 * Math.floor(t), false)
                    n.seed = p.seed
                    fall.pieces[i] = n
                }
            }
            fall.requestPaint()
        }
    }

    onPaint: {
        const ctx = getContext("2d")
        ctx.reset()
        for (let i = 0; i < pieces.length; ++i) {
            const p = pieces[i]
            const x = p.drawX !== undefined ? p.drawX : p.x
            ctx.globalAlpha = p.op
            if (kind === "snow") {
                ctx.fillStyle = flake
                ctx.beginPath()
                ctx.arc(x, p.y, p.w / 2, 0, 6.2832)
                ctx.fill()
            } else {
                // A leaf: two arcs meeting at the tips, with a stem line.
                ctx.save()
                ctx.translate(x, p.y)
                ctx.rotate(p.angle)
                ctx.scale(1, 0.55 + 0.45 * Math.abs(Math.sin(p.angle * 1.7)))
                ctx.fillStyle = leafTones[p.tone]
                ctx.beginPath()
                ctx.moveTo(-p.w, 0)
                ctx.quadraticCurveTo(0, -p.w * 0.75, p.w, 0)
                ctx.quadraticCurveTo(0, p.w * 0.75, -p.w, 0)
                ctx.fill()
                ctx.strokeStyle = leafTones[(p.tone + 3) % 5]
                ctx.lineWidth = Math.max(1, p.w / 9)
                ctx.beginPath()
                ctx.moveTo(-p.w * 1.25, 0)
                ctx.lineTo(p.w * 0.8, 0)
                ctx.stroke()
                ctx.restore()
            }
        }
        ctx.globalAlpha = 1
    }
}
