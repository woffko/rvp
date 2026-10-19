// The GPU power budget's self-test (`rvp --selftest=gpu`, D17), loaded by
// Main.qml's self-test. Readings it gives (Player.feed_gpu_power) take the
// cards' place, and the budget waits seconds instead of minutes. A card at
// 99 % of its rated power makes RVP take its steps in order (here the EWA
// scalers, then the pixel shaders: without the Direct3D 11 video
// processor or a VapourSynth script those steps change nothing and are
// left out); at 20 % they come back, the last first; turning the budget
// off gives back at once what was taken. Then M18's smoothing (MVTools
// quality at twice the rate) is lightened, blended and stopped, and comes
// back the same way; without a VapourSynth runtime that has MVTools it is
// Blend, which the budget only stops. Settings change as the settings
// window changes them.
//
// Files: testdata/media/fixture-60s.mkv.
import QtQuick
import Rvp
import RvpNative

Item {
    id: gpu
    required property Player player
    required property var t
    required property var host
    // The runtime has MVTools: smoothing runs it (else Blend).
    property bool mvtools: false

    function setSetting(path, value) {
        return JSON.parse(player.set_setting(path, JSON.stringify(value)))
    }
    // What mpv gets and what the budget took, read once a tick (each
    // diagnostics read waits for mpv's core).
    function look() {
        const d = JSON.parse(player.diagnostics())
        const budget = JSON.parse(player.gpu_power_json())
        const scalers = d.scale + "/" + d.dscale + "/" + d.cscale
        return {
            shaders: String(d.glsl_shaders),
            chosen: scalers.startsWith("ewa_lanczossharp/ewa_lanczos/"),
            light: scalers.startsWith("spline36/spline36/"),
            taken: budget.taken.join(","),
            on: budget.on,
            text: scalers + " " + String(d.glsl_shaders) + " [" + budget.taken.join(",") + "] "
                  + (t.status().osd || "") + " (" + feeds + " readings)"
        }
    }
    // The budget's steps taken, by name.
    function taken() { return JSON.parse(player.gpu_power_json()).taken.join(",") }
    property int feeds: 0
    // A reading of a 350 W card at `share`.
    function feed(share) {
        feeds += 1
        player.feed_gpu_power(350 * share, 350)
    }
    function ours(l) { return l.shaders.indexOf("selftest-gpu.glsl") >= 0 }

    function tick() {
        const t = gpu.t
        const s = t.status()
        switch (t.step) {
        case 0:
            t.next()
            break
        case 1:
            // Past the start, whose first frames keep a software renderer busy.
            t.waitFor("gpu_file_plays", () => s.loaded && s.time > 2, 20000)
            break
        case 2: {
            const path = Tools.write_file(t.outDir + "/selftest-gpu.glsl",
                "//!HOOK MAIN\n//!BIND HOOKED\n//!DESC RVP self-test (no change)\n"
                + "vec4 hook() { return HOOKED_texOff(0); }\n")
            const errors = [
                setSetting("render.power-budget", 90),
                setSetting("scaler", "ewa_lanczossharp"),
                setSetting("render.dscale", "ewa_lanczos"),
                setSetting("shaders.files", [path]),
                setSetting("shaders.enabled", true)
            ].map(r => r.error).filter(e => e !== "")
            t.check("gpu_settings", path !== "" && errors.length === 0, errors.join("; "))
            t.next()
            break
        }
        case 3: {
            const l = look()
            t.waitFor("gpu_as_chosen", () => l.chosen && ours(l), 3000, () => l.text)
            break
        }
        case 4: {
            // At 99 % the lighter scalers come first...
            feed(0.99)
            const l = look()
            t.waitFor("gpu_over_scalers", () => l.light && ours(l) && l.taken === "scalers", 10000,
                      () => l.text)
            break
        }
        case 5: {
            // ...then the pixel shaders.
            feed(0.99)
            const l = look()
            t.waitFor("gpu_over_shaders", () => l.shaders === "" && l.taken === "scalers,shaders", 10000,
                      () => l.text)
            break
        }
        case 6:
            // Nothing else of RVP's runs: the steps stay as they are.
            feed(0.99)
            if (Date.now() - t.stepStarted > 2000) {
                const l = look()
                t.check("gpu_nothing_more", l.taken === "scalers,shaders", l.text)
                t.next()
            }
            break
        case 7: {
            // Under the budget less 10 points they come back, the last first.
            feed(0.2)
            const l = look()
            t.waitFor("gpu_under_shaders_back", () => ours(l) && l.light && l.taken === "scalers", 10000,
                      () => l.text)
            break
        }
        case 8: {
            feed(0.2)
            const l = look()
            t.waitFor("gpu_under_scalers_back", () => l.chosen && l.taken === "", 10000, () => l.text)
            break
        }
        case 9: {
            feed(0.99)
            const l = look()
            t.waitFor("gpu_over_again", () => l.light && l.taken === "scalers", 10000, () => l.text)
            break
        }
        case 10: {
            const result = setSetting("render.power-budget", 100)
            t.check("gpu_budget_off_set", result.error === "", result.error)
            t.next()
            break
        }
        case 11: {
            // The window's tick (once a second) gives every step back.
            const l = look()
            t.waitFor("gpu_off_gives_back", () => l.chosen && l.taken === "" && l.on === false, 3000,
                      () => l.text)
            break
        }
        case 12: {
            const errors = [
                setSetting("render.power-budget", 90),
                setSetting("scaler", ""),
                setSetting("render.dscale", ""),
                setSetting("shaders.enabled", false),
                setSetting("shaders.files", []),
                // M18's smoothing next: MVTools quality at twice the rate.
                setSetting("smoothing.engine", "mvtools"),
                setSetting("smoothing.preset", "quality"),
                setSetting("smoothing.target", "2x"),
                setSetting("smoothing.enabled", true)
            ].map(r => r.error).filter(e => e !== "")
            t.check("gpu_settings_restored", errors.length === 0, errors.join("; "))
            t.next()
            break
        }
        case 13:
            if (t.waitFor("gpu_smoothing_runs", () => t.status().smoothing !== "Off", 5000, () => t.status().smoothing))
                mvtools = t.status().smoothing.startsWith("MVTools Quality")
            break
        case 14: {
            // Above the budget: a lighter preset, Blend, off (Blend alone:
            // only off).
            feed(0.99)
            const now = t.status().smoothing
            const want = mvtools ? "smoothing-lighter,smoothing-blend,smoothing-off" : "smoothing-off"
            t.waitFor("gpu_smoothing_steps", () => taken() === want && now === "Off (the GPU power budget)",
                      15000, () => now + " [" + taken() + "]")
            break
        }
        case 15: {
            // Below it they come back, the last first, to where it began.
            feed(0.2)
            const now = t.status().smoothing
            t.waitFor("gpu_smoothing_back", () => taken() === ""
                          && (mvtools ? now.startsWith("MVTools Quality") : now === "Blend"),
                      20000, () => now + " [" + taken() + "]")
            break
        }
        case 16: {
            const errors = [
                setSetting("smoothing.enabled", false),
                setSetting("smoothing.preset", "medium"),
                setSetting("smoothing.target", "display")
            ].map(r => r.error).filter(e => e !== "")
            t.check("gpu_smoothing_settings_restored", errors.length === 0, errors.join("; "))
            t.finish()
            break
        }
        }
    }
}
