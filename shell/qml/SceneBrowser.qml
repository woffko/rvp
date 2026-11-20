// The scene browsers (M11: Shift+Z, Alt+Z, Browse Scenes by Subtitle
// Intervals), loaded on first use: the pictures of the file that plays at
// time intervals, markers or subtitle lines, from the thumbnail core
// (Player.sceneReady tells when one arrived); a click jumps there. Only Qt
// Quick: no controls are needed.
import QtQuick
import Rvp
import RvpNative

Rectangle {
    id: browser
    required property Player player
    property var scenes: []
    property string title: ""
    property int pictureWidth: 192
    property int pictureHeight: 108
    property bool closeOnChange: true
    visible: false
    color: "#e6101014"
    focus: visible

    // `json`: rvp_app::scenes::Browser.
    function show(json) {
        const data = JSON.parse(json)
        title = data.title
        pictureWidth = data.width
        pictureHeight = data.height
        closeOnChange = data.close_on_change
        opacity = Math.max(0.2, data.opacity / 100)
        scenes = data.scenes
        visible = true
        grid.currentIndex = 0
        grid.positionViewAtBeginning()
        grid.forceActiveFocus()
    }
    function close() {
        visible = false
        scenes = []
        browser.player.scenes_closed()
    }
    function choose(index) {
        if (index < 0 || index >= scenes.length)
            return
        browser.player.scene_chosen(scenes[index].time)
        if (closeOnChange)
            close()
    }

    // Clicks stay in the browser.
    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.AllButtons
        onWheel: (wheel) => wheel.accepted = false
    }

    Text {
        id: heading
        x: 12
        y: 8
        color: "#e8e8f0"
        font.pixelSize: 15
        font.bold: true
        textFormat: Text.PlainText
        text: browser.title + " (" + browser.scenes.length + ")"
    }
    Text {
        anchors.right: parent.right
        anchors.rightMargin: 12
        y: 4
        color: closeArea.containsMouse ? "#ffffff" : "#a0a0b0"
        font.pixelSize: 20
        text: "×"
        MouseArea {
            id: closeArea
            anchors.fill: parent
            anchors.margins: -6
            hoverEnabled: true
            onClicked: browser.close()
        }
    }

    GridView {
        id: grid
        anchors.fill: parent
        anchors.topMargin: 36
        anchors.leftMargin: 8
        anchors.rightMargin: 8
        clip: true
        cellWidth: browser.pictureWidth + 12
        cellHeight: browser.pictureHeight + 34
        model: browser.scenes
        keyNavigationEnabled: true
        Keys.onEscapePressed: browser.close()
        Keys.onReturnPressed: browser.choose(currentIndex)
        Keys.onEnterPressed: browser.choose(currentIndex)
        delegate: Item {
            id: cell
            required property var modelData
            required property int index
            width: grid.cellWidth
            height: grid.cellHeight
            Rectangle {
                anchors.fill: parent
                anchors.margins: 2
                radius: 3
                color: cell.GridView.isCurrentItem || hover.containsMouse ? "#40608cff" : "transparent"
            }
            Rectangle {
                id: frame
                x: 6
                y: 6
                width: browser.pictureWidth
                height: browser.pictureHeight
                color: "#000000"
                ThumbView {
                    id: picture
                    key: "thumb/scene/" + cell.index
                    image_width: browser.pictureWidth
                    image_height: browser.pictureHeight
                    width: browser.pictureWidth
                    height: browser.pictureHeight
                }
                Connections {
                    target: browser.player
                    function onSceneReady(index) {
                        if (index === cell.index)
                            picture.update()
                    }
                }
            }
            Text {
                anchors.top: frame.bottom
                anchors.topMargin: 3
                x: 6
                width: browser.pictureWidth
                elide: Text.ElideRight
                color: "#d8d8e0"
                font.pixelSize: 12
                textFormat: Text.PlainText
                text: cell.modelData.label
            }
            MouseArea {
                id: hover
                anchors.fill: parent
                hoverEnabled: true
                onClicked: {
                    grid.currentIndex = cell.index
                    browser.choose(cell.index)
                }
            }
        }
    }
}
