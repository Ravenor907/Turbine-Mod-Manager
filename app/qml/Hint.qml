import QtQuick

// One or more keys ("lb+rb") and what they do.
//
// win (required): Main.qml's window. inBar: a hint of the bottom bar's legend (BottomBar sets it), where a hint for
// one button is also a button.
Row {
    id: hint
    required property var win
    property string k: ""
    property string label: ""
    property string pre: ""           // "hold": the legend reads "hold A Delete"
    property string k2: ""            // a second key and its words in one hint ("RT Click, LT Menu")
    property string label2: ""
    property bool inBar: false
    // In the legend, a hint for one button is also a button: a tap or click does what the button does.
    readonly property bool clickable: inBar && ["a", "b", "x", "y", "r3"].indexOf(k) >= 0
    // After the mouse the hint is a button with its name on it and no key cap.
    readonly property bool named: win.mouseLegend && clickable && label !== "" && k2 === ""
    // A finger-sized target in the bar (the key caps alone are 24 px).
    height: clickable ? Math.max(win.tpx(44), implicitHeight) : implicitHeight
    spacing: win.px(6)
    TapHandler { enabled: hint.clickable; onTapped: { if (!hint.win.gate(3, "hint " + hint.k)) return; hint.win.note("action", "Legend " + hint.k + " (" + hint.label + ")"); if (hint.k === "r3") hint.win.r3Click(); else hint.win.act(hint.k) } }
    HoverHandler { enabled: hint.clickable; onHoveredChanged: hint.win.hover(hint, hovered) }
    Label { win: hint.win; visible: hint.pre !== "" && !hint.named; text: hint.pre; font.pixelSize: hint.win.fpx(15); color: hint.win.colors.soft; anchors.verticalCenter: parent.verticalCenter }
    Repeater {
        model: hint.k.split("+")
        Key { required property string modelData; win: hint.win; k: modelData; visible: modelData !== ""; anchors.verticalCenter: parent.verticalCenter }
    }
    Rectangle {
        visible: hint.named
        width: namedText.implicitWidth + hint.win.px(22); height: Math.max(hint.win.px(32), hint.win.tpx(26)); radius: hint.win.px(6)
        anchors.verticalCenter: parent.verticalCenter
        color: hint.win.colors.raised; border.color: hint.win.colors.accentLo; border.width: Math.max(1, hint.win.px(1))
        Text { id: namedText; anchors.centerIn: parent; text: (hint.pre !== "" ? hint.pre + " " : "") + hint.label; color: hint.win.colors.text; font.family: hint.win.bodyFont; font.pixelSize: hint.win.fpx(14) }
    }
    Label { win: hint.win; visible: hint.label !== "" && !hint.named; text: hint.label + (hint.k2 !== "" ? "," : ""); font.pixelSize: hint.win.fpx(15); color: hint.win.colors.soft; anchors.verticalCenter: parent.verticalCenter }
    Key { win: hint.win; visible: hint.k2 !== ""; k: hint.k2; anchors.verticalCenter: parent.verticalCenter }
    Label { win: hint.win; visible: hint.label2 !== ""; text: hint.label2; font.pixelSize: hint.win.fpx(15); color: hint.win.colors.soft; anchors.verticalCenter: parent.verticalCenter }
}
