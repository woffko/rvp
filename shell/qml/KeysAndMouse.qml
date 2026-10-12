// The settings window's key and mouse page (PotPlayer's Preferences >
// Keys). It edits the bindings through Player (editor_*): a key or gesture
// already in use asks before it moves, Reset gives an action PotPlayer's
// defaults back, and every change applies and is saved at once.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Rvp

Item {
    id: editor
    required property Player player

    property var rows: []
    property var gestures: []
    property string filter: ""
    property string scopeFilter: ""
    property string notice: ""
    // The row waiting for a key (its object), and a change waiting for a
    // yes because the key or gesture belongs to another action.
    property var capturing: null
    property var pending: null

    // The page is shown: edit the bindings in use now.
    function begin() {
        if (!editorOpen)
            player.editor_open()
        editorOpen = true
        gestures = JSON.parse(player.editor_gestures())
        capturing = null
        pending = null
        notice = "Select an action, then add a key or a mouse gesture."
        reload()
    }
    // The settings window closes.
    function end() {
        if (editorOpen)
            player.editor_close(false)
        editorOpen = false
    }
    property bool editorOpen: false
    function reload() { rows = JSON.parse(player.editor_rows()) }
    // Applies and saves the edited bindings, then goes on editing.
    function commit() {
        player.editor_close(true)
        player.editor_open()
        reload()
    }

    // "x1-double-click" -> "X1 Double Click".
    function gestureText(name) {
        return name.split("-").map(w => w.charAt(0).toUpperCase() + w.slice(1)).join(" ")
    }
    function scopeText(scope) { return scope === "playlist" ? "Playlist" : "Window" }

    function visibleRows() {
        return rows.filter(r => (scopeFilter === "" || r.scope === scopeFilter)
                           && (filter === "" || r.label.toLowerCase().indexOf(filter) >= 0
                               || r.keys.join(" ").toLowerCase().indexOf(filter) >= 0))
    }

    function startCapture(row) {
        pending = null
        capturing = row
        notice = "Press a key for “" + row.label + "” (" + scopeText(row.scope)
                 + "). Esc cancels."
        catcher.forceActiveFocus()
    }
    function keyPressed(event) {
        const row = capturing
        if (!row)
            return
        event.accepted = true
        if (event.key === Qt.Key_Escape && event.modifiers === Qt.NoModifier) {
            capturing = null
            notice = "Cancelled."
            return
        }
        const owner = JSON.parse(player.editor_key_owner(row.scope, event.key, event.modifiers))
        if (owner.key === undefined)
            return // a modifier alone: wait for the key
        capturing = null
        if (owner.owner !== "" && owner.owner !== row.name) {
            pending = { kind: "key", row: row, key: event.key, modifiers: event.modifiers,
                        text: owner.key, owner: owner.label }
            notice = owner.key + " is used by “" + owner.label + "”. Move it to “"
                     + row.label + "”?"
            return
        }
        bindKey(row, event.key, event.modifiers)
    }
    function bindKey(row, key, modifiers) {
        const result = JSON.parse(player.editor_bind_key(row.scope, row.name, key, modifiers))
        notice = result.key + " → “" + row.label + "”"
                 + (result.displaced !== "" ? " (taken from another action)" : "")
        commit()
    }
    function chooseGesture(row, gesture) {
        const owner = player.editor_gesture_owner(gesture)
        if (owner !== "" && owner !== row.label) {
            pending = { kind: "gesture", row: row, gesture: gesture, text: gestureText(gesture),
                        owner: owner }
            notice = gestureText(gesture) + " is used by “" + owner + "”. Move it to “"
                     + row.label + "”?"
            return
        }
        bindGesture(row, gesture)
    }
    function bindGesture(row, gesture) {
        player.editor_bind_gesture(row.name, gesture)
        notice = gestureText(gesture) + " → “" + row.label + "”"
        commit()
    }
    function confirm(yes) {
        const p = pending
        pending = null
        if (!yes || !p) {
            notice = "Unchanged."
            return
        }
        if (p.kind === "key") {
            bindKey(p.row, p.key, p.modifiers)
        } else if (p.kind === "gesture") {
            bindGesture(p.row, p.gesture)
        } else {
            player.editor_reset_all()
            notice = "All keys and mouse actions are PotPlayer's defaults again."
            commit()
        }
    }

    SystemPalette {
        id: colors
    }

    // Takes the key while one is being captured.
    Item {
        id: catcher
        Keys.onPressed: (event) => editor.keyPressed(event)
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 8

        RowLayout {
            Layout.fillWidth: true
            Label { text: "Find:" }
            TextField {
                Layout.fillWidth: true
                placeholderText: "action or key"
                onTextChanged: editor.filter = text.toLowerCase()
            }
            ComboBox {
                model: ["All", "Window", "Playlist"]
                onActivated: (index) => editor.scopeFilter = ["", "window", "playlist"][index]
            }
        }

        ListView {
            id: list
            objectName: "keyRows"
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: editor.visibleRows()
            ScrollBar.vertical: ScrollBar {}
            delegate: Rectangle {
                id: rowItem
                required property var modelData
                required property int index
                width: list.width
                height: rowLayout.implicitHeight + 8
                color: index % 2 ? colors.alternateBase : colors.base
                RowLayout {
                    id: rowLayout
                    anchors.fill: parent
                    anchors.margins: 4
                    spacing: 6
                    Label {
                        text: editor.scopeText(rowItem.modelData.scope)
                        opacity: 0.6
                        Layout.preferredWidth: 60
                    }
                    Label {
                        text: rowItem.modelData.label
                        elide: Text.ElideRight
                        Layout.preferredWidth: 220
                    }
                    Flow {
                        Layout.fillWidth: true
                        spacing: 4
                        Repeater {
                            model: rowItem.modelData.keys
                            delegate: Button {
                                required property string modelData
                                text: modelData + "  ×"
                                flat: true
                                onClicked: {
                                    editor.player.editor_unbind_key(rowItem.modelData.scope, modelData)
                                    editor.notice = modelData + " removed"
                                    editor.commit()
                                }
                            }
                        }
                        Repeater {
                            model: rowItem.modelData.gestures
                            delegate: Button {
                                required property string modelData
                                text: editor.gestureText(modelData) + "  ×"
                                flat: true
                                onClicked: {
                                    editor.player.editor_unbind_gesture(modelData)
                                    editor.notice = editor.gestureText(modelData) + " removed"
                                    editor.commit()
                                }
                            }
                        }
                    }
                    Button {
                        text: "Key…"
                        onClicked: editor.startCapture(rowItem.modelData)
                    }
                    Button {
                        text: "Mouse…"
                        visible: rowItem.modelData.scope === "window"
                        onClicked: gestureMenu.popup()
                        Menu {
                            id: gestureMenu
                            Repeater {
                                model: editor.gestures
                                delegate: MenuItem {
                                    required property string modelData
                                    text: editor.gestureText(modelData)
                                    onTriggered: editor.chooseGesture(rowItem.modelData, modelData)
                                }
                            }
                        }
                    }
                    Button {
                        text: "Reset"
                        onClicked: {
                            editor.player.editor_reset(rowItem.modelData.scope, rowItem.modelData.name)
                            editor.notice = "“" + rowItem.modelData.label + "” has PotPlayer's keys again"
                            editor.commit()
                        }
                    }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Label {
                Layout.fillWidth: true
                text: editor.notice
                wrapMode: Text.Wrap
            }
            Button {
                text: editor.pending !== null && editor.pending.kind === "all" ? "Reset" : "Move"
                visible: editor.pending !== null
                onClicked: editor.confirm(true)
            }
            Button {
                text: "Keep"
                visible: editor.pending !== null
                onClicked: editor.confirm(false)
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Label {
                text: "X1 is the Back button, X2 Forward."
                opacity: 0.6
            }
            Item { Layout.fillWidth: true }
            Button {
                text: "Reset All"
                onClicked: {
                    editor.pending = { kind: "all" }
                    editor.notice = "Give every action PotPlayer's keys and mouse actions again?"
                }
            }
        }
    }
}
