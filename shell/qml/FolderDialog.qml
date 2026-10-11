// The folder dialog, loaded on first use: a folder to add to the shown
// album, or PotPlayer's playlist folder to import (read-only).
import QtQuick
import QtQuick.Dialogs
import Rvp
import RvpNative

Item {
    id: holder
    required property Player player
    readonly property alias dialog: folderDialog
    // "add" or "import".
    property string purpose: "add"

    function ask(what) {
        holder.purpose = what
        folderDialog.open()
    }

    FolderDialog {
        id: folderDialog
        title: holder.purpose === "import" ? "Import PotPlayer Playlists (Playlist folder)"
                                           : "Add Folder"
        onAccepted: {
            const path = Tools.local_file(selectedFolder)
            if (holder.purpose === "import")
                holder.player.import_playlists(path)
            else
                holder.player.add_folder(path)
        }
    }
}
