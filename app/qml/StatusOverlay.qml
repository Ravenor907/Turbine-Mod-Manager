import QtQuick
import QtQuick.Window
import QtCore

// The window that carries Turbine's status card over other apps while a download is caught.
// - Desktop Mode (hud false): a small frameless panel kept above other windows, in the top right corner (16 px from
//   the edges), at most 360 design px wide. Drag it by its body; where it was left is remembered. It takes clicks
//   and taps (Add it, installer choices) but never takes the keyboard focus from the browser. While a download waits
//   for the player (the browser's Save box), only a small pill shows ("Press Save in the browser"), so it never
//   covers the Save button.
// - Game Mode (hud true): a transparent window the size of the screen that never takes input. Main.qml marks it
//   for gamescope (GAMESCOPE_EXTERNAL_OVERLAY, see qtglue.cpp) before showing it and takes the mark off when it
//   hides; the card is drawn in the bottom left corner.
Window {
    id: win
    required property var ui
    property bool hud: false
    property var status: null
    property var fomod: null

    signal confirm(bool yes)
    signal pick()
    signal choose(int group, int option)
    signal next()
    signal back()
    signal stop()
    signal openTurbine()

    title: hud ? "Turbine status overlay" : "Turbine status"
    color: "transparent"
    visible: false
    flags: hud ? (Qt.Window | Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint | Qt.WindowTransparentForInput | Qt.WindowDoesNotAcceptFocus)
               : (Qt.Window | Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint | Qt.WindowDoesNotAcceptFocus)
    readonly property int margin: ui.px(16)
    // The browser's Save box is waiting: the pill only.
    readonly property bool waiting: !!status && status.level === "wait"
    Settings {
        id: place
        category: "overlay"
        property int x: -1
        property int y: -1
    }
    x: hud ? 0 : (place.x >= 0 ? Math.min(place.x, Screen.desktopAvailableWidth - width) : Screen.desktopAvailableWidth - width - margin)
    y: hud ? 0 : (place.y >= 0 ? Math.min(place.y, Screen.desktopAvailableHeight - height) : margin)
    width: hud ? Screen.width : (waiting ? pill.width : card.width)
    height: hud ? Screen.height : (waiting ? pill.height : card.height)
    // Dragged by the window system: where it ends up is kept for next time.
    onXChanged: if (!hud && dragging) place.x = x
    onYChanged: if (!hud && dragging) place.y = y
    property bool dragging: false

    Rectangle {
        id: pill
        visible: win.waiting
        x: win.hud ? win.width - width - win.margin : 0
        y: win.hud ? win.margin : 0
        width: pillText.implicitWidth + win.ui.px(36)
        height: win.ui.px(36)
        radius: height / 2
        color: win.ui.c.panel
        border.color: win.ui.c.accent
        border.width: Math.max(1, win.ui.px(1.5))
        Rectangle { x: win.ui.px(14); anchors.verticalCenter: parent.verticalCenter; width: win.ui.px(8); height: width; radius: width / 2; color: win.ui.c.accentHi }
        Text {
            id: pillText
            x: win.ui.px(28)
            anchors.verticalCenter: parent.verticalCenter
            text: win.status && win.status.text ? win.status.text : "Press Save in the browser"
            color: win.ui.c.text
            font.family: win.ui.bodyFont
            font.pixelSize: win.ui.px(15)
            font.bold: true
        }
    }
    StatusCard {
        id: card
        visible: !win.waiting
        ui: win.ui
        status: win.status
        fomod: win.fomod
        interactive: !win.hud
        x: win.hud ? win.margin : 0
        y: win.hud ? win.height - height - win.margin * 2 : 0
        onConfirm: (yes) => win.confirm(yes)
        onPick: win.pick()
        onChoose: (g, o) => win.choose(g, o)
        onNext: win.next()
        onBack: win.back()
        onStop: win.stop()
        onOpenTurbine: win.openTurbine()
    }
    // Drag the Desktop panel by its body (its buttons still take taps).
    DragHandler {
        enabled: !win.hud
        target: null
        onActiveChanged: if (active) { win.dragging = true; win.startSystemMove() } else win.dragging = false
    }
}
