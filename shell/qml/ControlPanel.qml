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
    // The menus' loader: a press that closed a menu stays with it.
    required property Loader menus
    property bool swallowing: false
    // The first time it shows, the panel goes where it was last (saved in
    // the state; not on Wayland, which reports no positions) or to the
    // bottom right of the video; later it stays where it was.
    property bool placed: false

    width: Math.max(1, player.panel_width)
    height: Math.max(1, player.panel_height)
    minimumWidth: width
    maximumWidth: width
    minimumHeight: height
    maximumHeight: height
    flags: Qt.Tool | Qt.FramelessWindowHint | (player.on_top ? Qt.WindowStaysOnTopHint : 0)
    // General: "Hide all windows when minimized" (M14): an owned window
    // hides with its owner.
    transientParent: (player.general_look ? JSON.parse(player.general_look) : {}).hideWithOwner === false ? null : owner
    title: qsTr("Control Panel")
    color: "black"
    visible: player.panel_shown
    // Skins > Opacity > Control Panel (M14), fading in as it shows when
    // window animations are on.
    property real fade: 1
    opacity: (player.panel_opacity > 0 ? player.panel_opacity : 1) * fade
    NumberAnimation {
        id: fadeIn
        target: panel
        property: "fade"
        from: 0
        to: 1
        duration: 150
    }

    onVisibleChanged: {
        if (!visible)
            return
        if (player.animations)
            fadeIn.restart()
        if (!placed) {
            placed = true
            const video = player.video_rect
            const screen = owner.screen
            let px = owner.x + video.x + video.width - width - 16
            let py = owner.y + video.y + video.height - height - 16
            if (player.panel_has_position && panel.onScreen(player.panel_x, player.panel_y)) {
                px = player.panel_x
                py = player.panel_y
            } else if (screen) {
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

    // True when the point lies on one of the screens.
    function onScreen(px, py) {
        for (const screen of Qt.application.screens) {
            if (px >= screen.virtualX && py >= screen.virtualY
                    && px < screen.virtualX + screen.width && py < screen.virtualY + screen.height)
                return true
        }
        return false
    }

    // Saves the place shortly after a move settles.
    onXChanged: placeTimer.restart()
    onYChanged: placeTimer.restart()
    Timer {
        id: placeTimer
        interval: 500
        onTriggered: {
            if (panel.visible && panel.placed && Qt.platform.pluginName !== "wayland")
                panel.player.panel_moved(panel.x, panel.y)
        }
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
            panel.swallowing = panel.menus.item !== null && panel.menus.item.swallowsPress()
            if (!panel.swallowing)
                panel.player.panel_press(mouse.x, mouse.y, mouse.button)
        }
        onReleased: (mouse) => {
            if (panel.swallowing)
                panel.swallowing = false
            else
                panel.player.panel_release(mouse.x, mouse.y, mouse.button)
        }
        onDoubleClicked: (mouse) => {
            if (!panel.swallowing)
                panel.player.panel_double(mouse.x, mouse.y, mouse.button)
        }
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
        // The style's own content item, as plain text (Tip.qml).
        contentItem: Text {
            text: tip.text
            font: tip.font
            wrapMode: Text.Wrap
            color: tip.palette.toolTipText
            textFormat: Text.PlainText
        }
    }
}
