// One of the skin's other windows (M9): a classic Winamp skin's main
// window, equalizer or playlist. Player lays it out, paints it whole
// (SkinView surface 10 + index) and takes its pointer input; it lies
// docked to the main window (Winamp 5's video window): `above` it, or
// stacked `below` it with the other shown ones, and moves with it. A drag
// on it moves the whole group, as dragging Winamp's main window does; the
// resizable playlist resizes from its edges in its skin's steps. Hidden in
// fullscreen, where the video shows alone.
import QtQuick
import Rvp
import RvpNative

Window {
    id: win
    required property Player player
    required property Window owner
    required property int index
    readonly property var all: player.extra_windows ? JSON.parse(player.extra_windows) : []
    readonly property var info: index < all.length ? all[index] : ({})
    // Where a drag started: the pointer on the screen, the main window.
    property real pressX: 0
    property real pressY: 0
    property bool moving: false
    property real startX: 0
    property real startY: 0
    property real ownerX: 0
    property real ownerY: 0

    // The windows docked below the main window before this one.
    function offsetBelow() {
        let offset = 0
        for (let i = 0; i < index; i++) {
            const other = all[i]
            if (other.shown && other.dock === "below")
                offset += other.height
        }
        return offset
    }

    width: info.width || 1
    height: info.height || 1
    minimumWidth: info.step ? 275 : (info.min ? info.min[0] : width)
    minimumHeight: info.step ? 116 : (info.min ? info.min[1] : height)
    maximumWidth: info.step ? 16384 : (info.max ? info.max[0] : width)
    maximumHeight: info.step ? 16384 : (info.max ? info.max[1] : height)
    // Docked above or below the main window, or (a VLC theme's windows)
    // where the theme puts them beside it.
    x: owner.x + (info.dock === "offset" ? info.offset[0] : 0)
    y: info.dock === "above" ? owner.y - height
       : info.dock === "offset" ? owner.y + info.offset[1]
       : owner.y + owner.height + offsetBelow()
    flags: Qt.Tool | Qt.FramelessWindowHint | (player.on_top ? Qt.WindowStaysOnTopHint : 0)
    transientParent: owner
    color: "black"
    title: "RVP"
    visible: info.shown === true && owner.visible && owner.visibility !== Window.FullScreen
             && owner.visibility !== Window.Minimized

    onVisibleChanged: {
        if (visible) {
            applyShape()
            Tools.repaint(view, false)
        }
    }
    onClosing: (close) => player.extra_closed(index)
    // A classic skin's region.txt, or a VLC theme's drawn pixels.
    function applyShape() {
        if (info.mask)
            Tools.set_window_mask_rects(win, info.mask, 1)
        else
            Tools.set_window_mask(win, info.region || "", 1)
    }
    onInfoChanged: if (visible) applyShape()
    // The classic playlist's sizes go in its skin's steps.
    onWidthChanged: resizeTimer.restart()
    onHeightChanged: resizeTimer.restart()
    Timer {
        id: resizeTimer
        interval: 100
        onTriggered: {
            const step = win.info.step
            if (!step)
                return
            const w = 275 + Math.max(0, Math.round((win.width - 275) / step[0])) * step[0]
            const h = 116 + Math.max(0, Math.round((win.height - 116) / step[1])) * step[1]
            if (w !== win.width || h !== win.height) {
                win.width = w
                win.height = h
            }
            win.player.extra_resized(win.index, w, h)
        }
    }

    // A drag moves the main window, and with it the group.
    function startMove() {
        moving = true
        startX = pressX
        startY = pressY
        ownerX = owner.x
        ownerY = owner.y
    }
    function startResize(edges) {
        win.startSystemResize(edges)
    }

    SkinView {
        id: view
        surface: 10 + win.index
        anchors.fill: parent
    }

    Connections {
        target: win.player
        function onExtraDamaged(changed) {
            if (changed === win.index)
                Tools.repaint(view, false)
        }
    }

    Item {
        focus: true
        Keys.onPressed: (event) => {
            event.accepted = win.player.key(event.key, event.modifiers)
        }
    }

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.AllButtons
        hoverEnabled: true
        onPressed: (mouse) => {
            win.pressX = win.x + mouse.x
            win.pressY = win.y + mouse.y
            win.moving = false
            win.player.extra_press(win.index, mouse.x, mouse.y, mouse.button, mouse.modifiers)
        }
        onReleased: (mouse) => {
            win.moving = false
            win.player.extra_release(win.index, mouse.x, mouse.y, mouse.button)
        }
        onDoubleClicked: (mouse) => win.player.extra_double(win.index, mouse.x, mouse.y, mouse.button)
        onPositionChanged: (mouse) => {
            if (win.moving) {
                const gx = win.x + mouse.x
                const gy = win.y + mouse.y
                win.owner.setGeometry(win.ownerX + gx - win.startX, win.ownerY + gy - win.startY,
                                      win.owner.width, win.owner.height)
                return
            }
            win.player.extra_move(win.index, mouse.x, mouse.y)
        }
        onWheel: (wheel) => win.player.extra_wheel(win.index, wheel.x, wheel.y, wheel.angleDelta.y)
        onExited: win.player.extra_leave(win.index)
    }
}
