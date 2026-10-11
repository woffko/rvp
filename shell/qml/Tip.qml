// The tooltip of skin controls (section 4), loaded on first use.
import QtQuick
import QtQuick.Controls

Item {
    id: holder
    readonly property alias tip: toolTip

    ToolTip {
        id: toolTip
        popupType: Popup.Window
        delay: 0
        timeout: 5000
    }
}
