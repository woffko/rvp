// M13's self-test (`rvp --selftest=m13`), loaded by Main.qml's self-test:
// the subtitle style keys, the subtitle browser's edits shown at once,
// Input Subtitles, saving the subtitles elsewhere (into --out, never the
// fixture's file), the Adjust Subtitle Sync dialog, syncing to the next
// subtitle, Combine Subtitle and the Subtitles group of the settings
// window; with --speech-check also subtitles made from the audio (the CPU
// engine, the tiny model in RVP_WHISPER_MODELS). Input goes through
// Player's handlers, as in the functional check.
//
// Files: testdata/media/fixture-chapters.mkv with its .srt (lines at 1, 6,
// 11 and 16 s).
import QtQuick
import Rvp
import RvpNative

Item {
    id: m13
    required property Player player
    required property var t
    required property var host

    function diagnostics() { return JSON.parse(player.diagnostics()) }
    function subText() { return String(diagnostics().sub_text) }
    function shown(s) {
        const d = diagnostics()
        return "sid " + d.sid + " text " + JSON.stringify(d.sub_text) + " delay " + d.sub_delay
               + " time " + s.time + " osd " + (s.osd || "")
    }
    function editor() { return host.subtitleEditor() }
    function setSetting(path, value) {
        return JSON.parse(player.set_setting(path, JSON.stringify(value)))
    }
    function outFile(name) { return t.outDir + "/" + name }

    function tick() {
        const t = m13.t
        const s = t.status()
        switch (t.step) {
        case 0:
            t.next()
            break
        case 1:
            t.waitFor("m13_file_plays", () => s.loaded && String(s.path).endsWith("fixture-chapters.mkv")
                          && diagnostics().sid !== "no", 20000, () => shown(s))
            break
        case 2:
            if (s.pause === false)
                player.run_action("play-pause")
            player.jump_to("0:00:01.5", false)
            t.next()
            break
        case 3:
            t.waitFor("m13_subtitle_shows", () => subText() === "First line", 5000, () => shown(s))
            break
        case 4:
            player.key(Qt.Key_B, Qt.AltModifier)
            t.check("m13_alt_b_bold", diagnostics().sub_bold === "yes"
                        && t.status().osd === "Bold subtitles: on", shown(t.status()))
            player.key(Qt.Key_B, Qt.AltModifier)
            t.check("m13_alt_b_again", diagnostics().sub_bold === "no", shown(s))
            player.key(Qt.Key_E, Qt.AltModifier)
            t.next()
            break
        case 5:
            if (t.waitFor("m13_browser_opens", () => editor() && editor().browser.visible
                              && editor().data.cues.length === 4, 5000,
                          () => editor() ? JSON.stringify(editor().data).slice(0, 200) : "not loaded")) {
                editor().choose(0)
                editor().textArea.text = "Edited line"
                editor().applyEditor()
            }
            break
        case 6:
            t.waitFor("m13_edit_shows_at_once", () => subText() === "Edited line"
                          && editor().data.dirty === true, 5000, () => shown(s))
            break
        case 7:
            player.key(Qt.Key_P, Qt.AltModifier)
            t.next()
            break
        case 8:
            if (t.waitFor("m13_input_opens", () => editor().input.visible, 3000)) {
                player.subtitle_edit("input-begin", -1, "")
                player.subtitle_edit("input", -1, "Typed line")
            }
            break
        case 9:
            t.waitFor("m13_typed_line_shows", () => subText().indexOf("Typed line") >= 0, 5000,
                      () => shown(s))
            break
        case 10:
            // Saved elsewhere: the fixture's file stays as it is.
            player.video_dialog_answer("save-subtitle", outFile("m13-saved.srt"))
            t.next()
            break
        case 11:
            t.waitFor("m13_saved_elsewhere", () => s.osd === "Saved m13-saved.srt", 3000, () => shown(s))
            break
        case 12:
            player.run_action("subtitle-sync-dialog")
            t.next()
            break
        case 13:
            if (t.waitFor("m13_sync_dialog_opens", () => host.subtitleSync()
                              && host.subtitleSync().dialog.visible, 3000))
                player.set_subtitle_delay(1.25)
            break
        case 14:
            if (t.waitFor("m13_sync_dialog_sets_the_sync", () => diagnostics().sub_delay === 1.25, 3000,
                          () => shown(s))) {
                host.subtitleSync().dialog.close()
                player.run_action("subtitle-delay-reset")
            }
            break
        case 15:
            if (t.waitFor("m13_sync_reset", () => diagnostics().sub_delay === 0, 3000, () => shown(s)))
                player.jump_to("0:00:03", false)
            break
        case 16:
            if (s.time !== null && Math.abs(s.time - 3) < 0.1) {
                player.run_action("subtitle-sync-next")
                t.next()
            } else if (Date.now() - t.stepStarted > 5000) {
                t.check("m13_at_3_s", false, shown(s))
                t.next()
            }
            break
        case 17:
            // The next subtitle (the edited file's second, at 6 s) shows now.
            if (t.waitFor("m13_sync_to_the_next", () => diagnostics().sub_delay < -1
                              && (s.osd || "").startsWith("Subtitle sync: "), 3000, () => shown(s)))
                player.run_action("subtitle-delay-reset")
            break
        case 18: {
            const srt = String(s.path).replace(/\.mkv$/, ".srt")
            player.combine_subtitle(srt)
            t.next()
            break
        }
        case 19:
            t.waitFor("m13_combine_shows_a_second", () => diagnostics().secondary_sid !== "no"
                          && (s.osd || "").startsWith("Second subtitles: "), 3000, () => shown(s))
            break
        case 20:
            editor().browser.close()
            editor().input.close()
            // With --speech-check: subtitles from the audio with the CPU
            // engine and the tiny model (RVP_WHISPER_MODELS), into the
            // state folder.
            if (player.has_option("speech-check")) {
                setSetting("speech.beside-video", false)
                setSetting("speech.file.engine", "cpu")
                setSetting("speech.file.model", "tiny")
                player.run_action("speech-start")
            }
            t.next()
            break
        case 21:
            if (!player.has_option("speech-check")) {
                t.next()
                break
            }
            t.waitFor("m13_speech_makes_subtitles", () => /^Subtitles from audio: (\d+ lines in |no speech found)/.test(t.status().osd || ""),
                      180000, () => shown(t.status()))
            break
        case 22:
            player.key(Qt.Key_F5, 0)
            t.next()
            break
        case 23: {
            const loader = host.settingsWindow()
            if (!t.waitFor("m13_settings_open", () => loader && loader.visible, 3000))
                break
            loader.showKey("subtitle-browser")
            t.check("m13_settings_subtitle_pages", loader.currentGroup.title === "Subtitles"
                        && ["subtitle-style", "subtitle-location", "subtitle-sync", "subtitle-browser", "speech"]
                            .every(key => loader.tabKeys.indexOf(key) >= 0),
                    loader.currentGroup.title + ": " + loader.tabKeys.join(","))
            loader.close()
            t.finish()
            break
        }
        }
    }
}
