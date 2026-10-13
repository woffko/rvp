// The control panel (F7), loaded the first time it opens: a small frameless
// tool window with the skin's ControlBoxSkinArea. Player lays it out,
// paints it whole (SkinView surface 1) and takes its pointer input; keys go
// to the bindings as in the main window.
import QtQuick
import QtQuick.Controls
import QtQuick.Window
import Rvp
import RvpNative

Window {
    id: panel
    required property Player player
    required property Window owner
    // The first time it shows, the panel goes to the bottom right of the
    // video; later it stays where it was.
    property bool placed: false

    width: Math.max(1, player.panel_width)
    height: Math.max(1, player.panel_height)
    minimumWidth: width
    maximumWidth: width
    minimumHeight: height
    maximumHeight: height
    flags: Qt.Tool | Qt.FramelessWindowHint | (player.on_top ? Qt.WindowStaysOnTopHint : 0)
    transientParent: owner
    title: "Control Panel"
    color: "black"
    visible: player.panel_shown

    onVisibleChanged: {
        if (!visible)
            return
        if (!placed) {
            placed = true
            const video = player.video_rect
            const screen = owner.screen
            let px = owner.x + video.x + video.width - width - 16
            let py = owner.y + video.y + video.height - height - 16
            if (screen) {
                px = Math.max(screen.virtualX, Math.min(px, screen.virtualX + screen.desktopAvailableWidth - width))
                py = Math.max(screen.virtualY, Math.min(py, screen.virtualY + screen.desktopAvailableHeight - height))
            }
            x = px
            y = py
        }
        Tools.repaint(view, false)
    }
    onClosing: (close) => player.panel_closed()

    function startMove() {
        panel.startSystemMove()
    }

    SkinView {
        id: view
        surface: 1
        anchors.fill: parent
    }

    Connections {
        target: panel.player
        function onPanelDamaged() {
            Tools.repaint(view, false)
        }
    }

    Item {
        id: keys
        focus: true
        Keys.onPressed: (event) => {
            event.accepted = panel.player.key(event.key, event.modifiers)
        }
    }

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.AllButtons
        hoverEnabled: true
        onPressed: (mouse) => {
            keys.forceActiveFocus()
            tip.close()
            panel.player.panel_press(mouse.x, mouse.y, mouse.button)
        }
        onReleased: (mouse) => panel.player.panel_release(mouse.x, mouse.y, mouse.button)
        onDoubleClicked: (mouse) => panel.player.panel_double(mouse.x, mouse.y, mouse.button)
        onPositionChanged: (mouse) => {
            tip.close()
            tipTimer.restart()
            panel.player.panel_move(mouse.x, mouse.y)
        }
        onExited: {
            tipTimer.stop()
            tip.close()
            panel.player.panel_leave()
        }
    }

    // Tooltips of the panel's controls (section 4).
    Timer {
        id: tipTimer
        interval: 600
        onTriggered: {
            if (panel.player.panel_tooltip === "")
                return
            tip.text = panel.player.panel_tooltip
            tip.open()
        }
    }
    ToolTip {
        id: tip
        popupType: Popup.Window
        delay: 0
        timeout: 5000
    }
}
