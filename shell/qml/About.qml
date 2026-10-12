// The About box (F1), loaded on first use.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Rvp

Item {
    id: holder
    required property Player player
    readonly property alias dialog: aboutDialog

    function ask() {
        aboutText.text = holder.player.about_text()
        aboutDialog.open()
    }

    Dialog {
        id: aboutDialog
        title: "About RVP"
        modal: true
        anchors.centerIn: parent
        standardButtons: Dialog.Ok
        contentItem: ColumnLayout {
            Label {
                id: aboutText
                Layout.preferredWidth: 360
                wrapMode: Text.Wrap
            }
            Label {
                text: "Plays through libmpv and draws PotPlayer's skins; PotPlayer's and SVP's files are read where they are installed."
                Layout.preferredWidth: 360
                wrapMode: Text.Wrap
                opacity: 0.7
            }
        }
    }
}
