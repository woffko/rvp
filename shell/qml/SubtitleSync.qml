// The Adjust Subtitle Sync dialog (PotPlayer's Subtitles > Subtitle Sync),
// loaded on first use: the sync moves as the slider or the field moves, and
// the frame rate the subtitles were made for converts their timing (mpv's
// sub-fps; PotPlayer's Convert Subtitle FPS).
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Rvp
import RvpNative

Item {
    id: holder
    required property Player player
    readonly property alias dialog: syncDialog
    // While the dialog sets its controls from the current state.
    property bool loading: false
    // The rates of PotPlayer's Convert Subtitle FPS items; 0: the video's.
    readonly property var rates: [0, 12, 15, 23.976, 24, 25, 29.97, 30]

    // `milliseconds`: the current sync.
    function ask(milliseconds) {
        holder.loading = true
        syncField.value = milliseconds
        syncSlider.value = Math.max(syncSlider.from, Math.min(syncSlider.to, milliseconds))
        const fps = holder.player.subtitle_fps()
        const index = holder.rates.findIndex(r => Math.abs(r - fps) < 0.0005)
        rateBox.currentIndex = index >= 0 ? index : 0
        holder.loading = false
        syncDialog.open()
    }

    function apply(milliseconds) {
        if (!holder.loading)
            holder.player.set_subtitle_delay(milliseconds / 1000)
    }

    Dialog {
        // Over the Direct3D 11 renderer's native video window only a
        // window of its own shows (D4).
        popupType: Tools.native_video ? Popup.Window : Popup.Item
        id: syncDialog
        objectName: "subtitleSyncDialog"
        title: qsTr("Adjust Subtitle Sync")
        modal: false
        anchors.centerIn: parent
        standardButtons: Dialog.Close
        contentItem: ColumnLayout {
            spacing: 8
            Label { text: qsTr("Subtitle sync in milliseconds (later is positive):") }
            Slider {
                id: syncSlider
                objectName: "subtitleSyncSlider"
                Layout.preferredWidth: 420
                from: -10000
                to: 10000
                stepSize: 50
                onMoved: {
                    syncField.value = value
                    holder.apply(value)
                }
            }
            RowLayout {
                SpinBox {
                    id: syncField
                    objectName: "subtitleSyncField"
                    from: -600000
                    to: 600000
                    stepSize: 50
                    editable: true
                    onValueModified: {
                        holder.loading = true
                        syncSlider.value = Math.max(syncSlider.from, Math.min(syncSlider.to, value))
                        holder.loading = false
                        holder.apply(value)
                    }
                }
                Button {
                    text: qsTr("Default")
                    onClicked: {
                        syncField.value = 0
                        syncSlider.value = 0
                        holder.player.run_action("subtitle-delay-reset")
                    }
                }
            }
            Label { text: qsTr("The subtitles were made for (their timing follows the video's rate):") }
            ComboBox {
                id: rateBox
                objectName: "subtitleRateBox"
                Layout.preferredWidth: 300
                model: holder.rates.map(r => r === 0 ? "The video's frame rate" : r + " fps")
                onActivated: (index) => {
                    if (!holder.loading)
                        holder.player.set_subtitle_fps(holder.rates[index])
                }
            }
        }
    }
}
