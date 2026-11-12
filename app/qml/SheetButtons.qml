import QtQuick

// The row of buttons every sheet with buttons uses (Before you play, the loader prompt, the plain sheets, text
// entry, Delete for good). One rule everywhere, kept in Main.qml (buttonSheet): the D-pad moves the highlight, A
// presses the highlighted button, B is the cancel button (Start does nothing here). Each button carries its key:
// A on the highlighted one, B on the cancel one (no caps after the mouse). The main button is filled with the accent;
// the highlight is a ring around a button, never a fill. One row: when the buttons don't fit side by side, the cancel
// button leaves the row (B still does it, and the D-pad passes it: cancelHidden). A tap presses a button too.
Flow {
    id: sb
    required property var ui
    property var labels: []               // the buttons' words, in order
    property int focusIndex: 0            // the highlighted one (-1: none, the focus is elsewhere on the sheet)
    property int mainIndex: -1            // the main button: filled with the accent
    property int cancelIndex: -1          // the one B presses (it shows B's cap)
    signal activated(int index)
    // Pressed down (true) and let go or slid off (false): the hold buttons (Delete for good).
    signal held(int index, bool down)

    spacing: ui.px(10)
    function itemAt(i) { return rep.itemAt(i) }
    // Every button side by side, at its own width.
    property int measureTick: 0
    readonly property real naturalWidth: {
        let w = measureTick * 0
        for (let i = 0; i < rep.count; i++) { const it = rep.itemAt(i); if (it) w += it.want + (i > 0 ? spacing : 0) }
        return w
    }
    readonly property bool cancelHidden: cancelIndex >= 0 && cancelIndex < labels.length && labels.length > 1 && focusIndex !== cancelIndex
                                         && width > 0 && naturalWidth > width + 0.5
    Repeater {
        id: rep
        // By count, so a label that changes (Keep holding A: 2...) updates in place: a rebuilt button would lose
        // the finger or the pointer pressing it, and with it the release that stops a hold.
        model: sb.labels.length
        onItemAdded: sb.measureTick++
        onItemRemoved: sb.measureTick++
        ActionButton {
            id: btn
            required property int index
            ui: sb.ui
            label: sb.labels[index] || ""
            focused: index === sb.focusIndex
            primary: index === sb.mainIndex
            cap: focused ? "a" : index === sb.cancelIndex ? "b" : ""
            capRoom: true
            readonly property bool gone: sb.cancelHidden && index === sb.cancelIndex
            onWantChanged: sb.measureTick++
            visible: !gone
            width: gone ? 0 : want
            onClicked: sb.activated(btn.index)
            onHeld: (down) => sb.held(btn.index, down)
        }
    }
}
