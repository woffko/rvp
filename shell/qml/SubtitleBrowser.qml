// The subtitle browser and editor (PotPlayer's Browse/Edit/Sync Subtitles,
// Alt+E) and Input Subtitles (Alt+P), loaded on first use: tool windows of
// their own beside the video. The cues come from
// Player.subtitle_browser_json; every edit goes to Player.subtitle_edit,
// which shows it in the video at once (rvp_app::subtitle_editor).
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import Rvp

Item {
    id: holder
    required property Player player
    property var data: ({ error: "", name: "", cues: [], speed: 1, delay: 0, settings: {} })
    readonly property var settings: data.settings || {}
    // The chosen cue, and the one that shows now (or the next).
    property int selected: -1
    property int current: -1
    readonly property alias browser: browserWindow
    readonly property alias input: inputWindow
    // The chosen cue's text field (self-test).
    readonly property alias textArea: textArea

    function ask(typing) {
        if (typing) {
            inputWindow.show()
            inputWindow.raise()
            inputWindow.requestActivate()
            inputText.forceActiveFocus()
        } else {
            reload()
            browserWindow.show()
            browserWindow.raise()
            browserWindow.requestActivate()
        }
    }

    function reload() {
        holder.data = JSON.parse(holder.player.subtitle_browser_json())
        if (holder.selected >= holder.data.cues.length)
            holder.selected = holder.data.cues.length - 1
        holder.fillEditor()
    }

    function edit(op, index, text) {
        holder.player.subtitle_edit(op, index, text)
        holder.reload()
    }

    // `0:01:02.500` for milliseconds.
    function clock(ms) {
        const sign = ms < 0 ? "-" : ""
        let rest = Math.abs(Math.round(ms))
        const hours = Math.floor(rest / 3600000)
        const minutes = Math.floor(rest / 60000) % 60
        const seconds = Math.floor(rest / 1000) % 60
        const millis = rest % 1000
        return sign + hours + ":" + String(minutes).padStart(2, "0") + ":"
            + String(seconds).padStart(2, "0") + "." + String(millis).padStart(3, "0")
    }
    function parseClock(text) {
        const match = /^\s*(-?)(?:(\d+):)?(\d+):(\d+)(?:[.,](\d{1,3}))?\s*$/.exec(text)
        if (!match)
            return NaN
        const ms = ((Number(match[2] || 0) * 60 + Number(match[3])) * 60 + Number(match[4])) * 1000
            + Number((match[5] || "0").padEnd(3, "0"))
        return match[1] === "-" ? -ms : ms
    }
    // A cue's time as the browser shows it: as it plays (with the sync) or
    // as the file has it.
    function shown(ms) {
        return holder.settings["synced-times"] ? ms * holder.data.speed + holder.data.delay : ms
    }
    function own(ms) {
        return holder.settings["synced-times"] ? (ms - holder.data.delay) / holder.data.speed : ms
    }
    function timesText(cue) {
        switch (holder.settings.times) {
        case "start":
            return holder.clock(holder.shown(cue.start))
        case "none":
            return ""
        default:
            return holder.clock(holder.shown(cue.start)) + " - " + holder.clock(holder.shown(cue.end))
        }
    }
    function fillEditor() {
        const cue = holder.data.cues[holder.selected]
        startField.text = cue ? holder.clock(holder.shown(cue.start)) : ""
        endField.text = cue ? holder.clock(holder.shown(cue.end)) : ""
        textArea.text = cue ? cue.text : ""
    }
    function choose(index) {
        holder.selected = index
        cueList.currentIndex = index
        holder.fillEditor()
    }
    // The editor's fields go into the chosen cue.
    function applyEditor() {
        const cue = holder.data.cues[holder.selected]
        if (!cue)
            return
        const index = holder.selected
        if (textArea.text !== cue.text)
            holder.player.subtitle_edit("text", index, textArea.text)
        const start = holder.parseClock(startField.text)
        const end = holder.parseClock(endField.text)
        if (!isNaN(end) && Math.round(holder.own(end)) !== cue.end)
            holder.player.subtitle_edit("end", index, holder.clock(holder.own(end)))
        if (!isNaN(start) && Math.round(holder.own(start)) !== cue.start)
            holder.player.subtitle_edit("start", index, holder.clock(holder.own(start)))
        holder.reload()
    }
    function syncText() {
        const parts = []
        if (Math.abs(holder.data.delay) >= 1)
            parts.push("sync " + (holder.data.delay > 0 ? "+" : "") + (holder.data.delay / 1000).toFixed(3) + " s")
        if (Math.abs(holder.data.speed - 1) > 1e-6)
            parts.push("rate x" + holder.data.speed.toFixed(4))
        return parts.join(", ")
    }

    Window {
        id: browserWindow
        objectName: "subtitleBrowser"
        transientParent: holder.Window.window
        flags: Qt.Tool
        width: 620
        height: 520
        minimumWidth: 420
        minimumHeight: 320
        title: "Subtitle Browser" + (holder.data.name ? " - " + holder.data.name : "")
            + (holder.data.dirty ? " *" : "")
        opacity: (holder.settings.opacity || 100) / 100

        Timer {
            interval: 250
            repeat: true
            running: browserWindow.visible
            onTriggered: {
                const now = holder.player.subtitle_cue_now()
                if (now === holder.current)
                    return
                holder.current = now
                if (now >= 0 && holder.settings.follow)
                    cueList.positionViewAtIndex(now, ListView.Contain)
            }
        }

        Page {
            anchors.fill: parent
            padding: 8
            ColumnLayout {
                anchors.fill: parent
                spacing: 6
                Label {
                    Layout.fillWidth: true
                    visible: text !== ""
                    wrapMode: Text.Wrap
                    text: holder.data.error !== "" ? holder.data.error : holder.syncText()
                }
                ListView {
                    id: cueList
                    objectName: "subtitleCueList"
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    model: holder.data.cues
                    ScrollBar.vertical: ScrollBar {}
                    delegate: ItemDelegate {
                        id: row
                        required property var modelData
                        required property int index
                        width: ListView.view.width
                        highlighted: index === holder.selected
                        onClicked: holder.choose(index)
                        onDoubleClicked: holder.edit("jump", index, "")
                        background: Rectangle {
                            color: row.highlighted ? "#40608cff"
                                : row.index === holder.current ? "#30ffd040" : "transparent"
                        }
                        contentItem: RowLayout {
                            spacing: 10
                            Label {
                                text: holder.timesText(row.modelData)
                                visible: text !== ""
                                font.family: "monospace"
                                font.pointSize: holder.settings["font-size"] || 10
                                Layout.alignment: Qt.AlignTop
                            }
                            Label {
                                Layout.fillWidth: true
                                text: row.modelData.text
                                textFormat: Text.PlainText
                                wrapMode: Text.Wrap
                                elide: Text.ElideRight
                                maximumLineCount: holder.settings.lines || 2
                                font.family: holder.settings.font || Qt.application.font.family
                                font.pointSize: holder.settings["font-size"] || 10
                            }
                        }
                    }
                }
                GridLayout {
                    columns: 4
                    Layout.fillWidth: true
                    Label { text: "Start" }
                    TextField { id: startField; objectName: "subtitleStartField"; Layout.preferredWidth: 130 }
                    Button {
                        text: "Now"
                        enabled: holder.selected >= 0
                        onClicked: holder.edit("start-now", holder.selected, "")
                    }
                    Button {
                        text: "Sync here"
                        enabled: holder.selected >= 0
                        ToolTip.visible: hovered
                        ToolTip.text: "Move this subtitle and the later ones so that it starts now"
                        onClicked: holder.edit("sync-here", holder.selected, "")
                    }
                    Label { text: "End" }
                    TextField { id: endField; objectName: "subtitleEndField"; Layout.preferredWidth: 130 }
                    Button {
                        text: "Now"
                        enabled: holder.selected >= 0
                        onClicked: holder.edit("end-now", holder.selected, "")
                    }
                    Item { Layout.fillWidth: true }
                }
                ScrollView {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 70
                    TextArea {
                        id: textArea
                        objectName: "subtitleTextArea"
                        wrapMode: TextEdit.Wrap
                        placeholderText: "The chosen subtitle's text"
                    }
                }
                Flow {
                    Layout.fillWidth: true
                    spacing: 4
                    Button {
                        text: "Apply"
                        enabled: holder.selected >= 0
                        onClicked: holder.applyEditor()
                    }
                    Button { text: "Insert"; onClicked: { holder.edit("insert", holder.selected, ""); holder.choose(holder.player.subtitle_cue_now()) } }
                    Button { text: "Delete"; enabled: holder.selected >= 0; onClicked: holder.edit("delete", holder.selected, "") }
                    Button { text: "Split"; enabled: holder.selected >= 0; onClicked: holder.edit("split", holder.selected, "") }
                    Button { text: "Join"; enabled: holder.selected >= 0; onClicked: holder.edit("join", holder.selected, "") }
                    Button { text: "Undo"; enabled: holder.data.undo === true; onClicked: holder.edit("undo", -1, "") }
                    Button { text: "Redo"; enabled: holder.data.redo === true; onClicked: holder.edit("redo", -1, "") }
                    TextField {
                        id: shiftField
                        width: 80
                        placeholderText: "ms"
                        validator: IntValidator { bottom: -3600000; top: 3600000 }
                    }
                    Button {
                        text: "Shift from here"
                        enabled: holder.selected >= 0 && shiftField.acceptableInput
                        onClicked: holder.edit("shift", holder.selected, shiftField.text)
                    }
                    Button {
                        text: "Save"
                        enabled: holder.data.cues.length > 0
                        ToolTip.visible: hovered
                        ToolTip.text: "Save into the file, with the sync"
                        onClicked: holder.edit("save", -1, "")
                    }
                    Button { text: "Save As…"; enabled: holder.data.cues.length > 0; onClicked: holder.edit("save-as", -1, "") }
                }
            }
        }
    }

    Window {
        id: inputWindow
        objectName: "inputSubtitles"
        transientParent: holder.Window.window
        flags: Qt.Tool
        width: 460
        height: 200
        minimumWidth: 320
        minimumHeight: 160
        title: "Input Subtitles"

        Page {
            anchors.fill: parent
            padding: 8
            ColumnLayout {
                anchors.fill: parent
                Label {
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                    text: "Type a subtitle and press Enter: it shows from where you began typing. Shift+Enter starts a new line."
                }
                ScrollView {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    TextArea {
                        id: inputText
                        objectName: "inputSubtitleText"
                        wrapMode: TextEdit.Wrap
                        onTextChanged: {
                            if (text !== "" && !inputText.begun) {
                                inputText.begun = true
                                holder.player.subtitle_edit("input-begin", -1, "")
                            }
                        }
                        property bool begun: false
                        Keys.onEnterPressed: (event) => inputText.enter(event)
                        Keys.onReturnPressed: (event) => inputText.enter(event)
                        function enter(event) {
                            if (event.modifiers & Qt.ShiftModifier) {
                                event.accepted = false
                                return
                            }
                            holder.player.subtitle_edit("input", -1, inputText.text)
                            inputText.begun = false
                            inputText.text = ""
                            if (browserWindow.visible)
                                holder.reload()
                        }
                    }
                }
                RowLayout {
                    Button { text: "Browse/Edit…"; onClicked: holder.ask(false) }
                    Item { Layout.fillWidth: true }
                    Button { text: "Save"; onClicked: holder.player.subtitle_edit("save", -1, "") }
                    Button { text: "Save As…"; onClicked: holder.player.subtitle_edit("save-as", -1, "") }
                }
            }
        }
    }
}
