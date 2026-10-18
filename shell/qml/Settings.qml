// The settings window (F5, PotPlayer's Preferences), loaded on first use.
// It shows RVP's own options from settings.json (rvp_app::preferences):
// every change goes through Player.set_setting, which checks it, applies
// it to the running player and saves the file. Only the skin and the
// VSScript library wait for the next start.
import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs as Dialogs
import QtQuick.Layouts
import Rvp
import RvpNative

ApplicationWindow {
    id: win
    required property Player player
    title: "Settings"
    width: 860
    height: 640
    minimumWidth: 640
    minimumHeight: 440

    // Keys of rvp_app::SettingsPage, in the order of the pages in `stack`.
    readonly property var pages: [
        { key: "playback", title: "Playback" },
        { key: "time-jump", title: "Time Jump" },
        { key: "skip", title: "Skip" },
        { key: "section-repeat", title: "Section Repeat" },
        { key: "scene-browser", title: "Scene Browser" },
        { key: "playlist", title: "Playlist" },
        { key: "devices", title: "Device" },
        { key: "audio", title: "Audio" },
        { key: "audio-volume", title: "Volume Control" },
        { key: "equalizer", title: "Equalizer" },
        { key: "normalizer", title: "Normalizer/Freeverb" },
        { key: "audio-effects", title: "Crystality/Noise Reduction" },
        { key: "audio-sync", title: "Language/Sync/Other" },
        { key: "video", title: "Video" },
        { key: "aspect-ratio", title: "Aspect Ratio" },
        { key: "extend-crop", title: "Extend/Crop" },
        { key: "resize", title: "Resize" },
        { key: "frame-size", title: "Frame Size" },
        { key: "stereo", title: "3D and 360°" },
        { key: "shaders", title: "Pixel Shaders" },
        { key: "svp", title: "SVP and VapourSynth" },
        { key: "subtitles", title: "Subtitles" },
        { key: "input", title: "Keys and Mouse" },
        { key: "skins", title: "Skins" },
        { key: "window-size", title: "Window Size" },
        { key: "fullscreen-mode", title: "Fullscreen Mode" },
        { key: "capture", title: "Video Capture" },
        { key: "visualization", title: "Visualization" }
    ]
    // PotPlayer's tree: each group opens into its pages, and the selected
    // group's pages show as tabs over the page. `page` is the group's own
    // page (General has none yet: M14 brings PotPlayer's general options).
    readonly property var tree: [
        { title: "General", page: "", pages: ["input", "skins", "visualization", "playlist"] },
        { title: "Playback", page: "playback",
          pages: ["time-jump", "skip", "section-repeat", "scene-browser", "window-size", "fullscreen-mode"] },
        { title: "Subtitles", page: "subtitles", pages: [] },
        { title: "Device", page: "devices", pages: [] },
        { title: "Video", page: "video",
          pages: ["aspect-ratio", "resize", "extend-crop", "frame-size", "stereo", "shaders", "svp", "capture"] },
        { title: "Audio", page: "audio",
          pages: ["audio-volume", "equalizer", "normalizer", "audio-effects", "audio-sync"] }
    ]
    // The shown page (an index of `pages`) and the open groups.
    property int current: 0
    property var expanded: ({})
    readonly property string currentKey: pages[current].key
    readonly property var currentGroup: groupOf(currentKey)
    readonly property var tabKeys: groupKeys(currentGroup)
    // The tree's rows: the groups, and the pages of the open ones.
    readonly property var rows: {
        const list = []
        for (const group of tree) {
            const keys = groupKeys(group)
            list.push({ title: group.title, key: keys[0], depth: 0, group: group.title,
                        own: group.page !== "", children: group.pages.length > 0 })
            if (expanded[group.title] === true) {
                for (const key of group.pages)
                    list.push({ title: titleOf(key), key: key, depth: 1, group: group.title,
                                own: true, children: false })
            }
        }
        return list
    }
    property var view: ({ settings: {}, choices: {} })
    readonly property var s: view.settings
    property string notice: ""
    property bool restartNeeded: false

    function begin(page) {
        reload(true)
        const index = pages.findIndex(p => p.key === page)
        showPage(index >= 0 ? index : 0)
        notice = ""
        show()
        raise()
        requestActivate()
    }
    function showPage(index) {
        current = index
        const group = groupOf(pages[index].key)
        if (expanded[group.title] !== true)
            expanded = Object.assign({}, expanded, { [group.title]: true })
        tabs.currentIndex = tabKeys.indexOf(pages[index].key)
        if (pages[index].key === "input" && keysPage.item)
            keysPage.item.begin()
    }
    function showKey(key) {
        const index = pages.findIndex(p => p.key === key)
        if (index >= 0)
            showPage(index)
    }
    function groupOf(key) {
        for (const group of tree) {
            if (group.page === key || group.pages.indexOf(key) >= 0)
                return group
        }
        return tree[0]
    }
    // A group's pages in tab order: its own page first.
    function groupKeys(group) {
        return (group.page !== "" ? [group.page] : []).concat(group.pages)
    }
    function titleOf(key) {
        const page = pages.find(p => p.key === key)
        return page ? page.title : key
    }
    function toggleGroup(title) {
        expanded = Object.assign({}, expanded, { [title]: expanded[title] !== true })
    }
    // The page item at an index (self-test: every page loads).
    function pageItem(index) { return stack.children[index] }
    // `devices`: read the audio devices too (only when the window opens:
    // asking the system for them can cost video frames).
    function reload(devices) {
        const next = JSON.parse(player.settings_view(devices === true))
        if (devices !== true && view.choices)
            next.choices["audio-device"] = view.choices["audio-device"]
        view = next
    }
    function value(path) {
        let v = s
        for (const part of path.split(".")) {
            if (v === undefined || v === null)
                return undefined
            v = v[part]
        }
        return v
    }
    function report(result, done) {
        const r = JSON.parse(result)
        if (r.error !== "")
            notice = r.error
        else if (r.notes.length > 0)
            notice = r.notes.join("; ")
        else
            notice = done
        if (r.restart)
            restartNeeded = true
        reload()
    }
    function set(path, newValue) {
        report(player.set_setting(path, JSON.stringify(newValue)), "Applied.")
    }
    // The slider of a setting (self-test).
    function amount(path) {
        const find = (item) => {
            if (item.objectName === "amount:" + path)
                return item
            for (const child of item.children) {
                const found = find(child)
                if (found)
                    return found
            }
            return null
        }
        return find(contentItem)
    }
    // A list setting with one item added, removed or moved by `delta`.
    function listAdd(path, items) {
        const list = (value(path) || []).slice()
        for (const item of items)
            if (list.indexOf(item) < 0)
                list.push(item)
        set(path, list)
    }
    function listRemove(path, index) {
        const list = (value(path) || []).slice()
        list.splice(index, 1)
        set(path, list)
    }
    function listMove(path, index, delta) {
        const list = (value(path) || []).slice()
        const to = index + delta
        if (to < 0 || to >= list.length)
            return
        const item = list.splice(index, 1)[0]
        list.splice(to, 0, item)
        set(path, list)
    }
    // Device descriptions by name; `none` names the empty value.
    function deviceTexts(none) {
        const texts = { "": none, "auto": none }
        for (const d of view.choices["audio-device"] || [])
            if (d.name !== "auto")
                texts[d.name] = d.description
        return texts
    }
    // A choice list that keeps a value typed into the file.
    function withCurrent(list, current) {
        const values = list ? list.slice() : []
        if (current !== undefined && current !== null && values.indexOf(current) < 0)
            values.push(current)
        return values
    }
    onClosing: {
        if (keysPage.item)
            keysPage.item.end()
    }

    component Heading: Label {
        font.bold: true
        topPadding: 10
    }
    component Note: Label {
        opacity: 0.7
        wrapMode: Text.Wrap
        Layout.fillWidth: true
    }
    component Check: CheckBox {
        property string path
        checked: win.value(path) === true
        onToggled: win.set(path, checked)
    }
    // Seconds as the seek and sync steps show them.
    function seconds(v) {
        if (v < 1)
            return Math.round(v * 1000) + " ms"
        if (v < 60)
            return Math.round(v * 100) / 100 + " s"
        if (v < 3600)
            return Math.round(v / 6) / 10 + " min"
        return Math.round(v / 360) / 10 + " h"
    }
    // A number set with a slider (the owner prefers sliders to typed
    // numbers, 2026-10-16), its value beside it. The slider runs from `from`
    // to `to` in `step`s, or over the entries of `values`. It applies when
    // the mouse lets go of it, at once with the keys, and while it moves
    // when `live`. (A type named after a JavaScript global, such as Number,
    // hides that global in this file: Number(text) failed, and typed values
    // never applied.)
    component Amount: RowLayout {
        id: amount
        property string label
        property string path
        property string unit
        property real from: 0
        property real to: 100
        property real step: 1
        // Instead of a range: the values it stops at, ascending.
        property var values: []
        // Shown value = stored value * factor (percent of a scale).
        property real factor: 1
        // Text of a shown value; the unit follows the number by default.
        property var format: null
        property bool live: false
        readonly property bool stepped: values.length > 0
        readonly property real shown: {
            const v = win.value(amount.path)
            return v === undefined || v === null ? NaN : v * amount.factor
        }
        objectName: "amount:" + path
        // The slider position of a shown value: the nearest stop.
        function position(v) {
            if (!stepped)
                return Math.min(Math.max(v, from), to)
            let best = 0
            for (let i = 1; i < values.length; ++i)
                if (Math.abs(values[i] - v) < Math.abs(values[best] - v))
                    best = i
            return best
        }
        function shownAt(position) {
            return stepped ? values[Math.round(position)] : Math.round(position / step) * step
        }
        function text(v) {
            if (format)
                return format(v)
            return String(Math.round(v * 1000) / 1000) + (unit !== "" ? " " + unit : "")
        }
        function commit() {
            const v = shownAt(slider.value)
            if (isNaN(shown) || Math.abs(v - shown) > 1e-9)
                win.set(amount.path, v / factor)
        }
        // Moves the slider to the stop nearest `v` and applies it, as a
        // move with the keys does (self-test).
        function pick(v) {
            slider.value = position(v)
            commit()
        }
        Label {
            text: amount.label
            Layout.preferredWidth: 240
        }
        Slider {
            id: slider
            Layout.preferredWidth: 280
            from: amount.stepped ? 0 : amount.from
            to: amount.stepped ? amount.values.length - 1 : amount.to
            stepSize: amount.stepped ? 1 : amount.step
            snapMode: Slider.SnapAlways
            value: isNaN(amount.shown) ? from : amount.position(amount.shown)
            onMoved: {
                if (!pressed || amount.live)
                    amount.commit()
            }
            onPressedChanged: {
                if (!pressed)
                    amount.commit()
            }
        }
        Label {
            // The stored value until the slider leaves its stop.
            text: amount.text(slider.pressed || isNaN(amount.shown)
                              || amount.position(amount.shown) !== slider.value
                              ? amount.shownAt(slider.value) : amount.shown)
            Layout.minimumWidth: 80
        }
    }
    component Choice: RowLayout {
        id: choice
        property string label
        property string path
        property var values: []
        // Display texts by value; missing ones show the value.
        property var texts: ({})
        Label {
            text: choice.label
            Layout.preferredWidth: 240
        }
        ComboBox {
            Layout.preferredWidth: 300
            model: choice.values.map(v => choice.texts[v] !== undefined ? choice.texts[v] : v)
            currentIndex: choice.values.indexOf(win.value(choice.path))
            onActivated: (index) => win.set(choice.path, choice.values[index])
        }
    }
    component Field: RowLayout {
        id: field
        property string label
        property string path
        property string placeholder
        // Unset (null) when empty: optional paths.
        property bool optional: false
        default property alias extra: tail.data
        Label {
            text: field.label
            Layout.preferredWidth: 240
        }
        TextField {
            Layout.fillWidth: true
            placeholderText: field.placeholder
            text: {
                const v = win.value(field.path)
                return v === undefined || v === null ? "" : String(v)
            }
            onEditingFinished: {
                const v = win.value(field.path)
                const old = v === undefined || v === null ? "" : String(v)
                if (text !== old)
                    win.set(field.path, field.optional && text === "" ? null : text)
            }
        }
        RowLayout {
            id: tail
        }
    }
    // One of RVP's video effects (the `effects` set).
    component Effect: CheckBox {
        property string effect
        checked: (win.s.effects || []).indexOf(effect) >= 0
        onToggled: {
            const list = (win.s.effects || []).filter(e => e !== effect)
            if (checked)
                list.push(effect)
            win.set("effects", list)
        }
    }
    // A file list (`path`) with move and remove buttons.
    // Adds one of the catalogue's shaders (RVP's, PotPlayer's, the
    // folders') to a list: those of its stage and the HLSL ones.
    component ShaderPicker: RowLayout {
        id: picker
        property string target
        property string stage
        readonly property var choices: (win.view.choices["shader-choices"] || [])
            .filter(c => c.stage === picker.stage || c.stage === "any")
        ComboBox {
            id: shaderChoice
            Layout.preferredWidth: 320
            model: picker.choices.map(c => c.name + (c.stage === "any" ? " (HLSL)" : ""))
        }
        Button {
            text: "Add"
            enabled: shaderChoice.currentIndex >= 0
            onClicked: win.listAdd(picker.target, [picker.choices[shaderChoice.currentIndex].path])
        }
    }

    component FileList: ColumnLayout {
        id: fileList
        property string path
        property bool ordered: false
        // Optional radio selection of one item (`selectPath`).
        property string selectPath
        Layout.fillWidth: true
        Repeater {
            model: win.value(fileList.path) || []
            delegate: RowLayout {
                id: row
                required property string modelData
                required property int index
                Layout.fillWidth: true
                RadioButton {
                    visible: fileList.selectPath !== ""
                    checked: win.value(fileList.selectPath) === row.modelData
                    onClicked: win.set(fileList.selectPath, row.modelData)
                }
                Label {
                    text: row.modelData
                    elide: Text.ElideMiddle
                    Layout.fillWidth: true
                }
                Button {
                    visible: fileList.ordered
                    text: "Up"
                    enabled: row.index > 0
                    onClicked: win.listMove(fileList.path, row.index, -1)
                }
                Button {
                    visible: fileList.ordered
                    text: "Down"
                    onClicked: win.listMove(fileList.path, row.index, 1)
                }
                Button {
                    text: "Remove"
                    onClicked: {
                        if (fileList.selectPath !== "" && win.value(fileList.selectPath) === row.modelData)
                            win.set(fileList.selectPath, null)
                        win.listRemove(fileList.path, row.index)
                    }
                }
            }
        }
    }
    component Page: ScrollView {
        id: page
        default property alias content: column.data
        contentWidth: availableWidth
        ColumnLayout {
            id: column
            width: page.availableWidth - 16
            x: 8
            spacing: 6
        }
    }

    // Folder and file choosers; `target` is the setting they fill.
    // Qualified: Rvp's own FolderDialog.qml has the same name.
    Dialogs.FolderDialog {
        id: folderChooser
        property string target
        onAccepted: win.set(target, Tools.local_file(selectedFolder))
    }
    // Adds the chosen folder to the list setting `target`.
    Dialogs.FolderDialog {
        id: folderListChooser
        property string target
        onAccepted: win.listAdd(target, [Tools.local_file(selectedFolder)])
    }
    // Adds the chosen files to the list setting `target`.
    Dialogs.FileDialog {
        id: listChooser
        property string target
        fileMode: Dialogs.FileDialog.OpenFiles
        onAccepted: win.listAdd(target, selectedFiles.map(url => Tools.local_file(url)))
    }
    // Fills the single file setting `target`.
    Dialogs.FileDialog {
        id: fileChooser
        property string target
        onAccepted: win.set(target, Tools.local_file(selectedFile))
    }
    function chooseFiles(target, title, filters) {
        listChooser.target = target
        listChooser.title = title
        listChooser.nameFilters = filters
        listChooser.open()
    }
    function chooseFile(target, title, filters) {
        fileChooser.target = target
        fileChooser.title = title
        fileChooser.nameFilters = filters
        fileChooser.open()
    }
    Dialogs.FontDialog {
        id: subtitleFontChooser
        title: "Subtitle Font"
        onAccepted: win.set("subtitle-font", selectedFont.family)
    }
    Dialogs.FileDialog {
        id: skinChooser
        title: "PotPlayer skin"
        nameFilters: ["PotPlayer skins (*.dsf)", "All files (*)"]
        onAccepted: win.set("skin", Tools.local_file(selectedFile))
    }
    Dialogs.FileDialog {
        id: importChooser
        title: "Import Settings"
        nameFilters: ["Settings (*.json)", "All files (*)"]
        onAccepted: win.report(win.player.import_settings(Tools.local_file(selectedFile)), "Imported.")
    }
    Dialogs.FileDialog {
        id: exportChooser
        title: "Export Settings"
        fileMode: Dialogs.FileDialog.SaveFile
        defaultSuffix: "json"
        nameFilters: ["Settings (*.json)"]
        onAccepted: win.report(win.player.export_settings(Tools.local_file(selectedFile)), "Exported.")
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 8

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 10

            // The tree: a group's row shows its own page (or its first
            // one) and opens it; the sign beside it opens or closes it.
            ListView {
                id: pageTree
                Layout.preferredWidth: 200
                Layout.fillHeight: true
                model: win.rows
                clip: true
                delegate: ItemDelegate {
                    id: row
                    required property var modelData
                    width: pageTree.width
                    leftPadding: 8 + modelData.depth * 18 + 16
                    text: modelData.title
                    font.bold: modelData.depth === 0
                    highlighted: modelData.depth === 0 ? modelData.own && modelData.key === win.currentKey
                                                       : modelData.key === win.currentKey
                    onClicked: win.showKey(modelData.key)
                    Label {
                        x: 8 + row.modelData.depth * 18
                        anchors.verticalCenter: parent.verticalCenter
                        visible: row.modelData.children
                        text: win.expanded[row.modelData.group] === true ? "\u2212" : "+"
                        MouseArea {
                            anchors.fill: parent
                            anchors.margins: -4
                            onClicked: win.toggleGroup(row.modelData.group)
                        }
                    }
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 0

                // The selected group's pages, as PotPlayer shows them.
                TabBar {
                    id: tabs
                    objectName: "pageTabs"
                    Layout.fillWidth: true
                    Repeater {
                        model: win.tabKeys
                        delegate: TabButton {
                            required property string modelData
                            text: win.titleOf(modelData)
                            width: implicitWidth
                            onClicked: win.showKey(modelData)
                        }
                    }
                }

                StackLayout {
                    id: stack
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    currentIndex: win.current

                    Page {
                        Heading { text: "Resume" }
                        Check { path: "remember-positions"; text: "Remember video playback positions" }
                        Check { path: "playback.remember-audio-positions"; text: "Remember audio playback positions" }
                        Check { path: "reopen-last-file"; text: "Reopen the last file, paused, when RVP starts without files" }
                        Heading { text: "Playback" }
                        Choice {
                            label: "At the end of a file"
                            path: "repeat"
                            values: ["off", "album", "item"]
                            texts: ({ "off": "Play the next one", "album": "Repeat the playlist",
                                      "item": "Repeat the file" })
                        }
                        Check { path: "playback.shuffle"; text: "Shuffle playback" }
                        Choice {
                            label: "When playback failed"
                            path: "playback.on-failure"
                            values: ["next", "stop"]
                            texts: ({ "next": "Play the next one", "stop": "Stop" })
                        }
                        Amount { label: "Speed step"; path: "playback.speed-step"; values: [0.01, 0.02, 0.05, 0.1, 0.2, 0.25, 0.5]; format: (v) => v + "x" }
                        Check { path: "playback.keep-speed"; text: "Keep playback speed changes for the next file" }
                        Check { path: "playback.external-audio"; text: "Auto load external audio files" }
                        FileList { path: "playback.audio-folders"; ordered: true }
                        Button {
                            text: "Add a folder for external audio…"
                            onClicked: {
                                folderListChooser.target = "playback.audio-folders"
                                folderListChooser.open()
                            }
                        }
                        Choice {
                            visible: win.view.windows === true
                            label: "Process priority (at the next start)"
                            path: "playback.priority"
                            values: ["idle", "belownormal", "normal", "abovenormal", "high"]
                            texts: ({ "idle": "Idle", "belownormal": "Below normal", "normal": "Normal",
                                      "abovenormal": "Above normal", "high": "High" })
                        }
                        Check { path: "playback.folder-neighbours"; text: "Load previous/next files in the play folder" }
                        Check { path: "playback.auto-rotate"; text: "Auto rotate files based on metadata" }
                        Check { path: "playback.retitle"; text: "Auto retitle items based on metadata" }
                        Check { path: "playback.close-at-end"; text: "Close playback when the playlist ends" }
                        Check { path: "playback.recycle-played"; text: "Offer played files to the Recycle Bin" }
                        Heading { text: "Seek bar" }
                        Check { path: "playback.seek-tooltip"; text: "Show time info on the tooltip" }
                        Check { path: "playback.seek-preview"; text: "Show thumbnail previews" }
                        Check { path: "playback.preview-title"; text: "Include the title of the bookmark or chapter" }
                        Check { path: "playback.preview-time"; text: "Include the time" }
                        Check { path: "playback.seek-markers"; text: "Show markers of bookmarks, chapters and A-B points" }
                        Check { path: "playback.pause-while-seeking"; text: "Pause when seeking with the mouse" }
                        Heading { text: "When playback finished" }
                        Choice {
                            label: "The action"
                            path: "playback.finished.action"
                            values: ["shutdown", "sleep", "hibernate"]
                            texts: ({ "shutdown": "Shut down", "sleep": "Sleep", "hibernate": "Hibernate" })
                        }
                        Amount { label: "Notice before it (cancellable)"; path: "playback.finished.countdown"; from: 5; to: 300; step: 5; unit: "s" }
                        Check { path: "playback.finished.remember"; text: "Remember the playback finished action" }
                        Heading { text: "Window" }
                        Check { path: "fit-window-to-video"; text: "Fit the window to a newly opened video" }
                        Check { path: "playback.center-window"; text: "Start playback in a centred window" }
                        Check { path: "keep-aspect-window"; text: "Keep the video's shape while the window is resized" }
                        Check { path: "prevent-screen-saver"; text: "Prevent screen saver during playback" }
                        Check { path: "playback.hide-pointer"; text: "Hide the mouse pointer over the playing video" }
                        Choice {
                            label: "Stay on top"
                            path: "on-top"
                            values: ["never", "always", "while-playing", "fullscreen-playing"]
                            texts: ({ "never": "Never", "always": "Always", "while-playing": "While playing",
                                      "fullscreen-playing": "While playing in fullscreen" })
                        }
                        Heading { text: "Opening" }
                        Choice {
                            label: "Default open action"
                            path: "open.default-open"
                            values: ["files", "url", "folder"]
                            texts: ({ "files": "Open File(s)", "url": "Open URL", "folder": "Open Folder" })
                        }
                        Choice {
                            label: "When files are opened"
                            path: "open.when-opened"
                            values: ["selected", "similar", "all"]
                            texts: ({ "selected": "Add only the files opened", "similar": "Add similar files too",
                                      "all": "Add every file of the folder" })
                        }
                        Check { path: "open.recent"; text: "Store and show recent items" }
                        Check { path: "open.ytdl"; text: "Open streaming sites with yt-dlp" }
                        Field { label: "yt-dlp program"; path: "open.ytdl-path"; placeholder: "yt-dlp" }
                        Field { label: "yt-dlp format"; path: "open.ytdl-format"; placeholder: "bestvideo+bestaudio/best" }
                    }

                    Page {
                        Heading { text: "Time jump (the arrow keys)" }
                        Amount { label: "Left / Right"; path: "seek-seconds.0"; values: [0.5, 1, 2, 3, 5, 10, 15, 20, 30, 45, 60, 90, 120, 180, 300, 600, 900, 1800, 3600]; format: win.seconds }
                        Amount { label: "Ctrl+Left / Right"; path: "seek-seconds.1"; values: [0.5, 1, 2, 3, 5, 10, 15, 20, 30, 45, 60, 90, 120, 180, 300, 600, 900, 1800, 3600]; format: win.seconds }
                        Amount { label: "Shift+Left / Right"; path: "seek-seconds.2"; values: [0.5, 1, 2, 3, 5, 10, 15, 20, 30, 45, 60, 90, 120, 180, 300, 600, 900, 1800, 3600]; format: win.seconds }
                        Amount { label: "Ctrl+Alt+Left / Right"; path: "seek-seconds.3"; values: [0.5, 1, 2, 3, 5, 10, 15, 20, 30, 45, 60, 90, 120, 180, 300, 600, 900, 1800, 3600]; format: win.seconds }
                        Check { path: "keyframe-seeks"; text: "Jump to keyframes (faster, less exact)" }
                    }

                    Page {
                        Heading { text: "Skip" }
                        Check { path: "playback.skip.enabled"; text: "Enable the skip feature" }
                        Check { path: "playback.skip.intro"; text: "Skip the intro" }
                        Amount { label: "Intro length"; path: "playback.skip.intro-seconds"; from: 1; to: 600; step: 1; format: win.seconds }
                        Check { path: "playback.skip.ending"; text: "Skip the ending" }
                        Amount { label: "Ending length"; path: "playback.skip.ending-seconds"; from: 1; to: 600; step: 1; format: win.seconds }
                        Check { path: "playback.skip.chapters"; text: "Skip chapters" }
                        Field { label: "Chapters to skip"; path: "playback.skip.chapter-names"; placeholder: "Opening;Ending;3" }
                        Note { text: "Titles (a part, any case) or numbers, separated by semicolons." }
                    }

                    Page {
                        Heading { text: "Section repeat (A-B)" }
                        Amount { label: "Repeats (0: endless)"; path: "playback.ab.loops"; from: 0; to: 100; step: 1 }
                        Amount { label: "Fine move"; path: "playback.ab.step"; values: [0.01, 0.05, 0.1, 0.2, 0.5, 1]; format: win.seconds }
                        Amount { label: "Repeat behind the current point"; path: "playback.ab.behind"; from: 1; to: 60; step: 1; format: win.seconds }
                        Check { path: "playback.ab.remember"; text: "Remember the A-B repeat lists" }
                        Check { path: "playback.ab.prevent-backward"; text: "Prevent backward navigation from point A" }
                        Check { path: "playback.ab.toggle-current"; text: "Repeat-current commands also turn the repeat off" }
                        Heading { text: "Bookmarks" }
                        Check { path: "playback.bookmarks.prompt"; text: "Prompt before adding a bookmark" }
                        Check { path: "playback.bookmarks.in-menu"; text: "Show the bookmark list in the Chapter/Bookmark menu" }
                        Check { path: "playback.bookmarks.auto-delete"; text: "Delete a bookmark once it is chosen" }
                    }

                    Page {
                        Heading { text: "Scene browser" }
                        Amount { label: "Scene width"; path: "scenes.size"; values: [96, 128, 160, 192, 224, 256, 320, 400, 480]; unit: "px" }
                        Amount { label: "Time interval (0: auto)"; path: "scenes.interval"; values: [0, 5, 10, 15, 30, 60, 120, 300, 600]; format: (v) => v === 0 ? "Auto" : win.seconds(v) }
                        Amount { label: "Opacity"; path: "scenes.opacity"; from: 20; to: 100; step: 5; unit: "%" }
                        Check { path: "scenes.close-on-change"; text: "Close on scene changes" }
                        Check { path: "scenes.fall-back-to-time"; text: "Time-based scenes when there is no marker or subtitle" }
                    }

                    Page {
                        Heading { text: "PotPlayer's playlists" }
                        Field {
                            label: "Playlist folder"
                            path: "potplayer-playlists"
                            optional: true
                            placeholder: win.view.windows ? "%APPDATA%\\PotPlayerMini64\\Playlist" : "not set"
                            Button {
                                text: "Browse…"
                                onClicked: {
                                    folderChooser.target = "potplayer-playlists"
                                    folderChooser.open()
                                }
                            }
                        }
                        RowLayout {
                            Button {
                                text: "Import Playlists Now"
                                onClicked: {
                                    win.player.run_action("import-playlists")
                                    win.notice = "Imported (see the message in the player window)."
                                }
                            }
                            Note { text: "Reads PotPlayer's .dpl files; PotPlayer's files are not changed." }
                        }
                        Heading {
                            text: "Drives in PotPlayer's playlists"
                            visible: !win.view.windows
                        }
                        Note {
                            visible: !win.view.windows
                            text: "Windows drive letters in imported playlists and the Linux folders they stand for; other drives become /mnt/<letter>."
                        }
                        Repeater {
                            model: win.view.windows ? [] : Object.keys(win.s["drive-map"] || {})
                            delegate: RowLayout {
                                required property string modelData
                                Label {
                                    text: modelData
                                    Layout.preferredWidth: 60
                                }
                                TextField {
                                    Layout.fillWidth: true
                                    text: win.s["drive-map"][modelData]
                                    onEditingFinished: {
                                        const map = Object.assign({}, win.s["drive-map"])
                                        map[modelData] = text
                                        win.set("drive-map", map)
                                    }
                                }
                                Button {
                                    text: "Remove"
                                    onClicked: {
                                        const map = Object.assign({}, win.s["drive-map"])
                                        delete map[modelData]
                                        win.set("drive-map", map)
                                    }
                                }
                            }
                        }
                        RowLayout {
                            visible: !win.view.windows
                            TextField {
                                id: newDrive
                                placeholderText: "V:"
                                Layout.preferredWidth: 60
                            }
                            TextField {
                                id: newFolder
                                placeholderText: "/mnt/media"
                                Layout.fillWidth: true
                            }
                            Button {
                                text: "Add"
                                onClicked: {
                                    const map = Object.assign({}, win.s["drive-map"])
                                    map[newDrive.text.toUpperCase()] = newFolder.text
                                    win.set("drive-map", map)
                                    newDrive.text = ""
                                    newFolder.text = ""
                                }
                            }
                        }
                        Heading { text: "Playlist" }
                        Choice {
                            label: "View items as"
                            path: "playlist.thumbnail-rows"
                            values: [0, 2, 3, 4, 5, 6, 7]
                            texts: ({ 0: "List", 2: "Thumbnails (2 rows) + details", 3: "Thumbnails (3 rows) + details",
                                      4: "Thumbnails (4 rows) + details", 5: "Thumbnails (5 rows) + details",
                                      6: "Thumbnails (6 rows) + details", 7: "Thumbnails (7 rows) + details" })
                        }
                        Amount { label: "Thumbnail position"; path: "playlist.thumbnail-position"; from: 0; to: 95; step: 5; unit: "%" }
                        Check { path: "playlist.wide-thumbnails"; text: "Use a 16:9 thumbnail ratio" }
                        Check { path: "playlist.detect-durations"; text: "Detect durations after adding files to the list" }
                        Check { path: "playlist.remember-default"; text: "Remember the Default album and recent URLs" }
                        Check { path: "playlist.instant-albums"; text: "Start albums chosen in menus at once" }
                        Check { path: "playlist.append-on-open"; text: "Append opened files rather than clear the album" }
                        Check { path: "playlist.no-duplicates"; text: "Do not add duplicate files" }
                        Check { path: "playlist.sort-dropped"; text: "Sort dropped files before adding them" }
                        Check { path: "playlist.focus-main"; text: "Give the window the keyboard when an entry plays" }
                    }

                    Page {
                        Heading { text: "Screen capture" }
                        Choice {
                            label: "Method"
                            path: "devices.capture"
                            values: ["gdigrab", "ddagrab"]
                            texts: ({ "gdigrab": "GDI (any desktop)", "ddagrab": "Desktop Duplication (Windows 8 and later)" })
                        }
                        Amount { label: "Frames a second"; path: "devices.capture-fps"; values: [5, 10, 15, 24, 25, 30, 50, 60]; unit: "fps" }
                        Check { path: "devices.capture-pointer"; text: "Show the mouse pointer" }
                        Heading { text: "Webcam and other devices" }
                        Field { label: "Device opened without asking"; path: "devices.webcam"; placeholder: "ask every time" }
                        Heading { text: "Discs" }
                        Field { label: "DVD drive or folder"; path: "devices.dvd"; placeholder: "ask every time" }
                        Field { label: "Blu-ray drive or folder"; path: "devices.bluray"; placeholder: "ask every time" }
                        Field { label: "CD drive"; path: "devices.cd"; placeholder: "libmpv's default" }
                        Note { text: "Discs play their titles; their menus come with a later version." }
                    }

                    Page {
                        Heading { text: "Audio Output" }
                        Choice {
                            label: "Audio Renderer"
                            path: "audio-device"
                            values: win.withCurrent((win.view.choices["audio-device"] || []).map(d => d.name),
                                                    win.s["audio-device"])
                            texts: win.deviceTexts("Auto select (Recommended)")
                        }
                        Choice {
                            label: "Pass-through Renderer"
                            path: "audio.passthrough-device"
                            values: win.withCurrent([""].concat((win.view.choices["audio-device"] || [])
                                                    .map(d => d.name).filter(n => n !== "auto")),
                                                    win.value("audio.passthrough-device"))
                            texts: win.deviceTexts("Default Audio Device (Recommended)")
                        }
                        Choice {
                            label: "2nd Audio Renderer"
                            path: "audio.second-device"
                            values: win.withCurrent([""].concat((win.view.choices["audio-device"] || [])
                                                    .map(d => d.name).filter(n => n !== "auto")),
                                                    win.value("audio.second-device"))
                            texts: win.deviceTexts("Disable (Recommended)")
                        }
                        Choice {
                            label: "Speakers"
                            path: "output-speakers"
                            values: (win.view.choices["output-speakers"] || []).map(c => c.value)
                            texts: {
                                const texts = {}
                                for (const c of win.view.choices["output-speakers"] || [])
                                    texts[c.value] = c.label
                                return texts
                            }
                        }
                        Choice {
                            label: "Output format"
                            path: "audio.sample-format"
                            values: ["same", "s16", "s24", "s32", "float", "s16-dither"]
                            texts: ({ "same": "As the output device takes it", "s16": "16-bit",
                                      "s24": "24-bit", "s32": "32-bit", "float": "32-bit (floating point)",
                                      "s16-dither": "16-bit (dithering)" })
                        }
                        Choice {
                            label: "Sample rate"
                            path: "audio.sample-rate"
                            values: win.view.choices["sample-rate"] || [0]
                            texts: {
                                const texts = {}
                                for (const rate of win.view.choices["sample-rate"] || [0])
                                    texts[rate] = rate === 0 ? "Same as input (Recommended)" : rate + " Hz"
                                return texts
                            }
                        }
                        Choice {
                            label: "Resample"
                            path: "audio.resample-quality"
                            values: ["high", "normal", "low"]
                            texts: ({ "high": "High quality", "normal": "Normal quality (Recommended)",
                                      "low": "Low quality" })
                        }
                        Choice {
                            label: "Change the sample rate"
                            path: "audio.resample-when"
                            values: ["always", "below", "above"]
                            texts: ({ "always": "Always", "below": "When sample rate is smaller than",
                                      "above": "When sample rate is larger than" })
                        }
                        Amount { label: "That sample rate"; path: "audio.resample-limit"; unit: "Hz"; values: [8000, 11025, 12000, 16000, 22050, 24000, 32000, 44100, 48000, 96000, 192000] }
                        Choice {
                            label: "Output"
                            path: "audio.output-mode"
                            values: ["pcm", "passthrough", "ac3-encode"]
                            texts: ({ "pcm": "PCM (Default) (Recommended)", "passthrough": "Pass-through",
                                      "ac3-encode": "Pass-through after AC3 re-encoding" })
                        }
                        Amount { label: "AC3 re-encoding bitrate"; path: "audio.ac3-bitrate"; unit: "kbps"; values: [64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384, 448, 512, 576, 640] }
                        Choice {
                            label: "Pass-through when the source has"
                            path: "audio.passthrough-min-channels"
                            values: [null, 2, 3, 4, 5, 6, 7, 8]
                            texts: ({ "null": "Any number of channels", 2: "2 channels or more",
                                      3: "3 channels or more", 4: "4 channels or more", 5: "5 channels or more",
                                      6: "6 channels or more", 7: "7 channels or more", 8: "8 channels" })
                        }
                        Check { path: "audio.exclusive"; text: "Exclusive mode (the device plays only RVP)" }
                        Heading { text: "Mixer Settings" }
                        Check { path: "audio.mixer.normalize"; text: "Normalize matrix (scale the mix down so it cannot clip)" }
                        Check { path: "audio.mixer.expand-center"; text: "Expand stereo to center" }
                        Check { path: "audio.mixer.expand-surround"; text: "Expand stereo to surround" }
                        Heading { text: "Mixer Level" }
                        Amount { label: "Front"; path: "audio.mixer.front"; unit: "dB"; from: -30; to: 12; step: 0.5 }
                        Amount { label: "Center"; path: "audio.mixer.center"; unit: "dB"; from: -30; to: 12; step: 0.5 }
                        Amount { label: "Surround"; path: "audio.mixer.surround"; unit: "dB"; from: -30; to: 12; step: 0.5 }
                        Amount { label: "LFE"; path: "audio.mixer.lfe"; unit: "dB"; from: -30; to: 12; step: 0.5 }
                        Note {
                            text: "Stereo mixes 5.1 and 7.1 down as PotPlayer does (centre and LFE at -3 dB, "
                                  + "surround channels at their level) before the equalizer and the normalizer."
                        }
                        Heading { text: "Headphone & Dolby Decoder Settings" }
                        Amount { label: "Distance (virtual speakers' room)"; path: "audio.virtualizer.distance"; from: 0; to: 100 }
                        Amount { label: "Delay (decoded surround)"; path: "audio.virtualizer.delay"; unit: "ms"; from: 0; to: 100 }
                        Heading { text: "LFE Redirection" }
                        Check { path: "audio.lfe-redirect.enabled"; text: "Enable" }
                        Check { path: "audio.lfe-redirect.remove-low"; text: "Remove low frequencies from the other speakers" }
                        Amount { label: "Low frequency"; path: "audio.lfe-redirect.frequency"; unit: "Hz"; from: 20; to: 300; step: 5 }
                        Amount { label: "Gain"; path: "audio.lfe-redirect.gain"; unit: "dB"; from: -20; to: 20; step: 0.5 }
                        Heading { text: "Second Device" }
                        Amount { label: "Sync of the 2nd device"; path: "audio.second-delay"; unit: "ms"; from: -2000; to: 2000; step: 10 }
                    }

                    Page {
                        Heading { text: "Volume control setting" }
                        Choice {
                            label: "Method"
                            path: "audio.volume-method"
                            values: ["player", "wave", "master"]
                            texts: ({ "player": "Default volume control (Recommended)",
                                      "wave": "System Wave volume control",
                                      "master": "System Master volume control" })
                        }
                        Check { path: "audio.wave-to-max"; text: "Adjust wave volume to max" }
                        Check { path: "audio.accelerate"; text: "Accelerate control speed" }
                        Amount { label: "Volume step (Up, Down, wheel)"; path: "volume-step"; unit: "%"; from: 1; to: 20 }
                        Amount { label: "Max amplification (above 100 amplifies)"; path: "volume-max"; unit: "%"
                                 values: [100, 110, 120, 130, 150, 175, 200, 250, 300, 400, 500, 700, 1000] }
                        CheckBox {
                            text: "Startup volume"
                            checked: win.value("audio.startup-volume") !== null
                                     && win.value("audio.startup-volume") !== undefined
                            onToggled: win.set("audio.startup-volume", checked ? 50 : null)
                        }
                        Amount { label: "Volume at every start"; path: "audio.startup-volume"; unit: "%"; from: 0; to: 200
                                 enabled: win.value("audio.startup-volume") !== null }
                        Heading { text: "Individual volume control" }
                        Check { path: "audio.channel-volumes.enabled"; text: "Enable individual volume control" }
                        Amount { label: "Master volume"; path: "audio.channel-volumes.master"; unit: "dB"; from: -40; to: 12; step: 0.5; live: true }
                        Amount { label: "Left channel"; path: "audio.channel-volumes.left"; unit: "dB"; from: -40; to: 12; step: 0.5; live: true }
                        Amount { label: "Right channel"; path: "audio.channel-volumes.right"; unit: "dB"; from: -40; to: 12; step: 0.5; live: true }
                        Amount { label: "Center channel"; path: "audio.channel-volumes.center"; unit: "dB"; from: -40; to: 12; step: 0.5; live: true }
                        Amount { label: "BackL channel"; path: "audio.channel-volumes.back-left"; unit: "dB"; from: -40; to: 12; step: 0.5; live: true }
                        Amount { label: "BackR channel"; path: "audio.channel-volumes.back-right"; unit: "dB"; from: -40; to: 12; step: 0.5; live: true }
                        Amount { label: "SideL channel"; path: "audio.channel-volumes.side-left"; unit: "dB"; from: -40; to: 12; step: 0.5; live: true }
                        Amount { label: "SideR channel"; path: "audio.channel-volumes.side-right"; unit: "dB"; from: -40; to: 12; step: 0.5; live: true }
                        Amount { label: "LFE channel"; path: "audio.channel-volumes.lfe"; unit: "dB"; from: -40; to: 12; step: 0.5; live: true }
                        Heading { text: "Pre-amplify" }
                        Check { path: "audio.preamp.enabled"; text: "Pre-amplify AC3/DTS Vol." }
                        Amount { label: "PreAmp"; path: "audio.preamp.gain"; unit: "dB"; from: 0; to: 20; step: 0.5 }
                    }

                    Page {
                        Heading { text: "Equalizer" }
                        Check { path: "equalizer.enabled"; text: "Enable (Shift+E), before the normalizer" }
                        RowLayout {
                            Label { text: "Preset"; Layout.preferredWidth: 240 }
                            ComboBox {
                                id: presetBox
                                Layout.preferredWidth: 300
                                model: win.view.choices["equalizer-preset"] || []
                                currentIndex: (win.view.choices["equalizer-preset"] || [])
                                    .indexOf(win.value("equalizer.preset"))
                                onActivated: (index) => {
                                    player.run_menu_command("e:" + index)
                                    win.reload()
                                }
                            }
                            Button {
                                text: "0 dB"
                                onClicked: win.set("equalizer.bands", [0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
                            }
                        }
                        Check { path: "equalizer.superequalizer"; text: "SuperEQ (Naoki Shibata's equalizer, 18 bands)" }
                        Repeater {
                            model: ["60 Hz", "170 Hz", "310 Hz", "600 Hz", "1 kHz", "3 kHz", "6 kHz", "12 kHz", "14 kHz", "16 kHz"]
                            delegate: Amount {
                                required property string modelData
                                required property int index
                                label: modelData
                                path: "equalizer.bands." + index
                                unit: "dB"
                                from: -20
                                to: 20
                                step: 0.5
                                live: true
                            }
                        }
                        Note { text: "The bands are also on the control panel (F7)." }
                    }

                    Page {
                        Heading { text: "Normalizer (Auto Volume Control)" }
                        Check { path: "normalizer.enabled"; text: "Enable (Shift+N)" }
                        Check { path: "normalizer.video-only"; text: "Operate only with video playback" }
                        Note { text: "Method: TomSteady 32-bit floating point arithmetic, PotPlayer's recommended one." }
                        Check { path: "normalizer.boost"; text: "Boost audio (raise quiet passages)" }
                        Amount { label: "Level"; path: "normalizer.level"; unit: "%"; from: 10; to: 100 }
                        Amount { label: "Release time"; path: "normalizer.release"; from: 5; to: 10 }
                        Heading { text: "Freeverb" }
                        Check { path: "audio.freeverb.enabled"; text: "Enable (Shift+R)" }
                        Note { text: "It operates only in 2 channels." }
                        Amount { label: "Room size"; path: "audio.freeverb.room-size"; from: 0; to: 1; step: 0.01 }
                        Amount { label: "Damping"; path: "audio.freeverb.damping"; from: 0; to: 1; step: 0.01 }
                        Amount { label: "Wet level"; path: "audio.freeverb.wet"; from: 0; to: 1; step: 0.001 }
                        Amount { label: "Dry level"; path: "audio.freeverb.dry"; from: 0; to: 1; step: 0.01 }
                        Amount { label: "Width"; path: "audio.freeverb.width"; from: 0; to: 1; step: 0.01 }
                    }

                    Page {
                        Heading { text: "Crystallization" }
                        Check { path: "audio.crystality.enabled"; text: "Enable (Shift+C)" }
                        Amount { label: "Bandwidth extender"; path: "audio.crystality.bandwidth-extender"; from: 0; to: 100 }
                        Amount { label: "Extender's HPF"; path: "audio.crystality.extender-hpf"; from: 1; to: 3 }
                        Amount { label: "Harmonic booster"; path: "audio.crystality.harmonic-booster"; from: 0; to: 100 }
                        Amount { label: "Echo volume"; path: "audio.crystality.echo-volume"; from: 0; to: 100 }
                        Amount { label: "Echo feedback"; path: "audio.crystality.echo-feedback"; from: 0; to: 100 }
                        Amount { label: "Extra stereo"; path: "audio.crystality.extra-stereo"; from: 0; to: 100 }
                        Heading { text: "Noise Reduction" }
                        Check { path: "audio.noise-reduction.enabled"; text: "Enable (Shift+D)" }
                        Heading { text: "Crossfeed" }
                        Check { path: "audio.crossfeed.enabled"; text: "Enable" }
                        Amount { label: "Remove low frequencies"; path: "audio.crossfeed.cut"; unit: "Hz"; from: 300; to: 2000; step: 50 }
                        Amount { label: "Gain"; path: "audio.crossfeed.feed"; unit: "dB"; from: 1; to: 15; step: 0.5 }
                        Heading { text: "Voice and Channels" }
                        Check { path: "audio.voice-removal"; text: "Voice Removal (Shift+V)" }
                        Check { path: "audio.voice-emphasis"; text: "Voice Emphasis (Shift+A)" }
                        Check { path: "audio.swap-channels"; text: "Swap Stereo Channels (Shift+T)" }
                    }

                    Page {
                        Heading { text: "Language/Stream Selection" }
                        Choice {
                            label: "Selection method"
                            path: "audio.selection"
                            values: ["languages", "stream", "quality", "default"]
                            texts: ({ "languages": "Prefer specified languages (Recommended)",
                                      "stream": "Prefer specified stream",
                                      "quality": "Prefer highest quality ones",
                                      "default": "Use default selection" })
                        }
                        Field { label: "Preferred languages"; path: "audio.languages"; placeholder: "rus,eng" }
                        Field { label: "Turn off subtitles with"; path: "audio.subtitles-off"; placeholder: "rus" }
                        Amount { label: "Preferred stream (0: none)"; path: "audio.preferred-stream"; from: 0; to: 9 }
                        Check { path: "audio.remember-tracks"; text: "Remember audio track selections" }
                        Heading { text: "Synchronization" }
                        Choice {
                            label: "Audio sync control"
                            path: "audio.sync-keep"
                            values: ["never", "runtime", "always"]
                            texts: ({ "never": "Do not keep audio sync changes",
                                      "runtime": "Keep audio sync changes over whole runtime",
                                      "always": "Always keep audio sync changes" })
                        }
                        Amount { label: "Audio sync step (Shift+, and Shift+.)"; path: "audio-delay-step"; values: [0.001, 0.005, 0.01, 0.02, 0.025, 0.05, 0.1, 0.2, 0.25, 0.5, 1, 2, 5, 10]; format: win.seconds }
                        Amount { label: "Default audio sync"; path: "audio.default-delay"; unit: "ms"; from: -2000; to: 2000; step: 10 }
                        Heading { text: "Speed" }
                        Choice {
                            label: "Time stretching"
                            path: "audio.stretch"
                            values: ["high-quality", "default-quality", "pitch"]
                            texts: ({ "high-quality": "High quality audio time stretching (Recommended)",
                                      "default-quality": "Default quality audio time stretching",
                                      "pitch": "Audio pitch shifting" })
                        }
                        Heading { text: "Audio Recording" }
                        Field { label: "Storage folder"; path: "audio.recording.folder"; optional: true; placeholder: "Music/RVP" }
                        Check { path: "audio.recording.in-media-folder"; text: "Store recorded audios into play folders (if available)" }
                    }

                    Page {
                        Heading { text: "Decoding" }
                        Choice {
                            label: "Hardware decoding"
                            path: "hwdec"
                            values: win.withCurrent(win.view.choices.hwdec, win.s.hwdec)
                            texts: ({ "no": "Off (software)", "auto-copy": "auto-copy (SVP needs copy-back)" })
                        }
                        Heading { text: "Output" }
                        Choice {
                            label: "Video sync"
                            path: "video-sync"
                            values: win.withCurrent(win.view.choices["video-sync"], win.s["video-sync"])
                            texts: ({ "audio": "audio (video follows the sound)",
                                      "display-resample": "display-resample (smooth, resamples audio)" })
                        }
                        Choice {
                            label: "Scaler"
                            path: "scaler"
                            values: win.withCurrent(win.view.choices.scaler, win.s.scaler)
                            texts: ({ "": "mpv's default" })
                        }
                        Heading { text: "Deinterlacing" }
                        Choice {
                            label: "Deinterlace (Ctrl+I)"
                            path: "deinterlace"
                            values: ["auto", "on", "off"]
                            texts: ({ "auto": "Auto (interlaced frames only)", "on": "Always", "off": "Off" })
                        }
                        Heading { text: "Colour" }
                        Check { path: "color.enabled"; text: "Use the colour adjustments (Q switches them off and back)" }
                        Amount { label: "Brightness (W, E)"; path: "color.brightness"; from: -100; live: true }
                        Amount { label: "Contrast (R, T)"; path: "color.contrast"; from: -100; live: true }
                        Amount { label: "Saturation (Y, U)"; path: "color.saturation"; from: -100; live: true }
                        Amount { label: "Hue (I, O)"; path: "color.hue"; from: -100; live: true }
                        Amount { label: "Gamma"; path: "color.gamma"; from: -100; live: true }
                        Heading { text: "Effects" }
                        Effect { effect: "deblock"; text: "Deblock (Ctrl+H)" }
                        Effect { effect: "denoise"; text: "Denoise 3D (Ctrl+N)" }
                        Effect { effect: "soften"; text: "Soften (Ctrl+B)" }
                        Effect { effect: "sharpen"; text: "Sharpen (Ctrl+R)" }
                        Effect { effect: "flip-vertical"; text: "Flip vertical (Ctrl+P)" }
                        Effect { effect: "flip-horizontal"; text: "Flip horizontal (Ctrl+Z)" }
                        Effect { effect: "rotate"; text: "Rotation" }
                        Effect { effect: "auto-level"; text: "Auto level control" }
                        Effect { effect: "levels"; text: "Level control (Ctrl+L)" }
                        Effect { effect: "gradual-denoise"; text: "Gradual denoise" }
                        Effect { effect: "temporal-denoise"; text: "Temporary denoise (Ctrl+M)" }
                        Effect { effect: "offset"; text: "Luma and chroma offset" }
                        Effect { effect: "motion-blur"; text: "Motion blur" }
                        Effect { effect: "warp-sharp"; text: "WarpSharp (approximated: FFmpeg's contrast adaptive sharpening)" }
                        Check { path: "deband"; text: "Deband (mpv's renderer)" }
                        Note { text: "Effects run as RVP's own filters before SVP's, which they leave in place; while one is on, hardware decoding copies the frames back." }
                        Heading { text: "Effect strengths" }
                        Amount { label: "Rotation"; path: "video-effects.rotate-angle"; from: -180; to: 270; step: 1; unit: "°" }
                        Amount { label: "Soften: radius"; path: "video-effects.soften.0"; from: 0.1; to: 5; step: 0.1 }
                        Amount { label: "Soften: luma"; path: "video-effects.soften.1"; from: 0; to: 1; step: 0.05 }
                        Amount { label: "Soften: chroma"; path: "video-effects.soften.2"; from: 0; to: 1; step: 0.05 }
                        Amount { label: "Sharpen: luma"; path: "video-effects.sharpen.0"; from: 0; to: 2; step: 0.05 }
                        Amount { label: "Sharpen: chroma"; path: "video-effects.sharpen.1"; from: 0; to: 2; step: 0.05 }
                        Amount { label: "Deblock"; path: "video-effects.deblock"; from: 1; to: 10 }
                        Amount { label: "Gradual denoise"; path: "video-effects.gradual"; from: 1; to: 10 }
                        Amount { label: "Denoise 3D: luma"; path: "video-effects.denoise.0"; from: 0; to: 20; step: 0.5 }
                        Amount { label: "Denoise 3D: chroma"; path: "video-effects.denoise.1"; from: 0; to: 20; step: 0.5 }
                        Amount { label: "Denoise 3D: time"; path: "video-effects.denoise.2"; from: 0; to: 20; step: 0.5 }
                        Check { path: "video-effects.denoise-quality"; text: "Denoise 3D in high quality (non-local means, slow)" }
                        Amount { label: "Temporal noise reducer: A"; path: "video-effects.temporal.0"; from: 0.005; to: 0.3; step: 0.005 }
                        Amount { label: "Temporal noise reducer: B"; path: "video-effects.temporal.1"; from: 0.005; to: 0.3; step: 0.005 }
                        Amount { label: "Temporal noise reducer: frames"; path: "video-effects.temporal.2"; from: 3; to: 29; step: 2 }
                        Amount { label: "WarpSharp: depth"; path: "video-effects.warpsharp"; from: 0; to: 1; step: 0.05 }
                        Amount { label: "Motion blur: frames"; path: "video-effects.motion-blur"; from: 2; to: 10 }
                        Amount { label: "Deband: threshold"; path: "video-effects.deband.0"; from: 1; to: 200 }
                        Amount { label: "Deband: radius"; path: "video-effects.deband.1"; from: 1; to: 64 }
                        Heading { text: "Levels and offsets" }
                        Check { path: "video-effects.levels.auto-full-range"; text: "Auto level control: full range (0-255)" }
                        Check { path: "video-effects.levels.full-range"; text: "Level control: full range output (0-255)" }
                        Check { path: "video-effects.levels.luma-only"; text: "Change only the luminance" }
                        Check { path: "video-effects.levels.auto-input"; text: "Input range: automatic (the frame's own)" }
                        Amount { label: "Input: from"; path: "video-effects.levels.input.0"; from: 0; to: 255 }
                        Amount { label: "Input: to"; path: "video-effects.levels.input.1"; from: 0; to: 255 }
                        Amount { label: "Output: from"; path: "video-effects.levels.output.0"; from: 0; to: 255 }
                        Amount { label: "Output: to"; path: "video-effects.levels.output.1"; from: 0; to: 255 }
                        Amount { label: "Gamma correction"; path: "video-effects.levels.gamma"; from: 0.1; to: 4; step: 0.05 }
                        Amount { label: "Luma offset X"; path: "video-effects.levels.luma-offset.0"; from: -32; to: 32; unit: "px" }
                        Amount { label: "Luma offset Y"; path: "video-effects.levels.luma-offset.1"; from: -32; to: 32; unit: "px" }
                        Amount { label: "Chroma offset X"; path: "video-effects.levels.chroma-offset.0"; from: -32; to: 32; unit: "px" }
                        Amount { label: "Chroma offset Y"; path: "video-effects.levels.chroma-offset.1"; from: -32; to: 32; unit: "px" }
                    }

                    Page {
                        Heading { text: "Aspect Ratio" }
                        Field { label: "Custom aspect ratio"; path: "picture.custom-aspect"; placeholder: "21:9" }
                        Choice {
                            label: "Correct the aspect ratio of the output device"
                            path: "picture.output-device"
                            values: ["monitor", "tv4x3", "tv16x9", "auto"]
                            texts: ({ "monitor": "Monitor output (square pixels)", "tv4x3": "TV-OUT 4:3",
                                      "tv16x9": "TV-OUT 16:9", "auto": "Detect from the screen's size" })
                        }
                        Check { path: "picture.adjust-to-window"; text: "Adjust the aspect ratio to the window (not in fullscreen)" }
                        Heading { text: "Aspect ratio mode in fullscreen" }
                        Check { path: "picture.fullscreen-mode-limited"; text: "Use the chosen mode only while the margin around the video is small" }
                        Amount { label: "Margin below"; path: "picture.fullscreen-margin"; from: 0; to: 50; unit: "%" }
                        Heading { text: "Bottom margin for subtitles" }
                        Check { path: "picture.subtitle-margin"; text: "Provide a bottom margin for subtitles" }
                        Choice {
                            label: "As bottom margin"
                            path: "picture.subtitle-room"
                            values: ["one-line", "two-lines", "three-lines", "available", "available-always"]
                            texts: ({ "one-line": "Reserve 1 line", "two-lines": "Reserve 2 lines",
                                      "three-lines": "Reserve 3 lines", "available": "Reserve the space below the video",
                                      "available-always": "Reserve the space below the video, even without subtitles" })
                        }
                        Check { path: "picture.subtitle-margin-resize"; text: "Make room by shrinking the picture (otherwise the video moves up)" }
                        Check { path: "picture.subtitle-margin-fullscreen-only"; text: "Only in fullscreen" }
                        Note { text: "Text subtitles go into the room under the video. Keep Video Frame and the other aspect ratio modes are in the Aspect Ratio menu." }
                    }

                    Page {
                        Heading { text: "Extend/Crop" }
                        Choice {
                            label: "Method"
                            path: "picture.extend"
                            values: ["none", "extend", "crop", "pixels"]
                            texts: ({ "none": "Do nothing", "extend": "Extend to the aspect ratio",
                                      "crop": "Crop to the aspect ratio", "pixels": "Extend or crop to the size" })
                        }
                        Field { label: "Aspect ratio"; path: "picture.extend-ratio"; placeholder: "4:3" }
                        Amount { label: "Width"; path: "picture.extend-size.0"; from: 160; to: 7680; step: 16; unit: "px" }
                        Amount { label: "Height"; path: "picture.extend-size.1"; from: 120; to: 4320; step: 8; unit: "px" }
                        Amount { label: "Horizontal position"; path: "picture.position.0"; from: 0; to: 100; unit: "%"; live: true }
                        Amount { label: "Vertical position"; path: "picture.position.1"; from: 0; to: 100; unit: "%"; live: true }
                        Heading { text: "Bottom margin" }
                        Amount { label: "Black below the picture"; path: "picture.bottom-margin"; values: [0, 5, 10, 15, 20, 25, 30]; unit: "%" }
                        Check { path: "picture.bottom-margin-subtitles-only"; text: "Only when subtitles exist" }
                        Note { text: "Extending and cropping run as RVP's own filters before SVP's (the frame changes, so SVP processes what shows); while they are on, hardware decoding copies the frames back. The positions move the picture inside the new frame (Video > Extend/Crop > Move)." }
                    }

                    Page {
                        Heading { text: "Resize the video frame" }
                        Choice {
                            label: "Resize video"
                            path: "picture.resize.when"
                            values: ["never", "smaller", "larger", "always"]
                            texts: ({ "never": "Do nothing", "smaller": "When the input is smaller than the size",
                                      "larger": "When the input is larger than the size", "always": "Always" })
                        }
                        Choice {
                            label: "Size"
                            path: "picture.resize.target"
                            values: ["display", "multiply", "size"]
                            texts: ({ "display": "The display's resolution", "multiply": "Multiply by", "size": "Specify size" })
                        }
                        Amount { label: "Multiply by"; path: "picture.resize.factor"; from: 0.25; to: 4; step: 0.25 }
                        Amount { label: "Width"; path: "picture.resize.width"; from: 320; to: 7680; step: 16; unit: "px" }
                        Amount { label: "Height"; path: "picture.resize.height"; from: 240; to: 4320; step: 8; unit: "px" }
                        Check { path: "picture.resize.auto-height"; text: "Calculate the height from the width" }
                        Choice {
                            label: "Method"
                            path: "picture.resize.method"
                            values: win.view.choices["resize-method"] || []
                        }
                        Note { text: "Resizing changes the frame before RVP's effects and SVP (FFmpeg's scale filter, keeping the video's shape); mpv's renderer still fits the picture to the window with its own scalers (Video page)." }
                    }

                    Page {
                        Heading { text: "Frame size presets" }
                        Repeater {
                            model: (win.s.picture && win.s.picture.presets) ? win.s.picture.presets.length : 0
                            delegate: ColumnLayout {
                                required property int index
                                Layout.fillWidth: true
                                Field { label: "Preset " + (index + 1); path: "picture.presets." + index + ".name" }
                                Amount { label: "Width"; path: "picture.presets." + index + ".width"; from: 10; to: 400; step: 5; unit: "%" }
                                Amount { label: "Height"; path: "picture.presets." + index + ".height"; from: 10; to: 400; step: 5; unit: "%" }
                            }
                        }
                        RowLayout {
                            Button {
                                text: "Add Preset"
                                enabled: !!win.s.picture && win.s.picture.presets.length < 20
                                onClicked: {
                                    const list = win.s.picture.presets.slice()
                                    list.push({ name: "Preset " + (list.length + 1), width: 100, height: 100 })
                                    win.set("picture.presets", list)
                                }
                            }
                            Button {
                                text: "Remove Last"
                                enabled: !!win.s.picture && win.s.picture.presets.length > 1
                                onClicked: win.set("picture.presets", win.s.picture.presets.slice(0, -1))
                            }
                        }
                        Check { path: "picture.remember-frame"; text: "Remember video size/position changes (the next file keeps them)" }
                        Heading { text: "Screen margins" }
                        Amount { label: "Top"; path: "picture.margins.top"; from: 0; to: 40; unit: "%" }
                        Amount { label: "Left"; path: "picture.margins.left"; from: 0; to: 40; unit: "%" }
                        Amount { label: "Bottom"; path: "picture.margins.bottom"; from: 0; to: 40; unit: "%" }
                        Amount { label: "Right"; path: "picture.margins.right"; from: 0; to: 40; unit: "%" }
                        Check { path: "picture.margins-fullscreen-only"; text: "Use the margins only in fullscreen" }
                        Check { path: "picture.remember-margins"; text: "Remember screen margin changes (these values come back at the next start)" }
                    }

                    Page {
                        Heading { text: "3D video" }
                        Choice {
                            label: "3D video mode"
                            path: "stereo.mode"
                            values: ["off", "on", "auto"]
                            texts: ({ "off": "Off", "on": "On", "auto": "On for files that say they are 3D" })
                        }
                        Choice {
                            label: "Input (source)"
                            path: "stereo.input"
                            values: ["side-by-side", "top-bottom", "original"]
                            texts: ({ "side-by-side": "Side by side", "top-bottom": "Top and bottom",
                                      "original": "Original image (not 3D)" })
                        }
                        Check { path: "stereo.full-resolution"; text: "Full resolution (each image as wide or tall as the frame)" }
                        Check { path: "stereo.swap"; text: "Swap left and right images" }
                        Check { path: "stereo.auto-detect"; text: "Detect the input from the file (stereo metadata, suffixes such as -lr, -ab, hsbs)" }
                        Choice {
                            label: "Output (screen)"
                            path: "stereo.output"
                            values: ["side-by-side", "top-bottom", "left", "right", "rows", "columns",
                                     "red-cyan-gray", "red-cyan-half", "red-cyan-color", "red-cyan-dubois",
                                     "green-magenta-gray", "green-magenta-half", "green-magenta-color", "green-magenta-dubois",
                                     "yellow-blue-gray", "yellow-blue-half", "yellow-blue-color", "yellow-blue-dubois",
                                     "red-blue-gray", "red-green-gray"]
                            texts: ({ "side-by-side": "Side by side", "top-bottom": "Top and bottom",
                                      "left": "3D to 2D (left or top image)", "right": "3D to 2D (right or bottom image)",
                                      "rows": "Row interleave", "columns": "Column interleave",
                                      "red-cyan-gray": "Anaglyph red/cyan gray", "red-cyan-half": "Anaglyph red/cyan half colored",
                                      "red-cyan-color": "Anaglyph red/cyan colored", "red-cyan-dubois": "Anaglyph red/cyan Dubois",
                                      "green-magenta-gray": "Anaglyph green/magenta gray", "green-magenta-half": "Anaglyph green/magenta half colored",
                                      "green-magenta-color": "Anaglyph green/magenta colored", "green-magenta-dubois": "Anaglyph green/magenta Dubois",
                                      "yellow-blue-gray": "Anaglyph yellow/blue gray", "yellow-blue-half": "Anaglyph yellow/blue half colored",
                                      "yellow-blue-color": "Anaglyph yellow/blue colored", "yellow-blue-dubois": "Anaglyph yellow/blue Dubois",
                                      "red-blue-gray": "Anaglyph red/blue gray", "red-green-gray": "Anaglyph red/green gray" })
                        }
                        Amount { label: "Right image offset X"; path: "stereo.offset.0"; from: -100; to: 100; step: 2; unit: "px" }
                        Amount { label: "Right image offset Y"; path: "stereo.offset.1"; from: -100; to: 100; step: 2; unit: "px" }
                        Check { path: "stereo.invert"; text: "Flip both images horizontally" }
                        Note { text: "FFmpeg's stereo3d converts the frame before RVP's other filters and SVP's. Shutter glasses, NVIDIA 3D Vision and Windows' stereo output need a stereo swap chain, which mpv's renderer does not have." }
                        Heading { text: "360° video" }
                        Choice {
                            label: "360° video mode"
                            path: "vr.mode"
                            values: ["off", "on", "auto"]
                            texts: ({ "off": "Off", "on": "On", "auto": "On for files named as 360° video" })
                        }
                        Choice {
                            label: "Projection"
                            path: "vr.projection"
                            values: ["auto", "equirectangular", "dual-fisheye", "cubemap", "equi-angular", "equi-angular-stereo"]
                            texts: ({ "auto": "From the frame's shape", "equirectangular": "Equirectangular",
                                      "dual-fisheye": "Dual fisheye", "cubemap": "Cubemap (3x2)",
                                      "equi-angular": "Equi-angular cubemap 3x2 (mono 2D)",
                                      "equi-angular-stereo": "Equi-angular cubemap 2x3 (stereo 3D)" })
                        }
                        Check { path: "vr.four-views"; text: "Four camera angles (front, left, right, rear)" }
                        Amount { label: "Field of view"; path: "vr.fov"; from: 30; to: 150; step: 5; unit: "°" }
                        Note { text: "Drag the video with the mouse to look around." }
                    }

                    Page {
                        Heading { text: "Pre-resize pixel shaders" }
                        Check { path: "shaders.enabled"; text: "Use pre-resize pixel shaders (Shift+P)" }
                        FileList { path: "shaders.files"; ordered: true }
                        ShaderPicker { target: "shaders.files"; stage: "pre" }
                        Heading { text: "Post-resize pixel shaders" }
                        Check { path: "shaders.post-enabled"; text: "Use post-resize pixel shaders (Ctrl+Alt+P)" }
                        FileList { path: "shaders.post-files"; ordered: true }
                        ShaderPicker { target: "shaders.post-files"; stage: "post" }
                        Heading { text: "Shader folders" }
                        Note {
                            text: win.view.choices["potplayer-shaders"]
                                  ? "PotPlayer's pixel shaders (HLSL, translated when they load): " + win.view.choices["potplayer-shaders"]
                                  : "PotPlayer's pixel shader folder was not found."
                        }
                        FileList { path: "shaders.folders" }
                        RowLayout {
                            Button {
                                text: "Add a Folder…"
                                onClicked: {
                                    folderListChooser.target = "shaders.folders"
                                    folderListChooser.open()
                                }
                            }
                            Button {
                                text: "Add Shader Files…"
                                onClicked: win.chooseFiles("shaders.files", "Pixel shaders",
                                                           ["Shaders (*.glsl *.hook *.hlsl *.txt *.fx)", "All files (*)"])
                            }
                            Button {
                                text: "Open Shader Folder"
                                onClicked: win.player.run_action("shaders-folder")
                            }
                            Button {
                                text: "Reload"
                                onClicked: win.player.run_action("shaders-reload")
                            }
                        }
                        Note { text: "The lists run in their order: pre-resize shaders on the video's own picture, post-resize ones after scaling. mpv's own shaders (.glsl, .hook) run where they say; RVP's set comes with RVP. RVP's shader folder: " + (win.view.shader_folder || "") }
                    }

                    Page {
                        Heading { text: "SVP" }
                        Field {
                            label: "IPC endpoint SVP attaches to"
                            path: "ipc-server"
                            placeholder: "empty: SVP mode off"
                            Button {
                                text: "Default"
                                onClicked: win.set("ipc-server", win.view["default-ipc"])
                            }
                        }
                        Label {
                            text: "Status: " + (win.view.svp_status || "")
                            wrapMode: Text.Wrap
                            Layout.fillWidth: true
                        }
                        Note {
                            text: "Endpoint in use: " + (win.view.svp_endpoint || "none")
                                  + ". SVP 4 Manager finds a player by this pipe or socket while it plays; a second RVP window uses mpvSockets/<process id> in the temporary folder, which SVP finds too. In SVP mode hardware decoding copies the frames back, so SVP's filter gets them."
                        }
                        Heading { text: "VapourSynth" }
                        Check { path: "vapoursynth.enabled"; text: "Run the selected script (before SVP's filter)" }
                        FileList { path: "vapoursynth.slots"; selectPath: "vapoursynth.script" }
                        Button {
                            text: "Add Script…"
                            onClicked: win.chooseFiles("vapoursynth.slots", "VapourSynth scripts",
                                                       ["VapourSynth scripts (*.vpy)", "All files (*)"])
                        }
                        Label { text: "Plugins loaded first (mvtools for PotPlayer's motion interpolation)" }
                        FileList { path: "vapoursynth.plugins" }
                        Button {
                            text: "Add Plugins…"
                            onClicked: win.chooseFiles("vapoursynth.plugins", "VapourSynth plugins",
                                                       [win.view.windows ? "Plugins (*.dll)" : "Plugins (*.so)", "All files (*)"])
                        }
                        Amount { label: "Buffered frames"; path: "vapoursynth.buffered-frames"; from: 1; to: 64 }
                        Field {
                            label: "VSScript library"
                            path: "vapoursynth.library"
                            optional: true
                            placeholder: "SVP's runtime"
                            Button {
                                text: "Browse…"
                                onClicked: win.chooseFile("vapoursynth.library", "VSScript library",
                                                          [win.view.windows ? "VSScript (VSScript.dll)" : "VSScript (libvapoursynth-script.so*)",
                                                           "All files (*)"])
                            }
                        }
                        Note {
                            text: "In use: " + (win.view.vapoursynth_runtime || "none found")
                                  + ". A new library takes effect at the next start. PotPlayer's .vpy scripts run unchanged: RVP wraps them for today's VapourSynth (get_core) and loads the plugins first."
                        }
                    }

                    Page {
                        Heading { text: "Subtitles" }
                        Field { label: "Preferred languages"; path: "subtitle-languages"; placeholder: "en,ru" }
                        Choice {
                            label: "Load subtitle files"
                            path: "subtitle-auto"
                            values: win.withCurrent(win.view.choices["subtitle-auto"], win.s["subtitle-auto"])
                            texts: ({ "no": "No", "exact": "With the video's name",
                                      "fuzzy": "Containing the video's name", "all": "All in the folder" })
                        }
                        Amount { label: "Size"; path: "subtitle-scale"; unit: "%"; factor: 100; from: 30; to: 300; step: 5 }
                        Field {
                            label: "Font"
                            path: "subtitle-font"
                            placeholder: "mpv's default (sans-serif)"
                            Button {
                                text: "Choose…"
                                onClicked: {
                                    const family = win.value("subtitle-font")
                                    subtitleFontChooser.currentFont = Qt.font({
                                        family: family ? family : "sans-serif", pointSize: 20 })
                                    subtitleFontChooser.open()
                                }
                            }
                        }
                        Amount { label: "Sync step (< and >)"; path: "subtitle-delay-step"; values: [0.01, 0.05, 0.1, 0.2, 0.25, 0.5, 1, 2, 5, 10, 30, 60]; format: win.seconds }
                    }

                    Loader {
                        id: keysPage
                        Component.onCompleted: setSource("KeysAndMouse.qml", { player: win.player })
                    }

                    Page {
                        Heading { text: "Skin" }
                        Field {
                            label: "PotPlayer skin (.dsf)"
                            path: "skin"
                            optional: true
                            placeholder: "RVP Default (built in)"
                            Button {
                                text: "Browse…"
                                onClicked: skinChooser.open()
                            }
                        }
                        Repeater {
                            model: win.view.choices.skin || []
                            delegate: RadioButton {
                                required property string modelData
                                text: modelData
                                checked: win.s.skin === modelData
                                onClicked: win.set("skin", modelData)
                            }
                        }
                        Note { text: "Empty: RVP Default, RVP's own skin. RVP also draws PotPlayer's .dsf skins (BlackSpace is the one it is made for). The skin changes at the next start." }
                    }

                    Page {
                        Heading { text: "Window size definition (relative to the desktop)" }
                        Amount { label: "Alt+1"; path: "window.desktop-sizes.0"; from: 10; to: 100; unit: "%" }
                        Amount { label: "Alt+2"; path: "window.desktop-sizes.1"; from: 10; to: 100; unit: "%" }
                        Amount { label: "Alt+3"; path: "window.desktop-sizes.2"; from: 10; to: 100; unit: "%" }
                        Amount { label: "Alt+4"; path: "window.desktop-sizes.3"; from: 10; to: 100; unit: "%" }
                        Heading { text: "Newly opened videos" }
                        Choice {
                            label: "Window size"
                            path: "window.open-size"
                            values: ["video", "last", "custom", "desktop"]
                            texts: ({ "video": "The video's size", "last": "Keep the window's size",
                                      "custom": "The custom size", "desktop": "The Alt+1 size" })
                        }
                        Amount { label: "Video size times"; path: "window.open-scale"; values: [0.25, 0.5, 0.75, 1, 1.25, 1.5, 2, 3, 4]; format: v => v + "x" }
                        Amount { label: "Twice the size below a width of"; path: "window.double-below"; from: 0; to: 1920; step: 32; unit: "px"; format: v => v === 0 ? "never" : v + " px" }
                        Check { path: "window.open-first-only"; text: "Operate only with the first playback at a run" }
                        Note { text: "These apply when \"Fit the window to a newly opened video\" is on (Playback)." }
                        Heading { text: "Custom size and steps" }
                        Amount { label: "Custom width"; path: "window.custom-size.0"; from: 160; to: 7680; step: 16; unit: "px" }
                        Amount { label: "Custom height"; path: "window.custom-size.1"; from: 160; to: 4320; step: 8; unit: "px" }
                        Amount { label: "Increase and decrease"; path: "window.grow-step"; from: 1; to: 50; unit: "%" }
                        Amount { label: "Move step"; path: "window.move-step"; from: 1; to: 200; unit: "px" }
                    }

                    Page {
                        Heading { text: "When on fullscreen mode" }
                        Choice {
                            label: "Display"
                            path: "window.fullscreen-display"
                            values: ["current", "primary"]
                            texts: ({ "current": "The window's display", "primary": "The primary display" })
                        }
                        Check { path: "window.black-other-displays"; text: "Fill other monitors black in fullscreen" }
                        Choice {
                            label: "Stay on top"
                            path: "on-top"
                            values: ["never", "always", "while-playing", "fullscreen-playing"]
                            texts: ({ "never": "Never", "always": "Always", "while-playing": "While playing",
                                      "fullscreen-playing": "While playing in fullscreen" })
                        }
                        Note { text: "Fullscreen (on Another Display) is Ctrl+Shift+Enter; Alt+Enter, Ctrl+Enter and Ctrl+Alt+Enter keep the aspect ratio, stretch, or stretch keeping the aspect ratio." }
                    }

                    Page {
                        Heading { text: "Saved frames" }
                        Field {
                            label: "Folder"
                            path: "image-folder"
                            optional: true
                            placeholder: "Pictures/RVP in the home folder"
                            Button {
                                text: "Browse…"
                                onClicked: {
                                    folderChooser.target = "image-folder"
                                    folderChooser.open()
                                }
                            }
                        }
                        Check { path: "capture.next-to-media"; text: "Store captured images into play folders (if available)" }
                        Choice {
                            label: "Format"
                            path: "image-format"
                            values: ["png", "jpeg", "bmp", "webp"]
                            texts: ({ "png": "PNG", "jpeg": "JPEG", "bmp": "BMP", "webp": "WEBP" })
                        }
                        Amount { label: "JPEG and WEBP quality"; path: "capture.quality"; from: 10; to: 100; step: 5; unit: "%" }
                        Heading { text: "File names and pictures" }
                        Check { path: "capture.name-date"; text: "Include system date and time in filename" }
                        Check { path: "capture.name-elapsed"; text: "Include file elapsed time in filename" }
                        Check { path: "capture.with-subtitles"; text: "Include subtitles in source frames" }
                        Check { path: "capture.correct-aspect"; text: "Correct aspect ratio when capturing" }
                        Note { text: "Screen frames (Ctrl+Alt+E, Ctrl+Alt+C) are the picture as shown, with subtitles and mpv's messages." }
                        Heading { text: "Consecutive images (Ctrl+G)" }
                        Amount { label: "Every"; path: "capture.interval"; values: [0.1, 0.2, 0.5, 1, 2, 3, 5, 10, 15, 30, 60, 120, 300, 600, 1800, 3600]; format: win.seconds }
                        Amount { label: "Images"; path: "capture.count"; values: [1, 2, 3, 5, 10, 20, 30, 50, 100, 200, 500, 1000, 10000] }
                        Heading { text: "Thumbnail image (Alt+N)" }
                        Amount { label: "Columns"; path: "capture.sheet.columns"; from: 1; to: 10 }
                        Amount { label: "Rows"; path: "capture.sheet.rows"; from: 1; to: 20 }
                        Amount { label: "Width"; path: "capture.sheet.width"; from: 320; to: 7680; step: 160; unit: "px" }
                        Check { path: "capture.sheet.info"; text: "File information above the pictures" }
                        Check { path: "capture.sheet.times"; text: "The time on each picture" }
                        Check { path: "capture.sheet.exact"; text: "Exact frames (slower)" }
                        Heading { text: "Recording" }
                        Field {
                            label: "Streams (Record Stream, Ctrl+A)"
                            path: "record.stream-folder"
                            optional: true
                            placeholder: "Videos/RVP in the home folder"
                            Button {
                                text: "Browse…"
                                onClicked: {
                                    folderChooser.target = "record.stream-folder"
                                    folderChooser.open()
                                }
                            }
                        }
                        Field {
                            label: "Videos (Record Video, Alt+C)"
                            path: "record.video-folder"
                            optional: true
                            placeholder: "Videos/RVP in the home folder"
                            Button {
                                text: "Browse…"
                                onClicked: {
                                    folderChooser.target = "record.video-folder"
                                    folderChooser.open()
                                }
                            }
                        }
                        Check { path: "record.next-to-media"; text: "Store video files into play folders (if available)" }
                        Note { text: "Record Stream saves the file as it comes, without decoding. Record Video encodes the stretch from its start to its stop (H.264 and AAC) with RVP's video filters, after it stops." }
                    }

                    Page {
                        Heading { text: "Visualization settings" }
                        Check { path: "visual.enabled"; text: "Enable visualization for audio tracks" }
                        Check { path: "visual.album-art"; text: "Display album art/cover pictures" }
                        Choice {
                            label: "Visualization mode"
                            path: "visual.mode"
                            values: ["bars", "spectrum", "scope", "waves", "slideshow"]
                            texts: ({ "bars": "Spectrum bars", "spectrum": "Spectrogram", "scope": "Vector scope",
                                      "waves": "Waveform", "slideshow": "Slideshow" })
                        }
                        Amount { label: "Width"; path: "visual.size.0"; from: 320; to: 3840; step: 160; unit: "px" }
                        Amount { label: "Height"; path: "visual.size.1"; from: 180; to: 2160; step: 90; unit: "px" }
                        Amount { label: "Frames a second"; path: "visual.rate"; from: 5; to: 60; step: 5 }
                        Amount { label: "Change visualization every"; path: "visual.change-every"; values: [0, 10, 15, 20, 30, 60, 120, 300, 600]; format: v => v === 0 ? "never" : win.seconds(v) }
                        Heading { text: "Slideshow" }
                        Note { text: "Without folders, the slideshow shows the pictures of the playing file's folder." }
                        FileList { path: "visual.folders"; ordered: true }
                        Button {
                            text: "Add a folder of pictures…"
                            onClicked: {
                                folderListChooser.target = "visual.folders"
                                folderListChooser.open()
                            }
                        }
                        Amount { label: "Display time"; path: "visual.slide-seconds"; values: [1, 2, 3, 5, 8, 10, 15, 20, 30, 60, 120, 300]; format: win.seconds }
                        Amount { label: "Transition"; path: "visual.fade-seconds"; values: [0, 0.25, 0.5, 1, 1.5, 2, 3, 5]; format: win.seconds }
                        Choice {
                            label: "Fill mode"
                            path: "visual.fill"
                            values: ["fit", "fill", "stretch"]
                            texts: ({ "fit": "Fit (keep the whole picture)", "fill": "Fill (crop)", "stretch": "Stretch" })
                        }
                        Check { path: "visual.random"; text: "Random slideshow" }
                        Check { path: "visual.show-name"; text: "Show filename" }
                    }
                }
            }
        }

        Label {
            Layout.fillWidth: true
            text: win.notice + (win.restartNeeded ? (win.notice !== "" ? "  " : "") + "Restart RVP to apply it." : "")
            wrapMode: Text.Wrap
        }

        RowLayout {
            Layout.fillWidth: true
            Label {
                Layout.fillWidth: true
                elide: Text.ElideMiddle
                opacity: 0.7
                text: (win.view.portable ? "Portable: " : "") + (win.view.file || "")
            }
            Button {
                text: "Import…"
                onClicked: importChooser.open()
            }
            Button {
                text: "Export…"
                onClicked: exportChooser.open()
            }
            Button {
                text: "Close"
                onClicked: win.close()
            }
        }
    }
}
