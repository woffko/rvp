// The file dialogs, loaded on first use (Qt Quick Dialogs is not needed to
// start RVP): media files to open or add, and a subtitle file to load.
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
        fileDialog.open()
    }
    function askSubtitle() {
        subtitleDialog.open()
    }

    FileDialog {
        id: subtitleDialog
        title: "Load Subtitle"
        nameFilters: ["Subtitles (*.srt *.ass *.ssa *.sub *.idx *.sup *.vtt *.smi)", "All files (*)"]
        onAccepted: holder.player.load_subtitle(Tools.local_file(selectedFile))
    }

    FileDialog {
        id: fileDialog
        title: "Open File(s)"
        fileMode: FileDialog.OpenFiles
        onAccepted: {
            const paths = []
            for (const url of selectedFiles)
                paths.push(Tools.local_file(url))
            holder.player.open_files(paths.join("\n"), holder.append)
        }
    }
}
