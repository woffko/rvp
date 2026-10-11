// The file dialog, loaded on first use (Qt Quick Dialogs is not needed to
// start RVP).
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
