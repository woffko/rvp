// The settings window (F5, PotPlayer's Preferences), loaded on first use.
// It shows RVP's own options from settings.json (rvp_app::preferences):
// every change goes through Player.set_setting, which checks it, applies
// it to the running player and saves the file. Only the skin waits for the
// next start.
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
        { key: "video", title: "Video" },
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
    component Number: RowLayout {
        id: number
        property string label
        property string path
        property string unit
        // Shown value = stored value * factor (percent of a scale).
        property real factor: 1
        Label {
            text: number.label
            Layout.preferredWidth: 240
        }
        TextField {
            Layout.preferredWidth: 90
            text: {
                const v = win.value(number.path)
                return v === undefined ? "" : String(Math.round(v * number.factor * 1000) / 1000)
            }
            validator: DoubleValidator {
                locale: "C"
                notation: DoubleValidator.StandardNotation
            }
            onEditingFinished: {
                const v = Number(text) / number.factor
                if (text !== "" && !isNaN(v) && v !== win.value(number.path))
                    win.set(number.path, v)
            }
        }
        Label {
            text: number.unit
            opacity: 0.7
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
                    Number { label: "Left / Right"; path: "seek-seconds.0"; unit: "s" }
                    Number { label: "Ctrl+Left / Right"; path: "seek-seconds.1"; unit: "s" }
                    Number { label: "Shift+Left / Right"; path: "seek-seconds.2"; unit: "s" }
                    Number { label: "Ctrl+Alt+Left / Right"; path: "seek-seconds.3"; unit: "s" }
                    Check { path: "keyframe-seeks"; text: "Seek to keyframes (faster, less exact)" }
                    Heading { text: "Resume" }
                    Check { path: "remember-positions"; text: "Resume every file at its last position" }
                    Check { path: "reopen-last-file"; text: "Reopen the last file, paused, when RVP starts without files" }
                    Heading { text: "Window" }
                    Check { path: "fit-window-to-video"; text: "Fit the window to a newly opened video" }
                    Check { path: "keep-aspect-window"; text: "Keep the video's shape while the window is resized" }
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
                    Heading { text: "Normalizer" }
                    Check { path: "normalizer.enabled"; text: "Normalize the volume (Shift+N)" }
                    Check { path: "normalizer.boost"; text: "Boost audio (raise quiet passages)" }
                    Number { label: "Level"; path: "normalizer.level"; unit: "% (10-100)" }
                    Number { label: "Release time"; path: "normalizer.release"; unit: "(1-50)" }
                    Heading { text: "Volume" }
                    Number { label: "Step (Up, Down, wheel)"; path: "volume-step"; unit: "%" }
                    Number { label: "Maximum (above 100 amplifies)"; path: "volume-max"; unit: "%" }
                    Heading { text: "Output" }
                    Choice {
                        label: "Audio device"
                        path: "audio-device"
                        values: win.withCurrent((win.view.choices["audio-device"] || []).map(d => d.name),
                                                win.s["audio-device"])
                        texts: {
                            const texts = {}
                            for (const d of win.view.choices["audio-device"] || [])
                                texts[d.name] = d.description
                            return texts
                        }
                    }
                    Number { label: "Audio sync step (Shift+, and Shift+.)"; path: "audio-delay-step"; unit: "s" }
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
                    Heading { text: "SVP" }
                    Field {
                        label: "IPC endpoint SVP attaches to"
                        path: "ipc-server"
                        placeholder: "empty: SVP cannot attach"
                        Button {
                            text: "Default"
                            onClicked: win.set("ipc-server", win.view["default-ipc"])
                        }
                    }
                    Note {
                        text: "SVP finds a player by this pipe or socket while it plays. SVP's profiles, VapourSynth scripts, shaders and the rest of video processing come in M6."
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
                    Number { label: "Size"; path: "subtitle-scale"; unit: "%"; factor: 100 }
                    Number { label: "Sync step (< and >)"; path: "subtitle-delay-step"; unit: "s" }
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
                        placeholder: win.view.windows ? "PotPlayer's BlackSpace" : "not set"
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
                    Note { text: "RVP draws PotPlayer's .dsf skins (BlackSpace is the one it is made for). The skin changes at the next start." }
                }
            }
        }

        Label {
            Layout.fillWidth: true
            text: win.notice + (win.restartNeeded ? (win.notice !== "" ? "  " : "") + "Restart RVP for the new skin." : "")
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
