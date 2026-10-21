// The Open URL dialog, loaded on first use.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Rvp
import RvpNative

Item {
    id: holder
    required property Player player
    readonly property alias dialog: urlDialog

    function ask() {
        urlField.text = ""
        urlDialog.open()
        urlField.forceActiveFocus()
    }

    Dialog {
        // Over the Direct3D 11 renderer's native video window only a
        // window of its own shows (D4).
        popupType: Tools.native_video ? Popup.Window : Popup.Item
        id: urlDialog
        title: qsTr("Open URL")
        modal: true
        anchors.centerIn: parent
        standardButtons: Dialog.Ok | Dialog.Cancel
        onAccepted: {
            const url = urlField.text.trim()
            if (url !== "")
                holder.player.open_files(url, false)
        }
        contentItem: ColumnLayout {
            Label { text: qsTr("Address (http, https, rtmp, ...):") }
            TextField {
                id: urlField
                Layout.preferredWidth: 420
                onAccepted: urlDialog.accept()
            }
        }
    }
}
