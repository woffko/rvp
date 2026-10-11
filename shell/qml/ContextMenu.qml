// The context menu, loaded on first use so that starting RVP does not wait
// for Qt Quick Controls: PotPlayer's texts with the keys bound to them.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Rvp
import RvpNative

Item {
    id: holder
    required property Player player
    // --log-geometry: list the windows once the menu's window is mapped.
    property bool logWindows: false
    readonly property alias menu: contextMenu

    component ActionItem: MenuItem {
        id: entry
        property string actionName
        text: holder.player.menu_label(actionName)
        onTriggered: holder.player.run_action(actionName)
        contentItem: RowLayout {
            spacing: 24
            Label {
                text: entry.text
                Layout.fillWidth: true
            }
            Label {
                text: holder.player.menu_shortcut(entry.actionName)
                opacity: 0.7
            }
        }
    }

    // The menu's window maps after `opened`; list the windows a little later.
    Timer {
        id: windowsLog
        interval: 400
        onTriggered: Tools.log("menu_windows=" + Tools.windows_json())
    }

    Menu {
        id: contextMenu
        popupType: Popup.Window
        onOpened: if (holder.logWindows) windowsLog.restart()
        ActionItem { actionName: "open-files" }
        ActionItem { actionName: "close-media" }
        MenuSeparator {}
        ActionItem { actionName: "play-pause" }
        ActionItem { actionName: "previous" }
        ActionItem { actionName: "next" }
        Menu {
            title: "Seek"
            popupType: Popup.Window
            ActionItem { actionName: "seek-backward-short" }
            ActionItem { actionName: "seek-forward-short" }
            ActionItem { actionName: "seek-backward-medium" }
            ActionItem { actionName: "seek-forward-medium" }
            ActionItem { actionName: "seek-backward-long" }
            ActionItem { actionName: "seek-forward-long" }
            ActionItem { actionName: "seek-backward-very-long" }
            ActionItem { actionName: "seek-forward-very-long" }
        }
        MenuSeparator {}
        ActionItem { actionName: "volume-up" }
        ActionItem { actionName: "volume-down" }
        ActionItem { actionName: "toggle-mute" }
        ActionItem { actionName: "toggle-subtitles" }
        MenuSeparator {}
        ActionItem { actionName: "toggle-fullscreen" }
        ActionItem { actionName: "toggle-fullscreen-stretch" }
        ActionItem { actionName: "maximize-restore" }
        ActionItem { actionName: "minimize" }
        ActionItem { actionName: "toggle-playlist" }
        MenuSeparator {}
        ActionItem { actionName: "exit" }
    }
}
