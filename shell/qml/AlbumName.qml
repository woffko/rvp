// The album name dialog (Add Album, Rename Album), loaded on first use.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Rvp
import RvpNative

Item {
    id: holder
    required property Player player
    readonly property alias dialog: nameDialog
    property bool rename: false
    property alias text: nameField.text

    function ask(renaming, current) {
        holder.rename = renaming
        nameField.text = current
        nameDialog.open()
        nameField.selectAll()
        nameField.forceActiveFocus()
    }

    Dialog {
        // Over the Direct3D 11 renderer's native video window only a
        // window of its own shows (D4).
        popupType: Tools.native_video ? Popup.Window : Popup.Item
        id: nameDialog
        title: holder.rename ? qsTr("Rename Album") : qsTr("Add Album")
        modal: true
        anchors.centerIn: parent
        standardButtons: Dialog.Ok | Dialog.Cancel
        onAccepted: holder.player.name_album(nameField.text, holder.rename)
        contentItem: ColumnLayout {
            Label { text: qsTr("Album name:") }
            TextField {
                id: nameField
                Layout.preferredWidth: 280
                onAccepted: nameDialog.accept()
            }
        }
    }
}
