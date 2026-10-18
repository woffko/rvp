// The About box (F1), loaded on first use.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Rvp
import RvpNative

Item {
    id: holder
    required property Player player
    readonly property alias dialog: aboutDialog

    function ask() {
        aboutText.text = holder.player.about_text()
        aboutDialog.open()
    }

    Dialog {
        // Over the Direct3D 11 renderer's native video window only a
        // window of its own shows (D4).
        popupType: Tools.native_video ? Popup.Window : Popup.Item
        id: aboutDialog
        title: "About RVP"
        modal: true
        anchors.centerIn: parent
        standardButtons: Dialog.Ok
        contentItem: ColumnLayout {
            spacing: 12
            // RVP's wordmark (D10), dark letters on a light dialog.
            Image {
                Layout.alignment: Qt.AlignHCenter
                Layout.preferredHeight: 48
                Layout.preferredWidth: Math.round(48 * implicitWidth / Math.max(1, implicitHeight))
                source: aboutDialog.palette.window.hslLightness > 0.5
                        ? "qrc:/qt/qml/Rvp/look/logo/wordmark-dark.png"
                        : "qrc:/qt/qml/Rvp/look/logo/wordmark-light.png"
                fillMode: Image.PreserveAspectFit
                smooth: true
                mipmap: true
            }
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
