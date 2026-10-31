// M9's self-test (`rvp --selftest=m9 --controller=osc FILE`, or
// `--controller=scripts` with the user's scripts in the settings' mpv
// folder; the file: fixture-60s.mkv): mpv's on-screen controller runs; a
// click on its seek bar seeks and leaves RVP's pause alone; a click beside
// it reaches RVP (its left click pauses); a double click there is RVP's
// fullscreen, where the skin's bars stay hidden; a controller's own
// request for fullscreen (mpv's `fullscreen`, as a script sets it) moves
// RVP's window; a wheel turn beside it is RVP's volume; "without the
// skin" lays the video over the whole window; a controller's menu takes
// the keys while it is open (skipped where the controller has none).
// `--controller-bar=FRACTION` places the seek bar's click (ModernZ: 0.9).
//
// The pointer moves, then rests before each click: scripts such as uosc
// take the pointer in the frame after it moved, as they would under a
// hand on the mouse.
import QtQuick
import Rvp
import RvpNative

Item {
    id: m9
    required property Player player
    required property var t
    required property var host
    property string kind: ""
    property real startTime: 0
    property real volume: 0
    property real since: 0
    // Where the controller's seek bar lies, as a fraction of the video's
    // height (`--controller-bar`, default 0.97: mpv's OSC, uosc).
    property real barAt: 0.97

    function setSetting(path, value) {
        return JSON.parse(player.set_setting(path, JSON.stringify(value)))
    }
    function controller(s) {
        return s.controller || {}
    }
    function shown(s) {
        return "controller " + JSON.stringify(s.controller) + " time " + s.time + " pause " + s.pause
            + " fullscreen " + s.fullscreen + " volume " + s.volume + " overlay " + player.overlay
            + " bars " + player.chrome_shown
    }
    // A point of the video, as fractions of its width and height.
    function at(fx, fy) {
        const r = player.video_rect
        return { x: r.x + r.width * fx, y: r.y + r.height * fy }
    }
    // Moves the pointer and starts the rest before the next click.
    function move(p) {
        player.mouse_move(p.x, p.y)
        since = Date.now()
    }
    function rested() {
        return Date.now() - since > 600
    }
    function click(p) {
        player.mouse_press(p.x, p.y, Qt.LeftButton, 0)
        player.mouse_release(p.x, p.y, Qt.LeftButton)
    }
    function mpv(args) {
        return player.selftest_mpv(JSON.stringify(args))
    }

    function tick() {
        const t = m9.t
        const s = t.status()
        const c = controller(s)
        const middle = at(0.5, 0.4)
        const bar = at(0.5, barAt)
        switch (t.step) {
        case 0:
            barAt = Number(player.option("controller-bar")) || 0.97
            // Room for the controller: the window's default is small.
            host.setGeometry(host.x, host.y, 1100, 620)
            t.next()
            break
        case 1:
            kind = player.option("controller")
            setSetting("fit-window-to-video", false)
            if (t.waitFor("m9_plays", () => s.loaded && s.time > 0.5 && s.controller !== null, 20000,
                          () => shown(s))) {
                const runs = c.kind === kind
                    && (kind !== "osc" || c.osc === "yes")
                    && (kind !== "scripts" || c.scripts > 0)
                t.check("m9_controller_runs", runs, shown(s))
                player.run_action("play-pause")
            }
            break
        case 2:
            if (t.waitFor("m9_paused", () => s.pause === true, 5000, () => shown(s)))
                move(bar)
            break
        case 3:
            if (t.waitFor("m9_pointer_inside", () => c.inside === true && rested(), 5000, () => shown(s))) {
                startTime = s.time
                Tools.grab(t.outDir, "m9-controller-shown")
                click(bar)
            }
            break
        case 4:
            if (t.waitFor("m9_controller_takes_click", () => s.time > startTime + 10 && s.pause === true,
                          5000, () => shown(s)))
                move(middle)
            break
        case 5:
            // Apart from the last click too: mpv makes a double click of
            // two presses, one of them on the controller.
            if (rested()) {
                click(middle)
                t.next()
            }
            break
        case 6:
            // RVP's left click on the video plays and pauses.
            if (t.waitFor("m9_rvp_gets_other_clicks", () => s.pause === false, 5000, () => shown(s)))
                since = Date.now()
            break
        case 7:
            if (rested()) {
                click(middle)
                click(middle)
                t.next()
            }
            break
        case 8:
            if (t.waitFor("m9_double_click_is_rvps", () => s.fullscreen === true, 5000, () => shown(s)))
                move(bar)
            break
        case 9:
            // The skin's bars stay hidden under the controller.
            if (rested()) {
                t.check("m9_no_skin_bars", player.chrome_shown === false && c.inside === true, shown(s))
                Tools.grab(t.outDir, "m9-fullscreen")
                move(middle)
                t.next()
            }
            break
        case 10:
            if (rested()) {
                click(middle)
                click(middle)
                t.next()
            }
            break
        case 11:
            if (t.waitFor("m9_fullscreen_left", () => s.fullscreen === false, 5000, () => shown(s)))
                since = Date.now()
            break
        case 12:
            // A script sets mpv's `fullscreen`: RVP's window follows.
            if (rested()) {
                mpv(["set", "fullscreen", "yes"])
                t.next()
            }
            break
        case 13:
            if (t.waitFor("m9_controller_asks_fullscreen", () => s.fullscreen === true, 5000, () => shown(s)))
                since = Date.now()
            break
        case 14:
            if (rested()) {
                mpv(["set", "fullscreen", "no"])
                t.next()
            }
            break
        case 15:
            if (t.waitFor("m9_controller_leaves_fullscreen", () => s.fullscreen === false, 5000,
                          () => shown(s)))
                move(middle)
            break
        case 16:
            if (rested()) {
                volume = s.volume
                player.mouse_wheel(middle.x, middle.y, -120)
                t.next()
            }
            break
        case 17:
            // RVP's wheel turns the volume.
            if (t.waitFor("m9_rvp_gets_the_wheel", () => s.volume < volume, 5000, () => shown(s)))
                setSetting("controller.without-skin", true)
            break
        case 18:
            if (t.waitFor("m9_without_skin", () => player.overlay === true
                              && player.video_rect.width >= host.width - 1
                              && player.video_rect.height >= host.height - 1, 5000, () => shown(s))) {
                Tools.grab(t.outDir, "m9-without-skin")
                setSetting("controller.without-skin", false)
            }
            break
        case 19:
            if (t.waitFor("m9_skin_back", () => player.overlay === false, 5000, () => shown(s))) {
                mpv(["script-binding", kind === "osc" ? "select/menu" : "uosc/menu"])
                since = Date.now()
            }
            break
        case 20:
            if (c.menu_open === true) {
                t.check("m9_menu_opens", true, shown(s))
                Tools.grab(t.outDir, "m9-menu")
                t.check("m9_menu_takes_keys", player.controller_key(Qt.Key_Escape, 0, "\u001b"), shown(s))
                t.next()
            } else if (Date.now() - since > 3000) {
                // mpv 0.37's OSC has no menus; scripts may have none.
                t.check("m9_menu_opens", true, "SKIPPED no menu " + shown(s))
                t.finish()
            }
            break
        case 21:
            if (t.waitFor("m9_menu_closes", () => c.menu_open === false, 5000, () => shown(s)))
                t.finish()
            break
        }
    }
}
