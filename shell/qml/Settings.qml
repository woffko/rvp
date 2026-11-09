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
    title: qsTr("Settings")
    // Dialog Box: "Always keep modeless dialog boxes on top" (M14).
    flags: Qt.Window | ((win.player.general_look ? JSON.parse(win.player.general_look) : {}).dialogOnTop === true
                        ? Qt.WindowStaysOnTopHint : 0)
    width: 860
    height: 640
    minimumWidth: 640
    minimumHeight: 440

    // The calibration window (Calibration.qml), made on first use: the
    // Motion Smoothing page's button and the self-test open it.
    function openCalibration() {
        if (!calibrationLoader.item)
            calibrationLoader.setSource("Calibration.qml", { player: win.player, owner: win })
        calibrationLoader.item.begin()
        return calibrationLoader.item
    }
    Loader { id: calibrationLoader }

    // Keys of rvp_app::SettingsPage, in the order of the pages in `stack`.
    readonly property var pages: [
        { key: "playback", title: qsTr("Playback") },
        { key: "time-jump", title: qsTr("Time Jump") },
        { key: "skip", title: qsTr("Skip") },
        { key: "section-repeat", title: qsTr("Section Repeat") },
        { key: "scene-browser", title: qsTr("Scene Browser") },
        { key: "playlist", title: qsTr("Playlist") },
        { key: "devices", title: qsTr("Device") },
        { key: "audio", title: qsTr("Audio") },
        { key: "audio-volume", title: qsTr("Volume Control") },
        { key: "equalizer", title: qsTr("Equalizer") },
        { key: "normalizer", title: qsTr("Normalizer/Freeverb") },
        { key: "audio-effects", title: qsTr("Crystality/Noise Reduction") },
        { key: "audio-sync", title: qsTr("Language/Sync/Other") },
        { key: "video", title: qsTr("Video") },
        { key: "aspect-ratio", title: qsTr("Aspect Ratio") },
        { key: "extend-crop", title: qsTr("Extend/Crop") },
        { key: "resize", title: qsTr("Resize") },
        { key: "frame-size", title: qsTr("Frame Size") },
        { key: "stereo", title: qsTr("3D and 360°") },
        { key: "shaders", title: qsTr("Pixel Shaders") },
        { key: "svp", title: qsTr("SVP and VapourSynth") },
        { key: "smoothing", title: qsTr("Motion Smoothing") },
        { key: "subtitles", title: qsTr("Subtitles") },
        { key: "subtitle-style", title: qsTr("Font Style") },
        { key: "subtitle-location", title: qsTr("Fade/3D/Location") },
        { key: "subtitle-sync", title: qsTr("Language/Sync/Other") },
        { key: "subtitle-browser", title: qsTr("Subtitle Browser") },
        { key: "speech", title: qsTr("Subtitles from Audio") },
        { key: "input", title: qsTr("Keys and Mouse") },
        { key: "input-options", title: qsTr("Keyboard and Mouse Options") },
        { key: "skins", title: qsTr("Skins") },
        { key: "logo", title: qsTr("Logo") },
        { key: "osc", title: qsTr("On Screen Control") },
        { key: "osd", title: qsTr("OSD Messages") },
        { key: "window-size", title: qsTr("Window Size") },
        { key: "fullscreen-mode", title: qsTr("Fullscreen Mode") },
        { key: "capture", title: qsTr("Video Capture") },
        { key: "visualization", title: qsTr("Visualization") },
        { key: "decoders", title: qsTr("Decoders") },
        { key: "general", title: qsTr("General") },
        { key: "start", title: qsTr("When Starting") },
        { key: "dialogs", title: qsTr("Dialog Box") },
        { key: "association", title: qsTr("Association") },
        { key: "screensaver", title: qsTr("Screen Saver") },
        { key: "presets", title: qsTr("Configuration") },
        { key: "tts", title: qsTr("TTS") },
        { key: "touch", title: qsTr("Touch") },
        { key: "xinput", title: qsTr("XInput") },
        { key: "remote", title: qsTr("Remote Control") },
        { key: "log", title: qsTr("Log") }
    ]
    // PotPlayer's tree: each group opens into its pages, and the selected
    // group's pages show as tabs over the page. `page` is the group's own
    // page (General has none yet: M14 brings PotPlayer's general options).
    readonly property var tree: [
        { title: qsTr("General"), page: "general",
          pages: ["start", "input", "input-options", "skins", "logo", "osc", "visualization", "osd", "dialogs", "playlist"] },
        { title: qsTr("Playback"), page: "playback",
          pages: ["time-jump", "skip", "section-repeat", "scene-browser", "window-size", "fullscreen-mode"] },
        { title: qsTr("Subtitles"), page: "subtitles", pages: ["subtitle-style", "subtitle-location", "subtitle-sync", "subtitle-browser", "speech"] },
        { title: qsTr("Device"), page: "devices", pages: [] },
        { title: qsTr("Filter Control"), page: "decoders", pages: [] },
        { title: qsTr("Video"), page: "video",
          pages: ["aspect-ratio", "resize", "extend-crop", "frame-size", "stereo", "shaders", "svp", "smoothing",
                  "capture"] },
        { title: qsTr("Association"), page: "association", pages: [] },
        { title: qsTr("Screen Saver"), page: "screensaver", pages: [] },
        { title: qsTr("Configuration"), page: "presets", pages: [] },
        { title: qsTr("Accessibility"), page: "", pages: ["tts", "touch", "xinput"] },
        { title: qsTr("Remote Control"), page: "remote", pages: [] },
        { title: qsTr("Audio"), page: "audio",
          pages: ["audio-volume", "equalizer", "normalizer", "audio-effects", "audio-sync"] },
        { title: qsTr("Log"), page: "log", pages: [] }
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
    // The GPU power budget's last reading (the Video page shows it).
    property var gpuPower: ({})
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
        if (resetPending) {
            resetPending = false
            notice = ""
        }
        current = index
        // Preferences opens on this page next time, also after a restart.
        player.settings_page_shown(pages[index].key)
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

    // Search (M14): every option's text and every heading of every page,
    // gathered when a search starts; `found` holds the matches.
    property string searchText: ""
    property var searchIndex: []
    readonly property var found: {
        const words = searchText.toLowerCase().split(/\s+/).filter(w => w !== "")
        if (words.length === 0)
            return []
        return searchIndex.filter(entry => {
            const text = (entry.text + " " + pages[entry.page].title).toLowerCase()
            return words.every(word => text.indexOf(word) >= 0)
        }).slice(0, 200)
    }
    // The items of a page that name a setting (`path`) or head a group.
    function pageEntries(item, page, out) {
        for (const child of item.children || []) {
            const named = typeof child.path === "string" && child.path !== ""
            const text = named ? (child.label !== undefined && child.label !== "" ? child.label : child.text)
                               : child.objectName === "heading" ? child.text : undefined
            if (typeof text === "string" && text !== "")
                out.push({ page: page, text: text, item: child, heading: !named })
            pageEntries(child, page, out)
        }
        return out
    }
    function search(text) {
        if (searchText === "" && text !== "") {
            let index = []
            for (let i = 0; i < pages.length; i++)
                index = index.concat(pageEntries(stack.children[i], i, []))
            searchIndex = index
        }
        searchText = text
    }
    // Shows a match: its page, scrolled to it, and marks it a moment.
    function openFound(entry) {
        showPage(entry.page)
        Qt.callLater(() => {
            const page = stack.children[entry.page]
            const flick = page.contentItem
            if (flick && flick.contentY !== undefined) {
                const at = entry.item.mapToItem(flick.contentItem, 0, 0)
                flick.contentY = Math.max(0, Math.min(at.y - 24, flick.contentHeight - flick.height))
            }
            const spot = entry.item.mapToItem(win.contentItem, 0, 0)
            mark.x = spot.x - 4
            mark.y = spot.y - 2
            mark.width = Math.min(entry.item.width + 8, win.width - mark.x)
            mark.height = entry.item.height + 4
            mark.opacity = 1
            markFade.restart()
        })
    }
    // Reset page (M14): the settings the shown page names go back to
    // RVP's defaults once confirmed.
    property bool resetPending: false
    function pagePaths(index) {
        const paths = []
        for (const entry of pageEntries(stack.children[index], index, []))
            if (!entry.heading && paths.indexOf(entry.item.path) < 0)
                paths.push(entry.item.path)
        return paths
    }
    function resetPage() {
        const paths = pagePaths(current)
        resetPending = false
        if (paths.length > 0)
            report(player.reset_settings(JSON.stringify(paths)), qsTr("The page's options are RVP's defaults again."))
    }
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
        report(player.set_setting(path, JSON.stringify(newValue)), qsTr("Applied."))
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
        objectName: "heading"
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
    // The GPU power budget's reading, refreshed while the Video page shows.
    Timer {
        interval: 2000
        repeat: true
        triggeredOnStart: true
        running: win.visible && win.currentKey === "video"
        onTriggered: win.gpuPower = JSON.parse(player.gpu_power_json())
    }
    function gpuPowerText() {
        const gpu = gpuPower
        if (gpu.on !== true)
            return qsTr("Off: RVP does not read the card's power.")
        if (!gpu.cards || gpu.cards.length === 0)
            return gpu.reading ? qsTr("No NVIDIA or AMD card's power can be read.")
                               : qsTr("Read while a file plays.")
        let text = qsTr("Now: %1").arg(gpu.cards.map(c => c.unit === "%"
                                           ? qsTr("%1 %2 % of its power").arg(c.name).arg(Math.round(c.draw))
                                           : qsTr("%1 %2 W of %3 W (%4 %)").arg(c.name).arg(Math.round(c.draw))
                                             .arg(Math.round(c.rated)).arg(Math.round(100 * c.draw / c.rated))).join(", "))
        if (gpu.taken.length > 0)
            text += qsTr(". Lightened: %1.").arg(gpu.taken.join(", "))
        return text
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
        // Shown while the setting is unset (libmpv's own default), with a
        // Default button once it is set.
        property var fallback: undefined
        readonly property bool unset: {
            const v = win.value(amount.path)
            return v === undefined || v === null
        }
        readonly property bool stepped: values.length > 0
        readonly property real shown: {
            const v = win.value(amount.path)
            if (v === undefined || v === null)
                return amount.fallback === undefined || amount.fallback === null ? NaN : amount.fallback * amount.factor
            return v * amount.factor
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
            // Longer labels (translations) wrap rather than run over.
            wrapMode: Text.WordWrap
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
        Button {
            visible: amount.fallback !== undefined && !amount.unset
            text: qsTr("Default")
            onClicked: win.set(amount.path, null)
        }
    }
    // A colour as `#AARRGGBB` (the alpha is the opacity), chosen in Qt's
    // colour dialog; unset: `fallback` (libmpv's default).
    component ColorChoice: RowLayout {
        id: colorChoice
        property string label
        property string path
        property string fallback
        readonly property string current: {
            const v = win.value(colorChoice.path)
            return v === undefined || v === null ? colorChoice.fallback : v
        }
        Label {
            text: colorChoice.label
            // Longer labels (translations) wrap rather than run over.
            wrapMode: Text.WordWrap
            Layout.preferredWidth: 240
        }
        Rectangle {
            implicitWidth: 48
            implicitHeight: 24
            border.width: 1
            border.color: "gray"
            // QML reads #AARRGGBB as mpv writes it.
            color: colorChoice.current !== "" ? colorChoice.current : "transparent"
        }
        Button {
            text: qsTr("Choose…")
            onClicked: {
                colorChooser.target = colorChoice.path
                colorChooser.selectedColor = colorChoice.current !== "" ? colorChoice.current : "white"
                colorChooser.open()
            }
        }
        Button {
            visible: win.value(colorChoice.path) !== undefined && win.value(colorChoice.path) !== null
            text: qsTr("Default")
            onClicked: win.set(colorChoice.path, null)
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
            // Longer labels (translations) wrap rather than run over.
            wrapMode: Text.WordWrap
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
            // Longer labels (translations) wrap rather than run over.
            wrapMode: Text.WordWrap
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
            text: qsTr("Add")
            enabled: shaderChoice.currentIndex >= 0
            onClicked: win.listAdd(picker.target, [picker.choices[shaderChoice.currentIndex].path])
        }
    }

    // Common sizes for a `[width, height]` setting (`target.0`/`.1`), or
    // for one with `width` and `height` fields.
    component SizePresets: RowLayout {
        id: sizes
        property string target
        Label { text: qsTr("Sizes:") }
        Repeater {
            model: [[1280, 720], [1920, 1080], [2560, 1440], [3840, 2160]]
            delegate: Button {
                required property var modelData
                text: modelData[0] + "×" + modelData[1]
                onClicked: {
                    if (Array.isArray(win.value(sizes.target))) {
                        win.set(sizes.target, modelData)
                    } else {
                        win.set(sizes.target + ".width", modelData[0])
                        win.set(sizes.target + ".height", modelData[1])
                    }
                }
            }
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
                    text: qsTr("Up")
                    enabled: row.index > 0
                    onClicked: win.listMove(fileList.path, row.index, -1)
                }
                Button {
                    visible: fileList.ordered
                    text: qsTr("Down")
                    onClicked: win.listMove(fileList.path, row.index, 1)
                }
                Button {
                    text: qsTr("Remove")
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

    // A search match's place, marked a moment after it is shown.
    Rectangle {
        id: mark
        z: 100
        opacity: 0
        color: "transparent"
        border.width: 2
        border.color: markColors.highlight
        radius: 3
        SystemPalette {
            id: markColors
        }
        NumberAnimation on opacity {
            id: markFade
            running: false
            from: 1
            to: 0
            duration: 1800
            easing.type: Easing.InQuad
        }
    }

    // Context Menu Items (M14): the top-level items of RVP's context menu,
    // each shown or hidden (skins.menu-hidden).
    Dialog {
        id: menuItems
        title: qsTr("Context Menu Items")
        modal: true
        anchors.centerIn: parent
        width: Math.min(win.width - 40, 420)
        height: Math.min(win.height - 40, 560)
        standardButtons: Dialog.Close
        property var labels: []
        function begin() {
            labels = JSON.parse(win.player.menu_json("context-all"))
                .filter(item => !item.separator).map(item => item.label)
            open()
        }
        ListView {
            anchors.fill: parent
            clip: true
            model: menuItems.labels
            delegate: CheckBox {
                required property string modelData
                text: modelData.replace(/&/g, "&&")
                checked: (win.value("skins.menu-hidden") || []).indexOf(modelData) < 0
                onToggled: {
                    const hidden = (win.value("skins.menu-hidden") || []).filter(label => label !== modelData)
                    if (!checked)
                        hidden.push(modelData)
                    win.set("skins.menu-hidden", hidden)
                }
            }
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
    // Fills the colour setting `target` as `#AARRGGBB`.
    Dialogs.ColorDialog {
        id: colorChooser
        property string target
        title: qsTr("Subtitle Colour")
        options: Dialogs.ColorDialog.ShowAlphaChannel
        onAccepted: {
            const hex = v => ("0" + Math.round(v * 255).toString(16)).slice(-2).toUpperCase()
            const c = selectedColor
            win.set(target, "#" + hex(c.a) + hex(c.r) + hex(c.g) + hex(c.b))
        }
    }
    // The OSD Messages page's fonts: `target` is the setting.
    Dialogs.FontDialog {
        id: osdFontChooser
        property string target
        title: qsTr("OSD Font")
        onAccepted: win.set(target, selectedFont.family)
    }
    function chooseOsdFont(path) {
        const family = win.value(path)
        osdFontChooser.target = path
        osdFontChooser.currentFont = Qt.font({ family: family ? family : Qt.application.font.family, pixelSize: 20 })
        osdFontChooser.open()
    }
    Dialogs.FontDialog {
        id: subtitleFontChooser
        title: qsTr("Subtitle Font")
        onAccepted: win.set("subtitle-font", selectedFont.family)
    }
    Dialogs.FileDialog {
        id: skinChooser
        title: qsTr("PotPlayer skin")
        nameFilters: [qsTr("PotPlayer skins (*.dsf)"), qsTr("All files (*)")]
        onAccepted: win.set("skin", Tools.local_file(selectedFile))
    }
    Dialogs.FileDialog {
        id: importChooser
        title: qsTr("Import Settings")
        nameFilters: [qsTr("Settings (*.json)"), qsTr("All files (*)")]
        onAccepted: win.report(win.player.import_settings(Tools.local_file(selectedFile)), qsTr("Imported."))
    }
    Dialogs.FileDialog {
        id: exportChooser
        title: qsTr("Export Settings")
        fileMode: Dialogs.FileDialog.SaveFile
        defaultSuffix: "json"
        nameFilters: [qsTr("Settings (*.json)")]
        onAccepted: win.report(win.player.export_settings(Tools.local_file(selectedFile)), qsTr("Exported."))
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 8

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 10

            ColumnLayout {
                Layout.preferredWidth: 200
                Layout.maximumWidth: 200
                Layout.fillHeight: true
                spacing: 4

                TextField {
                    id: searchField
                    objectName: "settingsSearch"
                    Layout.fillWidth: true
                    placeholderText: qsTr("Find an option")
                    selectByMouse: true
                    onTextChanged: win.search(text)
                    Keys.onEscapePressed: text = ""
                }

                // The matches while a search runs: the page and the option.
                ListView {
                    id: foundList
                    objectName: "settingsFound"
                    visible: win.searchText !== ""
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    model: win.found
                    clip: true
                    ScrollBar.vertical: ScrollBar {}
                    delegate: ItemDelegate {
                        required property var modelData
                        width: foundList.width
                        contentItem: ColumnLayout {
                            spacing: 0
                            Label {
                                Layout.fillWidth: true
                                text: win.pages[modelData.page].title
                                opacity: 0.6
                                font.pointSize: Math.max(7, Qt.application.font.pointSize - 1)
                                elide: Text.ElideRight
                            }
                            Label {
                                Layout.fillWidth: true
                                text: modelData.text
                                font.bold: modelData.heading
                                wrapMode: Text.WordWrap
                                maximumLineCount: 3
                                elide: Text.ElideRight
                            }
                        }
                        onClicked: win.openFound(modelData)
                    }
                    Label {
                        anchors.centerIn: parent
                        visible: win.found.length === 0
                        text: qsTr("Nothing found")
                        opacity: 0.6
                    }
                }

                // The tree: a group's row shows its own page (or its first
                // one) and opens it; the sign beside it opens or closes it.
                ListView {
                    id: pageTree
                    visible: win.searchText === ""
                    Layout.fillWidth: true
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
                        Heading { text: qsTr("Resume") }
                        Check { path: "remember-positions"; text: qsTr("Remember video playback positions") }
                        Check { path: "playback.remember-audio-positions"; text: qsTr("Remember audio playback positions") }
                        Check { path: "reopen-last-file"; text: qsTr("Reopen the last file, paused, when RVP starts without files") }
                        Heading { text: qsTr("Playback") }
                        Choice {
                            label: qsTr("At the end of a file")
                            path: "repeat"
                            values: ["off", "album", "item"]
                            texts: ({ "off": qsTr("Play the next one"), "album": qsTr("Repeat the playlist"),
                                      "item": qsTr("Repeat the file") })
                        }
                        Check { path: "playback.shuffle"; text: qsTr("Shuffle playback") }
                        Choice {
                            label: qsTr("When playback failed")
                            path: "playback.on-failure"
                            values: ["next", "stop"]
                            texts: ({ "next": qsTr("Play the next one"), "stop": qsTr("Stop") })
                        }
                        Amount { label: qsTr("Speed step"); path: "playback.speed-step"; values: [0.01, 0.02, 0.05, 0.1, 0.2, 0.25, 0.5]; format: (v) => v + "x" }
                        Check { path: "playback.keep-speed"; text: qsTr("Keep playback speed changes for the next file") }
                        Check { path: "playback.external-audio"; text: qsTr("Auto load external audio files") }
                        FileList { path: "playback.audio-folders"; ordered: true }
                        Button {
                            text: qsTr("Add a folder for external audio…")
                            onClicked: {
                                folderListChooser.target = "playback.audio-folders"
                                folderListChooser.open()
                            }
                        }
                        Choice {
                            visible: win.view.windows === true
                            label: qsTr("Process priority (at the next start)")
                            path: "playback.priority"
                            values: ["idle", "belownormal", "normal", "abovenormal", "high"]
                            texts: ({ "idle": qsTr("Idle"), "belownormal": qsTr("Below normal"), "normal": qsTr("Normal"),
                                      "abovenormal": qsTr("Above normal"), "high": qsTr("High") })
                        }
                        Check { path: "playback.folder-neighbours"; text: qsTr("Load previous/next files in the play folder") }
                        Check { path: "playback.auto-rotate"; text: qsTr("Auto rotate files based on metadata") }
                        Check { path: "playback.retitle"; text: qsTr("Auto retitle items based on metadata") }
                        Check { path: "playback.close-at-end"; text: qsTr("Close playback when the playlist ends") }
                        Check { path: "playback.recycle-played"; text: qsTr("Offer played files to the Recycle Bin") }
                        Heading { text: qsTr("Seek bar") }
                        Check { path: "playback.seek-tooltip"; text: qsTr("Show time info on the tooltip") }
                        Check { path: "playback.seek-preview"; text: qsTr("Show thumbnail previews") }
                        Check { path: "playback.preview-title"; text: qsTr("Include the title of the bookmark or chapter") }
                        Check { path: "playback.preview-time"; text: qsTr("Include the time") }
                        Check { path: "playback.seek-markers"; text: qsTr("Show markers of bookmarks, chapters and A-B points") }
                        Check { path: "playback.pause-while-seeking"; text: qsTr("Pause when seeking with the mouse") }
                        Heading { text: qsTr("When playback finished") }
                        Choice {
                            label: qsTr("The action")
                            path: "playback.finished.action"
                            values: ["shutdown", "sleep", "hibernate"]
                            texts: ({ "shutdown": qsTr("Shut down"), "sleep": qsTr("Sleep"), "hibernate": qsTr("Hibernate") })
                        }
                        Amount { label: qsTr("Notice before it (cancellable)"); path: "playback.finished.countdown"; from: 5; to: 300; step: 5; unit: "s" }
                        Check { path: "playback.finished.remember"; text: qsTr("Remember the playback finished action") }
                        Heading { text: qsTr("Window") }
                        Check { path: "fit-window-to-video"; text: qsTr("Fit the window to a newly opened video") }
                        Check { path: "playback.center-window"; text: qsTr("Start playback in a centred window") }
                        Check { path: "keep-aspect-window"; text: qsTr("Keep the video's shape while the window is resized") }
                        Check { path: "prevent-screen-saver"; text: qsTr("Prevent screen saver during playback") }
                        Check { path: "playback.hide-pointer"; text: qsTr("Hide the mouse pointer over the playing video") }
                        Choice {
                            label: qsTr("Stay on top")
                            path: "on-top"
                            values: ["never", "always", "while-playing", "fullscreen-playing"]
                            texts: ({ "never": qsTr("Never"), "always": qsTr("Always"), "while-playing": qsTr("While playing"),
                                      "fullscreen-playing": qsTr("While playing in fullscreen") })
                        }
                        Heading { text: qsTr("Opening") }
                        Choice {
                            label: qsTr("Default open action")
                            path: "open.default-open"
                            values: ["files", "url", "folder"]
                            texts: ({ "files": qsTr("Open File(s)"), "url": qsTr("Open URL"), "folder": qsTr("Open Folder") })
                        }
                        Choice {
                            label: qsTr("When files are opened")
                            path: "open.when-opened"
                            values: ["selected", "similar", "all"]
                            texts: ({ "selected": qsTr("Add only the files opened"), "similar": qsTr("Add similar files too"),
                                      "all": qsTr("Add every file of the folder") })
                        }
                        Check { path: "open.recent"; text: qsTr("Store and show recent items") }
                        Check { path: "open.ytdl"; text: qsTr("Open streaming sites with yt-dlp") }
                        Field { label: qsTr("yt-dlp program"); path: "open.ytdl-path"; placeholder: qsTr("yt-dlp") }
                        Field { label: qsTr("yt-dlp format"); path: "open.ytdl-format"; placeholder: qsTr("bestvideo+bestaudio/best") }
                    }

                    Page {
                        Heading { text: qsTr("Time jump (the arrow keys)") }
                        Amount { label: qsTr("Left / Right"); path: "seek-seconds.0"; values: [0.5, 1, 2, 3, 5, 10, 15, 20, 30, 45, 60, 90, 120, 180, 300, 600, 900, 1800, 3600]; format: win.seconds }
                        Amount { label: qsTr("Ctrl+Left / Right"); path: "seek-seconds.1"; values: [0.5, 1, 2, 3, 5, 10, 15, 20, 30, 45, 60, 90, 120, 180, 300, 600, 900, 1800, 3600]; format: win.seconds }
                        Amount { label: qsTr("Shift+Left / Right"); path: "seek-seconds.2"; values: [0.5, 1, 2, 3, 5, 10, 15, 20, 30, 45, 60, 90, 120, 180, 300, 600, 900, 1800, 3600]; format: win.seconds }
                        Amount { label: qsTr("Ctrl+Alt+Left / Right"); path: "seek-seconds.3"; values: [0.5, 1, 2, 3, 5, 10, 15, 20, 30, 45, 60, 90, 120, 180, 300, 600, 900, 1800, 3600]; format: win.seconds }
                        Check { path: "keyframe-seeks"; text: qsTr("Jump to keyframes (faster, less exact)") }
                    }

                    Page {
                        Heading { text: qsTr("Skip") }
                        Check { path: "playback.skip.enabled"; text: qsTr("Enable the skip feature") }
                        Check { path: "playback.skip.intro"; text: qsTr("Skip the intro") }
                        Amount { label: qsTr("Intro length"); path: "playback.skip.intro-seconds"; from: 1; to: 600; step: 1; format: win.seconds }
                        Check { path: "playback.skip.ending"; text: qsTr("Skip the ending") }
                        Amount { label: qsTr("Ending length"); path: "playback.skip.ending-seconds"; from: 1; to: 600; step: 1; format: win.seconds }
                        Check { path: "playback.skip.chapters"; text: qsTr("Skip chapters") }
                        Field { label: qsTr("Chapters to skip"); path: "playback.skip.chapter-names"; placeholder: qsTr("Opening;Ending;3") }
                        Note { text: qsTr("Titles (a part, any case) or numbers, separated by semicolons.") }
                    }

                    Page {
                        Heading { text: qsTr("Section repeat (A-B)") }
                        Amount { label: qsTr("Repeats (0: endless)"); path: "playback.ab.loops"; from: 0; to: 100; step: 1 }
                        Amount { label: qsTr("Fine move"); path: "playback.ab.step"; values: [0.01, 0.05, 0.1, 0.2, 0.5, 1]; format: win.seconds }
                        Amount { label: qsTr("Repeat behind the current point"); path: "playback.ab.behind"; from: 1; to: 60; step: 1; format: win.seconds }
                        Check { path: "playback.ab.remember"; text: qsTr("Remember the A-B repeat lists") }
                        Check { path: "playback.ab.prevent-backward"; text: qsTr("Prevent backward navigation from point A") }
                        Check { path: "playback.ab.toggle-current"; text: qsTr("Repeat-current commands also turn the repeat off") }
                        Heading { text: qsTr("Bookmarks") }
                        Check { path: "playback.bookmarks.prompt"; text: qsTr("Prompt before adding a bookmark") }
                        Check { path: "playback.bookmarks.in-menu"; text: qsTr("Show the bookmark list in the Chapter/Bookmark menu") }
                        Check { path: "playback.bookmarks.auto-delete"; text: qsTr("Delete a bookmark once it is chosen") }
                    }

                    Page {
                        Heading { text: qsTr("Scene browser") }
                        Amount { label: qsTr("Scene width"); path: "scenes.size"; values: [96, 128, 160, 192, 224, 256, 320, 400, 480]; unit: "px" }
                        Amount { label: qsTr("Time interval (0: auto)"); path: "scenes.interval"; values: [0, 5, 10, 15, 30, 60, 120, 300, 600]; format: (v) => v === 0 ? "Auto" : win.seconds(v) }
                        Amount { label: qsTr("Opacity"); path: "scenes.opacity"; from: 20; to: 100; step: 5; unit: "%" }
                        Check { path: "scenes.close-on-change"; text: qsTr("Close on scene changes") }
                        Check { path: "scenes.fall-back-to-time"; text: qsTr("Time-based scenes when there is no marker or subtitle") }
                    }

                    Page {
                        Heading { text: qsTr("PotPlayer's playlists") }
                        Field {
                            label: qsTr("Playlist folder")
                            path: "potplayer-playlists"
                            optional: true
                            placeholder: win.view.windows ? qsTr("PotPlayer's own folder") : qsTr("not set")
                            Button {
                                text: qsTr("Browse…")
                                onClicked: {
                                    folderChooser.target = "potplayer-playlists"
                                    folderChooser.open()
                                }
                            }
                        }
                        RowLayout {
                            Button {
                                text: qsTr("Import Playlists Now")
                                onClicked: {
                                    win.player.run_action("import-playlists")
                                    win.notice = "Imported (see the message in the player window)."
                                }
                            }
                            Note { text: qsTr("Reads PotPlayer's .dpl files; PotPlayer's files are not changed.") }
                        }
                        Heading {
                            text: qsTr("Drives in PotPlayer's playlists")
                            visible: !win.view.windows
                        }
                        Note {
                            visible: !win.view.windows
                            text: qsTr("Windows drive letters in imported playlists and the Linux folders they stand for; other drives become /mnt/<letter>.")
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
                                    text: qsTr("Remove")
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
                                placeholderText: qsTr("V:")
                                Layout.preferredWidth: 60
                            }
                            TextField {
                                id: newFolder
                                placeholderText: qsTr("/mnt/media")
                                Layout.fillWidth: true
                            }
                            Button {
                                text: qsTr("Add")
                                onClicked: {
                                    const map = Object.assign({}, win.s["drive-map"])
                                    map[newDrive.text.toUpperCase()] = newFolder.text
                                    win.set("drive-map", map)
                                    newDrive.text = ""
                                    newFolder.text = ""
                                }
                            }
                        }
                        Heading { text: qsTr("Playlist") }
                        Choice {
                            label: qsTr("View items as")
                            path: "playlist.thumbnail-rows"
                            values: [0, 2, 3, 4, 5, 6, 7]
                            texts: ({ 0: "List", 2: "Thumbnails (2 rows) + details", 3: "Thumbnails (3 rows) + details",
                                      4: "Thumbnails (4 rows) + details", 5: "Thumbnails (5 rows) + details",
                                      6: "Thumbnails (6 rows) + details", 7: "Thumbnails (7 rows) + details" })
                        }
                        Amount { label: qsTr("Thumbnail position"); path: "playlist.thumbnail-position"; from: 0; to: 95; step: 5; unit: "%" }
                        Check { path: "playlist.wide-thumbnails"; text: qsTr("Use a 16:9 thumbnail ratio") }
                        Check { path: "playlist.detect-durations"; text: qsTr("Detect durations after adding files to the list") }
                        Check { path: "playlist.remember-default"; text: qsTr("Remember the Default album and recent URLs") }
                        Check { path: "playlist.instant-albums"; text: qsTr("Start albums chosen in menus at once") }
                        Check { path: "playlist.append-on-open"; text: qsTr("Append opened files rather than clear the album") }
                        Check { path: "playlist.no-duplicates"; text: qsTr("Do not add duplicate files") }
                        Check { path: "playlist.sort-dropped"; text: qsTr("Sort dropped files before adding them") }
                        Check { path: "playlist.focus-main"; text: qsTr("Give the window the keyboard when an entry plays") }
                    }

                    Page {
                        Heading { text: qsTr("Screen capture") }
                        Choice {
                            label: qsTr("Method")
                            path: "devices.capture"
                            values: ["gdigrab", "ddagrab"]
                            texts: ({ "gdigrab": qsTr("GDI (any desktop)"), "ddagrab": qsTr("Desktop Duplication (Windows 8 and later)") })
                        }
                        Amount { label: qsTr("Frames a second"); path: "devices.capture-fps"; values: [5, 10, 15, 24, 25, 30, 50, 60]; unit: "fps" }
                        Check { path: "devices.capture-pointer"; text: qsTr("Show the mouse pointer") }
                        Heading { text: qsTr("Webcam and other devices") }
                        Field { label: qsTr("Device opened without asking"); path: "devices.webcam"; placeholder: qsTr("ask every time") }
                        Heading { text: qsTr("Discs") }
                        Field { label: qsTr("DVD drive or folder"); path: "devices.dvd"; placeholder: qsTr("ask every time") }
                        Field { label: qsTr("Blu-ray drive or folder"); path: "devices.bluray"; placeholder: qsTr("ask every time") }
                        Field { label: qsTr("CD drive"); path: "devices.cd"; placeholder: qsTr("libmpv's default") }
                        Check { path: "devices.dvd-menus"; text: qsTr("Play whole DVDs through their menus") }
                        Field { label: qsTr("Menu language (two letters)"); path: "devices.dvd-menu-language"; placeholder: qsTr("the disc's default") }
                        Check { path: "devices.dvd-skip-opening"; text: qsTr("Go straight to a DVD's menu, past its opening") }
                        Check { path: "devices.bluray-menus"; text: qsTr("Play whole Blu-rays through their menus") }
                        Note { text: qsTr("Encrypted discs need the user's own libdvdcss-2.dll (DVD) or libaacs.dll and its keys (Blu-ray) next to rvp.exe; RVP includes neither.") }
                    }

                    Page {
                        Heading { text: qsTr("Audio Output") }
                        Choice {
                            label: qsTr("Audio Renderer")
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
                            texts: ({ "same": qsTr("As the output device takes it"), "s16": qsTr("16-bit"),
                                      "s24": qsTr("24-bit"), "s32": qsTr("32-bit"), "float": qsTr("32-bit (floating point)"),
                                      "s16-dither": qsTr("16-bit (dithering)") })
                        }
                        Choice {
                            label: qsTr("Sample rate")
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
                            texts: ({ "high": qsTr("High quality"), "normal": qsTr("Normal quality (Recommended)"),
                                      "low": qsTr("Low quality") })
                        }
                        Choice {
                            label: qsTr("Change the sample rate")
                            path: "audio.resample-when"
                            values: ["always", "below", "above"]
                            texts: ({ "always": qsTr("Always"), "below": qsTr("When sample rate is smaller than"),
                                      "above": qsTr("When sample rate is larger than") })
                        }
                        Amount { label: qsTr("That sample rate"); path: "audio.resample-limit"; unit: "Hz"; values: [8000, 11025, 12000, 16000, 22050, 24000, 32000, 44100, 48000, 96000, 192000] }
                        Choice {
                            label: qsTr("Output")
                            path: "audio.output-mode"
                            values: ["pcm", "passthrough", "ac3-encode"]
                            texts: ({ "pcm": qsTr("PCM (Default) (Recommended)"), "passthrough": qsTr("Pass-through"),
                                      "ac3-encode": qsTr("Pass-through after AC3 re-encoding") })
                        }
                        Amount { label: qsTr("AC3 re-encoding bitrate"); path: "audio.ac3-bitrate"; unit: "kbps"; values: [64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384, 448, 512, 576, 640] }
                        Choice {
                            label: qsTr("Pass-through when the source has")
                            path: "audio.passthrough-min-channels"
                            values: [null, 2, 3, 4, 5, 6, 7, 8]
                            texts: ({ "null": qsTr("Any number of channels"), 2: "2 channels or more",
                                      3: "3 channels or more", 4: "4 channels or more", 5: "5 channels or more",
                                      6: "6 channels or more", 7: "7 channels or more", 8: "8 channels" })
                        }
                        Check { path: "audio.exclusive"; text: qsTr("Exclusive mode (the device plays only RVP)") }
                        Heading { text: qsTr("Mixer Settings") }
                        Check { path: "audio.mixer.normalize"; text: qsTr("Normalize matrix (scale the mix down so it cannot clip)") }
                        Check { path: "audio.mixer.expand-center"; text: qsTr("Expand stereo to center") }
                        Check { path: "audio.mixer.expand-surround"; text: qsTr("Expand stereo to surround") }
                        Heading { text: qsTr("Mixer Level") }
                        Amount { label: qsTr("Front"); path: "audio.mixer.front"; unit: "dB"; from: -30; to: 12; step: 0.5 }
                        Amount { label: qsTr("Center"); path: "audio.mixer.center"; unit: "dB"; from: -30; to: 12; step: 0.5 }
                        Amount { label: qsTr("Surround"); path: "audio.mixer.surround"; unit: "dB"; from: -30; to: 12; step: 0.5 }
                        Amount { label: qsTr("LFE"); path: "audio.mixer.lfe"; unit: "dB"; from: -30; to: 12; step: 0.5 }
                        Note {
                            text: qsTr("Stereo mixes 5.1 and 7.1 down as PotPlayer does (centre and LFE at -3 dB, ")
                                  + "surround channels at their level) before the equalizer and the normalizer."
                        }
                        Heading { text: qsTr("Headphone & Dolby Decoder Settings") }
                        Amount { label: qsTr("Distance (virtual speakers' room)"); path: "audio.virtualizer.distance"; from: 0; to: 100 }
                        Amount { label: qsTr("Delay (decoded surround)"); path: "audio.virtualizer.delay"; unit: "ms"; from: 0; to: 100 }
                        Heading { text: qsTr("LFE Redirection") }
                        Check { path: "audio.lfe-redirect.enabled"; text: qsTr("Enable") }
                        Check { path: "audio.lfe-redirect.remove-low"; text: qsTr("Remove low frequencies from the other speakers") }
                        Amount { label: qsTr("Low frequency"); path: "audio.lfe-redirect.frequency"; unit: "Hz"; from: 20; to: 300; step: 5 }
                        Amount { label: qsTr("Gain"); path: "audio.lfe-redirect.gain"; unit: "dB"; from: -20; to: 20; step: 0.5 }
                        Heading { text: qsTr("Second Device") }
                        Amount { label: qsTr("Sync of the 2nd device"); path: "audio.second-delay"; unit: "ms"; from: -2000; to: 2000; step: 10 }
                    }

                    Page {
                        Heading { text: qsTr("Volume control setting") }
                        Choice {
                            label: qsTr("Method")
                            path: "audio.volume-method"
                            values: ["player", "wave", "master"]
                            texts: ({ "player": qsTr("Default volume control (Recommended)"),
                                      "wave": qsTr("System Wave volume control"),
                                      "master": qsTr("System Master volume control") })
                        }
                        Check { path: "audio.wave-to-max"; text: qsTr("Adjust wave volume to max") }
                        Check { path: "audio.accelerate"; text: qsTr("Accelerate control speed") }
                        Amount { label: qsTr("Volume step (Up, Down, wheel)"); path: "volume-step"; unit: "%"; from: 1; to: 20 }
                        Amount { label: qsTr("Max amplification (above 100 amplifies)"); path: "volume-max"; unit: "%"
                                 values: [100, 110, 120, 130, 150, 175, 200, 250, 300, 400, 500, 700, 1000] }
                        CheckBox {
                            text: qsTr("Startup volume")
                            checked: win.value("audio.startup-volume") !== null
                                     && win.value("audio.startup-volume") !== undefined
                            onToggled: win.set("audio.startup-volume", checked ? 50 : null)
                        }
                        Amount { label: qsTr("Volume at every start"); path: "audio.startup-volume"; unit: "%"; from: 0; to: 200
                                 enabled: win.value("audio.startup-volume") !== null }
                        Heading { text: qsTr("Individual volume control") }
                        Check { path: "audio.channel-volumes.enabled"; text: qsTr("Enable individual volume control") }
                        Amount { label: qsTr("Master volume"); path: "audio.channel-volumes.master"; unit: "dB"; from: -40; to: 12; step: 0.5; live: true }
                        Amount { label: qsTr("Left channel"); path: "audio.channel-volumes.left"; unit: "dB"; from: -40; to: 12; step: 0.5; live: true }
                        Amount { label: qsTr("Right channel"); path: "audio.channel-volumes.right"; unit: "dB"; from: -40; to: 12; step: 0.5; live: true }
                        Amount { label: qsTr("Center channel"); path: "audio.channel-volumes.center"; unit: "dB"; from: -40; to: 12; step: 0.5; live: true }
                        Amount { label: qsTr("BackL channel"); path: "audio.channel-volumes.back-left"; unit: "dB"; from: -40; to: 12; step: 0.5; live: true }
                        Amount { label: qsTr("BackR channel"); path: "audio.channel-volumes.back-right"; unit: "dB"; from: -40; to: 12; step: 0.5; live: true }
                        Amount { label: qsTr("SideL channel"); path: "audio.channel-volumes.side-left"; unit: "dB"; from: -40; to: 12; step: 0.5; live: true }
                        Amount { label: qsTr("SideR channel"); path: "audio.channel-volumes.side-right"; unit: "dB"; from: -40; to: 12; step: 0.5; live: true }
                        Amount { label: qsTr("LFE channel"); path: "audio.channel-volumes.lfe"; unit: "dB"; from: -40; to: 12; step: 0.5; live: true }
                        Heading { text: qsTr("Pre-amplify") }
                        Check { path: "audio.preamp.enabled"; text: qsTr("Pre-amplify AC3/DTS Vol.") }
                        Amount { label: qsTr("PreAmp"); path: "audio.preamp.gain"; unit: "dB"; from: 0; to: 20; step: 0.5 }
                    }

                    Page {
                        Heading { text: qsTr("Equalizer") }
                        Check { path: "equalizer.enabled"; text: qsTr("Enable (Shift+E), before the normalizer") }
                        RowLayout {
                            Label { text: qsTr("Preset"); Layout.preferredWidth: 240 }
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
                                text: qsTr("0 dB")
                                onClicked: win.set("equalizer.bands", [0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
                            }
                        }
                        Check { path: "equalizer.superequalizer"; text: qsTr("SuperEQ (Naoki Shibata's equalizer, 18 bands)") }
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
                        Note { text: qsTr("The bands are also on the control panel (F7).") }
                    }

                    Page {
                        Heading { text: qsTr("Normalizer (Auto Volume Control)") }
                        Check { path: "normalizer.enabled"; text: qsTr("Enable (Shift+N)") }
                        Check { path: "normalizer.video-only"; text: qsTr("Operate only with video playback") }
                        Note { text: qsTr("Method: TomSteady 32-bit floating point arithmetic, PotPlayer's recommended one.") }
                        Check { path: "normalizer.boost"; text: qsTr("Boost audio (raise quiet passages)") }
                        Amount { label: qsTr("Level"); path: "normalizer.level"; unit: "%"; from: 10; to: 100 }
                        Amount { label: qsTr("Release time"); path: "normalizer.release"; from: 5; to: 10 }
                        Heading { text: qsTr("Freeverb") }
                        Check { path: "audio.freeverb.enabled"; text: qsTr("Enable (Shift+R)") }
                        Note { text: qsTr("It operates only in 2 channels.") }
                        Amount { label: qsTr("Room size"); path: "audio.freeverb.room-size"; from: 0; to: 1; step: 0.01 }
                        Amount { label: qsTr("Damping"); path: "audio.freeverb.damping"; from: 0; to: 1; step: 0.01 }
                        Amount { label: qsTr("Wet level"); path: "audio.freeverb.wet"; from: 0; to: 1; step: 0.001 }
                        Amount { label: qsTr("Dry level"); path: "audio.freeverb.dry"; from: 0; to: 1; step: 0.01 }
                        Amount { label: qsTr("Width"); path: "audio.freeverb.width"; from: 0; to: 1; step: 0.01 }
                    }

                    Page {
                        Heading { text: qsTr("Crystallization") }
                        Check { path: "audio.crystality.enabled"; text: qsTr("Enable (Shift+C)") }
                        Amount { label: qsTr("Bandwidth extender"); path: "audio.crystality.bandwidth-extender"; from: 0; to: 100 }
                        Amount { label: qsTr("Extender's HPF"); path: "audio.crystality.extender-hpf"; from: 1; to: 3 }
                        Amount { label: qsTr("Harmonic booster"); path: "audio.crystality.harmonic-booster"; from: 0; to: 100 }
                        Amount { label: qsTr("Echo volume"); path: "audio.crystality.echo-volume"; from: 0; to: 100 }
                        Amount { label: qsTr("Echo feedback"); path: "audio.crystality.echo-feedback"; from: 0; to: 100 }
                        Amount { label: qsTr("Extra stereo"); path: "audio.crystality.extra-stereo"; from: 0; to: 100 }
                        Heading { text: qsTr("Noise Reduction") }
                        Check { path: "audio.noise-reduction.enabled"; text: qsTr("Enable (Shift+D)") }
                        Heading { text: qsTr("Crossfeed") }
                        Check { path: "audio.crossfeed.enabled"; text: qsTr("Enable") }
                        Amount { label: qsTr("Remove low frequencies"); path: "audio.crossfeed.cut"; unit: "Hz"; from: 300; to: 2000; step: 50 }
                        Amount { label: qsTr("Gain"); path: "audio.crossfeed.feed"; unit: "dB"; from: 1; to: 15; step: 0.5 }
                        Heading { text: qsTr("Voice and Channels") }
                        Check { path: "audio.voice-removal"; text: qsTr("Voice Removal (Shift+V)") }
                        Check { path: "audio.voice-emphasis"; text: qsTr("Voice Emphasis (Shift+A)") }
                        Check { path: "audio.swap-channels"; text: qsTr("Swap Stereo Channels (Shift+T)") }
                    }

                    Page {
                        Heading { text: qsTr("Language/Stream Selection") }
                        Choice {
                            label: qsTr("Selection method")
                            path: "audio.selection"
                            values: ["languages", "stream", "quality", "default"]
                            texts: ({ "languages": qsTr("Prefer specified languages (Recommended)"),
                                      "stream": qsTr("Prefer specified stream"),
                                      "quality": qsTr("Prefer highest quality ones"),
                                      "default": qsTr("Use default selection") })
                        }
                        Field { label: qsTr("Preferred languages"); path: "audio.languages"; placeholder: qsTr("rus,eng") }
                        Field { label: qsTr("Turn off subtitles with"); path: "audio.subtitles-off"; placeholder: qsTr("rus") }
                        Amount { label: qsTr("Preferred stream (0: none)"); path: "audio.preferred-stream"; from: 0; to: 9 }
                        Check { path: "audio.remember-tracks"; text: qsTr("Remember audio track selections") }
                        Heading { text: qsTr("Synchronization") }
                        Choice {
                            label: qsTr("Audio sync control")
                            path: "audio.sync-keep"
                            values: ["never", "runtime", "always"]
                            texts: ({ "never": qsTr("Do not keep audio sync changes"),
                                      "runtime": qsTr("Keep audio sync changes over whole runtime"),
                                      "always": qsTr("Always keep audio sync changes") })
                        }
                        Amount { label: qsTr("Audio sync step (Shift+, and Shift+.)"); path: "audio-delay-step"; values: [0.001, 0.005, 0.01, 0.02, 0.025, 0.05, 0.1, 0.2, 0.25, 0.5, 1, 2, 5, 10]; format: win.seconds }
                        Amount { label: qsTr("Default audio sync"); path: "audio.default-delay"; unit: "ms"; from: -2000; to: 2000; step: 10 }
                        Heading { text: qsTr("Speed") }
                        Choice {
                            label: qsTr("Time stretching")
                            path: "audio.stretch"
                            values: ["high-quality", "default-quality", "pitch"]
                            texts: ({ "high-quality": qsTr("High quality audio time stretching (Recommended)"),
                                      "default-quality": qsTr("Default quality audio time stretching"),
                                      "pitch": qsTr("Audio pitch shifting") })
                        }
                        Heading { text: qsTr("Audio Recording") }
                        Field { label: qsTr("Storage folder"); path: "audio.recording.folder"; optional: true; placeholder: qsTr("Music/RVP") }
                        Check { path: "audio.recording.in-media-folder"; text: qsTr("Store recorded audios into play folders (if available)") }
                    }

                    Page {
                        Heading { text: qsTr("Decoding") }
                        Choice {
                            label: qsTr("Hardware decoding")
                            path: "hwdec"
                            values: win.withCurrent(win.view.choices.hwdec, win.s.hwdec)
                            texts: ({ "no": qsTr("Off (software)"), "auto-copy": qsTr("auto-copy (SVP needs copy-back)") })
                        }
                        Heading { text: qsTr("Output") }
                        Choice {
                            label: qsTr("Video renderer (when RVP starts)")
                            path: "render.renderer"
                            values: win.view.choices.renderer || []
                            texts: ({ "opengl": qsTr("OpenGL in the window (mpv's gpu renderer)"),
                                      "direct3d11": qsTr("Direct3D 11 window (mpv's gpu-next, HDR output)"),
                                      "vulkan": qsTr("Vulkan window (mpv's gpu-next, HDR output, NVIDIA Optical Flow smoothing)") })
                        }
                        Note {
                            visible: Qt.platform.os !== "windows"
                            text: qsTr("The Vulkan window is an X11 window: on a Wayland desktop RVP then starts through XWayland.")
                        }
                        Check {
                            visible: Qt.platform.os === "windows"
                            path: "render.hdr-output"
                            text: qsTr("Send HDR video to an HDR display as HDR (Direct3D 11)")
                        }
                        Check {
                            visible: Qt.platform.os === "windows"
                            path: "render.ten-bit"
                            text: qsTr("10-bit output (Direct3D 11, when RVP starts)")
                        }
                        Choice {
                            visible: Qt.platform.os === "windows"
                            label: qsTr("GPU Super Resolution (Direct3D 11)")
                            path: "render.super-resolution"
                            values: win.view.choices["super-resolution"] || []
                            texts: ({ "off": qsTr("Off"), "nvidia": qsTr("NVIDIA RTX Video Super Resolution"),
                                      "intel": qsTr("Intel Video Super Resolution") })
                        }
                        Check {
                            visible: Qt.platform.os === "windows"
                            path: "render.rtx-hdr"
                            text: qsTr("NVIDIA RTX Video HDR (Direct3D 11)")
                        }
                        Note {
                            visible: Qt.platform.os === "windows"
                            text: qsTr("Both run in the GPU's video processor on Direct3D 11 frames: hardware decoding without copy-back, so not with SVP or RVP's own video filters.")
                        }
                        Choice {
                            label: qsTr("Video sync")
                            path: "video-sync"
                            values: win.withCurrent(win.view.choices["video-sync"], win.s["video-sync"])
                            texts: ({ "audio": qsTr("audio (video follows the sound)"),
                                      "display-resample": qsTr("display-resample (smooth, resamples audio)") })
                        }
                        Choice {
                            label: qsTr("Upscaler")
                            path: "scaler"
                            values: win.withCurrent(win.view.choices.scaler, win.s.scaler)
                            texts: ({ "": qsTr("mpv's default") })
                        }
                        Choice {
                            label: qsTr("Downscaler")
                            path: "render.dscale"
                            values: win.view.choices["render-scaler"] || []
                            texts: ({ "": qsTr("mpv's default") })
                        }
                        Choice {
                            label: qsTr("Chroma scaler")
                            path: "render.cscale"
                            values: win.view.choices["render-scaler"] || []
                            texts: ({ "": qsTr("mpv's default") })
                        }
                        Heading { text: qsTr("GPU power") }
                        Amount {
                            label: qsTr("Power budget")
                            path: "render.power-budget"
                            from: 50
                            to: 100
                            step: 5
                            format: v => v >= 100 ? "Off" : v + " % of the card's rated power"
                        }
                        Note { text: win.gpuPowerText() }
                        Note {
                            text: qsTr("Above the budget RVP lightens its own work, a step every few seconds: EWA scalers become spline36, then GPU Super Resolution and RTX Video HDR, the pixel shaders and the VapourSynth script go off. Each comes back once the card stays 10 points below the budget for a minute. RVP never changes the card's own settings, and SVP and other programs are not limited. NVIDIA cards are read through NVML; AMD cards through Windows' adapter counter (Windows) and amdgpu (Linux).")
                        }
                        Heading { text: qsTr("HDR and colour") }
                        Check { path: "render.tone-mapping-on"; text: qsTr("Map HDR to the display (SMPTE ST 2084 correction)") }
                        Choice {
                            label: qsTr("Tone mapping")
                            path: "render.tone-mapping"
                            values: win.view.choices["tone-mapping"] || []
                            texts: ({ "auto": qsTr("auto (mpv's choice)") })
                        }
                        Check { path: "render.hdr-metadata"; text: qsTr("Use the file's mastering metadata (SMPTE ST 2086) instead of measured peaks") }
                        Check { path: "render.icc"; text: qsTr("Use the display's ICC profile (colour gamut correction)") }
                        Amount { label: qsTr("SDR monitor luminance"); path: "render.target-peak"; values: [0, 100, 120, 160, 200, 203, 250, 300, 400, 500, 600, 800, 1000]; format: v => v === 0 ? "mpv's choice" : v + " nits" }
                        Check { path: "render.limited-range"; text: qsTr("Use limited (16-235) colour range") }
                        Choice {
                            label: qsTr("YCbCr to RGB matrix")
                            path: "render.matrix"
                            values: win.view.choices["matrix"] || []
                            texts: ({ "auto": qsTr("From the file (auto)") })
                        }
                        Heading { text: qsTr("Deinterlacing") }
                        Choice {
                            label: qsTr("Deinterlace (Ctrl+I)")
                            path: "deinterlace"
                            values: ["auto", "on", "off"]
                            texts: ({ "auto": qsTr("Auto (interlaced frames only)"), "on": qsTr("Always"), "off": qsTr("Off") })
                        }
                        Choice {
                            label: qsTr("Software deinterlacer")
                            path: "render.deinterlacer"
                            values: win.view.choices["deinterlacer"] || []
                            texts: ({ "": qsTr("mpv's choice"), "bwdif": qsTr("bwdif (Bob Weaver)"), "yadif": qsTr("yadif"), "w3fdif": qsTr("w3fdif") })
                        }
                        Choice {
                            label: qsTr("Field order")
                            path: "render.field-order"
                            values: win.view.choices["field-order"] || []
                            texts: ({ "auto": qsTr("From the file"), "top": qsTr("Top field first"), "bottom": qsTr("Bottom field first") })
                        }
                        Heading { text: qsTr("Colour") }
                        Check { path: "color.enabled"; text: qsTr("Use the colour adjustments (Q switches them off and back)") }
                        Amount { label: qsTr("Brightness (W, E)"); path: "color.brightness"; from: -100; live: true }
                        Amount { label: qsTr("Contrast (R, T)"); path: "color.contrast"; from: -100; live: true }
                        Amount { label: qsTr("Saturation (Y, U)"); path: "color.saturation"; from: -100; live: true }
                        Amount { label: qsTr("Hue (I, O)"); path: "color.hue"; from: -100; live: true }
                        Amount { label: qsTr("Gamma"); path: "color.gamma"; from: -100; live: true }
                        Heading { text: qsTr("Effects") }
                        Effect { effect: "deblock"; text: qsTr("Deblock (Ctrl+H)") }
                        Effect { effect: "denoise"; text: qsTr("Denoise 3D (Ctrl+N)") }
                        Effect { effect: "soften"; text: qsTr("Soften (Ctrl+B)") }
                        Effect { effect: "sharpen"; text: qsTr("Sharpen (Ctrl+R)") }
                        Effect { effect: "flip-vertical"; text: qsTr("Flip vertical (Ctrl+P)") }
                        Effect { effect: "flip-horizontal"; text: qsTr("Flip horizontal (Ctrl+Z)") }
                        Effect { effect: "rotate"; text: qsTr("Rotation") }
                        Effect { effect: "auto-level"; text: qsTr("Auto level control") }
                        Effect { effect: "levels"; text: qsTr("Level control (Ctrl+L)") }
                        Effect { effect: "gradual-denoise"; text: qsTr("Gradual denoise") }
                        Effect { effect: "temporal-denoise"; text: qsTr("Temporary denoise (Ctrl+M)") }
                        Effect { effect: "offset"; text: qsTr("Luma and chroma offset") }
                        Effect { effect: "motion-blur"; text: qsTr("Motion blur") }
                        Effect { effect: "warp-sharp"; text: qsTr("WarpSharp (approximated: FFmpeg's contrast adaptive sharpening)") }
                        Check { path: "deband"; text: qsTr("Deband (mpv's renderer)") }
                        Note { text: qsTr("Effects run as RVP's own filters before SVP's, which they leave in place; while one is on, hardware decoding copies the frames back.") }
                        Heading { text: qsTr("Effect strengths") }
                        Amount { label: qsTr("Rotation"); path: "video-effects.rotate-angle"; from: -180; to: 270; step: 1; unit: "°" }
                        Amount { label: qsTr("Soften: radius"); path: "video-effects.soften.0"; from: 0.1; to: 5; step: 0.1 }
                        Amount { label: qsTr("Soften: luma"); path: "video-effects.soften.1"; from: 0; to: 1; step: 0.05 }
                        Amount { label: qsTr("Soften: chroma"); path: "video-effects.soften.2"; from: 0; to: 1; step: 0.05 }
                        Amount { label: qsTr("Sharpen: luma"); path: "video-effects.sharpen.0"; from: 0; to: 2; step: 0.05 }
                        Amount { label: qsTr("Sharpen: chroma"); path: "video-effects.sharpen.1"; from: 0; to: 2; step: 0.05 }
                        Amount { label: qsTr("Deblock"); path: "video-effects.deblock"; from: 1; to: 10 }
                        Amount { label: qsTr("Gradual denoise"); path: "video-effects.gradual"; from: 1; to: 10 }
                        Amount { label: qsTr("Denoise 3D: luma"); path: "video-effects.denoise.0"; from: 0; to: 20; step: 0.5 }
                        Amount { label: qsTr("Denoise 3D: chroma"); path: "video-effects.denoise.1"; from: 0; to: 20; step: 0.5 }
                        Amount { label: qsTr("Denoise 3D: time"); path: "video-effects.denoise.2"; from: 0; to: 20; step: 0.5 }
                        Check { path: "video-effects.denoise-quality"; text: qsTr("Denoise 3D in high quality (non-local means, slow)") }
                        Amount { label: qsTr("Temporal noise reducer: A"); path: "video-effects.temporal.0"; from: 0.005; to: 0.3; step: 0.005 }
                        Amount { label: qsTr("Temporal noise reducer: B"); path: "video-effects.temporal.1"; from: 0.005; to: 0.3; step: 0.005 }
                        Amount { label: qsTr("Temporal noise reducer: frames"); path: "video-effects.temporal.2"; from: 3; to: 29; step: 2 }
                        Amount { label: qsTr("WarpSharp: depth"); path: "video-effects.warpsharp"; from: 0; to: 1; step: 0.05 }
                        Amount { label: qsTr("Motion blur: frames"); path: "video-effects.motion-blur"; from: 2; to: 10 }
                        Amount { label: qsTr("Deband: threshold"); path: "video-effects.deband.0"; from: 1; to: 200 }
                        Amount { label: qsTr("Deband: radius"); path: "video-effects.deband.1"; from: 1; to: 64 }
                        Heading { text: qsTr("Levels and offsets") }
                        Check { path: "video-effects.levels.auto-full-range"; text: qsTr("Auto level control: full range (0-255)") }
                        Check { path: "video-effects.levels.full-range"; text: qsTr("Level control: full range output (0-255)") }
                        Check { path: "video-effects.levels.luma-only"; text: qsTr("Change only the luminance") }
                        Check { path: "video-effects.levels.auto-input"; text: qsTr("Input range: automatic (the frame's own)") }
                        Amount { label: qsTr("Input: from"); path: "video-effects.levels.input.0"; from: 0; to: 255 }
                        Amount { label: qsTr("Input: to"); path: "video-effects.levels.input.1"; from: 0; to: 255 }
                        Amount { label: qsTr("Output: from"); path: "video-effects.levels.output.0"; from: 0; to: 255 }
                        Amount { label: qsTr("Output: to"); path: "video-effects.levels.output.1"; from: 0; to: 255 }
                        Amount { label: qsTr("Gamma correction"); path: "video-effects.levels.gamma"; from: 0.1; to: 4; step: 0.05 }
                        Amount { label: qsTr("Luma offset X"); path: "video-effects.levels.luma-offset.0"; from: -32; to: 32; unit: "px" }
                        Amount { label: qsTr("Luma offset Y"); path: "video-effects.levels.luma-offset.1"; from: -32; to: 32; unit: "px" }
                        Amount { label: qsTr("Chroma offset X"); path: "video-effects.levels.chroma-offset.0"; from: -32; to: 32; unit: "px" }
                        Amount { label: qsTr("Chroma offset Y"); path: "video-effects.levels.chroma-offset.1"; from: -32; to: 32; unit: "px" }
                    }

                    Page {
                        Heading { text: qsTr("Aspect Ratio") }
                        Field { label: qsTr("Custom aspect ratio"); path: "picture.custom-aspect"; placeholder: "21:9" }
                        Choice {
                            label: qsTr("Correct the aspect ratio of the output device")
                            path: "picture.output-device"
                            values: ["monitor", "tv4x3", "tv16x9", "auto"]
                            texts: ({ "monitor": qsTr("Monitor output (square pixels)"), "tv4x3": qsTr("TV-OUT 4:3"),
                                      "tv16x9": qsTr("TV-OUT 16:9"), "auto": qsTr("Detect from the screen's size") })
                        }
                        Check { path: "picture.adjust-to-window"; text: qsTr("Adjust the aspect ratio to the window (not in fullscreen)") }
                        Heading { text: qsTr("Aspect ratio mode in fullscreen") }
                        Check { path: "picture.fullscreen-mode-limited"; text: qsTr("Use the chosen mode only while the margin around the video is small") }
                        Amount { label: qsTr("Margin below"); path: "picture.fullscreen-margin"; from: 0; to: 50; unit: "%" }
                        Heading { text: qsTr("Bottom margin for subtitles") }
                        Check { path: "picture.subtitle-margin"; text: qsTr("Provide a bottom margin for subtitles") }
                        Choice {
                            label: qsTr("As bottom margin")
                            path: "picture.subtitle-room"
                            values: ["one-line", "two-lines", "three-lines", "available", "available-always"]
                            texts: ({ "one-line": qsTr("Reserve 1 line"), "two-lines": qsTr("Reserve 2 lines"),
                                      "three-lines": qsTr("Reserve 3 lines"), "available": qsTr("Reserve the space below the video"),
                                      "available-always": qsTr("Reserve the space below the video, even without subtitles") })
                        }
                        Check { path: "picture.subtitle-margin-resize"; text: qsTr("Make room by shrinking the picture (otherwise the video moves up)") }
                        Check { path: "picture.subtitle-margin-fullscreen-only"; text: qsTr("Only in fullscreen") }
                        Note { text: qsTr("Text subtitles go into the room under the video. Keep Video Frame and the other aspect ratio modes are in the Aspect Ratio menu.") }
                    }

                    Page {
                        Heading { text: qsTr("Extend/Crop") }
                        Choice {
                            label: qsTr("Method")
                            path: "picture.extend"
                            values: ["none", "extend", "crop", "pixels"]
                            texts: ({ "none": qsTr("Do nothing"), "extend": qsTr("Extend to the aspect ratio"),
                                      "crop": qsTr("Crop to the aspect ratio"), "pixels": qsTr("Extend or crop to the size") })
                        }
                        Field { label: qsTr("Aspect ratio"); path: "picture.extend-ratio"; placeholder: "4:3" }
                        Amount { label: qsTr("Width"); path: "picture.extend-size.0"; from: 160; to: 7680; step: 16; unit: "px" }
                        Amount { label: qsTr("Height"); path: "picture.extend-size.1"; from: 120; to: 4320; step: 8; unit: "px" }
                        Amount { label: qsTr("Horizontal position"); path: "picture.position.0"; from: 0; to: 100; unit: "%"; live: true }
                        Amount { label: qsTr("Vertical position"); path: "picture.position.1"; from: 0; to: 100; unit: "%"; live: true }
                        Check { path: "picture.extend-first"; text: qsTr("Use video extension before resizing") }
                        Check { path: "picture.extend-scale-height"; text: qsTr("Scale only the frame's height (no bars, no cutting)") }
                        Heading { text: qsTr("Bottom margin") }
                        Amount { label: qsTr("Black below the picture"); path: "picture.bottom-margin"; values: [0, 5, 10, 15, 20, 25, 30]; unit: "%" }
                        Check { path: "picture.bottom-margin-subtitles-only"; text: qsTr("Only when subtitles exist") }
                        Note { text: qsTr("Extending and cropping run as RVP's own filters before SVP's (the frame changes, so SVP processes what shows); while they are on, hardware decoding copies the frames back. The positions move the picture inside the new frame (Video > Extend/Crop > Move).") }
                    }

                    Page {
                        Heading { text: qsTr("Resize the video frame") }
                        Choice {
                            label: qsTr("Resize video")
                            path: "picture.resize.when"
                            values: ["never", "smaller", "larger", "always"]
                            texts: ({ "never": qsTr("Do nothing"), "smaller": qsTr("When the input is smaller than"),
                                      "larger": qsTr("When the input is larger than"), "always": qsTr("Always") })
                        }
                        Check { path: "picture.resize.condition-display"; text: qsTr("Compare with the display's resolution") }
                        Amount { label: qsTr("Or with: width"); path: "picture.resize.condition-size.0"; from: 160; to: 7680; step: 16; unit: "px" }
                        Amount { label: qsTr("Or with: height"); path: "picture.resize.condition-size.1"; from: 120; to: 4320; step: 8; unit: "px" }
                        SizePresets { target: "picture.resize.condition-size" }
                        Choice {
                            label: qsTr("Size")
                            path: "picture.resize.target"
                            values: ["display", "multiply", "size"]
                            texts: ({ "display": qsTr("The display's resolution"), "multiply": qsTr("Multiply by"), "size": qsTr("Specify size") })
                        }
                        Amount { label: qsTr("Multiply by"); path: "picture.resize.factor"; from: 0.25; to: 4; step: 0.25 }
                        Amount { label: qsTr("Width"); path: "picture.resize.width"; from: 320; to: 7680; step: 16; unit: "px" }
                        Amount { label: qsTr("Height"); path: "picture.resize.height"; from: 240; to: 4320; step: 8; unit: "px" }
                        SizePresets { target: "picture.resize" }
                        Check { path: "picture.resize.auto-height"; text: qsTr("Calculate the height from the width") }
                        Check { path: "picture.resize.keep-aspect"; text: qsTr("Keep the video's aspect ratio inside the size") }
                        Choice {
                            label: qsTr("Method")
                            path: "picture.resize.method"
                            values: win.view.choices["resize-method"] || []
                        }
                        Amount { label: qsTr("Method's parameter"); path: "picture.resize.param"; from: 0; to: 10; step: 0.1; format: v => v === 0 ? "FFmpeg's default" : String(v) }
                        Check { path: "picture.resize.interlaced"; text: qsTr("Interlaced mode (scale the fields apart)") }
                        Note { text: qsTr("Resizing changes the frame before RVP's effects and SVP (FFmpeg's scale filter, keeping the video's shape); mpv's renderer still fits the picture to the window with its own scalers (Video page).") }
                    }

                    Page {
                        Heading { text: qsTr("Frame size presets") }
                        Repeater {
                            model: (win.s.picture && win.s.picture.presets) ? win.s.picture.presets.length : 0
                            delegate: ColumnLayout {
                                required property int index
                                Layout.fillWidth: true
                                Field { label: qsTr("Preset %1").arg((index + 1)); path: "picture.presets." + index + ".name" }
                                Amount { label: qsTr("Width"); path: "picture.presets." + index + ".width"; from: 10; to: 400; step: 5; unit: "%" }
                                Amount { label: qsTr("Height"); path: "picture.presets." + index + ".height"; from: 10; to: 400; step: 5; unit: "%" }
                            }
                        }
                        RowLayout {
                            Button {
                                text: qsTr("Add Preset")
                                enabled: !!win.s.picture && win.s.picture.presets.length < 20
                                onClicked: {
                                    const list = win.s.picture.presets.slice()
                                    list.push({ name: "Preset " + (list.length + 1), width: 100, height: 100 })
                                    win.set("picture.presets", list)
                                }
                            }
                            Button {
                                text: qsTr("Remove Last")
                                enabled: !!win.s.picture && win.s.picture.presets.length > 1
                                onClicked: win.set("picture.presets", win.s.picture.presets.slice(0, -1))
                            }
                        }
                        Check { path: "picture.remember-frame"; text: qsTr("Remember video size/position changes (the next file keeps them)") }
                        Heading { text: qsTr("Screen margins") }
                        Amount { label: qsTr("Top"); path: "picture.margins.top"; from: 0; to: 40; unit: "%" }
                        Amount { label: qsTr("Left"); path: "picture.margins.left"; from: 0; to: 40; unit: "%" }
                        Amount { label: qsTr("Bottom"); path: "picture.margins.bottom"; from: 0; to: 40; unit: "%" }
                        Amount { label: qsTr("Right"); path: "picture.margins.right"; from: 0; to: 40; unit: "%" }
                        Check { path: "picture.margins-fullscreen-only"; text: qsTr("Use the margins only in fullscreen") }
                        Check { path: "picture.remember-margins"; text: qsTr("Remember screen margin changes (these values come back at the next start)") }
                    }

                    Page {
                        Heading { text: qsTr("3D video") }
                        Choice {
                            label: qsTr("3D video mode")
                            path: "stereo.mode"
                            values: ["off", "on", "auto"]
                            texts: ({ "off": qsTr("Off"), "on": qsTr("On"), "auto": qsTr("On for files that say they are 3D") })
                        }
                        Choice {
                            label: qsTr("Input (source)")
                            path: "stereo.input"
                            values: ["side-by-side", "top-bottom", "original"]
                            texts: ({ "side-by-side": qsTr("Side by side"), "top-bottom": qsTr("Top and bottom"),
                                      "original": qsTr("Original image (not 3D)") })
                        }
                        Check { path: "stereo.full-resolution"; text: qsTr("Full resolution (each image as wide or tall as the frame)") }
                        Check { path: "stereo.swap"; text: qsTr("Swap left and right images") }
                        Check { path: "stereo.auto-detect"; text: qsTr("Detect the input from the file (stereo metadata, suffixes such as -lr, -ab, hsbs)") }
                        Choice {
                            label: qsTr("Output (screen)")
                            path: "stereo.output"
                            values: ["side-by-side", "top-bottom", "left", "right", "rows", "columns",
                                     "red-cyan-gray", "red-cyan-half", "red-cyan-color", "red-cyan-dubois",
                                     "green-magenta-gray", "green-magenta-half", "green-magenta-color", "green-magenta-dubois",
                                     "yellow-blue-gray", "yellow-blue-half", "yellow-blue-color", "yellow-blue-dubois",
                                     "red-blue-gray", "red-green-gray"]
                            texts: ({ "side-by-side": qsTr("Side by side"), "top-bottom": qsTr("Top and bottom"),
                                      "left": qsTr("3D to 2D (left or top image)"), "right": qsTr("3D to 2D (right or bottom image)"),
                                      "rows": qsTr("Row interleave"), "columns": qsTr("Column interleave"),
                                      "red-cyan-gray": qsTr("Anaglyph red/cyan gray"), "red-cyan-half": qsTr("Anaglyph red/cyan half colored"),
                                      "red-cyan-color": qsTr("Anaglyph red/cyan colored"), "red-cyan-dubois": qsTr("Anaglyph red/cyan Dubois"),
                                      "green-magenta-gray": qsTr("Anaglyph green/magenta gray"), "green-magenta-half": qsTr("Anaglyph green/magenta half colored"),
                                      "green-magenta-color": qsTr("Anaglyph green/magenta colored"), "green-magenta-dubois": qsTr("Anaglyph green/magenta Dubois"),
                                      "yellow-blue-gray": qsTr("Anaglyph yellow/blue gray"), "yellow-blue-half": qsTr("Anaglyph yellow/blue half colored"),
                                      "yellow-blue-color": qsTr("Anaglyph yellow/blue colored"), "yellow-blue-dubois": qsTr("Anaglyph yellow/blue Dubois"),
                                      "red-blue-gray": qsTr("Anaglyph red/blue gray"), "red-green-gray": qsTr("Anaglyph red/green gray") })
                        }
                        Amount { label: qsTr("Right image offset X"); path: "stereo.offset.0"; from: -100; to: 100; step: 2; unit: "px" }
                        Amount { label: qsTr("Right image offset Y"); path: "stereo.offset.1"; from: -100; to: 100; step: 2; unit: "px" }
                        Check { path: "stereo.invert"; text: qsTr("Flip both images horizontally") }
                        Note { text: qsTr("FFmpeg's stereo3d converts the frame before RVP's other filters and SVP's. Shutter glasses, NVIDIA 3D Vision and Windows' stereo output need a stereo swap chain, which mpv's renderer does not have.") }
                        Heading { text: qsTr("360° video") }
                        Choice {
                            label: qsTr("360° video mode")
                            path: "vr.mode"
                            values: ["off", "on", "auto"]
                            texts: ({ "off": qsTr("Off"), "on": qsTr("On"), "auto": qsTr("On for files named as 360° video") })
                        }
                        Choice {
                            label: qsTr("Projection")
                            path: "vr.projection"
                            values: ["auto", "equirectangular", "dual-fisheye", "cubemap", "equi-angular", "equi-angular-stereo"]
                            texts: ({ "auto": qsTr("From the frame's shape"), "equirectangular": qsTr("Equirectangular"),
                                      "dual-fisheye": qsTr("Dual fisheye"), "cubemap": qsTr("Cubemap (3x2)"),
                                      "equi-angular": qsTr("Equi-angular cubemap 3x2 (mono 2D)"),
                                      "equi-angular-stereo": qsTr("Equi-angular cubemap 2x3 (stereo 3D)") })
                        }
                        Check { path: "vr.four-views"; text: qsTr("Four camera angles (front, left, right, rear)") }
                        Amount { label: qsTr("Field of view"); path: "vr.fov"; from: 30; to: 150; step: 5; unit: "°" }
                        Note { text: qsTr("Drag the video with the mouse to look around.") }
                    }

                    Page {
                        Heading { text: qsTr("Pre-resize pixel shaders") }
                        Check { path: "shaders.enabled"; text: qsTr("Use pre-resize pixel shaders (Shift+P)") }
                        FileList { path: "shaders.files"; ordered: true }
                        ShaderPicker { target: "shaders.files"; stage: "pre" }
                        Heading { text: qsTr("Post-resize pixel shaders") }
                        Check { path: "shaders.post-enabled"; text: qsTr("Use post-resize pixel shaders (Ctrl+Alt+P)") }
                        FileList { path: "shaders.post-files"; ordered: true }
                        ShaderPicker { target: "shaders.post-files"; stage: "post" }
                        Heading { text: qsTr("Shader folders") }
                        Note {
                            text: win.view.choices["potplayer-shaders"]
                                  ? "PotPlayer's pixel shaders (HLSL, translated when they load): " + win.view.choices["potplayer-shaders"]
                                  : "PotPlayer's pixel shader folder was not found."
                        }
                        FileList { path: "shaders.folders" }
                        RowLayout {
                            Button {
                                text: qsTr("Add a Folder…")
                                onClicked: {
                                    folderListChooser.target = "shaders.folders"
                                    folderListChooser.open()
                                }
                            }
                            Button {
                                text: qsTr("Add Shader Files…")
                                onClicked: win.chooseFiles("shaders.files", "Pixel shaders",
                                                           ["Shaders (*.glsl *.hook *.hlsl *.txt *.fx)", "All files (*)"])
                            }
                            Button {
                                text: qsTr("Open Shader Folder")
                                onClicked: win.player.run_action("shaders-folder")
                            }
                            Button {
                                text: qsTr("Reload")
                                onClicked: win.player.run_action("shaders-reload")
                            }
                        }
                        Note { text: qsTr("The lists run in their order: pre-resize shaders on the video's own picture, post-resize ones after scaling. mpv's own shaders (.glsl, .hook) run where they say; RVP's set comes with RVP. RVP's shader folder: %1").arg((win.view.shader_folder || "")) }
                    }

                    Page {
                        Heading { text: qsTr("SVP") }
                        Field {
                            label: qsTr("IPC endpoint SVP attaches to")
                            path: "ipc-server"
                            placeholder: qsTr("empty: SVP mode off")
                            Button {
                                text: qsTr("Default")
                                onClicked: win.set("ipc-server", win.view["default-ipc"])
                            }
                        }
                        Label {
                            text: qsTr("Status: %1").arg((win.view.svp_status || ""))
                            wrapMode: Text.Wrap
                            Layout.fillWidth: true
                        }
                        Note {
                            text: qsTr("Endpoint in use: %1").arg((win.view.svp_endpoint || "none"))
                                  + ". SVP 4 Manager finds a player by this pipe or socket while it plays; a second RVP window uses mpvSockets/<process id> in the temporary folder, which SVP finds too. In SVP mode hardware decoding copies the frames back, so SVP's filter gets them."
                        }
                        Heading { text: qsTr("VapourSynth") }
                        Check { path: "vapoursynth.enabled"; text: qsTr("Run the selected script (before SVP's filter)") }
                        FileList { path: "vapoursynth.slots"; selectPath: "vapoursynth.script" }
                        Button {
                            text: qsTr("Add Script…")
                            onClicked: win.chooseFiles("vapoursynth.slots", "VapourSynth scripts",
                                                       ["VapourSynth scripts (*.vpy)", "All files (*)"])
                        }
                        Label { text: qsTr("Plugins loaded first (mvtools for PotPlayer's motion interpolation)") }
                        FileList { path: "vapoursynth.plugins" }
                        Button {
                            text: qsTr("Add Plugins…")
                            onClicked: win.chooseFiles("vapoursynth.plugins", "VapourSynth plugins",
                                                       [win.view.windows ? "Plugins (*.dll)" : "Plugins (*.so)", "All files (*)"])
                        }
                        Label {
                            visible: win.view.windows === true
                            text: qsTr("AviSynth plugins loaded first (VapourSynth's avs.LoadPlugin, with AviSynth+ installed)")
                        }
                        FileList { visible: win.view.windows === true; path: "vapoursynth.avs-plugins" }
                        Button {
                            visible: win.view.windows === true
                            text: qsTr("Add AviSynth Plugins…")
                            onClicked: win.chooseFiles("vapoursynth.avs-plugins", "AviSynth plugins",
                                                       ["AviSynth plugins (*.dll)", "All files (*)"])
                        }
                        Amount { label: qsTr("Buffered frames"); path: "vapoursynth.buffered-frames"; from: 1; to: 64 }
                        Field {
                            label: qsTr("VSScript library")
                            path: "vapoursynth.library"
                            optional: true
                            placeholder: qsTr("RVP's runtime")
                            Button {
                                text: qsTr("Browse…")
                                onClicked: win.chooseFile("vapoursynth.library", "VSScript library",
                                                          [win.view.windows ? "VSScript (VSScript.dll)" : "VSScript (libvapoursynth-script.so*)",
                                                           "All files (*)"])
                            }
                        }
                        Note {
                            text: qsTr("In use: %1").arg((win.view.vapoursynth_runtime || "none found"))
                                  + ". A new library takes effect at the next start. PotPlayer's .vpy scripts run unchanged: RVP wraps them for today's VapourSynth (get_core) and loads the plugins first."
                        }
                    }

                    Page {
                        Heading { text: qsTr("Motion Smoothing") }
                        Check { path: "smoothing.enabled"; text: qsTr("Smooth motion (Ctrl+Shift+I)") }
                        Choice {
                            label: qsTr("Engine")
                            path: "smoothing.engine"
                            values: ["blend", "mvtools", "rife", "rife-trt", "fruc-vulkan"]
                            texts: ({ "blend": qsTr("Blend: neighbouring frames blended at the display's rate"),
                                      "mvtools": qsTr("MVTools: motion compensation on the CPU"),
                                      "rife": qsTr("RIFE: optical flow on the GPU (Vulkan)"),
                                      "rife-trt": qsTr("RIFE (TensorRT): optical flow on NVIDIA's cards, with NVIDIA's TensorRT"),
                                      "fruc-vulkan": qsTr("NVIDIA Optical Flow: RTX 30 and later") })
                        }
                        Item {
                            id: tensorrt
                            // RIFE (TensorRT)'s installation, read again each
                            // second while the page shows.
                            property var status: ({})
                            Timer {
                                interval: 1000
                                repeat: true
                                triggeredOnStart: true
                                running: win.visible && win.currentKey === "smoothing"
                                onTriggered: tensorrt.status = JSON.parse(win.player.tensorrt_status() || "{}")
                            }
                            function gb(bytes) {
                                return (bytes / 1e9).toFixed(1)
                            }
                            function text() {
                                const status = tensorrt.status
                                if (status.state === "installing") {
                                    const stage = status.stage === "checking" ? qsTr("checking")
                                                : status.stage === "unpacking" ? qsTr("unpacking") : qsTr("downloading")
                                    return qsTr("RIFE (TensorRT): %1, %2 of %3 GB").arg(stage).arg(gb(status.done || 0)).arg(gb(status.total || status.download || 0))
                                }
                                if (status.state === "installed")
                                    return status.ready
                                        ? qsTr("RIFE (TensorRT): NVIDIA's TensorRT %1 is installed for %2 and its engine is built.").arg(status.version).arg(status.builder)
                                        : qsTr("RIFE (TensorRT): NVIDIA's TensorRT %1 is installed for %2; the calibration builds its engine (a few minutes) when nothing plays.").arg(status.version).arg(status.builder)
                                if (status.state === "other-card")
                                    return status.card
                                        ? qsTr("RIFE (TensorRT): NVIDIA's TensorRT %1 is installed for %2, and this card needs %3: download it again.").arg(status.version).arg(status.builder).arg(status.card)
                                        : qsTr("RIFE (TensorRT): NVIDIA's TensorRT %1 is installed for %2, and this computer has no NVIDIA card it runs on.").arg(status.version).arg(status.builder)
                                if (status.error)
                                    return qsTr("RIFE (TensorRT): %1").arg(status.error)
                                return qsTr("RIFE (TensorRT) needs NVIDIA's TensorRT: a %1 GB download from NVIDIA, of which about %2 GB stay.").arg(gb(status.download || 0)).arg(gb(status.kept || 0))
                            }
                        }
                        RowLayout {
                            visible: tensorrt.status.state !== undefined && tensorrt.status.state !== "unavailable"
                            Note {
                                Layout.fillWidth: true
                                text: tensorrt.text()
                            }
                            Button {
                                text: qsTr("Download…")
                                visible: tensorrt.status.state === "missing"
                                         || (tensorrt.status.state === "other-card" && !!tensorrt.status.card)
                                onClicked: tensorrtConsent.open()
                            }
                            Button {
                                text: qsTr("Cancel")
                                visible: tensorrt.status.state === "installing"
                                onClicked: win.player.tensorrt_cancel()
                            }
                            Button {
                                text: qsTr("Remove")
                                visible: tensorrt.status.state === "installed" || tensorrt.status.state === "other-card"
                                onClicked: {
                                    const error = win.player.tensorrt_remove()
                                    if (error !== "")
                                        tensorrt.status = Object.assign({}, tensorrt.status, { error: error })
                                }
                            }
                        }
                        ProgressBar {
                            Layout.fillWidth: true
                            visible: tensorrt.status.state === "installing"
                            from: 0
                            to: 1
                            value: (tensorrt.status.done || 0) / Math.max(1, tensorrt.status.total || 0)
                        }
                        Dialog {
                            id: tensorrtConsent
                            parent: Overlay.overlay
                            anchors.centerIn: parent
                            width: Math.min(560, win.width - 40)
                            modal: true
                            title: qsTr("Download NVIDIA's TensorRT")
                            contentItem: Label {
                                wrapMode: Text.Wrap
                                textFormat: Text.StyledText
                                text: qsTr("RVP downloads NVIDIA TensorRT %1 and CUDA's runtime from NVIDIA's servers (%2 GB) and keeps the files this card needs in RVP's state folder. They are NVIDIA's software under NVIDIA's terms, not RVP's: by downloading them you accept the NVIDIA TensorRT Software License Agreement and the CUDA End User License Agreement.").arg(tensorrt.status.version || "").arg(tensorrt.gb(tensorrt.status.download || 0))
                                      + "<br><br><a href=\"https://docs.nvidia.com/deeplearning/tensorrt/latest/reference/sla.html\">" + qsTr("TensorRT's licence") + "</a> · "
                                      + "<a href=\"https://docs.nvidia.com/cuda/eula/index.html\">" + qsTr("CUDA's EULA") + "</a>"
                                onLinkActivated: link => Qt.openUrlExternally(link)
                            }
                            footer: DialogButtonBox {
                                Button {
                                    text: qsTr("Accept and Download")
                                    DialogButtonBox.buttonRole: DialogButtonBox.AcceptRole
                                }
                                Button {
                                    text: qsTr("Cancel")
                                    DialogButtonBox.buttonRole: DialogButtonBox.RejectRole
                                }
                            }
                            onAccepted: win.player.tensorrt_install()
                        }
                        Choice {
                            label: qsTr("Preset")
                            path: "smoothing.preset"
                            values: ["fast", "medium", "quality"]
                            texts: ({ "fast": qsTr("Fast"), "medium": qsTr("Medium"), "quality": qsTr("Quality") })
                        }
                        Choice {
                            label: qsTr("Target rate")
                            path: "smoothing.target"
                            values: win.view.choices["smoothing-target"] || []
                            texts: ({ "display": qsTr("The display's rate"), "2x": qsTr("Twice the video's rate") })
                        }
                        Amount {
                            label: qsTr("Motion compensation up to")
                            path: "smoothing.max-height"
                            values: win.view.choices["smoothing-max-height"] || [1080]
                            format: v => v + "p (Blend above)"
                        }
                        Amount {
                            label: qsTr("Show the original first for")
                            path: "smoothing.original-first"
                            values: win.view.choices["smoothing-original-first"] || [1]
                            format: v => v === 0 ? qsTr("Off") : qsTr("%1 s").arg(v)
                        }
                        Choice {
                            label: qsTr("At a scene change")
                            path: "smoothing.scene-change"
                            values: win.view.choices["smoothing-scene-change"] || []
                            texts: ({ "off": qsTr("Interpolate across it (one scene morphs into the next)"),
                                      "normal": qsTr("Repeat the scene's last frame"),
                                      "high": qsTr("Repeat the scene's last frame (more sensitive: alike scenes too, now and then a fast pan)") })
                        }
                        Amount {
                            label: qsTr("Step down after dropped frames over")
                            path: "smoothing.drop-window"
                            values: win.view.choices["smoothing-drop-window"] || [20]
                            format: v => v === 0 ? qsTr("never") : qsTr("%1 s").arg(v)
                        }
                        Note { text: qsTr("Now: %1").arg((win.view.smoothing || "")) }
                        Note {
                            text: qsTr("Here: MVTools %1").arg((win.view.smoothing_engines && win.view.smoothing_engines.mvtools ? "found" : "missing"))
                                  + ", RIFE " + (win.view.smoothing_engines && win.view.smoothing_engines.rife ? "found" : "missing")
                                  + ", NVIDIA Optical Flow " + (win.view.smoothing_engines && win.view.smoothing_engines.fruc ? "found" : "missing")
                                  + " (in RVP's VapourSynth runtime and libmpv)."
                        }
                        Item {
                            id: calibration
                            // The calibration's state, read again each second
                            // while the page shows.
                            property var status: ({})
                            Timer {
                                interval: 1000
                                repeat: true
                                triggeredOnStart: true
                                running: win.visible && win.currentKey === "smoothing"
                                onTriggered: calibration.status = JSON.parse(win.player.calibration_status() || "{}")
                            }
                            function presetText(preset) {
                                return preset === "Fast" ? qsTr("Fast") : preset === "Medium" ? qsTr("Medium") : qsTr("Quality")
                            }
                            function summary() {
                                const status = calibration.status
                                let parts = (status.rows || []).map(row => row.height > 0
                                    ? qsTr("%1 %2 up to %3p").arg(row.engine).arg(presetText(row.preset)).arg(row.height)
                                    : qsTr("%1 %2 below 480p").arg(row.engine).arg(presetText(row.preset)))
                                parts = parts.concat((status.failed || []).map(engine => qsTr("%1 does not run here").arg(engine)))
                                return parts.join(", ")
                            }
                            function text() {
                                const status = calibration.status
                                if (status.state === "running")
                                    return qsTr("Calibration: measuring while nothing plays (%1 done, at most %2 left)").arg(status.done || 0).arg(status.left || 0)
                                if (status.state === "measured")
                                    return qsTr("Calibration of %1 at 60 frames a second: %2").arg(status.date || "").arg(summary())
                                if (status.state === "unavailable")
                                    return qsTr("Calibration: no engine to measure here.")
                                if (status.state === "cancelled")
                                    return qsTr("Calibration: cancelled; it starts again with Start in its window.")
                                return qsTr("Calibration: not measured yet; it runs when smoothing is on and nothing plays for a few seconds.")
                            }
                        }
                        RowLayout {
                            Note {
                                Layout.fillWidth: true
                                text: calibration.text()
                            }
                            Button {
                                text: calibration.status.state === "measured" ? qsTr("Recalibrate…") : qsTr("Calibrate…")
                                onClicked: win.openCalibration()
                            }
                        }
                        Note {
                            text: qsTr("Smoothing stays off while SVP is attached, while RVP's own VapourSynth script runs and when the video plays at the target rate already; while SVP 4 Manager runs, it waits 8 s for SVP after a file starts. An engine that cannot run here gives way: RIFE and NVIDIA Optical Flow to MVTools, MVTools to Blend; one whose output drops more than 5 % of its frames over the time above, or more than half of them over 5 s, steps down (a lighter preset, then MVTools, then Blend; RIFE gives way to MVTools at once) until the file ends. Above the GPU power budget, smoothing is lightened first. After a start or a seek the original shows for the time above while MVTools or RIFE get ready, as SVP does; RIFE (TensorRT) starts with its own frames. The Log page's Info level records each step and the seconds that dropped frames.")
                        }
                        Note {
                            text: qsTr("The calibration measures once how many frames a second MVTools and RIFE make here at each preset from 480p to 2160p, as SVP does, and again when the processors, graphics cards, their drivers or the engines change; each file then starts at the best preset that keeps its rate with a fifth to spare, at most the preset above. NVIDIA Optical Flow is not measured. Steps down taken while other programs loaded the processors come back at the next seek once that load is gone.")
                        }
                    }

                    Page {
                        Heading { text: qsTr("Subtitles") }
                        Check { path: "subtitles.show"; text: qsTr("Show subtitles when a file starts") }
                        Choice {
                            label: qsTr("Load subtitle files")
                            path: "subtitle-auto"
                            values: win.withCurrent(win.view.choices["subtitle-auto"], win.s["subtitle-auto"])
                            texts: ({ "no": qsTr("No"), "exact": qsTr("Only those that match the video's name"),
                                      "fuzzy": qsTr("Those containing the video's name"), "all": qsTr("All in the folder") })
                        }
                        Check { path: "subtitles.auto-reload"; text: qsTr("Reload subtitle files when they change") }
                        Choice {
                            label: qsTr("ASS/SSA subtitle style")
                            path: "subtitles.ass-override"
                            values: win.view.choices["subtitles.ass-override"] || []
                            texts: ({ "no": qsTr("The script's own style"), "scale": qsTr("The script's style at RVP's size"),
                                      "yes": qsTr("RVP's style where the script allows"), "force": qsTr("RVP's style over all styles"),
                                      "strip": qsTr("Plain text in RVP's style") })
                        }
                        Check { path: "subtitles.override-font"; text: qsTr("ASS scripts take RVP's font") }
                        Check { path: "subtitles.override-color"; text: qsTr("ASS scripts take RVP's text colour") }
                        Check {
                            path: "subtitles.fonts-folder"
                            text: qsTr("Load fonts from the fonts folder%1").arg((win.view.fonts_folder ? " (" + win.view.fonts_folder + ")" : ""))
                        }
                        Heading { text: qsTr("Placement") }
                        Check { path: "subtitles.text-inside"; text: qsTr("Display text subtitles inside the video") }
                        Check { path: "subtitles.under-video"; text: qsTr("Force subtitles to display under the video") }
                        Check { path: "subtitles.bitmap-outside"; text: qsTr("Display bitmap subtitles on the whole screen (outside the video)") }
                        Check { path: "subtitles.fix-aspect"; text: qsTr("Correct the shapes of scripts made for anamorphic video") }
                        Note { text: qsTr("Under the video, text subtitles go into the black bars or the bottom margin (Video > Bottom Margin).") }
                        Heading { text: qsTr("Processing") }
                        Check { path: "subtitles.remove-sdh"; text: qsTr("Remove notes for the hearing impaired ([...] and (...))") }
                        Check { path: "subtitles.remove-blank"; text: qsTr("Remove subtitles that are only spaces") }
                        Note { text: qsTr("mpv (libass) draws every subtitle: PotPlayer's output surfaces, display engines and font renderers have no counterpart, and ASS animations always play.") }
                    }

                    Page {
                        Heading { text: qsTr("Font") }
                        Field {
                            label: qsTr("Font")
                            path: "subtitle-font"
                            placeholder: qsTr("mpv's default (sans-serif)")
                            Button {
                                text: qsTr("Choose…")
                                onClicked: {
                                    const family = win.value("subtitle-font")
                                    subtitleFontChooser.currentFont = Qt.font({
                                        family: family ? family : "sans-serif", pointSize: 20 })
                                    subtitleFontChooser.open()
                                }
                            }
                        }
                        Amount { label: qsTr("Size"); path: "subtitle-scale"; unit: "%"; factor: 100; from: 30; to: 300; step: 5 }
                        Check { path: "subtitles.bold"; text: qsTr("Bold") }
                        Check { path: "subtitles.italic"; text: qsTr("Italic") }
                        Amount { label: qsTr("Letter spacing"); path: "subtitles.spacing"; from: -10; to: 10; step: 0.5; fallback: (win.view.subtitle_defaults || {})["spacing"] }
                        Amount { label: qsTr("Line spacing"); path: "subtitles.line-spacing"; from: -50; to: 50; step: 1; fallback: (win.view.subtitle_defaults || {})["line-spacing"] }
                        Amount { label: qsTr("Letter width"); path: "subtitles.scale-x"; unit: "%"; from: 10; to: 400; step: 5; fallback: 100 }
                        Amount { label: qsTr("Letter height"); path: "subtitles.scale-y"; unit: "%"; from: 10; to: 400; step: 5; fallback: 100 }
                        Heading { text: qsTr("Colours, outline and shadow") }
                        ColorChoice { label: qsTr("Text"); path: "subtitles.color"; fallback: (win.view.subtitle_defaults || {})["color"] || "" }
                        ColorChoice { label: qsTr("Outline"); path: "subtitles.outline-color"; fallback: (win.view.subtitle_defaults || {})["outline-color"] || "" }
                        ColorChoice { label: qsTr("Shadow and box"); path: "subtitles.back-color"; fallback: (win.view.subtitle_defaults || {})["back-color"] || "" }
                        Amount { label: qsTr("Outline width"); path: "subtitles.outline-size"; from: 0; to: 10; step: 0.25; fallback: (win.view.subtitle_defaults || {})["outline-size"] }
                        Amount { label: qsTr("Shadow offset"); path: "subtitles.shadow-offset"; from: 0; to: 10; step: 0.25; fallback: (win.view.subtitle_defaults || {})["shadow-offset"] }
                        Amount { label: qsTr("Outline blur"); path: "subtitles.blur"; from: 0; to: 20; step: 0.5; fallback: (win.view.subtitle_defaults || {})["blur"] }
                        Choice {
                            label: qsTr("Border")
                            path: "subtitles.border-style"
                            values: win.view.choices["subtitles.border-style"] || []
                            texts: ({ "outline-and-shadow": qsTr("Outline and shadow"),
                                      "opaque-box": qsTr("An opaque box in the outline's colour around each line"),
                                      "background-box": qsTr("One box in the shadow's colour behind all lines") })
                        }
                        Heading { text: qsTr("Position and margins") }
                        Choice {
                            label: qsTr("Paragraph align")
                            path: "subtitles.align"
                            values: win.view.choices["subtitles.align"] || []
                            texts: ({ "left": qsTr("Left"), "center": qsTr("Centre"), "right": qsTr("Right") })
                        }
                        Amount { label: qsTr("Vertical position"); path: "subtitles.position"; unit: "%"; from: 0; to: 150; step: 1; fallback: 100 }
                        Amount { label: qsTr("Margin at the sides"); path: "subtitles.margin-x"; from: 0; to: 300; step: 5; fallback: (win.view.subtitle_defaults || {})["margin-x"] }
                        Amount { label: qsTr("Margin at the bottom or top"); path: "subtitles.margin-y"; from: 0; to: 300; step: 5; fallback: (win.view.subtitle_defaults || {})["margin-y"] }
                        Note { text: qsTr("A value without a Default button is libmpv's own. Margins are in pixels of a 720-line picture; mpv keeps one margin for both sides and one for the bottom or top. Moves left and right (Subtitles > Subtitle Style) go from the left edge through the centre to the right edge.") }
                    }

                    Page {
                        Heading { text: qsTr("Folders for subtitle files") }
                        Note { text: qsTr("Besides the video's folder, RVP looks for subtitles with the video's name in these folders (a name without a drive is inside the video's folder).") }
                        FileList { path: "subtitles.folders"; ordered: true }
                        Button {
                            text: qsTr("Add…")
                            onClicked: {
                                folderListChooser.target = "subtitles.folders"
                                folderListChooser.open()
                            }
                        }
                        Heading { text: qsTr("Text encoding") }
                        Choice {
                            label: qsTr("Files that are not UTF-8")
                            path: "subtitles.encoding"
                            values: win.view.choices["subtitles.encoding"] || []
                            texts: ({ "auto": qsTr("Guess (Automatic)") })
                        }
                        Heading { text: qsTr("Fade and 3D") }
                        Note { text: qsTr("mpv fades only the events of ASS scripts that ask for it (\\fad); other subtitles cannot fade. mpv draws subtitles once over the finished picture, not once for each eye, so there is no stereo 3D subtitle mode (docs/spikes/m12-studies.md).") }
                    }

                    Page {
                        Heading { text: qsTr("Language") }
                        Field { label: qsTr("Preferred languages"); path: "subtitle-languages"; placeholder: qsTr("en,ru") }
                        Check { path: "subtitles.only-preferred"; text: qsTr("Don't select subtitles of other languages") }
                        Check { path: "subtitles.second-auto"; text: qsTr("Show a second subtitle when there is one of these languages") }
                        Field { label: qsTr("Second subtitle languages"); path: "subtitles.second-languages"; placeholder: qsTr("en") }
                        Amount { label: qsTr("Second subtitle position"); path: "subtitles.second-position"; unit: "%"; from: 0; to: 150; step: 1; fallback: (win.view.subtitle_defaults || {})["second-position"] }
                        Check { path: "subtitles.remember-tracks"; text: qsTr("Remember the chosen subtitles for each file") }
                        Heading { text: qsTr("Synchronization") }
                        Amount { label: qsTr("Sync step (< and >)"); path: "subtitle-delay-step"; values: [0.01, 0.05, 0.1, 0.2, 0.25, 0.5, 1, 2, 5, 10, 30, 60]; format: win.seconds }
                        Amount { label: qsTr("Sync step (Ctrl+< and Ctrl+>)"); path: "subtitles.sync-step-2"; values: [0.01, 0.05, 0.1, 0.2, 0.25, 0.5, 1, 2, 5, 10, 30, 60]; format: win.seconds }
                        Amount { label: qsTr("Sync step (Alt+< and Alt+>)"); path: "subtitles.sync-step-3"; values: [0.01, 0.05, 0.1, 0.2, 0.25, 0.5, 1, 2, 5, 10, 30, 60]; format: win.seconds }
                        Check { path: "subtitles.keep-sync"; text: qsTr("Keep subtitle sync changes for the next files") }
                        Check { path: "subtitles.sync-from-next"; text: qsTr("Sync changes move only the subtitles after the current one (subtitle files RVP edits: SRT, ASS, WebVTT)") }
                    }

                    Page {
                        Heading { text: qsTr("Subtitle browser (Alt+E)") }
                        Choice {
                            label: qsTr("Times")
                            path: "subtitles.browser.times"
                            values: win.view.choices["subtitles.browser.times"] || []
                            texts: ({ "start-end": qsTr("Start and end"), "start": qsTr("Start"), "none": qsTr("None") })
                        }
                        Check { path: "subtitles.browser.synced-times"; text: qsTr("Show the times as they play (with the sync)") }
                        Amount { label: qsTr("Opacity"); path: "subtitles.browser.opacity"; unit: "%"; from: 20; to: 100; step: 5 }
                        Field { label: qsTr("Font"); path: "subtitles.browser.font"; placeholder: qsTr("The window's font") }
                        Amount { label: qsTr("Font size"); path: "subtitles.browser.font-size"; from: 6; to: 48; step: 1 }
                        Amount { label: qsTr("Lines of each subtitle"); path: "subtitles.browser.lines"; from: 1; to: 5; step: 1 }
                        Check { path: "subtitles.browser.follow"; text: qsTr("Follow the subtitle that shows") }
                        Note { text: qsTr("The browser edits subtitle files (SRT, ASS, WebVTT): edits show at once and Save writes them into the file with the sync. libmpv gives no access to the subtitles inside a video.") }
                    }

                    Page {
                        Heading { text: qsTr("Subtitles from audio (whisper.cpp, offline)") }
                        Choice {
                            label: qsTr("Engine")
                            path: "speech.file.engine"
                            values: win.view.choices["speech.engine"] || []
                            texts: (win.view.speech || {}).engines || ({})
                        }
                        Choice {
                            label: qsTr("Model")
                            path: "speech.file.model"
                            values: win.view.choices["speech.model"] || []
                            texts: (win.view.speech || {}).models || ({})
                        }
                        Choice {
                            label: qsTr("Language")
                            path: "speech.file.language"
                            values: win.view.choices["speech.language"] || []
                            texts: (win.view.speech || {}).languages || ({})
                        }
                        Check { path: "speech.voice-filter"; text: qsTr("Apply a voice emphasis filter when extracting the sound") }
                        Check { path: "speech.beside-video"; text: qsTr("Save the subtitles in the video's folder when possible") }
                        Check { path: "speech.show"; text: qsTr("Show made subtitles when they arrive") }
                        Choice {
                            label: qsTr("When a file starts")
                            path: "speech.auto"
                            values: win.view.choices["speech.auto"] || []
                            texts: ({ "off": qsTr("Do not make subtitles"), "always": qsTr("Make subtitles"),
                                      "no-subtitles": qsTr("Make subtitles when the file has none") })
                        }
                        RowLayout {
                            Button {
                                text: (win.view.speech || {}).running ? "Stop" : "Make subtitles now"
                                onClicked: { win.player.run_action("speech-start"); win.reload(false) }
                            }
                        }
                        Heading { text: qsTr("Real time") }
                        Choice {
                            label: qsTr("Use")
                            path: "speech.realtime"
                            values: win.view.choices["speech.realtime"] || []
                            texts: ({ "off": qsTr("Not used"), "always": qsTr("Always"), "no-subtitles": qsTr("When the file has no subtitles"),
                                      "streams": qsTr("For sources that are not files") })
                        }
                        Choice {
                            label: qsTr("Engine")
                            path: "speech.realtime-recognizer.engine"
                            values: win.view.choices["speech.engine"] || []
                            texts: (win.view.speech || {}).engines || ({})
                        }
                        Choice {
                            label: qsTr("Model")
                            path: "speech.realtime-recognizer.model"
                            values: win.view.choices["speech.model"] || []
                            texts: (win.view.speech || {}).models || ({})
                        }
                        Choice {
                            label: qsTr("Language")
                            path: "speech.realtime-recognizer.language"
                            values: win.view.choices["speech.language"] || []
                            texts: (win.view.speech || {}).languages || ({})
                        }
                        Note { text: qsTr("Real time makes 30 s pieces from the playback position on, two ahead of it, and again from where a seek lands.") }
                        Heading { text: qsTr("Engines and models") }
                        Field { label: qsTr("Models folder"); path: "speech.model-folder"; optional: true; placeholder: "whisper-models" }
                        RowLayout {
                            Label { text: qsTr("Models: %1").arg(((win.view.speech || {}).model_folder || "")); Layout.fillWidth: true; wrapMode: Text.WrapAnywhere }
                            Button { text: qsTr("Open"); onClicked: win.player.run_action("speech-model-folder") }
                        }
                        RowLayout {
                            Label { text: qsTr("Engines: %1").arg(((win.view.speech || {}).engine_folder || "")); Layout.fillWidth: true; wrapMode: Text.WrapAnywhere }
                            Button { text: qsTr("Open"); onClicked: win.player.run_action("speech-engine-folder") }
                        }
                        Note { text: qsTr("Models are whisper.cpp's ggml-<model>.bin files from %1 that you download into the models folder; RVP downloads nothing and sends nothing anywhere. RVP's CPU and Vulkan engine comes with it; whisper.cpp's CUDA and BLAS builds (%2) go into the engines folder's cuda and blas folders. Above the GPU power budget, the next pieces and jobs run on the CPU.").arg(((win.view.speech || {}).models_url || "")).arg(((win.view.speech || {}).engines_url || "")) }
                    }

                    Loader {
                        id: keysPage
                        Component.onCompleted: setSource("KeysAndMouse.qml", { player: win.player })
                    }

                    Page {
                        Heading { text: qsTr("Keyboard") }
                        Check { path: "input.disable-default-keys"; text: qsTr("Disable default hotkeys (your own keys stay)") }
                        Check { path: "input.media-keys"; text: qsTr("Support the keyboard's media keys in RVP's window") }
                        Check { path: "input.global-media-keys"; text: qsTr("Take the media keys while another window has the keyboard") }
                        Check { path: "input.volume-keys"; text: qsTr("The keyboard's volume keys change RVP's volume") }
                        Check { path: "input.navigation-keys"; text: qsTr("The keyboard's back and forward keys go to the previous and next item") }
                        Note { text: qsTr("While RVP's window has no keyboard, Windows' media controls take the media keys (General: system media transport controls).") }
                        Heading { text: qsTr("Mouse") }
                        Check { path: "input.left-click-video-only"; text: qsTr("Operate the left click in the video area only") }
                        Check { path: "input.drag-title-only"; text: qsTr("Prevent the window from being dragged outside of the title bar") }
                        Check { path: "input.drag-picture"; text: qsTr("Alt, Ctrl and Shift with a drag on the video pan, zoom and stretch the picture") }
                        Check { path: "input.drag-subtitles"; text: qsTr("Ctrl+Alt with a drag on the video moves the subtitles") }
                        Check { path: "input.x-wheel"; text: qsTr("Operate the wheel while X1 or X2 is held (X1/X2 Wheel gestures on the Keys and Mouse page)") }
                    }

                    Page {
                        Heading { text: qsTr("Skin") }
                        Field {
                            label: qsTr("PotPlayer skin (.dsf)")
                            path: "skin"
                            optional: true
                            placeholder: qsTr("RVP Default (built in)")
                            Button {
                                text: qsTr("Browse…")
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
                        Note { text: qsTr("Empty: RVP Default, RVP's own skin. RVP also draws PotPlayer's .dsf skins (BlackSpace is the one it is made for), from its skins folder (%1), next to the chosen skin and PotPlayer's. The skin changes at once (also from the Skins menu).").arg((win.view.skins_folder || "")) }
                        RowLayout {
                            Button { text: qsTr("Rescan Skin Files"); onClicked: win.reload(false) }
                            Button {
                                text: qsTr("Open Skin Folder")
                                onClicked: Qt.openUrlExternally(win.player.settings_folder_url("skins"))
                            }
                        }
                        Heading { text: qsTr("Skin modes") }
                        Check { path: "skins.video-skin-at-start"; text: qsTr("Use the video skin at program start") }
                        Check { path: "skins.lock-mode"; text: qsTr("Disable switching between skin modes (the audio skin for files without video)") }
                        Check { path: "skins.remember-mode-window"; text: qsTr("Remember the window's position and size of each skin mode") }
                        Check { path: "skins.keep-size"; text: qsTr("Keep the window's size while switching between skin modes") }
                        Heading { text: qsTr("The skin over the video") }
                        Check { path: "skins.auto-hide"; text: qsTr("Auto hide the main window's skin while a video shows (its bars come over the video)") }
                        Check { path: "skins.fullscreen-resize"; text: qsTr("In fullscreen the video makes room for the bars while they show") }
                        Check { path: "skins.animations"; text: qsTr("Enable window animations (the bars and the control panel fade)") }
                        Heading { text: qsTr("mpv's on-screen controller") }
                        Choice {
                            label: qsTr("Controller")
                            path: "controller.kind"
                            values: ["", "osc", "scripts"]
                            texts: ({ "": qsTr("None (the skin's controls)"), "osc": qsTr("mpv's OSC"),
                                      "scripts": qsTr("Scripts from RVP's mpv folder") })
                        }
                        Check { path: "controller.without-skin"; text: qsTr("Without the skin: the window shows only the video and the controller") }
                        RowLayout {
                            Button {
                                text: qsTr("Open mpv Folder")
                                onClicked: Qt.openUrlExternally(win.player.settings_folder_url("mpv"))
                            }
                            Button { text: qsTr("Rescan Scripts"); onClicked: win.reload(false) }
                        }
                        Note { text: qsTr("The controller draws over the video and takes the pointer there; in fullscreen it takes the place of the skin's bars. Scripts such as uosc or ModernZ go into the scripts folder of RVP's mpv folder (%1), their fonts into fonts and their options into script-opts, as in mpv's own folder. Scripts found: %2. A new choice takes effect when RVP starts again.").arg(win.view.mpv_folder || "").arg((win.view.mpv_scripts || []).join(", ") || qsTr("none")) }
                        Heading { text: qsTr("Context menu") }
                        Choice {
                            label: qsTr("Context menu design")
                            path: "skins.menu-design"
                            values: ["skin", "default"].concat(win.view.menu_files || [])
                            texts: ({ "skin": qsTr("Skin defined/Default (recommended)"), "default": qsTr("Default") })
                        }
                        RowLayout {
                            Button {
                                text: qsTr("Open Menu Folder")
                                onClicked: Qt.openUrlExternally(win.player.settings_folder_url("menus"))
                            }
                            Button { text: qsTr("Context Menu Items…"); onClicked: menuItems.begin() }
                        }
                        Choice {
                            label: qsTr("Context menu style")
                            path: "skins.menu-style"
                            values: ["auto", "system", "skin-1", "skin-2"]
                            texts: ({ "auto": qsTr("Auto select"), "system": qsTr("System default"),
                                      "skin-1": qsTr("Style #1 (defined in skin)"), "skin-2": qsTr("Style #2 (defined in skin)") })
                        }
                        Check { path: "skins.menu-dark"; text: qsTr("System dark mode (dark menus while the system is dark)") }
                        Amount { label: qsTr("Transparency of the menus"); path: "skins.menu-transparency"; from: 0; to: 90; step: 10; unit: "%" }
                        Amount { label: qsTr("Size of the menus"); path: "skins.menu-size"; from: 80; to: 200; step: 10; unit: "%" }
                        Note { text: qsTr("Menu files hold PotPlayer's menu items (its Menus folder's format), in RVP's menus folder (%1) and, on Windows, PotPlayer's. The system's menus are Windows' own; elsewhere RVP draws them.").arg((win.view.menus_folder || "")) }
                        Heading { text: qsTr("Times in the skin") }
                        Check { path: "skins.reverse-times"; text: qsTr("Reverse the elapsed and remaining times") }
                        Check { path: "skins.speed-times"; text: qsTr("Correct the times according to the playback speed") }
                        Heading { text: qsTr("Opacity") }
                        Amount { label: qsTr("Main window"); path: "skins.opacity"; from: 10; to: 100; step: 10; unit: "%" }
                        Amount { label: qsTr("Control panel"); path: "skins.panel-opacity"; from: 10; to: 100; step: 10; unit: "%" }
                    }

                    Page {
                        Heading { text: qsTr("Logo while playback is off") }
                        Choice {
                            label: qsTr("Logo")
                            path: "skins.idle-logo"
                            values: ["skin", "default"].concat(win.view.logo_list || [])
                            texts: ({ "skin": qsTr("[Skin defined]"), "default": qsTr("<Default logo>") })
                        }
                        Heading { text: qsTr("Logo while starting playback") }
                        Choice {
                            label: qsTr("Logo")
                            path: "skins.start-logo"
                            values: ["same", "skin", "default"].concat(win.view.logo_list || [])
                            texts: ({ "same": qsTr("{Same as the first logo}"), "skin": qsTr("[Skin defined]"), "default": qsTr("<Default logo>") })
                        }
                        Heading { text: qsTr("Logo settings") }
                        Choice {
                            label: qsTr("Logo position")
                            path: "skins.logo-position"
                            values: ["center", "top-left", "top-right", "bottom-left", "bottom-right", "fit"]
                            texts: ({ "center": qsTr("Center"), "top-left": qsTr("Top left"), "top-right": qsTr("Top right"),
                                      "bottom-left": qsTr("Bottom left"), "bottom-right": qsTr("Bottom right"),
                                      "fit": qsTr("Fit to the video area") })
                        }
                        Check { path: "skins.random-logo"; text: qsTr("Change the logo randomly on each loading") }
                        RowLayout {
                            Button {
                                text: qsTr("Open Logo Folder")
                                onClicked: Qt.openUrlExternally(win.player.settings_folder_url("logos"))
                            }
                            Button { text: qsTr("Rescan Logo Files"); onClicked: win.reload(false) }
                        }
                        Note { text: qsTr("Pictures (PNG, JPEG, BMP, GIF) in RVP's logo folder (%1) and, on Windows, PotPlayer's. The skin's own logo: PotPlayer's Flash logos (BlackSpace's) cannot show, so RVP's shows instead.").arg((win.view.logos_folder || "")) }
                    }

                    Page {
                        Heading { text: qsTr("On Screen Control") }
                        Choice {
                            label: qsTr("Method")
                            path: "skins.osc"
                            values: [true, false]
                            texts: ({ "true": qsTr("The skin's bars over the video (Qt)"), "false": qsTr("Disable") })
                        }
                        Check { path: "skins.osc-keep-while-over"; text: qsTr("Disable auto hide of the bars while the mouse pointer is over them") }
                        Amount { label: qsTr("Opacity"); path: "skins.osc-opacity"; from: 10; to: 100; step: 5; unit: "%" }
                        Heading { text: qsTr("Zoom skinned windows") }
                        Amount { label: qsTr("Main window"); path: "skins.zoom"; from: 100; to: 300; step: 25; unit: "%" }
                        Amount { label: qsTr("Control panel"); path: "skins.panel-zoom"; from: 100; to: 300; step: 25; unit: "%" }
                        Note { text: qsTr("RVP's playlist is the main window's attached panel: it zooms with it.") }
                        Heading { text: qsTr("Color themes") }
                        Choice {
                            label: qsTr("Color theme")
                            path: "skins.color-theme"
                            values: ["", "random", "custom"].concat(win.view.skin_themes || [])
                            texts: ({ "": qsTr("Do not use"), "random": qsTr("Random (each start)"), "custom": qsTr("Custom") })
                        }
                        RowLayout {
                            RadioButton {
                                text: qsTr("HSB color theme")
                                checked: win.value("skins.custom.hsb") === true
                                onClicked: win.set("skins.custom.hsb", true)
                            }
                            RadioButton {
                                text: qsTr("RGB color theme")
                                checked: win.value("skins.custom.hsb") === false
                                onClicked: win.set("skins.custom.hsb", false)
                            }
                            Button { text: qsTr("Set Custom Color Theme…"); onClicked: win.player.run_action("theme-customize") }
                        }
                    }

                    Page {
                        Heading { text: qsTr("General OSD settings") }
                        Check { path: "osd.messages"; text: qsTr("Show general info messages") }
                        Choice {
                            label: qsTr("Output")
                            path: "osd.output"
                            values: ["window", "video"]
                            texts: ({ "window": qsTr("Over the video (RVP's window)"), "video": qsTr("In the picture (mpv's OSD)") })
                        }
                        Choice {
                            label: qsTr("Horizontal align")
                            path: "osd.h-align"
                            values: ["left", "center", "right"]
                            texts: ({ "left": qsTr("Left"), "center": qsTr("Center"), "right": qsTr("Right") })
                        }
                        Choice {
                            label: qsTr("Vertical align")
                            path: "osd.v-align"
                            values: ["top", "center", "bottom"]
                            texts: ({ "top": qsTr("Top"), "center": qsTr("Center"), "bottom": qsTr("Bottom") })
                        }
                        Amount { label: qsTr("Top margin"); path: "osd.top-margin"; from: 0; to: 200; unit: "px" }
                        Amount { label: qsTr("Right margin"); path: "osd.right-margin"; from: 0; to: 200; unit: "px" }
                        Check { path: "osd.auto-size"; text: qsTr("Auto font size (follows the video's height)") }
                        Field {
                            label: qsTr("Font")
                            path: "osd.font"
                            placeholder: qsTr("Qt's font")
                            Button {
                                text: qsTr("Choose…")
                                onClicked: win.chooseOsdFont("osd.font")
                            }
                        }
                        Amount { label: qsTr("Font size"); path: "osd.font-size"; from: 8; to: 96; unit: "px" }
                        Field {
                            label: qsTr("Info font")
                            path: "osd.info-font"
                            placeholder: qsTr("Qt's font")
                            Button {
                                text: qsTr("Choose…")
                                onClicked: win.chooseOsdFont("osd.info-font")
                            }
                        }
                        Amount { label: qsTr("Info font size"); path: "osd.info-font-size"; from: 8; to: 96; unit: "px" }
                        Check { path: "osd.bold"; text: qsTr("Bold") }
                        // The unset colours and sizes are osd.rs's TEXT_COLOR and the others.
                        Heading { text: qsTr("Colours, outline and shadow") }
                        ColorChoice { label: qsTr("Text"); path: "osd.color"; fallback: "#FFF0F0F0" }
                        ColorChoice { label: qsTr("Outline"); path: "osd.outline-color"; fallback: "#FF000000" }
                        ColorChoice { label: qsTr("Shadow"); path: "osd.shadow-color"; fallback: "#80000000" }
                        ColorChoice { label: qsTr("Info box"); path: "osd.box-color"; fallback: "#99000000" }
                        Amount { label: qsTr("Outline width"); path: "osd.outline-size"; from: 0; to: 10; step: 1; unit: "px"; fallback: 1 }
                        Amount { label: qsTr("Shadow offset"); path: "osd.shadow-offset"; from: 0; to: 10; step: 1; unit: "px"; fallback: 0 }
                        Note { text: qsTr("The notices and the info texts (Tab, Scroll Lock) take these colours; in the picture mpv draws the notices with them.") }
                        Heading { text: qsTr("Content of Misc Info (Scroll Lock)") }
                        Check { path: "osd.misc.show"; text: qsTr("Display misc info") }
                        Check { path: "osd.misc.key-fullscreen-only"; text: qsTr("Operate only in fullscreen") }
                        Check { path: "osd.misc.fullscreen-only"; text: qsTr("Display only in fullscreen") }
                        Choice {
                            label: qsTr("Playback time")
                            path: "osd.misc.time"
                            values: ["none", "elapsed", "remaining", "both"]
                            texts: ({ "none": qsTr("Do not show"), "elapsed": qsTr("Elapsed"), "remaining": qsTr("Remaining"),
                                      "both": qsTr("Elapsed and remaining") })
                        }
                        Check { path: "osd.misc.percent"; text: qsTr("Percent value") }
                        Check { path: "osd.misc.frames"; text: qsTr("Number of elapsed frames") }
                        Choice {
                            label: qsTr("Local time")
                            path: "osd.misc.clock"
                            values: ["none", "minutes", "seconds"]
                            texts: ({ "none": qsTr("Do not show"), "minutes": qsTr("Hours and minutes"), "seconds": qsTr("With seconds") })
                        }
                        Check { path: "osd.misc.position"; text: qsTr("Playlist sequence number") }
                        Check { path: "osd.misc.file"; text: qsTr("Filename") }
                        Check { path: "osd.misc.chapter"; text: qsTr("Bookmark/chapter info") }
                        Check { path: "osd.misc.power"; text: qsTr("System power state (the battery)") }
                        Heading { text: qsTr("Content of Short Playback Info (Shift+Tab)") }
                        Check { path: "osd.short.file"; text: qsTr("Filename") }
                        Check { path: "osd.short.subtitle"; text: qsTr("Subtitle name") }
                        Check { path: "osd.short.time"; text: qsTr("Playback time") }
                        Check { path: "osd.short.section"; text: qsTr("Marked section (A-B)") }
                        Check { path: "osd.short.codec"; text: qsTr("Codec info") }
                    }

                    Page {
                        Heading { text: qsTr("Window size definition (relative to the desktop)") }
                        Amount { label: qsTr("Alt+1"); path: "window.desktop-sizes.0"; from: 10; to: 100; unit: "%" }
                        Amount { label: qsTr("Alt+2"); path: "window.desktop-sizes.1"; from: 10; to: 100; unit: "%" }
                        Amount { label: qsTr("Alt+3"); path: "window.desktop-sizes.2"; from: 10; to: 100; unit: "%" }
                        Amount { label: qsTr("Alt+4"); path: "window.desktop-sizes.3"; from: 10; to: 100; unit: "%" }
                        Heading { text: qsTr("Newly opened videos") }
                        Choice {
                            label: qsTr("Window size")
                            path: "window.open-size"
                            values: ["video", "last", "custom", "desktop"]
                            texts: ({ "video": qsTr("The video's size"), "last": qsTr("Keep the window's size"),
                                      "custom": qsTr("The custom size"), "desktop": qsTr("The Alt+1 size") })
                        }
                        Amount { label: qsTr("Video size times"); path: "window.open-scale"; values: [0.25, 0.5, 0.75, 1, 1.25, 1.5, 2, 3, 4]; format: v => v + "x" }
                        Amount { label: qsTr("Twice the size below a width of"); path: "window.double-below"; from: 0; to: 1920; step: 32; unit: "px"; format: v => v === 0 ? "never" : v + " px" }
                        Check { path: "window.open-first-only"; text: qsTr("Operate only with the first playback at a run") }
                        Note { text: qsTr("These apply when \"Fit the window to a newly opened video\" is on (Playback).") }
                        Heading { text: qsTr("Custom size and steps") }
                        Amount { label: qsTr("Custom width"); path: "window.custom-size.0"; from: 160; to: 7680; step: 16; unit: "px" }
                        Amount { label: qsTr("Custom height"); path: "window.custom-size.1"; from: 160; to: 4320; step: 8; unit: "px" }
                        Amount { label: qsTr("Increase and decrease"); path: "window.grow-step"; from: 1; to: 50; unit: "%" }
                        Amount { label: qsTr("Move step"); path: "window.move-step"; from: 1; to: 200; unit: "px" }
                    }

                    Page {
                        Heading { text: qsTr("When on fullscreen mode") }
                        Choice {
                            label: qsTr("Display")
                            path: "window.fullscreen-display"
                            values: ["current", "primary"]
                            texts: ({ "current": qsTr("The window's display"), "primary": qsTr("The primary display") })
                        }
                        Check { path: "window.black-other-displays"; text: qsTr("Fill other monitors black in fullscreen") }
                        Check {
                            visible: win.view.windows === true
                            path: "window.restrict-cursor"
                            text: qsTr("Restrict mouse cursor to current display")
                        }
                        Choice {
                            label: qsTr("Stay on top")
                            path: "on-top"
                            values: ["never", "always", "while-playing", "fullscreen-playing"]
                            texts: ({ "never": qsTr("Never"), "always": qsTr("Always"), "while-playing": qsTr("While playing"),
                                      "fullscreen-playing": qsTr("While playing in fullscreen") })
                        }
                        Note { text: qsTr("Fullscreen (on Another Display) is Ctrl+Shift+Enter; Alt+Enter, Ctrl+Enter and Ctrl+Alt+Enter keep the aspect ratio, stretch, or stretch keeping the aspect ratio.") }
                    }

                    Page {
                        Heading { text: qsTr("Saved frames") }
                        Field {
                            label: qsTr("Folder")
                            path: "image-folder"
                            optional: true
                            placeholder: qsTr("Pictures/RVP in the home folder")
                            Button {
                                text: qsTr("Browse…")
                                onClicked: {
                                    folderChooser.target = "image-folder"
                                    folderChooser.open()
                                }
                            }
                        }
                        Check { path: "capture.next-to-media"; text: qsTr("Store captured images into play folders (if available)") }
                        Choice {
                            label: qsTr("Format")
                            path: "image-format"
                            values: ["png", "jpeg", "bmp", "webp"]
                            texts: ({ "png": qsTr("PNG"), "jpeg": qsTr("JPEG"), "bmp": qsTr("BMP"), "webp": qsTr("WEBP") })
                        }
                        Amount { label: qsTr("JPEG and WEBP quality"); path: "capture.quality"; from: 10; to: 100; step: 5; unit: "%" }
                        Heading { text: qsTr("File names and pictures") }
                        Check { path: "capture.name-date"; text: qsTr("Include system date and time in filename") }
                        Check { path: "capture.name-elapsed"; text: qsTr("Include file elapsed time in filename") }
                        Check { path: "capture.with-subtitles"; text: qsTr("Include subtitles in source frames") }
                        Check { path: "capture.correct-aspect"; text: qsTr("Correct aspect ratio when capturing") }
                        Note { text: qsTr("Screen frames (Ctrl+Alt+E, Ctrl+Alt+C) are the picture as shown, with subtitles and mpv's messages.") }
                        Heading { text: qsTr("Consecutive images (Ctrl+G)") }
                        Amount { label: qsTr("Every"); path: "capture.interval"; values: [0.1, 0.2, 0.5, 1, 2, 3, 5, 10, 15, 30, 60, 120, 300, 600, 1800, 3600]; format: win.seconds }
                        Amount { label: qsTr("Images"); path: "capture.count"; values: [1, 2, 3, 5, 10, 20, 30, 50, 100, 200, 500, 1000, 10000] }
                        Heading { text: qsTr("Thumbnail image (Alt+N)") }
                        Amount { label: qsTr("Columns"); path: "capture.sheet.columns"; from: 1; to: 10 }
                        Amount { label: qsTr("Rows"); path: "capture.sheet.rows"; from: 1; to: 20 }
                        Amount { label: qsTr("Width"); path: "capture.sheet.width"; from: 320; to: 7680; step: 160; unit: "px" }
                        Check { path: "capture.sheet.info"; text: qsTr("File information above the pictures") }
                        Check { path: "capture.sheet.times"; text: qsTr("The time on each picture") }
                        Check { path: "capture.sheet.exact"; text: qsTr("Exact frames (slower)") }
                        Heading { text: qsTr("Recording") }
                        Field {
                            label: qsTr("Streams (Record Stream, Ctrl+A)")
                            path: "record.stream-folder"
                            optional: true
                            placeholder: qsTr("Videos/RVP in the home folder")
                            Button {
                                text: qsTr("Browse…")
                                onClicked: {
                                    folderChooser.target = "record.stream-folder"
                                    folderChooser.open()
                                }
                            }
                        }
                        Field {
                            label: qsTr("Videos (Record Video, Alt+C)")
                            path: "record.video-folder"
                            optional: true
                            placeholder: qsTr("Videos/RVP in the home folder")
                            Button {
                                text: qsTr("Browse…")
                                onClicked: {
                                    folderChooser.target = "record.video-folder"
                                    folderChooser.open()
                                }
                            }
                        }
                        Check { path: "record.next-to-media"; text: qsTr("Store video files into play folders (if available)") }
                        Note { text: qsTr("Record Stream saves the file as it comes, without decoding. Record Video encodes the stretch from its start to its stop (H.264 and AAC) with RVP's video filters, after it stops.") }
                    }

                    Page {
                        Heading { text: qsTr("Visualization settings") }
                        Check { path: "visual.enabled"; text: qsTr("Enable visualization for audio tracks") }
                        Check { path: "visual.album-art"; text: qsTr("Display album art/cover pictures") }
                        Choice {
                            label: qsTr("Visualization mode")
                            path: "visual.mode"
                            values: ["bars", "spectrum", "scope", "waves", "slideshow"]
                            texts: ({ "bars": qsTr("Spectrum bars"), "spectrum": qsTr("Spectrogram"), "scope": qsTr("Vector scope"),
                                      "waves": qsTr("Waveform"), "slideshow": qsTr("Slideshow") })
                        }
                        Amount { label: qsTr("Width"); path: "visual.size.0"; from: 320; to: 3840; step: 160; unit: "px" }
                        Amount { label: qsTr("Height"); path: "visual.size.1"; from: 180; to: 2160; step: 90; unit: "px" }
                        Amount { label: qsTr("Frames a second"); path: "visual.rate"; from: 5; to: 60; step: 5 }
                        Amount { label: qsTr("Change visualization every"); path: "visual.change-every"; values: [0, 10, 15, 20, 30, 60, 120, 300, 600]; format: v => v === 0 ? "never" : win.seconds(v) }
                        Heading { text: qsTr("Slideshow") }
                        Note { text: qsTr("Without folders, the slideshow shows the pictures of the playing file's folder.") }
                        FileList { path: "visual.folders"; ordered: true }
                        Button {
                            text: qsTr("Add a folder of pictures…")
                            onClicked: {
                                folderListChooser.target = "visual.folders"
                                folderListChooser.open()
                            }
                        }
                        Amount { label: qsTr("Display time"); path: "visual.slide-seconds"; values: [1, 2, 3, 5, 8, 10, 15, 20, 30, 60, 120, 300]; format: win.seconds }
                        Amount { label: qsTr("Transition"); path: "visual.fade-seconds"; values: [0, 0.25, 0.5, 1, 1.5, 2, 3, 5]; format: win.seconds }
                        Choice {
                            label: qsTr("Fill mode")
                            path: "visual.fill"
                            values: ["fit", "fill", "stretch"]
                            texts: ({ "fit": qsTr("Fit (keep the whole picture)"), "fill": qsTr("Fill (crop)"), "stretch": qsTr("Stretch") })
                        }
                        Check { path: "visual.random"; text: qsTr("Random slideshow") }
                        Check { path: "visual.show-name"; text: qsTr("Show filename") }
                    }

                    Page {
                        Heading { text: qsTr("Hardware decoding") }
                        Note { text: qsTr("The codecs mpv decodes in hardware when Hardware decoding (Video) is on; the others are decoded in software. mpv takes a change with the next file.") }
                        Repeater {
                            model: win.view.choices["hardware-codecs"] || []
                            delegate: CheckBox {
                                required property var modelData
                                text: modelData.text
                                checked: (win.s.decoders && win.s.decoders.hardware || []).indexOf(modelData.codec) >= 0
                                onClicked: {
                                    const list = (win.s.decoders.hardware || []).filter(c => c !== modelData.codec)
                                    if (checked)
                                        list.push(modelData.codec)
                                    win.set("decoders.hardware", list)
                                }
                            }
                        }
                        Heading { text: qsTr("Software decoding") }
                        Choice {
                            label: qsTr("AV1 decoder")
                            path: "decoders.av1"
                            values: ["dav1d", "ffmpeg"]
                            texts: ({ "dav1d": qsTr("dav1d (faster)"), "ffmpeg": qsTr("FFmpeg's own") })
                        }
                        Amount { label: qsTr("Decoder threads"); path: "decoders.threads"; from: 0; to: 32; format: v => v === 0 ? "automatic" : String(v) }
                        Note { text: qsTr("PotPlayer's DirectShow filters, splitters and their priorities have no place in RVP: libmpv decodes and splits with FFmpeg.") }
                    }

                    Page {
                        Heading { text: qsTr("General settings") }
                        Choice {
                            label: qsTr("Interface language")
                            path: "general.language"
                            values: ["auto", "en", "ru"]
                            // Each language by its own name.
                            texts: ({ "auto": qsTr("The system's"), "en": "English", "ru": "Русский" })
                        }
                        Choice {
                            label: qsTr("Stay on top")
                            path: "on-top"
                            values: ["never", "always", "while-playing", "fullscreen-playing"]
                            texts: ({ "never": qsTr("Never"), "always": qsTr("Always"), "while-playing": qsTr("While playing"),
                                      "fullscreen-playing": qsTr("While playing in fullscreen") })
                        }
                        Choice {
                            label: qsTr("When files are opened")
                            path: "open.when-opened"
                            values: ["selected", "similar", "all"]
                            texts: ({ "selected": qsTr("Add only the files opened"), "similar": qsTr("Add similar files too"),
                                      "all": qsTr("Add every file of the folder") })
                        }
                        Choice {
                            label: qsTr("Taskbar/Tray icon")
                            path: "general.tray"
                            values: ["taskbar", "both", "minimize-to-tray"]
                            texts: ({ "taskbar": qsTr("Taskbar only"), "both": qsTr("Taskbar and tray"),
                                      "minimize-to-tray": qsTr("Tray while minimized") })
                        }
                        Choice {
                            label: qsTr("Multiple instances")
                            path: "general.instances"
                            values: ["single", "multiple"]
                            texts: ({ "single": qsTr("One instance: new files go to it"), "multiple": qsTr("Each start opens its own window") })
                        }
                        Check { path: "general.cascade"; text: qsTr("Use non-overlapping multi instances") }
                        Check { path: "general.hide-windows-minimized"; text: qsTr("Hide all windows when minimized") }
                        Check { path: "general.pause-minimized"; text: qsTr("Pause playback when minimized") }
                        Check { path: "general.pause-focus-lost"; text: qsTr("Pause playback when focus lost") }
                        Check { path: "general.pause-screen-locked"; text: qsTr("Pause playback when screen locked") }
                        Check { path: "general.resize-from-center"; text: qsTr("Resize the window based on the center") }
                        Check { path: "open.recent"; text: qsTr("Store and show recent items") }
                        Check { path: "general.no-focus-at-start"; text: qsTr("Don't get focus when started with a file") }
                        Check { path: "general.drop-menu"; text: qsTr("Use advanced drag and drop interaction (dropped files ask what to do)") }
                        Check { path: "general.hide-errors-when-playing"; text: qsTr("Ignore error messages if playback starts") }
                        Check { path: "general.tooltips"; text: qsTr("Show tooltips") }
                        Check { path: "general.snap"; text: qsTr("Snap to the screen's edges") }
                        Check { path: "general.taskbar"; text: qsTr("Enable enhanced taskbar features (progress, buttons, jump list)") }
                        Check { path: "general.media-controls"; text: qsTr("Use the system's media transport controls") }
                        Check { path: "general.default-size-on-close"; text: qsTr("Default window size on closing file") }
                        Check { path: "general.run-at-startup"; text: qsTr("Automatically run at system startup") }
                        Check { path: "general.rotate-portrait-images"; text: qsTr("Auto rotate pictures based on their dimensions") }
                    }

                    Page {
                        Heading { text: qsTr("When starting") }
                        Choice {
                            label: qsTr("Window position")
                            path: "general.start.position"
                            values: ["last", "center", "custom"]
                            texts: ({ "last": qsTr("Where it was last"), "center": qsTr("Centered"), "custom": qsTr("At this position") })
                        }
                        Amount { label: qsTr("X"); path: "general.start.x"; from: 0; to: 7680; step: 10; unit: "px" }
                        Amount { label: qsTr("Y"); path: "general.start.y"; from: 0; to: 4320; step: 10; unit: "px" }
                        Choice {
                            label: qsTr("Window size")
                            path: "general.start.size"
                            values: ["last", "custom"]
                            texts: ({ "last": qsTr("As it was last"), "custom": qsTr("This size") })
                        }
                        Amount { label: qsTr("Width"); path: "general.start.width"; from: 200; to: 7680; step: 10; unit: "px" }
                        Amount { label: qsTr("Height"); path: "general.start.height"; from: 120; to: 4320; step: 10; unit: "px" }
                        Check { path: "general.start.restore-state"; text: qsTr("Restore the window to its last state (maximized or normal)") }
                        Check { path: "reopen-last-file"; text: qsTr("Play the last played item") }
                    }

                    Page {
                        Heading { text: qsTr("Dialog boxes") }
                        Field { label: qsTr("Font"); path: "general.dialog.font"; placeholder: qsTr("Qt's font") }
                        Amount { label: qsTr("Font size"); path: "general.dialog.font-size"; from: 0; to: 72; unit: "pt"; format: v => v === 0 ? "Qt's" : v + " pt" }
                        Check { path: "general.dialog.on-top"; text: qsTr("Always keep modeless dialog boxes on top") }
                        Check { path: "general.dialog.dpi-aware"; text: qsTr("Use DPI awareness (Qt scales the windows with the display; at the next start)") }
                    }

                    Page {
                        id: associationPage
                        property var types: []
                        property string result: ""
                        function refresh() {
                            types = JSON.parse(win.player.associations_json())
                        }
                        // Sets the types `pick` chooses on (`true`) or off.
                        function select(pick, on) {
                            const off = new Set(win.value("associations.off") || [])
                            for (const t of types) {
                                if (pick(t)) {
                                    if (on)
                                        off.delete(t.extension)
                                    else
                                        off.add(t.extension)
                                }
                            }
                            win.set("associations.off", Array.from(off).sort())
                            refresh()
                        }
                        Component.onCompleted: refresh()
                        onVisibleChanged: if (visible) refresh()
                        Heading { text: qsTr("Associations (for the current user)") }
                        ListView {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 260
                            clip: true
                            model: associationPage.types
                            delegate: RowLayout {
                                required property var modelData
                                width: ListView.view.width
                                CheckBox {
                                    checked: modelData.on
                                    text: "." + modelData.extension
                                    Layout.preferredWidth: 110
                                    onToggled: associationPage.select(t => t.extension === modelData.extension, checked)
                                }
                                Label {
                                    text: modelData.description + (modelData.registered ? "" : "  (not registered)")
                                    opacity: modelData.registered ? 1 : 0.6
                                    Layout.fillWidth: true
                                    elide: Text.ElideRight
                                }
                            }
                        }
                        Flow {
                            Layout.fillWidth: true
                            spacing: 6
                            Button { text: qsTr("Select Video Ext"); onClicked: associationPage.select(t => t.kind === "video", true) }
                            Button { text: qsTr("Select Audio Ext"); onClicked: associationPage.select(t => t.kind === "audio", true) }
                            Button { text: qsTr("Select Subtitle Ext"); onClicked: associationPage.select(t => t.kind === "subtitle", true) }
                            Button { text: qsTr("Select Playlist Ext"); onClicked: associationPage.select(t => t.kind === "playlist", true) }
                            Button { text: qsTr("Select All"); onClicked: associationPage.select(t => true, true) }
                            Button { text: qsTr("Deselect All"); onClicked: associationPage.select(t => true, false) }
                            Button {
                                text: qsTr("Restore to Default")
                                onClicked: {
                                    win.set("associations.off", [])
                                    win.set("associations.extra", {})
                                    win.set("associations.descriptions", {})
                                    associationPage.refresh()
                                }
                            }
                        }
                        RowLayout {
                            TextField { id: newExtension; placeholderText: qsTr("Extension"); Layout.preferredWidth: 110 }
                            ComboBox { id: newKind; model: ["video", "audio", "subtitle", "playlist"] }
                            TextField { id: newDescription; placeholderText: qsTr("Description"); Layout.fillWidth: true }
                            Button {
                                text: qsTr("Add")
                                onClicked: {
                                    const ext = newExtension.text.trim().replace(/^\./, "").toLowerCase()
                                    if (ext === "")
                                        return
                                    const extra = Object.assign({}, win.value("associations.extra") || {})
                                    extra[ext] = newKind.currentText
                                    win.set("associations.extra", extra)
                                    if (newDescription.text.trim() !== "") {
                                        const names = Object.assign({}, win.value("associations.descriptions") || {})
                                        names[ext] = newDescription.text.trim()
                                        win.set("associations.descriptions", names)
                                    }
                                    associationPage.refresh()
                                }
                            }
                            Button {
                                text: qsTr("Delete")
                                onClicked: {
                                    const ext = newExtension.text.trim().replace(/^\./, "").toLowerCase()
                                    const extra = Object.assign({}, win.value("associations.extra") || {})
                                    delete extra[ext]
                                    win.set("associations.extra", extra)
                                    associationPage.refresh()
                                }
                            }
                        }
                        Check { path: "associations.folder-menu"; text: qsTr("Add RVP to the folder context menu") }
                        Check { path: "associations.add-menu"; text: qsTr("Add \"Add to\" to the file context menu") }
                        Check { path: "associations.play-menu"; text: qsTr("Add \"Play with\" to the file context menu") }
                        Field {
                            label: qsTr("File Open dialog's extensions")
                            path: "associations.dialog-extensions"
                            placeholder: qsTr("Every file (mkv mp4 ... to limit it)")
                        }
                        Heading { text: qsTr("AutoPlay") }
                        Repeater {
                            model: [["video", "Video files"], ["audio", "Audio files"], ["cd", "Audio CD"], ["vcd", "VCD"],
                                    ["svcd", "SVCD"], ["dvd", "DVD"], ["bluray", "Blu-ray"]]
                            delegate: CheckBox {
                                required property var modelData
                                text: modelData[1]
                                checked: (win.value("associations.autoplay") || []).indexOf(modelData[0]) >= 0
                                onToggled: {
                                    const on = new Set(win.value("associations.autoplay") || [])
                                    if (checked)
                                        on.add(modelData[0])
                                    else
                                        on.delete(modelData[0])
                                    win.set("associations.autoplay", Array.from(on).sort())
                                }
                            }
                        }
                        RowLayout {
                            Button {
                                text: qsTr("Apply to System")
                                onClicked: {
                                    associationPage.result = win.player.apply_associations()
                                    associationPage.refresh()
                                }
                            }
                            Button { text: qsTr("Clear and Rebuild the Icon Cache"); onClicked: win.player.rebuild_icon_cache() }
                            Button {
                                text: qsTr("Default Programs…")
                                onClicked: Qt.openUrlExternally("ms-settings:defaultapps?registeredAppUser=RVP")
                            }
                        }
                        Note { text: associationPage.result }
                        Note { text: qsTr("RVP registers itself for these types and never makes itself their default: Windows' Default apps chooses that. RVP's files show RVP's icon.") }
                    }

                    Page {
                        Heading { text: qsTr("Screen saver") }
                        Field {
                            label: qsTr("Folder or file to play")
                            path: "screensaver.media"
                            placeholder: qsTr("RVP's logo")
                            Button {
                                text: qsTr("Browse…")
                                onClicked: {
                                    folderChooser.target = "screensaver.media"
                                    folderChooser.open()
                                }
                            }
                        }
                        Check { path: "screensaver.shuffle"; text: qsTr("Shuffle") }
                        Check { path: "screensaver.mute"; text: qsTr("Without sound") }
                        RowLayout {
                            Button {
                                text: qsTr("Set up…")
                                onClicked: screenSaverResult.text = win.player.use_as_screensaver()
                            }
                            Button { text: qsTr("Preview"); onClicked: Tools.run_self(["--screensaver"]) }
                        }
                        Note { id: screenSaverResult; text: "" }
                        Note { text: qsTr("Set up makes RVP (rvp.scr beside rvp.exe) the current user's screen saver and opens Windows' screen saver settings for its wait. Windows' small preview there stays empty: RVP draws in its own window only.") }
                    }

                    Page {
                        id: presetPage
                        // The selected preset: 0 the default, then the list's.
                        property int selected: 0
                        readonly property var list: win.value("presets.list") || []
                        readonly property var preset: selected > 0 && selected <= list.length ? list[selected - 1] : null
                        readonly property var conditions: preset ? (preset.conditions || {}) : ({})
                        function store(list) { win.set("presets.list", list) }
                        function copyList() { return JSON.parse(JSON.stringify(list)) }
                        // Sets one condition of the selected preset (null: off).
                        function condition(name, value) {
                            if (selected === 0)
                                return
                            const copy = copyList()
                            copy[selected - 1].conditions = copy[selected - 1].conditions || {}
                            copy[selected - 1].conditions[name] = value
                            store(copy)
                        }
                        Heading { text: qsTr("Configuration management") }
                        ListView {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 140
                            clip: true
                            model: [""].concat(presetPage.list.map(preset => preset.name))
                            delegate: ItemDelegate {
                                required property var modelData
                                required property int index
                                width: ListView.view.width
                                highlighted: index === presetPage.selected
                                text: (modelData === "" ? "Default" : modelData)
                                      + ((win.value("presets.active") || "") === modelData ? "  (active)" : "")
                                onClicked: {
                                    presetPage.selected = index
                                    presetName.text = modelData
                                }
                            }
                        }
                        RowLayout {
                            TextField { id: presetName; placeholderText: qsTr("Name"); Layout.fillWidth: true }
                            Button {
                                text: qsTr("Add")
                                onClicked: {
                                    const copy = presetPage.copyList()
                                    let name = presetName.text.trim()
                                    if (name === "" || copy.some(p => p.name === name))
                                        name = "Preset " + (copy.length + 1)
                                    copy.push({ name: name, conditions: {}, values: null })
                                    presetPage.store(copy)
                                    presetPage.selected = copy.length
                                }
                            }
                            Button {
                                text: qsTr("Duplicate")
                                enabled: presetPage.selected > 0
                                onClicked: {
                                    const copy = presetPage.copyList()
                                    const source = JSON.parse(JSON.stringify(copy[presetPage.selected - 1]))
                                    source.name = source.name + " (copy)"
                                    copy.push(source)
                                    presetPage.store(copy)
                                }
                            }
                            Button {
                                text: qsTr("Activate")
                                onClicked: win.player.run_menu_command("p:" + presetPage.selected)
                            }
                        }
                        RowLayout {
                            Button {
                                text: qsTr("Up")
                                enabled: presetPage.selected > 1
                                onClicked: {
                                    const copy = presetPage.copyList()
                                    const i = presetPage.selected - 1
                                    copy.splice(i - 1, 0, copy.splice(i, 1)[0])
                                    presetPage.store(copy)
                                    presetPage.selected -= 1
                                }
                            }
                            Button {
                                text: qsTr("Down")
                                enabled: presetPage.selected > 0 && presetPage.selected < presetPage.list.length
                                onClicked: {
                                    const copy = presetPage.copyList()
                                    const i = presetPage.selected - 1
                                    copy.splice(i + 1, 0, copy.splice(i, 1)[0])
                                    presetPage.store(copy)
                                    presetPage.selected += 1
                                }
                            }
                            Button {
                                text: qsTr("Rename")
                                enabled: presetPage.selected > 0
                                onClicked: {
                                    const name = presetName.text.trim()
                                    const copy = presetPage.copyList()
                                    if (name === "" || copy.some(p => p.name === name))
                                        return
                                    const old = copy[presetPage.selected - 1].name
                                    copy[presetPage.selected - 1].name = name
                                    presetPage.store(copy)
                                    if ((win.value("presets.active") || "") === old)
                                        win.set("presets.active", name)
                                }
                            }
                            Button {
                                text: qsTr("Delete")
                                enabled: presetPage.selected > 0
                                onClicked: {
                                    const copy = presetPage.copyList()
                                    if ((win.value("presets.active") || "") === copy[presetPage.selected - 1].name)
                                        win.player.run_menu_command("p:0")
                                    copy.splice(presetPage.selected - 1, 1)
                                    presetPage.store(copy)
                                    presetPage.selected = 0
                                }
                            }
                        }
                        Check { path: "presets.reopen"; text: qsTr("Reopen current item when preset is manually activated") }
                        Check { path: "presets.copy-to-default"; text: qsTr("Copy preset values into default preset when preset is manually activated") }
                        Check { path: "presets.return-at-end"; text: qsTr("Return to previous preset at playback end when preset is auto activated") }
                        Check { path: "presets.default-at-start"; text: qsTr("Activate default preset at program start") }
                        Heading { text: qsTr("Auto-Preset (the selected preset activates for files that meet every condition)") }
                        Repeater {
                            model: [["file-name", "File name (* and ?)", "text"], ["album", "Album name", "text"],
                                    ["power", "Power source (ac or battery)", "text"], ["duration", "File duration (sec)", "number"],
                                    ["width", "Video width", "number"], ["height", "Video height", "number"],
                                    ["fourcc", "Video codec (FourCC)", "text"], ["fps", "Video FPS", "number"],
                                    ["aspect", "Video aspect ratio", "number"], ["audio-format", "Audio format", "text"],
                                    ["sample-rate", "Audio sample rate", "number"], ["channels", "Audio channels", "number"]]
                            delegate: RowLayout {
                                id: conditionRow
                                required property var modelData
                                readonly property var current: presetPage.conditions[modelData[0]]
                                enabled: presetPage.selected > 0
                                CheckBox {
                                    text: conditionRow.modelData[1]
                                    checked: conditionRow.current !== undefined && conditionRow.current !== null
                                    Layout.preferredWidth: 240
                                    onToggled: presetPage.condition(conditionRow.modelData[0], !checked ? null
                                        : conditionRow.modelData[2] === "number" ? { op: "eq", value: Number(valueField.text) || 0 }
                                        : valueField.text)
                                }
                                ComboBox {
                                    id: opBox
                                    visible: conditionRow.modelData[2] === "number"
                                    model: ["eq", "ne", "lt", "le", "gt", "ge"]
                                    displayText: ({ eq: "=", ne: "≠", lt: "<", le: "≤", gt: ">", ge: "≥" })[currentText]
                                    currentIndex: conditionRow.current && conditionRow.current.op ? model.indexOf(conditionRow.current.op) : 0
                                    Layout.preferredWidth: 70
                                    onActivated: if (conditionRow.current)
                                        presetPage.condition(conditionRow.modelData[0], { op: currentText, value: conditionRow.current.value })
                                }
                                TextField {
                                    id: valueField
                                    Layout.fillWidth: true
                                    text: conditionRow.current === undefined || conditionRow.current === null ? ""
                                        : conditionRow.modelData[2] === "number" ? String(conditionRow.current.value) : conditionRow.current
                                    onEditingFinished: if (conditionRow.current !== undefined && conditionRow.current !== null)
                                        presetPage.condition(conditionRow.modelData[0], conditionRow.modelData[2] === "number"
                                            ? { op: opBox.currentText, value: Number(text) || 0 } : text)
                                }
                            }
                        }
                        CheckBox {
                            text: qsTr("If audio playback")
                            enabled: presetPage.selected > 0
                            checked: presetPage.conditions["audio-only"] === true
                            onToggled: presetPage.condition("audio-only", checked)
                        }
                        CheckBox {
                            text: qsTr("If subtitles exist")
                            enabled: presetPage.selected > 0
                            checked: presetPage.conditions["subtitles"] === true
                            onToggled: presetPage.condition("subtitles", checked)
                        }
                    }

                    Page {
                        id: ttsPage
                        property var voices: []
                        property var outputs: []
                        onVisibleChanged: if (visible) {
                            voices = JSON.parse(win.player.tts_voices_json())
                            outputs = JSON.parse(win.player.tts_outputs_json())
                        }
                        Heading { text: qsTr("TTS (Text to Speech) setup") }
                        Check { path: "accessibility.tts.messages"; text: qsTr("Read the messages displayed on the screen") }
                        Check { path: "accessibility.tts.subtitles"; text: qsTr("Read the movie subtitles comprised of text") }
                        Check { path: "accessibility.tts.tooltips"; text: qsTr("Read the tooltips displayed on mouse rollover of skin items") }
                        Choice {
                            label: qsTr("Output device")
                            path: "accessibility.output"
                            values: [""].concat(ttsPage.outputs)
                            texts: ({ "": qsTr("Windows' default") })
                        }
                        Choice {
                            label: qsTr("TTS engine (voice)")
                            path: "accessibility.voice"
                            values: [""].concat(ttsPage.voices)
                            texts: ({ "": qsTr("Windows' default") })
                        }
                        Amount { label: qsTr("Speed"); path: "accessibility.rate"; from: -10; to: 10 }
                        Amount { label: qsTr("Volume"); path: "accessibility.volume"; from: 0; to: 100; unit: "%" }
                        RowLayout {
                            TextField { id: ttsTest; text: qsTr("RVP reads this aloud."); Layout.fillWidth: true }
                            Button { text: qsTr("Test"); onClicked: win.player.speak(ttsTest.text) }
                        }
                        Note { text: qsTr("Windows' speech (SAPI) reads; on Linux there are no voices yet.") }
                        Heading { text: qsTr("UIA (User Interface Automation) setup for screen reader programs") }
                        Check { path: "accessibility.screen-reader.messages"; text: qsTr("Read the messages displayed on the screen") }
                        Check { path: "accessibility.screen-reader.subtitles"; text: qsTr("Read the movie subtitles comprised of text") }
                        Check { path: "accessibility.screen-reader.tooltips"; text: qsTr("Read the tooltips displayed on mouse rollover of skin items") }
                        Heading { text: qsTr("Window title setup") }
                        Check { path: "accessibility.title.messages"; text: qsTr("Output the messages displayed on the screen as window titles") }
                        Check { path: "accessibility.title.subtitles"; text: qsTr("Output the movie subtitles comprised of text as window titles") }
                        Check { path: "accessibility.title.tooltips"; text: qsTr("Output the tooltips displayed on mouse rollover of skin items as window titles") }
                    }

                    Page {
                        Heading { text: qsTr("Touch settings") }
                        Check { path: "accessibility.touch"; text: qsTr("Enable the touch feature") }
                        Choice {
                            label: qsTr("Single finger swipe action")
                            path: "accessibility.swipe"
                            values: ["seek-volume", "seek", "volume", "none"]
                            texts: ({ "seek-volume": qsTr("Sideways seeks, up and down changes the volume"), "seek": qsTr("Seek"),
                                      "volume": qsTr("Volume"), "none": qsTr("Nothing") })
                        }
                        Check { path: "accessibility.touch-skin"; text: qsTr("Use the skin's touch layout (skins that have one)") }
                    }

                    Page {
                        Heading { text: qsTr("XInput (Xbox controller) settings") }
                        Check { path: "accessibility.xinput"; text: qsTr("Enable XInput") }
                        Check { path: "accessibility.xinput-focus-only"; text: qsTr("Operate only when RVP has the focus") }
                        Choice {
                            label: qsTr("Mouse emulation")
                            path: "accessibility.xinput-mouse"
                            values: ["off", "left", "right"]
                            texts: ({ "off": qsTr("Off"), "left": qsTr("Left stick (A clicks)"), "right": qsTr("Right stick (A clicks)") })
                        }
                        Note { text: qsTr("A plays and pauses, B fullscreen, X mute, Y playback info, the shoulders previous and next, Back closes playback, Start plays and pauses, the pad's up and down the volume and left and right seek. Windows only; Linux's controllers come later.") }
                    }

                    Page {
                        id: remotePage
                        // The control API's state (Player.control_info) and
                        // its token while shown.
                        property var info: ({})
                        property string token: ""
                        // What Add to Unified Remote did (Linux).
                        property string added: ""
                        function refresh() { info = JSON.parse(win.player.control_info()) }
                        Timer {
                            interval: 1000
                            repeat: true
                            triggeredOnStart: true
                            running: win.visible && win.currentKey === "remote"
                            onTriggered: remotePage.refresh()
                        }
                        Heading { text: qsTr("Remote control") }
                        Check { path: "remote.enabled"; text: qsTr("Let remotes control RVP (its control API on this computer)") }
                        RowLayout {
                            Label {
                                text: qsTr("Port")
                                wrapMode: Text.WordWrap
                                Layout.preferredWidth: 240
                            }
                            TextField {
                                id: remotePort
                                objectName: "remote-port"
                                Layout.preferredWidth: 120
                                // A port is an address, not an amount: typed.
                                validator: IntValidator { bottom: 1024; top: 65535 }
                                text: String(win.value("remote.port") || "")
                                onEditingFinished: {
                                    if (acceptableInput && Number(text) !== win.value("remote.port"))
                                        win.set("remote.port", Number(text))
                                }
                            }
                        }
                        RowLayout {
                            Label {
                                text: qsTr("Address")
                                wrapMode: Text.WordWrap
                                Layout.preferredWidth: 240
                            }
                            Label {
                                text: remotePage.info.address || ""
                                textFormat: Text.PlainText
                            }
                        }
                        RowLayout {
                            Label {
                                text: qsTr("Token")
                                wrapMode: Text.WordWrap
                                Layout.preferredWidth: 240
                            }
                            TextField {
                                Layout.fillWidth: true
                                readOnly: true
                                text: remotePage.token
                                placeholderText: qsTr("Hidden")
                                font.family: "monospace"
                            }
                            Button {
                                text: remotePage.token === "" ? qsTr("Show") : qsTr("Hide")
                                onClicked: remotePage.token = remotePage.token === "" ? win.player.control_token() : ""
                            }
                            Button {
                                text: qsTr("Copy")
                                onClicked: Tools.set_clipboard_text(win.player.control_token())
                            }
                            Button {
                                text: qsTr("Renew")
                                onClicked: {
                                    const renewed = win.player.renew_control_token()
                                    remotePage.token = remotePage.token === "" ? "" : renewed
                                    remotePage.refresh()
                                }
                            }
                        }
                        Note {
                            objectName: "remote-state"
                            text: remotePage.info.error
                                  ? qsTr("Cannot listen: %1").arg(remotePage.info.error)
                                  : remotePage.info.listening ? qsTr("Listening on this computer only (127.0.0.1).")
                                                              : qsTr("Off.")
                        }
                        Note {
                            visible: Qt.platform.os === "windows"
                            text: qsTr("Remotes on a phone reach RVP through a program on this computer, such as Unified Remote's server: give its RVP remote the port and the token. RVP's remote for Unified Remote is in the “remote” folder beside rvp.exe; the installer copies it into Unified Remote when that is installed. Renew makes a new token: remotes with the old one are refused.")
                        }
                        Note {
                            visible: Qt.platform.os !== "windows"
                            text: qsTr("Remotes on a phone reach RVP through a program on this computer, such as Unified Remote's server: give its RVP remote the port and the token. Add to Unified Remote copies RVP's remote into the server's custom remotes. Renew makes a new token: remotes with the old one are refused.")
                        }
                        RowLayout {
                            visible: Qt.platform.os !== "windows"
                            Button {
                                text: qsTr("Add to Unified Remote")
                                onClicked: remotePage.added = win.player.add_unified_remote()
                            }
                            Label {
                                Layout.fillWidth: true
                                text: remotePage.added
                                wrapMode: Text.Wrap
                                textFormat: Text.PlainText
                            }
                        }
                    }

                    Page {
                        id: logPage
                        // The log's last lines, read again each second
                        // while the page shows.
                        property string recent: ""
                        Timer {
                            interval: 1000
                            repeat: true
                            triggeredOnStart: true
                            running: win.visible && win.currentKey === "log"
                            onTriggered: logPage.recent = win.player.log_recent()
                        }
                        Heading { text: qsTr("Log") }
                        Choice {
                            label: qsTr("Level")
                            path: "log.level"
                            values: win.view.choices["log.level"] || ["off"]
                            texts: ({ "off": qsTr("Off"), "critical": qsTr("Critical: errors"),
                                      "warning": qsTr("Warning: errors and problems worked around"),
                                      "info": qsTr("Info: also files, settings, smoothing, dropped frames"),
                                      "debug": qsTr("Debug: also each second of playback, mpv's details") })
                        }
                        // Empty: the state folder; the file's path shows
                        // below.
                        Field {
                            // A path: the whole row, its buttons below.
                            Layout.fillWidth: true
                            label: qsTr("Folder")
                            path: "log.folder"
                            optional: true
                        }
                        RowLayout {
                            Note {
                                textFormat: Text.PlainText
                                text: qsTr("File: %1").arg(win.view.log_file || "")
                            }
                            Button {
                                text: qsTr("Browse…")
                                onClicked: {
                                    folderChooser.target = "log.folder"
                                    folderChooser.open()
                                }
                            }
                            Button {
                                text: qsTr("Open Folder")
                                onClicked: Qt.openUrlExternally(win.player.settings_folder_url("log"))
                            }
                        }
                        Note {
                            text: qsTr("The log keeps what RVP and mpv report, with the time. An empty folder is RVP's state folder; a new one takes effect at once. At 16 MB the file moves to rvp.old.log beside it. Debug adds a line for each second of playback: the frame rates, dropped and late frames, the frames drawn and the processors' load.")
                        }
                        Heading { text: qsTr("Last lines, newest first") }
                        ScrollView {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 300
                            TextArea {
                                objectName: "log-recent"
                                readOnly: true
                                wrapMode: TextEdit.Wrap
                                textFormat: TextEdit.PlainText
                                font.family: "monospace"
                                text: logPage.recent.split("\n").slice(-200).reverse().join("\n")
                                placeholderText: qsTr("Nothing logged yet.")
                            }
                        }
                    }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Label {
                Layout.fillWidth: true
                text: win.notice + (win.restartNeeded ? (win.notice !== "" ? "  " : "") + qsTr("Restart RVP to apply it.") : "")
                wrapMode: Text.Wrap
            }
            Button {
                visible: win.resetPending
                text: qsTr("Reset")
                onClicked: win.resetPage()
            }
            Button {
                visible: win.resetPending
                text: qsTr("Keep")
                onClicked: {
                    win.resetPending = false
                    win.notice = qsTr("Unchanged.")
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Label {
                Layout.fillWidth: true
                elide: Text.ElideMiddle
                opacity: 0.7
                text: (win.view.portable ? qsTr("Portable: %1") : "%1").arg(win.view.file || "")
            }
            Button {
                objectName: "resetPage"
                text: qsTr("Reset Page")
                enabled: win.visible && win.pagePaths(win.current).length > 0
                onClicked: {
                    win.resetPending = true
                    win.notice = qsTr("Put the options of “%1” back to RVP's defaults?").arg(win.pages[win.current].title)
                }
            }
            Button {
                text: qsTr("Import…")
                onClicked: importChooser.open()
            }
            Button {
                text: qsTr("Export…")
                onClicked: exportChooser.open()
            }
            Button {
                text: qsTr("Close")
                onClicked: win.close()
            }
        }
    }
}
