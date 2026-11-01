// M16's self-test (`rvp --selftest=m16`), loaded by Main.qml's self-test:
// the synthetic DVD with menus (tools/test-clips/make-m16-dvd.sh, its
// folder in --dvd-menus=FOLDER) plays through RVP's navigator. The main
// menu shows with its first button lit; the arrows and the pointer move
// the selection; a click plays title 1 with the machine's first audio
// stream; DVD Control's Audio, Subtitle, Chapter and Root Menu open the
// disc's menus, whose buttons choose the audio stream, the subtitles and
// chapter 2; a click plays title 1 again, whose end is a 3 s still before
// the main menu; with the menus turned off the disc plays as mpv's dvd://.
// Then the synthetic Blu-ray (tools/test-clips/make-m16-bluray.py, its
// folder in --bd-menus=FOLDER): its HDMV menu shows, the arrows and the
// pointer change the selection (the picture), a click plays title 1 with
// the disc's first audio stream, the top menu (Title Menu) comes back and
// its buttons choose the second audio stream and the second chapter; with
// the menus turned off it plays as mpv's bd://. Each part is skipped
// without its library or disc.
import QtQuick
import Rvp
import RvpNative

Item {
    id: m16
    required property Player player
    required property var t
    required property var host
    property string folder: ""
    property string bdFolder: ""
    property string digest: ""
    property real stillSince: 0

    function setSetting(path, value) {
        return JSON.parse(player.set_setting(path, JSON.stringify(value)))
    }
    function disc(s) {
        return s.disc || {}
    }
    function shown(s) {
        return "disc " + JSON.stringify(s.disc) + " path " + s.path + " time " + s.time
    }
    // A point of the menu's frame (720x480) in the window.
    function at(s, x, y) {
        const w = disc(s).window
        return w ? { x: w.x0 + x * w.sx, y: w.y0 + y * w.sy } : null
    }
    // Buttons are 160x100 at x = 80, 280, 480 and y = 190.
    function buttonCentre(s, number) {
        return at(s, 160 + (number - 1) * 200, 240)
    }
    // The Blu-ray's buttons are 280x160 at x = 320, 820, 1320 and y = 700
    // of its 1920x1080 plane.
    function bdButton(s, number) {
        return at(s, 460 + (number - 1) * 500, 780)
    }
    function click(p) {
        player.mouse_move(p.x, p.y)
        player.mouse_press(p.x, p.y, Qt.LeftButton, 0)
        player.mouse_release(p.x, p.y, Qt.LeftButton)
    }
    function bdMenu(s) {
        const d = disc(s)
        return d.kind === "bluray" && d.menu === true && d.picture === true && d.still === true
    }
    function bdTitle(s, title) {
        const d = disc(s)
        return d.kind === "bluray" && d.title === title && d.picture === false && s.loaded && s.time > 0.3
    }
    // The Blu-ray part, or the end without it.
    function blurayOrFinish(s) {
        bdFolder = player.option("bd-menus")
        if (!s.bluray_menus || bdFolder === "") {
            t.check("m16_bd_menus", true, "SKIPPED " + (s.bluray_menus ? "no --bd-menus" : "no libbluray"))
            t.finish()
            return
        }
        player.open_disc(bdFolder, true)
        t.next()
    }
    function key(code) {
        player.key(code, 0)
    }
    function menuShows(s, domain) {
        const d = disc(s)
        return d.domain === domain && d.picture === true && d.button !== null && d.button !== undefined
    }
    function titlePlays(s, part) {
        const d = disc(s)
        return d.domain === 2 && d.title === 1 && (part === undefined || d.part === part)
            && d.picture === false && s.loaded && s.time > 0.3
    }

    function tick() {
        const t = m16.t
        const s = t.status()
        switch (t.step) {
        case 0:
            folder = player.option("dvd-menus")
            setSetting("fit-window-to-video", false)
            if (!s.disc_menus || folder === "") {
                t.check("m16_disc_menus", true, "SKIPPED " + (s.disc_menus ? "no --dvd-menus" : "no libdvdnav"))
                t.step = 15
                blurayOrFinish(s)
                break
            }
            player.open_disc(folder, false)
            t.next()
            break
        case 1:
            if (t.waitFor("m16_menu_shows", () => menuShows(s, 4) && disc(s).button === 1
                              && disc(s).still === true, 20000, () => shown(s)))
                key(Qt.Key_Right)
            break
        case 2:
            if (t.waitFor("m16_arrow_selects", () => disc(s).button === 2, 5000, () => shown(s))) {
                Tools.grab(t.outDir, "m16-menu-button-2")
                const p = buttonCentre(s, 3)
                player.mouse_move(p.x, p.y)
            }
            break
        case 3:
            if (t.waitFor("m16_pointer_selects", () => disc(s).button === 3, 5000, () => shown(s))) {
                const p = buttonCentre(s, 1)
                player.mouse_move(p.x, p.y)
                player.mouse_press(p.x, p.y, Qt.LeftButton, 0)
                player.mouse_release(p.x, p.y, Qt.LeftButton)
            }
            break
        case 4:
            // The title with the stream the disc's program chose (the
            // navigator reports it a moment after the title starts).
            if (t.waitFor("m16_click_plays_title", () => titlePlays(s) && disc(s).audio_source !== null, 20000,
                          () => shown(s))) {
                // The machine's first audio stream: AC-3 0x80.
                t.check("m16_machine_audio", disc(s).audio_source === 0x80, shown(s))
                player.run_action("dvd-audio-menu")
            }
            break
        case 5:
            if (t.waitFor("m16_audio_menu", () => menuShows(s, 8), 20000, () => shown(s))) {
                key(Qt.Key_Right)
                key(Qt.Key_Return)
            }
            break
        case 6:
            if (t.waitFor("m16_audio_choice", () => titlePlays(s) && disc(s).audio_source === 0x81, 20000,
                          () => shown(s)))
                player.run_action("dvd-subtitle-menu")
            break
        case 7:
            if (t.waitFor("m16_subtitle_menu", () => menuShows(s, 8), 20000, () => shown(s))) {
                key(Qt.Key_Right)
                key(Qt.Key_Return)
            }
            break
        case 8:
            if (t.waitFor("m16_subtitle_choice", () => titlePlays(s) && disc(s).sub_source === 0x20, 20000,
                          () => shown(s)))
                player.run_action("dvd-chapter-menu")
            break
        case 9:
            if (t.waitFor("m16_chapter_menu", () => menuShows(s, 8), 20000, () => shown(s))) {
                key(Qt.Key_Right)
                key(Qt.Key_Return)
            }
            break
        case 10:
            if (t.waitFor("m16_chapter_plays", () => titlePlays(s, 2), 20000, () => shown(s)))
                player.run_action("dvd-root-menu")
            break
        case 11:
            // The title set's root menu goes back to the main menu.
            if (t.waitFor("m16_root_menu", () => menuShows(s, 4) && disc(s).still === true, 20000,
                          () => shown(s))) {
                const p = buttonCentre(s, 1)
                player.mouse_press(p.x, p.y, Qt.LeftButton, 0)
                player.mouse_release(p.x, p.y, Qt.LeftButton)
            }
            break
        case 12:
            if (t.waitFor("m16_title_again", () => titlePlays(s, 1), 20000, () => shown(s)))
                player.seek_fraction(0.8)
            break
        case 13:
            if (t.waitFor("m16_timed_still", () => disc(s).timed_still === true && disc(s).domain === 2, 20000,
                          () => shown(s)))
                stillSince = Date.now()
            break
        case 14:
            if (t.waitFor("m16_still_ends_in_menu", () => menuShows(s, 4), 20000, () => shown(s))) {
                const waited = (Date.now() - stillSince) / 1000
                t.check("m16_still_lasts", waited > 2 && waited < 6, waited.toFixed(1) + " s")
                setSetting("devices.dvd-menus", false)
                player.open_disc(folder, false)
            }
            break
        case 15:
            if (t.waitFor("m16_menus_off", () => s.loaded && String(s.path).indexOf("dvd://") === 0
                              && s.disc === null, 20000, () => shown(s))) {
                setSetting("devices.dvd-menus", true)
                t.step = 15
                blurayOrFinish(s)
            }
            break
        case 16:
            if (t.waitFor("m16_bd_menu_shows", () => bdMenu(s), 30000, () => shown(s))) {
                digest = disc(s).picture_digest
                key(Qt.Key_Right)
            }
            break
        case 17:
            if (t.waitFor("m16_bd_arrow_selects", () => disc(s).picture_digest !== digest, 5000,
                          () => shown(s))) {
                digest = disc(s).picture_digest
                const p = bdButton(s, 3)
                player.mouse_move(p.x, p.y)
            }
            break
        case 18:
            if (t.waitFor("m16_bd_pointer_selects", () => disc(s).picture_digest !== digest, 5000,
                          () => shown(s)))
                click(bdButton(s, 1))
            break
        case 19:
            t.waitFor("m16_bd_click_plays_title", () => bdTitle(s, 1), 30000, () => shown(s))
            break
        case 20:
            // The disc's first audio stream (PID 0x1100).
            if (t.waitFor("m16_bd_disc_audio", () => disc(s).audio_source === 0x1100, 10000, () => shown(s)))
                player.run_action("dvd-title-menu")
            break
        case 21:
            if (t.waitFor("m16_bd_top_menu", () => bdMenu(s), 30000, () => shown(s)))
                click(bdButton(s, 3))
            break
        case 22:
            if (t.waitFor("m16_bd_audio_choice", () => bdTitle(s, 1) && disc(s).audio_source === 0x1101, 30000,
                          () => shown(s)))
                player.run_action("dvd-title-menu")
            break
        case 23:
            if (t.waitFor("m16_bd_top_menu_again", () => bdMenu(s), 30000, () => shown(s)))
                click(bdButton(s, 2))
            break
        case 24:
            if (t.waitFor("m16_bd_chapter_2", () => bdTitle(s, 2) && disc(s).part === 2, 30000, () => shown(s))) {
                setSetting("devices.bluray-menus", false)
                player.open_disc(bdFolder, true)
            }
            break
        case 25:
            if (t.waitFor("m16_bd_menus_off", () => s.loaded && String(s.path).indexOf("bd://") === 0
                              && s.disc === null, 30000, () => shown(s))) {
                setSetting("devices.bluray-menus", true)
                t.finish()
            }
            break
        }
    }
}
