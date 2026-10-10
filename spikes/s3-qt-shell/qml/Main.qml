import QtQuick
import QtQuick.Window
import QtQuick.Controls
import RvpSpike
import RvpSpikeNative

Window {
    id: root
    width: 960
    height: 540
    minimumWidth: 120
    minimumHeight: 40
    visible: true
    color: "black"
    title: "RVP spike S3"
    flags: Qt.Window | Qt.FramelessWindowHint

    readonly property var args: Qt.application.arguments
    function hasArg(name) { return args.indexOf(name) >= 0 }
    function argValue(name, fallback) {
        const index = args.indexOf(name)
        return index >= 0 && index + 1 < args.length ? args[index + 1] : fallback
    }

    readonly property bool selftest: hasArg("--selftest")
    // Outside the self-test, --width/--height set the initial size (the
    // host input check starts at 440x77).
    Component.onCompleted: {
        if (!selftest && hasArg("--width")) {
            width = Number(argValue("--width", "960"))
            height = Number(argValue("--height", "540"))
        }
    }
    // --wid: mpv draws into a native child window instead of the OpenGL item.
    readonly property bool widMode: hasArg("--wid")
    // --chrome full|partial|layered|synced|none: how the skin is repainted
    // once per second during playback: the whole item, the time label's
    // rectangle of the whole item, a separate small item that holds only
    // the time label (the big item is left alone), that small item updated
    // together with the next video frame, or not at all.
    readonly property string chromeMode: argValue("--chrome", "full")
    readonly property string outDir: argValue("--out", "s3-out")
    property int step: 0
    property int failures: 0

    function check(name, ok, detail) {
        Tools.log(name + "=" + (ok ? "PASS" : "FAIL") + (detail ? " " + detail : ""))
        if (!ok) failures += 1
    }

    SkinItem {
        id: skin
        anchors.fill: parent
        skin_path: root.argValue("--skin", "")
        onWidthChanged: relayout(width, height)
        onHeightChanged: relayout(width, height)
        Component.onCompleted: relayout(width, height)
    }

    // The time label as its own layer, for --chrome layered.
    SkinItem {
        id: timeLayer
        visible: root.chromeMode === "layered" || root.chromeMode === "synced"
        z: 1
        skin_path: skin.skin_path
        region: skin.time_rect
        x: skin.time_rect.x
        y: skin.time_rect.y
        width: skin.time_rect.width
        height: skin.time_rect.height
    }
    MpvItem {
        id: video
        visible: !root.widMode
        x: skin.video_rect.width > 0 ? skin.video_rect.x : 0
        y: skin.video_rect.width > 0 ? skin.video_rect.y : 0
        width: skin.video_rect.width > 0 ? skin.video_rect.width : root.width
        height: skin.video_rect.width > 0 ? skin.video_rect.height : root.height
    }

    MpvController {
        id: controller
    }

    // Title bar: drag moves the window through the compositor.
    MouseArea {
        anchors { left: parent.left; right: parent.right; top: parent.top }
        height: 32
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onPressed: (mouse) => {
            if (mouse.button === Qt.RightButton) {
                menu.popup()
            } else {
                Tools.log("start_system_move=" + root.startSystemMove())
            }
        }
    }

    // Bottom-right grip: drag resizes the window through the compositor.
    MouseArea {
        anchors { right: parent.right; bottom: parent.bottom }
        width: 16
        height: 16
        cursorShape: Qt.SizeFDiagCursor
        onPressed: Tools.log("start_system_resize=" + root.startSystemResize(Qt.RightEdge | Qt.BottomEdge))
    }

    // Right click anywhere else opens the context menu. With --log-pointer a
    // middle click shrinks the window to the 440x77 of the menu test (the
    // input check uses the mouse, never keys: keys would go to whichever
    // window has focus, possibly the terminal that runs the check).
    MouseArea {
        anchors.fill: parent
        z: -1
        acceptedButtons: Qt.RightButton | Qt.MiddleButton
        onPressed: (mouse) => {
            if (mouse.button === Qt.MiddleButton) {
                if (root.hasArg("--log-pointer")) {
                    root.width = 440
                    root.height = 77
                }
            } else {
                menu.popup()
            }
        }
    }

    onWidthChanged: {
        if (hasArg("--log-geometry")) Tools.log("window_size=" + width + "x" + height)
    }
    onHeightChanged: {
        if (hasArg("--log-geometry")) Tools.log("window_size=" + width + "x" + height)
    }

    Menu {
        id: menu
        popupType: Popup.Window
        onOpened: if (root.hasArg("--log-geometry")) menuLog.restart()
        MenuItem { text: "Open File(s)...\tF3" }
        Menu {
            title: "Open"
            MenuItem { text: "Open Folder..." }
            MenuItem { text: "Open URL..." }
        }
        MenuItem { text: "Close Playback\tF4" }
        MenuSeparator {}
        Menu {
            title: "Playback"
            MenuItem { text: "Play/Pause\tSpace" }
            MenuItem { text: "Stop" }
        }
        MenuItem { text: "Subtitles" }
        MenuItem { text: "Video" }
        MenuItem { text: "Audio" }
        MenuItem { text: "Filters" }
        MenuItem { text: "Skins" }
        MenuSeparator {}
        MenuItem { text: "Fullscreen (Keep AR)\tEnter" }
        MenuItem { text: "Preferences...\tF5" }
        MenuItem { text: "Playlist...\tF6" }
        MenuSeparator {}
        MenuItem { text: "Exit\tAlt+F4"; onTriggered: Qt.quit() }
    }

    // The popup window maps after `opened`; log the window list a little later.
    Timer {
        id: menuLog
        interval: 400
        onTriggered: Tools.log("menu_windows=" + Tools.windowsJson())
    }

    Timer {
        interval: 700
        repeat: true
        running: root.selftest
        onTriggered: root.runStep()
    }

    // True while the video plays. Offscreen grabs (for session 0, where
    // windows are never exposed) only apply to the OpenGL item path.
    property bool videoActive: false
    readonly property bool pumpFrames: videoActive && !widMode
    property int offscreenRenders: 0
    Timer {
        interval: 40
        repeat: true
        running: root.pumpFrames
        onTriggered: root.offscreenRenders += Tools.renderOffscreen()
    }

    readonly property real videoSeconds: Number(argValue("--video-seconds", "6"))
    readonly property bool gpuChecks: hasArg("--gpu-checks")

    // Repaint the skin once per second while the video plays, as the time
    // display will in the real player.
    Timer {
        interval: 1000
        repeat: true
        running: root.videoActive
        onTriggered: {
            if (root.chromeMode === "full")
                skin.update()
            else if (root.chromeMode === "partial")
                skin.repaint_time()
            else if (root.chromeMode === "layered")
                timeLayer.update()
            else if (root.chromeMode === "synced")
                Tools.updateWithNextFrame(timeLayer)
            controller.refresh()
        }
    }

    Timer {
        id: videoPhase
        interval: root.videoSeconds * 1000
        onTriggered: root.finish()
    }

    // --log-pointer: report pointer positions and sizes so an external
    // input injector can prove that system move and resize work.
    MouseArea {
        anchors.fill: parent
        z: -2
        enabled: root.hasArg("--log-pointer")
        hoverEnabled: true
        acceptedButtons: Qt.NoButton
        onPositionChanged: (mouse) => Tools.log("pointer_local=" + Math.round(mouse.x) + "," + Math.round(mouse.y) + " size=" + root.width + "x" + root.height)
    }

    function runStep() {
        step += 1
        if (step === 1) {
            Tools.log("environment=" + Tools.environmentJson())
            Tools.log("graphics_api=" + skin.GraphicsInfo.api + " (OpenGL=" + GraphicsInfo.OpenGL + ")")
            Tools.log("skin_status=" + skin.status)
            check("frameless", (root.flags & Qt.FramelessWindowHint) !== 0)
            root.width = 440
            root.height = 77
        } else if (step === 2) {
            menu.popup(10, 50)
        } else if (step === 3) {
            const windows = JSON.parse(Tools.windowsJson())
            Tools.log("windows=" + JSON.stringify(windows))
            let popup = null
            for (const w of windows) {
                if (w.visible && w.title !== root.title) popup = w
            }
            if (popup === null && Qt.platform.pluginName === "wayland") {
                // Wayland compositors grab popups only in response to real
                // input, so a scripted popup() is refused. The kit's input
                // check opens the menu with a real right click instead.
                Tools.log("menu_window_created=SKIPPED Wayland needs real input (see the input check)")
            } else {
                check("menu_window_created", popup !== null)
            }
            if (popup !== null) {
                check("menu_extends_beyond_window", popup.height > root.height,
                      "menu " + popup.width + "x" + popup.height + " main " + root.width + "x" + root.height)
            }
            Tools.grabAll(outDir, "menu")
        } else if (step === 4) {
            menu.close()
            // Without a pressed mouse button the compositor may refuse: this
            // only shows the API is reachable. Real drags need real input.
            Tools.log("start_system_move_without_press=" + root.startSystemMove())
            Tools.log("start_system_resize_without_press=" + root.startSystemResize(Qt.RightEdge | Qt.BottomEdge))
            root.width = Number(argValue("--width", "960"))
            root.height = Number(argValue("--height", "540"))
        } else if (step === 5) {
            Tools.log("video_rect=" + skin.video_rect.x + "," + skin.video_rect.y + " " + skin.video_rect.width + "x" + skin.video_rect.height)
            root.videoActive = true
            timeLayer.relayout(root.width, root.height)
            const source = argValue("--video", "av://lavfi:testsrc2=size=1280x720:rate=24000/1001")
            if (widMode) {
                const wid = Tools.createChildWindow(video.x, video.y, video.width, video.height)
                Tools.log("video_output=wid " + wid)
                controller.start_wid(source, argValue("--hwdec", "no"), wid)
            } else {
                Tools.log("video_output=opengl item chrome=" + chromeMode)
                controller.start(source, argValue("--hwdec", "no"))
            }
            videoPhase.start()
        }
    }

    function finish() {
        controller.refresh()
        Tools.log("mpv_status=" + controller.status)
        Tools.log("mpv_hwdec_current=" + controller.hwdec_current)
        Tools.log("mpv_events_on_qt_thread=" + controller.events_on_qt_thread)
        Tools.log("mpv_estimated_vf_fps=" + controller.estimated_vf_fps)
        Tools.log("mpv_time_pos=" + controller.time_pos)
        Tools.log("mpv_frames_rendered=" + controller.frames_rendered)
        Tools.log("mpv_frame_drop_count=" + controller.frame_drop_count)
        Tools.log("mpv_vo_delayed_frame_count=" + controller.vo_delayed_frame_count)
        Tools.log("mpv_mistimed_frame_count=" + controller.mistimed_frame_count)
        Tools.log("mpv_decoder_frame_drop_count=" + controller.decoder_frame_drop_count)
        Tools.log("mpv_estimated_display_fps=" + controller.estimated_display_fps)
        Tools.log("mpv_vsync_jitter=" + controller.vsync_jitter)
        Tools.log("mpv_display_fps=" + controller.display_fps)
        Tools.log("screen_refresh_hz=" + Tools.refreshRate())
        Tools.log("skin_paint_stats=" + skin.paint_stats())
        Tools.log("time_layer_paint_stats=" + timeLayer.paint_stats())
        Tools.log("time_rect=" + skin.time_rect.x + "," + skin.time_rect.y + " " + skin.time_rect.width + "x" + skin.time_rect.height)
        Tools.log("mpv_current_vo=" + controller.current_vo)
        Tools.log("skin_ops_painted=" + skin.ops_painted)
        Tools.log("offscreen_renders=" + root.offscreenRenders)
        check("wakeup_callback_reached_qt_thread", controller.events_on_qt_thread > 0)
        check("video_playing", controller.time_pos > 1, "time_pos " + controller.time_pos)
        if (!widMode)
            check("video_frames_rendered", controller.frames_rendered > 10)
        check("skin_painted", skin.ops_painted > 50)
        if (gpuChecks) {
            const expected = controller.estimated_vf_fps * root.videoSeconds * 0.9
            if (!widMode)
                check("gpu_frames_kept_up", controller.frames_rendered >= expected,
                      "rendered " + controller.frames_rendered + " expected >= " + Math.round(expected))
            check("gpu_no_decoder_drops", controller.decoder_frame_drop_count === 0,
                  "decoder drops " + controller.decoder_frame_drop_count)
            check("gpu_no_dropped_frames", controller.frame_drop_count === 0,
                  "drops " + controller.frame_drop_count)
        } else {
            Tools.log("gpu_checks=SKIPPED")
        }
        Tools.grabAll(outDir, "shell")
        Tools.log("failures=" + failures)
        Tools.quit(failures === 0 ? 0 : 1)
    }
}
