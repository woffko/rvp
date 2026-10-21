// Skins > Color Themes > Customize... (M14), loaded on first use: the
// custom theme as HSB (hue, saturation) or RGB offsets. Apply shows it
// (each Apply makes one tinted copy of the skin, so the sliders do not
// apply as they move).
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Rvp
import RvpNative

Item {
    id: holder
    required property Player player
    readonly property alias dialog: themeDialog

    function setting(path, value) {
        holder.player.set_setting(path, JSON.stringify(value))
    }

    function ask() {
        const custom = JSON.parse(holder.player.settings_view(false)).settings.skins.custom
        hsbBox.checked = custom.hsb
        rgbBox.checked = !custom.hsb
        hue.value = custom.hue
        sat.value = custom.sat
        red.value = custom.red
        green.value = custom.green
        blue.value = custom.blue
        themeDialog.open()
    }

    function apply() {
        holder.setting("skins.custom", {
            hsb: hsbBox.checked, hue: Math.round(hue.value), sat: Math.round(sat.value),
            red: Math.round(red.value), green: Math.round(green.value), blue: Math.round(blue.value)
        })
        holder.setting("skins.color-theme", "custom")
    }

    component Channel: RowLayout {
        property alias value: slider.value
        property alias from: slider.from
        property alias to: slider.to
        property string label
        Label { text: parent.label; Layout.preferredWidth: 90 }
        Slider { id: slider; stepSize: 1; Layout.preferredWidth: 260 }
        Label { text: Math.round(slider.value); Layout.preferredWidth: 40 }
    }

    Dialog {
        // Over the Direct3D 11 renderer's native video window only a
        // window of its own shows (D4).
        popupType: Tools.native_video ? Popup.Window : Popup.Item
        id: themeDialog
        objectName: "colorThemeDialog"
        title: qsTr("Customize Color Theme")
        modal: false
        anchors.centerIn: parent
        standardButtons: Dialog.Apply | Dialog.Close
        onApplied: holder.apply()
        contentItem: ColumnLayout {
            RowLayout {
                RadioButton { id: hsbBox; text: qsTr("HSB color theme") }
                RadioButton { id: rgbBox; text: qsTr("RGB color theme") }
            }
            Channel { id: hue; label: qsTr("Hue"); from: -128; to: 128; enabled: hsbBox.checked }
            Channel { id: sat; label: qsTr("Saturation"); from: 0; to: 100; enabled: hsbBox.checked }
            Channel { id: red; label: qsTr("Red"); from: -255; to: 255; enabled: rgbBox.checked }
            Channel { id: green; label: qsTr("Green"); from: -255; to: 255; enabled: rgbBox.checked }
            Channel { id: blue; label: qsTr("Blue"); from: -255; to: 255; enabled: rgbBox.checked }
            Label { text: qsTr("HSB colours the skin with one hue; RGB shifts its colours.") }
        }
    }
}
