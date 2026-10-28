import QtQuick 2.5
import QtQuick.Window 2.2
import net.asivery.AppLoad 1.0
import net.asivery.Framebuffer 1.0

// Jean's screen as a grid of small framebuffer items. xochitl damages the panel per Qt item, so one big item means every update re-drives the whole
// screen; with many small ones an update to the seconds (or a percentage) dirties only the item it falls in, and the rest of the panel is left alone.
//
// AppLoad gives an app a logical canvas of 1620x2160 and scales it onto the screen (0.8667 on the reMarkable 2, 1404x1872). A resampled dither is ruined,
// and the grid would not fit, so `stage` undoes that: it measures how big the canvas really is on the screen and counter-scales itself, and sits at the
// screen's own top-left corner. Inside `stage` one unit is one panel pixel, so a tile pixel is a panel pixel.
//
// The backend (backend/entry, jean-tiles.c) keeps the keys, sizes and counts below in step: tile i is framebuffer 5000000 + i, 117x117 screen px, 12 across.
Rectangle {
    id: root
    anchors.fill: parent
    color: "white"
    signal close
    function unloading() { endpoint.terminate(); }

    AppLoad { id: endpoint; applicationID: "jean-tiles" }

    readonly property int cols: 12
    readonly property int rows: 16
    readonly property int tw: 117
    readonly property int th: 117

    Item {
        id: stage
        transformOrigin: Item.TopLeft
        width: root.cols * root.tw
        height: root.rows * root.th

        Repeater {
            model: root.cols * root.rows
            FBController {
                framebufferID: 5000000 + index
                x: (index % root.cols) * root.tw
                y: Math.floor(index / root.cols) * root.th
                width: root.tw
                height: root.th
                onDragDown: if (index < root.cols) root.close()   // a drag down from the top edge closes the app, as for any AppLoad framebuffer app
            }
        }
    }

    // How big the canvas is on the screen, measured, since the window can be moved, maximized or resized: panel pixels per canvas unit.
    function fit() {
        const a = root.mapToItem(null, 0, 0), b = root.mapToItem(null, root.width, root.height);
        const net = (b.x - a.x) / root.width;
        if (!(net > 0)) return;
        const s = 1 / net, x = -a.x / net, y = -a.y / net;
        if (Math.abs(stage.scale - s) > 1e-9 || Math.abs(stage.x - x) > 1e-6 || Math.abs(stage.y - y) > 1e-6) {
            stage.scale = s; stage.x = x; stage.y = y;
            console.log("jean-tiles: canvas " + root.width + "x" + root.height + " is " + (b.x - a.x) + "x" + (b.y - a.y) + " px on screen at " + a.x + "," + a.y + "; stage scale " + s);
        }
    }
    Timer { interval: 400; running: true; repeat: true; triggeredOnStart: true; onTriggered: root.fit() }
}
