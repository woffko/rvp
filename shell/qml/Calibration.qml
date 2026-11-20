// Motion smoothing's calibration (M18, the owner, 2026-11-07), opened from
// the Motion Smoothing page and loaded on first use: Start (it pauses
// playback), Cancel, the progress and the results, read from the player
// twice a second while the window shows (Player.calibration_status).
// Closing the window leaves a run going on.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Rvp
import RvpNative

Window {
    id: calibration
    required property Player player
    required property Window owner
    property var status: ({})

    width: 560
    height: 440
    minimumWidth: 420
    minimumHeight: 320
    title: qsTr("Motion Smoothing Calibration")
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
        status = JSON.parse(player.calibration_status() || "{}")
    }

    function presetText(preset) {
        return preset === "Fast" ? qsTr("Fast") : preset === "Medium" ? qsTr("Medium") : qsTr("Quality")
    }

    function stateText() {
        if (status.state === "running" && status.current && status.current.indexOf("TensorRT") >= 0)
            return qsTr("Measuring %1… The first measurement builds TensorRT's engine for this card: a few minutes.").arg(status.current)
        if (status.state === "running")
            return status.current ? qsTr("Measuring %1…").arg(status.current) : qsTr("Starting…")
        if (status.state === "measured")
            return qsTr("Measured on %1.").arg(status.date || "")
        if (status.state === "unavailable")
            return qsTr("No engine to measure here: RVP's VapourSynth runtime with MVTools or RIFE is missing.")
        if (status.state === "cancelled")
            return qsTr("Cancelled. It starts again with Start.")
        return qsTr("Not measured yet.")
    }

    Timer {
        interval: 500
        repeat: true
        running: calibration.visible
        onTriggered: calibration.refresh()
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 10

        Label {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            text: qsTr("RVP measures how many frames a second MVTools and RIFE make on this machine at each preset from 480p to 2160p, on a generated picture with no window or sound, as SVP does. It takes one to three minutes, and playback pauses for it. Each file then starts at the best preset that keeps its rate. It measures again on its own when the processors, graphics cards, their drivers or the engines change.")
        }
        Label {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            font.bold: true
            textFormat: Text.PlainText
            text: calibration.stateText()
        }
        ProgressBar {
            Layout.fillWidth: true
            from: 0
            to: 1
            visible: calibration.status.state === "running"
            indeterminate: (calibration.status.done || 0) === 0
            value: (calibration.status.done || 0) / Math.max(1, (calibration.status.done || 0) + (calibration.status.left || 0))
        }
        Label {
            visible: calibration.status.state === "running"
            textFormat: Text.PlainText
            text: qsTr("%1 done, at most %2 left").arg(calibration.status.done || 0).arg(calibration.status.left || 0)
            opacity: 0.7
        }
        Label {
            visible: (calibration.status.rows || []).length > 0 || (calibration.status.failed || []).length > 0
            text: qsTr("What keeps 60 frames a second with a fifth to spare:")
        }
        ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            ColumnLayout {
                spacing: 2
                Repeater {
                    model: calibration.status.rows || []
                    Label {
                        required property var modelData
                        textFormat: Text.PlainText
                        text: modelData.height > 0
                              ? qsTr("%1 %2: up to %3p").arg(modelData.engine).arg(calibration.presetText(modelData.preset)).arg(modelData.height)
                              : qsTr("%1 %2: below 480p").arg(modelData.engine).arg(calibration.presetText(modelData.preset))
                    }
                }
                Repeater {
                    model: calibration.status.failed || []
                    Label {
                        required property var modelData
                        textFormat: Text.PlainText
                        text: qsTr("%1: does not run here").arg(modelData)
                    }
                }
            }
        }
        RowLayout {
            Layout.alignment: Qt.AlignRight
            Button {
                text: qsTr("Start")
                enabled: calibration.status.state !== "running" && calibration.status.state !== "unavailable"
                onClicked: {
                    calibration.player.calibrate_now()
                    calibration.refresh()
                }
            }
            Button {
                text: qsTr("Cancel")
                enabled: calibration.status.state === "running"
                onClicked: {
                    calibration.player.cancel_calibration()
                    calibration.refresh()
                }
            }
            Button {
                text: qsTr("Close")
                onClicked: calibration.close()
            }
        }
    }
}
