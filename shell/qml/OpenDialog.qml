// The file dialogs, loaded on first use (Qt Quick Dialogs is not needed to
// start RVP): media files to open or add, a subtitle file to load, where
// to save the current frame or an audio recording, and the subtitle font.
import QtQuick
import QtQuick.Dialogs
import Rvp
import RvpNative

Item {
    id: holder
    required property Player player
    readonly property alias dialog: fileDialog
    // Add to the shown album instead of playing.
    property bool append: false

    function ask(adding) {
        holder.append = adding
        fileDialog.title = adding ? "Add File(s)" : "Open File(s)"
        // Association: "Set File Extensions in File Open Dialog" (M14).
        const extensions = (JSON.parse(holder.player.settings_view(false)).settings.associations["dialog-extensions"] || "")
            .split(/[\s,;]+/).filter(ext => ext !== "").map(ext => "*." + ext.replace(/^[*.]+/, ""))
        fileDialog.nameFilters = extensions.length > 0 ? ["Media (" + extensions.join(" ") + ")", "All files (*)"]
                                                       : ["All files (*)"]
        fileDialog.open()
    }
    // `combine`: a second subtitle shown with the first (Combine Subtitle).
    function askSubtitle(combine) {
        subtitleDialog.combine = combine === true
        subtitleDialog.title = subtitleDialog.combine ? "Combine Subtitle" : "Load Subtitle"
        subtitleDialog.open()
    }
    // `family`: the current subtitle font (empty: mpv's default).
    function askFont(family) {
        fontDialog.currentFont = Qt.font({ family: family !== "" ? family : "sans-serif", pointSize: 20 })
        fontDialog.open()
    }

    FontDialog {
        id: fontDialog
        title: qsTr("Subtitle Font")
        onAccepted: holder.player.apply_subtitle_font(selectedFont.family)
    }

    // `suggested`: a file URL in the image folder.
    function askFrame(suggested) {
        frameDialog.selectedFile = suggested
        frameDialog.currentFolder = suggested.substring(0, suggested.lastIndexOf("/"))
        frameDialog.open()
    }

    FileDialog {
        id: frameDialog
        title: qsTr("Save Current Frame As")
        fileMode: FileDialog.SaveFile
        nameFilters: ["Images (*.png *.jpg *.jpeg)", "All files (*)"]
        onAccepted: holder.player.save_frame_to(Tools.local_file(selectedFile))
    }

    // `suggested`: a file URL in the recording folder.
    function askRecording(suggested) {
        recordDialog.selectedFile = suggested
        recordDialog.currentFolder = suggested.substring(0, suggested.lastIndexOf("/"))
        recordDialog.open()
    }

    FileDialog {
        id: recordDialog
        title: qsTr("Record Audio")
        fileMode: FileDialog.SaveFile
        nameFilters: ["WAV audio (*.wav)", "All files (*)"]
        onAccepted: holder.player.start_recording(Tools.local_file(selectedFile))
    }

    FileDialog {
        id: subtitleDialog
        property bool combine: false
        title: qsTr("Load Subtitle")
        nameFilters: ["Subtitles (*.srt *.ass *.ssa *.sub *.idx *.sup *.vtt *.smi)", "All files (*)"]
        onAccepted: {
            if (combine)
                holder.player.combine_subtitle(Tools.local_file(selectedFile))
            else
                holder.player.load_subtitle(Tools.local_file(selectedFile))
        }
    }

    FileDialog {
        id: fileDialog
        title: qsTr("Open File(s)")
        fileMode: FileDialog.OpenFiles
        onAccepted: {
            const paths = []
            for (const url of selectedFiles)
                paths.push(Tools.local_file(url))
            holder.player.open_files(paths.join("\n"), holder.append)
        }
    }
}
