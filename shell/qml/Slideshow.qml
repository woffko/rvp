// The slideshow for audio files (M12's Visualization > Slideshow): the
// pictures Player.slideshow lists (JSON, `App::slideshow_json`: each one's
// file URL, escaped there, and its name), one after another with a
// cross-fade, over the video area. Jump to previous/next slide moves it
// (`move`).
import QtQuick
import Rvp

Item {
    id: show
    required property Player player
    property var data: ({ shown: false, files: [] })
    // The pictures' order (shuffled with "Random Slideshow") and where it is.
    property var order: []
    property int index: 0
    // The image that shows now: `first` or `second`.
    property bool firstShows: true
    visible: data.shown === true && order.length > 0

    Connections {
        target: show.player
        function onSlideshowChanged() { show.reload() }
    }
    Component.onCompleted: reload()

    function reload() {
        data = JSON.parse(show.player.slideshow || "{}")
        const files = data.files || []
        const list = files.map((_, i) => i)
        if (data.random) {
            for (let i = list.length - 1; i > 0; i--) {
                const j = Math.floor(Math.random() * (i + 1))
                const swap = list[i]
                list[i] = list[j]
                list[j] = swap
            }
        }
        order = list
        index = 0
        if (order.length > 0)
            showSlide()
    }
    function move(step) {
        if (order.length === 0)
            return
        index = (index + step + order.length) % order.length
        showSlide()
        timer.restart()
    }
    function showSlide() {
        const slide = data.files[order[index]]
        const next = firstShows ? second : first
        next.source = slide.url
        firstShows = !firstShows
        name.text = slide.name
    }
    function fillMode() {
        if (data.fill === "fill")
            return Image.PreserveAspectCrop
        if (data.fill === "stretch")
            return Image.Stretch
        return Image.PreserveAspectFit
    }

    Rectangle {
        anchors.fill: parent
        color: "black"
    }
    Image {
        id: first
        anchors.fill: parent
        asynchronous: true
        fillMode: show.fillMode()
        opacity: show.firstShows ? 1 : 0
        Behavior on opacity { NumberAnimation { duration: (show.data.fade || 0) * 1000 } }
    }
    Image {
        id: second
        anchors.fill: parent
        asynchronous: true
        fillMode: show.fillMode()
        opacity: show.firstShows ? 0 : 1
        Behavior on opacity { NumberAnimation { duration: (show.data.fade || 0) * 1000 } }
    }
    Text {
        id: name
        textFormat: Text.PlainText
        visible: show.data.names === true
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.margins: 8
        color: "white"
        style: Text.Outline
        styleColor: "black"
        font.pixelSize: 14
    }
    Timer {
        id: timer
        interval: Math.max(1, show.data.seconds || 5) * 1000
        repeat: true
        running: show.visible
        onTriggered: show.move(1)
    }
}
