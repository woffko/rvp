// M9's self-test of VLC's skins (`rvp --selftest=vlc --skin=THEME.vlt
// FILE`; the file: fixture-60s.mkv): RVP builds its skin from the theme;
// the theme's conditions hold for a playing file and its texts are
// filled; its pause button (shown while playing, VLC's
// `visible="vlc.isPlaying"`) pauses, after which the play button shows
// and plays again; its volume slider takes the pointer; a theme with
// transparent parts gives the window its shape. Buttons are clicked where
// the hit test finds them (`Player.control_point`: VLC's take only their
// opaque pixels), in whichever window of the theme shows them; a theme
// without a pause button skips its steps. Each step grabs the windows.
import QtQuick
import Rvp
import RvpNative

Item {
    id: vlc
    required property Player player
    required property var t
    required property var host
    property real volume: 0

    function setSetting(path, value) {
        return JSON.parse(player.set_setting(path, JSON.stringify(value)))
    }
    function shown(s) {
        return "vlc " + JSON.stringify(s.vlc) + " mask " + s.window_mask + " pause " + s.pause
            + " volume " + s.volume + " video " + JSON.stringify(player.video_rect)
            + " pause button " + player.control_point("RVP:play-pause")
            + " play button " + player.control_point("RVP:play")
    }
    function point(command) {
        return JSON.parse(player.control_point(command))
    }
    function click(at) {
        if (at.window < 0) {
            player.mouse_move(at.x, at.y)
            player.mouse_press(at.x, at.y, Qt.LeftButton, 0)
            player.mouse_release(at.x, at.y, Qt.LeftButton)
        } else {
            player.extra_move(at.window, at.x, at.y)
            player.extra_press(at.window, at.x, at.y, Qt.LeftButton, 0)
            player.extra_release(at.window, at.x, at.y, Qt.LeftButton)
        }
    }

    function tick() {
        const t = vlc.t
        const s = t.status()
        const v = s.vlc || {}
        switch (t.step) {
        case 0:
            t.next()
            break
        case 1:
            setSetting("fit-window-to-video", false)
            if (t.waitFor("vlc_plays", () => s.loaded && s.time > 0.5 && v.conditions > 0, 20000,
                          () => shown(s))) {
                t.check("vlc_conditions_hold", v.on > 0, shown(s))
                t.check("vlc_texts", (v.texts || []).some((text) => text.length > 0), shown(s))
                t.check("vlc_video", player.video_rect.width > 0 && player.video_rect.height > 0, shown(s))
                Tools.grab(t.outDir, "vlc-start")
            }
            break
        case 2: {
            const pause = point("RVP:play-pause")
            if (pause.window === undefined && !pause.bound) {
                // VLC's `vlc.play()` only plays: a theme without `vlc.pause()`
                // cannot pause.
                t.check("vlc_pause_button", true, "SKIPPED the theme has no pause button")
                t.check("vlc_pauses", true, "SKIPPED")
                player.run_action("play-pause")
                t.step = 4
                t.stepStarted = Date.now()
            } else if (t.waitFor("vlc_pause_button", () => pause.window !== undefined, 5000, () => shown(s))) {
                click(pause)
            }
            break
        }
        case 3:
            t.waitFor("vlc_pauses", () => s.pause === true, 5000, () => shown(s))
            break
        case 4:
            // The play button shows now (`vlc.isPaused` or `not vlc.isPlaying`).
            if (t.waitFor("vlc_play_button", () => s.pause === true && point("RVP:play").window !== undefined,
                          5000, () => shown(s))) {
                Tools.grab(t.outDir, "vlc-paused")
                click(point("RVP:play"))
            }
            break
        case 5:
            if (t.waitFor("vlc_plays_again", () => s.pause === false, 5000, () => shown(s))) {
                volume = s.volume
                // Near the slider's low end: its first sixteenth.
                const slider = player.control_rect("CMD_VOLUME_CTRL")
                if (slider.width > 0)
                    click({ window: -1, x: slider.x + slider.width * 0.05, y: slider.y + slider.height / 2 })
                else
                    t.check("vlc_volume", true, "SKIPPED no volume slider in the main window")
            }
            break
        case 6:
            if (player.control_rect("CMD_VOLUME_CTRL").width === 0) {
                t.next()
                break
            }
            t.waitFor("vlc_volume", () => s.volume < volume, 5000, () => shown(s))
            break
        case 7:
            t.check("vlc_shape", true, "window mask rectangles: " + s.window_mask)
            Tools.grab(t.outDir, "vlc-final")
            t.finish()
            break
        }
    }
}
