import QtQuick
import QtQuick.Shapes

// "Add mods": a side sheet listing the zip files in Downloads (name, size, how old), the focused one with A Add.
// When there are none it says how to get one, and a plain line says that browsing mods online comes later.
// It fills its parent; show and hide it with visible.
//
// ui (required): an object that gives
//   ui.c          colors: ground, panel, card, raised, line, text, soft, muted, accent, accentHi, accentLo,
//                 onAccent, focusFill, ok, warn, light (bool); stop is optional (a fallback is used without it)
//   ui.px(n)      design pixels to screen pixels
//   ui.titleFont  font family for titles
//   ui.bodyFont   font family for everything else
Overlay {
    id: root
    onOutsideTapped: root.closed()

    required property var ui
    // 0.56: a download's name as people say it: no .zip, no Nexus numbers, spaces for underscores (the file's own
    // name is the small line under it).
    function niceName(f) {
        let n = String(f).replace(/\.(zip|7z|rar)$/i, "")
        const m = n.match(/^(.*?)-\d+(-[0-9A-Za-z]+)*-\d{9,}$/)
        if (m && m[1].length > 1) n = m[1]
        n = n.replace(/_/g, " ").replace(/\s+/g, " ").trim()
        return n !== "" ? n : String(f)
    }

    property string title: "Add from Downloads"
    property string subtitle: "Mod files in Downloads, newest first"
    // Each file: { name, size (text, e.g. "4.1 MB"), age (text, e.g. "2 days ago"), note (optional, e.g. "Already added") }
    property var files: []
    property int focusIndex: 0
    property string actionLabel: "Add"
    property string emptyText: "No mod files in Downloads yet."
    property string laterText: ""
    // Y's action as a button too (touch has no Y): "Find", the pop-up that chooses which files show (0.61).
    property string otherLabel: ""
    signal otherClicked()
    property real bottomInset: ui.px(8)   // the sheet layer already ends above the bottom bar

    signal activated(int index)
    signal closed()

    readonly property int count: (files || []).length

    anchors.fill: parent

    Rectangle {
        anchors.fill: parent
        color: Qt.rgba(0.02, 0.016, 0.012, 0.74)
    }

    Rectangle {
        id: sheet
        // The top and right edges sit just outside the parent, so only the left and bottom lines show.
        anchors.right: parent.right
        anchors.rightMargin: -border.width
        anchors.top: parent.top
        anchors.topMargin: -border.width
        border.width: Math.max(1, root.ui.px(1.5))
        border.color: root.ui.c.line
        width: Math.min(parent.width - root.ui.px(40), root.ui.px(420))
        height: Math.max(root.ui.px(160), parent.height - root.bottomInset + border.width)
        color: root.ui.c.panel
        bottomLeftRadius: root.ui.px(6)

        CardGuard {}   // taps on the sheet don't close it

        Column {
            id: head
            x: root.ui.px(18)
            y: root.ui.px(16)
            width: parent.width - root.ui.px(32)
            spacing: root.ui.px(3)
            Text {
                width: parent.width
                text: root.title || ""
                elide: Text.ElideRight
                color: root.ui.c.text
                font.family: root.ui.titleFont
                font.pixelSize: root.ui.fpx(17)
                font.bold: true
            }
            Text {
                visible: text !== ""
                width: parent.width
                text: root.subtitle || ""
                wrapMode: Text.WordWrap
                color: root.ui.c.muted
                font.family: root.ui.bodyFont
                font.pixelSize: root.ui.fpx(14)
            }
            Rectangle {
                objectName: "addOther"
                visible: root.otherLabel !== ""
                width: otherText.implicitWidth + root.ui.px(24); height: root.ui.tpx(40); radius: root.ui.px(6)
                color: root.ui.c.raised; border.color: root.ui.c.line; border.width: Math.max(1, root.ui.px(1))
                Text { id: otherText; anchors.centerIn: parent; text: root.otherLabel; color: root.ui.c.text; font.family: root.ui.bodyFont; font.pixelSize: root.ui.fpx(14); font.bold: true }
                HoverHandler { onHoveredChanged: root.ui.hover(parent, hovered) }
                TapHandler { onTapped: root.otherClicked() }
            }
            Item { width: 1; height: root.ui.px(7) }
            Rectangle { width: parent.width; height: Math.max(1, root.ui.px(1)); color: root.ui.c.line }
        }

        // The plain line about what comes later, pinned to the bottom of the sheet.
        Text {
            id: laterLine
            visible: text !== ""
            anchors.bottom: parent.bottom
            anchors.bottomMargin: root.ui.px(14)
            x: root.ui.px(18)
            width: parent.width - root.ui.px(32)
            text: root.laterText || ""
            wrapMode: Text.WordWrap
            color: root.ui.c.muted
            font.family: root.ui.bodyFont
            font.pixelSize: root.ui.fpx(14)
        }

        ListView {
            id: fileList
            visible: root.count > 0
            anchors.top: head.bottom
            anchors.topMargin: root.ui.px(8)
            anchors.bottom: laterLine.visible ? laterLine.top : parent.bottom
            anchors.bottomMargin: root.ui.px(10)
            x: root.ui.px(14)
            width: parent.width - root.ui.px(26)
            clip: true
            spacing: root.ui.px(4)
            boundsBehavior: Flickable.StopAtBounds
            model: root.files || []
            currentIndex: Math.max(0, Math.min(root.focusIndex, root.count - 1))
            highlightFollowsCurrentItem: false
            onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
            delegate: Rectangle {
                id: fileRow
                required property int index
                required property var modelData
                readonly property bool focused: index === root.focusIndex
                readonly property string note: modelData.note || ""
                width: fileList.width
                height: Math.max(root.ui.px(54), fileText.implicitHeight + root.ui.px(14))
                radius: root.ui.px(6)
                color: root.ui.c.card

                Rectangle {
                    visible: fileRow.focused
                    anchors.fill: parent
                    radius: fileRow.radius
                    border.width: Math.max(1, root.ui.px(1.5))
                    border.color: root.ui.c.accent
                    gradient: Gradient {
                        orientation: Gradient.Horizontal
                        GradientStop { position: 0; color: root.ui.c.focusFill }
                        GradientStop { position: 1; color: Qt.tint(root.ui.c.card, Qt.rgba(root.ui.c.focusFill.r, root.ui.c.focusFill.g, root.ui.c.focusFill.b, 0.5)) }
                    }
                }
                HoverHandler { onHoveredChanged: root.ui.hover(parent, hovered) }
                TapHandler { onTapped: root.activated(fileRow.index) }

                // Zip icon tile.
                Rectangle {
                    id: zipTile
                    x: root.ui.px(12)
                    anchors.verticalCenter: parent.verticalCenter
                    width: root.ui.px(28)
                    height: width
                    radius: root.ui.px(6)
                    color: fileRow.focused ? Qt.tint(root.ui.c.card, Qt.rgba(root.ui.c.accent.r, root.ui.c.accent.g, root.ui.c.accent.b, 0.28)) : root.ui.c.raised
                    Shape {
                        anchors.centerIn: parent
                        width: root.ui.px(18)
                        height: width
                        preferredRendererType: Shape.CurveRenderer
                        ShapePath {
                            strokeColor: fileRow.focused ? root.ui.c.accentHi : root.ui.c.soft
                            strokeWidth: 2
                            fillColor: "transparent"
                            capStyle: ShapePath.RoundCap
                            joinStyle: ShapePath.RoundJoin
                            scale: Qt.size(root.ui.px(18) / 24, root.ui.px(18) / 24)
                            PathSvg { path: "M14 3H6a1 1 0 0 0-1 1v16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V8z M14 3v5h5 M10 5h1 M10 8h1 M10 11h1 M10 14v3h1v-3z" }
                        }
                    }
                }

                Row {
                    id: fileEnd
                    visible: fileRow.focused
                    anchors.right: parent.right
                    anchors.rightMargin: root.ui.px(12)
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: root.ui.px(6)
                    KeyCap { ui: root.ui; k: "a"; anchors.verticalCenter: parent.verticalCenter }
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: root.actionLabel || ""
                        color: root.ui.c.soft
                        font.family: root.ui.bodyFont
                        font.pixelSize: root.ui.fpx(14)
                    }
                }

                Column {
                    id: fileText
                    anchors.left: zipTile.right
                    anchors.leftMargin: root.ui.px(12)
                    anchors.right: fileEnd.visible ? fileEnd.left : parent.right
                    anchors.rightMargin: root.ui.px(12)
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: root.ui.px(1)
                    Text {
                        width: parent.width
                        text: root.niceName(fileRow.modelData.name || "")
                        elide: Text.ElideRight
                        color: root.ui.c.text
                        font.family: root.ui.bodyFont
                        font.pixelSize: root.ui.fpx(16)
                        font.weight: Font.DemiBold
                    }
                    Text {
                        width: parent.width
                        text: [root.niceName(fileRow.modelData.name || "") !== (fileRow.modelData.name || "") ? fileRow.modelData.name : "", fileRow.modelData.size || "", fileRow.modelData.age || "", fileRow.note].filter(t => t !== "").join(" · ")
                        elide: Text.ElideRight
                        color: fileRow.focused ? root.ui.c.soft : root.ui.c.muted
                        font.family: root.ui.bodyFont
                        font.pixelSize: root.ui.fpx(14)
                    }
                }
            }
        }

        // Nothing to add yet.
        Column {
            visible: root.count === 0
            anchors.top: head.bottom
            anchors.topMargin: root.ui.px(18)
            x: root.ui.px(18)
            width: parent.width - root.ui.px(36)
            spacing: root.ui.px(8)
            Text {
                width: parent.width
                text: "Nothing to add yet"
                color: root.ui.c.text
                font.family: root.ui.bodyFont
                font.pixelSize: root.ui.fpx(16)
                font.weight: Font.DemiBold
            }
            Text {
                width: parent.width
                text: root.emptyText || ""
                wrapMode: Text.WordWrap
                color: root.ui.c.soft
                font.family: root.ui.bodyFont
                font.pixelSize: root.ui.fpx(15)
            }
        }
    }

    // A controller button drawn as a small cap.
    component KeyCap: Rectangle {
        required property var ui
        property string k: ""
        implicitWidth: Math.max(ui.px(24), capLabel.implicitWidth + ui.px(12))
        implicitHeight: ui.px(24)
        width: implicitWidth
        height: implicitHeight
        radius: k.length > 1 ? ui.px(6) : height / 2
        color: ui.c.raised
        border.color: ui.c.accentLo
        border.width: Math.max(1, ui.px(1))
        Text {
            id: capLabel
            anchors.centerIn: parent
            text: parent.k.toUpperCase()
            color: parent.ui.c.text
            font.family: parent.ui.bodyFont
            font.pixelSize: parent.ui.fpx(14)
            font.bold: true
        }
    }
}
