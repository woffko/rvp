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
    width: 1280
    height: 720
    minimumWidth: 320
    minimumHeight: 200
    visible: false
    color: "black"
    title: player.window_title
    flags: Qt.Window | Qt.FramelessWindowHint | (player.on_top ? Qt.WindowStaysOnTopHint : 0)

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

    Component.onCompleted: {
        player.start()
        // Requests from other instances (rvp /add ...) go to the player.
        Tools.instance_receiver(player)
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
    onVisibilityChanged: {
        player.window_state(root.visibility === Window.Maximized, root.active)
        if (Date.now() - root.stateRequested < 2000)
            stateCheck.restart()
        else if (root.isFullscreen !== player.fullscreen && root.visibility !== Window.Minimized
                 && root.visibility !== Window.Hidden)
            player.window_fullscreen(root.isFullscreen)
    }
    onActiveChanged: player.window_state(root.visibility === Window.Maximized, root.active)
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
            dialogLoader.item.askSubtitle()
            break
        case requestOpenExternal:
            Qt.openUrlExternally(player.external_url)
            break
        case requestAbout:
            aboutLoader.active = true
            aboutLoader.item.ask()
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
        }
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
            player.mouse_press(mouse.x, mouse.y, mouse.button, mouse.modifiers)
        }
        onReleased: (mouse) => player.mouse_release(mouse.x, mouse.y, mouse.button)
        onDoubleClicked: (mouse) => player.mouse_double(mouse.x, mouse.y, mouse.button)
        onPositionChanged: (mouse) => {
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
            if (tipLoader.item)
                tipLoader.item.tip.close()
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

    // The OSD: short notices over the video (volume, normalizer), as
    // PotPlayer shows them at the top left.
    Text {
        id: osdText
        visible: player.osd !== ""
        z: 20
        x: player.video_rect.x + 16
        y: player.video_rect.y + 12
        text: player.osd
        color: "#f0f0f0"
        style: Text.Outline
        styleColor: "#000000"
        font.pixelSize: 20
        onTextChanged: if (text !== "") osdTimer.restart()
        Timer {
            id: osdTimer
            interval: 2000
            onTriggered: player.osd = ""
        }
    }

    // Playback information (Tab), refreshed every second while shown.
    Rectangle {
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
            font.pixelSize: 14
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
    //                         kit's kill test)
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
        property int failures: 0
        property var before: ({})
        property var dragTo: ({})
        property bool nextSent: false
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
        functionalTick()
    }

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
            root.requestVisibility(Window.Windowed)
            t.next()
            break
        case 23:
            // The compositor restores the old size first; resize after it.
            if (root.visibility === Window.Windowed && Date.now() - t.stepStarted > 500) {
                root.width = 900
                root.height = 500
                t.next()
            } else if (Date.now() - t.stepStarted > 3000) {
                t.check("restored_window", false, "visibility " + root.visibility)
                t.next()
            }
            break
        case 24:
            t.waitFor("relayout_on_resize", () => player.video_rect.width > 0
                          && player.video_rect.x + player.video_rect.width <= root.width
                          && player.video_rect.y + player.video_rect.height <= root.height
                          && root.width === 900, 3000,
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
            if (t.step >= 200)
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
            t.next()
            break
        }
        case 201:
            if (root.settled("menus"))
                player.run_menu_command("w:speed-up")
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
            // A changed setting applies at once: a bigger volume step (the
            // action, as Down goes to the playlist while it has the keys).
            const win = settingsLoader.item
            win.showPage(win.pages.findIndex(p => p.key === "audio"))
            win.set("volume-step", 5)
            t.before = t.status()
            player.run_action("volume-down")
            break
        }
        case 210:
            t.waitFor("setting_applies_at_once", () => s.volume_step === 5 && s.volume !== null
                          && Math.abs(s.volume - (t.before.volume - 5)) < 0.01, 2000,
                      () => "step " + s.volume_step + " volume " + t.before.volume + " -> " + s.volume)
            break
        case 211:
            if (!root.settled("setting_change"))
                break
            settingsLoader.item.set("volume-step", 1)
            t.check("setting_restored", t.status().volume_step === 1, "step " + t.status().volume_step)
            settingsLoader.item.close()
            break
        case 212:
            t.waitFor("settings_window_closes", () => !settingsLoader.item.visible && !s.editor_open, 2000)
            break
        case 213:
            root.settled("settings_close")
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
            t.waitFor("tab_click_shows_album", () => s.shown_album === 0 || !player.has_skin, 2000,
                      () => "shown " + s.shown_album)
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
            root.width = 900
            root.height = 760
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
