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

    // Hidden only where the tray shows the icon: a desktop without a
    // StatusNotifierItem host (GNOME without its AppIndicator extension)
    // shows none, and nothing would bring the window back (the Linux
    // audit's M6).
    Connections {
        target: tray.owner
        function onVisibilityChanged() {
            if (tray.toTray && icon.available && tray.owner.visibility === Window.Minimized)
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
            // Wayland tells a program nothing of its minimized state: the
            // active window goes down, any other comes back.
            const shown = Qt.platform.pluginName === "wayland"
                ? tray.owner.visible && tray.owner.active
                : tray.owner.visible && tray.owner.visibility !== Window.Minimized
            if (shown)
                tray.owner.showMinimized()
            else
                tray.restore()
        }
        menu: Platform.Menu {
            Platform.MenuItem {
                text: qsTr("Show RVP")
                onTriggered: tray.restore()
            }
            Platform.MenuSeparator {}
            Platform.MenuItem {
                text: qsTr("Play/Pause")
                onTriggered: tray.player.run_action("play-pause")
            }
            Platform.MenuItem {
                text: qsTr("Previous")
                onTriggered: tray.player.run_action("previous")
            }
            Platform.MenuItem {
                text: qsTr("Next")
                onTriggered: tray.player.run_action("next")
            }
            Platform.MenuSeparator {}
            Platform.MenuItem {
                text: qsTr("Exit")
                onTriggered: tray.player.run_action("exit")
            }
        }
    }
}
