// General: "Taskbar/Tray icon" (M14), loaded when the tray icon is on: RVP's
// icon next to the clock (Qt's system tray, on Windows), with a menu, and
// with "minimize to tray" the window leaves the taskbar while minimized.
import QtQuick
import Qt.labs.platform as Platform
import Rvp
import RvpNative

Item {
    id: tray
    required property Player player
    required property Window owner
    readonly property bool toTray: (player.general_look ? JSON.parse(player.general_look) : {}).tray === "minimize-to-tray"

    function restore() {
        tray.owner.showNormal()
        tray.owner.raise()
        tray.owner.requestActivate()
    }

    Connections {
        target: tray.owner
        function onVisibilityChanged() {
            if (tray.toTray && tray.owner.visibility === Window.Minimized)
                tray.owner.hide()
        }
    }

    Platform.SystemTrayIcon {
        id: icon
        visible: true
        icon.source: "qrc:/qt/qml/Rvp/look/icons/rvp-32.png"
        tooltip: tray.owner.title
        onActivated: (reason) => {
            if (reason === Platform.SystemTrayIcon.Context)
                return
            if (tray.owner.visible && tray.owner.visibility !== Window.Minimized)
                tray.owner.showMinimized()
            else
                tray.restore()
        }
        menu: Platform.Menu {
            Platform.MenuItem {
                text: "Show RVP"
                onTriggered: tray.restore()
            }
            Platform.MenuSeparator {}
            Platform.MenuItem {
                text: "Play/Pause"
                onTriggered: tray.player.run_action("play-pause")
            }
            Platform.MenuItem {
                text: "Previous"
                onTriggered: tray.player.run_action("previous")
            }
            Platform.MenuItem {
                text: "Next"
                onTriggered: tray.player.run_action("next")
            }
            Platform.MenuSeparator {}
            Platform.MenuItem {
                text: "Exit"
                onTriggered: tray.player.run_action("exit")
            }
        }
    }
}
