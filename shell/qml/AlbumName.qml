// The album name dialog (Add Album, Rename Album), loaded on first use.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Rvp

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
        id: nameDialog
        title: holder.rename ? "Rename Album" : "Add Album"
        modal: true
        anchors.centerIn: parent
        standardButtons: Dialog.Ok | Dialog.Cancel
        onAccepted: holder.player.name_album(nameField.text, holder.rename)
        contentItem: ColumnLayout {
            Label { text: "Album name:" }
            TextField {
                id: nameField
                Layout.preferredWidth: 280
                onAccepted: nameDialog.accept()
            }
        }
    }
}
