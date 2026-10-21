// M14's self-test (`rvp --selftest=m14`), loaded by Main.qml's self-test:
// the skin, its colour themes, the windows' opacity and the audio skin
// switched while RVP runs, and the Misc menu's copied times. Input goes
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
            if (t.waitFor("m14_window_opaque_again", () => Math.abs(host.opacity - 1) < 0.01, 3000, () => shown(s))) {
                // Each skin mode keeps its window; a mode that forces its
                // smallest size (BlackSpace's audio skin) gets it.
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
            // The audio skin when the skin has one (BlackSpace), else the
            // video skin stays (RVP Default).
            t.check("m14_audio_skin_for_audio", firstSkin === "AudioSkin.xml"
                        || (firstSkin === "VideoSkin.xml" && s.skin_id === baseId), shown(s))
            // No video: a logo shows (RVP's for skins without one of
            // their own, or BlackSpace's Flash logo).
            t.check("m14_logo_without_video", player.skin_logo || player.logo_source.startsWith("qrc:"),
                    "logo " + player.logo_source + " skin " + player.skin_logo)
            t.next()
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
                player.run_action("previous")
            }
            break
        case 9:
            t.waitFor("m14_video_skin_again", () => String(s.path).endsWith("fixture-60s.mkv")
                          && s.skin_id === baseId, 10000, () => shown(s))
            break
        case 10:
            if (t.waitFor("m14_video_window_again", () => Math.abs(host.height - videoHeight) <= 2, 5000,
                          () => host.width + "x" + host.height + " video " + videoHeight))
                setSetting("skins.auto-hide", true)
            break
        case 11:
            // "Auto hide main skin": the video takes the window and the
            // bars come over it with the pointer.
            if (t.waitFor("m14_auto_hide_skin", () => player.overlay
                              && Math.abs(player.video_rect.width - host.width) <= 1
                              && Math.abs(player.video_rect.height - host.height) <= 1, 5000,
                          () => "overlay " + player.overlay + " video " + player.video_rect + " window "
                              + host.width + "x" + host.height))
                player.mouse_move(host.width / 2, host.height - 3)
            break
        case 12:
            if (t.waitFor("m14_bars_over_the_video", () => player.chrome_shown, 3000,
                          () => "shown " + player.chrome_shown)) {
                setSetting("skins.osc", false)
                player.mouse_leave()
                player.mouse_move(host.width / 2, host.height - 3)
            }
            break
        case 13:
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
            t.waitFor("m14_system_info_window", () => Tools.windows_json().indexOf("Playback/System Info") >= 0,
                      5000, () => Tools.windows_json())
            break
        case 21:
            t.finish()
            break
        }
    }
}
