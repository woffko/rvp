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

    // Keys of rvp_app::SettingsPage.
    readonly property var pages: [
        { key: "playback", title: "Playback" },
        { key: "playlist", title: "Playlist" },
        { key: "audio", title: "Audio" },
        { key: "audio-volume", title: "Audio: Volume Control" },
        { key: "equalizer", title: "Audio: Equalizer" },
        { key: "normalizer", title: "Audio: Normalizer/Freeverb" },
        { key: "audio-effects", title: "Audio: Crystality/Noise Reduction" },
        { key: "audio-sync", title: "Audio: Language/Sync/Other" },
        { key: "video", title: "Video" },
        { key: "shaders", title: "Shaders" },
        { key: "svp", title: "SVP and VapourSynth" },
        { key: "subtitles", title: "Subtitles" },
        { key: "input", title: "Keys and Mouse" },
        { key: "skins", title: "Skins" }
    ]
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
        pageList.currentIndex = index
        if (pages[index].key === "input" && keysPage.item)
            keysPage.item.begin()
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

            ListView {
                id: pageList
                Layout.preferredWidth: 190
                Layout.fillHeight: true
                model: win.pages
                clip: true
                delegate: ItemDelegate {
                    required property var modelData
                    required property int index
                    width: pageList.width
                    text: modelData.title
                    highlighted: ListView.isCurrentItem
                    onClicked: win.showPage(index)
                }
            }

            StackLayout {
                id: stack
                Layout.fillWidth: true
                Layout.fillHeight: true
                currentIndex: pageList.currentIndex

                Page {
                    Heading { text: "Seek steps" }
                    Amount { label: "Left / Right"; path: "seek-seconds.0"; values: [0.5, 1, 2, 3, 5, 10, 15, 20, 30, 45, 60, 90, 120, 180, 300, 600, 900, 1800, 3600]; format: win.seconds }
                    Amount { label: "Ctrl+Left / Right"; path: "seek-seconds.1"; values: [0.5, 1, 2, 3, 5, 10, 15, 20, 30, 45, 60, 90, 120, 180, 300, 600, 900, 1800, 3600]; format: win.seconds }
                    Amount { label: "Shift+Left / Right"; path: "seek-seconds.2"; values: [0.5, 1, 2, 3, 5, 10, 15, 20, 30, 45, 60, 90, 120, 180, 300, 600, 900, 1800, 3600]; format: win.seconds }
                    Amount { label: "Ctrl+Alt+Left / Right"; path: "seek-seconds.3"; values: [0.5, 1, 2, 3, 5, 10, 15, 20, 30, 45, 60, 90, 120, 180, 300, 600, 900, 1800, 3600]; format: win.seconds }
                    Check { path: "keyframe-seeks"; text: "Seek to keyframes (faster, less exact)" }
                    Heading { text: "Resume" }
                    Check { path: "remember-positions"; text: "Resume every file at its last position" }
                    Check { path: "reopen-last-file"; text: "Reopen the last file, paused, when RVP starts without files" }
                    Heading { text: "Window" }
                    Check { path: "fit-window-to-video"; text: "Fit the window to a newly opened video" }
                    Check { path: "keep-aspect-window"; text: "Keep the video's shape while the window is resized" }
                    Check { path: "prevent-screen-saver"; text: "Prevent screen saver during playback" }
                    Choice {
                        label: "Stay on top"
                        path: "on-top"
                        values: ["never", "always", "while-playing"]
                        texts: ({ "never": "Never", "always": "Always", "while-playing": "While playing" })
                    }
                    Choice {
                        label: "At the end of a file"
                        path: "repeat"
                        values: ["off", "album", "item"]
                        texts: ({ "off": "Play the next one", "album": "Repeat the playlist",
                                  "item": "Repeat the file" })
                    }
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
                    Check { path: "deband"; text: "Deband (mpv's renderer)" }
                    Note { text: "Effects run as RVP's own filters before SVP's, which they leave in place; while one is on, hardware decoding copies the frames back." }
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
                    Choice {
                        label: "Format"
                        path: "image-format"
                        values: ["png", "jpeg"]
                        texts: ({ "png": "PNG", "jpeg": "JPEG" })
                    }
                }

                Page {
                    Heading { text: "GLSL shaders" }
                    Check { path: "shaders.enabled"; text: "Use the shaders (Shift+P)" }
                    FileList { path: "shaders.files"; ordered: true }
                    RowLayout {
                        Button {
                            text: "Add Shaders…"
                            onClicked: win.chooseFiles("shaders.files", "GLSL shaders",
                                                       ["mpv shaders (*.glsl *.hook)", "All files (*)"])
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
                    Note { text: "mpv's user shaders run in the listed order. PotPlayer's HLSL pixel shaders do not run in mpv. Shader folder: " + (win.view.shader_folder || "") }
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
