// M9's self-test of classic Winamp skins (`rvp --selftest=classic
// --skin=SKIN.wsz FILE`; the file: fixture-60s.mkv): RVP builds its skin
// from the classic one; the main window shows docked above the video
// window, the equalizer and playlist hidden; the main window's EQ and PL
// buttons show them; Pause and Play act; the volume slider and an
// equalizer band take the pointer; the playlist resizes in its steps; the
// video window's frame lies around the video. Each step grabs the windows.
import QtQuick
import Rvp
import RvpNative

Item {
    id: classic
    required property Player player
    required property var t
    required property var host
    property real volume: 0
    property real since: 0

    function setSetting(path, value) {
        return JSON.parse(player.set_setting(path, JSON.stringify(value)))
    }
    function extras(s) {
        return s.extras || []
    }
    function shown(s) {
        return "extras " + JSON.stringify(s.extras) + " pause " + s.pause + " time " + s.time
            + " volume " + s.volume + " eq " + JSON.stringify(s.eq_bands) + " video " + JSON.stringify(player.video_rect)
    }
    // A click in extra window `index` at its skin's pixels.
    function click(index, x, y) {
        player.extra_move(index, x, y)
        player.extra_press(index, x, y, Qt.LeftButton, 0)
        player.extra_release(index, x, y, Qt.LeftButton)
    }
    function window(s, area) {
        return extras(s).find((w) => w.area === area) || {}
    }

    function tick() {
        const t = classic.t
        const s = t.status()
        switch (t.step) {
        case 0:
            t.next()
            break
        case 1:
            setSetting("fit-window-to-video", false)
            if (t.waitFor("classic_plays", () => s.loaded && s.time > 0.5 && extras(s).length === 3, 20000,
                          () => shown(s))) {
                const main = window(s, "WinampMainSkinArea")
                t.check("classic_windows", main.shown === true && main.width === 275 && main.height === 116
                        && window(s, "WinampEqualizerSkinArea").shown === false
                        && window(s, "WinampPlaylistSkinArea").shown === false, shown(s))
                // The video window's frame: 11 pixels left, 20 at the top
                // (video.bmp's) or 12 (the playlist frame's pieces).
                const video = player.video_rect
                t.check("classic_video_frame", video.y === 20 && (video.x === 11 || video.x === 12)
                        && video.width > 200, shown(s))
                Tools.grab(t.outDir, "classic-start")
                // EQ and PL toggles of the main window.
                click(0, 230, 63)
                click(0, 253, 63)
            }
            break
        case 2:
            if (t.waitFor("classic_toggles_show_windows", () => window(s, "WinampEqualizerSkinArea").shown === true
                              && window(s, "WinampPlaylistSkinArea").shown === true, 5000, () => shown(s))) {
                Tools.grab(t.outDir, "classic-all")
                // Pause (62, 88, 23 x 18).
                click(0, 73, 97)
            }
            break
        case 3:
            if (t.waitFor("classic_pause", () => s.pause === true, 5000, () => shown(s)))
                click(0, 50, 97)
            break
        case 4:
            if (t.waitFor("classic_play", () => s.pause === false, 5000, () => shown(s))) {
                volume = s.volume
                // The volume slider (107, 57, 68 x 13) at its left end.
                player.extra_move(0, 109, 63)
                player.extra_press(0, 109, 63, Qt.LeftButton, 0)
                player.extra_release(0, 109, 63, Qt.LeftButton)
            }
            break
        case 5:
            if (t.waitFor("classic_volume", () => s.volume < volume, 5000, () => shown(s))) {
                // The first band (78, 38, 14 x 63) at its top: +12 dB.
                player.extra_move(1, 85, 40)
                player.extra_press(1, 85, 40, Qt.LeftButton, 0)
                player.extra_release(1, 85, 40, Qt.LeftButton)
            }
            break
        case 6:
            if (t.waitFor("classic_equalizer_band", () => s.eq_enabled === true && s.eq_bands[0] > 6, 5000,
                          () => shown(s))) {
                player.extra_resized(2, 300, 261)
                since = Date.now()
            }
            break
        case 7:
            if (t.waitFor("classic_playlist_resizes", () => window(s, "WinampPlaylistSkinArea").width === 300
                              && window(s, "WinampPlaylistSkinArea").height === 261, 5000, () => shown(s))) {
                Tools.grab(t.outDir, "classic-final")
                // The main window's PL button hides the playlist again.
                click(0, 253, 63)
            }
            break
        case 8:
            if (t.waitFor("classic_playlist_hides", () => window(s, "WinampPlaylistSkinArea").shown === false,
                          5000, () => shown(s)))
                t.finish()
            break
        }
    }
}
