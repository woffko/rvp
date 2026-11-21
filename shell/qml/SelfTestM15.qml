// M15's self-test (`rvp --selftest=m15`), loaded by Main.qml's self-test:
// the control API turned on by the Remote page's setting and called as a
// user's script calls it (HTTP on 127.0.0.1 with the token): the status,
// an action, the volume and a seek reach the window's thread, a request
// without the token is refused, a nonce is handed out without one and a
// signature that does not match it is refused, and turning the API off
// closes its port. The port is --remote-port (48932 by default, so that
// the owner's RVP on 48931 is left alone).
//
// With --remote-serve=SECONDS the API first waits that long, or until
// --remote-expect=N commands came from outside (m15-all.ps1's check that
// Unified Remote's server reaches RVP), and the checks report them: each
// was signed, and no request from outside carried the token (the remote
// never sends it). The test's own calls follow.
//
// File: testdata/media/fixture-60s.mkv. The token never goes into the log.
import QtQuick
import Rvp
import RvpNative

Item {
    id: m15
    required property Player player
    required property var t
    required property var host
    property var info: ({})
    property string token: ""
    property string base: ""
    // The last answer: { status, body } (null while one is awaited).
    property var answer: null
    property real serveUntil: 0
    property int commandsBefore: 0
    property int signedBefore: 0
    property int bearerBefore: 0
    property int refusedBefore: 0

    function setSetting(path, value) {
        return JSON.parse(player.set_setting(path, JSON.stringify(value)))
    }
    // `authorization`: none given, the token (Bearer); false, nothing; a
    // text, that header.
    function call(method, path, body, authorization) {
        answer = null
        const request = new XMLHttpRequest()
        request.onreadystatechange = function() {
            if (request.readyState !== XMLHttpRequest.DONE)
                return
            let value = null
            try {
                value = JSON.parse(request.responseText)
            } catch (error) {
                value = request.responseText
            }
            m15.answer = { status: request.status, body: value }
        }
        request.open(method, base + path)
        if (authorization === undefined)
            request.setRequestHeader("Authorization", "Bearer " + token)
        else if (authorization !== false)
            request.setRequestHeader("Authorization", authorization)
        if (body !== undefined) {
            request.setRequestHeader("Content-Type", "application/json")
            request.send(JSON.stringify(body))
        } else {
            request.send()
        }
    }
    function answered(status) {
        return answer !== null && answer.status === status
    }
    function shown() {
        return "answer " + JSON.stringify(answer) + " info " + JSON.stringify(info)
    }

    function tick() {
        const t = m15.t
        const s = t.status()
        info = JSON.parse(player.control_info())
        switch (t.step) {
        case 0:
            t.next()
            break
        case 1:
            if (t.waitFor("m15_file_plays", () => s.loaded && String(s.path).endsWith("fixture-60s.mkv"), 20000,
                          () => String(s.path))) {
                const port = Number(player.option("remote-port")) || 48932
                setSetting("remote.port", port)
                setSetting("remote.enabled", true)
            }
            break
        case 2:
            if (t.waitFor("m15_api_listens", () => info.listening === true
                              && String(info.address).startsWith("http://127.0.0.1:"), 5000, shown)) {
                token = player.control_token()
                t.check("m15_token_made", /^[0-9a-f]{64}$/.test(token), "length " + token.length)
                base = String(info.address).replace(/\/$/, "")
            }
            break
        case 3: {
            // With --remote-serve, commands from outside first (Unified
            // Remote's server), before this test's own calls.
            const serve = Number(player.option("remote-serve")) || 0
            if (serve > 0 && serveUntil === 0) {
                serveUntil = Date.now() + serve * 1000
                commandsBefore = info.commands
                signedBefore = info.signed
                bearerBefore = info.bearer
                refusedBefore = info.refused
                Tools.log("m15_serving address=" + base + " seconds=" + serve)
                break
            }
            if (serve > 0) {
                const expected = Number(player.option("remote-expect")) || 1
                const came = info.commands - commandsBefore
                if (came < expected && Date.now() <= serveUntil)
                    break
                const signed = info.signed - signedBefore
                const bearer = info.bearer - bearerBefore
                const refused = info.refused - refusedBefore
                t.check("m15_outside_commands", came >= expected,
                        came + " of " + expected + " commands; state " + (s.pause ? "paused" : "playing")
                        + " volume " + s.volume + " mute " + s.mute)
                t.check("m15_outside_signed", signed >= expected,
                        signed + " signed requests, " + refused + " refused")
                t.check("m15_outside_no_token", bearer === 0, bearer + " requests with the token")
            }
            call("GET", "/api/status")
            t.next()
            break
        }
        case 4:
            if (t.waitFor("m15_status_answers", () => answered(200) && answer.body.state === "playing"
                              && String(answer.body.file).endsWith("fixture-60s.mkv"), 5000, shown))
                call("POST", "/api/action/play-pause")
            break
        case 5:
            if (t.waitFor("m15_action_reaches_rvp", () => answered(200) && s.pause === true, 5000, shown))
                call("POST", "/api/volume", { percent: 30 })
            break
        case 6:
            if (t.waitFor("m15_volume_reaches_rvp", () => answered(200) && Math.abs(s.volume - 30) < 1, 5000,
                          () => shown() + " volume " + s.volume))
                call("POST", "/api/seek", { to: 20 })
            break
        case 7:
            if (t.waitFor("m15_seek_reaches_rvp", () => answered(200) && Math.abs(s.time - 20) < 1.5, 5000,
                          () => shown() + " time " + s.time))
                call("GET", "/api/status", undefined, false)
            break
        case 8:
            if (t.waitFor("m15_no_token_refused", () => answered(401), 5000, shown))
                call("GET", "/api/nonce", undefined, false)
            break
        case 9:
            // A nonce without the token; a signature that is not the
            // token's is refused, and the action does not run.
            if (t.waitFor("m15_nonce_given", () => answered(200) && /^[0-9a-f]{32}$/.test(answer.body.nonce),
                          5000, shown))
                call("POST", "/api/action/play-pause", undefined,
                     "RVP-HMAC-SHA256 nonce=" + (answer && answer.body ? answer.body.nonce : "")
                     + ", signature=" + "0".repeat(64))
            break
        case 10:
            if (t.waitFor("m15_bad_signature_refused", () => answered(401) && s.pause === true, 5000, shown))
                call("POST", "/api/action/play-pause")
            break
        case 11:
            if (t.waitFor("m15_playback_again", () => answered(200) && s.pause === false, 5000, shown))
                setSetting("remote.enabled", false)
            break
        case 12:
            if (t.waitFor("m15_api_off", () => info.listening === false, 5000, shown))
                call("GET", "/api/status")
            break
        case 13:
            if (t.waitFor("m15_port_closed", () => answer !== null && answer.status === 0, 5000, shown))
                t.finish()
            break
        }
    }
}
