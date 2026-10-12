// The folder dialog, loaded on first use: a folder to open or to add to
// the shown album, PotPlayer's playlist folder to import (read-only), or
// the folder for saved frames.
import QtQuick
import QtQuick.Dialogs
import Rvp
import RvpNative

Item {
    id: holder
    required property Player player
    readonly property alias dialog: folderDialog
    // "open", "add", "import" or "images".
    property string purpose: "add"

    function ask(what) {
        holder.purpose = what
        folderDialog.open()
    }

    FolderDialog {
        id: folderDialog
        title: ({ "open": "Open Folder", "add": "Add Folder",
                  "import": "Import PotPlayer Playlists (Playlist folder)",
                  "images": "Folder for Saved Frames" })[holder.purpose]
        onAccepted: {
            const path = Tools.local_file(selectedFolder)
            if (holder.purpose === "import")
                holder.player.import_playlists(path)
            else if (holder.purpose === "open")
                holder.player.open_folder(path)
            else if (holder.purpose === "images")
                holder.player.set_image_folder(path)
            else
                holder.player.add_folder(path)
        }
    }
}
