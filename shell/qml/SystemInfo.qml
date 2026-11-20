// Playback/System Info (Ctrl+F1, M14), loaded on first use: the file, its
// video and audio, the rendering and the system, read from mpv once a
// second while the window shows (Player.system_info_json), with a button
// that copies it all.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Rvp
import RvpNative

Window {
    id: info
    required property Player player
    required property Window owner
    property var sections: []

    width: 560
    height: 620
    minimumWidth: 360
    minimumHeight: 300
    title: qsTr("Playback/System Info")
    transientParent: owner
    // Dialog Box: "Always keep modeless dialog boxes on top" (M14).
    flags: Qt.Dialog | (player.on_top || (player.general_look ? JSON.parse(player.general_look) : {}).dialogOnTop === true
                        ? Qt.WindowStaysOnTopHint : 0)
    color: palette.window

    function begin() {
        refresh()
        show()
        raise()
        requestActivate()
    }
    function refresh() {
        const read = JSON.parse(info.player.system_info_json())
        // Qt's version and the screens come from Qt itself.
        const env = JSON.parse(Tools.environment_json())
        const system = read.find(section => section.title === "System")
        if (system) {
            system.rows.push(["Qt", env.qt_version + " (" + env.platform + ")"])
            for (const screen of env.screens)
                system.rows.push(["Display " + screen.name, screen.width + "x" + screen.height + " at "
                                  + Math.round(screen.refresh_rate * 100) / 100 + " Hz, scale "
                                  + screen.device_pixel_ratio])
        }
        sections = read
    }
    function asText() {
        return sections.map(section => section.title + "\n" + section.rows
            .filter(row => row[1] !== null && row[1] !== undefined)
            .map(row => "  " + row[0] + ": " + row[1]).join("\n")).join("\n\n")
    }

    Timer {
        interval: 1000
        repeat: true
        running: info.visible
        onTriggered: info.refresh()
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 8
        ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            ColumnLayout {
                width: info.width - 40
                spacing: 4
                Repeater {
                    model: info.sections
                    delegate: ColumnLayout {
                        required property var modelData
                        Layout.fillWidth: true
                        spacing: 2
                        Label {
                            textFormat: Text.PlainText
                            text: modelData.title
                            font.bold: true
                            Layout.topMargin: 6
                        }
                        Repeater {
                            model: modelData.rows.filter(row => row[1] !== null && row[1] !== undefined)
                            delegate: RowLayout {
                                required property var modelData
                                Layout.fillWidth: true
                                Label {
                                    textFormat: Text.PlainText
                                    text: modelData[0]
                                    opacity: 0.7
                                    Layout.preferredWidth: 150
                                }
                                Label {
                                    textFormat: Text.PlainText
                                    text: String(modelData[1])
                                    wrapMode: Text.WrapAnywhere
                                    Layout.fillWidth: true
                                }
                            }
                        }
                    }
                }
            }
        }
        RowLayout {
            Layout.alignment: Qt.AlignRight
            Button {
                text: qsTr("Copy")
                onClicked: Tools.set_clipboard_text(info.asText())
            }
            Button {
                text: qsTr("Close")
                onClicked: info.close()
            }
        }
    }
}
