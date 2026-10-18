// M12's self-test (`rvp --selftest=m12`), loaded by Main.qml's self-test:
// the Window Size commands (sizes, the size dialogs, moves, fullscreen on
// another display, staying on top in fullscreen), capture (BMP through Qt,
// the screen frame through mpv, the clipboard, consecutive images, the
// thumbnail image) and the settings window's tree with its tabs. Input
// goes through Player's handlers, as in the functional check.
//
// Files: testdata/media/fixture-60s.mkv.
import QtQuick
import Rvp
import RvpNative

Item {
    id: m12
    required property Player player
    required property var t
    required property var host
    property var before: ({})
    property string images: ""
    property string lastSaved: ""
    property int filesBefore: 0
    // --selftest=shaders: the HLSL shaders and where mpv's log was read to.
    property var hlsl: []
    property int shaderIndex: 0
    property int logOffset: 0
    property int shaderFailures: 0

    function setSetting(path, value) {
        return JSON.parse(player.set_setting(path, JSON.stringify(value)))
    }
    function area() { return host.screenArea() }
    function near(value, wanted, tolerance) { return Math.abs(value - wanted) <= tolerance }
    function files() { return JSON.parse(Tools.list_files(images)) }
    function dialogs() { return host.videoDialogs() }
    function moves() { return Qt.platform.pluginName !== "wayland" }
    // The file the last "Saved NAME" notice named.
    function saved(s) {
        const osd = s.osd || ""
        return osd.startsWith("Saved ") && !osd.startsWith("Saved the thumbnail") ? osd.substring(6) : ""
    }

    // `rvp --selftest=shaders --mpv-log=FILE --mpv-options=log-file=FILE`:
    // every shader of the catalogue, one at a time: RVP's set in the list
    // of its stage, the HLSL ones (PotPlayer's PxShader folder, the
    // settings' folders) translated into the post-resize list; the errors
    // mpv logs meanwhile say whether it compiled.
    function shaderTick() {
        const t = m12.t
        const s = t.status()
        const log = player.option("mpv-log")
        switch (t.step) {
        case 0: {
            const view = JSON.parse(player.settings_view(false))
            hlsl = view.choices["shader-choices"] || []
            t.check("shader_catalogue", hlsl.length > 0 && log !== "",
                    hlsl.filter(c => c.stage === "any").length + " HLSL shaders of " + hlsl.length + ", log " + log)
            t.next()
            break
        }
        case 1:
            t.waitFor("shader_file_plays", () => s.loaded && s.time > 0.3, 20000)
            break
        case 2:
            if (shaderIndex >= hlsl.length) {
                t.check("shaders_compile", shaderFailures === 0,
                        (hlsl.length - shaderFailures) + " of " + hlsl.length + " compiled")
                t.finish()
                break
            }
            logOffset += Tools.read_file(log, logOffset).length
            if (hlsl[shaderIndex].stage === "pre") {
                setSetting("shaders.post-files", [])
                setSetting("shaders.files", [hlsl[shaderIndex].path])
                setSetting("shaders.enabled", true)
            } else {
                setSetting("shaders.files", [])
                setSetting("shaders.post-files", [hlsl[shaderIndex].path])
                setSetting("shaders.post-enabled", true)
            }
            t.next()
            break
        case 3: {
            if (Date.now() - t.stepStarted < 800)
                break
            const text = Tools.read_file(log, logOffset)
            logOffset += text.length
            const errors = text.split("\n").filter(line => line.indexOf("][e][") >= 0)
            const name = hlsl[shaderIndex].name
            t.check("shader " + name, errors.length === 0, errors.slice(0, 3).join(" | "))
            // The picture with the shader, for a look (a grab costs frames).
            if (player.has_option("shader-grabs"))
                Tools.grab(t.outDir, "shader-" + shaderIndex)
            if (errors.length > 0)
                shaderFailures += 1
            shaderIndex += 1
            t.step = 2
            break
        }
        }
    }

    function tick() {
        const t = m12.t
        const s = t.status()
        switch (t.step) {
        case 0:
            setSetting("fit-window-to-video", false)
            images = t.outDir + "/m12-images"
            setSetting("image-folder", images)
            host.width = 1000
            host.height = 700
            t.next()
            break
        case 1:
            if (t.waitFor("m12_file_plays", () => s.loaded && s.video_width > 0 && s.time > 0.5, 20000,
                          () => "path " + s.path)) {
                player.run_action("window-custom")
            }
            break
        // The window's sizes (R1's Window Size menu).
        case 2:
            if (t.waitFor("window_custom_size", () => near(host.width, 960, 2), 3000,
                          () => "width " + host.width))
                player.run_action("window-minimum")
            break
        case 3:
            if (t.waitFor("window_minimum", () => host.width === host.minimumWidth, 3000,
                          () => "width " + host.width + " minimum " + host.minimumWidth))
                player.run_action("window-maximum")
            break
        case 4: {
            const a = area()
            if (t.waitFor("window_maximum_fills", () => host.width >= a.width - 2 || host.height >= a.height - 2,
                          3000, () => host.width + "x" + host.height + " area " + a.width + "x" + a.height))
                player.run_action("window-desktop-1")
            break
        }
        case 5: {
            const a = area()
            const box = (v, whole) => v <= whole * 0.3 + 2
            if (t.waitFor("window_desktop_size", () => box(host.width, a.width) && box(host.height, a.height)
                              && (host.width >= a.width * 0.3 - 2 || host.height >= a.height * 0.3 - 2),
                          3000, () => host.width + "x" + host.height + " area " + a.width + "x" + a.height)) {
                before = { width: host.width }
                player.run_action("window-larger")
            }
            break
        }
        case 6:
            if (t.waitFor("window_grows", () => host.width > before.width, 3000,
                          () => before.width + " -> " + host.width)) {
                before = { x: host.x }
                player.run_action("window-move-left")
            }
            break
        case 7:
            if (!moves()) {
                t.check("window_moves", true, "SKIPPED Wayland places windows itself")
                t.next()
                break
            }
            if (t.waitFor("window_moves", () => host.x === before.x - 20 || host.x === area().x, 3000,
                          () => before.x + " -> " + host.x))
                player.run_action("window-top-left")
            break
        case 8:
            if (!moves()) {
                t.next()
                break
            }
            if (t.waitFor("window_corner", () => host.x === area().x && host.y === area().y, 3000,
                          () => host.x + "," + host.y + " area " + area().x + "," + area().y))
                player.run_action("window-custom-dialog")
            break
        case 9:
            if (!moves())
                player.run_action("window-custom-dialog")
            if (t.waitFor("window_size_dialog_opens", () => dialogs() && dialogs().current
                              && dialogs().current.objectName === "inputDialog" && dialogs().current.opened,
                          3000)) {
                dialogs().current.close()
                dialogs().answer("800x500")
            }
            break
        case 10:
            if (t.waitFor("window_size_dialog_applies", () => near(host.width, 800, 2), 3000,
                          () => "width " + host.width))
                player.run_action("window-control")
            break
        case 11:
            if (t.waitFor("window_control_dialog_opens", () => dialogs().current
                              && dialogs().current.objectName === "windowDialog" && dialogs().current.opened,
                          3000)) {
                dialogs().current.close()
                host.placeWindow(720, 480, area().x + 10, area().y + 10)
            }
            break
        case 12:
            if (t.waitFor("window_control_places", () => near(host.width, 720, 2)
                              && (!moves() || host.x === area().x + 10), 3000,
                          () => host.width + " at " + host.x)) {
                setSetting("on-top", "fullscreen-playing")
                player.run_action("fullscreen-other-display")
            }
            break
        // Fullscreen on another display (here the only one), and staying
        // on top while playing in fullscreen.
        case 13:
            t.waitFor("fullscreen_other_display", () => host.visibility === Window.FullScreen
                          && s.fullscreen && s.on_top, 5000,
                      () => "visibility " + host.visibility + " on_top " + s.on_top)
            break
        case 14:
            if (host.settled("fullscreen_other"))
                player.run_action("fullscreen-other-display")
            break
        case 15:
            if (t.waitFor("fullscreen_other_display_back", () => host.visibility === Window.Windowed
                              && !s.fullscreen && !s.on_top, 5000,
                          () => "visibility " + host.visibility + " on_top " + s.on_top)) {
                setSetting("on-top", "never")
                setSetting("image-format", "bmp")
                player.run_action("save-frame")
            }
            break
        // Capture: BMP through Qt, the screen frame through mpv.
        case 16:
            if (t.waitFor("capture_bmp_saved", () => saved(s) !== "", 5000, () => "osd " + s.osd)) {
                const info = JSON.parse(Tools.image_info(images + "/" + saved(s)))
                t.check("capture_bmp", info.format === "bmp" && info.width === s.video_width,
                        saved(s) + " " + JSON.stringify(info))
                lastSaved = saved(s)
                setSetting("image-format", "png")
                player.run_action("save-screen")
            }
            break
        case 17:
            if (t.waitFor("capture_screen_saved", () => saved(s) !== "" && saved(s) !== lastSaved, 5000,
                          () => "osd " + s.osd)) {
                const info = JSON.parse(Tools.image_info(images + "/" + saved(s)))
                // The picture as shown: the video area's size.
                const rect = player.video_rect
                t.check("capture_screen_frame", info.format === "png" && info.width > s.video_width
                            && near(info.width, rect.width, 2),
                        saved(s) + " " + JSON.stringify(info) + " video area " + rect.width)
                player.run_action("copy-frame")
            }
            break
        case 18:
            if (t.waitFor("capture_clipboard_copied", () => s.osd === "Copied the frame to the clipboard",
                          5000, () => "osd " + s.osd)) {
                const info = JSON.parse(Tools.clipboard_image_info())
                t.check("capture_clipboard", info.width === s.video_width && info.height > 0,
                        JSON.stringify(info))
                filesBefore = files().length
                setSetting("capture.interval", 0.2)
                setSetting("capture.count", 3)
                player.run_action("consecutive-toggle")
            }
            break
        case 19:
            t.waitFor("capture_consecutive", () => files().length === filesBefore + 3
                          && s.osd === "Consecutive capture done (3 images)", 8000,
                      () => (files().length - filesBefore) + " images, osd " + s.osd)
            break
        case 20:
            player.run_action("thumbnail-image")
            t.next()
            break
        case 21:
            if (t.waitFor("thumbnail_dialog_opens", () => dialogs().current
                              && dialogs().current.objectName === "sheetDialog" && dialogs().current.opened,
                          3000)) {
                dialogs().current.close()
                dialogs().answer(JSON.stringify({ columns: 3, rows: 2, width: 640, info: true,
                                                  times: true, exact: false }))
            }
            break
        case 22:
            if (t.waitFor("thumbnail_image_saved", () => (s.osd || "").startsWith("Saved the thumbnail image "),
                          20000, () => "osd " + s.osd)) {
                const name = s.osd.substring("Saved the thumbnail image ".length)
                const info = JSON.parse(Tools.image_info(images + "/" + name))
                // 640 wide: three or four lines of information (with the
                // audio's) over two rows of three 4:3 pictures 208 wide
                // (sheet_layout's geometry: gaps of 4, lines of 14).
                const rows = 4 + 2 * (Math.round(208 * 3 / 4) + 4)
                t.check("thumbnail_image", info.format === "png" && info.width === 640
                            && (info.height === 3 * 14 + 8 + rows || info.height === 4 * 14 + 8 + rows),
                        name + " " + JSON.stringify(info))
                player.key(Qt.Key_F5, 0)
            }
            break
        // The settings window: PotPlayer's tree, the group's pages as tabs.
        case 23: {
            const loader = host.settingsWindow()
            if (!t.waitFor("settings_open_m12", () => loader && loader.visible, 3000))
                break
            loader.showKey("capture")
            const keys = loader.tabKeys
            t.check("settings_tree_tabs", loader.currentGroup.title === "Video" && keys[0] === "video"
                        && keys.indexOf("capture") >= 0 && loader.rows.some(r => r.key === "capture"),
                    loader.currentGroup.title + ": " + keys.join(","))
            loader.close()
            break
        }
        case 24:
            t.finish()
            break
        }
    }
}
