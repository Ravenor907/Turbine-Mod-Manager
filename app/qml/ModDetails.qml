import QtQuick

// A mod's details before installing: picture and facts on the left (version, size, updated, downloads, where from),
// then its name, "Code mod", what it does, what else it brings ("Also installs", with Included, Have it and Not
// found), and Install with how much it downloads. A installs; Main.qml handles the keys.
Item {
    id: md
    required property var ui
    property var info: null               // the engine's details answer
    property bool focused: true
    property string installNote: ""       // e.g. "Downloading..." while it runs

    signal installClicked()
    signal versionsClicked()

    readonly property bool loading: !info || info.loading === true
    readonly property var install: info && info.install ? info.install : []
    readonly property var extra: install.filter(x => !x.main)
    readonly property real total: install.reduce((a, x) => a + (x.size || 0), 0)
    readonly property int otherCount: info && info.versions ? info.versions.length : 0
    readonly property bool canInstall: !loading && !info.error && install.length > 0
    // Free Nexus accounts: A opens the mod's page instead of installing.
    readonly property bool openInstead: !loading && !!info && info.premium === false && !!info.page
    // Nexus requirements: {missing, installed, unchecked, steps}.
    readonly property var needs: info && info.needs ? info.needs : null
    function stepFor(name) { return needs ? (needs.steps || []).find(x => x.name === name) : undefined }
    readonly property var needRows: !needs ? [] :
        (needs.missing || []).map(r => {
            const st = stepFor(r.name)
            return { tag: "Missing", lvl: "warn", name: r.name,
                     words: st && st.action === "download" ? "comes with Install" : "its page opens with A" }
        })
        .concat((needs.installed || []).map(r => ({ tag: "Have it", lvl: "ok", name: r.name, words: "already installed" })))
        .concat((needs.unchecked || []).map(r => ({ tag: r.isDlc ? "Game DLC" : "Outside Nexus", lvl: "", name: r.name,
                                                    words: r.isDlc ? "game add-on you need" : "get it from its own site" })))
    function size(b) {
        if (!b) return "Not listed"
        if (b >= 1073741824) return (b / 1073741824).toFixed(1) + " GB"
        if (b >= 1048576) return (b / 1048576).toFixed(1) + " MB"
        if (b >= 1024) return Math.round(b / 1024) + " KB"
        return b + " bytes"
    }
    function count(n) {
        if (n >= 1000000) return (n / 1000000).toFixed(1).replace(".0", "") + "M"
        if (n >= 1000) return Math.round(n / 1000) + "K"
        return String(n || 0)
    }
    readonly property string installWords: {
        if (!info || loading) return ""
        if (info.message) return info.message
        if (install.length === 0) return (info.already || []).length > 0 ? "You have it, and everything it needs." : "Nothing to download."
        const main = install.find(x => x.main)
        let w = size(main ? main.size : 0)
        if (extra.length > 0) w += " plus " + size(total - (main ? main.size : 0)) + " for " + extra.map(x => x.name).join(", ")
        return w
    }

    Text {
        visible: md.loading || !!(md.info && md.info.error)
        anchors.centerIn: parent
        width: Math.min(parent.width, md.ui.px(520))
        horizontalAlignment: Text.AlignHCenter
        wrapMode: Text.WordWrap
        text: md.loading ? "Reading the details..." : (md.info.error || "")
        color: md.ui.c.soft
        font.family: md.ui.bodyFont
        font.pixelSize: md.ui.px(16)
    }

    Flickable {
        id: view
        visible: !md.loading && !(md.info && md.info.error)
        anchors.fill: parent
        contentHeight: Math.max(left.implicitHeight, right.implicitHeight) + md.ui.px(8)
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        Column {
            id: left
            width: Math.min(md.ui.px(280), parent.width * 0.34)
            spacing: md.ui.px(8)
            Rectangle {
                width: parent.width; height: width * 0.56
                radius: md.ui.px(8)
                color: md.ui.c.raised
                clip: true
                Image {
                    id: pic
                    anchors.fill: parent
                    source: md.info && md.info.icon ? md.info.icon : ""
                    fillMode: Image.PreserveAspectFit
                    asynchronous: true
                    visible: status === Image.Ready
                }
                Text {
                    visible: !pic.visible
                    anchors.centerIn: parent
                    text: md.info && md.info.name ? md.info.name.substring(0, 2).toUpperCase() : ""
                    color: md.ui.c.soft
                    font.family: md.ui.titleFont; font.pixelSize: md.ui.px(30); font.bold: true
                }
            }
            Rectangle {
                width: parent.width
                height: facts.implicitHeight + md.ui.px(16)
                radius: md.ui.px(8)
                color: md.ui.c.card
                Column {
                    id: facts
                    x: md.ui.px(12); y: md.ui.px(8)
                    width: parent.width - md.ui.px(24)
                    spacing: md.ui.px(4)
                    Repeater {
                        model: !md.info || md.loading ? [] : [
                            ["Version", md.info.version || "Not listed"],
                            ["Size", md.size(md.info.size)],
                            ["Updated", md.info.updated || "Not listed"],
                            ["Downloads", md.count(md.info.downloads)],
                            ["By", md.info.owner || ""],
                            ["From", md.info.source === "nexus" ? "Nexus Mods" : "Thunderstore"]
                        ]
                        Item {
                            required property var modelData
                            width: facts.width; height: md.ui.px(22)
                            Text { text: parent.modelData[0]; color: md.ui.c.muted; font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(14); anchors.verticalCenter: parent.verticalCenter }
                            Text {
                                anchors.right: parent.right; anchors.verticalCenter: parent.verticalCenter
                                width: parent.width * 0.62; horizontalAlignment: Text.AlignRight; elide: Text.ElideRight
                                text: parent.modelData[1]; color: md.ui.c.text; font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(14); font.bold: true
                            }
                        }
                    }
                }
            }
        }
        Column {
            id: right
            x: left.width + md.ui.px(16)
            width: view.width - x
            spacing: md.ui.px(10)
            Row {
                spacing: md.ui.px(10)
                Text { text: md.info && md.info.name ? md.info.name : ""; color: md.ui.c.text; font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(22); font.bold: true }
                Rectangle {
                    visible: !!md.info && md.info.code === true
                    anchors.verticalCenter: parent.verticalCenter
                    width: cm.implicitWidth + md.ui.px(14); height: md.ui.px(26); radius: md.ui.px(5)
                    color: md.ui.c.raised
                    Text { id: cm; anchors.centerIn: parent; text: "Code mod"; color: md.ui.c.text; font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(14); font.bold: true }
                }
                Rectangle {
                    visible: !!md.info && md.info.deprecated === true
                    anchors.verticalCenter: parent.verticalCenter
                    width: dp.implicitWidth + md.ui.px(14); height: md.ui.px(26); radius: md.ui.px(5)
                    color: md.ui.c.raised
                    Text { id: dp; anchors.centerIn: parent; text: "No longer looked after"; color: md.ui.c.warn; font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(14); font.bold: true }
                }
            }
            Text {
                width: parent.width
                wrapMode: Text.WordWrap
                textFormat: Text.PlainText
                maximumLineCount: 6
                elide: Text.ElideRight
                text: md.info && md.info.summary ? md.info.summary : ""
                color: md.ui.c.soft
                font.family: md.ui.bodyFont
                font.pixelSize: md.ui.px(16)
            }
            // What else it brings
            Column {
                visible: md.extra.length + (md.info && md.info.already ? md.info.already.length : 0)
                         + (md.info && md.info.skippedLoaders ? md.info.skippedLoaders.length : 0) + (md.info && md.info.missing ? md.info.missing.length : 0) > 0
                width: parent.width
                spacing: md.ui.px(6)
                Text {
                    text: md.extra.length > 0 ? "Also installs: " + md.extra.map(x => x.name + " " + x.version).join(", ") : "What it needs"
                    color: md.ui.c.accentText
                    font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(16); font.bold: true
                    width: parent.width; elide: Text.ElideRight
                }
                Repeater {
                    model: !md.info || md.loading ? [] :
                        md.extra.map(x => ({ tag: "Included", lvl: "ok", name: x.name + " " + x.version, words: x.summary || "" }))
                        .concat((md.info.already || []).map(x => ({ tag: "Have it", lvl: "", name: x, words: "already installed" })))
                        .concat((md.info.skippedLoaders || []).map(x => ({ tag: "Have it", lvl: "", name: x, words: "the mod loader, already installed" })))
                        .concat((md.info.missing || []).map(x => ({ tag: "Not found", lvl: "warn", name: x, words: "not on Thunderstore; the mod may not work without it" })))
                    Row {
                        required property var modelData
                        spacing: md.ui.px(8)
                        width: right.width
                        Rectangle {
                            width: tg.implicitWidth + md.ui.px(14); height: md.ui.px(26); radius: md.ui.px(5)
                            color: md.ui.c.raised
                            Text { id: tg; anchors.centerIn: parent; text: parent.parent.modelData.tag; color: parent.parent.modelData.lvl === "ok" ? md.ui.c.ok : parent.parent.modelData.lvl === "warn" ? md.ui.c.warn : md.ui.c.text; font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(14); font.bold: true }
                        }
                        Text { text: parent.modelData.name; color: md.ui.c.text; font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(15); font.bold: true; anchors.verticalCenter: parent.verticalCenter }
                        Text { width: parent.width * 0.5; elide: Text.ElideRight; text: parent.modelData.words; color: md.ui.c.soft; font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(14); anchors.verticalCenter: parent.verticalCenter }
                    }
                }
            }
            Row {
                spacing: md.ui.px(12)
                Rectangle {
                    visible: md.canInstall
                    width: ir.implicitWidth + md.ui.px(30); height: md.ui.px(44); radius: md.ui.px(8)
                    gradient: Gradient {
                        GradientStop { position: 0; color: md.ui.c.accentHi }
                        GradientStop { position: 1; color: md.ui.c.accent }
                    }
                    border.color: md.ui.c.text
                    border.width: md.focused ? md.ui.px(2) : 0
                    Row {
                        id: ir
                        anchors.centerIn: parent
                        spacing: md.ui.px(8)
                        Rectangle {
                            width: md.ui.px(26); height: width; radius: width / 2
                            anchors.verticalCenter: parent.verticalCenter
                            color: Qt.rgba(0, 0, 0, 0.18)
                            Text { anchors.centerIn: parent; text: "A"; color: md.ui.c.onAccent; font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(14); font.bold: true }
                        }
                        Text { text: md.openInstead ? "Open on Nexus" : md.install.some(x => x.upgrade && x.main) ? "Update" : "Install"; color: md.ui.c.onAccent; font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(17); font.bold: true; anchors.verticalCenter: parent.verticalCenter }
                    }
                    TapHandler { onTapped: md.installClicked() }
                }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    width: right.width - (md.canInstall ? md.ui.px(md.openInstead ? 220 : 160) : 0)
                    wrapMode: Text.WordWrap
                    text: md.installNote !== "" ? md.installNote : md.installWords
                    color: md.info && md.info.message ? md.ui.c.warn : md.ui.c.soft
                    font.family: md.ui.bodyFont
                    font.pixelSize: md.ui.px(15)
                }
            }
            // X: Other versions (older Thunderstore versions, or a Nexus mod's other files).
            Rectangle {
                id: otherRow
                visible: md.otherCount > 0
                width: otherWords.implicitWidth + md.ui.px(48); height: md.ui.px(40); radius: md.ui.px(8)
                color: md.ui.c.card
                border.color: md.ui.c.line
                Row {
                    id: otherWords
                    anchors.centerIn: parent
                    spacing: md.ui.px(8)
                    Rectangle {
                        width: md.ui.px(24); height: width; radius: width / 2; color: md.ui.c.raised; border.color: md.ui.c.accentLo
                        anchors.verticalCenter: parent.verticalCenter
                        Text { anchors.centerIn: parent; text: "X"; color: md.ui.c.text; font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(13); font.bold: true }
                    }
                    Text { text: md.otherCount + (md.otherCount === 1 ? " other version" : " other versions"); color: md.ui.c.text; font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(15); font.bold: true; anchors.verticalCenter: parent.verticalCenter }
                }
                TapHandler { onTapped: md.versionsClicked() }
            }
            // Nexus requirements: "Also needs", with have-it and missing states.
            Column {
                visible: md.needRows.length > 0 || (!!md.info && !!md.info.needsNote)
                width: parent.width
                spacing: md.ui.px(6)
                Text {
                    text: "Also needs"
                    color: md.ui.c.accentText
                    font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(16); font.bold: true
                }
                Text {
                    visible: !!md.info && !!md.info.needsNote
                    width: parent.width; wrapMode: Text.WordWrap
                    text: md.info && md.info.needsNote ? md.info.needsNote : ""
                    color: md.ui.c.soft; font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(14)
                }
                Repeater {
                    model: md.needRows
                    Row {
                        required property var modelData
                        spacing: md.ui.px(8)
                        width: right.width
                        Rectangle {
                            width: Math.max(md.ui.px(84), ntg.implicitWidth + md.ui.px(14)); height: md.ui.px(26); radius: md.ui.px(5)
                            color: md.ui.c.raised
                            Text { id: ntg; anchors.centerIn: parent; text: parent.parent.modelData.tag; color: parent.parent.modelData.lvl === "ok" ? md.ui.c.ok : parent.parent.modelData.lvl === "warn" ? md.ui.c.warn : md.ui.c.text; font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(14); font.bold: true }
                        }
                        Text { width: Math.min(implicitWidth, parent.width * 0.45); elide: Text.ElideRight; text: parent.modelData.name; color: md.ui.c.text; font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(15); font.bold: true; anchors.verticalCenter: parent.verticalCenter }
                        Text { width: parent.width * 0.4; elide: Text.ElideRight; text: parent.modelData.words; color: md.ui.c.soft; font.family: md.ui.bodyFont; font.pixelSize: md.ui.px(14); anchors.verticalCenter: parent.verticalCenter }
                    }
                }
            }
        }
    }
}
