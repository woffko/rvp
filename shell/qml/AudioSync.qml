// The Adjust Audio Sync dialog (PotPlayer's Audio > Audio Sync), loaded on
// first use: the sync moves as the slider or the field moves.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Rvp
import RvpNative

Item {
    id: holder
    required property Player player
    readonly property alias dialog: syncDialog
    // While the dialog sets its controls from the current sync.
    property bool loading: false

    // `milliseconds`: the current sync.
    function ask(milliseconds) {
        holder.loading = true
        syncField.value = milliseconds
        syncSlider.value = Math.max(syncSlider.from, Math.min(syncSlider.to, milliseconds))
        holder.loading = false
        syncDialog.open()
    }

    function apply(milliseconds) {
        if (!holder.loading)
            holder.player.set_audio_delay(milliseconds / 1000)
    }

    Dialog {
        // Over the Direct3D 11 renderer's native video window only a
        // window of its own shows (D4).
        popupType: Tools.native_video ? Popup.Window : Popup.Item
        id: syncDialog
        objectName: "audioSyncDialog"
        title: "Adjust Audio Sync"
        modal: false
        anchors.centerIn: parent
        standardButtons: Dialog.Close
        contentItem: ColumnLayout {
            spacing: 8
            Label { text: "Audio sync in milliseconds (later is positive):" }
            Slider {
                id: syncSlider
                objectName: "audioSyncSlider"
                Layout.preferredWidth: 420
                from: -2000
                to: 2000
                stepSize: 10
                onMoved: {
                    syncField.value = value
                    holder.apply(value)
                }
            }
            RowLayout {
                SpinBox {
                    id: syncField
                    objectName: "audioSyncField"
                    from: -10000
                    to: 10000
                    stepSize: 10
                    editable: true
                    onValueModified: {
                        holder.loading = true
                        syncSlider.value = Math.max(syncSlider.from, Math.min(syncSlider.to, value))
                        holder.loading = false
                        holder.apply(value)
                    }
                }
                Button {
                    text: "Default"
                    onClicked: {
                        syncField.value = 0
                        syncSlider.value = 0
                        holder.player.run_action("audio-delay-reset")
                    }
                }
            }
        }
    }
}
