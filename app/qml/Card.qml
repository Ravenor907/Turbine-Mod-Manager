import QtQuick

// The one card look for everything shown in the middle of the window (dialogs, the game menu, the pickers) and for
// the side sheets: the panel color, the shared radius, a thin line, and the accent band along the top edge. The band
// is part of the fill (a gradient with a hard stop), so the rounded corners clip it like the card itself.
//
// ui (required): Main.qml's kit. band: the accent band along the top. fill: the card's color.
Rectangle {
    id: card
    required property var ui
    property bool band: true
    property color fill: ui.c.panel
    radius: ui.radius
    border.width: Math.max(1, ui.px(1))
    border.color: ui.c.line
    readonly property real bandPart: band && height > 0 ? Math.min(0.5, Math.max(2, ui.px(3)) / height) : 0
    gradient: Gradient {
        GradientStop { position: 0; color: card.band ? card.ui.c.accent : card.fill }
        GradientStop { position: card.bandPart; color: card.band ? card.ui.c.accent : card.fill }
        GradientStop { position: Math.min(1, card.bandPart + 0.0005); color: card.fill }
        GradientStop { position: 1; color: card.fill }
    }
}
