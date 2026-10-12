// The menus, loaded on first use: PotPlayer's context menu, the playlist
// panel's menu and the skin's menus, built from Player.menu_json
// (rvp_app::menu) each time one opens, so the keys bound now and RVP's
// check marks show. Items run Player.run_menu_command.
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
    // The menu built last and how many commands it has (self-test).
    property Menu current: null
    property int commandCount: 0

    component Entry: MenuItem {
        id: entry
        property string label
        property string commandId
        property string shortcut
        // -1: no check mark; 0 off; 1 on.
        property int mark: -1
        // "&" in PotPlayer's texts is a character, not a mnemonic.
        text: label.replace(/&/g, "&&")
        enabled: commandId !== ""
        onTriggered: holder.player.run_menu_command(commandId)
        contentItem: RowLayout {
            spacing: 24
            Label {
                text: (entry.mark === 1 ? "✓ " : entry.mark === 0 ? "  " : "") + entry.label
                opacity: entry.enabled ? 1 : 0.5
                Layout.fillWidth: true
            }
            Label {
                text: entry.shortcut
                opacity: 0.7
            }
        }
    }

    Component {
        id: menuComponent
        Menu {
            popupType: Popup.Window
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
        for (const item of items) {
            if (item.separator) {
                // Items belong to the menu's list, which deletes them with it.
                menu.addItem(separatorComponent.createObject(menu.contentItem))
            } else if (item.children.length > 0) {
                const sub = menuComponent.createObject(menu, { title: item.label.replace(/&/g, "&&") })
                fill(sub, item.children)
                menu.addMenu(sub)
            } else {
                menu.addItem(entryComponent.createObject(menu.contentItem, {
                    label: item.label, commandId: item.id, shortcut: item.shortcut, mark: item.checked
                }))
                if (item.id !== "")
                    holder.commandCount += 1
            }
        }
    }

    // Builds the menu of `kind` ("context", "playlist" or "skin").
    function build(kind) {
        if (holder.current) {
            holder.current.close()
            holder.current.destroy()
        }
        holder.commandCount = 0
        const menu = menuComponent.createObject(holder)
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
