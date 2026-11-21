// M12's dialogs, loaded on first use: a value typed for a purpose (a custom
// aspect ratio, the Extend/Crop size, the custom window size), Window Size
// Control, consecutive images, the thumbnail image, and the files and
// folder of the recordings. Player.dialog holds
// the open request's data as JSON; the answer goes to
// Player.video_dialog_answer.
import QtQuick
import QtQuick.Controls
// Qualified: Rvp's own FolderDialog.qml has the same name.
import QtQuick.Dialogs as Dialogs
import QtQuick.Layouts
import Rvp
import RvpNative

Item {
    id: holder
    required property Player player
    // The main window (Window Size Control places it).
    required property var window
    property var data: ({})
    // The open dialog, and the consecutive images' interval (self-test).
    property var current: null
    property alias consecutiveInterval: intervalSlider

    function show(json) {
        data = JSON.parse(json)
        switch (data.kind) {
        case "input":
            inputDialog.title = data.title
            inputLabel.text = data.label
            inputField.text = data.value
            open(inputDialog)
            inputField.selectAll()
            inputField.forceActiveFocus()
            break
        case "consecutive":
            intervalSlider.pick(data.interval)
            countBox.value = data.count
            open(consecutiveDialog)
            break
        case "sheet":
            columnsBox.value = data.sheet.columns
            rowsBox.value = data.sheet.rows
            sheetWidthBox.value = data.sheet.width
            infoCheck.checked = data.sheet.info
            timesCheck.checked = data.sheet.times
            exactCheck.checked = data.sheet.exact
            open(sheetDialog)
            break
        case "save":
            saveDialog.title = data.title
            saveDialog.nameFilters = data.filters
            saveDialog.selectedFile = data.file
            open(saveDialog)
            break
        case "folder":
            folderDialog.title = data.title
            folderDialog.currentFolder = data.folder
            open(folderDialog)
            break
        case "window-control":
            widthBox.value = holder.window.width
            heightBox.value = holder.window.height
            leftBox.value = holder.window.x
            topBox.value = holder.window.y
            open(windowDialog)
            break
        }
    }
    function open(dialog) {
        current = dialog
        dialog.open()
    }
    // The answer as the dialog's OK gives it (self-test).
    function answer(text) {
        holder.player.video_dialog_answer(holder.data.purpose, text)
    }

    Dialog {
        // Over the Direct3D 11 renderer's native video window only a
        // window of its own shows (D4).
        popupType: Tools.native_video ? Popup.Window : Popup.Item
        id: inputDialog
        objectName: "inputDialog"
        modal: true
        anchors.centerIn: parent
        standardButtons: Dialog.Ok | Dialog.Cancel
        onAccepted: holder.answer(inputField.text)
        ColumnLayout {
            Label { id: inputLabel; textFormat: Text.PlainText }
            TextField {
                id: inputField
                Layout.preferredWidth: 320
                onAccepted: inputDialog.accept()
            }
        }
    }

    // Record Stream and Record Video: the file to record into.
    Dialogs.FileDialog {
        id: saveDialog
        objectName: "saveDialog"
        fileMode: Dialogs.FileDialog.SaveFile
        onAccepted: holder.answer(Tools.local_file(selectedFile))
    }
    // The video storage folder.
    Dialogs.FolderDialog {
        id: folderDialog
        objectName: "folderDialog"
        onAccepted: holder.answer(Tools.local_file(selectedFolder))
    }

    // Capture Consecutive Images: every so many seconds of playback, a
    // number of images. The interval is a slider (no decimal to type in
    // the locale's way) over the stops of the Capture page's.
    Dialog {
        popupType: Tools.native_video ? Popup.Window : Popup.Item
        id: consecutiveDialog
        objectName: "consecutiveDialog"
        title: qsTr("Capture Consecutive Images")
        modal: true
        anchors.centerIn: parent
        standardButtons: Dialog.Ok | Dialog.Cancel
        onAccepted: holder.answer(JSON.stringify({ interval: intervalSlider.seconds,
                                                    count: countBox.value }))
        GridLayout {
            columns: 2
            Label { text: qsTr("Every (seconds)") }
            RowLayout {
                Slider {
                    id: intervalSlider
                    objectName: "intervalSlider"
                    // Settings.qml's capture.interval stops.
                    readonly property var values: [0.1, 0.2, 0.5, 1, 2, 3, 5, 10, 15, 30, 60, 120, 300,
                                                   600, 1800, 3600]
                    readonly property real seconds: values[Math.round(value)]
                    Layout.preferredWidth: 200
                    from: 0
                    to: values.length - 1
                    stepSize: 1
                    snapMode: Slider.SnapAlways
                    // The stop nearest `v` (the dialog's start, self-test).
                    function pick(v) {
                        let best = 0
                        for (let i = 1; i < values.length; ++i)
                            if (Math.abs(values[i] - v) < Math.abs(values[best] - v))
                                best = i
                        value = best
                    }
                }
                Label {
                    textFormat: Text.PlainText
                    text: String(intervalSlider.seconds)
                    Layout.minimumWidth: 40
                }
            }
            Label { text: qsTr("Images") }
            SpinBox { id: countBox; from: 1; to: 10000; editable: true }
        }
    }

    // Create Thumbnail Image: a contact sheet of the playing file.
    Dialog {
        popupType: Tools.native_video ? Popup.Window : Popup.Item
        id: sheetDialog
        objectName: "sheetDialog"
        title: qsTr("Create Thumbnail Image")
        modal: true
        anchors.centerIn: parent
        standardButtons: Dialog.Ok | Dialog.Cancel
        onAccepted: holder.answer(JSON.stringify({
            columns: columnsBox.value, rows: rowsBox.value, width: sheetWidthBox.value,
            info: infoCheck.checked, times: timesCheck.checked, exact: exactCheck.checked }))
        GridLayout {
            columns: 2
            Label { text: qsTr("Columns") }
            SpinBox { id: columnsBox; from: 1; to: 10; editable: true }
            Label { text: qsTr("Rows") }
            SpinBox { id: rowsBox; from: 1; to: 20; editable: true }
            Label { text: qsTr("Image width (pixels)") }
            SpinBox { id: sheetWidthBox; from: 320; to: 7680; stepSize: 160; editable: true }
            CheckBox { id: infoCheck; text: qsTr("File information above the pictures"); Layout.columnSpan: 2 }
            CheckBox { id: timesCheck; text: qsTr("The time on each picture"); Layout.columnSpan: 2 }
            CheckBox { id: exactCheck; text: qsTr("Exact frames (slower)"); Layout.columnSpan: 2 }
        }
    }

    // Window Size Control: the window's size and place in pixels.
    Dialog {
        popupType: Tools.native_video ? Popup.Window : Popup.Item
        id: windowDialog
        objectName: "windowDialog"
        title: qsTr("Window Size Control")
        modal: true
        anchors.centerIn: parent
        standardButtons: Dialog.Ok | Dialog.Cancel
        onAccepted: holder.window.placeWindow(widthBox.value, heightBox.value, leftBox.value, topBox.value)
        GridLayout {
            columns: 2
            Label { text: qsTr("Width") }
            SpinBox { id: widthBox; from: 160; to: 16384; editable: true }
            Label { text: qsTr("Height") }
            SpinBox { id: heightBox; from: 120; to: 16384; editable: true }
            Label { text: qsTr("Left") }
            SpinBox { id: leftBox; from: -32768; to: 32767; editable: true }
            Label { text: qsTr("Top") }
            SpinBox { id: topBox; from: -32768; to: 32767; editable: true }
        }
    }
}
