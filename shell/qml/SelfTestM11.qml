// M11's self-test (`rvp --selftest=m11`), loaded by Main.qml's self-test:
// the thumbnail core (the seek bar's tooltip and preview, the scene
// browsers, the playlist's thumbnail view, durations), keyframe steps, the
// jump dialog, the current subtitle, A-B subtitle and marked sections, the
// skip feature, bookmarks, the queue, sorting, search, playlist files, the
// power actions through the dry-run hook (never a real one), the Recycle
// Bin, a DVD folder, external audio, the clipboard, favorites and recent
// items, yt-dlp's fallback and the capture devices. Input goes through
// Player's handlers, as in the functional check.
//
// Files: testdata/media/fixture-chapters.mkv (with its .srt), then
// fixture-60s.mkv and fixture-3s.mkv; `--dvd-folder=FOLDER` names the DVD fixture
// (testdata/media/dvd). The command line is checked with `/seek=0:00:03`
// and `/title="M11 title"`.
import QtQuick
import Rvp
import RvpNative

Item {
    id: m11
    required property Player player
    required property var t
    required property var host
    property int scenesArrived: 0
    property var scenes: []
    property string recycleFile: ""
    property string dryRunBefore: ""

    Connections {
        target: m11.player
        function onSceneReady(index) { m11.scenesArrived += 1 }
    }

    function near(value, wanted, tolerance) {
        return value !== null && Math.abs(value - wanted) <= tolerance
    }
    function endsWith(text, end) {
        return text !== null && String(text).endsWith(end)
    }
    function setSetting(path, value) {
        return JSON.parse(player.set_setting(path, JSON.stringify(value)))
    }
    function seekBar() { return player.control_rect("CMD_TIMEBAR_CTRL") }

    function tick() {
        const t = m11.t
        const s = t.status()
        switch (t.step) {
        case 0:
            // The fixtures are 96x72: a window fitted to them leaves the
            // seek bar and the playlist a few pixels. No fits after the
            // first file's (step 2 makes the window larger).
            setSetting("fit-window-to-video", false)
            t.next()
            break
        case 1:
            if (t.waitFor("chapters_file_plays", () => s.loaded && endsWith(s.path, "fixture-chapters.mkv")
                              && s.duration > 19 && s.marks >= 3, 20000,
                          () => "path " + s.path + " marks " + s.marks)) {
                m11.chaptersFile = s.path
                // The command line: /seek and /title.
                t.check("command_line_seek", s.time !== null && s.time >= 2.5, "time " + s.time)
                t.check("command_line_title", s.media_title === "M11 title", "title " + s.media_title)
                if (s.pause === false)
                    player.run_action("play-pause")
            }
            break
        case 2: {
            // The first file was fitted when it loaded. The window system
            // applies a new size later: wait for the skin's layout at that
            // size (a seek bar of some 800 pixels).
            if (host.width < 900) {
                host.setGeometry(host.x, host.y, 1000, 700)
            }
            const r = seekBar()
            if (r.width < 400 && Date.now() - t.stepStarted < 5000)
                break
            player.mouse_move(r.x + r.width / 2, r.y + r.height / 2)
            t.next()
            break
        }
        case 3:
            t.waitFor("seek_bar_time_tooltip", () => s.seek_tip_shown && s.seek_tip.indexOf("00:00:1") === 0,
                      3000, () => "tip " + s.seek_tip + " bar " + JSON.stringify(seekBar())
                          + " window " + host.width + "x" + host.height)
            break
        case 4:
            if (t.waitFor("seek_bar_preview", () => s.preview_width > 0, 8000,
                          () => "preview " + s.preview_width + "x" + s.preview_height))
                player.mouse_leave()
            break
        case 5:
            if (t.waitFor("seek_tip_hides", () => !s.seek_tip_shown, 2000))
                player.jump_to("0:00:04.5", false)
            break
        // Keyframes every 2 s.
        case 6:
            if (t.waitFor("jump_dialog_time", () => near(s.time, 4.5, 0.15), 5000, () => "time " + s.time))
                player.key(Qt.Key_Right, Qt.ControlModifier | Qt.ShiftModifier)
            break
        case 7:
            if (t.waitFor("next_keyframe", () => near(s.time, 6.0, 0.05), 5000, () => "time " + s.time))
                player.key(Qt.Key_Left, Qt.ControlModifier | Qt.ShiftModifier)
            break
        case 8:
            if (t.waitFor("previous_keyframe", () => near(s.time, 4.0, 0.05), 5000, () => "time " + s.time))
                player.run_action("jump-dialog")
            break
        case 9: {
            const dialogs = host.playbackDialogs()
            if (t.waitFor("jump_dialog_opens", () => dialogs !== null && dialogs.current !== null
                              && dialogs.current.visible, 5000)) {
                dialogs.current.close()
                // Frame 115 at 10 fps.
                player.jump_to("115", true)
            }
            break
        }
        case 10:
            if (t.waitFor("jump_to_frame", () => near(s.time, 11.5, 0.15), 5000, () => "time " + s.time))
                player.key(Qt.Key_Home, Qt.ControlModifier)
            break
        // The third line runs from 11 s to 12.5 s.
        case 11:
            if (t.waitFor("current_subtitle_position", () => near(s.time, 11.0, 0.15)
                              && near(s.sub_start, 11.0, 0.01), 5000,
                          () => "time " + s.time + " line " + s.sub_start))
                player.key(Qt.Key_Backslash, Qt.ControlModifier)
            break
        case 12:
            if (t.waitFor("ab_subtitle_section", () => s.ab_active && near(s.ab_a, 11.0, 0.05)
                              && near(s.ab_b, 12.5, 0.05), 3000,
                          () => "a " + s.ab_a + " b " + s.ab_b + " active " + s.ab_active))
                player.key(Qt.Key_BracketLeft, Qt.AltModifier)
            break
        case 13:
            if (t.waitFor("ab_point_a_later", () => near(s.ab_a, 11.1, 0.02), 3000, () => "a " + s.ab_a)) {
                player.section_edit("add", -1, "Line three")
                player.run_action("ab-clear")
            }
            break
        case 14:
            if (t.waitFor("ab_list_section", () => s.sections === 1 && !s.ab_active, 3000,
                          () => "sections " + s.sections))
                player.key(Qt.Key_Backslash, Qt.AltModifier)
            break
        case 15:
            if (t.waitFor("repeat_marked_section", () => s.ab_active && near(s.ab_a, 11.1, 0.02), 3000,
                          () => "a " + s.ab_a)) {
                player.run_action("ab-clear")
                setSetting("playback.skip.chapter-names", "Part A")
                setSetting("playback.skip.chapters", true)
                setSetting("playback.skip.enabled", true)
                player.jump_to("0:00:06", false)
            }
            break
        case 16:
            if (t.waitFor("skip_chapter", () => s.time !== null && s.time >= 9.9, 5000, () => "time " + s.time)) {
                setSetting("playback.skip.enabled", false)
                player.key(Qt.Key_P, 0)
            }
            break
        case 17:
            if (t.waitFor("bookmark_added", () => s.bookmarks === 1 && s.marks === 4, 3000,
                          () => "bookmarks " + s.bookmarks + " marks " + s.marks)) {
                m11.scenesArrived = 0
                player.key(Qt.Key_Z, Qt.ShiftModifier)
            }
            break
        case 18: {
            const browser = host.sceneBrowser()
            if (t.waitFor("time_scenes", () => browser !== null && browser.visible && browser.scenes.length >= 5
                              && m11.scenesArrived >= 3, 15000,
                          () => "scenes " + (browser ? browser.scenes.length : -1) + " pictures " + m11.scenesArrived)) {
                m11.scenes = browser.scenes
                browser.choose(2)
            }
            break
        }
        case 19:
            if (t.waitFor("scene_chosen", () => near(s.time, m11.scenes[2].time, 0.2), 5000,
                          () => "time " + s.time + " scene " + m11.scenes[2].time))
                player.key(Qt.Key_Z, Qt.AltModifier)
            break
        case 20: {
            const browser = host.sceneBrowser()
            // Three chapters after the start, the bookmark and the start.
            if (t.waitFor("marker_scenes", () => browser !== null && browser.visible && browser.scenes.length === 5,
                          5000, () => "scenes " + (browser ? browser.scenes.length : -1))) {
                browser.close()
                // The rows ask for their pictures while the panel shows.
                if (!s.playlist_attached)
                    player.run_action("toggle-playlist")
                player.run_list_action("view-thumbnails-3")
            }
            break
        }
        case 21:
            if (t.waitFor("playlist_thumbnails", () => s.list_rows === 3 && s.list_thumbnails >= 2, 15000,
                          () => "rows " + s.list_rows + " thumbnails " + s.list_thumbnails
                              + " attached " + s.playlist_attached)) {
                player.run_list_action("view-list")
                player.run_list_action("detect-durations")
            }
            break
        case 22:
            if (t.waitFor("durations_detected", () => s.durations_known === s.shown_entries, 15000,
                          () => "known " + s.durations_known + " of " + s.shown_entries)) {
                player.run_list_action("cursor-end")
                player.run_list_action("queue-add")
            }
            break
        case 23:
            if (t.waitFor("queued", () => s.queue === 1, 3000, () => "queue " + s.queue))
                player.run_action("next")
            break
        // The queue wins over the next entry (fixture-60s).
        case 24:
            if (t.waitFor("queue_plays_first", () => endsWith(s.path, "fixture-3s.mkv") && s.queue === 0,
                          10000, () => "path " + s.path))
                player.run_list_action("sort-duration")
            break
        case 25:
            if (t.waitFor("sort_by_duration", () => s.shown_first === "fixture-3s.mkv", 3000,
                          () => "first " + s.shown_first)) {
                player.list_dialog_answer("search", "chapters")
            }
            break
        case 26:
            if (t.waitFor("search_selects", () => s.cursor === 1 && s.selected === 1, 3000,
                          () => "cursor " + s.cursor + " selected " + s.selected)) {
                m11.listFile = t.outDir + "/album.m3u8"
                player.list_dialog_answer("save", m11.listFile)
                player.list_dialog_answer("open", m11.listFile)
            }
            break
        case 27:
            if (t.waitFor("playlist_file_round_trip", () => s.shown_entries === 6, 5000,
                          () => "entries " + s.shown_entries)) {
                player.run_list_action("remove-duplicates")
            }
            break
        case 28:
            if (t.waitFor("duplicates_removed", () => s.shown_entries === 3, 3000,
                          () => "entries " + s.shown_entries)) {
                // The power notice through the dry-run hook: a timer of a
                // second starts the countdown, which is cancelled first.
                // The timer comes from its dialog (Misc > Shutdown in).
                setSetting("playback.finished.countdown", 5)
                m11.dryRunBefore = s.power_dry_run
                player.run_action("shutdown-in")
            }
            break
        case 29: {
            const dialogs = host.playbackDialogs()
            if (t.waitFor("timer_dialog_opens", () => dialogs !== null && dialogs.current !== null
                              && dialogs.current.objectName === "timerDialog" && dialogs.current.opened,
                          5000)) {
                // What the dialog takes, and what is no time: negative
                // parts, minutes or seconds past 59, no time at all, an
                // "At" time no day has, an "In" time past the limit.
                const wrong = []
                const refused = [["-5", false], ["0", false], ["", false], ["0:00", false], ["1:60", false],
                                 ["1:-5", false], ["1:30:60", false], ["1.5", false], ["abc", false],
                                 ["14401", false], ["99:99", true], ["24:00", true], ["12:60", true],
                                 ["-1:00", true], ["1:02:03", true], ["", true]]
                for (const [text, at] of refused)
                    if (dialogs.timerAnswer(text, at) !== null)
                        wrong.push((at ? "at " : "in ") + JSON.stringify(text))
                const taken = [["90", false, 5400, "1:30"], ["1:30", false, 5400, "1:30"],
                               ["0:00:30", false, 30, "0:00:30"], ["14400", false, 864000, "240:00"]]
                for (const [text, at, seconds, label] of taken) {
                    const answer = dialogs.timerAnswer(text, at)
                    if (answer === null || answer.seconds !== seconds || answer.label !== label)
                        wrong.push("in " + JSON.stringify(text) + " " + JSON.stringify(answer))
                }
                const at = dialogs.timerAnswer("23:59", true)
                if (at === null || !(at.seconds > 0 && at.seconds <= 86400) || at.label !== "23:59")
                    wrong.push("at \"23:59\" " + JSON.stringify(at))
                t.check("timer_dialog_reads_times", wrong.length === 0, wrong.join(", "))
                // A negative "In" time once fired at once.
                const error = dialogs.typeTimer("-5", false)
                const after = t.status()
                t.check("timer_dialog_refuses_negative", error !== "" && after.power_timer === null
                            && after.power_seconds === 0,
                        "error " + JSON.stringify(error) + " timer " + after.power_timer)
            }
            break
        }
        case 30: {
            const dialogs = host.playbackDialogs()
            if (t.waitFor("timer_dialog_stays_open", () => dialogs.current.opened, 3000)) {
                dialogs.typeTimer("0:00:01", false)
                t.check("timer_dialog_sets_timer", t.status().power_timer === "0:00:01",
                        "timer " + t.status().power_timer)
            }
            break
        }
        case 31:
            if (t.waitFor("power_countdown_starts", () => s.power_seconds > 0, 5000,
                          () => "seconds " + s.power_seconds))
                player.cancel_countdown()
            break
        case 32:
            if (t.waitFor("power_countdown_cancelled", () => s.power_seconds === 0
                              && s.power_dry_run === m11.dryRunBefore, 3000,
                          () => "seconds " + s.power_seconds + " dry run " + s.power_dry_run))
                player.set_power_timer(1, 1, false, "1 s")
            break
        case 33:
            t.waitFor("power_action_dry_run", () => s.power_dry_run.indexOf("shut down") >= 0
                          && s.power_seconds === 0, 12000,
                      () => "seconds " + s.power_seconds + " dry run " + s.power_dry_run)
            break
        case 34: {
            // The Recycle Bin: a file of the test's own.
            m11.recycleFile = Tools.write_file(t.outDir + "/recycle-me.mkv", "RVP")
            player.open_files(m11.recycleFile, true)
            player.run_list_action("cursor-end")
            player.run_list_action("delete-files")
            t.next()
            break
        }
        case 35: {
            const dialogs = host.playbackDialogs()
            if (t.waitFor("recycle_asks_first", () => dialogs !== null && dialogs.data.kind === "recycle"
                              && dialogs.current !== null && dialogs.current.visible, 5000)) {
                dialogs.current.close()
                const error = player.recycle(JSON.stringify(dialogs.data.files),
                                             JSON.stringify(dialogs.data.entries))
                // The entry goes with its file: a status read after it.
                const after = t.status()
                // Windows' Recycle Bin, Linux's freedesktop.org trash.
                t.check("recycle_bin", error === "" && after.shown_entries === 3,
                        "error " + error + " entries " + after.shown_entries)
                // mpv's own DVD playback: the menus (M16) stay off here.
                setSetting("devices.dvd-menus", false)
                player.open_disc(player.option("dvd-folder"), false)
            }
            break
        }
        case 36:
            // A libmpv without libdvdnav (Ubuntu's in the WSL sysroot has
            // it; some distributions' do not): RVP refuses the disc, and the
            // audio joins a file that plays.
            if (!s.dvd_supported) {
                if (!m11.dvdSkipped) {
                    m11.dvdSkipped = true
                    t.check("dvd_folder_plays", true, "SKIPPED this libmpv opens no DVD")
                    if (!(s.loaded && s.duration - s.time > 5))
                        player.open_files(m11.chaptersFile, false)
                }
                if (t.waitFor("audio_joins_a_file", () => s.loaded && s.duration - s.time > 5
                                  && s.audio_tracks > 0, 10000,
                              () => "path " + s.path + " time " + s.time)) {
                    m11.audioTracks = s.audio_tracks
                    player.load_external_audio(player.option("audio"))
                }
                break
            }
            if (t.waitFor("dvd_folder_plays", () => s.loaded && String(s.path).indexOf("dvd://") === 0
                              && s.disc_titles === 2, 20000,
                          () => "path " + s.path + " titles " + s.disc_titles)) {
                m11.audioTracks = s.audio_tracks
                player.load_external_audio(player.option("audio"))
            }
            break
        case 37:
            if (t.waitFor("external_audio", () => s.audio_tracks === m11.audioTracks + 1, 8000,
                          () => "tracks " + s.audio_tracks)) {
                Tools.set_clipboard_text(player.option("clip"))
                player.run_action("open-clipboard")
            }
            break
        case 38:
            if (t.waitFor("open_from_clipboard", () => endsWith(s.path, "fixture-60s.mkv"), 10000,
                          () => "path " + s.path))
                player.run_action("favorite-item")
            break
        case 39:
            // The files of the command line and the DVD (unless skipped).
            if (t.waitFor("favorites_and_recent", () => s.favorites === 1
                              && s.recent >= (m11.dvdSkipped ? 3 : 4), 3000,
                          () => "favorites " + s.favorites + " recent " + s.recent)) {
                setSetting("open.ytdl-path", "rvp-no-such-yt-dlp")
                player.open_files("https://127.0.0.1:9/watch?v=m11", false)
            }
            break
        case 40:
            t.waitFor("ytdl_falls_back", () => String(s.osd).indexOf("yt-dlp:") === 0, 10000,
                      () => "osd " + s.osd)
            break
        case 41: {
            let devices = null
            try { devices = JSON.parse(player.devices_json()) } catch (error) { devices = null }
            t.check("capture_devices_listed", Array.isArray(devices),
                    "devices " + (devices ? devices.length : "none"))
            const stats = JSON.parse(player.status()).thumbnail_stats
            t.check("thumbnails_off_the_gui_thread", stats !== null && stats.rendered > 0
                        && stats.off_gui_thread === true, JSON.stringify(stats))
            t.finish()
            break
        }
        }
    }
    property string listFile: ""
    property int audioTracks: 0
    property string chaptersFile: ""
    property bool dvdSkipped: false
}
