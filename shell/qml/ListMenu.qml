// The playlist panel's context menu, loaded on first use: PotPlayer's
// playlist commands (R1 table 118) with the keys bound to them.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Rvp

Item {
    id: holder
    required property Player player
    readonly property alias menu: listMenu
    // Bumped each time the menu opens (keys may have changed).
    property int revision: 0

    component ListItem: MenuItem {
        id: entry
        property string actionName
        text: holder.revision >= 0 ? holder.player.list_menu_label(actionName) : ""
        onTriggered: holder.player.run_list_action(actionName)
        contentItem: RowLayout {
            spacing: 24
            Label {
                text: entry.text
                Layout.fillWidth: true
            }
            Label {
                text: holder.revision >= 0 ? holder.player.list_menu_shortcut(entry.actionName) : ""
                opacity: 0.7
            }
        }
    }

    Menu {
        id: listMenu
        popupType: Popup.Window
        onAboutToShow: holder.revision += 1
        ListItem { actionName: "play" }
        MenuSeparator {}
        ListItem { actionName: "add-files" }
        ListItem { actionName: "add-folder" }
        MenuItem {
            text: holder.player.menu_label("import-playlists")
            onTriggered: holder.player.run_action("import-playlists")
        }
        MenuSeparator {}
        ListItem { actionName: "remove-selected" }
        ListItem { actionName: "remove-unselected" }
        ListItem { actionName: "remove-missing" }
        ListItem { actionName: "remove-all" }
        MenuSeparator {}
        ListItem { actionName: "select-all" }
        ListItem { actionName: "invert-selection" }
        MenuSeparator {}
        Menu {
            title: "Sort"
            popupType: Popup.Window
            ListItem { actionName: "sort-ascending" }
            ListItem { actionName: "sort-descending" }
            ListItem { actionName: "shuffle" }
        }
        Menu {
            title: "Move"
            popupType: Popup.Window
            ListItem { actionName: "move-up" }
            ListItem { actionName: "move-down" }
            ListItem { actionName: "move-top" }
            ListItem { actionName: "move-bottom" }
        }
        MenuSeparator {}
        ListItem { actionName: "undo" }
        ListItem { actionName: "redo" }
        MenuSeparator {}
        ListItem { actionName: "add-album" }
        ListItem { actionName: "rename-album" }
        ListItem { actionName: "delete-album" }
        MenuSeparator {}
        ListItem { actionName: "close" }
    }
}
