// M14's self-test (`rvp --selftest=m14`), loaded by Main.qml's self-test:
// the skin, its colour themes, the windows' opacity and the audio skin
// switched while RVP runs, the Misc menu's copied times, the interface
// in Russian, and the settings window's search and Reset Page. Input goes
// through Player's handlers, as in the functional check.
//
// Files: testdata/media/fixture-60s.mkv, then fixture-tone.ogg (no video:
// the audio skin when the skin has one). With --skin=BlackSpace_en.dsf the
// skin's own themes and audio skin are checked; with RVP Default the
// custom theme.
import QtQuick
import Rvp
import RvpNative

Item {
    id: m14
    required property Player player
    required property var t
    required property var host
    property var baseId: null
    property string firstSkin: ""
    property int videoHeight: 0
    property var rememberBefore: true
    property bool audioChecked: false
    // When the test restored a maximized window (0: it did not).
    property real restored: 0
    property var fitBefore: null
    // Where the pointer went over the bars (case 11).
    property real pointerY: 0
    // The tooltip case 12 opens.
    property var tipShown: null

    function view() { return JSON.parse(player.settings_view(false)) }
    function setSetting(path, value) {
        return JSON.parse(player.set_setting(path, JSON.stringify(value)))
    }
    function shown(s) {
        return "skin " + s.skin_file + " #" + s.skin_id + " opacity " + host.opacity + " osd " + (s.osd || "")
    }

    function tick() {
        const t = m14.t
        const s = t.status()
        switch (t.step) {
        case 0:
            t.next()
            break
        case 1:
            if (t.waitFor("m14_video_skin", () => s.loaded && String(s.path).endsWith("fixture-60s.mkv")
                              && /VideoSkin|Skin\.xml/i.test(s.skin_file), 20000, () => shown(s))) {
                baseId = s.skin_id
                const themes = view().skin_themes
                if (themes.length > 0)
                    setSetting("skins.color-theme", themes[0])
                else
                    player.run_action("theme-custom")
            }
            break
        case 2:
            if (t.waitFor("m14_theme_tints_the_skin", () => s.skin_id !== baseId && s.skin_id !== null, 5000,
                          () => shown(s)))
                player.run_action("theme-none")
            break
        case 3:
            if (t.waitFor("m14_no_theme_is_the_skin_again", () => s.skin_id === baseId, 5000, () => shown(s)))
                player.run_action("opacity-50")
            break
        case 4:
            if (t.waitFor("m14_window_opacity", () => Math.abs(host.opacity - 0.5) < 0.01, 3000, () => shown(s)))
                player.run_action("opacity-100")
            break
        case 5:
            // GNOME maximizes a first window that covers most of the
            // screen, and a maximized window keeps its size when the audio
            // skin comes (as on Windows): restored, and settled (a Wayland
            // compositor answers with the size it restores).
            if (host.visibility === Window.Maximized) {
                host.requestVisibility(Window.Windowed)
                restored = Date.now()
            }
            if (t.waitFor("m14_window_opaque_again", () => Math.abs(host.opacity - 1) < 0.01
                              && host.visibility === Window.Windowed && Date.now() - restored > 1500,
                          5000, () => shown(s) + " visibility " + host.visibility)) {
                // Each skin mode keeps its window; a mode that forces its
                // smallest size (BlackSpace's audio skin) gets it. The fit
                // to a newly opened video would size the window again.
                fitBefore = view().settings["fit-window-to-video"]
                setSetting("fit-window-to-video", false)
                setSetting("skins.keep-size", false)
                setSetting("skins.remember-mode-window", true)
                videoHeight = host.height
                player.run_action("next")
            }
            break
        case 6:
            if (t.waitFor("m14_audio_file_plays", () => String(s.path).endsWith("fixture-tone.ogg"), 10000,
                          () => shown(s)))
                firstSkin = s.skin_file
            break
        case 7:
            if (!audioChecked) {
                audioChecked = true
                // The audio skin when the skin has one (BlackSpace), else
                // the video skin stays (RVP Default).
                t.check("m14_audio_skin_for_audio", firstSkin === "AudioSkin.xml"
                            || (firstSkin === "VideoSkin.xml" && s.skin_id === baseId), shown(s))
                // No video: a logo shows (RVP's for skins without one of
                // their own, or BlackSpace's Flash logo).
                t.check("m14_logo_without_video", player.skin_logo || player.logo_source.startsWith("qrc:"),
                        "logo " + player.logo_source + " skin " + player.skin_logo)
            }
            // BlackSpace's audio skin draws the spectrum: the tone lights it.
            if (firstSkin !== "AudioSkin.xml") {
                t.check("m14_spectrum_moves", true, "SKIPPED: the skin draws no spectrum")
                t.next()
            } else {
                if (t.waitFor("m14_spectrum_moves", () => player.spectrum_shown && s.spectrum > 0.05, 8000,
                              () => "spectrum " + s.spectrum + " shown " + player.spectrum_shown))
                    Tools.grab(t.outDir, "spectrum")
            }
            break
        case 8:
            if (firstSkin !== "AudioSkin.xml" || !s.skin_forces_height) {
                t.check("m14_audio_skin_window", true, "SKIPPED: the skin has no audio skin forcing its height")
                player.run_action("previous")
                t.next()
            } else if (t.waitFor("m14_audio_skin_window", () => host.height === host.minimumHeight
                                     && host.height < videoHeight, 5000,
                                 () => host.width + "x" + host.height + " minimum " + host.minimumWidth + "x"
                                     + host.minimumHeight + " video " + videoHeight)) {
                // Mini Audio Mode: the audio skin's own button shows
                // AudioMini.xml, whose button goes back.
                const click = (command) => {
                    const r = player.control_rect(command)
                    if (r.width > 0) {
                        player.mouse_press(r.x + r.width / 2, r.y + r.height / 2, Qt.LeftButton, 0)
                        player.mouse_release(r.x + r.width / 2, r.y + r.height / 2, Qt.LeftButton)
                    }
                }
                click("CMD_LOAD_SKIN_XML")
                const mini = t.status().skin_file
                click("CMD_LOAD_SKIN_XML")
                const back = t.status().skin_file
                t.check("m14_mini_audio_mode", mini === "AudioMini.xml" && back === "AudioSkin.xml",
                        mini + " then " + back)
                player.run_action("previous")
            }
            break
        case 9:
            t.waitFor("m14_video_skin_again", () => String(s.path).endsWith("fixture-60s.mkv")
                          && s.skin_id === baseId, 10000, () => shown(s))
            break
        case 10:
            if (t.waitFor("m14_video_window_again", () => Math.abs(host.height - videoHeight) <= 2, 5000,
                          () => host.width + "x" + host.height + " video " + videoHeight)) {
                setSetting("fit-window-to-video", fitBefore)
                setSetting("skins.auto-hide", true)
            }
            break
        case 11:
            // "Auto hide main skin": the video takes the window and the
            // bars come over it with the pointer.
            if (t.waitFor("m14_auto_hide_skin", () => player.overlay
                              && Math.abs(player.video_rect.width - host.width) <= 1
                              && Math.abs(player.video_rect.height - host.height) <= 1, 5000,
                          () => "overlay " + player.overlay + " video " + player.video_rect + " window "
                              + host.width + "x" + host.height)) {
                pointerY = host.height - 3
                player.mouse_move(host.width / 2, pointerY)
            }
            break
        case 12:
            if (t.waitFor("m14_bars_over_the_video", () => player.chrome_shown, 3000,
                          () => "shown " + player.chrome_shown)) {
                // Only the bar under the pointer shows, as in PotPlayer:
                // the bottom one here.
                const bar = player.shown_bar
                t.check("m14_only_the_pointers_bar", bar.height > 0 && bar.y <= pointerY
                            && pointerY < bar.y + bar.height && bar.y > pointerY / 2,
                        String(bar) + " pointer at " + pointerY + " window " + host.width + "x" + host.height)
                // A tooltip by a pointer on the bottom bar (checked once
                // its window is placed, in the next step).
                tipShown = host.showTip("RVP", host.width / 2, pointerY)
                setSetting("skins.osc", false)
                player.mouse_leave()
                player.mouse_move(host.width / 2, host.height - 3)
            }
            break
        case 13:
            if (tipShown !== null) {
                // It opens by the pointer, inside the window (below it
                // where it fits: a docked playlist leaves room): Qt moves
                // a tooltip window that runs off the screen to its top.
                const at = tipShown.contentItem.mapToGlobal(0, 0)
                const y = at.y - host.contentItem.mapToGlobal(0, 0).y
                t.check("m14_tooltip_by_the_pointer", tipShown.opened && y >= 0
                            && y + tipShown.implicitHeight <= host.height
                            && Math.abs(pointerY - y) < 3 * tipShown.implicitHeight,
                        "tip text at " + Math.round(y) + " height " + tipShown.implicitHeight + " pointer at "
                            + pointerY + " window " + host.width + "x" + host.height)
                tipShown.close()
                tipShown = null
            }
            t.check("m14_osc_off_hides_the_bars", !player.chrome_shown, "shown " + player.chrome_shown)
            setSetting("skins.osc", true)
            setSetting("skins.auto-hide", false)
            t.next()
            break
        case 14:
            t.waitFor("m14_skin_back_around_the_video", () => !player.overlay
                          && player.video_rect.width < host.width, 5000,
                      () => "overlay " + player.overlay + " video " + player.video_rect)
            break
        case 15: {
            // Another skin from the Skins menu's list: RVP Default (its
            // first entry) or, from RVP Default, the next one.
            const v = view()
            if ((v.choices.skin || []).length === 0) {
                t.check("m14_skin_switches_while_running", true, "SKIPPED: RVP Default is the only skin here")
                t.next()
                t.next()
                break
            }
            player.run_menu_command(v.skin_shown === null ? "d:s1" : "d:s0")
            t.next()
            break
        }
        case 16:
            t.waitFor("m14_skin_switches_while_running", () => s.skin_id !== baseId
                          && (s.osd || "").startsWith("Skin: "), 5000, () => shown(s))
            break
        case 17:
            player.run_action("copy-elapsed-time")
            t.next()
            break
        case 18: {
            const copied = Tools.clipboard_text()
            t.check("m14_copy_elapsed_time", /^\d+:\d\d$/.test(copied), copied)
            t.next()
            break
        }
        case 19: {
            // Context Menu Design: a menu file of RVP's menus folder, in
            // PotPlayer's format; Context Menu Items hides a top-level one.
            const folder = Tools.local_file(player.settings_folder_url("menus"))
            Tools.write_file(folder + "/rvp-test.xml", "<Menu Title=\"{$CP0=Test menu$}\">"
                             + "<MenuItem CmdID='ID_PLAY_PAUSE' /><MenuItem CmdID='' />"
                             + "<SubMenu Name='$MENU_VIEW'><MenuItem CmdID='CMD_POPUPMENU_SKIN' /></SubMenu></Menu>")
            setSetting("skins.menu-design", "rvp-test.xml")
            const top = () => JSON.parse(player.menu_json("context")).filter(item => !item.separator)
            let items = top()
            t.check("m14_menu_file", items.length === 2 && items[1].label === "View",
                    items.map(item => item.label).join(", "))
            setSetting("skins.menu-hidden", ["View"])
            items = top()
            t.check("m14_menu_items_hidden", items.length === 1, items.map(item => item.label).join(", "))
            setSetting("skins.menu-design", "skin")
            setSetting("skins.menu-hidden", [])
            t.check("m14_menu_back_to_rvps", top().length > 10, top().length + " items")
            // Context Menu Style: the system's menus at 70 % opacity.
            setSetting("skins.menu-style", "system")
            setSetting("skins.menu-transparency", 30)
            const look = JSON.parse(player.menu_look_json())
            t.check("m14_menu_look", look.native === true && Math.abs(look.opacity - 0.7) < 0.01,
                    JSON.stringify(look))
            setSetting("skins.menu-style", "auto")
            setSetting("skins.menu-transparency", 0)
            // Zoom Skinned Windows: the skin draws half as large again.
            const bar = player.control_rect("CMD_TIMEBAR_CTRL")
            setSetting("skins.zoom", 150)
            const zoomed = player.control_rect("CMD_TIMEBAR_CTRL")
            t.check("m14_skin_zoom", bar.height > 0 && Math.abs(zoomed.height - bar.height * 1.5) <= 1,
                    bar.height + " -> " + zoomed.height)
            setSetting("skins.zoom", 100)
            // Misc info (Scroll Lock) and the short info (Shift+Tab).
            player.key(Qt.Key_ScrollLock, 0)
            t.check("m14_misc_info", /\d\d:\d\d:\d\d/.test(player.misc_info), player.misc_info)
            player.key(Qt.Key_ScrollLock, 0)
            t.check("m14_misc_info_off", player.misc_info === "", player.misc_info)
            player.key(Qt.Key_Backtab, Qt.ShiftModifier)
            t.check("m14_short_info", player.osd.indexOf("fixture-60s.mkv") >= 0
                        && player.osd.indexOf("Subtitles") >= 0, player.osd)
            // Playback/System Info (Ctrl+F1).
            const sections = JSON.parse(player.system_info_json())
            const video = sections.find(section => section.title === "Video")
            const codec = video ? video.rows.find(row => row[0] === "Codec") : null
            t.check("m14_system_info_reads_mpv", codec && /264/.test(codec[1]), JSON.stringify(video))
            player.key(Qt.Key_F1, Qt.ControlModifier)
            t.next()
            break
        }
        case 20:
            // By its window's type: the title is in the interface's language.
            t.waitFor("m14_system_info_window", () => JSON.parse(Tools.windows_json())
                          .some(w => w.visible && String(w["class"]).indexOf("SystemInfo") === 0),
                      5000, () => Tools.windows_json())
            break
        case 21: {
            // General: pause while minimized or without the focus, and play
            // on when that ends (only what the window paused).
            setSetting("general.pause-minimized", true)
            setSetting("general.pause-focus-lost", true)
            player.window_attention(0, true)
            t.next()
            break
        }
        case 22:
            if (t.waitFor("m14_pause_when_minimized", () => s.pause === true, 3000, () => "pause " + s.pause))
                player.window_attention(0, false)
            break
        case 23:
            if (t.waitFor("m14_play_on_when_restored", () => s.pause === false, 3000, () => "pause " + s.pause))
                player.window_attention(1, false)
            break
        case 24:
            if (t.waitFor("m14_pause_when_focus_lost", () => s.pause === true, 3000, () => "pause " + s.pause))
                player.window_attention(1, true)
            break
        case 25:
            if (t.waitFor("m14_play_on_with_focus", () => s.pause === false, 3000, () => "pause " + s.pause)) {
                setSetting("general.pause-minimized", false)
                setSetting("general.pause-focus-lost", false)
            }
            break
        case 26: {
            // Presets (Configuration): a preset holds its own values; the
            // default's wait meanwhile.
            setSetting("presets.list", [{ name: "Quiet", conditions: {}, values: null }])
            const step = view().settings["volume-step"]
            player.run_menu_command("p:1")
            t.check("m14_preset_activates", view().settings.presets.active === "Quiet",
                    "active " + view().settings.presets.active)
            setSetting("volume-step", 7)
            player.run_menu_command("p:0")
            t.check("m14_default_preset_keeps_its_values", view().settings.presets.active === ""
                        && view().settings["volume-step"] === step, "step " + view().settings["volume-step"])
            player.run_menu_command("p:1")
            t.check("m14_preset_keeps_its_values", view().settings["volume-step"] === 7,
                    "step " + view().settings["volume-step"])
            player.run_menu_command("p:0")
            // Auto-Preset: the tone file meets the file name condition.
            setSetting("presets.list", [{ name: "Quiet", conditions: { "file-name": "*tone*" },
                                          values: JSON.parse(JSON.stringify(view().settings.presets.list[0].values)) }])
            player.run_action("next")
            t.next()
            break
        }
        case 27:
            if (t.waitFor("m14_preset_auto_activates", () => String(s.path).endsWith("fixture-tone.ogg")
                              && view().settings.presets.active === "Quiet", 10000,
                          () => "path " + s.path + " active " + view().settings.presets.active))
                player.run_menu_command("p:0")
            break
        case 28: {
            // Accessibility: notices as the window's title, a swipe that
            // seeks, the voices (none outside Windows) and the skin's
            // touch layout.
            setSetting("accessibility.title.messages", true)
            player.run_action("theme-none")
            t.check("m14_messages_as_title", host.title.indexOf("Color theme") >= 0, host.title)
            setSetting("accessibility.title.messages", false)
            setSetting("accessibility.touch", true)
            player.touch_swipe(0.25, 0.01)
            t.check("m14_swipe_seeks", player.osd === "+30 s", player.osd)
            setSetting("accessibility.touch", false)
            const voices = JSON.parse(player.tts_voices_json())
            t.check("m14_voices_list", Array.isArray(voices), JSON.stringify(voices))
            baseId = s.skin_id
            setSetting("accessibility.touch-skin", true)
            t.next()
            break
        }
        case 29:
            if (t.waitFor("m14_touch_layout_loads", () => s.skin_id !== baseId, 5000, () => shown(s)))
                setSetting("accessibility.touch-skin", false)
            break
        case 30:
            // Interface language: RVP's Russian translation loads and its
            // texts fit their controls in the settings window.
            setSetting("general.language", "ru")
            t.next()
            break
        case 31:
            if (t.waitFor("m14_russian_loads", () => qsTranslate("Menu", "Open File(s)...") === "Открыть файл(ы)...",
                          5000, () => qsTranslate("Menu", "Open File(s)...") + " " + Qt.uiLanguage)) {
                player.run_action("preferences")
            }
            break
        case 32: {
            const settings = host.settingsWindow()
            if (!settings || !settings.visible) {
                if (Date.now() - t.stepStarted > 10000) {
                    t.check("m14_russian_fits", false, "no settings window")
                    t.next()
                }
                break
            }
            const overflow = []
            const walk = (item, page) => {
                for (const child of item.children || []) {
                    if (!child.visible)
                        continue
                    if (child.text !== undefined && typeof child.text === "string" && child.text !== ""
                            && child.width > 0 && (child.truncated === true
                                || (child.wrapMode === Text.NoWrap && child.elide === Text.ElideNone
                                    && child.implicitWidth > child.width + 2 && child.clip !== true)))
                        overflow.push(page + ": " + child.text)
                    walk(child, page)
                }
            }
            for (let i = 0; i < settings.pages.length; i++) {
                settings.showPage(i)
                const page = settings.pageItem(i)
                if (page)
                    walk(page, settings.pages[i].key)
            }
            t.check("m14_russian_fits", overflow.length === 0, overflow.length + " " + overflow.slice(0, 40).join(" | "))
            settings.close()
            setSetting("general.language", "en")
            t.next()
            break
        }
        case 33:
            // The settings window's search and Reset Page: the volume
            // page's step goes back to its default, another page's option
            // stays.
            rememberBefore = view().settings["remember-positions"]
            setSetting("volume-step", 7)
            setSetting("remember-positions", !rememberBefore)
            player.run_action("preferences")
            t.next()
            break
        case 34: {
            const settings = host.settingsWindow()
            const ready = settings && settings.visible
            let match = undefined
            if (ready) {
                settings.search("")
                settings.search("volume step")
                match = settings.found.find(entry => entry.item.path === "volume-step")
            }
            if (t.waitFor("m14_settings_search", () => match !== undefined, 10000,
                          () => ready ? settings.found.length + " found of " + settings.searchIndex.length + ", "
                                        + settings.searchIndex.slice(0, 3).map(e => e.text).join(" | ")
                                        + " " + Qt.uiLanguage
                                      : "no settings window") && match) {
                settings.openFound(match)
                settings.search("")
                const page = settings.currentKey
                settings.resetPage()
                const now = view().settings
                t.check("m14_reset_page", page === settings.pages[match.page].key && now["volume-step"] === 1
                        && now["remember-positions"] === !rememberBefore,
                        page + " step " + now["volume-step"] + " remember " + now["remember-positions"])
                setSetting("remember-positions", rememberBefore)
                settings.close()
            }
            break
        }
        case 35:
            // The playlist's Create Thumbnail Image(s): a sheet for each
            // selected entry, one after another.
            setSetting("image-folder", t.outDir + "/m14-images")
            player.run_list_action("select-all")
            player.run_list_action("thumbnail-images")
            t.next()
            break
        case 36: {
            const dialogs = host.videoDialogs()
            if (t.waitFor("m14_entry_thumbnails_dialog", () => dialogs.current
                              && dialogs.current.objectName === "sheetDialog" && dialogs.current.opened,
                          3000)) {
                dialogs.current.close()
                dialogs.answer(JSON.stringify({ columns: 2, rows: 2, width: 320, info: true,
                                                times: true, exact: false }))
            }
            break
        }
        case 37: {
            const images = () => JSON.parse(Tools.list_files(t.outDir + "/m14-images"))
            t.waitFor("m14_entry_thumbnail_images", () => images().length === 2, 30000,
                      () => images().join(", ") + " osd " + s.osd)
            break
        }
        case 38: {
            // Windows, with --registry-check (the test VM's m14-all.ps1):
            // the start with the session (the Run key) and the file types
            // (HKCU\Software\Classes), each put back after.
            if (Qt.platform.os !== "windows" || !player.has_option("registry-check")) {
                t.check("m14_run_at_startup", true, "SKIPPED: Windows with --registry-check only")
                t.check("m14_associations", true, "SKIPPED: Windows with --registry-check only")
                t.next()
                break
            }
            const run = "Software\\Microsoft\\Windows\\CurrentVersion\\Run"
            setSetting("general.run-at-startup", true)
            const added = Tools.registry_has_value(run, "RVP")
            setSetting("general.run-at-startup", false)
            const removed = !Tools.registry_has_value(run, "RVP")
            t.check("m14_run_at_startup", added && removed, "added " + added + " removed " + removed)
            // An installed copy's types stay as they are.
            if (JSON.parse(player.associations_json()).some(type => type.registered)) {
                t.check("m14_associations", true, "SKIPPED: RVP's types are registered already")
                t.next()
                break
            }
            player.apply_associations()
            const types = JSON.parse(player.associations_json())
            const registered = types.filter(type => type.registered).length
            setSetting("associations.off", types.map(type => type.extension))
            player.apply_associations()
            const left = JSON.parse(player.associations_json()).filter(type => type.registered).length
            setSetting("associations.off", [])
            t.check("m14_associations", types.length > 0 && registered === types.length && left === 0,
                    registered + " of " + types.length + " registered, " + left + " left after")
            t.next()
            break
        }
        case 39:
            t.finish()
            break
        }
    }
}
