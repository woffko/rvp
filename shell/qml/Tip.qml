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
        // The style's own content item, as plain text: a skin's tooltip,
        // or a title in it, shows as written.
        contentItem: Text {
            text: toolTip.text
            font: toolTip.font
            wrapMode: Text.Wrap
            color: toolTip.palette.toolTipText
            textFormat: Text.PlainText
        }
    }
}
