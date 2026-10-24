// The RVP window: a frameless window with the skin's static layer, the
// video item, one painted item per dynamic skin region, and the input
// handling, menu and dialogs. Logic lives in Rust (Player); this file only
// wires Qt Quick to it.
import QtQuick
import QtQuick.Window
import Rvp
import RvpNative

Window {
    id: root
    // The media controls find the main window by this name (M8).
    objectName: "rvpMain"
    width: 1280
    height: 720
    // The skin's limits (M14); 0 until the skin is laid out.
    minimumWidth: player.min_width > 0 ? player.min_width : 320
    minimumHeight: player.min_height > 0 ? player.min_height : 200
    visible: false
    color: "black"
    flags: Qt.Window | Qt.FramelessWindowHint | (player.on_top ? Qt.WindowStaysOnTopHint : 0)
           | (root.startNoFocus ? Qt.WindowDoesNotAcceptFocus : 0)
    // Skins > Opacity (M14); 0 until the core starts.
    opacity: player.window_opacity > 0 ? player.window_opacity : 1

    // Request kinds of Player.uiRequest (src/player.rs, mod request).
    readonly property int requestOpenFiles: 1
    readonly property int requestFullscreen: 2
    readonly property int requestMinimize: 3
    readonly property int requestMaximizeRestore: 4
    readonly property int requestExit: 5
    readonly property int requestContextMenu: 6
    readonly property int requestStartMove: 7
    readonly property int requestStartResize: 8
    readonly property int requestSettings: 9
    readonly property int requestAddFiles: 10
    readonly property int requestAddFolder: 11
    readonly property int requestImportPlaylists: 12
    readonly property int requestListMenu: 13
    readonly property int requestRaise: 14
    readonly property int requestAlbumName: 15
    readonly property int requestSkinMenu: 16
    readonly property int requestOpenFolder: 17
    readonly property int requestOpenUrl: 18
    readonly property int requestOpenSubtitle: 19
    readonly property int requestImageFolder: 20
    readonly property int requestOpenExternal: 21
    readonly property int requestAbout: 22
    readonly property int requestPanelMove: 23
    readonly property int requestSaveFrameAs: 24
    readonly property int requestSubtitleFont: 25
    readonly property int requestAudioSync: 26
    readonly property int requestSubtitleSync: 37
    readonly property int requestSubtitleEditor: 38
    readonly property int requestColorTheme: 39
    readonly property int requestSystemInfo: 40
    readonly property int requestDropMenu: 41
    readonly property int requestRecordAs: 27
    readonly property int requestRecordFolder: 28
    readonly property int requestDialog: 29
    readonly property int requestCopyText: 30
    readonly property int requestReadClipboard: 31
    readonly property int requestShowFile: 32
    readonly property int requestScenes: 33
    readonly property int requestVideoDialog: 34
    readonly property int requestWindow: 35
    readonly property int requestSlide: 36

    property bool wasMaximized: false
    // The window state RVP last asked for and when (see applyVisibility).
    property int desiredVisibility: Window.Windowed
    property real stateRequested: 0
    // --log-pointer and --log-geometry: report pointer positions, window
    // sizes and the menu's windows for the kit's input check.
    property bool logPointer: false
    property bool logGeometry: false
    property bool closing: false
    property bool pointerIdle: false
    readonly property bool isFullscreen: visibility === Window.FullScreen

    Player {
        id: player
        onUiRequest: (kind, argument) => root.handleRequest(kind, argument)
        onDamaged: (staticLayer, regions, synced) => root.repaint(staticLayer, regions, synced)
        onFitWindow: (width, height) => root.fit(width, height)
    }

    // RVP as the screen saver (M14, --screensaver): any key, click or
    // move of the mouse ends it.
    // Set once the player has read the command line.
    property bool screensaver: false
    MouseArea {
        anchors.fill: parent
        visible: root.screensaver
        z: 100
        hoverEnabled: true
        cursorShape: Qt.BlankCursor
        property point first: Qt.point(-1, -1)
        onPositionChanged: (mouse) => {
            if (first.x < 0)
                first = Qt.point(mouse.x, mouse.y)
            else if (Math.abs(mouse.x - first.x) + Math.abs(mouse.y - first.y) > 12)
                Qt.quit()
        }
        onPressed: Qt.quit()
        onWheel: Qt.quit()
    }

    // The General page's window options (M14).
    readonly property var generalLook: player.general_look ? JSON.parse(player.general_look) : ({})
    // While a start with files must not take the keyboard (M14).
    property bool startNoFocus: false
    onGeneralLookChanged: {
        Tools.set_app_font(generalLook.dialogFont || "", generalLook.dialogFontSize || 0)
        applyLanguage()
    }
    // General: "Interface language" (M14): RVP's own translations
    // (i18n/qml_<language>.qm beside this file), which the engine loads
    // when Qt.uiLanguage changes; "auto" takes the system's when RVP has it.
    function applyLanguage() {
        const wanted = generalLook.language || "auto"
        const language = wanted === "auto" ? (Qt.locale().name.startsWith("ru") ? "ru" : "en") : wanted
        // English is the source text: the engine drops its translation only
        // for an empty language (a language without a file keeps the last
        // one, and at start it loads the system's).
        const ui = language === "en" ? "" : language
        if (Qt.uiLanguage !== ui)
            Qt.uiLanguage = ui
    }

    Component.onCompleted: {
        player.start()
        screensaver = player.has_option("screensaver")
        // Requests from other instances (rvp /add ...) go to the player.
        Tools.instance_receiver(player)
        if (player.restore_width > 0 && player.restore_height > 0) {
            width = player.restore_width
            height = player.restore_height
        }
        if (player.restore_position && root.onScreen(player.restore_x, player.restore_y)) {
            x = player.restore_x
            y = player.restore_y
        } else if (generalLook.startCenter === true && root.screen) {
            x = root.screen.virtualX + Math.round((root.screen.desktopAvailableWidth - width) / 2)
            y = root.screen.virtualY + Math.round((root.screen.desktopAvailableHeight - height) / 2)
        }
        if (generalLook.noFocusAtStart === true && generalLook.startFiles === true) {
            startNoFocus = true
            noFocusTimer.start()
        }
        // The screen saver (M14) plays in fullscreen; its page opens for
        // Windows' screen saver settings.
        if (root.screensaver)
            showFullScreen()
        else if (player.restore_maximized)
            showMaximized()
        else
            show()
        if (player.has_option("screensaver-settings")) {
            settingsLoader.active = true
            settingsLoader.item.begin("screensaver")
        }
        player.resized(width, height)
        player.set_screen(Tools.screen_json(root))
        logPointer = player.has_option("log-pointer")
        logGeometry = player.has_option("log-geometry")
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

    onWidthChanged: {
        player.resized(width, height)
        aspectTimer.restart()
        if (logGeometry)
            Tools.log("window_size=" + width + "x" + height)
    }
    onHeightChanged: {
        player.resized(width, height)
        aspectTimer.restart()
        if (logGeometry)
            Tools.log("window_size=" + width + "x" + height)
    }

    // Once a resize settles, the window takes the video's shape (R1
    // finding 1); during a drag the size changes faster than this.
    Timer {
        id: aspectTimer
        interval: 300
        onTriggered: {
            if (root.visibility === Window.Windowed)
                player.fit_aspect()
        }
    }
    // Accessibility (M14): notices, text subtitles and tooltips read aloud,
    // announced to screen readers and shown as the window's title.
    readonly property var accessibilityLook: player.accessibility_look ? JSON.parse(player.accessibility_look) : ({})
    property string titleOverride: ""
    title: titleOverride !== "" ? titleOverride : player.window_title
    function present(kind, text) {
        const look = root.accessibilityLook
        if (text === "")
            return
        if (look.tts && look.tts[kind] === true)
            player.speak(text)
        if (look.screenReader && look.screenReader[kind] === true)
            accessibleNotice.Accessible.announce(text)
        if (look.title && look.title[kind] === true) {
            root.titleOverride = text.replace(/\n/g, " ")
            titleTimer.restart()
        }
    }
    // Accessibility > Touch (M14): a finger's swipe over the video.
    Item {
        x: player.video_rect.x
        y: player.video_rect.y
        width: player.video_rect.width
        height: player.video_rect.height
        DragHandler {
            target: null
            enabled: root.accessibilityLook.touch === true
            acceptedDevices: PointerDevice.TouchScreen
            onActiveChanged: {
                if (!active)
                    player.touch_swipe(translation.x / Math.max(1, parent.width),
                                       translation.y / Math.max(1, parent.height))
            }
        }
    }
    Item {
        id: accessibleNotice
        Accessible.role: Accessible.StaticText
    }
    Timer {
        id: titleTimer
        interval: 3000
        onTriggered: root.titleOverride = ""
    }
    Connections {
        target: player
        function onOsdChanged() { root.present("messages", player.osd) }
        function onSub_textChanged() { root.present("subtitles", player.sub_text) }
        function onTooltipChanged() { root.present("tooltips", player.tooltip) }
    }

    // Windows' taskbar buttons and the screen lock (M14's General page).
    Connections {
        target: Tools
        function onTaskbarButton(button) {
            player.run_action(["previous", "play-pause", "next"][button])
        }
        function onMediaKey(key) {
            player.run_action(["play-pause", "close-media", "next", "previous"][key])
        }
        function onSessionLocked(locked) {
            player.window_attention(2, locked)
        }
        // Accessibility > XInput (M14): RVP's buttons.
        function onGamepadButton(button) {
            const actions = ["play-pause", "toggle-fullscreen", "toggle-mute", "playback-info", "previous", "next",
                             "close-media", "play-pause", "volume-up", "volume-down",
                             "seek-backward-medium", "seek-forward-medium"]
            if (button >= 0 && button < actions.length)
                player.run_action(actions[button])
        }
    }
    // General: the tray icon (Qt's), loaded when the page asks for it.
    Loader {
        id: trayLoader
        active: root.generalLook.tray === "both" || root.generalLook.tray === "minimize-to-tray"
        Component.onCompleted: setSource("Tray.qml", { player: player, owner: root })
    }

    // A start with files leaves the keyboard where it was for a moment.
    Timer {
        id: noFocusTimer
        interval: 1500
        onTriggered: root.startNoFocus = false
    }
    onVisibilityChanged: {
        // General: pause while minimized (M14).
        player.window_attention(0, root.visibility === Window.Minimized)
        player.window_state(root.visibility === Window.Maximized, root.active)
        if (Date.now() - root.stateRequested < 2000)
            stateCheck.restart()
        else if (root.isFullscreen !== player.fullscreen && root.visibility !== Window.Minimized
                 && root.visibility !== Window.Hidden)
            player.window_fullscreen(root.isFullscreen)
    }
    onActiveChanged: {
        // General: pause while the focus is elsewhere (M14).
        player.window_attention(1, root.active)
        player.window_state(root.visibility === Window.Maximized, root.active)
        player.confine_cursor(root.isFullscreen && root.active)
    }
    onIsFullscreenChanged: player.confine_cursor(root.isFullscreen && root.active)
    onXChanged: {
        geometryTimer.restart()
        snapTimer.restart()
        if (logGeometry)
            Tools.log("window_place=" + x + "," + y)
    }
    onYChanged: {
        geometryTimer.restart()
        snapTimer.restart()
        if (logGeometry)
            Tools.log("window_place=" + x + "," + y)
    }

    // General: a moved window snaps to the screen's edges (M14).
    Timer {
        id: snapTimer
        interval: 400
        onTriggered: root.snapToEdges()
    }
    function snapToEdges() {
        if (root.generalLook.snap !== true || root.visibility !== Window.Windowed
                || Qt.platform.pluginName === "wayland")
            return
        const area = root.screenArea()
        const reach = 16
        let x = root.x
        let y = root.y
        if (Math.abs(x - area.x) < reach)
            x = area.x
        else if (Math.abs(area.x + area.width - x - root.width) < reach)
            x = area.x + area.width - root.width
        if (Math.abs(y - area.y) < reach)
            y = area.y
        else if (Math.abs(area.y + area.height - y - root.height) < reach)
            y = area.y + area.height - root.height
        if (x !== root.x || y !== root.y)
            root.setWindowGeometry(x, y, root.width, root.height, "snap")
    }

    // The skin's spectrum (M14): 25 times a second while it shows.
    Timer {
        interval: 40
        repeat: true
        running: player.spectrum_shown && root.visibility !== Window.Minimized
            && root.visibility !== Window.Hidden
        onTriggered: player.tick_spectrum()
    }

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
        triggeredOnStart: true
        onTriggered: {
            player.tick()
            // The skin's clock (CMD_SYSTEM_TIME_MESSAGE).
            player.set_clock(Qt.formatTime(new Date(), "hh:mm:ss"))
        }
    }
    Timer {
        interval: 100
        repeat: true
        running: true
        onTriggered: player.flush_chrome()
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
        case requestAddFiles:
            dialogLoader.active = true
            dialogLoader.item.ask(kind === requestAddFiles)
            break
        case requestAddFolder:
        case requestImportPlaylists:
            folderLoader.active = true
            folderLoader.item.ask(kind === requestImportPlaylists ? "import" : "add")
            break
        case requestListMenu:
            menuLoader.active = true
            menuLoader.item.show("playlist")
            break
        case requestSkinMenu:
            menuLoader.active = true
            menuLoader.item.show("skin")
            break
        case requestOpenFolder:
        case requestImageFolder:
            folderLoader.active = true
            folderLoader.item.ask(kind === requestOpenFolder ? "open" : "images")
            break
        case requestOpenUrl:
            urlLoader.active = true
            urlLoader.item.ask()
            break
        case requestOpenSubtitle:
            dialogLoader.active = true
            dialogLoader.item.askSubtitle(argument === 1)
            break
        case requestOpenExternal:
            Qt.openUrlExternally(player.external_url)
            break
        case requestAbout:
            aboutLoader.active = true
            aboutLoader.item.ask()
            break
        case requestPanelMove:
            if (panelLoader.item)
                panelLoader.item.startMove()
            break
        case requestSaveFrameAs:
            dialogLoader.active = true
            dialogLoader.item.askFrame(player.frame_path)
            break
        case requestSubtitleFont:
            dialogLoader.active = true
            dialogLoader.item.askFont(player.subtitle_font)
            break
        case requestAudioSync:
            syncLoader.active = true
            syncLoader.item.ask(argument)
            break
        case requestSubtitleSync:
            subtitleSyncLoader.active = true
            subtitleSyncLoader.item.ask(argument)
            break
        case requestSubtitleEditor:
            subtitleEditorLoader.active = true
            subtitleEditorLoader.item.ask(argument === 1)
            break
        case requestColorTheme:
            colorThemeLoader.active = true
            colorThemeLoader.item.ask()
            break
        case requestSystemInfo:
            systemInfoLoader.active = true
            systemInfoLoader.item.begin()
            break
        case requestDropMenu:
            dropMenuLoader.active = true
            dropMenuLoader.item.ask(player.dialog)
            break
        case requestRecordAs:
            dialogLoader.active = true
            dialogLoader.item.askRecording(player.record_path)
            break
        case requestRecordFolder:
            folderLoader.active = true
            folderLoader.item.ask("recordings")
            break
        case requestAlbumName:
            albumLoader.active = true
            albumLoader.item.ask(argument === 1, player.album_name)
            break
        case requestSettings:
            settingsLoader.active = true
            settingsLoader.item.begin(player.settings_page)
            break
        case requestRaise:
            if (root.visibility === Window.Minimized)
                root.showNormal()
            root.raise()
            root.requestActivate()
            break
        case requestFullscreen:
            if (argument) {
                root.wasMaximized = root.visibility === Window.Maximized
                // Fullscreen Mode's "Display: primary" (M12).
                if (player.fullscreen_primary)
                    root.placeOnScreen(Qt.application.screens[0])
                root.requestVisibility(Window.FullScreen)
            } else {
                root.requestVisibility(root.wasMaximized ? Window.Maximized : Window.Windowed)
            }
            break
        case requestMinimize:
            root.showMinimized()
            break
        case requestMaximizeRestore:
            root.requestVisibility(root.visibility === Window.Maximized ? Window.Windowed : Window.Maximized)
            break
        case requestExit:
            root.close()
            break
        case requestContextMenu:
            menuLoader.active = true
            menuLoader.item.logWindows = root.logGeometry
            menuLoader.item.show("context")
            break
        case requestStartMove:
            root.startSystemMove()
            break
        case requestStartResize:
            root.startSystemResize(argument)
            break
        case requestDialog:
            playbackLoader.active = true
            playbackLoader.item.show(player.dialog)
            break
        case requestCopyText:
            Tools.set_clipboard_text(player.dialog)
            break
        case requestReadClipboard:
            player.paste_text(argument, Tools.clipboard_text())
            break
        case requestShowFile:
            Tools.show_file(player.dialog)
            break
        case requestScenes:
            sceneLoader.active = true
            sceneLoader.item.show(player.dialog)
            break
        case requestVideoDialog:
            videoDialogLoader.active = true
            videoDialogLoader.item.show(player.dialog)
            break
        case requestWindow:
            root.windowRequest(JSON.parse(player.dialog))
            break
        case requestSlide:
            if (slideshowLoader.item)
                slideshowLoader.item.move(argument)
            break
        }
    }

    // The free area of the window's screen (without task bars), in the
    // window's coordinates.
    function screenArea() {
        const area = JSON.parse(Tools.screen_json(root))
        if (!(area.available_width > 0))
            return { x: root.x, y: root.y, width: root.width, height: root.height }
        return { x: area.x, y: area.y, width: area.available_width, height: area.available_height }
    }

    // M12's Window Size commands (crates/app/src/window.rs). Sizes keep
    // the video's shape and stay inside the screen's free area, moves stop
    // at its edges, and a maximized window is restored first. Wayland does
    // not let a program place its windows: moves do nothing there.
    function windowRequest(r) {
        if (r.kind === "fullscreen-other-display") {
            root.fullscreenElsewhere()
            return
        }
        if (root.visibility === Window.FullScreen)
            return
        if (root.visibility !== Window.Windowed)
            root.requestVisibility(Window.Windowed)
        const area = root.screenArea()
        switch (r.kind) {
        case "minimum":
            root.resizeWindow(root.minimumWidth, root.minimumHeight, area, "corner")
            break
        case "maximum":
            root.boxWindow(area.width, area.height, area, "screen")
            break
        case "desktop":
            root.boxWindow(area.width * r.percent / 100, area.height * r.percent / 100, area, "screen")
            break
        case "grow":
            root.boxWindow(root.width * r.factor, root.height * r.factor, area, "center")
            break
        case "size":
            // General: "Resize window based on the center" (M14).
            root.resizeWindow(r.width, r.height, area, root.generalLook.resizeCenter === true ? "center" : "corner")
            break
        case "place":
            if (r.x === null || r.y === null || !root.onScreen(r.x, r.y))
                root.resizeWindow(r.width, r.height, area, "corner")
            else
                root.placeInArea(r.x, r.y, Math.round(Math.max(root.minimumWidth, Math.min(r.width, area.width))),
                                 Math.round(Math.max(root.minimumHeight, Math.min(r.height, area.height))), area)
            break
        case "move":
            root.moveWindow(r.placement, r.step, area)
            break
        }
    }

    // The largest window inside the box whose video keeps its shape (idle
    // windows keep the video area's shape).
    function boxWindow(boxWidth, boxHeight, area, anchor) {
        const chromeWidth = root.width - player.video_rect.width
        const chromeHeight = root.height - player.video_rect.height
        const aspect = player.video_aspect > 0 ? player.video_aspect
            : Math.max(1, player.video_rect.width) / Math.max(1, player.video_rect.height)
        const videoWidth = Math.max(1, Math.min(boxWidth - chromeWidth, (boxHeight - chromeHeight) * aspect))
        root.resizeWindow(videoWidth + chromeWidth, videoWidth / aspect + chromeHeight, area, anchor)
    }

    // Sets the window's size within the screen's free area. `anchor`:
    // "corner" keeps the top left corner, "center" the window's centre,
    // "screen" centres it on the screen.
    function resizeWindow(width, height, area, anchor) {
        const w = Math.round(Math.max(root.minimumWidth, Math.min(width, area.width)))
        const h = Math.round(Math.max(root.minimumHeight, Math.min(height, area.height)))
        let x = root.x
        let y = root.y
        if (anchor === "center") {
            x = Math.round(root.x + root.width / 2 - w / 2)
            y = Math.round(root.y + root.height / 2 - h / 2)
        } else if (anchor === "screen") {
            x = area.x + Math.round((area.width - w) / 2)
            y = area.y + Math.round((area.height - h) / 2)
        }
        root.placeInArea(x, y, w, h, area)
    }

    function moveWindow(placement, step, area) {
        if (Qt.platform.pluginName === "wayland")
            return
        let x = root.x
        let y = root.y
        const right = area.x + area.width - root.width
        const bottom = area.y + area.height - root.height
        switch (placement) {
        case "left": x -= step; break
        case "right": x += step; break
        case "up": y -= step; break
        case "down": y += step; break
        case "center":
            x = area.x + Math.round((area.width - root.width) / 2)
            y = area.y + Math.round((area.height - root.height) / 2)
            break
        case "top-left": x = area.x; y = area.y; break
        case "top-right": x = right; y = area.y; break
        case "bottom-left": x = area.x; y = bottom; break
        case "bottom-right": x = right; y = bottom; break
        }
        root.placeInArea(x, y, root.width, root.height, area)
    }

    // The window at x, y (kept inside `area`) with this size.
    function placeInArea(x, y, width, height, area) {
        root.setWindowGeometry(Math.max(area.x, Math.min(x, area.x + area.width - width)),
                               Math.max(area.y, Math.min(y, area.y + area.height - height)), width, height,
                               "command")
    }

    // One geometry change for the window's place and size: Windows reports
    // a change after the call returns, so `x` set after `width` put back
    // the old width. Wayland places windows itself: the size only. A fit
    // to the video's shape still waiting for the previous size would undo
    // this one: the new size starts its own.
    function setWindowGeometry(x, y, width, height, why) {
        if (logGeometry)
            Tools.log("window_set=" + x + "," + y + " " + width + "x" + height + " " + why)
        aspectTimer.stop()
        if (Qt.platform.pluginName === "wayland")
            root.setGeometry(root.x, root.y, width, height)
        else
            root.setGeometry(x, y, width, height)
    }

    // The Window Size Control dialog's numbers.
    function placeWindow(width, height, x, y) {
        if (root.visibility === Window.FullScreen)
            return
        if (root.visibility !== Window.Windowed)
            root.requestVisibility(Window.Windowed)
        const there = root.onScreen(x, y)
        root.setWindowGeometry(there ? x : root.x, there ? y : root.y, Math.max(root.minimumWidth, width),
                               Math.max(root.minimumHeight, height), "dialog")
    }

    // Moves the window (not fullscreen) onto `screen`, centred.
    function placeOnScreen(screen) {
        if (!screen || Qt.platform.pluginName === "wayland" || root.screen.name === screen.name)
            return
        if (root.visibility !== Window.Windowed)
            root.showNormal()
        root.setWindowGeometry(screen.virtualX + Math.max(0, Math.round((screen.width - root.width) / 2)),
                               screen.virtualY + Math.max(0, Math.round((screen.height - root.height) / 2)),
                               root.width, root.height, "screen")
    }

    // "Fullscreen (on Another Display)": fullscreen on the next screen, or
    // back to the window from fullscreen.
    function fullscreenElsewhere() {
        if (root.visibility === Window.FullScreen) {
            player.window_fullscreen(false)
            root.requestVisibility(root.wasMaximized ? Window.Maximized : Window.Windowed)
            return
        }
        const screens = Qt.application.screens
        let index = 0
        for (let i = 0; i < screens.length; ++i) {
            if (screens[i].name === root.screen.name)
                index = i
        }
        root.wasMaximized = root.visibility === Window.Maximized
        root.placeOnScreen(screens[(index + 1) % screens.length])
        player.window_fullscreen(true)
        root.requestVisibility(Window.FullScreen)
    }

    // Window states RVP asks for. On Wayland a state request made before
    // the compositor applied the previous one is lost (seen on GNOME:
    // leaving fullscreen right after entering it, restoring right after
    // maximizing), so for two seconds after RVP's own request the state is
    // checked and asked for again. Later changes come from the window
    // system and are followed (onVisibilityChanged).
    function requestVisibility(state) {
        root.desiredVisibility = state
        root.stateRequested = Date.now()
        root.applyVisibility()
        stateCheck.restart()
    }
    function applyVisibility() {
        if (root.visibility === root.desiredVisibility)
            return
        if (root.desiredVisibility === Window.FullScreen)
            root.showFullScreen()
        else if (root.desiredVisibility === Window.Maximized)
            root.showMaximized()
        else
            root.showNormal()
    }
    Timer {
        id: stateCheck
        interval: 300
        onTriggered: {
            if (Date.now() - root.stateRequested < 2000 && root.visibility !== root.desiredVisibility) {
                root.applyVisibility()
                restart()
            }
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
        if (videoWidth <= 0 || videoHeight <= 0)
            return
        // Never below the window's minimum: the video grows, keeping its
        // shape (with the playlist docked, a small video leaves the window
        // under its minimum height).
        const grow = Math.max(1, (root.minimumWidth - chromeWidth) / videoWidth,
                              (root.minimumHeight - chromeHeight) / videoHeight)
        const scale = Math.min(grow, (maxWidth - chromeWidth) / videoWidth,
                               (maxHeight - chromeHeight) / videoHeight)
        const width = Math.round(videoWidth * scale + chromeWidth)
        const height = Math.round(videoHeight * scale + chromeHeight)
        // General: "Resize window based on the center" (M14).
        let x = root.generalLook.resizeCenter === true ? root.x + Math.round((root.width - width) / 2) : root.x
        let y = root.generalLook.resizeCenter === true ? root.y + Math.round((root.height - height) / 2) : root.y
        if (screen) {
            // "Start playback in centered window" (M11).
            if (player.center_window) {
                x = screen.virtualX + Math.round((maxWidth - width) / 2)
                y = screen.virtualY + Math.round((maxHeight - height) / 2)
            }
            x = Math.max(screen.virtualX, Math.min(x, screen.virtualX + maxWidth - width))
            y = Math.max(screen.virtualY, Math.min(y, screen.virtualY + maxHeight - height))
        }
        root.setWindowGeometry(x, y, width, height, "fit")
    }

    // Over the video only the bar the pointer is on shows (M14).
    readonly property bool oneBar: player.overlay && player.shown_bar.width > 0
    function inShownBar(x, y, width, height) {
        const bar = player.shown_bar
        return !oneBar || (x < bar.x + bar.width && bar.x < x + width
                           && y < bar.y + bar.height && bar.y < y + height)
    }

    // The skin's static layer: under the video in the window, over it in
    // fullscreen and with "Auto hide main skin" (where it only holds the
    // bars, at On Screen Control's opacity, fading when animations are on,
    // cut to the bar the pointer is on).
    Item {
        id: staticClip
        x: root.oneBar ? player.shown_bar.x : 0
        y: root.oneBar ? player.shown_bar.y : 0
        width: root.oneBar ? player.shown_bar.width : root.width
        height: root.oneBar ? player.shown_bar.height : root.height
        clip: root.oneBar
        z: player.overlay ? 2 : 0

        SkinView {
            id: staticSkin
            readonly property bool shown: player.has_skin && (!player.overlay || player.chrome_shown)
            region_index: -1
            x: -staticClip.x
            y: -staticClip.y
            width: root.width
            height: root.height
            opacity: shown ? (player.overlay && player.osc_opacity > 0 ? player.osc_opacity : 1) : 0
            visible: opacity > 0
            Behavior on opacity {
                enabled: player.animations && player.overlay
                NumberAnimation { duration: 150 }
            }
        }
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

    // D4's Direct3D 11 renderer (M12): mpv draws into a native window over
    // the video item (cpp/video_window.cpp). It follows the video's place,
    // hides while no video shows, and leaves holes where RVP's own layers
    // cover the video, since nothing in the scene draws over a native
    // window. Dialogs open as windows of their own then.
    function videoHole(item) {
        if (!item || !item.visible || item.width <= 0 || item.height <= 0)
            return null
        const corner = item.mapToItem(null, 0, 0)
        return [corner.x, corner.y, item.width, item.height]
    }
    readonly property string videoWindowPlace: !player.native_video ? "" : JSON.stringify({
        x: player.video_rect.x,
        y: player.video_rect.y,
        width: player.video_rect.width,
        height: player.video_rect.height,
        visible: player.has_video && root.visibility !== Window.Minimized && root.visibility !== Window.Hidden,
        holes: [videoHole(seekTip), videoHole(powerBox), videoHole(infoBox), videoHole(miscInfoBox),
                videoHole(messageBox), videoHole(sceneLoader.item), videoHole(slideshowLoader.item)]
            .filter(hole => hole !== null)
            .concat(player.overlay && staticSkin.visible
                    ? JSON.parse(player.chrome_bars || "[]").filter(bar => root.inShownBar(bar[0], bar[1], bar[2], bar[3]))
                    : [])
    })
    onVideoWindowPlaceChanged: Tools.place_video_window(videoWindowPlace)

    // The logo over the video area while no video picture shows (D10,
    // M14's Skins > Logo): RVP's, a picture of the logo folders, or the
    // skin's own (SkinView surface 2), after a moment so that it does not
    // flash between files.
    Item {
        id: logoLayer
        readonly property bool idle: !player.has_video
        property bool shown: idle
        x: player.video_rect.x
        y: player.video_rect.y
        width: player.video_rect.width
        height: player.video_rect.height
        z: 1.5
        opacity: shown ? 1 : 0
        visible: opacity > 0
        Behavior on opacity {
            NumberAnimation { duration: 200 }
        }
        onIdleChanged: {
            idleDelay.stop()
            if (idle)
                idleDelay.start()
            else
                shown = false
        }
        Timer {
            id: idleDelay
            interval: 400
            onTriggered: logoLayer.shown = logoLayer.idle
        }
        Image {
            id: idleLogo
            readonly property bool rvp: player.logo_source.startsWith("qrc:")
            readonly property string place: player.logo_position || "center"
            readonly property int margin: 16
            readonly property real room: Math.max(1, Math.min((parent.width - 2 * margin) / Math.max(1, implicitWidth),
                                                               (parent.height - 2 * margin) / Math.max(1, implicitHeight)))
            source: player.skin_logo ? "" : player.logo_source
            fillMode: Image.PreserveAspectFit
            smooth: true
            mipmap: true
            // RVP's logo: at most 200 pixels, a quarter of the height; a
            // picture: its size, smaller where the area is; "fit": as
            // large as the area.
            height: Math.round(rvp && place !== "fit" ? Math.min(200, parent.height * 0.25)
                               : place === "fit" ? parent.height - 2 * margin
                               : Math.min(implicitHeight, implicitHeight * Math.min(1, room)))
            width: Math.round(place === "fit" ? parent.width - 2 * margin
                              : height * implicitWidth / Math.max(1, implicitHeight))
            x: Math.round(place.endsWith("left") ? margin : place.endsWith("right") ? parent.width - width - margin
                          : (parent.width - width) / 2)
            y: Math.round(place.startsWith("top") ? margin : place.startsWith("bottom") ? parent.height - height - margin
                          : (parent.height - height) / 2)
            visible: source !== "" && height >= 16
        }
        SkinView {
            id: skinLogo
            surface: 2
            anchors.fill: parent
            visible: player.skin_logo
        }
        Connections {
            target: player
            function onLogoDamaged() {
                Tools.repaint(skinLogo, false)
            }
        }
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
            opacity: staticSkin.opacity
            visible: staticSkin.visible && root.inShownBar(x, y, width, height)
        }
    }

    // Keyboard: every key goes to the bindings.
    Item {
        id: keys
        focus: true
        Keys.onPressed: (event) => {
            if (root.screensaver) {
                Qt.quit()
                return
            }
            event.accepted = player.key(event.key, event.modifiers)
        }
    }

    MouseArea {
        id: pointer
        anchors.fill: parent
        z: 10
        acceptedButtons: Qt.AllButtons
        hoverEnabled: true
        // Fullscreen hides an idle pointer; in the window it hides over the
        // playing video with "Hide mouse cursor during playback" (M11).
        readonly property bool overVideo: mouseX >= player.video_rect.x
            && mouseX < player.video_rect.x + player.video_rect.width
            && mouseY >= player.video_rect.y
            && mouseY < player.video_rect.y + player.video_rect.height
        cursorShape: root.pointerIdle && (player.fullscreen || (player.hide_pointer && overVideo))
            ? Qt.BlankCursor : player.cursor_shape
        onPressed: (mouse) => {
            keys.forceActiveFocus()
            root.pointerPress(mouse.x, mouse.y, mouse.button, mouse.modifiers)
        }
        onReleased: (mouse) => root.pointerRelease(mouse.x, mouse.y, mouse.button)
        onDoubleClicked: (mouse) => {
            if (!root.swallowing)
                player.mouse_double(mouse.x, mouse.y, mouse.button)
        }
        onPositionChanged: (mouse) => {
            // A self-test moves the pointer through Player's handlers; the
            // window's own hover (Qt repeats it every frame while items
            // change) would undo its moves.
            if (selftest.mode !== "")
                return
            if (root.logPointer)
                Tools.log("pointer_local=" + Math.round(mouse.x) + "," + Math.round(mouse.y)
                          + " size=" + root.width + "x" + root.height)
            root.pointerIdle = false
            idleTimer.restart()
            tipTimer.restart()
            if (tipLoader.item)
                tipLoader.item.tip.close()
            player.mouse_move(mouse.x, mouse.y)
        }
        onWheel: (wheel) => player.mouse_wheel(wheel.x, wheel.y, wheel.angleDelta.y)
        onExited: {
            if (selftest.mode !== "")
                return
            if (tipLoader.item)
                tipLoader.item.tip.close()
            player.mouse_leave()
        }
    }

    // A press that closed a menu stays with the menu, and so does its
    // release (Menus.swallowsPress).
    property bool swallowing: false
    function menuTookPress() {
        return menuLoader.item !== null && menuLoader.item.swallowsPress()
    }
    function pointerPress(x, y, button, modifiers) {
        swallowing = menuTookPress()
        if (!swallowing)
            player.mouse_press(x, y, button, modifiers)
    }
    function pointerRelease(x, y, button) {
        if (swallowing)
            swallowing = false
        else
            player.mouse_release(x, y, button)
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
                tipLoader.active = true
                const tip = tipLoader.item.tip
                tip.text = player.tooltip
                tip.x = Math.min(pointer.mouseX + 12, root.width - tip.implicitWidth)
                tip.y = pointer.mouseY + 20
                tip.open()
            }
        }
    }

    // The tooltip, menu and file dialog load on first use: Qt Quick
    // Controls and Dialogs are not needed to show the window.
    Loader {
        id: tipLoader
        anchors.fill: parent
        active: false
        source: "Tip.qml"
    }
    Loader {
        id: menuLoader
        anchors.fill: parent
        active: false
        Component.onCompleted: setSource("Menus.qml", { player: player })
    }
    Loader {
        id: dialogLoader
        active: false
        Component.onCompleted: setSource("OpenDialog.qml", { player: player })
    }
    Loader {
        id: folderLoader
        active: false
        Component.onCompleted: setSource("FolderDialog.qml", { player: player })
    }
    Loader {
        id: urlLoader
        anchors.fill: parent
        z: 30
        active: false
        Component.onCompleted: setSource("UrlDialog.qml", { player: player })
    }
    Loader {
        id: aboutLoader
        anchors.fill: parent
        z: 30
        active: false
        Component.onCompleted: setSource("About.qml", { player: player })
    }
    Loader {
        id: syncLoader
        anchors.fill: parent
        z: 30
        active: false
        Component.onCompleted: setSource("AudioSync.qml", { player: player })
    }
    Loader {
        id: subtitleSyncLoader
        anchors.fill: parent
        z: 30
        active: false
        Component.onCompleted: setSource("SubtitleSync.qml", { player: player })
    }
    Loader {
        id: subtitleEditorLoader
        active: false
        Component.onCompleted: setSource("SubtitleBrowser.qml", { player: player })
    }
    Loader {
        id: colorThemeLoader
        anchors.fill: parent
        z: 30
        active: false
        Component.onCompleted: setSource("ColorTheme.qml", { player: player })
    }
    // General: "Use advanced drag and drop interaction" (M14): dropped
    // files ask what to do.
    Loader {
        id: dropMenuLoader
        anchors.fill: parent
        z: 30
        active: false
        Component.onCompleted: setSource("DropMenu.qml", { player: player })
    }

    // Playback/System Info (Ctrl+F1, M14): a window of its own.
    Loader {
        id: systemInfoLoader
        active: false
        Component.onCompleted: setSource("SystemInfo.qml", { player: player, owner: root })
    }
    Loader {
        id: albumLoader
        anchors.fill: parent
        z: 30
        active: false
        Component.onCompleted: setSource("AlbumName.qml", { player: player })
    }
    Loader {
        id: settingsLoader
        active: false
        Component.onCompleted: setSource("Settings.qml", { player: player })
    }
    Loader {
        id: playbackLoader
        anchors.fill: parent
        z: 30
        active: false
        Component.onCompleted: setSource("PlaybackDialogs.qml", { player: player })
    }
    Loader {
        id: videoDialogLoader
        anchors.fill: parent
        z: 30
        active: false
        Component.onCompleted: setSource("VideoDialogs.qml", { player: player, window: root })
    }
    // Fullscreen Mode's "Fill other monitors black in fullscreen" (M12):
    // a black window on each other screen while RVP is fullscreen.
    Instantiator {
        model: root.isFullscreen && player.black_other_displays ? Qt.application.screens : []
        delegate: Window {
            required property var modelData
            flags: Qt.FramelessWindowHint | Qt.WindowDoesNotAcceptFocus
            color: "black"
            title: qsTr("RVP")
            x: modelData.virtualX
            y: modelData.virtualY
            width: modelData.width
            height: modelData.height
            visibility: modelData.name !== root.screen.name ? Window.FullScreen : Window.Hidden
        }
    }
    // The slideshow for audio files lies over the video, under the bars
    // of fullscreen (M12), created the first time it shows.
    Loader {
        id: slideshowLoader
        x: player.video_rect.x
        y: player.video_rect.y
        width: player.video_rect.width
        height: player.video_rect.height
        z: 1.5
        active: player.slideshow !== "" || item !== null
        Component.onCompleted: setSource("Slideshow.qml", { player: player })
        onLoaded: item.anchors.fill = slideshowLoader
    }
    // The scene browser lies over the video (M11).
    Loader {
        id: sceneLoader
        x: player.video_rect.x
        y: player.video_rect.y
        width: player.video_rect.width
        height: player.video_rect.height
        z: 26
        active: false
        Component.onCompleted: setSource("SceneBrowser.qml", { player: player })
        onLoaded: {
            item.anchors.fill = sceneLoader
        }
    }
    // The control panel's window (F7), created the first time it shows.
    Loader {
        id: panelLoader
        active: player.panel_shown || item !== null
        Component.onCompleted: setSource("ControlPanel.qml", { player: player, owner: root, menus: menuLoader })
    }

    DropArea {
        anchors.fill: parent
        onDropped: (drop) => {
            if (!drop.hasUrls)
                return
            const paths = []
            for (const url of drop.urls)
                paths.push(Tools.local_file(url))
            player.drop_files(paths.join("\n"))
            drop.accept()
        }
    }

    // The OSD Messages page's look (M14): alignment, margins and fonts.
    readonly property var osdLook: player.osd_look ? JSON.parse(player.osd_look) : ({})
    // A font size of the page, following the video's height when so set.
    function osdSize(size) {
        const base = size > 0 ? size : 20
        return Math.max(8, Math.round(osdLook.auto === true ? base * player.video_rect.height / 720 : base))
    }

    // The OSD: short notices over the video (volume, normalizer), as
    // PotPlayer shows them at the top left (OSD Messages places them).
    Text {
        id: osdText
        readonly property string h: root.osdLook.h || "left"
        readonly property string v: root.osdLook.v || "top"
        readonly property int marginX: root.osdLook.right !== undefined ? root.osdLook.right : 16
        readonly property int marginY: root.osdLook.top !== undefined ? root.osdLook.top : 12
        // mpv shows it over the Direct3D 11 renderer's window, and in the
        // picture when the page says so.
        visible: player.osd !== "" && !player.native_video && root.osdLook.video !== true
        z: 20
        x: player.video_rect.x + (h === "left" ? marginX : h === "right" ? player.video_rect.width - width - marginX
                                                      : (player.video_rect.width - width) / 2)
        y: player.video_rect.y + (v === "top" ? marginY : v === "bottom" ? player.video_rect.height - height - marginY
                                                     : (player.video_rect.height - height) / 2)
        text: player.osd
        color: "#f0f0f0"
        style: Text.Outline
        styleColor: "#000000"
        font.family: root.osdLook.font || Qt.application.font.family
        font.pixelSize: root.osdSize(root.osdLook.size)
        onTextChanged: {
            if (text === "")
                return
            osdTimer.restart()
            if (player.native_video || root.osdLook.video === true)
                player.mpv_osd(text, osdTimer.interval)
        }
        Timer {
            id: osdTimer
            interval: 2000
            onTriggered: player.osd = ""
        }
    }

    // The seek bar's tooltip (M11): the time under the pointer, the
    // chapter or bookmark there, and the thumbnail core's preview.
    Rectangle {
        id: seekTip
        visible: player.seek_tip_shown && (player.seek_tip !== "" || player.preview_width > 0)
        z: 22
        readonly property int pad: 4
        width: Math.max(seekTipText.implicitWidth, player.preview_width) + 2 * pad
        height: (player.preview_width > 0 ? player.preview_height + pad : 0)
                + (player.seek_tip !== "" ? seekTipText.implicitHeight : 0) + 2 * pad
        x: Math.max(0, Math.min(root.width - width, player.seek_tip_x - width / 2))
        y: Math.max(0, player.seek_tip_y - height - 6)
        radius: 3
        color: "#e0181820"
        border.color: "#60ffffff"
        ThumbView {
            id: previewView
            visible: player.preview_width > 0
            x: (seekTip.width - width) / 2
            y: seekTip.pad
            width: player.preview_width
            height: player.preview_height
            key: "thumb/preview"
            image_width: player.preview_width
            image_height: player.preview_height
            Connections {
                target: player
                function onPreview_serialChanged() { previewView.update() }
            }
        }
        Text {
            id: seekTipText
            x: (seekTip.width - width) / 2
            y: seekTip.height - seekTip.pad - implicitHeight
            horizontalAlignment: Text.AlignHCenter
            color: "#f0f0f0"
            font.pixelSize: 13
            text: player.seek_tip
        }
    }

    // The notice before shutting down, sleeping or hibernating (M11):
    // it counts down, and Cancel stops it.
    Rectangle {
        id: powerBox
        visible: player.power_seconds > 0
        z: 28
        anchors.centerIn: parent
        width: Math.max(powerText.implicitWidth, 260) + 40
        height: powerText.implicitHeight + cancelPower.height + 40
        radius: 6
        color: "#f0202028"
        border.color: "#80ffb000"
        Text {
            id: powerText
            anchors.horizontalCenter: parent.horizontalCenter
            y: 14
            horizontalAlignment: Text.AlignHCenter
            color: "#f0f0f0"
            font.pixelSize: 16
            text: qsTr("%1\nin %2 seconds").arg(player.power_notice).arg(player.power_seconds)
        }
        Rectangle {
            id: cancelPower
            objectName: "cancelPower"
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.bottom: parent.bottom
            anchors.bottomMargin: 12
            width: 120
            height: 30
            radius: 4
            color: cancelArea.containsMouse ? "#505868" : "#383e4a"
            Text {
                anchors.centerIn: parent
                color: "#ffffff"
                text: qsTr("Cancel")
            }
            MouseArea {
                id: cancelArea
                anchors.fill: parent
                hoverEnabled: true
                onClicked: player.cancel_countdown()
            }
        }
    }

    // Misc info (Scroll Lock, M14): a line in the video's top right corner.
    Rectangle {
        id: miscInfoBox
        visible: player.misc_info !== ""
        z: 20
        x: player.video_rect.x + player.video_rect.width - width - 12
        y: player.video_rect.y + 12
        width: miscInfoText.implicitWidth + 16
        height: miscInfoText.implicitHeight + 8
        radius: 4
        color: "#99000000"
        Text {
            id: miscInfoText
            anchors.centerIn: parent
            color: "#f0f0f0"
            font.family: root.osdLook.infoFont || Qt.application.font.family
            font.pixelSize: root.osdSize(root.osdLook.infoSize > 0 ? root.osdLook.infoSize : 14)
            text: player.misc_info
        }
    }

    // Playback information (Tab), refreshed every second while shown.
    Rectangle {
        id: infoBox
        visible: player.info_shown
        z: 20
        x: player.video_rect.x + 12
        y: player.video_rect.y + 44
        width: infoText.implicitWidth + 16
        height: infoText.implicitHeight + 12
        radius: 4
        color: "#99000000"
        Text {
            id: infoText
            x: 8
            y: 6
            color: "#f0f0f0"
            font.family: root.osdLook.infoFont || Qt.application.font.family
            font.pixelSize: root.osdSize(root.osdLook.infoSize > 0 ? root.osdLook.infoSize : 14)
            text: ""
        }
        Timer {
            running: parent.visible
            repeat: true
            interval: 1000
            triggeredOnStart: true
            onTriggered: infoText.text = player.info_text()
        }
    }

    // Non-blocking messages (section 4: no modal interruptions).
    Rectangle {
        id: messageBox
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

    // --selftest: a scripted check that prints check=PASS/FAIL lines and the
    // usability measurements, then exits with 0 when everything passed.
    // Input goes through the same handlers as real input (Player.key,
    // Player.mouse_*), never through the window system.
    //   --selftest            functional check (default)
    //   --selftest=playback   plays --play-seconds=N (default 20), with
    //                         --fullscreen in fullscreen; --gpu-checks
    //                         fails on dropped frames; --svp waits for
    //                         SVP's @svp filter; --next plays the album's
    //                         second entry; logs time= every second (the
    //                         kit's kill test) and at the end mpv's
    //                         diagnostics and the smoothing plan
    //   --selftest=upright    plays a clip (testdata/media/fixture-marker.mkv)
    //                         and compares the picture with --reference=PNG
    //                         (default: the clip's name with .png): upright,
    //                         not flipped or mirrored; SKIPPED when the grab
    //                         has no video (Windows session 0)
    //   --selftest=svp        SVP's acceptance (section 7): SVP attaches
    //                         (@svp) within --svp-wait seconds (default
    //                         90), the frame rate reaches at least
    //                         --svp-factor (default 1.9) times the file's,
    //                         and after seeks audio and video stay
    //                         together (|avsync| under 0.1 s); then RVP's
    //                         sharpen filter and deinterlacing go on and
    //                         off while SVP works: RVP's filter runs before
    //                         @svp, SVP keeps interpolating and its entry
    //                         is unchanged; the same for M12's crop and
    //                         two of its effects
    //   --selftest=vpy        a VapourSynth script slot (M6): RVP writes a
    //                         PotPlayer-style script (vs.get_core()) that
    //                         doubles the frame rate, or runs --vpy=SCRIPT
    //                         with --vpy-plugins=DLL;DLL (PotPlayer's
    //                         motioninterpolation.vpy with mvtools), in its
    //                         slot and checks that the rate rose; SKIPPED
    //                         without a VapourSynth runtime
    //   --selftest=speed      goal H's carry-over: 1.5 s of steady playback
    //                         from --speed-at seconds (default 1) count the
    //                         frames the machine drops anyway; then the
    //                         speed goes to 1.1x and 1.5 s later back to 1x.
    //                         The frames dropped in the 1.5 s after that and
    //                         mpv's A/V offset then are printed (baseline_
    //                         drops=, speed_back_drops=, speed_back_avsync=)
    //                         and checked: at most 2 frames more than the
    //                         baseline, offset under 0.1 s (SKIPPED when
    //                         the baseline is above 2: software rendering)
    //   --selftest=m11        M11's checks (SelfTestM11.qml): the thumbnail
    //                         core, jumps, A-B, skips, bookmarks, the
    //                         playlist's commands, the power actions
    //                         through the dry-run hook, discs (--dvd-folder=DIR),
    //                         external audio (--audio=FILE), the clipboard
    //                         (--clip=FILE), yt-dlp's fallback; the files:
    //                         fixture-chapters.mkv, fixture-60s.mkv,
    //                         fixture-3s.mkv, with /seek=0:00:03 and
    //                         /title="M11 title"
    //   --selftest=m12        M12's checks (SelfTestM12.qml): the window's
    //                         sizes, dialogs, moves and fullscreen on another
    //                         display, capture (BMP, the screen frame, the
    //                         clipboard, consecutive images, the thumbnail
    //                         image) and the settings window's tree; the
    //                         file: fixture-60s.mkv
    //   --selftest=shaders    the HLSL pixel shaders of the catalogue, one
    //                         at a time in the post-resize list; needs
    //                         --mpv-log=FILE and --mpv-options=log-file=FILE
    //                         (mpv's errors say whether each compiled)
    //   --selftest=direct3d11 D4's renderer (render.renderer "direct3d11"
    //                         in the settings, Windows): mpv's native video
    //                         window, its place, holes and fullscreen
    //   --selftest=gpu        the GPU power budget (SelfTestGpu.qml, D17):
    //                         readings it gives make RVP take the lighter
    //                         scalers and the shaders off, give them back
    //                         and give everything back when the budget is
    //                         turned off, then lighten M18's smoothing,
    //                         blend and stop it; the file: fixture-60s.mkv
    //   --selftest=smoothing  M18's motion smoothing (SelfTestSmoothing.qml):
    //                         Ctrl+Shift+I on and off, MVTools raising the
    //                         rate (Blend without a runtime that has it),
    //                         the engines and presets, its settings page;
    //                         the file: fixture-60s.mkv
    //   --selftest=m13        M13's subtitles (SelfTestM13.qml): bold, the
    //                         browser's edits shown at once, Input
    //                         Subtitles, saving elsewhere (--out), the sync
    //                         dialog, syncing to the next subtitle, Combine
    //                         Subtitle and the Subtitles settings pages;
    //                         --speech-check also makes subtitles from the
    //                         audio (whisper.cpp's CPU engine, the tiny
    //                         model); the file: fixture-chapters.mkv with
    //                         its .srt
    //   --selftest=m14        M14's checks (SelfTestM14.qml): a colour theme
    //                         and none, the window's opacity, the audio
    //                         skin for a file without video, another skin
    //                         from the Skins menu, the copied time; the
    //                         files: fixture-60s.mkv, fixture-tone.ogg
    //   --selftest=speech     subtitles from a file's audio (SelfTestM13.qml,
    //                         the host session): the whole file with
    //                         --speech-engine and --speech-model, timed,
    //                         then real time while it plays
    //   --selftest=resume     expects the last file reopened paused within
    //                         --expect-tolerance (default 2.5) seconds of
    //                         --expect-time=SECONDS; with --expect-entry=I
    //                         and --expect-entries=N also the album entry
    //   --instance=NAME       the functional check also starts a second
    //                         RVP with /add on this channel
    QtObject {
        id: selftest
        // Read in begin(): the command line is parsed by Player.start().
        property string mode: ""
        property int step: 0
        property var drops: []
        property int dropTotal: 0
        property int dropLast: 0
        property int decoderTotal: 0
        property int decoderLast: 0
        property real svpSeen: -1
        property real stepStarted: 0
        // When a step last asked for room before a file's end.
        property real roomAsked: 0
        property int failures: 0
        property var before: ({})
        property var dragTo: ({})
        property bool nextSent: false
        // The orientation check saw video in the window's grab.
        property bool seesVideo: false
        property string outDir: "rvp-selftest"

        function begin() {
            mode = player.option("selftest")
            if (player.option("out") !== "")
                outDir = player.option("out")
            Tools.log("environment=" + Tools.environment_json())
            testTimer.start()
        }
        function status() { return JSON.parse(player.status()) }
        function check(name, ok, detail) {
            // The dropped frames so far: which step costs frames.
            const drops = status().frame_drop_count
            Tools.log("check " + name + "=" + (ok ? "PASS" : "FAIL") + (detail ? " " + detail : "")
                      + " [drops " + (drops === null ? 0 : drops) + "]")
            if (!ok)
                failures += 1
        }
        function next() {
            step += 1
            stepStarted = Date.now()
        }
        // Seeks back near the start once the file is past 60 % of its
        // length: filter and SVP steps that met the file's end on a loaded
        // machine waited for the next file and timed out (goal P).
        function room() {
            const s = status()
            if (s.duration > 0 && s.time > 0.6 * s.duration)
                player.seek_fraction(0.05)
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
        if (selftest.mode === "playback")
            return playbackTick()
        if (selftest.mode === "resume")
            return resumeTick()
        if (selftest.mode === "upright")
            return uprightTick()
        if (selftest.mode === "svp")
            return svpTick()
        if (selftest.mode === "vpy")
            return vpyTick()
        if (selftest.mode === "speed")
            return speedTick()
        if (selftest.mode === "m11")
            return m11Tick()
        if (selftest.mode === "m12")
            return m12Tick()
        if (selftest.mode === "shaders" || selftest.mode === "direct3d11")
            return m12Tick(selftest.mode)
        if (selftest.mode === "gpu")
            return gpuTick()
        if (selftest.mode === "smoothing")
            return smoothingTick()
        if (selftest.mode === "m13")
            return m13Tick()
        if (selftest.mode === "speech")
            return m13Tick(true)
        if (selftest.mode === "m14")
            return m14Tick()
        functionalTick()
    }

    // --selftest=m14: M14's skins, themes and the rest (SelfTestM14.qml).
    Loader {
        id: m14Loader
        active: false
        Component.onCompleted: setSource("SelfTestM14.qml", { player: player, t: selftest, host: root })
    }
    function m14Tick() {
        if (!m14Loader.item) {
            m14Loader.active = true
            return
        }
        m14Loader.item.tick()
    }

    // --selftest=m13: M13's subtitles (SelfTestM13.qml).
    Loader {
        id: m13Loader
        active: false
        Component.onCompleted: setSource("SelfTestM13.qml", { player: player, t: selftest, host: root })
    }
    function m13Tick(speech) {
        if (!m13Loader.item) {
            m13Loader.active = true
            return
        }
        if (speech)
            m13Loader.item.tickSpeech()
        else
            m13Loader.item.tick()
    }

    // --selftest=smoothing: M18's motion smoothing (SelfTestSmoothing.qml).
    Loader {
        id: smoothingLoader
        active: false
        Component.onCompleted: setSource("SelfTestSmoothing.qml", { player: player, t: selftest, host: root })
    }
    function smoothingTick() {
        if (!smoothingLoader.item) {
            smoothingLoader.active = true
            return
        }
        smoothingLoader.item.tick()
    }

    // --selftest=gpu: the GPU power budget (SelfTestGpu.qml).
    Loader {
        id: gpuLoader
        active: false
        Component.onCompleted: setSource("SelfTestGpu.qml", { player: player, t: selftest, host: root })
    }
    function gpuTick() {
        if (!gpuLoader.item) {
            gpuLoader.active = true
            return
        }
        gpuLoader.item.tick()
    }

    // --selftest=m11: M11's checks (SelfTestM11.qml).
    Loader {
        id: m11Loader
        active: false
        Component.onCompleted: setSource("SelfTestM11.qml", { player: player, t: selftest, host: root })
    }
    function m11Tick() {
        if (!m11Loader.item) {
            m11Loader.active = true
            return
        }
        m11Loader.item.tick()
    }
    // --selftest=m12: M12's checks (SelfTestM12.qml).
    Loader {
        id: m12Loader
        active: false
        Component.onCompleted: setSource("SelfTestM12.qml", { player: player, t: selftest, host: root })
    }
    function m12Tick(kind) {
        if (!m12Loader.item) {
            m12Loader.active = true
            return
        }
        if (kind === "shaders")
            m12Loader.item.shaderTick()
        else if (kind === "direct3d11")
            m12Loader.item.rendererTick()
        else
            m12Loader.item.tick()
    }
    function settingsWindow() { return settingsLoader.item }
    // M11's dialogs and scene browser, for the self-test.
    function playbackDialogs() { return playbackLoader.item }
    function videoDialogs() { return videoDialogLoader.item }
    // M13's subtitle browser and Input Subtitles, and the sync dialog.
    function subtitleEditor() { return subtitleEditorLoader.item }
    function subtitleSync() { return subtitleSyncLoader.item }
    // The output device's aspect and the rotation by aspect ratio follow
    // the screen the window is on.
    onScreenChanged: player.set_screen(Tools.screen_json(root))
    function sceneBrowser() { return sceneLoader.item }

    function resumeTick() {
        const t = selftest
        const s = t.status()
        const expected = Number(player.option("expect-time"))
        switch (t.step) {
        case 0:
            t.next()
            break
        case 1:
            t.waitFor("last_file_reopened", () => s.loaded && s.time !== null, 15000,
                      () => "path " + s.path)
            break
        case 2: {
            // The resume seek follows the load.
            const tolerance = Number(player.option("expect-tolerance") || "2.5")
            t.waitFor("resumed_at_position", () => Math.abs(s.time - expected) <= tolerance, 5000,
                      () => "time " + s.time + " expected " + expected + " within " + tolerance)
            break
        }
        case 3:
            t.check("reopened_paused", s.pause === true, "pause " + s.pause)
            if (player.option("expect-entry") !== "")
                t.check("reopened_album_entry",
                        s.playlist_pos === Number(player.option("expect-entry"))
                            && s.playlist_count === Number(player.option("expect-entries")),
                        "entry " + s.playlist_pos + " of " + s.playlist_count)
            t.finish()
            break
        }
    }

    function uprightTick() {
        const t = selftest
        const s = t.status()
        switch (t.step) {
        case 0:
            t.next()
            break
        case 1:
            t.waitFor("file_playing", () => s.loaded && s.video_width > 0 && s.time > 0.5
                          && s.video_aspect > 0, 20000, () => "time " + s.time)
            break
        case 2:
            // A second for the picture to settle in the window.
            if (Date.now() - t.stepStarted < 1000)
                break
            t.seesVideo = root.checkPicture("picture_upright", "upright", s, false)
            // The pointer over the picture changes nothing.
            player.mouse_move(player.video_rect.x + player.video_rect.width / 2,
                              player.video_rect.y + player.video_rect.height / 2)
            t.next()
            break
        case 3:
            if (Date.now() - t.stepStarted < 700)
                break
            root.checkPicture("picture_with_pointer", "pointer", s, !t.seesVideo)
            player.key(Qt.Key_Return, 0)
            t.next()
            break
        case 4:
            if (t.waitFor("upright_fullscreen", () => root.isFullscreen && s.fullscreen, 3000,
                          () => "visibility " + root.visibility))
                // Over the picture the bars stay hidden.
                player.mouse_move(root.width / 2, root.height / 2)
            break
        case 5:
            if (Date.now() - t.stepStarted < 1000)
                break
            root.checkPicture("picture_fullscreen", "fullscreen", s, !t.seesVideo)
            // Over the bottom edge the bars show, over the picture.
            player.mouse_move(root.width / 2, root.height - 4)
            t.next()
            break
        case 6:
            if (Date.now() - t.stepStarted < 700)
                break
            root.checkPicture("picture_fullscreen_bars", "fullscreen-bars", s, !t.seesVideo)
            player.key(Qt.Key_Return, 0)
            t.finish()
            break
        }
    }

    // Grabs the window and compares the picture in the video item with the
    // reference image (the clip's `.png`); records `name`. Returns false
    // when the grab shows no video at all. `skip` only notes the result
    // (session-0 grabs show no video).
    function checkPicture(name, grab, s, skip) {
        const t = selftest
        const reference = player.option("reference") !== "" ? player.option("reference")
            : String(s.path).replace(/\.[^.\/\\]+$/, ".png")
        const r = JSON.parse(Tools.compare_frame(video, s.video_aspect, reference))
        Tools.grab(t.outDir, grab)
        const detail = "reference " + reference + " distances upright " + r.upright
            + " flipped " + r.flipped + " mirrored " + r.mirrored + " rotated " + r.rotated
            + " picture " + JSON.stringify(r.picture)
        if (r.error) {
            t.check(name, false, r.error + " " + reference)
            return false
        }
        if (skip || r.dark) {
            if (skip)
                Tools.log("check " + name + "=SKIPPED the grabs show no video (" + detail + ")")
            else
                t.check(name, false, "the picture is black (" + detail + ")")
            return false
        }
        t.check(name, r.upright < 30 && r.upright < r.flipped && r.upright < r.mirrored
                    && r.upright < r.rotated, detail)
        return true
    }

    function speedTick() {
        const t = selftest
        const s = t.status()
        const at = Number(player.option("speed-at") || "1")
        // Counts increments only: a seek restarts mpv's counter.
        const countDrops = () => {
            const count = s.frame_drop_count === null ? 0 : s.frame_drop_count
            t.dropTotal += count >= t.dropLast ? count - t.dropLast : count
            t.dropLast = count
        }
        switch (t.step) {
        case 0:
            t.next()
            break
        case 1:
            if (t.waitFor("file_playing", () => s.loaded && s.video_width > 0 && s.time >= at, 30000,
                          () => "time " + s.time)) {
                t.dropTotal = 0
                t.dropLast = s.frame_drop_count === null ? 0 : s.frame_drop_count
            }
            break
        case 2:
            // The baseline: 1.5 s of steady playback.
            countDrops()
            if (Date.now() - t.stepStarted >= 1500) {
                t.before = { baseline: t.dropTotal }
                Tools.log("speed_up_at=" + s.time)
                player.run_action("speed-up")
                t.next()
            }
            break
        case 3:
            if (Date.now() - t.stepStarted >= 1500) {
                player.run_action("speed-default")
                t.dropTotal = 0
                t.dropLast = s.frame_drop_count === null ? 0 : s.frame_drop_count
                t.next()
            }
            break
        case 4: {
            countDrops()
            if (Date.now() - t.stepStarted < 1500)
                break
            const d = JSON.parse(player.diagnostics())
            const baseline = t.before.baseline
            Tools.log("baseline_drops=" + baseline + " speed_back_drops=" + t.dropTotal
                      + " speed_back_avsync=" + d.avsync + " time=" + s.time + " normalizer=" + s.normalizer)
            if (baseline > 2)
                Tools.log("check speed_back_steady=SKIPPED this machine drops " + baseline
                          + " frames in 1.5 s of steady playback (after the change " + t.dropTotal + ")")
            else
                t.check("speed_back_steady", t.dropTotal <= baseline + 2 && d.avsync !== null
                            && Math.abs(d.avsync) < 0.1,
                        "dropped " + t.dropTotal + " (baseline " + baseline + "), avsync " + d.avsync)
            t.finish()
            break
        }
        }
    }

    function vpyTick() {
        const t = selftest
        const s = t.status()
        switch (t.step) {
        case 0:
            t.next()
            break
        case 1:
            t.waitFor("file_playing", () => s.loaded && s.video_width > 0 && s.time > 0.5, 20000,
                      () => "time " + s.time)
            break
        case 2: {
            const runtime = JSON.parse(player.settings_view(false)).vapoursynth_runtime
            if (!runtime) {
                Tools.log("check vpy_slot_runs=SKIPPED no VapourSynth runtime")
                t.finish()
                break
            }
            const given = player.option("vpy")
            const plugins = player.option("vpy-plugins").split(";").filter(p => p !== "")
            const path = given !== "" ? given : Tools.write_file(t.outDir + "/selftest-double.vpy",
                "import vapoursynth as vs\n"
                + "core = vs.get_core()\n"
                + "clip = video_in\n"
                + "clip = core.std.Interleave([clip, clip])\n"
                + "rate = int(round(container_fps * 1000))\n"
                + "clip = core.std.AssumeFPS(clip, fpsnum=rate * 2, fpsden=1000)\n"
                + "clip.set_output()\n")
            const result = JSON.parse(player.set_setting("vapoursynth", JSON.stringify({
                "enabled": true, "slots": [path], "script": path, "plugins": plugins,
                "library": null, "buffered-frames": 4 })))
            Tools.log("vpy_runtime=" + runtime + " script=" + path)
            t.check("vpy_slot_set", !result.error, result.error || "")
            t.next()
            break
        }
        case 3:
            t.waitFor("vpy_slot_runs", () => s.vf !== null && s.vf.indexOf("@rvpvs") >= 0
                          && s.container_fps > 0 && s.estimated_vf_fps >= s.container_fps * 1.8, 20000,
                      () => "vf " + s.vf + ", " + s.container_fps + " fps in, " + s.estimated_vf_fps + " fps out")
            break
        case 4: {
            // Off again: the slot leaves the chain.
            const result = JSON.parse(player.set_setting("vapoursynth.enabled", "false"))
            t.check("vpy_slot_set_off", !result.error, result.error || "")
            t.next()
            break
        }
        case 5:
            t.waitFor("vpy_slot_off", () => (s.vf || "").indexOf("@rvpvs") < 0, 3000, () => "vf " + s.vf)
            break
        case 6:
            t.finish()
            break
        }
    }

    function svpTick() {
        const t = selftest
        const s = t.status()
        const factor = Number(player.option("svp-factor") || "1.9")
        switch (t.step) {
        case 0:
            Tools.log("svp_endpoint=" + JSON.parse(player.settings_view(false)).svp_endpoint)
            t.next()
            break
        case 1:
            t.waitFor("file_playing", () => s.loaded && s.video_width > 0 && s.time > 0.5, 20000,
                      () => "time " + s.time)
            break
        case 2:
            // SVP's first run of a profile may build its engine for a while.
            t.waitFor("svp_attached", () => s.vf !== null && s.vf.indexOf("@svp") >= 0,
                      Number(player.option("svp-wait") || "90") * 1000,
                      () => "after " + Math.round((Date.now() - t.stepStarted) / 100) / 10 + " s, vf "
                          + s.vf + ", status " + s.svp_status)
            break
        case 3:
            // From the observed properties: reading mpv's every tick would
            // cost frames.
            t.waitFor("svp_target_fps", () => s.container_fps > 0
                          && s.estimated_vf_fps >= s.container_fps * factor, 30000,
                      () => s.container_fps + " fps in, " + s.estimated_vf_fps + " fps out")
            break
        case 4:
        case 6:
        case 8: {
            // Seeks: forward, back, to the middle; then a few seconds of play.
            const target = t.step === 4 ? s.time + 20 : t.step === 6 ? Math.max(0, s.time - 15) : s.duration / 2
            t.before = { time: s.time, target: target }
            player.seek_fraction(target / s.duration)
            t.next()
            break
        }
        case 5:
        case 7:
        case 9: {
            if (Date.now() - t.stepStarted < 4000)
                break
            const d = JSON.parse(player.diagnostics())
            const name = "svp_seek_" + ((t.step - 3) / 2) + "_in_sync"
            t.check(name, d.avsync !== null && Math.abs(d.avsync) < 0.1 && s.vf.indexOf("@svp") >= 0
                        && Math.abs(s.time - t.before.target) < 10,
                    "to " + Math.round(t.before.target) + " s, now " + s.time + ", avsync " + d.avsync
                        + ", " + d.estimated_vf_fps + " fps")
            t.next()
            break
        }
        case 10:
            // mpv's estimate needs a few seconds of frames after a seek.
            if (t.waitFor("svp_still_interpolating", () => s.estimated_vf_fps >= s.container_fps * factor
                              && s.playing, 10000, () => s.estimated_vf_fps + " fps, playing " + s.playing)) {
                // Filter order with SVP attached (M6 d): RVP's own filter
                // and deinterlacing on.
                t.before = { svp: s.vf.substring(s.vf.indexOf("@svp")) }
                player.run_action("sharpen")
                player.run_action("deinterlace-on")
            }
            break
        case 11: {
            const sharpen = s.vf.indexOf("@rvpsharpen")
            if (Date.now() - t.stepStarted < 5000)
                break
            const d = JSON.parse(player.diagnostics())
            t.check("svp_after_rvp_filter", sharpen >= 0 && sharpen < s.vf.indexOf("@svp")
                        && d.deinterlace === "yes" && s.estimated_vf_fps >= s.container_fps * factor,
                    "vf " + s.vf + ", deinterlace " + d.deinterlace + ", " + s.estimated_vf_fps + " fps")
            player.run_action("sharpen")
            player.run_action("deinterlace-auto")
            t.next()
            break
        }
        case 12:
            if (t.waitFor("svp_entry_untouched", () => s.vf.indexOf("@rvpsharpen") < 0
                              && s.vf.substring(s.vf.indexOf("@svp")) === t.before.svp
                              && s.estimated_vf_fps >= s.container_fps * factor, 10000,
                          () => "vf " + s.vf + ", " + s.estimated_vf_fps + " fps")) {
                // M12's filters with SVP: a crop to 16:9 and two effects.
                player.run_action("crop-16-9")
                player.run_action("auto-level")
                player.run_action("warpsharp")
            }
            break
        case 13: {
            // The chain starts again with SVP's filter in it: mpv's estimate
            // needs its frames first.
            const svp = s.vf.indexOf("@svp")
            const before = ["@rvpcrop:", "@rvpautolevel:", "@rvpwarpsharp:"].every(label => {
                const at = s.vf.indexOf(label)
                return at >= 0 && at < svp
            })
            if (t.waitFor("svp_after_m12_filters", () => before && s.vf.substring(svp) === t.before.svp
                              && s.estimated_vf_fps >= s.container_fps * factor, 15000,
                          () => "vf " + s.vf + ", " + s.estimated_vf_fps + " fps")) {
                player.run_action("extend-off")
                player.run_action("auto-level")
                player.run_action("warpsharp")
            }
            break
        }
        case 14:
            t.waitFor("svp_m12_filters_off", () => s.vf.indexOf("@rvpcrop:") < 0
                          && s.vf.indexOf("@rvpautolevel:") < 0 && s.vf.indexOf("@rvpwarpsharp:") < 0
                          && s.vf.substring(s.vf.indexOf("@svp")) === t.before.svp
                          && s.estimated_vf_fps >= s.container_fps * factor, 10000,
                      () => "vf " + s.vf + ", " + s.estimated_vf_fps + " fps")
            break
        case 15:
            Tools.log("diagnostics_end=" + player.diagnostics())
            t.finish()
            break
        }
    }

    function playbackTick() {
        const t = selftest
        const s = t.status()
        const seconds = Number(player.option("play-seconds") || "20")
        switch (t.step) {
        case 0:
            t.next()
            break
        case 1:
            // --next: once the first entry plays, the second one.
            if (player.has_option("next") && !t.nextSent && s.loaded && s.playlist_pos === 0) {
                player.run_action("next")
                t.nextSent = true
            }
            t.waitFor("file_playing", () => s.loaded && s.video_width > 0 && s.time > 0.5
                          && (!player.has_option("next") || s.playlist_pos === 1), 20000,
                      () => "time " + s.time + " entry " + s.playlist_pos)
            break
        case 2:
            if (player.has_option("fullscreen"))
                player.run_action("toggle-fullscreen")
            t.next()
            break
        case 3:
            if (!player.has_option("fullscreen") || (root.isFullscreen && Date.now() - t.stepStarted > 1500)) {
                // Count from here: the start-up and the mode switch are not
                // steady playback.
                t.dropLast = s.frame_drop_count === null ? 0 : s.frame_drop_count
                t.decoderLast = s.decoder_frame_drop_count === null ? 0 : s.decoder_frame_drop_count
                Tools.log("diagnostics_start=" + player.diagnostics())
                t.next()
            } else if (Date.now() - t.stepStarted > 5000) {
                t.check("enter_fullscreen", false, "visibility " + root.visibility)
                t.next()
            }
            break
        case 4: {
            // Counts come from the status (observed properties); reading
            // mpv's properties directly every tick would itself drop frames.
            // A looping file restarts the counter: count increments only.
            const count = s.frame_drop_count === null ? 0 : s.frame_drop_count
            t.dropTotal += count >= t.dropLast ? count - t.dropLast : count
            t.dropLast = count
            const decoder = s.decoder_frame_drop_count === null ? 0 : s.decoder_frame_drop_count
            t.decoderTotal += decoder >= t.decoderLast ? decoder - t.decoderLast : decoder
            t.decoderLast = decoder
            const elapsed = (Date.now() - t.stepStarted) / 1000
            if (Math.floor(elapsed) > t.drops.length) {
                t.drops.push(t.dropTotal)
                Tools.log("time=" + s.time + " entry=" + s.playlist_pos)
            }
            if (t.svpSeen < 0 && s.vf !== null && s.vf.indexOf("@svp") >= 0)
                t.svpSeen = elapsed
            if (elapsed >= seconds)
                t.next()
            break
        }
        case 5: {
            const d = JSON.parse(player.diagnostics())
            const dropped = t.dropTotal
            const decoderDropped = t.decoderTotal
            Tools.log("diagnostics_end=" + player.diagnostics())
            Tools.log("smoothing=" + s.smoothing)
            Tools.log("frame_drop_timeline=" + t.drops.join(","))
            Tools.log("frames_dropped=" + dropped + " decoder_dropped=" + decoderDropped
                      + " seconds=" + seconds + " fullscreen=" + root.isFullscreen)
            if (player.has_option("gpu-checks")) {
                t.check("gpu_no_dropped_frames", dropped === 0, "dropped " + dropped)
                t.check("gpu_no_decoder_drops", decoderDropped === 0, "decoder dropped " + decoderDropped)
            }
            if (player.has_option("svp"))
                t.check("svp_filter_attached", t.svpSeen >= 0, "after " + t.svpSeen + " s, vf " + d.vf)
            t.check("still_playing", s.playing, "time " + s.time)
            t.finish()
            break
        }
        }
    }

    function functionalTick() {
        const t = selftest
        const s = t.status()
        switch (t.step) {
        case 0:
            t.stepStarted = Date.now()
            // RVP's own art comes from the build's Qt resources.
            t.check("logo_resources", player.skin_logo || idleLogo.status === Image.Ready,
                    "idle logo status " + idleLogo.status)
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
            // Over the video: over the playlist the wheel scrolls the list.
            player.mouse_wheel(player.video_rect.x + player.video_rect.width / 2,
                               player.video_rect.y + player.video_rect.height / 2, -120)
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
        case 20: {
            // The bars over the video once the window has the screen (the
            // window system reports the size after the state).
            const full = player.overlay && player.video_rect.height >= root.height - 1
                && root.height > 400
            if (!full && Date.now() - t.stepStarted < 3000)
                break
            Tools.grab(t.outDir, "fullscreen")
            if (full && player.has_skin) {
                // The bar under the pointer shows, and the video leaves it
                // room as PotPlayer's does (M14, "Auto resize video
                // depending on fullscreen pop-ups"); in the middle the video
                // takes the screen again.
                player.mouse_move(root.width / 2, root.height - 3)
                const bar = player.shown_bar
                const video = player.video_rect
                t.check("fullscreen_video_leaves_the_bar_room", player.chrome_shown && bar.height > 0
                            && video.y + video.height <= bar.y + 1 && video.height > root.height / 2,
                        "bar " + bar + " video " + video + " shown " + player.chrome_shown)
                player.mouse_move(root.width / 2, root.height / 2)
                t.check("fullscreen_video_takes_the_screen_again", !player.chrome_shown
                            && player.video_rect.height >= root.height - 1, "video " + player.video_rect)
                player.mouse_leave()
            } else {
                t.check("fullscreen_video_leaves_the_bar_room", true,
                        "SKIPPED: the window did not take the screen (" + root.width + "x" + root.height + ")")
            }
            player.mouse_double(root.width / 2, root.height / 2, Qt.LeftButton)
            t.next()
            break
        }
        case 21:
            t.waitFor("double_click_leaves_fullscreen", () => !root.isFullscreen && !s.fullscreen, 3000,
                      () => "visibility " + root.visibility)
            break
        case 22:
            root.requestVisibility(Window.Windowed)
            t.next()
            break
        case 23:
            // The compositor restores the old size first; resize after it.
            if (root.visibility === Window.Windowed && Date.now() - t.stepStarted > 500) {
                root.setGeometry(root.x, root.y, 900, 500)
                t.next()
            } else if (Date.now() - t.stepStarted > 3000) {
                t.check("restored_window", false, "visibility " + root.visibility)
                t.next()
            }
            break
        case 24:
            // 300 ms later the window may take the video's shape (M3), which
            // moves the size by a few pixels.
            t.waitFor("relayout_on_resize", () => player.video_rect.width > 0
                          && player.video_rect.x + player.video_rect.width <= root.width
                          && player.video_rect.y + player.video_rect.height <= root.height
                          && Math.abs(root.width - 900) <= 40 && root.height <= 500, 3000,
                      () => "video " + player.video_rect.width + "x" + player.video_rect.height
                          + " window " + root.width + "x" + root.height)
            break
        case 25:
            player.seek_fraction(0.5)
            t.next()
            break
        case 26:
            t.waitFor("seek_bar_to_middle", () => s.duration > 0 && Math.abs(s.time - s.duration / 2) < 2,
                      3000, () => "time " + s.time + " of " + s.duration)
            break
        case 27:
            player.run_action("toggle-playlist")
            t.next()
            break
        case 28:
            if (t.waitFor("playlist_toggles", () => s.playlist_attached === false || !player.has_skin, 2000))
                player.run_action("toggle-playlist")
            break
        case 29:
            // The menus, dialogs and tooltip load on first use.
            menuLoader.active = true
            dialogLoader.active = true
            tipLoader.active = true
            folderLoader.active = true
            urlLoader.active = true
            aboutLoader.active = true
            t.before = s
            t.next()
            break
        case 30: {
            t.check("menu_dialog_tooltip_load", menuLoader.status === Loader.Ready
                    && dialogLoader.status === Loader.Ready && tipLoader.status === Loader.Ready
                    && folderLoader.status === Loader.Ready && urlLoader.status === Loader.Ready
                    && aboutLoader.status === Loader.Ready,
                    "status " + menuLoader.status + "," + dialogLoader.status + "," + tipLoader.status
                    + "," + folderLoader.status + "," + urlLoader.status + "," + aboutLoader.status)
            // What a drop or the file dialog does with a file URL: append
            // the playing file once more.
            const path = String(s.path).replace(/\\/g, "/")
            const url = path.startsWith("/") ? "file://" + path : "file:///" + path
            player.open_files(Tools.local_file(url), true)
            t.next()
            break
        }
        case 31:
            t.waitFor("dropped_file_appended", () => s.playlist_count === t.before.playlist_count + 1, 3000,
                      () => "playlist " + t.before.playlist_count + " -> " + s.playlist_count)
            break
        case 32:
            // M3: the playlist panel, albums, normalizer, editor, instance.
            t.step = 100
            t.stepStarted = Date.now()
            break
        default:
            if (t.step >= 300)
                m6Tick()
            else if (t.step >= 200)
                m4Tick()
            else
                m3Tick()
        }
    }

    // How many commands a menu's JSON has, submenus included.
    function countCommands(items) {
        let count = 0
        for (const item of items)
            count += item.children.length > 0 ? countCommands(item.children) : (item.id !== "" ? 1 : 0)
        return count
    }

    // The item at a path of labels in a menu's JSON.
    function menuItem(items, path) {
        let found = null
        for (const label of path) {
            found = items.find(item => item.label === label)
            if (!found)
                return null
            items = found.children
        }
        return found
    }

    // M4 and M5. Each step that may cost video frames is followed by a
    // pause and a `settled_*` line, so the drop counts show which step
    // costs them (goal F's open item).
    function m4Tick() {
        const t = selftest
        const s = t.status()
        switch (t.step) {
        case 200: {
            Tools.log("m4_start=" + JSON.stringify({ path: s.path, time: s.time, pos: s.playlist_pos,
                                                     album: s.playing_album, pause: s.pause }))
            // The menus show PotPlayer's tree with the keys bound now.
            const context = JSON.parse(player.menu_json("context"))
            const play = root.menuItem(context, ["Playback", "Play/Pause Playback"])
            const frame = root.menuItem(context, ["Playback", "Jump (to)", "Previous frame"])
            const normalizer = root.menuItem(context, ["Audio", "Audio Effects", "Normalizer"])
            t.check("context_menu_keys", play !== null && play.shortcut === "Space"
                        && frame !== null && frame.shortcut === "D",
                    "play " + (play && play.shortcut) + " frame " + (frame && frame.shortcut))
            t.check("context_menu_marks", normalizer !== null && normalizer.checked === (s.normalizer ? 1 : 0),
                    "normalizer " + (normalizer && normalizer.checked))
            const list = JSON.parse(player.menu_json("playlist"))
            const undo = root.menuItem(list, ["Undo"])
            t.check("playlist_menu_keys", list[0].id === "l:play" && undo !== null && undo.shortcut === "Ctrl+Z",
                    "first " + list[0].label + " undo " + (undo && undo.shortcut))
            // Built as Qt Quick menus (opening needs real input on Wayland);
            // submenus fill when they open.
            menuLoader.active = true
            const started = Date.now()
            const count = menuLoader.item.build("context").count
            const ms = Date.now() - started
            const commands = root.countCommands(context)
            t.check("context_menu_builds", count > 15 && commands > 100,
                    count + " top items in " + ms + " ms, " + commands + " commands")
            const listCount = menuLoader.item.build("playlist").count
            t.check("playlist_menu_builds", listCount > 5 && root.countCommands(list) > 15,
                    listCount + " top items, " + root.countCommands(list) + " commands")
            // A press on the video that closed a menu stays with the menu,
            // and so does its release: no play/pause (as a menu begins to
            // close when Qt hands such a press on).
            menuLoader.item.closedAt = Date.now()
            const video = root.middle(player.video_rect)
            t.before = s
            root.pointerPress(video.x, video.y, Qt.LeftButton, 0)
            const kept = root.swallowing
            root.pointerRelease(video.x, video.y, Qt.LeftButton)
            t.check("menu_closing_press_kept", kept && !root.swallowing, "kept " + kept)
            t.next()
            break
        }
        case 201:
            if (root.settled("menus")) {
                t.check("menu_closing_press_no_pause", s.pause === t.before.pause,
                        "pause " + t.before.pause + " -> " + s.pause)
                player.run_menu_command("w:speed-up")
            }
            break
        case 202:
            // A menu command runs its action.
            t.waitFor("menu_command_runs", () => s.speed !== null && Math.abs(s.speed - 1.1) < 0.01,
                      2000, () => "speed " + s.speed)
            break
        case 203:
            if (root.settled("speed_up"))
                player.run_menu_command("w:speed-default")
            break
        case 204:
            t.waitFor("speed_back", () => s.speed !== null && Math.abs(s.speed - 1) < 0.01, 2000,
                      () => "speed " + s.speed)
            break
        case 205:
            if (root.settled("speed_back"))
                player.key(Qt.Key_F5, 0)
            break
        case 206:
            // F5 opens the settings window.
            t.waitFor("f5_opens_settings", () => settingsLoader.status === Loader.Ready
                          && settingsLoader.item.visible, 3000)
            break
        case 207: {
            if (!root.settled("settings_open"))
                break
            if (settingsLoader.status !== Loader.Ready) {
                t.step = 220
                break
            }
            const win = settingsLoader.item
            const loaded = []
            for (let i = 0; i < win.pages.length; i++) {
                win.showPage(i)
                const page = win.pageItem(i)
                if (page && page.visible && page.width > 0)
                    loaded.push(win.pages[i].key)
            }
            t.check("settings_pages_load", loaded.length === win.pages.length, loaded.join(","))
            win.showPage(win.pages.findIndex(p => p.key === "input"))
            const rows = JSON.parse(player.editor_rows())
            const owner = JSON.parse(player.editor_key_owner("window", Qt.Key_Space, 0))
            t.check("keys_page_rows", t.status().editor_open && rows.length > 50, rows.length + " actions")
            t.check("keys_page_finds_owner", owner.owner === "play-pause", owner.key + " -> " + owner.owner)
            break
        }
        case 208:
            // A picture of the window for the record (a grab costs frames).
            if (root.settled("settings_pages"))
                Tools.grab(t.outDir, "settings")
            break
        case 209: {
            if (!root.settled("settings_grab"))
                break
            // A changed setting applies at once: a bigger volume step, set
            // with its slider (the action, as Down goes to the playlist while
            // it has the keys).
            const win = settingsLoader.item
            win.showPage(win.pages.findIndex(p => p.key === "audio"))
            win.amount("volume-step").pick(5)
            t.before = t.status()
            player.run_action("volume-down")
            break
        }
        case 210:
            t.waitFor("setting_applies_at_once", () => s.volume_step === 5 && s.volume !== null
                          && Math.abs(s.volume - (t.before.volume - 5)) < 0.01, 2000,
                      () => "step " + s.volume_step + " volume " + t.before.volume + " -> " + s.volume)
            break
        case 211: {
            if (!root.settled("setting_change"))
                break
            // The sliders apply what they show: the volume maximum went back
            // to 200 % when it was typed (Number() failed in that file).
            const win = settingsLoader.item
            win.amount("volume-max").pick(100)
            const lowered = t.status().volume_max
            win.amount("volume-max").pick(200)
            win.amount("volume-step").pick(1)
            t.check("settings_sliders_apply", lowered === 100 && t.status().volume_max === 200,
                    "volume-max " + lowered + " then " + t.status().volume_max)
            t.check("setting_restored", t.status().volume_step === 1, "step " + t.status().volume_step)
            settingsLoader.item.close()
            break
        }
        case 212:
            t.waitFor("settings_window_closes", () => !settingsLoader.item.visible && !s.editor_open, 2000)
            break
        case 213:
            root.settled("settings_close")
            break
        default:
            // M6: the control panel, video processing and SVP.
            t.step = 300
            t.stepStarted = Date.now()
            break
        }
    }

    function panelClick(command) {
        const r = player.panel_rect(command)
        const p = root.middle(r)
        player.panel_press(p.x, p.y, Qt.LeftButton)
        player.panel_release(p.x, p.y, Qt.LeftButton)
        return r.width > 0
    }
    // A press on a slider of the panel at `fraction` of its length (from
    // the left, or from the bottom for vertical ones).
    function panelSlide(command, fraction, vertical) {
        const r = player.panel_rect(command)
        const x = vertical ? r.x + r.width / 2 : r.x + fraction * (r.width - 1)
        const y = vertical ? r.y + (1 - fraction) * (r.height - 1) : r.y + r.height / 2
        player.panel_press(x, y, Qt.LeftButton)
        player.panel_release(x, y, Qt.LeftButton)
        return r.width > 0
    }

    // M6: F7's control panel (driven through its own pointer handlers), a
    // colour and an equalizer change, a shader toggle and SVP's diagnostic.
    function m6Tick() {
        const t = selftest
        const s = t.status()
        switch (t.step) {
        case 300:
            Tools.log("m6_start=" + JSON.stringify({ path: s.path, time: s.time, pos: s.playlist_pos,
                                                     album: s.playing_album, pause: s.pause }))
            if (!player.has_skin) {
                Tools.log("check f7_opens_panel=SKIPPED no skin")
                t.step = 315
                break
            }
            player.key(Qt.Key_F7, 0)
            t.next()
            break
        case 301:
            t.waitFor("f7_opens_panel", () => s.panel_shown && panelLoader.status === Loader.Ready
                          && panelLoader.item.visible && player.panel_rect("CMD_EQULIZER_0").width > 0,
                      3000, () => "panel " + player.panel_width + "x" + player.panel_height)
            break
        case 302:
            if (root.settled("panel_open"))
                root.panelClick("CMD_SELECT_PAGE:1")
            break
        case 303:
            if (t.waitFor("panel_shows_video_page", () => s.panel_page === 1, 2000,
                          () => "page " + s.panel_page)) {
                t.room()
                root.panelSlide("CMD_COLOR_OFFSET", 0.75, false)
            }
            break
        case 304: {
            t.waitFor("color_slider_applies", () => {
                if (s.brightness < 30 || s.brightness > 70)
                    return false
                const d = JSON.parse(player.diagnostics())
                return d.brightness === s.brightness
            }, 2000, () => "brightness " + s.brightness + " mpv " + JSON.parse(player.diagnostics()).brightness)
            break
        }
        case 305:
            if (root.settled("color_change"))
                root.panelClick("CMD_COLOR_OFFSET_DEFAULT")
            break
        case 306:
            if (t.waitFor("color_reset_applies", () => s.brightness === 0
                              && JSON.parse(player.diagnostics()).brightness === 0, 2000,
                          () => "brightness " + s.brightness))
                root.panelClick("CMD_SELECT_PAGE:0")
            break
        case 307:
            if (s.panel_page === 0) {
                t.before = s
                root.panelSlide("CMD_EQULIZER_4", 1, true)
                t.next()
            } else if (Date.now() - t.stepStarted > 2000) {
                t.check("panel_shows_audio_page", false, "page " + s.panel_page)
                t.step = 311
            }
            break
        case 308:
            // The chain: PotPlayer's downmix (stereo speakers, the
            // default), the equalizer, the normalizer.
            t.waitFor("equalizer_slider_applies", () => s.eq_enabled && s.eq_bands[4] > 11
                          && s.af !== null && s.af.indexOf("rvpdownmix") >= 0
                          && s.af.indexOf("rvpdownmix") < s.af.indexOf("rvpeq")
                          && s.af.indexOf("rvpeq") < s.af.indexOf("rvpnorm"), 3000,
                      () => "band " + s.eq_bands[4] + " af " + s.af)
            break
        case 309:
            if (root.settled("equalizer_change"))
                root.panelClick("ID_AUDIO_EQULIZER")
            break
        case 310:
            t.waitFor("equalizer_button_toggles", () => !s.eq_enabled && s.af !== null
                          && s.af.indexOf("rvpeq") < 0 && s.af.indexOf("rvpnorm") >= 0
                          && s.af.indexOf("rvpdownmix") >= 0
                          && s.af.indexOf("rvpdownmix") < s.af.indexOf("rvpnorm"), 3000,
                      () => "af " + s.af)
            break
        case 311:
            // The panel's volume slider runs to the highest volume and shows
            // the volume on the OSD, as the keys do (owner, 2026-10-16).
            t.before = s
            root.panelSlide("CMD_MASTER_VOLUME", 1, true)
            t.next()
            break
        case 312:
            if (t.waitFor("volume_slider_reaches_max", () => s.volume !== null && s.volume_max !== null
                              && Math.abs(s.volume - s.volume_max) < 0.5
                              && String(s.osd).indexOf("Volume") >= 0, 2000,
                          () => "volume " + s.volume + " max " + s.volume_max + " osd " + s.osd))
                root.panelSlide("CMD_MASTER_VOLUME", t.before.volume / s.volume_max, true)
            break
        case 313:
            player.key(Qt.Key_F7, 0)
            t.next()
            break
        case 314:
            t.waitFor("f7_closes_panel", () => !s.panel_shown
                          && (panelLoader.item === null || !panelLoader.item.visible), 2000)
            break
        case 315: {
            // A shader of RVP's own that changes nothing.
            const path = Tools.write_file(t.outDir + "/selftest.glsl",
                "//!HOOK MAIN\n//!BIND HOOKED\n//!DESC RVP self-test (no change)\n"
                + "vec4 hook() { return HOOKED_texOff(0); }\n")
            const result = JSON.parse(player.set_setting("shaders.files", JSON.stringify([path])))
            t.check("shader_list_set", path !== "" && !result.error, path + " " + (result.error || ""))
            player.key(Qt.Key_P, Qt.ShiftModifier)
            t.next()
            break
        }
        case 316:
            t.waitFor("shift_p_shaders_on", () => s.shaders_enabled
                          && String(JSON.parse(player.diagnostics()).glsl_shaders).indexOf("selftest.glsl") >= 0,
                      2000, () => "shaders " + JSON.parse(player.diagnostics()).glsl_shaders)
            break
        case 317:
            if (root.settled("shaders_on"))
                player.key(Qt.Key_P, Qt.ShiftModifier)
            break
        case 318:
            t.waitFor("shift_p_shaders_off", () => !s.shaders_enabled
                          && String(JSON.parse(player.diagnostics()).glsl_shaders) === "", 2000,
                      () => "shaders " + JSON.parse(player.diagnostics()).glsl_shaders)
            break
        case 319: {
            // SVP mode on an endpoint of the test's own: SVP does not attach
            // (no Manager on the test machines), and RVP says why.
            const endpoint = Qt.platform.os === "windows" ? "rvp-selftest-svp" : t.outDir + "/svp.sock"
            t.room()
            const result = JSON.parse(player.set_setting("ipc-server", JSON.stringify(endpoint)))
            t.check("svp_mode_on", !result.error && s.loaded, endpoint + " " + (result.error || ""))
            t.next()
            break
        }
        case 320:
            t.waitFor("svp_diagnostic_shows", () => String(s.svp_status).startsWith("SVP did not attach")
                          && s.message === s.svp_status
                          && JSON.parse(player.settings_view(false)).svp_status === s.svp_status,
                      20000, () => "status " + s.svp_status + " message " + s.message + " playing "
                          + s.playing + " time " + s.time + " path " + s.path)
            break
        case 321: {
            const result = JSON.parse(player.set_setting("ipc-server", "\"\""))
            const status = t.status().svp_status
            t.check("svp_mode_off", !result.error && status === "SVP mode is off", "status " + status)
            t.next()
            break
        }
        // M10: an effect in the chain and out, PotPlayer's speaker choices,
        // the individual volume, Freeverb's response, the sync dialog, the
        // second device, recording, the system's master volume and Audio
        // Stretching.
        case 322: {
            // The window takes the keyboard back from the playlist (whose
            // Shift+C copies files, M11): two clicks on the video, which
            // pause and play again.
            const cx = player.video_rect.x + player.video_rect.width / 2
            const cy = player.video_rect.y + player.video_rect.height / 2
            for (let i = 0; i < 2; ++i) {
                player.mouse_press(cx, cy, Qt.LeftButton, 0)
                player.mouse_release(cx, cy, Qt.LeftButton)
            }
            t.room()
            player.key(Qt.Key_C, Qt.ShiftModifier)
            t.next()
            break
        }
        case 323:
            if (t.waitFor("shift_c_crystality_on", () => s.af !== null && s.af.indexOf("rvpcrystality") >= 0
                              && String(s.osd).indexOf("Crystality: On") >= 0, 3000,
                          () => "af " + s.af + " osd " + s.osd))
                player.key(Qt.Key_C, Qt.ShiftModifier)
            break
        case 324:
            t.waitFor("shift_c_crystality_off", () => s.af !== null && s.af.indexOf("rvpcrystality") < 0
                          && s.af.indexOf("rvpnorm") >= 0, 3000, () => "af " + s.af)
            break
        case 325: {
            const find = (items, label) => {
                for (const item of items) {
                    if (item.label === label)
                        return item
                    const inner = find(item.children || [], label)
                    if (inner)
                        return inner
                }
                return null
            }
            const speakers = find(JSON.parse(player.menu_json("context")), "Output Speakers")
            t.check("speaker_menu_lists_potplayers_26", speakers !== null && speakers.children.length === 26,
                    speakers ? speakers.children.length + " choices" : "no Output Speakers")
            const result = JSON.parse(player.set_setting("audio.channel-volumes.enabled", "true"))
            t.check("individual_volume_set", !result.error, result.error || "")
            t.next()
            break
        }
        case 326:
            if (t.waitFor("individual_volume_in_chain", () => s.af !== null && s.af.indexOf("rvpgains") >= 0
                              && s.af.indexOf("rvpnorm") < s.af.indexOf("rvpgains"), 3000,
                          () => "af " + s.af)) {
                player.set_setting("audio.channel-volumes.enabled", "false")
                player.key(Qt.Key_R, Qt.ShiftModifier)
            }
            break
        case 327:
            if (t.waitFor("shift_r_freeverb_on", () => s.af !== null && s.af.indexOf("rvpreverb") >= 0
                              && s.af.indexOf("rvpgains") < 0 && s.af.indexOf(".wav") >= 0, 3000,
                          () => "af " + s.af))
                player.key(Qt.Key_R, Qt.ShiftModifier)
            break
        case 328:
            if (t.waitFor("shift_r_freeverb_off", () => s.af !== null && s.af.indexOf("rvpreverb") < 0,
                          3000, () => "af " + s.af))
                player.run_action("audio-sync-dialog")
            break
        case 329:
            if (t.waitFor("audio_sync_dialog_opens", () => syncLoader.status === Loader.Ready
                              && syncLoader.item.dialog.opened, 3000)) {
                syncLoader.item.dialog.close()
                // The second device: its core gets --mpv-options too, so
                // the self-test's ao=null keeps it silent.
                const result = JSON.parse(player.set_setting("audio.second-device", "\"auto\""))
                t.check("second_device_set", !result.error, result.error || "")
            }
            break
        case 330:
            if (t.waitFor("second_device_follows", () => s.second_device === "auto"
                              && s.second_offset_ms !== null && Math.abs(s.second_offset_ms) < 80
                              && (s.second_playing || !s.playing), 15000,
                          () => "device " + s.second_device + " playing " + s.second_playing
                              + " offset " + s.second_offset_ms + " main playing " + s.playing))
                player.set_setting("audio.second-device", "\"\"")
            break
        case 331:
            if (t.waitFor("second_device_off", () => s.second_device === "", 3000,
                          () => "device " + s.second_device))
                player.start_recording(t.outDir + "/recording.wav")
            break
        case 332: {
            // Windows records RVP's own sound (process loopback); Linux
            // with goal S, and a system without audio refuses: its refusal
            // is on the OSD, and the recorder never starts.
            if (s.recording === true) {
                if (Date.now() - t.stepStarted > 2000) {
                    t.check("recording_starts", true, String(s.osd))
                    player.run_action("record-audio-toggle")
                    t.next()
                }
            } else if (Date.now() - t.stepStarted > 1500) {
                Tools.log("check recording_starts=SKIPPED " + s.osd)
                t.step = 334
                t.stepStarted = Date.now()
            }
            break
        }
        case 333:
            t.waitFor("recording_stops", () => s.recording === false
                          && String(s.osd).indexOf("Audio recording stopped: ") === 0,
                      5000, () => "recording " + s.recording + " osd " + s.osd)
            break
        case 334:
            // The system's master volume, up and back down: the device's
            // answer (Windows), or why not.
            player.run_action("system-master-up")
            t.next()
            break
        case 335:
            if (t.waitFor("master_volume_answers", () => String(s.osd).toLowerCase().indexOf("master volume") >= 0,
                          3000, () => "osd " + s.osd))
                player.run_action("system-master-down")
            break
        case 336:
            // Audio Stretching: Shift+J raises the pitch 1 % (rubberband in
            // the tempo filter's place), Shift+K takes it back. Away from
            // the file's end, where chain changes wait for the next file
            // (a slow machine gets near the short file's end by now).
            if (s.duration > 0 && s.duration - s.time < 8) {
                if (Date.now() - t.roomAsked > 2000) {
                    t.roomAsked = Date.now()
                    if (s.duration < 20)
                        player.run_action("previous")
                    else
                        player.seek_fraction(0.25)
                }
                if (Date.now() - t.stepStarted > 15000) {
                    t.check("audio_stretching_room", false, "time " + s.time + " of " + s.duration)
                    t.next()
                }
                break
            }
            player.key(Qt.Key_J, Qt.ShiftModifier)
            t.next()
            break
        case 337:
            if (t.waitFor("shift_j_audio_stretching_up", () => s.af !== null && s.af.indexOf("rubberband") >= 0
                              && String(s.osd).indexOf("Audio Stretching : 101%") >= 0, 3000,
                          () => "af " + s.af + " osd " + s.osd))
                player.key(Qt.Key_K, Qt.ShiftModifier)
            break
        case 338:
            t.waitFor("shift_k_audio_stretching_off", () => s.af !== null && s.af.indexOf("rubberband") < 0
                          && String(s.osd).indexOf("Audio Stretching : 100%") >= 0, 3000,
                      () => "af " + s.af + " osd " + s.osd)
            break
        default:
            // Back to M3's steps.
            t.step = 126
            t.stepStarted = Date.now()
            break
        }
    }

    // After a step that may cost frames: once 1.5 s passed, logs the drops
    // so far as `check settled_NAME=PASS [drops N]` and goes on; returns
    // true at that tick.
    function settled(name) {
        const t = selftest
        if (Date.now() - t.stepStarted < 1500)
            return false
        t.check("settled_" + name, true, "")
        t.next()
        return true
    }

    function middle(r) { return { x: r.x + r.width / 2, y: r.y + r.height / 2 } }
    function clickAt(p, modifiers) {
        player.mouse_press(p.x, p.y, Qt.LeftButton, modifiers)
        player.mouse_release(p.x, p.y, Qt.LeftButton)
    }

    function m3Tick() {
        const t = selftest
        const s = t.status()
        switch (t.step) {
        case 100:
            if (!player.has_skin || !s.playlist_attached) {
                Tools.log("check playlist_panel=SKIPPED no panel")
                t.step = 117
                break
            }
            // A click selects the row and gives the panel the keyboard.
            clickAt(middle(player.list_rect("row", 1)), 0)
            t.next()
            break
        case 101:
            t.waitFor("list_click_selects", () => s.cursor === 1 && s.selected === 1
                          && s.focus === "playlist", 2000,
                      () => "cursor " + s.cursor + " selected " + s.selected + " focus " + s.focus)
            break
        case 102:
            player.key(Qt.Key_A, Qt.ControlModifier)
            t.next()
            break
        case 103:
            t.waitFor("list_ctrl_a_selects_all", () => s.selected === s.shown_entries, 2000,
                      () => s.selected + " of " + s.shown_entries)
            break
        case 104:
            t.before = s
            player.key(Qt.Key_Delete, 0)
            t.next()
            break
        case 105:
            t.waitFor("list_del_removes", () => s.shown_entries === 0, 2000,
                      () => "entries " + s.shown_entries)
            break
        case 106:
            player.key(Qt.Key_Z, Qt.ControlModifier)
            t.next()
            break
        case 107:
            t.waitFor("list_ctrl_z_undoes", () => s.shown_entries === t.before.shown_entries, 2000,
                      () => "entries " + s.shown_entries)
            break
        case 108:
            // Only the first row selected, then dragged below the last one.
            clickAt(middle(player.list_rect("row", 0)), 0)
            t.next()
            break
        case 109: {
            t.before = s
            const from = middle(player.list_rect("row", 0))
            const last = player.list_rect("row", s.shown_entries - 1)
            t.dragTo = { x: from.x, y: last.y + last.height - 2 }
            player.mouse_press(from.x, from.y, Qt.LeftButton, 0)
            player.mouse_move(from.x, from.y + 8)
            player.mouse_move(t.dragTo.x, t.dragTo.y)
            t.next()
            break
        }
        case 110:
            t.check("list_drag_marks_drop", s.drop_before === s.shown_entries,
                    "drop before " + s.drop_before)
            player.mouse_release(t.dragTo.x, t.dragTo.y, Qt.LeftButton)
            t.next()
            break
        case 111:
            t.waitFor("list_drag_reorders", () => s.shown_first !== t.before.shown_first
                          && s.drop_before === null, 2000,
                      () => t.before.shown_first + " -> " + s.shown_first)
            break
        case 112:
            t.before = s
            player.mouse_double(middle(player.list_rect("row", 0)).x,
                                middle(player.list_rect("row", 0)).y, Qt.LeftButton)
            t.next()
            break
        case 113:
            t.waitFor("list_double_click_plays", () => s.playlist_pos === 0 && s.loaded
                          && String(s.path).endsWith(t.before.shown_first), 5000,
                      () => "entry " + s.playlist_pos + " path " + s.path)
            break
        case 114:
            t.before = s
            player.run_list_action("add-album")
            t.next()
            break
        case 115:
            if (albumLoader.status === Loader.Ready && albumLoader.item.dialog.opened) {
                albumLoader.item.text = "Selftest"
                albumLoader.item.dialog.accept()
                t.next()
            } else if (Date.now() - t.stepStarted > 3000) {
                t.check("album_added", false, "no album name dialog")
                t.step = 117
            }
            break
        case 116:
            if (t.waitFor("album_added", () => s.albums === t.before.albums + 1
                              && s.shown_album === t.before.shown_album + 1 && s.shown_entries === 0,
                          2000, () => "albums " + s.albums + " shown " + s.shown_album))
                clickAt(middle(player.list_rect("tab", 0)), 0)
            break
        case 117:
            if (t.waitFor("tab_click_shows_album", () => s.shown_album === 0 || !player.has_skin, 2000,
                          () => "shown " + s.shown_album))
                Tools.log("m3_tabs=" + JSON.stringify({ path: s.path, time: s.time, pos: s.playlist_pos,
                                                        album: s.playing_album, pause: s.pause }))
            break
        case 118:
            player.key(Qt.Key_N, Qt.ShiftModifier)
            t.next()
            break
        case 119:
            if (t.waitFor("shift_n_normalizer_off", () => s.normalizer === false
                              && (s.af === null || s.af.indexOf("rvpnorm") < 0), 2000,
                          () => "af " + s.af))
                player.key(Qt.Key_N, Qt.ShiftModifier)
            break
        case 120:
            t.waitFor("shift_n_normalizer_on", () => s.normalizer === true && s.af !== null
                          && s.af.indexOf("rvpnorm") >= 0, 2000, () => "af " + s.af)
            break
        case 121:
            // Volume goes past 100 % (up to volume-max, 200 %).
            if (s.volume !== null && s.volume > 100.5) {
                t.check("volume_above_100", true, "volume " + s.volume + " osd " + s.osd)
                t.next()
            } else if (Date.now() - t.stepStarted > 5000) {
                t.check("volume_above_100", false, "volume " + s.volume)
                t.next()
            } else {
                player.run_action("volume-up")
            }
            break
        case 122:
            // M4 and M5: the menus and the settings window.
            t.step = 200
            t.stepStarted = Date.now()
            break
        case 126: {
            const name = player.option("instance")
            if (name === "") {
                Tools.log("check second_instance_adds=SKIPPED no --instance")
                t.step = 128
                break
            }
            t.before = s
            Tools.run_self(["--instance=" + name, "/add", String(s.path)])
            t.next()
            break
        }
        case 127:
            t.waitFor("second_instance_adds", () => s.shown_entries === t.before.shown_entries + 1,
                      20000, () => "entries " + t.before.shown_entries + " -> " + s.shown_entries)
            break
        case 128:
            if (root.visibility !== Window.Windowed || !s.video_aspect) {
                Tools.log("check window_keeps_aspect=SKIPPED")
                t.step = 130
                break
            }
            root.setGeometry(root.x, root.y, 900, 760)
            t.next()
            break
        case 129:
            t.waitFor("window_keeps_aspect", () => root.height < 760 && player.video_rect.height > 0
                          && Math.abs(player.video_rect.width / player.video_rect.height
                                      - s.video_aspect) < 0.03, 4000,
                      () => "video " + player.video_rect.width + "x" + player.video_rect.height
                          + " aspect " + s.video_aspect + " window " + root.width + "x" + root.height)
            break
        case 130:
            // Let a few seconds of playback run for the measurements.
            if (Date.now() - t.stepStarted > 2000)
                t.finish()
            break
        }
    }
}
