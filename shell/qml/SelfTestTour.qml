// The screenshot tour (`rvp --selftest=tour`), loaded by Main.qml's
// self-test: shows RVP's main views one after another for the README's
// pictures (`tools/screenshots/tour.sh`, which runs it under Xvfb). The
// main window takes the screen's top left corner (as wide as the screen,
// as high as the video's shape lets it be) and the other windows open over
// it, so the screen's capture cut to the main window is the picture. Each
// view prints `tour_shot=NAME WIDTHxHEIGHT` (the main window's size) once
// it has settled and then stays as it is for
// `hold` milliseconds while the script captures the screen. Input goes
// through Player's handlers, as in the functional check.
//
// Files: two videos; the second has subtitles (an `.srt` of the same
// name beside it). `--tour-at=FIRST,SECOND` are the seconds each file is
// shown at (default: 40 % of each).
import QtQuick
import Rvp
import RvpNative

Item {
    id: tour
    required property Player player
    required property var t
    required property var host
    required property var settings
    required property var panel
    required property var menus
    // How long a view stays still after its line.
    readonly property int hold: 2000
    // Nothing happens before this time (a view settling, the OSD fading).
    property real waitUntil: 0
    property string firstPath: ""

    function wait(ms) {
        waitUntil = Date.now() + ms
    }
    // The next step, `ms` from now.
    function after(ms) {
        wait(ms)
        t.next()
    }
    function shot(name) {
        Tools.log("tour_shot=" + name + " " + host.width + "x" + host.height)
        after(hold)
    }
    // "h:mm:ss.s" for Player.jump_to.
    function clock(seconds) {
        const h = Math.floor(seconds / 3600)
        const m = Math.floor(seconds / 60) % 60
        const s = seconds - 60 * Math.floor(seconds / 60)
        return h + ":" + String(m).padStart(2, "0") + ":" + s.toFixed(1).padStart(4, "0")
    }
    // The second to show the `index`th file at.
    function showAt(index, duration) {
        const given = player.option("tour-at").split(",")
        const value = Number(given[index])
        return given.length > index && given[index] !== "" && value > 0 ? value : 0.4 * duration
    }
    // Centers a window of RVP's over the main window.
    function center(win) {
        win.x = host.x + Math.round((host.width - win.width) / 2)
        win.y = host.y + Math.round((host.height - win.height) / 2)
    }
    function setSetting(path, value) {
        return JSON.parse(player.set_setting(path, JSON.stringify(value)))
    }
    function page(key) {
        const win = settings.item
        win.showPage(win.pages.findIndex(p => p.key === key))
    }

    function tick() {
        if (Date.now() < waitUntil)
            return
        const s = t.status()
        switch (t.step) {
        case 0:
            // Not the video's own size.
            setSetting("fit-window-to-video", false)
            host.setGeometry(host.screen.virtualX, host.screen.virtualY, host.screen.width, host.screen.height)
            t.next()
            break
        case 1:
            if (t.waitFor("tour_first_file", () => s.loaded && s.time > 0.5, 20000)) {
                firstPath = String(s.path)
                player.run_action("play-pause")
            }
            break
        case 2:
            if (t.waitFor("tour_first_paused", () => s.pause === true, 5000)) {
                player.jump_to(clock(showAt(0, s.duration)), false)
                wait(4000)
            }
            break
        case 3:
            shot("player")
            break
        case 4:
            // Where the whole menu fits below.
            menus.active = true
            menus.item.build("context").popup(Math.round(0.55 * host.width), 60)
            after(1500)
            break
        case 5:
            shot("menu")
            break
        case 6:
            menus.item.current.close()
            player.run_action("preferences")
            after(1000)
            break
        case 7:
            if (t.waitFor("tour_settings_open", () => settings.item !== null && settings.item.visible, 5000)) {
                page("smoothing")
                center(settings.item)
                wait(1500)
            }
            break
        case 8:
            shot("settings")
            break
        case 9:
            page("input")
            after(1500)
            break
        case 10:
            shot("settings-keys")
            break
        case 11:
            settings.item.close()
            player.run_action("control-panel")
            after(1000)
            break
        case 12:
            if (t.waitFor("tour_panel_open", () => panel.item !== null && panel.item.visible, 5000)) {
                // Over the video, beside the playlist.
                panel.item.x = host.x + Math.round(0.75 * host.width) - panel.item.width
                panel.item.y = host.y + 72
                wait(1500)
            }
            break
        case 13:
            shot("control-panel")
            break
        case 14:
            player.run_action("control-panel")
            player.run_action("next")
            t.next()
            break
        case 15:
            if (t.waitFor("tour_second_file", () => s.loaded && String(s.path) !== firstPath && s.time > 0.5,
                          20000))
                player.run_action("play-pause")
            break
        case 16:
            if (t.waitFor("tour_second_paused", () => s.pause === true, 5000)) {
                player.jump_to(clock(showAt(1, s.duration)), false)
                wait(4000)
            }
            break
        case 17: {
            const r = player.control_rect("CMD_TIMEBAR_CTRL")
            player.mouse_move(r.x + 0.125 * r.width, r.y + r.height / 2)
            t.next()
            break
        }
        case 18:
            if (t.waitFor("tour_seek_preview", () => s.preview_width > 0, 8000))
                wait(1000)
            break
        case 19:
            shot("seek-preview")
            break
        case 20:
            player.mouse_leave()
            t.finish()
            break
        }
    }
}
