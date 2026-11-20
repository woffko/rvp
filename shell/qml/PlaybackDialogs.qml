// M11's dialogs, loaded on first use (Qt Quick Controls and Dialogs are not
// needed to show the window): Jump to Time/Frame, a bookmark's title, the
// bookmark list, the A-B repeat list, the shutdown timer, the Recycle Bin's
// confirmation, the playlist's dialogs (titles, renaming, search, URLs,
// file information, playlist files, moving and copying files), capture
// devices, discs, external audio and favorites. Player.dialog holds the
// open request's data as JSON; the answers go to Player's invokables.
import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs as Dialogs
import QtQuick.Layouts
import Rvp
import RvpNative

Item {
    id: holder
    required property Player player
    property var data: ({})
    // The open dialog (self-test).
    property var current: null

    function show(json) {
        data = JSON.parse(json)
        switch (data.kind) {
        case "jump":
            jumpField.text = holder.clock(data.time)
            jumpFrames.checked = false
            jumpError.text = ""
            open(jumpDialog)
            jumpField.selectAll()
            jumpField.forceActiveFocus()
            break
        case "bookmark-title":
            titleField.text = data.title
            open(titleDialog)
            titleField.selectAll()
            titleField.forceActiveFocus()
            break
        case "bookmarks":
            reloadBookmarks()
            open(bookmarksDialog)
            break
        case "sections":
            reloadSections()
            open(sectionsDialog)
            break
        case "power-timer":
            timerAt.checked = data.timer !== "in"
            timerIn.checked = data.timer === "in"
            timerExit.checked = false
            timerField.text = data.timer === "in" ? "1:00" : Qt.formatTime(new Date(Date.now() + 3600000), "hh:mm")
            timerError.text = ""
            open(timerDialog)
            break
        case "recycle":
            open(recycleDialog)
            break
        case "list":
            showList()
            break
        case "devices":
            deviceField.text = data.devices.length > 0 ? data.devices[0].name : ""
            open(deviceDialog)
            break
        case "disc":
            if (data.files) {
                discFileDialog.nameFilters = data.bluray
                    ? [qsTr("Blu-ray playlists (%1)").arg("*.mpls"), qsTr("All files (*)")]
                    : [qsTr("DVD files (%1)").arg("*.IFO"), qsTr("All files (*)")]
                discFileDialog.title = data.bluray ? qsTr("Open Blu-ray files (%1)").arg("*.MPLS")
                                                   : qsTr("Open DVD files (%1)").arg("*.IFO")
                discFileDialog.open()
            } else {
                discFolderDialog.title = data.bluray ? qsTr("Open Blu-ray (the disc's drive or folder)")
                                                     : qsTr("Open DVD (the disc's drive or VIDEO_TS folder)")
                discFolderDialog.open()
            }
            break
        case "audio":
            audioDialog.open()
            break
        case "favorites":
            favorites.model = JSON.parse(player.favorites_json())
            open(favoritesDialog)
            break
        }
    }
    function open(dialog) {
        current = dialog
        dialog.open()
    }
    // `1:02:03.5` for the jump dialog.
    function clock(seconds) {
        const total = Math.max(0, seconds)
        const h = Math.floor(total / 3600)
        const m = Math.floor(total / 60) % 60
        const s = total - h * 3600 - m * 60
        const sec = s.toFixed(1)
        return h + ":" + (m < 10 ? "0" : "") + m + ":" + (s < 10 ? "0" : "") + sec
    }

    // Jump to Specified Time/Frame (G).
    Dialog {
        // Over the Direct3D 11 renderer's native video window only a
        // window of its own shows (D4).
        popupType: Tools.native_video ? Popup.Window : Popup.Item
        id: jumpDialog
        objectName: "jumpDialog"
        title: qsTr("Jump to Specified Time/Frame")
        modal: true
        anchors.centerIn: parent
        standardButtons: Dialog.Ok | Dialog.Cancel
        onAccepted: {
            const error = holder.player.jump_to(jumpField.text, jumpFrames.checked)
            if (error !== "") {
                jumpError.text = error
                open()
            }
        }
        contentItem: ColumnLayout {
            RowLayout {
                RadioButton {
                    id: jumpTime
                    text: qsTr("Time (h:mm:ss.s, or +/- seconds)")
                    checked: true
                    onToggled: jumpField.text = holder.clock(holder.data.time || 0)
                }
                RadioButton {
                    id: jumpFrames
                    text: qsTr("Frame")
                    enabled: (holder.data.fps || 0) > 0
                    onToggled: jumpField.text = String(Math.floor((holder.data.time || 0) * holder.data.fps))
                }
            }
            TextField {
                id: jumpField
                objectName: "jumpField"
                Layout.preferredWidth: 320
                onAccepted: jumpDialog.accept()
            }
            Label {
                textFormat: Text.PlainText
                text: qsTr("Duration: %1").arg(holder.clock(holder.data.duration || 0))
                      + ((holder.data.fps || 0) > 0 ? "  (" + holder.data.fps.toFixed(3) + " fps)" : "")
                opacity: 0.7
            }
            Label {
                id: jumpError
                textFormat: Text.PlainText
                color: "#c03030"
                visible: text !== ""
            }
        }
    }

    // A new bookmark's title (Prompt before adding a bookmark).
    Dialog {
        popupType: Tools.native_video ? Popup.Window : Popup.Item
        id: titleDialog
        title: qsTr("Add Bookmark")
        modal: true
        anchors.centerIn: parent
        standardButtons: Dialog.Ok | Dialog.Cancel
        onAccepted: holder.player.add_bookmark(holder.data.time, titleField.text)
        contentItem: ColumnLayout {
            Label { text: qsTr("Title:") }
            TextField {
                id: titleField
                Layout.preferredWidth: 320
                onAccepted: titleDialog.accept()
            }
        }
    }

    // A list of the playing file's bookmarks or A-B sections with a
    // title to edit; `edit(op, index, text)` changes them.
    component MarkList: ColumnLayout {
        id: markList
        property var items: []
        property alias index: list.currentIndex
        property alias title: markTitle.text
        property var label: (item) => item.title
        Layout.preferredWidth: 460
        ListView {
            id: list
            Layout.fillWidth: true
            Layout.preferredHeight: 220
            clip: true
            model: markList.items
            highlight: Rectangle { color: "#30608cff" }
            delegate: ItemDelegate {
                id: markRow
                required property var modelData
                required property int index
                width: ListView.view.width
                text: markList.label(modelData)
                // The title as written: Qt's own label would take markup in
                // it, and "&" for a mnemonic.
                contentItem: Label {
                    textFormat: Text.PlainText
                    text: markRow.text
                    font: markRow.font
                    elide: Text.ElideRight
                    verticalAlignment: Text.AlignVCenter
                }
                onClicked: {
                    list.currentIndex = index
                    markTitle.text = modelData.title
                }
            }
        }
        TextField {
            id: markTitle
            Layout.fillWidth: true
            placeholderText: qsTr("Title")
        }
    }

    function reloadBookmarks() {
        bookmarkList.items = JSON.parse(player.bookmarks_json())
    }
    Dialog {
        popupType: Tools.native_video ? Popup.Window : Popup.Item
        id: bookmarksDialog
        objectName: "bookmarksDialog"
        title: qsTr("Bookmarks")
        modal: false
        anchors.centerIn: parent
        standardButtons: Dialog.Close
        contentItem: ColumnLayout {
            MarkList {
                id: bookmarkList
                label: (item) => holder.clock(item.time) + "   " + item.title
            }
            RowLayout {
                Button {
                    text: qsTr("Add Here")
                    onClicked: { holder.player.bookmark_edit("add", -1, ""); holder.reloadBookmarks() }
                }
                Button {
                    text: qsTr("Jump")
                    enabled: bookmarkList.index >= 0
                    onClicked: holder.player.bookmark_edit("jump", bookmarkList.index, "")
                }
                Button {
                    text: qsTr("Rename")
                    enabled: bookmarkList.index >= 0
                    onClicked: {
                        holder.player.bookmark_edit("rename", bookmarkList.index, bookmarkList.title)
                        holder.reloadBookmarks()
                    }
                }
                Button {
                    text: qsTr("Delete")
                    enabled: bookmarkList.index >= 0
                    onClicked: {
                        holder.player.bookmark_edit("delete", bookmarkList.index, "")
                        holder.reloadBookmarks()
                    }
                }
            }
        }
    }

    function reloadSections() {
        sectionList.items = JSON.parse(player.sections_json())
    }
    Dialog {
        popupType: Tools.native_video ? Popup.Window : Popup.Item
        id: sectionsDialog
        objectName: "sectionsDialog"
        title: qsTr("A-B Repeat List")
        modal: false
        anchors.centerIn: parent
        standardButtons: Dialog.Close
        contentItem: ColumnLayout {
            MarkList {
                id: sectionList
                label: (item) => holder.clock(item.start) + " - " + holder.clock(item.end) + "   " + item.title
            }
            RowLayout {
                Button {
                    text: qsTr("Add A-B")
                    onClicked: {
                        holder.player.section_edit("add", -1, sectionList.title)
                        holder.reloadSections()
                    }
                }
                Button {
                    text: qsTr("Repeat")
                    enabled: sectionList.index >= 0
                    onClicked: holder.player.section_edit("repeat", sectionList.index, "")
                }
                Button {
                    text: qsTr("Rename")
                    enabled: sectionList.index >= 0
                    onClicked: {
                        holder.player.section_edit("rename", sectionList.index, sectionList.title)
                        holder.reloadSections()
                    }
                }
                Button {
                    text: qsTr("Delete")
                    enabled: sectionList.index >= 0
                    onClicked: {
                        holder.player.section_edit("delete", sectionList.index, "")
                        holder.reloadSections()
                    }
                }
            }
        }
    }

    // The shutdown timer (Misc > Shutdown at/in, Add a task to turn off PC
    // or exit RVP). The local time is QML's.
    Dialog {
        popupType: Tools.native_video ? Popup.Window : Popup.Item
        id: timerDialog
        objectName: "timerDialog"
        title: qsTr("Shutdown Timer")
        modal: true
        anchors.centerIn: parent
        standardButtons: Dialog.Ok | Dialog.Cancel
        onAccepted: {
            const parts = timerField.text.trim().split(":").map(Number)
            if (parts.length < 1 || parts.length > 3 || parts.some(isNaN)) {
                timerError.text = qsTr("Write the time as hh:mm")
                open()
                return
            }
            let seconds = 0
            let label = ""
            if (timerAt.checked) {
                const target = new Date()
                target.setHours(parts[0], parts.length > 1 ? parts[1] : 0, 0, 0)
                if (target.getTime() <= Date.now())
                    target.setDate(target.getDate() + 1)
                seconds = (target.getTime() - Date.now()) / 1000
                label = Qt.formatTime(target, "hh:mm")
            } else {
                for (const part of parts)
                    seconds = seconds * 60 + part
                // "1:30" is an hour and a half; "45" is minutes.
                seconds *= parts.length === 3 ? 1 : 60
                // The menu names the time h:mm (h:mm:ss), the forms its
                // translations have, however it was written ("90" too).
                const whole = Math.round(seconds)
                const pad = n => String(n).padStart(2, "0")
                label = Math.floor(whole / 3600) + ":" + pad(Math.floor(whole % 3600 / 60))
                        + (whole % 60 ? ":" + pad(whole % 60) : "")
            }
            holder.player.set_power_timer(timerAt.checked ? 0 : 1, seconds, timerExit.checked, label)
        }
        contentItem: ColumnLayout {
            RowLayout {
                RadioButton { id: timerAt; text: qsTr("At (hh:mm)"); checked: true }
                RadioButton { id: timerIn; text: qsTr("In (h:mm, or minutes)") }
            }
            TextField {
                id: timerField
                Layout.preferredWidth: 200
                onAccepted: timerDialog.accept()
            }
            CheckBox {
                id: timerExit
                text: qsTr("Only exit RVP (otherwise the computer does the When Playback Finished action)")
            }
            Label {
                id: timerError
                textFormat: Text.PlainText
                color: "#c03030"
                visible: text !== ""
            }
        }
    }

    // The Recycle Bin's confirmation.
    Dialog {
        popupType: Tools.native_video ? Popup.Window : Popup.Item
        id: recycleDialog
        objectName: "recycleDialog"
        title: qsTr("Send to the Recycle Bin")
        modal: true
        anchors.centerIn: parent
        standardButtons: Dialog.Yes | Dialog.No
        onAccepted: {
            const error = holder.player.recycle(JSON.stringify(holder.data.files || []),
                                                JSON.stringify(holder.data.entries || []))
            if (error !== "")
                holder.player.message = error
        }
        // A child, not the contentItem: Fusion's titled Dialog loops on
        // its implicitWidth with an assigned contentItem here.
        ColumnLayout {
            Label {
                textFormat: Text.PlainText
                text: qsTr("Send %1 file(s) to the Recycle Bin?").arg((holder.data.files || []).length)
                font.bold: true
            }
            Repeater {
                model: (holder.data.files || []).slice(0, 12)
                delegate: Label {
                    required property string modelData
                    textFormat: Text.PlainText
                    text: modelData
                    elide: Text.ElideMiddle
                    Layout.preferredWidth: 540
                }
            }
        }
    }

    // The playlist's dialogs.
    readonly property var listTitles: ({
        "title": qsTr("Edit title of selected items"), "rename": qsTr("Rename filename of selected items"),
        "search": qsTr("Search title and filenames"), "url": qsTr("Add URL"), "info": qsTr("File information")
    })
    function showList() {
        const kind = data.list
        if (kind === "open") {
            playlistOpenDialog.open()
            return
        }
        if (kind === "save") {
            playlistSaveDialog.open()
            return
        }
        if (kind.startsWith("move") || kind.startsWith("copy")) {
            filesFolderDialog.title = kind.startsWith("move") ? "Move files to" : "Copy files to"
            filesFolderDialog.open()
            return
        }
        listText.text = data.text
        listText.readOnly = kind === "info"
        open(listDialog)
        listText.selectAll()
        listText.forceActiveFocus()
    }
    Dialog {
        popupType: Tools.native_video ? Popup.Window : Popup.Item
        id: listDialog
        objectName: "listDialog"
        title: holder.listTitles[holder.data.list] || ""
        modal: true
        anchors.centerIn: parent
        standardButtons: holder.data.list === "info" ? Dialog.Close : (Dialog.Ok | Dialog.Cancel)
        onAccepted: {
            if (holder.data.list !== "info")
                holder.player.list_dialog_answer(holder.data.list, listText.text)
        }
        contentItem: ColumnLayout {
            Label {
                visible: holder.data.list === "url"
                text: qsTr("Addresses, one a line:")
            }
            ScrollView {
                Layout.preferredWidth: 520
                Layout.preferredHeight: holder.data.list === "info" || holder.data.list === "url" ? 220 : 40
                TextArea {
                    id: listText
                    wrapMode: TextArea.Wrap
                    Keys.onReturnPressed: (event) => {
                        if (holder.data.list === "url" || holder.data.list === "info")
                            event.accepted = false
                        else
                            listDialog.accept()
                    }
                }
            }
        }
    }
    Dialogs.FileDialog {
        id: playlistOpenDialog
        title: qsTr("Open a playlist")
        nameFilters: [qsTr("Playlists (%1)").arg("*.m3u8 *.m3u *.pls *.dpl"), qsTr("All files (*)")]
        onAccepted: holder.player.list_dialog_answer("open", Tools.local_file(selectedFile))
    }
    Dialogs.FileDialog {
        id: playlistSaveDialog
        title: qsTr("Save playlist as")
        fileMode: Dialogs.FileDialog.SaveFile
        nameFilters: ["M3U8 (*.m3u8)", "M3U (*.m3u)", "PLS (*.pls)", "PotPlayer (*.dpl)"]
        onAccepted: holder.player.list_dialog_answer("save", Tools.local_file(selectedFile))
    }
    Dialogs.FolderDialog {
        id: filesFolderDialog
        onAccepted: holder.player.list_dialog_answer(holder.data.list, Tools.local_file(selectedFolder))
    }

    // Webcams and other capture devices.
    Dialog {
        popupType: Tools.native_video ? Popup.Window : Popup.Item
        id: deviceDialog
        objectName: "deviceDialog"
        title: qsTr("Open Webcam/Other Device")
        modal: true
        anchors.centerIn: parent
        standardButtons: Dialog.Ok | Dialog.Cancel
        onAccepted: {
            if (deviceField.text.trim() !== "")
                holder.player.open_device(deviceField.text.trim())
        }
        contentItem: ColumnLayout {
            Label {
                text: (holder.data.devices || []).length > 0
                      ? qsTr("Devices:") : qsTr("The system lists no capture devices. A device's name:")
            }
            Repeater {
                model: holder.data.devices || []
                delegate: RadioButton {
                    id: deviceChoice
                    required property var modelData
                    text: modelData.label ? modelData.label + " (" + modelData.name + ")" : modelData.name
                    checked: deviceField.text === modelData.name
                    onToggled: deviceField.text = modelData.name
                    // The system's name for the device, as written.
                    contentItem: Label {
                        textFormat: Text.PlainText
                        text: deviceChoice.text
                        elide: Text.ElideRight
                        leftPadding: deviceChoice.indicator.width + deviceChoice.spacing
                        verticalAlignment: Text.AlignVCenter
                    }
                }
            }
            TextField {
                id: deviceField
                Layout.preferredWidth: 360
            }
        }
    }

    Dialogs.FolderDialog {
        id: discFolderDialog
        onAccepted: holder.player.open_disc(Tools.local_file(selectedFolder), holder.data.bluray === true)
    }
    Dialogs.FileDialog {
        id: discFileDialog
        onAccepted: holder.player.open_disc(Tools.local_file(selectedFile), holder.data.bluray === true)
    }
    Dialogs.FileDialog {
        id: audioDialog
        title: qsTr("Load External Audio")
        nameFilters: [qsTr("Audio (%1)").arg("*.mp3 *.m4a *.aac *.ac3 *.dts *.flac *.mka *.ogg *.opus *.wav *.wma"),
                      qsTr("All files (*)")]
        onAccepted: holder.player.load_external_audio(Tools.local_file(selectedFile))
    }

    // Edit Favorites.
    Dialog {
        popupType: Tools.native_video ? Popup.Window : Popup.Item
        id: favoritesDialog
        objectName: "favoritesDialog"
        title: qsTr("Edit Favorites")
        modal: true
        anchors.centerIn: parent
        standardButtons: Dialog.Save | Dialog.Cancel
        onAccepted: holder.player.set_favorites(JSON.stringify(favorites.model))
        contentItem: ColumnLayout {
            ListView {
                id: favorites
                Layout.preferredWidth: 520
                Layout.preferredHeight: 240
                clip: true
                model: []
                highlight: Rectangle { color: "#30608cff" }
                delegate: ItemDelegate {
                    id: favoriteRow
                    required property var modelData
                    required property int index
                    width: ListView.view.width
                    text: modelData.title + (modelData.folder ? "  (folder)" : "") + "   " + modelData.path
                    // The name and path as written (as the marks' rows).
                    contentItem: Label {
                        textFormat: Text.PlainText
                        text: favoriteRow.text
                        font: favoriteRow.font
                        elide: Text.ElideRight
                        verticalAlignment: Text.AlignVCenter
                    }
                    onClicked: favorites.currentIndex = index
                }
            }
            RowLayout {
                Button {
                    text: qsTr("Remove")
                    enabled: favorites.currentIndex >= 0
                    onClicked: {
                        const list = favorites.model.slice()
                        list.splice(favorites.currentIndex, 1)
                        favorites.model = list
                    }
                }
                Button {
                    text: qsTr("Up")
                    enabled: favorites.currentIndex > 0
                    onClicked: {
                        const list = favorites.model.slice()
                        const i = favorites.currentIndex
                        list.splice(i - 1, 0, list.splice(i, 1)[0])
                        favorites.model = list
                        favorites.currentIndex = i - 1
                    }
                }
                Button {
                    text: qsTr("Down")
                    enabled: favorites.currentIndex >= 0 && favorites.currentIndex < favorites.count - 1
                    onClicked: {
                        const list = favorites.model.slice()
                        const i = favorites.currentIndex
                        list.splice(i + 1, 0, list.splice(i, 1)[0])
                        favorites.model = list
                        favorites.currentIndex = i + 1
                    }
                }
            }
        }
    }
}
