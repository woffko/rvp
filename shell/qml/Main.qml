// The RVP window: a frameless window with the skin's static layer, the
// video item, one painted item per dynamic skin region, and the input
// handling, menu and dialogs. Logic lives in Rust (Player); this file only
// wires Qt Quick to it.
import QtQuick
import QtQuick.Window
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import Rvp
import RvpNative

Window {
    id: root
    width: 1280
    height: 720
    minimumWidth: 320
    minimumHeight: 200
    visible: false
    color: "black"
    title: player.window_title
    flags: Qt.Window | Qt.FramelessWindowHint

    // Request kinds of Player.uiRequest (src/player.rs, mod request).
    readonly property int requestOpenFiles: 1
    readonly property int requestFullscreen: 2
    readonly property int requestMinimize: 3
    readonly property int requestMaximizeRestore: 4
    readonly property int requestExit: 5
    readonly property int requestContextMenu: 6
    readonly property int requestStartMove: 7
    readonly property int requestStartResize: 8

    property bool wasMaximized: false
    property bool closing: false
    property bool pointerIdle: false
    readonly property bool isFullscreen: visibility === Window.FullScreen

    Player {
        id: player
        onUiRequest: (kind, argument) => root.handleRequest(kind, argument)
        onDamaged: (staticLayer, regions, synced) => root.repaint(staticLayer, regions, synced)
        onFitWindow: (width, height) => root.fit(width, height)
    }

    Component.onCompleted: {
        player.start()
        if (player.restore_width > 0 && player.restore_height > 0) {
            width = player.restore_width
            height = player.restore_height
        }
        if (player.restore_position && root.onScreen(player.restore_x, player.restore_y)) {
            x = player.restore_x
            y = player.restore_y
        }
        if (player.restore_maximized)
            showMaximized()
        else
            show()
        player.resized(width, height)
        keys.forceActiveFocus()
        if (player.has_option("selftest"))
            selftest.begin()
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

    onWidthChanged: player.resized(width, height)
    onHeightChanged: player.resized(width, height)
    onVisibilityChanged: player.window_state(visibility === Window.Maximized, active)
    onActiveChanged: player.window_state(visibility === Window.Maximized, active)
    onXChanged: geometryTimer.restart()
    onYChanged: geometryTimer.restart()

    // Saves the normal geometry shortly after it settles.
    Timer {
        id: geometryTimer
        interval: 500
        onTriggered: {
            if (root.visibility === Window.Windowed || root.visibility === Window.Maximized)
                player.window_geometry(root.x, root.y, root.width, root.height,
                                       Qt.platform.pluginName !== "wayland")
        }
    }
    Connections {
        target: root
        function onWidthChanged() { geometryTimer.restart() }
        function onHeightChanged() { geometryTimer.restart() }
    }

    Timer {
        interval: 1000
        repeat: true
        running: true
        onTriggered: player.tick()
    }

    onClosing: (close) => {
        if (!root.closing) {
            root.closing = true
            player.shutdown()
        }
    }

    function handleRequest(kind, argument) {
        switch (kind) {
        case requestOpenFiles:
            fileDialog.open()
            break
        case requestFullscreen:
            if (argument) {
                root.wasMaximized = root.visibility === Window.Maximized
                root.showFullScreen()
            } else if (root.wasMaximized) {
                root.showMaximized()
            } else {
                root.showNormal()
            }
            break
        case requestMinimize:
            root.showMinimized()
            break
        case requestMaximizeRestore:
            if (root.visibility === Window.Maximized)
                root.showNormal()
            else
                root.showMaximized()
            break
        case requestExit:
            root.close()
            break
        case requestContextMenu:
            contextMenu.popup()
            break
        case requestStartMove:
            root.startSystemMove()
            break
        case requestStartResize:
            root.startSystemResize(argument)
            break
        }
    }

    // One repaint path for the chrome (goal C rule 1).
    function repaint(staticLayer, regions, synced) {
        if (staticLayer)
            Tools.repaint(staticSkin, synced)
        if (regions.length === 0)
            return
        for (const index of regions.split(",")) {
            const item = regionItems.itemAt(Number(index))
            if (item)
                Tools.repaint(item, synced)
        }
    }

    // A newly opened video at 100 %, within the screen and keeping the
    // video's aspect ratio when it has to shrink.
    function fit(targetWidth, targetHeight) {
        if (root.visibility !== Window.Windowed)
            return
        const screen = root.screen
        const maxWidth = screen ? screen.desktopAvailableWidth : targetWidth
        const maxHeight = screen ? screen.desktopAvailableHeight : targetHeight
        const chromeWidth = root.width - player.video_rect.width
        const chromeHeight = root.height - player.video_rect.height
        let videoWidth = targetWidth - chromeWidth
        let videoHeight = targetHeight - chromeHeight
        const scale = Math.min(1, (maxWidth - chromeWidth) / videoWidth,
                               (maxHeight - chromeHeight) / videoHeight)
        root.width = Math.round(videoWidth * scale + chromeWidth)
        root.height = Math.round(videoHeight * scale + chromeHeight)
        if (screen && Qt.platform.pluginName !== "wayland") {
            root.x = Math.max(screen.virtualX, Math.min(root.x, screen.virtualX + maxWidth - root.width))
            root.y = Math.max(screen.virtualY, Math.min(root.y, screen.virtualY + maxHeight - root.height))
        }
    }

    // The skin's static layer: under the video in the window, over it in
    // fullscreen (where it only holds the bars).
    SkinView {
        id: staticSkin
        region_index: -1
        anchors.fill: parent
        visible: player.has_skin && (!player.fullscreen || player.chrome_shown)
        z: player.fullscreen ? 2 : 0
    }

    MpvItem {
        id: video
        x: player.video_rect.x
        y: player.video_rect.y
        // At least one pixel: the renderer must run to create mpv's render
        // context before files load.
        width: Math.max(1, player.video_rect.width)
        height: Math.max(1, player.video_rect.height)
        z: 1
    }

    Repeater {
        id: regionItems
        model: player.region_count
        delegate: SkinView {
            required property int index
            readonly property rect area: player.region_rect(index, player.layout_serial)
            region_index: index
            x: area.x
            y: area.y
            width: area.width
            height: area.height
            z: 3
            visible: staticSkin.visible
        }
    }

    // Keyboard: every key goes to the bindings.
    Item {
        id: keys
        focus: true
        Keys.onPressed: (event) => {
            event.accepted = player.key(event.key, event.modifiers)
        }
    }

    MouseArea {
        id: pointer
        anchors.fill: parent
        z: 10
        acceptedButtons: Qt.AllButtons
        hoverEnabled: true
        cursorShape: root.pointerIdle && player.fullscreen ? Qt.BlankCursor : player.cursor_shape
        onPressed: (mouse) => {
            keys.forceActiveFocus()
            player.mouse_press(mouse.x, mouse.y, mouse.button)
        }
        onReleased: (mouse) => player.mouse_release(mouse.x, mouse.y, mouse.button)
        onDoubleClicked: (mouse) => player.mouse_double(mouse.x, mouse.y, mouse.button)
        onPositionChanged: (mouse) => {
            root.pointerIdle = false
            idleTimer.restart()
            tipTimer.restart()
            tip.close()
            player.mouse_move(mouse.x, mouse.y)
        }
        onWheel: (wheel) => player.mouse_wheel(wheel.angleDelta.y)
        onExited: {
            tip.close()
            player.mouse_leave()
        }
    }

    // Fullscreen hides the pointer after two idle seconds.
    Timer {
        id: idleTimer
        interval: 2000
        onTriggered: root.pointerIdle = true
    }

    // Tooltips on skin controls (section 4).
    Timer {
        id: tipTimer
        interval: 600
        onTriggered: {
            if (player.tooltip !== "") {
                tip.x = Math.min(pointer.mouseX + 12, root.width - tip.implicitWidth)
                tip.y = pointer.mouseY + 20
                tip.open()
            }
        }
    }
    ToolTip {
        id: tip
        text: player.tooltip
        popupType: Popup.Window
        delay: 0
        timeout: 5000
    }

    DropArea {
        anchors.fill: parent
        onDropped: (drop) => {
            if (!drop.hasUrls)
                return
            const paths = []
            for (const url of drop.urls)
                paths.push(Tools.local_file(url))
            player.open_files(paths.join("\n"), false)
            drop.accept()
        }
    }

    FileDialog {
        id: fileDialog
        title: "Open File(s)"
        fileMode: FileDialog.OpenFiles
        onAccepted: {
            const paths = []
            for (const url of selectedFiles)
                paths.push(Tools.local_file(url))
            player.open_files(paths.join("\n"), false)
        }
    }

    // Non-blocking messages (section 4: no modal interruptions).
    Rectangle {
        visible: player.message !== ""
        z: 20
        anchors.horizontalCenter: parent.horizontalCenter
        y: 40
        width: Math.min(parent.width - 40, messageText.implicitWidth + 24)
        height: messageText.implicitHeight + 12
        radius: 4
        color: "#cc202028"
        Text {
            id: messageText
            anchors.centerIn: parent
            width: Math.min(implicitWidth, root.width - 64)
            elide: Text.ElideRight
            color: "#e8e8f0"
            text: player.message
        }
        Timer {
            running: parent.visible
            interval: 5000
            onTriggered: player.message = ""
        }
    }

    // A menu entry for an action: PotPlayer's text and the bound key.
    component ActionItem: MenuItem {
        id: entry
        property string actionName
        text: player.menu_label(actionName)
        onTriggered: player.run_action(actionName)
        contentItem: RowLayout {
            spacing: 24
            Label {
                text: entry.text
                Layout.fillWidth: true
            }
            Label {
                text: player.menu_shortcut(entry.actionName)
                opacity: 0.7
            }
        }
    }

    Menu {
        id: contextMenu
        popupType: Popup.Window
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

    // --selftest: a scripted functional check that prints check=PASS/FAIL
    // lines and the usability measurements, then exits with the number of
    // failures (0 = pass). Input goes through the same handlers as real
    // input (Player.key, Player.mouse_*), never through the window system.
    QtObject {
        id: selftest
        property int step: 0
        property real stepStarted: 0
        property int failures: 0
        property var before: ({})
        readonly property string outDir: player.option("out") !== "" ? player.option("out") : "rvp-selftest"

        function begin() {
            Tools.log("environment=" + Tools.environment_json())
            testTimer.start()
        }
        function status() { return JSON.parse(player.status()) }
        function check(name, ok, detail) {
            Tools.log("check " + name + "=" + (ok ? "PASS" : "FAIL") + (detail ? " " + detail : ""))
            if (!ok)
                failures += 1
        }
        function next() {
            step += 1
            stepStarted = Date.now()
        }
        // Waits for `condition`; on timeout records a failure. Returns true
        // when the step is over.
        function waitFor(name, condition, timeoutMs, detail) {
            if (condition()) {
                check(name, true, detail ? detail() : "")
                next()
                return true
            }
            if (Date.now() - stepStarted > timeoutMs) {
                check(name, false, "timed out" + (detail ? " " + detail() : ""))
                next()
                return true
            }
            return false
        }
        function finish() {
            Tools.grab(outDir, "final")
            for (const line of player.metrics().split("\n"))
                if (line !== "")
                    Tools.log("metric " + line)
            Tools.log("status=" + player.status())
            Tools.log("failures=" + failures)
            testTimer.stop()
            root.closing = true
            player.shutdown()
            Qt.exit(failures === 0 ? 0 : 1)
        }
    }

    Timer {
        id: offscreenPump
        // Windows session 0 never exposes windows: render them by grabbing.
        interval: 40
        repeat: true
        running: testTimer.running
        onTriggered: Tools.render_offscreen()
    }

    Timer {
        id: testTimer
        interval: 50
        repeat: true
        onTriggered: root.selftestTick()
    }

    function selftestTick() {
        const t = selftest
        const s = t.status()
        switch (t.step) {
        case 0:
            t.stepStarted = Date.now()
            t.next()
            break
        case 1:
            t.waitFor("file_playing", () => s.loaded && s.video_width > 0 && s.time > 0.5, 15000,
                      () => "time " + s.time + " ao " + s.current_ao + " hwdec " + s.hwdec_current)
            break
        case 2:
            Tools.grab(t.outDir, "playing")
            t.before = s
            player.run_action("seek-forward-short")
            t.next()
            break
        case 3:
            t.waitFor("seek_forward_short", () => s.time >= t.before.time + 4, 3000,
                      () => "from " + t.before.time + " to " + s.time)
            break
        case 4:
            player.key(Qt.Key_Space, 0)
            t.next()
            break
        case 5:
            t.waitFor("key_space_pauses", () => s.pause === true, 2000)
            break
        case 6:
            player.key(Qt.Key_Space, 0)
            t.next()
            break
        case 7:
            t.waitFor("key_space_resumes", () => s.playing, 2000)
            break
        case 8:
            t.before = s
            player.mouse_wheel(-120)
            t.next()
            break
        case 9:
            t.waitFor("wheel_lowers_volume", () => s.volume !== null && s.volume < t.before.volume, 2000,
                      () => t.before.volume + " -> " + s.volume)
            break
        case 10:
            player.key(Qt.Key_M, 0)
            t.next()
            break
        case 11:
            t.waitFor("key_m_mutes", () => s.mute === true, 2000)
            break
        case 12:
            player.run_action("toggle-mute")
            t.next()
            break
        case 13:
            t.waitFor("unmuted", () => s.mute === false, 2000)
            break
        case 14:
            if (s.playlist_count > 1) {
                player.key(Qt.Key_PageDown, 0)
                t.next()
            } else {
                Tools.log("check next_file=SKIPPED one file")
                t.step = 18
            }
            break
        case 15:
            t.waitFor("key_pgdn_next_file", () => s.playlist_pos === 1, 5000)
            break
        case 16:
            player.key(Qt.Key_PageUp, 0)
            t.next()
            break
        case 17:
            t.waitFor("key_pgup_previous_file", () => s.playlist_pos === 0 && s.loaded, 5000)
            break
        case 18:
            player.key(Qt.Key_Return, 0)
            t.next()
            break
        case 19:
            t.waitFor("enter_fullscreen", () => root.isFullscreen && s.fullscreen, 3000,
                      () => "visibility " + root.visibility)
            break
        case 20:
            Tools.grab(t.outDir, "fullscreen")
            player.mouse_double(root.width / 2, root.height / 2, Qt.LeftButton)
            t.next()
            break
        case 21:
            t.waitFor("double_click_leaves_fullscreen", () => !root.isFullscreen && !s.fullscreen, 3000,
                      () => "visibility " + root.visibility)
            break
        case 22:
            root.showNormal()
            root.width = 900
            root.height = 500
            t.next()
            break
        case 23:
            t.waitFor("relayout_on_resize", () => player.video_rect.width > 0
                          && player.video_rect.x + player.video_rect.width <= root.width
                          && player.video_rect.y + player.video_rect.height <= root.height
                          && root.width === 900, 3000,
                      () => "video " + player.video_rect.width + "x" + player.video_rect.height
                          + " window " + root.width + "x" + root.height)
            break
        case 24:
            player.seek_fraction(0.5)
            t.next()
            break
        case 25:
            t.waitFor("seek_bar_to_middle", () => s.duration > 0 && Math.abs(s.time - s.duration / 2) < 2,
                      3000, () => "time " + s.time + " of " + s.duration)
            break
        case 26:
            player.run_action("toggle-playlist")
            t.next()
            break
        case 27:
            if (t.waitFor("playlist_toggles", () => s.playlist_attached === false || !player.has_skin, 2000))
                player.run_action("toggle-playlist")
            break
        case 28:
            // Let a few seconds of playback run for the measurements.
            if (Date.now() - t.stepStarted > 2000)
                t.finish()
            break
        }
    }
}
