// M12's dialogs, loaded on first use: a value typed for a purpose (a custom
// aspect ratio, the Extend/Crop size, the custom window size) and Window
// Size Control. Player.dialog holds the open request's data as JSON; the
// answer goes to Player.video_dialog_answer.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Rvp

Item {
    id: holder
    required property Player player
    // The main window (Window Size Control places it).
    required property var window
    property var data: ({})
    // The open dialog (self-test).
    property var current: null

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
        id: inputDialog
        objectName: "inputDialog"
        modal: true
        anchors.centerIn: parent
        standardButtons: Dialog.Ok | Dialog.Cancel
        onAccepted: holder.answer(inputField.text)
        ColumnLayout {
            Label { id: inputLabel }
            TextField {
                id: inputField
                Layout.preferredWidth: 320
                onAccepted: inputDialog.accept()
            }
        }
    }

    // Window Size Control: the window's size and place in pixels.
    Dialog {
        id: windowDialog
        objectName: "windowDialog"
        title: "Window Size Control"
        modal: true
        anchors.centerIn: parent
        standardButtons: Dialog.Ok | Dialog.Cancel
        onAccepted: holder.window.placeWindow(widthBox.value, heightBox.value, leftBox.value, topBox.value)
        GridLayout {
            columns: 2
            Label { text: "Width" }
            SpinBox { id: widthBox; from: 160; to: 16384; editable: true }
            Label { text: "Height" }
            SpinBox { id: heightBox; from: 120; to: 16384; editable: true }
            Label { text: "Left" }
            SpinBox { id: leftBox; from: -32768; to: 32767; editable: true }
            Label { text: "Top" }
            SpinBox { id: topBox; from: -32768; to: 32767; editable: true }
        }
    }
}
