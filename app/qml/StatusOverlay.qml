import QtQuick
import QtQuick.Window

// The window that carries Turbine's status card over other apps while a download is caught.
// - Desktop Mode (hud false): a small frameless panel kept above other windows, in the bottom right corner. It
//   takes clicks and taps (Add it, installer choices) but never takes the keyboard focus from the browser.
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
    readonly property int margin: ui.px(18)
    x: hud ? 0 : Screen.desktopAvailableWidth - width - margin
    y: hud ? 0 : Screen.desktopAvailableHeight - height - margin
    width: hud ? Screen.width : card.width
    height: hud ? Screen.height : card.height

    StatusCard {
        id: card
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
}
