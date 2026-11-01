// M18's self-test (`rvp --selftest=smoothing`), loaded by Main.qml's
// self-test. Ctrl+Shift+I turns motion smoothing on and off while the file
// plays. With a VapourSynth runtime that has MVTools, RVP's filter
// (@rvpsmooth) runs and the frame rate rises; without one, smoothing gives
// way to Blend. The engines and presets change through the actions the
// Motion Smoothing menu runs, and the settings window has its page. Input
// goes through Player's handlers, as in the functional check.
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
        case 0:
            t.next()
            break
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
        case 5:
            player.run_action("smoothing-blend")
            t.next()
            break
        case 6:
            t.waitFor("blend_from_the_menu", () => s.smoothing === "Blend" && chain().indexOf("@rvpsmooth") < 0,
                      5000, () => shown(s))
            break
        case 7:
            player.run_action("smoothing-mvtools")
            player.run_action("smoothing-fast")
            t.next()
            break
        case 8:
            if (mvtools)
                t.waitFor("mvtools_fast_from_the_menu", () => s.smoothing.startsWith("MVTools Fast")
                              && chain().indexOf("@rvpsmooth:vapoursynth") >= 0, 5000, () => shown(s))
            else
                t.waitFor("mvtools_gives_way_to_blend", () => s.smoothing === "Blend", 3000, () => shown(s))
            break
        case 9:
            player.key(Qt.Key_I, Qt.ControlModifier | Qt.ShiftModifier)
            t.next()
            break
        case 10:
            t.waitFor("ctrl_shift_i_turns_it_off", () => s.smoothing === "Off"
                          && chain().indexOf("@rvpsmooth") < 0, 5000, () => shown(s))
            break
        case 11:
            player.key(Qt.Key_F5, 0)
            t.next()
            break
        case 12: {
            const loader = host.settingsWindow()
            if (!t.waitFor("settings_open_smoothing", () => loader && loader.visible, 3000))
                break
            loader.showKey("smoothing")
            // By the group's page: its title is in the interface's language.
            t.check("settings_smoothing_page", loader.currentGroup.page === "video"
                        && loader.tabKeys.indexOf("smoothing") >= 0 && loader.view.smoothing === "Off",
                    loader.currentGroup.title + ": " + loader.tabKeys.join(",") + " now " + loader.view.smoothing)
            loader.close()
            break
        }
        case 13: {
            const errors = [setSetting("smoothing.preset", "medium")].map(r => r.error).filter(e => e !== "")
            t.check("smoothing_settings_restored", errors.length === 0, errors.join("; "))
            t.finish()
            break
        }
        }
    }
}
