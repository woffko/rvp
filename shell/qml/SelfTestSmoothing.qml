// M18's self-test (`rvp --selftest=smoothing`), loaded by Main.qml's
// self-test. Ctrl+Shift+I turns motion smoothing on and off while the file
// plays. With a VapourSynth runtime that has MVTools, RVP's filter
// (@rvpsmooth) runs and the frame rate rises; without one, smoothing gives
// way to Blend. The engines and presets change through the actions the
// Motion Smoothing menu runs, and the settings window has its page. The
// log runs at Debug meanwhile and records the plans, mpv's messages and
// the seconds of playback. Input goes through Player's handlers, as in the
// functional check.
//
// Files: testdata/media/fixture-60s.mkv.
import QtQuick
import Rvp
import RvpNative

Item {
    id: smoothing
    required property Player player
    required property var t
    required property var host
    // The runtime has MVTools: smoothing turned on runs it.
    property bool mvtools: false
    // The thumbnail core's pictures before the seek.
    property int thumbnailsBefore: 0

    function diagnostics() { return JSON.parse(player.diagnostics()) }
    function chain() { return String(diagnostics().vf) }
    function setSetting(path, value) {
        return JSON.parse(player.set_setting(path, JSON.stringify(value)))
    }
    function shown(s) {
        const d = diagnostics()
        return "[" + s.smoothing + "] vf " + d.vf + ", " + d.container_fps + " -> " + d.estimated_vf_fps
               + " fps, osd " + (s.osd || "")
    }

    function tick() {
        const t = smoothing.t
        const s = t.status()
        switch (t.step) {
        case 0: {
            const set = setSetting("log.level", "debug")
            t.check("log_level_set", set.error === "", set.error)
            t.next()
            break
        }
        case 1:
            t.waitFor("smoothing_file_plays", () => s.loaded && s.time > 1, 20000)
            break
        case 2:
            t.check("smoothing_off_by_default", s.smoothing === "Off", s.smoothing)
            player.key(Qt.Key_I, Qt.ControlModifier | Qt.ShiftModifier)
            t.next()
            break
        case 3:
            if (t.waitFor("ctrl_shift_i_turns_it_on", () => s.smoothing !== "Off"
                              && (s.osd || "").startsWith("Motion smoothing: "), 5000, () => shown(s)))
                mvtools = s.smoothing.startsWith("MVTools")
            break
        case 4:
            // MVTools runs as RVP's filter and the rate rises; without it,
            // Blend.
            if (mvtools)
                t.waitFor("mvtools_raises_the_rate", () => chain().indexOf("@rvpsmooth:vapoursynth") >= 0
                              && diagnostics().estimated_vf_fps > 1.5 * diagnostics().container_fps,
                          15000, () => shown(s))
            else
                t.waitFor("blend_without_mvtools", () => s.smoothing === "Blend"
                              && chain().indexOf("@rvpsmooth") < 0, 3000, () => shown(s))
            break
        case 5: {
            // SVP's way: mpv loads the script again after a seek, whose
            // first second shows the original (`original-first`); the
            // filter stays in, and the rate rises again. Every frame from
            // the keyframe on goes through the filter, which counts its
            // frames (with mpv's hr-seek-framedrop the picture ran ahead
            // of the sound after a seek), and the thumbnail core renders
            // the target for the window meanwhile.
            if (mvtools) {
                t.check("seeks_keep_every_frame_for_the_filter", diagnostics().hr_seek_framedrop === "no",
                        diagnostics().hr_seek_framedrop)
                const stats = s.thumbnail_stats
                smoothing.thumbnailsBefore = stats.rendered + stats.cancelled
                player.key(Qt.Key_Right, 0)
            }
            t.next()
            break
        }
        case 6:
            if (mvtools)
                t.waitFor("mvtools_smooths_after_a_seek", () => s.smoothing.startsWith("MVTools")
                              && chain().indexOf("@rvpsmooth:vapoursynth") >= 0
                              && diagnostics().estimated_vf_fps > 1.5 * diagnostics().container_fps,
                          10000, () => shown(s))
            else
                t.next()
            break
        case 7: {
            // The target's picture was asked for (whether it came before
            // mpv's own or not), and none shows once playback restarted.
            if (mvtools) {
                const stats = s.thumbnail_stats
                Tools.log("metric seek_pictures_shown=" + s.seek_picture_serial)
                t.check("seek_picture_asked_and_gone", stats.rendered + stats.cancelled > smoothing.thumbnailsBefore
                            && s.seek_picture_width === 0,
                        JSON.stringify(stats) + " picture " + s.seek_picture_width + " serial " + s.seek_picture_serial)
            }
            t.next()
            break
        }
        case 8: {
            // The setting and the plan it led to.
            const recent = player.log_recent()
            t.check("log_records_the_plan", recent.indexOf("setting log.level") >= 0
                        && recent.indexOf("smoothing: ") >= 0,
                    recent.split("\n").slice(-6).join(" | "))
            player.run_action("smoothing-blend")
            t.next()
            break
        }
        case 9:
            t.waitFor("blend_from_the_menu", () => s.smoothing === "Blend" && chain().indexOf("@rvpsmooth") < 0,
                      5000, () => shown(s))
            break
        case 10:
            player.run_action("smoothing-mvtools")
            player.run_action("smoothing-fast")
            t.next()
            break
        case 11:
            if (mvtools)
                t.waitFor("mvtools_fast_from_the_menu", () => s.smoothing.startsWith("MVTools Fast")
                              && chain().indexOf("@rvpsmooth:vapoursynth") >= 0, 5000, () => shown(s))
            else
                t.waitFor("mvtools_gives_way_to_blend", () => s.smoothing === "Blend", 3000, () => shown(s))
            break
        case 12:
            player.key(Qt.Key_I, Qt.ControlModifier | Qt.ShiftModifier)
            t.next()
            break
        case 13:
            t.waitFor("ctrl_shift_i_turns_it_off", () => s.smoothing === "Off"
                          && chain().indexOf("@rvpsmooth") < 0, 5000, () => shown(s))
            break
        case 14:
            player.key(Qt.Key_F5, 0)
            t.next()
            break
        case 15: {
            const loader = host.settingsWindow()
            if (!t.waitFor("settings_open_smoothing", () => loader && loader.visible, 3000))
                break
            loader.showKey("smoothing")
            // By the group's page: its title is in the interface's language.
            t.check("settings_smoothing_page", loader.currentGroup.page === "video"
                        && loader.tabKeys.indexOf("smoothing") >= 0 && loader.view.smoothing === "Off",
                    loader.currentGroup.title + ": " + loader.tabKeys.join(",") + " now " + loader.view.smoothing)
            // The calibration window: a self-test measures only when asked.
            const calibration = loader.openCalibration()
            t.check("calibration_window_opens", calibration && calibration.visible
                        && ["waiting", "unavailable"].indexOf(calibration.status.state) >= 0,
                    calibration ? JSON.stringify(calibration.status) : "no window")
            if (calibration)
                calibration.close()
            // The Log page is a group of its own.
            loader.showKey("log")
            t.check("settings_log_page", loader.currentGroup.page === "log" && loader.currentKey === "log"
                        && String(loader.view.log_file).endsWith("rvp.log"),
                    loader.currentKey + " " + loader.view.log_file)
            loader.close()
            break
        }
        case 16: {
            // mpv's messages and each second of playback.
            const recent = player.log_recent()
            t.check("log_records_playback", s.log_lines > 0 && s.log_mpv_lines > 0
                        && recent.indexOf("playback ") >= 0,
                    s.log_lines + " lines, " + s.log_mpv_lines + " of mpv's; "
                        + recent.split("\n").slice(-6).join(" | "))
            // The Log page's folder: the file moves there at once.
            const folder = String(s.state_file).replace(/[\\/][^\\/]*$/, "") + "/moved-log"
            const moved = setSetting("log.folder", folder)
            t.check("log_folder_set", moved.error === "" && moved.notes.length === 0,
                    moved.error + " " + moved.notes.join("; "))
            t.next()
            break
        }
        case 17:
            t.waitFor("log_moves_to_its_folder", () => /moved-log[\\/]rvp\.log$/.test(String(s.log_open))
                          && player.log_recent().indexOf("level debug") >= 0, 3000, () => String(s.log_open))
            break
        case 18: {
            const errors = [setSetting("smoothing.preset", "medium"), setSetting("log.folder", null),
                            setSetting("log.level", "off")]
                .map(r => r.error).filter(e => e !== "")
            t.check("smoothing_settings_restored", errors.length === 0 && t.status().log_open === null,
                    errors.join("; ") + " " + t.status().log_open)
            t.finish()
            break
        }
        }
    }
}
