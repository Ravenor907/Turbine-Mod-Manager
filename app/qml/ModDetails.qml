import QtQuick

// A mod's page, pushed over Get mods. Left (60%): the largest picture the source gives, name, author, version,
// size, updated date, likes (Nexus endorsements) and downloads, then the summary. Right (40%): the main button with one line
// under it ("Get it on Nexus", "Install", "Installed · On"), "Other files", then "Also needs" (each requirement with
// Installed, Will be added or Not yet; A on one opens its page). On a narrow window the right column goes under the
// picture. Main.qml moves the focus (focusIndex: 0 main button, 1 Other files, 2 and on the requirements) and scrolls
// with the D-pad (scroll()). After 8 seconds with Turbine still in front, two buttons show under the main one
// ("Open in Steam's browser", "Always use Steam's browser": helpLabels, helpFocus, helpClicked). No key caps on the
// page: the legend says what A and X do.
Item {
    id: md
    required property var ui
    property var info: null               // the engine's details answer
    property string fallbackPicture: ""   // the card's thumbnail, while the page has no picture of its own
    property bool focused: true
    property string mainLabel: ""
    property string mainLine: ""
    property bool fileRows: false         // the main file's line and the Other files row (else under X Options)
    property bool mainBusy: false
    property real mainProgress: -1
    property int focusIndex: 0
    property var helpLabels: []           // the two browser buttons, when they show
    property int helpFocus: -1

    signal installClicked()
    signal helpClicked(int index)
    signal versionsClicked()
    signal needClicked(int index)

    readonly property bool loading: !info || info.loading === true
    readonly property int otherCount: info && info.versions ? info.versions.length : 0
    readonly property bool stacked: width < ui.px(640)
    // Nexus requirements: {missing, installed, unchecked, steps}.
    readonly property var needs: info && info.needs ? info.needs : null
    function stepFor(name) { return needs ? (needs.steps || []).find(x => x.name === name) : undefined }
    readonly property var needRows: !needs ? [] :
        (needs.installed || []).map(r => ({ tag: "Installed", lvl: "ok", r: r }))
        .concat((needs.missing || []).map(r => {
            const st = stepFor(r.name)
            return { tag: st && st.action === "download" ? "Will be added" : "Not yet", lvl: "", r: r }
        }))
        .concat((needs.unchecked || []).map(r => ({ tag: r.isDlc ? "Game DLC" : "Outside Nexus", lvl: "", r: r })))
    // Thunderstore: what Install brings along.
    readonly property var install: info && info.install ? info.install : []
    readonly property var extra: install.filter(x => !x.main)
    function size(b) {
        if (!b) return "Not listed"
        if (b >= 1073741824) return (b / 1073741824).toFixed(1) + " GB"
        if (b >= 1048576) return (b / 1048576).toFixed(1) + " MB"
        if (b >= 1024) return Math.round(b / 1024) + " KB"
        return b + " bytes"
    }
    // "2025-03-04" reads as "Mar 4, 2025"; anything else stays as it is.
    function niceDate(t) {
        const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(t || "")
        return m ? Qt.formatDate(new Date(+m[1], +m[2] - 1, +m[3]), "MMM d, yyyy") : (t || "")
    }
    // A size stays together and with what comes before it ("24.0 MB" never breaks before MB).
    function keepTogether(t) { return (t || "").replace(/ (bytes|KB|MB|GB)\b/g, "\u00a0$1").replace(/ · /g, "\u00a0· ") }
    function count(n) {
        n = n || 0
        if (n >= 1000000) return (n / 1000000).toFixed(1).replace(".0", "") + "M"
        if (n >= 1000) return Math.round(n / 1000) + "K"
        return String(n)
    }
    function scroll(d) {
        view.contentY = Math.max(0, Math.min(Math.max(0, view.contentHeight - view.height), view.contentY + d * ui.px(70)))
    }
    function focusItem() {
        if (helpFocus >= 0) return helpRep.itemAt(helpFocus)
        return focusIndex === 0 ? mainBtn : focusIndex === 1 ? otherRow : needRep.itemAt(focusIndex - 2)
    }
    onHelpFocusChanged: follow()
    onFocusIndexChanged: follow()
    function follow() { Qt.callLater(md.followNow) }
    function followNow() {
        const it = md.focusItem()
        if (!it) return
        const p = it.mapToItem(view.contentItem, 0, 0)
        if (p.y < view.contentY) view.contentY = Math.max(0, p.y - ui.px(8))
        else if (p.y + it.height > view.contentY + view.height) view.contentY = p.y + it.height - view.height + ui.px(8)
    }
    onInfoChanged: view.contentY = 0

    Text {
        // Only while reading: a page that couldn't be read is still shown (Main.qml fills it from the card).
        visible: md.loading || !!(md.info && md.info.error)
        anchors.centerIn: parent
        width: Math.min(parent.width, md.ui.px(520))
        horizontalAlignment: Text.AlignHCenter
        wrapMode: Text.WordWrap
        text: md.loading ? "Reading the mod's page..." : (md.info.error || "")
        color: md.ui.c.soft
        font.family: md.ui.bodyFont
        font.pixelSize: md.ui.px(16)
    }

    Flickable {
        id: view
        visible: !md.loading && !(md.info && md.info.error)
        anchors.fill: parent
        contentHeight: md.stacked ? leftCol.implicitHeight + rightCol.implicitHeight + md.ui.px(24) : Math.max(leftCol.implicitHeight, rightCol.implicitHeight) + md.ui.px(8)
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        Column {
            id: leftCol
            width: md.stacked ? view.width : view.width * 0.6 - md.ui.px(8)
            spacing: md.ui.px(8)
            Rectangle {
                width: parent.width; height: Math.min(width * 9 / 16, md.ui.px(300), view.height * 0.4)
                radius: md.ui.px(6)
                color: md.ui.c.raised
                clip: true
                Image {
                    id: pic
                    anchors.fill: parent
                    source: md.info ? (md.info.picture || md.info.icon || md.fallbackPicture) : md.fallbackPicture
                    fillMode: Image.PreserveAspectCrop
                    asynchronous: true
                    visible: status === Image.Ready
                }
                Text {
                    visible: !pic.visible
                    anchors.centerIn: parent
                    text: md.info && md.info.name ? md.info.name.substring(0, 1).toUpperCase() : ""
                    color: md.ui.c.muted
                    font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(48); font.bold: true
                }
            }
            Text {
                width: parent.width
                wrapMode: Text.WordWrap
                text: md.info && md.info.name ? md.info.name : ""
                color: md.ui.c.text; font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(22); font.weight: Font.DemiBold
            }
            Text {
                width: parent.width
                wrapMode: Text.WordWrap
                text: !md.info || md.loading ? "" : [
                    md.info.owner ? "By " + md.info.owner : "",
                    md.info.version ? "Version " + md.info.version : "",
                    md.info.size ? md.size(md.info.size) : "",
                    (md.info.updatedText || md.info.updated) ? "Updated " + md.niceDate(md.info.updatedText || md.info.updated) : ""
                ].filter(x => x !== "").join(" · ")
                color: md.ui.c.soft; font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(14)
            }
            Text {
                text: !md.info || md.loading ? "" : md.count(md.info.rating) + (md.info.source === "nexus" ? " likes" : " ratings") + " · " + md.count(md.info.downloads) + " downloads"
                color: md.ui.c.muted; font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(14)
            }
            Text {
                width: parent.width
                wrapMode: Text.WordWrap
                textFormat: Text.PlainText
                text: md.info && md.info.summary ? md.info.summary : ""
                color: md.ui.c.soft
                font.family: md.ui.bodyFont
                font.pixelSize: md.ui.px(15)
            }
        }
        Column {
            id: rightCol
            x: md.stacked ? 0 : view.width * 0.6 + md.ui.px(8)
            y: md.stacked ? leftCol.implicitHeight + md.ui.px(16) : 0
            width: md.stacked ? view.width : view.width * 0.4 - md.ui.px(8)
            spacing: md.ui.px(10)
            Rectangle {
                id: mainBtn
                readonly property bool foc: md.focused && md.focusIndex === 0
                width: parent.width
                height: md.ui.px(48)
                radius: md.ui.px(8)
                // Filled brass only with the focus; otherwise a brass outline, so two things never both look focused
                // (the browser buttons under it can have the focus).
                color: md.ui.c.card
                border.color: foc ? md.ui.c.text : md.ui.c.accent
                border.width: md.ui.px(2)
                Rectangle {
                    visible: mainBtn.foc
                    anchors.fill: parent
                    anchors.margins: mainBtn.border.width
                    radius: mainBtn.radius - mainBtn.border.width
                    gradient: Gradient {
                        GradientStop { position: 0; color: md.ui.c.accentHi }
                        GradientStop { position: 1; color: md.ui.c.accent }
                    }
                }
                Rectangle {
                    visible: md.mainProgress >= 0
                    anchors.left: parent.left; anchors.bottom: parent.bottom
                    anchors.margins: md.ui.px(3)
                    width: (parent.width - md.ui.px(6)) * Math.max(0, Math.min(1, md.mainProgress))
                    height: md.ui.px(4); radius: md.ui.px(2)
                    color: md.ui.c.onAccent
                    opacity: 0.5
                }
                Text { anchors.centerIn: parent; text: md.mainLabel; color: mainBtn.foc ? md.ui.c.onAccent : md.ui.c.accentText; font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(18); font.bold: true }
                HoverHandler { onHoveredChanged: md.ui.hover(parent, hovered) }
                TapHandler { onTapped: md.installClicked() }
            }
            Text {
                visible: text !== ""
                width: parent.width
                wrapMode: Text.WordWrap
                text: md.mainLine
                color: md.ui.c.soft
                font.family: md.ui.bodyFont
                font.pixelSize: md.ui.px(14)
            }
            Text {
                visible: md.helpLabels.length > 0
                width: parent.width
                wrapMode: Text.WordWrap
                text: "Didn't see Nexus open?"
                color: md.ui.c.text
                font.family: md.ui.bodyFont
                font.pixelSize: md.ui.px(14)
                font.weight: Font.DemiBold
            }
            Repeater {
                id: helpRep
                model: md.helpLabels
                Rectangle {
                    required property int index
                    required property string modelData
                    readonly property bool foc: md.focused && md.helpFocus === index
                    width: rightCol.width; height: md.ui.px(44); radius: md.ui.px(6)
                    color: foc ? md.ui.c.focusFill : md.ui.c.card
                    border.color: foc ? md.ui.c.accent : md.ui.c.line
                    border.width: foc ? md.ui.px(2) : Math.max(1, md.ui.px(1))
                    Text { x: md.ui.px(12); anchors.verticalCenter: parent.verticalCenter; text: parent.modelData; color: md.ui.c.text; font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(15); font.weight: Font.DemiBold }
                    HoverHandler { onHoveredChanged: md.ui.hover(parent, hovered) }
                    TapHandler { onTapped: md.helpClicked(parent.index) }
                }
            }
            // Which file and its size, and Other files: under X Options (a first look shouldn't ask for a choice).
            Text {
                visible: md.fileRows && !!md.info && !!md.info.mainLine
                width: parent.width
                wrapMode: Text.WordWrap
                text: md.info && md.info.mainLine ? md.keepTogether(md.info.mainLine) : ""
                color: md.ui.c.text
                font.family: md.ui.bodyFont
                font.pixelSize: md.ui.px(14)
            }
            Text {
                visible: !!md.info && !!md.info.filesNote
                width: parent.width
                wrapMode: Text.WordWrap
                text: md.info && md.info.filesNote ? md.info.filesNote : ""
                color: md.ui.c.soft
                font.family: md.ui.bodyFont
                font.pixelSize: md.ui.px(14)
            }
            Rectangle {
                id: otherRow
                visible: md.fileRows && md.otherCount > 0
                width: parent.width; height: md.ui.px(40); radius: md.ui.px(6)
                color: md.focused && md.focusIndex === 1 ? md.ui.c.focusFill : md.ui.c.card
                border.color: md.ui.c.accent
                border.width: md.focused && md.focusIndex === 1 ? md.ui.px(2) : 0
                Row {
                    x: md.ui.px(12)
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: md.ui.px(8)
                    Text { text: "Other files (" + md.otherCount + ")"; color: md.ui.c.text; font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(15); font.weight: Font.DemiBold; anchors.verticalCenter: parent.verticalCenter }
                }
                HoverHandler { onHoveredChanged: md.ui.hover(parent, hovered) }
                TapHandler { onTapped: md.versionsClicked() }
            }
            // Thunderstore: what Install brings along.
            Text {
                visible: md.extra.length > 0
                width: parent.width
                wrapMode: Text.WordWrap
                text: "Also installs: " + md.extra.map(x => x.name + " " + x.version).join(", ")
                color: md.ui.c.soft; font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(14)
            }
            Text {
                visible: md.needRows.length > 0 || (!!md.info && !!md.info.needsNote)
                text: "Also needs"
                color: md.ui.c.accentText
                font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(16); font.weight: Font.DemiBold
            }
            Text {
                visible: !!md.info && !!md.info.needsNote
                width: parent.width; wrapMode: Text.WordWrap
                text: md.info && md.info.needsNote ? md.info.needsNote : ""
                color: md.ui.c.soft; font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(14)
            }
            Repeater {
                id: needRep
                model: md.needRows
                Rectangle {
                    id: nr
                    required property int index
                    required property var modelData
                    readonly property bool foc: md.focused && md.focusIndex === index + 2
                    width: rightCol.width; height: md.ui.px(44); radius: md.ui.px(6)
                    color: foc ? md.ui.c.focusFill : md.ui.c.card
                    border.color: md.ui.c.accent
                    border.width: foc ? md.ui.px(2) : 0
                    Text {
                        x: md.ui.px(12)
                        width: parent.width - tag.width - md.ui.px(30)
                        anchors.verticalCenter: parent.verticalCenter
                        elide: Text.ElideRight
                        text: nr.modelData.r.name
                        color: md.ui.c.text; font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(15); font.weight: Font.DemiBold
                    }
                    Rectangle {
                        id: tag
                        anchors.right: parent.right; anchors.rightMargin: md.ui.px(8)
                        anchors.verticalCenter: parent.verticalCenter
                        width: tt.implicitWidth + md.ui.px(14); height: md.ui.px(26); radius: md.ui.px(5)
                        color: md.ui.c.raised
                        Text { id: tt; anchors.centerIn: parent; text: nr.modelData.tag; color: nr.modelData.lvl === "ok" ? md.ui.c.ok : md.ui.c.text; font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(13); font.bold: true }
                    }
                    HoverHandler { onHoveredChanged: md.ui.hover(parent, hovered) }
                    TapHandler { onTapped: md.needClicked(nr.index) }
                }
            }
        }
    }

    // More below: a fade at the bottom and how to scroll (0.56).
    Rectangle {
        visible: view.visible && view.contentHeight > view.height + 2 && !view.atYEnd
        anchors.left: parent.left; anchors.right: parent.right; anchors.bottom: parent.bottom
        height: md.ui.px(46)
        gradient: Gradient {
            GradientStop { position: 0; color: "transparent" }
            GradientStop { position: 1; color: md.ui.c.ground }
        }
        Text {
            anchors.right: parent.right; anchors.bottom: parent.bottom; anchors.margins: md.ui.px(4)
            text: "Left stick: scroll"
            color: md.ui.c.soft; font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(13)
        }
    }
}
