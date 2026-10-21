// General: "Use advanced drag and drop interaction" (M14): what dropped
// files are for, chosen from a small menu at the pointer. Loaded on first
// use (Main.qml imports only Qt Quick).
import QtQuick
import QtQuick.Controls
import Rvp
import RvpNative

Item {
    id: holder
    required property Player player
    property string paths: ""

    function ask(text) {
        holder.paths = text
        menu.popup()
    }

    Menu {
        id: menu
        popupType: Popup.Window
        MenuItem {
            text: "Play"
            onTriggered: holder.player.drop_choice(0, holder.paths)
        }
        MenuItem {
            text: "Add to the album"
            onTriggered: holder.player.drop_choice(1, holder.paths)
        }
        MenuItem {
            text: "Load as subtitles"
            onTriggered: holder.player.drop_choice(2, holder.paths)
        }
    }
}
