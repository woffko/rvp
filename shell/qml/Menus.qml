// The menus, loaded on first use: PotPlayer's context menu, the playlist
// panel's menu and the skin's menus, built from Player.menu_json
// (rvp_app::menu) each time one opens, so the keys bound now and RVP's
// check marks show. Items run Player.run_menu_command. Only the top level
// is built at once; a submenu gets its items when it first opens, so a
// right click during playback stays short (building all of PotPlayer's
// tree at once held the GUI thread long enough to drop video frames).
// Skins > Context Menu Style (M14): the system's own menus, the skin's
// colours or dark ones, a transparency and a size (Player.menu_look_json).
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Rvp
import RvpNative

Item {
    id: holder
    required property Player player
    // --log-geometry: list the windows once the menu's window is mapped.
    property bool logWindows: false
    // The menu built last (self-test).
    property Menu current: null
    // When a menu last began to close (Date.now()).
    property real closedAt: 0
    // How the menus look, read as one opens.
    property var look: ({})
    readonly property bool nativeMenus: look.native === true
    // The colours RVP draws with: the skin's style, or dark ones while the
    // system is dark; none keeps Qt's style.
    readonly property var colors: look.colors ? look.colors
        : look.dark === true && Qt.styleHints.colorScheme === Qt.ColorScheme.Dark
          ? { back: "#262628", border: "#3c3c40", text: "#ececf0", selectedText: "#ffffff",
              selectedBack: "#3a5f96", dimText: "#7c7c84", line: "#46464c" }
          : null
    readonly property real basePoint: Qt.application.font.pointSize > 0 ? Qt.application.font.pointSize : 9

    // Whether a press now belongs to a menu: one is open (or fading out),
    // or one began to close a moment ago. Qt closes a menu on a press
    // outside it and lets the press go on to the window beneath, where a
    // click on the video paused; PotPlayer's menus keep that click, so the
    // windows ignore such a press and its release (owner, 2026-10-16).
    function swallowsPress() {
        return (current !== null && current.visible) || Date.now() - closedAt < 250
    }

    component Entry: MenuItem {
        id: entry
        property string label
        property string commandId
        property string shortcut
        // -1: no check mark; 0 off; 1 on.
        property int mark: -1
        // "&" in PotPlayer's texts is a character, not a mnemonic. The
        // system's menus show the key after a tab and their own marks.
        text: label.replace(/&/g, "&&") + (holder.nativeMenus && shortcut !== "" ? "\t" + shortcut : "")
        checkable: holder.nativeMenus && mark >= 0
        checked: mark === 1
        enabled: commandId !== ""
        onTriggered: holder.player.run_menu_command(commandId)
        contentItem: RowLayout {
            spacing: 24
            Label {
                text: (entry.mark === 1 ? "✓ " : entry.mark === 0 ? "  " : "") + entry.label
                opacity: entry.enabled ? 1 : 0.5
                elide: Text.ElideRight
                Layout.fillWidth: true
            }
            Label {
                text: entry.shortcut
                opacity: 0.7
            }
        }
    }

    // An item in the skin's or the dark colours.
    component StyledEntry: Entry {
        id: styled
        readonly property var c: holder.colors
        background: Rectangle {
            color: styled.highlighted && styled.enabled ? styled.c.selectedBack : "transparent"
        }
        contentItem: RowLayout {
            spacing: 24
            Label {
                text: (styled.mark === 1 ? "✓ " : styled.mark === 0 ? "  " : "") + styled.label
                color: !styled.enabled ? styled.c.dimText : styled.highlighted ? styled.c.selectedText : styled.c.text
                elide: Text.ElideRight
                Layout.fillWidth: true
            }
            Label {
                text: styled.shortcut
                color: styled.highlighted ? styled.c.selectedText : styled.c.dimText
            }
        }
    }
    // A submenu's entry in those colours.
    component StyledSubEntry: MenuItem {
        id: sub
        readonly property var c: holder.colors
        background: Rectangle {
            color: sub.highlighted ? sub.c.selectedBack : "transparent"
        }
        contentItem: Label {
            text: sub.text
            color: sub.highlighted ? sub.c.selectedText : sub.c.text
            elide: Text.ElideRight
        }
    }

    Component {
        id: menuComponent
        Menu {
            id: submenu
            popupType: holder.nativeMenus ? Popup.Native : Popup.Window
            opacity: holder.look.opacity > 0 ? holder.look.opacity : 1
            font.pointSize: holder.basePoint * (holder.look.scale > 0 ? holder.look.scale : 1)
            // Qt's Menu keeps its background's width (200 pixels) whatever
            // its items need, so long labels ran into their shortcuts with
            // Ubuntu's fonts: take the widest item's width.
            width: {
                let widest = 0
                for (let i = 0; i < count; ++i) {
                    const item = itemAt(i)
                    if (item)
                        widest = Math.max(widest, item.implicitWidth)
                }
                return Math.max(implicitBackgroundWidth, widest + leftPadding + rightPadding)
            }
            // The items still to add (JSON), the first time it opens.
            property var pending: []
            onAboutToShow: holder.fillPending(submenu)
            onAboutToHide: holder.closedAt = Date.now()
        }
    }
    // The same menu in the skin's or the dark colours.
    Component {
        id: styledMenuComponent
        Menu {
            id: styledMenu
            popupType: Popup.Window
            opacity: holder.look.opacity > 0 ? holder.look.opacity : 1
            font.pointSize: holder.basePoint * (holder.look.scale > 0 ? holder.look.scale : 1)
            width: {
                let widest = 0
                for (let i = 0; i < count; ++i) {
                    const item = itemAt(i)
                    if (item)
                        widest = Math.max(widest, item.implicitWidth)
                }
                return Math.max(implicitBackgroundWidth, widest + leftPadding + rightPadding)
            }
            background: Rectangle {
                implicitWidth: 200
                color: holder.colors.back
                border.color: holder.colors.border
            }
            delegate: StyledSubEntry {}
            property var pending: []
            onAboutToShow: holder.fillPending(styledMenu)
            onAboutToHide: holder.closedAt = Date.now()
        }
    }
    Component {
        id: styledEntryComponent
        StyledEntry {}
    }
    Component {
        id: styledSeparatorComponent
        MenuSeparator {
            contentItem: Rectangle {
                implicitHeight: 1
                color: holder.colors.line
            }
        }
    }
    Component {
        id: entryComponent
        Entry {}
    }
    Component {
        id: separatorComponent
        MenuSeparator {}
    }

    function fill(menu, items) {
        const styled = holder.colors !== null && !holder.nativeMenus
        for (const item of items) {
            if (item.separator) {
                // Items belong to the menu's list, which deletes them with it.
                menu.addItem((styled ? styledSeparatorComponent : separatorComponent).createObject(menu.contentItem))
            } else if (item.children.length > 0) {
                menu.addMenu((styled ? styledMenuComponent : menuComponent).createObject(menu, {
                    title: Tools.tr_menu(item.label).replace(/&/g, "&&"), pending: item.children
                }))
            } else {
                menu.addItem((styled ? styledEntryComponent : entryComponent).createObject(menu.contentItem, {
                    label: Tools.tr_menu(item.label), commandId: item.id, shortcut: item.shortcut, mark: item.checked
                }))
            }
        }
    }
    function fillPending(menu) {
        if (menu.pending.length === 0)
            return
        const items = menu.pending
        menu.pending = []
        fill(menu, items)
    }

    // Builds the menu of `kind` ("context", "playlist" or "skin").
    function build(kind) {
        if (holder.current) {
            holder.current.close()
            holder.current.destroy()
        }
        holder.look = JSON.parse(holder.player.menu_look_json())
        const styled = holder.colors !== null && !holder.nativeMenus
        const menu = (styled ? styledMenuComponent : menuComponent).createObject(holder)
        fill(menu, JSON.parse(holder.player.menu_json(kind)))
        holder.current = menu
        return menu
    }

    // Shows it at the pointer.
    function show(kind) {
        build(kind).popup()
        if (holder.logWindows)
            windowsLog.restart()
    }

    // The menu's window maps after it opens; list the windows a little later.
    Timer {
        id: windowsLog
        interval: 400
        onTriggered: Tools.log("menu_windows=" + Tools.windows_json())
    }
}
